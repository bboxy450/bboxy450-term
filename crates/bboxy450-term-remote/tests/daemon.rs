// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The daemon keeps sessions across dropped links, driven only through the
//! public Client and the real binary (serve pump → daemon socket → scripted
//! fake ACP agent process). Unattended; kills only the daemon pid it recorded.
#![allow(clippy::disallowed_methods)]
use bboxy450_term_remote::client::{Client, Received, Transport};
use std::os::unix::fs::PermissionsExt;

const AGENT: &str = "#!/usr/bin/env python3\nimport sys, json\nn = 0\nfor line in sys.stdin:\n    n += 1\n    print(json.dumps({'jsonrpc': '2.0', 'method': 'echo', 'params': {'n': n}}))\n    sys.stdout.flush()\n";

fn until(c: &mut Client, mut want: impl FnMut(&Received) -> bool) -> Received {
    loop {
        let r = c.recv().unwrap().expect("link open");
        if want(&r) {
            return r;
        }
    }
}

fn shown_n(r: &Received) -> Option<u64> {
    let Received::Show { envelope, .. } = r else {
        return None;
    };
    serde_json::from_str::<serde_json::Value>(envelope).ok()?["params"]["n"].as_u64()
}

/// Kills the recorded daemon pid and removes the scratch dir, even on panic.
struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Ok(pid) = std::fs::read_to_string(self.0.join("s/daemon.pid")) {
            let _ = std::process::Command::new("kill").arg(pid.trim()).status();
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn sessions_outlive_a_dropped_link_and_queued_ops_apply_once_on_reconnect() {
    // Short path: macOS caps Unix socket paths near 104 bytes.
    let dir = std::path::PathBuf::from(format!("/tmp/bbr-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let _cleanup = Cleanup(dir.clone());
    let agent = dir.join("agent.py");
    std::fs::write(&agent, AGENT).unwrap();
    std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o700)).unwrap();
    unsafe { std::env::set_var("BBOXY450_REMOTE_AGENT", &agent) };
    let state = dir.join("s");
    let transport = Transport::Local {
        program: env!("CARGO_BIN_EXE_bboxy450-term-remote").into(),
        state: state.to_string_lossy().into(),
    };
    let mut c = Client::new(transport, "swarm");
    c.connect().unwrap();
    c.spawn("s1", dir.to_str().unwrap()).unwrap();
    until(&mut c, |r| matches!(r, Received::Cleared { .. }));
    c.op("s1", "m1", r#"{"jsonrpc":"2.0","method":"session/prompt"}"#)
        .unwrap();
    let mut got = (false, None);
    while !(got.0 && got.1.is_some()) {
        let r = c.recv().unwrap().unwrap();
        got.0 |= r
            == Received::Applied {
                message_id: "m1".into(),
            };
        got.1 = got.1.or(shown_n(&r));
    }
    assert_eq!(got.1, Some(1));

    // SSH drops. The op queued while down is sent on reconnect, exactly once,
    // to the same live agent process (its count continues at 2).
    c.disconnect();
    c.op("s1", "m2", r#"{"jsonrpc":"2.0","method":"session/prompt"}"#)
        .unwrap();
    assert!(c.is_pending("m2"), "queued while the link is down");
    c.connect().unwrap();
    let (mut n, mut applied) = (None, false);
    while !(applied && n.is_some()) {
        let r = c.recv().unwrap().expect("link open");
        applied |= r
            == Received::Applied {
                message_id: "m2".into(),
            };
        n = n.or(shown_n(&r));
    }
    assert_eq!(
        n,
        Some(2),
        "same agent process, second line, no replay of the first"
    );
    assert!(!c.is_pending("m2"));
    drop(c);

    let pid: i32 = std::fs::read_to_string(state.join("daemon.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(pid > 0, "daemon recorded its pid");
}
