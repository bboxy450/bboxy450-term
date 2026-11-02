// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Origin: P2 task #5084, ported from bboxy450-log-converter into bboxy450-term.
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

#![allow(
    clippy::disallowed_methods,
    reason = "unattended filesystem and process fixtures, never a UI turn"
)]
mod common;
use bboxy450_log_converter::bxl::*;
use common::*;
use std::{
    io::Write,
    process::{Command, Stdio},
    time::Duration,
};
#[test]
fn subprocess_peer() {
    let Ok(root) = std::env::var("LC_P2_PEER_ROOT") else {
        return;
    };
    let root = std::path::Path::new(&root);
    let mode = std::env::var("LC_P2_PEER_MODE").unwrap();
    if mode == "writer" {
        let mut writer = Writer::open(root, &session()).unwrap();
        for seq in 0..40 {
            writer.append(&[record(seq)], None).unwrap();
            writer.sync().unwrap();
        }
        writer.close().unwrap();
    } else {
        let mut tail = Tail::open(root).unwrap();
        let mut seqs = vec![];
        for _ in 0..20 {
            seqs.push(
                tail.next_event(Duration::from_secs(15))
                    .unwrap()
                    .expect("reader progress")
                    .value()
                    .seq,
            );
        }
        if mode == "reader-a" {
            let cursor = tail.cursor();
            drop(tail);
            tail = Tail::resume(root, &cursor).unwrap();
        }
        for _ in 20..40 {
            seqs.push(
                tail.next_event(Duration::from_secs(15))
                    .unwrap()
                    .expect("reader progress")
                    .value()
                    .seq,
            );
        }
        assert_eq!(seqs, (0..40).collect::<Vec<_>>());
        std::fs::write(
            root.parent().unwrap().join(mode),
            serde_json::to_vec(&seqs).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn two_reader_processes_follow_a_writer_and_one_resumes_without_duplication() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("race.bxl.d");
    Writer::open(&root, &session()).unwrap().close().unwrap();
    let spawn = |mode| {
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "subprocess_peer", "--nocapture"])
            .env("LC_P2_PEER_ROOT", &root)
            .env("LC_P2_PEER_MODE", mode)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    let a = spawn("reader-a");
    let b = spawn("reader-b");
    let w = spawn("writer");
    for child in [w, a, b] {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for name in ["reader-a", "reader-b"] {
        let seqs: Vec<u64> =
            serde_json::from_slice(&std::fs::read(tmp.path().join(name)).unwrap()).unwrap();
        assert_eq!(seqs, (0..40).collect::<Vec<_>>());
    }
}
#[test]
fn a_partial_last_frame_never_advances_the_tail_cursor() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let live = tmp.path().join("live");
    std::fs::create_dir(&live).unwrap();
    let mut writer = Writer::open(&source, &session()).unwrap();
    writer.append(&[record(0)], None).unwrap();
    writer.sync().unwrap();
    let prefix = std::fs::read(source.join("000001.bxl")).unwrap();
    writer.append(&[record(1)], None).unwrap();
    writer.sync().unwrap();
    let full = std::fs::read(source.join("000001.bxl")).unwrap();
    writer.close().unwrap();
    let suffix = &full[prefix.len()..];
    std::fs::write(live.join("000001.bxl"), &prefix).unwrap();
    let mut tail = Tail::open(&live).unwrap();
    assert_eq!(
        tail.next_event(Duration::ZERO)
            .unwrap()
            .unwrap()
            .value()
            .seq,
        0
    );
    let cursor = tail.cursor();
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(live.join("000001.bxl"))
        .unwrap();
    file.write_all(&suffix[..suffix.len() / 2]).unwrap();
    file.sync_all().unwrap();
    for _ in 0..3 {
        assert!(tail.next_event(Duration::ZERO).unwrap().is_none());
        assert_eq!(tail.cursor(), cursor);
    }
    file.write_all(&suffix[suffix.len() / 2..]).unwrap();
    file.sync_all().unwrap();
    assert_eq!(
        tail.next_event(Duration::from_secs(1))
            .unwrap()
            .unwrap()
            .value()
            .seq,
        1
    );
}
