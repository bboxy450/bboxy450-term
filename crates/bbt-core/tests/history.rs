use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use bbt_core::Term;

struct Counted;
thread_local! {
    static COUNT: Cell<bool> = const { Cell::new(false) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

// This one-test process counts requested allocations on its own test thread,
// independently of the core's retained-capacity accounting.
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && COUNT.try_with(Cell::get).unwrap_or(false) {
            let _ = BYTES.try_with(|bytes| bytes.set(bytes.get() + layout.size()));
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if COUNT.try_with(Cell::get).unwrap_or(false) {
            let _ = BYTES.try_with(|bytes| bytes.set(bytes.get().saturating_sub(layout.size())));
        }
        unsafe { System.dealloc(pointer, layout) };
    }
    unsafe fn realloc(&self, pointer: *mut u8, old: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(pointer, old, size) };
        if !next.is_null() && COUNT.try_with(Cell::get).unwrap_or(false) {
            let _ = BYTES.try_with(|bytes| {
                bytes.set(bytes.get().saturating_sub(old.size()) + size);
            });
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: Counted = Counted;

fn text(row: &[bbt_core::Cell]) -> String {
    row.iter()
        .map(|cell| cell.c)
        .collect::<String>()
        .trim_end()
        .into()
}

fn remove_new_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            object.remove("spare_rows");
            object.remove("scrolled_rows");
            for value in object.values_mut() {
                remove_new_fields(value);
            }
        }
        serde_json::Value::Array(values) => values.iter_mut().for_each(remove_new_fields),
        _ => {}
    }
}

#[test]
fn bounded_history_windows_checkpoint_compatibility_and_heap_accounting() {
    let mut term = Term::new_with_spare_rows(80, 24, 128, 0);
    let empty = term.retained_heap_bytes().unwrap();
    assert!(empty < 128 * 1024, "no eager 2 MiB sync buffer: {empty}");
    let mut reference = Term::new(80, 24, 0);
    for line in 1..=20_000 {
        let bytes = format!("\x1b[3{}mline {line}\x1b[0m\r\n", line % 8);
        term.advance(bytes.as_bytes());
        reference.advance(bytes.as_bytes());
    }
    assert!(term.history_bounds().start > 10_000);
    assert!(term.history_window(0, 24).is_none());
    let live = term.history_bounds().end - 24;
    assert_eq!(
        term.history_window(live, 24),
        reference.history_window(live, 24)
    );
    assert!(term.retained_heap_bytes().unwrap() < 1024 * 1024);

    // Seeking outside the resident window reconstructs from the original prefix.
    let mut beginning = Term::new_with_spare_rows(80, 24, 128, 0);
    for line in 1..=60 {
        beginning.advance(format!("line {line}\r\n").as_bytes());
    }
    let rows = beginning.history_window(0, 24).unwrap();
    assert_eq!(text(&rows[0]), "line 1");
    assert_eq!(text(&rows[23]), "line 24");

    let mut old = serde_json::to_value(beginning.checkpoint()).unwrap();
    remove_new_fields(&mut old);
    let mut restored = Term::new(80, 24, 0);
    restored.restore(serde_json::from_value(old).unwrap());
    assert_eq!(restored.history_window(0, 24).unwrap(), rows);
    let before = restored.history_bounds().end;
    restored.advance(b"next\r\n");
    assert_eq!(restored.history_bounds().end, before + 1);

    beginning.advance(b"\x1b[6n");
    assert!(beginning.retained_heap_bytes().is_none());
    drop(beginning.take_events());
    assert!(beginning.retained_heap_bytes().is_some());

    let fixture = (1..=500)
        .map(|n| format!("line {n}\r\n"))
        .collect::<String>();
    BYTES.with(|bytes| bytes.set(0));
    COUNT.with(|count| count.set(true));
    let mut watched = Vec::with_capacity(50);
    for _ in 0..50 {
        let mut term = Term::new_with_spare_rows(80, 24, 128, 0);
        term.advance(fixture.as_bytes());
        term.advance(b"\x1b]8;;https://example.invalid/history\x1b\\e\xcc\x81\x1b]8;;\x1b\\");
        drop(term.take_events());
        watched.push(term);
    }
    let actual = BYTES.with(Cell::get);
    let accounted: usize = watched
        .iter()
        .map(|term| term.retained_heap_bytes().unwrap())
        .sum();
    COUNT.with(|count| count.set(false));
    assert!(
        actual <= accounted,
        "allocation accounting understated {actual} > {accounted}"
    );
    assert!(accounted < 50 * 4_000_000);
    eprintln!("50 bounded core histories: requested_heap={actual} conservative_heap={accounted}");
}
