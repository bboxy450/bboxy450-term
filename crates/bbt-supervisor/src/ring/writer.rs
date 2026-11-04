// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One disk writer for a supervisor. The PTY actor never waits for ring IO.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use mio::Waker;
use ulid::Ulid;

use super::{HEADER, Ring, Settings};
use crate::protocol::{Event, Failure, FailureCode, OUTPUT_CHUNK_BYTES, Reply};

#[cfg(test)]
#[path = "writer/crash_tests.rs"]
mod crash_tests;
#[cfg(test)]
#[path = "writer/probe.rs"]
mod probe;

const SEAT_BYTES: usize = 64 * 1024 * 1024;
const TOTAL_BYTES: usize = 128 * 1024 * 1024;
const MAX_JOBS: usize = 8192;
const TICK: Duration = Duration::from_millis(20);
const DURABLE_INTERVAL: Duration = Duration::from_secs(5);
const DURABLE_BYTES: u64 = 16 * 1024 * 1024;
const STORAGE_RETRY: Duration = Duration::from_millis(250);
type Key = (Ulid, u64);
pub(crate) type Response = mpsc::Receiver<Result<Reply, Failure>>;

#[derive(Default)]
struct Pressure {
    bytes: usize,
    peak: usize,
    full_count: u64,
    full: bool,
    full_since: Option<std::time::Instant>,
    full_byte_end: u64,
    unordered: VecDeque<(u64, std::time::Instant)>,
    last_output: Option<std::time::Instant>,
    #[cfg(test)]
    probe: Option<probe::Probe>,
}
struct Queue {
    jobs: VecDeque<Job>,
    pressure: HashMap<Key, Pressure>,
    bytes: usize,
    jobs_in_flight: usize,
    stop: bool,
    failures: HashMap<Key, Failure>,
    seat_capacity: usize,
    total_capacity: usize,
    job_capacity: usize,
    timings: Timings,
    undurable_bytes: usize,
    peak_undurable_bytes: usize,
    durable_paused: bool,
    storage_paused: bool,
    durable_pauses: u64,
    durable_rounds: u64,
    debt_since: Option<std::time::Instant>,
    durable_capacity: usize,
    #[cfg(test)]
    manual_ticks: bool,
}
struct Timings {
    busy_ns: u64,
    rounds: u64,
    max_ns: u64,
    histogram: [u64; 256],
}
impl Default for Timings {
    fn default() -> Self {
        Self {
            busy_ns: 0,
            rounds: 0,
            max_ns: 0,
            histogram: [0; 256],
        }
    }
}
impl Timings {
    fn round(&mut self, elapsed: Duration) {
        let ns = elapsed.as_nanos().min(u64::MAX as u128) as u64;
        self.rounds += 1;
        self.max_ns = self.max_ns.max(ns);
        let bucket = ns.div_ceil(1_000_000).saturating_sub(1).min(255) as usize;
        self.histogram[bucket] += 1;
    }
    fn p99_ns(&self) -> u64 {
        if self.rounds == 0 {
            return 0;
        }
        let rank = (self.rounds * 99).div_ceil(100);
        let mut seen = 0;
        for (i, &count) in self.histogram.iter().enumerate() {
            seen += count;
            if seen >= rank {
                return if i == 255 {
                    self.max_ns
                } else {
                    (i as u64 + 1) * 1_000_000
                };
            }
        }
        self.max_ns
    }
}
impl Default for Queue {
    fn default() -> Self {
        Self {
            jobs: VecDeque::new(),
            pressure: HashMap::new(),
            bytes: 0,
            jobs_in_flight: 0,
            stop: false,
            failures: HashMap::new(),
            seat_capacity: SEAT_BYTES,
            total_capacity: TOTAL_BYTES,
            job_capacity: MAX_JOBS,
            timings: Timings::default(),
            undurable_bytes: 0,
            peak_undurable_bytes: 0,
            durable_paused: false,
            storage_paused: false,
            durable_pauses: 0,
            durable_rounds: 0,
            debt_since: None,
            durable_capacity: DURABLE_BYTES as usize,
            #[cfg(test)]
            manual_ticks: false,
        }
    }
}
impl Queue {
    fn automatic_ticks(&self) -> bool {
        #[cfg(test)]
        if self.manual_ticks {
            return false;
        }
        true
    }
}
struct Shared {
    queue: Mutex<Queue>,
    ready: Condvar,
    wake: Arc<Waker>,
    #[cfg(test)]
    disk_full: Arc<std::sync::atomic::AtomicBool>,
}
enum Job {
    Register(Box<Ring>),
    Append(Key, Event, usize),
    Query(Key, Query, mpsc::SyncSender<Result<Reply, Failure>>, usize),
    Close(Key),
    #[cfg(test)]
    Gate(mpsc::SyncSender<()>, mpsc::Receiver<()>),
    #[cfg(test)]
    Tick(mpsc::SyncSender<()>),
}
pub(crate) enum Query {
    Recovery,
    History { from: u64, to: u64 },
    Checkpoint(Vec<u8>),
}
pub(crate) struct CaptureWriter {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}
pub(crate) struct Handle {
    shared: Arc<Shared>,
    key: Key,
    root: PathBuf,
    settings: Settings,
    first_sequence: u64,
    sequence: u64,
}
impl CaptureWriter {
    pub fn start(wake: Arc<Waker>, directory: Option<&Path>) -> io::Result<Self> {
        let journal = directory
            .map(super::journal::Journal::open)
            .transpose()
            .map_err(io::Error::other)?;
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            ready: Condvar::new(),
            wake,
            #[cfg(test)]
            disk_full: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        });
        #[cfg(test)]
        let journal = journal.map(|mut journal| {
            journal.disk_full = Some(shared.disk_full.clone());
            journal
        });
        let state = shared.clone();
        let thread = thread::Builder::new()
            .name("bbt-capture-writer".into())
            .spawn(move || run(state, journal))?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }
    pub fn register(&self, mut ring: Ring) -> Handle {
        // Group hot-file durability on the IO thread. The 20 ms deadline and
        // explicit control/checkpoint commits still bound a quiet tail.
        ring.group_bytes = 1024 * 1024;
        ring.grouped = true;
        let status = ring.status();
        let handle = Handle {
            shared: self.shared.clone(),
            key: (ring.boundary.seat, ring.boundary.generation),
            root: ring.directory().into(),
            settings: ring.settings().clone(),
            first_sequence: status.first_sequence,
            sequence: status.captured_sequence,
        };
        let mut q = self.shared.queue.lock().unwrap();
        q.pressure.insert(
            handle.key,
            Pressure {
                full_byte_end: status.byte_end,
                #[cfg(test)]
                probe: probe::Probe::open(&handle.root).expect("open owned crash-fixture probe"),
                ..Default::default()
            },
        );
        q.jobs.push_back(Job::Register(Box::new(ring)));
        drop(q);
        self.shared.ready.notify_one();
        handle
    }
}
impl Drop for CaptureWriter {
    fn drop(&mut self) {
        self.shared.queue.lock().unwrap().stop = true;
        self.shared.ready.notify_one();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Handle {
    pub fn directory(&self) -> &Path {
        &self.root
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn first_sequence(&self) -> u64 {
        self.first_sequence
    }
    pub fn capture_capacity(&self) -> usize {
        OUTPUT_CHUNK_BYTES.min(self.settings.capacity_bytes as usize - HEADER)
    }
    pub fn can_capture(&self) -> bool {
        let mut q = self.shared.queue.lock().unwrap();
        let cost = self.capture_capacity() + HEADER;
        let now = std::time::Instant::now();
        let durable_limit = q.undurable_bytes + self.capture_capacity() > q.durable_capacity
            || q.debt_since
                .is_some_and(|since| now.duration_since(since) >= DURABLE_INTERVAL);
        if durable_limit && !q.durable_paused {
            q.durable_paused = true;
            q.durable_pauses += 1;
            self.shared.ready.notify_one();
        }
        let tick_limit = q.pressure[&self.key]
            .unordered
            .front()
            .is_some_and(|(_, at)| now.duration_since(*at) >= TICK);
        let available =
            has_room(&q, self.key, cost) && !q.durable_paused && !q.storage_paused && !tick_limit;
        let pressure = q.pressure.get_mut(&self.key).unwrap();
        if !available && !pressure.full {
            pressure.full_count += 1;
            pressure.full_since = Some(now);
        }
        if available {
            pressure.full_since = None;
        }
        pressure.full = !available;
        available
    }
    pub fn can_control(&self) -> bool {
        let q = self.shared.queue.lock().unwrap();
        q.jobs.len().max(q.jobs_in_flight) + 2 <= q.job_capacity
            && has_room(&q, self.key, self.capture_capacity() + 2 * HEADER + 256)
    }
    pub fn append_event(&mut self, event: &Event) -> io::Result<()> {
        if event.seat() != self.key.0 || event.sequence() != Some(self.sequence + 1) {
            return Err(io::Error::other("queued ring cursor discontinuity"));
        }
        let cost = match event {
            Event::Output { bytes, .. } => bytes.len() + HEADER,
            _ => HEADER + 256,
        };
        let mut q = self.shared.queue.lock().unwrap();
        if let Some(error) = q.failures.get(&self.key)
            && (error.code != FailureCode::Busy || matches!(event, Event::Output { .. }))
        {
            return Err(io::Error::new(
                if error.code == FailureCode::Busy {
                    io::ErrorKind::WouldBlock
                } else {
                    io::ErrorKind::Other
                },
                error.to_string(),
            ));
        }
        if !has_room(&q, self.key, cost) {
            return Err(io::Error::other("capture queue reservation missing"));
        }
        if let Event::Output { bytes, .. } = event {
            let now = std::time::Instant::now();
            let tick_full = q.pressure[&self.key]
                .unordered
                .front()
                .is_some_and(|(_, at)| now.duration_since(*at) >= TICK);
            let debt_full = q.durable_paused
                || q.undurable_bytes + bytes.len() > q.durable_capacity
                || q.debt_since
                    .is_some_and(|at| now.duration_since(at) >= DURABLE_INTERVAL);
            if tick_full || debt_full {
                if debt_full && !q.durable_paused {
                    q.durable_paused = true;
                    q.durable_pauses += 1;
                }
                let pressure = q.pressure.get_mut(&self.key).unwrap();
                if !pressure.full {
                    pressure.full = true;
                    pressure.full_count += 1;
                    pressure.full_since = Some(now);
                }
                self.shared.ready.notify_one();
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "capture admission paused",
                ));
            }
        }
        charge(&mut q, self.key, cost);
        if let Event::Output { bytes, seq, .. } = event {
            let now = std::time::Instant::now();
            q.undurable_bytes += bytes.len();
            q.peak_undurable_bytes = q.peak_undurable_bytes.max(q.undurable_bytes);
            q.debt_since.get_or_insert(now);
            let pressure = q.pressure.get_mut(&self.key).unwrap();
            pressure.unordered.push_back((*seq, now));
            pressure.last_output = Some(now);
            #[cfg(test)]
            if let Some(probe) = &pressure.probe {
                probe.record(bytes.len());
            }
        }
        q.jobs.push_back(Job::Append(self.key, event.clone(), cost));
        self.sequence += 1;
        drop(q);
        self.shared.ready.notify_one();
        Ok(())
    }
    pub fn request(&self, query: Query) -> Result<Response, Failure> {
        let cost = match &query {
            Query::Checkpoint(bytes) => bytes.len(),
            _ => 0,
        };
        let (tx, rx) = mpsc::sync_channel(1);
        let mut q = self.shared.queue.lock().unwrap();
        if let Some(error) = q.failures.get(&self.key) {
            return Err(error.clone());
        }
        if !has_room(&q, self.key, cost) {
            return Err(Failure::new(FailureCode::Busy, "capture writer queue full"));
        }
        charge(&mut q, self.key, cost);
        q.jobs.push_back(Job::Query(self.key, query, tx, cost));
        drop(q);
        self.shared.ready.notify_one();
        Ok(rx)
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        self.shared
            .queue
            .lock()
            .unwrap()
            .jobs
            .push_back(Job::Close(self.key));
        self.shared.ready.notify_one();
    }
}
fn has_room(q: &Queue, key: Key, cost: usize) -> bool {
    q.jobs.len().max(q.jobs_in_flight) < q.job_capacity
        && q.bytes + cost <= q.total_capacity
        && q.pressure
            .get(&key)
            .is_some_and(|p| p.bytes + cost <= q.seat_capacity)
}
fn charge(q: &mut Queue, key: Key, cost: usize) {
    q.bytes += cost;
    q.jobs_in_flight += 1;
    let p = q.pressure.get_mut(&key).unwrap();
    p.bytes += cost;
    p.peak = p.peak.max(p.bytes);
}
fn release(shared: &Shared, key: Key, cost: usize) {
    let mut q = shared.queue.lock().unwrap();
    q.bytes -= cost;
    q.jobs_in_flight -= 1;
    q.pressure.get_mut(&key).unwrap().bytes -= cost;
    if q.jobs_in_flight == 0 && q.undurable_bytes + OUTPUT_CHUNK_BYTES <= q.durable_capacity {
        q.durable_paused = false;
    }
    // Commits and control replies wake the actor. Raw credit release must not
    // force an O(seats) poller pass for every append before its tick commits.
}
fn failure(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Failure {
    let error = error.into();
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(error.as_ref());
    let mut full = false;
    while let Some(value) = cause {
        full |= value
            .downcast_ref::<io::Error>()
            .is_some_and(|error| matches!(error.raw_os_error(), Some(libc::ENOSPC | libc::EDQUOT)));
        cause = value.source();
    }
    Failure::new(
        if full {
            FailureCode::Busy
        } else {
            FailureCode::Io
        },
        format!("capture writer failed: {error}"),
    )
}
fn pause_storage(shared: &Shared, key: Key, error: Failure) {
    let mut q = shared.queue.lock().unwrap();
    if !q.storage_paused {
        q.durable_pauses += 1;
    }
    q.storage_paused = true;
    q.durable_paused = true;
    q.failures.insert(key, error);
}
struct Tail {
    ring: Ring,
    records: VecDeque<super::journal::Record>,
    sequence: u64,
    byte_end: u64,
    committed_sequence: u64,
    committed_end: u64,
    bytes: usize,
    checkpoint: Option<(u64, Vec<u8>)>,
    rows: Vec<super::RowPoint>,
}
impl Tail {
    fn new(ring: Ring) -> super::Result<Self> {
        let status = ring.status();
        let mut rows = Vec::new();
        let checkpoint = ring
            .latest_checkpoint()?
            .map(|bytes| {
                super::Saved::decode(&bytes).map(|saved| {
                    rows = saved.index;
                    (saved.sequence, bytes)
                })
            })
            .transpose()?;
        Ok(Self {
            sequence: status.captured_sequence,
            byte_end: status.byte_end,
            committed_sequence: status.committed_sequence,
            committed_end: status.byte_end,
            ring,
            records: VecDeque::new(),
            bytes: 0,
            checkpoint,
            rows,
        })
    }
    fn status(&self) -> super::Status {
        let mut status = self.ring.status();
        status.captured_sequence = self.sequence;
        status.committed_sequence = self.committed_sequence;
        status.byte_end = self.byte_end;
        status.capture.records += self.records.len() as u64;
        status
    }
    fn append(
        &mut self,
        journal: &mut super::journal::Journal,
        event: &Event,
    ) -> super::Result<()> {
        if event.sequence() != Some(self.sequence + 1)
            || super::event_generation(event) != self.ring.boundary.generation
        {
            return Err(super::invalid("journal event discontinuity"));
        }
        let (kind, bytes) = super::encode_event(event)?;
        let record = journal.append(
            (self.ring.boundary.seat, self.ring.boundary.generation),
            kind,
            self.sequence + 1,
            self.byte_end,
            &bytes,
        )?;
        self.sequence += 1;
        if kind == 1 {
            self.byte_end += bytes.len() as u64;
        }
        self.bytes += bytes.len() + HEADER;
        self.records.push_back(record);
        Ok(())
    }
    fn checkpoint(
        &mut self,
        journal: &mut super::journal::Journal,
        bytes: Vec<u8>,
    ) -> super::Result<u64> {
        let saved = super::Saved::decode(&bytes)?;
        let expected_end = if saved.sequence <= self.ring.boundary.sequence {
            self.ring.byte_end_at(saved.sequence)?
        } else {
            let r = self
                .records
                .iter()
                .rev()
                .find(|r| r.kind != 4 && r.sequence == saved.sequence)
                .ok_or_else(|| super::invalid("checkpoint cursor not captured"))?;
            r.byte_start + if r.kind == 1 { r.length as u64 } else { 0 }
        };
        if saved.seat != self.ring.boundary.seat
            || saved.generation != self.ring.boundary.generation
            || saved.sequence > self.sequence
            || saved.byte_end != expected_end
        {
            return Err(super::invalid("checkpoint does not cover journal cursor"));
        }
        if let Some((seq, old)) = &self.checkpoint {
            if saved.sequence < *seq || (saved.sequence == *seq && bytes != *old) {
                return Err(super::invalid("stale/reused checkpoint cursor"));
            }
            if bytes == *old {
                return Ok(saved.sequence);
            }
        }
        // The parser can trail capture. Name the parsed frame, not the newer
        // publication tail. A compacted prefix uses a conservative earlier floor
        // (extra replay is safe; skipping unparsed records is not).
        let covered = self
            .records
            .iter()
            .rev()
            .find(|r| r.kind != 4 && r.sequence <= saved.sequence)
            .map(|r| r.position.clone())
            .unwrap_or_else(|| journal.floor());
        let record = journal.checkpoint(
            (saved.seat, saved.generation),
            saved.sequence,
            saved.byte_end,
            &bytes,
            &covered,
        )?;
        // Old checkpoints remain in the WAL for recovery, but only the newest
        // is projected. Counting superseded copies would seal mostly duplicates.
        self.records.retain(|r| {
            if r.kind == 4 {
                self.bytes -= r.length + HEADER;
                false
            } else {
                true
            }
        });
        self.bytes += record.length + HEADER;
        self.records.push_back(record);
        self.rows = saved.index;
        self.checkpoint = Some((saved.sequence, bytes));
        Ok(saved.sequence)
    }
    fn history(
        &self,
        journal: &mut super::journal::Journal,
        from: u64,
        to: u64,
    ) -> super::Result<super::History> {
        if from > to {
            return Err(super::invalid("invalid history range"));
        }
        let first = self.ring.first_base() + 1;
        let gap = (from < first || to > self.sequence).then(|| super::HistoryGap {
            requested_from: from,
            requested_to: to,
            retained_from: (first <= self.sequence).then_some(first),
            retained_to: (first <= self.sequence).then_some(self.sequence),
            reason: "requested range outside retained ring+journal history".into(),
        });
        let mut records = Vec::new();
        let mut next = None;
        if from <= self.ring.boundary.sequence {
            let page = self
                .ring
                .history(from, to.min(self.ring.boundary.sequence))?;
            records = page.records;
            next = page.next;
        }
        let mut size = records
            .iter()
            .map(super::page_weight)
            .sum::<super::Result<usize>>()?;
        if next.is_none() {
            for r in self
                .records
                .iter()
                .filter(|r| r.kind != 4 && r.sequence >= from && r.sequence <= to)
            {
                if !records.is_empty() && size >= super::PAGE_BYTES {
                    next = Some(r.sequence);
                    break;
                }
                let item = super::IndexedEvent {
                    event: super::decode_event(
                        r.kind,
                        r.key.0,
                        r.key.1,
                        r.sequence,
                        &journal.payload(r)?,
                    )?,
                    row: self
                        .rows
                        .binary_search_by_key(&r.sequence, |p| p.sequence)
                        .ok()
                        .map(|i| self.rows[i].clone()),
                };
                let bytes = super::page_weight(&item)?;
                if !records.is_empty() && size + bytes > super::PAGE_BYTES {
                    next = Some(r.sequence);
                    break;
                }
                records.push(item);
                size += bytes;
            }
        }
        Ok(super::History {
            status: self.status(),
            records,
            gap,
            next,
        })
    }
    /// Projection work is outside the hot tick. The journal has already been
    /// committed before this starts; a torn projection is simply rebuilt.
    fn compact(&mut self, journal: &mut super::journal::Journal, seal: bool) -> super::Result<()> {
        if self.records.is_empty() {
            return Ok(());
        }
        self.ring.grouped = true;
        let mut position = None;
        for r in &self.records {
            if r.kind != 4 && r.sequence > self.ring.boundary.sequence {
                self.ring.append_event(&super::decode_event(
                    r.kind,
                    r.key.0,
                    r.key.1,
                    r.sequence,
                    &journal.payload(r)?,
                )?)?;
            }
            position = Some(r.position.clone());
        }
        if let Some((_, bytes)) = &self.checkpoint {
            self.ring.checkpoint(bytes.clone())?;
        }
        self.ring.boundary.journal_position = position.clone();
        if seal {
            // Appending at capacity already seals the full data segment. Make
            // its remaining checkpoint/hot tail durable without sealing a
            // second tiny segment containing only the checkpoint.
            self.ring.flush()?;
            if let (Some(position), Some((seq, _))) = (position, &self.checkpoint) {
                journal.projected(
                    (self.ring.boundary.seat, self.ring.boundary.generation),
                    position,
                    *seq,
                )?;
            }
        } else {
            self.ring.commit(false)?;
        }
        self.records.clear();
        self.bytes = 0;
        Ok(())
    }
}

fn run(shared: Arc<Shared>, mut journal: Option<super::journal::Journal>) {
    let mut rings: HashMap<Key, Tail> = HashMap::new();
    let mut repair_projection: HashSet<Key> = HashSet::new();
    let mut retry_at = std::time::Instant::now();
    let mut next_flush = std::time::Instant::now() + TICK;
    loop {
        if shared.queue.lock().unwrap().storage_paused && std::time::Instant::now() >= retry_at {
            let result = (|| -> Result<(), Failure> {
                let journal = journal.as_mut().ok_or_else(|| failure("journal absent"))?;
                commit_group(&mut rings, journal, &shared, true)?;
                for key in &repair_projection {
                    if let Some(tail) = rings.get_mut(key) {
                        tail.ring = Ring::open(
                            &tail.ring.root,
                            key.0,
                            key.1,
                            tail.ring.first_base(),
                            tail.ring.settings().clone(),
                        )
                        .map_err(failure)?;
                        tail.compact(journal, true).map_err(failure)?;
                    }
                }
                Ok(())
            })();
            match result {
                Ok(()) => {
                    repair_projection.clear();
                    let mut q = shared.queue.lock().unwrap();
                    q.storage_paused = false;
                    q.failures
                        .retain(|_, error| error.code != FailureCode::Busy);
                    if q.undurable_bytes + OUTPUT_CHUNK_BYTES <= q.durable_capacity {
                        q.durable_paused = false;
                    }
                    drop(q);
                    let _ = shared.wake.wake();
                }
                Err(error) if error.code == FailureCode::Busy => {}
                Err(error) => {
                    let mut q = shared.queue.lock().unwrap();
                    for &key in rings.keys() {
                        q.failures.insert(key, error.clone());
                    }
                    q.storage_paused = false;
                }
            }
            retry_at = std::time::Instant::now() + STORAGE_RETRY;
        }
        let (job, automatic_ticks) = {
            let mut q = shared.queue.lock().unwrap();
            if q.storage_paused {
                if q.stop {
                    break;
                }
                // Output remains charged in the bounded queue. Control queries
                // get Busy promptly instead of waiting behind a failed append.
                if let Some(position) = q.jobs.iter().position(|job| matches!(job, Job::Query(..)))
                {
                    (q.jobs.remove(position), q.automatic_ticks())
                } else {
                    drop(
                        shared
                            .ready
                            .wait_timeout(
                                q,
                                retry_at.saturating_duration_since(std::time::Instant::now()),
                            )
                            .unwrap(),
                    );
                    continue;
                }
            } else {
                if q.jobs.is_empty() && !q.stop {
                    q = if q.automatic_ticks() {
                        shared
                            .ready
                            .wait_timeout(
                                q,
                                next_flush.saturating_duration_since(std::time::Instant::now()),
                            )
                            .unwrap()
                            .0
                    } else {
                        shared.ready.wait(q).unwrap()
                    };
                }
                if q.stop && q.jobs.is_empty() {
                    break;
                }
                let automatic_ticks = q.automatic_ticks();
                (q.jobs.pop_front(), automatic_ticks)
            }
        };
        let active_started = std::time::Instant::now();
        let active = job.is_some();
        #[cfg(test)]
        let mut tick_reply = None;
        match job {
            Some(Job::Register(ring)) => {
                let key = (ring.boundary.seat, ring.boundary.generation);
                let bound = journal
                    .as_ref()
                    .ok_or_else(|| super::invalid("JournalDirectoryRequired"))
                    .and_then(|j| j.bind(&ring));
                if let Err(error) = bound {
                    let error = failure(error);
                    if error.code == FailureCode::Busy {
                        pause_storage(&shared, key, error);
                        shared
                            .queue
                            .lock()
                            .unwrap()
                            .jobs
                            .push_front(Job::Register(ring));
                        retry_at = std::time::Instant::now() + STORAGE_RETRY;
                        continue;
                    }
                    shared.queue.lock().unwrap().failures.insert(key, error);
                    continue;
                }
                let result = Tail::new(*ring);
                match result {
                    Ok(tail) => {
                        rings.insert(key, tail);
                    }
                    Err(e) => {
                        shared
                            .queue
                            .lock()
                            .unwrap()
                            .failures
                            .insert(key, failure(e));
                    }
                }
            }
            Some(Job::Append(key, event, cost)) => {
                let result = rings
                    .get_mut(&key)
                    .ok_or_else(|| super::invalid("ring absent"))
                    .and_then(|tail| tail.append(journal.as_mut().unwrap(), &event));
                if let Err(e) = result {
                    let error = failure(e);
                    if error.code == FailureCode::Busy {
                        pause_storage(&shared, key, error);
                        shared
                            .queue
                            .lock()
                            .unwrap()
                            .jobs
                            .push_front(Job::Append(key, event, cost));
                        retry_at = std::time::Instant::now() + STORAGE_RETRY;
                        continue;
                    }
                    shared.queue.lock().unwrap().failures.insert(key, error);
                }
                release(&shared, key, cost);
                let seal = rings
                    .get(&key)
                    .is_some_and(|tail| tail.bytes as u64 >= tail.ring.settings().capacity_bytes);
                if seal {
                    let result = commit_group(&mut rings, journal.as_mut().unwrap(), &shared, true)
                        .and_then(|_| {
                            rings
                                .get_mut(&key)
                                .unwrap()
                                .compact(journal.as_mut().unwrap(), true)
                                .map_err(failure)
                        });
                    if let Err(e) = result {
                        if e.code == FailureCode::Busy {
                            repair_projection.insert(key);
                            pause_storage(&shared, key, e);
                            retry_at = std::time::Instant::now() + STORAGE_RETRY;
                        } else {
                            shared.queue.lock().unwrap().failures.insert(key, e);
                        }
                    }
                }
            }
            Some(Job::Query(key, query, reply, cost)) => {
                let failed = {
                    let q = shared.queue.lock().unwrap();
                    q.failures.get(&key).cloned().or_else(|| {
                        q.storage_paused.then(|| {
                            Failure::new(
                                FailureCode::Busy,
                                "capture persistence paused: storage full",
                            )
                        })
                    })
                };
                let checkpoint = matches!(query, Query::Checkpoint(_));
                let mut result = if let Some(e) = failed {
                    Err(e)
                } else {
                    rings
                        .get_mut(&key)
                        .ok_or_else(|| failure("ring absent"))
                        .and_then(|tail| {
                            let journal = journal.as_mut().unwrap();
                            match query {
                                Query::Recovery => Ok(Reply::RingRecovery {
                                    status: Box::new(tail.status()),
                                    checkpoint: tail.checkpoint.as_ref().map(|(_, b)| b.clone()),
                                }),
                                Query::History { from, to } => tail
                                    .history(journal, from, to)
                                    .map(|history| Reply::RingHistory {
                                        history: Box::new(history),
                                    })
                                    .map_err(failure),
                                Query::Checkpoint(bytes) => tail
                                    .checkpoint(journal, bytes)
                                    .map(|sequence| Reply::CheckpointCommitted { sequence })
                                    .map_err(failure),
                            }
                        })
                };
                if result.is_ok()
                    && checkpoint
                    && let Err(e) =
                        commit_group(&mut rings, journal.as_mut().unwrap(), &shared, true)
                {
                    result = Err(e);
                }
                if let Err(error) = &result
                    && error.code == FailureCode::Busy
                    && checkpoint
                {
                    pause_storage(&shared, key, error.clone());
                    retry_at = std::time::Instant::now() + STORAGE_RETRY;
                }
                release(&shared, key, cost);
                let result = result.map(|mut result| {
                    let status = match &mut result {
                        Reply::RingRecovery { status, .. } => Some(status.as_mut()),
                        Reply::RingHistory { history } => Some(&mut history.status),
                        _ => None,
                    };
                    if let Some(status) = status {
                        let q = shared.queue.lock().unwrap();
                        let p = &q.pressure[&key];
                        status.capture.queued_bytes = p.bytes;
                        status.capture.peak_queue_bytes = p.peak;
                        status.capture.queue_full_count = p.full_count;
                        status.capture.writer_busy_ns = q.timings.busy_ns;
                        status.capture.commit_rounds = q.timings.rounds;
                        status.capture.commit_p99_ns = q.timings.p99_ns();
                        status.capture.commit_max_ns = q.timings.max_ns;
                        status.capture.undurable_bytes = q.undurable_bytes;
                        status.capture.peak_undurable_bytes = q.peak_undurable_bytes;
                        status.capture.durable_pauses = q.durable_pauses;
                        status.capture.durable_rounds = q.durable_rounds;
                        status.paused = p
                            .full_since
                            .is_some_and(|at| at.elapsed() >= Duration::from_secs(1));
                    }
                    result
                });
                let _ = reply.try_send(result);
                let _ = shared.wake.wake();
            }
            Some(Job::Close(key)) => {
                if let Some(journal) = journal.as_mut() {
                    let _ = commit_group(&mut rings, journal, &shared, true);
                }
                rings.remove(&key);
                let mut q = shared.queue.lock().unwrap();
                q.pressure.remove(&key);
                q.failures.remove(&key);
            }
            #[cfg(test)]
            Some(Job::Gate(started, resume)) => {
                let _ = started.send(());
                let _ = resume.recv();
            }
            #[cfg(test)]
            Some(Job::Tick(reply)) => {
                next_flush = std::time::Instant::now();
                tick_reply = Some(reply);
            }
            None => {}
        }
        if active {
            shared.queue.lock().unwrap().timings.busy_ns +=
                active_started.elapsed().as_nanos() as u64;
        }
        let force = automatic_ticks && {
            let q = shared.queue.lock().unwrap();
            q.jobs.is_empty()
                && q.jobs_in_flight == 0
                && (q.durable_paused
                    || journal.as_ref().is_some_and(|j| j.durable_due())
                    || q.debt_since
                        .is_some_and(|at| at.elapsed() >= DURABLE_INTERVAL))
        };
        #[cfg(test)]
        let tick_requested = tick_reply.is_some();
        #[cfg(not(test))]
        let tick_requested = false;
        if !shared.queue.lock().unwrap().storage_paused
            && (tick_requested
                || automatic_ticks && (std::time::Instant::now() >= next_flush || force))
        {
            let started = std::time::Instant::now();
            if let Some(journal) = journal.as_mut() {
                if let Err(e) = commit_group(&mut rings, journal, &shared, force) {
                    for &key in rings.keys() {
                        if e.code == FailureCode::Busy {
                            pause_storage(&shared, key, e.clone());
                        } else {
                            shared.queue.lock().unwrap().failures.insert(key, e.clone());
                        }
                    }
                    retry_at = std::time::Instant::now() + STORAGE_RETRY;
                }
                // Never compact trickling seats once per tick. Wait for actual
                // idle, or their bounded journal tail reaching the seal size.
                let idle = {
                    let q = shared.queue.lock().unwrap();
                    let globally_idle = q.jobs.is_empty()
                        && !q.durable_paused
                        && q.pressure.values().all(|p| {
                            !p.full
                                && p.unordered.is_empty()
                                && p.last_output
                                    .is_none_or(|at| at.elapsed() >= Duration::from_millis(200))
                        });
                    if !automatic_ticks || !globally_idle {
                        None
                    } else {
                        rings.iter().find_map(|(&key, tail)| {
                            let p = &q.pressure[&key];
                            (!tail.records.is_empty()
                                && !p.full
                                && p.unordered.is_empty()
                                && p.last_output
                                    .is_some_and(|at| at.elapsed() >= Duration::from_millis(200)))
                            .then_some(key)
                        })
                    }
                };
                // One projection at a time, then service requests/admissions.
                // A stalled writer is not evidence that a seat became idle.
                if let Some(key) = idle
                    && let Err(e) = rings.get_mut(&key).unwrap().compact(journal, false)
                {
                    let error = failure(e);
                    if error.code == FailureCode::Busy {
                        repair_projection.insert(key);
                        pause_storage(&shared, key, error);
                        retry_at = std::time::Instant::now() + STORAGE_RETRY;
                    } else {
                        shared.queue.lock().unwrap().failures.insert(key, error);
                    }
                }
            }
            shared.queue.lock().unwrap().timings.busy_ns += started.elapsed().as_nanos() as u64;
            next_flush = std::time::Instant::now() + TICK;
        }
        #[cfg(test)]
        if let Some(reply) = tick_reply {
            let _ = reply.send(());
        }
    }
    if let Some(journal) = journal.as_mut() {
        let _ = commit_group(&mut rings, journal, &shared, true);
        // A durable WAL is sufficient at shutdown. Derived rings may lag;
        // forcing all projections here would turn stop into O(seat data) IO.
    }
}
fn commit_group(
    rings: &mut HashMap<Key, Tail>,
    journal: &mut super::journal::Journal,
    shared: &Shared,
    strong: bool,
) -> Result<(), Failure> {
    let started = std::time::Instant::now();
    if !journal.commit(strong).map_err(failure)? {
        return Ok(());
    }
    let mut q = shared.queue.lock().unwrap();
    for (&key, tail) in rings.iter_mut() {
        if tail.committed_sequence != tail.sequence {
            tail.ring.stats.ordered_commits += 1;
        }
        tail.committed_sequence = tail.sequence;
        tail.committed_end = tail.byte_end;
        let Some(p) = q.pressure.get_mut(&key) else {
            continue;
        };
        while p
            .unordered
            .front()
            .is_some_and(|(seq, _)| *seq <= tail.sequence)
        {
            p.unordered.pop_front();
        }
        if strong {
            let refund = tail.byte_end.saturating_sub(p.full_byte_end) as usize;
            p.full_byte_end = tail.byte_end;
            q.undurable_bytes = q.undurable_bytes.saturating_sub(refund);
        }
    }
    if strong {
        q.durable_rounds += 1;
        q.debt_since = q
            .pressure
            .values()
            .filter_map(|p| p.unordered.front().map(|(_, at)| *at))
            .min();
        if q.jobs_in_flight == 0 && q.undurable_bytes + OUTPUT_CHUNK_BYTES <= q.durable_capacity {
            q.durable_paused = false;
        }
    }
    q.timings.round(started.elapsed());
    drop(q);
    let _ = shared.wake.wake();
    if journal.rotate_due() {
        journal.rotate().map_err(failure)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::disallowed_methods)] // Private deterministic ENOSPC injection.
    fn disk_full_pauses_capture_without_poisoning_the_writer_and_retries() {
        use std::sync::atomic::Ordering;
        let temp = tempfile::tempdir().unwrap();
        let poll = mio::Poll::new().unwrap();
        let wake = Arc::new(Waker::new(poll.registry(), mio::Token(0)).unwrap());
        let writer = CaptureWriter::start(wake, Some(&temp.path().join("journal"))).unwrap();
        let seat = Ulid::new();
        let root = temp.path().join("ring");
        let mut handle =
            writer.register(Ring::open(&root, seat, 1, 0, Settings::default()).unwrap());
        handle
            .request(Query::Recovery)
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        writer.shared.disk_full.store(true, Ordering::Release);
        handle
            .append_event(&Event::Output {
                seat,
                generation: 1,
                seq: 1,
                bytes: b"retained across disk full".to_vec(),
            })
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !writer.shared.queue.lock().unwrap().storage_paused {
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        assert!(!writer.thread.as_ref().unwrap().is_finished());
        assert!(!handle.can_capture());
        assert!(handle.can_control());
        assert_eq!(
            handle.request(Query::Recovery).err().unwrap().code,
            FailureCode::Busy
        );
        assert_eq!(
            handle
                .append_event(&Event::Output {
                    seat,
                    generation: 1,
                    seq: 2,
                    bytes: b"later".to_vec()
                })
                .unwrap_err()
                .kind(),
            io::ErrorKind::WouldBlock
        );
        assert!(writer.shared.queue.lock().unwrap().undurable_bytes <= DURABLE_BYTES as usize);
        // Exit/resize records still have bounded control credit. Rejecting an
        // exit with WouldBlock would escape Seat::tick and kill the owner.
        handle
            .append_event(&Event::Exited {
                seat,
                generation: 1,
                seq: 2,
                status: crate::protocol::ExitInfo {
                    code: 0,
                    signal: None,
                },
            })
            .unwrap();
        writer.shared.disk_full.store(false, Ordering::Release);
        writer.shared.ready.notify_one();
        loop {
            if let Ok(response) = handle.request(Query::Recovery)
                && let Ok(Ok(Reply::RingRecovery { status, .. })) =
                    response.recv_timeout(Duration::from_secs(1))
                && status.captured_sequence == 2
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "writer did not recover after space returned"
            );
            thread::sleep(Duration::from_millis(5));
        }
        drop(handle);
        drop(writer);
        let restored = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        let history = restored.history(1, 1).unwrap();
        assert!(
            matches!(&history.records[0].event, Event::Output { bytes, .. } if bytes == b"retained across disk full")
        );
    }

    #[test]
    #[allow(clippy::disallowed_methods)] // A partial buffered append must retry at the same sequence.
    fn disk_full_append_retry_keeps_the_sequence_and_crc_prefix() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let temp = tempfile::tempdir().unwrap();
        let seat = Ulid::new();
        let root = temp.path().join("ring");
        let mut journal =
            super::super::journal::Journal::open(&temp.path().join("journal")).unwrap();
        let ring = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        journal.bind(&ring).unwrap();
        let mut tail = Tail::new(ring).unwrap();
        let full = Arc::new(AtomicBool::new(true));
        journal.disk_full = Some(full.clone());
        let mut failed = false;
        for seq in 1..=160 {
            let event = Event::Output {
                seat,
                generation: 1,
                seq,
                bytes: vec![seq as u8; OUTPUT_CHUNK_BYTES],
            };
            if let Err(error) = tail.append(&mut journal, &event) {
                assert_eq!(failure(error).code, FailureCode::Busy);
                assert_eq!(tail.sequence, seq - 1);
                full.store(false, Ordering::Release);
                tail.append(&mut journal, &event).unwrap();
                failed = true;
            }
        }
        assert!(failed);
        journal.commit(true).unwrap();
        drop(tail);
        drop(journal);
        let restored = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        assert_eq!(restored.status().captured_sequence, 160);
        let mut from = 1;
        while from <= 160 {
            let page = restored.history(from, 160).unwrap();
            for record in &page.records {
                assert!(
                    matches!(&record.event,Event::Output {seq,bytes,..} if *seq==from && bytes.iter().all(|byte| *byte==from as u8))
                );
                from += 1;
            }
        }
    }

    #[test]
    #[allow(clippy::disallowed_methods)] // Reopen the durable prefix after an interrupted projection.
    fn interrupted_projection_reopens_before_retrying_the_journal_tail() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("ring");
        let seat = Ulid::new();
        let mut journal =
            super::super::journal::Journal::open(&temp.path().join("journal")).unwrap();
        let ring = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        journal.bind(&ring).unwrap();
        let mut tail = Tail::new(ring).unwrap();
        for seq in 1..=80 {
            tail.append(
                &mut journal,
                &Event::Output {
                    seat,
                    generation: 1,
                    seq,
                    bytes: vec![seq as u8; 1024],
                },
            )
            .unwrap();
        }
        journal.commit(true).unwrap();
        tail.ring.grouped = true;
        for record in tail.records.iter().take(20) {
            tail.ring
                .append_event(
                    &super::super::decode_event(
                        record.kind,
                        seat,
                        1,
                        record.sequence,
                        &journal.payload(record).unwrap(),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        tail.ring.flush().unwrap();
        for record in tail.records.iter().skip(20).take(20) {
            tail.ring
                .append_event(
                    &super::super::decode_event(
                        record.kind,
                        seat,
                        1,
                        record.sequence,
                        &journal.payload(record).unwrap(),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        tail.ring = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        tail.compact(&mut journal, true).unwrap();
        drop(tail);
        drop(journal);
        let restored = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        assert_eq!(restored.status().captured_sequence, 80);
        let mut from = 1;
        while from <= 80 {
            let history = restored.history(from, 80).unwrap();
            assert!(!history.records.is_empty());
            for record in history.records {
                assert_eq!(record.event.sequence(), Some(from));
                from += 1;
            }
        }
    }

    struct OpenGate(Option<mpsc::SyncSender<()>>);
    impl Drop for OpenGate {
        fn drop(&mut self) {
            if let Some(resume) = self.0.take() {
                let _ = resume.send(());
            }
        }
    }
    fn tick(writer: &CaptureWriter) {
        let (done, completed) = mpsc::sync_channel(1);
        writer
            .shared
            .queue
            .lock()
            .unwrap()
            .jobs
            .push_back(Job::Tick(done));
        writer.shared.ready.notify_one();
        completed.recv_timeout(Duration::from_secs(30)).unwrap();
    }

    // keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
    #[test]
    #[allow(clippy::disallowed_methods)] // Owned ring and journal paging fixture.
    fn ring_and_journal_paging_is_contiguous_wire_bounded_and_does_not_decode_past_full() {
        let root = tempfile::tempdir().unwrap();
        let seat = Ulid::new();
        let mut ring =
            Ring::open(&root.path().join("seat"), seat, 1, 0, Settings::default()).unwrap();
        let events: Vec<_> = (1..=6)
            .map(|seq| Event::Output {
                seat,
                generation: 1,
                seq,
                bytes: vec![255; OUTPUT_CHUNK_BYTES],
            })
            .collect();
        for event in &events[..3] {
            ring.append_event(event).unwrap();
        }
        ring.flush().unwrap();
        let mut tail = Tail::new(ring).unwrap();
        let mut journal =
            super::super::journal::Journal::open(&root.path().join("journal")).unwrap();
        for event in &events[3..] {
            tail.append(&mut journal, event).unwrap();
        }
        journal.commit(false).unwrap();
        tail.committed_sequence = tail.sequence;
        tail.committed_end = tail.byte_end;
        let mut from = 1;
        let mut actual = Vec::new();
        while from <= 6 {
            let page = tail.history(&mut journal, from, 6).unwrap();
            assert!(page.gap.is_none());
            assert!(!page.records.is_empty());
            assert_eq!(page.records[0].event.sequence(), Some(from));
            super::super::paging_tests::wire_page(&page);
            actual.extend(page.records.iter().map(|r| r.event.clone()));
            from = page.next.unwrap_or(7);
        }
        assert_eq!(actual, events);
        // Corrupt only the unrequested index entry. Its own page must still
        // reject it; the preceding full page must not try to decode it.
        tail.records[1].kind = u16::MAX;
        let page = tail.history(&mut journal, 4, 6).unwrap();
        assert_eq!(page.next, Some(5));
        assert_eq!(page.records.len(), 1);
        assert!(tail.history(&mut journal, 5, 5).is_err());
    }

    // keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
    #[test]
    fn undurable_admission_budget_pauses_until_a_full_flush_returns_credit() {
        let poll = mio::Poll::new().unwrap();
        let wake = Arc::new(Waker::new(poll.registry(), mio::Token(0)).unwrap());
        let temp = tempfile::tempdir().unwrap();
        let writer = CaptureWriter::start(wake, Some(&temp.path().join("journal"))).unwrap();
        let seat = Ulid::new();
        let mut handle = writer.register(
            Ring::open(&temp.path().join("seat"), seat, 1, 0, Settings::default()).unwrap(),
        );
        writer.shared.queue.lock().unwrap().durable_capacity = 2 * OUTPUT_CHUNK_BYTES;
        let (started, ready) = mpsc::sync_channel(1);
        let (resume, gate) = mpsc::sync_channel(1);
        writer
            .shared
            .queue
            .lock()
            .unwrap()
            .jobs
            .push_back(Job::Gate(started, gate));
        writer.shared.ready.notify_one();
        ready.recv_timeout(Duration::from_secs(30)).unwrap();
        for seq in 1..=2 {
            assert!(handle.can_capture());
            handle
                .append_event(&Event::Output {
                    seat,
                    generation: 1,
                    seq,
                    bytes: vec![b'x'; OUTPUT_CHUNK_BYTES],
                })
                .unwrap();
        }
        assert!(!handle.can_capture());
        {
            let q = writer.shared.queue.lock().unwrap();
            assert_eq!(q.undurable_bytes, 2 * OUTPUT_CHUNK_BYTES);
            assert!(q.durable_paused);
            assert_eq!(q.durable_pauses, 1);
        }
        resume.send(()).unwrap();
        tick(&writer);
        assert!(handle.can_capture());
        let Reply::RingRecovery { status, .. } = handle
            .request(Query::Recovery)
            .unwrap()
            .recv_timeout(Duration::from_secs(30))
            .unwrap()
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(status.capture.undurable_bytes, 0);
        assert_eq!(status.capture.peak_undurable_bytes, 2 * OUTPUT_CHUNK_BYTES);
        assert!(status.capture.durable_rounds >= 1);
        drop(handle);
        drop(writer);
    }

    // keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
    #[test]
    fn orderly_shutdown_leaves_a_durable_journal_that_rebuilds_the_projection() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("seat");
        let seat = Ulid::new();
        let poll = mio::Poll::new().unwrap();
        let wake = Arc::new(Waker::new(poll.registry(), mio::Token(0)).unwrap());
        let writer = CaptureWriter::start(wake, Some(&temp.path().join("journal"))).unwrap();
        let mut handle =
            writer.register(Ring::open(&root, seat, 1, 0, Settings::default()).unwrap());
        let event = Event::Output {
            seat,
            generation: 1,
            seq: 1,
            bytes: b"\x1b[31mretained".to_vec(),
        };
        let mut playback = super::super::Playback::new(seat, 1, crate::SeatSize::default());
        playback.apply(&event).unwrap();
        handle.append_event(&event).unwrap();
        assert!(matches!(
            handle
                .request(Query::Checkpoint(playback.saved().encode().unwrap()))
                .unwrap()
                .recv_timeout(Duration::from_secs(30))
                .unwrap()
                .unwrap(),
            Reply::CheckpointCommitted { sequence: 1 }
        ));
        // Fault fixture suppresses optional idle projection, independently of
        // scheduler load; close/shutdown must only need the authoritative WAL.
        writer
            .shared
            .queue
            .lock()
            .unwrap()
            .pressure
            .get_mut(&(seat, 1))
            .unwrap()
            .full = true;
        drop(handle);
        drop(writer);
        assert_eq!(
            super::super::load_boundary(&root.join("committed"))
                .unwrap()
                .sequence,
            0
        );
        let ring = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        assert_eq!(ring.status().committed_sequence, 1);
        assert_eq!(ring.replay().unwrap().byte_end, playback.byte_end);
        assert_eq!(ring.history(1, 1).unwrap().records[0].event, event);
    }
    // keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
    #[test]
    fn one_writer_tick_orders_two_trickling_seats_without_full_flushes() {
        let poll = mio::Poll::new().unwrap();
        let wake = Arc::new(Waker::new(poll.registry(), mio::Token(0)).unwrap());
        let temp = tempfile::tempdir().unwrap();
        let writer = CaptureWriter::start(wake, Some(&temp.path().join("journal"))).unwrap();
        // Measure one explicit tick. Real elapsed time may otherwise trigger
        // the separate idle projection, which also records an ordered commit.
        writer.shared.queue.lock().unwrap().manual_ticks = true;
        let mut handles = Vec::new();
        for i in 0..2 {
            let seat = Ulid::new();
            let ring = Ring::open(
                &temp.path().join(i.to_string()),
                seat,
                1,
                0,
                Settings::default(),
            )
            .unwrap();
            let mut handle = writer.register(ring);
            handle
                .append_event(&Event::Output {
                    seat,
                    generation: 1,
                    seq: 1,
                    bytes: b"trickle\n".to_vec(),
                })
                .unwrap();
            handles.push(handle);
        }
        for handle in &handles {
            let Reply::RingRecovery { status, .. } = handle
                .request(Query::Recovery)
                .unwrap()
                .recv_timeout(Duration::from_secs(30))
                .unwrap()
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(status.captured_sequence, 1);
            assert_eq!(
                status.committed_sequence, 0,
                "append committed before explicit tick"
            );
        }
        tick(&writer);
        for handle in &handles {
            let Reply::RingRecovery { status, .. } = handle
                .request(Query::Recovery)
                .unwrap()
                .recv_timeout(Duration::from_secs(30))
                .unwrap()
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(status.committed_sequence, 1);
            assert_eq!(status.capture.durable_commits, 0);
            assert_eq!(
                status.capture.ordered_commits, 1,
                "one tick ordered each seat exactly once"
            );
        }
        drop(handles);
        drop(writer);
        for i in 0..2 {
            let root = temp.path().join(i.to_string());
            let boundary = super::super::load_boundary(&root.join("committed")).unwrap();
            let ring = Ring::open(&root, boundary.seat, 1, 0, Settings::default()).unwrap();
            assert_eq!(ring.history(1, 1).unwrap().records.len(), 1);
        }
    }

    // keeps: spec mux-daemon.md — Ring records, checkpoints and eviction
    #[test]
    fn stalled_disk_writer_keeps_admission_nonblocking_and_bounded_then_replays_every_byte() {
        let poll = mio::Poll::new().unwrap();
        let wake = Arc::new(Waker::new(poll.registry(), mio::Token(0)).unwrap());
        let temp = tempfile::tempdir().unwrap();
        let writer = CaptureWriter::start(wake, Some(&temp.path().join("journal"))).unwrap();
        let seat = Ulid::new();
        let ring = Ring::open(&temp.path().join("seat"), seat, 1, 0, Settings::default()).unwrap();
        let mut handle = writer.register(ring);
        // Use the actual byte admission path with a small fault-fixture budget.
        let budget = 2 * (OUTPUT_CHUNK_BYTES + HEADER);
        writer.shared.queue.lock().unwrap().seat_capacity = budget;
        let (started, ready) = mpsc::sync_channel(1);
        let (resume, gate) = mpsc::sync_channel(1);
        writer
            .shared
            .queue
            .lock()
            .unwrap()
            .jobs
            .push_back(Job::Gate(started, gate));
        writer.shared.ready.notify_one();
        ready.recv_timeout(Duration::from_secs(30)).unwrap();
        let resume = OpenGate(Some(resume));
        let (admitted, completed) = mpsc::sync_channel(1);
        let admission = thread::spawn(move || {
            for seq in 1..=2 {
                assert!(handle.can_capture());
                handle
                    .append_event(&Event::Output {
                        seat,
                        generation: 1,
                        seq,
                        bytes: vec![seq as u8; OUTPUT_CHUNK_BYTES],
                    })
                    .unwrap();
            }
            assert!(!handle.can_capture());
            assert!(!handle.can_capture());
            admitted.send(handle).unwrap();
        });
        // The writer is still gated. Completion proves admission did not wait
        // for disk work; this timeout is only a deadlock guard.
        let handle = completed.recv_timeout(Duration::from_secs(30)).unwrap();
        admission.join().unwrap();
        drop(resume);
        tick(&writer);
        let response = handle
            .request(Query::Recovery)
            .unwrap()
            .recv_timeout(Duration::from_secs(30))
            .unwrap()
            .unwrap();
        let Reply::RingRecovery { status, .. } = response else {
            panic!()
        };
        assert_eq!(status.byte_end as usize, 2 * OUTPUT_CHUNK_BYTES);
        assert_eq!(status.capture.peak_queue_bytes, budget);
        assert_eq!(status.capture.queue_full_count, 1);
        assert!(handle.can_capture());
        drop(handle);
        drop(writer);
        let ring = Ring::open(&temp.path().join("seat"), seat, 1, 0, Settings::default()).unwrap();
        let mut from = 1;
        while from <= 2 {
            let history = ring.history(from, 2).unwrap();
            assert!(history.gap.is_none());
            for record in &history.records {
                let Event::Output { bytes, seq, .. } = &record.event else {
                    panic!()
                };
                assert_eq!(bytes, &vec![*seq as u8; OUTPUT_CHUNK_BYTES]);
            }
            from = history.next.unwrap_or(3);
        }
    }
}
