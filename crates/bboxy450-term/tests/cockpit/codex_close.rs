// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

#![allow(clippy::disallowed_methods)] // Private process and checkout witness.

use bbt_mux::{
    Supervisor, Ulid,
    adapters::{HarnessRecord, launch::LaunchOptions, role_preset},
    logic::{Logic, LogicConfig},
    registry::{Lifecycle, Position, SeatRecord},
};
use bbt_proto::ProcessStatus;
use ed25519_dalek::SigningKey;
use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

const CASE: &str = "cockpit::codex_close::closed_codex_seat_releases_its_checkout";
const PROBE_LIMIT: Duration = Duration::from_secs(15);

#[test]
#[ignore = "private real-Codex process boundary; make test-boundaries runs it"]
fn closed_codex_seat_releases_its_checkout() {
    if let Some(codex) = std::env::var_os("BBOXY_CLOSE_FIXTURE_CODEX") {
        witness(Path::new(&codex));
        return;
    }
    let codex = std::env::var_os("CODEX_BIN").unwrap_or_else(|| "codex".into());
    let path = std::env::var_os("PATH").unwrap();
    let binary = std::env::split_paths(&path)
        .map(|dir| dir.join(&codex))
        .find_map(|candidate| candidate.canonicalize().ok().filter(|p| p.is_file()))
        .expect("the Codex CLI is required for the private process boundary witness");
    let root = tempfile::Builder::new()
        .prefix("bb-codex-close-")
        .tempdir_in("/tmp")
        .unwrap();
    let root = root.path().canonicalize().unwrap();
    let home = root.join("home");
    let codex_home = home.join(".codex");
    let managed = codex_home.join("packages/app-server-daemon/current/bin/codex");
    fs::create_dir_all(managed.parent().unwrap()).unwrap();
    symlink(&binary, &managed).unwrap();
    let state = codex_home.join("app-server-daemon");
    fs::create_dir(&state).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(
        state.join("settings.json"),
        r#"{"shutdownGraceSeconds":0,"updater":{"autoUpdateEnabled":false}}"#,
    )
    .unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", CASE, "--nocapture"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("HOME", &home)
        .env("CODEX_HOME", &codex_home)
        .env("TERM", "xterm-256color")
        .env("LANG", "en_US.UTF-8")
        .env("BBOXY_CLOSE_FIXTURE_CODEX", &binary)
        .current_dir(&root)
        .output()
        .unwrap();
    // Keep the private root alive through cleanup, even if the child panicked.
    if state.join("daemon.pid").is_file() {
        let stopped = Command::new(&binary)
            .args(["app-server", "daemon", "stop"])
            .env_clear()
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("HOME", &home)
            .env("CODEX_HOME", &codex_home)
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(stopped.status.success(), "private daemon stop: {stopped:?}");
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{stdout}");
}

fn holders(checkout: &Path) -> String {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-nP", "-Fnpcf", "+D"])
        .arg(checkout)
        .output()
        .unwrap();
    assert!(
        output.stderr.is_empty() && matches!(output.status.code(), Some(0 | 1)),
        "{output:?}"
    );
    String::from_utf8(output.stdout).unwrap()
}

fn wait_for(description: &str, mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + PROBE_LIMIT;
    while !check() {
        assert!(Instant::now() < deadline, "{description}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn witness(codex: &Path) {
    let root = std::env::current_dir().unwrap();
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let codex_home = home.join(".codex");
    let checkouts = [root.join("closed"), root.join("control")];
    let script = root.join("mcp.py");
    fs::write(
        &script,
        r#"
import json, os, pathlib, sys
hold = open("held.txt")
pathlib.Path(sys.argv[1]).write_text(str(os.getpid()))
for line in sys.stdin:
    request = json.loads(line)
    if "id" not in request:
        continue
    if request["method"] == "initialize":
        response = {"protocolVersion": request["params"]["protocolVersion"],
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "process-fixture", "version": "1"}}
    else:
        response = {"tools": []}
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": response}), flush=True)
"#,
    )
    .unwrap();
    let mut config = String::from(
        r#"
model = "fixture-model"
model_provider = "fixture"
check_for_update_on_startup = false
cli_auth_credentials_store = "file"
[model_providers.fixture]
name = "Local no-turn fixture"
base_url = "http://127.0.0.1:1/v1"
wire_api = "responses"
requires_openai_auth = false
[features]
code_mode = true
code_mode_prewarm = true
"#,
    );
    for checkout in &checkouts {
        fs::create_dir(checkout).unwrap();
        fs::write(checkout.join("held.txt"), "held by test MCP").unwrap();
        config.push_str(&format!(
            "\n[projects.{}]\ntrust_level = \"trusted\"\n",
            json_path(checkout)
        ));
    }
    fs::write(codex_home.join("config.toml"), config).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(bbt_mux::Config {
        journal_directory: Some(root.join("journal")),
        ..Default::default()
    })
    .unwrap();
    let repository = root.join("repository");
    fs::create_dir(&repository).unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: root.join("run"),
            repository,
            ring: Default::default(),
        },
        SigningKey::from_bytes(&[42; 32]),
        runtime.handle(),
    )
    .unwrap();
    let client = &logic.client;
    let swarm = Ulid::new();
    let seats = checkouts.each_ref().map(|checkout| {
        let mut record = SeatRecord::new(Ulid::new(), swarm, "archive process witness", "codex");
        record.role = role_preset("executor").unwrap();
        let id = record.id;
        let mut harness = HarnessRecord::builtin("codex").unwrap();
        harness.hooks = false;
        let marker = checkout.with_extension("pid");
        harness.command = vec![
            codex.to_string_lossy().into_owned(),
            "-c".into(),
            "mcp_servers.close_fixture.command=\"/usr/bin/python3\"".into(),
            "-c".into(),
            format!(
                "mcp_servers.close_fixture.args=[{},{}]",
                json_path(&script),
                json_path(&marker)
            ),
            "-c".into(),
            format!("mcp_servers.close_fixture.cwd={}", json_path(checkout)),
        ];
        let info = runtime
            .block_on(client.spawn_harness(
                record,
                harness,
                LaunchOptions {
                    transport_argv: None,
                    via: None,
                    env: vec![],
                    cwd: checkout.clone(),
                    role_prompt: "No model turn for this fixture".into(),
                    executable: PathBuf::from(env!("CARGO_BIN_EXE_bboxy450-term")),
                    user_home: home.clone(),
                    user_zdotdir: None,
                    resume: false,
                },
            ))
            .unwrap();
        wait_for("real Codex MCP child did not open its checkout", || {
            if let Ok(pid) = fs::read_to_string(&marker) {
                let owners = holders(checkout);
                owners.lines().any(|line| line == format!("p{pid}")) && owners.contains("/held.txt")
            } else {
                false
            }
        });
        wait_for("seat's exact thread id was not recorded", || {
            runtime
                .block_on(client.record(id))
                .unwrap()
                .harness_session_id
                .is_some()
        });
        (info, marker)
    });
    let [(seat_a, _), (seat_b, control)] = seats;
    let (a, generation) = (seat_a.seat, seat_a.generation);
    let (b, other_generation) = (seat_b.seat, seat_b.generation);
    let process_start = bbt_mux::adapters::native::process_start;
    let control_started = process_start(seat_b.pid).unwrap();
    let daemon: serde_json::Value =
        serde_json::from_slice(&fs::read(codex_home.join("app-server-daemon/daemon.pid")).unwrap())
            .unwrap();
    let daemon_pid = daemon["pid"].as_u64().unwrap().to_string();
    let daemon_cwd = || {
        let output = Command::new("/usr/sbin/lsof")
            .args(["-nP", "-a", "-p", &daemon_pid, "-d", "cwd", "-Fn"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "fixture daemon {daemon_pid} is gone"
        );
        String::from_utf8(output.stdout).unwrap()
    };
    assert!(daemon_cwd().lines().any(|line| line == "n/"));
    let pos = Position {
        swarm,
        tab: Ulid::new(),
        split_path: vec![],
        ratios: vec![],
        layout_revision: 1,
    };
    let operation = Ulid::new();
    runtime
        .block_on(client.close(b, other_generation, operation, pos.clone(), 1000))
        .unwrap();
    let close = runtime.block_on(client.record(b)).unwrap().close.unwrap();
    runtime
        .block_on(client.undo(b, other_generation, close.revision, 1))
        .unwrap();

    let operation = Ulid::new();
    runtime
        .block_on(client.close(a, generation, operation, pos.clone(), 50))
        .unwrap();
    runtime
        .block_on(client.close(a, generation, operation, pos, 50))
        .unwrap();
    wait_for("closed Codex checkout kept a cwd or an open file", || {
        bbt_mux::store::now_ms() >= close.deadline_ms
            && runtime.block_on(client.record(a)).unwrap().lifecycle == Lifecycle::ClosedRetained
            && holders(&checkouts[0]).is_empty()
    });
    println!("real seat close: no checkout holders");
    assert!(daemon_cwd().lines().any(|line| line == "n/"));
    let control_owner = format!("p{}", fs::read_to_string(control).unwrap());
    assert!(
        holders(&checkouts[1])
            .lines()
            .any(|line| line == control_owner)
    );
    assert_eq!(process_start(seat_b.pid).unwrap(), control_started);
    // SAFETY: the supervisor yielded this exact private TUI PID at spawn,
    // and its start stamp was just revalidated. No daemon or group is signalled.
    assert_eq!(
        unsafe { libc::kill(seat_b.pid as libc::pid_t, libc::SIGHUP) },
        0
    );
    wait_for("broken control TUI still running", || {
        matches!(
            runtime.block_on(client.gui_seat(b)).unwrap().state.process,
            ProcessStatus::Exited { .. }
        )
    });
    std::thread::sleep(PROBE_LIMIT);
    assert!(
        holders(&checkouts[1])
            .lines()
            .any(|line| line == control_owner)
    );
    println!("broken control: bare TUI stop leaves the shared daemon's MCP child");
    logic.shutdown().unwrap();
}

fn json_path(path: &Path) -> String {
    serde_json::to_string(path).unwrap()
}
