#![allow(clippy::disallowed_methods)] // Bounded synthetic PTY/socket IO fixture.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use bbt_mux::logic::{Logic, LogicConfig, SocketSupervisor, SupervisorConnection};
use bbt_mux::registry::SeatRecord;
use bbt_mux::{Action, Config, Event, LaunchSpec, Reply, Supervisor, Ulid};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

struct RequestFlood {
    client: SocketSupervisor,
    gate: PathBuf,
    armed: Arc<AtomicBool>,
    requests: Arc<AtomicU64>,
    replayed: Arc<AtomicU64>,
    exit: Arc<AtomicU64>,
}
impl SupervisorConnection for RequestFlood {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        let reply = self.client.request(action)?;
        if let Reply::RingHistory { history } = &reply {
            self.replayed
                .fetch_add(history.records.len() as u64, Ordering::Release);
        }
        Ok(reply)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        if self.armed.swap(false, Ordering::AcqRel) {
            let started = Instant::now();
            std::fs::write(&self.gate, b"go")?;
            for _ in 0..10_000 {
                self.client.request(Action::ListOwnedSeats)?;
                self.requests.fetch_add(1, Ordering::Release);
            }
            eprintln!(
                "socket backlog: 10000 requests completed in {:?}",
                started.elapsed()
            );
        }
        let event = self.client.receive(timeout)?;
        if let Some(Event::Exited { seq, .. }) = &event {
            self.exit.store(*seq, Ordering::Release);
        }
        Ok(event)
    }
}

#[test]
fn socket_request_flood_replays_exact_output_and_delivers_exit_to_logic() {
    let root = tempfile::Builder::new()
        .prefix("w12g-replay-")
        .tempdir_in("/tmp")
        .unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let socket = root.path().join("supervisor.sock");
    let supervisor = Supervisor::start(Config {
        socket_path: Some(socket.clone()),
        journal_directory: Some(root.path().join("supervisor-journal")),
        ..Default::default()
    })
    .unwrap();
    let armed = Arc::new(AtomicBool::new(false));
    let requests = Arc::new(AtomicU64::new(0));
    let replayed = Arc::new(AtomicU64::new(0));
    let exit = Arc::new(AtomicU64::new(0));
    let gate = root.path().join("start");
    let connection = RequestFlood {
        client: SocketSupervisor::connect(&socket, supervisor.socket_capability()).unwrap(),
        gate: gate.clone(),
        armed: armed.clone(),
        requests: requests.clone(),
        replayed: replayed.clone(),
        exit: exit.clone(),
    };
    let repository = root.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    let logic = Logic::start(
        connection,
        LogicConfig {
            run_dir: root.path().join("logic"),
            repository,
            ring: Default::default(),
        },
        SigningKey::from_bytes(&[97; 32]),
        runtime.handle(),
    )
    .unwrap();
    let mut spec = LaunchSpec::new("python3", root.path());
    // Raw PTY, deterministic synthetic bytes, no model/transcript/keychain.
    spec.args = vec![
        "-c".into(),
        r#"
import os, sys, time, tty
tty.setraw(1)
while not os.path.exists(sys.argv[1]): time.sleep(.001)
block = (b'0123456789abcdef' * 255) + b'0123456789abcde\n'
for _ in range(16384):
    rest = memoryview(block)
    while rest: rest = rest[os.write(1, rest):]
os.write(1, b'\r\nW12G-TAIL\r\n')
"#
        .into(),
        gate.to_string_lossy().into(),
    ];
    let seat = spec.seat;
    let info = runtime
        .block_on(logic.client.spawn(
            SeatRecord::new(seat, Ulid::new(), "socket flood", "synthetic"),
            spec,
        ))
        .unwrap();
    eprintln!("socket backlog: owned_pid={} seat={seat}", info.pid);
    armed.store(true, Ordering::Release);
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(90), async {
            loop {
                let status = logic.client.adapter_status(seat, 1).await.unwrap();
                if status.source == "exit" {
                    assert_eq!(status.detail.as_deref(), Some("exit 0"));
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("logic never observed the full-rate producer's exit");
    });
    assert_eq!(requests.load(Ordering::Acquire), 10_000);
    let logic_replayed_records = replayed.load(Ordering::Acquire);
    assert!(
        logic_replayed_records > 0,
        "logic did not replay a coalesced span"
    );
    let exit_sequence = exit.load(Ordering::Acquire);
    assert!(
        exit_sequence > 1024,
        "fixture must exceed the old notification cap"
    );
    let snapshot = runtime.block_on(logic.client.snapshot(seat, 1)).unwrap();
    assert_eq!(snapshot.sequence, exit_sequence);
    assert!(snapshot.screen.gap.is_none());
    let text: String = snapshot
        .screen
        .cells
        .iter()
        .flatten()
        .map(|c| c.c)
        .collect();
    assert!(text.contains("W12G-TAIL"));
    let mut from = 1;
    let mut actual = Sha256::new();
    let mut bytes = 0;
    while from <= exit_sequence {
        let history = runtime
            .block_on(logic.client.history(seat, 1, from, exit_sequence))
            .unwrap();
        assert!(history.gap.is_none(), "history gap: {:?}", history.gap);
        assert!(!history.records.is_empty(), "empty history at {from}");
        for record in history.records {
            assert_eq!(record.event.sequence(), Some(from));
            from += 1;
            if let Event::Output { bytes: chunk, .. } = record.event {
                actual.update(&chunk);
                bytes += chunk.len();
            }
        }
    }
    let mut expected = Sha256::new();
    let mut block = b"0123456789abcdef".repeat(255);
    block.extend_from_slice(b"0123456789abcde\n");
    for _ in 0..16384 {
        expected.update(&block);
    }
    expected.update(b"\r\nW12G-TAIL\r\n");
    assert_eq!(bytes, 64 * 1024 * 1024 + b"\r\nW12G-TAIL\r\n".len());
    let digest = actual.finalize();
    assert_eq!(digest, expected.finalize());
    eprintln!(
        "socket backlog: requests=10000, exact_bytes={bytes}, sha256={digest:x}, exit_sequence={exit_sequence}, logic_replayed_records={logic_replayed_records}"
    );
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
