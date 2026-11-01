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
#[test]
fn raw_included_compression_and_frame_budgets_are_structural() {
    let tmp = tempfile::tempdir().unwrap();
    let (options, _clock) = options();
    let mut writer = Writer::with_options(tmp.path(), &session(), options).unwrap();
    let text = "synthetic repeated source line ".repeat(1100);
    let mut native = 0;
    for seq in 0..80 {
        let event = event(seq, &text);
        native += event.raw.as_ref().unwrap().bytes.len() + 1;
        writer
            .append(&[Record::event(&event).unwrap()], None)
            .unwrap();
    }
    writer.close().unwrap();
    let file = std::fs::read(tmp.path().join("000001.bxl")).unwrap();
    assert!(file.len() * 100 <= native * 35, "{} / {native}", file.len());
    let mut offset = 0;
    let mut frames = 0;
    while offset < file.len() {
        let magic = u32::from_le_bytes(file[offset..offset + 4].try_into().unwrap());
        if (0x184d2a50..=0x184d2a5f).contains(&magic) {
            offset +=
                8 + u32::from_le_bytes(file[offset + 4..offset + 8].try_into().unwrap()) as usize;
            continue;
        }
        let compressed = zstd::zstd_safe::find_frame_compressed_size(&file[offset..]).unwrap();
        let plain = zstd::stream::decode_all(&file[offset..offset + compressed]).unwrap();
        assert!(plain.len() <= MAX_FRAME_BYTES);
        assert!(file[offset + 4] & 4 != 0);
        frames += 1;
        offset += compressed;
    }
    assert!(frames > 1);
    let mut reader = Reader::open(tmp.path()).unwrap();
    while let Some(frame) = reader.next_frame().unwrap() {
        assert!(frame.records.iter().filter(|r| r.seq().is_some()).count() <= MAX_EVENTS);
    }
    assert_eq!(seqs(tmp.path()).len(), 80);
}
#[test]
fn rotation_reserves_space_for_the_final_tables_and_keeps_old_segments_immutable() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut options, _clock) = options();
    options.segment_bytes = 4096;
    let mut writer = Writer::with_options(tmp.path(), &session(), options).unwrap();
    let mut random = 7u64;
    let mut text = String::new();
    for _ in 0..700 {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        text.push((b'a' + (random % 26) as u8) as char);
    }
    let mut previous = vec![];
    for seq in 0..30 {
        writer
            .append(
                &[Record::event(&event(seq, &format!("{seq}{text}"))).unwrap()],
                None,
            )
            .unwrap();
        writer.sync().unwrap();
        if seq == 0 {
            previous = std::fs::read(tmp.path().join("000001.bxl")).unwrap();
        }
    }
    writer.close().unwrap();
    assert!(tmp.path().join("000002.bxl").exists());
    assert_eq!(
        &std::fs::read(tmp.path().join("000001.bxl")).unwrap()[..previous.len()],
        previous
    );
    for entry in std::fs::read_dir(tmp.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|s| s == "bxl") {
            assert!(std::fs::metadata(path).unwrap().len() <= 4096);
        }
    }
    assert_eq!(seqs(tmp.path()), (0..30).collect::<Vec<_>>());
}

#[test]
#[ignore = "100 MB corpus runs only in ci-local via make test-log-converter-native"]
fn hundred_mb_raw_included_size_budget() {
    let tmp = tempfile::tempdir().unwrap();
    let (options, _clock) = options();
    let mut writer = Writer::with_options(tmp.path(), &session(), options).unwrap();
    let text = "synthetic repeated source line ".repeat(1100);
    let mut native = 0_u64;
    let mut count = 0_u64;
    while native < 100_000_000 {
        let event = event(count, &text);
        native += event.raw.as_ref().unwrap().bytes.len() as u64 + 1;
        writer
            .append(&[Record::event(&event).unwrap()], None)
            .unwrap();
        count += 1;
    }
    writer.close().unwrap();
    let compressed: u64 = std::fs::read_dir(tmp.path())
        .unwrap()
        .map(|p| p.unwrap())
        .filter(|p| p.path().extension().is_some_and(|e| e == "bxl"))
        .map(|p| p.metadata().unwrap().len())
        .sum();
    assert!(compressed * 100 <= native * 35, "{compressed} / {native}");
    let mut reader = Reader::open(tmp.path()).unwrap();
    let mut delivered = 0;
    while let Some(event) = reader.next_event().unwrap() {
        assert_eq!(event.value().seq, delivered);
        assert!(event.value().raw.is_some());
        delivered += 1;
    }
    assert_eq!(delivered, count);
}
