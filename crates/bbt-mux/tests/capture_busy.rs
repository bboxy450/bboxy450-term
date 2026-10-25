#![allow(clippy::disallowed_methods)] // Bounded real-PTY/IO-worker fault fixture.

mod common;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use bbt_mux::logic::{Logic, LogicConfig, SupervisorConnection};
use bbt_mux::registry::SeatRecord;
use bbt_mux::{Action, Client, Event, Failure, FailureCode, LaunchSpec, Reply, Supervisor, Ulid};

struct Congested {
    client: Client,
    blocked: Arc<AtomicBool>,
    rejected: Arc<AtomicU64>,
    committed: Arc<AtomicU64>,
}
impl SupervisorConnection for Congested {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        if self.blocked.load(Ordering::Acquire)
            && matches!(
                action,
                Action::RingRecovery { .. }
                    | Action::ReadRing { .. }
                    | Action::ReadRingBatch { .. }
                    | Action::PublishCheckpoint { .. }
            )
        {
            self.rejected.fetch_add(1, Ordering::Relaxed);
            return Err(Failure {
                code: FailureCode::Busy,
                message: "injected capture admission pressure".into(),
            }
            .into());
        }
        let reply = self.client.request(action)?;
        if let Reply::CheckpointCommitted { sequence } = &reply {
            self.committed.store(*sequence, Ordering::Release);
        }
        Ok(reply)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        SupervisorConnection::receive(&mut self.client, timeout)
    }
}

// keeps: spec mux-daemon
#[test]
fn capture_busy_exhaustion_keeps_logic_alive_and_retries_the_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join(".bboxy");
    std::fs::create_dir(&repository).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let blocked = Arc::new(AtomicBool::new(false));
    let rejected = Arc::new(AtomicU64::new(0));
    let committed = Arc::new(AtomicU64::new(0));
    let logic = Logic::start(
        Congested {
            client: supervisor.connect().unwrap(),
            blocked: blocked.clone(),
            rejected: rejected.clone(),
            committed: committed.clone(),
        },
        LogicConfig {
            run_dir: temp.path().join("run"),
            repository,
            ring: bbt_mux::ring::Settings {
                checkpoint_bytes: 256,
                ..Default::default()
            },
        },
        ed25519_dalek::SigningKey::from_bytes(&[61; 32]),
        runtime.handle(),
    )
    .unwrap();
    let seat = Ulid::new();
    let mut launch = LaunchSpec::new("python3", temp.path());
    launch.seat = seat;
    launch.args = vec!["-c".into(), "import os,time\nwhile True:\n os.write(1,b'capture busy fixture '+b'x'*512+b'\\n'); time.sleep(0.01)".into()];
    runtime
        .block_on(logic.client.spawn(
            SeatRecord::new(seat, Ulid::new(), "busy fixture", "python"),
            launch,
        ))
        .unwrap();
    blocked.store(true, Ordering::Release);
    let until = Instant::now() + Duration::from_secs(5);
    while rejected.load(Ordering::Relaxed) < 12 {
        assert!(
            Instant::now() < until,
            "did not exhaust two bounded retry attempts"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // The old implementation terminated the worker on the first Busy. A read
    // command must still be serviced after two exhausted retry windows.
    let live = runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
    assert!(live.sequence > 0);
    blocked.store(false, Ordering::Release);
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if committed.load(Ordering::Acquire) >= live.sequence {
            break;
        }
        assert!(
            Instant::now() < until,
            "checkpoint was not retried after capacity recovered"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}

struct PagedReplay {
    client: Client,
    hold: Arc<AtomicBool>,
    delay: Arc<AtomicBool>,
    pages: Arc<AtomicU64>,
    held_sequence: Arc<AtomicU64>,
    checkpoint: Arc<AtomicU64>,
    latest: Option<Event>,
}
impl SupervisorConnection for PagedReplay {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        let reply = self.client.request(action)?;
        if let Reply::CheckpointCommitted { sequence } = &reply {
            self.checkpoint.store(*sequence, Ordering::Release);
        }
        if self.delay.load(Ordering::Acquire) {
            for history in common::ring_histories(&reply) {
                if !history.records.is_empty() {
                    self.pages.fetch_add(1, Ordering::Release);
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
        Ok(reply)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        if self.hold.load(Ordering::Acquire) {
            let mut next = SupervisorConnection::receive(&mut self.client, timeout)?;
            while let Some(event) = next {
                self.held_sequence
                    .store(event.sequence().unwrap(), Ordering::Release);
                self.latest = Some(event);
                next = SupervisorConnection::receive(&mut self.client, Duration::ZERO)?;
            }
            return Ok(None);
        }
        if let Some(event) = self.latest.take() {
            return Ok(Some(event));
        }
        SupervisorConnection::receive(&mut self.client, timeout)
    }
}

// keeps: spec mux-daemon
#[test]
fn large_capture_catchup_yields_to_control_between_history_pages() {
    let temp = tempfile::tempdir().unwrap();
    let repository = temp.path().join(".bboxy");
    std::fs::create_dir(&repository).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(common::capture_config(temp.path())).unwrap();
    let hold = Arc::new(AtomicBool::new(true));
    let delay = Arc::new(AtomicBool::new(false));
    let pages = Arc::new(AtomicU64::new(0));
    let held_sequence = Arc::new(AtomicU64::new(0));
    let checkpoint = Arc::new(AtomicU64::new(0));
    let logic = Logic::start(
        PagedReplay {
            client: supervisor.connect().unwrap(),
            hold: hold.clone(),
            delay: delay.clone(),
            pages: pages.clone(),
            held_sequence: held_sequence.clone(),
            checkpoint: checkpoint.clone(),
            latest: None,
        },
        LogicConfig {
            run_dir: temp.path().join("run"),
            repository,
            ring: Default::default(),
        },
        ed25519_dalek::SigningKey::from_bytes(&[62; 32]),
        runtime.handle(),
    )
    .unwrap();
    let mut output = LaunchSpec::new("python3", temp.path());
    output.args = vec![
        "-c".into(),
        "import os,time\nwhile not os.path.exists('go'): time.sleep(.002)\ndata=b'x'*(4*1024*1024)+b'\\nREPLAY-COMPLETE\\n'\nwhile data:\n n=os.write(1,data);data=data[n:]\ntime.sleep(60)".into(),
    ];
    let id = output.seat;
    let swarm = Ulid::new();
    runtime
        .block_on(logic.client.spawn(
            SeatRecord::new(id, swarm, "replay fixture", "python"),
            output,
        ))
        .unwrap();
    let mut quiet = LaunchSpec::new("/bin/sleep", temp.path());
    quiet.args = vec!["60".into()];
    let other = quiet.seat;
    runtime
        .block_on(logic.client.spawn(
            SeatRecord::new(other, swarm, "control fixture", "shell"),
            quiet,
        ))
        .unwrap();
    std::fs::write(temp.path().join("go"), []).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    let through = loop {
        let status = runtime
            .block_on(logic.client.history(id, 1, u64::MAX, u64::MAX))
            .unwrap()
            .status;
        if status.byte_end >= 4 * 1024 * 1024 + 17
            && held_sequence.load(Ordering::Acquire) >= status.captured_sequence
        {
            break status.captured_sequence;
        }
        assert!(Instant::now() < until, "fixture prefix was not captured");
        std::thread::sleep(Duration::from_millis(5));
    };
    delay.store(true, Ordering::Release);
    hold.store(false, Ordering::Release);
    let until = Instant::now() + Duration::from_secs(5);
    while pages.load(Ordering::Acquire) == 0 {
        assert!(Instant::now() < until, "catch-up never started");
        std::thread::sleep(Duration::from_millis(2));
    }
    let started = Instant::now();
    let record = runtime
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(1), logic.client.record(other)).await
        })
        .expect("control starved behind the whole replay prefix")
        .unwrap();
    let latency = started.elapsed();
    assert_eq!(record.id, other);
    assert!(
        checkpoint.load(Ordering::Acquire) < through,
        "control was observed only after catch-up had completed"
    );
    delay.store(false, Ordering::Release);
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let snap = runtime.block_on(logic.client.snapshot(id, 1)).unwrap();
        if snap.sequence >= through {
            let text: String = snap
                .screen
                .cells
                .iter()
                .flatten()
                .map(|cell| cell.c)
                .collect();
            assert!(text.contains("REPLAY-COMPLETE"));
            break;
        }
        assert!(Instant::now() < until, "yielded replay did not finish");
        std::thread::sleep(Duration::from_millis(10));
    }
    println!(
        "control answered during paged replay: {latency:?}; pages={}; final sequence={through}",
        pages.load(Ordering::Acquire)
    );
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
