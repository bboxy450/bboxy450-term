// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Isolated closed-source observation, immutable custody and owned C1 execution.
//! No function in this module runs on the mux/reactor/UI owner.
#![allow(
    clippy::disallowed_methods,
    reason = "dedicated C1 source/process/result owners"
)]
use crate::registry::dream_c1::{
    Bookmark, ExistingJournal, Journal, JournalConfig, LaunchBookmark, Record, private_dir,
};
use bboxy450_harness_wake_up::{
    dream_bootstrap as bootstrap,
    dream_c1::{self as transport, Worker, wire as w},
};
use bbt_proto::dream::{self as public, Code, State};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, Metadata, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicI64, AtomicU8, AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const CHUNK: usize = 64 * 1024;
// One source transfer/hash unit consumes at most64KiB. Product CPU remains
// measured independently of the test-only allocator observer, with a1ms limit.
const HASH_CHUNK: usize = CHUNK;
const LINE: usize = 256 * 1024;
const INPUT: u64 = 64 * 1024 * 1024;
const DISK: u64 = 128 * 1024 * 1024;
pub(crate) const WORK_HEAP: u64 = 22 * 1024 * 1024;
pub(crate) const CONTROL_HEAP: u64 = 2 * 1024 * 1024;
pub(crate) const HELPER_HEADROOM: u64 = 8 * 1024 * 1024;
const RESULT: u64 = 16 * 1024 * 1024;
const UNIT: Duration = Duration::from_millis(1);
const REST: Duration = Duration::from_millis(9);

// The run, rather than a Budget instance or owner thread, retains work debt.
// No controls use this lane. A held native return keeps its lease; another
// parent owner can only wait/refuse, never overlap or erase its cost.
pub(crate) fn monotonic_ns() -> Result<u64, Code> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: one owned fixed timespec; no host state or external pointer.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) } != 0
        || time.tv_sec < 0
        || !(0..1_000_000_000).contains(&time.tv_nsec)
    {
        return Err(Code::ResourceLimit);
    }
    u64::try_from(time.tv_sec)
        .ok()
        .and_then(|seconds| seconds.checked_mul(1_000_000_000))
        .and_then(|base| base.checked_add(time.tv_nsec as u64))
        .ok_or(Code::ResourceLimit)
}

struct WorkDebt {
    epoch: Instant,
    held: AtomicBool,
    child_work: AtomicBool,
    child_bootstrap: AtomicBool,
    due_ns: AtomicU64,
    poisoned: AtomicBool,
    spans: AtomicU64,
    charged_ns: AtomicU64,
    paid_ns: AtomicU64,
    max_debt_ns: AtomicU64,
    admission_violations: AtomicU64,
    native_process_calls: AtomicU64,
    native_publish_calls: AtomicU64,
    native_publish_mask: AtomicU8,
    native_cpu_ns: AtomicU64,
    native_max_cpu_ns: AtomicU64,
    native_max_wall_ns: AtomicU64,
    native_process_max_cpu_ns: AtomicU64,
    native_process_max_wall_ns: AtomicU64,
    native_publish_max_cpu_ns: AtomicU64,
    native_publish_max_wall_ns: AtomicU64,
    native_app_max_cpu_ns: AtomicU64,
    inclusive_max_cpu_ns: AtomicU64,
    native_overruns: AtomicU64,
    native_clock_failures: AtomicU64,
    native_effects: AtomicU64,
    process_errno: AtomicI64,
    publish_errno: AtomicI64,
    process_effect: AtomicU8,
    publish_effect: AtomicU8,
    publish_dev: AtomicU64,
    publish_ino: AtomicU64,
}
impl Default for WorkDebt {
    fn default() -> Self {
        Self {
            epoch: Instant::now(),
            held: AtomicBool::new(false),
            child_work: AtomicBool::new(false),
            child_bootstrap: AtomicBool::new(false),
            due_ns: AtomicU64::new(0),
            poisoned: AtomicBool::new(false),
            spans: AtomicU64::new(0),
            charged_ns: AtomicU64::new(0),
            paid_ns: AtomicU64::new(0),
            max_debt_ns: AtomicU64::new(0),
            admission_violations: AtomicU64::new(0),
            native_process_calls: AtomicU64::new(0),
            native_publish_calls: AtomicU64::new(0),
            native_publish_mask: AtomicU8::new(0),
            native_cpu_ns: AtomicU64::new(0),
            native_max_cpu_ns: AtomicU64::new(0),
            native_max_wall_ns: AtomicU64::new(0),
            native_process_max_cpu_ns: AtomicU64::new(0),
            native_process_max_wall_ns: AtomicU64::new(0),
            native_publish_max_cpu_ns: AtomicU64::new(0),
            native_publish_max_wall_ns: AtomicU64::new(0),
            native_app_max_cpu_ns: AtomicU64::new(0),
            inclusive_max_cpu_ns: AtomicU64::new(0),
            native_overruns: AtomicU64::new(0),
            native_clock_failures: AtomicU64::new(0),
            native_effects: AtomicU64::new(0),
            process_errno: AtomicI64::new(0),
            publish_errno: AtomicI64::new(0),
            process_effect: AtomicU8::new(0),
            publish_effect: AtomicU8::new(0),
            publish_dev: AtomicU64::new(0),
            publish_ino: AtomicU64::new(0),
        }
    }
}
struct DebtFrame {
    debt: Arc<WorkDebt>,
    cpu: Duration,
}
std::thread_local! {
    static WORK_FRAME: std::cell::RefCell<Option<DebtFrame>> = const { std::cell::RefCell::new(None) };
}
impl WorkDebt {
    fn now_ns(&self) -> u64 {
        nanos(self.epoch.elapsed())
    }
    fn remaining_ns(&self) -> u64 {
        self.due_ns
            .load(Ordering::Acquire)
            .saturating_sub(self.now_ns())
    }
    fn charge(&self, cpu: Result<Duration, Code>, end: Instant) {
        let cpu = cpu.unwrap_or_else(|_| {
            self.poisoned.store(true, Ordering::Release);
            UNIT
        });
        let cost = nanos(cpu.max(UNIT)).saturating_mul(9);
        let end_ns = nanos(end.saturating_duration_since(self.epoch));
        self.due_ns
            .fetch_max(end_ns.saturating_add(cost), Ordering::AcqRel);
        self.charged_ns.fetch_add(cost, Ordering::Relaxed);
        self.max_debt_ns.fetch_max(cost, Ordering::Relaxed);
        self.spans.fetch_add(1, Ordering::Relaxed);
        self.inclusive_max_cpu_ns
            .fetch_max(nanos(cpu), Ordering::Relaxed);
    }
    fn wait(&self, deadline: Option<Instant>, required: bool) -> Result<(), Code> {
        loop {
            if !required && self.poisoned.load(Ordering::Acquire) {
                return Err(Code::ResourceLimit);
            }
            if deadline.is_some_and(|d| Instant::now() >= d) {
                return Err(Code::Deadline);
            }
            let left = self.remaining_ns();
            if left == 0 {
                return Ok(());
            }
            let before = Instant::now();
            std::thread::sleep(Duration::from_nanos(left).min(REST));
            self.paid_ns
                .fetch_add(nanos(before.elapsed()).min(left), Ordering::Relaxed);
        }
    }
    fn enter(self: &Arc<Self>, deadline: Option<Instant>, required: bool) -> Result<(), Code> {
        if WORK_FRAME.with(|f| f.borrow().is_some()) {
            return Err(Code::Busy);
        }
        loop {
            self.wait(deadline, required)?;
            if self.child_work.load(Ordering::Acquire)
                || self.child_bootstrap.load(Ordering::Acquire)
            {
                if !required {
                    return Err(Code::Busy);
                }
                std::thread::sleep(REST);
                continue;
            }
            if self
                .held
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                if self.remaining_ns() != 0
                    || (self.child_work.load(Ordering::Acquire)
                        || self.child_bootstrap.load(Ordering::Acquire))
                {
                    self.held.store(false, Ordering::Release);
                    continue;
                }
                match bbt_rt::fault::thread_cpu_time() {
                    Ok(cpu) => {
                        WORK_FRAME.with(|f| {
                            *f.borrow_mut() = Some(DebtFrame {
                                debt: self.clone(),
                                cpu,
                            })
                        });
                        return Ok(());
                    }
                    Err(_) => {
                        self.poisoned.store(true, Ordering::Release);
                        if required {
                            // Retain the exclusive cleanup lease even when its
                            // measurement is unavailable; no successful work is
                            // admitted on this poisoned run afterward.
                            WORK_FRAME.with(|f| {
                                *f.borrow_mut() = Some(DebtFrame {
                                    debt: self.clone(),
                                    cpu: Duration::ZERO,
                                })
                            });
                            return Ok(());
                        }
                        self.held.store(false, Ordering::Release);
                        return Err(Code::ResourceLimit);
                    }
                }
            }
            if deadline.is_some_and(|d| Instant::now() >= d) {
                return Err(Code::Deadline);
            }
            std::thread::sleep(REST);
        }
    }
    // A partial serde owner may require cleanup while its application unit is
    // still on this stack. Pay that prefix before cleanup, keep the exclusive
    // lease, then resume a fresh application segment after cleanup debt is paid.
    fn suspend(&self) -> bool {
        WORK_FRAME.with(|f| {
            let mut frame = f.borrow_mut();
            let Some(frame) = frame
                .as_mut()
                .filter(|f| std::ptr::eq(self, Arc::as_ptr(&f.debt)))
            else {
                return false;
            };
            let used = bbt_rt::fault::thread_cpu_time()
                .map_err(|_| Code::ResourceLimit)
                .and_then(|now| now.checked_sub(frame.cpu).ok_or(Code::ResourceLimit));
            self.charge(used, Instant::now());
            true
        })
    }
    fn resume(&self) {
        WORK_FRAME.with(|f| {
            if let Some(frame) = f
                .borrow_mut()
                .as_mut()
                .filter(|f| std::ptr::eq(self, Arc::as_ptr(&f.debt)))
            {
                match bbt_rt::fault::thread_cpu_time() {
                    Ok(cpu) => frame.cpu = cpu,
                    Err(_) => self.poisoned.store(true, Ordering::Release),
                }
            }
        });
    }
    fn finish(&self, measured: Option<Result<Duration, Code>>) {
        let frame = WORK_FRAME.with(|f| {
            let mut frame = f.borrow_mut();
            if frame
                .as_ref()
                .is_some_and(|f| std::ptr::eq(self, Arc::as_ptr(&f.debt)))
            {
                frame.take()
            } else {
                None
            }
        });
        if let Some(frame) = frame {
            let used = measured.unwrap_or_else(|| {
                bbt_rt::fault::thread_cpu_time()
                    .map_err(|_| Code::ResourceLimit)
                    .and_then(|now| now.checked_sub(frame.cpu).ok_or(Code::ResourceLimit))
            });
            self.charge(used, Instant::now());
            self.held.store(false, Ordering::Release);
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct NativeSample {
    cpu: Result<Duration, Code>,
    wall: Duration,
    inclusive: Result<Duration, Code>,
}
#[cfg(test)]
type ProcessReturnHook = Box<dyn FnOnce(&std::process::Child)>;
#[cfg(test)]
std::thread_local! {
    // A fixed scoped test hook can withhold a *returned real effect* before its
    // budget verdict. It is never a production native classification escape.
    static NATIVE_RETURN_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
    static PROCESS_RETURN_HOOK: std::cell::RefCell<Option<ProcessReturnHook>> = const { std::cell::RefCell::new(None) };
}
struct NativeOutcome<T: Drain> {
    // Inspect/take the real effect before inspecting verdict. A timing refusal
    // does not turn a returned process/inode into an absent effect.
    effect: Paced<Result<T, Code>>,
    // Retain the actual OS error owner through a timing/clock verdict. Its raw
    // errno is also in bounded per-tag diagnostics; Err is not absence proof.
    raw_error: Option<Paced<std::io::Error>>,
    verdict: Result<(), Code>,
    sample: NativeSample,
    returned_at: Instant,
    // Same-host CLOCK_MONOTONIC uncertainty around that original Instant.
    // Never replace the readiness origin or derive auxiliary CPU classes.
    entry_clock: Option<transport::BootstrapEntryClockBounds>,
}
impl<T: Drain> Drop for NativeOutcome<T> {
    fn drop(&mut self) {
        // The retained io::Error is retired through the same measured cleanup
        // lane after its errno and typed verdict have been consumed.
        drop(self.raw_error.take());
    }
}

#[cfg(all(test, target_os = "macos"))]
mod isolated_heap {
    use super::*;
    pub static ENABLED: AtomicBool = AtomicBool::new(false);
    pub static LIVE_PEAK: AtomicU64 = AtomicU64::new(0);
    pub static TOUCHED_PEAK: AtomicU64 = AtomicU64::new(0);
    pub static SAMPLES: AtomicU64 = AtomicU64::new(0);
    pub static OBSERVER_CPU_NS: AtomicU64 = AtomicU64::new(0);
    // ABI copied from the platform SDK malloc/malloc.h, malloc_statistics_t.
    // System malloc sums all zones for a null zone; this includes test baseline.
    #[repr(C)]
    struct Statistics {
        blocks_in_use: libc::c_uint,
        size_in_use: usize,
        max_size_in_use: usize,
        size_allocated: usize,
    }
    unsafe extern "C" {
        fn malloc_zone_statistics(zone: *mut libc::c_void, stats: *mut Statistics);
    }
    pub fn sample() {
        if !ENABLED.load(Ordering::Relaxed) {
            return;
        }
        let cpu = bbt_rt::fault::thread_cpu_time().ok();
        let mut stats = Statistics {
            blocks_in_use: 0,
            size_in_use: 0,
            max_size_in_use: 0,
            size_allocated: 0,
        };
        // SAFETY: SDK-defined caller stack ABI; null selects all local malloc zones.
        unsafe { malloc_zone_statistics(std::ptr::null_mut(), &mut stats) };
        LIVE_PEAK.fetch_max(stats.size_in_use as u64, Ordering::Relaxed);
        TOUCHED_PEAK.fetch_max(stats.max_size_in_use as u64, Ordering::Relaxed);
        SAMPLES.fetch_add(1, Ordering::Relaxed);
        if let (Some(before), Ok(after)) = (cpu, bbt_rt::fault::thread_cpu_time()) {
            OBSERVER_CPU_NS.fetch_add(nanos(after.saturating_sub(before)), Ordering::Relaxed);
        }
    }
}
fn sample_isolated_heap() {
    #[cfg(all(test, target_os = "macos"))]
    isolated_heap::sample();
}

#[derive(Clone, Copy)]
pub(crate) enum HeapLane {
    Work = 0,
    Control = 1,
}

/// Requested allocation capacity, separate from RSS and allocator usable-size overhead.
/// Every allocation this owner performs has a preceding fallible reservation.
pub(crate) struct Heap {
    cleanup: Arc<disposal::Cleanup>,
    reserved: [AtomicU64; 2],
    live: [AtomicU64; 2],
    cumulative: [AtomicU64; 2],
    peak: [AtomicU64; 2],
    reserved_peak: [AtomicU64; 2],
    reserved_cumulative: [AtomicU64; 2],
}
impl Default for Heap {
    fn default() -> Self {
        Self {
            cleanup: Arc::new(disposal::Cleanup::default()),
            reserved: [AtomicU64::new(0), AtomicU64::new(0)],
            live: [AtomicU64::new(0), AtomicU64::new(0)],
            cumulative: [AtomicU64::new(0), AtomicU64::new(0)],
            peak: [AtomicU64::new(0), AtomicU64::new(0)],
            reserved_peak: [AtomicU64::new(0), AtomicU64::new(0)],
            reserved_cumulative: [AtomicU64::new(0), AtomicU64::new(0)],
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeapCounters {
    pub reserved: u64,
    pub live: u64,
    pub cumulative: u64,
    pub peak: u64,
    pub reserved_peak: u64,
    pub reserved_cumulative: u64,
}
impl Heap {
    pub fn counters(&self, lane: HeapLane) -> HeapCounters {
        let lane = lane as usize;
        HeapCounters {
            reserved: self.reserved[lane].load(Ordering::Acquire),
            live: self.live[lane].load(Ordering::Acquire),
            cumulative: self.cumulative[lane].load(Ordering::Acquire),
            peak: self.peak[lane].load(Ordering::Acquire),
            reserved_peak: self.reserved_peak[lane].load(Ordering::Acquire),
            reserved_cumulative: self.reserved_cumulative[lane].load(Ordering::Acquire),
        }
    }
    pub fn reserve(self: &Arc<Self>, lane: HeapLane, bytes: usize) -> Result<Reservation, Code> {
        let bound = match lane {
            HeapLane::Work => WORK_HEAP,
            HeapLane::Control => CONTROL_HEAP,
        };
        let bytes = u64::try_from(bytes).map_err(|_| Code::ResourceLimit)?;
        let old = self.reserved[lane as usize]
            .try_update(Ordering::AcqRel, Ordering::Acquire, |old| {
                old.checked_add(bytes).filter(|new| *new <= bound)
            })
            .map_err(|_| Code::ResourceLimit)?;
        self.reserved_peak[lane as usize].fetch_max(old + bytes, Ordering::AcqRel);
        self.reserved_cumulative[lane as usize].fetch_add(bytes, Ordering::AcqRel);
        Ok(Reservation {
            heap: self.clone(),
            lane,
            bytes,
            allocated: false,
        })
    }
}
pub(crate) struct Reservation {
    heap: Arc<Heap>,
    lane: HeapLane,
    bytes: u64,
    allocated: bool,
}
impl Reservation {
    fn grow(&mut self, bytes: usize) -> Result<(), Code> {
        if self.allocated {
            return Err(Code::ResourceLimit);
        }
        let mut extra = self.heap.reserve(self.lane, bytes)?;
        self.bytes = self
            .bytes
            .checked_add(extra.bytes)
            .ok_or(Code::ResourceLimit)?;
        extra.bytes = 0;
        Ok(())
    }
    fn allocated(&mut self) {
        let lane = self.lane as usize;
        let live = self.heap.live[lane].fetch_add(self.bytes, Ordering::AcqRel) + self.bytes;
        self.heap.peak[lane].fetch_max(live, Ordering::AcqRel);
        self.heap.cumulative[lane].fetch_add(self.bytes, Ordering::AcqRel);
        self.allocated = true;
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        let lane = self.lane as usize;
        if self.allocated {
            self.heap.live[lane].fetch_sub(self.bytes, Ordering::AcqRel);
        }
        self.heap.reserved[lane].fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

/// Local fallible allocation; no global allocator replacement or post-effect sampling.
/// Allocation charges remain live until the exact buffer is deallocated.
pub(crate) struct HeapBuffer {
    pointer: std::ptr::NonNull<u8>,
    len: usize,
    capacity: usize,
    reservation: Reservation,
}
// SAFETY: the buffer has one owner; its only shared reference is the atomic quota.
unsafe impl Send for HeapBuffer {}
impl HeapBuffer {
    pub fn new(heap: &Arc<Heap>, lane: HeapLane, capacity: usize) -> Result<Self, Code> {
        let mut reservation = heap.reserve(lane, capacity)?;
        let pointer = if capacity == 0 {
            std::ptr::NonNull::dangling()
        } else {
            let layout =
                std::alloc::Layout::array::<u8>(capacity).map_err(|_| Code::ResourceLimit)?;
            // SAFETY: nonzero checked layout; null is handled as a fallible resource refusal.
            let raw = unsafe { std::alloc::alloc(layout) };
            std::ptr::NonNull::new(raw).ok_or(Code::ResourceLimit)?
        };
        reservation.allocated();
        sample_isolated_heap();
        Ok(Self {
            pointer,
            len: 0,
            capacity,
            reservation,
        })
    }
    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: only initialized bytes up to len are exposed; storage stays owned.
        unsafe { std::slice::from_raw_parts(self.pointer.as_ptr(), self.len) }
    }
    pub fn extend(&mut self, bytes: &[u8]) -> Result<(), Code> {
        let end = self
            .len
            .checked_add(bytes.len())
            .filter(|n| *n <= self.capacity)
            .ok_or(Code::ResourceLimit)?;
        // SAFETY: input is an independent shared slice; owned destination has proven capacity.
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.pointer.as_ptr().add(self.len),
                bytes.len(),
            );
        }
        self.len = end;
        Ok(())
    }
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.len = 0;
    }
    pub fn consume(&mut self, bytes: usize) -> Result<(), Code> {
        if bytes > self.len {
            return Err(Code::ProtocolError);
        }
        // SAFETY: both ranges belong to this buffer and overlap is handled by copy.
        unsafe {
            std::ptr::copy(
                self.pointer.as_ptr().add(bytes),
                self.pointer.as_ptr(),
                self.len - bytes,
            );
        }
        self.len -= bytes;
        Ok(())
    }
    pub fn ensure(&mut self, needed: usize, bound: usize) -> Result<(), Code> {
        if needed > bound {
            return Err(Code::ResourceLimit);
        }
        if needed > self.capacity {
            let capacity = needed
                .checked_add(CHUNK - 1)
                .map(|n| n / CHUNK * CHUNK)
                .ok_or(Code::ResourceLimit)?
                .min(bound);
            self.replace_capacity(capacity)?;
        }
        Ok(())
    }
    /// The old allocation remains charged while the replacement is allocated/copied.
    pub fn replace_capacity(&mut self, capacity: usize) -> Result<(), Code> {
        if capacity < self.len {
            return Err(Code::ResourceLimit);
        }
        let mut next = Self::new(&self.reservation.heap, self.reservation.lane, capacity)?;
        next.extend(self.as_slice())?;
        std::mem::swap(self, &mut next);
        drop(next);
        Ok(())
    }
}
impl Drop for HeapBuffer {
    fn drop(&mut self) {
        if self.capacity != 0 {
            disposal::cleanup_unit(&self.reservation.heap.cleanup, || {
                // SAFETY: this exact pointer/layout pair was allocated once by
                // new and is unique. The reservation stays live through free.
                unsafe {
                    std::alloc::dealloc(
                        self.pointer.as_ptr(),
                        std::alloc::Layout::array::<u8>(self.capacity)
                            .expect("validated allocation layout"),
                    );
                }
            });
        }
        // The reservation field drops after this method, after physical deallocation.
    }
}
impl std::ops::Deref for HeapBuffer {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
impl std::fmt::Write for HeapBuffer {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.extend(text.as_bytes()).map_err(|_| std::fmt::Error)
    }
}
#[cfg(test)]
struct HeapWriter<'a>(&'a mut HeapBuffer);
#[cfg(test)]
impl Write for HeapWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .extend(bytes)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) const DEFERRED: u8 = 0;
pub(crate) const PREPARING: u8 = 1;
pub(crate) const RUNNING: u8 = 2;
pub(crate) const PAUSED: u8 = 3;
pub(crate) const CANCELLING: u8 = 4;
pub(crate) const SETTLING: u8 = 5;
pub(crate) const COMPLETED: u8 = 6;
pub(crate) const FAILED: u8 = 7;
pub(crate) const CANCELLED: u8 = 8;
pub(crate) const INTERRUPTED: u8 = 9;
pub(crate) const QUARANTINED: u8 = 10;

pub(crate) fn state(value: u8) -> State {
    match value {
        DEFERRED => State::Deferred,
        PREPARING => State::Preparing,
        RUNNING => State::Running,
        PAUSED => State::Paused,
        CANCELLING => State::Cancelling,
        SETTLING => State::Settling,
        COMPLETED => State::Completed,
        FAILED => State::Failed,
        CANCELLED => State::Cancelled,
        INTERRUPTED => State::Interrupted,
        _ => State::Quarantined,
    }
}

/// Controls are independent of snapshot IO, output frames and journal persistence.
pub(crate) type AuthorityGuard = dyn Fn() -> Result<(), Code> + Send + Sync;
#[derive(Clone)]
pub(crate) struct PreparedGuard {
    pub callback: Arc<AuthorityGuard>,
    /// Cumulative preparation CPU, including the cooperative checkpoints.
    pub setup_cpu_ns: u64,
    /// Maximum measured application unit; cumulative CPU is never clamped.
    pub setup_max_unit_cpu_ns: u64,
    pub retire_session: Option<Arc<crate::identity::PublicFenceSession>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FinalHandoff {
    Permit,
    Start,
}
#[cfg(test)]
type FinalAdmissionHook = Box<dyn FnOnce(FinalHandoff) + Send>;
pub(crate) struct Control {
    #[cfg(debug_assertions)]
    pub private_fixture: bool,
    #[cfg(debug_assertions)]
    pub omit_rest: AtomicBool,
    #[cfg(debug_assertions)]
    pub source_hold_done: AtomicBool,
    pub cancelled: AtomicBool,
    #[cfg(test)]
    pub(crate) final_admission_hook: std::sync::Mutex<Option<FinalAdmissionHook>>,
    #[cfg(test)]
    pub(crate) final_admission_sample: std::sync::Mutex<Option<Result<Duration, Code>>>,
    pub launch_incarnation: AtomicU64,
    pub native_pending_since_ns: AtomicU64,
    pub native_quarantine_observed: AtomicBool,
    pub(crate) native_birth_unknown: AtomicBool,
    pub(crate) bootstrap_control_cpu_ns: AtomicU64,
    pub(crate) bootstrap_control_max_cpu_ns: AtomicU64,
    pub(crate) bootstrap_control_overruns: AtomicU64,
    pub(crate) bootstrap_clock_failures: AtomicU64,
    pub(crate) bootstrap_permit_attempts: AtomicU64,
    pub(crate) start_admission_attempts: AtomicU64,
    pub(crate) bootstrap_hello_observed: AtomicBool,
    pub(crate) bootstrap_entry: OnceLock<transport::BootstrapEntryEvidence>,
    pub(crate) bootstrap_revoked: AtomicBool,
    pub(crate) bootstrap_parent_handoffs: AtomicU64,
    pub(crate) bootstrap_child_handoffs: AtomicU64,
    /// True also represents unknown foreground; no expensive unit is admitted then.
    pub foreground: Arc<AtomicBool>,
    pub activity_revision: Arc<AtomicU64>,
    /// Production revision+foreground word with a local closed-exit fence;
    /// fixtures may omit it. The fence is masked before worker-wire revisions.
    pub activity_latch: Option<Arc<AtomicU64>>,
    pub authority_guard: Option<Arc<AuthorityGuard>>,
    pub authority_setup_cpu_ns: AtomicU64,
    pub authority_setup_max_unit_cpu_ns: AtomicU64,
    pub authority_prepared_guard: OnceLock<Arc<AuthorityGuard>>,
    pub authority_monitor_setup_cpu_ns: AtomicU64,
    pub authority_monitor_max_cpu_ns: AtomicU64,
    pub authority_monitor_overruns: AtomicU64,
    pub state: AtomicU8,
    pub settled: AtomicBool,
    pub revision: AtomicU64,
    pub max_unit_cpu_ns: AtomicU64,
    pub unit_overruns: AtomicU64,
    pub heap: Arc<Heap>,
    pub(crate) cleanup: Arc<disposal::Cleanup>,
    pub unit_overrun_line: AtomicU64,
    pub child_peak_rss_bytes: AtomicU64,
    pub child_user_cpu_ns: AtomicU64,
    pub child_system_cpu_ns: AtomicU64,
    pub child_usage_observed: AtomicBool,
    pub producer_metrics_observed: AtomicBool,
    pub producer_native_profile_observed: AtomicBool,
    pub producer_native_profile_verified: AtomicBool,
    pub native_metrics_authorized: AtomicBool,
    pub(crate) native_provision: OnceLock<Arc<runner_provision::ProvisionProof>>,
    pub(crate) native_provision_credit: OnceLock<Reservation>,
    pub producer_metrics: [AtomicU64; transport::PRODUCER_METRIC_KEYS.len()],
    pub source_owner: OnceLock<std::thread::ThreadId>,
    pub owner_refusals: AtomicU64,
    pub snapshot_owner_thread_cpu_ns: AtomicU64,
    pub snapshot_owner_wall_ns: AtomicU64,
    pub snapshot_active_cpu_ns: AtomicU64,
    pub snapshot_active_wall_ns: AtomicU64,
    pub source_copy_unit_max_cpu_ns: AtomicU64,
    pub blob_copy_unit_max_cpu_ns: AtomicU64,
    pub snapshot_tiny_records: AtomicU64,
    pub snapshot_staged_records: AtomicU64,
    pub process_owner_thread_cpu_ns: AtomicU64,
    pub process_owner_wall_ns: AtomicU64,
    pub work_unit_count: AtomicU64,
    pub rest_wait_ns: AtomicU64,
    pub rest_wait_cpu_ns: AtomicU64,
    pub rest_skips: AtomicU64,
    pub rest_violations: AtomicU64,
    pub min_inter_unit_gap_ns: AtomicU64,
    pub reactor_units: AtomicU64,
    pub reactor_max_cpu_ns: AtomicU64,
    pub reactor_cpu_ns: AtomicU64,
    pub reactor_wall_ns: AtomicU64,
    pub reactor_overruns: AtomicU64,
    pub reactor_clock_failures: AtomicU64,
    pub reactor_rest_wall_ns: AtomicU64,
    pub reactor_rest_cpu_ns: AtomicU64,
    pub reactor_min_gap_ns: AtomicU64,
    pub dispatch_guard_max_cpu_ns: AtomicU64,
    pub dispatch_guard_overruns: AtomicU64,
    pub codec_prepare_max_cpu_ns: AtomicU64,
    pub worker_spawn_cpu_ns: AtomicU64,
    pub worker_start_cpu_ns: AtomicU64,
    pub native_dispatch_overruns: AtomicU64,
    pub checkpoint_path_max_cpu_ns: AtomicU64,
    pub checkpoint_open_max_cpu_ns: AtomicU64,
    pub checkpoint_config_max_cpu_ns: AtomicU64,
    pub checkpoint_save_max_cpu_ns: AtomicU64,
    pub checkpoint_overruns: AtomicU64,
}
impl Default for Control {
    fn default() -> Self {
        let heap = Arc::new(Heap::default());
        let cleanup = heap.cleanup.clone();
        Self {
            #[cfg(debug_assertions)]
            private_fixture: false,
            #[cfg(debug_assertions)]
            omit_rest: AtomicBool::new(false),
            #[cfg(debug_assertions)]
            source_hold_done: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            #[cfg(test)]
            final_admission_hook: std::sync::Mutex::new(None),
            #[cfg(test)]
            final_admission_sample: std::sync::Mutex::new(None),
            launch_incarnation: AtomicU64::new(0),
            native_pending_since_ns: AtomicU64::new(0),
            native_quarantine_observed: AtomicBool::new(false),
            native_birth_unknown: AtomicBool::new(false),
            bootstrap_control_cpu_ns: AtomicU64::new(0),
            bootstrap_control_max_cpu_ns: AtomicU64::new(0),
            bootstrap_control_overruns: AtomicU64::new(0),
            bootstrap_clock_failures: AtomicU64::new(0),
            bootstrap_permit_attempts: AtomicU64::new(0),
            start_admission_attempts: AtomicU64::new(0),
            bootstrap_hello_observed: AtomicBool::new(false),
            bootstrap_entry: OnceLock::new(),
            bootstrap_revoked: AtomicBool::new(false),
            bootstrap_parent_handoffs: AtomicU64::new(0),
            bootstrap_child_handoffs: AtomicU64::new(0),
            foreground: Arc::new(AtomicBool::new(true)),
            activity_revision: Arc::new(AtomicU64::new(0)),
            activity_latch: None,
            authority_guard: None,
            authority_setup_cpu_ns: AtomicU64::new(0),
            authority_setup_max_unit_cpu_ns: AtomicU64::new(0),
            authority_prepared_guard: OnceLock::new(),
            authority_monitor_setup_cpu_ns: AtomicU64::new(0),
            authority_monitor_max_cpu_ns: AtomicU64::new(0),
            authority_monitor_overruns: AtomicU64::new(0),
            state: AtomicU8::new(DEFERRED),
            settled: AtomicBool::new(false),
            revision: AtomicU64::new(0),
            max_unit_cpu_ns: AtomicU64::new(0),
            unit_overruns: AtomicU64::new(0),
            heap,
            cleanup,
            unit_overrun_line: AtomicU64::new(0),
            child_peak_rss_bytes: AtomicU64::new(0),
            child_user_cpu_ns: AtomicU64::new(0),
            child_system_cpu_ns: AtomicU64::new(0),
            child_usage_observed: AtomicBool::new(false),
            producer_metrics_observed: AtomicBool::new(false),
            producer_native_profile_observed: AtomicBool::new(false),
            producer_native_profile_verified: AtomicBool::new(false),
            native_metrics_authorized: AtomicBool::new(false),
            native_provision: OnceLock::new(),
            native_provision_credit: OnceLock::new(),
            producer_metrics: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            source_owner: OnceLock::new(),
            owner_refusals: AtomicU64::new(0),
            snapshot_owner_thread_cpu_ns: AtomicU64::new(0),
            snapshot_owner_wall_ns: AtomicU64::new(0),
            snapshot_active_cpu_ns: AtomicU64::new(0),
            snapshot_active_wall_ns: AtomicU64::new(0),
            source_copy_unit_max_cpu_ns: AtomicU64::new(0),
            blob_copy_unit_max_cpu_ns: AtomicU64::new(0),
            snapshot_tiny_records: AtomicU64::new(0),
            snapshot_staged_records: AtomicU64::new(0),
            process_owner_thread_cpu_ns: AtomicU64::new(0),
            process_owner_wall_ns: AtomicU64::new(0),
            work_unit_count: AtomicU64::new(0),
            rest_wait_ns: AtomicU64::new(0),
            rest_wait_cpu_ns: AtomicU64::new(0),
            rest_skips: AtomicU64::new(0),
            rest_violations: AtomicU64::new(0),
            min_inter_unit_gap_ns: AtomicU64::new(u64::MAX),
            reactor_units: AtomicU64::new(0),
            reactor_max_cpu_ns: AtomicU64::new(0),
            reactor_cpu_ns: AtomicU64::new(0),
            reactor_wall_ns: AtomicU64::new(0),
            reactor_overruns: AtomicU64::new(0),
            reactor_clock_failures: AtomicU64::new(0),
            reactor_rest_wall_ns: AtomicU64::new(0),
            reactor_rest_cpu_ns: AtomicU64::new(0),
            reactor_min_gap_ns: AtomicU64::new(u64::MAX),
            dispatch_guard_max_cpu_ns: AtomicU64::new(0),
            dispatch_guard_overruns: AtomicU64::new(0),
            codec_prepare_max_cpu_ns: AtomicU64::new(0),
            worker_spawn_cpu_ns: AtomicU64::new(0),
            worker_start_cpu_ns: AtomicU64::new(0),
            native_dispatch_overruns: AtomicU64::new(0),
            checkpoint_path_max_cpu_ns: AtomicU64::new(0),
            checkpoint_open_max_cpu_ns: AtomicU64::new(0),
            checkpoint_config_max_cpu_ns: AtomicU64::new(0),
            checkpoint_save_max_cpu_ns: AtomicU64::new(0),
            checkpoint_overruns: AtomicU64::new(0),
        }
    }
}
impl Control {
    pub(crate) fn install_native_provision(
        &self,
        proof: Arc<runner_provision::ProvisionProof>,
    ) -> Result<(), Code> {
        proof.still_current()?;
        // Shared retained proof is conservatively charged to each live job;
        // its separate cache credit remains until the last physical owner frees it.
        let mut credit = self
            .heap
            .reserve(HeapLane::Work, proof.retained_declared_bytes())?;
        credit.allocated();
        self.native_provision_credit
            .set(credit)
            .map_err(|_| Code::StaleContext)?;
        self.native_provision
            .set(proof)
            .map_err(|_| Code::StaleContext)?;
        Ok(())
    }
    pub(crate) fn authorize_native_metrics(&self) {
        self.native_metrics_authorized
            .store(true, Ordering::Release);
    }
    fn begin_child_bootstrap(&self) -> Result<(), Code> {
        let debt = &self.cleanup.debt;
        if debt.poisoned.load(Ordering::Acquire) {
            return Err(Code::ResourceLimit);
        }
        if debt
            .held
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(Code::Busy);
        }
        let ready = debt.remaining_ns() == 0
            && !debt.child_work.load(Ordering::Acquire)
            && !debt.child_bootstrap.load(Ordering::Acquire);
        if ready {
            debt.child_bootstrap.store(true, Ordering::Release);
            self.bootstrap_child_handoffs
                .fetch_add(1, Ordering::Relaxed);
        }
        debt.held.store(false, Ordering::Release);
        if ready { Ok(()) } else { Err(Code::Busy) }
    }
    fn end_child_bootstrap(&self, qualified_hello: bool) -> Result<(), Code> {
        self.cleanup
            .debt
            .child_bootstrap
            .store(false, Ordering::Release);
        if qualified_hello {
            self.bootstrap_hello_observed.store(true, Ordering::Release);
            self.bootstrap_parent_handoffs
                .fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            self.cleanup.debt.poisoned.store(true, Ordering::Release);
            Err(Code::ResourceLimit)
        }
    }
    /// Reactor admission is an atomic observation/claim, never a debt wait.
    /// The metadata owner must already be retired before this Start boundary.
    fn begin_child_work(&self) -> Result<(), Code> {
        let debt = &self.cleanup.debt;
        if debt.poisoned.load(Ordering::Acquire) {
            return Err(Code::ResourceLimit);
        }
        if debt
            .held
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err(Code::Busy);
        }
        let ready = debt.remaining_ns() == 0
            && !debt.child_work.load(Ordering::Acquire)
            && !debt.child_bootstrap.load(Ordering::Acquire);
        if ready {
            debt.child_work.store(true, Ordering::Release);
        }
        debt.held.store(false, Ordering::Release);
        if ready { Ok(()) } else { Err(Code::Busy) }
    }
    /// Only the owner that observed actual reap calls this. Missing child debt
    /// evidence prevents subsequent result work, while required settlement can
    /// still close retained ownership and publish honest diagnostics.
    fn end_child_work(&self, child_debt_paid: bool) -> Result<(), Code> {
        self.cleanup.debt.child_work.store(false, Ordering::Release);
        if child_debt_paid {
            Ok(())
        } else {
            self.cleanup.debt.poisoned.store(true, Ordering::Release);
            Err(Code::ResourceLimit)
        }
    }
    pub fn installed_authority_guard(&self) -> Option<Arc<AuthorityGuard>> {
        self.authority_prepared_guard
            .get()
            .cloned()
            .or_else(|| self.authority_guard.clone())
    }
    /// Installed only by the admitted authority owner after its durable/permit
    /// fence. Deferred controls retain no prepared database connection.
    pub fn install_authority_guard(&self, guard: PreparedGuard) -> Result<(), Code> {
        if guard.setup_max_unit_cpu_ns > 1_000_000
            || guard.setup_cpu_ns < guard.setup_max_unit_cpu_ns
            || guard.setup_cpu_ns > public::MAX_SAFE
        {
            return Err(Code::ResourceLimit);
        }
        let mut installed = false;
        self.authority_prepared_guard.get_or_init(|| {
            installed = true;
            self.authority_setup_cpu_ns
                .store(guard.setup_cpu_ns, Ordering::Release);
            self.authority_setup_max_unit_cpu_ns
                .store(guard.setup_max_unit_cpu_ns, Ordering::Release);
            guard.callback
        });
        if installed { Ok(()) } else { Err(Code::Busy) }
    }
    pub fn with_activity(foreground: Arc<AtomicBool>, activity_revision: Arc<AtomicU64>) -> Self {
        Self {
            foreground,
            activity_revision,
            ..Self::default()
        }
    }
    pub fn is_foreground(&self) -> bool {
        self.activity_latch.as_ref().map_or_else(
            || self.foreground.load(Ordering::Acquire),
            |latch| latch.load(Ordering::Acquire) & 1 != 0,
        )
    }
    pub fn activity_version(&self) -> u64 {
        self.activity_latch.as_ref().map_or_else(
            || self.activity_revision.load(Ordering::Acquire),
            |latch| super::harness_status::activity_revision(latch.load(Ordering::Acquire)),
        )
    }
    fn phase(&self, phase: u8) {
        let prior = self
            .state
            .try_update(Ordering::AcqRel, Ordering::Acquire, |old| {
                if old >= COMPLETED || old >= CANCELLING && phase < CANCELLING {
                    None
                } else {
                    Some(phase)
                }
            });
        if prior.is_ok_and(|old| old != phase) {
            self.revision.fetch_add(1, Ordering::Release);
        }
    }
}

#[cfg(test)]
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn line_end(bytes: &[u8]) -> Option<usize> {
    // SAFETY: memchr reads exactly the supplied live slice and returns within it.
    let found = unsafe { libc::memchr(bytes.as_ptr().cast(), b'\n' as libc::c_int, bytes.len()) };
    (!found.is_null()).then(|| found as usize - bytes.as_ptr() as usize)
}
fn stamp(m: &Metadata) -> (u64, u64, u64, (i64, i64), (i64, i64)) {
    (
        m.dev(),
        m.ino(),
        m.len(),
        (m.mtime(), m.mtime_nsec()),
        (m.ctime(), m.ctime_nsec()),
    )
}
fn owned(m: &Metadata) -> bool {
    // SAFETY: geteuid has no pointer arguments or side effects.
    m.is_file() && m.uid() == unsafe { libc::geteuid() }
}
fn open_source(path: &Path) -> Result<File, Code> {
    let m = std::fs::symlink_metadata(path).map_err(|_| Code::SourceUnavailable)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| Code::SourceUnavailable)?;
    let actual = file.metadata().map_err(|_| Code::SourceUnavailable)?;
    if !owned(&actual) || !m.is_file() || stamp(&m) != stamp(&actual) {
        return Err(Code::SourceChanged);
    }
    Ok(file)
}

/// The same before-effect guard is used by production snapshot IO and the private
/// wrong-owner witness. A reactor attempt cannot open even an unavailable path.
pub(crate) fn guarded_source_open(control: &Control, path: &Path) -> Result<File, Code> {
    if control.source_owner.get() != Some(&std::thread::current().id()) {
        control.owner_refusals.fetch_add(1, Ordering::Relaxed);
        return Err(Code::ResourceLimit);
    }
    open_source(path)
}

struct OwnerClock<'a> {
    cpu: Duration,
    wall: Instant,
    cpu_out: &'a AtomicU64,
    wall_out: &'a AtomicU64,
}
impl<'a> OwnerClock<'a> {
    fn new(cpu_out: &'a AtomicU64, wall_out: &'a AtomicU64) -> Result<Self, Code> {
        Ok(Self {
            cpu: bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?,
            wall: Instant::now(),
            cpu_out,
            wall_out,
        })
    }
}
impl Drop for OwnerClock<'_> {
    fn drop(&mut self) {
        if let Ok(now) = bbt_rt::fault::thread_cpu_time() {
            self.cpu_out
                .fetch_add(nanos(now.saturating_sub(self.cpu)), Ordering::Relaxed);
        }
        self.wall_out
            .fetch_add(nanos(self.wall.elapsed()), Ordering::Relaxed);
    }
}
fn nanos(value: Duration) -> u64 {
    value.as_nanos().min(u64::MAX as u128) as u64
}

#[cfg(debug_assertions)]
fn private_fixture_entry(path: &Path, fifo: bool) -> Result<Option<Metadata>, Code> {
    use std::os::unix::fs::FileTypeExt;
    let m = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Code::StorageUnavailable),
    };
    // SAFETY: geteuid has no pointer arguments or side effects.
    if m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o777 != 0o600
        || m.nlink() != 1
        || if fifo {
            !m.file_type().is_fifo()
        } else {
            !m.is_file() || m.len() != 0
        }
    {
        return Err(Code::StorageUnavailable);
    }
    Ok(Some(m))
}

#[cfg(debug_assertions)]
fn prepare_private_job_faults(
    job: &Path,
    record: &Record,
    budget: &mut Budget<'_>,
) -> Result<(), Code> {
    use std::os::unix::ffi::OsStrExt;
    if !budget.control.private_fixture {
        return Ok(());
    }
    let _credit = budget
        .control
        .heap
        .reserve(HeapLane::Work, 2 * libc::PATH_MAX as usize + 256)?;
    let parent = job.parent().ok_or(Code::StorageUnavailable)?;
    let join = |base: &Path, name: &str| -> Result<PathBuf, Code> {
        let bytes = base
            .as_os_str()
            .as_bytes()
            .len()
            .checked_add(name.len() + 1)
            .ok_or(Code::ResourceLimit)?;
        if bytes >= libc::PATH_MAX as usize {
            return Err(Code::ResourceLimit);
        }
        let mut path = PathBuf::new();
        path.try_reserve_exact(bytes)
            .map_err(|_| Code::ResourceLimit)?;
        path.push(base);
        path.push(name);
        Ok(path)
    };
    let mut selected = false;
    for mode in ["hold-source", "omit-rest", "hot-owner"] {
        let name = Paced::new(
            budget.unit(|| {
                use std::fmt::Write as _;
                let mut name = String::new();
                name.try_reserve_exact(128)
                    .map_err(|_| Code::ResourceLimit)?;
                write!(
                    &mut name,
                    ".fixture-{mode}-{}-{}",
                    record.context.seat.0, record.context.generation.0
                )
                .map_err(|_| Code::ResourceLimit)?;
                Ok(name)
            })?,
            budget.control,
        );
        let configured = Paced::new(budget.unit(|| join(parent, name.as_str()))?, budget.control);
        if budget
            .unit(|| private_fixture_entry(&configured, false))?
            .is_none()
        {
            continue;
        }
        if selected {
            return Err(Code::BadRequest);
        }
        selected = true;
        if mode == "hold-source" {
            budget.unit(|| {
                // Fixed stack path: source-free fixture setup is ordinary
                // parent work before birth, never an interpreter prefix.
                let base = job.as_os_str().as_bytes();
                let suffix = b"/test-source-read.fifo";
                let mut path = [0u8; libc::PATH_MAX as usize];
                if base.len() + suffix.len() + 1 > path.len() {
                    return Err(Code::ResourceLimit);
                }
                path[..base.len()].copy_from_slice(base);
                path[base.len()..base.len() + suffix.len()].copy_from_slice(suffix);
                // SAFETY: bounded owned path has one initialized trailing NUL.
                if unsafe { libc::mkfifo(path.as_ptr().cast(), 0o600) } != 0 {
                    let _actual = std::io::Error::last_os_error();
                    return Err(Code::StorageUnavailable);
                }
                Ok(())
            })?;
        } else {
            let marker = if mode == "hot-owner" {
                "test-hot-owner"
            } else {
                "test-omit-rest"
            };
            let path = Paced::new(budget.unit(|| join(job, marker))?, budget.control);
            let file = budget.unit(|| create(&path))?;
            drop(Paced::new(file, budget.control));
        }
    }
    Ok(())
}

/// Debug-only, host-authenticated private-account fixture. The real source fd is
/// already open/verified when this syscall blocks its isolated snapshot owner.
#[cfg(debug_assertions)]
fn hold_private_source(job: &Path, control: &Control) -> Result<(), Code> {
    use std::os::fd::AsRawFd;
    if control.source_hold_done.swap(true, Ordering::AcqRel) {
        return Ok(());
    }
    let path = job.join("test-source-read.fifo");
    let Some(expected) = private_fixture_entry(&path, true)? else {
        return Ok(());
    };
    let mut input = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(&path)
        .map_err(|_| Code::StorageUnavailable)?;
    let writer = OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(&path)
        .map_err(|_| Code::StorageUnavailable)?;
    for fd in [&input, &writer] {
        let current = fd.metadata().map_err(|_| Code::StorageUnavailable)?;
        if (current.dev(), current.ino(), current.mode(), current.uid())
            != (
                expected.dev(),
                expected.ino(),
                expected.mode(),
                expected.uid(),
            )
        {
            return Err(Code::StorageUnavailable);
        }
    }
    // SAFETY: both fcntl calls operate only on this owned, verified FIFO fd.
    let flags = unsafe { libc::fcntl(input.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(input.as_raw_fd(), libc::F_SETFL, flags & !libc::O_NONBLOCK) } != 0
    {
        return Err(Code::StorageUnavailable);
    }
    publish(
        job,
        "source-read-held.json",
        "private-source",
        b"{\"owner\":\"snapshot\",\"source_fd_validated\":true,\"fifo_read\":true}",
    )?;
    let mut one = [0u8; 1];
    loop {
        match input.read(&mut one) {
            Ok(1) => break,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            _ => return Err(Code::SourceUnavailable),
        }
    }
    drop(writer);
    Ok(())
}

/// Only constructed after the process/socket/session discovery has authenticated ownership.
pub(crate) struct Reader {
    file: File,
    path: PathBuf,
    identity: (u64, u64),
    session: public::Ulid,
    generation: u64,
    offset: u64,
    cursor: u64,
    pending: Vec<u8>,
    seq: u64,
    repo: Option<String>,
    turns: BTreeSet<String>,
    closed: Option<(u64, String)>,
    delivered: bool,
    last_line_len: u32,
    last_line_sha256: String,
    head_len: u64,
}
impl Reader {
    pub fn open(root: &Path, session: &str, generation: u64) -> Result<Self, Code> {
        bbt_rt::assert_not_ui_thread();
        let session =
            public::Ulid::try_from(session.to_owned()).map_err(|_| Code::SourceMismatch)?;
        if generation == 0 {
            return Err(Code::SourceMismatch);
        }
        let path = root.join("sessions").join(format!("{}.jsonl", session.0));
        let file = open_source(&path)?;
        let m = file.metadata().map_err(|_| Code::SourceUnavailable)?;
        if m.len() > INPUT {
            return Err(Code::ResourceLimit);
        }
        Ok(Self {
            file,
            path,
            identity: (m.dev(), m.ino()),
            session,
            generation,
            offset: 0,
            cursor: 0,
            pending: Vec::new(),
            seq: 0,
            repo: None,
            turns: BTreeSet::new(),
            closed: None,
            delivered: false,
            last_line_len: 0,
            last_line_sha256: String::new(),
            head_len: m.len(),
        })
    }

    /// Restore only this exact retained source, with a bounded raw prefix anchor.
    /// Appended open-log bytes are allowed; a closed bookmark must remain immutable.
    pub fn resume(bookmark: &Bookmark) -> Result<Self, Code> {
        bbt_rt::assert_not_ui_thread();
        if bookmark.generation == 0
            || bookmark.last_line_len as usize > LINE
            || bookmark.turns.len() > 128
            || bookmark.cursor < bookmark.last_line_len as u64
            || bookmark.observed_seq > 65_536
        {
            return Err(Code::SourceMismatch);
        }
        let mut file = open_source(&bookmark.path)?;
        let m = file.metadata().map_err(|_| Code::SourceUnavailable)?;
        if (m.dev(), m.ino()) != (bookmark.device, bookmark.inode)
            || m.len() < bookmark.cursor
            || m.len() > INPUT
            || bookmark.closed_seq > 0 && !immutable_source(bookmark, &m)
        {
            return Err(Code::SourceChanged);
        }
        if bookmark.observed_seq == 0 {
            if bookmark.cursor != 0 || bookmark.last_line_len != 0 {
                return Err(Code::SourceMismatch);
            }
        } else {
            if bookmark.last_line_len == 0 {
                return Err(Code::SourceMismatch);
            }
            file.seek(SeekFrom::Start(
                bookmark.cursor - bookmark.last_line_len as u64,
            ))
            .map_err(|_| Code::SourceChanged)?;
            let control = Control::default();
            control.foreground.store(false, Ordering::Release);
            let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
            let mut anchor = Vec::new();
            anchor
                .try_reserve_exact(bookmark.last_line_len as usize)
                .map_err(|_| Code::ResourceLimit)?;
            anchor.resize(bookmark.last_line_len as usize, 0);
            for chunk in anchor.chunks_mut(CHUNK) {
                budget.unit(|| file.read_exact(chunk).map_err(|_| Code::SourceChanged))?;
            }
            if !anchor.ends_with(b"\n")
                || budget.hash(&anchor[..anchor.len() - 1])? != bookmark.last_line_sha256
            {
                return Err(Code::SourceChanged);
            }
            std::thread::sleep(REST);
        }
        file.seek(SeekFrom::Start(bookmark.cursor))
            .map_err(|_| Code::SourceChanged)?;
        let turns: BTreeSet<_> = bookmark.turns.iter().cloned().collect();
        if turns.len() != bookmark.turns.len() || bookmark.closed_seq > 0 && !turns.is_empty() {
            return Err(Code::SourceMismatch);
        }
        Ok(Self {
            file,
            path: bookmark.path.clone(),
            identity: (m.dev(), m.ino()),
            session: bookmark.session.clone(),
            generation: bookmark.generation,
            offset: bookmark.cursor,
            cursor: bookmark.cursor,
            pending: Vec::new(),
            seq: bookmark.observed_seq,
            repo: (!bookmark.repo.is_empty()).then(|| bookmark.repo.clone()),
            turns,
            closed: (bookmark.closed_seq > 0)
                .then(|| (bookmark.closed_seq, bookmark.closed_line_sha256.clone())),
            delivered: false,
            last_line_len: bookmark.last_line_len,
            last_line_sha256: bookmark.last_line_sha256.clone(),
            head_len: m.len(),
        })
    }

    /// Exact source/cursor state, including open sources; no source enumeration.
    pub fn bookmark(&self) -> Result<Bookmark, Code> {
        let m = self.file.metadata().map_err(|_| Code::SourceUnavailable)?;
        let path = std::fs::symlink_metadata(&self.path).map_err(|_| Code::SourceChanged)?;
        if !owned(&m)
            || stamp(&m) != stamp(&path)
            || (m.dev(), m.ino()) != self.identity
            || m.len() < self.cursor
            || self.closed.is_some() && (self.cursor != m.len() || !self.pending.is_empty())
        {
            return Err(Code::SourceChanged);
        }
        Ok(Bookmark {
            session: self.session.clone(),
            generation: self.generation,
            path: self.path.clone(),
            repo: self.repo.clone().unwrap_or_default(),
            pid: 0,
            process_start: String::new(),
            socket: PathBuf::new(),
            root: String::new(),
            owner_context: None,
            owner_fence: None,
            root_revision: 0,
            workspace_revision: 0,
            device: m.dev(),
            inode: m.ino(),
            len: m.len(),
            mtime: (m.mtime(), m.mtime_nsec()),
            ctime: (m.ctime(), m.ctime_nsec()),
            closed_seq: self.closed.as_ref().map_or(0, |v| v.0),
            closed_line_sha256: self
                .closed
                .as_ref()
                .map_or_else(String::new, |v| v.1.clone()),
            cursor: self.cursor,
            observed_seq: self.seq,
            last_line_len: self.last_line_len,
            last_line_sha256: self.last_line_sha256.clone(),
            turns: self.turns.iter().cloned().collect(),
        })
    }

    pub fn at_head(&self) -> bool {
        self.offset == self.head_len && !self.pending.contains(&b'\n')
    }

    /// Each read/hash/parse subunit is measured; large raw rows yield in preflight.
    pub fn poll(&mut self) -> Result<(Option<Bookmark>, Option<bool>), Code> {
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
        let result = self.poll_one(&mut budget);
        std::thread::sleep(REST);
        if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
            Err(Code::ResourceLimit)
        } else {
            result
        }
    }
    fn poll_one(
        &mut self,
        budget: &mut Budget<'_>,
    ) -> Result<(Option<Bookmark>, Option<bool>), Code> {
        let m = budget
            .unit(|| std::fs::symlink_metadata(&self.path).map_err(|_| Code::SourceUnavailable))?;
        self.head_len = m.len();
        if !owned(&m) || (m.dev(), m.ino()) != self.identity || m.len() < self.offset {
            return Err(Code::SourceChanged);
        }
        if m.len() > INPUT {
            return Err(Code::ResourceLimit);
        }
        if let Some(end) = line_end(&self.pending) {
            if end + 1 > LINE || self.closed.is_some() {
                return Err(Code::MalformedIr);
            }
            let mut raw = Vec::new();
            raw.try_reserve_exact(end + 1)
                .map_err(|_| Code::ResourceLimit)?;
            budget.unit(|| {
                raw.extend_from_slice(&self.pending[..=end]);
                self.pending.copy_within(end + 1.., 0);
                self.pending.truncate(self.pending.len() - end - 1);
                Ok(())
            })?;
            self.cursor += raw.len() as u64;
            self.last_line_len = raw.len() as u32;
            self.last_line_sha256 = budget.hash(&raw[..raw.len() - 1])?;
            let payload = budget.json(&raw).map_err(|code| {
                if code == Code::ProtocolError {
                    Code::MalformedIr
                } else {
                    code
                }
            })?;
            let event: Paced<Value> = budget.parse_credited(&raw, payload).map_err(|code| {
                if code == Code::ProtocolError {
                    Code::MalformedIr
                } else {
                    code
                }
            })?;
            budget.unit(|| {
                if event["session"].as_str() != Some(&self.session.0)
                    || event["v"]["major"] != 1
                    || event["v"]["minor"].as_u64().is_none_or(|v| v > 3)
                {
                    return Err(Code::UnsupportedIr);
                }
                if event["seq"].as_u64() != Some(self.seq + 1)
                    || event["at"].as_u64() != Some(self.seq + 1)
                    || event["epoch"].as_u64().is_none_or(|v| v == 0)
                {
                    return Err(Code::MalformedIr);
                }
                self.seq += 1;
                if self.seq > 65_536 {
                    return Err(Code::ResourceLimit);
                }
                let e = &event["event"];
                match e["type"].as_str() {
                    Some("session_started") if self.seq == 1 => {
                        let meta = &e["meta"];
                        if meta["session"].as_str() != Some(&self.session.0)
                            || !meta["source"].is_null()
                            || !meta["fork"].is_null()
                            || meta["protocol"]["major"] != 1
                            || meta["protocol"]["minor"].as_u64().is_none_or(|v| v > 3)
                        {
                            return Err(Code::UnsupportedIr);
                        }
                        let repo = meta["vcs"]["repo"]
                            .as_str()
                            .ok_or(Code::SourceUnavailable)?;
                        public::Repo::try_from(repo.to_owned())
                            .map_err(|_| Code::SourceMismatch)?;
                        self.repo = Some(repo.to_owned());
                    }
                    _ if self.seq == 1 => return Err(Code::MalformedIr),
                    Some("turn_queued" | "turn_started") => {
                        let turn = e["turn"].as_str().ok_or(Code::MalformedIr)?;
                        public::Ulid::try_from(turn.to_owned()).map_err(|_| Code::MalformedIr)?;
                        self.turns.insert(turn.into());
                        if self.turns.len() > 128 {
                            return Err(Code::ResourceLimit);
                        }
                    }
                    Some("turn_ended") => {
                        self.turns
                            .remove(e["turn"].as_str().ok_or(Code::MalformedIr)?);
                    }
                    Some("session_closed") => {
                        if !self.turns.is_empty() {
                            return Err(Code::MalformedIr);
                        }
                        self.closed = Some((self.seq, self.last_line_sha256.clone()));
                    }
                    _ => {}
                }
                Ok(())
            })?;
        } else if self.offset < m.len() {
            if self.pending.len() >= LINE {
                return Err(Code::ResourceLimit);
            }
            let mut bytes = [0; CHUNK];
            let n = budget.unit(|| {
                self.file
                    .read(&mut bytes)
                    .map_err(|_| Code::SourceUnavailable)
            })?;
            if n == 0 {
                return Err(Code::SourceChanged);
            }
            if self.pending.len() + n > LINE + CHUNK {
                return Err(Code::ResourceLimit);
            }
            self.pending
                .try_reserve_exact(n)
                .map_err(|_| Code::ResourceLimit)?;
            self.pending.extend_from_slice(&bytes[..n]);
            self.offset += n as u64;
            return Ok((None, None));
        }
        let caught = self.pending.is_empty() && self.offset == m.len();
        if caught {
            self.pending.shrink_to_fit();
        }
        let foreground = (caught && self.seq > 0).then_some(!self.turns.is_empty());
        if caught
            && !self.delivered
            && let Some((closed_seq, hash)) = &self.closed
        {
            let actual = self.file.metadata().map_err(|_| Code::SourceUnavailable)?;
            let path = std::fs::symlink_metadata(&self.path).map_err(|_| Code::SourceChanged)?;
            if stamp(&actual) != stamp(&m) || stamp(&path) != stamp(&m) {
                return Err(Code::SourceChanged);
            }
            self.delivered = true;
            return Ok((
                Some(Bookmark {
                    session: self.session.clone(),
                    generation: self.generation,
                    path: self.path.clone(),
                    repo: self.repo.clone().ok_or(Code::SourceUnavailable)?,
                    device: m.dev(),
                    inode: m.ino(),
                    len: m.len(),
                    mtime: (m.mtime(), m.mtime_nsec()),
                    ctime: (m.ctime(), m.ctime_nsec()),
                    closed_seq: *closed_seq,
                    closed_line_sha256: hash.clone(),
                    cursor: self.cursor,
                    pid: 0,
                    process_start: String::new(),
                    socket: PathBuf::new(),
                    root: String::new(),
                    owner_context: None,
                    owner_fence: None,
                    root_revision: 0,
                    workspace_revision: 0,
                    observed_seq: self.seq,
                    last_line_len: self.last_line_len,
                    last_line_sha256: self.last_line_sha256.clone(),
                    turns: self.turns.iter().cloned().collect(),
                }),
                foreground,
            ));
        }
        Ok((None, foreground))
    }
}

pub(crate) fn utility() -> Result<(), Code> {
    #[cfg(target_os = "macos")]
    {
        let mut class = libc::qos_class_t::QOS_CLASS_UNSPECIFIED;
        let mut relative = 0;
        // SAFETY: changes and queries only this dedicated worker thread's QoS.
        unsafe {
            if libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_UTILITY, 0) != 0
                || libc::pthread_get_qos_class_np(libc::pthread_self(), &mut class, &mut relative)
                    != 0
                || class as u32 != libc::qos_class_t::QOS_CLASS_UTILITY as u32
            {
                return Err(Code::RunnerUnavailable);
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        // SAFETY: Linux nice is per-thread; never alter the UI/process-wide priority.
        let tid = unsafe { libc::syscall(libc::SYS_gettid) } as u32;
        if unsafe { libc::setpriority(libc::PRIO_PROCESS, tid, 10) } != 0
            || unsafe { libc::getpriority(libc::PRIO_PROCESS, tid) } < 10
        {
            return Err(Code::RunnerUnavailable);
        }
    }
    Ok(())
}

// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-harness src/dream/c1.rs::Paced/disposal, signed
// 47182320. https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/c1.rs
// Producer policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
// Local ownership adapters delegate lexical/scalar decoding to serde and retain
// the producer-owned wire DTOs/schema/corpus as the sole protocol authority.
pub(crate) mod disposal {
    use super::*;
    use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
    use std::{cell::Cell, fmt, marker::PhantomData};

    std::thread_local! {
        // Thread CPU can only be adjusted by cleanup performed on this thread.
        // A concurrent process/snapshot/monitor owner's cleanup is never subtracted.
        static THREAD_CLEANUP_CPU: Cell<u64> = const { Cell::new(0) };
        static LAST_CLEANUP_END: Cell<Option<Instant>> = const { Cell::new(None) };
    }
    pub(super) fn thread_cleanup_cpu() -> u64 {
        THREAD_CLEANUP_CPU.with(Cell::get)
    }
    pub(super) fn last_cleanup_end() -> Option<Instant> {
        LAST_CLEANUP_END.with(Cell::get)
    }
    #[track_caller]
    pub(super) fn record_cleanup(control: &Cleanup, elapsed: Duration) {
        record_cleanup_at(
            control,
            elapsed,
            Instant::now(),
            "native_buffer",
            std::panic::Location::caller(),
        );
    }
    fn record_cleanup_at(
        control: &Cleanup,
        elapsed: Duration,
        body_end: Instant,
        kind: &'static str,
        origin: &'static std::panic::Location<'static>,
    ) {
        #[cfg(not(debug_assertions))]
        let _ = (kind, origin);
        let elapsed = nanos(elapsed);
        THREAD_CLEANUP_CPU.with(|cpu| cpu.set(cpu.get().saturating_add(elapsed)));
        LAST_CLEANUP_END.with(|end| end.set(Some(body_end)));
        control.cpu_ns.fetch_add(elapsed, Ordering::Relaxed);
        control
            .max_unit_cpu_ns
            .fetch_max(elapsed, Ordering::Relaxed);
        control.units.fetch_add(1, Ordering::Relaxed);
        if elapsed > nanos(UNIT) {
            control.overruns.fetch_add(1, Ordering::Relaxed);
            #[cfg(debug_assertions)]
            eprintln!(
                "C1 cleanup overrun cpu_ns={elapsed} type={} file={} line={}",
                if kind.len() <= 256 {
                    kind
                } else {
                    "bounded_owned_type"
                },
                origin.file(),
                origin.line()
            );
        }
    }
    pub(super) struct CleanupRegion<'a> {
        control: &'a Cleanup,
        nested: bool,
        before: Result<Duration, Code>,
        kind: &'static str,
        origin: &'static std::panic::Location<'static>,
    }
    impl<'a> CleanupRegion<'a> {
        #[track_caller]
        pub(super) fn begin(control: &'a Cleanup) -> Self {
            Self::begin_owned(
                control,
                "public_proof_unwind",
                std::panic::Location::caller(),
            )
        }
        fn begin_owned(
            control: &'a Cleanup,
            kind: &'static str,
            origin: &'static std::panic::Location<'static>,
        ) -> Self {
            let nested = control.debt.suspend();
            if !nested {
                while control.debt.enter(None, true).is_err() {
                    control.overruns.fetch_add(1, Ordering::Relaxed);
                    std::thread::sleep(REST);
                }
            }
            let before = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit);
            let _ = control.debt.wait(None, true);
            Self {
                control,
                nested,
                before,
                kind,
                origin,
            }
        }
    }
    impl Drop for CleanupRegion<'_> {
        fn drop(&mut self) {
            let control = self.control;
            let before = self.before;
            let body_end = Instant::now();
            let body_cpu = match (before, bbt_rt::fault::thread_cpu_time()) {
                (Ok(before), Ok(after)) => after.checked_sub(before).ok_or(Code::ResourceLimit),
                _ => Err(Code::ResourceLimit),
            };
            #[cfg(debug_assertions)]
            if body_cpu.is_err() {
                eprintln!("C1 cleanup clock unknown region=required_cleanup");
            }
            control.debt.charge(body_cpu, body_end);
            let _ = control.debt.wait(None, true);
            let elapsed = match (before, bbt_rt::fault::thread_cpu_time()) {
                (Ok(before), Ok(after)) => after
                    .checked_sub(before)
                    .unwrap_or(UNIT + Duration::from_nanos(1)),
                _ => UNIT + Duration::from_nanos(1),
            };
            if elapsed > body_cpu.unwrap_or(UNIT) {
                let cost = nanos(elapsed.max(UNIT)).saturating_mul(9);
                control.debt.due_ns.fetch_max(
                    nanos(body_end.saturating_duration_since(control.debt.epoch))
                        .saturating_add(cost),
                    Ordering::AcqRel,
                );
                let _ = control.debt.wait(None, true);
            }
            record_cleanup_at(control, elapsed, body_end, self.kind, self.origin);
            if self.nested {
                control.debt.resume();
            } else {
                WORK_FRAME.with(|f| {
                    f.borrow_mut().take();
                });
                control.debt.held.store(false, Ordering::Release);
            }
        }
    }
    #[track_caller]
    pub(super) fn cleanup_unit(control: &Cleanup, work: impl FnOnce()) {
        // Required destruction remains independent of foreground/cancel and
        // retains the same exclusive work debt through clocks and cleanup.
        cleanup_owned(control, "native_leaf", std::panic::Location::caller(), work);
    }
    fn cleanup_owned(
        control: &Cleanup,
        kind: &'static str,
        origin: &'static std::panic::Location<'static>,
        work: impl FnOnce(),
    ) {
        let _region = CleanupRegion::begin_owned(control, kind, origin);
        work();
    }
    pub(crate) trait Drain {
        /// Remove at most one payload/entry, following only the admitted depth.
        fn drain_step(&mut self) -> bool;
    }
    #[derive(Default)]
    pub(crate) struct Cleanup {
        pub(super) cpu_ns: AtomicU64,
        pub(super) max_unit_cpu_ns: AtomicU64,
        pub(super) overruns: AtomicU64,
        pub(super) units: AtomicU64,
        pub(super) capacity_refusals: AtomicU64,
        pub(super) debt: Arc<WorkDebt>,
    }
    pub(crate) struct Paced<T: Drain> {
        value: Option<T>,
        control: Arc<Cleanup>,
        origin: &'static std::panic::Location<'static>,
        // Destruction order is explicit: the exact owner empties before credit.
        pub(super) credit: Option<Reservation>,
    }
    impl<T: Drain> Paced<T> {
        #[track_caller]
        pub(super) fn new(value: T, control: &Control) -> Self {
            Self::at(value, control, std::panic::Location::caller())
        }
        pub(super) fn at(
            value: T,
            control: &Control,
            origin: &'static std::panic::Location<'static>,
        ) -> Self {
            Self {
                value: Some(value),
                control: control.cleanup.clone(),
                origin,
                credit: None,
            }
        }
        /// Only an explicit ownership transfer may bypass this guard's Drop.
        pub(crate) fn take_value(&mut self) -> T {
            self.value.take().unwrap()
        }
    }
    impl<T: Drain + Serialize> Serialize for Paced<T> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.value.as_ref().unwrap().serialize(serializer)
        }
    }
    impl<T: Drain + std::fmt::Debug> std::fmt::Debug for Paced<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_tuple("Paced").field(&self.value).finish()
        }
    }
    impl<T: Drain> std::ops::Deref for Paced<T> {
        type Target = T;
        fn deref(&self) -> &T {
            self.value.as_ref().unwrap()
        }
    }
    impl<T: Drain> std::ops::DerefMut for Paced<T> {
        fn deref_mut(&mut self) -> &mut T {
            self.value.as_mut().unwrap()
        }
    }
    impl<T: Drain> Drain for Paced<T> {
        fn drain_step(&mut self) -> bool {
            let Some(value) = self.value.as_mut() else {
                return false;
            };
            if !value.drain_step() {
                // Already inside the outer owner's measured cleanup unit.
                // Retire this empty inner owner/credit here, not recursively
                // through another paced Drop (which would double-count CPU).
                drop(self.value.take());
                drop(self.credit.take());
            }
            true
        }
    }
    impl<T: Drain> Drop for Paced<T> {
        fn drop(&mut self) {
            while self.value.is_some() {
                cleanup_owned(
                    &self.control,
                    std::any::type_name::<T>(),
                    self.origin,
                    || {
                        let before = bbt_rt::fault::thread_cpu_time().ok();
                        // The finite step count still applies when the clock fails.
                        for _ in 0..128 {
                            if !self.value.as_mut().unwrap().drain_step() {
                                drop(self.value.take());
                                break;
                            }
                            if before.is_none_or(|before| {
                                bbt_rt::fault::thread_cpu_time().map_or(true, |now| {
                                    now.saturating_sub(before) >= Duration::from_micros(400)
                                })
                            }) {
                                break;
                            }
                        }
                    },
                );
            }
            // credit remains present throughout all actual cleanup, even if a
            // cleanup clock fails or a unit exceeds its limit.
            drop(self.credit.take());
        }
    }
    macro_rules! leaves {
        ($($ty:ty),* $(,)?) => {$ (impl Drain for $ty {
            fn drain_step(&mut self) -> bool { false }
        })*};
    }
    leaves!(
        (),
        bool,
        u8,
        u32,
        u64,
        i64,
        usize,
        String,
        PathBuf,
        File,
        std::process::Child,
        std::io::Error,
        Metadata,
        (File, Metadata),
        (u64, u64),
        [u8; 32],
        crate::Ulid,
        bootstrap::Frozen,
        Duration,
        UnitClock,
        transport::JsonBounds,
        Reservation,
        Publication,
        transport::PreparedBuffers,
        transport::PreparedStart,
        bootstrap::Gate,
        ProbeClaim,
        LaunchBookmark,
        (crate::Ulid, crate::Ulid, crate::Ulid, crate::Ulid),
        transport::PipePair,
        transport::PreparedCommand,
        Worker,
        std::process::Command,
        ExistingJournal,
        Journal,
        w::Hex,
        w::PageRef,
        w::Blob,
        w::Context,
        crate::identity::PublicFence,
        w::Hello,
        w::Origin,
        w::Cluster,
        (String, u64, String),
        (String, String),
        (usize, String),
        (u32, u64, u64),
        public::Hex,
        // Fixed public context: bounded scalar IDs/epochs only, no collections.
        // Its identifier storage retires within the measured native-free leaf.
        public::Context,
        public::Cluster
    );
    impl<T: Drain> Drain for Option<T> {
        fn drain_step(&mut self) -> bool {
            let Some(value) = self.as_mut() else {
                return false;
            };
            if !value.drain_step() {
                self.take();
            }
            true
        }
    }
    impl<T: Drain> Drain for Result<T, Code> {
        fn drain_step(&mut self) -> bool {
            match self {
                Ok(value) => value.drain_step(),
                Err(_) => false,
            }
        }
    }
    impl<T: Drain> Drain for Vec<T> {
        fn drain_step(&mut self) -> bool {
            // Trivial scalar buffers have no element destruction. Retire their
            // one backing allocation in this measured native-free leaf rather
            // than inventing thousands of per-byte cleanup steps.
            if !std::mem::needs_drop::<T>() {
                return false;
            }
            let Some(value) = self.last_mut() else {
                return false;
            };
            if !value.drain_step() {
                self.pop();
            }
            true
        }
    }
    impl<K: Ord + Drain, V: Drain> Drain for BTreeMap<K, V> {
        fn drain_step(&mut self) -> bool {
            let Some(mut entry) = self.first_entry() else {
                return false;
            };
            if !entry.get_mut().drain_step() {
                entry.remove_entry();
            }
            true
        }
    }
    impl<T: Ord + Drain> Drain for BTreeSet<T> {
        fn drain_step(&mut self) -> bool {
            self.pop_first().is_some()
        }
    }
    impl Drain for Value {
        fn drain_step(&mut self) -> bool {
            match self {
                Value::Null => false,
                Value::Array(values) => values.drain_step(),
                Value::Object(values) => {
                    let Some((key, child)) = values.iter_mut().next() else {
                        return false;
                    };
                    if child.drain_step() {
                        return true;
                    }
                    // serde_json::Map exposes no pop-entry. This bounded stack
                    // copy runs only for leaf removal, never recursively. The
                    // largest reachable engine JSON frame is result.json768KiB;
                    // IR frames are256KiB and page frames16KiB before this port.
                    #[inline(never)]
                    fn remove<const N: usize>(values: &mut serde_json::Map<String, Value>) {
                        let mut key_bytes = [0; N];
                        let key = values.keys().next().unwrap();
                        let len = key.len();
                        assert!(len <= N, "preflight/frame key bound");
                        key_bytes[..len].copy_from_slice(key.as_bytes());
                        values.remove_entry(std::str::from_utf8(&key_bytes[..len]).unwrap());
                    }
                    if key.len() <= 512 {
                        remove::<512>(values);
                    } else {
                        remove::<{ 768 * 1024 }>(values);
                    }
                    true
                }
                _ => {
                    *self = Value::Null;
                    true
                }
            }
        }
    }
    macro_rules! fields {
        ($ty:ty; $($field:ident),+ $(,)?) => {impl Drain for $ty {
            fn drain_step(&mut self) -> bool { false $(|| self.$field.drain_step())+ }
        }};
    }
    fields!(w::ResultDescriptor; pages);
    fields!(w::Page; rows);
    fields!(w::Episode; fingerprints);
    fields!(w::Source; blobs);
    fields!(w::Manifest; sources);
    fields!(public::Page; rows);
    fields!(public::Episode; fingerprints);
    impl Drain for w::Row {
        fn drain_step(&mut self) -> bool {
            match self {
                Self::Episode(row) => row.drain_step(),
                Self::Cluster(_) => false,
            }
        }
    }
    impl Drain for public::Row {
        fn drain_step(&mut self) -> bool {
            match self {
                Self::Episode(row) => row.drain_step(),
                Self::Cluster(_) => false,
            }
        }
    }

    #[derive(Clone, Copy)]
    struct ValueSeed<'a>(&'a Control);
    impl<'de> DeserializeSeed<'de> for ValueSeed<'_> {
        type Value = Value;
        fn deserialize<D: serde::Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Value, D::Error> {
            struct JsonVisitor<'a>(&'a Control);
            impl<'de> Visitor<'de> for JsonVisitor<'_> {
                type Value = Value;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("JSON value")
                }
                fn visit_unit<E>(self) -> Result<Value, E> {
                    Ok(Value::Null)
                }
                fn visit_none<E>(self) -> Result<Value, E> {
                    Ok(Value::Null)
                }
                fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
                    Ok(v.into())
                }
                fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
                    Ok(v.into())
                }
                fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
                    Ok(v.into())
                }
                fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Value, E> {
                    serde_json::Number::from_f64(v)
                        .map(Value::Number)
                        .ok_or_else(|| E::custom("non-finite number"))
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Value, E> {
                    Ok(Value::String(v.to_owned()))
                }
                fn visit_string<E>(self, v: String) -> Result<Value, E> {
                    Ok(Value::String(v))
                }
                fn visit_seq<A: SeqAccess<'de>>(self, sequence: A) -> Result<Value, A::Error> {
                    let mut owner = VecVisitor {
                        control: self.0,
                        seed: ValueSeed(self.0),
                    }
                    .visit_seq(sequence)?;
                    Ok(Value::Array(owner.take_value()))
                }
                fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                    let mut owner = Paced::new(serde_json::Map::new(), self.0);
                    while let Some(key) = map.next_key::<String>()? {
                        let value = map.next_value_seed(ValueSeed(self.0))?;
                        if let Some(old) = owner.insert(key, value) {
                            // Preserve serde's last-value duplicate-key semantics,
                            // but do not recursively drop the superseded tree.
                            drop(Paced::new(old, self.0));
                        }
                    }
                    Ok(Value::Object(owner.take_value()))
                }
            }
            deserializer.deserialize_any(JsonVisitor(self.0))
        }
    }
    impl Drain for serde_json::Map<String, Value> {
        fn drain_step(&mut self) -> bool {
            if self.is_empty() {
                return false;
            }
            // Reuse Value's no-allocation path while retaining map ownership.
            let mut value = Value::Object(std::mem::take(self));
            let progress = value.drain_step();
            let Value::Object(map) = value else {
                unreachable!()
            };
            *self = map;
            progress
        }
    }
    struct LeafSeed<T>(PhantomData<fn() -> T>);
    impl<T> Copy for LeafSeed<T> {}
    impl<T> Clone for LeafSeed<T> {
        fn clone(&self) -> Self {
            *self
        }
    }
    impl<T> LeafSeed<T> {
        fn new() -> Self {
            Self(PhantomData)
        }
    }
    impl<'de, T: serde::Deserialize<'de>> DeserializeSeed<'de> for LeafSeed<T> {
        type Value = T;
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<T, D::Error> {
            T::deserialize(d)
        }
    }
    struct VecSeed<'a, S> {
        control: &'a Control,
        seed: S,
    }
    struct VecVisitor<'a, S> {
        control: &'a Control,
        seed: S,
    }
    impl<'de, 'a, S, T> Visitor<'de> for VecVisitor<'a, S>
    where
        S: DeserializeSeed<'de, Value = T> + Clone,
        T: Drain,
    {
        type Value = Paced<Vec<T>>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("array")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let mut owner = Paced::new(Vec::new(), self.control);
            while let Some(value) = sequence.next_element_seed(self.seed.clone())? {
                // The completed child already has an owner before any growth
                // verdict can fail. Preflight credit covers final entries;
                // retain an additional old-capacity credit through realloc.
                let mut value = Paced::new(value, self.control);
                if owner.len() == owner.capacity() {
                    let refused = || {
                        self.control
                            .cleanup
                            .capacity_refusals
                            .fetch_add(1, Ordering::Relaxed);
                        <A::Error as serde::de::Error>::custom("C1 collection capacity refused")
                    };
                    let old_bytes = owner
                        .capacity()
                        .checked_mul(std::mem::size_of::<T>())
                        .ok_or_else(refused)?;
                    let overlap = self
                        .control
                        .heap
                        .reserve(HeapLane::Work, old_bytes)
                        .map_err(|_| refused())?;
                    owner.try_reserve_exact(1).map_err(|_| refused())?;
                    drop(overlap);
                }
                owner.push(value.take_value());
            }
            Ok(owner)
        }
    }
    impl<'de, 'a, S, T> DeserializeSeed<'de> for VecSeed<'a, S>
    where
        S: DeserializeSeed<'de, Value = T> + Clone,
        T: Drain,
    {
        type Value = Paced<Vec<T>>;
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            d.deserialize_seq(VecVisitor {
                control: self.control,
                seed: self.seed,
            })
        }
    }

    // Fixed protocol adapters, not a second wire authority. All scalar types
    // are the pinned mirror's exact types; canonical corpus covers their joins.
    macro_rules! object_seed {
        ($seed:ident<$life:lifetime>($control:ident) => $output:ty { $($field:ident:$ty:ty => $decoder:expr),+ $(,)? } => $build:expr) => {
            #[derive(Clone,Copy)] struct $seed<$life>(&$life Control);
            impl<'de,$life> DeserializeSeed<'de> for $seed<$life> {
                type Value=$output;
                fn deserialize<D:serde::Deserializer<'de>>(self,d:D)->Result<Self::Value,D::Error> {
                    struct Fields<$life>(&$life Control);
                    impl<'de,$life> Visitor<'de> for Fields<$life> {
                        type Value=$output;
                        fn expecting(&self,f:&mut fmt::Formatter<'_>)->fmt::Result { f.write_str(stringify!($output)) }
                        #[allow(unused_mut,reason="some ownership fields transfer their paced collection")]
                        fn visit_map<A:MapAccess<'de>>(self,mut map:A)->Result<Self::Value,A::Error> {
                            let $control=self.0;
                            $(let mut $field:Option<$ty>=None;)+
                            while let Some(key)=map.next_key::<String>()? {
                                match key.as_str() {
                                    $(stringify!($field)=>{
                                        if $field.is_some() { return Err(serde::de::Error::duplicate_field(stringify!($field))); }
                                        $field=Some(map.next_value_seed($decoder)?);
                                    },)+
                                    _=>return Err(serde::de::Error::custom("unknown C1 object field")),
                                }
                            }
                            $(let mut $field=$field.ok_or_else(||serde::de::Error::missing_field(stringify!($field)))?;)+
                            Ok($build)
                        }
                    }
                    d.deserialize_map(Fields(self.0))
                }
            }
        };
    }
    object_seed!(DescriptorSeed<'a>(control)=>w::ResultDescriptor {
        schema:w::ResultSchema=>LeafSeed::new(), run_id:w::Ulid=>LeafSeed::new(),
        nonce:w::Ulid=>LeafSeed::new(), manifest_sha256:w::Hex=>LeafSeed::new(),
        algorithm:w::Algorithm=>LeafSeed::new(), source_count:u32=>LeafSeed::new(),
        episode_count:u32=>LeafSeed::new(), cluster_count:u32=>LeafSeed::new(),
        max_poll_cpu_ns:w::Number=>LeafSeed::new(), knowledge:w::Knowledge=>LeafSeed::new(),
        trust:w::Trust=>LeafSeed::new(), pages:Paced<Vec<w::PageRef>> => VecSeed {control,seed:LeafSeed::new()}
    }=>w::ResultDescriptor {schema,run_id,nonce,manifest_sha256,algorithm,source_count,
        episode_count,cluster_count,max_poll_cpu_ns,knowledge,trust,pages:pages.take_value()});
    object_seed!(PageSeed<'a>(control)=>w::Page {
        schema:w::PageSchema=>LeafSeed::new(),run_id:w::Ulid=>LeafSeed::new(),
        manifest_sha256:w::Hex=>LeafSeed::new(),index:u32=>LeafSeed::new(),
        rows:Paced<Vec<w::Row>> => VecSeed {control,seed:RowSeed(control)},next:Option<u32> => LeafSeed::new()
    }=>w::Page {schema,run_id,manifest_sha256,index,rows:rows.take_value(),next});
    #[derive(Clone, Copy)]
    struct RowSeed<'a>(&'a Control);
    impl<'de, 'a> DeserializeSeed<'de> for RowSeed<'a> {
        type Value = w::Row;
        fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
            struct Fields<'a>(&'a Control);
            impl<'de, 'a> Visitor<'de> for Fields<'a> {
                type Value = w::Row;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("C1 row")
                }
                fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                    let (mut kind, mut first_ms, mut last_ms) =
                        (None::<String>, None::<w::Number>, None::<w::Number>);
                    let (mut repo, mut fingerprint, mut count) =
                        (None::<w::Repo>, None::<w::Hex>, None::<u32>);
                    let (mut session, mut turn, mut origin, mut fingerprints) = (
                        None::<w::Ulid>,
                        None::<w::Ulid>,
                        None::<w::Origin>,
                        None::<Paced<Vec<w::Hex>>>,
                    );
                    macro_rules! read {
                        ($field:ident) => {{
                            if $field.is_some() {
                                return Err(serde::de::Error::duplicate_field(stringify!($field)));
                            }
                            $field = Some(map.next_value()?);
                        }};
                    }
                    while let Some(key) = map.next_key::<String>()? {
                        match key.as_str() {
                            "kind" => read!(kind),
                            "first_ms" => read!(first_ms),
                            "last_ms" => read!(last_ms),
                            "repo" => read!(repo),
                            "fingerprint" => read!(fingerprint),
                            "count" => read!(count),
                            "session" => read!(session),
                            "turn" => read!(turn),
                            "origin" => read!(origin),
                            "fingerprints" => {
                                if fingerprints.is_some() {
                                    return Err(serde::de::Error::duplicate_field("fingerprints"));
                                }
                                fingerprints = Some(map.next_value_seed(VecSeed {
                                    control: self.0,
                                    seed: LeafSeed::new(),
                                })?);
                            }
                            _ => return Err(serde::de::Error::custom("unknown C1 row field")),
                        }
                    }
                    macro_rules! required {
                        ($value:ident) => {
                            $value.ok_or_else(|| {
                                serde::de::Error::missing_field(stringify!($value))
                            })?
                        };
                    }
                    let first_ms = required!(first_ms);
                    let last_ms = required!(last_ms);
                    match required!(kind).as_str() {
                        "episode" => {
                            if repo.is_some() || fingerprint.is_some() || count.is_some() {
                                return Err(serde::de::Error::custom("cluster field in episode"));
                            }
                            let session = required!(session);
                            let turn = required!(turn);
                            let origin = required!(origin);
                            let mut fingerprints = required!(fingerprints);
                            Ok(w::Row::Episode(w::Episode {
                                session,
                                turn,
                                origin,
                                fingerprints: fingerprints.take_value(),
                                first_ms,
                                last_ms,
                            }))
                        }
                        "cluster" => {
                            if session.is_some()
                                || turn.is_some()
                                || origin.is_some()
                                || fingerprints.is_some()
                            {
                                return Err(serde::de::Error::custom("episode field in cluster"));
                            }
                            Ok(w::Row::Cluster(w::Cluster {
                                repo: required!(repo),
                                fingerprint: required!(fingerprint),
                                count: required!(count),
                                first_ms,
                                last_ms,
                            }))
                        }
                        _ => Err(serde::de::Error::custom("unknown C1 row kind")),
                    }
                }
            }
            d.deserialize_map(Fields(self.0))
        }
    }
    pub(super) trait OwnedDecode: Drain + Sized {
        fn decode<R: Read>(reader: R, control: &Control) -> serde_json::Result<Paced<Self>>;
    }
    macro_rules! decode {
        ($ty:ty,$seed:ident) => {
            impl OwnedDecode for $ty {
                fn decode<R: Read>(
                    reader: R,
                    control: &Control,
                ) -> serde_json::Result<Paced<Self>> {
                    let mut decoder = serde_json::Deserializer::from_reader(reader);
                    let value = $seed(control).deserialize(&mut decoder)?;
                    let owner = Paced::new(value, control);
                    decoder.end()?;
                    Ok(owner)
                }
            }
        };
    }
    decode!(Value, ValueSeed);
    decode!(w::ResultDescriptor, DescriptorSeed);
    decode!(w::Page, PageSeed);
}
use disposal::{Drain, OwnedDecode, Paced};

#[derive(Clone, Copy)]
struct UnitClock {
    cpu: Duration,
    cleanup: u64,
}

#[derive(Debug)]
enum SmallJson {
    Processed,
    Fallback(Option<transport::JsonBounds>),
}
impl Drain for SmallJson {
    fn drain_step(&mut self) -> bool {
        false
    }
}
enum TinyPrepared {
    Fallback(transport::JsonBounds),
    Parsed {
        owner: Paced<Value>,
        verdict: Result<(), Code>,
    },
}
impl Drain for TinyPrepared {
    fn drain_step(&mut self) -> bool {
        match self {
            Self::Fallback(_) => false,
            Self::Parsed { owner, .. } => owner.drain_step(),
        }
    }
}

impl UnitClock {
    fn now() -> Result<Self, Code> {
        Ok(Self {
            cpu: bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?,
            cleanup: disposal::thread_cleanup_cpu(),
        })
    }
    fn elapsed(self) -> Result<Duration, Code> {
        bbt_rt::fault::thread_cpu_time()
            .map_err(|_| Code::ResourceLimit)?
            .checked_sub(self.cpu)
            .ok_or(Code::ResourceLimit)?
            .checked_sub(Duration::from_nanos(
                disposal::thread_cleanup_cpu().saturating_sub(self.cleanup),
            ))
            .ok_or(Code::ResourceLimit)
    }
}

/// One isolated request's ordinary metadata units. Controls never borrow the
/// admitted job's work/native lane, so status/cancel remain independent of IO.
/// No native admission operation is exposed by this context.
#[derive(Default)]
pub(crate) struct JournalQuantum {
    control: Control,
    origin: Option<&'static std::panic::Location<'static>>,
}
impl JournalQuantum {
    pub(crate) fn preparation() -> Result<Self, Code> {
        // Fixed owner/context allocation is an ordinary measured prefix too.
        // It owns no native effect; its full after-end debt precedes file IO.
        let before = UnitClock::now()?;
        let owner = Self::default();
        let used = before.elapsed();
        owner.control.cleanup.debt.charge(used, Instant::now());
        let used = used?;
        owner
            .control
            .max_unit_cpu_ns
            .store(nanos(used), Ordering::Release);
        if used > UNIT {
            eprintln!(
                "dream_c1_preparation_owner_refused cpu_ns={} limit_ns=1000000",
                nanos(used)
            );
            return Err(Code::ResourceLimit);
        }
        owner.pay()?;
        Ok(owner)
    }
    pub(crate) fn at(origin: &'static std::panic::Location<'static>) -> Self {
        Self {
            origin: Some(origin),
            ..Self::default()
        }
    }
    #[track_caller]
    pub(crate) fn retain<T: Drain>(&self, value: T) -> disposal::Paced<T> {
        Paced::at(
            value,
            &self.control,
            self.origin.unwrap_or(std::panic::Location::caller()),
        )
    }
    pub(crate) fn check(&self) -> Result<(), Code> {
        if self.control.cleanup.overruns.load(Ordering::Acquire) > 0
            || self.control.cleanup.debt.poisoned.load(Ordering::Acquire)
        {
            Err(Code::ResourceLimit)
        } else {
            Ok(())
        }
    }
    pub(crate) fn max_unit_cpu_ns(&self) -> u64 {
        self.control.max_unit_cpu_ns.load(Ordering::Acquire)
    }
    #[cfg(test)]
    pub(crate) fn max_cleanup_unit_cpu_ns(&self) -> u64 {
        self.control.cleanup.max_unit_cpu_ns.load(Ordering::Acquire)
    }
    #[track_caller]
    pub(crate) fn public_prelude<T: Drain>(
        &self,
        work: impl FnOnce(&mut dyn FnMut() -> crate::store::Result<()>) -> Result<T, Code>,
    ) -> Result<T, Code> {
        self.public_prelude_sampled(work, UnitClock::elapsed)
    }
    #[track_caller]
    fn public_prelude_sampled<T: Drain>(
        &self,
        work: impl FnOnce(&mut dyn FnMut() -> crate::store::Result<()>) -> Result<T, Code>,
        mut sample: impl FnMut(UnitClock) -> Result<Duration, Code>,
    ) -> Result<T, Code> {
        self.check()?;
        let mut budget =
            Budget::for_control(&self.control, Instant::now() + Duration::from_secs(30));
        let mut cpu = Some(budget.begin_unit()?);
        let mut stopped = None;
        let mut unwind = None;
        let result = {
            let mut checkpoint = || -> crate::store::Result<()> {
                if stopped.is_some() {
                    return Err(crate::store::invalid("public prelude stopped"));
                }
                let Some(previous) = cpu.take() else {
                    return Err(crate::store::invalid("public prelude missing work owner"));
                };
                let next = budget
                    .finish_unit_sampled(previous, Ok(()), &mut sample)
                    .and_then(|()| budget.begin_unit());
                match next {
                    Ok(next) => cpu = Some(next),
                    Err(code) => {
                        stopped = Some(code);
                        // No new public proof runs after this callback fails.
                        // Required cleanup still acquires/retains the debt owner
                        // when an unknown clock prevents ordinary admission.
                        unwind = Some(disposal::CleanupRegion::begin(&self.control.cleanup));
                    }
                }
                if stopped.is_some() {
                    Err(crate::store::invalid("public prelude refused"))
                } else {
                    Ok(())
                }
            };
            work(&mut checkpoint)
        };
        // Capture any returned value BEFORE ending even a failed-clock cleanup
        // region. A malformed accessor cannot return an unowned T on refusal.
        let mut owner = Paced::new(result, &self.control);
        drop(unwind);
        if let Some(code) = stopped {
            drop(owner);
            return Err(code);
        }
        let Some(cpu) = cpu else {
            drop(owner);
            return Err(Code::ResourceLimit);
        };
        budget.finish_unit(cpu, owner.take_value())
    }
    pub(crate) fn pay(&self) -> Result<(), Code> {
        self.check()?;
        self.control
            .cleanup
            .debt
            .wait(Some(Instant::now() + Duration::from_secs(30)), false)
    }
    /// Only already-owned lifecycle teardown may run after ordinary admission
    /// failed. Capture its typed result before the required-cleanup clock; its
    /// refusal counters remain visible to the enclosing owner's final check.
    pub(crate) fn required<T: Drain>(
        &self,
        work: impl FnOnce() -> Result<T, Code>,
    ) -> Result<T, Code> {
        let mut result = None;
        disposal::cleanup_unit(&self.control.cleanup, || {
            result = Some(self.retain(work()));
        });
        result
            .expect("required cleanup ran its owned action")
            .take_value()
    }
    pub(crate) fn retire_session(
        &self,
        session: &crate::identity::PublicFenceSession,
    ) -> Result<(), Code> {
        retire_public_session(session, &self.control)
    }
    pub(crate) fn retire<T: Drain>(&self, value: T) {
        drop(self.retain(value));
    }
    #[track_caller]
    pub(crate) fn unit<T: Drain>(&self, work: impl FnOnce() -> Result<T, Code>) -> Result<T, Code> {
        if self.control.cleanup.overruns.load(Ordering::Acquire) > 0 {
            return Err(Code::ResourceLimit);
        }
        self.check()?;
        if WORK_FRAME.with(|frame| {
            frame
                .borrow()
                .as_ref()
                .is_some_and(|frame| Arc::ptr_eq(&frame.debt, &self.control.cleanup.debt))
        }) {
            // A finite control transaction/lookup may contain several bounded
            // statements in one ordinary unit. The OUTER full <=1ms verdict
            // includes every nested statement and all wrapper application CPU.
            return work();
        }
        let mut result = Paced::new(
            Budget::for_control(&self.control, Instant::now() + Duration::from_secs(30)).unit(work),
            &self.control,
        );
        self.check()?;
        result.take_value()
    }
}
impl Drain for crate::identity::PublicFenceSession {
    fn drain_step(&mut self) -> bool {
        false
    }
}
impl Drain
    for (
        Arc<AuthorityGuard>,
        Arc<crate::identity::PublicFenceSession>,
    )
{
    fn drain_step(&mut self) -> bool {
        false
    }
}
impl Drain for rusqlite::Connection {
    fn drain_step(&mut self) -> bool {
        false
    }
}
impl Drain for rusqlite::Transaction<'_> {
    fn drain_step(&mut self) -> bool {
        false
    }
}
impl Drain for rusqlite::Rows<'_> {
    fn drain_step(&mut self) -> bool {
        false
    }
}
impl Drain for rusqlite::Statement<'_> {
    fn drain_step(&mut self) -> bool {
        false
    }
}
impl Drain for (public::Context, crate::identity::PublicFence) {
    fn drain_step(&mut self) -> bool {
        false
    }
}
impl Drain for (Vec<u8>, String) {
    fn drain_step(&mut self) -> bool {
        self.0.drain_step()
    }
}
impl Drain for Bookmark {
    fn drain_step(&mut self) -> bool {
        self.turns.drain_step()
    }
}
impl Drain for Record {
    fn drain_step(&mut self) -> bool {
        self.sources.drain_step() || self.request.drain_step()
    }
}

struct Budget<'a> {
    control: &'a Control,
    control_only: bool,
    deadline: Instant,
    rest_until: Instant,
    previous_end: Option<Instant>,
    disk: u64,
    input: u64,
    envelopes: u64,
    declared: Option<Reservation>,
}
impl Drop for Budget<'_> {
    fn drop(&mut self) {
        // A parser/clock error can leave through `?` before its last verdict.
        // Preserve debt and prevent a later Budget from erasing that prefix.
        self.control.cleanup.debt.finish(None);
    }
}
impl<'a> Budget<'a> {
    fn new(control: &'a Control, deadline: Instant) -> Self {
        Self {
            control,
            control_only: false,
            deadline,
            rest_until: Instant::now(),
            previous_end: None,
            disk: 0,
            input: 0,
            envelopes: 0,
            declared: Some(
                control
                    .heap
                    .reserve(HeapLane::Work, 0)
                    .expect("zero declaration within enforced quota"),
            ),
        }
    }
    /// Captured lifecycle metadata only: independent of expensive-work
    /// admission, still bounded by the same CPU/rest/wall/heap checks.
    fn for_control(control: &'a Control, deadline: Instant) -> Self {
        let mut budget = Self::new(control, deadline);
        budget.control_only = true;
        budget
    }
    fn pay_work_debt(&self) -> Result<(), Code> {
        let debt = &self.control.cleanup.debt;
        loop {
            debt.wait(Some(self.deadline), false)?;
            if !debt.held.load(Ordering::Acquire) && debt.remaining_ns() == 0 {
                return Ok(());
            }
            if Instant::now() >= self.deadline {
                return Err(Code::Deadline);
            }
            std::thread::sleep(REST);
        }
    }
    fn native_process(
        &mut self,
        command: &mut std::process::Command,
    ) -> Result<NativeOutcome<std::process::Child>, Code> {
        self.native_process_impl(command, None)
    }
    fn native_process_impl(
        &mut self,
        command: &mut std::process::Command,
        forced: Option<NativeSample>,
    ) -> Result<NativeOutcome<std::process::Child>, Code> {
        let span = self.begin_unit()?;
        self.control
            .cleanup
            .debt
            .process_effect
            .store(1, Ordering::Release);
        let pending = match monotonic_ns() {
            Ok(time) => time,
            Err(code) => {
                return Err(self
                    .finish_unit::<()>(span, Err(code))
                    .err()
                    .unwrap_or(Code::ProtocolError));
            }
        };
        self.control
            .native_pending_since_ns
            .store(pending, Ordering::Release);
        let before = match self.native_admit(span, true) {
            Ok(cpu) => cpu,
            Err(code) => {
                self.control
                    .native_pending_since_ns
                    .store(0, Ordering::Release);
                return Err(code);
            }
        };
        let wall = Instant::now();
        // The only process native boundary: no command preparation or fcntl.
        let effect = command.spawn();
        let returned_at = Instant::now();
        // `effect` owns the returned Child OR exact io::Error before either
        // clock is consulted. No error mapping drops native evidence.
        let end = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit);
        let wall = wall.elapsed();
        // `pending` precedes native admission/spawn; this sample follows the
        // original returned_at. The return lies within these conservative
        // CLOCK_MONOTONIC bounds. This clock call is surrounding application
        // work, outside the measured native CPU subspan.
        let entry_clock = monotonic_ns().and_then(|upper| {
            transport::BootstrapEntryClockBounds::new(
                transport::BootstrapEntryClock::ClockGettimeMonotonic,
                pending,
                pending,
                upper,
            )
            .map_err(wire_error)
        });
        if effect.is_err() {
            self.control
                .native_birth_unknown
                .store(true, Ordering::Release);
        }
        let errno = effect.as_ref().err().and_then(std::io::Error::raw_os_error);
        self.control
            .cleanup
            .debt
            .process_errno
            .store(errno.map_or(0, i64::from), Ordering::Release);
        self.control
            .cleanup
            .debt
            .process_effect
            .store(if effect.is_ok() { 2 } else { 3 }, Ordering::Release);
        let (effect, raw_error) = match effect {
            Ok(child) => (Ok(child), None),
            Err(error) => (
                Err(Code::RunnerUnavailable),
                Some(Paced::new(error, self.control)),
            ),
        };
        let owner = Paced::new(effect, self.control);
        #[cfg(test)]
        PROCESS_RETURN_HOOK.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take()
                && let Ok(child) = &*owner
            {
                hook(child);
            }
        });
        #[cfg(test)]
        NATIVE_RETURN_HOOK.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take() {
                hook();
            }
        });
        let sample = forced.unwrap_or(NativeSample {
            cpu: end.and_then(|end| end.checked_sub(before).ok_or(Code::ResourceLimit)),
            wall,
            inclusive: span.elapsed(),
        });
        let mut outcome = self.native_finish(owner, raw_error, sample, true, returned_at);
        match entry_clock {
            Ok(bounds) => outcome.entry_clock = Some(bounds),
            Err(code) => {
                self.control
                    .bootstrap_clock_failures
                    .fetch_add(1, Ordering::Relaxed);
                outcome.verdict = Err(code);
            }
        }
        Ok(outcome)
    }
    fn native_publish(&mut self, publication: &mut Publication) -> Result<NativeOutcome<()>, Code> {
        self.native_publish_impl(publication, None)
    }
    fn native_publish_impl(
        &mut self,
        publication: &mut Publication,
        forced: Option<NativeSample>,
    ) -> Result<NativeOutcome<()>, Code> {
        let bit = match publication
            .target
            .file_name()
            .and_then(|name| name.to_str())
        {
            Some("manifest.json") => 1,
            Some("worker-started.json") => 2,
            _ => return Err(Code::BadRequest),
        };
        if publication.prechecked_identity.is_none() || publication.attempted {
            return Err(Code::BadRequest);
        }
        let span = self.begin_unit()?;
        let claimed = self.control.cleanup.debt.native_publish_mask.try_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |mask| (mask & bit == 0).then_some(mask | bit),
        );
        if claimed.is_err() {
            return Err(self
                .finish_unit::<()>(span, Err(Code::ResourceLimit))
                .err()
                .unwrap_or(Code::ProtocolError));
        }
        publication.attempted = true;
        let identity = publication.prechecked_identity.unwrap();
        self.control
            .cleanup
            .debt
            .publish_dev
            .store(identity.0, Ordering::Release);
        self.control
            .cleanup
            .debt
            .publish_ino
            .store(identity.1, Ordering::Release);
        self.control
            .cleanup
            .debt
            .publish_effect
            .store(1, Ordering::Release);
        let pending = match monotonic_ns() {
            Ok(time) => time,
            Err(code) => {
                return Err(self
                    .finish_unit::<()>(span, Err(code))
                    .err()
                    .unwrap_or(Code::ProtocolError));
            }
        };
        self.control
            .native_pending_since_ns
            .store(pending, Ordering::Release);
        let before = match self.native_admit(span, false) {
            Ok(cpu) => cpu,
            Err(code) => {
                self.control
                    .native_pending_since_ns
                    .store(0, Ordering::Release);
                return Err(code);
            }
        };
        let wall = Instant::now();
        // Exactly one prechecked atomic no-replace link. No metadata, flush,
        // path construction, target removal or retry is inside the native tag.
        let effect = std::fs::hard_link(&publication.temporary, &publication.target);
        let returned_at = Instant::now();
        // The owned native Result retains raw io::Error before clock sampling.
        let end = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit);
        let wall = wall.elapsed();
        let errno = effect.as_ref().err().and_then(std::io::Error::raw_os_error);
        self.control
            .cleanup
            .debt
            .publish_errno
            .store(errno.map_or(0, i64::from), Ordering::Release);
        self.control
            .cleanup
            .debt
            .publish_effect
            .store(if effect.is_ok() { 2 } else { 3 }, Ordering::Release);
        let (effect, raw_error) = match effect {
            Ok(()) => (Ok(()), None),
            Err(error) => (
                Err(Code::StorageUnavailable),
                Some(Paced::new(error, self.control)),
            ),
        };
        let owner = Paced::new(effect, self.control);
        #[cfg(test)]
        NATIVE_RETURN_HOOK.with(|hook| {
            if let Some(hook) = hook.borrow_mut().take() {
                hook();
            }
        });
        let sample = forced.unwrap_or(NativeSample {
            cpu: end.and_then(|end| end.checked_sub(before).ok_or(Code::ResourceLimit)),
            wall,
            inclusive: span.elapsed(),
        });
        Ok(self.native_finish(owner, raw_error, sample, false, returned_at))
    }
    fn native_admit(&mut self, span: UnitClock, process: bool) -> Result<Duration, Code> {
        if let Err(code) = self.gate() {
            return self.finish_unit(span, Err(code));
        }
        let used = span.elapsed();
        if used.is_err() || used.is_ok_and(|used| used > UNIT) {
            return self.finish_unit(span, Err(Code::ResourceLimit));
        }
        let calls = if process {
            &self.control.cleanup.debt.native_process_calls
        } else {
            &self.control.cleanup.debt.native_publish_calls
        };
        let cap = if process { 1 } else { 2 };
        if calls
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < cap).then_some(n + 1)
            })
            .is_err()
        {
            return self.finish_unit(span, Err(Code::ResourceLimit));
        }
        match bbt_rt::fault::thread_cpu_time() {
            Ok(cpu) => Ok(cpu),
            Err(_) => self.finish_unit(span, Err(Code::ResourceLimit)),
        }
    }
    fn native_finish<T: Drain>(
        &mut self,
        owner: Paced<Result<T, Code>>,
        raw_error: Option<Paced<std::io::Error>>,
        sample: NativeSample,
        process: bool,
        returned_at: Instant,
    ) -> NativeOutcome<T> {
        let debt = &self.control.cleanup.debt;
        if owner.is_ok() {
            debt.native_effects.fetch_add(1, Ordering::Relaxed);
        }
        let app = match (sample.inclusive, sample.cpu) {
            (Ok(total), Ok(native)) if total >= native => Ok(total - native),
            _ => Err(Code::ResourceLimit),
        };
        // Retain every available subspan even when another clock is missing.
        // A failed verdict never subtracts native cost from whole-owner CPU.
        debt.native_max_wall_ns
            .fetch_max(nanos(sample.wall), Ordering::Relaxed);
        let (kind_cpu, kind_wall) = if process {
            (
                &debt.native_process_max_cpu_ns,
                &debt.native_process_max_wall_ns,
            )
        } else {
            (
                &debt.native_publish_max_cpu_ns,
                &debt.native_publish_max_wall_ns,
            )
        };
        kind_wall.fetch_max(nanos(sample.wall), Ordering::Relaxed);
        if let Ok(native) = sample.cpu {
            debt.native_cpu_ns
                .fetch_add(nanos(native), Ordering::Relaxed);
            debt.native_max_cpu_ns
                .fetch_max(nanos(native), Ordering::Relaxed);
            kind_cpu.fetch_max(nanos(native), Ordering::Relaxed);
        }
        if let Ok(total) = sample.inclusive {
            debt.inclusive_max_cpu_ns
                .fetch_max(nanos(total), Ordering::Relaxed);
        }
        if let Ok(app) = app {
            debt.native_app_max_cpu_ns
                .fetch_max(nanos(app), Ordering::Relaxed);
        }
        let mut verdict = match (sample.cpu, sample.inclusive, app) {
            (Ok(native), Ok(total), Ok(app)) => {
                if native > UNIT * 2
                    || sample.wall > Duration::from_millis(25)
                    || app > UNIT
                    || total > UNIT * 3
                {
                    debt.native_overruns.fetch_add(1, Ordering::Relaxed);
                    Err(Code::ResourceLimit)
                } else {
                    Ok(())
                }
            }
            _ => {
                debt.native_clock_failures.fetch_add(1, Ordering::Relaxed);
                debt.poisoned.store(true, Ordering::Release);
                Err(Code::ResourceLimit)
            }
        };
        debt.finish(Some(sample.inclusive));
        self.control
            .native_pending_since_ns
            .store(0, Ordering::Release);
        if verdict.is_ok() && Instant::now() >= self.deadline {
            verdict = Err(Code::Deadline);
        }
        if verdict.is_ok() && !self.control_only && self.control.cancelled.load(Ordering::Acquire) {
            verdict = Err(Code::Cancelled);
        }
        NativeOutcome {
            effect: owner,
            raw_error,
            verdict,
            sample,
            returned_at,
            entry_clock: None,
        }
    }
    fn gate(&self) -> Result<(), Code> {
        loop {
            if !self.control_only && self.control.cleanup.overruns.load(Ordering::Acquire) > 0 {
                return Err(Code::ResourceLimit);
            }
            if !self.control_only && self.control.cancelled.load(Ordering::Acquire) {
                return Err(Code::Cancelled);
            }
            if Instant::now() >= self.deadline {
                return Err(Code::Deadline);
            }
            if self.control_only || !self.control.is_foreground() {
                return Ok(());
            }
            self.control.phase(PAUSED);
            std::thread::sleep(REST);
        }
    }
    fn begin_unit(&mut self) -> Result<UnitClock, Code> {
        loop {
            self.gate()?;
            if let Some(end) = disposal::last_cleanup_end() {
                self.rest_until = self.rest_until.max(end + REST);
                self.previous_end = Some(self.previous_end.map_or(end, |old| old.max(end)));
            }
            if let Some(rest) = self.rest_until.checked_duration_since(Instant::now()) {
                #[cfg(debug_assertions)]
                let omit =
                    self.control.private_fixture && self.control.omit_rest.load(Ordering::Acquire);
                #[cfg(not(debug_assertions))]
                let omit = false;
                if omit {
                    self.control.rest_skips.fetch_add(1, Ordering::Relaxed);
                } else {
                    let wall = Instant::now();
                    let cpu = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?;
                    std::thread::sleep(rest);
                    self.control
                        .rest_wait_ns
                        .fetch_add(nanos(wall.elapsed()), Ordering::Relaxed);
                    self.control.rest_wait_cpu_ns.fetch_add(
                        nanos(
                            bbt_rt::fault::thread_cpu_time()
                                .map_err(|_| Code::ResourceLimit)?
                                .saturating_sub(cpu),
                        ),
                        Ordering::Relaxed,
                    );
                }
            }
            self.gate()?;
            if let Some(end) = self.previous_end {
                let gap = end.elapsed();
                self.control
                    .min_inter_unit_gap_ns
                    .fetch_min(nanos(gap), Ordering::Relaxed);
                if gap < REST {
                    self.control.rest_violations.fetch_add(1, Ordering::Relaxed);
                }
            }
            self.control.work_unit_count.fetch_add(1, Ordering::Relaxed);
            #[cfg(debug_assertions)]
            if self.control.private_fixture && self.control.omit_rest.load(Ordering::Acquire) {
                // Deliberately broken private control: bypass the same durable
                // run debt, with an observable causal refusal counter.
                if self.control.cleanup.debt.remaining_ns() != 0 {
                    self.control
                        .cleanup
                        .debt
                        .admission_violations
                        .fetch_add(1, Ordering::Relaxed);
                    self.control
                        .cleanup
                        .debt
                        .due_ns
                        .store(self.control.cleanup.debt.now_ns(), Ordering::Release);
                }
            }
            self.control
                .cleanup
                .debt
                .enter(Some(self.deadline), self.control_only)?;
            if let Err(code) = self.gate() {
                self.control.cleanup.debt.finish(None);
                return Err(code);
            }
            let cpu = match UnitClock::now() {
                Ok(cpu) => cpu,
                Err(code) => {
                    self.control.cleanup.debt.finish(Some(Err(code)));
                    return Err(code);
                }
            };
            if !self.control_only
                && let Some(guard) = self
                    .control
                    .authority_prepared_guard
                    .get()
                    .or(self.control.authority_guard.as_ref())
            {
                let result = guard();
                let used = match cpu.elapsed() {
                    Ok(used) => used,
                    Err(code) => return self.finish_unit(cpu, Err(code)),
                };
                if result == Err(Code::Busy) && used <= UNIT {
                    // A shared prepared authority session may be owned by its
                    // monitor. Busy grants no work and is not stale authority.
                    self.finish_unit(cpu, Ok(()))?;
                    continue;
                }
                if result.is_err() || used > UNIT {
                    return self.finish_unit(cpu, result.and(Err(Code::ResourceLimit)));
                }
            }
            return Ok(cpu);
        }
    }
    #[track_caller]
    fn finish_unit<T: Drain>(
        &mut self,
        cpu: UnitClock,
        result: Result<T, Code>,
    ) -> Result<T, Code> {
        self.finish_unit_sampled(cpu, result, UnitClock::elapsed)
    }
    #[track_caller]
    fn finish_unit_sampled<T: Drain>(
        &mut self,
        cpu: UnitClock,
        result: Result<T, Code>,
        sample: impl FnOnce(UnitClock) -> Result<Duration, Code>,
    ) -> Result<T, Code> {
        let mut owner = Paced::new(result, self.control);
        let sampled = sample(cpu);
        #[cfg(debug_assertions)]
        if sampled.is_err() {
            eprintln!(
                "C1 clock unknown stage=finish_unit file={} line={}",
                std::panic::Location::caller().file(),
                std::panic::Location::caller().line()
            );
        }
        // Prefixes interrupted by required cleanup have already paid their
        // debt; charge only the resumed segment here (metrics keep full CPU).
        self.control.cleanup.debt.finish(if sampled.is_err() {
            Some(sampled)
        } else {
            None
        });
        let elapsed = sampled?;
        let end = Instant::now();
        // Test-only allocator observation follows the actual product CPU/end
        // capture; whole-owner clocks still include and label that probe cost.
        sample_isolated_heap();
        self.previous_end = Some(end);
        self.rest_until = end + REST;
        self.control.max_unit_cpu_ns.fetch_max(
            elapsed.as_nanos().min(u64::MAX as u128) as u64,
            Ordering::Relaxed,
        );
        if elapsed > UNIT {
            self.control.unit_overruns.fetch_add(1, Ordering::Relaxed);
            self.control.unit_overrun_line.store(
                std::panic::Location::caller().line() as u64,
                Ordering::Release,
            );
            #[cfg(debug_assertions)]
            eprintln!(
                "C1 unit overrun cpu_ns={} file={} line={}",
                elapsed.as_nanos(),
                std::panic::Location::caller().file(),
                std::panic::Location::caller().line()
            );
            return Err(Code::ResourceLimit);
        }
        if Instant::now() >= self.deadline {
            return Err(Code::Deadline);
        }
        if !self.control_only && self.control.cancelled.load(Ordering::Acquire) {
            return Err(Code::Cancelled);
        }
        owner.take_value()
    }
    #[track_caller]
    fn unit<T: Drain>(&mut self, work: impl FnOnce() -> Result<T, Code>) -> Result<T, Code> {
        let cpu = self.begin_unit()?;
        let result = work();
        self.finish_unit(cpu, result)
    }
    fn retain(&mut self, bytes: usize) -> Result<(), Code> {
        self.declared.as_mut().unwrap().grow(bytes)
    }
    fn hash(&mut self, bytes: &[u8]) -> Result<String, Code> {
        let mut hash = Sha256::new();
        self.hash_update(&mut hash, bytes)?;
        self.unit(|| Ok(format!("{:x}", hash.finalize())))
    }
    fn hash_update(&mut self, hash: &mut Sha256, bytes: &[u8]) -> Result<(), Code> {
        for chunk in bytes.chunks(HASH_CHUNK) {
            self.unit(|| {
                hash.update(chunk);
                Ok(())
            })?;
        }
        Ok(())
    }
    /// Check every4KiB, but fast parsing shares one measured unit. Fifteen
    /// checkpoints plus the scanner's at-most-six-byte escape span stay below
    ///64KiB; a0.5ms CPU checkpoint leaves the remaining1ms-unit margin intact.
    fn cooperate(&mut self, cpu: &mut UnitClock, checkpoints: &mut usize) -> Result<(), Code> {
        *checkpoints += 1;
        let used = cpu.elapsed()?;
        if used >= Duration::from_micros(500)
            || *checkpoints >= 15
            || self.control.cancelled.load(Ordering::Acquire)
            || self.control.is_foreground()
            || Instant::now() >= self.deadline
        {
            self.finish_unit(*cpu, Ok(()))?;
            *cpu = self.begin_unit()?;
            *checkpoints = 0;
        }
        Ok(())
    }
    fn json(&mut self, bytes: &[u8]) -> Result<Reservation, Code> {
        self.json_shape(bytes, transport::JsonShape::Any)
            .map(|(reservation, _)| reservation)
    }
    fn json_credit(&self, bounds: transport::JsonBounds) -> Result<Reservation, Code> {
        self.control.heap.reserve(
            HeapLane::Work,
            bounds.declared_payload_bytes().map_err(wire_error)?,
        )
    }
    /// The same scanner/owned decoder in one admitted unit for a bounded tiny
    /// record. Complex fallback carries its completed scan to the staged path.
    fn small_json(
        &mut self,
        bytes: &[u8],
        work: impl FnOnce(&Value) -> Result<(), Code>,
    ) -> Result<SmallJson, Code> {
        if bytes.len() > 8 * 1024 {
            return Ok(SmallJson::Fallback(None));
        }
        let control = self.control;
        let result = self.unit(|| {
            let bounds =
                transport::preflight_json(bytes, transport::JsonShape::Any).map_err(wire_error)?;
            if bounds.values.saturating_add(bounds.keys) > 128 {
                return Ok(TinyPrepared::Fallback(bounds));
            }
            let credit = control.heap.reserve(
                HeapLane::Work,
                bounds.declared_payload_bytes().map_err(wire_error)?,
            )?;
            let refusals = control.cleanup.capacity_refusals.load(Ordering::Acquire);
            let mut owner = Value::decode(std::io::Cursor::new(bytes), control).map_err(|_| {
                if control.cleanup.capacity_refusals.load(Ordering::Acquire) != refusals {
                    Code::ResourceLimit
                } else {
                    Code::ProtocolError
                }
            })?;
            owner.credit = Some(credit);
            let verdict = work(&owner);
            // Return the actual owner to finish_unit: it remains guarded through
            // CPU/end-clock/cancel/deadline verdicts, then retires outside the
            // completed work unit. Its required cleanup rests also satisfy the
            // work unit's next-admission rest, without installing another wait.
            Ok(TinyPrepared::Parsed { owner, verdict })
        });
        let result = match result {
            Ok(TinyPrepared::Fallback(bounds)) => Ok(SmallJson::Fallback(Some(bounds))),
            Ok(TinyPrepared::Parsed { owner, verdict }) => {
                drop(owner);
                verdict.map(|()| SmallJson::Processed)
            }
            Err(code) => Err(code),
        };
        if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
            Err(Code::ResourceLimit)
        } else {
            result
        }
    }
    fn json_shape(
        &mut self,
        bytes: &[u8],
        shape: transport::JsonShape,
    ) -> Result<(Reservation, transport::JsonBounds), Code> {
        let mut cpu = self.begin_unit()?;
        let mut checkpoints = 0;
        let mut stopped = None;
        let result = transport::preflight_json_cooperative(bytes, shape, || {
            match self.cooperate(&mut cpu, &mut checkpoints) {
                Ok(()) => Ok(()),
                Err(code) => {
                    stopped = Some(code);
                    Err(w::Code::ResourceLimit)
                }
            }
        });
        if let Some(code) = stopped {
            return Err(code);
        }
        let bounds = self.finish_unit(cpu, result.map_err(wire_error))?;
        let reservation = self.control.heap.reserve(
            HeapLane::Work,
            bounds.declared_payload_bytes().map_err(wire_error)?,
        )?;
        Ok((reservation, bounds))
    }
    #[cfg(test)]
    fn parse<T: OwnedDecode>(&mut self, bytes: &[u8]) -> Result<Paced<T>, Code> {
        let credit = self.json(bytes)?;
        self.parse_credited(bytes, credit)
    }
    fn parse_credited<T: OwnedDecode>(
        &mut self,
        bytes: &[u8],
        credit: Reservation,
    ) -> Result<Paced<T>, Code> {
        let cpu = self.begin_unit()?;
        struct Reader<'a, 'b> {
            bytes: &'a [u8],
            at: usize,
            next: usize,
            budget: &'a mut Budget<'b>,
            cpu: UnitClock,
            stopped: Option<Code>,
            checkpoints: usize,
            cleanup_tail: Option<UnitClock>,
        }
        impl Read for Reader<'_, '_> {
            #[inline]
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                if self.stopped.is_some() {
                    return Err(std::io::ErrorKind::PermissionDenied.into());
                }
                if out.is_empty() || self.at == self.bytes.len() {
                    return Ok(0);
                }
                if self.at >= self.next {
                    match self.budget.cooperate(&mut self.cpu, &mut self.checkpoints) {
                        Ok(()) => self.next = self.at.saturating_add(4096),
                        Err(code) => {
                            self.stopped = Some(code);
                            // The serde error walk is settlement. Every partial
                            // collection has its own paced owner; conservatively
                            // measure the remaining native/leaf unwind as well.
                            std::thread::sleep(REST);
                            self.cleanup_tail = UnitClock::now().ok();
                            return Err(std::io::ErrorKind::PermissionDenied.into());
                        }
                    }
                }
                out[0] = self.bytes[self.at];
                self.at += 1;
                Ok(1)
            }
        }
        let control = self.control;
        let capacity_refusals = control.cleanup.capacity_refusals.load(Ordering::Acquire);
        let mut reader = Reader {
            bytes,
            at: 0,
            next: 4096,
            budget: self,
            cpu,
            stopped: None,
            checkpoints: 0,
            cleanup_tail: None,
        };
        let mut result = T::decode(&mut reader, control);
        if let Ok(owner) = &mut result {
            owner.credit = Some(credit);
        }
        // On Err the separate credit still lives until this function returns,
        // after every partial visitor and native/leaf error owner was destroyed.
        if let Some(code) = reader.stopped {
            drop(result);
            let elapsed = reader
                .cleanup_tail
                .and_then(|clock| clock.elapsed().ok())
                .unwrap_or(UNIT + Duration::from_nanos(1));
            disposal::record_cleanup(&control.cleanup, elapsed);
            return Err(if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
                Code::ResourceLimit
            } else {
                code
            });
        }
        let result = result.map_err(|_| {
            if control.cleanup.capacity_refusals.load(Ordering::Acquire) != capacity_refusals {
                Code::ResourceLimit
            } else {
                Code::ProtocolError
            }
        });
        reader.budget.finish_unit(reader.cpu, result)
    }
    fn write_json<T: Serialize>(&mut self, value: &T, buffer: &mut HeapBuffer) -> Result<(), Code> {
        let cpu = self.begin_unit()?;
        struct Paced<'a, 'b> {
            buffer: &'a mut HeapBuffer,
            budget: &'a mut Budget<'b>,
            cpu: UnitClock,
            since: usize,
            stopped: Option<Code>,
            checkpoints: usize,
        }
        impl Write for Paced<'_, '_> {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                let mut at = 0;
                while at < bytes.len() {
                    if self.since >= 4096 {
                        match self.budget.cooperate(&mut self.cpu, &mut self.checkpoints) {
                            Ok(()) => {
                                self.since = 0;
                            }
                            Err(code) => {
                                self.stopped = Some(code);
                                return Err(std::io::ErrorKind::PermissionDenied.into());
                            }
                        }
                    }
                    let n = (bytes.len() - at).min(4096 - self.since);
                    self.buffer
                        .extend(&bytes[at..at + n])
                        .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
                    self.since += n;
                    at += n;
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut writer = Paced {
            buffer,
            budget: self,
            cpu,
            since: 0,
            stopped: None,
            checkpoints: 0,
        };
        let result = serde_json::to_writer(&mut writer, value).map_err(|_| Code::ResourceLimit);
        if let Some(code) = writer.stopped {
            return Err(code);
        }
        writer.budget.finish_unit(writer.cpu, result)
    }
    fn input(&mut self, bytes: u64) -> Result<(), Code> {
        self.input = self.input.checked_add(bytes).ok_or(Code::ResourceLimit)?;
        self.disk = self.disk.checked_add(bytes).ok_or(Code::ResourceLimit)?;
        if self.input > INPUT || self.disk > DISK {
            return Err(Code::ResourceLimit);
        }
        Ok(())
    }
}

fn immutable_source(bookmark: &Bookmark, m: &Metadata) -> bool {
    owned(m)
        && stamp(m)
            == (
                bookmark.device,
                bookmark.inode,
                bookmark.len,
                bookmark.mtime,
                bookmark.ctime,
            )
}
fn create(path: &Path) -> Result<File, Code> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| Code::StorageUnavailable)
}
#[cfg(test)]
fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, Code> {
    transport::canonical(value).map_err(|_| Code::ProtocolError)
}
/// Controlled raw capacities are fallible before serialization; the exact raw
/// structure is admitted/declared before constructing the canonical DOM.
fn encode_bounded<T: Serialize>(
    value: &T,
    budget: &mut Budget<'_>,
    bound: usize,
) -> Result<HeapBuffer, Code> {
    let mut raw = HeapBuffer::new(&budget.control.heap, HeapLane::Work, bound)?;
    budget.write_json(value, &mut raw)?;
    let payload = budget.json(&raw)?;
    let canonical: Paced<Value> = budget.parse_credited(&raw, payload)?;
    let mut encoded = HeapBuffer::new(&budget.control.heap, HeapLane::Work, bound)?;
    budget.write_json(&canonical, &mut encoded)?;
    drop(canonical);
    drop(raw);
    if budget.control.cleanup.overruns.load(Ordering::Acquire) > 0 {
        return Err(Code::ResourceLimit);
    }
    Ok(encoded)
}
fn encoded_fits(value: &impl Serialize, bound: usize) -> Result<(), Code> {
    struct Count {
        bytes: usize,
        bound: usize,
    }
    impl Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes = self
                .bytes
                .checked_add(bytes.len())
                .filter(|n| *n <= self.bound)
                .ok_or(std::io::ErrorKind::OutOfMemory)?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Count { bytes: 0, bound }, value).map_err(|_| Code::ResourceLimit)
}
fn mirror<T: DeserializeOwned>(value: &impl Serialize) -> Result<T, Code> {
    serde_json::from_value(serde_json::to_value(value).map_err(|_| Code::ProtocolError)?)
        .map_err(|_| Code::ProtocolError)
}

/// The public page is a fixed mirror, not a second serde tree. Each admitted
/// row's clone has one measured unit and a guarded completed-verdict owner.
fn mirror_page(value: &w::Page, budget: &mut Budget<'_>) -> Result<public::Page, Code> {
    let mut rows = Paced::new(Vec::new(), budget.control);
    for row in &value.rows {
        rows.push(budget.unit(|| {
            Ok(match row {
                w::Row::Cluster(row) => public::Row::Cluster(public::Cluster {
                    repo: public::Repo(row.repo.0.clone()),
                    fingerprint: public::Hex(row.fingerprint.0.clone()),
                    count: row.count,
                    first_ms: public::Number(row.first_ms.0),
                    last_ms: public::Number(row.last_ms.0),
                }),
                w::Row::Episode(row) => public::Row::Episode(public::Episode {
                    session: public::Ulid(row.session.0.clone()),
                    turn: public::Ulid(row.turn.0.clone()),
                    origin: public::Origin {
                        session: public::Ulid(row.origin.session.0.clone()),
                        seq: public::Number(row.origin.seq.0),
                        digest: row.origin.digest.clone(),
                    },
                    fingerprints: row
                        .fingerprints
                        .iter()
                        .map(|value| public::Hex(value.0.clone()))
                        .collect(),
                    first_ms: public::Number(row.first_ms.0),
                    last_ms: public::Number(row.last_ms.0),
                }),
            })
        })?);
    }
    Ok(public::Page {
        schema: public::PageSchema::V1,
        run_id: public::Ulid(value.run_id.0.clone()),
        manifest_sha256: public::Hex(value.manifest_sha256.0.clone()),
        index: value.index,
        rows: rows.take_value(),
        next: value.next,
    })
}

/// Adapted from bboxy450-harness src/dream/inspect.rs::blobs, not arbitrary JSON recursion.
/// https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/inspect.rs
/// Keep typed carrier coverage and the producer's independent inventory validation attached.
fn add_reference(value: &Value, out: &mut BTreeMap<String, w::Blob>) -> Result<(), Code> {
    if value.is_null() {
        return Ok(());
    }
    // Refuse before serde can clone an oversized scalar from the existing IR.
    if value.as_object().is_none_or(|v| v.len() != 3)
        || value["sha256"]
            .as_str()
            .is_none_or(|v| v.len() != 64 || !hex(v))
        || value["mime"]
            .as_str()
            .is_none_or(|v| v.is_empty() || v.len() > 256)
        || value["len"].as_u64().is_none_or(|v| v > w::MAX_SAFE)
    {
        return Err(Code::BlobMismatch);
    }
    let blob: w::Blob = serde::Deserialize::deserialize(value).map_err(|_| Code::BlobMismatch)?;
    if let Some(old) = out.get(&blob.sha256.0) {
        if old != &blob {
            return Err(Code::BlobMismatch);
        }
    } else {
        if out.len() >= 1024 {
            return Err(Code::ResourceLimit);
        }
        out.insert(blob.sha256.0.clone(), blob);
    }
    Ok(())
}

fn reference_blocks(
    value: &Value,
    visit: &mut impl FnMut(&Value, bool) -> Result<(), Code>,
) -> Result<(), Code> {
    if let Some(blocks) = value.as_array() {
        for block in blocks {
            // Even non-Blob blocks visit the bounded unit on the staged path;
            // a long text-only array cannot become an unmeasured traversal.
            visit(block, true)?;
        }
    }
    Ok(())
}

fn visit_references(
    event: &Value,
    mut visit: impl FnMut(&Value, bool) -> Result<(), Code>,
) -> Result<(), Code> {
    let body = match event["type"].as_str() {
        Some("item_completed") => &event["body"],
        Some("item_snapshot") => &event["partial"],
        _ => return Ok(()),
    };
    match body["kind"].as_str() {
        Some("user_message") => reference_blocks(&body["blocks"], &mut visit)?,
        Some("tool_call") => {
            visit(&body["spill"], false)?;
            reference_blocks(&body["output"], &mut visit)?;
        }
        Some("terminal") => visit(&body["output"], false)?,
        Some("image" | "native" | "receipt") => visit(&body["blob"], false)?,
        Some("compaction") => visit(&body["summary"], false)?,
        _ => {}
    }
    Ok(())
}

fn add_carrier(
    value: &Value,
    block: bool,
    out: &mut BTreeMap<String, w::Blob>,
) -> Result<(), Code> {
    if block {
        if matches!(value["type"].as_str(), Some("image" | "audio" | "resource")) {
            add_reference(&value["blob"], out)
        } else {
            Ok(())
        }
    } else {
        add_reference(value, out)
    }
}

fn references(
    event: &Value,
    out: &mut BTreeMap<String, w::Blob>,
    budget: &mut Budget<'_>,
) -> Result<(), Code> {
    visit_references(event, |value, block| {
        budget.unit(|| add_carrier(value, block, out))
    })
}

fn observe_source_record(event: &Value, bookmark: &Bookmark, seq: &mut u64) -> Result<bool, Code> {
    *seq += 1;
    if event["session"].as_str() != Some(&bookmark.session.0)
        || event["seq"].as_u64() != Some(*seq)
        || event["at"].as_u64() != Some(*seq)
    {
        return Err(Code::MalformedIr);
    }
    if event["v"]["major"] != 1 || event["v"]["minor"].as_u64().is_none_or(|v| v > 3) {
        return Err(Code::UnsupportedIr);
    }
    if *seq == 1 && !event["event"]["meta"]["fork"].is_null() {
        return Err(Code::UnsupportedIr);
    }
    if *seq == 1
        && (event["event"]["type"] != "session_started"
            || event["event"]["meta"]["vcs"]["repo"].as_str() != Some(&bookmark.repo)
            || !event["event"]["meta"]["source"].is_null())
    {
        return Err(Code::SourceMismatch);
    }
    Ok(event["event"]["type"] == "session_closed")
}

struct Snapshot {
    manifest: Paced<w::Manifest>,
    hash: String,
    _retained: Reservation,
}
fn snapshot(
    job: &Path,
    record: &Record,
    control: &Control,
    deadline: Instant,
) -> Result<Snapshot, Code> {
    utility()?;
    // A retained binding is never overwritten. The actual guarded open below
    // refuses a mismatched owner before any source filesystem effect.
    let _ = control.source_owner.set(std::thread::current().id());
    let _clock = OwnerClock::new(
        &control.snapshot_active_cpu_ns,
        &control.snapshot_active_wall_ns,
    )?;
    let mut budget = Budget::new(control, deadline);
    let input = job.join("input");
    budget.unit(|| private_dir(&input))?;
    budget.retain(
        record
            .sources
            .len()
            .checked_mul(std::mem::size_of::<w::Source>())
            .ok_or(Code::ResourceLimit)?,
    )?;
    let mut sources = Paced::new(Vec::new(), control);
    sources
        .try_reserve_exact(record.sources.len())
        .map_err(|_| Code::ResourceLimit)?;
    let mut distinct = Paced::new(BTreeSet::new(), control);
    let _distinct_payload = control
        .heap
        .reserve(HeapLane::Work, 1024 * (std::mem::size_of::<String>() + 64))?;
    for bookmark in &record.sources {
        budget.input(bookmark.len)?;
        let mut file = budget.unit(|| guarded_source_open(control, &bookmark.path))?;
        let before = budget.unit(|| file.metadata().map_err(|_| Code::SourceUnavailable))?;
        if !immutable_source(bookmark, &before) {
            return Err(Code::SourceChanged);
        }
        #[cfg(debug_assertions)]
        if control.private_fixture {
            hold_private_source(job, control)?;
        }
        let target = input.join(format!("{}.jsonl", bookmark.session.0));
        let mut output = budget.unit(|| create(&target))?;
        let mut hash = Sha256::new();
        let mut pending = HeapBuffer::new(&control.heap, HeapLane::Work, 0)?;
        // Exact declared key/value/maximum-string payload for this bounded inventory.
        // BTree node slack belongs to the separately measured helper headroom.
        let _blob_payload = control.heap.reserve(
            HeapLane::Work,
            // One current decoded candidate overlaps the1024 retained entries.
            1025 * (std::mem::size_of::<String>() + std::mem::size_of::<w::Blob>() + 64 + 64 + 256),
        )?;
        let mut blobs = Paced::new(BTreeMap::new(), control);
        let mut seq = 0u64;
        let mut closed = false;
        let mut last_hash = String::new();
        let mut read = 0u64;
        loop {
            let mut chunk = [0u8; CHUNK];
            let n = copy_chunk(
                &mut file,
                &mut output,
                &mut hash,
                &mut chunk,
                bookmark.len.saturating_sub(read),
                Code::SourceUnavailable,
                Code::SourceChanged,
                &control.source_copy_unit_max_cpu_ns,
                &mut budget,
            )?;
            if n == 0 {
                break;
            }
            read = read.checked_add(n as u64).ok_or(Code::ResourceLimit)?;
            if read > bookmark.len {
                return Err(Code::SourceChanged);
            }
            if pending.len() + n > LINE + CHUNK {
                return Err(Code::ResourceLimit);
            }
            pending.ensure(pending.len() + n, LINE + CHUNK)?;
            pending.extend(&chunk[..n])?;
            while let Some(end) = line_end(&pending) {
                if end + 1 > LINE || closed {
                    return Err(Code::MalformedIr);
                }
                budget.envelopes += 1;
                if budget.envelopes > 65_536 {
                    return Err(Code::ResourceLimit);
                }
                // Adapted from bboxy450-harness src/dream/c1.rs,47182320:
                // https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/c1.rs
                // The same<=8KiB/<=128values+keys frontier combines only tiny
                // records. Consumer preflight credits/owned decode/retirement
                // and the exact raw terminal-line hash remain authoritative.
                let raw = &pending[..=end];
                let small = budget
                    .small_json(raw, |event| {
                        closed = observe_source_record(event, bookmark, &mut seq)?;
                        visit_references(&event["event"], |value, block| {
                            add_carrier(value, block, &mut blobs)
                        })?;
                        if closed {
                            last_hash = format!("{:x}", Sha256::digest(&raw[..raw.len() - 1]));
                        }
                        Ok(())
                    })
                    .map_err(|code| {
                        if code == Code::ProtocolError {
                            Code::MalformedIr
                        } else {
                            code
                        }
                    })?;
                let payload = match small {
                    SmallJson::Processed => {
                        control
                            .snapshot_tiny_records
                            .fetch_add(1, Ordering::Relaxed);
                        pending.consume(end + 1)?;
                        continue;
                    }
                    SmallJson::Fallback(Some(bounds)) => budget.json_credit(bounds)?,
                    SmallJson::Fallback(None) => budget.json(raw).map_err(|code| {
                        if code == Code::ProtocolError {
                            Code::MalformedIr
                        } else {
                            code
                        }
                    })?,
                };
                let mut raw = HeapBuffer::new(&control.heap, HeapLane::Work, end + 1)?;
                raw.extend(&pending[..=end])?;
                pending.consume(end + 1)?;
                let event: Paced<Value> = budget.parse_credited(&raw, payload).map_err(|code| {
                    if code == Code::ProtocolError {
                        Code::MalformedIr
                    } else {
                        code
                    }
                })?;
                closed = budget.unit(|| observe_source_record(&event, bookmark, &mut seq))?;
                references(&event["event"], &mut blobs, &mut budget)?;
                if closed {
                    last_hash = budget.hash(&raw[..raw.len() - 1])?;
                }
                control
                    .snapshot_staged_records
                    .fetch_add(1, Ordering::Relaxed);
            }
            if pending.len() >= LINE {
                return Err(Code::ResourceLimit);
            }
        }
        if !pending.is_empty()
            || read != bookmark.len
            || !closed
            || seq != bookmark.closed_seq
            || last_hash != bookmark.closed_line_sha256
        {
            return Err(Code::SourceChanged);
        }
        budget.unit(|| output.sync_all().map_err(|_| Code::StorageUnavailable))?;
        let after = budget.unit(|| file.metadata().map_err(|_| Code::SourceChanged))?;
        let path = budget
            .unit(|| std::fs::symlink_metadata(&bookmark.path).map_err(|_| Code::SourceChanged))?;
        if !immutable_source(bookmark, &after) || !immutable_source(bookmark, &path) {
            return Err(Code::SourceChanged);
        }
        let source_hash = budget.unit(|| Ok(format!("{:x}", hash.finalize())))?;
        let retained_blobs = blobs.values().try_fold(0usize, |sum, b| {
            sum.checked_add(std::mem::size_of::<w::Blob>() + b.sha256.0.len() + b.mime.0.len())
                .ok_or(Code::ResourceLimit)
        })?;
        budget.retain(retained_blobs + bookmark.repo.len() + bookmark.session.0.len() + 64 + 64)?;
        let mut inventory = Paced::new(Vec::new(), control);
        inventory
            .try_reserve_exact(blobs.len())
            .map_err(|_| Code::ResourceLimit)?;
        inventory.extend(blobs.take_value().into_values());
        if !inventory.is_empty() {
            let dir = input.join(&bookmark.session.0);
            budget.unit(|| private_dir(&dir))?;
            let dir = dir.join("blobs");
            budget.unit(|| private_dir(&dir))?;
            for blob in inventory.iter() {
                if !distinct.contains(&blob.sha256.0) {
                    if distinct.len() >= 1024 {
                        return Err(Code::ResourceLimit);
                    }
                    distinct.insert(blob.sha256.0.clone());
                }
                budget.input(blob.len.0)?;
                let source = bookmark
                    .path
                    .parent()
                    .ok_or(Code::SourceUnavailable)?
                    .join(&bookmark.session.0)
                    .join("blobs")
                    .join(&blob.sha256.0);
                copy_blob(&source, &dir.join(&blob.sha256.0), blob, &mut budget)?;
            }
        }
        sources.push(w::Source {
            session: w::Ulid(bookmark.session.0.clone()),
            source_generation: w::Number(bookmark.generation),
            repo: w::Repo(bookmark.repo.clone()),
            len: w::Number(bookmark.len),
            sha256: w::Hex(source_hash),
            closed_seq: w::Number(bookmark.closed_seq),
            closed_line_sha256: w::Hex(bookmark.closed_line_sha256.clone()),
            blobs: inventory.take_value(),
        });
    }
    let manifest = Paced::new(
        w::Manifest {
            schema: w::ManifestSchema::V1,
            run_id: w::Ulid(record.view.run_id.0.clone()),
            nonce: w::Ulid(record.nonce.0.clone()),
            context: mirror(&record.context)?,
            trigger: if record.trigger {
                w::Trigger::SessionClosed
            } else {
                w::Trigger::Manual
            },
            algorithm: w::Algorithm::D0C1,
            sources: sources.take_value(),
        },
        control,
    );
    let bytes = encode_bounded(&manifest, &mut budget, LINE)?;
    let hash = budget.hash(&bytes)?;
    budget.disk += bytes.len() as u64;
    if budget.disk > DISK {
        return Err(Code::ResourceLimit);
    }
    publish_paced(
        &input,
        "manifest.json",
        &record.nonce.0,
        &bytes,
        &mut budget,
    )?;
    Ok(Snapshot {
        manifest,
        hash,
        _retained: budget.declared.take().unwrap(),
    })
}

fn copy_blob(
    source: &Path,
    target: &Path,
    blob: &w::Blob,
    budget: &mut Budget<'_>,
) -> Result<(), Code> {
    let control = budget.control;
    let mut file =
        budget.unit(|| guarded_source_open(control, source).map_err(|_| Code::BlobMismatch))?;
    let before = budget.unit(|| file.metadata().map_err(|_| Code::BlobMismatch))?;
    if before.len() != blob.len.0 {
        return Err(Code::BlobMismatch);
    }
    let mut out = budget.unit(|| create(target))?;
    let mut hash = Sha256::new();
    let mut count = 0u64;
    loop {
        let mut chunk = [0u8; CHUNK];
        let n = copy_chunk(
            &mut file,
            &mut out,
            &mut hash,
            &mut chunk,
            blob.len.0.saturating_sub(count),
            Code::BlobMismatch,
            Code::BlobMismatch,
            &control.blob_copy_unit_max_cpu_ns,
            budget,
        )?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > blob.len.0 {
            return Err(Code::BlobMismatch);
        }
    }
    budget.unit(|| out.sync_all().map_err(|_| Code::StorageUnavailable))?;
    let after = budget.unit(|| file.metadata().map_err(|_| Code::BlobMismatch))?;
    let path = budget.unit(|| std::fs::symlink_metadata(source).map_err(|_| Code::BlobMismatch))?;
    let actual = budget.unit(|| Ok(format!("{:x}", hash.finalize())))?;
    if count != blob.len.0
        || actual != blob.sha256.0
        || stamp(&before) != stamp(&after)
        || stamp(&before) != stamp(&path)
    {
        return Err(Code::BlobMismatch);
    }
    Ok(())
}

// One immutable transfer touches at most64KiB: read, exact-length fence, hash,
// write. A foreground/cancel observation between those operations prevents the
// next effect. Pausing retains the already-read chunk and hash phase; resuming
// never rereads or rehashes it. Every retry is a newly guarded/rested unit.
#[allow(clippy::too_many_arguments)]
fn copy_chunk(
    input: &mut File,
    output: &mut File,
    hash: &mut Sha256,
    chunk: &mut [u8; CHUNK],
    remaining: u64,
    read_error: Code,
    length_error: Code,
    maximum: &AtomicU64,
    budget: &mut Budget<'_>,
) -> Result<usize, Code> {
    let control = budget.control;
    let mut count = None;
    let mut hashed = false;
    let mut written = false;
    while !written {
        budget.unit(|| {
            let cpu = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?;
            let result = (|| {
                if control.cancelled.load(Ordering::Acquire) {
                    return Err(Code::Cancelled);
                }
                if control.is_foreground() {
                    return Ok(());
                }
                let n = match count {
                    Some(n) => n,
                    None => {
                        let n = input.read(chunk).map_err(|_| read_error)?;
                        if n as u64 > remaining {
                            return Err(length_error);
                        }
                        count = Some(n);
                        n
                    }
                };
                if n == 0 {
                    written = true;
                    return Ok(());
                }
                if control.cancelled.load(Ordering::Acquire) {
                    return Err(Code::Cancelled);
                }
                if control.is_foreground() {
                    return Ok(());
                }
                if !hashed {
                    hash.update(&chunk[..n]);
                    hashed = true;
                }
                if control.cancelled.load(Ordering::Acquire) {
                    return Err(Code::Cancelled);
                }
                if control.is_foreground() {
                    return Ok(());
                }
                output
                    .write_all(&chunk[..n])
                    .map_err(|_| Code::StorageUnavailable)?;
                written = true;
                Ok(())
            })();
            let used = bbt_rt::fault::thread_cpu_time()
                .map_err(|_| Code::ResourceLimit)?
                .saturating_sub(cpu);
            maximum.fetch_max(nanos(used), Ordering::Relaxed);
            result
        })?;
    }
    count.ok_or(Code::SourceUnavailable)
}

#[derive(Serialize)]
struct BirthBinding<'a> {
    profile: &'static str,
    purpose: &'static str,
    probe: bool,
    op_id: &'a public::Ulid,
    run_id: &'a public::Ulid,
    nonce: &'a public::Ulid,
    incarnation: u64,
    context: Option<&'a public::Context>,
    worker_sha256: &'a str,
}
fn birth_binding(binding: &BirthBinding<'_>, budget: &mut Budget<'_>) -> Result<[u8; 32], Code> {
    if binding.profile != bootstrap::PROFILE
        || !matches!(binding.purpose, "inspect-c1" | "read-prepare")
        || binding.probe != binding.context.is_none()
        || binding.incarnation == 0
        || binding.worker_sha256.len() != 64
        || !binding
            .worker_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Code::BadRequest);
    }
    // Canonical input borrows the retained public context; no provider/source
    // discovery or cloning a transcript is part of launch binding.
    let bytes = encode_bounded(binding, budget, 4096)?;
    let mut hash = Sha256::new();
    budget.hash_update(&mut hash, &bytes)?;
    let digest = budget.unit(|| Ok(<[u8; 32]>::from(hash.finalize())))?;
    drop(bytes);
    Ok(digest)
}
fn bootstrap_control<T: Copy>(
    control: &Control,
    work: impl FnOnce() -> Result<T, Code>,
) -> Result<T, Code> {
    bootstrap_control_sampled(control, work, None)
}
fn bootstrap_control_sampled<T: Copy>(
    control: &Control,
    work: impl FnOnce() -> Result<T, Code>,
    sample: Option<Result<Duration, Code>>,
) -> Result<T, Code> {
    let clock_failure = || {
        control
            .bootstrap_clock_failures
            .fetch_add(1, Ordering::Relaxed);
        control.cleanup.debt.poisoned.store(true, Ordering::Release);
        Code::ResourceLimit
    };
    let before = bbt_rt::fault::thread_cpu_time().map_err(|_| clock_failure())?;
    // Only fixed finite control results ((),bool,scalars) cross this helper.
    // Worker/Gate retain every pipe/effect/attempt before its clock verdict;
    // no bulk owner cleanup is admitted during child bootstrap ownership.
    let result = work();
    let measured = bbt_rt::fault::thread_cpu_time()
        .map_err(|_| Code::ResourceLimit)
        .and_then(|now| now.checked_sub(before).ok_or(Code::ResourceLimit));
    let used = sample.unwrap_or(measured).map_err(|_| clock_failure())?;
    control
        .bootstrap_control_cpu_ns
        .fetch_add(nanos(used), Ordering::Relaxed);
    control
        .bootstrap_control_max_cpu_ns
        .fetch_max(nanos(used), Ordering::Relaxed);
    control
        .max_unit_cpu_ns
        .fetch_max(nanos(used), Ordering::Relaxed);
    if used > UNIT {
        control
            .bootstrap_control_overruns
            .fetch_add(1, Ordering::Relaxed);
        control.unit_overruns.fetch_add(1, Ordering::Relaxed);
        return Err(Code::ResourceLimit);
    }
    result
}

fn final_admission_control<T: Copy>(
    control: &Control,
    handoff: FinalHandoff,
    work: impl FnOnce() -> Result<T, Code>,
) -> Result<T, Code> {
    let attempts = match handoff {
        FinalHandoff::Permit => &control.bootstrap_permit_attempts,
        FinalHandoff::Start => &control.start_admission_attempts,
    };
    if attempts
        .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
            (n == 0).then_some(1)
        })
        .is_err()
    {
        return Err(Code::ProtocolError);
    }
    #[cfg(test)]
    let hook = {
        control
            .final_admission_hook
            .try_lock()
            .map_err(|_| Code::Busy)?
            .take()
    };
    #[cfg(test)]
    if let Some(hook) = hook {
        hook(handoff);
    }
    #[cfg(test)]
    let sample = control
        .final_admission_sample
        .try_lock()
        .map_err(|_| Code::Busy)?
        .take();
    #[cfg(not(test))]
    let sample = None;
    bootstrap_control_sampled(control, work, sample)
}

/// Fixed private capability history for this Runtime incarnation. It never
/// carries authority or source/results and never becomes a public wire DTO.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct ProbeAudit {
    pub stage: &'static str,
    pub error: Option<Code>,
    pub cleanup_error: Option<Code>,
    pub op: [u8; 16],
    pub run: [u8; 16],
    pub nonce: [u8; 16],
    pub birth: [u8; 16],
    pub incarnation: u64,
    pub binding: [u8; 32],
    pub worker_sha256: [u8; 32],
    pub pipe: (u32, u64, u64),
    pub pid: u32,
    pub native_effect: u8,
    pub native_errno: Option<i32>,
    pub claim_identity: Option<(u64, u64)>,
    #[serde(serialize_with = "serialize_probe_stamp")]
    pub process_start: [u8; 64],
    pub process_start_len: u8,
    pub known_birth: bool,
    pub unknown_birth: bool,
    pub qualified_hello: bool,
    pub entry_evidence: Option<transport::BootstrapEntryEvidence>,
    pub actual_reaped: bool,
    pub stdout_stderr_eof: bool,
    pub exit_status: Option<i32>,
    pub claim_released: bool,
    pub empty_directories_removed: bool,
    pub parent_cost_known: bool,
    pub parent_unit_max_cpu_ns: u64,
    pub parent_cleanup_max_cpu_ns: u64,
    pub native_process_max_cpu_ns: u64,
    pub native_process_max_wall_ns: u64,
    pub final_control_max_cpu_ns: u64,
    pub parent_remaining_debt_ns: u64,
    pub child_cost_known: bool,
    pub child_terminal_max_cpu_ns: Option<u64>,
    pub child_remaining_debt_ns: Option<u64>,
}
fn serialize_probe_stamp<S: serde::Serializer>(
    value: &[u8; 64],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    value.as_slice().serialize(serializer)
}
const _: () = assert!(std::mem::size_of::<ProbeAudit>() <= 1024);
impl ProbeAudit {
    pub(crate) fn empty(incarnation: u64) -> Self {
        Self {
            stage: "prepare",
            error: None,
            cleanup_error: None,
            op: [0; 16],
            run: [0; 16],
            nonce: [0; 16],
            birth: [0; 16],
            incarnation,
            binding: [0; 32],
            worker_sha256: [0; 32],
            pipe: (0, 0, 0),
            pid: 0,
            native_effect: 0,
            native_errno: None,
            claim_identity: None,
            process_start: [0; 64],
            process_start_len: 0,
            known_birth: false,
            unknown_birth: false,
            qualified_hello: false,
            entry_evidence: None,
            actual_reaped: false,
            stdout_stderr_eof: false,
            exit_status: None,
            claim_released: false,
            empty_directories_removed: false,
            parent_cost_known: false,
            parent_unit_max_cpu_ns: 0,
            parent_cleanup_max_cpu_ns: 0,
            native_process_max_cpu_ns: 0,
            native_process_max_wall_ns: 0,
            final_control_max_cpu_ns: 0,
            parent_remaining_debt_ns: 0,
            child_cost_known: false,
            child_terminal_max_cpu_ns: None,
            child_remaining_debt_ns: None,
        }
    }
    fn process_stamp(&mut self, stamp: &str) -> Result<(), Code> {
        if stamp.is_empty() || stamp.len() > self.process_start.len() || !stamp.is_ascii() {
            return Err(Code::ResourceLimit);
        }
        self.process_start[..stamp.len()].copy_from_slice(stamp.as_bytes());
        self.process_start_len = stamp.len() as u8;
        Ok(())
    }
    fn observe_reap(&mut self, worker: &mut Worker) -> Result<(), Code> {
        use std::os::unix::process::ExitStatusExt;
        let exit = worker.try_wait().map_err(wire_error)?;
        self.actual_reaped = exit.is_some();
        self.stdout_stderr_eof = worker.is_eof();
        self.exit_status = exit.map(ExitStatusExt::into_raw);
        self.qualified_hello = worker.bootstrap_observation().qualified_hello;
        self.entry_evidence = Some(worker.bootstrap_entry_evidence());
        Ok(())
    }
    fn capture_cost(&mut self, control: &Control) {
        let debt = &control.cleanup.debt;
        self.unknown_birth = control.native_birth_unknown.load(Ordering::Acquire);
        self.native_effect = debt.process_effect.load(Ordering::Acquire);
        let errno = debt.process_errno.load(Ordering::Acquire);
        self.native_errno = (errno != 0).then(|| i32::try_from(errno).ok()).flatten();
        self.parent_cost_known = !debt.poisoned.load(Ordering::Acquire)
            && debt.native_clock_failures.load(Ordering::Acquire) == 0
            && control.bootstrap_clock_failures.load(Ordering::Acquire) == 0;
        self.parent_unit_max_cpu_ns = control.max_unit_cpu_ns.load(Ordering::Acquire);
        self.parent_cleanup_max_cpu_ns = control.cleanup.max_unit_cpu_ns.load(Ordering::Acquire);
        self.native_process_max_cpu_ns = debt.native_process_max_cpu_ns.load(Ordering::Acquire);
        self.native_process_max_wall_ns = debt.native_process_max_wall_ns.load(Ordering::Acquire);
        self.final_control_max_cpu_ns =
            control.bootstrap_control_max_cpu_ns.load(Ordering::Acquire);
        self.parent_remaining_debt_ns = debt.remaining_ns();
    }
}
pub(crate) struct ProbeOutcome {
    pub audit: ProbeAudit,
    pub verdict: Result<transport::DreamOwner, Code>,
}
impl ProbeOutcome {
    pub(crate) fn refused(stage: &'static str, code: Code) -> Self {
        let mut audit = ProbeAudit::empty(0);
        audit.stage = stage;
        audit.error = Some(code);
        Self {
            audit,
            verdict: Err(code),
        }
    }
}
/// Only exact directories created by this probe are eligible for empty rmdir.
/// A missing inode, replacement or nonempty entry retains the claim and paths.
struct ProbeDirectories {
    job: PathBuf,
    temporary: PathBuf,
    job_created: bool,
    temporary_created: bool,
    job_identity: Option<(u64, u64)>,
    temporary_identity: Option<(u64, u64)>,
}
impl ProbeDirectories {
    fn new(root: &Path, run: crate::Ulid) -> Result<Self, Code> {
        use std::os::unix::ffi::OsStrExt;
        if root.as_os_str().as_bytes().len() + 32 >= libc::PATH_MAX as usize {
            return Err(Code::ResourceLimit);
        }
        let job = root.join(run.to_string());
        let temporary = job.join("tmp");
        Ok(Self {
            job,
            temporary,
            job_created: false,
            temporary_created: false,
            job_identity: None,
            temporary_identity: None,
        })
    }
    fn create_one(
        path: &Path,
        created: &mut bool,
        identity: &mut Option<(u64, u64)>,
    ) -> Result<(), Code> {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(path)
            .map_err(|_| Code::StorageUnavailable)?;
        *created = true; // Retain the actual effect before metadata/timing errors.
        let metadata = std::fs::symlink_metadata(path).map_err(|_| Code::StorageUnavailable)?;
        if !metadata.is_dir()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o777 != 0o700
        {
            return Err(Code::StorageUnavailable);
        }
        *identity = Some((metadata.dev(), metadata.ino()));
        Ok(())
    }
    fn create_job(&mut self) -> Result<(), Code> {
        Self::create_one(&self.job, &mut self.job_created, &mut self.job_identity)
    }
    fn create_temporary(&mut self) -> Result<(), Code> {
        Self::create_one(
            &self.temporary,
            &mut self.temporary_created,
            &mut self.temporary_identity,
        )
    }
    fn remove_one(
        path: &Path,
        created: &mut bool,
        identity: Option<(u64, u64)>,
    ) -> Result<(), Code> {
        if !*created {
            return Ok(());
        }
        let expected = identity.ok_or(Code::StorageUnavailable)?;
        let metadata = std::fs::symlink_metadata(path).map_err(|_| Code::StorageUnavailable)?;
        if !metadata.is_dir()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o777 != 0o700
            || (metadata.dev(), metadata.ino()) != expected
        {
            return Err(Code::StorageUnavailable);
        }
        // rmdir itself proves empty; no directory scan or recursive deletion.
        std::fs::remove_dir(path).map_err(|_| Code::StorageUnavailable)?;
        *created = false;
        Ok(())
    }
    fn remove_temporary(&mut self) -> Result<(), Code> {
        Self::remove_one(
            &self.temporary,
            &mut self.temporary_created,
            self.temporary_identity,
        )
    }
    fn remove_job(&mut self) -> Result<(), Code> {
        Self::remove_one(&self.job, &mut self.job_created, self.job_identity)
    }
}
impl Drain for ProbeDirectories {
    fn drain_step(&mut self) -> bool {
        if !self.temporary.as_os_str().is_empty() {
            drop(std::mem::take(&mut self.temporary));
            return true;
        }
        if !self.job.as_os_str().is_empty() {
            drop(std::mem::take(&mut self.job));
            return true;
        }
        false
    }
}
fn sync_probe_root(root: &Path, budget: &mut Budget<'_>) -> Result<(), Code> {
    let directory = Paced::new(
        budget.unit(|| {
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(root)
                .map_err(|_| Code::StorageUnavailable)
        })?,
        budget.control,
    );
    budget.unit(|| directory.sync_all().map_err(|_| Code::StorageUnavailable))
}

struct ProbeClaim {
    path: PathBuf,
    file: File,
    identity: (u64, u64),
    token: [u8; 16],
}
impl ProbeClaim {
    fn create(root: &Path, token: [u8; 16]) -> Result<Self, Code> {
        let path = root.join("bootstrap-probe.claim");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|_| Code::RunnerUnavailable)?;
        file.write_all(&token)
            .map_err(|_| Code::StorageUnavailable)?;
        let metadata = file.metadata().map_err(|_| Code::StorageUnavailable)?;
        Ok(Self {
            path,
            file,
            identity: (metadata.dev(), metadata.ino()),
            token,
        })
    }
    fn release(&mut self) -> Result<(), Code> {
        let descriptor = self.file.metadata().map_err(|_| Code::StorageUnavailable)?;
        let named = std::fs::symlink_metadata(&self.path).map_err(|_| Code::StorageUnavailable)?;
        if !named.is_file()
            || named.nlink() != 1
            || named.len() != 16
            || descriptor.len() != 16
            || named.mode() & 0o777 != 0o600
            || named.uid() != unsafe { libc::geteuid() }
            || (descriptor.dev(), descriptor.ino()) != self.identity
            || (named.dev(), named.ino()) != self.identity
        {
            return Err(Code::StorageUnavailable);
        }
        let mut token = [0; 16];
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|_| Code::StorageUnavailable)?;
        self.file
            .read_exact(&mut token)
            .map_err(|_| Code::StorageUnavailable)?;
        if token != self.token {
            return Err(Code::StorageUnavailable);
        }
        std::fs::remove_file(&self.path).map_err(|_| Code::StorageUnavailable)
    }
}
/// The account capability probe uses the SAME gated birth/debt/Hello controller
/// as real jobs. Null context is explicit65255; it can never produce a Start.
/// A durable single owned claim prevents overlapping/unknown predecessor probes.
const STOPPED_ENTRY_WORKER_SHA256: &str =
    "12ba4006b694bdce3c44d199de460d09f0f2414fafb7fd17880921a7d397a3b4";
fn entry_worker_eligible(proof: &runner_provision::ProvisionProof) -> Result<(), Code> {
    // Historical approval is not permission to relaunch the stopped tuple.
    // No hashing, substitute worker, new tuple approval or readiness retry.
    if proof.worker_sha256() == STOPPED_ENTRY_WORKER_SHA256 {
        Err(Code::RunnerUnavailable)
    } else {
        Ok(())
    }
}

pub(crate) fn probe_gated(
    root: &Path,
    proof: Arc<runner_provision::ProvisionProof>,
    incarnation: u64,
) -> ProbeOutcome {
    let mut audit = ProbeAudit::empty(incarnation);
    let verdict = probe_gated_core(root, proof, incarnation, &mut audit, |_, _| Ok(()));
    audit.error = verdict.as_ref().err().copied();
    ProbeOutcome { audit, verdict }
}
#[cfg(test)]
fn probe_gated_observe(
    root: &Path,
    proof: Arc<runner_provision::ProvisionProof>,
    incarnation: u64,
    observe: impl FnOnce(&mut Worker, &Control) -> Result<(), Code>,
) -> Result<transport::DreamOwner, Code> {
    probe_gated_core(
        root,
        proof,
        incarnation,
        &mut ProbeAudit::empty(incarnation),
        observe,
    )
}
fn probe_gated_core(
    root: &Path,
    proof: Arc<runner_provision::ProvisionProof>,
    incarnation: u64,
    audit: &mut ProbeAudit,
    observe: impl FnOnce(&mut Worker, &Control) -> Result<(), Code>,
) -> Result<transport::DreamOwner, Code> {
    let control = Control::default();
    let result = (|| {
        if incarnation == 0 {
            return Err(Code::RunnerUnavailable);
        }
        control.foreground.store(false, Ordering::Release);
        control
            .launch_incarnation
            .store(incarnation, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut budget = Budget::new(&control, deadline);
        audit.stage = "entry_tuple_eligibility";
        budget.unit(|| entry_worker_eligible(&proof))?;
        audit.stage = "provision_bind";
        budget.unit(|| control.install_native_provision(proof.clone()))?;
        let (op, run, nonce, birth) = budget.unit(|| {
            Ok((
                crate::Ulid::new(),
                crate::Ulid::new(),
                crate::Ulid::new(),
                crate::Ulid::new(),
            ))
        })?;
        budget.unit(|| {
            audit.op = op.to_bytes();
            audit.run = run.to_bytes();
            audit.nonce = nonce.to_bytes();
            audit.birth = birth.to_bytes();
            let hash = proof.worker_sha256().as_bytes();
            if hash.len() != 64 {
                return Err(Code::RunnerUnavailable);
            }
            for (i, pair) in hash.as_chunks::<2>().0.iter().enumerate() {
                let digit = |b: u8| match b {
                    b'0'..=b'9' => Some(b - b'0'),
                    b'a'..=b'f' => Some(b - b'a' + 10),
                    _ => None,
                };
                audit.worker_sha256[i] = (digit(pair[0]).ok_or(Code::RunnerUnavailable)? << 4)
                    | digit(pair[1]).ok_or(Code::RunnerUnavailable)?;
            }
            Ok(())
        })?;
        audit.stage = "claim";
        budget.unit(|| private_dir(root))?;
        let mut claim = Paced::new(
            budget.unit(|| ProbeClaim::create(root, birth.to_bytes()))?,
            &control,
        );
        audit.claim_identity = Some(claim.identity);
        // Persist claim custody before birth. Unknown native effects leave this
        // exact inode/token in place rather than creating a replacement probe.
        budget.unit(|| claim.file.sync_all().map_err(|_| Code::StorageUnavailable))?;
        sync_probe_root(root, &mut budget)?;
        let mut directories =
            Paced::new(budget.unit(|| ProbeDirectories::new(root, run))?, &control);
        let mut born_and_reaped = false;
        let result = (|| {
            audit.stage = "directories";
            budget.unit(|| directories.create_job())?;
            let (job, directory) = canonical_job(&directories.job, &mut budget)?;
            let _directory = Paced::new(directory, &control);
            budget.unit(|| directories.create_temporary())?;
            audit.stage = "binding";
            let op = public::Ulid(op.to_string());
            let run = public::Ulid(run.to_string());
            let nonce = public::Ulid(nonce.to_string());
            let binding = birth_binding(
                &BirthBinding {
                    profile: bootstrap::PROFILE,
                    purpose: "inspect-c1",
                    probe: true,
                    op_id: &op,
                    run_id: &run,
                    nonce: &nonce,
                    incarnation,
                    context: None,
                    worker_sha256: proof.worker_sha256(),
                },
                &mut budget,
            )?;
            audit.binding = binding;
            audit.stage = "gate";
            let frozen = bootstrap::Frozen::inspect(
                true,
                birth.to_bytes(),
                bootstrap::ulid(run.0.as_bytes()).map_err(|_| Code::BadRequest)?,
                bootstrap::ulid(nonce.0.as_bytes()).map_err(|_| Code::BadRequest)?,
                binding,
                incarnation,
            );
            let mut gate = Paced::new(
                budget.unit(|| bootstrap::Gate::prepare(frozen).map_err(wire_error))?,
                &control,
            );
            audit.pipe = gate.identity();
            audit.stage = "buffers";
            let mut protected = control
                .heap
                .reserve(HeapLane::Control, transport::PROTECTED_BYTES)?;
            let mut buffers = Paced::new(
                budget.unit(|| Worker::prepare_buffers().map_err(wire_error))?,
                &control,
            );
            protected.allocated();
            let mut input = Paced::new(
                budget.unit(|| Worker::prepare_pipe().map_err(wire_error))?,
                &control,
            );
            let mut output = Paced::new(
                budget.unit(|| Worker::prepare_pipe().map_err(wire_error))?,
                &control,
            );
            let mut stderr = Paced::new(
                budget.unit(|| Worker::prepare_pipe().map_err(wire_error))?,
                &control,
            );
            audit.stage = "command";
            let mut prepared = Paced::new(
                budget.unit(|| {
                    Worker::prepare_command(
                        proof.capability_binary(),
                        &job,
                        buffers.take_value(),
                        input.take_value(),
                        output.take_value(),
                        stderr.take_value(),
                        gate.take_value(),
                    )
                    .map_err(wire_error)
                })?,
                &control,
            );
            budget.unit(|| prepared.configure_input().map_err(wire_error))?;
            budget.unit(|| prepared.configure_output().map_err(wire_error))?;
            budget.unit(|| prepared.configure_stderr().map_err(wire_error))?;
            let mut command = Paced::new(
                budget.unit(|| prepared.take_command().map_err(wire_error))?,
                &control,
            );
            budget.unit(|| proof.still_current())?;
            audit.stage = "native_birth";
            let mut native = budget.native_process(&mut command)?;
            let (worker, unused) = match native.effect.take_value() {
                Ok(child) => prepared.take_value().attach_child(child),
                Err(code) => return Err(native.verdict.err().unwrap_or(code)),
            };
            audit.known_birth = true;
            audit.pid = worker.pid();
            audit.stage = "spawn_setup";
            let mut worker = Paced::new(worker, &control);
            let mut command = Some(command);
            let mut unused = Some(Paced::new(unused, &control));
            let ready = match native.returned_at.checked_add(bootstrap::WINDOW) {
                Some(ready) => ready.min(deadline),
                None => {
                    revoke_bootstrap(&mut worker, &control);
                    drop(command.take());
                    drop(unused.take());
                    settle_probe(&mut worker, &control);
                    born_and_reaped = true;
                    let _ = bootstrap_control(&control, || audit.observe_reap(&mut worker));
                    return Err(Code::ResourceLimit);
                }
            };
            let setup = bootstrap_control(&control, || {
                worker
                    .install_bootstrap_entry_clock_bounds(
                        native.entry_clock.ok_or(Code::ResourceLimit)?,
                    )
                    .map_err(wire_error)?;
                worker
                    .bootstrap_spawn_returned(native.returned_at, deadline)
                    .map_err(wire_error)?;
                let stamp = transport::process_stamp(worker.pid()).map_err(wire_error)?;
                audit.process_stamp(&stamp)
            });
            let preparation = (|| {
                native.verdict?;
                setup?;
                audit.stage = "pre_permit_debt";
                budget.deadline = ready;
                budget.pay_work_debt()?;
                drop(command.take());
                drop(unused.take());
                budget.pay_work_debt()?;
                budget.unit(|| proof.child_image_matches(worker.pid()))?;
                budget.pay_work_debt()?;
                audit.stage = "permit_hello";
                bootstrap_handshake(&mut worker, &control, ready, true)
            })();
            if let Err(code) = preparation {
                revoke_bootstrap(&mut worker, &control);
                drop(command.take());
                drop(unused.take());
                settle_probe(&mut worker, &control);
                born_and_reaped = true;
                let _ = bootstrap_control(&control, || audit.observe_reap(&mut worker));
                return Err(code);
            }
            audit.qualified_hello = worker.bootstrap_observation().qualified_hello;
            audit.stage = "observe";
            let observed = observe(&mut worker, &control);
            // Child is paid and parked. Parent's command/stdio copies were retired
            // before grant. Only EOF/control and actual reap follow; NEVER Start.
            budget.deadline = deadline;
            drop(command);
            drop(unused);
            worker.close_input();
            settle_probe(&mut worker, &control);
            born_and_reaped = true;
            bootstrap_control(&control, || audit.observe_reap(&mut worker))?;
            observed?;
            if control.bootstrap_control_overruns.load(Ordering::Acquire) > 0
                || control.bootstrap_clock_failures.load(Ordering::Acquire) > 0
                || control.cleanup.overruns.load(Ordering::Acquire) > 0
            {
                return Err(Code::ResourceLimit);
            }
            audit.stage = "terminal_metrics";
            let metrics = worker.producer_metrics().ok_or(Code::RunnerUnavailable)?;
            audit.child_cost_known = metrics.work_debt_paid_for(&run.0, &nonce.0);
            audit.child_terminal_max_cpu_ns = metrics.get("terminal_inclusive_max_cpu_ns");
            audit.child_remaining_debt_ns = metrics.get("terminal_remaining_debt_ns");
            if !audit.child_cost_known {
                return Err(Code::RunnerUnavailable);
            }
            for name in [
                "terminal_app_max_cpu_ns",
                "terminal_control_max_cpu_ns",
                "control_parts_input_max_cpu_ns",
                "control_parts_protocol_max_cpu_ns",
                "control_parts_settlement_max_cpu_ns",
                "control_parts_output_max_cpu_ns",
            ] {
                if metrics.get(name).is_none_or(|value| value > 1_000_000) {
                    return Err(Code::ResourceLimit);
                }
            }
            if metrics.get("native_compute_owner_create_count") != Some(1)
                || metrics.get("native_compute_owner_create_effects") != Some(1)
                || metrics.get("native_compute_owner_create_failures_with_effect") != Some(0)
                || metrics
                    .get("native_compute_owner_create_max_cpu_ns")
                    .is_none_or(|value| value > 2_000_000)
                || metrics
                    .get("native_compute_owner_create_max_wall_ns")
                    .is_none_or(|value| value > 25_000_000)
                || metrics.get("native_publish_noreplace_count") != Some(0)
            {
                return Err(Code::ResourceLimit);
            }
            Ok(transport::DreamOwner::TermC1)
        })();
        budget.deadline = deadline;
        // Only actual reap or a proven pre-birth refusal permits empty cleanup.
        // Unknown native effects retain the exact durable account claim and paths.
        let known = !control.native_birth_unknown.load(Ordering::Acquire)
            && (born_and_reaped
                || (control.cleanup.debt.process_effect.load(Ordering::Acquire) <= 1
                    && control.native_pending_since_ns.load(Ordering::Acquire) == 0));
        let cleanup = if known {
            let mut settlement =
                Budget::for_control(&control, Instant::now() + Duration::from_secs(6));
            (|| {
                audit.stage = if result.is_ok() {
                    "empty_directory_cleanup"
                } else {
                    audit.stage
                };
                settlement.unit(|| directories.remove_temporary())?;
                settlement.unit(|| directories.remove_job())?;
                audit.empty_directories_removed =
                    !directories.job_created && !directories.temporary_created;
                sync_probe_root(root, &mut settlement)?;
                settlement.unit(|| claim.release())?;
                audit.claim_released = true;
                sync_probe_root(root, &mut settlement)
            })()
        } else {
            Ok(())
        };
        audit.cleanup_error = cleanup.err();
        drop(directories);
        drop(claim);
        if control.cleanup.overruns.load(Ordering::Acquire) > 0
            || control.bootstrap_control_overruns.load(Ordering::Acquire) > 0
            || control.bootstrap_clock_failures.load(Ordering::Acquire) > 0
        {
            return Err(Code::ResourceLimit);
        }
        // Successful capability publication follows actual owner retirement
        // and the entire debt of final descriptor/path cleanup.
        budget.pay_work_debt()?;
        let result = result.and_then(|owner| cleanup.map(|()| owner));
        if result.is_ok() {
            audit.stage = "complete";
        }
        result
    })();
    // One bounded fixed-size observation; never a new disk truth store.
    let observed = bootstrap_control(&control, || {
        audit.capture_cost(&control);
        Ok(())
    });
    if control.bootstrap_clock_failures.load(Ordering::Acquire) > 0 {
        audit.parent_cost_known = false;
    }
    result.and_then(|owner| observed.map(|()| owner))
}

fn bootstrap_handshake(
    worker: &mut Worker,
    control: &Control,
    ready: Instant,
    probe: bool,
) -> Result<(), Code> {
    while control.is_foreground() && !probe {
        if control.cancelled.load(Ordering::Acquire) {
            return Err(Code::Cancelled);
        }
        let left = ready
            .checked_duration_since(Instant::now())
            .ok_or(Code::Deadline)?;
        std::thread::sleep(left.min(REST));
    }
    // Exactly one final control authorization, transfer and permit attempt.
    // 65259 covers only this prepared public guard (not payload/setup work).
    final_admission_control(control, FinalHandoff::Permit, || {
        if !probe {
            effect_guard(control)?;
        }
        if control.cancelled.load(Ordering::Acquire) {
            return Err(Code::Cancelled);
        }
        if (!probe && control.is_foreground()) || Instant::now() >= ready {
            return Err(Code::Deadline);
        }
        control.begin_child_bootstrap()?;
        worker.bootstrap_grant().map_err(wire_error)
    })?;
    loop {
        if control.cancelled.load(Ordering::Acquire) {
            return Err(Code::Cancelled);
        }
        if !probe && control.is_foreground() {
            return Err(Code::Cancelled);
        }
        if Instant::now() >= ready {
            return Err(Code::Deadline);
        }
        // Framing discards the finite Hello inside the measured control span;
        // no allocation-bearing/bulk owner is dropped while child owns work.
        if bootstrap_control(control, || {
            worker
                .hello_entry_step()
                .map(|hello| hello.is_some())
                .map_err(wire_error)
        })? {
            control.end_child_bootstrap(true)?;
            return Ok(());
        }
        worker.wait_io(ready.saturating_duration_since(Instant::now()).min(REST));
    }
}
fn revoke_bootstrap(worker: &mut Worker, control: &Control) {
    let _ = bootstrap_control(control, || {
        worker.bootstrap_revoke();
        Ok(())
    });
    if !control.bootstrap_hello_observed.load(Ordering::Acquire) {
        control.bootstrap_revoked.store(true, Ordering::Release);
    }
}

struct Publication {
    temporary: PathBuf,
    target: PathBuf,
    file: File,
    prechecked_identity: Option<(u64, u64)>,
    attempted: bool,
}
impl Publication {
    fn new(dir: &Path, name: &str, nonce: &str) -> Result<Self, Code> {
        let temporary = dir.join(format!(".{nonce}-{name}.tmp"));
        let file = create(&temporary)?;
        Ok(Self {
            temporary,
            target: dir.join(name),
            file,
            prechecked_identity: None,
            attempted: false,
        })
    }
    fn write(&mut self, bytes: &[u8]) -> Result<(), Code> {
        self.file
            .write_all(bytes)
            .map_err(|_| Code::StorageUnavailable)
    }
    fn sync_file(&self) -> Result<(), Code> {
        self.file.sync_all().map_err(|_| Code::StorageUnavailable)
    }
    fn identity(&self) -> Result<(u64, u64), Code> {
        self.file
            .metadata()
            .map(|m| (m.dev(), m.ino()))
            .map_err(|_| Code::StorageUnavailable)
    }
    fn precheck_noreplace(&mut self, identity: (u64, u64)) -> Result<(), Code> {
        self.verify_temporary(identity)?;
        match std::fs::symlink_metadata(&self.target) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err(Code::StorageUnavailable),
        }
        self.prechecked_identity = Some(identity);
        Ok(())
    }
    fn link(&self) -> Result<(), Code> {
        std::fs::hard_link(&self.temporary, &self.target).map_err(|_| Code::StorageUnavailable)
    }
    fn verify_temporary(&self, identity: (u64, u64)) -> Result<(), Code> {
        let current =
            std::fs::symlink_metadata(&self.temporary).map_err(|_| Code::StorageUnavailable)?;
        if (current.dev(), current.ino()) != identity {
            return Err(Code::StorageUnavailable);
        }
        Ok(())
    }
    fn unlink_temporary(&self) -> Result<(), Code> {
        std::fs::remove_file(&self.temporary).map_err(|_| Code::StorageUnavailable)
    }
}
fn publish(dir: &Path, name: &str, nonce: &str, bytes: &[u8]) -> Result<(), Code> {
    let mut publication = Publication::new(dir, name, nonce)?;
    publication.write(bytes)?;
    publication.sync_file()?;
    let identity = publication.identity()?;
    publication.link()?;
    publication.verify_temporary(identity)?;
    publication.unlink_temporary()?;
    File::open(dir)
        .and_then(|file| file.sync_all())
        .map_err(|_| Code::StorageUnavailable)
}
/// Manifest data and each publication syscall have distinct compute/rest units;
/// one measured unit never batches create+write+fsync+link+unlink+directory fsync.
fn publish_paced(
    dir: &Path,
    name: &str,
    nonce: &str,
    bytes: &[u8],
    budget: &mut Budget<'_>,
) -> Result<(), Code> {
    let mut publication = budget.unit(|| Publication::new(dir, name, nonce))?;
    for chunk in bytes.chunks(CHUNK) {
        budget.unit(|| publication.write(chunk))?;
    }
    budget.unit(|| publication.sync_file())?;
    let identity = budget.unit(|| publication.identity())?;
    budget.unit(|| publication.precheck_noreplace(identity))?;
    let mut outcome = budget.native_publish(&mut publication)?;
    // Preserve both the staged inode and a possibly-created target even when
    // timing/context fails. No unlink/retry follows a refused native verdict.
    let effect = outcome.effect.take_value();
    outcome.verdict?;
    effect?;
    budget.unit(|| publication.verify_temporary(identity))?;
    budget.unit(|| publication.unlink_temporary())?;
    let directory = budget.unit(|| File::open(dir).map_err(|_| Code::StorageUnavailable))?;
    budget.unit(|| directory.sync_all().map_err(|_| Code::StorageUnavailable))
}

fn wire_error(code: w::Code) -> Code {
    match code {
        w::Code::BadRequest => Code::BadRequest,
        w::Code::VersionMismatch => Code::VersionMismatch,
        w::Code::NotFound => Code::NotFound,
        w::Code::StaleContext => Code::StaleContext,
        w::Code::OpIdReused => Code::OpIdReused,
        w::Code::Busy => Code::Busy,
        w::Code::RunnerUnavailable => Code::RunnerUnavailable,
        w::Code::SourceUnavailable => Code::SourceUnavailable,
        w::Code::SourceOpen => Code::SourceOpen,
        w::Code::SourceChanged => Code::SourceChanged,
        w::Code::MalformedIr => Code::MalformedIr,
        w::Code::UnsupportedIr => Code::UnsupportedIr,
        w::Code::SourceMismatch => Code::SourceMismatch,
        w::Code::BlobMismatch => Code::BlobMismatch,
        w::Code::ResourceLimit => Code::ResourceLimit,
        w::Code::StorageUnavailable => Code::StorageUnavailable,
        w::Code::WorkerLost => Code::WorkerLost,
        w::Code::ProtocolError => Code::ProtocolError,
        w::Code::Cancelled => Code::Cancelled,
        w::Code::Deadline => Code::Deadline,
    }
}

/// Persist fixed request, exact process identity and manifest before Start can have an effect.
pub(crate) fn checkpoint(job: &Path, record: &Record, control: &Control) -> Result<(), Code> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let journal = open_checkpoint(job, record, control, deadline)?;
    let saved = save_checkpoint(&journal, record, control, deadline);
    drop(journal);
    if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
        return Err(Code::ResourceLimit);
    }
    saved
}

// A single metadata owner keeps this connection from the PID checkpoint through
// immutable manifest and first-Progress publication. The shared authority reader
// is the only other connection; no reopen/setup can interrupt the reactor.
fn open_checkpoint(
    job: &Path,
    record: &Record,
    control: &Control,
    deadline: Instant,
) -> Result<disposal::Paced<Journal>, Code> {
    let mut budget = Budget::for_control(control, deadline);
    let existing = checkpoint_stage(
        &mut budget,
        control,
        &control.checkpoint_path_max_cpu_ns,
        || {
            Journal::existing_path(
                job.parent().ok_or(Code::StorageUnavailable)?,
                &record.context.seat,
            )
        },
    )?;
    let existing = disposal::Paced::new(existing, control);
    let journal = checkpoint_stage(
        &mut budget,
        control,
        &control.checkpoint_open_max_cpu_ns,
        || Journal::connect_existing(&existing),
    )?;
    let journal = disposal::Paced::new(journal, control);
    checkpoint_stage(
        &mut budget,
        control,
        &control.checkpoint_config_max_cpu_ns,
        || journal.configure_busy(),
    )?;
    let mut header = Paced::new(
        checkpoint_stage(
            &mut budget,
            control,
            &control.checkpoint_config_max_cpu_ns,
            || journal.prepare_version(),
        )?,
        control,
    );
    let statement = &mut *header;
    let mut rows = Paced::new(
        checkpoint_stage(
            &mut budget,
            control,
            &control.checkpoint_config_max_cpu_ns,
            move || Ok(Journal::rows(statement)),
        )?,
        control,
    );
    let version = checkpoint_stage(
        &mut budget,
        control,
        &control.checkpoint_config_max_cpu_ns,
        || Journal::read_version_first(&mut rows),
    )?;
    drop(rows);
    drop(header);
    if version != 1 {
        return Err(Code::StorageUnavailable);
    }
    for configure in [
        JournalConfig::Cache,
        JournalConfig::Mmap,
        JournalConfig::Sync,
    ] {
        let statement = Paced::new(
            checkpoint_stage(
                &mut budget,
                control,
                &control.checkpoint_config_max_cpu_ns,
                || journal.prepare_config(configure),
            )?,
            control,
        );
        drop(statement);
    }
    drop(existing);
    if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
        return Err(Code::ResourceLimit);
    }
    Ok(journal)
}

fn save_checkpoint(
    journal: &Journal,
    record: &Record,
    control: &Control,
    deadline: Instant,
) -> Result<(), Code> {
    let mut budget = Budget::for_control(control, deadline);
    checkpoint_stage(
        &mut budget,
        control,
        &control.checkpoint_save_max_cpu_ns,
        || journal.save(record),
    )
}

fn checkpoint_stage<T: Drain>(
    budget: &mut Budget<'_>,
    control: &Control,
    maximum: &AtomicU64,
    work: impl FnOnce() -> Result<T, Code>,
) -> Result<T, Code> {
    // Start the stage timer inside the unit, after its mandatory rest.
    let mut used = Duration::ZERO;
    let result = budget.unit(|| {
        let before = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?;
        let mut completed = disposal::Paced::new(work(), control);
        used = bbt_rt::fault::thread_cpu_time()
            .map_err(|_| Code::ResourceLimit)?
            .saturating_sub(before);
        completed.take_value()
    });
    let mut completed = disposal::Paced::new(result, control);
    maximum.fetch_max(nanos(used), Ordering::Relaxed);
    if used > UNIT {
        control.checkpoint_overruns.fetch_add(1, Ordering::Relaxed);
    }
    if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
        return Err(Code::ResourceLimit);
    }
    completed.take_value()
}

fn private_file(path: &Path, bound: u64) -> Result<(File, Metadata), Code> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| Code::ProtocolError)?;
    let m = file.metadata().map_err(|_| Code::ProtocolError)?;
    let path_m = std::fs::symlink_metadata(path).map_err(|_| Code::ProtocolError)?;
    if !owned(&m)
        || !path_m.is_file()
        || stamp(&m) != stamp(&path_m)
        || m.nlink() != 1
        || m.mode() & 0o077 != 0
        || m.len() > bound
    {
        return Err(Code::ResourceLimit);
    }
    Ok((file, m))
}

fn read_private(path: &Path, bound: u64, budget: &mut Budget<'_>) -> Result<HeapBuffer, Code> {
    let (mut file, before) = budget.unit(|| private_file(path, bound))?;
    let mut bytes = HeapBuffer::new(&budget.control.heap, HeapLane::Work, before.len() as usize)?;
    loop {
        let mut chunk = [0; CHUNK];
        let n = budget.unit(|| file.read(&mut chunk).map_err(|_| Code::ProtocolError))?;
        if n == 0 {
            break;
        }
        if bytes.len() as u64 + n as u64 > bound {
            return Err(Code::ResourceLimit);
        }
        bytes.extend(&chunk[..n])?;
    }
    let after = budget.unit(|| file.metadata().map_err(|_| Code::ProtocolError))?;
    let path_m =
        budget.unit(|| std::fs::symlink_metadata(path).map_err(|_| Code::ProtocolError))?;
    if bytes.len() as u64 != before.len()
        || stamp(&before) != stamp(&after)
        || stamp(&before) != stamp(&path_m)
    {
        return Err(Code::ProtocolError);
    }
    Ok(bytes)
}

#[cfg(test)]
fn canonical_decode<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
    heap: &Arc<Heap>,
) -> Result<T, Code> {
    let bounds = transport::preflight_json(bytes, transport::JsonShape::Any).map_err(wire_error)?;
    // Typed result plus its canonical serde Value overlap while checking byte identity.
    let _typed = heap.reserve(
        HeapLane::Work,
        bounds.declared_payload_bytes().map_err(wire_error)?,
    )?;
    let _canonical = heap.reserve(
        HeapLane::Work,
        bounds.declared_payload_bytes().map_err(wire_error)?
            + std::mem::size_of::<Value>()
            + std::mem::size_of::<String>()
            + "next".len(),
    )?;
    let value: T = serde_json::from_slice(bytes).map_err(|_| Code::ProtocolError)?;
    let mut canonical = HeapBuffer::new(
        heap,
        HeapLane::Work,
        bytes.len() + br#",\"next\":null"#.len(),
    )?;
    let mapped = serde_json::to_value(&value).map_err(|_| Code::ProtocolError)?;
    sample_isolated_heap();
    serde_json::to_writer(HeapWriter(&mut canonical), &mapped).map_err(|_| Code::ProtocolError)?;
    if canonical.as_slice() != bytes {
        return Err(Code::ProtocolError);
    }
    Ok(value)
}

/// Canonical lexical structure is checked without a second tree. Typed serde
/// consumes a paced reader and cannot turn a large descriptor into one burst.
fn decode_canonical<T: OwnedDecode>(
    bytes: &[u8],
    budget: &mut Budget<'_>,
    require_next: bool,
) -> Result<Paced<T>, Code> {
    let (credit, bounds) = budget.json_shape(bytes, transport::JsonShape::Canonical)?;
    if require_next && !bounds.root_next {
        return Err(Code::ProtocolError);
    }
    budget.parse_credited(bytes, credit)
}

fn hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Called only after Done and actual successful child reap; output bytes alone confer nothing.
fn validate_output(
    job: &Path,
    record: &mut Record,
    manifest: &w::Manifest,
    done: &w::Done,
    control: &Control,
    deadline: Instant,
) -> Result<(), Code> {
    let mut budget = Budget::new(control, deadline);
    let output = job.join("output");
    budget.unit(|| private_dir(&output))?;
    let bytes = read_private(&output.join("result.json"), 768 * 1024, &mut budget)?;
    let _result_payload = budget.json(&bytes)?;
    let result: Paced<w::ResultDescriptor> = decode_canonical(&bytes, &mut budget, false)?;
    let result_hash = budget.hash(&bytes)?;
    let descriptor_len = bytes.len() as u64;
    drop(bytes);
    if result.run_id.0 != record.view.run_id.0
        || result.nonce.0 != record.nonce.0
        || result.manifest_sha256.0
            != record
                .view
                .manifest_sha256
                .as_ref()
                .ok_or(Code::ProtocolError)?
                .0
        || done.result_sha256.as_ref().map(|v| v.0.as_str()) != Some(&result_hash)
        || result.source_count as usize != record.sources.len()
        || result.pages.is_empty()
        || result.pages.len() > 4096
        || result.pages.len() != done.pages as usize
        || result.episode_count as u64 + result.cluster_count as u64 > 4096
        || (result.max_poll_cpu_ns.0 > 1_000_000
            && !approved_descriptor_capture(control, result.max_poll_cpu_ns.0))
        || (control
            .producer_native_profile_verified
            .load(Ordering::Acquire)
            && !approved_descriptor_capture(control, result.max_poll_cpu_ns.0))
    {
        return Err(Code::ProtocolError);
    }
    if control
        .producer_native_profile_verified
        .load(Ordering::Acquire)
    {
        let metadata = budget.unit(|| {
            std::fs::symlink_metadata(output.join("result.json")).map_err(|_| Code::ProtocolError)
        })?;
        if metadata.dev() != control.producer_metrics[64].load(Ordering::Acquire)
            || metadata.ino() != control.producer_metrics[65].load(Ordering::Acquire)
        {
            return Err(Code::ProtocolError);
        }
    }
    let mut total = descriptor_len;
    let mut rows = 0u64;
    let mut episodes = 0u32;
    let mut clusters = 0u32;
    let mut previous_episode: Option<(String, u64, String)> = None;
    let mut previous_cluster: Option<(String, String)> = None;
    let mut seen_origins = Paced::new(BTreeSet::new(), control);
    // Repo indices avoid duplicating a 4096-byte repository for each fingerprint.
    budget.retain(
        manifest.sources.len() * 2 * std::mem::size_of::<String>()
            + manifest
                .sources
                .iter()
                .map(|s| s.repo.0.len())
                .sum::<usize>(),
    )?;
    let repositories = Paced::new(
        manifest
            .sources
            .iter()
            .map(|s| s.repo.0.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<String>>(),
        control,
    );
    let mut expected = Paced::new(BTreeMap::<(usize, String), (u32, u64, u64)>::new(), control);
    // Declared, validated row counts bound retained logical entries before any growth.
    budget.retain(
        result.episode_count as usize * (std::mem::size_of::<(String, u64, String)>() + 26 + 71)
            + result.cluster_count as usize
                * (std::mem::size_of::<((usize, String), (u32, u64, u64))>() + 64)
            + 2 * (std::mem::size_of::<(String, String)>() + 4096 + 64)
            + 2 * (std::mem::size_of::<(String, u64, String)>() + 26 + 71),
    )?;
    for (index, reference) in result.pages.iter().enumerate() {
        if reference.index as usize != index || reference.len.0 > 16 * 1024 {
            return Err(Code::ProtocolError);
        }
        total = total
            .checked_add(reference.len.0)
            .ok_or(Code::ResourceLimit)?;
        if total > RESULT {
            return Err(Code::ResourceLimit);
        }
        let page_bytes = read_private(
            &output.join(format!("page-{index:06}.json")),
            16 * 1024,
            &mut budget,
        )?;
        let _page_payload = budget.json(&page_bytes)?;
        let page: Paced<w::Page> = decode_canonical(&page_bytes, &mut budget, true)?;
        if reference.len.0 != page_bytes.len() as u64
            || reference.sha256.0 != budget.hash(&page_bytes)?
            || page.run_id != result.run_id
            || page.manifest_sha256 != result.manifest_sha256
            || page.index as usize != index
            || page.rows.len() > 128
            || page.next != (index + 1 < result.pages.len()).then_some(index as u32 + 1)
            || (page.rows.is_empty()
                && (result.pages.len() != 1
                    || result.episode_count != 0
                    || result.cluster_count != 0))
        {
            return Err(Code::ProtocolError);
        }
        for row in &page.rows {
            rows += 1;
            if rows > 4096 {
                return Err(Code::ResourceLimit);
            }
            budget.unit(|| {
                encoded_fits(row, 8 * 1024)?;
                match row {
                    w::Row::Episode(row) => {
                        if episodes >= result.episode_count {
                            return Err(Code::ProtocolError);
                        }
                        if previous_cluster.is_some()
                            || row.fingerprints.len() > 32
                            || row.first_ms.0 > row.last_ms.0
                            || row.origin.session != row.session
                            || row.origin.seq.0 == 0
                            || !row.origin.digest.strip_prefix("sha256:").is_some_and(hex)
                            || !row.fingerprints.windows(2).all(|v| v[0] < v[1])
                        {
                            return Err(Code::ProtocolError);
                        }
                        let source = manifest
                            .sources
                            .iter()
                            .find(|s| s.session == row.session)
                            .ok_or(Code::ProtocolError)?;
                        if row.origin.seq.0 > source.closed_seq.0 {
                            return Err(Code::ProtocolError);
                        }
                        let origin = (
                            row.session.0.clone(),
                            row.origin.seq.0,
                            row.origin.digest.clone(),
                        );
                        if previous_episode.as_ref().is_some_and(|old| old >= &origin)
                            || !seen_origins.insert(origin.clone())
                        {
                            return Err(Code::ProtocolError);
                        }
                        previous_episode = Some(origin);
                        let repo = repositories
                            .binary_search(&source.repo.0)
                            .map_err(|_| Code::ProtocolError)?;
                        for fingerprint in &row.fingerprints {
                            if !expected.contains_key(&(repo, fingerprint.0.clone()))
                                && expected.len() >= result.cluster_count as usize
                            {
                                return Err(Code::ProtocolError);
                            }
                            let entry = expected.entry((repo, fingerprint.0.clone())).or_insert((
                                0,
                                row.first_ms.0,
                                row.last_ms.0,
                            ));
                            entry.0 += 1;
                            entry.1 = entry.1.min(row.first_ms.0);
                            entry.2 = entry.2.max(row.last_ms.0);
                        }
                        episodes += 1;
                    }
                    w::Row::Cluster(row) => {
                        if clusters >= result.cluster_count {
                            return Err(Code::ProtocolError);
                        }
                        let key = (row.repo.0.clone(), row.fingerprint.0.clone());
                        if previous_cluster.as_ref().is_some_and(|old| old >= &key)
                            || row.first_ms.0 > row.last_ms.0
                        {
                            return Err(Code::ProtocolError);
                        }
                        previous_cluster = Some(key);
                        let repo = repositories
                            .binary_search(&row.repo.0)
                            .map_err(|_| Code::ProtocolError)?;
                        if expected.remove(&(repo, row.fingerprint.0.clone()))
                            != Some((row.count, row.first_ms.0, row.last_ms.0))
                        {
                            return Err(Code::ProtocolError);
                        }
                        clusters += 1;
                    }
                }
                Ok(())
            })?;
        }
    }
    if episodes != result.episode_count || clusters != result.cluster_count || !expected.is_empty()
    {
        return Err(Code::ProtocolError);
    }
    // Input was capped independently; output+index remains under the explicit job allocation.
    let inputs: u64 = manifest
        .sources
        .iter()
        .map(|s| s.len.0 + s.blobs.iter().map(|b| b.len.0).sum::<u64>())
        .sum();
    if inputs.checked_add(total).is_none_or(|n| n > DISK) {
        return Err(Code::ResourceLimit);
    }
    record.view.result_sha256 = Some(public::Hex(result_hash));
    record.view.pages = result.pages.len() as u32;
    checkpoint(job, record, control)?;
    Ok(())
}

fn approved_descriptor_capture(control: &Control, captured: u64) -> bool {
    control.native_metrics_authorized.load(Ordering::Acquire)
        && control
            .producer_native_profile_verified
            .load(Ordering::Acquire)
        && captured <= 3_000_000
        && control.producer_metrics[45].load(Ordering::Acquire) == captured
        && captured <= control.producer_metrics[46].load(Ordering::Acquire)
        && control.producer_metrics[46].load(Ordering::Acquire) <= 3_000_000
}

#[cfg(test)]
pub(crate) fn test_cleanup_units(control: &Control) -> u64 {
    control.cleanup.units.load(Ordering::Acquire)
}

pub(crate) fn retire_public_session(
    session: &crate::identity::PublicFenceSession,
    control: &Control,
) -> Result<(), Code> {
    let waiting = Instant::now();
    let mut quarantined = false;
    let connection = loop {
        match session.try_take_connection() {
            Ok(connection) => break connection,
            Err(Code::Busy) => {
                if waiting.elapsed() >= Duration::from_secs(6) && !quarantined {
                    control.state.store(QUARANTINED, Ordering::Release);
                    control.revision.fetch_add(1, Ordering::AcqRel);
                    quarantined = true;
                }
                std::thread::park_timeout(REST);
            }
            Err(code) => return Err(code),
        }
    };
    if let Some(connection) = connection {
        let connection = Paced::new(connection, control);
        // rusqlite0.40.2 Connection::drop flushes the statement LRU before its
        // InnerConnection field closes SQLite. Retire those real phases in
        // distinct required units; no close/native allowance is introduced.
        // Capacity is fixed to three before publication. Shrink one slot per
        // required unit to separate native finalization from connection close.
        for capacity in (0..3).rev() {
            disposal::cleanup_unit(&control.cleanup, || {
                connection.set_prepared_statement_cache_capacity(capacity)
            });
        }
        drop(connection);
    }
    if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
        Err(Code::ResourceLimit)
    } else if quarantined {
        Err(Code::WorkerLost)
    } else {
        Ok(())
    }
}

/// Immutable requested page reads run on a separate result owner, never a Status hot path.
#[cfg(test)]
pub(crate) fn read_page(job: &Path, record: &Record, index: u32) -> Result<public::Page, Code> {
    read_page_guarded(job, record, index, None)
}
pub(crate) fn read_page_guarded(
    job: &Path,
    record: &Record,
    index: u32,
    authority_guard: Option<PreparedGuard>,
) -> Result<public::Page, Code> {
    let retirement = authority_guard
        .as_ref()
        .and_then(|guard| guard.retire_session.clone());
    let control = Control {
        authority_setup_cpu_ns: AtomicU64::new(
            authority_guard.as_ref().map_or(0, |g| g.setup_cpu_ns),
        ),
        authority_setup_max_unit_cpu_ns: AtomicU64::new(
            authority_guard
                .as_ref()
                .map_or(0, |g| g.setup_max_unit_cpu_ns),
        ),
        authority_guard: authority_guard.map(|g| g.callback),
        ..Control::default()
    };
    let result = (|| {
        if control
            .authority_setup_max_unit_cpu_ns
            .load(Ordering::Acquire)
            > 1_000_000
            || control.authority_setup_cpu_ns.load(Ordering::Acquire)
                < control
                    .authority_setup_max_unit_cpu_ns
                    .load(Ordering::Acquire)
            || control.authority_setup_cpu_ns.load(Ordering::Acquire) > public::MAX_SAFE
        {
            return Err(Code::ResourceLimit);
        }
        if record.view.state != State::Completed || !record.view.settled {
            return Err(Code::NotFound);
        }
        if index >= record.view.pages {
            return Err(Code::BadRequest);
        }
        utility()?;
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(2));
        let output = job.join("output");
        let bytes = read_private(&output.join("result.json"), 768 * 1024, &mut budget)?;
        let _result_payload = budget.json(&bytes)?;
        if budget.hash(&bytes)?
            != record
                .view
                .result_sha256
                .as_ref()
                .ok_or(Code::ProtocolError)?
                .0
        {
            return Err(Code::ProtocolError);
        }
        let result: Paced<w::ResultDescriptor> = decode_canonical(&bytes, &mut budget, false)?;
        drop(bytes);
        let reference = result.pages.get(index as usize).ok_or(Code::BadRequest)?;
        let bytes = read_private(
            &output.join(format!("page-{index:06}.json")),
            16 * 1024,
            &mut budget,
        )?;
        let _page_payload = budget.json(&bytes)?;
        if bytes.len() as u64 != reference.len.0 || budget.hash(&bytes)? != reference.sha256.0 {
            return Err(Code::ProtocolError);
        }
        let page: Paced<w::Page> = decode_canonical(&bytes, &mut budget, true)?;
        drop(bytes);
        if page.run_id.0 != record.view.run_id.0
            || page.manifest_sha256.0
                != record
                    .view
                    .manifest_sha256
                    .as_ref()
                    .ok_or(Code::ProtocolError)?
                    .0
            || page.index != index
        {
            return Err(Code::ProtocolError);
        }
        let mut public = Paced::new(mirror_page(&page, &mut budget)?, &control);
        // The final result cannot escape before all decoded owners have actually
        // retired, including cleanup after an earlier cancellation/clock failure.
        drop(page);
        drop(result);
        if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
            return Err(Code::ResourceLimit);
        }
        Ok(public.take_value())
    })();
    let mut result = Paced::new(result, &control);
    let retired = retirement
        .as_ref()
        .map_or(Ok(()), |session| retire_public_session(session, &control));
    retired?;
    if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
        return Err(Code::ResourceLimit);
    }
    result.take_value()
}

struct Frames {
    seq: u64,
    previous: Option<w::WorkerOut>,
    done: Option<w::Done>,
}
impl Frames {
    fn observe(
        &mut self,
        frame: w::WorkerOut,
        record: &Record,
        control: &Control,
    ) -> Result<(), Code> {
        if self.done.is_some() {
            return Err(Code::ProtocolError);
        }
        let (seq, run, nonce) = match &frame {
            w::WorkerOut::Progress(v) => (v.seq.0, &v.run_id.0, &v.nonce.0),
            w::WorkerOut::Done(v) => (v.seq.0, &v.run_id.0, &v.nonce.0),
            w::WorkerOut::Hello(_) => return Err(Code::ProtocolError),
        };
        if run != &record.view.run_id.0 || nonce != &record.nonce.0 {
            return Err(Code::ProtocolError);
        }
        if seq == self.seq && self.previous.as_ref() == Some(&frame) {
            return Ok(());
        }
        if seq != self.seq + 1 {
            return Err(Code::ProtocolError);
        }
        self.seq = seq;
        self.previous = Some(frame.clone());
        match frame {
            w::WorkerOut::Progress(v) => {
                if v.bytes.0 > INPUT || v.envelopes.0 > 65_536 {
                    return Err(Code::ResourceLimit);
                }
                match v.phase {
                    w::Phase::Paused => control.phase(PAUSED),
                    w::Phase::Settling => control.phase(SETTLING),
                    _ => control.phase(RUNNING),
                }
            }
            w::WorkerOut::Done(v) => {
                if v.outcome == w::Outcome::Completed
                    && (v.error.is_some() || v.result_sha256.is_none() || v.pages == 0)
                    || v.outcome != w::Outcome::Completed
                        && (v.result_sha256.is_some() || v.pages != 0)
                {
                    return Err(Code::ProtocolError);
                }
                self.done = Some(v);
                control.phase(SETTLING);
            }
            w::WorkerOut::Hello(_) => unreachable!(),
        }
        Ok(())
    }
}

fn checked_poll(worker: &mut Worker, control: &Control) -> Result<Option<w::WorkerOut>, Code> {
    let cpu = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?;
    let frame = worker.receive().map_err(wire_error);
    let used = bbt_rt::fault::thread_cpu_time()
        .map_err(|_| Code::ResourceLimit)?
        .saturating_sub(cpu);
    control.max_unit_cpu_ns.fetch_max(
        used.as_nanos().min(u64::MAX as u128) as u64,
        Ordering::Relaxed,
    );
    if used > UNIT {
        control.unit_overruns.fetch_add(1, Ordering::Relaxed);
        return Err(Code::ResourceLimit);
    }
    frame
}
fn effect_guard(control: &Control) -> Result<(), Code> {
    if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
        return Err(Code::ResourceLimit);
    }
    let cpu = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?;
    let result = control
        .installed_authority_guard()
        .map_or(Ok(()), |guard| guard());
    let used = bbt_rt::fault::thread_cpu_time()
        .map_err(|_| Code::ResourceLimit)?
        .saturating_sub(cpu);
    control
        .dispatch_guard_max_cpu_ns
        .fetch_max(nanos(used), Ordering::Relaxed);
    control
        .max_unit_cpu_ns
        .fetch_max(nanos(used), Ordering::Relaxed);
    if used > UNIT {
        control
            .dispatch_guard_overruns
            .fetch_add(1, Ordering::Relaxed);
        control.unit_overruns.fetch_add(1, Ordering::Relaxed);
        return Err(Code::ResourceLimit);
    }
    result
}

struct ReactorClock<'a> {
    control: &'a Control,
    cpu: Duration,
    wall: Instant,
}
impl<'a> ReactorClock<'a> {
    fn new(control: &'a Control) -> Result<Self, Code> {
        control.reactor_units.fetch_add(1, Ordering::Relaxed);
        let cpu = bbt_rt::fault::thread_cpu_time().map_err(|_| {
            control
                .reactor_clock_failures
                .fetch_add(1, Ordering::Relaxed);
            Code::ResourceLimit
        })?;
        Ok(Self {
            control,
            cpu,
            wall: Instant::now(),
        })
    }
    fn batch_due(&self) -> Result<bool, Code> {
        Ok(bbt_rt::fault::thread_cpu_time()
            .map_err(|_| Code::ResourceLimit)?
            .saturating_sub(self.cpu)
            >= Duration::from_micros(500))
    }
    fn finish(self) -> Result<(), Code> {
        let used = bbt_rt::fault::thread_cpu_time()
            .map_err(|_| {
                self.control
                    .reactor_clock_failures
                    .fetch_add(1, Ordering::Relaxed);
                Code::ResourceLimit
            })?
            .saturating_sub(self.cpu);
        self.control
            .reactor_cpu_ns
            .fetch_add(nanos(used), Ordering::Relaxed);
        self.control
            .reactor_wall_ns
            .fetch_add(nanos(self.wall.elapsed()), Ordering::Relaxed);
        self.control
            .reactor_max_cpu_ns
            .fetch_max(nanos(used), Ordering::Relaxed);
        self.control
            .max_unit_cpu_ns
            .fetch_max(nanos(used), Ordering::Relaxed);
        if used > UNIT {
            self.control
                .reactor_overruns
                .fetch_add(1, Ordering::Relaxed);
            self.control.unit_overruns.fetch_add(1, Ordering::Relaxed);
            #[cfg(test)]
            eprintln!(
                "C1 reactor whole-iteration overrun cpu_ns={}",
                used.as_nanos()
            );
            return Err(Code::ResourceLimit);
        }
        Ok(())
    }
}

/// Resolve platform aliases only for the newly created private job, retaining
/// its exact directory inode. No source or manifest is opened by this operation.
fn canonical_job(job: &Path, budget: &mut Budget<'_>) -> Result<(PathBuf, File), Code> {
    let before =
        budget.unit(|| std::fs::symlink_metadata(job).map_err(|_| Code::StorageUnavailable))?;
    let canonical = budget.unit(|| job.canonicalize().map_err(|_| Code::StorageUnavailable))?;
    let directory = budget.unit(|| {
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&canonical)
            .map_err(|_| Code::StorageUnavailable)
    })?;
    let opened = budget.unit(|| directory.metadata().map_err(|_| Code::StorageUnavailable))?;
    let path = budget
        .unit(|| std::fs::symlink_metadata(&canonical).map_err(|_| Code::StorageUnavailable))?;
    // SAFETY: geteuid has no pointer arguments or side effects.
    if !before.is_dir()
        || !opened.is_dir()
        || opened.uid() != unsafe { libc::geteuid() }
        || opened.mode() & 0o777 != 0o700
        || (before.dev(), before.ino()) != (opened.dev(), opened.ino())
        || (path.dev(), path.ino()) != (opened.dev(), opened.ino())
    {
        return Err(Code::StorageUnavailable);
    }
    Ok((canonical, directory))
}

/// Retains process and snapshot ownership through real exit, including a blocked syscall.
#[cfg(test)]
pub(crate) fn execute(
    binary: &Path,
    job: &Path,
    record: &mut Record,
    control: &Control,
) -> Result<(), Code> {
    if control.launch_incarnation.load(Ordering::Acquire) == 0 {
        control
            .launch_incarnation
            .store(monotonic_ns()?, Ordering::Release);
    }
    if control.native_provision.get().is_none() {
        let proof = runner_provision::verify_native_provision(binary, false)?;
        control.install_native_provision(Arc::new(proof))?;
    }
    execute_with_settlement(binary, job, record, control, || Ok(()))
}
pub(crate) fn execute_with_settlement(
    binary: &Path,
    job: &Path,
    record: &mut Record,
    control: &Control,
    settle: impl FnOnce() -> Result<(), Code>,
) -> Result<(), Code> {
    bbt_rt::assert_not_ui_thread();
    let mut result = execute_inner(binary, job, record, control);
    if let Err(code) = settle() {
        result = Err(code);
    }
    if control.cleanup.overruns.load(Ordering::Acquire) > 0 {
        result = Err(Code::ResourceLimit);
    }
    if let Err(code) = write_metrics(job, record, control)
        && result.is_ok()
    {
        result = Err(code);
    }
    record.view.revision.0 = record
        .view
        .revision
        .0
        .saturating_add(control.revision.load(Ordering::Acquire))
        .saturating_add(1);
    let unknown_birth = control.native_birth_unknown.load(Ordering::Acquire);
    record.view.settled = !unknown_birth;
    record.view.state = if unknown_birth {
        State::Quarantined
    } else if result.is_ok() {
        State::Completed
    } else if result == Err(Code::Cancelled) {
        State::Cancelled
    } else {
        State::Failed
    };
    record.view.error = result.as_ref().err().map(|code| public::Error {
        code: *code,
        source: None,
    });
    if result.is_err() {
        record.view.pages = 0;
        record.view.result_sha256 = None;
    }
    // Final status is durable before publishing the terminal settled flag.
    let saved = checkpoint(job, record, control);
    if let Err(code) = saved {
        record.view.state = State::Failed;
        record.view.error = Some(public::Error { code, source: None });
        control.state.store(FAILED, Ordering::Release);
    } else {
        control.state.store(
            if result.is_ok() {
                COMPLETED
            } else if result == Err(Code::Cancelled) {
                CANCELLED
            } else {
                FAILED
            },
            Ordering::Release,
        );
    }
    if unknown_birth {
        control.state.store(QUARANTINED, Ordering::Release);
    }
    control.settled.store(!unknown_birth, Ordering::Release);
    result.and(saved)
}

/// Failed Hello/checkpoint still owns the exact child; do not let Drop hide an unsettled wait.
fn settle_probe(worker: &mut Worker, control: &Control) {
    revoke_bootstrap(worker, control);
    control.phase(SETTLING);
    worker.close_input();
    let since = Instant::now();
    let mut term = false;
    let mut kill = false;
    loop {
        let mut settled = false;
        let _ = bootstrap_control(control, || {
            if worker.receive().is_err() {
                worker.discard_pending_output();
            }
            settled = worker.try_wait().is_ok_and(|exit| exit.is_some()) && worker.is_eof();
            if settled {
                retain_child_usage(worker, control);
                let _ = control
                    .bootstrap_entry
                    .set(worker.bootstrap_entry_evidence());
            }
            if !settled && since.elapsed() >= Duration::from_secs(2) && !term {
                let _ = worker.terminate(false);
                term = true;
            }
            if !settled && since.elapsed() >= Duration::from_secs(4) && !kill {
                let _ = worker.terminate(true);
                kill = true;
            }
            if !settled && since.elapsed() >= Duration::from_secs(6) {
                control.phase(QUARANTINED);
            }
            Ok(())
        });
        if settled {
            break;
        }
        std::thread::sleep(REST);
    }
    if control.cleanup.debt.child_bootstrap.load(Ordering::Acquire) {
        let _ = control.end_child_bootstrap(false);
    }
}

fn retain_child_usage(worker: &Worker, control: &Control) {
    if let Some(usage) = worker.usage() {
        control
            .child_peak_rss_bytes
            .store(usage.peak_rss_bytes, Ordering::Release);
        control
            .child_user_cpu_ns
            .store(usage.user_cpu_ns, Ordering::Release);
        control
            .child_system_cpu_ns
            .store(usage.system_cpu_ns, Ordering::Release);
        control.child_usage_observed.store(true, Ordering::Release);
    }
    let metrics = worker.producer_metrics();
    control.producer_native_profile_observed.store(
        metrics.as_ref().is_some_and(|m| m.has_native_profile()),
        Ordering::Release,
    );
    for (index, value) in control.producer_metrics.iter().enumerate() {
        value.store(
            metrics
                .as_ref()
                .and_then(|m| m.values[index])
                .unwrap_or(u64::MAX),
            Ordering::Release,
        );
    }
    control
        .producer_metrics_observed
        .store(metrics.is_some(), Ordering::Release);
}

struct FixedJsonWriter<'a, const N: usize>(&'a mut FixedBytes<N>);
impl<const N: usize> std::io::Write for FixedJsonWriter<'_, N> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .extend(bytes)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::FileTooLarge))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn write_metrics(job: &Path, record: &Record, control: &Control) -> Result<(), Code> {
    use std::fmt::Write as _;
    let mut budget = Budget::for_control(control, Instant::now() + Duration::from_secs(30));
    let bytes = budget.unit(|| {
    let work = control.heap.counters(HeapLane::Work);
    let lane = control.heap.counters(HeapLane::Control);
    // No final heap-buffer destructor follows the immutable metric snapshot.
    // Terminal disaggregated native/debt evidence uses an exact16KiB stack
    // buffer (inside the existing fixed2MiB isolated-owner stack allocation).
    let mut bytes = FixedBytes {
        bytes: [0; 16384],
        len: 0,
    };
    write!(&mut bytes,"{{\"schema\":\"bboxy.dream.c1.host-metrics/1\",\"run_id\":\"{}\",\"nonce\":\"{}\",\"work_limit_bytes\":{},\"protected_control_bytes\":{},\"opaque_helper_headroom_bytes\":{},\"declared_work_reserved_bytes\":{},\"tracked_work_live_bytes\":{},\"tracked_work_peak_bytes\":{},\"tracked_work_cumulative_bytes\":{},\"declared_control_reserved_bytes\":{},\"tracked_control_live_bytes\":{},\"tracked_control_peak_bytes\":{},\"tracked_control_cumulative_bytes\":{},\"max_unit_cpu_ns\":{},\"unit_overruns\":{},\"unit_overrun_line\":{},\"child_usage_observed\":{},\"child_peak_rss_bytes\":{},\"child_process_user_cpu_ns\":{},\"child_process_system_cpu_ns\":{}}}",
        record.view.run_id.0,record.nonce.0,WORK_HEAP,CONTROL_HEAP,HELPER_HEADROOM,
        work.reserved,work.live,work.peak,work.cumulative,lane.reserved,lane.live,lane.peak,lane.cumulative,
        control.max_unit_cpu_ns.load(Ordering::Acquire),control.unit_overruns.load(Ordering::Acquire),control.unit_overrun_line.load(Ordering::Acquire),
        control.child_usage_observed.load(Ordering::Acquire),control.child_peak_rss_bytes.load(Ordering::Acquire),control.child_user_cpu_ns.load(Ordering::Acquire),control.child_system_cpu_ns.load(Ordering::Acquire)).map_err(|_|Code::ResourceLimit)?;
    bytes.truncate(bytes.len() - 1)?;
    write!(&mut bytes,",\"declared_work_reserved_peak_bytes\":{},\"declared_work_cumulative_credits_bytes\":{},\"declared_control_reserved_peak_bytes\":{},\"declared_control_cumulative_credits_bytes\":{}",work.reserved_peak,work.reserved_cumulative,lane.reserved_peak,lane.reserved_cumulative).map_err(|_|Code::ResourceLimit)?;
    for (name, value) in [
        (
            "snapshot_owner_thread_cpu_ns",
            &control.snapshot_owner_thread_cpu_ns,
        ),
        ("snapshot_owner_wall_ns", &control.snapshot_owner_wall_ns),
        ("snapshot_active_cpu_ns", &control.snapshot_active_cpu_ns),
        ("snapshot_active_wall_ns", &control.snapshot_active_wall_ns),
        (
            "source_copy_unit_max_cpu_ns",
            &control.source_copy_unit_max_cpu_ns,
        ),
        (
            "blob_copy_unit_max_cpu_ns",
            &control.blob_copy_unit_max_cpu_ns,
        ),
        ("snapshot_tiny_records", &control.snapshot_tiny_records),
        ("snapshot_staged_records", &control.snapshot_staged_records),
        (
            "process_owner_thread_cpu_ns",
            &control.process_owner_thread_cpu_ns,
        ),
        ("process_owner_wall_ns", &control.process_owner_wall_ns),
        ("work_unit_count", &control.work_unit_count),
        ("cleanup_cpu_ns", &control.cleanup.cpu_ns),
        ("max_cleanup_unit_cpu_ns", &control.cleanup.max_unit_cpu_ns),
        ("cleanup_overruns", &control.cleanup.overruns),
        ("cleanup_units", &control.cleanup.units),
        (
            "collection_capacity_refusals",
            &control.cleanup.capacity_refusals,
        ),
        ("rest_wait_ns", &control.rest_wait_ns),
        ("rest_wait_cpu_ns", &control.rest_wait_cpu_ns),
        ("rest_skips", &control.rest_skips),
        ("rest_violations", &control.rest_violations),
        ("min_inter_unit_gap_ns", &control.min_inter_unit_gap_ns),
        ("source_owner_refusals", &control.owner_refusals),
        ("authority_setup_cpu_ns", &control.authority_setup_cpu_ns),
        ("authority_setup_max_unit_cpu_ns", &control.authority_setup_max_unit_cpu_ns),
        (
            "authority_monitor_setup_cpu_ns",
            &control.authority_monitor_setup_cpu_ns,
        ),
        (
            "authority_monitor_max_cpu_ns",
            &control.authority_monitor_max_cpu_ns,
        ),
        (
            "authority_monitor_overruns",
            &control.authority_monitor_overruns,
        ),
        ("reactor_units", &control.reactor_units),
        ("reactor_max_cpu_ns", &control.reactor_max_cpu_ns),
        ("reactor_cpu_ns", &control.reactor_cpu_ns),
        ("reactor_wall_ns", &control.reactor_wall_ns),
        ("reactor_overruns", &control.reactor_overruns),
        ("reactor_clock_failures", &control.reactor_clock_failures),
        ("reactor_rest_wall_ns", &control.reactor_rest_wall_ns),
        ("reactor_rest_cpu_ns", &control.reactor_rest_cpu_ns),
        ("reactor_min_gap_ns", &control.reactor_min_gap_ns),
        (
            "dispatch_guard_max_cpu_ns",
            &control.dispatch_guard_max_cpu_ns,
        ),
        ("dispatch_guard_overruns", &control.dispatch_guard_overruns),
        (
            "codec_prepare_max_cpu_ns",
            &control.codec_prepare_max_cpu_ns,
        ),
        ("worker_spawn_cpu_ns", &control.worker_spawn_cpu_ns),
        ("worker_start_cpu_ns", &control.worker_start_cpu_ns),
        (
            "native_dispatch_overruns",
            &control.native_dispatch_overruns,
        ),
        (
            "checkpoint_path_max_cpu_ns",
            &control.checkpoint_path_max_cpu_ns,
        ),
        (
            "checkpoint_open_max_cpu_ns",
            &control.checkpoint_open_max_cpu_ns,
        ),
        (
            "checkpoint_config_max_cpu_ns",
            &control.checkpoint_config_max_cpu_ns,
        ),
        (
            "checkpoint_save_max_cpu_ns",
            &control.checkpoint_save_max_cpu_ns,
        ),
        ("checkpoint_overruns", &control.checkpoint_overruns),
        ("bootstrap_control_cpu_ns",&control.bootstrap_control_cpu_ns),
        ("bootstrap_control_max_cpu_ns",&control.bootstrap_control_max_cpu_ns),
        ("bootstrap_control_overruns",&control.bootstrap_control_overruns),
        ("bootstrap_clock_failures",&control.bootstrap_clock_failures),
        ("bootstrap_permit_attempts",&control.bootstrap_permit_attempts),
        ("bootstrap_parent_handoffs",&control.bootstrap_parent_handoffs),
        ("bootstrap_child_handoffs",&control.bootstrap_child_handoffs),
    ] {
        write!(
            &mut bytes,
            ",\"{}\":{}",
            name,
            value.load(Ordering::Acquire)
        )
        .map_err(|_| Code::ResourceLimit)?;
    }
    let debt = &control.cleanup.debt;
    for (name, value) in [
        ("work_debt_spans", &debt.spans),
        ("work_debt_charged_ns", &debt.charged_ns),
        ("work_debt_paid_wall_ns", &debt.paid_ns),
        ("work_debt_max_ns", &debt.max_debt_ns),
        ("work_debt_admission_violations", &debt.admission_violations),
        ("native_process_calls", &debt.native_process_calls),
        ("native_publish_calls", &debt.native_publish_calls),
        ("native_cpu_ns", &debt.native_cpu_ns),
        ("native_max_cpu_ns", &debt.native_max_cpu_ns),
        ("native_max_wall_ns", &debt.native_max_wall_ns),
        ("native_process_max_cpu_ns", &debt.native_process_max_cpu_ns),
        ("native_process_max_wall_ns", &debt.native_process_max_wall_ns),
        ("native_publish_max_cpu_ns", &debt.native_publish_max_cpu_ns),
        ("native_publish_max_wall_ns", &debt.native_publish_max_wall_ns),
        ("native_app_max_cpu_ns", &debt.native_app_max_cpu_ns),
        ("inclusive_max_cpu_ns", &debt.inclusive_max_cpu_ns),
        ("native_overruns", &debt.native_overruns),
        ("native_clock_failures", &debt.native_clock_failures),
        ("native_effects", &debt.native_effects),
    ] {
        write!(
            &mut bytes,
            ",\"{}\":{}",
            name,
            value.load(Ordering::Acquire)
        )
        .map_err(|_| Code::ResourceLimit)?;
    }
    write!(&mut bytes, ",\"work_debt_remaining_ns\":{},\"work_debt_poisoned\":{},\"native_metrics_profile\":\"c1-native-debt/1\",\"metrics_capture_cutoff\":\"after_reap_settlement_before_terminal_metrics_write\"", debt.remaining_ns(),debt.poisoned.load(Ordering::Acquire)).map_err(|_| Code::ResourceLimit)?;
    write!(&mut bytes,",\"native_birth_unknown\":{},\"native_quarantine_observed\":{}",control.native_birth_unknown.load(Ordering::Acquire),control.native_quarantine_observed.load(Ordering::Acquire)).map_err(|_|Code::ResourceLimit)?;
    write!(&mut bytes,",\"bootstrap_profile\":\"dream-bootstrap/1\",\"bootstrap_hello_observed\":{},\"bootstrap_revoked\":{}",control.bootstrap_hello_observed.load(Ordering::Acquire),control.bootstrap_revoked.load(Ordering::Acquire)).map_err(|_|Code::ResourceLimit)?;
    if let Some(launch)=&record.launch {
        write!(&mut bytes,",\"bootstrap_birth\":\"{}\",\"bootstrap_binding\":\"{}\",\"bootstrap_incarnation\":{},\"bootstrap_pipe_fd\":{},\"bootstrap_pipe_dev\":{},\"bootstrap_pipe_ino\":{}",launch.birth.0,launch.binding.0,launch.incarnation,launch.pipe_fd,launch.pipe_device,launch.pipe_inode).map_err(|_|Code::ResourceLimit)?;
    }
    write!(&mut bytes, ",\"original_op_id\":\"{}\",\"process_native_errno\":{},\"publish_native_errno\":{},\"process_native_effect\":{},\"publish_native_effect\":{},\"publish_native_dev\":{},\"publish_native_ino\":{}", record.op_id.0, debt.process_errno.load(Ordering::Acquire),debt.publish_errno.load(Ordering::Acquire),debt.process_effect.load(Ordering::Acquire),debt.publish_effect.load(Ordering::Acquire),debt.publish_dev.load(Ordering::Acquire),debt.publish_ino.load(Ordering::Acquire)).map_err(|_|Code::ResourceLimit)?;
    write!(&mut bytes, ",\"cost_known\":{},\"native_effect_observation\":\"known_success_count_other_returns_not_absence_proof\"", !debt.poisoned.load(Ordering::Acquire)).map_err(|_|Code::ResourceLimit)?;
    write!(
        &mut bytes,
        ",\"producer_metrics_observed\":{},\"producer_native_cumulative_measured\":false,\"producer_native_profile_observed\":{},\"producer_native_profile_verified\":{},\"native_metrics_authorized\":{}",
        control.producer_metrics_observed.load(Ordering::Acquire),control.producer_native_profile_observed.load(Ordering::Acquire),control.producer_native_profile_verified.load(Ordering::Acquire),control.native_metrics_authorized.load(Ordering::Acquire)
    )
    .map_err(|_| Code::ResourceLimit)?;
    write!(&mut bytes, ",\"native_provision_development\":{},\"native_provision_control_only\":{}",
        control.native_provision.get().is_some_and(|p| p.is_private_development()),
        control.native_provision.get().is_some_and(|p| p.control_only_wrapper()))
        .map_err(|_| Code::ResourceLimit)?;
    if control.producer_native_profile_observed.load(Ordering::Acquire) {
        bytes.extend(b",\"producer_native_metrics_profile\":\"c1-native-debt/1\",\"producer_terminal_cutoff\":\"joined_final_control_pre_telemetry\"")?;
    }
    bytes.extend(b",\"codec_live_requested_capacities_accounted\":true,\"codec_partial_allocation_transients_measured\":false")?;
    if control.producer_metrics_observed.load(Ordering::Acquire) {
        for (index, key) in transport::PRODUCER_METRIC_KEYS.iter().enumerate() {
            let value = control.producer_metrics[index].load(Ordering::Acquire);
            if value != u64::MAX {
                write!(&mut bytes, ",\"producer_{}\":{}", key, value)
                    .map_err(|_| Code::ResourceLimit)?;
            }
        }
    }
    bytes.extend(b",\"bootstrap_entry_evidence\":")?;
    serde_json::to_writer(FixedJsonWriter(&mut bytes),&control.bootstrap_entry.get())
        .map_err(|_|Code::ResourceLimit)?;
    bytes.extend(b"}")?;
    Ok(bytes)
    })?;
    // Diagnostics are explicitly partial/unproven until the complete/reaped
    // run commits. No third atomic publication consumes a native permit.
    // Retain the private diagnostic inode on any timing/write failure.
    let mut file = budget.unit(|| create(&job.join("metrics.json")))?;
    for chunk in bytes.as_slice().chunks(4096) {
        budget.unit(|| file.write_all(chunk).map_err(|_| Code::StorageUnavailable))?;
    }
    budget.unit(|| file.sync_all().map_err(|_| Code::StorageUnavailable))?;
    drop(Paced::new(file, control));
    // Required settlement is permitted on a poisoned run but can never produce
    // a successful completion. A successful metrics write pays all its debt.
    control.cleanup.debt.wait(Some(budget.deadline), true)
}

struct FixedBytes<const N: usize> {
    bytes: [u8; N],
    len: usize,
}
impl<const N: usize> Drain for FixedBytes<N> {
    fn drain_step(&mut self) -> bool {
        false
    }
}
impl<const N: usize> FixedBytes<N> {
    fn len(&self) -> usize {
        self.len
    }
    fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
    fn truncate(&mut self, length: usize) -> Result<(), Code> {
        if length > self.len {
            return Err(Code::ResourceLimit);
        }
        self.len = length;
        Ok(())
    }
    fn extend(&mut self, bytes: &[u8]) -> Result<(), Code> {
        let end = self
            .len
            .checked_add(bytes.len())
            .filter(|end| *end <= N)
            .ok_or(Code::ResourceLimit)?;
        self.bytes[self.len..end].copy_from_slice(bytes);
        self.len = end;
        Ok(())
    }
}
impl<const N: usize> std::fmt::Write for FixedBytes<N> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let end = self
            .len
            .checked_add(text.len())
            .filter(|end| *end <= N)
            .ok_or(std::fmt::Error)?;
        self.bytes[self.len..end].copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}
fn write_start_receipt(
    job: &Path,
    record: &Record,
    control: &Control,
    deadline: Instant,
) -> Result<(), Code> {
    use std::fmt::Write as _;
    let mut bytes = FixedBytes {
        bytes: [0; 1024],
        len: 0,
    };
    write!(&mut bytes,"{{\"run_id\":\"{}\",\"nonce\":\"{}\",\"seq\":0,\"worker_pid\":{},\"worker_start\":\"{}\",\"phase\":\"spawned_hello_validated\",\"effect_stage\":\"spawned_hello_validated\",\"work_started\":false}}",
        record.view.run_id.0,record.nonce.0,record.worker_pid.ok_or(Code::ProtocolError)?,record.worker_start.as_deref().ok_or(Code::ProtocolError)?).map_err(|_|Code::ResourceLimit)?;
    let mut budget = Budget::new(control, deadline);
    publish_paced(
        job,
        "worker-started.json",
        &record.nonce.0,
        &bytes.bytes[..bytes.len],
        &mut budget,
    )
}

fn prepare_start_frame(
    control: &Control,
    run: &str,
    nonce: &str,
    manifest: &str,
    deadline: Instant,
    revision: u64,
) -> Result<transport::PreparedStart, Code> {
    let mut work = Budget::new(control, deadline);
    let frame = work.unit(|| {
        let remaining = deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(30_000) as u32;
        if remaining == 0 {
            return Err(Code::Deadline);
        }
        transport::PreparedStart::from_parts(run, nonce, manifest, remaining, revision, false)
            .map_err(wire_error)
    })?;
    work.pay_work_debt()?;
    Ok(frame)
}
fn start_frame_current(control: &Control, revision: u64) -> bool {
    !control.is_foreground() && control.activity_version() == revision
}

fn execute_inner(
    binary: &Path,
    job: &Path,
    record: &mut Record,
    control: &Control,
) -> Result<(), Code> {
    let _clock = OwnerClock::new(
        &control.process_owner_thread_cpu_ns,
        &control.process_owner_wall_ns,
    )?;
    if control.cancelled.load(Ordering::Acquire) {
        return Err(Code::Cancelled);
    }
    utility()?;
    if record.launch.is_some() {
        return Err(Code::WorkerLost);
    }
    control.phase(PREPARING);
    let deadline = Instant::now() + Duration::from_secs(30);
    // Manager admits only observed quiescence. A foreground race after that admission
    // pauses this active owner's bounded wall budget; it is not a deferred retry loop.
    let mut preparation = Budget::new(control, deadline);
    let proof = control
        .native_provision
        .get()
        .ok_or(Code::RunnerUnavailable)?;
    preparation.unit(|| entry_worker_eligible(proof))?;
    preparation.unit(|| private_dir(job))?;
    let (canonical, job_directory) = canonical_job(job, &mut preparation)?;
    let _job_directory = Paced::new(job_directory, control);
    let job = canonical.as_path();
    preparation.unit(|| private_dir(&job.join("tmp")))?;
    preparation.unit(|| private_dir(&job.join("output")))?;
    #[cfg(debug_assertions)]
    prepare_private_job_faults(job, record, &mut preparation)?;
    // Actual preallocated pending/output/stderr/reusable scratch requested capacities.
    let mut _protected = control
        .heap
        .reserve(HeapLane::Control, transport::PROTECTED_BYTES)?;
    let buffers = preparation.unit(|| {
        let cpu = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?;
        let mut prepared = Paced::new(Worker::prepare_buffers().map_err(wire_error), control);
        if prepared.is_ok() {
            _protected.allocated();
        }
        let used = bbt_rt::fault::thread_cpu_time()
            .map_err(|_| Code::ResourceLimit)?
            .saturating_sub(cpu);
        control
            .codec_prepare_max_cpu_ns
            .fetch_max(nanos(used), Ordering::Relaxed);
        prepared.take_value()
    })?;
    let mut buffers = Paced::new(buffers, control);
    let mut input = Paced::new(
        preparation.unit(|| Worker::prepare_pipe().map_err(wire_error))?,
        control,
    );
    let mut output = Paced::new(
        preparation.unit(|| Worker::prepare_pipe().map_err(wire_error))?,
        control,
    );
    let mut stderr = Paced::new(
        preparation.unit(|| Worker::prepare_pipe().map_err(wire_error))?,
        control,
    );
    let incarnation = control.launch_incarnation.load(Ordering::Acquire);
    if incarnation == 0 {
        return Err(Code::RunnerUnavailable);
    }
    let birth = preparation.unit(|| Ok(crate::Ulid::new()))?;
    let binding = birth_binding(
        &BirthBinding {
            profile: bootstrap::PROFILE,
            purpose: "inspect-c1",
            probe: false,
            op_id: &record.op_id,
            run_id: &record.view.run_id,
            nonce: &record.nonce,
            incarnation,
            context: Some(&record.context),
            worker_sha256: proof.worker_sha256(),
        },
        &mut preparation,
    )?;
    let frozen = preparation.unit(|| {
        Ok(bootstrap::Frozen::inspect(
            false,
            birth.to_bytes(),
            bootstrap::ulid(record.view.run_id.0.as_bytes()).map_err(|_| Code::BadRequest)?,
            bootstrap::ulid(record.nonce.0.as_bytes()).map_err(|_| Code::BadRequest)?,
            binding,
            incarnation,
        ))
    })?;
    let mut gate = Paced::new(
        preparation.unit(|| bootstrap::Gate::prepare(frozen).map_err(wire_error))?,
        control,
    );
    let (pipe_fd, pipe_device, pipe_inode) = gate.identity();
    record.launch = Some(preparation.unit(|| {
        Ok(LaunchBookmark {
            birth: public::Ulid(birth.to_string()),
            binding: public::Hex(binding.iter().map(|byte| format!("{byte:02x}")).collect()),
            incarnation,
            worker_sha256: public::Hex(proof.worker_sha256().to_owned()),
            pipe_fd,
            pipe_device,
            pipe_inode,
        })
    })?);
    // Keep the existing bounded connection prepared through native birth/PID
    // custody. No connection setup consumes the250ms post-spawn readiness.
    let birth_journal = open_checkpoint(job, record, control, deadline)?;
    save_checkpoint(&birth_journal, record, control, deadline)?;
    let prepared = preparation.unit(|| {
        Worker::prepare_command(
            binary,
            job,
            buffers.take_value(),
            input.take_value(),
            output.take_value(),
            stderr.take_value(),
            gate.take_value(),
        )
        .map_err(wire_error)
    })?;
    let mut prepared = Paced::new(prepared, control);
    preparation.unit(|| prepared.configure_input().map_err(wire_error))?;
    preparation.unit(|| prepared.configure_output().map_err(wire_error))?;
    preparation.unit(|| prepared.configure_stderr().map_err(wire_error))?;
    let mut command = Paced::new(
        preparation.unit(|| prepared.take_command().map_err(wire_error))?,
        control,
    );
    preparation.unit(|| {
        control
            .native_provision
            .get()
            .ok_or(Code::RunnerUnavailable)?
            .still_current()
    })?;
    let mut native = preparation.native_process(&mut command)?;
    // Retain the exact Child/pipe/codec owner BEFORE consulting any timing,
    // missing-clock, cancellation or authority verdict from the native span.
    let (worker, unused_command) = match native.effect.take_value() {
        Ok(child) => prepared.take_value().attach_child(child),
        Err(code) => {
            if let Ok(cpu) = native.sample.cpu {
                control
                    .worker_spawn_cpu_ns
                    .store(nanos(cpu), Ordering::Release);
            }
            let verdict = native.verdict;
            let debt = Budget::for_control(control, deadline).pay_work_debt();
            return Err(verdict.err().or(debt.err()).unwrap_or(code));
        }
    };
    let mut worker = Paced::new(worker, control);
    let mut command = Some(command);
    let mut unused_command = Some(Paced::new(unused_command, control));
    record.worker_pid = Some(worker.pid());
    if let Ok(cpu) = native.sample.cpu {
        control
            .worker_spawn_cpu_ns
            .store(nanos(cpu), Ordering::Release);
    }
    let ready = match native.returned_at.checked_add(bootstrap::WINDOW) {
        Some(ready) => ready.min(deadline),
        None => {
            revoke_bootstrap(&mut worker, control);
            drop(command.take());
            drop(unused_command.take());
            settle_probe(&mut worker, control);
            return Err(Code::ResourceLimit);
        }
    };
    let setup = bootstrap_control(control, || {
        worker
            .install_bootstrap_entry_clock_bounds(native.entry_clock.ok_or(Code::ResourceLimit)?)
            .map_err(wire_error)?;
        worker
            .bootstrap_spawn_returned(native.returned_at, deadline)
            .map_err(wire_error)
    });
    let failed = native.verdict.err().or(setup.err());
    if let Some(code) = failed {
        revoke_bootstrap(&mut worker, control);
        drop(command.take());
        drop(unused_command.take());
        settle_probe(&mut worker, control);
        return Err(code);
    }
    preparation.deadline = ready;
    let before_grant = (|| {
        preparation.pay_work_debt()?;
        // Child is still behind its private gate; parent owns expensive work.
        // Retire ALL parent's child pipe-writers before any permit/EOF wait.
        drop(command.take());
        drop(unused_command.take());
        preparation.pay_work_debt()?;
        record.worker_start =
            Some(preparation.unit(|| transport::process_stamp(worker.pid()).map_err(wire_error))?);
        preparation.unit(|| {
            #[cfg(debug_assertions)]
            if control.private_fixture && proof.control_only_wrapper() {
                // Explicit private protocol-fault fixture only. Its source is
                // never a positive bootstrap/native conformance authority.
                return Ok(());
            }
            proof.child_image_matches(worker.pid())
        })?;
        save_checkpoint(&birth_journal, record, control, ready)?;
        preparation.pay_work_debt()?;
        bootstrap_handshake(&mut worker, control, ready, false)
    })();
    if let Err(code) = before_grant {
        revoke_bootstrap(&mut worker, control);
        drop(command.take());
        drop(unused_command.take());
        settle_probe(&mut worker, control);
        return Err(code);
    }
    preparation.deadline = deadline;
    drop(birth_journal);
    if worker.protected_capacity_bytes() != transport::PROTECTED_BYTES {
        settle_probe(&mut worker, control);
        return Err(Code::ResourceLimit);
    }
    if !proof.control_only_wrapper() {
        control.authorize_native_metrics();
    }
    // Matching paid Hello returns expensive ownership to the parent. Its
    // command/stdio copies were already retired BEFORE granting bootstrap.
    drop(command);
    drop(unused_command);
    #[cfg(debug_assertions)]
    if control.private_fixture {
        match private_fixture_entry(&job.join("test-omit-rest"), false) {
            Ok(value) => control.omit_rest.store(value.is_some(), Ordering::Release),
            Err(code) => {
                settle_probe(&mut worker, control);
                return Err(code);
            }
        }
        match private_fixture_entry(&job.join("test-hot-owner"), false) {
            Ok(Some(_)) => {
                let _ = control.source_owner.set(std::thread::current().id());
            }
            Ok(None) => {}
            Err(code) => {
                settle_probe(&mut worker, control);
                return Err(code);
            }
        }
    }
    // Journal bounds the complete record to16KiB and own catalogue rows to16KiB.
    // This conservative clone declaration allows the latter bound for each source.
    // Source turn vectors additionally retain at most128 String descriptors.
    if record.sources.len() > 32 {
        settle_probe(&mut worker, control);
        return Err(Code::ResourceLimit);
    }
    let _source_copy = match control.heap.reserve(
        HeapLane::Work,
        16 * 1024
            + std::mem::size_of::<Record>()
            + record.sources.len()
                * (16 * 1024
                    + std::mem::size_of::<Bookmark>()
                    + 128 * std::mem::size_of::<String>()),
    ) {
        Ok(reservation) => reservation,
        Err(code) => {
            settle_probe(&mut worker, control);
            return Err(code);
        }
    };
    let mut source_record = record.clone();
    let mut snapshot_result = None;
    let mut stop: Option<(Code, Instant)> = None;
    let mut started = false;
    let mut start_refused = false;
    let mut prepared_start: Option<(u64, transport::PreparedStart)> = None;
    let mut child_work_claimed = false;
    let mut cancel_sent = false;
    let mut activity_sent = 0;
    let mut term_sent = false;
    let mut kill_sent = false;
    let mut exit = None;
    let mut frames = Frames {
        seq: 0,
        previous: None,
        done: None,
    };
    let result = std::thread::scope(|scope| -> Result<(), Code> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut receipt_settling = None;
        let snapshot_owner = scope.spawn(move || {
            let _owner_clock = match OwnerClock::new(
                &control.snapshot_owner_thread_cpu_ns,
                &control.snapshot_owner_wall_ns,
            ) {
                Ok(clock) => clock,
                Err(code) => {
                    let _ = sender.send(Err(code));
                    return;
                }
            };
            let journal = open_checkpoint(job, &source_record, control, deadline);
            let value = journal
                .as_ref()
                .map_err(|code| *code)
                .and_then(|journal| save_checkpoint(journal, &source_record, control, deadline))
                .and_then(|()| snapshot(job, &source_record, control, deadline))
                .and_then(|snapshot| {
                    source_record.view.manifest_sha256 = Some(public::Hex(snapshot.hash.clone()));
                    save_checkpoint(
                        journal.as_ref().map_err(|code| *code)?,
                        &source_record,
                        control,
                        deadline,
                    )?;
                    // Native publication occurs before any child computation.
                    // This seq0 receipt proves process+Hello, never Progress.
                    write_start_receipt(job, &source_record, control, deadline)?;
                    Ok(snapshot)
                });
            drop(journal);
            let _ = sender.send(value);
        });
        let mut previous_reactor_end: Option<Instant> = None;
        loop {
            if let Some(end) = previous_reactor_end {
                control
                    .reactor_min_gap_ns
                    .fetch_min(nanos(end.elapsed()), Ordering::Relaxed);
            }
            let meter = ReactorClock::new(control);
            let mut finish = false;
            if meter.is_err() {
                let since = stop.map_or_else(Instant::now, |(_, since)| since);
                stop = Some((Code::ResourceLimit, since));
            }
            if stop.is_none() && control.cancelled.load(Ordering::Acquire) {
                stop = Some((Code::Cancelled, Instant::now()));
            }
            if stop.is_none() && Instant::now() >= deadline {
                stop = Some((Code::Deadline, Instant::now()));
            }
            if snapshot_result.is_none() {
                match receiver.try_recv() {
                    Ok(result) => {
                        if let Err(code) = &result
                            && stop.is_none()
                        {
                            stop = Some((*code, Instant::now()));
                        }
                        snapshot_result = Some(result);
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if stop.is_none() {
                            stop = Some((Code::WorkerLost, Instant::now()));
                        }
                        snapshot_result = Some(Err(Code::WorkerLost));
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
            if stop.is_none()
                && !started
                && snapshot_owner.is_finished()
                && control.cleanup.debt.remaining_ns() == 0
                && !control.cleanup.debt.held.load(Ordering::Acquire)
                && !control.is_foreground()
                && let Some(Ok(snapshot)) = &snapshot_result
            {
                record.view.manifest_sha256 = Some(public::Hex(snapshot.hash.clone()));
                let revision = control.activity_version();
                // One bounded frozen frame is refreshed only on a monotonic
                // activity edge, never by replaying source/hash/manifest work.
                if prepared_start
                    .as_ref()
                    .is_none_or(|(old, _)| *old != revision)
                {
                    match prepare_start_frame(
                        control,
                        &record.view.run_id.0,
                        &record.nonce.0,
                        &snapshot.hash,
                        deadline,
                        revision,
                    ) {
                        Ok(frame) => prepared_start = Some((revision, frame)),
                        Err(code) => {
                            start_refused = true;
                            worker.close_input();
                            stop = Some((code, Instant::now()));
                        }
                    }
                }
                // Foreground/revision changes during paid preparation return
                // to bounded admission state BEFORE consuming the one final
                // authority callback/Start attempt. No authorization is cached.
                if stop.is_none() && start_frame_current(control, revision) {
                    let frame = &prepared_start
                        .as_ref()
                        .expect("successful bounded Start preparation")
                        .1;
                    control.phase(PREPARING);
                    let before = control.bootstrap_control_cpu_ns.load(Ordering::Acquire);
                    let admission = final_admission_control(control, FinalHandoff::Start, || {
                        effect_guard(control)?;
                        if control.cancelled.load(Ordering::Acquire) {
                            return Err(Code::Cancelled);
                        }
                        if !start_frame_current(control, revision) {
                            return Err(Code::Cancelled);
                        }
                        if Instant::now() >= deadline {
                            return Err(Code::Deadline);
                        }
                        control.begin_child_work()?;
                        child_work_claimed = true;
                        worker.send_prepared_start(frame).map_err(wire_error)?;
                        started = true;
                        activity_sent = revision;
                        Ok(())
                    });
                    control.worker_start_cpu_ns.store(
                        control
                            .bootstrap_control_cpu_ns
                            .load(Ordering::Acquire)
                            .saturating_sub(before),
                        Ordering::Release,
                    );
                    if let Err(code) = admission {
                        // Post-check/clock/write refusal retires stdin. The
                        // original child remains owned through actual reap;
                        // no later control flush can complete a refused Start.
                        start_refused = true;
                        worker.close_input();
                        stop = Some((code, Instant::now()));
                    }
                } else if stop.is_none() {
                    control.phase(PAUSED);
                }
            }
            if let Some((_, since)) = stop {
                worker.discard_pending_output();
                control.cancelled.store(true, Ordering::Release);
                control.phase(CANCELLING);
                if !cancel_sent {
                    if started && !start_refused && worker.flush().is_ok_and(|ready| ready) {
                        let _ = worker.send(&w::WorkerIn::Cancel(w::WorkerCancel {
                            protocol: w::Protocol::V1,
                            run_id: w::Ulid(record.view.run_id.0.clone()),
                            nonce: w::Ulid(record.nonce.0.clone()),
                        }));
                    }
                    cancel_sent = true;
                }
                if since.elapsed() >= Duration::from_millis(100) || !started {
                    worker.close_input();
                }
                if since.elapsed() >= Duration::from_secs(2) && !term_sent {
                    let _ = worker.terminate(false);
                    term_sent = true;
                }
                if since.elapsed() >= Duration::from_secs(4) && !kill_sent {
                    let _ = worker.terminate(true);
                    kill_sent = true;
                }
                if since.elapsed() >= Duration::from_secs(6) {
                    control.phase(QUARANTINED);
                }
            } else if started && frames.done.is_none() {
                let revision = control.activity_version();
                if revision > activity_sent && worker.flush().is_ok_and(|ready| ready) {
                    match worker.send(&w::WorkerIn::Activity(w::Activity {
                        protocol: w::Protocol::V1,
                        run_id: w::Ulid(record.view.run_id.0.clone()),
                        nonce: w::Ulid(record.nonce.0.clone()),
                        revision: w::Number(revision),
                        foreground: control.is_foreground(),
                    })) {
                        Ok(_) => activity_sent = revision,
                        Err(error) => stop = Some((wire_error(error), Instant::now())),
                    }
                }
            }
            if started || stop.is_some() {
                // A fixed batch cannot let saturated progress delay cancellation or another root.
                for _ in 0..4 {
                    if let Ok(clock) = &meter {
                        match clock.batch_due() {
                            Ok(true) => break,
                            Ok(false) => {}
                            Err(_) => {
                                let since = stop.map_or_else(Instant::now, |(_, since)| since);
                                stop = Some((Code::ResourceLimit, since));
                                break;
                            }
                        }
                    }
                    let polled = if meter.is_err() {
                        worker.receive().map_err(wire_error)
                    } else {
                        checked_poll(&mut worker, control)
                    };
                    match polled {
                        Ok(Some(frame)) => {
                            if let Err(code) = frames.observe(frame, record, control)
                                && stop.is_none()
                            {
                                stop = Some((code, Instant::now()));
                            }
                        }
                        Ok(None) => break,
                        Err(code) => {
                            if stop.is_none() {
                                stop = Some((code, Instant::now()));
                            }
                            break;
                        }
                    }
                    if control.cancelled.load(Ordering::Acquire) {
                        break;
                    }
                }
            }
            match worker.try_wait() {
                Ok(Some(status)) => {
                    exit = Some(status);
                    retain_child_usage(&worker, control);
                }
                Ok(None) => {}
                Err(_) => {
                    control.phase(QUARANTINED);
                    if stop.is_none() {
                        stop = Some((Code::WorkerLost, Instant::now()));
                    }
                }
            }
            if exit.is_some() && worker.is_eof() && snapshot_result.is_some() {
                if snapshot_owner.is_finished() {
                    finish = true;
                }
                let since = receipt_settling.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_secs(6) {
                    control.phase(QUARANTINED);
                }
            }
            if frames.done.is_some() && stop.is_none() {
                // Done is not proof of process settlement. EOF asks for no further work.
                worker.close_input();
                if let Some(done) = &frames.done
                    && done.outcome != w::Outcome::Completed
                {
                    stop = Some((
                        done.error
                            .as_ref()
                            .map(|v| wire_error(v.code))
                            .unwrap_or(Code::Cancelled),
                        Instant::now(),
                    ));
                }
            }
            if exit.is_some() && !started && stop.is_none() {
                stop = Some((Code::WorkerLost, Instant::now()));
            }
            if let Ok(clock) = meter
                && clock.finish().is_err()
            {
                let since = stop.map_or_else(Instant::now, |(_, since)| since);
                stop = Some((Code::ResourceLimit, since));
            }
            previous_reactor_end = Some(Instant::now());
            if finish {
                break;
            }
            let rest_wall = Instant::now();
            let rest_cpu = bbt_rt::fault::thread_cpu_time().ok();
            std::thread::sleep(REST);
            control
                .reactor_rest_wall_ns
                .fetch_add(nanos(rest_wall.elapsed()), Ordering::Relaxed);
            if let (Some(before), Ok(after)) = (rest_cpu, bbt_rt::fault::thread_cpu_time()) {
                control
                    .reactor_rest_cpu_ns
                    .fetch_add(nanos(after.saturating_sub(before)), Ordering::Relaxed);
            }
        }
        if child_work_claimed {
            let debt_paid = control.native_metrics_authorized.load(Ordering::Acquire)
                && worker
                    .producer_metrics()
                    .is_some_and(|m| m.work_debt_paid_for(&record.view.run_id.0, &record.nonce.0));
            if let Err(code) = control.end_child_work(debt_paid)
                && stop.is_none()
            {
                stop = Some((code, Instant::now()));
            }
        }
        // A held read is not detached: its actual owner joins before settlement/slot release.
        if snapshot_owner.join().is_err() {
            return Err(Code::WorkerLost);
        }
        if let Some((code, _)) = stop {
            return Err(code);
        }
        let done = frames.done.as_ref().ok_or(Code::WorkerLost)?;
        if done.outcome != w::Outcome::Completed || !exit.is_some_and(|status| status.success()) {
            return Err(Code::WorkerLost);
        }
        if control.cancelled.load(Ordering::Acquire) {
            return Err(Code::Cancelled);
        }
        // Actual reap and both output EOFs precede final receipt validation.
        // Missing/duplicate/malformed diagnostics cannot certify completion;
        // abrupt cancellation above retains unknown metrics honestly.
        let entry = worker.bootstrap_entry_evidence();
        if entry.status != transport::BootstrapEntryEvidenceState::Known {
            return Err(Code::ProtocolError);
        }
        let metrics = worker.producer_metrics().ok_or(Code::ResourceLimit)?;
        if !control.native_metrics_authorized.load(Ordering::Acquire)
            || !control.child_usage_observed.load(Ordering::Acquire)
            || !metrics.valid_native_completion(&record.view.run_id.0, &record.nonce.0, done.pages)
        {
            return Err(Code::ResourceLimit);
        }
        control
            .producer_native_profile_verified
            .store(true, Ordering::Release);
        let snapshot = snapshot_result
            .as_ref()
            .ok_or(Code::WorkerLost)?
            .as_ref()
            .map_err(|code| *code)?;
        validate_output(job, record, &snapshot.manifest, done, control, deadline)
    });
    let captured = bootstrap_control(control, || {
        let _ = control
            .bootstrap_entry
            .set(worker.bootstrap_entry_evidence());
        Ok(())
    });
    result.and(captured)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;

    const SESSION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const TURN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

    fn private_job() -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix("c1-job-")
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap()
    }

    fn envelope(seq: u64, event: Value) -> Value {
        json!({"v":{"major":1,"minor":3},"session":SESSION,"seq":seq,"at":seq,
            "epoch":1,"ts_ms":seq,"event":event})
    }
    fn events(closed: bool) -> Vec<Value> {
        let mut events = vec![
            envelope(
                1,
                json!({"type":"session_started","meta":{"session":SESSION,
                "cwd":"/private/tmp/dream-fixture","protocol":{"major":1,"minor":3},
                "profile":null,"fork":null,"harness":"fixture","created_ms":0,
                "source":null,"vcs":{"repo":"github.com/fixture/dream"}}}),
            ),
            envelope(2, json!({"type":"turn_queued","turn":TURN})),
            envelope(3, json!({"type":"turn_started","turn":TURN})),
            envelope(
                4,
                json!({"type":"turn_ended","turn":TURN,"reason":{"reason":"completed"},"usage":null}),
            ),
        ];
        if closed {
            events.push(envelope(5, json!({"type":"session_closed"})));
        }
        events
    }
    struct Source {
        root: tempfile::TempDir,
        path: PathBuf,
        bytes: Vec<u8>,
    }
    fn source(events: &[Value], crlf: bool) -> Source {
        let root = tempfile::tempdir().unwrap();
        private_dir(&root.path().join("sessions")).unwrap();
        let path = root
            .path()
            .join("sessions")
            .join(format!("{SESSION}.jsonl"));
        let mut bytes = Vec::new();
        for event in events {
            bytes.extend(serde_json::to_vec(event).unwrap());
            bytes.extend(if crlf {
                b"\r\n".as_slice()
            } else {
                b"\n".as_slice()
            });
        }
        create(&path).unwrap().write_all(&bytes).unwrap();
        Source { root, path, bytes }
    }
    fn close(source: &Source) -> Bookmark {
        let mut reader = Reader::open(source.root.path(), SESSION, 1).unwrap();
        for _ in 0..32 {
            if let Some(bookmark) = reader.poll().unwrap().0 {
                return bookmark;
            }
        }
        panic!("fixture close was not observed")
    }
    fn record(sources: Vec<Bookmark>) -> Record {
        let id = public::Ulid(SESSION.into());
        Record {
            trigger: true,
            identity_fence: None,
            worker_pid: None,
            worker_start: None,
            launch: None,
            started: false,
            owner_incarnation: id.clone(),
            context: public::Context {
                account: public::Id("fixture".into()),
                account_epoch: public::Number(1),
                seat: id.clone(),
                generation: public::Number(1),
                grant_epoch: public::Number(1),
                swarm: id.clone(),
                swarm_revision: public::Number(1),
                workspace: public::Id("fixture".into()),
                workspace_revision: public::Number(1),
                root: public::Id("fixture".into()),
                root_revision: public::Number(1),
                provider: public::Id("fixture".into()),
                provider_epoch: public::Number(1),
            },
            op_id: id.clone(),
            nonce: public::Ulid(TURN.into()),
            sources,
            request: Vec::new(),
            view: public::RunView {
                run_id: id,
                revision: public::Number(1),
                state: State::Deferred,
                settled: false,
                request_sha256: public::Hex("0".repeat(64)),
                manifest_sha256: None,
                result_sha256: None,
                pages: 0,
                error: None,
            },
        }
    }

    #[test]
    fn closed_only_reader_keeps_turn_end_idle_and_native_eof_ineligible() {
        let source = source(&events(false), false);
        let mut reader = Reader::open(source.root.path(), SESSION, 1).unwrap();
        let mut quiescent = false;
        for _ in 0..16 {
            let (bookmark, foreground) = reader.poll().unwrap();
            assert!(
                bookmark.is_none(),
                "turn end/EOF is not authoritative session closure"
            );
            quiescent |= foreground == Some(false);
        }
        assert!(
            quiescent,
            "foreground priority and source eligibility are separate facts"
        );
        assert_eq!(std::fs::read(&source.path).unwrap(), source.bytes);
        let empty = self::source(&[], false);
        let mut reader = Reader::open(empty.root.path(), SESSION, 1).unwrap();
        let empty = reader.poll().unwrap();
        assert!(empty.0.is_none() && empty.1.is_none());
    }

    #[test]
    fn durable_close_retains_exact_raw_crlf_digest_and_one_cursor_bookmark() {
        let source = source(&events(true), true);
        let mut reader = Reader::open(source.root.path(), SESSION, 1).unwrap();
        let mut bookmarks = Vec::new();
        for _ in 0..20 {
            if let Some(bookmark) = reader.poll().unwrap().0 {
                bookmarks.push(bookmark);
            }
        }
        assert_eq!(bookmarks.len(), 1);
        let bookmark = &bookmarks[0];
        assert_eq!(bookmark.closed_seq, 5);
        assert_eq!(bookmark.cursor, source.bytes.len() as u64);
        assert_eq!(bookmark.len, bookmark.cursor);
        let final_line = source.bytes[..source.bytes.len() - 1]
            .rsplit(|b| *b == b'\n')
            .next()
            .unwrap();
        assert_eq!(bookmark.closed_line_sha256, digest(final_line));
        assert!(final_line.ends_with(b"\r"));
        assert_ne!(
            bookmark.closed_line_sha256,
            digest(final_line.strip_suffix(b"\r").unwrap())
        );
        assert_eq!(std::fs::read(&source.path).unwrap(), source.bytes);
    }

    #[test]
    fn fork_header_refuses_explicitly_before_inherited_history_or_any_worker() {
        let mut values = events(true);
        values[0]["event"]["meta"]["fork"] = json!([TURN, 3]);
        let source = source(&values, false);
        let mut reader = Reader::open(source.root.path(), SESSION, 1).unwrap();
        let mut refused = false;
        for _ in 0..8 {
            match reader.poll() {
                Err(Code::UnsupportedIr) => {
                    refused = true;
                    break;
                }
                Ok((bookmark, _)) => assert!(bookmark.is_none()),
                Err(code) => panic!("unexpected fork refusal {code:?}"),
            }
        }
        assert!(refused, "fork header never proceeds to a C1 worker");
        assert_eq!(std::fs::read(&source.path).unwrap(), source.bytes);
    }

    #[test]
    fn snapshot_rejects_mutation_after_retained_close_without_rewriting_source() {
        let source = source(&events(true), false);
        let bookmark = close(&source);
        OpenOptions::new()
            .append(true)
            .open(&source.path)
            .unwrap()
            .write_all(b" ")
            .unwrap();
        let changed = std::fs::read(&source.path).unwrap();
        let job = private_job();
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let result = snapshot(
            job.path(),
            &record(vec![bookmark]),
            &control,
            Instant::now() + Duration::from_secs(2),
        );
        assert!(matches!(result, Err(Code::SourceChanged)));
        assert_eq!(std::fs::read(&source.path).unwrap(), changed);
        assert!(!job.path().join("input/manifest.json").exists());
    }

    #[test]
    #[ignore = "load-sensitive per-unit CPU budget (ResourceLimit at load ~18); re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp (93377961366081)"]
    fn custody_copies_exact_ir_and_only_typed_blob_carriers_into_private_new_inodes() {
        let blob_bytes = b"private fixture\nwith zero\0byte";
        let hash = digest(blob_bytes);
        let mut values = events(true);
        values.insert(
            3,
            envelope(
                4,
                json!({"type":"item_started","turn":TURN,
            "item":TURN,"kind":"image","parent":null,"head":null}),
            ),
        );
        values.insert(4, envelope(5, json!({"type":"item_completed","item":TURN,"status":"completed",
            "body":{"kind":"image","mime":"image/png","blob":{"sha256":hash,"len":blob_bytes.len(),"mime":"image/png"}},
            "ext":{"blob":{"sha256":"f".repeat(64),"len":99,"mime":"ignored/extension"}}})));
        for (i, value) in values.iter_mut().enumerate() {
            value["seq"] = json!(i + 1);
            value["at"] = json!(i + 1);
        }
        let source = source(&values, true);
        let original_blobs = source.root.path().join("sessions").join(SESSION);
        private_dir(&original_blobs).unwrap();
        let original_blobs = original_blobs.join("blobs");
        private_dir(&original_blobs).unwrap();
        let original_blob = original_blobs.join(&hash);
        create(&original_blob)
            .unwrap()
            .write_all(blob_bytes)
            .unwrap();
        let bookmark = close(&source);
        let job = private_job();
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let snapshot = snapshot(
            job.path(),
            &record(vec![bookmark]),
            &control,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
        assert_eq!(
            control.snapshot_tiny_records.load(Ordering::Acquire),
            values.len() as u64,
            "the CRLF/typed-blob fixture must use the bounded tiny record path",
        );
        assert_eq!(control.snapshot_staged_records.load(Ordering::Acquire), 0);
        assert_eq!(snapshot.manifest.sources.len(), 1);
        assert_eq!(
            snapshot.manifest.sources[0].blobs.len(),
            1,
            "untyped extension data is not a BlobRef"
        );
        assert_eq!(snapshot.manifest.sources[0].blobs[0].sha256.0, hash);
        assert_eq!(snapshot.manifest.sources[0].sha256.0, digest(&source.bytes));
        let copied = job.path().join("input").join(format!("{SESSION}.jsonl"));
        assert_eq!(std::fs::read(&copied).unwrap(), source.bytes);
        assert_ne!(
            std::fs::metadata(&copied).unwrap().ino(),
            std::fs::metadata(&source.path).unwrap().ino()
        );
        assert_eq!(std::fs::metadata(&copied).unwrap().mode() & 0o777, 0o600);
        assert_eq!(
            std::fs::read(
                job.path()
                    .join("input")
                    .join(SESSION)
                    .join("blobs")
                    .join(&hash)
            )
            .unwrap(),
            blob_bytes
        );
        assert_eq!(std::fs::read(&original_blob).unwrap(), blob_bytes);
        assert_eq!(std::fs::read(&source.path).unwrap(), source.bytes);
        let manifest = std::fs::read(job.path().join("input/manifest.json")).unwrap();
        assert_eq!(digest(&manifest), snapshot.hash);
        assert!(!manifest.ends_with(b"\n"));
    }

    #[test]
    fn immutable_transfer_combines_read_hash_write_and_refuses_oversize_before_effects() {
        let job = private_job();
        let bytes = [b'x'; 8192];
        let source = job.path().join("source");
        create(&source).unwrap().write_all(&bytes).unwrap();
        let mut input = File::open(&source).unwrap();
        let mut output = create(&job.path().join("copy")).unwrap();
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
        let mut hash = Sha256::new();
        let mut chunk = [0; CHUNK];
        assert_eq!(
            copy_chunk(
                &mut input,
                &mut output,
                &mut hash,
                &mut chunk,
                bytes.len() as u64,
                Code::SourceUnavailable,
                Code::SourceChanged,
                &control.source_copy_unit_max_cpu_ns,
                &mut budget,
            ),
            Ok(bytes.len()),
        );
        assert_eq!(control.work_unit_count.load(Ordering::Acquire), 1);
        assert_eq!(std::fs::read(job.path().join("copy")).unwrap(), bytes);
        assert_eq!(format!("{:x}", hash.finalize()), digest(&bytes));
        assert!(control.source_copy_unit_max_cpu_ns.load(Ordering::Acquire) > 0);

        input.rewind().unwrap();
        let mut refused = create(&job.path().join("refused")).unwrap();
        let mut hash = Sha256::new();
        assert_eq!(
            copy_chunk(
                &mut input,
                &mut refused,
                &mut hash,
                &mut chunk,
                (bytes.len() - 1) as u64,
                Code::SourceUnavailable,
                Code::SourceChanged,
                &control.source_copy_unit_max_cpu_ns,
                &mut budget,
            ),
            Err(Code::SourceChanged),
        );
        assert_eq!(refused.metadata().unwrap().len(), 0);
        assert_eq!(format!("{:x}", hash.finalize()), digest(b""));
        assert_eq!(std::fs::read(source).unwrap(), bytes);
    }

    #[test]
    fn staged_text_blocks_still_each_visit_the_bounded_work_gate() {
        let event = json!({"type":"item_completed","body":{"kind":"user_message","blocks":[
            {"type":"text","text":"one"},
            {"type":"text","text":"two"},
            {"type":"text","text":"three"}
        ]}});
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
        let mut blobs = BTreeMap::new();
        references(&event, &mut blobs, &mut budget).unwrap();
        assert!(blobs.is_empty());
        assert_eq!(control.work_unit_count.load(Ordering::Acquire), 3);
        assert_eq!(control.rest_violations.load(Ordering::Acquire), 0);
    }

    #[test]
    fn inclusive_descriptor_requires_provisioned_authority_and_verified_terminal_capture() {
        let control = Control::default();
        control.producer_metrics[45].store(1_500_000, Ordering::Release);
        control.producer_metrics[46].store(2_000_000, Ordering::Release);
        assert!(!approved_descriptor_capture(&control, 1_500_000));
        control
            .producer_native_profile_verified
            .store(true, Ordering::Release);
        assert!(
            !approved_descriptor_capture(&control, 1_500_000),
            "a parsed/profile verdict grants no executable authority"
        );
        // Explicit component fixture authorization; production obtains this
        // separate grant only from the source/compiler/features/image receipt.
        control.authorize_native_metrics();
        assert!(approved_descriptor_capture(&control, 1_500_000));
        assert!(!approved_descriptor_capture(&control, 1_400_000));
        assert!(!approved_descriptor_capture(&control, 3_000_001));
        control
            .producer_native_profile_verified
            .store(false, Ordering::Release);
        assert!(!approved_descriptor_capture(&control, 1_500_000));
    }

    fn progress(record: &Record, seq: u64) -> w::WorkerOut {
        w::WorkerOut::Progress(w::Progress {
            protocol: w::Protocol::V1,
            run_id: w::Ulid(record.view.run_id.0.clone()),
            nonce: w::Ulid(record.nonce.0.clone()),
            seq: w::Number(seq),
            phase: w::Phase::Replay,
            bytes: w::Number(0),
            envelopes: w::Number(0),
            cpu_ns: w::Number(0),
        })
    }
    fn frames() -> Frames {
        Frames {
            seq: 0,
            previous: None,
            done: None,
        }
    }

    #[test]
    fn framed_owner_rejects_nonce_gap_conflicting_duplicate_and_extra_done() {
        let record = record(Vec::new());
        let control = Control::default();
        let first = progress(&record, 1);
        let mut accepted = frames();
        accepted.observe(first.clone(), &record, &control).unwrap();
        accepted.observe(first, &record, &control).unwrap();
        assert_eq!(
            accepted.seq, 1,
            "identical progress retransmission is harmless"
        );
        let mut conflicting = progress(&record, 1);
        if let w::WorkerOut::Progress(v) = &mut conflicting {
            v.bytes = w::Number(1);
        }
        assert_eq!(
            accepted.observe(conflicting, &record, &control),
            Err(Code::ProtocolError)
        );
        assert_eq!(
            frames().observe(progress(&record, 2), &record, &control),
            Err(Code::ProtocolError)
        );
        let mut foreign = progress(&record, 1);
        if let w::WorkerOut::Progress(v) = &mut foreign {
            v.nonce = w::Ulid(SESSION.into());
        }
        assert_eq!(
            frames().observe(foreign, &record, &control),
            Err(Code::ProtocolError)
        );
        let done = w::WorkerOut::Done(w::Done {
            protocol: w::Protocol::V1,
            run_id: w::Ulid(record.view.run_id.0.clone()),
            nonce: w::Ulid(record.nonce.0.clone()),
            seq: w::Number(2),
            outcome: w::Outcome::Completed,
            result_sha256: Some(w::Hex("0".repeat(64))),
            pages: 1,
            error: None,
        });
        accepted.observe(done.clone(), &record, &control).unwrap();
        assert_eq!(
            accepted.observe(done, &record, &control),
            Err(Code::ProtocolError)
        );
        assert!(
            !control.settled.load(Ordering::Acquire),
            "Done never settles a live process owner"
        );
    }

    #[test]
    fn immutable_page_status_detects_changed_bytes_after_completed_publication() {
        let job = private_job();
        private_dir(&job.path().join("output")).unwrap();
        let mut record = record(Vec::new());
        record.view.state = State::Completed;
        record.view.settled = true;
        record.view.pages = 1;
        record.view.manifest_sha256 = Some(public::Hex("0".repeat(64)));
        let page = w::Page {
            schema: w::PageSchema::V1,
            run_id: w::Ulid(SESSION.into()),
            manifest_sha256: w::Hex("0".repeat(64)),
            index: 0,
            rows: Vec::new(),
            next: None,
        };
        let bytes = serialize(&page).unwrap();
        let page_path = job.path().join("output/page-000000.json");
        create(&page_path).unwrap().write_all(&bytes).unwrap();
        let result = w::ResultDescriptor {
            schema: w::ResultSchema::V1,
            run_id: w::Ulid(SESSION.into()),
            nonce: w::Ulid(TURN.into()),
            manifest_sha256: w::Hex("0".repeat(64)),
            algorithm: w::Algorithm::D0C1,
            source_count: 0,
            episode_count: 0,
            cluster_count: 0,
            max_poll_cpu_ns: w::Number(0),
            knowledge: w::Knowledge::NotRequested,
            trust: w::Trust::NotEvaluated,
            pages: vec![w::PageRef {
                index: 0,
                len: w::Number(bytes.len() as u64),
                sha256: w::Hex(digest(&bytes)),
            }],
        };
        let bytes = serialize(&result).unwrap();
        record.view.result_sha256 = Some(public::Hex(digest(&bytes)));
        create(&job.path().join("output/result.json"))
            .unwrap()
            .write_all(&bytes)
            .unwrap();
        assert_eq!(
            read_page_guarded(
                job.path(),
                &record,
                0,
                Some(PreparedGuard {
                    callback: Arc::new(|| Err(Code::StaleContext)),
                    setup_cpu_ns: 0,
                    setup_max_unit_cpu_ns: 0,
                    retire_session: None,
                })
            ),
            Err(Code::StaleContext)
        );
        assert_eq!(read_page(job.path(), &record, 0).unwrap().index, 0);
        assert_eq!(read_page(job.path(), &record, 1), Err(Code::BadRequest));
        OpenOptions::new()
            .append(true)
            .open(&page_path)
            .unwrap()
            .write_all(b" ")
            .unwrap();
        assert_eq!(read_page(job.path(), &record, 0), Err(Code::ProtocolError));
    }

    #[test]
    fn unavailable_runner_precedes_every_source_open_and_manifest_creation() {
        let source = source(&events(true), false);
        let mut bookmark = close(&source);
        bookmark.path = source.root.path().join("unavailable-source");
        let job = private_job();
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut record = record(vec![bookmark]);
        assert_eq!(
            execute_inner(
                &job.path().join("missing-runner"),
                job.path(),
                &mut record,
                &control
            ),
            Err(Code::RunnerUnavailable)
        );
        assert!(
            !job.path().join("input").exists(),
            "no input/manifest work before matching Hello"
        );
        assert_eq!(std::fs::read(&source.path).unwrap(), source.bytes);
    }

    #[test]
    fn foreground_gate_has_independent_cancel_and_never_admits_a_unit() {
        let control = Control::default();
        let worked = AtomicBool::new(false);
        std::thread::scope(|scope| {
            let handle = scope.spawn(|| {
                let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(2));
                budget.unit(|| {
                    worked.store(true, Ordering::Release);
                    Ok(())
                })
            });
            let until = Instant::now() + Duration::from_millis(100);
            while control.state.load(Ordering::Acquire) != PAUSED && Instant::now() < until {
                std::thread::yield_now();
            }
            assert_eq!(state(control.state.load(Ordering::Acquire)), State::Paused);
            assert!(!worked.load(Ordering::Acquire));
            control.cancelled.store(true, Ordering::Release);
            assert_eq!(handle.join().unwrap(), Err(Code::Cancelled));
        });
        assert!(!worked.load(Ordering::Acquire));
    }

    #[test]
    fn retained_open_cursor_resumes_exact_append_and_refuses_changed_raw_anchor() {
        let source = source(&events(false), false);
        let mut reader = Reader::open(source.root.path(), SESSION, 1).unwrap();
        for _ in 0..16 {
            if reader.poll().unwrap().1 == Some(false) {
                break;
            }
        }
        let bookmark = reader.bookmark().unwrap();
        assert_eq!(bookmark.closed_seq, 0);
        assert_eq!(bookmark.observed_seq, 4);
        assert_eq!(bookmark.cursor, source.bytes.len() as u64);
        let last = envelope(5, json!({"type":"session_closed"}));
        let mut appended = serde_json::to_vec(&last).unwrap();
        appended.push(b'\n');
        OpenOptions::new()
            .append(true)
            .open(&source.path)
            .unwrap()
            .write_all(&appended)
            .unwrap();
        let mut resumed = Reader::resume(&bookmark).unwrap();
        assert_eq!(
            resumed.seq, 4,
            "no prefix replay or source discovery on resume"
        );
        let closed = (0..16).find_map(|_| resumed.poll().unwrap().0).unwrap();
        assert_eq!(closed.closed_seq, 5);
        assert_eq!(closed.cursor, bookmark.cursor + appended.len() as u64);
        let mut original = OpenOptions::new().write(true).open(&source.path).unwrap();
        original
            .seek(SeekFrom::Start(
                bookmark.cursor - bookmark.last_line_len as u64,
            ))
            .unwrap();
        original.write_all(b" ").unwrap();
        assert!(matches!(
            Reader::resume(&bookmark),
            Err(Code::SourceChanged)
        ));
    }

    #[test]
    fn packed_activity_is_the_only_production_work_gate_and_retains_aba_revision() {
        let latch = Arc::new(AtomicU64::new(2)); // revision one, quiescent A1
        let control = Control {
            activity_latch: Some(latch.clone()),
            ..Control::default()
        };
        assert!(!control.is_foreground());
        assert_eq!(control.activity_version(), 1);
        control.foreground.store(false, Ordering::Release);
        latch.store(5, Ordering::Release); // revision two, foreground B2
        assert!(
            control.is_foreground(),
            "legacy projection cannot clear foreground work"
        );
        latch.store(6, Ordering::Release); // revision three, quiescent A3
        assert!(!control.is_foreground());
        assert_eq!(control.activity_version(), 3);
        assert_eq!(
            latch.compare_exchange(2, 8, Ordering::AcqRel, Ordering::Acquire),
            Err(6)
        );
        assert_eq!(
            control.activity_version(),
            3,
            "old A1 cannot publish over A3"
        );
        latch.store(
            super::super::harness_status::CLOSED_EXIT_ACTIVITY | 8,
            Ordering::Release,
        );
        assert!(!control.is_foreground());
        assert_eq!(
            control.activity_version(),
            4,
            "local closure/exit proof is not a wire revision bit"
        );
    }

    #[test]
    fn requested_heap_exhaustion_refuses_realloc_before_effect_and_keeps_control_available() {
        let control = Control::default();
        let mut bytes = HeapBuffer::new(&control.heap, HeapLane::Work, 4096).unwrap();
        bytes.extend(b"original bytes").unwrap();
        let _rest = control
            .heap
            .reserve(HeapLane::Work, WORK_HEAP as usize - 4096)
            .unwrap();
        let before = control.heap.counters(HeapLane::Work);
        assert_eq!(bytes.replace_capacity(8192), Err(Code::ResourceLimit));
        assert_eq!(bytes.capacity(), 4096);
        assert_eq!(bytes.as_slice(), b"original bytes");
        assert_eq!(
            control.heap.counters(HeapLane::Work),
            before,
            "no second allocation or cumulative growth on refusal"
        );
        let mut status = HeapBuffer::new(&control.heap, HeapLane::Control, 256).unwrap();
        status.extend(b"{\"ok\":true}").unwrap();
        control.cancelled.store(true, Ordering::Release);
        control.phase(CANCELLING);
        assert!(control.cancelled.load(Ordering::Acquire));
        assert_eq!(
            state(control.state.load(Ordering::Acquire)),
            State::Cancelling
        );
        assert_eq!(status.as_slice(), b"{\"ok\":true}");
        assert_eq!(control.heap.counters(HeapLane::Control).live, 256);
    }

    #[test]
    fn successful_realloc_charges_old_and_new_until_old_storage_is_freed() {
        let heap = Arc::new(Heap::default());
        let mut bytes = HeapBuffer::new(&heap, HeapLane::Work, 4096).unwrap();
        bytes.extend(b"owned").unwrap();
        bytes.replace_capacity(8192).unwrap();
        let stats = heap.counters(HeapLane::Work);
        assert_eq!(stats.live, 8192);
        assert_eq!(stats.reserved, 8192);
        assert_eq!(stats.cumulative, 4096 + 8192);
        assert_eq!(stats.peak, 4096 + 8192);
        assert_eq!(stats.reserved_peak, 4096 + 8192);
        assert_eq!(stats.reserved_cumulative, 4096 + 8192);
        assert_eq!(bytes.as_slice(), b"owned");
        bytes.clear();
        assert!(bytes.is_empty());
        drop(bytes);
        assert_eq!(heap.counters(HeapLane::Work).live, 0);
        assert_eq!(heap.counters(HeapLane::Work).reserved, 0);
        assert_eq!(heap.counters(HeapLane::Work).cumulative, 4096 + 8192);
    }

    #[test]
    fn serde_overlap_is_reserved_before_general_materialization() {
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let bytes = br#"{"a":[1,2,3],"s":"payload"}"#;
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(2));
        let _first = budget.json(bytes).unwrap();
        let used = control.heap.counters(HeapLane::Work).reserved;
        let _rest = control
            .heap
            .reserve(HeapLane::Work, (WORK_HEAP - used) as usize)
            .unwrap();
        assert!(matches!(
            canonical_decode::<Value>(bytes, &control.heap),
            Err(Code::ResourceLimit)
        ));
        assert_eq!(
            control.heap.counters(HeapLane::Work).live,
            0,
            "declared serde entries are not mislabeled actual tracked buffers"
        );
        assert_eq!(control.heap.counters(HeapLane::Work).cumulative, 0);
        assert!(
            control.heap.counters(HeapLane::Work).reserved_cumulative > 0,
            "declared serde credits remain distinct from tracked allocated capacity"
        );
        let _emergency = HeapBuffer::new(&control.heap, HeapLane::Control, 4096).unwrap();
    }

    #[test]
    fn structural_json_limits_reject_before_serde_and_count_values_plus_keys() {
        let bounds =
            transport::preflight_json(br#"{"a":[1,2],"b":"text"}"#, transport::JsonShape::Any)
                .unwrap();
        assert_eq!((bounds.values, bounds.keys), (5, 2));
        let deep = format!("{}0{}", "[".repeat(33), "]".repeat(33));
        assert!(matches!(
            transport::preflight_json(deep.as_bytes(), transport::JsonShape::Any),
            Err(w::Code::ResourceLimit)
        ));
        let wide = format!("[{}]", vec!["0"; 4097].join(","));
        assert!(matches!(
            transport::preflight_json(wide.as_bytes(), transport::JsonShape::Any),
            Err(w::Code::ResourceLimit)
        ));
        let group = format!("[{}]", vec!["0"; 4096].join(","));
        let many = format!("[{}]", vec![group; 16].join(","));
        assert!(matches!(
            transport::preflight_json(many.as_bytes(), transport::JsonShape::Any),
            Err(w::Code::ResourceLimit)
        ));
        assert!(matches!(
            transport::preflight_json(br#"{"x":{}}"#, transport::JsonShape::WorkerIn),
            Err(w::Code::ResourceLimit)
        ));
        assert!(matches!(
            transport::preflight_json(br#"{"x":[]}"#, transport::JsonShape::WorkerIn),
            Err(w::Code::ResourceLimit)
        ));
    }

    #[test]
    fn cooperative_preflight_yields_without_rescan_and_cancel_stops_before_materialization() {
        let bytes = format!("{{\"payload\":\"{}\"}}", "a".repeat(128 * 1024));
        let mut checkpoints = 0;
        let bounds = transport::preflight_json_cooperative(
            bytes.as_bytes(),
            transport::JsonShape::Any,
            || {
                checkpoints += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(bounds.string_bytes, 128 * 1024 + "payload".len());
        assert!(checkpoints >= 32);
        let mut stopped = 0;
        assert!(matches!(
            transport::preflight_json_cooperative(
                bytes.as_bytes(),
                transport::JsonShape::Any,
                || {
                    stopped += 1;
                    Err(w::Code::Cancelled)
                }
            ),
            Err(w::Code::Cancelled)
        ));
        assert_eq!(stopped, 1);
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(3));
        let _declared = budget.json(bytes.as_bytes()).unwrap();
        let parsed: Paced<Value> = budget.parse(bytes.as_bytes()).unwrap();
        assert_eq!(parsed["payload"].as_str().unwrap().len(), 128 * 1024);
        assert!(
            control.work_unit_count.load(Ordering::Acquire)
                >= (bytes.len().div_ceil(64 * 1024) * 2) as u64
        );
        assert_eq!(control.unit_overruns.load(Ordering::Acquire), 0);
        assert_eq!(control.rest_violations.load(Ordering::Acquire), 0);
    }

    #[test]
    fn canonical_lexical_checks_order_escapes_numbers_and_paces_large_descriptor() {
        for bytes in [
            r#"{"a":1,"b":"\u0001\n\\\"é"}"#.as_bytes(),
            br#"{"next":null,"rows":[]}"#.as_slice(),
        ] {
            transport::preflight_json(bytes, transport::JsonShape::Canonical).unwrap();
        }
        for bytes in [
            br#"{ "a":1}"#.as_slice(),
            br#"{"b":1,"a":2}"#,
            br#"{"a":1,"a":2}"#,
            br#"{"a":"\/"}"#,
            br#"{"a":"\u0061"}"#,
            br#"{"a":"\u000a"}"#,
            br#"{"a":"\u001F"}"#,
            br#"{"a":1.0}"#,
            br#"{"a":-0}"#,
            br#"{"a":1e0}"#,
        ] {
            assert!(
                transport::preflight_json(bytes, transport::JsonShape::Canonical).is_err(),
                "bad canonical bytes {bytes:?}"
            );
        }
        let descriptor = w::ResultDescriptor {
            schema: w::ResultSchema::V1,
            run_id: w::Ulid(SESSION.into()),
            nonce: w::Ulid(TURN.into()),
            manifest_sha256: w::Hex("0".repeat(64)),
            algorithm: w::Algorithm::D0C1,
            source_count: 1,
            episode_count: 4096,
            cluster_count: 0,
            max_poll_cpu_ns: w::Number(0),
            knowledge: w::Knowledge::NotRequested,
            trust: w::Trust::NotEvaluated,
            pages: (0..4096)
                .map(|index| w::PageRef {
                    index,
                    len: w::Number(1),
                    sha256: w::Hex("0".repeat(64)),
                })
                .collect(),
        };
        let bytes = serialize(&descriptor).unwrap();
        assert!(bytes.len() > 256 * 1024);
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(10));
        let decoded: Paced<w::ResultDescriptor> =
            decode_canonical(&bytes, &mut budget, false).unwrap();
        assert_eq!(decoded.pages.len(), 4096);
        assert!(
            control.work_unit_count.load(Ordering::Acquire)
                >= (bytes.len().div_ceil(64 * 1024) * 2) as u64
        );
        assert_eq!(control.unit_overruns.load(Ordering::Acquire), 0);
        assert_eq!(control.rest_violations.load(Ordering::Acquire), 0);
    }

    #[test]
    fn current_authority_guard_refuses_stale_and_aba_before_unit_effect() {
        let revision = Arc::new(AtomicU64::new(1));
        let observed = revision.clone();
        let control = Control {
            authority_guard: Some(Arc::new(move || {
                if observed.load(Ordering::Acquire) == 1 {
                    Ok(())
                } else {
                    Err(Code::StaleContext)
                }
            })),
            ..Control::default()
        };
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(2));
        let effects = AtomicU64::new(0);
        budget
            .unit(|| {
                effects.fetch_add(1, Ordering::Relaxed);
                Ok(())
            })
            .unwrap();
        for value in [2, 3] {
            // A1 ->B2 ->A3 remains stale for the original capture.
            revision.store(value, Ordering::Release);
            assert_eq!(
                budget.unit(|| {
                    effects.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }),
                Err(Code::StaleContext)
            );
        }
        assert_eq!(effects.load(Ordering::Acquire), 1);
        assert!(control.max_unit_cpu_ns.load(Ordering::Acquire) > 0);
    }

    #[test]
    fn slow_authority_control_is_counted_and_refuses_work_before_effect() {
        let control = Control {
            authority_guard: Some(Arc::new(|| {
                let cpu = bbt_rt::fault::thread_cpu_time().unwrap();
                while bbt_rt::fault::thread_cpu_time()
                    .unwrap()
                    .saturating_sub(cpu)
                    < Duration::from_millis(2)
                {
                    std::hint::spin_loop();
                }
                Ok(())
            })),
            ..Control::default()
        };
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(2));
        let effect = AtomicBool::new(false);
        assert_eq!(
            budget.unit(|| {
                effect.store(true, Ordering::Release);
                Ok(())
            }),
            Err(Code::ResourceLimit)
        );
        assert!(!effect.load(Ordering::Acquire));
        assert_eq!(control.unit_overruns.load(Ordering::Acquire), 1);
        assert!(control.max_unit_cpu_ns.load(Ordering::Acquire) >= 2_000_000);
    }
    #[test]
    fn prepared_authority_installs_only_once_and_setup_overrun_has_no_effect() {
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        assert!(control.authority_prepared_guard.get().is_none());
        assert_eq!(
            control.install_authority_guard(PreparedGuard {
                callback: Arc::new(|| Ok(())),
                setup_cpu_ns: 1_000_001,
                setup_max_unit_cpu_ns: 1_000_001,
                retire_session: None,
            }),
            Err(Code::ResourceLimit)
        );
        assert!(control.authority_prepared_guard.get().is_none());
        control
            .install_authority_guard(PreparedGuard {
                callback: Arc::new(|| Err(Code::StaleContext)),
                setup_cpu_ns: 123,
                setup_max_unit_cpu_ns: 123,
                retire_session: None,
            })
            .unwrap();
        assert_eq!(
            control.install_authority_guard(PreparedGuard {
                callback: Arc::new(|| Ok(())),
                setup_cpu_ns: 456,
                setup_max_unit_cpu_ns: 456,
                retire_session: None,
            }),
            Err(Code::Busy)
        );
        assert_eq!(control.authority_setup_cpu_ns.load(Ordering::Acquire), 123);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(1));
        let effect = AtomicBool::new(false);
        assert_eq!(
            budget.unit(|| {
                effect.store(true, Ordering::Release);
                Ok(())
            }),
            Err(Code::StaleContext)
        );
        assert!(!effect.load(Ordering::Acquire));
    }

    #[test]
    fn dream_c1_cooperative_preparation_preserves_cumulative_cost_and_paid_phase_maxima() {
        let quantum = JournalQuantum::preparation().unwrap();
        let before = bbt_rt::fault::thread_cpu_time().unwrap();
        quantum
            .public_prelude(|checkpoint| {
                for _ in 0..8 {
                    let start = bbt_rt::fault::thread_cpu_time().unwrap();
                    while bbt_rt::fault::thread_cpu_time()
                        .unwrap()
                        .checked_sub(start)
                        .unwrap()
                        < Duration::from_micros(200)
                    {
                        std::hint::spin_loop();
                    }
                    let finished = Instant::now();
                    checkpoint().map_err(|_| Code::ResourceLimit)?;
                    assert!(finished.elapsed() >= Duration::from_millis(9));
                }
                Ok(())
            })
            .unwrap();
        let cumulative = nanos(
            bbt_rt::fault::thread_cpu_time()
                .unwrap()
                .checked_sub(before)
                .unwrap(),
        );
        let due = quantum.control.cleanup.debt.due_ns.load(Ordering::Acquire);
        quantum.pay().unwrap();
        let maximum = quantum.max_unit_cpu_ns();
        assert!(cumulative > 1_000_000 && maximum <= 1_000_000);
        assert!(cumulative >= maximum);
        let debt = &quantum.control.cleanup.debt;
        assert!(debt.spans.load(Ordering::Acquire) >= 10);
        assert_eq!(debt.remaining_ns(), 0);
        // paid_ns counts explicit sleeps, not all after-end elapsed rest.
        // Each checkpoint above starts the next phase only after its full debt;
        // the last phase's exact deadline must also have elapsed before return.
        assert!(debt.now_ns() >= due);
        eprintln!(
            "dream_c1_preparation_units cumulative_cpu_ns={cumulative} max_unit_cpu_ns={maximum} spans={} charged_ns={} explicit_sleep_ns={} remaining_ns=0",
            debt.spans.load(Ordering::Acquire),
            debt.charged_ns.load(Ordering::Acquire),
            debt.paid_ns.load(Ordering::Acquire)
        );
        let control = Control::default();
        control
            .install_authority_guard(PreparedGuard {
                callback: Arc::new(|| Ok(())),
                setup_cpu_ns: cumulative,
                setup_max_unit_cpu_ns: maximum,
                retire_session: None,
            })
            .unwrap();
        assert_eq!(
            control.authority_setup_cpu_ns.load(Ordering::Acquire),
            cumulative
        );
        assert_eq!(
            control
                .authority_setup_max_unit_cpu_ns
                .load(Ordering::Acquire),
            maximum
        );
    }

    #[test]
    fn source_owner_guard_refuses_before_open_and_accepts_only_bound_snapshot_thread() {
        let source = source(&events(true), false);
        let control = Control::default();
        // Refusal has precedence even over an unavailable pathname: no fs effect.
        assert!(matches!(
            guarded_source_open(&control, Path::new("/unavailable-source")),
            Err(Code::ResourceLimit)
        ));
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    control
                        .source_owner
                        .set(std::thread::current().id())
                        .unwrap();
                    let fd = guarded_source_open(&control, &source.path).unwrap();
                    assert_eq!(fd.metadata().unwrap().len(), source.bytes.len() as u64);
                })
                .join()
                .unwrap();
        });
        assert!(matches!(
            guarded_source_open(&control, &source.path),
            Err(Code::ResourceLimit)
        ));
        assert_eq!(control.owner_refusals.load(Ordering::Acquire), 2);
        assert_eq!(std::fs::read(&source.path).unwrap(), source.bytes);
    }
    #[test]
    fn canonical_job_resolves_private_parent_alias_and_retains_same_inode() {
        let root = private_job();
        let real = root.path().join("real");
        private_dir(&real).unwrap();
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        let job = alias.join("job");
        private_dir(&job).unwrap();
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(2));
        let (canonical, directory) = canonical_job(&job, &mut budget).unwrap();
        assert_eq!(canonical, real.canonicalize().unwrap().join("job"));
        assert_eq!(
            directory.metadata().unwrap().ino(),
            std::fs::metadata(&job).unwrap().ino()
        );
        assert!(
            !canonical.join("input").exists(),
            "canonicalizing an owned job does not inspect source/manifest data"
        );
    }
    #[test]
    fn whole_reactor_cpu_control_detects_work_outside_receive() {
        let control = Control::default();
        let clock = ReactorClock::new(&control).unwrap();
        let cpu = bbt_rt::fault::thread_cpu_time().unwrap();
        while bbt_rt::fault::thread_cpu_time()
            .unwrap()
            .saturating_sub(cpu)
            < Duration::from_millis(2)
        {
            std::hint::spin_loop();
        }
        assert!(clock.batch_due().unwrap());
        assert_eq!(clock.finish(), Err(Code::ResourceLimit));
        assert_eq!(control.reactor_overruns.load(Ordering::Acquire), 1);
        assert!(control.reactor_max_cpu_ns.load(Ordering::Acquire) >= 2_000_000);
    }

    #[test]
    fn real_rest_wait_records_sleep_cpu_and_private_omission_fails_same_duty_observer() {
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(2));
        for _ in 0..3 {
            budget.unit(|| Ok(())).unwrap();
        }
        assert_eq!(control.work_unit_count.load(Ordering::Acquire), 3);
        assert_eq!(control.rest_skips.load(Ordering::Acquire), 0);
        assert_eq!(control.rest_violations.load(Ordering::Acquire), 0);
        assert!(control.min_inter_unit_gap_ns.load(Ordering::Acquire) >= 9_000_000);
        assert!(control.rest_wait_ns.load(Ordering::Acquire) >= 18_000_000);
        assert!(
            control.rest_wait_cpu_ns.load(Ordering::Acquire)
                < control.rest_wait_ns.load(Ordering::Acquire)
        );
        #[cfg(debug_assertions)]
        {
            let mutant = Control {
                private_fixture: true,
                ..Control::default()
            };
            mutant.foreground.store(false, Ordering::Release);
            mutant.omit_rest.store(true, Ordering::Release);
            let mut budget = Budget::new(&mutant, Instant::now() + Duration::from_secs(2));
            for _ in 0..3 {
                budget.unit(|| Ok(())).unwrap();
            }
            assert!(mutant.rest_skips.load(Ordering::Acquire) > 0);
            assert!(mutant.rest_violations.load(Ordering::Acquire) > 0);
            assert!(mutant.min_inter_unit_gap_ns.load(Ordering::Acquire) < 9_000_000);
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    fn private_fifo_holds_actual_snapshot_owner_through_cancel_until_io_returns() {
        use std::os::unix::ffi::OsStrExt;
        let source = source(&events(true), false);
        let bookmark = close(&source);
        let job = private_job();
        let path = job.path().join("test-source-read.fifo");
        let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: terminated pathname inside this test's private directory.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let control = Control {
            private_fixture: true,
            ..Control::default()
        };
        control.foreground.store(false, Ordering::Release);
        std::thread::scope(|scope| {
            let (sender, receiver) = mpsc::sync_channel(1);
            let record = record(vec![bookmark]);
            let job_path = job.path();
            let control_ref = &control;
            let handle = scope.spawn(move || {
                sender
                    .send(snapshot(
                        job_path,
                        &record,
                        control_ref,
                        Instant::now() + Duration::from_secs(3),
                    ))
                    .ok();
            });
            let deadline = Instant::now() + Duration::from_secs(2);
            while !job.path().join("source-read-held.json").exists() {
                assert!(
                    Instant::now() < deadline,
                    "actual source owner did not reach FIFO"
                );
                std::thread::sleep(Duration::from_millis(1));
            }
            control.cancelled.store(true, Ordering::Release);
            assert!(
                matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)),
                "held owner cannot settle on cancel alone"
            );
            OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&path)
                .unwrap()
                .write_all(b"x")
                .unwrap();
            assert!(matches!(
                receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
                Err(Code::Cancelled)
            ));
            handle.join().unwrap();
        });
        assert!(!job.path().join("input/manifest.json").exists());
        assert_eq!(std::fs::read(&source.path).unwrap(), source.bytes);
    }

    #[test]
    #[ignore = "requires explicit actual producer fork export; no synthetic fallback"]
    fn actual_consumer_store_fork_frontier_rejects_before_parent_rows() {
        let root = PathBuf::from(
            std::env::var_os("DREAM_C1_FORK_BUNDLE")
                .expect("DREAM_C1_FORK_BUNDLE required; no fallback"),
        );
        assert!(root.is_absolute());
        let manifest_bytes = std::fs::read(root.join("manifest.json")).unwrap();
        let manifest: w::Manifest = transport::decode(&manifest_bytes).unwrap();
        assert_eq!(manifest.sources.len(), 1);
        let source = &manifest.sources[0];
        assert_eq!(source.closed_seq.0, 5);
        let path = root
            .join("sessions")
            .join(format!("{}.jsonl", source.session.0));
        let original = std::fs::read(&path).unwrap();
        assert_eq!(original.len() as u64, source.len.0);
        assert_eq!(digest(&original), source.sha256.0);
        let first = line_end(&original).unwrap() + 1;
        let header: Value = serde_json::from_slice(&original[..first]).unwrap();
        assert!(
            !header["event"]["meta"]["fork"].is_null(),
            "actual Store fork header required"
        );
        let mut reader =
            Reader::open(&root, &source.session.0, source.source_generation.0).unwrap();
        let mut refused = None;
        for _ in 0..4 {
            match reader.poll() {
                Err(code) => {
                    refused = Some(code);
                    break;
                }
                Ok((bookmark, _)) => assert!(bookmark.is_none()),
            }
        }
        assert_eq!(refused, Some(Code::UnsupportedIr));
        assert_eq!(reader.seq, 1);
        assert_eq!(
            reader.cursor, first as u64,
            "no inherited parent row was interpreted"
        );
        let copy = private_job();
        private_dir(&copy.path().join("sessions")).unwrap();
        let copy_path = copy
            .path()
            .join("sessions")
            .join(format!("{}.jsonl", source.session.0));
        let second = first + line_end(&original[first..]).unwrap() + 1;
        let mut malformed = original[..first].to_vec();
        malformed.extend_from_slice(b"{malformed-parent-row\n");
        malformed.extend_from_slice(&original[second..]);
        create(&copy_path).unwrap().write_all(&malformed).unwrap();
        let mut broken =
            Reader::open(copy.path(), &source.session.0, source.source_generation.0).unwrap();
        let mut broken_refusal = None;
        for _ in 0..4 {
            if let Err(code) = broken.poll() {
                broken_refusal = Some(code);
                break;
            }
        }
        assert_eq!(broken_refusal, Some(Code::UnsupportedIr));
        assert_eq!(broken.seq, 1);
        assert_eq!(broken.cursor, first as u64);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            original,
            "actual exported fork bytes remain immutable"
        );
        let evidence = private_job().keep();
        let mut receipt = json!({"scope":"actual Store fork Reader frontier component; not full transcript-law or admitted MCP proof","store_root":root,"source_session":source.session.0,"source_generation":source.source_generation.0,"source_sha256":source.sha256.0,"source_len":source.len.0,"closed_seq":source.closed_seq.0,"header_fork_present":true,"error":"unsupported_ir","observed_seq":reader.seq,"observed_cursor":reader.cursor,"header_len":first,"inherited_parent_rows_interpreted":0,"inspection_worker_spawned":false,"malformed_parent_copy_still_header_refused":true,"original_source_unchanged":true});
        let mut output = create(&evidence.join("fork-frontier.json")).unwrap();
        serde_json::to_writer(&mut output, &receipt).unwrap();
        output.sync_all().unwrap();
        eprintln!(
            "C1_FORK_FRONTIER_RECEIPT={}",
            evidence.join("fork-frontier.json").display()
        );
        if let Some(directory) = std::env::var_os("DREAM_C1_EVIDENCE") {
            let directory = PathBuf::from(directory);
            assert!(directory.is_absolute());
            let nonce = std::env::var("DREAM_C1_GATE_NONCE")
                .expect("driver evidence requires DREAM_C1_GATE_NONCE");
            let binary = PathBuf::from(
                std::env::var_os("DREAM_C1_RUNNER")
                    .expect("driver evidence requires exact DREAM_C1_RUNNER"),
            );
            assert!(binary.is_absolute());
            let mut hash = Sha256::new();
            let mut input = File::open(&binary).unwrap();
            let mut chunk = [0; CHUNK];
            loop {
                let n = input.read(&mut chunk).unwrap();
                if n == 0 {
                    break;
                }
                hash.update(&chunk[..n]);
            }
            receipt["schema"] = json!("bboxy.dream.c1.runtime-evidence/1");
            receipt["driver_nonce"] = json!(nonce);
            receipt["runner_sha256"] = json!(format!("{:x}", hash.finalize()));
            let mut output = create(&directory.join("runtime-fork-subset.json")).unwrap();
            serde_json::to_writer(&mut output, &receipt).unwrap();
            output.sync_all().unwrap();
        }
    }

    /// Component memory witness only. The application's admitted own-seat source
    /// and API witnesses remain separate; this test consumes an actual Store export.
    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "requires explicit actual producer Store bundle and pinned real runner"]
    fn isolated_actual_store_worker_full_host_heap_observed_under_32mib() {
        isolated_actual_store_heap_jobs(1);
    }
    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "requires explicit actual producer Store bundle and pinned real runner"]
    fn isolated_four_actual_store_workers_full_host_heap_observed_under_128mib() {
        isolated_actual_store_heap_jobs(4);
    }
    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "requires explicit pinned real runner for Hello/reap overflow witness"]
    fn isolated_parent_work_overflow_preserves_controls_and_reaps_real_probe() {
        let binary = PathBuf::from(
            std::env::var_os("DREAM_C1_RUNNER").expect("DREAM_C1_RUNNER required; no fallback"),
        );
        assert!(binary.is_absolute());
        let evidence = private_job().keep();
        isolated_heap::ENABLED.store(true, Ordering::Release);
        let proof = Arc::new(runner_provision::verify_native_provision(&binary, false).unwrap());
        let mut witnessed = None;
        let mut observed_heap = None;
        probe_gated_observe(
            &evidence,
            proof,
            monotonic_ns().unwrap(),
            |worker, control| {
                let pid = worker.pid();
                let mut budget = Budget::new(control, Instant::now() + Duration::from_secs(30));
                let process_start =
                    budget.unit(|| transport::process_stamp(pid).map_err(wire_error))?;
                let remaining = WORK_HEAP - control.heap.counters(HeapLane::Work).reserved;
                let mut held = None;
                budget.unit(|| {
                    held = Some(HeapBuffer::new(
                        &control.heap,
                        HeapLane::Work,
                        remaining as usize,
                    )?);
                    Ok(())
                })?;
                let before = control.heap.counters(HeapLane::Work);
                budget.unit(|| {
                    if !matches!(
                        HeapBuffer::new(&control.heap, HeapLane::Work, 1),
                        Err(Code::ResourceLimit)
                    ) {
                        return Err(Code::ProtocolError);
                    }
                    if control.heap.counters(HeapLane::Work) != before {
                        return Err(Code::ProtocolError);
                    }
                    Ok(())
                })?;
                let at = Instant::now();
                control.cancelled.store(true, Ordering::Release);
                control.phase(CANCELLING);
                let observed = state(control.state.load(Ordering::Acquire));
                let settled = control.settled.load(Ordering::Acquire);
                let cached_control_ns = nanos(at.elapsed());
                assert_eq!(observed, State::Cancelling);
                assert!(!settled);
                assert_eq!(control.heap.counters(HeapLane::Work).live, WORK_HEAP);
                let at = Instant::now();
                settle_probe(worker, control);
                let reap_ns = nanos(at.elapsed());
                assert!(worker.usage().is_some(), "actual wait4 reap required");
                sample_isolated_heap();
                drop(held);
                witnessed = Some((pid, process_start, before, cached_control_ns, reap_ns));
                observed_heap = Some(control.heap.clone());
                Ok(())
            },
        )
        .unwrap();
        let (pid, process_start, before, cached_control_ns, reap_ns) = witnessed.unwrap();
        let heap = observed_heap.unwrap();
        let live = isolated_heap::LIVE_PEAK.load(Ordering::Acquire);
        let touched = isolated_heap::TOUCHED_PEAK.load(Ordering::Acquire);
        let work = heap.counters(HeapLane::Work);
        let protected = heap.counters(HeapLane::Control);
        let mut sha = Sha256::new();
        let mut file = File::open(&binary).unwrap();
        let mut bytes = [0; CHUNK];
        loop {
            let n = file.read(&mut bytes).unwrap();
            if n == 0 {
                break;
            }
            sha.update(&bytes[..n]);
        }
        let receipt = json!({"scope":"parent controlled-capacity overflow, actual Hello/EOF/reap and cachedControl primitives; not admitted MCP/ctl","runner_sha256":format!("{:x}",sha.finalize()),"worker_pid":pid,"worker_start":process_start,"actual_hello":true,"overflow_refused_before_allocation":true,"while_exhausted_work_live_bytes":before.live,"while_exhausted_work_reserved_bytes":before.reserved,"cached_cancel_status_max_ns":cached_control_ns,"real_exit_reaped":true,"reap_ns":reap_ns,"source_or_manifest_opened":false,"observed_full_host_live_heap_peak_bytes":live,"allocator_all_zones_max_touched_bytes":if touched>0{json!(touched)}else{Value::Null},"allocator_max_touched_supported":touched>0,"full_parent_heap_ceiling_bytes":32*1024*1024,"not_all_times_proof":true,"work_declared_live_bytes":work.reserved,"work_declared_peak_bytes":work.reserved_peak,"work_declared_cumulative_bytes":work.reserved_cumulative,"work_tracked_capacity_peak_bytes":work.peak,"work_tracked_capacity_cumulative_bytes":work.cumulative,"control_declared_live_bytes":protected.reserved,"control_declared_peak_bytes":protected.reserved_peak,"control_declared_cumulative_bytes":protected.reserved_cumulative,"control_tracked_capacity_peak_bytes":protected.peak,"heap_observer_thread_cpu_ns":isolated_heap::OBSERVER_CPU_NS.load(Ordering::Acquire)});
        let mut out = create(&evidence.join("parent-overflow.json")).unwrap();
        serde_json::to_writer(&mut out, &receipt).unwrap();
        out.sync_all().unwrap();
        eprintln!(
            "C1_PARENT_OVERFLOW_RECEIPT={}",
            evidence.join("parent-overflow.json").display()
        );
        if let Some(path) = std::env::var_os("DREAM_C1_EVIDENCE") {
            let path = PathBuf::from(path);
            assert!(path.is_absolute());
            let nonce = std::env::var("DREAM_C1_GATE_NONCE")
                .expect("driver evidence requires DREAM_C1_GATE_NONCE");
            let mut runtime = receipt.clone();
            runtime["schema"] = json!("bboxy.dream.c1.runtime-evidence/1");
            runtime["driver_nonce"] = json!(nonce);
            let mut out = create(&path.join("runtime-parent-overflow.json")).unwrap();
            serde_json::to_writer(&mut out, &runtime).unwrap();
            out.sync_all().unwrap();
        }
        isolated_heap::ENABLED.store(false, Ordering::Release);
        assert!(live > 0 && live <= 32 * 1024 * 1024 && touched <= 32 * 1024 * 1024);
        assert_eq!(work.reserved_peak, WORK_HEAP);
        assert_eq!(work.reserved, 0);
        assert_eq!(work.live, 0);
        assert_eq!(protected.reserved_peak, CONTROL_HEAP);
        assert_eq!(protected.reserved, 0);
        assert!(cached_control_ns < 100_000_000 && reap_ns < 2_000_000_000);
    }
    #[cfg(target_os = "macos")]
    fn isolated_actual_store_heap_jobs(instances: usize) {
        let root = PathBuf::from(
            std::env::var_os("DREAM_C1_STORE_BUNDLE")
                .expect("DREAM_C1_STORE_BUNDLE required; no synthetic/missing fallback"),
        );
        let binary = PathBuf::from(
            std::env::var_os("DREAM_C1_RUNNER")
                .expect("DREAM_C1_RUNNER required; no PATH fallback"),
        );
        assert!(root.is_absolute() && binary.is_absolute());
        isolated_heap::ENABLED.store(true, Ordering::Release);
        sample_isolated_heap();
        let manifest_bytes = std::fs::read(root.join("manifest.json")).unwrap();
        transport::preflight_json(&manifest_bytes, transport::JsonShape::Any).unwrap();
        let manifest: w::Manifest = serde_json::from_slice(&manifest_bytes).unwrap();
        assert_eq!(
            manifest.sources.len(),
            1,
            "this witness is one exact immutable Store export"
        );
        let expected = &manifest.sources[0];
        assert!(
            expected.len.0 >= 2 * 1024 * 1024,
            "actual multiMiB IR required"
        );
        assert!(
            expected.blobs.iter().any(|b| b.len.0 >= 4 * 1024 * 1024),
            "actual Store blob inventory required"
        );
        let mut reader =
            Reader::open(&root, &expected.session.0, expected.source_generation.0).unwrap();
        let mut bookmark = None;
        for _ in 0..expected.closed_seq.0 * 8 + 64 {
            if let Some(value) = reader.poll().unwrap().0 {
                bookmark = Some(value);
                break;
            }
        }
        let bookmark = bookmark.expect("authoritative typed durable close was not observed");
        assert_eq!(bookmark.closed_seq, expected.closed_seq.0);
        assert_eq!(bookmark.closed_line_sha256, expected.closed_line_sha256.0);
        assert_eq!(bookmark.repo, expected.repo.0);
        assert_eq!(bookmark.len, expected.len.0);
        fn file_sha(path: &Path) -> String {
            let mut file = File::open(path).unwrap();
            let mut sha = Sha256::new();
            let mut bytes = [0; CHUNK];
            loop {
                let n = file.read(&mut bytes).unwrap();
                if n == 0 {
                    break;
                }
                sha.update(&bytes[..n]);
            }
            format!("{:x}", sha.finalize())
        }
        assert_eq!(file_sha(&bookmark.path), expected.sha256.0);
        let evidence = private_job().keep();
        let mut runs = Vec::new();
        let mut controls = Vec::new();
        for index in 0..instances {
            let runtime = evidence.join(format!("dream-c1-{index}"));
            private_dir(&runtime).unwrap();
            drop(Journal::open(&runtime, &public::Ulid(manifest.context.seat.0.clone())).unwrap());
            let mut record = record(vec![bookmark.clone()]);
            record.context = mirror(&manifest.context).unwrap();
            record.view.run_id = public::Ulid(if index == 0 {
                manifest.run_id.0.clone()
            } else {
                ulid::Ulid::new().to_string()
            });
            record.nonce = public::Ulid(if index == 0 {
                manifest.nonce.0.clone()
            } else {
                ulid::Ulid::new().to_string()
            });
            record.op_id = public::Ulid(ulid::Ulid::new().to_string());
            record.trigger = false;
            record.started = true;
            record.view.state = State::Preparing;
            let job = runtime.join(&record.view.run_id.0);
            runs.push((job, record));
            let control = Arc::new(Control::default());
            control.foreground.store(false, Ordering::Release);
            controls.push(control);
        }
        let mut outcomes = vec![None; instances];
        let mut status_max_wall_ns = 0u64;
        let mut status_samples = 0u64;
        std::thread::scope(|scope| {
            let (sender, receiver) = mpsc::sync_channel(instances);
            for (index, ((job, record), control)) in runs.iter_mut().zip(&controls).enumerate() {
                let sender = sender.clone();
                let binary = &binary;
                scope.spawn(move || {
                    let result = execute(binary, job, record, control);
                    let _ = sender.send((index, result));
                });
            }
            drop(sender);
            let deadline = Instant::now() + Duration::from_secs(120);
            while outcomes.iter().any(Option::is_none) {
                let at = Instant::now();
                for control in &controls {
                    std::hint::black_box(state(control.state.load(Ordering::Acquire)));
                    std::hint::black_box(control.settled.load(Ordering::Acquire));
                }
                status_max_wall_ns = status_max_wall_ns.max(nanos(at.elapsed()));
                status_samples += 1;
                match receiver.recv_timeout(Duration::from_millis(2)) {
                    Ok((index, result)) => outcomes[index] = Some(result),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        for outcome in &mut outcomes {
                            if outcome.is_none() {
                                *outcome = Some(Err(Code::WorkerLost));
                            }
                        }
                    }
                }
                if Instant::now() >= deadline {
                    for control in &controls {
                        control.cancelled.store(true, Ordering::Release);
                    }
                }
            }
        });
        sample_isolated_heap();
        let live = isolated_heap::LIVE_PEAK.load(Ordering::Acquire);
        let touched = isolated_heap::TOUCHED_PEAK.load(Ordering::Acquire);
        let samples = isolated_heap::SAMPLES.load(Ordering::Acquire);
        let mut job_receipts = Vec::new();
        let mut child_heap_sum = 0u64;
        let mut all_ok = true;
        for (index, ((job, record), control)) in runs.iter().zip(&controls).enumerate() {
            let peak = control.producer_metrics[1].load(Ordering::Acquire);
            let startup = control.producer_metrics[2].load(Ordering::Acquire);
            let measured = control.producer_metrics_observed.load(Ordering::Acquire)
                && peak != u64::MAX
                && startup != u64::MAX;
            let child_peak = if measured { peak.max(startup) } else { 0 };
            child_heap_sum = child_heap_sum.checked_add(child_peak).unwrap();
            let host_metrics: Value = std::fs::read(job.join("metrics.json"))
                .ok()
                .and_then(|v| serde_json::from_slice(&v).ok())
                .unwrap_or(Value::Null);
            let work = control.heap.counters(HeapLane::Work);
            let protected = control.heap.counters(HeapLane::Control);
            let completed = outcomes[index] == Some(Ok(()))
                && record.view.state == State::Completed
                && record.view.settled
                && control.child_usage_observed.load(Ordering::Acquire);
            all_ok &= completed
                && measured
                && child_peak <= 96 * 1024 * 1024
                && control.producer_metrics[9].load(Ordering::Acquire) == 0
                && control.producer_metrics[10].load(Ordering::Acquire) <= 78 * 1024 * 1024
                && control.producer_metrics[12].load(Ordering::Acquire) == 2 * 1024 * 1024
                && control.producer_metrics[13].load(Ordering::Acquire) == 16 * 1024 * 1024
                && control.producer_metrics[14].load(Ordering::Acquire) == 96 * 1024 * 1024
                && work.reserved_peak <= WORK_HEAP
                && protected.reserved_peak <= CONTROL_HEAP;
            job_receipts.push(json!({"run_id":record.view.run_id.0,"worker_pid":record.worker_pid,"worker_start":record.worker_start,"job":job,"result":format!("{:?}",outcomes[index]),"completed":completed,"settled":record.view.settled,"real_exit_reaped":control.child_usage_observed.load(Ordering::Acquire),"child_heap_measured":measured,"child_heap_peak_bytes":child_peak,"child_heap_ceiling_bytes":96*1024*1024,"child_declared_live_bytes":control.producer_metrics[9].load(Ordering::Acquire),"child_declared_peak_bytes":control.producer_metrics[10].load(Ordering::Acquire),"child_declared_cumulative_bytes":control.producer_metrics[11].load(Ordering::Acquire),"child_control_reserved_bytes":control.producer_metrics[12].load(Ordering::Acquire),"child_helper_reserved_bytes":control.producer_metrics[13].load(Ordering::Acquire),"child_credit_ceiling_bytes":control.producer_metrics[14].load(Ordering::Acquire),"parent_work_declared_live_bytes":work.reserved,"parent_work_declared_peak_bytes":work.reserved_peak,"parent_work_declared_cumulative_bytes":work.reserved_cumulative,"parent_control_declared_live_bytes":protected.reserved,"parent_control_declared_peak_bytes":protected.reserved_peak,"parent_control_declared_cumulative_bytes":protected.reserved_cumulative,"host_metrics":host_metrics}));
        }
        // The observer cannot attribute opaque allocations to individual jobs.
        // Bound the baseline and all jobs together by one parent share: this
        // collective upper bound conservatively proves every per-job parent.
        let parent_ceiling = 32 * 1024 * 1024;
        let parent_conservative = live.max(touched);
        let receipt = json!({"scope":"isolated actual Store/worker component, not own-seat admission, root caps or SourceKey dedup","measurement":"macOS malloc_zone_statistics all zones; full isolated host including baseline, no subtraction","instances":instances,"same_immutable_store_reused_for_component_load":instances>1,"observed_full_host_live_heap_peak_bytes":live,"allocator_all_zones_max_touched_bytes":if touched>0{json!(touched)}else{Value::Null},"allocator_max_touched_supported":touched>0,"parent_heap_conservative_bytes":parent_conservative,"parent_heap_ceiling_bytes":parent_ceiling,"observed_whole_four_host_below_one_parent_share":instances==4&&parent_conservative<=32*1024*1024,"child_heap_sum_conservative_bytes":child_heap_sum,"child_heap_sum_ceiling_bytes":96*1024*1024*instances as u64,"combined_parent_child_conservative_bytes":parent_conservative+child_heap_sum,"combined_ceiling_bytes":128*1024*1024*instances as u64,"conservative_sum_of_noncoincident_peaks":true,"samples":samples,"not_all_times_proof":true,"heap_observer_thread_cpu_ns":isolated_heap::OBSERVER_CPU_NS.load(Ordering::Acquire),"unit_cpu_excludes_test_heap_observer":true,"owner_cpu_includes_test_heap_observer":true,"hash_unit_bytes":HASH_CHUNK,"runner":binary,"runner_sha256":file_sha(&binary),"store_root":root,"source_session":expected.session.0,"source_sha256":expected.sha256.0,"source_len":expected.len.0,"blob_sha256":expected.blobs[0].sha256.0,"control_measurement":"engine cached atomics; admitted MCP/ctl evidence is separate","status_samples":status_samples,"status_max_wall_ns":status_max_wall_ns,"jobs":job_receipts,"all_jobs_completed":all_ok});
        let mut receipt_file = create(&evidence.join("isolated-host-heap.json")).unwrap();
        serde_json::to_writer(&mut receipt_file, &receipt).unwrap();
        receipt_file.sync_all().unwrap();
        eprintln!(
            "C1_ISOLATED_HOST_HEAP_RECEIPT={} instances={} live={} touched={} samples={} all_ok={}",
            evidence.join("isolated-host-heap.json").display(),
            instances,
            live,
            touched,
            samples,
            all_ok
        );
        if let Some(driver_path) = std::env::var_os("DREAM_C1_EVIDENCE") {
            let driver_path = PathBuf::from(driver_path);
            assert!(driver_path.is_absolute());
            let nonce = std::env::var("DREAM_C1_GATE_NONCE")
                .expect("explicit driver evidence requires DREAM_C1_GATE_NONCE");
            let mut runtime = receipt.clone();
            runtime["schema"] = json!("bboxy.dream.c1.runtime-evidence/1");
            runtime["driver_nonce"] = json!(nonce);
            let path = driver_path.join(if instances == 1 {
                "runtime-isolated-heap.json"
            } else {
                "runtime-isolated-four-heap.json"
            });
            let mut output = create(&path).unwrap();
            serde_json::to_writer(&mut output, &runtime).unwrap();
            output.sync_all().unwrap();
        }
        isolated_heap::ENABLED.store(false, Ordering::Release);
        assert!(
            all_ok,
            "actual workers must complete/reap/measure within their declared shares; receipt={}",
            evidence.display()
        );
        assert!(samples > 100 && live > 0);
        assert!(
            parent_conservative <= parent_ceiling
                && child_heap_sum <= 96 * 1024 * 1024 * instances as u64
                && parent_conservative + child_heap_sum <= 128 * 1024 * 1024 * instances as u64,
            "isolated component memory ceiling exceeded; receipt={}",
            evidence.display()
        );
        assert!(
            status_samples > 0 && status_max_wall_ns < 100_000_000,
            "cached component control lane stalled"
        );
        assert_eq!(
            file_sha(&runs[0].1.sources[0].path),
            expected.sha256.0,
            "original Store bytes immutable"
        );
    }
}

// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-harness src/dream/c1.rs::tests, signed47182320:
// https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/c1.rs
#[cfg(test)]
mod disposal_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dream_c1_public_prelude_refusal_retains_lexical_and_returned_owners() {
        struct ProofOwner {
            debt: Arc<WorkDebt>,
            drops: Arc<AtomicU64>,
        }
        impl Drain for ProofOwner {
            fn drain_step(&mut self) -> bool {
                false
            }
        }
        impl Drop for ProofOwner {
            fn drop(&mut self) {
                assert!(
                    WORK_FRAME.with(|frame| frame
                        .borrow()
                        .as_ref()
                        .is_some_and(|frame| Arc::ptr_eq(&frame.debt, &self.debt))),
                    "public proof escaped required cleanup ownership"
                );
                self.drops.fetch_add(1, Ordering::AcqRel);
            }
        }
        for missing_clock in [false, true] {
            for returns_owner in [false, true] {
                let quantum = JournalQuantum::default();
                let drops = Arc::new(AtomicU64::new(0));
                let result = quantum.public_prelude_sampled(
                    |checkpoint| {
                        let owner = ProofOwner {
                            debt: quantum.control.cleanup.debt.clone(),
                            drops: drops.clone(),
                        };
                        let verdict = checkpoint();
                        assert!(verdict.is_err());
                        if returns_owner {
                            // Even a broken accessor ignoring the refusal cannot
                            // release its completed value outside paced ownership.
                            Ok(owner)
                        } else {
                            verdict.map_err(|_| Code::StaleContext)?;
                            Ok(owner)
                        }
                    },
                    |_| {
                        if missing_clock {
                            Err(Code::ResourceLimit)
                        } else {
                            Ok(UNIT + Duration::from_nanos(1))
                        }
                    },
                );
                assert!(matches!(result, Err(Code::ResourceLimit)));
                assert_eq!(drops.load(Ordering::Acquire), 1);
                assert!(quantum.control.cleanup.units.load(Ordering::Acquire) > 0);
                assert!(!quantum.control.cleanup.debt.held.load(Ordering::Acquire));
                assert!(WORK_FRAME.with(|frame| frame.borrow().is_none()));
                assert_eq!(
                    quantum
                        .control
                        .cleanup
                        .debt
                        .poisoned
                        .load(Ordering::Acquire),
                    missing_clock
                );
            }
        }
    }

    #[test]
    fn dream_c1_tiny_json_combines_scan_decode_work_and_reuses_complex_fallback() {
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(3));
        let mut called = 0;
        assert!(matches!(
            budget
                .small_json(br#"{"v":1,"event":{"type":"session_closed"}}"#, |value| {
                    called += 1;
                    assert_eq!(value["event"]["type"], "session_closed");
                    Ok(())
                })
                .unwrap(),
            SmallJson::Processed
        ));
        assert_eq!(called, 1);
        assert_eq!(control.work_unit_count.load(Ordering::Acquire), 1);
        assert_eq!(control.heap.counters(HeapLane::Work).reserved, 0);
        cleanup_green(&control);

        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(3));
        let bytes = serde_json::to_vec(&vec![0; 150]).unwrap();
        let SmallJson::Fallback(Some(bounds)) = budget
            .small_json(&bytes, |_| panic!("complex fallback ran tiny work"))
            .unwrap()
        else {
            panic!("complex record was not staged")
        };
        assert_eq!(control.work_unit_count.load(Ordering::Acquire), 1);
        let credit = budget.json_credit(bounds).unwrap();
        let owner: Paced<Value> = budget.parse_credited(&bytes, credit).unwrap();
        assert_eq!(owner.as_array().unwrap().len(), 150);
        assert_eq!(
            control.work_unit_count.load(Ordering::Acquire),
            2,
            "fallback scanned the already admitted raw structure twice"
        );
        drop(owner);
        cleanup_green(&control);

        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(3));
        let bytes = serde_json::to_vec(&json!({"payload":"x".repeat(8193)})).unwrap();
        assert!(matches!(
            budget
                .small_json(&bytes, |_| panic!("large fallback ran tiny work"))
                .unwrap(),
            SmallJson::Fallback(None)
        ));
        assert_eq!(
            control.work_unit_count.load(Ordering::Acquire),
            0,
            "size-only fallback performed an unnecessary scan/unit"
        );
    }
    #[test]
    fn dream_c1_tiny_json_error_and_cpu_verdict_keep_owned_cleanup() {
        for overrun in [false, true] {
            let control = Control::default();
            control.foreground.store(false, Ordering::Release);
            let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(3));
            let result = budget.small_json(br#"{"retained":[{"leaf":"owned"}]}"#, |_| {
                if overrun {
                    let start = bbt_rt::fault::thread_cpu_time().unwrap();
                    while bbt_rt::fault::thread_cpu_time()
                        .unwrap()
                        .saturating_sub(start)
                        < Duration::from_millis(2)
                    {
                        std::hint::spin_loop();
                    }
                    Ok(())
                } else {
                    Err(Code::SourceMismatch)
                }
            });
            assert!(
                matches!(result, Err(Code::ResourceLimit)) && overrun
                    || matches!(result, Err(Code::SourceMismatch)) && !overrun
            );
            assert_eq!(control.heap.counters(HeapLane::Work).reserved, 0);
            assert_eq!(
                control.unit_overruns.load(Ordering::Acquire),
                u64::from(overrun)
            );
            cleanup_green(&control);
        }
    }
    #[test]
    fn dream_c1_owned_seed_unknown_field_errors_never_include_untrusted_keys() {
        let control = Control::default();
        let key = "historical-provider-instructions".repeat(4096);
        let descriptor = serde_json::to_vec(&json!({(key.clone()):null})).unwrap();
        let error = <w::ResultDescriptor as OwnedDecode>::decode(
            std::io::Cursor::new(&descriptor),
            &control,
        )
        .unwrap_err();
        assert!(error.to_string().len() < 96);
        assert!(!error.to_string().contains("historical-provider"));
        let page = serde_json::to_vec(&json!({"rows":[{(key):null}]})).unwrap();
        let error =
            <w::Page as OwnedDecode>::decode(std::io::Cursor::new(&page), &control).unwrap_err();
        assert!(error.to_string().len() < 96);
        assert!(!error.to_string().contains("historical-provider"));
    }

    #[test]
    fn dream_c1_collection_growth_reserves_old_overlap_and_preserves_resource_limit() {
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(2));
        let bytes = b"[1,2]";
        let credit = budget.json(bytes).unwrap();
        // Final-entry credit is admitted, but no additional old+new allocation
        // overlap remains. The second entry must fail before reallocating.
        let remaining = WORK_HEAP - control.heap.counters(HeapLane::Work).reserved;
        let held = control
            .heap
            .reserve(HeapLane::Work, remaining as usize)
            .unwrap();
        assert!(matches!(
            budget.parse_credited::<Value>(bytes, credit),
            Err(Code::ResourceLimit)
        ));
        assert_eq!(control.cleanup.capacity_refusals.load(Ordering::Acquire), 1);
        assert_eq!(control.heap.counters(HeapLane::Work).reserved, remaining);
        drop(held);
        assert_eq!(control.heap.counters(HeapLane::Work).reserved, 0);
        cleanup_green(&control);
    }

    #[test]
    fn dream_c1_native_buffer_free_is_measured_and_retains_reservation_through_cancel() {
        let control = Control::default();
        assert!(Arc::ptr_eq(&control.cleanup, &control.heap.cleanup));
        let mut bytes = HeapBuffer::new(&control.heap, HeapLane::Work, 128 * 1024).unwrap();
        bytes.extend(b"owned native allocation").unwrap();
        assert_eq!(control.heap.counters(HeapLane::Work).live, 128 * 1024);
        control.cancelled.store(true, Ordering::Release);
        control.foreground.store(true, Ordering::Release);
        drop(bytes);
        assert_eq!(control.heap.counters(HeapLane::Work).live, 0);
        assert_eq!(control.heap.counters(HeapLane::Work).reserved, 0);
        assert_eq!(control.cleanup.units.load(Ordering::Acquire), 1);
        cleanup_green(&control);
    }

    struct DropProbe {
        remaining: usize,
        heap: Arc<Heap>,
        dropped: Arc<AtomicBool>,
    }
    impl Drain for DropProbe {
        fn drain_step(&mut self) -> bool {
            if self.remaining == 0 {
                return false;
            }
            let before = bbt_rt::fault::thread_cpu_time().unwrap();
            while bbt_rt::fault::thread_cpu_time()
                .unwrap()
                .saturating_sub(before)
                < Duration::from_micros(120)
            {
                std::hint::spin_loop();
            }
            self.remaining -= 1;
            true
        }
    }
    impl Drop for DropProbe {
        fn drop(&mut self) {
            assert_eq!(self.remaining, 0, "unstepped completed owner destruction");
            assert_eq!(
                self.heap.counters(HeapLane::Work).reserved,
                1024,
                "credit was released before the exact owner"
            );
            self.dropped.store(true, Ordering::Release);
        }
    }
    fn cleanup_green(control: &Control) {
        assert_eq!(control.cleanup.overruns.load(Ordering::Acquire), 0);
        assert!(control.cleanup.max_unit_cpu_ns.load(Ordering::Acquire) <= 1_000_000);
        assert!(control.cleanup.cpu_ns.load(Ordering::Acquire) > 0);
    }
    #[test]
    fn dream_c1_shared_authority_busy_defers_before_work_without_false_stale() {
        let calls = Arc::new(AtomicU64::new(0));
        let checked = calls.clone();
        let control = Control {
            authority_guard: Some(Arc::new(move || {
                if checked.fetch_add(1, Ordering::AcqRel) < 2 {
                    Err(Code::Busy)
                } else {
                    Ok(())
                }
            })),
            ..Control::default()
        };
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(2));
        let mut effects = 0;
        let start = Instant::now();
        budget
            .unit(|| {
                effects += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(effects, 1);
        assert_eq!(calls.load(Ordering::Acquire), 3);
        assert!(start.elapsed() >= REST * 2);
        assert_eq!(control.rest_violations.load(Ordering::Acquire), 0);
        assert_eq!(control.unit_overruns.load(Ordering::Acquire), 0);
    }
    #[test]
    fn dream_c1_captured_control_budget_ignores_work_admission_but_keeps_wall_cpu_bounds() {
        let control = Control {
            authority_guard: Some(Arc::new(|| {
                panic!("forensic metadata asked for current work authority")
            })),
            ..Control::default()
        };
        control.cancelled.store(true, Ordering::Release);
        control.foreground.store(true, Ordering::Release);
        let mut budget = Budget::for_control(&control, Instant::now() + Duration::from_secs(2));
        budget.unit(|| Ok(())).unwrap();
        assert!(matches!(
            budget.finish_unit_sampled(UnitClock::now().unwrap(), Ok(()), |_| Ok(
                UNIT + Duration::from_nanos(1)
            )),
            Err(Code::ResourceLimit)
        ));
        let mut expired = Budget::for_control(&control, Instant::now() - Duration::from_millis(1));
        assert!(matches!(
            expired.unit::<()>(|| panic!("expired metadata work was admitted")),
            Err(Code::Deadline)
        ));
    }
    #[test]
    fn dream_c1_rejected_owned_results_drain_before_credit_for_cpu_clock_cancel_deadline() {
        for failure in 0..4 {
            let control = Control::default();
            control.foreground.store(false, Ordering::Release);
            let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
            let dropped = Arc::new(AtomicBool::new(false));
            let mut owner = Paced::new(
                DropProbe {
                    remaining: 16,
                    heap: control.heap.clone(),
                    dropped: dropped.clone(),
                },
                &control,
            );
            owner.credit = Some(control.heap.reserve(HeapLane::Work, 1024).unwrap());
            if failure == 2 {
                control.cancelled.store(true, Ordering::Release);
            }
            if failure == 3 {
                budget.deadline = Instant::now() - Duration::from_millis(1);
            }
            let before = Instant::now();
            let result =
                budget.finish_unit_sampled(
                    UnitClock::now().unwrap(),
                    Ok(owner),
                    |_| match failure {
                        0 => Ok(UNIT + Duration::from_nanos(1)),
                        1 => Err(Code::ResourceLimit),
                        _ => Ok(Duration::ZERO),
                    },
                );
            assert!(
                matches!(result, Err(Code::ResourceLimit)) && failure < 2
                    || matches!(result, Err(Code::Cancelled)) && failure == 2
                    || matches!(result, Err(Code::Deadline)) && failure == 3
            );
            assert!(dropped.load(Ordering::Acquire));
            assert_eq!(control.heap.counters(HeapLane::Work).reserved, 0);
            assert!(
                before.elapsed() >= REST * 2,
                "cleanup was not divided by real rests"
            );
            cleanup_green(&control);
        }
    }
    fn dense_value() -> Vec<u8> {
        let mut value = json!({"dense":(0..4096).map(|_|json!({"leaf":["retained"]})).collect::<Vec<_>>(),"padding":""});
        let len = serde_json::to_vec(&value).unwrap().len();
        value["padding"] = json!("p".repeat(LINE - len - 32));
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(bytes.len() > LINE - 64 && bytes.len() < LINE);
        bytes
    }
    #[test]
    fn dream_c1_dense_value_success_and_later_error_scope_retirement_are_paced() {
        let bytes = dense_value();
        for later_error in [false, true] {
            let control = Control::default();
            control.foreground.store(false, Ordering::Release);
            let before = Instant::now();
            let result = (|| -> Result<(), Code> {
                let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(10));
                let value: Paced<Value> = budget.parse(&bytes)?;
                assert_eq!(value["dense"].as_array().unwrap().len(), 4096);
                assert!(control.heap.counters(HeapLane::Work).reserved > 0);
                if later_error {
                    return Err(Code::SourceMismatch);
                }
                Ok(())
            })();
            assert_eq!(
                result,
                if later_error {
                    Err(Code::SourceMismatch)
                } else {
                    Ok(())
                }
            );
            assert_eq!(control.heap.counters(HeapLane::Work).reserved, 0);
            assert!(control.cleanup.units.load(Ordering::Acquire) >= 32);
            assert!(before.elapsed() >= REST * 32);
            cleanup_green(&control);
        }
    }
    #[test]
    fn dream_c1_partial_serde_value_after_mid_reader_stop_has_owned_paced_collections() {
        let bytes = dense_value();
        let armed = Arc::new(AtomicBool::new(false));
        let checks = Arc::new(AtomicU64::new(0));
        let (active, count) = (armed.clone(), checks.clone());
        let control = Control {
            authority_guard: Some(Arc::new(move || {
                if active.load(Ordering::Acquire) && count.fetch_add(1, Ordering::AcqRel) >= 1 {
                    Err(Code::Cancelled)
                } else {
                    Ok(())
                }
            })),
            ..Control::default()
        };
        control.foreground.store(false, Ordering::Release);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(10));
        let credit = budget.json(&bytes).unwrap();
        armed.store(true, Ordering::Release);
        let result = budget.parse_credited::<Value>(&bytes, credit);
        assert!(
            matches!(&result, Err(Code::Cancelled)),
            "partial reader result={:?} guard_checks={} max_work_ns={} cleanup_units={} cleanup_cpu_ns={} cleanup_max_ns={} cleanup_overruns={} capacity_refusals={}",
            result.as_ref().err(),
            checks.load(Ordering::Acquire),
            control.max_unit_cpu_ns.load(Ordering::Acquire),
            control.cleanup.units.load(Ordering::Acquire),
            control.cleanup.cpu_ns.load(Ordering::Acquire),
            control.cleanup.max_unit_cpu_ns.load(Ordering::Acquire),
            control.cleanup.overruns.load(Ordering::Acquire),
            control.cleanup.capacity_refusals.load(Ordering::Acquire)
        );
        assert!(checks.load(Ordering::Acquire) >= 2);
        assert_eq!(control.heap.counters(HeapLane::Work).reserved, 0);
        assert!(control.cleanup.units.load(Ordering::Acquire) > 0);
        cleanup_green(&control);
    }
    #[test]
    fn dream_c1_typed_descriptor_and_nested_episode_partial_collections_retire_before_error() {
        for invalid_in_array in [false, true] {
            let control = Control::default();
            control.foreground.store(false, Ordering::Release);
            let mut pages = (0..4096)
                .map(|index| json!({"index":index,"len":1,"sha256":"0".repeat(64)}))
                .collect::<Vec<_>>();
            if invalid_in_array {
                pages[2048]["len"] = json!("bad late page field");
            }
            let descriptor = json!({"schema":"bboxy.dream.c1.result/1","run_id":"01ARZ3NDEKTSV4RRFFQ69G5FAV",
                "nonce":"01ARZ3NDEKTSV4RRFFQ69G5FAW","manifest_sha256":"0".repeat(64),"algorithm":"d0-c1/1",
                "source_count":1,"episode_count":0,"cluster_count":0,"max_poll_cpu_ns":0,
                "knowledge":"not_requested","trust":if invalid_in_array {"not_evaluated"}else{"bad after completed pages"},"pages":pages});
            let bytes = public::canonical(&descriptor).unwrap();
            assert!(bytes.len() > LINE && bytes.len() < 768 * 1024);
            let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(10));
            let result = budget.parse::<w::ResultDescriptor>(&bytes);
            assert!(
                matches!(&result, Err(Code::ProtocolError)),
                "descriptor invalid_in_array={} result={:?} max_work_ns={} cleanup_units={} cleanup_cpu_ns={} cleanup_max_ns={} cleanup_overruns={} capacity_refusals={}",
                invalid_in_array,
                result.as_ref().err(),
                control.max_unit_cpu_ns.load(Ordering::Acquire),
                control.cleanup.units.load(Ordering::Acquire),
                control.cleanup.cpu_ns.load(Ordering::Acquire),
                control.cleanup.max_unit_cpu_ns.load(Ordering::Acquire),
                control.cleanup.overruns.load(Ordering::Acquire),
                control.cleanup.capacity_refusals.load(Ordering::Acquire)
            );
            assert_eq!(control.heap.counters(HeapLane::Work).reserved, 0);
            assert!(control.cleanup.units.load(Ordering::Acquire) >= 16);
            cleanup_green(&control);
        }
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let page = json!({"schema":"bboxy.dream.c1.page/1","run_id":"01ARZ3NDEKTSV4RRFFQ69G5FAV","manifest_sha256":"0".repeat(64),"index":0,"next":null,
            "rows":[{"kind":"episode","session":"01ARZ3NDEKTSV4RRFFQ69G5FAV","turn":"01ARZ3NDEKTSV4RRFFQ69G5FAW",
                "origin":{"session":"01ARZ3NDEKTSV4RRFFQ69G5FAV","seq":1,"digest":"sha256:original"},
                "fingerprints":vec!["0".repeat(64);200],"first_ms":1,"last_ms":null}]});
        let bytes = public::canonical(&page).unwrap();
        assert!(bytes.len() < 16 * 1024);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
        assert!(matches!(
            budget.parse::<w::Page>(&bytes),
            Err(Code::ProtocolError)
        ));
        assert_eq!(control.heap.counters(HeapLane::Work).reserved, 0);
        assert!(control.cleanup.units.load(Ordering::Acquire) >= 2);
        cleanup_green(&control);
    }
}

/// Local executable/profile binding. All IO, hashing and retirement belongs to
/// the existing isolated capability/process owner, never a shared async owner.
pub(crate) mod runner_provision {
    use super::*;
    use serde::Deserialize;
    use std::os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    };

    const SCHEMA: &str = "bboxy.dream.c1.provision/1";
    const PROFILE: &str = "c1-native-debt/1";
    const BOOTSTRAP_PROFILE: &str = "dream-bootstrap/1";
    const RECEIPT_MAX: usize = 8192;
    #[derive(Clone, Copy)]
    struct Expected<'a> {
        source: &'a str,
        compiler: &'a str,
        binary: &'a str,
    }
    // Decision66058 approves this exact canonical Make1.99.1 build closure.
    // This is pre-birth eligibility, not a successful runtime/profile verdict.
    // Match the canonical build-side APPROVED_NATIVE_BUILD; stopped historical
    // workers remain refused separately, without a fallback tuple or retry.
    const APPROVED_NATIVE: Option<Expected<'static>> = Some(Expected {
        source: "2ce4359df034405043d419fe550b0f6cafe7887e",
        compiler: "e003fd3d385fcd0de16bc6b049124efb20207831dd810fd4ab39ff079bbb12a5",
        binary: "2b85c7114545f35a62842115ae91e08e1a22fbfcd24dfd27e3aa45fa1d3837cb",
    });
    // Exact signed development milestone only; never receipt self-approval or
    // final production conformance. Cleared when the reviewed tuple is promoted.
    const PRIVATE_DEVELOPMENT_NATIVE: Option<Expected<'static>> = None;

    #[derive(Clone, Copy, PartialEq, Eq)]
    struct Stamp {
        dev: u64,
        ino: u64,
        len: u64,
        mtime: i64,
        mtime_ns: i64,
        ctime: i64,
        ctime_ns: i64,
        mode: u32,
        uid: u32,
        links: u64,
    }
    impl Stamp {
        fn of(m: &std::fs::Metadata) -> Self {
            Self {
                dev: m.dev(),
                ino: m.ino(),
                len: m.len(),
                mtime: m.mtime(),
                mtime_ns: m.mtime_nsec(),
                ctime: m.ctime(),
                ctime_ns: m.ctime_nsec(),
                mode: m.mode(),
                uid: m.uid(),
                links: m.nlink(),
            }
        }
    }
    struct Seal {
        path: std::mem::ManuallyDrop<PathBuf>,
        file: std::mem::ManuallyDrop<File>,
        stamp: Stamp,
        cleanup: Arc<disposal::Cleanup>,
    }
    impl Seal {
        fn open(path: &Path, receipt: bool, budget: &mut Budget<'_>) -> Result<Self, Code> {
            let control = budget.control;
            if !path.is_absolute() || path.as_os_str().as_bytes().len() > libc::PATH_MAX as usize {
                return Err(Code::RunnerUnavailable);
            }
            let canonical =
                budget.unit(|| path.canonicalize().map_err(|_| Code::RunnerUnavailable))?;
            let mut canonical = Paced::new(canonical, control);
            if canonical.as_path() != path {
                return Err(Code::RunnerUnavailable);
            }
            let file = budget.unit(|| {
                std::fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(path)
                    .map_err(|_| Code::RunnerUnavailable)
            })?;
            let mut file = Paced::new(file, control);
            let m = budget.unit(|| file.metadata().map_err(|_| Code::RunnerUnavailable))?;
            if !m.is_file()
                || m.uid() != unsafe { libc::geteuid() }
                || m.nlink() != 1
                || if receipt {
                    m.len() == 0 || m.len() > RECEIPT_MAX as u64 || m.mode() & 0o077 != 0
                } else {
                    m.mode() & 0o022 != 0 || m.mode() & 0o111 == 0
                }
            {
                return Err(Code::RunnerUnavailable);
            }
            let seal = Self {
                path: std::mem::ManuallyDrop::new(canonical.take_value()),
                file: std::mem::ManuallyDrop::new(file.take_value()),
                stamp: Stamp::of(&m),
                cleanup: control.cleanup.clone(),
            };
            budget.unit(|| seal.current())?;
            Ok(seal)
        }
        fn current(&self) -> Result<(), Code> {
            let held = self.file.metadata().map_err(|_| Code::RunnerUnavailable)?;
            let named = std::fs::symlink_metadata(self.path.as_path())
                .map_err(|_| Code::RunnerUnavailable)?;
            if !named.is_file() || Stamp::of(&held) != self.stamp || Stamp::of(&named) != self.stamp
            {
                Err(Code::RunnerUnavailable)
            } else {
                Ok(())
            }
        }
    }
    impl Drain for Seal {
        fn drain_step(&mut self) -> bool {
            false
        }
    }
    impl Drop for Seal {
        fn drop(&mut self) {
            // Each sole owned field retires once on the isolated owner, with
            // actual close/free CPU measured and debt paid by existing cleanup.
            disposal::cleanup_unit(&self.cleanup, || unsafe {
                std::mem::ManuallyDrop::drop(&mut self.file)
            });
            disposal::cleanup_unit(&self.cleanup, || unsafe {
                std::mem::ManuallyDrop::drop(&mut self.path)
            });
        }
    }
    pub(crate) struct ProvisionProof {
        launch: Seal,
        receipt: Seal,
        worker: Option<(Seal, Seal)>,
        worker_sha256: String,
        private_development: bool,
        _credit: Reservation,
    }
    impl ProvisionProof {
        pub(crate) fn capability_binary(&self) -> &Path {
            self.worker
                .as_ref()
                .map_or(self.launch.path.as_path(), |(worker, _)| {
                    worker.path.as_path()
                })
        }
        pub(crate) fn control_only_wrapper(&self) -> bool {
            self.is_private_wrapper()
        }
        /// Instrumented wrappers are private causal-control paths only: their
        /// interpreter/prelude is not an approved earliest bootstrap gate.
        pub(crate) fn is_private_wrapper(&self) -> bool {
            self.worker.is_some()
        }
        // Four retained paths plus bounded canonical/open/check overlap occupy
        // at most sixteen PATH_MAX slots. Include the token/Arc and its retained
        // verification ledger/cleanup/debt objects, not an opaque-helper debit.
        const RETAINED_DECLARED: usize = 16 * libc::PATH_MAX as usize
            + std::mem::size_of::<Self>()
            + std::mem::size_of::<Heap>()
            + std::mem::size_of::<disposal::Cleanup>()
            + std::mem::size_of::<WorkDebt>()
            + 8 * std::mem::size_of::<usize>()
            + 64;
        pub(crate) fn retained_declared_bytes(&self) -> usize {
            Self::RETAINED_DECLARED
        }
        pub(crate) fn worker_sha256(&self) -> &str {
            &self.worker_sha256
        }
        pub(crate) fn is_private_development(&self) -> bool {
            self.private_development
        }
        /// Metadata only; call immediately before actual spawn on its isolated
        /// owner. Both held descriptor and named inode must remain unchanged.
        pub(crate) fn still_current(&self) -> Result<(), Code> {
            self.launch.current()?;
            self.receipt.current()?;
            if let Some((worker, receipt)) = &self.worker {
                worker.current()?;
                receipt.current()?;
            }
            Ok(())
        }
        /// A private wrapper's JSON is not proof it executed the approved image.
        /// Check the actual child after Hello before enabling native metrics.
        pub(crate) fn child_image_matches(&self, pid: u32) -> Result<(), Code> {
            self.still_current()?;
            if self.is_private_wrapper() {
                // A later exec does not retroactively gate an interpreter's
                // earlier controlled argv/allocation prelude. Fault-only path.
                return Err(Code::RunnerUnavailable);
            }
            #[cfg(target_os = "macos")]
            {
                let mut path = [0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
                let len = unsafe {
                    libc::proc_pidpath(
                        i32::try_from(pid).map_err(|_| Code::RunnerUnavailable)?,
                        path.as_mut_ptr().cast(),
                        path.len() as u32,
                    )
                };
                if len <= 0 {
                    return Err(Code::RunnerUnavailable);
                }
                let end = path
                    .iter()
                    .position(|b| *b == 0)
                    .ok_or(Code::RunnerUnavailable)?;
                let actual = Path::new(std::ffi::OsStr::from_bytes(&path[..end]))
                    .canonicalize()
                    .map_err(|_| Code::RunnerUnavailable)?;
                let expected = self
                    .worker
                    .as_ref()
                    .map_or(&self.launch, |(worker, _)| worker);
                if actual.as_path() != expected.path.as_path() {
                    return Err(Code::RunnerUnavailable);
                }
                expected.current()?;
                Ok(())
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = pid;
                Err(Code::RunnerUnavailable)
            }
        }
    }
    #[derive(Deserialize)]
    struct Role<'a> {
        #[serde(borrow)]
        role: &'a str,
    }
    impl Drain for Role<'_> {
        fn drain_step(&mut self) -> bool {
            false
        }
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct AdmissionImage<'a> {
        #[serde(borrow)]
        path: &'a str,
        device: u64,
        inode: u64,
        sha256: &'a str,
        mtime_ns: &'a str,
        ctime_ns: &'a str,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FirstLaunchAdmission<'a> {
        #[serde(borrow)]
        schema: &'a str,
        argument: &'a str,
        #[serde(borrow)]
        image: AdmissionImage<'a>,
        state: &'a str,
        wall_ns: u64,
        exit_code: i32,
        actual_reap: bool,
        stdout_bytes: u64,
        stderr_bytes: u64,
        stdout_sha256: &'a str,
        stderr_sha256: &'a str,
        pid: u32,
        error: Option<&'a str>,
    }
    impl FirstLaunchAdmission<'_> {
        fn matches(&self, binary: &Seal, expected: Expected<'_>) -> bool {
            let hex = |value: &str| value.len()==64 && value.bytes().all(|b|b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            let stamp = |seconds:i64,nanos:i64| {
                if !(0..1_000_000_000).contains(&nanos) {return None;}
                u64::try_from(seconds).ok().and_then(|v|v.checked_mul(1_000_000_000)).and_then(|v|v.checked_add(nanos as u64))
            };
            self.schema=="bboxy.dream.c1.first-launch/2" && self.argument=="--help"
                && self.state=="admitted" && self.actual_reap && self.exit_code==0
                && self.wall_ns<=2_000_000_000 && self.error.is_none() && self.pid>0
                && self.stdout_bytes<=8192 && self.stderr_bytes<=8192
                && hex(self.stdout_sha256) && hex(self.stderr_sha256)
                && Path::new(self.image.path)==binary.path.as_path()
                && self.image.device==binary.stamp.dev && self.image.inode==binary.stamp.ino
                && self.image.sha256==expected.binary
                && matches!((self.image.mtime_ns.parse::<u64>().ok(),stamp(binary.stamp.mtime,binary.stamp.mtime_ns)),(Some(a),Some(b)) if a==b)
                && matches!((self.image.ctime_ns.parse::<u64>().ok(),stamp(binary.stamp.ctime,binary.stamp.ctime_ns)),(Some(a),Some(b)) if a==b)
        }
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct WorkerReceipt<'a> {
        #[serde(borrow)]
        schema: &'a str,
        role: &'a str,
        metrics_profile: &'a str,
        bootstrap_profile: &'a str,
        source_commit: &'a str,
        compiler_sha256: &'a str,
        binary_sha256: &'a str,
        build_profile: &'a str,
        default_features: bool,
        #[serde(borrow)]
        explicit_features: [&'a str; 0],
        #[serde(borrow)]
        effective_features: [&'a str; 1],
        test_fixtures: bool,
        #[serde(borrow)]
        first_launch_admission: FirstLaunchAdmission<'a>,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct WrapperReceipt<'a> {
        #[serde(borrow)]
        schema: &'a str,
        role: &'a str,
        metrics_profile: &'a str,
        bootstrap_profile: &'a str,
        binary_sha256: &'a str,
        actual_worker: &'a str,
        actual_worker_sha256: &'a str,
    }
    impl Drain for WrapperReceipt<'_> {
        fn drain_step(&mut self) -> bool {
            false
        }
    }
    fn read_receipt(
        seal: &mut Seal,
        budget: &mut Budget<'_>,
    ) -> Result<([u8; RECEIPT_MAX], usize), Code> {
        let len = usize::try_from(seal.stamp.len).map_err(|_| Code::RunnerUnavailable)?;
        let mut bytes = [0u8; RECEIPT_MAX];
        budget.unit(|| {
            seal.file
                .read_exact(&mut bytes[..len])
                .map_err(|_| Code::RunnerUnavailable)
        })?;
        budget.unit(|| seal.current())?;
        budget.unit(|| {
            transport::preflight_json(&bytes[..len], transport::JsonShape::Any)
                .map(|_| ())
                .map_err(|_| Code::RunnerUnavailable)
        })?;
        Ok((bytes, len))
    }
    fn hash_binary(seal: &mut Seal, budget: &mut Budget<'_>) -> Result<String, Code> {
        let mut chunk = [0u8; 64 * 1024];
        let mut hash = Sha256::new();
        let mut left = seal.stamp.len;
        while left > 0 {
            let count = usize::try_from(left.min(chunk.len() as u64))
                .map_err(|_| Code::RunnerUnavailable)?;
            budget.unit(|| {
                seal.file
                    .read_exact(&mut chunk[..count])
                    .map_err(|_| Code::RunnerUnavailable)
            })?;
            budget.unit(|| {
                hash.update(&chunk[..count]);
                Ok(())
            })?;
            left -= count as u64;
        }
        budget.unit(|| seal.current())?;
        budget.unit(|| Ok(format!("{:x}", hash.finalize())))
    }
    fn worker_fields(
        bytes: &[u8],
        expected: Expected<'_>,
        binary: &Seal,
        budget: &mut Budget<'_>,
    ) -> Result<(), Code> {
        budget.unit(|| {
            let value: WorkerReceipt<'_> =
                serde_json::from_slice(bytes).map_err(|_| Code::RunnerUnavailable)?;
            if value.schema != SCHEMA
                || value.role != "worker"
                || value.metrics_profile != PROFILE
                || value.bootstrap_profile != BOOTSTRAP_PROFILE
                || value.source_commit != expected.source
                || value.compiler_sha256 != expected.compiler
                || value.binary_sha256 != expected.binary
                || value.build_profile != "release"
                || !value.default_features
                || !value.explicit_features.is_empty()
                || value.effective_features != ["acp-agent"]
                || value.test_fixtures
                || !value.first_launch_admission.matches(binary, expected)
            {
                return Err(Code::RunnerUnavailable);
            }
            Ok(())
        })
    }
    fn worker_pair(
        binary: &Path,
        expected: Expected<'_>,
        budget: &mut Budget<'_>,
    ) -> Result<(Seal, Seal), Code> {
        let mut binary = Seal::open(binary, false, budget)?;
        let mut receipt = Seal::open(&binary.path.with_extension("dream-c1.json"), true, budget)?;
        let (bytes, len) = read_receipt(&mut receipt, budget)?;
        worker_fields(&bytes[..len], expected, &binary, budget)?;
        if hash_binary(&mut binary, budget)? != expected.binary {
            return Err(Code::RunnerUnavailable);
        }
        Ok((binary, receipt))
    }
    fn verify_expected(
        binary: &Path,
        private_test_account: bool,
        expected: Expected<'_>,
    ) -> Result<ProvisionProof, Code> {
        let hex = |value: &str, len: usize| {
            value.len() == len
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if !hex(expected.source, 40) || !hex(expected.compiler, 64) || !hex(expected.binary, 64) {
            return Err(Code::RunnerUnavailable);
        }
        utility()?;
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        // Sixteen PATH_MAX slots cover retained/fresh/canonical paths across
        // the one permitted wrapper level; parser strings borrow the8KiB input.
        let credit = control
            .heap
            .reserve(HeapLane::Work, ProvisionProof::RETAINED_DECLARED)?;
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(30));
        let mut launch = Seal::open(binary, false, &mut budget)?;
        let mut receipt = Seal::open(&binary.with_extension("dream-c1.json"), true, &mut budget)?;
        let (bytes, len) = read_receipt(&mut receipt, &mut budget)?;
        let role = budget.unit(|| {
            serde_json::from_slice::<Role<'_>>(&bytes[..len]).map_err(|_| Code::RunnerUnavailable)
        })?;
        let worker_sha256 = budget.unit(|| Ok(expected.binary.to_owned()))?;
        if role.role == "worker" {
            worker_fields(&bytes[..len], expected, &launch, &mut budget)?;
            if hash_binary(&mut launch, &mut budget)? != expected.binary {
                return Err(Code::RunnerUnavailable);
            }
            budget.pay_work_debt()?;
            return Ok(ProvisionProof {
                launch,
                receipt,
                worker: None,
                worker_sha256,
                private_development: false,
                _credit: credit,
            });
        }
        if role.role != "private_wrapper" || !cfg!(debug_assertions) || !private_test_account {
            return Err(Code::RunnerUnavailable);
        }
        let value = budget.unit(|| {
            serde_json::from_slice::<WrapperReceipt<'_>>(&bytes[..len])
                .map_err(|_| Code::RunnerUnavailable)
        })?;
        if value.schema != SCHEMA
            || value.role != "private_wrapper"
            || value.metrics_profile != PROFILE
            || value.bootstrap_profile != BOOTSTRAP_PROFILE
            || value.actual_worker_sha256 != expected.binary
            || !hex(value.binary_sha256, 64)
        {
            return Err(Code::RunnerUnavailable);
        }
        if hash_binary(&mut launch, &mut budget)? != value.binary_sha256 {
            return Err(Code::RunnerUnavailable);
        }
        let worker = worker_pair(Path::new(value.actual_worker), expected, &mut budget)?;
        let proof = ProvisionProof {
            launch,
            receipt,
            worker: Some(worker),
            worker_sha256,
            private_development: false,
            _credit: credit,
        };
        budget.unit(|| proof.still_current())?;
        budget.pay_work_debt()?;
        Ok(proof)
    }
    /// Verify BEFORE any candidate process birth, including the source-free
    /// probe. Unknown/old workers cannot be executed to discover gate support.
    /// Runs on the isolated capability owner; never probes/builds/searches PATH.
    pub(crate) fn verify_native_provision(
        binary: &Path,
        private_test_account: bool,
    ) -> Result<ProvisionProof, Code> {
        // Select at most one immutable tuple before verification. There is no
        // retry under another tuple after a timing/hash/metadata refusal.
        let development =
            APPROVED_NATIVE.is_none() && cfg!(debug_assertions) && private_test_account;
        let expected = if development {
            PRIVATE_DEVELOPMENT_NATIVE
        } else {
            APPROVED_NATIVE
        };
        let mut proof = verify_approved(binary, private_test_account, expected)?;
        proof.private_development = development;
        Ok(proof)
    }
    fn verify_approved(
        binary: &Path,
        private_test_account: bool,
        expected: Option<Expected<'_>>,
    ) -> Result<ProvisionProof, Code> {
        verify_expected(
            binary,
            private_test_account,
            expected.ok_or(Code::RunnerUnavailable)?,
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        const SOURCE: &str = "1111111111111111111111111111111111111111";
        const COMPILER: &str = "2222222222222222222222222222222222222222222222222222222222222222";
        fn write(path: &Path, bytes: &[u8], mode: u32) {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(mode)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(path)
                .unwrap();
            file.write_all(bytes).unwrap();
            file.sync_all().unwrap();
        }
        fn fixture() -> (tempfile::TempDir, PathBuf, String, Value) {
            let root = tempfile::tempdir().unwrap();
            let dir = root.path().canonicalize().unwrap();
            let binary = dir.join("worker");
            let bytes = b"private validator image, never executed";
            write(&binary, bytes, 0o700);
            let digest = format!("{:x}", Sha256::digest(bytes));
            let metadata=std::fs::metadata(&binary).unwrap();
            let admission=json!({"schema":"bboxy.dream.c1.first-launch/2","argument":"--help","image":{"path":binary,"device":metadata.dev(),"inode":metadata.ino(),"sha256":digest,
                "mtime_ns":(metadata.mtime() as u64*1_000_000_000+metadata.mtime_nsec() as u64).to_string(),"ctime_ns":(metadata.ctime() as u64*1_000_000_000+metadata.ctime_nsec() as u64).to_string()},
                "state":"admitted","wall_ns":1_000_000,"exit_code":0,"actual_reap":true,
                "stdout_bytes":0,"stderr_bytes":0,"stdout_sha256":format!("{:x}",Sha256::digest([])),
                "stderr_sha256":format!("{:x}",Sha256::digest([])),"pid":std::process::id(),"error":null});
            let receipt = json!({"schema":SCHEMA,"role":"worker","metrics_profile":PROFILE,"bootstrap_profile":BOOTSTRAP_PROFILE,"source_commit":SOURCE,
                "compiler_sha256":COMPILER,"binary_sha256":digest,"build_profile":"release","default_features":true,
                "explicit_features":[],"effective_features":["acp-agent"],"test_fixtures":false,"first_launch_admission":admission});
            (root, binary, digest, receipt)
        }
        fn expected(digest: &str) -> Expected<'_> {
            Expected {
                source: SOURCE,
                compiler: COMPILER,
                binary: digest,
            }
        }
        fn receipt(binary: &Path, value: &Value) {
            write(
                &binary.with_extension("dream-c1.json"),
                &serde_json::to_vec(value).unwrap(),
                0o600,
            );
        }

        #[test]
        fn native_provision_no_approved_tuple_never_grants_from_json() {
            assert!(matches!(
                verify_approved(Path::new("/unread/unapproved"), true, None),
                Err(Code::RunnerUnavailable)
            ));
        }
        #[test]
        fn native_provision_first_launch_missing_failed_unknown_and_foreign_inode_refuse() {
            for field in ["missing","wall_ns","exit_code","actual_reap","state","path","inode","sha256","ctime_ns","argument","schema"] {
                let (_root,binary,digest,mut value)=fixture();
                match field {
                    "missing"=>{value.as_object_mut().unwrap().remove("first_launch_admission");},
                    "wall_ns"=>value["first_launch_admission"][field]=json!(2_000_000_001u64),
                    "exit_code"=>value["first_launch_admission"][field]=json!(1),
                    "actual_reap"=>value["first_launch_admission"][field]=json!(false),
                    "state"=>value["first_launch_admission"][field]=json!("started"),
                    "argument"=>value["first_launch_admission"][field]=json!("--version"),
                    "schema"=>value["first_launch_admission"][field]=json!("bboxy.dream.c1.first-launch/1"),
                    "path"=>value["first_launch_admission"]["image"][field]=json!("/foreign/worker"),
                    "inode"=>value["first_launch_admission"]["image"][field]=json!(0),
                    "sha256"=>value["first_launch_admission"]["image"][field]=json!("0".repeat(64)),
                    "ctime_ns"=>value["first_launch_admission"]["image"][field]=json!("0"),
                    _=>unreachable!(),
                }
                receipt(&binary,&value);
                assert!(matches!(verify_expected(&binary,false,expected(&digest)),Err(Code::RunnerUnavailable)),"{field}");
                assert_eq!(std::fs::read(binary).unwrap(),b"private validator image, never executed");
            }
        }
        #[test]
        fn native_provision_missing_bootstrap_profile_refuses_without_candidate_birth() {
            let (_root, binary, digest, mut value) = fixture();
            value.as_object_mut().unwrap().remove("bootstrap_profile");
            receipt(&binary, &value);
            assert!(matches!(
                verify_expected(&binary, false, expected(&digest)),
                Err(Code::RunnerUnavailable)
            ));
            // The validator only opens/hashes owned bytes. This deliberately
            // non-executable image is never run to discover bootstrap support.
            assert_eq!(
                std::fs::read(&binary).unwrap(),
                b"private validator image, never executed"
            );
        }
        #[test]
        fn native_provision_exact_tuple_hash_and_inode_remain_bound() {
            let (_root, binary, digest, value) = fixture();
            receipt(&binary, &value);
            let proof = verify_expected(&binary, false, expected(&digest)).unwrap();
            proof.still_current().unwrap();
            assert!(
                matches!(
                    proof.child_image_matches(std::process::id()),
                    Err(Code::RunnerUnavailable)
                ),
                "a copied receipt cannot authorize a different actual process image"
            );
            let old = binary.with_extension("old");
            std::fs::rename(&binary, &old).unwrap();
            std::fs::copy(&old, &binary).unwrap();
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
            assert!(
                matches!(proof.still_current(), Err(Code::RunnerUnavailable)),
                "equal bytes with another named inode must refuse"
            );
        }
        #[test]
        fn native_provision_wrong_compiler_source_features_profile_and_unknown_fields_refuse() {
            for field in [
                "source_commit",
                "compiler_sha256",
                "metrics_profile",
                "bootstrap_profile",
                "test_fixtures",
                "explicit_features",
                "effective_features",
                "unknown",
            ] {
                let (_root, binary, digest, mut value) = fixture();
                value[field] = match field {
                    "test_fixtures" => json!(true),
                    "explicit_features" => json!(["test-fixtures"]),
                    "effective_features" => json!(["other"]),
                    _ => json!("mismatch"),
                };
                receipt(&binary, &value);
                assert!(
                    matches!(
                        verify_expected(&binary, false, expected(&digest)),
                        Err(Code::RunnerUnavailable)
                    ),
                    "unapproved {field}"
                );
            }
        }
        #[test]
        fn native_provision_symlink_links_oversize_and_binary_mutation_refuse() {
            for fault in ["symlink", "links", "oversize", "binary"] {
                let (_root, binary, digest, value) = fixture();
                let path = binary.with_extension("dream-c1.json");
                receipt(&binary, &value);
                match fault {
                    "symlink" => {
                        let other = path.with_extension("held");
                        std::fs::rename(&path, &other).unwrap();
                        std::os::unix::fs::symlink(other, &path).unwrap();
                    }
                    "links" => {
                        std::fs::hard_link(&path, path.with_extension("link")).unwrap();
                    }
                    "oversize" => {
                        std::fs::write(&path, vec![b' '; RECEIPT_MAX + 1]).unwrap();
                    }
                    "binary" => {
                        std::fs::write(&binary, b"changed private executable payload").unwrap();
                    }
                    _ => unreachable!(),
                }
                assert!(
                    matches!(
                        verify_expected(&binary, false, expected(&digest)),
                        Err(Code::RunnerUnavailable)
                    ),
                    "unsafe {fault}"
                );
            }
        }
        #[test]
        #[cfg(debug_assertions)]
        fn native_provision_private_wrapper_needs_host_authority_and_current_actual_worker() {
            let (root, worker, digest, value) = fixture();
            receipt(&worker, &value);
            let wrapper = root.path().canonicalize().unwrap().join("wrapper");
            let bytes = b"#!/bin/sh\nexit 0\n";
            write(&wrapper, bytes, 0o700);
            let value = json!({"schema":SCHEMA,"role":"private_wrapper","metrics_profile":PROFILE,"bootstrap_profile":BOOTSTRAP_PROFILE,
                "binary_sha256":format!("{:x}",Sha256::digest(bytes)),"actual_worker":worker,"actual_worker_sha256":digest});
            receipt(&wrapper, &value);
            assert!(matches!(
                verify_expected(&wrapper, false, expected(&digest)),
                Err(Code::RunnerUnavailable)
            ));
            let proof = verify_expected(&wrapper, true, expected(&digest)).unwrap();
            proof.still_current().unwrap();
            assert!(matches!(
                proof.child_image_matches(std::process::id()),
                Err(Code::RunnerUnavailable)
            ));
            std::fs::write(&worker, b"changed actual worker").unwrap();
            assert!(
                matches!(proof.still_current(), Err(Code::RunnerUnavailable)),
                "wrapper must retain actual worker FD/stamp too"
            );
        }
    }
}

#[cfg(test)]
mod quantum_debt_tests {
    use super::*;
    use serde_json::json;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    // These are injected-timing causal tests with real private effects. They
    // are not measurements of native syscall maxima or joined conformance.
    fn control() -> Control {
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        control
    }
    fn sample(cpu_us: u64, total_us: u64, wall_ms: u64) -> NativeSample {
        NativeSample {
            cpu: Ok(Duration::from_micros(cpu_us)),
            wall: Duration::from_millis(wall_ms),
            inclusive: Ok(Duration::from_micros(total_us)),
        }
    }
    fn staged(dir: &Path, nonce: &str) -> Publication {
        let mut publication = Publication::new(dir, "manifest.json", nonce).unwrap();
        publication.write(b"private exact staged effect\n").unwrap();
        publication.sync_file().unwrap();
        let identity = publication.identity().unwrap();
        publication.precheck_noreplace(identity).unwrap();
        publication
    }
    fn assert_exact_link(publication: &Publication) {
        let staged = std::fs::symlink_metadata(&publication.temporary).unwrap();
        let linked = std::fs::symlink_metadata(&publication.target).unwrap();
        assert_eq!((staged.dev(), staged.ino()), (linked.dev(), linked.ino()));
        assert_eq!(linked.nlink(), 2);
        assert_eq!(
            std::fs::read(&publication.target).unwrap(),
            b"private exact staged effect\n"
        );
    }

    #[test]
    fn dream_c1_before_start_activity_defers_without_spending_final_attempt() {
        let control = control();
        let run = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let nonce = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
        let manifest = "0".repeat(64);
        let deadline = Instant::now() + Duration::from_secs(5);
        let first = prepare_start_frame(&control, run, nonce, &manifest, deadline, 0).unwrap();
        assert!(!first.as_bytes().is_empty());
        assert_eq!(control.cleanup.debt.remaining_ns(), 0);
        control.foreground.store(true, Ordering::Release);
        control.activity_revision.store(1, Ordering::Release);
        assert!(!start_frame_current(&control, 0));
        assert_eq!(control.start_admission_attempts.load(Ordering::Acquire), 0);
        assert!(!control.cancelled.load(Ordering::Acquire));
        control.foreground.store(false, Ordering::Release);
        control.activity_revision.store(2, Ordering::Release);
        assert!(!start_frame_current(&control, 0));
        let next = prepare_start_frame(&control, run, nonce, &manifest, deadline, 2).unwrap();
        assert!(start_frame_current(&control, 2));
        assert_eq!(control.start_admission_attempts.load(Ordering::Acquire), 0);
        assert_ne!(first.as_bytes(), next.as_bytes());
        final_admission_control(&control, FinalHandoff::Start, || Ok(())).unwrap();
        assert_eq!(control.start_admission_attempts.load(Ordering::Acquire), 1);
        assert_eq!(control.work_unit_count.load(Ordering::Acquire), 2);
    }

    #[test]
    fn dream_c1_probe_claim_survives_owner_loss_and_preserves_replacement() {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        private_dir(root.path()).unwrap();
        let mut original = ProbeClaim::create(root.path(), [1; 16]).unwrap();
        original.file.sync_all().unwrap();
        assert!(ProbeClaim::create(root.path(), [2; 16]).is_err());
        let old_inode = original.identity;
        // A new in-process owner sees the same durable exclusion after the old
        // descriptor disappears. There is no restart adoption or birth retry.
        drop(original);
        assert!(ProbeClaim::create(root.path(), [3; 16]).is_err());
        let named = root.path().join("bootstrap-probe.claim");
        assert_eq!(
            (
                std::fs::metadata(&named).unwrap().dev(),
                std::fs::metadata(&named).unwrap().ino()
            ),
            old_inode
        );
        // Reopen the original exact custody only to exercise its release fence;
        // this is a private metadata test, never Runtime predecessor adoption.
        original = ProbeClaim {
            path: named.clone(),
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&named)
                .unwrap(),
            identity: old_inode,
            token: [1; 16],
        };
        let retired = root.path().join("test-owned-retired-claim");
        std::fs::rename(&named, &retired).unwrap();
        let mut replacement = ProbeClaim::create(root.path(), [4; 16]).unwrap();
        assert!(original.release().is_err());
        assert_eq!(std::fs::read(&named).unwrap(), [4; 16]);
        replacement.release().unwrap();
        std::fs::remove_file(retired).unwrap();
    }

    #[test]
    fn dream_c1_probe_empty_cleanup_is_exact_nonrecursive_and_inode_fenced() {
        use std::os::unix::fs::DirBuilderExt;
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        private_dir(root.path()).unwrap();
        let mut directories = ProbeDirectories::new(root.path(), crate::Ulid::new()).unwrap();
        directories.create_job().unwrap();
        directories.create_temporary().unwrap();
        let foreign = directories.temporary.join("test-owned-foreign-content");
        std::fs::write(&foreign, b"preserve exact bytes").unwrap();
        assert!(directories.remove_temporary().is_err());
        assert_eq!(std::fs::read(&foreign).unwrap(), b"preserve exact bytes");
        std::fs::remove_file(&foreign).unwrap();
        let retired = directories.job.join("test-owned-retired-tmp");
        std::fs::rename(&directories.temporary, &retired).unwrap();
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&directories.temporary)
            .unwrap();
        assert!(
            directories.remove_temporary().is_err(),
            "replacement inode was deleted"
        );
        assert!(directories.temporary.is_dir());
        std::fs::remove_dir(&directories.temporary).unwrap();
        std::fs::rename(&retired, &directories.temporary).unwrap();
        directories.remove_temporary().unwrap();
        directories.remove_job().unwrap();
        assert!(!directories.job.exists());
        assert!(std::mem::size_of::<ProbeAudit>() <= 1024);
        let audit = ProbeOutcome::refused("provision", Code::RunnerUnavailable).audit;
        assert!(!audit.known_birth && !audit.actual_reaped && !audit.parent_cost_known);
        assert!(serde_json::to_vec(&audit).unwrap().len() <= 4096);
    }

    #[test]
    fn dream_c1_native_full_inclusive_cost_owes_rest_after_effect_and_forbids_original_retry() {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let control = control();
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
        let mut publication = staged(root.path(), "original");
        let before = control.cleanup.debt.now_ns();
        let mut outcome = budget
            .native_publish_impl(&mut publication, Some(sample(2000, 2500, 1)))
            .unwrap();
        let effect = outcome.effect.take_value();
        assert!(
            effect.is_ok(),
            "real no-replace effect must be retained before its verdict"
        );
        assert_eq!(outcome.verdict, Ok(()));
        let charged = control.cleanup.debt.charged_ns.load(Ordering::Acquire);
        assert_eq!(
            charged, 22_500_000,
            "full2.5ms owes22.5ms; native cost cannot be subtracted"
        );
        let due = control.cleanup.debt.due_ns.load(Ordering::Acquire);
        assert!(
            due.saturating_sub(charged) >= before,
            "saved earlier idle time paid a later effect"
        );
        budget.pay_work_debt().unwrap();
        assert_eq!(control.cleanup.debt.remaining_ns(), 0);
        assert!(control.cleanup.debt.now_ns() >= due);
        assert_exact_link(&publication);
        assert!(matches!(
            budget.native_publish_impl(&mut publication, Some(sample(0, 0, 0))),
            Err(Code::BadRequest)
        ));
        assert_eq!(
            control
                .cleanup
                .debt
                .native_publish_calls
                .load(Ordering::Acquire),
            1
        );
        assert_eq!(
            control.cleanup.debt.native_effects.load(Ordering::Acquire),
            1
        );
    }

    #[test]
    fn dream_c1_native_link_cpu_wall_and_clock_refusals_preserve_real_inode() {
        for forced in [
            sample(3000, 3000, 1),
            sample(1000, 1000, 26),
            NativeSample {
                cpu: Err(Code::ResourceLimit),
                wall: Duration::from_millis(1),
                inclusive: Err(Code::ResourceLimit),
            },
        ] {
            let root = tempfile::tempdir().unwrap();
            let control = control();
            let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
            let mut publication = staged(root.path(), "refused");
            let mut outcome = budget
                .native_publish_impl(&mut publication, Some(forced))
                .unwrap();
            assert!(outcome.effect.take_value().is_ok());
            assert_eq!(outcome.verdict, Err(Code::ResourceLimit));
            assert!(publication.attempted);
            assert_exact_link(&publication);
            assert_eq!(
                control.cleanup.debt.native_effects.load(Ordering::Acquire),
                1
            );
            if forced.cpu.is_err() {
                assert!(control.cleanup.debt.poisoned.load(Ordering::Acquire));
                assert_eq!(budget.pay_work_debt(), Err(Code::ResourceLimit));
                assert!(matches!(budget.unit(|| Ok(())), Err(Code::ResourceLimit)));
            } else {
                assert_eq!(
                    control.cleanup.debt.charged_ns.load(Ordering::Acquire),
                    nanos(forced.inclusive.unwrap().max(UNIT)) * 9
                );
                budget.pay_work_debt().unwrap();
            }
            // Repeating an original operation must not turn a timing failure
            // into StorageUnavailable or a second no-replace attempt.
            assert!(matches!(
                budget.native_publish_impl(&mut publication, Some(forced)),
                Err(Code::BadRequest)
            ));
            assert_exact_link(&publication);
        }
    }

    type ComponentPipes = (
        std::io::PipeWriter,
        std::io::PipeReader,
        std::io::PipeReader,
    );
    fn prepared_component(job: &Path) -> (ComponentPipes, std::process::Command) {
        use std::os::unix::process::CommandExt;
        let (input, write) = std::io::pipe().unwrap();
        let (read, output) = std::io::pipe().unwrap();
        let (errors, stderr) = std::io::pipe().unwrap();
        // Native-effect component only. This is not a Dream worker/Hello/gate
        // launcher and cannot establish bootstrap or producer conformance.
        let mut command = std::process::Command::new("/bin/sleep");
        command
            .arg("30")
            .env_clear()
            .current_dir(job)
            .stdin(std::process::Stdio::from(input))
            .stdout(std::process::Stdio::from(output))
            .stderr(std::process::Stdio::from(stderr));
        // SAFETY: child-only fixed async-signal-safe process/group/priority setup.
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 || libc::setpriority(libc::PRIO_PROCESS, 0, 10) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        ((write, read, errors), command)
    }
    struct ComponentChild(std::process::Child);
    impl Drop for ComponentChild {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }
    fn reap_component(child: &mut ComponentChild, pid: u32, start: &str) {
        assert_eq!(transport::process_stamp(pid).unwrap(), start);
        assert!(child.0.try_wait().unwrap().is_none());
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        let mut status = 0;
        // SAFETY: queries only the exact child just reaped by its sole owner.
        assert_eq!(
            unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
        assert!(!transport::process_stamp(pid).is_ok_and(|current| current == start));
    }

    #[test]
    fn dream_c1_native_process_bad_timing_retains_component_child_until_actual_reap() {
        for forced in [
            sample(3000, 3000, 1),
            sample(1000, 1000, 26),
            NativeSample {
                cpu: Err(Code::ResourceLimit),
                wall: Duration::from_millis(1),
                inclusive: Err(Code::ResourceLimit),
            },
        ] {
            let root = tempfile::tempdir().unwrap();
            let job = root.path().canonicalize().unwrap();
            let (_pipes, mut command) = prepared_component(&job);
            let control = control();
            let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
            let mut outcome = budget
                .native_process_impl(&mut command, Some(forced))
                .unwrap();
            let child = outcome
                .effect
                .take_value()
                .expect("real effect must be captured before failed verdict");
            let mut child = ComponentChild(child);
            drop(command);
            let pid = child.0.id();
            let start = transport::process_stamp(pid).unwrap();
            assert_eq!(outcome.verdict, Err(Code::ResourceLimit));
            assert_eq!(
                control
                    .cleanup
                    .debt
                    .native_process_calls
                    .load(Ordering::Acquire),
                1
            );
            assert_eq!(
                control.cleanup.debt.native_effects.load(Ordering::Acquire),
                1
            );
            if forced.cpu.is_ok() {
                budget.pay_work_debt().unwrap();
            } else {
                assert_eq!(budget.pay_work_debt(), Err(Code::ResourceLimit));
            }
            // Component failure settlement retains the real typed process,
            // including clock failure. No C1 candidate is launched ungated.
            reap_component(&mut child, pid, &start);
        }
    }

    #[test]
    fn dream_c1_real_native_effect_cancel_and_aba_between_effect_and_verdict_never_retries() {
        for aba in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let epoch = Arc::new(AtomicU64::new(1));
            let observed = epoch.clone();
            let control = Arc::new(Control {
                authority_guard: Some(Arc::new(move || {
                    if observed.load(Ordering::Acquire) == 1 {
                        Ok(())
                    } else {
                        Err(Code::StaleContext)
                    }
                })),
                ..Control::default()
            });
            control.foreground.store(false, Ordering::Release);
            let mut publication = staged(root.path(), "interrupt");
            let interrupted = control.clone();
            NATIVE_RETURN_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move || {
                    if aba {
                        epoch.store(2, Ordering::Release);
                        epoch.store(3, Ordering::Release); // A label returns; incarnation does not.
                    } else {
                        interrupted.cancelled.store(true, Ordering::Release);
                    }
                }));
            });
            let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
            let mut outcome = budget
                .native_publish_impl(&mut publication, Some(sample(1000, 1000, 1)))
                .unwrap();
            assert!(
                outcome.effect.take_value().is_ok(),
                "real inode disappeared on an interrupt verdict"
            );
            if aba {
                // native_finish returns the typed effect immediately. Fresh
                // authority is rechecked at the next ordinary/debt boundary.
                assert_eq!(outcome.verdict, Ok(()));
                assert!(matches!(budget.unit(|| Ok(())), Err(Code::StaleContext)));
            } else {
                assert_eq!(outcome.verdict, Err(Code::Cancelled));
                assert!(matches!(budget.unit(|| Ok(())), Err(Code::Cancelled)));
            }
            assert_exact_link(&publication);
            assert_eq!(
                control.cleanup.debt.native_effects.load(Ordering::Acquire),
                1
            );
            assert!(matches!(
                budget.native_publish_impl(&mut publication, Some(sample(0, 0, 0))),
                Err(Code::BadRequest)
            ));
        }
    }

    #[test]
    fn dream_c1_withheld_returned_effect_retains_lease_and_independent_cached_controls() {
        let root = tempfile::tempdir().unwrap();
        let control = Arc::new(control());
        let publication = staged(root.path(), "held");
        let target = publication.target.clone();
        let identity = publication.identity().unwrap();
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
        std::thread::scope(|scope| {
            let owner = control.clone();
            let thread = scope.spawn(move || {
                NATIVE_RETURN_HOOK.with(|hook| {
                    *hook.borrow_mut() = Some(Box::new(move || {
                        ready_tx.send(()).unwrap();
                        release_rx
                            .recv_timeout(Duration::from_secs(2))
                            .expect("private returned-effect hold must be released autonomously");
                    }));
                });
                let mut publication = publication;
                let mut budget = Budget::new(&owner, Instant::now() + Duration::from_secs(5));
                // Controlled >25ms verdict; the hook is a returned-effect hold,
                // not evidence of an uninterruptible kernel call or real timing.
                let mut outcome = budget
                    .native_publish_impl(&mut publication, Some(sample(1000, 1000, 26)))
                    .unwrap();
                let effect = outcome.effect.take_value();
                done_tx.send(outcome.verdict).unwrap();
                assert!(effect.is_ok());
                assert_exact_link(&publication);
            });
            ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(control.cleanup.debt.held.load(Ordering::Acquire));
            let linked = std::fs::symlink_metadata(&target).unwrap();
            assert_eq!((linked.dev(), linked.ino()), identity);
            assert!(matches!(
                done_rx.try_recv(),
                Err(std::sync::mpsc::TryRecvError::Empty)
            ));
            for _ in 0..200 {
                let at = Instant::now();
                // This is the cached Control lane, explicitly not an admitted
                // MCP/ctl latency claim; those real API witnesses stay separate.
                let _ = control.state.load(Ordering::Acquire);
                control.cancelled.store(true, Ordering::Release);
                let _ = control.revision.load(Ordering::Acquire);
                assert!(at.elapsed() <= Duration::from_millis(100));
                assert!(control.cleanup.debt.held.load(Ordering::Acquire));
            }
            assert!(
                matches!(
                    done_rx.try_recv(),
                    Err(std::sync::mpsc::TryRecvError::Empty)
                ),
                "effect owner retired before the held verdict returned"
            );
            release_tx.send(()).unwrap();
            assert_eq!(
                done_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
                Err(Code::ResourceLimit)
            );
            thread.join().unwrap();
        });
        assert!(!control.cleanup.debt.held.load(Ordering::Acquire));
        assert_eq!(
            control
                .cleanup
                .debt
                .native_publish_calls
                .load(Ordering::Acquire),
            1
        );
        assert_eq!(
            control.cleanup.debt.native_effects.load(Ordering::Acquire),
            1
        );
    }

    #[test]
    fn dream_c1_two_ms_parser_control_sqlite_cleanup_do_not_acquire_native_exemption() {
        for class in ["parser", "control tick", "SQLite wrapper"] {
            let control = control();
            let sql = rusqlite::Connection::open_in_memory().unwrap();
            let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
            let clock = budget.begin_unit().unwrap();
            let actual_owned_value: Value = match class {
                "parser" => serde_json::from_slice(br#"{"retained":[1,2,3]}"#).unwrap(),
                "control tick" => {
                    json!({"state":control.state.load(Ordering::Acquire),"revision":control.revision.load(Ordering::Acquire)})
                }
                "SQLite wrapper" => {
                    json!({"row":sql.query_row("SELECT 1", [], |row|row.get::<_,i64>(0)).unwrap()})
                }
                _ => unreachable!(),
            };
            let result = budget.finish_unit_sampled(clock, Ok(actual_owned_value), |_| {
                Ok(Duration::from_millis(2))
            });
            assert!(
                matches!(result, Err(Code::ResourceLimit)),
                "{class} was falsely excused as native"
            );
            assert_eq!(
                control
                    .cleanup
                    .debt
                    .native_process_calls
                    .load(Ordering::Acquire),
                0
            );
            assert_eq!(
                control
                    .cleanup
                    .debt
                    .native_publish_calls
                    .load(Ordering::Acquire),
                0
            );
            assert_eq!(control.unit_overruns.load(Ordering::Acquire), 1);
        }
        let control = control();
        let actual_owned_value: Value = serde_json::from_slice(br#"{"retained":[1,2,3]}"#).unwrap();
        drop(Paced::new(actual_owned_value, &control));
        disposal::record_cleanup(&control.cleanup, Duration::from_millis(2));
        assert_eq!(control.cleanup.overruns.load(Ordering::Acquire), 1);
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
        assert!(matches!(budget.unit(|| Ok(())), Err(Code::ResourceLimit)));
    }

    #[test]
    #[cfg(debug_assertions)]
    fn dream_c1_private_debt_bypass_and_repeated_native_controls_fail_causally() {
        let root = tempfile::tempdir().unwrap();
        let control = Control {
            private_fixture: true,
            ..control()
        };
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
        let mut publication = staged(root.path(), "mutant");
        let mut first = budget
            .native_publish_impl(&mut publication, Some(sample(2000, 2500, 1)))
            .unwrap();
        assert!(first.effect.take_value().is_ok());
        assert_eq!(first.verdict, Ok(()));
        assert_eq!(
            control.cleanup.debt.charged_ns.load(Ordering::Acquire),
            22_500_000
        );
        control.omit_rest.store(true, Ordering::Release);
        // Existing authenticated private mutant removes the actual debt guard;
        // the independent observation must reject this admission causally.
        budget.unit(|| Ok(())).unwrap();
        assert!(
            control
                .cleanup
                .debt
                .admission_violations
                .load(Ordering::Acquire)
                > 0,
            "broken debt admission escaped the actual observer"
        );
        control.omit_rest.store(false, Ordering::Release);
        budget.pay_work_debt().unwrap();

        // Deliberately remove only original-publication guards in this private
        // test. The real no-overwrite syscall is attempted again and refuses;
        // counting that repeated call falsifies original-op at-most-once even
        // though the OS still protects the original inode from replacement.
        publication.attempted = false;
        control
            .cleanup
            .debt
            .native_publish_mask
            .store(0, Ordering::Release);
        let mut second = budget
            .native_publish_impl(&mut publication, Some(sample(1000, 1000, 1)))
            .unwrap();
        assert_eq!(second.effect.take_value(), Err(Code::StorageUnavailable));
        assert_eq!(
            control
                .cleanup
                .debt
                .native_publish_calls
                .load(Ordering::Acquire),
            2
        );
        assert!(
            control
                .cleanup
                .debt
                .native_publish_calls
                .load(Ordering::Acquire)
                > 1,
            "repeated-original-operation control did not violate its selected-call bound"
        );
        assert_eq!(
            control.cleanup.debt.native_effects.load(Ordering::Acquire),
            1
        );
        assert_exact_link(&publication);
    }

    #[test]
    fn dream_c1_birth_binding_probe_null_is_explicit_and_real_context_is_required() {
        let control = control();
        let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
        let id = public::Ulid("01ARZ3NDEKTSV4RRFFQ69G5FAV".into());
        let context = public::Context {
            account: public::Id("private-account".into()),
            account_epoch: public::Number(1),
            seat: id.clone(),
            generation: public::Number(1),
            grant_epoch: public::Number(1),
            swarm: id.clone(),
            swarm_revision: public::Number(1),
            workspace: public::Id("private-workspace".into()),
            workspace_revision: public::Number(1),
            root: public::Id("private-root".into()),
            root_revision: public::Number(1),
            provider: public::Id("private-provider".into()),
            provider_epoch: public::Number(1),
        };
        let digest = "0".repeat(64);
        let mut binding = BirthBinding {
            profile: bootstrap::PROFILE,
            purpose: "inspect-c1",
            probe: true,
            op_id: &id,
            run_id: &id,
            nonce: &id,
            incarnation: 1,
            context: None,
            worker_sha256: &digest,
        };
        // Input-shape regression only. Producer owns the canonical binding
        // vectors/128-byte permit; no alternate digest fixture is defined here.
        let serialized = serde_json::to_value(&binding).unwrap();
        assert!(serialized.as_object().unwrap().contains_key("context"));
        assert_eq!(serialized["context"], Value::Null);
        let probe = birth_binding(&binding, &mut budget).unwrap();
        binding.probe = false;
        assert_eq!(birth_binding(&binding, &mut budget), Err(Code::BadRequest));
        binding.context = Some(&context);
        let real = birth_binding(&binding, &mut budget).unwrap();
        assert_ne!(probe, real);
        binding.probe = true;
        assert_eq!(birth_binding(&binding, &mut budget), Err(Code::BadRequest));
        assert_eq!(
            control
                .cleanup
                .debt
                .native_process_calls
                .load(Ordering::Acquire),
            0
        );
    }

    #[test]
    fn dream_c1_final_public_control_is_once_after_debt_and_retains_component_child_on_refusal() {
        for fault in ["none", "tamper", "aba", "held", "clock", "cpu"] {
            let root = tempfile::tempdir().unwrap();
            let job = root.path().canonicalize().unwrap();
            let path = job.join("public.sqlite");
            let database = rusqlite::Connection::open(&path).unwrap();
            database.execute_batch("CREATE TABLE public_epoch(id INTEGER PRIMARY KEY,epoch INTEGER); INSERT INTO public_epoch VALUES(1,1);").unwrap();
            // This fixed public-scalar fixture exercises the production final
            // control helper, not Registry::Authority's full identity proof.
            // Full current-row/actor/API witnesses remain separate.
            drop(
                database
                    .prepare_cached("SELECT epoch FROM public_epoch WHERE id=1")
                    .unwrap(),
            );
            let database = Arc::new(std::sync::Mutex::new(database));
            let observed = database.clone();
            let control = Control {
                authority_guard: Some(Arc::new(move || {
                    let database = observed.try_lock().map_err(|_| Code::Busy)?;
                    let epoch: i64 = database
                        .prepare_cached("SELECT epoch FROM public_epoch WHERE id=1")
                        .and_then(|mut statement| statement.query_row([], |row| row.get(0)))
                        .map_err(|_| Code::StaleContext)?;
                    if epoch == 1 {
                        Ok(())
                    } else {
                        Err(Code::StaleContext)
                    }
                })),
                ..control()
            };
            let (_pipes, mut command) = prepared_component(&job);
            let mut budget = Budget::new(&control, Instant::now() + Duration::from_secs(5));
            let mut outcome = budget
                .native_process_impl(&mut command, Some(sample(1000, 1000, 1)))
                .unwrap();
            let mut child = ComponentChild(outcome.effect.take_value().unwrap());
            drop(command);
            assert_eq!(outcome.verdict, Ok(()));
            let pid = child.0.id();
            let start = transport::process_stamp(pid).unwrap();
            assert!(control.cleanup.debt.remaining_ns() > 0);
            budget.pay_work_debt().unwrap();
            assert_eq!(control.cleanup.debt.remaining_ns(), 0);
            let charged = control.cleanup.debt.charged_ns.load(Ordering::Acquire);
            if matches!(fault, "tamper" | "aba") {
                let path = path.clone();
                *control.final_admission_hook.lock().unwrap() = Some(Box::new(move |site| {
                    assert_eq!(site, FinalHandoff::Permit);
                    let writer = rusqlite::Connection::open(path).unwrap();
                    writer
                        .execute("UPDATE public_epoch SET epoch=2 WHERE id=1", [])
                        .unwrap();
                    if fault == "aba" {
                        // A display label may return, its public incarnation
                        // remains monotonically distinct rather than reset to1.
                        writer
                            .execute("UPDATE public_epoch SET epoch=3 WHERE id=1", [])
                            .unwrap();
                    }
                }));
            }
            if fault == "clock" {
                *control.final_admission_sample.lock().unwrap() = Some(Err(Code::ResourceLimit));
            } else if fault == "cpu" {
                *control.final_admission_sample.lock().unwrap() =
                    Some(Ok(Duration::from_millis(2)));
            }
            let held = (fault == "held").then(|| database.lock().unwrap());
            let effects = AtomicU64::new(0);
            let admission = final_admission_control(&control, FinalHandoff::Permit, || {
                effect_guard(&control)?;
                effects.fetch_add(1, Ordering::Release);
                Ok(())
            });
            drop(held);
            match fault {
                "none" => assert_eq!(admission, Ok(())),
                "tamper" | "aba" => assert_eq!(admission, Err(Code::StaleContext)),
                "held" => assert_eq!(admission, Err(Code::Busy)),
                "clock" | "cpu" => assert_eq!(admission, Err(Code::ResourceLimit)),
                _ => unreachable!(),
            }
            assert_eq!(control.bootstrap_permit_attempts.load(Ordering::Acquire), 1);
            assert_eq!(
                control.cleanup.debt.charged_ns.load(Ordering::Acquire),
                charged,
                "final control recursively created work debt"
            );
            assert_eq!(
                effects.load(Ordering::Acquire),
                u64::from(matches!(fault, "none" | "clock" | "cpu")),
                "post-effect timing refusal was mistaken for absent effect"
            );
            assert_eq!(
                final_admission_control(&control, FinalHandoff::Permit, || {
                    effects.fetch_add(1, Ordering::Release);
                    Ok(())
                }),
                Err(Code::ProtocolError)
            );
            assert_eq!(control.bootstrap_permit_attempts.load(Ordering::Acquire), 1);
            assert_eq!(transport::process_stamp(pid).unwrap(), start);
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "refusal forgot the actual child owner"
            );
            for _ in 0..200 {
                let at = Instant::now();
                let _ = control.state.load(Ordering::Acquire);
                control.cancelled.store(true, Ordering::Release);
                let _ = control.revision.load(Ordering::Acquire);
                assert!(at.elapsed() <= Duration::from_millis(100));
            }
            // These are independent cached-control/component custody checks,
            // not admitted MCP latency or a gated producer process witness.
            reap_component(&mut child, pid, &start);
        }
    }

    mod final_authority_controls {
        use super::*;
        use crate::store::Store;
        use std::os::unix::fs::OpenOptionsExt;

        /// Exact private inode replacement, restored before the real fixture
        /// shuts down. It never deletes a path outside its own account root.
        struct AccountSwap {
            named: PathBuf,
            retained: PathBuf,
        }
        impl AccountSwap {
            fn new(named: &Path) -> Self {
                let retained = named.with_extension("final-control-original");
                assert!(!retained.exists());
                std::fs::rename(named, &retained).unwrap();
                let swap = Self {
                    named: named.to_owned(),
                    retained,
                };
                let mut replacement = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(&swap.named)
                    .unwrap();
                replacement
                    .write_all(b"private replaced account inode\n")
                    .unwrap();
                replacement.sync_all().unwrap();
                swap
            }
        }
        impl Drop for AccountSwap {
            fn drop(&mut self) {
                std::fs::remove_file(&self.named).unwrap();
                std::fs::rename(&self.retained, &self.named).unwrap();
            }
        }

        fn check(fault: &'static str, site: FinalHandoff) {
            crate::logic::dream_public_api_tests::with_final_public_authority(
                |authority, account, fence| {
                    // The helper retains the actual Logic/Supervisor/PTYS and
                    // identity/native-custody spies. This is the production prepared
                    // read-only guard, not a substitute public-scalar callback.
                    let (context, captured, guard) = authority.test_prepared_guard().unwrap();
                    let control = Arc::new(control());
                    control.install_authority_guard(guard).unwrap();
                    let root = tempfile::tempdir().unwrap();
                    let job = root.path().canonicalize().unwrap();
                    let (_pipes, mut command) = prepared_component(&job);
                    let mut budget =
                        Budget::new(&control, Instant::now() + Duration::from_secs(10));
                    let mut native = budget
                        .native_process_impl(&mut command, Some(sample(1000, 1000, 1)))
                        .unwrap();
                    let mut child = ComponentChild(native.effect.take_value().unwrap());
                    drop(command);
                    assert_eq!(native.verdict, Ok(()));
                    let pid = child.0.id();
                    let start = transport::process_stamp(pid).unwrap();
                    assert!(control.cleanup.debt.remaining_ns() > 0);
                    budget.pay_work_debt().unwrap();
                    assert_eq!(control.cleanup.debt.remaining_ns(), 0);
                    let charged = control.cleanup.debt.charged_ns.load(Ordering::Acquire);
                    let effects = AtomicU64::new(0);
                    let swap = Arc::new(std::sync::Mutex::new(None));
                    let hook_swap = swap.clone();
                    let writer = account.clone();
                    let hook_fence = fence.clone();
                    let seat = context.seat.0.clone();
                    let weak = Arc::downgrade(&control);
                    *control.final_admission_hook.lock().unwrap() = Some(Box::new(
                        move |actual_site| {
                            assert_eq!(actual_site, site);
                            let control = weak.upgrade().unwrap();
                            assert_eq!(
                                control.cleanup.debt.remaining_ns(),
                                0,
                                "fresh authorization ran before the old work debt was paid"
                            );
                            match fault {
                                "row" => {
                                    let device = captured.device.unwrap();
                                    let key = format!("identity/device/{device}");
                                    let mut bytes = writer.get(&key).unwrap().unwrap();
                                    bytes.push(b' '); // valid JSON, changed exact public-row digest.
                                    writer.put(&key, &bytes).unwrap();
                                }
                                "head" => {
                                    let bytes = writer.get("identity/head").unwrap().unwrap();
                                    let mut head: crate::identity::Signed<
                                        crate::identity::AccountHead,
                                    > = serde_json::from_slice(&bytes).unwrap();
                                    head.payload
                                        .revoked_devices
                                        .push(head.payload.active_devices[0].device_id);
                                    writer
                                        .put("identity/head", &serde_json::to_vec(&head).unwrap())
                                        .unwrap();
                                }
                                "grant" => {
                                    assert_eq!(writer.connection().unwrap().execute(
                                "UPDATE msg_policies SET epoch=epoch+1 WHERE seat=?1", [&seat]).unwrap(), 1);
                                }
                                "aba" => {
                                    let original = writer.get("identity/head").unwrap().unwrap();
                                    writer
                                        .put("identity/head", b"private intermediate B")
                                        .unwrap();
                                    writer.put("identity/head", &original).unwrap();
                                }
                                "cell" => {
                                    writer.put("identity/head", &[b'x'; 16 * 1024 + 1]).unwrap()
                                }
                                "inode" => {
                                    *hook_swap.lock().unwrap() =
                                        Some(AccountSwap::new(&writer.path))
                                }
                                "expiry" => {
                                    hook_fence.test_public_clock(captured.expires_at.unwrap())
                                }
                                "none" | "held" | "clock" | "cpu" => {}
                                _ => panic!("unknown final authority fault"),
                            }
                        },
                    ));
                    let held = (fault == "held")
                        .then(|| authority.port.host.test_dream_hold_selection().unwrap());
                    if fault == "clock" {
                        *control.final_admission_sample.lock().unwrap() =
                            Some(Err(Code::ResourceLimit));
                    } else if fault == "cpu" {
                        *control.final_admission_sample.lock().unwrap() =
                            Some(Ok(Duration::from_millis(2)));
                    }
                    let before = Instant::now();
                    let verdict = final_admission_control(&control, site, || {
                        effect_guard(&control)?;
                        effects.fetch_add(1, Ordering::Release);
                        Ok(())
                    });
                    let elapsed = before.elapsed();
                    drop(held);
                    drop(swap.lock().unwrap().take());
                    assert!(
                        elapsed <= Duration::from_millis(250),
                        "actual final authority {fault} blocked control for {elapsed:?}"
                    );
                    match fault {
                        "none" => assert_eq!(verdict, Ok(())),
                        "clock" | "cpu" => assert_eq!(verdict, Err(Code::ResourceLimit)),
                        _ => assert_eq!(verdict, Err(Code::StaleContext)),
                    }
                    assert_eq!(
                        effects.load(Ordering::Acquire),
                        u64::from(matches!(fault, "none" | "clock" | "cpu"))
                    );
                    assert_eq!(
                        control.cleanup.debt.charged_ns.load(Ordering::Acquire),
                        charged,
                        "bounded final control must not recursively create work debt"
                    );
                    let attempts = match site {
                        FinalHandoff::Permit => &control.bootstrap_permit_attempts,
                        FinalHandoff::Start => &control.start_admission_attempts,
                    };
                    assert_eq!(attempts.load(Ordering::Acquire), 1);
                    assert_eq!(
                        final_admission_control(&control, site, || {
                            effects.fetch_add(1, Ordering::Release);
                            Ok(())
                        }),
                        Err(Code::ProtocolError)
                    );
                    assert_eq!(attempts.load(Ordering::Acquire), 1);
                    assert_eq!(transport::process_stamp(pid).unwrap(), start);
                    assert!(child.0.try_wait().unwrap().is_none());
                    for _ in 0..200 {
                        let at = Instant::now();
                        let _ = control.state.load(Ordering::Acquire);
                        control.cancelled.store(true, Ordering::Release);
                        let _ = control.revision.load(Ordering::Acquire);
                        assert!(at.elapsed() <= Duration::from_millis(100));
                    }
                    reap_component(&mut child, pid, &start);
                    eprintln!(
                        "dream_c1_final_actual_public site={site:?} fault={fault} checks=1 effects={} wall_us={} cpu_ns={} native_custody_delta=0 component_reaped=true",
                        effects.load(Ordering::Acquire),
                        elapsed.as_micros(),
                        control.bootstrap_control_max_cpu_ns.load(Ordering::Acquire)
                    );
                    // Child/pipes are real retained effects, but /bin/sleep is only
                    // a custody component. This is not gated producer/bootstrap,
                    // qualifying Hello/Start or admitted MCP latency evidence.
                },
            );
        }

        #[test]
        fn dream_c1_final_actual_authority_rechecks_row_head_grant_inode_aba_after_debt() {
            for site in [FinalHandoff::Permit, FinalHandoff::Start] {
                for fault in ["row", "head", "grant", "inode", "aba", "cell", "expiry"] {
                    check(fault, site);
                }
            }
        }

        #[test]
        fn dream_c1_final_actual_authority_held_clock_cpu_refusals_keep_owned_child() {
            for site in [FinalHandoff::Permit, FinalHandoff::Start] {
                for fault in ["none", "held", "clock", "cpu"] {
                    check(fault, site);
                }
            }
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    #[ignore = "requires canonically provisioned real worker; make test-dream-c1 executes"]
    fn dream_c1_private_signed_gated_worker_held_parent_paid_probe_and_refusals() {
        use std::cell::RefCell;
        use std::rc::Rc;

        // REQUIRED provision input: no PATH, unknown-worker discovery, runtime
        // build, old artifact fallback or missing-fixture green result. The
        // canonical target explicitly runs this ignored external fixture.
        let binary = PathBuf::from(
            std::env::var_os("DREAM_C1_RUNNER")
                .expect("gated component witness requires provisioned DREAM_C1_RUNNER"),
        );
        let proof = Arc::new(
            runner_provision::verify_native_provision(&binary, true)
                .expect("approved source/compiler/hash/bootstrap receipt must verify BEFORE birth"),
        );
        let private_development = proof.is_private_development();
        let artifact_class = if private_development {
            "signed_private_development"
        } else {
            "approved_production"
        };
        assert!(!proof.control_only_wrapper());
        assert_eq!(proof.capability_binary(), binary);
        let mut rows = Vec::new();
        for mode in ["paid_probe", "cancel", "expired"] {
            let root = tempfile::tempdir().unwrap();
            let parent = root.path().canonicalize().unwrap();
            let control = Arc::new(control());
            let deadline = Instant::now() + Duration::from_secs(30);
            let mut budget = Budget::new(&control, deadline);
            budget
                .unit(|| control.install_native_provision(proof.clone()))
                .unwrap();
            let (op, run, nonce, birth) = budget
                .unit(|| {
                    Ok((
                        crate::Ulid::new(),
                        crate::Ulid::new(),
                        crate::Ulid::new(),
                        crate::Ulid::new(),
                    ))
                })
                .unwrap();
            let job = parent.join(run.to_string());
            budget.unit(|| private_dir(&job)).unwrap();
            budget.unit(|| private_dir(&job.join("tmp"))).unwrap();
            let op = public::Ulid(op.to_string());
            let run = public::Ulid(run.to_string());
            let nonce = public::Ulid(nonce.to_string());
            let binding = birth_binding(
                &BirthBinding {
                    profile: bootstrap::PROFILE,
                    purpose: "inspect-c1",
                    probe: true,
                    op_id: &op,
                    run_id: &run,
                    nonce: &nonce,
                    incarnation: 1,
                    context: None,
                    worker_sha256: proof.worker_sha256(),
                },
                &mut budget,
            )
            .unwrap();
            let frozen = bootstrap::Frozen::inspect(
                true,
                birth.to_bytes(),
                bootstrap::ulid(run.0.as_bytes()).unwrap(),
                bootstrap::ulid(nonce.0.as_bytes()).unwrap(),
                binding,
                1,
            );
            let mut gate = Paced::new(
                budget
                    .unit(|| bootstrap::Gate::prepare(frozen).map_err(wire_error))
                    .unwrap(),
                &control,
            );
            let pipe_identity = gate.identity();
            let mut protected = control
                .heap
                .reserve(HeapLane::Control, transport::PROTECTED_BYTES)
                .unwrap();
            let mut buffers = Paced::new(
                budget
                    .unit(|| Worker::prepare_buffers().map_err(wire_error))
                    .unwrap(),
                &control,
            );
            protected.allocated();
            let mut input = Paced::new(
                budget
                    .unit(|| Worker::prepare_pipe().map_err(wire_error))
                    .unwrap(),
                &control,
            );
            let mut output = Paced::new(
                budget
                    .unit(|| Worker::prepare_pipe().map_err(wire_error))
                    .unwrap(),
                &control,
            );
            let mut stderr = Paced::new(
                budget
                    .unit(|| Worker::prepare_pipe().map_err(wire_error))
                    .unwrap(),
                &control,
            );
            let prepared = budget
                .unit(|| {
                    Worker::prepare_command(
                        proof.capability_binary(),
                        &job,
                        buffers.take_value(),
                        input.take_value(),
                        output.take_value(),
                        stderr.take_value(),
                        gate.take_value(),
                    )
                    .map_err(wire_error)
                })
                .unwrap();
            let prepared = Rc::new(RefCell::new(Some(Paced::new(prepared, &control))));
            {
                let mut held = prepared.borrow_mut();
                let prepared = held.as_mut().unwrap();
                budget
                    .unit(|| prepared.configure_input().map_err(wire_error))
                    .unwrap();
                budget
                    .unit(|| prepared.configure_output().map_err(wire_error))
                    .unwrap();
                budget
                    .unit(|| prepared.configure_stderr().map_err(wire_error))
                    .unwrap();
            }
            let command = {
                let mut held = prepared.borrow_mut();
                budget
                    .unit(|| held.as_mut().unwrap().take_command().map_err(wire_error))
                    .unwrap()
            };
            let mut command = Some(Paced::new(command, &control));
            budget.unit(|| proof.still_current()).unwrap();
            // Observe the actual source-bound child before its native verdict.
            // The hook never panics while it holds the sole Child effect: it
            // records observations, then attachment/reap precede assertions.
            let evidence = Rc::new(RefCell::new(None));
            let observed = evidence.clone();
            let hook_prepared = prepared.clone();
            let hook_control = control.clone();
            PROCESS_RETURN_HOOK.with(|hook| {
                *hook.borrow_mut() = Some(Box::new(move |child| {
                    let start = transport::process_stamp(child.id()).ok();
                    let held_work = hook_control.cleanup.debt.held.load(Ordering::Acquire);
                    let no_transfer = !hook_control
                        .cleanup
                        .debt
                        .child_bootstrap
                        .load(Ordering::Acquire);
                    let no_attempt = hook_control
                        .bootstrap_permit_attempts
                        .load(Ordering::Acquire)
                        == 0;
                    let until = Instant::now() + Duration::from_millis(72);
                    let waiting = loop {
                        let read = hook_prepared
                            .borrow_mut()
                            .as_mut()
                            .unwrap()
                            .poll_bootstrap_markers();
                        match read {
                            Ok(markers) if markers.waiting => break Ok(markers),
                            Err(code) => break Err(code),
                            _ if Instant::now() >= until => break Err(w::Code::Deadline),
                            _ => std::thread::sleep(REST),
                        }
                    };
                    // WNOWAIT observes only this exact owned child's exit
                    // state without collecting it or transferring reaping.
                    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
                    let live_at_waiting = unsafe {
                        libc::waitid(
                            libc::P_PID,
                            child.id(),
                            &mut info,
                            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                        ) == 0
                            && info.si_pid() == 0
                    };
                    let mut control_max = Duration::ZERO;
                    for _ in 0..16 {
                        let at = Instant::now();
                        let _ = hook_control.state.load(Ordering::Acquire);
                        let _ = hook_control.revision.load(Ordering::Acquire);
                        control_max = control_max.max(at.elapsed());
                    }
                    // This short returned-effect hold is not an expensive span,
                    // paid debt, a kernel-call hold, or a native timing exemption.
                    std::thread::sleep(Duration::from_millis(2));
                    if mode == "cancel" {
                        hook_control.cancelled.store(true, Ordering::Release);
                    }
                    if mode == "expired" {
                        std::thread::sleep(bootstrap::WINDOW + Duration::from_millis(1));
                    }
                    *observed.borrow_mut() = Some((
                        child.id(),
                        start,
                        held_work,
                        no_transfer,
                        no_attempt,
                        waiting,
                        live_at_waiting,
                        control_max,
                    ));
                }))
            });
            // ALWAYS actual samples. Hook control CPU remains inside the real
            // inclusive span: native<=2ms/25ms, app<=1ms, inclusive<=3ms.
            let mut native = budget.native_process(command.as_mut().unwrap()).unwrap();
            let sample = native.sample;
            let native_verdict = native.verdict;
            let returned_at = native.returned_at;
            let child = native.effect.take_value().unwrap();
            let (worker, unused) = prepared
                .borrow_mut()
                .take()
                .unwrap()
                .take_value()
                .attach_child(child);
            let mut worker = Paced::new(worker, &control);
            let mut unused = Some(Paced::new(unused, &control));
            let ready = returned_at
                .checked_add(bootstrap::WINDOW)
                .unwrap()
                .min(deadline);
            let setup = bootstrap_control(&control, || {
                worker
                    .bootstrap_spawn_returned(returned_at, deadline)
                    .map_err(wire_error)
            });
            let admission = (|| {
                native_verdict?;
                setup?;
                budget.deadline = ready;
                budget.pay_work_debt()?;
                drop(command.take());
                drop(unused.take());
                budget.pay_work_debt()?;
                budget.unit(|| proof.child_image_matches(worker.pid()))?;
                budget.pay_work_debt()?;
                bootstrap_handshake(&mut worker, &control, ready, true)
            })();
            // On ALL pre-grant failures close every duplicate stdio/Command
            // owner before EOF settlement, keeping protected capacity retained.
            drop(command.take());
            drop(unused.take());
            let original_deadline_preserved = worker
                .bootstrap_deadline()
                .is_ok_and(|deadline| deadline == ready);
            let after = bootstrap_control(&control, || {
                worker.poll_bootstrap_markers().map_err(wire_error)
            });
            let mut probe_start_refused = false;
            if mode == "paid_probe" && admission.is_ok() {
                // Parent permanent source-free guard only. Producer hostile
                // Start decoding is independently tested at its actual wire.
                let start = w::WorkerIn::Start(w::Start {
                    protocol: w::Protocol::V1,
                    run_id: w::Ulid(run.0.clone()),
                    nonce: w::Ulid(nonce.0.clone()),
                    manifest_sha256: w::Hex("0".repeat(64)),
                    timeout_ms: 30_000,
                    activity_revision: w::Number(0),
                    foreground: false,
                });
                probe_start_refused = worker.send(&start) == Err(w::Code::ProtocolError);
            }
            let observation = worker.bootstrap_observation();
            let same_pipe = worker.bootstrap_identity() == pipe_identity;
            settle_probe(&mut worker, &control);
            let reaped = worker.try_wait().unwrap().is_some() && worker.is_eof();
            let usage = worker.usage().unwrap();
            let evidence = evidence.borrow_mut().take().unwrap();
            let (
                pid,
                process_start,
                held_work,
                no_transfer,
                no_attempt,
                waiting,
                live_at_waiting,
                control_max,
            ) = evidence;
            let process_start =
                process_start.expect("actual gated child has exact kernel start stamp");
            let waiting =
                waiting.expect("actual signed worker must emit its bounded waiting marker");
            assert!(waiting.waiting && !waiting.paid_hello);
            assert_eq!(waiting.waiting_admissions, [0, 0, 0]);
            assert!(
                live_at_waiting,
                "waiting marker must come from the still-live exact child"
            );
            assert!(held_work && no_transfer && no_attempt);
            assert!(same_pipe && original_deadline_preserved);
            assert!(control_max <= Duration::from_millis(100));
            assert!(
                sample.cpu.unwrap() <= Duration::from_millis(2),
                "actual native sample: {sample:?}"
            );
            assert!(
                sample.wall <= Duration::from_millis(25),
                "actual native sample: {sample:?}"
            );
            assert!(
                sample.inclusive.unwrap() <= Duration::from_millis(3),
                "actual inclusive sample: {sample:?}"
            );
            assert!(
                control
                    .cleanup
                    .debt
                    .native_app_max_cpu_ns
                    .load(Ordering::Acquire)
                    <= 1_000_000
            );
            assert!(reaped && control.child_usage_observed.load(Ordering::Acquire));
            assert!(!transport::process_stamp(pid).is_ok_and(|current| current == process_start));
            let mut status = 0;
            assert_eq!(
                unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) },
                -1
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ECHILD)
            );
            assert!(
                !job.join("manifest.json").exists() && !job.join("output/result.json").exists()
            );
            let after = after.unwrap();
            if mode == "paid_probe" {
                assert_eq!(admission, Ok(()), "native={sample:?}");
                assert!(observation.granted && observation.qualified_hello);
                assert_eq!(observation.bytes_written, Some(128));
                assert!(after.paid_hello);
                assert_eq!(after.paid_admissions, [1, 1, 1, 1]);
                assert!((1..=4).contains(&after.permit_reads));
                assert!(after.control_max_cpu_ns <= 1_000_000);
                assert!(probe_start_refused);
                assert_eq!(control.bootstrap_permit_attempts.load(Ordering::Acquire), 1);
            } else {
                assert_eq!(
                    admission,
                    Err(if mode == "cancel" {
                        Code::Cancelled
                    } else {
                        Code::Deadline
                    })
                );
                assert!(!observation.granted && !observation.qualified_hello && !after.paid_hello);
                assert_eq!(control.bootstrap_permit_attempts.load(Ordering::Acquire), 0);
            }
            assert_eq!(control.start_admission_attempts.load(Ordering::Acquire), 0);
            rows.push(json!({"mode":mode,"run_id":run,"nonce":nonce,"birth":birth,"pid":pid,"process_start":process_start,
                "waiting_admissions":waiting.waiting_admissions,"live_at_waiting_wnowait":live_at_waiting,"paid_admissions":after.paid_admissions,
                "paid_hello":after.paid_hello,"permit_attempts":control.bootstrap_permit_attempts.load(Ordering::Acquire),
                "probe_start_parent_refused":probe_start_refused,"source_absent":true,"retained_real_effect_before_verdict":held_work,
                "actual_native_cpu_ns":nanos(sample.cpu.unwrap()),"actual_native_wall_ns":nanos(sample.wall),
                "actual_inclusive_cpu_ns":nanos(sample.inclusive.unwrap()),"actual_native_app_cpu_ns":control.cleanup.debt.native_app_max_cpu_ns.load(Ordering::Acquire),
                "same_origin_readiness_ms":250,"original_deadline_preserved":original_deadline_preserved,"injected_timing":false,
                "actual_reaped_and_both_eof":reaped,"waitpid_echild":true,"child_process_user_cpu_ns":usage.user_cpu_ns,"child_process_system_cpu_ns":usage.system_cpu_ns}));
            drop(worker);
            drop(protected);
        }
        if let Some(evidence) = std::env::var_os("DREAM_C1_EVIDENCE") {
            use std::os::unix::fs::OpenOptionsExt;
            let path = PathBuf::from(evidence).join("runtime-bootstrap-component.json");
            let receipt = json!({"schema":"bboxy.dream.c1.bootstrap-component/1",
                "driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").expect("driver evidence requires nonce"),
                "private_development_only":private_development,"artifact_class":artifact_class,
                "producer_native_conformance":false,"worker_sha256":proof.worker_sha256(),
                "bootstrap_profile":"dream-bootstrap/1","cases":rows});
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(path)
                .unwrap();
            file.write_all(&serde_json::to_vec(&receipt).unwrap())
                .unwrap();
            file.sync_all().unwrap();
        }
    }
}
