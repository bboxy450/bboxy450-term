#![allow(clippy::disallowed_methods)] // In-memory replay fixtures on the IO owner.

use super::*;
use bbt_supervisor::protocol::{
    MAX_RING_BATCH_BYTES, MAX_RING_BATCH_MEMBERS, RingRead, RingReadResult,
};
use std::sync::Mutex;

#[derive(Default)]
struct Calls {
    singleton: usize,
    batches: Vec<Vec<Ulid>>,
}

struct Synthetic(Arc<Mutex<Calls>>);

fn first_event(seat: Ulid) -> Event {
    Event::Output {
        seat,
        generation: 1,
        seq: 1,
        bytes: format!("S{seat}\r\n").into_bytes(),
    }
}
fn notification(seat: Ulid) -> Event {
    Event::Output {
        seat,
        generation: 1,
        seq: 2,
        bytes: b"tail".to_vec(),
    }
}
fn history(read: &RingRead) -> Box<crate::ring::History> {
    assert_eq!((read.from, read.to), (1, 1));
    let event = first_event(read.attachment.seat);
    let Event::Output { bytes, .. } = &event else {
        unreachable!()
    };
    Box::new(crate::ring::History {
        status: crate::ring::Status {
            generation: 1,
            first_sequence: 1,
            committed_sequence: 2,
            captured_sequence: 2,
            byte_start: 0,
            byte_end: bytes.len() as u64 + 4,
            hot_bytes: bytes.len() as u64 + 4,
            paused: false,
            recovery_notice: None,
            capture: Default::default(),
        },
        records: vec![crate::ring::IndexedEvent { event, row: None }],
        gap: None,
        next: None,
    })
}

impl SupervisorConnection for Synthetic {
    fn request(&mut self, action: Action) -> Result<Reply> {
        match action {
            Action::ListOwnedSeats => Ok(Reply::Seats { seats: vec![] }),
            Action::ReadRing {
                attachment,
                from,
                to,
            } => {
                self.0.lock().unwrap().singleton += 1;
                Ok(Reply::RingHistory {
                    history: history(&RingRead {
                        attachment,
                        from,
                        to,
                    }),
                })
            }
            Action::ReadRingBatch { reads, max_bytes } => {
                assert!(!reads.is_empty() && reads.len() <= MAX_RING_BATCH_MEMBERS);
                assert!(max_bytes as usize <= MAX_RING_BATCH_BYTES);
                self.0
                    .lock()
                    .unwrap()
                    .batches
                    .push(reads.iter().map(|r| r.attachment.seat).collect());
                Ok(Reply::RingHistoryBatch {
                    results: reads
                        .into_iter()
                        .map(|read| RingReadResult {
                            seat: read.attachment.seat,
                            generation: read.attachment.generation,
                            from: read.from,
                            to: read.to,
                            result: Ok(history(&read)),
                        })
                        .collect(),
                })
            }
            Action::PublishCheckpoint { checkpoint, .. } => Ok(Reply::CheckpointCommitted {
                sequence: RingSaved::decode(&checkpoint)?.sequence,
            }),
            other => Err(invalid(format!("unexpected synthetic action: {other:?}"))),
        }
    }
    fn receive(&mut self, _: Duration) -> Result<Option<Event>> {
        Ok(None)
    }
}

fn worker(
    root: &std::path::Path,
    runtime: &tokio::runtime::Runtime,
    calls: Arc<Mutex<Calls>>,
) -> Worker {
    let config = LogicConfig {
        run_dir: root.join("run"),
        repository: root.join("repo"),
        ring: Default::default(),
    };
    std::fs::create_dir_all(config.repository.join(".bboxy")).unwrap();
    let key = SigningKey::from_bytes(&[56; 32]);
    let identity = crate::identity::IdentityRuntime::open(
        &config.run_dir,
        key.clone(),
        runtime.handle().clone(),
    )
    .unwrap();
    let bus = BusHandle::start(
        runtime.handle(),
        Broker::open(&config.run_dir, key).unwrap(),
        128,
    )
    .unwrap();
    let (tx, _rx) = mpsc::channel(128);
    let client = LogicClient {
        tx,
        bus: bus.clone(),
        exit_cause: Arc::new(OnceLock::new()),
        capture_diagnostics: Arc::new(Recorder::default()),
        gui_peers: Arc::default(),
        captures: Arc::default(),
        gui_watch_epoch: Arc::default(),
    };
    Worker::open(
        vec![Box::new(LocalProvider::new(Synthetic(calls)))],
        config,
        bus,
        runtime.handle().clone(),
        client,
        identity,
        None,
    )
    .unwrap()
}

fn add_view(worker: &mut Worker, id: Ulid) {
    let size = bbt_supervisor::SeatSize {
        cols: 10,
        rows: 2,
        ..Default::default()
    };
    let mut state = Playback::new(id, 1, size);
    let tap = OscTap::install(&mut state.term);
    let record = SeatRecord::new(id, Ulid::from(999_999u128), "synthetic", "zsh");
    worker.registry.insert(&record).unwrap();
    let adapter = Session::new(
        HarnessRecord::builtin("zsh").unwrap(),
        Binding {
            seat: id,
            generation: 1,
            pid: 1_000_000,
            process_start: "synthetic".into(),
            session: format!("synthetic-{id}"),
            attachment_generation: 1,
            runtime: None,
        },
    )
    .unwrap();
    worker.attachments.insert(
        id,
        Attachment {
            seat: id,
            generation: 1,
            writer_lease: Ulid::from(7u128),
        },
    );
    worker.views.insert(
        id,
        ViewState {
            info: SeatInfo {
                seat: id,
                generation: 1,
                pid: 1_000_000,
                size,
                next_sequence: 1,
                exited: None,
                companion: None,
            },
            adapter,
            delivery_endpoint: crate::adapters::Endpoint::None,
            delivery_failure: None,
            tap,
            replies: VecDeque::new(),
            native_reader: None,
            state,
            view_generation: Ulid::new(),
            checkpoint_bytes: 0,
            checkpoint_sequence: 0,
            changed_at: Age::default(),
            last_parsed_at: Instant::now(),
            last_arrival_at: Instant::now(),
            captured_sequence: 0,
            checkpoint_pending: None,
            checkpoint_requested: false,
            checkpoint_paused: false,
            checkpoint_pauses: 0,
            idle_checkpoints: 0,
            forced_checkpoints: 0,
            checkpoint_retry_at: Instant::now(),
        },
    );
}

#[test]
fn ring_reads_are_batched() {
    const SEATS: usize = 967;
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let calls = Arc::new(Mutex::new(Calls::default()));
    let mut worker = worker(root.path(), &runtime, calls.clone());
    let mut pending = PendingSeats::default();
    for n in 1..=SEATS {
        let id = Ulid::from(n as u128);
        add_view(&mut worker, id);
        pending.push(HostId::Local, notification(id), 1, 1).unwrap();
    }
    pending.set_watched((1..=50).map(|n| (Ulid::from(n as u128), 1)).collect());
    for _ in 0..SEATS * 2 {
        if pending.is_empty() {
            break;
        }
        let before: u64 = worker.views.values().map(|v| v.sequence).sum();
        let mut progress = false;
        worker.ingest_one(&mut pending, &mut progress).unwrap();
        let after: u64 = worker.views.values().map(|v| v.sequence).sum();
        assert!(
            after - before <= 2,
            "one parsed seat page/notification per turn"
        );
    }
    assert!(pending.is_empty(), "all deterministic pages must advance");
    for (&id, view) in &worker.views {
        let mut serial = Playback::new(id, 1, view.info.size);
        serial.apply(&first_event(id)).unwrap();
        serial.apply(&notification(id)).unwrap();
        assert_eq!(view.sequence, serial.sequence, "{id}");
        assert_eq!(view.byte_end, serial.byte_end, "{id}");
        assert_eq!(
            Screen::from_term(
                &view.term,
                view.title.clone(),
                view.cwd.clone(),
                view.gap.clone()
            ),
            Screen::from_term(
                &serial.term,
                serial.title.clone(),
                serial.cwd.clone(),
                serial.gap.clone()
            ),
            "{id}"
        );
    }
    let calls = calls.lock().unwrap();
    assert_eq!(
        calls.singleton, 0,
        "hot sweep must not issue singleton ReadRing"
    );
    assert!(
        calls.batches.len() <= SEATS.div_ceil(MAX_RING_BATCH_MEMBERS),
        "{} batch requests",
        calls.batches.len()
    );
    assert_eq!(calls.batches.iter().map(Vec::len).sum::<usize>(), SEATS);
}

#[test]
fn indexed_prefetch_order_matches_the_actual_global_lanes() {
    for mask in 0u16..256 {
        for streak in 0..=PendingSeats::WATCHED_QUANTA {
            let make = || {
                let mut p = PendingSeats::default();
                for n in 0..8 {
                    let id = Ulid::from(n as u128 + 1);
                    let host = if n % 2 == 0 {
                        HostId::Local
                    } else {
                        HostId::Ssh {
                            host: "remote".into(),
                        }
                    };
                    p.push(host, notification(id), 1, 1).unwrap();
                }
                p.set_watched(
                    (0..8)
                        .filter(|n| mask & (1 << n) != 0)
                        .map(|n| (Ulid::from(n as u128 + 1), 1))
                        .collect(),
                );
                p.watched_streak = streak;
                p
            };
            for host in [
                HostId::Local,
                HostId::Ssh {
                    host: "remote".into(),
                },
            ] {
                let actual = make().ring_candidates(&host, MAX_RING_BATCH_MEMBERS);
                let mut reference = make();
                let mut expected = Vec::new();
                while let Some((key, _)) = reference.next() {
                    if key.0 == host {
                        expected.push(key.1);
                    }
                    reference.finish(key, true);
                }
                assert_eq!(
                    actual, expected,
                    "mask={mask} streak={streak} host={host:?}"
                );
            }
        }
    }
}
