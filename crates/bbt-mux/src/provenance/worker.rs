// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One paced IO owner per root. Cockpit calls only enqueue or try-receive;
//! filesystem/SQLite latency and recovery never run on the control worker.
use super::engine::{Engine, SigningSnapshot};
use crate::{
    Ulid,
    adapters::hook::ShimEvent,
    bus::BusHandle,
    identity::IdentityRuntime,
    index::Index,
    store::{Result, invalid},
};
use std::{
    collections::{BTreeSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const INPUTS: usize = 4096;
const MIN_REST: Duration = Duration::from_millis(250);
const IDLE_REST: Duration = Duration::from_secs(1);
const MAX_REST: Duration = Duration::from_secs(5);

enum Input {
    Register(Ulid, u64, PathBuf, bool),
    Claim(serde_json::Value),
}
struct Job {
    index: Arc<Index>,
    bus: BusHandle,
    runtime: tokio::runtime::Handle,
    identity: SigningSnapshot,
    active: Vec<(Ulid, u64, u64)>,
    inputs: VecDeque<Input>,
    overflow: bool,
}
struct Completed {
    result: Result<bool>,
    paths: BTreeSet<String>,
    rest: Duration,
}

pub(crate) struct Worker {
    engine: Arc<Mutex<Engine>>,
    jobs: mpsc::SyncSender<Job>,
    completed: mpsc::Receiver<Completed>,
    stopped: Arc<AtomicBool>,
    inputs: VecDeque<Input>,
    overflow: bool,
    running: bool,
    next: Instant,
    refreshed: BTreeSet<String>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Worker {
    pub fn open(index: &Index, repository: &Path, runtime: tokio::runtime::Handle) -> Result<Self> {
        let engine = Arc::new(Mutex::new(Engine::open(index, repository, runtime)?));
        let (jobs, incoming) = mpsc::sync_channel::<Job>(1);
        let (outgoing, completed) = mpsc::sync_channel(1);
        let stopped = Arc::new(AtomicBool::new(false));
        let (state, cancel) = (engine.clone(), stopped.clone());
        let thread = std::thread::Builder::new()
            .name("bbt-provenance".into())
            .spawn(move || {
                while !cancel.load(Ordering::Acquire) {
                    let job = match incoming.recv_timeout(Duration::from_secs(1)) {
                        Ok(job) => job,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    if cancel.load(Ordering::Acquire) {
                        break;
                    }
                    let started = Instant::now();
                    let Ok(mut engine) = state.lock() else {
                        break;
                    };
                    if cancel.load(Ordering::Acquire) {
                        break;
                    }
                    for input in job.inputs {
                        match input {
                            Input::Register(s, g, path, local) => {
                                engine.register(s, g, &path, local)
                            }
                            Input::Claim(metadata) => engine.note_claim(&metadata),
                        }
                    }
                    if job.overflow {
                        engine.reconcile_inputs();
                    }
                    let mut index = (*job.index).clone();
                    let result = engine.step_snapshot(
                        &mut index,
                        &job.bus,
                        &job.runtime,
                        &job.identity,
                        &job.active,
                    );
                    let paths = engine.take_refreshed();
                    drop(engine);
                    let rest = (started.elapsed() * 8).clamp(
                        if result.as_ref().is_ok_and(|changed| *changed) {
                            MIN_REST
                        } else {
                            IDLE_REST
                        },
                        MAX_REST,
                    );
                    if outgoing
                        .send(Completed {
                            result,
                            paths,
                            rest,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self {
            engine,
            jobs,
            completed,
            stopped,
            inputs: VecDeque::new(),
            overflow: false,
            running: false,
            next: Instant::now(),
            refreshed: BTreeSet::new(),
            thread: Some(thread),
        })
    }
    fn push(&mut self, input: Input) {
        // Registration/claims describe current state; a burst needs its latest
        // value, not a queue of superseded copies. Raw signed events are intact.
        match &input {
            Input::Register(seat, ..) => self
                .inputs
                .retain(|old| !matches!(old, Input::Register(s, ..) if s == seat)),
            Input::Claim(value) => self
                .inputs
                .retain(|old| !matches!(old, Input::Claim(v) if v["ref"] == value["ref"])),
        }
        if self.inputs.len() < INPUTS {
            self.inputs.push_back(input);
        } else {
            self.overflow = true;
        }
    }
    pub fn register(&mut self, seat: Ulid, generation: u64, cwd: &Path, local: bool) {
        self.push(Input::Register(seat, generation, cwd.into(), local));
    }
    // Hook/OSC edits are work in progress. Their normal status/audit lanes
    // remain intact; they do not trigger artifact hashing or provenance IO.
    pub fn hook(&mut self, _seat: Ulid, _generation: u64, _event: &ShimEvent) {}
    pub fn osc(&mut self, _seat: Ulid, _generation: u64, _params: &[Vec<u8>]) {}
    pub fn note_claim(&mut self, metadata: &serde_json::Value) {
        self.push(Input::Claim(serde_json::json!({"ref":metadata["ref"],"id":metadata["id"],
            "digest":metadata["digest"],"deleted":metadata["deleted"],"observed":metadata["observed"]})));
    }
    pub fn recovering(&self) -> bool {
        !self.inputs.is_empty()
            || self
                .engine
                .try_lock()
                .map_or(true, |engine| engine.recovering())
    }
    pub fn normalize(&self, seat: Ulid, generation: u64, reference: &str) -> Option<String> {
        self.engine
            .try_lock()
            .ok()?
            .normalize(seat, generation, reference)
    }
    pub fn claim_ready(&self, index: &Index, reference: &str) -> Result<bool> {
        self.engine
            .try_lock()
            .map_err(|_| invalid("artifact catalog projection pending"))?
            .claim_ready(index, reference)
    }
    pub fn take_refreshed(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.refreshed)
    }
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }
    pub fn step(
        &mut self,
        index: Arc<Index>,
        bus: &BusHandle,
        runtime: &tokio::runtime::Handle,
        identity: &IdentityRuntime,
        active: &[(Ulid, u64, u64)],
    ) -> Result<bool> {
        if self.stopped.load(Ordering::Acquire) {
            return Ok(false);
        }
        let mut changed = false;
        match self.completed.try_recv() {
            Ok(done) => {
                self.running = false;
                self.next = Instant::now() + done.rest;
                self.refreshed.extend(done.paths);
                changed = done.result?;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(invalid("provenance worker stopped"));
            }
            Err(mpsc::TryRecvError::Empty) => (),
        }
        if self.running || Instant::now() < self.next || self.stopped.load(Ordering::Acquire) {
            return Ok(changed);
        }
        let identity = SigningSnapshot::capture(identity)?;
        let job = Job {
            index,
            bus: bus.clone(),
            runtime: runtime.clone(),
            identity,
            active: active.to_vec(),
            inputs: std::mem::take(&mut self.inputs),
            overflow: std::mem::take(&mut self.overflow),
        };
        self.jobs
            .try_send(job)
            .map_err(|_| invalid("provenance worker unavailable"))?;
        self.running = true;
        Ok(changed)
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
        // Never join filesystem IO on the control thread. The worker retains
        // its actual root writer lease until its current bounded page ends.
        drop(self.thread.take());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bus::Broker, store::sqlite::SqliteStore};
    use ed25519_dalek::SigningKey;

    #[test]
    fn slow_background_provenance_never_owns_control_or_shutdown() {
        let temp = tempfile::tempdir().unwrap();
        let repository = temp.path().join("repository");
        std::fs::create_dir_all(&repository).unwrap();
        let run = temp.path().join("run");
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let key = SigningKey::from_bytes(&[64; 32]);
        let identity = IdentityRuntime::open(&run, key.clone(), runtime.handle().clone()).unwrap();
        let bus =
            BusHandle::start(runtime.handle(), Broker::open(&run, key).unwrap(), 128).unwrap();
        let mut index = Index::open(
            &repository.join(".bboxy"),
            SqliteStore::open(&run.join("index")).unwrap(),
        )
        .unwrap();
        index.route_activity(bus.clone(), runtime.handle().clone());
        let index = Arc::new(index);
        let mut worker = Worker::open(&index, &repository, runtime.handle().clone()).unwrap();
        // A deliberately stalled real worker, not an owner-side sleep. Enqueue,
        // catalogue checks, another poll and Drop must all stay nonblocking.
        let engine = worker.engine.clone();
        let gate = engine.lock().unwrap();
        let before = index.activity().last_sequence().unwrap();
        let started = Instant::now();
        worker.register(Ulid(1), 1, &repository, true);
        worker
            .step(index.clone(), &bus, runtime.handle(), &identity, &[])
            .unwrap();
        assert!(worker.recovering());
        assert!(worker.normalize(Ulid(1), 1, "src/main.rs").is_none());
        worker
            .step(index.clone(), &bus, runtime.handle(), &identity, &[])
            .unwrap();
        let thread = worker.thread.take().unwrap();
        drop(worker);
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "control waited for provenance IO"
        );
        drop(gate);
        thread.join().unwrap();
        assert_eq!(
            index.activity().last_sequence().unwrap(),
            before,
            "cancelled queued work published after close"
        );
    }
}
