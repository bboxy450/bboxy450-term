#![allow(clippy::disallowed_methods)] // Disposable file/process fault fixtures.
use bbt_core::{Color, NamedColor};
use bbt_mux::ring::{Playback, Ring, Settings, crc32c};
use bbt_mux::streams::Screen;
use bbt_mux::{Event, SeatSize, Ulid};
use std::io::Write;
use std::path::Path;

fn fixture(path: &Path, settings: Settings) -> (Ring, Playback) {
    let seat = Ulid::new();
    let live = Playback::new(
        seat,
        1,
        SeatSize {
            cols: 80,
            rows: 24,
            ..Default::default()
        },
    );
    let mut ring = Ring::open(path, seat, 1, 0, settings).unwrap();
    ring.checkpoint(live.saved().encode().unwrap()).unwrap();
    (ring, live)
}
fn output(ring: &mut Ring, live: &mut Playback, bytes: &[u8]) {
    let event = Event::Output {
        seat: live.seat,
        generation: live.generation,
        seq: live.sequence + 1,
        bytes: bytes.to_vec(),
    };
    ring.append_event(&event).unwrap();
    live.apply(&event).unwrap();
}
fn screen(live: &Playback) -> Screen {
    Screen::from_term(&live.term, live.title.clone(), live.cwd.clone(), None)
}
fn data(path: &Path) -> std::path::PathBuf {
    let bytes = std::fs::read(path.join("committed")).unwrap();
    let meta: serde_json::Value = serde_json::from_slice(&bytes[..bytes.len() - 4]).unwrap();
    path.join(meta["file"].as_str().unwrap())
}

#[test]
fn ring_red_sgr_checkpoint_restores_colour_and_replay_matches_every_live_cell() {
    assert_eq!(crc32c(b"123456789"), 0xe3069283);
    let temp = tempfile::tempdir().unwrap();
    let (mut ring, mut live) = fixture(temp.path(), Settings::default());
    output(&mut ring, &mut live, b"\x1b[31m");
    output(&mut ring, &mut live, &vec![b'R'; 4096]);
    ring.checkpoint(live.saved().encode().unwrap()).unwrap();
    output(&mut ring, &mut live, b"\r\nred tail");
    let replay = ring.replay().unwrap();
    assert_eq!(screen(&replay), screen(&live));
    assert!(
        replay
            .term
            .grid()
            .display_iter()
            .any(|c| c.cell.c == 'r' && c.cell.fg == Color::Named(NamedColor::Red))
    );
    drop(ring);
    let ring = Ring::open(temp.path(), live.seat, 1, 0, Settings::default()).unwrap();
    assert_eq!(screen(&ring.replay().unwrap()), screen(&live));
}

#[test]
fn ring_checkpoint_preserves_incomplete_csi_utf8_osc_and_suppresses_replayed_effects() {
    for (before, after) in [
        (b"\x1b[3".as_slice(), b"1mred\x1b[6n".as_slice()),
        (b"\xf0\x9f".as_slice(), b"\xa6\x8a".as_slice()),
        (
            b"\x1b]2;captured".as_slice(),
            b" title\x07\x1b]52;c;YQ==\x07".as_slice(),
        ),
        (
            b"\x1b[?2026hheld".as_slice(),
            b" text\x1b[?2026l".as_slice(),
        ),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let (mut ring, mut live) = fixture(temp.path(), Settings::default());
        output(&mut ring, &mut live, before);
        ring.checkpoint(live.saved().encode().unwrap()).unwrap();
        output(&mut ring, &mut live, after);
        let mut replay = ring.replay().unwrap();
        assert_eq!(screen(&replay), screen(&live));
        assert!(replay.term.take_events().is_empty());
    }
}

#[test]
fn ring_rejects_uncommitted_and_torn_suffix_then_can_commit_and_reopen_again() {
    let temp = tempfile::tempdir().unwrap();
    let (mut ring, mut live) = fixture(temp.path(), Settings::default());
    output(&mut ring, &mut live, b"first");
    let first = screen(&live);
    let seat = live.seat;
    let file = data(temp.path());
    std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .write_all(b"uncommitted19bytes!!")
        .unwrap();
    drop(ring);
    let mut ring = Ring::open(temp.path(), seat, 1, 0, Settings::default()).unwrap();
    assert!(ring.status().recovery_notice.is_some());
    assert_eq!(screen(&ring.replay().unwrap()), first);
    let length = std::fs::metadata(&file).unwrap().len();
    output(&mut ring, &mut live, b"SECOND");
    drop(ring);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&file)
        .unwrap()
        .set_len(length + 56 + 2)
        .unwrap();
    let mut ring = Ring::open(temp.path(), seat, 1, 0, Settings::default()).unwrap();
    assert_eq!(ring.status().committed_sequence, 1);
    assert_eq!(screen(&ring.replay().unwrap()), first);
    let mut recovered = ring.replay().unwrap();
    output(&mut ring, &mut recovered, b"replacement");
    drop(ring);
    let ring = Ring::open(temp.path(), seat, 1, 0, Settings::default()).unwrap();
    assert_eq!(screen(&ring.replay().unwrap()), screen(&recovered));
}

#[test]
fn ring_eviction_keeps_a_checkpoint_boundary_two_checkpoints_and_cold_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let settings = Settings {
        capacity_bytes: 32 * 1024,
        checkpoint_bytes: 1024,
        idle_ms: 25,
    };
    let (mut ring, mut live) = fixture(temp.path(), settings.clone());
    for i in 0..240 {
        output(
            &mut ring,
            &mut live,
            format!("{i:05} {}\r\n", "x".repeat(220)).as_bytes(),
        );
        if i % 4 == 3 {
            ring.checkpoint(live.saved().encode().unwrap()).unwrap();
        }
        assert!(ring.status().hot_bytes <= settings.capacity_bytes);
    }
    output(&mut ring, &mut live, b"tail after checkpoint");
    assert!(ring.status().first_sequence > 1);
    let page = ring.history(1, live.sequence).unwrap();
    assert!(page.gap.is_some());
    assert_eq!(
        page.gap.unwrap().retained_from,
        Some(ring.status().first_sequence)
    );
    assert_eq!(screen(&ring.replay().unwrap()), screen(&live));
    let raw = std::fs::read(data(temp.path())).unwrap();
    assert_eq!(u16::from_le_bytes(raw[6..8].try_into().unwrap()), 4);
    let mut offset = 0;
    let mut checkpoints = 0;
    while offset < raw.len() {
        if u16::from_le_bytes(raw[offset + 6..offset + 8].try_into().unwrap()) == 4 {
            checkpoints += 1;
        }
        offset +=
            56 + u32::from_le_bytes(raw[offset + 8..offset + 12].try_into().unwrap()) as usize;
    }
    assert!(checkpoints >= 2);
    use bbt_mux::store::{Blobs, blobs::LocalBlobs};
    assert!(
        !LocalBlobs::open(temp.path())
            .unwrap()
            .list()
            .unwrap()
            .is_empty()
    );
    drop(ring);
    let ring = Ring::open(temp.path(), live.seat, 1, 0, settings).unwrap();
    assert_eq!(screen(&ring.replay().unwrap()), screen(&live));
}

#[test]
fn ring_ten_thousand_logical_lines_round_trip_with_paginated_history_and_index() {
    let temp = tempfile::tempdir().unwrap();
    let (mut ring, mut live) = fixture(temp.path(), Settings::default());
    let mut fixture = Vec::new();
    for i in 0..10000 {
        fixture.extend(format!("line {i:05}\r\n").as_bytes());
    }
    for (i, chunk) in fixture.chunks(2048).enumerate() {
        output(&mut ring, &mut live, chunk);
        if i % 5 == 4 {
            ring.checkpoint(live.saved().encode().unwrap()).unwrap();
        }
    }
    ring.checkpoint(live.saved().encode().unwrap()).unwrap();
    assert_eq!(live.tracker.row, 10000);
    let mut from = 1;
    let mut bytes = Vec::new();
    while from <= live.sequence {
        let page = ring.history(from, live.sequence).unwrap();
        assert!(page.gap.is_none());
        for record in page.records {
            assert!(record.row.is_some());
            if let Event::Output { bytes: part, .. } = record.event {
                bytes.extend(part);
            }
        }
        from = page.next.unwrap_or(live.sequence + 1);
    }
    assert_eq!(bytes, fixture);
    assert_eq!(screen(&ring.replay().unwrap()), screen(&live));
}

#[test]
fn ring_logical_rows_ignore_osc_payload_newlines_and_advance_epochs_without_invented_tail_indexes()
{
    let temp = tempfile::tempdir().unwrap();
    let (mut ring, mut live) = fixture(temp.path(), Settings::default());
    output(&mut ring, &mut live, b"one\n\x1b]2;not\na row\x07two\n");
    assert_eq!(live.tracker.row, 2);
    assert!(ring.history(1, 1).unwrap().records[0].row.is_none());
    ring.checkpoint(live.saved().encode().unwrap()).unwrap();
    assert_eq!(
        ring.history(1, 1).unwrap().records[0]
            .row
            .as_ref()
            .unwrap()
            .last_row,
        2
    );
    output(&mut ring, &mut live, b"\x1b[?25;1049h\x1b[2J\x1bD\x1bE");
    let resize = Event::Resized {
        seat: live.seat,
        generation: 1,
        seq: live.sequence + 1,
        size: SeatSize {
            cols: 90,
            rows: 30,
            ..Default::default()
        },
    };
    ring.append_event(&resize).unwrap();
    live.apply(&resize).unwrap();
    assert_eq!(live.tracker.epoch, 3);
    assert_eq!(live.tracker.row, 4);
    assert_eq!(screen(&ring.replay().unwrap()), screen(&live));
}

#[test]
fn ring_rejects_wrong_generation_sequence_and_corruption_without_unbounded_allocation() {
    let temp = tempfile::tempdir().unwrap();
    let (mut ring, mut live) = fixture(temp.path(), Settings::default());
    let event = Event::Output {
        seat: live.seat,
        generation: 2,
        seq: 1,
        bytes: vec![1],
    };
    assert!(ring.append_event(&event).is_err());
    output(&mut ring, &mut live, b"data");
    assert!(Ring::open(temp.path(), live.seat, 2, 0, Settings::default()).is_err());
    let file = data(temp.path());
    drop(ring);
    let mut bytes = std::fs::read(&file).unwrap();
    bytes[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    std::fs::write(file, bytes).unwrap();
    assert!(Ring::open(temp.path(), live.seat, 1, 0, Settings::default()).is_err());
}
