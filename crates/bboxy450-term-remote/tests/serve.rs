// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Relay end to end with nobody present: a real Unix socket link, a scripted
//! fake ACP agent process (stands in for `bboxy450-harness acp`), reconnect
//! with resume, a duplicate op, an epoch change and a server restart.
#![allow(clippy::disallowed_methods)]
use bboxy450_term_remote::frame::{self, Frame, VERSION};
use bboxy450_term_remote::resume::{Accept, Cursor};
use bboxy450_term_remote::serve::Server;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Echoes each stdin line as one JSON-RPC notification with a running count.
const AGENT: &str = "#!/usr/bin/env python3\nimport sys, json\nn = 0\nfor line in sys.stdin:\n    n += 1\n    print(json.dumps({'jsonrpc': '2.0', 'method': 'echo', 'params': {'n': n, 'line': line.strip()}}))\n    sys.stdout.flush()\n";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bboxy-remote-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let agent = dir.join("agent.py");
    std::fs::write(&agent, AGENT).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o700)).unwrap();
    // SAFETY-free: tests run one per process under nextest; set before any spawn.
    unsafe { std::env::set_var("BBOXY450_REMOTE_AGENT", &agent) };
    dir
}

fn connect(server: &Arc<Server>) -> UnixStream {
    let (client, end) = UnixStream::pair().unwrap();
    let server = Arc::clone(server);
    std::thread::spawn(move || {
        let writer = end.try_clone().unwrap();
        let _ = server.serve(end, writer);
    });
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut c = client;
    frame::write(
        &mut c,
        &Frame::Hello {
            version: VERSION,
            swarm_id: "swarm".into(),
            host: "loopback".into(),
        },
    )
    .unwrap();
    assert!(matches!(
        frame::read(&mut c).unwrap(),
        Some(Frame::Welcome {
            version: VERSION,
            ..
        })
    ));
    c
}

fn op(c: &mut UnixStream, id: &str, text: &str) {
    let envelope =
        format!(r#"{{"jsonrpc":"2.0","method":"session/prompt","params":{{"text":"{text}"}}}}"#);
    frame::write(
        c,
        &Frame::Op {
            session: "s1".into(),
            message_id: id.into(),
            envelope,
        },
    )
    .unwrap();
}

/// Next frame that is not an event for which the predicate is false.
fn next(c: &mut UnixStream) -> Frame {
    loop {
        match frame::read(c).unwrap().expect("frame before timeout") {
            Frame::Ping => continue, // keepalives interleave anywhere
            f => return f,
        }
    }
}

fn event_n(f: &Frame) -> Option<(u64, u64, u64)> {
    let Frame::Event {
        epoch,
        seq,
        envelope,
        ..
    } = f
    else {
        return None;
    };
    let v: serde_json::Value = serde_json::from_str(envelope).unwrap();
    Some((*epoch, *seq, v["params"]["n"].as_u64().unwrap()))
}

fn wait_event(c: &mut UnixStream) -> (u64, u64, u64) {
    loop {
        let f = next(c);
        if let Some(e) = event_n(&f) {
            return e;
        }
    }
}

fn wait_applied(c: &mut UnixStream, id: &str) {
    loop {
        match next(c) {
            Frame::Applied { message_id } if message_id == id => return,
            Frame::Refused { message_id, reason } => panic!("{message_id} refused: {reason}"),
            _ => {}
        }
    }
}

#[test]
fn relay_applies_once_resumes_after_reconnect_and_resets_on_new_epoch() {
    let dir = scratch("relay");
    let state = dir.join("state");
    let server = Arc::new(Server::open(&state).unwrap());
    let mut c = connect(&server);
    frame::write(
        &mut c,
        &Frame::Spawn {
            session: "s1".into(),
            profile: None,
            cwd: dir.to_string_lossy().into(),
        },
    )
    .unwrap();
    let mut view = Cursor::new();
    view.reset(server.epoch(), 1);

    op(&mut c, "m1", "hello");
    wait_applied(&mut c, "m1");
    let (epoch, seq, n) = wait_event(&mut c);
    assert_eq!((epoch, seq, n), (server.epoch(), 1, 1));
    assert_eq!(view.accept(epoch, seq), Accept::Apply);

    // Duplicate op: acknowledged, never written to the agent again.
    op(&mut c, "m1", "hello");
    wait_applied(&mut c, "m1");
    op(&mut c, "m2", "again");
    wait_applied(&mut c, "m2");
    let (_, seq2, n2) = wait_event(&mut c);
    assert_eq!((seq2, n2), (2, 2), "agent saw exactly two lines");

    // Link drops before the view took seq 2; a new connection resumes from the cursor.
    drop(c);
    let mut c = connect(&server);
    frame::write(
        &mut c,
        &Frame::Resume {
            session: "s1".into(),
            epoch: view.epoch,
            after_seq: view.seq,
        },
    )
    .unwrap();
    let (e, s, n) = wait_event(&mut c);
    assert_eq!((s, n), (2, 2), "replay starts after the cursor");
    assert_eq!(view.accept(e, s), Accept::Apply);
    assert_eq!(view.accept(e, 1), Accept::Duplicate);
    assert_eq!(view.accept(e, 9), Accept::Gap);

    // A cursor from another epoch gets reset + the retained tail.
    frame::write(
        &mut c,
        &Frame::Resume {
            session: "s1".into(),
            epoch: 7,
            after_seq: 2,
        },
    )
    .unwrap();
    let Frame::Reset {
        epoch, from_seq, ..
    } = next(&mut c)
    else {
        panic!("expected reset")
    };
    assert_eq!((epoch, from_seq), (server.epoch(), 1));
    let mut fresh = Cursor::new();
    assert_eq!(
        fresh.accept(epoch, 1),
        Accept::Stale,
        "nothing before reset"
    );
    fresh.reset(epoch, from_seq);
    for want in [1, 2] {
        let (e, s, _) = wait_event(&mut c);
        assert_eq!(s, want);
        assert_eq!(fresh.accept(e, s), Accept::Apply);
    }
    drop(c);

    // Restart: new epoch, sessions gone, but applied ids stay applied.
    let old = server.epoch();
    drop(server);
    let server = Arc::new(Server::open(&state).unwrap());
    assert_ne!(server.epoch(), old);
    let mut c = connect(&server);
    op(&mut c, "m1", "hello");
    wait_applied(&mut c, "m1");
    op(&mut c, "m3", "lost session");
    let Frame::Refused { message_id, .. } = next(&mut c) else {
        panic!("expected refusal")
    };
    assert_eq!(message_id, "m3");
    frame::write(
        &mut c,
        &Frame::Resume {
            session: "s1".into(),
            epoch: old,
            after_seq: 2,
        },
    )
    .unwrap();
    let Frame::Reset { epoch, .. } = next(&mut c) else {
        panic!("expected reset after restart")
    };
    assert_eq!(epoch, server.epoch());
    assert_eq!(
        view.accept(epoch, 1),
        Accept::Stale,
        "old view refuses new-epoch events until reset"
    );
    drop(c);
    drop(server);
    std::fs::remove_dir_all(&dir).unwrap();
}
