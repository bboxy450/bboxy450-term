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
) -> (BTreeMap<SeatId, Arc<Historical>>, usize, Duration) {
    let mut displays = BTreeMap::new();
    let mut fetches = 0;
    let mut longest = Duration::ZERO;
    for _ in 0..100_000 {
        let start = Instant::now();
        if let Some(GuiAction::GetHistory { seat, from, to }) = history.next_action(Instant::now())
        {
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
        longest = longest.max(start.elapsed());
        if displays.len() == count {
            return (displays, fetches, longest);
        }
    }
    panic!("history did not converge to {count} displays");
}

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
    let top_start = Instant::now();
    history
        .request(seat, token, through, screen.clone(), Request::top())
        .unwrap();
    let (top, fetches, longest) = drive(&mut history, &records, 1);
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
    let top_elapsed = top_start.elapsed();

    let end_start = Instant::now();
    history
        .request(
            seat,
            token,
            through,
            screen.clone(),
            Request {
                anchor: Anchor::Live,
                lines: 1,
            },
        )
        .unwrap();
    let (join, _, join_quantum) = drive(&mut history, &records, 1);
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
    let end_elapsed = end_start.elapsed();
    let seek_start = Instant::now();
    history
        .request(
            seat,
            token,
            through,
            screen.clone(),
            Request {
                anchor: Anchor::Live,
                lines: (live_row - 10_000) as i64,
            },
        )
        .unwrap();
    let (middle, _, seek_quantum) = drive(&mut history, &records, 1);
    assert_eq!(
        middle[&seat].screen.cells,
        reference.history_window(10_000, 24).unwrap()
    );
    let seek_elapsed = seek_start.elapsed();
    history
        .request(seat, token, through, screen.clone(), Request::top())
        .unwrap();
    let (again, fetches, _) = drive(&mut history, &records, 1);
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
    eprintln!(
        "20k seek: top={top_elapsed:?}, capture-prefix-to-live={end_elapsed:?}, seek-row10000={seek_elapsed:?}, longest_parse_turn={:?}",
        longest.max(join_quantum).max(seek_quantum)
    );

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
    let (gap, _, _) = drive(&mut history, &gap_records, 1);
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
    history.forget(seat);

    // There is never an overlapping timeout retry. A correlated Busy result
    // releases the old request, then its exact range retries with backoff.
    let now = Instant::now();
    history
        .request(seat, token, through, screen.clone(), Request::top())
        .unwrap();
    assert!(history.next_action(now).is_some());
    assert!(history.next_action(now + Duration::from_secs(60)).is_none());
    history.rejected(true, now);
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
    let (displays, fetches, _) = drive(&mut fifty, &records, 50);
    let actual = BYTES.with(Cell::get);
    let accounted = fifty.retained_heap_bytes().unwrap();
    COUNT.with(|count| count.set(false));
    assert_eq!(fetches, 50);
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
}
