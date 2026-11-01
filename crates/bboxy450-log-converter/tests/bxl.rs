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
use bboxy450_log_converter::{bxl::*, ir::*};
use common::*;
use std::sync::atomic::Ordering;

#[test]
fn append_read_index_seek_blobs_cursor_and_rotation() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("test.bxl.d");
    let mut writer = Writer::open(&root, &session()).unwrap();
    assert!(matches!(
        Writer::open(&root, &session()),
        Err(Error::WriterBusy)
    ));
    writer
        .append(
            &[record(0), record(1)],
            Some(ciborium::value::Value::from(42)),
        )
        .unwrap();
    writer.sync().unwrap();
    let blob = Blob::new(b"synthetic blob".to_vec());
    writer
        .append(
            &[Record::Blob(blob.clone()), Record::Blob(blob.clone())],
            None,
        )
        .unwrap();
    writer.sync().unwrap();
    writer.seal().unwrap();
    writer
        .append(&[Record::Blob(blob), record(2)], None)
        .unwrap();
    writer.close().unwrap();
    assert_eq!(seqs(&root), vec![0, 1, 2]);
    let mut reader = Reader::open(&root).unwrap();
    assert!(reader.index_is_valid());
    let mut blobs = 0;
    let mut cursor = None;
    while let Some(frame) = reader.next_frame().unwrap() {
        if frame.source_cursor.is_some() {
            cursor = frame.source_cursor;
        }
        blobs += frame
            .records
            .iter()
            .filter(|r| matches!(r, Record::Blob(_)))
            .count();
    }
    assert_eq!(blobs, 1);
    assert_eq!(cursor, Some(42.into()));
    reader.seek_event(1).unwrap();
    assert_eq!(reader.next_event().unwrap().unwrap().value().seq, 1);
    let cursor = reader.cursor();
    let mut resumed = Reader::resume(&root, &cursor).unwrap();
    assert_eq!(resumed.next_event().unwrap().unwrap().value().seq, 2);
    assert!(resumed.next_event().unwrap().is_none());
}
#[test]
fn injected_deadline_count_byte_and_turn_end_close_frames() {
    let tmp = tempfile::tempdir().unwrap();
    let (options, clock) = options();
    let mut writer = Writer::with_options(tmp.path(), &session(), options).unwrap();
    writer.append(&[record(0)], None).unwrap();
    clock.0.store(249, Ordering::SeqCst);
    writer.poll().unwrap();
    assert_eq!(seqs(tmp.path()), Vec::<u64>::new());
    clock.0.store(250, Ordering::SeqCst);
    writer.poll().unwrap();
    assert_eq!(seqs(tmp.path()), vec![0]);
    for seq in 1..=256 {
        writer.append(&[record(seq)], None).unwrap();
    }
    assert_eq!(seqs(tmp.path()).len(), 257);
    let mut end = event(257, "");
    end.kind = EventKind::TurnEnd {
        outcome: TurnOutcome::Completed,
    };
    writer
        .append(&[Record::event(&end).unwrap()], None)
        .unwrap();
    assert_eq!(seqs(tmp.path()).len(), 258);
    writer.close().unwrap();
}
#[test]
fn recovery_is_idempotent_and_ignores_a_corrupt_index() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let mut writer = Writer::open(root, &session()).unwrap();
    writer.append(&[record(0)], None).unwrap();
    writer.close().unwrap();
    std::fs::write(root.join("000001.idx"), b"corrupt").unwrap();
    assert!(!Reader::open(root).unwrap().index_is_valid());
    assert_eq!(seqs(root), vec![0]);
    let one = recover(root, &session()).unwrap();
    let bytes = std::fs::read(root.join("000001.bxl")).unwrap();
    let two = recover(root, &session()).unwrap();
    assert_eq!(one, two);
    assert_eq!(std::fs::read(root.join("000001.bxl")).unwrap(), bytes);
    assert!(Reader::open(root).unwrap().index_is_valid());
}

#[test]
fn index_rename_keeps_existing_readers_safe_and_failed_publish_preserves_synced_frames() {
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let (options, _clock) = options();
    let mut writer = Writer::with_options(root, &session(), options).unwrap();
    let mut held = std::fs::File::open(root.join("000001.idx")).unwrap();
    let inode = held.metadata().unwrap().ino();
    let old = std::fs::read(root.join("000001.idx")).unwrap();
    writer.append(&[record(0), record(1)], None).unwrap();
    writer.sync().unwrap();
    assert_ne!(
        std::fs::metadata(root.join("000001.idx")).unwrap().ino(),
        inode
    );
    let mut held_bytes = vec![];
    held.read_to_end(&mut held_bytes).unwrap();
    assert_eq!(held_bytes, old);
    let mut reader = Reader::open(root).unwrap();
    assert_eq!(reader.next_event().unwrap().unwrap().value().seq, 0);
    let cursor = reader.cursor();
    let mut resumed = Reader::resume(root, &cursor).unwrap();
    assert_eq!(resumed.next_event().unwrap().unwrap().value().seq, 1);
    std::fs::create_dir(root.join("000001.idx.tmp")).unwrap();
    writer.append(&[record(2)], None).unwrap();
    assert!(writer.sync().is_err());
    drop(writer);
    std::fs::remove_dir(root.join("000001.idx.tmp")).unwrap();
    assert_eq!(recover(root, &session()).unwrap().next_event_seq, 3);
    assert_eq!(seqs(root), vec![0, 1, 2]);
    assert!(Reader::open(root).unwrap().index_is_valid());
}
#[test]
fn index_hint_cannot_point_beyond_its_declared_durable_prefix() {
    use ciborium::value::Value;
    let tmp = tempfile::tempdir().unwrap();
    let mut writer = Writer::open(tmp.path(), &session()).unwrap();
    writer.append(&[record(0)], None).unwrap();
    writer.close().unwrap();
    let path = tmp.path().join("000001.idx");
    let mut value: Value = ciborium::de::from_reader(std::fs::File::open(&path).unwrap()).unwrap();
    let Value::Map(fields) = &mut value else {
        panic!()
    };
    fields
        .iter_mut()
        .find(|(k, _)| *k == Value::from(0))
        .unwrap()
        .1 = 1.into();
    ciborium::ser::into_writer(&value, std::fs::File::create(path).unwrap()).unwrap();
    assert!(!Reader::open(tmp.path()).unwrap().index_is_valid());
    assert_eq!(seqs(tmp.path()), vec![0]);
}

#[test]
fn cursor_only_commit_survives_later_frames_without_a_source_position() {
    let tmp = tempfile::tempdir().unwrap();
    let mut writer = Writer::open(tmp.path(), &session()).unwrap();
    writer.append(&[], Some(42.into())).unwrap();
    writer.sync().unwrap();
    writer.append(&[record(0)], None).unwrap();
    writer.close().unwrap();
    let mut reader = Reader::open(tmp.path()).unwrap();
    assert_eq!(reader.next_event().unwrap().unwrap().value().seq, 0);
    assert!(reader.next_event().unwrap().is_none());
    assert_eq!(reader.source_cursor, Some(42.into()));
}
#[test]
fn duplicate_blob_hash_never_acknowledges_a_different_body() {
    let tmp = tempfile::tempdir().unwrap();
    let mut writer = Writer::open(tmp.path(), &session()).unwrap();
    let mut blob = Blob::new(b"synthetic blob".to_vec());
    writer.append(&[Record::Blob(blob.clone())], None).unwrap();
    writer.sync().unwrap();
    blob.bytes.push(1);
    assert!(matches!(
        writer.append(&[Record::Blob(blob)], None),
        Err(Error::Format(_))
    ));
    drop(writer);
    recover(tmp.path(), &session()).unwrap();
    let mut reader = Reader::open(tmp.path()).unwrap();
    let mut count = 0;
    while let Some(r) = reader.next_record().unwrap() {
        if let Record::Blob(blob) = r {
            assert_eq!(blob.bytes, b"synthetic blob");
            count += 1;
        }
    }
    assert_eq!(count, 1);
}
