// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Private process fixtures, no operator state.

use bbt_mux::{
    Config, Supervisor, Ulid,
    adapters::{HarnessRecord, launch, role_preset},
    protocol::CompanionSpec,
    registry::SeatRecord,
};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

// Independent assertions include prefix extensions and configuration lookalikes.
const MARKERS: &[&str] = &[
    "CLAUDECODE",
    "CLAUDE_PID",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_SESSION_ATTENDED",
    "CLAUDE_CODE_SESSION_FUTURE",
    "CLAUDE_CODE_PARENT_SESSION_ID",
    "CLAUDE_CODE_SUBAGENT_ID",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_CODE_MESSAGING_FUTURE",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_BRIDGE_SESSION_ID",
    "CLAUDE_CODE_BRIDGE_SESSION_FUTURE",
    "CLAUDE_CODE_REMOTE_SESSION_ID",
    "CLAUDE_CODE_CONTAINER_ID",
    "CLAUDE_CODE_WORKER_EPOCH",
    "CLAUDE_CODE_EXECPATH",
    "CLAUDE_CODE_INVOKED_SKILLS",
    "AI_AGENT",
    "CODEX_THREAD_ID",
    "CODEX_SESSION_ID",
    "CODEX_TURN_ID",
    "PI_SESSION_ID",
    "PI_SESSION_FILE",
    "OMP_SESSION_ID",
    "OMP_SESSION_FILE",
    "ICQ_CODEX_SOCKET",
    "ICQ_HARNESS_DESCRIPTOR",
];
const KEEP: &[&str] = &[
    "PATH",
    "HOME",
    "CODEX_HOME",
    "CLAUDE_CONFIG_DIR",
    "XDG_CONFIG_HOME",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_EFFORT_LEVEL",
    "CLAUDE_EFFORT",
    "CLAUDE_CODE_FORCE_SESSION_PERSISTENCE",
    "PI_CODING_AGENT_DIR",
];

fn values(path: &Path) -> BTreeMap<String, String> {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(bytes) = fs::read(path)
            && bytes.ends_with(b"\0")
        {
            return bytes
                .split(|b| *b == 0)
                .filter_map(|entry| {
                    let entry = std::str::from_utf8(entry).unwrap();
                    entry.split_once('=').map(|(k, v)| (k.into(), v.into()))
                })
                .collect();
        }
        assert!(
            Instant::now() < until,
            "private environment capture missing: {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn assert_clean(values: &BTreeMap<String, String>) {
    for key in MARKERS {
        assert!(
            !values.contains_key(*key),
            "caller session key crossed exec: {key}"
        );
    }
}

#[test]
#[ignore = "private process boundary; make test-boundaries runs it"]
fn caller_harness_markers_never_cross_any_profile_or_companion_exec() {
    if std::env::var_os("BBOXY_ENV_WITNESS_CHILD").is_some() {
        dirty_parent_supervisor_child();
        return;
    }
    let home = tempfile::tempdir().unwrap();
    // Prove marker exclusion without a persistence override, and separately
    // preserve an existing user preference without setting one in the product.
    for force in [None, Some("1")] {
        let mut process = Command::new(std::env::current_exe().unwrap());
        process.env_clear().env("BBOXY_ENV_WITNESS_CHILD", "1").args([
            "--ignored", "--exact",
            "launch_environment::caller_harness_markers_never_cross_any_profile_or_companion_exec",
        ]);
        for key in KEEP {
            if *key != "CLAUDE_CODE_FORCE_SESSION_PERSISTENCE" {
                process.env(key, "private-config-fixture");
            }
        }
        if let Some(value) = force {
            process.env("CLAUDE_CODE_FORCE_SESSION_PERSISTENCE", value);
        }
        process
            .env("HOME", home.path())
            .env("PATH", "/usr/bin:/bin");
        for key in MARKERS {
            process.env(key, "caller-session-fixture");
        }
        let output = process.output().unwrap();
        assert!(
            output.status.success(),
            "private profile child failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    }
}

fn dirty_parent_supervisor_child() {
    let root = tempfile::tempdir().unwrap();
    let spy = root.path().join("environment-spy");
    fs::write(
        &spy,
        "#!/bin/sh\n/usr/bin/env -0 > \"$1.tmp\"\n/bin/mv \"$1.tmp\" \"$1\"\nexec /bin/cat\n",
    )
    .unwrap();
    fs::set_permissions(&spy, fs::Permissions::from_mode(0o700)).unwrap();
    // This owner deliberately starts through a path that bypasses app cleanup.
    // Its launch plans must still scrub at the final exec boundary.
    for key in MARKERS {
        assert_eq!(std::env::var(key).unwrap(), "caller-session-fixture");
    }
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    for kind in ["claude", "codex", "zsh", "pi", "omp"] {
        for overrides in [false, true] {
            let output = root.path().join(format!("{kind}-{overrides}.env"));
            let service_output = root.path().join(format!("{kind}-{overrides}-service.env"));
            let mut harness = HarnessRecord::builtin(kind).unwrap();
            harness.command = vec![
                spy.to_string_lossy().into(),
                output.to_string_lossy().into(),
            ];
            if kind == "codex" {
                harness.command.push("--no-daemon".into());
            }
            let mut seat = SeatRecord::new(Ulid::new(), Ulid::new(), "env fixture", kind);
            seat.role = role_preset("executor").unwrap();
            let plan = launch::plan(
                &harness,
                &launch::LaunchContext {
                    seat: &seat,
                    cwd: root.path(),
                    role_prompt: "fixture",
                    executable: Path::new(env!("CARGO_BIN_EXE_bboxy450-term")),
                    overlay_parent: root.path(),
                    user_home: Path::new(&std::env::var_os("HOME").unwrap()),
                    user_zdotdir: None,
                    socket: Path::new("/tmp/env-fixture.sock"),
                    capability: [43; 32],
                    resume: false,
                },
            )
            .unwrap();
            let mut spec = plan.spec.clone();
            if overrides {
                for key in MARKERS {
                    spec.env
                        .insert((*key).into(), "explicit-stale-session".into());
                }
            }
            spec.companion = Some(Box::new(CompanionSpec {
                program: spy.to_string_lossy().into(),
                args: vec![service_output.to_string_lossy().into()],
                env: BTreeMap::from([(
                    "CLAUDE_CODE_MESSAGING_TOKEN".into(),
                    "stale-service".into(),
                )]),
                ready_socket: root.path().join(format!("{kind}-{overrides}.sock")),
            }));
            let info = client.spawn(spec).unwrap();
            for values in [values(&output), values(&service_output)] {
                assert_clean(&values);
                assert!(!values.contains_key("BBOXY_LAUNCH_ENV_UNSET"));
                for key in KEEP {
                    assert_eq!(
                        values.get(*key),
                        std::env::var(key).ok().as_ref(),
                        "lost {key}"
                    );
                }
                for key in [
                    "BBOXY_SOCK",
                    "BBOXY_CAPABILITY_FILE",
                    "BBOXY_ROLE",
                    "BBOXY_GENERATION",
                ] {
                    assert_eq!(values.get(key), plan.spec.env.get(key), "lost wiring {key}");
                }
            }
            let captured = values(&output);
            assert_eq!(captured["BBOXY_SEAT_ID"], info.seat.to_string());
            assert_eq!(captured["BBOXY_PROCESS_GENERATION"], "1");
        }
    }
    supervisor.shutdown().unwrap();
}

#[test]
#[ignore = "private negative process control; make test-boundaries runs it"]
fn broken_control_without_the_scrub_fails_the_environment_assertion() {
    if std::env::var_os("BBOXY_ENV_CONTROL_CHILD").is_some() {
        unscrubbed_control_child();
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .env_clear()
        .env("BBOXY_ENV_CONTROL_CHILD", "1")
        .env("CLAUDE_CODE_CHILD_SESSION", "caller-session-fixture")
        .args([
            "--ignored",
            "--exact",
            "launch_environment::broken_control_without_the_scrub_fails_the_environment_assertion",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(101), "broken control must fail");
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("caller session key crossed exec: CLAUDE_CODE_CHILD_SESSION")
    );
}

fn unscrubbed_control_child() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("broken.env");
    let output = Command::new("/bin/sh")
        .args(["-c", "/usr/bin/env -0 > \"$1\"", "control"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_clean(&values(&path));
}
