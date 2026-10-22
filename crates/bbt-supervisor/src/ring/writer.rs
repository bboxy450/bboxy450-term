//! One disk writer for a supervisor. The PTY actor never waits for ring IO.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::File;
use std::io;
use std::os::unix::fs::MetadataExt;
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
    durable_pauses: u64,
    durable_rounds: u64,
    debt_since: Option<std::time::Instant>,
    durable_capacity: usize,
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
            durable_pauses: 0,
            durable_rounds: 0,
            debt_since: None,
            durable_capacity: DURABLE_BYTES as usize,
        }
    }
}
struct Shared {
    queue: Mutex<Queue>,
    ready: Condvar,
    wake: Arc<Waker>,
}
enum Job {
    Register(Box<Ring>),
    Append(Key, Event, usize),
    Query(Key, Query, mpsc::SyncSender<Result<Reply, Failure>>, usize),
    Close(Key),
    #[cfg(test)]
    Gate(mpsc::SyncSender<()>, mpsc::Receiver<()>),
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
    pub fn start(wake: Arc<Waker>) -> io::Result<Self> {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            ready: Condvar::new(),
            wake,
        });
        let state = shared.clone();
        let thread = thread::Builder::new()
            .name("bbt-capture-writer".into())
            .spawn(move || run(state))?;
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
        let available = has_room(&q, self.key, cost) && !q.durable_paused && !tick_limit;
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
        if let Some(error) = q.failures.get(&self.key) {
            return Err(io::Error::other(error.to_string()));
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
    drop(q);
    let _ = shared.wake.wake();
}
fn failure(error: impl std::fmt::Display) -> Failure {
    Failure::new(FailureCode::Io, format!("capture writer failed: {error}"))
}
fn run(shared: Arc<Shared>) {
    let mut rings: HashMap<Key, Ring> = HashMap::new();
    let mut next_flush = std::time::Instant::now() + TICK;
    loop {
        let job = {
            let mut q = shared.queue.lock().unwrap();
            if q.jobs.is_empty() && !q.stop {
                q = shared
                    .ready
                    .wait_timeout(
                        q,
                        next_flush.saturating_duration_since(std::time::Instant::now()),
                    )
                    .unwrap()
                    .0;
            }
            if q.stop && q.jobs.is_empty() {
                break;
            }
            q.jobs.pop_front()
        };
        let active = job.is_some();
        let active_started = std::time::Instant::now();
        match job {
            Some(Job::Register(ring)) => {
                rings.insert((ring.boundary.seat, ring.boundary.generation), *ring);
            }
            Some(Job::Append(key, event, cost)) => {
                let result = rings
                    .get_mut(&key)
                    .ok_or_else(|| failure("ring absent"))
                    .and_then(|ring| ring.append_event(&event).map_err(failure));
                if let Err(error) = result {
                    shared.queue.lock().unwrap().failures.insert(key, error);
                }
                if rings.values().any(|ring| ring.strong_requested) {
                    if let Err(error) = commit_group(&mut rings, &shared, true, false) {
                        shared.queue.lock().unwrap().failures.insert(key, error);
                    }
                }
                release(&shared, key, cost);
            }
            Some(Job::Query(key, query, reply, cost)) => {
                // Never hold the admission mutex across filesystem IO.
                let failed = shared.queue.lock().unwrap().failures.get(&key).cloned();
                let mut result = failed.map_or_else(
                    || {
                        rings
                            .get_mut(&key)
                            .ok_or_else(|| failure("ring absent"))
                            .and_then(|ring| query_ring(ring, query).map_err(failure))
                    },
                    Err,
                );
                if result.is_ok() && rings.values().any(|ring| ring.strong_requested) {
                    if let Err(error) = commit_group(&mut rings, &shared, true, false) {
                        result = Err(error);
                    }
                }
                release(&shared, key, cost);
                let result = result.map(|mut result| {
                    let status = match &mut result {
                        Reply::RingRecovery { status, .. } => Some(status),
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
                if let Some(ring) = rings.get_mut(&key) {
                    let _ = ring.flush();
                }
                let _ = commit_group(&mut rings, &shared, true, false);
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
            None => {}
        }
        if active {
            shared.queue.lock().unwrap().timings.busy_ns +=
                active_started.elapsed().as_nanos() as u64;
        }
        let force_durable = {
            let q = shared.queue.lock().unwrap();
            q.jobs.is_empty()
                && q.jobs_in_flight == 0
                && (q.durable_paused
                    || q.debt_since
                        .is_some_and(|at| at.elapsed() >= DURABLE_INTERVAL))
        };
        if std::time::Instant::now() >= next_flush || force_durable {
            let round_started = std::time::Instant::now();
            let strong = force_durable;
            if let Err(error) = commit_group(&mut rings, &shared, strong, true) {
                for &key in rings.keys() {
                    shared
                        .queue
                        .lock()
                        .unwrap()
                        .failures
                        .insert(key, error.clone());
                }
            }
            shared.queue.lock().unwrap().timings.busy_ns +=
                round_started.elapsed().as_nanos() as u64;
            next_flush = std::time::Instant::now() + TICK;
        }
    }
    let _ = commit_group(&mut rings, &shared, true, true);
}

fn commit_group(
    rings: &mut HashMap<Key, Ring>,
    shared: &Shared,
    strong: bool,
    flush_pending: bool,
) -> Result<(), Failure> {
    let pending = rings.values().any(|ring| {
        ring.strong_dirty || ring.strong_requested || ring.boundary.length != ring.durable.length
    });
    if !pending {
        return Ok(());
    }
    let started = std::time::Instant::now();
    for ring in rings.values_mut() {
        // A checkpoint flushes its own file and all previously ordered device
        // data; it must not force every other seat's fresh tail into a new
        // marker between ticks. Only the common tick drains all pending tails.
        if flush_pending || ring.strong_requested {
            ring.flush_ordered().map_err(failure)?;
        }
    }
    let mut durable_ends = Vec::new();
    if strong {
        let mut devices = BTreeMap::new();
        for (&key, ring) in rings.iter() {
            if !ring.strong_dirty && !ring.strong_requested {
                continue;
            }
            // Submit file ordering and directory metadata before asking each
            // device to flush all of its previously submitted writes once.
            crate::durability::sync_directory(&File::open(&ring.root).map_err(failure)?)
                .map_err(failure)?;
            if ring.root.join("blobs").exists() {
                crate::durability::sync_directory(
                    &File::open(ring.root.join("blobs")).map_err(failure)?,
                )
                .map_err(failure)?;
            }
            let file = File::open(ring.data_path()).map_err(failure)?;
            devices
                .entry(file.metadata().map_err(failure)?.dev())
                .or_insert((key, file));
        }
        for (_, (key, file)) in devices {
            let sync_started = std::time::Instant::now();
            crate::durability::sync_file(&file, true).map_err(failure)?;
            rings.get_mut(&key).unwrap().stats.sync_ns += sync_started.elapsed().as_nanos() as u64;
        }
        for (&key, ring) in rings.iter_mut() {
            if ring.strong_dirty || ring.strong_requested {
                durable_ends.push((key, ring.durable.byte_end));
                ring.finish_durable().map_err(failure)?;
            }
        }
    }
    let mut q = shared.queue.lock().unwrap();
    for (&key, ring) in rings.iter() {
        let Some(pressure) = q.pressure.get_mut(&key) else {
            continue;
        };
        while pressure
            .unordered
            .front()
            .is_some_and(|(seq, _)| *seq <= ring.durable.sequence)
        {
            pressure.unordered.pop_front();
        }
    }
    if strong {
        for (key, end) in durable_ends {
            let refund = if let Some(pressure) = q.pressure.get_mut(&key) {
                let refund = end.saturating_sub(pressure.full_byte_end);
                pressure.full_byte_end = end;
                refund as usize
            } else {
                0
            };
            q.undurable_bytes = q.undurable_bytes.saturating_sub(refund);
        }
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
    Ok(())
}

fn query_ring(ring: &mut Ring, query: Query) -> super::Result<Reply> {
    match query {
        Query::Recovery => Ok(Reply::RingRecovery {
            status: ring.status(),
            checkpoint: ring.latest_checkpoint()?,
        }),
        Query::History { from, to } => Ok(Reply::RingHistory {
            history: Box::new(ring.history(from, to)?),
        }),
        Query::Checkpoint(bytes) => {
            let sequence = super::Saved::decode(&bytes)?.sequence;
            ring.checkpoint(bytes)?;
            Ok(Reply::CheckpointCommitted { sequence })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn undurable_admission_budget_pauses_until_a_full_flush_returns_credit() {
        let poll = mio::Poll::new().unwrap();
        let wake = Arc::new(Waker::new(poll.registry(), mio::Token(0)).unwrap());
        let writer = CaptureWriter::start(wake).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let seat = Ulid::new();
        let mut handle =
            writer.register(Ring::open(temp.path(), seat, 1, 0, Settings::default()).unwrap());
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
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
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
        let until = Instant::now() + Duration::from_secs(5);
        while !handle.can_capture() {
            assert!(
                Instant::now() < until,
                "full-flush credit did not restore admission"
            );
            thread::sleep(Duration::from_millis(2));
        }
        let Reply::RingRecovery { status, .. } = handle
            .request(Query::Recovery)
            .unwrap()
            .recv_timeout(Duration::from_secs(2))
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

    #[test]
    fn one_writer_tick_orders_two_trickling_seats_without_full_flushes() {
        let poll = mio::Poll::new().unwrap();
        let wake = Arc::new(Waker::new(poll.registry(), mio::Token(0)).unwrap());
        let writer = CaptureWriter::start(wake).unwrap();
        let temp = tempfile::tempdir().unwrap();
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
        let until = Instant::now() + Duration::from_secs(2);
        for handle in &handles {
            loop {
                let Reply::RingRecovery { status, .. } = handle
                    .request(Query::Recovery)
                    .unwrap()
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap()
                    .unwrap()
                else {
                    panic!()
                };
                if status.committed_sequence == 1 {
                    assert_eq!(
                        status.capture.durable_commits, 0,
                        "a hot tick used a full flush"
                    );
                    assert_eq!(status.capture.ordered_commits, 1);
                    break;
                }
                assert!(Instant::now() < until, "tick did not publish its boundary");
                thread::sleep(Duration::from_millis(2));
            }
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

    #[test]
    fn stalled_disk_writer_keeps_admission_nonblocking_and_bounded_then_replays_every_byte() {
        let poll = mio::Poll::new().unwrap();
        let wake = Arc::new(Waker::new(poll.registry(), mio::Token(0)).unwrap());
        let writer = CaptureWriter::start(wake).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let seat = Ulid::new();
        let ring = Ring::open(temp.path(), seat, 1, 0, Settings::default()).unwrap();
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
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        let unblock = thread::spawn(move || {
            thread::sleep(Duration::from_millis(150));
            let _ = resume.send(());
        });
        let start = Instant::now();
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
        assert!(
            start.elapsed() < Duration::from_millis(50),
            "admission waited on stalled IO"
        );
        {
            let q = writer.shared.queue.lock().unwrap();
            assert_eq!(q.pressure[&(seat, 1)].bytes, budget);
            assert_eq!(q.pressure[&(seat, 1)].full_count, 1);
        }
        let response = handle
            .request(Query::Recovery)
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        let Reply::RingRecovery { status, .. } = response else {
            panic!()
        };
        assert_eq!(status.byte_end as usize, 2 * OUTPUT_CHUNK_BYTES);
        assert_eq!(status.capture.peak_queue_bytes, budget);
        assert_eq!(status.capture.queue_full_count, 1);
        let until = Instant::now() + Duration::from_secs(5);
        while !handle.can_capture() {
            assert!(
                Instant::now() < until,
                "ordered tick did not restore admission after IO recovered"
            );
            thread::sleep(Duration::from_millis(2));
        }
        unblock.join().unwrap();
        drop(handle);
        drop(writer);
        let ring = Ring::open(temp.path(), seat, 1, 0, Settings::default()).unwrap();
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
