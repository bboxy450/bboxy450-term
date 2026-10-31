// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
    hold: Option<Ulid>,
    held: Vec<(
        RingRead,
        std::sync::mpsc::SyncSender<std::result::Result<Reply, crate::Failure>>,
    )>,
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
    fn start_capture(&mut self, action: Action) -> Result<bbt_supervisor::client::PendingCapture> {
        if let Action::ReadRing {
            attachment,
            from,
            to,
        } = &action
            && self.0.lock().unwrap().hold == Some(attachment.seat)
        {
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            self.0.lock().unwrap().held.push((
                RingRead {
                    attachment: attachment.clone(),
                    from: *from,
                    to: *to,
                },
                tx,
            ));
            return Ok(bbt_supervisor::client::PendingCapture::new(rx));
        }
        if let Action::ReadRingBatch { reads, .. } = &action
            && reads.len() == 1
            && self.0.lock().unwrap().hold == Some(reads[0].attachment.seat)
        {
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            self.0.lock().unwrap().held.push((reads[0].clone(), tx));
            return Ok(bbt_supervisor::client::PendingCapture::new(rx));
        }
        Ok(bbt_supervisor::client::PendingCapture::ready(
            self.request(action),
        ))
    }
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
                if reads
                    .iter()
                    .any(|read| self.0.lock().unwrap().hold == Some(read.attachment.seat))
                {
                    return Err(invalid("serial ingestion waited on a held seat"));
                }
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
        attention_observer_guard: Arc::default(),
        current_attention: Arc::default(),
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
            ring_retry_at: Instant::now(),
            tap,
            replies: VecDeque::new(),
            native_reader: None,
            shared_codex_runtime: None,
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
fn ring_reads_progress_independently() {
    sweep(false);
}

// keeps: invariant ring-reads-batched
#[test]
fn ring_reads_progress_independently_per_host() {
    sweep(true);
}

// keeps: invariant ring-reads-batched — #47 supersedes the aggregate barrier
#[test]
fn held_seat_does_not_block_other_seat() {
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let calls = Arc::new(Mutex::new(Calls::default()));
    let mut worker = worker(root.path(), &runtime, calls.clone());
    let mut pending = PendingSeats::default();
    for n in 1u128..=2 {
        let id = Ulid::from(n);
        add_view(&mut worker, id);
        pending.push(HostId::Local, notification(id), 1, 1).unwrap();
    }
    calls.lock().unwrap().hold = Some(Ulid::from(1u128));
    let until = Instant::now() + Duration::from_secs(2);
    while worker.views[&Ulid::from(2u128)].sequence < 2 {
        worker.ingest_one(&mut pending, &mut false).unwrap();
        assert!(Instant::now() < until, "unrelated seat did not progress");
        std::thread::yield_now();
    }
    assert_eq!(worker.views[&Ulid::from(1u128)].sequence, 0);
    assert_eq!(
        calls.lock().unwrap().held.len(),
        1,
        "one pending read per seat"
    );
    calls.lock().unwrap().held[0]
        .1
        .try_send(Err(crate::Failure {
            code: crate::FailureCode::Io,
            message: "one seat read expired".into(),
        }))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(2);
    while worker.views[&Ulid::from(1u128)].ring_refusal.is_none() {
        worker.ingest_one(&mut pending, &mut false).unwrap();
        assert!(Instant::now() < until);
    }
    assert_eq!(worker.views[&Ulid::from(1u128)].sequence, 0);
    assert_eq!(worker.views[&Ulid::from(2u128)].sequence, 2);
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
    let until = Instant::now() + Duration::from_secs(15);
    while !pending.is_empty() {
        assert!(
            Instant::now() < until,
            "all independent seat readers must advance"
        );
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
    let bound = SEATS;
    assert!(
        calls.batches.iter().all(|batch| batch.len() == 1),
        "a seat completion must not wait for a batch peer"
    );
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
    let until = Instant::now() + Duration::from_secs(3);
    while (5u128..=16).any(|n| worker.views[&Ulid::from(n)].sequence != 2)
        || (2u128..=4).any(|n| worker.views[&Ulid::from(n)].ring_refusal.is_none())
    {
        worker.ingest_one(&mut pending, &mut false).unwrap();
        assert!(Instant::now() < until, "healthy members did not finish");
        std::thread::yield_now();
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
    let until = Instant::now() + Duration::from_secs(3);
    while worker.views[&Ulid::from(1u128)].sequence != 2 {
        worker.ingest_one(&mut pending, &mut false).unwrap();
        assert!(Instant::now() < until, "Busy did not recover");
        std::thread::yield_now();
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

// keeps: spec mux-daemon — history and live output have independent completion slots
#[test]
fn held_history_is_bounded_and_does_not_block_live_echo() {
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let calls = Arc::new(Mutex::new(Calls::default()));
    let mut worker = worker(root.path(), &runtime, calls.clone());
    for n in 1u128..=2 {
        add_view(&mut worker, Ulid::from(n));
    }
    calls.lock().unwrap().hold = Some(Ulid::from(1u128));
    let (tx, mut rx) = oneshot::channel();
    worker.start_history(
        Ulid::from(1u128),
        1,
        1,
        1,
        history::Destination::Gui {
            masks: false,
            reply: tx,
        },
    );
    let (tx, mut refused) = oneshot::channel();
    worker.start_history(
        Ulid::from(1u128),
        1,
        1,
        1,
        history::Destination::Gui {
            masks: false,
            reply: tx,
        },
    );
    assert_eq!(
        refused
            .try_recv()
            .unwrap()
            .unwrap_err()
            .downcast_ref::<crate::Failure>()
            .unwrap()
            .code,
        crate::FailureCode::Busy
    );
    let mut pending = PendingSeats::default();
    let id = Ulid::from(2u128);
    pending.push(HostId::Local, notification(id), 1, 1).unwrap();
    let until = Instant::now() + Duration::from_secs(2);
    while worker.views[&id].sequence < 2 {
        worker.poll_history();
        worker.ingest_one(&mut pending, &mut false).unwrap();
        assert!(
            Instant::now() < until,
            "live echo waited for unrelated history"
        );
        std::thread::yield_now();
    }
    assert!(matches!(
        rx.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    assert_eq!(worker.history_reads.len(), 1);
    assert_eq!(worker.views[&Ulid::from(1u128)].sequence, 0);
}

// keeps: spec mux-daemon — generation and owner incarnation fence
#[test]
fn parser_completion_cannot_cross_owner_replacement() {
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let calls = Arc::new(Mutex::new(Calls::default()));
    let mut worker = worker(root.path(), &runtime, calls);
    let id = Ulid::from(1u128);
    add_view(&mut worker, id);
    ingestion::Parsers::start(&mut worker, id, vec![first_event(id)], true);
    worker.views.get_mut(&id).unwrap().view_generation = Ulid::new();
    assert!(
        worker
            .parsers
            .take(&worker.views[&id], &worker.attachments[&id], 1, true)
            .is_err()
    );
    assert_eq!(worker.views[&id].sequence, 0);
    assert!(!worker.parsers.contains(id));
    // Same seat/process labels may return, but the owner incarnation is fresh.
    worker.views.get_mut(&id).unwrap().view_generation = Ulid::new();
    assert!(!worker.parsers.contains(id));
}

// keeps: spec mux-daemon — admitted asynchronous work cannot be mistaken for idle
#[test]
fn new_read_and_parser_admission_are_progress_before_cursor_publication() {
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let calls = Arc::new(Mutex::new(Calls::default()));
    let mut worker = worker(root.path(), &runtime, calls.clone());
    let id = Ulid::from(1u128);
    add_view(&mut worker, id);
    calls.lock().unwrap().hold = Some(id);
    let mut pending = PendingSeats::default();
    pending.push(HostId::Local, notification(id), 1, 1).unwrap();
    let mut progress = false;
    worker.ingest_one(&mut pending, &mut progress).unwrap();
    assert!(progress, "a new per-seat request is not an idle turn");
    assert_eq!(worker.views[&id].sequence, 0);
    let mut unchanged = false;
    worker.ingest_one(&mut pending, &mut unchanged).unwrap();
    assert!(!unchanged, "polling the same held request must not spin");
}
