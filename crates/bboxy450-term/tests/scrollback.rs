// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    collections::BTreeMap,
    mem,
    sync::Arc,
    time::{Duration, Instant},
};

use bboxy450_term::scrollback::{
    Anchor, GapMarker, HEAP_BUDGET, Historical, History, Playback, Request,
};
use bbt_core::Term;
use bbt_mux::Ulid;
use bbt_proto::{GuiAction, Screen, SeatId, View, gui::HistoryRecord};

struct Counted;
thread_local! {
    static COUNT: Cell<bool> = const { Cell::new(false) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() && COUNT.try_with(Cell::get).unwrap_or(false) {
            let _ = BYTES.try_with(|b| b.set(b.get() + layout.size()));
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        if COUNT.try_with(Cell::get).unwrap_or(false) {
            let _ = BYTES.try_with(|b| b.set(b.get().saturating_sub(layout.size())));
        }
        unsafe { System.dealloc(p, layout) };
    }
    unsafe fn realloc(&self, p: *mut u8, old: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(p, old, size) };
        if !next.is_null() && COUNT.try_with(Cell::get).unwrap_or(false) {
            let _ = BYTES.try_with(|b| b.set(b.get().saturating_sub(old.size()) + size));
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: Counted = Counted;

fn view(seat: SeatId, generation: u64) -> View {
    View {
        seat: Ulid(seat.0.0),
        generation,
        view_generation: Ulid::new(),
    }
}
fn text(row: &[bbt_core::Cell]) -> String {
    row.iter()
        .map(|c| c.c)
        .collect::<String>()
        .trim_end()
        .into()
}
fn bounds(record: &HistoryRecord) -> (u64, u64) {
    match record {
        HistoryRecord::Bytes { sequence, .. } => (*sequence, *sequence),
        HistoryRecord::Gap { from, to, .. } => (*from, *to),
    }
}

fn page(records: &[HistoryRecord], from: u64, to: u64) -> (Vec<HistoryRecord>, Option<u64>) {
    let records: Vec<_> = records
        .iter()
        .filter(|r| bounds(r).0 >= from && bounds(r).1 <= to)
        .take(2)
        .cloned()
        .collect();
    let end = records.last().map_or(from, |r| bounds(r).1);
    (records, (end < to).then_some(end + 1))
}

fn drive(
    history: &mut History,
    records: &[HistoryRecord],
    count: usize,
) -> (BTreeMap<SeatId, Arc<Historical>>, usize) {
    let mut displays = BTreeMap::new();
    let mut fetches = 0;
    let now = Instant::now();
    for _ in 0..100_000 {
        if let Some(GuiAction::GetHistory { seat, from, to }) = history.next_action(now) {
            let (rows, next) = page(records, from, to);
            assert!(
                !rows.is_empty(),
                "requested missing fixture page {from}..{to}"
            );
            history.accept(seat, rows, next).unwrap();
            fetches += 1;
        }
        if let Some((seat, screen)) = history.step() {
            if let Some(screen) = screen.unwrap() {
                displays.insert(seat, screen);
            } else {
                displays.remove(&seat);
            }
        }
        if displays.len() == count {
            return (displays, fetches);
        }
    }
    panic!("history did not converge to {count} displays");
}

// keeps: spec c1-live-seats §4
#[test]
fn scrollback_dimension_admission_uses_the_heap_budget() {
    let token = view(SeatId::from_u128(1), 1);
    for (cols, rows) in [(768, 4), (8, 300)] {
        let term = Term::new_with_spare_rows(cols, rows, 0, 0);
        let screen = Arc::new(Screen::from_term(&term, String::new(), None, None));
        let playback = Playback::new(token, 0, screen)
            .unwrap_or_else(|reason| panic!("fitting {cols}x{rows} viewport refused: {reason}"));
        assert!(playback.retained_heap_bytes().unwrap() < HEAP_BUDGET);
    }

    let term = Term::new_with_spare_rows(2, 1, 0, 0);
    let template = Screen::from_term(&term, String::new(), None, None);
    for (cols, rows) in [(0, 1), (1, 1), (2, 0)] {
        let screen = Arc::new(Screen {
            cols,
            rows,
            cells: vec![],
            ..template.clone()
        });
        assert!(matches!(
            Playback::new(token, 0, screen),
            Err("history_dimensions_unsupported")
        ));
    }
    // The last case reaches the saturating preflight multiplication; none
    // may attempt to allocate a terminal for the deliberately huge viewport.
    for (cols, rows) in [
        (512, 256),
        (usize::MAX, usize::MAX),
        (usize::MAX / (mem::size_of::<bbt_core::Cell>() * 2), 1),
    ] {
        let screen = Arc::new(Screen {
            cols,
            rows,
            cells: vec![],
            ..template.clone()
        });
        assert!(matches!(
            Playback::new(token, 0, screen),
            Err("history_viewport_exceeds_budget")
        ));
    }
}

// keeps: spec c1-live-seats §4
#[test]
fn scrollback_pages_reach_twenty_thousand_lines_join_live_preserve_gaps_and_stay_bounded() {
    let mut bytes = b"\x1b[32m\x1b[?1h".to_vec();
    for index in 0..256 {
        bytes.extend_from_slice(format!("\x1b]4;{index};#{:06x}\x07", index * 0x010101).as_bytes());
    }
    for n in 1..=20_000 {
        if n == 1 || n == 20_000 {
            bytes.extend_from_slice(
                b"\x1b]8;id=history-fixture;https://example.invalid/history\x1b\\",
            );
        }
        bytes.extend_from_slice(format!("line {n:05}\r\n").as_bytes());
        if n == 1 || n == 20_000 {
            bytes.extend_from_slice(b"\x1b]8;;\x1b\\");
        }
    }
    let records: Vec<_> = bytes
        .chunks(4093)
        .enumerate()
        .map(|(n, bytes)| HistoryRecord::Bytes {
            sequence: n as u64 + 1,
            bytes: bytes.to_vec(),
        })
        .collect();
    let through = records.len() as u64;
    let mut reference = Term::new(80, 24, 20_000);
    reference.advance(&bytes);
    drop(reference.take_events());
    let screen = Arc::new(Screen::from_term(&reference, "fixture".into(), None, None));
    let seat = SeatId::from_u128(1);
    let token = view(seat, 1);
    let mut history = History::default();
    history
        .request(seat, token, through, screen.clone(), Request::top())
        .unwrap();
    let pending_sample = history.metrics(&BTreeMap::new());
    assert_eq!(
        pending_sample["retained_history_bytes"],
        serde_json::Value::Null
    );
    assert_eq!(
        pending_sample["panes"][0]["retained_history_bytes"],
        serde_json::Value::Null
    );
    let (top, fetches) = drive(&mut history, &records, 1);
    assert_eq!(fetches, 1, "top must not prefetch the remaining transcript");
    assert_eq!(text(&top[&seat].screen.cells[0]), "line 00001");
    assert_eq!(
        top[&seat].screen.cells,
        reference.history_window(0, 24).unwrap()
    );
    assert_eq!(
        top[&seat].screen.modes, screen.modes,
        "display never changes live keyboard modes"
    );
    assert!(
        !history.has_work(),
        "a held page must not cause an idle spin"
    );
    assert!(
        history
            .next_action(Instant::now() + Duration::from_secs(60))
            .is_none()
    );

    history
        .request(
            seat,
            token,
            through,
            screen.clone(),
            Request {
                anchor: Anchor::Live,
                lines: 1,
                op: None,
            },
        )
        .unwrap();
    let (join, _) = drive(&mut history, &records, 1);
    let live_row = reference.history_bounds().end - 24;
    assert_eq!(
        join[&seat].screen.cells,
        reference.history_window(live_row - 1, 24).unwrap()
    );
    assert_eq!(
        &join[&seat].screen.cells[1..],
        &screen.cells[..23],
        "no duplicated or missing row at the live join"
    );
    history
        .request(
            seat,
            token,
            through,
            screen.clone(),
            Request {
                anchor: Anchor::Live,
                lines: (live_row - 10_000) as i64,
                op: None,
            },
        )
        .unwrap();
    let (middle, _) = drive(&mut history, &records, 1);
    assert_eq!(
        middle[&seat].screen.cells,
        reference.history_window(10_000, 24).unwrap()
    );
    history
        .request(seat, token, through, screen.clone(), Request::top())
        .unwrap();
    let (again, fetches) = drive(&mut history, &records, 1);
    assert_eq!(fetches, 1);
    assert_eq!(text(&again[&seat].screen.cells[0]), "line 00001");
    history
        .request(seat, token, through, screen.clone(), Request::live())
        .unwrap();
    assert!(!history.contains(seat));
    assert_eq!(
        history.retained_heap_bytes(),
        Some(mem::size_of::<History>())
    );

    // Request and parser fairness have independent cursors: a long seek must
    // not steal the first page/parse turn from peers seeking just the top.
    let mut four = History::default();
    for id in 1..=4 {
        let seat = SeatId::from_u128(id);
        four.request(
            seat,
            view(seat, 1),
            through,
            screen.clone(),
            if id == 1 {
                Request {
                    anchor: Anchor::Live,
                    lines: 1,
                    op: None,
                }
            } else {
                Request::top()
            },
        )
        .unwrap();
    }
    let mut fetched = 0;
    let mut first = BTreeMap::new();
    let mut first_page = BTreeMap::new();
    for turn in 1..=1000 {
        if let Some(GuiAction::GetHistory { seat, from, to }) = four.next_action(Instant::now()) {
            let (records, next) = page(&records, from, to);
            four.accept(seat, records, next).unwrap();
            fetched += 1;
            first_page.entry(seat).or_insert(fetched);
        }
        if let Some((seat, Ok(Some(_)))) = four.step() {
            first.entry(seat).or_insert(turn);
        }
        if (2..=4).all(|id| first.contains_key(&SeatId::from_u128(id))) {
            break;
        }
    }
    for id in 2..=4 {
        let seat = SeatId::from_u128(id);
        assert!(
            first_page[&seat] <= 4,
            "peer's first page starved behind long seek"
        );
        // The palette prelude is larger than one 4 KiB parse quantum. Each
        // peer must display after its second turn, within two four-seat rounds.
        assert!(first[&seat] <= 8, "peer's parser starved behind long seek");
    }

    let gap_records = vec![
        HistoryRecord::Bytes {
            sequence: 1,
            bytes: b"line 1\r\nline 2\r\nline 3\r\n".to_vec(),
        },
        HistoryRecord::Gap {
            from: 2,
            to: 4,
            reason: "fixture missing capture".into(),
        },
        HistoryRecord::Bytes {
            sequence: 5,
            bytes: (4..=50)
                .map(|n| format!("line {n}\r\n"))
                .collect::<String>()
                .into_bytes(),
        },
    ];
    history
        .request(seat, token, 5, screen.clone(), Request::top())
        .unwrap();
    let (gap, _) = drive(&mut history, &gap_records, 1);
    assert_eq!(text(&gap[&seat].screen.cells[2]), "line 3");
    assert_eq!(text(&gap[&seat].screen.cells[3]), "");
    assert_eq!(
        gap[&seat].gaps,
        vec![GapMarker {
            row: 3,
            from: 2,
            to: 4
        }]
    );
    assert_eq!(text(&gap[&seat].screen.cells[4]), "line 4");
    let observed = history.metrics(&gap);
    assert_eq!(
        observed["panes"][0]["gap_markers"],
        serde_json::json!([{"row":3,"from":2,"to":4}])
    );
    assert_eq!(
        observed["retained_history_bytes"].as_u64(),
        history.retained_heap_bytes().map(|n| n as u64)
    );
    assert!(observed["panes"][0]["pages_fetched"].as_u64().unwrap() > 0);
    history.forget(seat);

    // There is never an overlapping timeout retry. A correlated Busy result
    // releases the old request, then its exact range retries with backoff.
    let now = Instant::now();
    history
        .request(seat, token, through, screen.clone(), Request::top())
        .unwrap();
    assert!(history.next_action(now).is_some());
    assert!(history.next_action(now + Duration::from_secs(60)).is_none());
    history.rejected(true, now, "Busy: history service busy");
    assert!(
        history
            .next_action(now + Duration::from_millis(99))
            .is_none()
    );
    assert!(
        history
            .next_action(now + Duration::from_millis(100))
            .is_some()
    );
    let (rows, next) = page(&records, 1, through);
    history.accept(seat, rows, next).unwrap();
    drive(&mut history, &records, 1);
    history.forget(seat);

    // A reply for the canceled generation drains its old fence and cannot
    // populate a newly created Playback for that same seat.
    history
        .request(seat, token, through, screen.clone(), Request::top())
        .unwrap();
    assert!(history.next_action(Instant::now()).is_some());
    history.forget(seat);
    let next_view = view(seat, 2);
    history
        .request(seat, next_view, through, screen.clone(), Request::top())
        .unwrap();
    let (old, next) = page(&records, 1, through);
    history.accept(seat, old, next).unwrap();
    assert!(!history.has_work());
    assert!(history.next_action(Instant::now()).is_some());
    history.disconnected();
    history.forget(seat);

    let large = Arc::new(Screen {
        cols: 512,
        rows: 256,
        cells: vec![],
        ..screen.as_ref().clone()
    });
    assert!(matches!(
        Playback::new(token, through, large),
        Err("history_viewport_exceeds_budget")
    ));

    BYTES.with(|bytes| bytes.set(0));
    COUNT.with(|count| count.set(true));
    let mut fifty = History::default();
    for id in 1..=50 {
        let seat = SeatId::from_u128(id);
        fifty
            .request(
                seat,
                view(seat, 1),
                through,
                Arc::new(screen.as_ref().clone()),
                Request::top(),
            )
            .unwrap();
    }
    let (displays, fetches) = drive(&mut fifty, &records, 50);
    let actual = BYTES.with(Cell::get);
    let accounted = fifty.retained_heap_bytes().unwrap();
    COUNT.with(|count| count.set(false));
    assert_eq!(fetches, 50);
    let sample = fifty.metrics(&displays);
    assert_eq!(sample["complete"], true);
    assert_eq!(sample["panes"].as_array().unwrap().len(), 50);
    assert_eq!(sample["retained_history_bytes"], accounted);
    for row in sample["panes"].as_array().unwrap() {
        assert!(row["retained_history_bytes"].as_u64().unwrap() <= HEAP_BUDGET as u64);
        assert_eq!(row["pages_fetched"], 1);
        assert!(row["pages_resident"].as_u64().unwrap() <= 1);
    }
    assert!(
        displays
            .values()
            .all(|view| text(&view.screen.cells[0]) == "line 00001")
    );
    assert!(accounted < 50 * HEAP_BUDGET, "50 Playback heap {accounted}");
    assert!(
        actual <= accounted,
        "owned allocations understated: {actual} > {accounted}"
    );
    eprintln!(
        "50 watched at top: requested_heap={actual}, conservative_heap={accounted}, budget={}",
        50 * HEAP_BUDGET
    );
    for id in 1..=50 {
        fifty.forget(SeatId::from_u128(id));
    }
    assert_eq!(fifty.retained_heap_bytes(), Some(mem::size_of::<History>()));
    let released = fifty.metrics(&BTreeMap::new());
    assert_eq!(
        released["retained_history_bytes"],
        mem::size_of::<History>()
    );
    assert_eq!(released["panes"], serde_json::json!([]));
}

fn scroll_fixture() -> (SeatId, View, Arc<Screen>, Vec<HistoryRecord>, u64) {
    let seat = SeatId::from_u128(51);
    let bytes: Vec<u8> = (1..=64)
        .flat_map(|n| format!("line {n:05}\r\n").into_bytes())
        .collect();
    let mut term = Term::new(80, 6, 128);
    term.advance(&bytes);
    drop(term.take_events());
    let row = term.history_bounds().end - 6;
    (
        seat,
        view(seat, 7),
        Arc::new(Screen::from_term(&term, String::new(), None, None)),
        vec![HistoryRecord::Bytes { sequence: 1, bytes }],
        row,
    )
}

// keeps: spec c1-live-seats §4
#[test]
fn ctl_scroll_completes_absolute_rows_cached_hits_and_u64_tail_without_signed_casts() {
    let (seat, token, screen, records, tail) = scroll_fixture();
    let mut history = History::default();
    let op = Ulid::new();
    history
        .request(
            seat,
            token,
            1,
            screen.clone(),
            Request::top().correlated(op),
        )
        .unwrap();
    let (top, _) = drive(&mut history, &records, 1);
    assert_eq!(top[&seat].top_row, 0);
    assert_eq!(text(&top[&seat].screen.cells[0]), "line 00001");
    let reply = history.take_completion().unwrap();
    assert_eq!((reply.id, reply.seat, reply.generation), (op, seat, 7));
    assert_eq!(
        reply.result.unwrap(),
        bboxy450_term::scrollback::Position {
            top_row: Some(0),
            live: false
        }
    );
    history
        .request(
            seat,
            token,
            1,
            screen.clone(),
            Request::row(17).correlated(Ulid::new()),
        )
        .unwrap();
    let (at, _) = drive(&mut history, &records, 1);
    assert_eq!(at[&seat].top_row, 17);
    assert_eq!(text(&at[&seat].screen.cells[0]), "line 00018");
    history.take_completion().unwrap();
    history
        .request(
            seat,
            token,
            1,
            screen.clone(),
            Request::row(17).correlated(Ulid::new()),
        )
        .unwrap();
    assert_eq!(
        history.take_completion().unwrap().result.unwrap().top_row,
        Some(17)
    );
    assert!(!history.has_work());
    assert!(history.next_action(Instant::now()).is_none());
    history
        .request(
            seat,
            token,
            1,
            screen,
            Request::row(u64::MAX).correlated(Ulid::new()),
        )
        .unwrap();
    for _ in 0..8 {
        history.step();
        if !history.contains(seat) {
            break;
        }
    }
    assert_eq!(
        history.take_completion().unwrap().result.unwrap(),
        bboxy450_term::scrollback::Position {
            top_row: Some(tail),
            live: true
        }
    );
    assert!(!history.contains(seat));
    let located = history.take_position().unwrap();
    assert_eq!(located.baseline, (token, 1));
    let mut readback = bboxy450_term::scrollback::Readback::default();
    readback.displayed(seat, Some((token, 2)), located.baseline, located.position);
    assert_eq!(
        readback.position(seat, None).top_row,
        None,
        "an older replay cannot index a newer live frame"
    );
}

// keeps: spec c1-live-seats §4
#[test]
fn ctl_scroll_finishes_live_noops_refusals_disconnects_and_local_supersession() {
    let (seat, token, screen, _, _) = scroll_fixture();
    let mut history = History::default();
    history
        .request(
            seat,
            token,
            1,
            screen.clone(),
            Request::live().correlated(Ulid::new()),
        )
        .unwrap();
    assert_eq!(
        history.take_completion().unwrap().result.unwrap(),
        bboxy450_term::scrollback::Position::default()
    );
    assert!(history.next_action(Instant::now()).is_none());
    history
        .request(
            seat,
            token,
            1,
            screen.clone(),
            Request::top().correlated(Ulid::new()),
        )
        .unwrap();
    history.next_action(Instant::now()).unwrap();
    assert_eq!(
        history.rejected(
            false,
            Instant::now(),
            "history_unavailable: retained data missing"
        ),
        Some(seat)
    );
    assert_eq!(
        history.take_completion().unwrap().result.unwrap_err(),
        "history_unavailable: retained data missing"
    );
    history
        .request(
            seat,
            token,
            1,
            screen.clone(),
            Request::top().correlated(Ulid::new()),
        )
        .unwrap();
    history.disconnected();
    assert_eq!(
        history.take_completion().unwrap().result.unwrap_err(),
        "history_disconnected"
    );
    history
        .request(
            seat,
            token,
            1,
            screen.clone(),
            Request::top().correlated(Ulid::new()),
        )
        .unwrap();
    history
        .request(seat, token, 1, screen, Request::lines(1))
        .unwrap();
    assert_eq!(
        history.take_completion().unwrap().result.unwrap_err(),
        "history_superseded"
    );
}

// keeps: spec c1-live-seats §4
#[test]
fn solved_scroll_readback_leaves_unindexed_live_null_and_invalidates_old_sequences() {
    use bboxy450_term::scrollback::{Position, Readback};
    let (seat, token, _, _, tail) = scroll_fixture();
    let mut readback = Readback::default();
    assert_eq!(readback.position(seat, None), Position::default());
    readback.displayed(
        seat,
        Some((token, 1)),
        (token, 1),
        Position {
            top_row: Some(tail),
            live: true,
        },
    );
    assert_eq!(readback.position(seat, None).top_row, Some(tail));
    assert!(!readback.observe(seat, Some((token, 1))));
    assert!(readback.observe(seat, Some((token, 2))));
    assert_eq!(readback.position(seat, None), Position::default());
    readback.displayed(
        seat,
        Some((token, 2)),
        (token, 2),
        Position {
            top_row: Some(tail),
            live: true,
        },
    );
    readback.retain(|_| false);
    assert_eq!(readback.position(seat, None), Position::default());
}
