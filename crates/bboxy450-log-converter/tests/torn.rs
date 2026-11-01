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
use std::io::{Seek, SeekFrom, Write};
fn ends(bytes: &[u8]) -> Vec<usize> {
    let mut offset = 0;
    let mut out = vec![];
    while offset < bytes.len() {
        let magic = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        if (0x184d2a50..=0x184d2a5f).contains(&magic) {
            offset +=
                8 + u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        } else {
            offset += zstd::zstd_safe::find_frame_compressed_size(&bytes[offset..]).unwrap();
            out.push(offset);
        }
    }
    out
}
#[test]
fn every_byte_of_the_last_two_frames_recovers_to_the_longest_valid_prefix() {
    let original = tempfile::tempdir().unwrap();
    let mut writer = Writer::open(original.path(), &session()).unwrap();
    for seq in 0..3 {
        writer.append(&[record(seq)], None).unwrap();
        writer.sync().unwrap();
    }
    writer.close().unwrap();
    let full = std::fs::read(original.path().join("000001.bxl")).unwrap();
    let boundaries = ends(&full);
    for cut in boundaries[0]..=boundaries[2] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("000001.bxl"), &full[..cut]).unwrap();
        std::fs::copy(original.path().join("000001.idx"), root.join("000001.idx")).unwrap();
        let expected = boundaries.iter().filter(|end| **end <= cut).count() as u64;
        let first = recover(root, &session()).unwrap();
        let stable = std::fs::read(root.join("000001.bxl")).unwrap();
        assert_eq!(first.next_event_seq, expected, "cut {cut}");
        assert_eq!(seqs(root), (0..expected).collect::<Vec<_>>());
        assert_eq!(
            recover(root, &session()).unwrap(),
            first,
            "second recovery at {cut}"
        );
        assert_eq!(std::fs::read(root.join("000001.bxl")).unwrap(), stable);
        let mut writer = Writer::open(root, &session()).unwrap();
        writer.append(&[record(expected)], None).unwrap();
        writer.close().unwrap();
        let after = std::fs::read(root.join("000001.bxl")).unwrap();
        assert_eq!(&after[..stable.len()], stable);
        assert_eq!(seqs(root), (0..=expected).collect::<Vec<_>>());
    }
}
#[test]
fn corrupt_checksum_does_not_yield_and_recovery_discards_only_the_bad_tail() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let mut writer = Writer::open(root, &session()).unwrap();
    writer.append(&[record(0)], None).unwrap();
    writer.sync().unwrap();
    let prefix = std::fs::read(root.join("000001.bxl")).unwrap();
    writer.append(&[record(1)], None).unwrap();
    writer.sync().unwrap();
    let mut full = std::fs::read(root.join("000001.bxl")).unwrap();
    writer.close().unwrap();
    let last = full.len() - 1;
    full[last] ^= 1;
    std::fs::write(root.join("000001.bxl"), &full).unwrap();
    let mut reader = Reader::open(root).unwrap();
    assert_eq!(reader.next_event().unwrap().unwrap().value().seq, 0);
    assert!(reader.next_event().is_err());
    recover(root, &session()).unwrap();
    assert_eq!(std::fs::read(root.join("000001.bxl")).unwrap(), prefix);
    assert_eq!(seqs(root), vec![0]);
    // The in-place-write control changes an accepted prefix; the real writer never does.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(root.join("000001.bxl"))
        .unwrap();
    file.seek(SeekFrom::Start(12)).unwrap();
    file.write_all(&[255]).unwrap();
    file.sync_all().unwrap();
    assert_ne!(std::fs::read(root.join("000001.bxl")).unwrap(), prefix);
}
