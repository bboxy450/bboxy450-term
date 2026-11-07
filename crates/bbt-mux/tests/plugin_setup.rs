// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Plugin setup (#57) end to end on a private account: the pinned copy's
//! status command, its login in a bare login terminal, the recorded result,
//! and cleanup of every process the login started. Unattended: the fixture
//! login is scripted.
#![allow(
    clippy::disallowed_methods,
    reason = "owned private account, fixture plugin processes and PTYs"
)]

use crate::common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bbt_mux::{
    Supervisor, Ulid,
    logic::{Logic, LogicConfig, PLUGIN_LOGIN_HARNESS},
    mcp::{Actor, Door},
    plugins::{DiscoveryRoots, LoginCommand, Registry, RegistryConfig},
    registry::Lifecycle,
    store::sqlite::SqliteStore,
};
use bbt_plugin::registry::{SetupState, State};
use bbt_plugin_fixture::Knobs;

fn operator() -> Actor {
    Actor::Operator {
        device_id: Ulid::from(91_u128),
        door: Door::Ctl,
        issued_by: None,
    }
}

struct Fixture {
    _temp: tempfile::TempDir,
    registry: Registry,
    candidate: String,
    data_dir: PathBuf,
}

/// A trusted, inspected (never enabled) root fixture declaring `auth`.
async fn trusted(auth: &str) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    Knobs {
        auth: Some(auth.into()),
        ..Knobs::root()
    }
    .install(
        &temp
            .path()
            .join("repo/.bboxy/plugins/bboxy450-term-data_source-root-fixture"),
    );
    let registry = Registry::with_config(
        SqliteStore::open(&temp.path().join("account")).unwrap(),
        DiscoveryRoots::new(&temp.path().join("app"), &temp.path().join("repo")),
        RegistryConfig {
            descriptor_timeout: Duration::from_secs(30),
            conformance_timeout: Duration::from_secs(30),
        },
    )
    .unwrap();
    let entry = registry.refresh().unwrap().remove(0);
    let candidate = entry.candidate.id.clone();
    let trusted = registry
        .trust(
            &operator(),
            &candidate,
            entry.candidate.sha256.as_deref().unwrap(),
            None,
        )
        .unwrap();
    let inspected = registry
        .inspection(&candidate)
        .unwrap()
        .unwrap()
        .run()
        .await;
    registry.finish_inspection(inspected).unwrap();
    let data_dir = PathBuf::from(&trusted.trust.unwrap().data_dir);
    Fixture {
        _temp: temp,
        registry,
        candidate,
        data_dir,
    }
}

fn gone(pid: u32) -> bool {
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        // Existence check only: signal 0 is never delivered.
        if unsafe { libc::kill(pid as i32, 0) } == -1
            && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

// keeps: #57 setup — status, login in a bare terminal, status again; the
// result is recorded for the pin and trust/enable are untouched.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_scripted_login_in_a_bare_terminal_logs_in_and_is_recorded() {
    let f = trusted("ok").await;
    let before = f.registry.refresh().unwrap().remove(0);
    assert!(before.setup.is_none(), "no setup recorded before it ran");
    let job = f.registry.setup(&f.candidate).unwrap();
    let run = tokio::task::spawn_blocking(move || job.run())
        .await
        .unwrap();
    assert_eq!(run.result.state, SetupState::LoggedIn, "{:?}", run.result);
    assert_eq!(run.result.login_exit, Some(0));
    assert_eq!(run.result.detail, "fixture account");
    assert!(
        run.login_output.contains("FIX-1234") && !run.login_output.contains("fixture title"),
        "terminal escapes are never observed: {:?}",
        run.login_output
    );
    assert!(
        f.data_dir.join("logged-in").exists(),
        "the login ran in the data dir"
    );
    for pid in &run.pids {
        assert!(gone(*pid), "setup child {pid} survived");
    }
    let entry = f.registry.finish_setup(run).unwrap();
    assert_eq!(entry.setup.as_ref().unwrap().state, SetupState::LoggedIn);
    assert_eq!(entry.state(), State::Disabled, "setup never enables");
    // Already logged in: the status alone answers; no login runs.
    let again = f.registry.setup(&f.candidate).unwrap();
    let run = tokio::task::spawn_blocking(move || again.run())
        .await
        .unwrap();
    assert_eq!(run.result.state, SetupState::LoggedIn);
    assert_eq!(run.result.login_exit, None);
    assert_eq!(run.pids.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_login_is_recorded_with_its_exit_code() {
    let f = trusted("refuse").await;
    let job = f.registry.setup(&f.candidate).unwrap();
    let run = tokio::task::spawn_blocking(move || job.run())
        .await
        .unwrap();
    assert_eq!(run.result.state, SetupState::LoginRefused);
    assert_eq!(run.result.login_exit, Some(2));
    let entry = f.registry.finish_setup(run).unwrap();
    assert_eq!(
        entry.setup.unwrap().state,
        SetupState::LoginRefused,
        "the operator sees the refusal"
    );
}

// keeps: #57 child cleanup — a login that never ends is stopped at its
// budget with every process it started, grandchildren included.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hanging_login_and_its_children_are_ended_at_the_budget() {
    let f = trusted("hang").await;
    let mut job = f.registry.setup(&f.candidate).unwrap();
    job.login_budget = Duration::from_secs(1);
    let started = Instant::now();
    let run = tokio::task::spawn_blocking(move || job.run())
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(
        run.result.state,
        SetupState::Failed,
        "{:?} {:?}",
        run.result,
        run.login_output
    );
    assert_eq!(run.result.code.as_deref(), Some("login_deadline_exceeded"));
    let grandchild: u32 = std::fs::read_to_string(f.data_dir.join("grandchild.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    for pid in run.pids.iter().chain([&grandchild]) {
        assert!(gone(*pid), "process {pid} outlived setup");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn setup_needs_a_trusted_shape_that_declares_auth() {
    let temp = tempfile::tempdir().unwrap();
    Knobs::root().install(
        &temp
            .path()
            .join("repo/.bboxy/plugins/bboxy450-term-data_source-root-fixture"),
    );
    let registry = Registry::with_config(
        SqliteStore::open(&temp.path().join("account")).unwrap(),
        DiscoveryRoots::new(&temp.path().join("app"), &temp.path().join("repo")),
        RegistryConfig {
            descriptor_timeout: Duration::from_secs(30),
            conformance_timeout: Duration::from_secs(30),
        },
    )
    .unwrap();
    let entry = registry.refresh().unwrap().remove(0);
    let candidate = entry.candidate.id.clone();
    assert!(registry.setup(&candidate).is_err(), "untrusted");
    registry
        .trust(
            &operator(),
            &candidate,
            entry.candidate.sha256.as_deref().unwrap(),
            None,
        )
        .unwrap();
    let inspected = registry
        .inspection(&candidate)
        .unwrap()
        .unwrap()
        .run()
        .await;
    registry.finish_inspection(inspected).unwrap();
    assert_eq!(
        registry.setup(&candidate).err().unwrap().code,
        "setup_unavailable"
    );
}

// The visible login seat (#57): a bare supervisor seat run by the logic owner.
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

fn logic_config(root: &Path) -> LogicConfig {
    let repository = root.join("repository");
    std::fs::create_dir_all(repository.join(".bboxy")).unwrap();
    LogicConfig {
        ring: Default::default(),
        run_dir: root.join("run"),
        repository,
    }
}

fn start_logic(supervisor: &Supervisor, root: &Path, runtime: &tokio::runtime::Runtime) -> Logic {
    Logic::start(
        supervisor.connect().unwrap(),
        logic_config(root),
        ed25519_dalek::SigningKey::from_bytes(&[57; 32]),
        runtime.handle(),
    )
    .unwrap()
}

// A login leader that never ends, with a child that ignores SIGHUP.
fn stubborn_login(dir: &Path, budget: Duration) -> LoginCommand {
    LoginCommand {
        name: "Fixture".into(),
        program: "/bin/sh".into(),
        args: vec![
            "-c".into(),
            "(trap '' HUP; exec /bin/sleep 600) & echo $! > child.pid; echo waiting for the operator; exec /bin/sleep 600".into(),
        ],
        cwd: dir.into(),
        env: vec![("PATH".into(), "/usr/bin:/bin".into())],
        budget,
    }
}

fn pid_in(dir: &Path) -> u32 {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(text) = std::fs::read_to_string(dir.join("child.pid"))
            && let Ok(pid) = text.trim().parse()
        {
            return pid;
        }
        assert!(Instant::now() < until, "the login never started its child");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn login_rows(
    runtime: &tokio::runtime::Runtime,
    logic: &Logic,
) -> Vec<bbt_mux::seat_socket::GuiSeat> {
    runtime
        .block_on(logic.client.gui_seats())
        .unwrap()
        .into_iter()
        .filter(|row| row.record.harness_kind == PLUGIN_LOGIN_HARNESS)
        .collect()
}

// keeps: #57 bounded wait — a login seat past its budget is terminated as a
// whole process group (a SIGHUP-ignoring child included), its row closes and
// the setup job hears a timeout with the text the row showed.
#[test]
fn a_login_seat_past_its_budget_ends_its_process_group_and_closes() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("data");
    std::fs::create_dir_all(&dir).unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let logic = start_logic(&supervisor, temp.path(), &runtime);
    let started = Instant::now();
    let run = runtime
        .block_on(
            logic
                .client
                .plugin_login(Ulid::new(), stubborn_login(&dir, Duration::from_secs(1))),
        )
        .unwrap();
    assert!(run.timed_out, "{run:?}");
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(run.output.contains("waiting for the operator"), "{run:?}");
    let child = pid_in(&dir);
    assert!(gone(child), "child {child} outlived the login seat");
    let rows = login_rows(&runtime, &logic);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].record.lifecycle, Lifecycle::ClosedRetained);
    assert_eq!(rows[0].record.name, "Fixture login");
    logic.shutdown().unwrap();
}

// keeps: #57 cleanup on close — a login whose setup job was lost with its
// logic owner is ended (process group) and its row closed when the next
// owner attaches; nobody can type into an orphaned login.
#[test]
fn a_login_seat_left_by_a_stopped_logic_owner_is_ended_on_reattach() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("data");
    std::fs::create_dir_all(&dir).unwrap();
    let runtime = runtime();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let logic = start_logic(&supervisor, temp.path(), &runtime);
    let client = logic.client.clone();
    let waiting = runtime.spawn(async move {
        client
            .plugin_login(Ulid::new(), stubborn_login(&dir, Duration::from_secs(600)))
            .await
    });
    let child = pid_in(&temp.path().join("data"));
    let rows = login_rows(&runtime, &logic);
    assert_eq!(rows[0].record.lifecycle, Lifecycle::Open);
    logic.shutdown().unwrap();
    assert_eq!(
        runtime.block_on(waiting).unwrap().unwrap_err().code,
        "login_outcome_unknown"
    );
    assert!(
        !gone(child),
        "the login outlives its owner until a new owner attaches"
    );
    let logic = start_logic(&supervisor, temp.path(), &runtime);
    assert!(
        gone(child),
        "child {child} outlived the reattached login seat"
    );
    let until = Instant::now() + Duration::from_secs(10);
    while login_rows(&runtime, &logic)[0].record.lifecycle != Lifecycle::ClosedRetained {
        assert!(Instant::now() < until, "the orphaned row never closed");
        std::thread::sleep(Duration::from_millis(20));
    }
    logic.shutdown().unwrap();
}
