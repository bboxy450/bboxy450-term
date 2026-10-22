#![allow(clippy::disallowed_methods)] // Bounded real-PTY/IO-worker fault fixture.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use bbt_mux::logic::{Logic, LogicConfig, SupervisorConnection};
use bbt_mux::registry::SeatRecord;
use bbt_mux::{
    Action, Client, Config, Event, Failure, FailureCode, LaunchSpec, Reply, Supervisor, Ulid,
};

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
    let supervisor = Supervisor::start(Config::default()).unwrap();
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
