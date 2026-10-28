// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "isolated hook subprocess fixture"
)]
use bboxy450_harness_wake_up::{
    Result, Ulid,
    hook::{self, HookSink, ShimEvent},
};
use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    time::Instant,
};

struct Sink;
impl HookSink for Sink {
    fn send(&self, seat: Ulid, event: ShimEvent, until: Instant) -> Result<()> {
        std::fs::write(
            std::env::var("HK0_HOOK_REPORT")?,
            serde_json::to_vec(&serde_json::json!({
                "seat":seat, "event":event, "remaining_ms":until.saturating_duration_since(Instant::now()).as_millis()
            }))?,
        )?;
        Ok(())
    }
}

/// Invoked only as an explicitly detached child by the parent below; it cannot
/// obtain the operator's controlling terminal and mutates no global environment.
// keeps: spec harness-kits §5
#[test]
#[ignore = "private subprocess helper, exercised by hook_cli_uses_the_caller_sink"]
fn hook_fixture_child() {
    let raw = std::env::var("HK0_HOOK_RAW").unwrap();
    hook::run_cli(&["--agent".into(), "claude".into(), raw], &Sink);
}

// keeps: spec harness-kits §5
#[test]
fn hook_cli_uses_the_caller_sink_for_registration_and_tty_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let id = Ulid::new();
    for registration in [true, false] {
        let report = temp.path().join(format!("{registration}.json"));
        let raw = serde_json::json!({
            "hook_event_name":if registration {"SessionStart"} else {"PreToolUse"},
            "session_id":"native-session", "tool_use_id":"fixture-tool", "token":"never-export",
            "bboxy_native_socket":"untrusted-stdin"
        })
        .to_string();
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args(["--ignored", "--exact", "hook_fixture_child", "--nocapture"])
            .env("BBOXY_SEAT_ID", id.to_string())
            .env("HK0_HOOK_REPORT", &report)
            .env("HK0_HOOK_RAW", raw)
            .env_remove("CLAUDE_CODE_MESSAGING_SOCKET")
            .stdin(Stdio::null());
        if registration {
            child.env(
                "CLAUDE_CODE_MESSAGING_SOCKET",
                "/tmp/owned-fixture-native.sock",
            );
        }
        // SAFETY: setsid is async-signal-safe and called in this owned child's
        // pre-exec path, before the Rust runtime starts. No tty is opened here.
        unsafe {
            child.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let result = child.output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
        assert_eq!(value["seat"], id.to_string());
        assert_eq!(value["event"]["raw"]["session_id"], "native-session");
        assert!(value["event"]["raw"].get("token").is_none());
        assert!(value["remaining_ms"].as_u64().unwrap() <= 50);
        if registration {
            assert_eq!(
                value["event"]["raw"]["bboxy_native_socket"],
                "/tmp/owned-fixture-native.sock"
            );
        } else {
            assert!(value["event"]["raw"].get("bboxy_native_socket").is_none());
        }
    }
}
