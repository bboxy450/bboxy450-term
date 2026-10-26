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
    refusals: BTreeMap<Ulid, crate::FailureCode>,
    reads: Vec<RingRead>,
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
                self.0.lock().unwrap().reads.extend(reads.iter().cloned());
                Ok(Reply::RingHistoryBatch {
                    results: reads
                        .into_iter()
                        .map(|read| RingReadResult {
                            seat: read.attachment.seat,
                            generation: read.attachment.generation,
                            from: read.from,
                            to: read.to,
                            result: match self
                                .0
                                .lock()
                                .unwrap()
                                .refusals
                                .get(&read.attachment.seat)
                                .copied()
                            {
                                Some(code) => Err(crate::Failure {
                                    code,
                                    message: "synthetic member refusal".into(),
                                }),
                                None => Ok(history(&read)),
                            },
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

fn remote() -> HostId {
    HostId::Ssh {
        host: "synthetic-remote".into(),
    }
}
struct Remote(Arc<Mutex<Calls>>);
impl Provider for Remote {
    fn id(&self) -> String {
        "synthetic-remote".into()
    }
    fn kind(&self) -> crate::provider::ProviderKind {
        crate::provider::ProviderKind::Ssh
    }
    fn hosts(&self) -> Vec<crate::provider::Host> {
        vec![crate::provider::Host {
            id: remote(),
            provider: self.id(),
            label: "synthetic remote".into(),
            region: None,
            state: crate::provider::LinkState::Connected { rtt_ms: 0 },
        }]
    }
    fn link(&mut self, host: HostId) -> Result<Box<dyn SupervisorConnection>> {
        assert_eq!(host, remote());
        Ok(Box::new(Synthetic(self.0.clone())))
    }
    fn poll(&mut self) -> Vec<crate::provider::ProviderEvent> {
        vec![]
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
        window_journal: None,
    };
    Worker::open(
        vec![
            Box::new(LocalProvider::new(Synthetic(calls.clone()))),
            Box::new(Remote(calls)),
        ],
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
            ring_refusal: None,
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

// keeps: invariant ring-reads-batched
#[test]
fn ring_reads_are_batched() {
    sweep(false);
}

// keeps: invariant ring-reads-batched
#[test]
fn ring_reads_are_batched_per_host() {
    sweep(true);
}

fn sweep(two_hosts: bool) {
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
        let host = if two_hosts && n % 2 == 0 {
            worker
                .registry
                .update(id, |r| {
                    r.location = crate::registry::Location::Ssh {
                        host: "synthetic-remote".into(),
                    };
                    Ok(())
                })
                .unwrap();
            remote()
        } else {
            HostId::Local
        };
        pending.push(host, notification(id), 1, 1).unwrap();
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
    let bound = if two_hosts {
        (SEATS / 2).div_ceil(MAX_RING_BATCH_MEMBERS)
            + SEATS.div_ceil(2).div_ceil(MAX_RING_BATCH_MEMBERS)
    } else {
        SEATS.div_ceil(MAX_RING_BATCH_MEMBERS)
    };
    if two_hosts {
        assert!(
            calls
                .batches
                .iter()
                .all(|batch| batch.iter().all(|id| id.0 % 2 == batch[0].0 % 2)),
            "batch crossed host boundary"
        );
    }
    println!(
        "seats={SEATS} hosts={} rpc={} bound={bound} singleton={}",
        if two_hosts { 2 } else { 1 },
        calls.batches.len(),
        calls.singleton
    );
    assert!(
        calls.batches.len() <= bound,
        "{} batch requests",
        calls.batches.len()
    );
    assert_eq!(calls.batches.iter().map(Vec::len).sum::<usize>(), SEATS);
}

// keeps: spec mux-daemon
#[test]
fn indexed_prefetch_order_matches_the_actual_global_lanes() {
    for mask in 0u16..256 {
        for streak in 0..=PendingSeats::WATCHED_QUANTA {
            for parked in 0..3 {
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
                        p.push(host, notification(id), 1, if n % 3 == 0 { 2 } else { 1 })
                            .unwrap();
                    }
                    p.set_watched(
                        (0..8)
                            .filter(|n| mask & (1 << n) != 0)
                            .map(|n| (Ulid::from(n as u128 + 1), 1))
                            .collect(),
                    );
                    p.watched_streak = streak;
                    if parked == 1 {
                        p.park(&HostId::Local);
                    }
                    if parked == 2 {
                        p.park(&HostId::Ssh {
                            host: "remote".into(),
                        });
                    }
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
                        if key.0 == host && reference.seats[&key].needs_ring() {
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
}

// keeps: spec mux-daemon
#[test]
fn refused_members_keep_their_cursors_while_other_members_finish() {
    use crate::FailureCode::{Busy, Limit, NotFound, StaleGeneration};
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let calls = Arc::new(Mutex::new(Calls::default()));
    let mut worker = worker(root.path(), &runtime, calls.clone());
    let failures = [
        (1u128, Busy),
        (2, NotFound),
        (3, StaleGeneration),
        (4, Limit),
    ];
    calls
        .lock()
        .unwrap()
        .refusals
        .extend(failures.map(|(n, code)| (Ulid::from(n), code)));
    let mut pending = PendingSeats::default();
    for n in 1u128..=16 {
        let id = Ulid::from(n);
        add_view(&mut worker, id);
        pending.push(HostId::Local, notification(id), 1, 1).unwrap();
    }
    for _ in 0..64 {
        worker.ingest_one(&mut pending, &mut false).unwrap();
    }
    for n in 5u128..=16 {
        assert_eq!(worker.views[&Ulid::from(n)].sequence, 2);
    }
    for (n, code) in failures {
        let view = &worker.views[&Ulid::from(n)];
        assert_eq!(view.sequence, 0, "refused cursor moved: {code:?}");
        assert_eq!(
            view.ring_refusal.as_ref().map(|f| f.code),
            if code == Busy { None } else { Some(code) }
        );
    }
    let refused_reads: Vec<_> = (2u128..=4)
        .map(|n| {
            calls
                .lock()
                .unwrap()
                .reads
                .iter()
                .filter(|r| r.attachment.seat == Ulid::from(n))
                .count()
        })
        .collect();
    assert_eq!(
        refused_reads,
        vec![1; 3],
        "definitive refusals were retried"
    );
    calls.lock().unwrap().refusals.remove(&Ulid::from(1u128));
    for _ in 0..16 {
        worker.ingest_one(&mut pending, &mut false).unwrap();
    }
    assert_eq!(
        worker.views[&Ulid::from(1u128)].sequence,
        2,
        "Busy did not recover"
    );
    let calls = calls.lock().unwrap();
    assert!(
        calls
            .reads
            .iter()
            .filter(|r| r.attachment.seat == Ulid::from(1u128))
            .all(|r| (r.from, r.to) == (1, 1)),
        "retry changed the pending target"
    );
    for n in 2u128..=4 {
        assert_eq!(worker.views[&Ulid::from(n)].sequence, 0);
        assert_eq!(
            calls
                .reads
                .iter()
                .filter(|r| r.attachment.seat == Ulid::from(n))
                .count(),
            1
        );
    }
}

// keeps: spec mux-daemon
#[test]
fn cached_page_preserves_unapplied_boundary_rows_and_rejects_other_generations() {
    let id = Ulid::from(1u128);
    let read = RingRead {
        attachment: Attachment {
            seat: id,
            generation: 1,
            writer_lease: Ulid::from(7u128),
        },
        from: 1,
        to: 2,
    };
    let mut data = history(&RingRead {
        to: 1,
        ..read.clone()
    });
    data.records.push(crate::ring::IndexedEvent {
        event: notification(id),
        row: None,
    });
    let cached = RingPage {
        read: read.clone(),
        result: Ok(data),
    };
    let first = cached
        .page(&RingRead {
            to: 1,
            ..read.clone()
        })
        .unwrap()
        .unwrap();
    assert_eq!(first.records.len(), 1);
    let tail = cached
        .page(&RingRead {
            from: 2,
            ..read.clone()
        })
        .unwrap()
        .unwrap();
    assert_eq!(tail.records[0].event, notification(id));
    let mut stale = read.clone();
    stale.attachment.generation += 1;
    assert!(cached.page(&stale).is_none());
    let mut writer = read;
    writer.attachment.writer_lease = Ulid::from(8u128);
    assert!(cached.page(&writer).is_none());
}
