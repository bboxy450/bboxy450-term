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
use bboxy450_log_converter::{
    bxl::*,
    ir::{self, cbor},
};
use ciborium::value::Value;
use common::*;
use std::{io::Cursor, process::Command};
fn bytes(value: &Value) -> Vec<u8> {
    let mut out = vec![];
    ciborium::ser::into_writer(value, &mut out).unwrap();
    out
}
fn value(bytes: &[u8]) -> Value {
    ciborium::de::from_reader(bytes).unwrap()
}
#[test]
fn newer_minor_unknown_header_record_event_and_cursor_parts_survive_old_reader() {
    let source = tempfile::tempdir().unwrap();
    let future = tempfile::tempdir().unwrap();
    let forwarded = tempfile::tempdir().unwrap();
    let mut event = value(&cbor::encode(&event(0, "future synthetic")).unwrap());
    let Value::Map(fields) = &mut event else {
        panic!()
    };
    fields.push((
        99.into(),
        Value::Map(vec![("future".into(), Value::Bytes(vec![0, 255]))]),
    ));
    let encoded = bytes(&event);
    let decoded = cbor::decode::<ir::Event>(&encoded).unwrap();
    let unknown = Value::Map(vec![
        (90.into(), "new_record_kind".into()),
        (91.into(), Value::Bytes(vec![9, 8, 7])),
    ]);
    let cursor = Value::Map(vec![
        (0.into(), 42.into()),
        (99.into(), "future_cursor".into()),
    ]);
    let mut writer = Writer::open(source.path(), &session()).unwrap();
    writer
        .append(
            &[Record::Unknown(unknown.clone()), Record::Event(decoded)],
            Some(cursor.clone()),
        )
        .unwrap();
    writer.sync().unwrap();
    let full = std::fs::read(source.path().join("000001.bxl")).unwrap();
    writer.close().unwrap();
    let old_header_len = u32::from_le_bytes(full[4..8].try_into().unwrap()) as usize;
    let mut header = value(&full[8..8 + old_header_len]);
    let Value::Map(fields) = &mut header else {
        panic!()
    };
    fields
        .iter_mut()
        .find(|(key, _)| *key == Value::from(2))
        .unwrap()
        .1 = 99.into();
    fields.push((99.into(), "future_header".into()));
    let payload = bytes(&header);
    let mut newer = 0x184d2a50u32.to_le_bytes().to_vec();
    newer.extend((payload.len() as u32).to_le_bytes());
    newer.extend(payload);
    newer.extend(0x184d2a5cu32.to_le_bytes());
    newer.extend(3u32.to_le_bytes());
    newer.extend([1, 2, 3]);
    newer.extend(&full[8 + old_header_len..]);
    std::fs::write(future.path().join("000001.bxl"), newer).unwrap();
    let mut reader = Reader::open(future.path()).unwrap();
    assert_eq!(reader.header.minor, 99);
    assert_eq!(reader.header.unknown.len(), 1);
    let mut records = vec![];
    let mut after = None;
    while let Some(frame) = reader.next_frame().unwrap() {
        records.extend(frame.records);
        after = frame.source_cursor;
    }
    assert_eq!(after, Some(cursor));
    assert!(
        records
            .iter()
            .any(|r| matches!(r,Record::Unknown(v) if *v==unknown))
    );
    let mut writer = Writer::open(forwarded.path(), &session()).unwrap();
    writer.append(&records, after).unwrap();
    writer.close().unwrap();
    let mut old = Reader::open(forwarded.path()).unwrap();
    assert_eq!(
        old.next_event().unwrap().unwrap().encode().unwrap(),
        encoded
    );
    assert!(old.next_event().unwrap().is_none());
}
#[test]
fn unknown_major_and_missing_dictionary_fail_explicitly() {
    let tmp = tempfile::tempdir().unwrap();
    let writer = Writer::open(tmp.path(), &session()).unwrap();
    writer.close().unwrap();
    let path = tmp.path().join("000001.bxl");
    let mut file = std::fs::read(&path).unwrap();
    let len = u32::from_le_bytes(file[4..8].try_into().unwrap()) as usize;
    let mut header = value(&file[8..8 + len]);
    let Value::Map(fields) = &mut header else {
        panic!()
    };
    fields
        .iter_mut()
        .find(|(key, _)| *key == Value::from(1))
        .unwrap()
        .1 = 2.into();
    let encoded = bytes(&header);
    assert_eq!(encoded.len(), len);
    file[8..8 + len].copy_from_slice(&encoded);
    std::fs::write(&path, file).unwrap();
    assert!(matches!(
        Reader::open(tmp.path()),
        Err(Error::Version {
            found: 2,
            supported: 1
        })
    ));
    let dict_root = tempfile::tempdir().unwrap();
    let samples = (0..100)
        .map(|n| {
            format!(
                "synthetic tool message number {n} with repeated arguments and outputs {}",
                "alpha beta gamma ".repeat(30)
            )
            .into_bytes()
        })
        .collect::<Vec<_>>();
    let dictionary = zstd::dict::from_samples(&samples, 1024).unwrap();
    let dict_id = zstd::zstd_safe::get_dict_id_from_dict(&dictionary)
        .unwrap()
        .get();
    let mut writer = Writer::with_options(
        dict_root.path(),
        &session(),
        Options {
            dictionary: Some(dictionary),
            ..Options::default()
        },
    )
    .unwrap();
    writer.append(&[record(0)], None).unwrap();
    writer.close().unwrap();
    assert_eq!(seqs(dict_root.path()), vec![0]);
    for file in std::fs::read_dir(dict_root.path().join("dicts")).unwrap() {
        std::fs::remove_file(file.unwrap().path()).unwrap();
    }
    assert!(
        matches!(Reader::open(dict_root.path()),Err(Error::DictionaryMissing {id}) if id==dict_id)
    );
}
#[test]
#[ignore = "pinned compatibility CLI runs in ci-local via make test-log-converter-native"]
fn pinned_upstream_zstd_reads_sealed_segments_and_standard_seek_table_offsets_match() {
    let tmp = tempfile::tempdir().unwrap();
    let mut writer = Writer::open(tmp.path(), &session()).unwrap();
    for n in 0..3 {
        writer.append(&[record(n)], None).unwrap();
        writer.sync().unwrap();
    }
    writer.close().unwrap();
    let path = tmp.path().join("000001.bxl");
    let binary = std::env::var("BXL_ZSTD").unwrap_or_else(|_| "zstd".into());
    let version = Command::new(&binary).arg("--version").output().unwrap();
    assert!(String::from_utf8_lossy(&version.stdout).contains("v1.5.7"));
    let output = Command::new(&binary)
        .args(["-d", "-c"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        Command::new(&binary)
            .arg("-t")
            .arg(&path)
            .output()
            .unwrap()
            .status
            .success()
    );
    let mut plain = Cursor::new(&output.stdout);
    let mut frames = 0;
    while plain.position() < output.stdout.len() as u64 {
        let item: Value = ciborium::de::from_reader(&mut plain).unwrap();
        assert!(matches!(item, Value::Array(_)));
        frames += 1;
    }
    assert_eq!(frames, 3);
    let file = std::fs::read(path).unwrap();
    let footer = &file[file.len() - 9..];
    assert_eq!(
        u32::from_le_bytes(footer[5..].try_into().unwrap()),
        0x8f92eab1
    );
    let count = u32::from_le_bytes(footer[..4].try_into().unwrap()) as usize;
    let start = file.len() - 9 - count * 8 - 8;
    assert_eq!(
        u32::from_le_bytes(file[start..start + 4].try_into().unwrap()),
        0x184d2a5e
    );
    let mut offset = 0;
    for n in 0..count {
        let row = start + 8 + n * 8;
        let size = u32::from_le_bytes(file[row..row + 4].try_into().unwrap()) as usize;
        let expected = u32::from_le_bytes(file[row + 4..row + 8].try_into().unwrap()) as usize;
        let magic = u32::from_le_bytes(file[offset..offset + 4].try_into().unwrap());
        if magic == 0xfd2fb528 {
            assert_eq!(
                zstd::stream::decode_all(&file[offset..offset + size])
                    .unwrap()
                    .len(),
                expected
            );
        } else {
            assert_eq!(expected, 0);
        }
        offset += size;
    }
    assert_eq!(offset, start);
}
