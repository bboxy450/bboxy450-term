//! One disk writer for a supervisor. The PTY actor never waits for ring IO.

use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use mio::Waker;
use ulid::Ulid;

use super::{HEADER, Ring, Settings};
use crate::protocol::{Event, Failure, FailureCode, OUTPUT_CHUNK_BYTES, Reply};

const SEAT_BYTES: usize = 64 * 1024 * 1024;
const TOTAL_BYTES: usize = 128 * 1024 * 1024;
const MAX_JOBS: usize = 8192;
type Key = (Ulid, u64);
pub(crate) type Response = mpsc::Receiver<Result<Reply, Failure>>;

#[derive(Default)]
struct Pressure {
    bytes: usize,
    peak: usize,
    full_count: u64,
    full: bool,
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
        q.pressure.insert(handle.key, Pressure::default());
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
        let available = has_room(&q, self.key, cost);
        let pressure = q.pressure.get_mut(&self.key).unwrap();
        if !available && !pressure.full {
            pressure.full_count += 1;
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
        charge(&mut q, self.key, cost);
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
    drop(q);
    let _ = shared.wake.wake();
}
fn failure(error: impl std::fmt::Display) -> Failure {
    Failure::new(FailureCode::Io, format!("capture writer failed: {error}"))
}
fn run(shared: Arc<Shared>) {
    let mut rings: HashMap<Key, Ring> = HashMap::new();
    let mut next_flush = std::time::Instant::now() + Duration::from_millis(20);
    loop {
        let job = {
            let mut q = shared.queue.lock().unwrap();
            if q.jobs.is_empty() && !q.stop {
                q = shared
                    .ready
                    .wait_timeout(q, Duration::from_millis(20))
                    .unwrap()
                    .0;
            }
            if q.stop && q.jobs.is_empty() {
                break;
            }
            q.jobs.pop_front()
        };
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
                release(&shared, key, cost);
            }
            Some(Job::Query(key, query, reply, cost)) => {
                // Never hold the admission mutex across filesystem IO.
                let failed = shared.queue.lock().unwrap().failures.get(&key).cloned();
                let result = failed.map_or_else(
                    || {
                        rings
                            .get_mut(&key)
                            .ok_or_else(|| failure("ring absent"))
                            .and_then(|ring| query_ring(ring, query).map_err(failure))
                    },
                    Err,
                );
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
                    }
                    result
                });
                let _ = reply.try_send(result);
                let _ = shared.wake.wake();
            }
            Some(Job::Close(key)) => {
                if let Some(mut ring) = rings.remove(&key) {
                    let _ = ring.flush();
                }
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
        if std::time::Instant::now() >= next_flush {
            for (&key, ring) in &mut rings {
                if let Err(error) = ring.flush_due() {
                    shared
                        .queue
                        .lock()
                        .unwrap()
                        .failures
                        .insert(key, failure(error));
                }
            }
            next_flush = std::time::Instant::now() + Duration::from_millis(20);
        }
    }
    for ring in rings.values_mut() {
        let _ = ring.flush();
    }
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
        assert!(handle.can_capture());
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
