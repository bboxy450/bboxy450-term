// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

//! Full-ci-only structural witness. The allocator counts live allocations,
//! independently of parser counters. No wall-clock gate or real session data.
use bboxy450_log_converter::{ir::SessionId, parse::claude::Parser};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    io::{self, BufReader, Read, Seek, SeekFrom, Write},
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};
struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
// Test-only allocator instrumentation delegates unchanged to System.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            let now = LIVE.fetch_add(l.size(), Ordering::SeqCst) + l.size();
            PEAK.fetch_max(now, Ordering::SeqCst);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Ordering::SeqCst);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, old: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(p, old, size) };
        if !next.is_null() {
            let live = if size >= old.size() {
                LIVE.fetch_add(size - old.size(), Ordering::SeqCst) + size - old.size()
            } else {
                LIVE.fetch_sub(old.size() - size, Ordering::SeqCst) - (old.size() - size)
            };
            PEAK.fetch_max(live, Ordering::SeqCst);
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;
struct ReadCounter {
    file: std::fs::File,
    count: Arc<AtomicU64>,
}
impl Read for ReadCounter {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let n = self.file.read(buffer)?;
        self.count.fetch_add(n as u64, Ordering::SeqCst);
        Ok(n)
    }
}
impl Seek for ReadCounter {
    fn seek(&mut self, offset: SeekFrom) -> io::Result<u64> {
        self.file.seek(offset)
    }
}
#[allow(clippy::disallowed_methods)] // Offline allocation/read instrumentation.
fn measure(path: &std::path::Path, wanted: usize) -> (usize, u64, u64) {
    let row = format!(
        "{}\n",
        serde_json::json!({"type":"user","sessionId":"synthetic-budget","isMeta":true,"message":{"content":"x".repeat(8000)}})
    );
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    let mut bytes = 0;
    while bytes < wanted {
        file.write_all(row.as_bytes()).unwrap();
        bytes += row.len();
    }
    drop(file);
    let counter = Arc::new(AtomicU64::new(0));
    let reader = BufReader::new(ReadCounter {
        file: std::fs::File::open(path).unwrap(),
        count: counter.clone(),
    });
    let baseline = LIVE.load(Ordering::SeqCst);
    PEAK.store(baseline, Ordering::SeqCst);
    let mut parser = Parser::new(reader, SessionId(ulid::Ulid::from(1u128))).unwrap();
    let mut events = 0;
    while let Some(batch) = parser.next_record().unwrap() {
        events += batch.len() as u64;
    }
    while let Some(batch) = parser.finish_tail().unwrap() {
        events += batch.len() as u64;
    }
    let peak = PEAK.load(Ordering::SeqCst).saturating_sub(baseline);
    assert_eq!(
        counter.load(Ordering::SeqCst),
        bytes as u64,
        "ClaudeReadsEachByteOnce"
    );
    assert_eq!(parser.cursor().offset, bytes as u64);
    (peak, bytes as u64, events)
}
#[test]
#[ignore = "100 MB allocation witness belongs only in full ci-local, never presubmit"]
#[allow(clippy::disallowed_methods)] // Owned temporary files; no UI thread.
fn hundred_megabytes_has_constant_live_memory_and_single_read() {
    let dir = std::env::temp_dir().join(format!("lc-budget-{}", ulid::Ulid::new()));
    std::fs::create_dir(&dir).unwrap();
    let small = measure(&dir.join("small.jsonl"), 1_000_000);
    let large = measure(&dir.join("large.jsonl"), 100_000_000);
    std::fs::remove_dir_all(dir).unwrap();
    assert!(
        large.0 <= small.0 + 2 * 1024 * 1024,
        "ClaudeBoundedMemory: small {}, large {}",
        small.0,
        large.0
    );
    println!(
        "{{\"small_peak\":{},\"large_peak\":{},\"small_bytes\":{},\"large_bytes\":{},\"large_events\":{}}}",
        small.0, large.0, small.1, large.1, large.2
    );
}
