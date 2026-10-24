#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
mod common;

use bbt_mux::{
    Supervisor, Ulid,
    adapters::{
        HarnessRecord,
        launch::LaunchOptions,
    },
    logic::{Logic, LogicConfig},
    registry::SeatRecord,
};
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

const PROMPT: &str =
    "ROLE: EXECUTOR\nFollow the assigned settings prompt: \"literal\" \\ path · next line.\n";

fn wait_for(mut predicate: impl FnMut() -> bool, description: &str) {
    let until = Instant::now() + Duration::from_secs(10);
    while !predicate() {
        assert!(Instant::now() < until, "{description}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn config_fixture(root: &Path) {
    for (file, contents) in [
        (
            ".claude/settings.json",
            r#"{"operator":"keep", "mcpServers":{"existing":{}}}"#,
        ),
        (".claude/settings.local.json", "operator-local"),
        (".claude/plugins/operator/plugin.json", "operator-plugin"),
        (".claude/skills/operator/SKILL.md", "operator-skill"),
        (".codex/config.toml", "operator = 'keep'\n"),
        (".codex/hooks.json", r#"{"operator":"keep"}"#),
    ] {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}

#[test]
fn first_logic_launch_wires_both_fixture_clients_without_user_config_writes() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("operator home");
    let before = temp.path().join("before");
    config_fixture(&home);
    config_fixture(&before);
    let cwd_config = temp.path().join(".mcp.json");
    fs::write(&cwd_config, "operator cwd MCP").unwrap();
    let repository = temp.path().join(".bboxy");
    fs::create_dir(&repository).unwrap();
    let shim = temp.path().join("our 'quoted' app");
    fs::write(
        &shim,
        include_bytes!("../../bboxy450-harness-wake-up/tests/fixtures/seat_wiring.py"),
    )
    .unwrap();
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o700)).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let run_dir = temp.path().join("run");
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run_dir.clone(),
            repository,
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[74; 32]),
        runtime.handle(),
    )
    .unwrap();

    for kind in ["claude", "codex"] {
        let mut record = SeatRecord::new(Ulid::new(), Ulid::new(), "wiring fixture", kind);
        record.role = bbt_mux::adapters::role_preset("executor").unwrap();
        let seat = record.id;
        let role = record.role.name.clone();
        let output = temp.path().join(format!("{kind}.json"));
        let mut harness = HarnessRecord::builtin(kind).unwrap();
        harness.command = vec![
            shim.to_string_lossy().into(),
            kind.into(),
            output.to_string_lossy().into(),
            shim.to_string_lossy().into(),
            PROMPT.into(),
        ];
        let info = runtime
            .block_on(logic.client.spawn_harness(
                record,
                harness,
                LaunchOptions {
                    cwd: temp.path().into(),
                    role_prompt: PROMPT.into(),
                    executable: shim.clone(),
                    user_home: home.clone(),
                    user_zdotdir: None,
                    resume: false,
                },
            ))
            .unwrap();
        println!(
            "fixture {kind} pid={} seat={seat} generation={}",
            info.pid, info.generation
        );
        wait_for(
            || output.is_file(),
            "fixture did not inspect first-launch wiring",
        );
        let report: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(report["kind"], kind);
        assert_eq!(report["identity"]["seat"], seat.to_string());
        assert_eq!(report["identity"]["generation"], "1");
        assert_eq!(report["identity"]["role"], role);
        assert_eq!(report["identity"]["capability_present"], true);
        let endpoint = runtime.block_on(logic.client.endpoint(seat)).unwrap();
        assert_eq!(
            report["identity"]["socket"],
            endpoint.path.to_string_lossy().as_ref()
        );
        assert_eq!(report["mcp"]["argv"], serde_json::json!(["mcp"]));
        assert_eq!(report["skill_present"], true);
        assert_eq!(report["role_prompt_preserved"], true);
        println!("fixture {kind} launch argv={}", report["argv"]);
        let overlays = run_dir.join(format!("seat-{seat}/overlays"));
        let overlay = fs::read_dir(&overlays)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(
            fs::metadata(&overlay).unwrap().permissions().mode() & 0o777,
            0o700
        );
        fs::write(output.with_extension("release"), "exit").unwrap();
        wait_for(
            || fs::read_dir(&overlays).unwrap().next().is_none(),
            "overlay survived child exit",
        );
        let state = runtime.block_on(logic.client.gui_seat(seat)).unwrap();
        assert_eq!(
            state.state.process,
            bbt_proto::ProcessStatus::Exited { status: 0 }
        );
    }
    logic.shutdown().unwrap();
    for directory in [".claude", ".codex"] {
        let diff = Command::new("diff")
            .arg("-r")
            .arg(before.join(directory))
            .arg(home.join(directory))
            .output()
            .unwrap();
        assert!(
            diff.status.success(),
            "diff -r: {}",
            String::from_utf8_lossy(&diff.stdout)
        );
        assert!(diff.stdout.is_empty());
        println!("fixture diff -r {directory}: exit 0, no differences");
    }
    assert_eq!(fs::read_to_string(cwd_config).unwrap(), "operator cwd MCP");
    println!(
        "real-harness orient/task_report UNPROVEN; fixture clients invoke no services or credentials"
    );
}
