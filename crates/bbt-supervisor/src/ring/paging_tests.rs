use super::*;
use crate::protocol::{Frame, Message, Reply};

// keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
#[test]
fn output_weight_bounds_real_json_for_all_bytes_and_boundary_metadata() {
    let mut record = IndexedEvent {
        event: Event::Output {
            seat: Ulid::from(u128::MAX),
            generation: u64::MAX,
            seq: u64::MAX,
            bytes: Vec::new(),
        },
        row: Some(RowPoint {
            sequence: u64::MAX,
            first_row: u64::MAX,
            last_row: u64::MAX,
            epoch: u64::MAX,
        }),
    };
    for byte in 0..=255 {
        let Event::Output { bytes, .. } = &mut record.event else {
            unreachable!()
        };
        *bytes = vec![byte; 257];
        assert!(page_weight(&record).unwrap() >= serde_json::to_vec(&record).unwrap().len());
    }
    for length in [0, 1, 255, 4096, OUTPUT_CHUNK_BYTES] {
        let Event::Output { bytes, .. } = &mut record.event else {
            unreachable!()
        };
        *bytes = (0..length).map(|n| n as u8).collect();
        assert!(page_weight(&record).unwrap() >= serde_json::to_vec(&record).unwrap().len());
        let no_row = IndexedEvent {
            event: record.event.clone(),
            row: None,
        };
        assert!(page_weight(&no_row).unwrap() >= serde_json::to_vec(&no_row).unwrap().len());
    }
    record.event = Event::Exited {
        seat: Ulid::from(u128::MAX),
        generation: u64::MAX,
        seq: u64::MAX,
        status: ExitInfo {
            code: u32::MAX,
            signal: Some("\"\\\n\u{0}🦊".into()),
        },
    };
    assert_eq!(
        page_weight(&record).unwrap(),
        serde_json::to_vec(&record).unwrap().len()
    );
    record.event = Event::Resized {
        seat: Ulid::from(u128::MAX),
        generation: u64::MAX,
        seq: u64::MAX,
        size: SeatSize {
            cols: u16::MAX,
            rows: u16::MAX,
            ..SeatSize::default()
        },
    };
    assert_eq!(
        page_weight(&record).unwrap(),
        serde_json::to_vec(&record).unwrap().len()
    );
}

pub(super) fn wire_page(page: &History) {
    let frame = Frame::new(Message::Reply {
        request_id: Ulid::from(u128::MAX),
        result: Ok(Reply::RingHistory {
            history: Box::new(page.clone()),
        }),
    });
    let bytes = crate::transport::encode(&frame).unwrap();
    assert!(bytes.len() <= crate::transport::MAX_FRAME_BYTES + 4);
    assert_eq!(
        u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize,
        bytes.len() - 4
    );
}

// keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
#[test]
#[allow(clippy::disallowed_methods)] // Owned temporary hot/cold capture fixture.
fn hot_and_cold_pages_fit_the_wire_and_keep_every_record_in_order() {
    let root = tempfile::tempdir().unwrap();
    let seat = Ulid::from(u128::MAX);
    let generation = u64::MAX;
    let mut ring = Ring::open(
        root.path(),
        seat,
        generation,
        0,
        Settings {
            capacity_bytes: 128 * 1024,
            checkpoint_bytes: 16 * 1024,
            ..Settings::default()
        },
    )
    .unwrap();
    let playback = Playback::new(seat, generation, SeatSize::default());
    ring.checkpoint(playback.saved().encode().unwrap()).unwrap();
    let mut expected = Vec::new();
    for seq in 1..=20 {
        let event = Event::Output {
            seat,
            generation,
            seq,
            bytes: (0..OUTPUT_CHUNK_BYTES)
                .map(|n| (n + seq as usize) as u8)
                .collect(),
        };
        ring.append_event(&event).unwrap();
        expected.push(event);
    }
    ring.flush().unwrap();
    assert!(!ring.sealed.is_empty());
    assert!(!ring.entries.is_empty());
    let mut actual = Vec::new();
    let mut from = 1;
    while from <= 20 {
        let page = ring.history(from, 20).unwrap();
        assert!(page.gap.is_none());
        assert!(!page.records.is_empty());
        assert_eq!(page.records[0].event.sequence(), Some(from));
        wire_page(&page);
        actual.extend(page.records.iter().map(|r| r.event.clone()));
        from = page.next.unwrap_or(21);
    }
    assert_eq!(actual, expected);
}

// keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
#[test]
#[allow(clippy::disallowed_methods)] // Fault only the unrequested in-memory entry.
fn a_full_page_returns_before_decoding_the_following_entry() {
    let root = tempfile::tempdir().unwrap();
    let seat = Ulid::new();
    let mut ring = Ring::open(root.path(), seat, 1, 0, Settings::default()).unwrap();
    for seq in 1..=2 {
        ring.append_event(&Event::Output {
            seat,
            generation: 1,
            seq,
            bytes: vec![255; OUTPUT_CHUNK_BYTES],
        })
        .unwrap();
    }
    ring.flush().unwrap();
    ring.entries[1].kind = u16::MAX;
    let page = ring.history(1, 2).unwrap();
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.next, Some(2));
    wire_page(&page);
    assert!(ring.history(2, 2).is_err());
}
