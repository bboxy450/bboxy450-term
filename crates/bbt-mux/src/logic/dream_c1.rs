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
use crate::registry::dream_c1::{Bookmark, Journal, Record, private_dir};
use bboxy450_harness_wake_up::dream_c1::{self as transport, Worker, wire as w};
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
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const CHUNK: usize = 64 * 1024;
// Hash work is separately bounded from64KiB filesystem transfer units. The
// measured64KiB hash+observer witness exceeded1ms;16KiB matches one wire read.
const HASH_CHUNK: usize = 16 * 1024;
const LINE: usize = 256 * 1024;
const INPUT: u64 = 64 * 1024 * 1024;
const DISK: u64 = 128 * 1024 * 1024;
pub(crate) const WORK_HEAP: u64 = 22 * 1024 * 1024;
pub(crate) const CONTROL_HEAP: u64 = 2 * 1024 * 1024;
pub(crate) const HELPER_HEADROOM: u64 = 8 * 1024 * 1024;
const RESULT: u64 = 16 * 1024 * 1024;
const UNIT: Duration = Duration::from_millis(1);
const REST: Duration = Duration::from_millis(9);

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
    fn truncate(&mut self, len: usize) -> Result<(), Code> {
        if len > self.len {
            return Err(Code::ProtocolError);
        }
        self.len = len;
        Ok(())
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
            // SAFETY: this exact pointer/layout pair was allocated once by new and is unique.
            unsafe {
                std::alloc::dealloc(
                    self.pointer.as_ptr(),
                    std::alloc::Layout::array::<u8>(self.capacity)
                        .expect("validated allocation layout"),
                );
            }
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
    pub setup_cpu_ns: u64,
}
pub(crate) struct Control {
    #[cfg(debug_assertions)]
    pub private_fixture: bool,
    #[cfg(debug_assertions)]
    pub omit_rest: AtomicBool,
    #[cfg(debug_assertions)]
    pub source_hold_done: AtomicBool,
    pub cancelled: AtomicBool,
    /// True also represents unknown foreground; no expensive unit is admitted then.
    pub foreground: Arc<AtomicBool>,
    pub activity_revision: Arc<AtomicU64>,
    /// Production authority: one atomic revision+foreground word; fixtures may omit it.
    pub activity_latch: Option<Arc<AtomicU64>>,
    pub authority_guard: Option<Arc<AuthorityGuard>>,
    pub authority_setup_cpu_ns: AtomicU64,
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
    pub unit_overrun_line: AtomicU64,
    pub child_peak_rss_bytes: AtomicU64,
    pub child_user_cpu_ns: AtomicU64,
    pub child_system_cpu_ns: AtomicU64,
    pub child_usage_observed: AtomicBool,
    pub producer_metrics_observed: AtomicBool,
    pub producer_metrics: [AtomicU64; 19],
    pub source_owner: OnceLock<std::thread::ThreadId>,
    pub owner_refusals: AtomicU64,
    pub snapshot_owner_thread_cpu_ns: AtomicU64,
    pub snapshot_owner_wall_ns: AtomicU64,
    pub snapshot_active_cpu_ns: AtomicU64,
    pub snapshot_active_wall_ns: AtomicU64,
    pub process_owner_thread_cpu_ns: AtomicU64,
    pub process_owner_wall_ns: AtomicU64,
    pub work_unit_count: AtomicU64,
    pub rest_wait_ns: AtomicU64,
    pub rest_wait_cpu_ns: AtomicU64,
    pub rest_skips: AtomicU64,
    pub rest_violations: AtomicU64,
    pub min_inter_unit_gap_ns: AtomicU64,
}
impl Default for Control {
    fn default() -> Self {
        Self {
            #[cfg(debug_assertions)]
            private_fixture: false,
            #[cfg(debug_assertions)]
            omit_rest: AtomicBool::new(false),
            #[cfg(debug_assertions)]
            source_hold_done: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            foreground: Arc::new(AtomicBool::new(true)),
            activity_revision: Arc::new(AtomicU64::new(0)),
            activity_latch: None,
            authority_guard: None,
            authority_setup_cpu_ns: AtomicU64::new(0),
            authority_prepared_guard: OnceLock::new(),
            authority_monitor_setup_cpu_ns: AtomicU64::new(0),
            authority_monitor_max_cpu_ns: AtomicU64::new(0),
            authority_monitor_overruns: AtomicU64::new(0),
            state: AtomicU8::new(DEFERRED),
            settled: AtomicBool::new(false),
            revision: AtomicU64::new(0),
            max_unit_cpu_ns: AtomicU64::new(0),
            unit_overruns: AtomicU64::new(0),
            heap: Arc::new(Heap::default()),
            unit_overrun_line: AtomicU64::new(0),
            child_peak_rss_bytes: AtomicU64::new(0),
            child_user_cpu_ns: AtomicU64::new(0),
            child_system_cpu_ns: AtomicU64::new(0),
            child_usage_observed: AtomicBool::new(false),
            producer_metrics_observed: AtomicBool::new(false),
            producer_metrics: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            source_owner: OnceLock::new(),
            owner_refusals: AtomicU64::new(0),
            snapshot_owner_thread_cpu_ns: AtomicU64::new(0),
            snapshot_owner_wall_ns: AtomicU64::new(0),
            snapshot_active_cpu_ns: AtomicU64::new(0),
            snapshot_active_wall_ns: AtomicU64::new(0),
            process_owner_thread_cpu_ns: AtomicU64::new(0),
            process_owner_wall_ns: AtomicU64::new(0),
            work_unit_count: AtomicU64::new(0),
            rest_wait_ns: AtomicU64::new(0),
            rest_wait_cpu_ns: AtomicU64::new(0),
            rest_skips: AtomicU64::new(0),
            rest_violations: AtomicU64::new(0),
            min_inter_unit_gap_ns: AtomicU64::new(u64::MAX),
        }
    }
}
impl Control {
    /// Installed only by the admitted authority owner after its durable/permit
    /// fence. Deferred controls retain no prepared database connection.
    pub fn install_authority_guard(&self, guard: PreparedGuard) -> Result<(), Code> {
        if guard.setup_cpu_ns > 1_000_000 {
            return Err(Code::ResourceLimit);
        }
        let mut installed = false;
        self.authority_prepared_guard.get_or_init(|| {
            installed = true;
            self.authority_setup_cpu_ns
                .store(guard.setup_cpu_ns, Ordering::Release);
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
            |latch| latch.load(Ordering::Acquire) >> 1,
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
        result
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
            let _payload = budget.json(&raw).map_err(|code| {
                if code == Code::ProtocolError {
                    Code::MalformedIr
                } else {
                    code
                }
            })?;
            let event: Value = budget.parse(&raw).map_err(|code| {
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

struct Budget<'a> {
    control: &'a Control,
    deadline: Instant,
    rest_until: Instant,
    previous_end: Option<Instant>,
    disk: u64,
    input: u64,
    envelopes: u64,
    declared: Reservation,
}
impl<'a> Budget<'a> {
    fn new(control: &'a Control, deadline: Instant) -> Self {
        Self {
            control,
            deadline,
            rest_until: Instant::now(),
            previous_end: None,
            disk: 0,
            input: 0,
            envelopes: 0,
            declared: control
                .heap
                .reserve(HeapLane::Work, 0)
                .expect("zero declaration within enforced quota"),
        }
    }
    fn gate(&self) -> Result<(), Code> {
        loop {
            if self.control.cancelled.load(Ordering::Acquire) {
                return Err(Code::Cancelled);
            }
            if Instant::now() >= self.deadline {
                return Err(Code::Deadline);
            }
            if !self.control.is_foreground() {
                return Ok(());
            }
            self.control.phase(PAUSED);
            std::thread::sleep(REST);
        }
    }
    fn begin_unit(&mut self) -> Result<Duration, Code> {
        self.gate()?;
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
        let cpu = bbt_rt::fault::thread_cpu_time().map_err(|_| Code::ResourceLimit)?;
        if let Some(guard) = self
            .control
            .authority_prepared_guard
            .get()
            .or(self.control.authority_guard.as_ref())
        {
            let result = guard();
            let used = bbt_rt::fault::thread_cpu_time()
                .map_err(|_| Code::ResourceLimit)?
                .saturating_sub(cpu);
            if result.is_err() || used > UNIT {
                return self.finish_unit(cpu, result.and(Err(Code::ResourceLimit)));
            }
        }
        Ok(cpu)
    }
    #[track_caller]
    fn finish_unit<T>(&mut self, cpu: Duration, result: Result<T, Code>) -> Result<T, Code> {
        let elapsed = bbt_rt::fault::thread_cpu_time()
            .map_err(|_| Code::ResourceLimit)?
            .saturating_sub(cpu);
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
            #[cfg(test)]
            eprintln!(
                "C1 unit overrun cpu_ns={} line={}",
                elapsed.as_nanos(),
                std::panic::Location::caller().line()
            );
            return Err(Code::ResourceLimit);
        }
        if Instant::now() >= self.deadline {
            return Err(Code::Deadline);
        }
        if self.control.cancelled.load(Ordering::Acquire) {
            return Err(Code::Cancelled);
        }
        result
    }
    #[track_caller]
    fn unit<T>(&mut self, work: impl FnOnce() -> Result<T, Code>) -> Result<T, Code> {
        let cpu = self.begin_unit()?;
        let result = work();
        self.finish_unit(cpu, result)
    }
    fn retain(&mut self, bytes: usize) -> Result<(), Code> {
        self.declared.grow(bytes)
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
    fn cooperate(&mut self, cpu: &mut Duration, checkpoints: &mut usize) -> Result<(), Code> {
        *checkpoints += 1;
        let used = bbt_rt::fault::thread_cpu_time()
            .map_err(|_| Code::ResourceLimit)?
            .saturating_sub(*cpu);
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
    fn parse<T: DeserializeOwned>(&mut self, bytes: &[u8]) -> Result<T, Code> {
        let cpu = self.begin_unit()?;
        struct Paced<'a, 'b> {
            bytes: &'a [u8],
            at: usize,
            next: usize,
            budget: &'a mut Budget<'b>,
            cpu: Duration,
            stopped: Option<Code>,
            checkpoints: usize,
        }
        impl Read for Paced<'_, '_> {
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
                        Ok(()) => {
                            self.next = self.at.saturating_add(4096);
                        }
                        Err(code) => {
                            self.stopped = Some(code);
                            return Err(std::io::ErrorKind::PermissionDenied.into());
                        }
                    }
                }
                // serde_json::IoRead uses std::io::Bytes and requests one byte.
                // Inspired by serde_json's adapter contract (independent credit):
                // https://github.com/serde-rs/json/blob/master/src/read.rs
                // A one-byte Read is valid for every nonempty output; avoid three
                // min operations and checked slice-copy setup on that hot path.
                out[0] = self.bytes[self.at];
                self.at += 1;
                Ok(1)
            }
        }
        let mut reader = Paced {
            bytes,
            at: 0,
            next: 4096,
            budget: self,
            cpu,
            stopped: None,
            checkpoints: 0,
        };
        let result = serde_json::from_reader(&mut reader).map_err(|_| Code::ProtocolError);
        if let Some(code) = reader.stopped {
            return Err(code);
        }
        reader.budget.finish_unit(reader.cpu, result)
    }
    fn write_json<T: Serialize>(&mut self, value: &T, buffer: &mut HeapBuffer) -> Result<(), Code> {
        let cpu = self.begin_unit()?;
        struct Paced<'a, 'b> {
            buffer: &'a mut HeapBuffer,
            budget: &'a mut Budget<'b>,
            cpu: Duration,
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
    let _payload = budget.json(&raw)?;
    let canonical: Value = budget.parse(&raw)?;
    let mut encoded = HeapBuffer::new(&budget.control.heap, HeapLane::Work, bound)?;
    budget.write_json(&canonical, &mut encoded)?;
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

/// Adapted from bboxy450-harness src/dream/inspect.rs::blobs, not arbitrary JSON recursion.
/// https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/inspect.rs
/// Keep typed carrier coverage and the producer's independent inventory validation attached.
fn references(
    event: &Value,
    out: &mut BTreeMap<String, w::Blob>,
    budget: &mut Budget<'_>,
) -> Result<(), Code> {
    fn add(value: &Value, out: &mut BTreeMap<String, w::Blob>) -> Result<(), Code> {
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
        let blob: w::Blob =
            serde::Deserialize::deserialize(value).map_err(|_| Code::BlobMismatch)?;
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
    fn blocks(
        value: &Value,
        out: &mut BTreeMap<String, w::Blob>,
        budget: &mut Budget<'_>,
    ) -> Result<(), Code> {
        if let Some(blocks) = value.as_array() {
            for block in blocks {
                budget.unit(|| {
                    if matches!(block["type"].as_str(), Some("image" | "audio" | "resource")) {
                        add(&block["blob"], out)
                    } else {
                        Ok(())
                    }
                })?;
            }
        }
        Ok(())
    }
    let body = match event["type"].as_str() {
        Some("item_completed") => &event["body"],
        Some("item_snapshot") => &event["partial"],
        _ => return Ok(()),
    };
    match body["kind"].as_str() {
        Some("user_message") => blocks(&body["blocks"], out, budget)?,
        Some("tool_call") => {
            budget.unit(|| add(&body["spill"], out))?;
            blocks(&body["output"], out, budget)?;
        }
        Some("terminal") => budget.unit(|| add(&body["output"], out))?,
        Some("image" | "native" | "receipt") => budget.unit(|| add(&body["blob"], out))?,
        Some("compaction") => budget.unit(|| add(&body["summary"], out))?,
        _ => {}
    }
    Ok(())
}

struct Snapshot {
    manifest: w::Manifest,
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
    let mut sources = Vec::new();
    sources
        .try_reserve_exact(record.sources.len())
        .map_err(|_| Code::ResourceLimit)?;
    let mut distinct = BTreeSet::new();
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
        let mut blobs = BTreeMap::new();
        let mut seq = 0u64;
        let mut closed = false;
        let mut last_hash = String::new();
        let mut read = 0u64;
        loop {
            let mut chunk = [0u8; CHUNK];
            let n = budget.unit(|| file.read(&mut chunk).map_err(|_| Code::SourceUnavailable))?;
            if n == 0 {
                break;
            }
            read = read.checked_add(n as u64).ok_or(Code::ResourceLimit)?;
            if read > bookmark.len {
                return Err(Code::SourceChanged);
            }
            budget.hash_update(&mut hash, &chunk[..n])?;
            budget.unit(|| {
                output
                    .write_all(&chunk[..n])
                    .map_err(|_| Code::StorageUnavailable)
            })?;
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
                let _payload = budget.json(&pending[..=end]).map_err(|code| {
                    if code == Code::ProtocolError {
                        Code::MalformedIr
                    } else {
                        code
                    }
                })?;
                let mut raw = HeapBuffer::new(&control.heap, HeapLane::Work, end + 1)?;
                raw.extend(&pending[..=end])?;
                pending.consume(end + 1)?;
                let event: Value = budget.parse(&raw).map_err(|code| {
                    if code == Code::ProtocolError {
                        Code::MalformedIr
                    } else {
                        code
                    }
                })?;
                budget.unit(|| {
                    seq += 1;
                    if event["session"].as_str() != Some(&bookmark.session.0)
                        || event["seq"].as_u64() != Some(seq)
                        || event["at"].as_u64() != Some(seq)
                    {
                        return Err(Code::MalformedIr);
                    }
                    if event["v"]["major"] != 1
                        || event["v"]["minor"].as_u64().is_none_or(|v| v > 3)
                    {
                        return Err(Code::UnsupportedIr);
                    }
                    if seq == 1 && !event["event"]["meta"]["fork"].is_null() {
                        return Err(Code::UnsupportedIr);
                    }
                    if seq == 1
                        && (event["event"]["type"] != "session_started"
                            || event["event"]["meta"]["vcs"]["repo"].as_str()
                                != Some(&bookmark.repo)
                            || !event["event"]["meta"]["source"].is_null())
                    {
                        return Err(Code::SourceMismatch);
                    }
                    closed = event["event"]["type"] == "session_closed";
                    Ok(())
                })?;
                references(&event["event"], &mut blobs, &mut budget)?;
                if closed {
                    last_hash = budget.hash(&raw[..raw.len() - 1])?;
                }
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
        let mut inventory = Vec::new();
        inventory
            .try_reserve_exact(blobs.len())
            .map_err(|_| Code::ResourceLimit)?;
        inventory.extend(blobs.into_values());
        if !inventory.is_empty() {
            let dir = input.join(&bookmark.session.0);
            budget.unit(|| private_dir(&dir))?;
            let dir = dir.join("blobs");
            budget.unit(|| private_dir(&dir))?;
            for blob in &inventory {
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
            blobs: inventory,
        });
    }
    let manifest = w::Manifest {
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
        sources,
    };
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
        _retained: budget.declared,
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
        let n = budget.unit(|| file.read(&mut chunk).map_err(|_| Code::BlobMismatch))?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > blob.len.0 {
            return Err(Code::BlobMismatch);
        }
        budget.hash_update(&mut hash, &chunk[..n])?;
        budget.unit(|| {
            out.write_all(&chunk[..n])
                .map_err(|_| Code::StorageUnavailable)
        })?;
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

struct Publication {
    temporary: PathBuf,
    target: PathBuf,
    file: File,
}
impl Publication {
    fn new(dir: &Path, name: &str, nonce: &str) -> Result<Self, Code> {
        let temporary = dir.join(format!(".{nonce}-{name}.tmp"));
        let file = create(&temporary)?;
        Ok(Self {
            temporary,
            target: dir.join(name),
            file,
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
    budget.unit(|| publication.link())?;
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
fn checkpoint(job: &Path, record: &Record) -> Result<(), Code> {
    Journal::open(
        job.parent().ok_or(Code::StorageUnavailable)?,
        &record.context.seat,
    )?
    .save(record)
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
fn decode_canonical<T: DeserializeOwned>(
    bytes: &[u8],
    budget: &mut Budget<'_>,
    require_next: bool,
) -> Result<T, Code> {
    let (_typed, bounds) = budget.json_shape(bytes, transport::JsonShape::Canonical)?;
    if require_next && !bounds.root_next {
        return Err(Code::ProtocolError);
    }
    budget.parse(bytes)
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
    let result: w::ResultDescriptor = decode_canonical(&bytes, &mut budget, false)?;
    let result_hash = budget.hash(&bytes)?;
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
        || result.max_poll_cpu_ns.0 > 1_000_000
    {
        return Err(Code::ProtocolError);
    }
    let mut total = bytes.len() as u64;
    let mut rows = 0u64;
    let mut episodes = 0u32;
    let mut clusters = 0u32;
    let mut previous_episode: Option<(String, u64, String)> = None;
    let mut previous_cluster: Option<(String, String)> = None;
    let mut seen_origins = BTreeSet::new();
    // Repo indices avoid duplicating a 4096-byte repository for each fingerprint.
    budget.retain(
        manifest.sources.len() * 2 * std::mem::size_of::<String>()
            + manifest
                .sources
                .iter()
                .map(|s| s.repo.0.len())
                .sum::<usize>(),
    )?;
    let repositories: Vec<String> = manifest
        .sources
        .iter()
        .map(|s| s.repo.0.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut expected: BTreeMap<(usize, String), (u32, u64, u64)> = BTreeMap::new();
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
        let page: w::Page = decode_canonical(&page_bytes, &mut budget, true)?;
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
    checkpoint(job, record)?;
    Ok(())
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
    if authority_guard
        .as_ref()
        .is_some_and(|g| g.setup_cpu_ns > 1_000_000)
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
    let control = Control {
        authority_setup_cpu_ns: AtomicU64::new(
            authority_guard.as_ref().map_or(0, |g| g.setup_cpu_ns),
        ),
        authority_guard: authority_guard.map(|g| g.callback),
        ..Control::default()
    };
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
    let result: w::ResultDescriptor = decode_canonical(&bytes, &mut budget, false)?;
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
    let page: w::Page = decode_canonical(&bytes, &mut budget, true)?;
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
    budget.unit(|| mirror(&page))
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

/// Retains process and snapshot ownership through real exit, including a blocked syscall.
pub(crate) fn execute(
    binary: &Path,
    job: &Path,
    record: &mut Record,
    control: &Control,
) -> Result<(), Code> {
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
    record.view.settled = true;
    record.view.state = if result.is_ok() {
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
    let saved = checkpoint(job, record);
    if saved.is_err() {
        record.view.state = State::Failed;
        record.view.error = Some(public::Error {
            code: Code::StorageUnavailable,
            source: None,
        });
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
    control.settled.store(true, Ordering::Release);
    result.and(saved)
}

/// Failed Hello/checkpoint still owns the exact child; do not let Drop hide an unsettled wait.
fn settle_probe(worker: &mut Worker, control: &Control) {
    control.phase(SETTLING);
    worker.close_input();
    let since = Instant::now();
    let mut term = false;
    let mut kill = false;
    loop {
        if worker.try_wait().is_ok_and(|exit| exit.is_some()) {
            retain_child_usage(worker, control);
            return;
        }
        if since.elapsed() >= Duration::from_secs(2) && !term {
            let _ = worker.terminate(false);
            term = true;
        }
        if since.elapsed() >= Duration::from_secs(4) && !kill {
            let _ = worker.terminate(true);
            kill = true;
        }
        if since.elapsed() >= Duration::from_secs(6) {
            control.phase(QUARANTINED);
        }
        std::thread::sleep(REST);
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
    if let Some(metrics) = worker.producer_metrics() {
        for (index, value) in metrics.values.iter().enumerate() {
            control.producer_metrics[index].store(value.unwrap_or(u64::MAX), Ordering::Release);
        }
        control
            .producer_metrics_observed
            .store(true, Ordering::Release);
    }
}

fn write_metrics(job: &Path, record: &Record, control: &Control) -> Result<(), Code> {
    use std::fmt::Write as _;
    let work = control.heap.counters(HeapLane::Work);
    let lane = control.heap.counters(HeapLane::Control);
    let mut bytes = HeapBuffer::new(&control.heap, HeapLane::Control, 4096)?;
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
            "process_owner_thread_cpu_ns",
            &control.process_owner_thread_cpu_ns,
        ),
        ("process_owner_wall_ns", &control.process_owner_wall_ns),
        ("work_unit_count", &control.work_unit_count),
        ("rest_wait_ns", &control.rest_wait_ns),
        ("rest_wait_cpu_ns", &control.rest_wait_cpu_ns),
        ("rest_skips", &control.rest_skips),
        ("rest_violations", &control.rest_violations),
        ("min_inter_unit_gap_ns", &control.min_inter_unit_gap_ns),
        ("source_owner_refusals", &control.owner_refusals),
        ("authority_setup_cpu_ns", &control.authority_setup_cpu_ns),
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
    ] {
        write!(
            &mut bytes,
            ",\"{}\":{}",
            name,
            value.load(Ordering::Acquire)
        )
        .map_err(|_| Code::ResourceLimit)?;
    }
    write!(
        &mut bytes,
        ",\"producer_metrics_observed\":{},\"producer_native_cumulative_measured\":false",
        control.producer_metrics_observed.load(Ordering::Acquire)
    )
    .map_err(|_| Code::ResourceLimit)?;
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
    bytes.extend(b"}")?;
    publish(job, "metrics.json", &record.nonce.0, &bytes)
}

#[derive(Clone, Copy)]
struct StartReceipt {
    seq: u64,
    phase: w::Phase,
}
struct FixedBytes<const N: usize> {
    bytes: [u8; N],
    len: usize,
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
fn write_start_receipt(job: &Path, record: &Record, receipt: StartReceipt) -> Result<(), Code> {
    use std::fmt::Write as _;
    let phase = match receipt.phase {
        w::Phase::Validate => "validate",
        w::Phase::Replay => "replay",
        w::Phase::Publish => "publish",
        w::Phase::Paused => "paused",
        w::Phase::Settling => "settling",
    };
    let mut bytes = FixedBytes {
        bytes: [0; 1024],
        len: 0,
    };
    write!(&mut bytes,"{{\"run_id\":\"{}\",\"nonce\":\"{}\",\"seq\":{},\"worker_pid\":{},\"worker_start\":\"{}\",\"phase\":\"{}\"}}",
        record.view.run_id.0,record.nonce.0,receipt.seq,record.worker_pid.ok_or(Code::ProtocolError)?,record.worker_start.as_deref().ok_or(Code::ProtocolError)?,phase).map_err(|_|Code::ResourceLimit)?;
    publish(
        job,
        "worker-started.json",
        &record.nonce.0,
        &bytes.bytes[..bytes.len],
    )
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
    control.phase(PREPARING);
    let deadline = Instant::now() + Duration::from_secs(30);
    // Manager admits only observed quiescence. A foreground race after that admission
    // pauses this active owner's bounded wall budget; it is not a deferred retry loop.
    let mut preparation = Budget::new(control, deadline);
    preparation.unit(|| private_dir(job))?;
    preparation.unit(|| private_dir(&job.join("tmp")))?;
    // Actual preallocated pending/output/stderr/reusable scratch requested capacities.
    let mut _protected = control
        .heap
        .reserve(HeapLane::Control, transport::PROTECTED_BYTES)?;
    let mut worker = Worker::spawn(binary, job).map_err(wire_error)?;
    _protected.allocated();
    if worker.protected_capacity_bytes() != transport::PROTECTED_BYTES {
        settle_probe(&mut worker, control);
        return Err(Code::ResourceLimit);
    }
    if let Err(error) = worker.hello() {
        settle_probe(&mut worker, control);
        return Err(wire_error(error));
    }
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
    record.worker_pid = Some(worker.pid());
    record.worker_start = match transport::process_stamp(worker.pid()) {
        Ok(stamp) => Some(stamp),
        Err(_) => {
            settle_probe(&mut worker, control);
            return Err(Code::WorkerLost);
        }
    };
    if let Err(code) = checkpoint(job, record) {
        settle_probe(&mut worker, control);
        return Err(code);
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
    let source_record = record.clone();
    let mut snapshot_result = None;
    let mut stop: Option<(Code, Instant)> = None;
    let mut started = false;
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
    let receipt_finished = AtomicBool::new(false);
    let receipt_failed = AtomicBool::new(false);
    std::thread::scope(|scope| -> Result<(), Code> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let (receipt_sender, receipt_receiver) = mpsc::sync_channel::<StartReceipt>(1);
        let mut receipt_sender = Some(receipt_sender);
        let finished = &receipt_finished;
        let failed = &receipt_failed;
        let mut receipt_sent = false;
        let mut receipt_settling = None;
        let snapshot_owner = scope.spawn(move || {
            let _owner_clock = match OwnerClock::new(
                &control.snapshot_owner_thread_cpu_ns,
                &control.snapshot_owner_wall_ns,
            ) {
                Ok(clock) => clock,
                Err(code) => {
                    let _ = sender.send(Err(code));
                    finished.store(true, Ordering::Release);
                    return;
                }
            };
            let value = snapshot(job, &source_record, control, deadline);
            let _ = sender.send(value);
            while let Ok(receipt) = receipt_receiver.recv() {
                if write_start_receipt(job, &source_record, receipt).is_err() {
                    failed.store(true, Ordering::Release);
                }
            }
            finished.store(true, Ordering::Release);
        });
        loop {
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
                && let Some(Ok(snapshot)) = &snapshot_result
            {
                record.view.manifest_sha256 = Some(public::Hex(snapshot.hash.clone()));
                if let Err(code) = checkpoint(job, record) {
                    stop = Some((code, Instant::now()));
                } else {
                    let revision = control.activity_version();
                    let remaining = deadline
                        .saturating_duration_since(Instant::now())
                        .as_millis()
                        .min(30_000) as u32;
                    if remaining == 0 {
                        stop = Some((Code::Deadline, Instant::now()));
                        continue;
                    }
                    let start = w::WorkerIn::Start(w::Start {
                        protocol: w::Protocol::V1,
                        run_id: w::Ulid(record.view.run_id.0.clone()),
                        nonce: w::Ulid(record.nonce.0.clone()),
                        manifest_sha256: w::Hex(snapshot.hash.clone()),
                        timeout_ms: remaining,
                        activity_revision: w::Number(revision),
                        foreground: control.is_foreground(),
                    });
                    match worker.send(&start) {
                        Ok(_) => {
                            started = true;
                            activity_sent = revision;
                            control.phase(RUNNING);
                        }
                        Err(error) => stop = Some((wire_error(error), Instant::now())),
                    }
                }
            }
            if let Some((_, since)) = stop {
                worker.discard_pending_output();
                control.cancelled.store(true, Ordering::Release);
                control.phase(CANCELLING);
                if !cancel_sent {
                    if started && worker.flush().is_ok_and(|ready| ready) {
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
                    match checked_poll(&mut worker, control) {
                        Ok(Some(frame)) => {
                            let receipt = if let w::WorkerOut::Progress(value) = &frame {
                                Some(StartReceipt {
                                    seq: value.seq.0,
                                    phase: value.phase,
                                })
                            } else {
                                None
                            };
                            if let Err(code) = frames.observe(frame, record, control)
                                && stop.is_none()
                            {
                                stop = Some((code, Instant::now()));
                            } else if stop.is_none()
                                && !receipt_sent
                                && let Some(receipt) = receipt
                            {
                                if receipt_sender
                                    .as_ref()
                                    .is_some_and(|sender| sender.try_send(receipt).is_ok())
                                {
                                    receipt_sent = true;
                                } else {
                                    stop = Some((Code::StorageUnavailable, Instant::now()));
                                }
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
            if receipt_failed.load(Ordering::Acquire) && stop.is_none() {
                stop = Some((Code::StorageUnavailable, Instant::now()));
            }
            if exit.is_some() && worker.is_eof() && snapshot_result.is_some() {
                receipt_sender.take();
                if receipt_finished.load(Ordering::Acquire) || snapshot_owner.is_finished() {
                    break;
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
            std::thread::sleep(REST);
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
        if control.producer_metrics_observed.load(Ordering::Acquire)
            && control.producer_metrics[1].load(Ordering::Acquire) > 96 * 1024 * 1024
        {
            return Err(Code::ResourceLimit);
        }
        let snapshot = snapshot_result
            .as_ref()
            .ok_or(Code::WorkerLost)?
            .as_ref()
            .map_err(|code| *code)?;
        validate_output(job, record, &snapshot.manifest, done, control, deadline)
    })
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
                    setup_cpu_ns: 0
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
        let parsed: Value = budget.parse(bytes.as_bytes()).unwrap();
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
        let decoded: w::ResultDescriptor = decode_canonical(&bytes, &mut budget, false).unwrap();
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
                setup_cpu_ns: 1_000_001
            }),
            Err(Code::ResourceLimit)
        );
        assert!(control.authority_prepared_guard.get().is_none());
        control
            .install_authority_guard(PreparedGuard {
                callback: Arc::new(|| Err(Code::StaleContext)),
                setup_cpu_ns: 123,
            })
            .unwrap();
        assert_eq!(
            control.install_authority_guard(PreparedGuard {
                callback: Arc::new(|| Ok(())),
                setup_cpu_ns: 456
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
        private_dir(&evidence.join("tmp")).unwrap();
        isolated_heap::ENABLED.store(true, Ordering::Release);
        let control = Control::default();
        control.foreground.store(false, Ordering::Release);
        let mut codec = control
            .heap
            .reserve(HeapLane::Control, transport::PROTECTED_BYTES)
            .unwrap();
        let mut worker = Worker::spawn(&binary, &evidence).unwrap();
        codec.allocated();
        worker.hello().unwrap();
        let pid = worker.pid();
        let process_start = transport::process_stamp(pid).unwrap();
        let held = HeapBuffer::new(&control.heap, HeapLane::Work, WORK_HEAP as usize).unwrap();
        let before = control.heap.counters(HeapLane::Work);
        assert!(matches!(
            HeapBuffer::new(&control.heap, HeapLane::Work, 1),
            Err(Code::ResourceLimit)
        ));
        assert_eq!(
            control.heap.counters(HeapLane::Work),
            before,
            "refusal precedes allocation/reallocation"
        );
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
        settle_probe(&mut worker, &control);
        let reap_ns = nanos(at.elapsed());
        assert!(worker.usage().is_some(), "actual wait4 reap required");
        sample_isolated_heap();
        drop(held);
        drop(worker);
        drop(codec);
        control.state.store(CANCELLED, Ordering::Release);
        control.settled.store(true, Ordering::Release);
        let live = isolated_heap::LIVE_PEAK.load(Ordering::Acquire);
        let touched = isolated_heap::TOUCHED_PEAK.load(Ordering::Acquire);
        let work = control.heap.counters(HeapLane::Work);
        let protected = control.heap.counters(HeapLane::Control);
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
        let parent_ceiling = 32 * 1024 * 1024 * instances as u64;
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
