// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Bounded synthetic PTY/socket IO fixture.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use bbt_mux::logic::{Logic, LogicConfig, SocketSupervisor, SupervisorConnection};
use bbt_mux::registry::SeatRecord;
use bbt_mux::{Action, Config, Event, LaunchSpec, Reply, Supervisor, Ulid};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use tokio::sync::{Notify, watch};

const NO_PROGRESS_TIMEOUT: Duration = Duration::from_secs(90);

struct RequestFlood {
    client: SocketSupervisor,
    gate: std::fs::File,
    armed: Arc<AtomicBool>,
    requests: Arc<AtomicU64>,
    replayed: Arc<AtomicU64>,
    replayed_through: Arc<AtomicU64>,
    exit: Arc<AtomicU64>,
    progress: watch::Sender<(u64, u64, u64)>,
    turns: Arc<Notify>,
    observation_turns: usize,
}
impl RequestFlood {
    fn progressed(&self) {
        let current = (
            self.requests.load(Ordering::Acquire),
            self.replayed_through.load(Ordering::Acquire),
            self.exit.load(Ordering::Acquire),
        );
        self.progress.send_if_modified(|previous| {
            if *previous == current {
                return false;
            }
            *previous = current;
            true
        });
    }
}
impl SupervisorConnection for RequestFlood {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        let kind = std::mem::discriminant(&action);
        let reply = self.client.request(action).inspect_err(|error| {
            eprintln!("socket backlog supervisor request {kind:?} failed: {error:?}");
        })?;
        let histories: Vec<_> = match &reply {
            Reply::RingHistory { history } => vec![history.as_ref()],
            Reply::RingHistoryBatch { results } => results
                .iter()
                .filter_map(|row| row.result.as_deref().ok())
                .collect(),
            _ => vec![],
        };
        for history in histories {
            self.replayed
                .fetch_add(history.records.len() as u64, Ordering::Release);
            if let Some(sequence) = history.records.last().and_then(|r| r.event.sequence()) {
                self.replayed_through.fetch_max(sequence, Ordering::Release);
            }
        }
        self.progressed();
        Ok(reply)
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        if self.armed.swap(false, Ordering::AcqRel) {
            self.gate.write_all(b"go")?;
            for _ in 0..10_000 {
                self.client.request(Action::ListOwnedSeats)?;
                self.requests.fetch_add(1, Ordering::Release);
                self.progressed();
            }
        }
        // This fixture has one producer. Its Exited event is the final record;
        // subsequent replay/history requests cannot receive another notification.
        // An exhausted fixture source need not spend 10 ms on each empty read.
        let event = if self.exit.load(Ordering::Acquire) == 0 {
            self.client.receive(timeout).inspect_err(|error| {
                eprintln!("socket backlog supervisor receive {timeout:?} failed: {error:?}");
            })?
        } else {
            None
        };
        if let Some(Event::Exited { seq, .. }) = &event {
            self.exit.store(*seq, Ordering::Release);
        }
        self.progressed();
        // Coalesce observer work by logical turns, not a real-clock poll.
        // The authoritative status/Screen checks decide completion; a fetched
        // ring cursor cannot, since the last notifications may apply live.
        self.observation_turns += 1;
        if self.observation_turns == 32 {
            self.observation_turns = 0;
            self.turns.notify_one();
        }
        Ok(event)
    }
}

// keeps: invariant no-chokepoints (capture/replay continues during a request flood)
// keeps: spec mux-daemon §Acceptance and remaining decisions (ordered ring replay)
#[test]
#[ignore = "night-only real socket boundary; scripts/night-test-boundaries.sh"]
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
    let replayed_through = Arc::new(AtomicU64::new(0));
    let exit = Arc::new(AtomicU64::new(0));
    let gate = root.path().join("start");
    let fifo = std::ffi::CString::new(gate.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: the terminated path belongs to this private fixture directory.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    use std::os::unix::fs::OpenOptionsExt;
    let gate_writer = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&gate)
        .unwrap();
    let (progress, mut watchdog_progress) = watch::channel((0, 0, 0));
    let turns = Arc::new(Notify::new());
    let connection = RequestFlood {
        client: SocketSupervisor::connect(&socket, supervisor.socket_capability()).unwrap(),
        gate: gate_writer,
        armed: armed.clone(),
        requests: requests.clone(),
        replayed: replayed.clone(),
        replayed_through: replayed_through.clone(),
        exit: exit.clone(),
        progress,
        turns: turns.clone(),
        observation_turns: 0,
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
import os, sys, tty
tty.setraw(1)
with open(sys.argv[1], 'rb', buffering=0) as start:
    assert start.read(2) == b'go'
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
    let observation = runtime.block_on(async {
        let completion = async {
            loop {
                let status = logic.client.adapter_status(seat, 1).await?;
                let exit_sequence = exit.load(Ordering::Acquire);
                if status.source == "exit" && exit_sequence != 0 {
                    assert_eq!(status.detail.as_deref(), Some("exit 0"));
                    let snapshot = logic.client.snapshot(seat, 1).await?;
                    if snapshot.sequence == exit_sequence {
                        break Ok::<_, bbt_mux::store::Error>(snapshot);
                    }
                }
                turns.notified().await;
            }
        };
        // This is an ordering/exact-replay regression, not a throughput gate.
        // A loaded O0 build can spend most of the former total 90 s allowance
        // on the request flood alone. Keep the same allowance as a no-progress
        // watchdog, notified independently while a Logic RPC is outstanding.
        let watchdog = async {
            loop {
                if !matches!(
                    tokio::time::timeout(NO_PROGRESS_TIMEOUT, watchdog_progress.changed()).await,
                    Ok(Ok(()))
                ) {
                    break;
                }
            }
        };
        tokio::select! {
            biased;
            result = completion => Some(result),
            () = watchdog => None,
        }
    });
    if !matches!(&observation, Some(Ok(_))) {
        // Drop otherwise discards the actor's actual error and leaves only
        // "logic stopped". Preserve it without retrying the failed fixture.
        let actor_exit = logic.shutdown();
        panic!(
            "socket backlog failure (None means no progress or progress source closed): observation={observation:?}, actor_exit={actor_exit:?}, requests={}, replayed={}, replayed_through={}, exit_sequence={}",
            requests.load(Ordering::Acquire),
            replayed.load(Ordering::Acquire),
            replayed_through.load(Ordering::Acquire),
            exit.load(Ordering::Acquire),
        );
    }
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
    let snapshot = observation.unwrap().unwrap();
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
