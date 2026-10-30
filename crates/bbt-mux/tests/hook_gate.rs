// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Production hook veto (G3): a real hook shim process, the seat socket,
//! the logic thread and the seat root's `PluginHost` deciding with a private
//! fixture plugin. The shim waits only for events the host's gate file
//! lists; everything else keeps the fire-and-forget path.
#![allow(
    clippy::disallowed_methods,
    reason = "owned private accounts, plugin fixtures and child processes; no operator account"
)]
mod common;
use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    adapters::hook::{Gate, GateEntry, GatePolicy, gate_path},
    identity::TestAccount,
    logic::{Logic, LogicConfig},
    mcp::{Actor, Door},
    plugins::{DiscoveryRoots, HostConfig, ProviderConfig, Registry, RegistryConfig},
    registry::SeatRecord,
    store::sqlite::SqliteStore,
};
use bbt_plugin::{
    policy::{FailurePolicy, Tier},
    registry::Approval,
    trust::Grants,
};
use serde_json::json;
use std::{
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn operator() -> Actor {
    Actor::Operator {
        device_id: Ulid::from(91_u128),
        door: Door::Ctl,
        issued_by: None,
    }
}

fn config() -> RegistryConfig {
    RegistryConfig {
        descriptor_timeout: Duration::from_secs(30),
        conformance_timeout: Duration::from_secs(30),
    }
}

/// Trust (with the block grant when given), check and enable a repo_graph
/// reference plugin whose `PreToolUse` before hook answers `decision`.
async fn plugin(
    temp: &Path,
    run: &Path,
    repo: &Path,
    decision: &str,
    block: Option<FailurePolicy>,
) {
    let source = include_str!("fixtures/plugins/bboxy450-term-data_source-reference")
        .replacen("SYSTEM = \"data_source\"", "SYSTEM = \"repo_graph\"", 1)
        .replacen(
            "HOOKS = []",
            r#"HOOKS = [{"event":"PreToolUse","phase":"before","budget_ms":800}]"#,
            1,
        )
        .replacen(
            "DECISION = \"allow\"",
            &format!("DECISION = {decision:?}"),
            1,
        );
    let path = repo.join(".bboxy/plugins/bboxy450-term-repo_graph-reference");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, source).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    let registry = Registry::with_config(
        SqliteStore::open(&run.join("account")).unwrap(),
        DiscoveryRoots::new(temp, repo),
        config(),
    )
    .unwrap();
    let entry = registry.refresh().unwrap().remove(0);
    let candidate = entry.candidate.id.clone();
    let approval = Approval {
        tier: Tier::Export,
        grants: Grants {
            payload_events: ["PreToolUse".to_owned()].into_iter().collect(),
            block,
            ..Default::default()
        },
    };
    registry
        .trust(
            &operator(),
            &candidate,
            entry.candidate.sha256.as_deref().unwrap(),
            Some(approval),
        )
        .unwrap();
    let inspected = registry
        .inspection(&candidate)
        .unwrap()
        .unwrap()
        .run()
        .await;
    registry.finish_inspection(inspected).unwrap();
    let checked = registry
        .conformance(&operator(), &candidate)
        .unwrap()
        .unwrap()
        .run()
        .await;
    let checked = registry.finish_conformance(checked).unwrap();
    assert!(checked.conformance.as_ref().unwrap().ok, "{checked:?}");
    registry.set_enabled(&operator(), &candidate, true).unwrap();
}

struct Shim {
    code: i32,
    stderr: String,
    elapsed: Duration,
}

/// Run the real hook shim (`adapters::hook::run_cli`) in a fresh session
/// with the seat's private environment and `raw` on stdin.
fn shim(seat: Ulid, run: &Path, socket: &Path, capability: [u8; 32], raw: &str) -> Shim {
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args(["--ignored", "--exact", "hook_shim_child", "--nocapture"])
        .env("BBOXY_SEAT_ID", seat.to_string())
        .env("BBOXY_SOCK", socket)
        .env(
            "BBOXY_CAPABILITY",
            capability
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        )
        .env("BBOXY_GENERATION", "1")
        .env("BBOXY_RUN_DIR", run)
        .env("BBOXY_HOOK_SOCKET_ONLY", "1")
        .env("HOOK_GATE_RAW", raw)
        .env_remove("CLAUDE_CODE_MESSAGING_SOCKET")
        .stdin(Stdio::null());
    // SAFETY: setsid is async-signal-safe and runs in this owned child's
    // pre-exec path; the shim can never reach the operator's terminal.
    unsafe {
        child.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let started = Instant::now();
    let output = child.output().unwrap();
    Shim {
        code: output.status.code().unwrap(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        elapsed: started.elapsed(),
    }
}

/// The shim process: `hook --agent claude <raw>`, exiting with its code.
#[test]
#[ignore = "private subprocess helper, exercised by gated_hooks_*"]
fn hook_shim_child() {
    let raw = std::env::var("HOOK_GATE_RAW").unwrap();
    let code = bbt_mux::adapters::hook::run_cli(&["--agent".into(), "claude".into(), raw]);
    std::process::exit(code);
}

async fn case(decision: &str, block: Option<FailurePolicy>) -> (Option<Gate>, Shim, Shim) {
    let temp = tempfile::tempdir().unwrap();
    let run = temp.path().join("run");
    let repo = temp.path().join("repo");
    let custody = TestAccount::open(&temp.path().join("custody"), &run).unwrap();
    plugin(temp.path(), &run, &repo, decision, block).await;
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let logic = Logic::start_custodied_with_plugin_config(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run.clone(),
            repository: repo.clone(),
            ring: Default::default(),
        },
        custody.key_store(),
        HostConfig {
            registry: config(),
            provider: ProviderConfig::default(),
        },
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let seat = Ulid::new();
    let record = SeatRecord::new(seat, Ulid::new(), "hook gate fixture", "zsh");
    let mut spec = LaunchSpec::new("/bin/cat", temp.path());
    spec.seat = seat;
    spec.generation = 1;
    logic.client.spawn(record, spec).await.unwrap();
    let endpoint = logic.client.endpoint(seat).await.unwrap();
    let (socket, capability) = (endpoint.path.clone(), endpoint.capability);
    let run_shim = |raw: String| {
        let (run, socket) = (run.clone(), socket.clone());
        tokio::task::spawn_blocking(move || shim(seat, &run, &socket, capability, &raw))
    };
    // The first observed hook registers the seat's gate with its root; the
    // workspace may still be mounting, so repeat until the gate settles.
    let gate_file = gate_path(&run, seat);
    // A harness launch creates the seat's private directory; this plain
    // `/bin/cat` seat stands in for it.
    fs::create_dir_all(gate_file.parent().unwrap()).unwrap();
    fs::set_permissions(
        gate_file.parent().unwrap(),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let start = json!({"hook_event_name":"SessionStart","session_id":"gate-fixture"}).to_string();
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        let started = run_shim(start.clone()).await.unwrap();
        assert_eq!(started.code, 0, "{}", started.stderr);
        if block.is_none() || gate_file.exists() || Instant::now() > until {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let gate = fs::read(&gate_file)
        .ok()
        .map(|bytes| serde_json::from_slice::<Gate>(&bytes).unwrap());
    let tool = json!({
        "hook_event_name":"PreToolUse","session_id":"gate-fixture",
        "tool_name":"Bash","tool_use_id":"gate-tool"
    })
    .to_string();
    let pre = run_shim(tool).await.unwrap();
    // PostToolUse has no before subscriber: always the unchanged path.
    let post = json!({
        "hook_event_name":"PostToolUse","session_id":"gate-fixture",
        "tool_name":"Bash","tool_use_id":"gate-tool"
    })
    .to_string();
    let after = run_shim(post).await.unwrap();
    logic.shutdown().unwrap();
    (gate, pre, after)
}

// keeps: G3 production veto / PluginHookGate — a blocking plugin's deny is
// honoured as the harness's documented exit 2, an allow passes, a timeout
// follows the grant's failure policy within the ceiling, and without a
// blocking plugin the shim never waits.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gated_hooks_veto_allow_time_out_by_policy_and_keep_the_fast_path() {
    for (decision, block, code, reason) in [
        (
            "deny",
            Some(FailurePolicy::FailOpen),
            2,
            Some("fixture veto"),
        ),
        ("allow", Some(FailurePolicy::FailClosed), 0, None),
        (
            "slow",
            Some(FailurePolicy::FailClosed),
            2,
            Some("hook failed closed: deadline_exceeded"),
        ),
        ("slow", Some(FailurePolicy::FailOpen), 0, None),
        ("deny", None, 0, None),
    ] {
        let (gate, pre, post) = case(decision, block).await;
        let label = format!("{decision} {block:?}: {}", pre.stderr);
        match block {
            Some(policy) => {
                let gate = gate.unwrap_or_else(|| panic!("{label}: no gate file"));
                let policy = match policy {
                    FailurePolicy::FailOpen => GatePolicy::FailOpen,
                    FailurePolicy::FailClosed => GatePolicy::FailClosed,
                };
                for name in ["PreToolUse", "pre_tool_use"] {
                    assert_eq!(
                        gate.events.get(name),
                        Some(&GateEntry {
                            policy,
                            ceiling_ms: 2_000
                        }),
                        "{label}"
                    );
                }
                assert!(!gate.events.contains_key("PostToolUse"), "{label}");
            }
            None => assert!(gate.is_none(), "{label}: an ungranted plugin gates nothing"),
        }
        assert_eq!(pre.code, code, "{label}");
        match reason {
            Some(reason) => assert!(pre.stderr.contains(reason), "{label}"),
            None => assert!(pre.stderr.is_empty(), "{label}"),
        }
        // Bounded by the 2 s before-tool ceiling minus the shim's margin,
        // plus process start.
        assert!(
            pre.elapsed < Duration::from_millis(2_500),
            "{label}: {:?}",
            pre.elapsed
        );
        if decision == "slow" {
            assert!(
                pre.elapsed >= Duration::from_millis(700),
                "{label}: {:?}",
                pre.elapsed
            );
        }
        assert_eq!(post.code, 0, "{label}");
        assert!(post.stderr.is_empty(), "{label}: {}", post.stderr);
        if block.is_none() {
            assert!(
                pre.elapsed < Duration::from_millis(700),
                "{label}: {:?}",
                pre.elapsed
            );
        }
    }
}
