// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Disposable owned PTY fixture.
use bbt_supervisor::{Action, Config, FailureCode, LaunchSpec, Supervisor};
// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn capture_requires_an_explicit_supervisor_journal_directory() {
    let root = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let mut launch = LaunchSpec::new("/bin/sleep", root.path());
    launch.args = vec!["30".into()];
    let seat = client.spawn(launch).unwrap();
    let result = client.request(Action::AttachRing {
        seat: seat.seat,
        generation: 1,
        directory: root.path().join("seat"),
        settings: Default::default(),
    });
    let error = result.unwrap_err();
    assert_eq!(error.code, FailureCode::Invalid);
    assert!(error.message.contains("JournalDirectoryRequired"));
    assert!(!root.path().join("seat").exists());
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn supervisor_shutdown_preserves_every_already_admitted_byte_for_offline_reopen() {
    use bbt_supervisor::ring::{Ring, Settings};
    use bbt_supervisor::{Event, Reply, Ulid};
    use std::time::{Duration, Instant};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("seat");
    let supervisor = Supervisor::start(Config {
        journal_directory: Some(temp.path().join("journal")),
        ..Default::default()
    })
    .unwrap();
    let client = supervisor.connect().unwrap();
    let mut launch = LaunchSpec::new("python3", temp.path());
    launch.args = vec![
        "-c".into(),
        "import os; input(); os.write(1,b'x'*65536); input()".into(),
    ];
    let seat = client.spawn(launch).unwrap();
    let Reply::Attached { attachment, .. } = client
        .request(Action::AttachRing {
            seat: seat.seat,
            generation: 1,
            directory: root.clone(),
            settings: Settings::default(),
        })
        .unwrap()
    else {
        panic!()
    };
    client.input(&attachment, Ulid::new(), b"go\n").unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let through = loop {
        let Reply::RingRecovery { status, .. } = client
            .request(Action::RingRecovery {
                attachment: attachment.clone(),
            })
            .unwrap()
        else {
            panic!()
        };
        if status.byte_end >= 65536 {
            break status.captured_sequence;
        }
        assert!(
            Instant::now() < deadline,
            "fixture output not admitted: {status:?}"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    let mut expected = Vec::new();
    let mut from = 1;
    while from <= through {
        let Reply::RingHistory { history } = client
            .request(Action::ReadRing {
                attachment: attachment.clone(),
                from,
                to: through,
            })
            .unwrap()
        else {
            panic!()
        };
        assert!(history.gap.is_none());
        for record in history.records {
            if let Event::Output { bytes, .. } = record.event {
                expected.extend(bytes);
            }
        }
        from = history.next.unwrap_or(through + 1);
    }
    supervisor.shutdown().unwrap();
    let ring = Ring::open(&root, seat.seat, 1, 0, Settings::default()).unwrap();
    let mut from = 1;
    let mut data = Vec::new();
    while from <= through {
        let page = ring.history(from, through).unwrap();
        assert!(page.gap.is_none());
        for record in page.records {
            if let Event::Output { bytes, .. } = record.event {
                data.extend(bytes);
            }
        }
        from = page.next.unwrap_or(through + 1);
    }
    assert_eq!(
        data, expected,
        "already-admitted capture changed during shutdown"
    );
}
