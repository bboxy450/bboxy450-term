// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private child processes, generated file identity, no window or native custody.
#![allow(
    clippy::disallowed_methods,
    reason = "owned private-process recovery fixtures"
)]
use std::{path::Path, time::Duration};

use bboxy450_term::cockpit::{Config, Event, Session};
use bbt_mux::{
    Ulid,
    store::{Store, object_dir, sqlite::SqliteStore},
};
use bbt_proto::{GuiAction, GuiReply, GuiSeat, SpawnSpec, SwarmId, Where};

fn authority(run: &Path) -> serde_json::Value {
    let account = SqliteStore::open(&object_dir(run, "account").unwrap()).unwrap();
    let head: serde_json::Value =
        serde_json::from_slice(&account.get("identity/head").unwrap().unwrap()).unwrap();
    let context: serde_json::Value =
        serde_json::from_slice(&account.get("identity/test-account").unwrap().unwrap()).unwrap();
    serde_json::json!({"head": head, "context": context})
}

async fn spawn(session: &mut Session, cwd: &Path) -> GuiSeat {
    let operation = Ulid::new();
    session
        .send(GuiAction::Spawn {
            id: operation,
            spec: SpawnSpec {
                via: None,
                args: Vec::new(),
                env: Vec::new(),
                command: None,
                swarm: SwarmId::from_u128(57),
                harness_kind: "zsh".into(),
                name: Some("test file identity".into()),
                role: None,
                cwd: cwd.to_string_lossy().into_owned(),
                location: Where::Local,
                resume: None,
                placement: None,
            },
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match session.receive().await.unwrap() {
                GuiReply::Spawned { op, seat } if op == operation => {
                    let seats = session.list().await.unwrap();
                    return seats
                        .into_iter()
                        .find(|item| bbt_proto::SeatId::from_u128(item.record.id.0) == seat)
                        .unwrap();
                }
                GuiReply::Error { op, reason } if op == operation => {
                    panic!("spawn after file bootstrap: {reason}")
                }
                _ => {}
            }
        }
    })
    .await
    .unwrap()
}

// keeps: decision #313
#[ignore = "night: real boundary, test-policy #327 B2"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logic_restart_keeps_the_test_root_device_pty_and_operator_spawn_authority() {
    let root = tempfile::Builder::new()
        .prefix("u0-")
        .tempdir_in("/tmp")
        .unwrap();
    let repository = root.path().join("repository");
    let shell_config = root.path().join("zsh-config");
    tokio::fs::create_dir_all(&repository).await.unwrap();
    tokio::fs::create_dir_all(&shell_config).await.unwrap();
    let account = root.path().join("keys");
    let run = account.join("run");
    let mut session = Session::start(Config {
        executable: env!("CARGO_BIN_EXE_bboxy450-term").into(),
        socket: run.join("mux.sock"),
        run_dir: run.clone(),
        repository,
        shell_config: Some(shell_config),
        ctl: true,
        test_account: Some(account),
    })
    .await
    .unwrap();
    let before = authority(&run);
    assert!(
        session
            .endpoint
            .identity
            .reason
            .as_deref()
            .is_some_and(|reason| reason.starts_with("test-account:"))
    );
    let first = spawn(&mut session, root.path()).await;
    let old_logic = session.logic.id();
    let supervisor = session.supervisor.id();
    session.logic.start_kill().unwrap(); // this Session's own recorded child only
    let restored = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Event::Reply(reply) = session.event().await.unwrap()
                && let GuiReply::Seats(seats) = *reply
            {
                break seats;
            }
        }
    })
    .await
    .unwrap();
    let kept = restored
        .iter()
        .find(|seat| seat.record.id == first.record.id)
        .unwrap();
    assert_eq!(kept.record.generation, first.record.generation);
    assert_eq!(kept.state.process, first.state.process);
    assert_eq!(session.supervisor.id(), supervisor);
    assert_ne!(session.logic.id(), old_logic);
    assert_eq!(authority(&run), before);
    let second = spawn(&mut session, root.path()).await;
    assert_ne!(second.record.id, first.record.id);
    assert_eq!(authority(&run), before);
    session.shutdown().await.unwrap();
}

// keeps: decision #313
#[test]
fn test_account_selector_is_explicit_and_rejects_bad_flags_before_custody() {
    let parse = |args: &[&str]| {
        Config::test_account_from_args(
            &args
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(parse(&[]).unwrap(), None);
    assert!(parse(&["--test-account"]).is_err());
    assert!(parse(&["--test-account", "relative"]).is_err());
    assert!(parse(&["--test-account", "/a/../b"]).is_err());
    assert!(parse(&["--unattended"]).is_err());
    assert!(parse(&["--test-account-typo", "/a"]).is_err());
    assert!(parse(&["--test-account="]).is_err());
    assert!(parse(&["--unattended=true"]).is_err());
    assert_eq!(parse(&["--test-account=/a"]).unwrap(), Some("/a".into()));
    assert!(parse(&["--test-account", "/a", "--test-account", "/b"]).is_err());
    assert_eq!(
        parse(&["--unattended", "--test-account", "/a"]).unwrap(),
        Some("/a".into())
    );
}

// Only the exact private backend PIDs this fixture started, with kernel births.
// Adapted from bboxy450-term, src/cockpit/owners.rs:
// https://github.com/bboxy450/bboxy450-term
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Cleanup is test-owned, not a
// product preserve-mode action. No private owner/capability record is needed.
#[cfg(target_os = "macos")]
struct UiOnlyCleanup(Vec<(u32, (u64, u64))>);
#[cfg(target_os = "macos")]
fn ui_only_birth(pid: u32) -> Option<(u64, u64)> {
    let mut value: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of_val(&value) as i32;
    let n = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut value as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    (n == size && value.pbi_status != 5).then_some((value.pbi_start_tvsec, value.pbi_start_tvusec))
}
#[cfg(target_os = "macos")]
impl UiOnlyCleanup {
    fn gui(&mut self, receipt: &serde_json::Value) {
        for value in [
            &receipt["after"]["process"]["pid"],
            &receipt["rollback"]["pid"],
        ] {
            if let Some(pid) = value.as_u64().and_then(|p| u32::try_from(p).ok())
                && let Some(birth) = ui_only_birth(pid)
            {
                self.0.push((pid, birth));
            }
        }
    }
    fn new(session: &Session) -> Self {
        Self(
            [
                session.supervisor.id().unwrap(),
                session.logic.id().unwrap(),
            ]
            .into_iter()
            .map(|pid| (pid, ui_only_birth(pid).unwrap()))
            .collect(),
        )
    }
}
#[cfg(target_os = "macos")]
impl Drop for UiOnlyCleanup {
    fn drop(&mut self) {
        for signal in [libc::SIGTERM, libc::SIGKILL] {
            for (pid, birth) in &self.0 {
                if ui_only_birth(*pid) == Some(*birth) {
                    unsafe { libc::kill(*pid as i32, signal) };
                }
            }
            let until = std::time::Instant::now() + Duration::from_secs(2);
            while self
                .0
                .iter()
                .any(|(pid, birth)| ui_only_birth(*pid) == Some(*birth))
                && std::time::Instant::now() < until
            {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn ui_only_config(root: &Path) -> Config {
    std::fs::create_dir_all(root.join("repository")).unwrap();
    let root = root.canonicalize().unwrap();
    let account = root.join("account");
    Config {
        executable: env!("CARGO_BIN_EXE_bboxy450-term").into(),
        socket: account.join("run/mux.sock"),
        run_dir: account.join("run"),
        repository: root.join("repository"),
        shell_config: Some(account.clone()),
        ctl: true,
        test_account: Some(account),
    }
}

// keeps: invariant preserve-live-work; exact private owners, no native account.
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "private owner refusals and reattach failure; make verify-ui-only-reopen"]
async fn ui_only_reopen_guard_refuses_refresh_and_failed_attach() {
    use bboxy450_term::cockpit::preserve_backend_preflight;
    let root = tempfile::Builder::new()
        .prefix("ui-guard-")
        .tempdir_in("/tmp")
        .unwrap();
    let config = ui_only_config(root.path());
    let mut first = Session::start(config.clone()).await.unwrap();
    let _owned = UiOnlyCleanup::new(&first);
    let seat = spawn(&mut first, root.path()).await;
    let expected = preserve_backend_preflight(&config).await.unwrap();
    let identity = authority(&config.run_dir);
    first.detach(None).await.unwrap();
    let path = config.run_dir.join("logic.owner.json");
    let original = std::fs::read(&path).unwrap();
    let mut owner: serde_json::Value = serde_json::from_slice(&original).unwrap();
    owner["ready"]["ui_control_version"] = serde_json::json!(0);
    std::fs::write(&path, serde_json::to_vec(&owner).unwrap()).unwrap();
    // Deliberately omit the preservation mode to exercise the real legacy
    // refresh branch as the negative control; production has no such switch.
    let refused = if std::env::var_os("BBOXY_UI_ONLY_REMOVE_GUARD").is_some() {
        Session::start(config.clone()).await
    } else {
        Session::start_preserving_backend(config.clone(), expected.clone()).await
    };
    if let Ok(unprotected) = refused {
        unprotected.shutdown().await.unwrap();
        panic!("preservation guard removed: incompatible owner was refreshed");
    }
    std::fs::write(&path, &original).unwrap();
    assert_eq!(preserve_backend_preflight(&config).await.unwrap(), expected);
    owner = serde_json::from_slice(&original).unwrap();
    owner["ready"]["root"] = serde_json::json!(root.path().join("missing-endpoint"));
    std::fs::write(&path, serde_json::to_vec(&owner).unwrap()).unwrap();
    assert!(
        Session::start_preserving_backend(config.clone(), expected.clone())
            .await
            .is_err()
    );
    std::fs::write(&path, &original).unwrap();
    assert_eq!(preserve_backend_preflight(&config).await.unwrap(), expected);
    let mut strict = Session::start_preserving_backend(config.clone(), expected.clone())
        .await
        .unwrap();
    strict.endpoint.root = root.path().join("failed-reattach");
    strict.drop_gui_connection();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Event::Disconnected(reason) = strict.event().await.unwrap()
                && reason.contains("preserve_backend")
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert!(
        strict.shutdown().await.is_err(),
        "strict hard quit stopped adopted owners"
    );
    assert_eq!(preserve_backend_preflight(&config).await.unwrap(), expected);
    let mut cleanup = Session::start(config.clone()).await.unwrap();
    let seats = cleanup.list().await.unwrap();
    let kept = seats
        .iter()
        .find(|s| s.record.id == seat.record.id)
        .unwrap();
    assert_eq!(kept.state.process, seat.state.process);
    assert_eq!(kept.record.generation, seat.record.generation);
    assert_eq!(authority(&config.run_dir), identity);
    cleanup.shutdown().await.unwrap();
    let cold = ui_only_config(&root.path().join("cold"));
    assert!(
        Session::start_preserving_backend(cold.clone(), expected)
            .await
            .is_err()
    );
    let probe = tokio::time::timeout(
        Duration::from_secs(6),
        tokio::process::Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
            .arg("--preserve-backend-check")
            .arg("--run-dir")
            .arg(&cold.run_dir)
            .arg("--test-account")
            .arg(cold.test_account.as_ref().unwrap())
            .arg("--repository")
            .arg(&cold.repository)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(!probe.status.success());
    assert!(!cold.run_dir.join("supervisor.owner.json").exists());
    println!(
        "ui-only guards: incompatible schema, failed observer attach, failed reconnect, strict shutdown and cold startup refused; same kernel owners/PTY/generation/authority"
    );
}

#[cfg(target_os = "macos")]
async fn ui_only_handoff(root: &Path, config: &Config, failure: bool) -> serde_json::Value {
    let prepared = std::env::var_os("BBOXY_PREPARED_GUI_BUNDLE").expect("canonical bundle");
    let mut command = tokio::process::Command::new("/usr/bin/python3");
    command
        .env_clear()
        .env("HOME", root)
        .env("ZDOTDIR", config.test_account.as_ref().unwrap())
        .env("PATH", std::env::var_os("PATH").unwrap())
        .env("LANG", "en_US.UTF-8")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/reopen-gui.py"))
        .args(["--private", "--headless", "--prepared"])
        .arg(prepared)
        .arg("--destination")
        .arg(root.join("installed"))
        .arg("--run-dir")
        .arg(&config.run_dir)
        .arg("--repository")
        .arg(&config.repository)
        .arg("--test-account")
        .arg(config.test_account.as_ref().unwrap());
    if failure {
        command.arg("--verify-failure-after-detach");
    }
    let output = tokio::time::timeout(Duration::from_secs(90), command.output())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(if failure { 2 } else { 0 }),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

// The initial native GUI owner must really exit before a different GUI claims
// its window. Detaching a Session in the parent test PID cannot stand in for it.
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "owned bootstrap child of ui_only_reopen_real_gui_and_supported_or_legacy_rollback"]
async fn ui_only_bootstrap_child() {
    use bboxy450_term::cockpit::preserve_backend_preflight;
    let root = std::path::PathBuf::from(std::env::var_os("BBOXY_UI_ONLY_BOOTSTRAP").unwrap());
    let config = ui_only_config(&root);
    let mut session = Session::start(config.clone()).await.unwrap();
    let cleanup = UiOnlyCleanup::new(&session);
    let seat = spawn(&mut session, &root).await;
    let expected = preserve_backend_preflight(&config).await.unwrap();
    let receipt = serde_json::json!({"seat":seat,"expected":expected,"authority":authority(&config.run_dir),"owners":cleanup.0});
    session.detach(None).await.unwrap();
    std::fs::write(
        root.join("bootstrap.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    std::mem::forget(cleanup); // receipt transfers these exact private owners to the parent
}

// keeps: invariant preserve-live-work; canonical offscreen GUI replacement, no vendor.
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "private canonical bundles and rendered GUI; make verify-ui-only-reopen"]
async fn ui_only_reopen_real_gui_and_supported_or_legacy_rollback() {
    use bboxy450_term::cockpit::preserve_backend_preflight;
    use bboxy450_term::ctl::{Client, Endpoint};
    let root = tempfile::Builder::new()
        .prefix("ui-only-")
        .tempdir_in("/tmp")
        .unwrap();
    let config = ui_only_config(root.path());
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "test_account::ui_only_bootstrap_child",
                "--nocapture",
            ])
            .env("BBOXY_UI_ONLY_BOOTSTRAP", root.path())
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let bootstrap: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.path().join("bootstrap.json")).unwrap())
            .unwrap();
    let mut owned = UiOnlyCleanup(serde_json::from_value(bootstrap["owners"].clone()).unwrap());
    let seat: GuiSeat = serde_json::from_value(bootstrap["seat"].clone()).unwrap();
    let expected: bboxy450_term::cockpit::BackendIdentity =
        serde_json::from_value(bootstrap["expected"].clone()).unwrap();
    let identity = bootstrap["authority"].clone();
    let first = ui_only_handoff(root.path(), &config, false).await;
    owned.gui(&first);
    let next = ui_only_handoff(root.path(), &config, false).await;
    owned.gui(&next);
    assert_ne!(
        first["after"]["gui"]["instance"],
        next["after"]["gui"]["instance"]
    );
    assert_eq!(first["before"]["backend"], next["after"]["backend"]);
    assert!(next["ready"]["frame_seq"].as_u64().unwrap() > 0);
    assert_eq!(
        next["ready"]["gui"]["source_revision"],
        next["installed_revision"]
    );
    assert_eq!(preserve_backend_preflight(&config).await.unwrap(), expected);
    let failed = ui_only_handoff(root.path(), &config, true).await;
    owned.gui(&failed);
    assert_eq!(failed["restored"], true, "{failed}");
    assert_eq!(failed["rollback"]["ready"]["gui_ready"], true, "{failed}");
    assert_eq!(preserve_backend_preflight(&config).await.unwrap(), expected);
    let endpoint = Endpoint::read(&config.run_dir.join("gui.endpoint")).unwrap();
    let mut client = Client::connect(&endpoint).unwrap();
    let seats = client.seats().unwrap();
    let kept = seats
        .iter()
        .find(|s| s.record.id == seat.record.id)
        .unwrap();
    assert_eq!(kept.state.process, seat.state.process);
    assert_eq!(kept.record.generation, seat.record.generation);
    drop(client);
    let bundle = root.path().join("installed/bboxy450-term.app");
    let status = tokio::process::Command::new("make")
        .arg("fixture-ui-only-legacy")
        .arg(format!("UI_ONLY_TEST_ROOT={}", root.path().display()))
        .arg(format!("BUNDLE={}", bundle.display()))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .status()
        .await
        .unwrap();
    assert!(status.success());
    let legacy = ui_only_handoff(root.path(), &config, true).await;
    assert_eq!(legacy["restored"], true, "{legacy}");
    assert!(
        legacy["rollback"].is_null(),
        "legacy binary was launched: {legacy}"
    );
    assert!(
        legacy["rollback_skipped"]
            .as_str()
            .unwrap()
            .contains("legacy")
    );
    assert_eq!(preserve_backend_preflight(&config).await.unwrap(), expected);
    let mut cleanup = Session::start(config.clone()).await.unwrap();
    let kept = cleanup
        .list()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.record.id == seat.record.id)
        .unwrap();
    assert_eq!(kept.state.process, seat.state.process);
    assert_eq!(kept.record.generation, seat.record.generation);
    assert_eq!(authority(&config.run_dir), identity);
    cleanup.shutdown().await.unwrap();
    println!(
        "ui-only GUI: rendered new source/instance/frame; same supervisor/Logic births and seat PID/generation/authority; supported rollback guarded, legacy bundle restored without launch"
    );
}
