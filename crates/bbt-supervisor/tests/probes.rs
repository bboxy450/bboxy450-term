// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Disposable PTY fixture, never the UI thread.
use bbt_supervisor::protocol::InputContext;
use bbt_supervisor::{
    Action, Attachment, Client, Config, Event, LaunchSpec, Reply, Supervisor, Ulid,
};
use std::time::{Duration, Instant};

fn output(client: &Client, seat: Ulid, needle: &[u8]) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut bytes = Vec::new();
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .expect("output watchdog elapsed");
        if let Event::Output {
            seat: from,
            bytes: chunk,
            ..
        } = client.recv_timeout(remaining).unwrap()
            && from == seat
        {
            bytes.extend(chunk);
            if bytes.windows(needle.len()).any(|window| window == needle) {
                return bytes;
            }
        }
    }
}

fn fixture(config: Config) -> (Supervisor, Client, Attachment) {
    let supervisor = Supervisor::start(config).unwrap();
    let client = supervisor.connect().unwrap();
    let mut spec = LaunchSpec::new("/bin/sh", std::env::current_dir().unwrap());
    spec.args = vec![
        "-c".into(),
        "stty raw -echo; printf READY; exec /bin/cat".into(),
    ];
    let info = client.spawn(spec).unwrap();
    let attachment = client.attach(info.seat, info.generation).unwrap();
    output(&client, info.seat, b"READY");
    (supervisor, client, attachment)
}

fn context(client: &Client, attachment: &Attachment) -> InputContext {
    match client
        .request(Action::InputContext {
            attachment: attachment.clone(),
        })
        .unwrap()
    {
        Reply::InputContext { context } => context,
        other => panic!("unexpected context reply: {other:?}"),
    }
}

// keeps: spec mux-daemon.md — Supervisor ↔ logic protocol
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn unconfigured_inputs_always_fence_the_context() {
    let (_supervisor, client, attachment) = fixture(Config::default());
    client.input(&attachment, Ulid::new(), b"draft").unwrap();
    output(&client, attachment.seat, b"draft");
    let mut before = context(&client, &attachment);
    assert!(before.draft);
    // No payload has special meaning unless the caller explicitly configures it.
    let stem = [0x1b, b']', b'7', b'5', b'0', b'1', b';', b'?'];
    for ending in [&[7][..], &[0x1b, b'\\'][..]] {
        let bytes = [stem.as_slice(), ending].concat();
        client.input(&attachment, Ulid::new(), &bytes).unwrap();
        assert_eq!(output(&client, attachment.seat, &bytes), bytes);
        let after = context(&client, &attachment);
        assert_eq!(
            after.epoch,
            before.epoch + 1,
            "unconfigured input did not fence"
        );
        assert!(after.draft);
        before = after;
    }
}

// keeps: spec mux-daemon.md — Supervisor ↔ logic protocol
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn configured_probes_preserve_context_and_still_deliver_once() {
    let probes = vec![b"\x1b]9000;?\x07".to_vec(), b"\x1b]9000;?\x1b\\".to_vec()];
    let (_supervisor, client, attachment) = fixture(Config {
        input_probes: probes.clone(),
        ..Default::default()
    });
    client.input(&attachment, Ulid::new(), b"draft").unwrap();
    output(&client, attachment.seat, b"draft");
    let before = context(&client, &attachment);
    assert!(before.draft);
    for bytes in &probes {
        let id = Ulid::new();
        let receipt = client.input(&attachment, id, bytes).unwrap();
        assert_eq!(receipt.input_id, id);
        client.input(&attachment, id, bytes).unwrap();
        assert_eq!(output(&client, attachment.seat, bytes), *bytes);
        let after = context(&client, &attachment);
        assert_eq!(after.epoch, before.epoch);
        assert_eq!(after.last_input_ms, before.last_input_ms);
        assert_eq!(after.draft, before.draft);
    }
    // A near match is ordinary input; the exact output also fences any
    // duplicate bytes incorrectly queued by a retried probe above.
    let near = [probes[0].as_slice(), b"x"].concat();
    client.input(&attachment, Ulid::new(), &near).unwrap();
    assert_eq!(output(&client, attachment.seat, &near), near);
    assert_eq!(context(&client, &attachment).epoch, before.epoch + 1);
}

// keeps: spec mux-daemon.md — Supervisor ↔ logic protocol
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn guarded_inputs_use_the_same_probe_allowlist() {
    let bytes = b"\x1b]9000;?\x07".to_vec();
    let (_supervisor, client, attachment) = fixture(Config {
        input_probes: vec![bytes.clone()],
        ..Default::default()
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    let before = loop {
        let before = context(&client, &attachment);
        if before.settled {
            break before;
        }
        assert!(
            Instant::now() < deadline,
            "context never settled: {before:?}"
        );
        std::thread::yield_now();
    };
    assert!(!before.draft);
    let id = Ulid::new();
    assert!(matches!(
        client.request(Action::GuardedInput {
            attachment: attachment.clone(),
            input_id: id,
            bytes: bytes.clone(),
            expected: before.clone(),
        }).unwrap(),
        Reply::Input { receipt } if receipt.input_id == id
    ));
    assert_eq!(output(&client, attachment.seat, &bytes), bytes);
    let after = context(&client, &attachment);
    assert_eq!(after.epoch, before.epoch);
    assert_eq!(after.last_input_ms, before.last_input_ms);
    assert!(!after.draft);
}
