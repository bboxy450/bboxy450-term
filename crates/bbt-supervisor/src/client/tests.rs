// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use crate::protocol::ExitInfo;
use crate::transport::{read_frame, write_frame};
use crate::{Attachment, Request, SeatInfo, SeatSize};
use std::os::unix::net::UnixListener;

fn output(seat: Ulid, generation: u64, seq: u64, byte: u8) -> Event {
    Event::Output {
        seat,
        generation,
        seq,
        bytes: vec![byte],
    }
}

// keeps: spec mux-daemon.md — typed provider refusal and reconnect recovery
#[test]
fn compatibility_capture_preserves_the_typed_admission_failure() {
    struct Down;
    impl SupervisorConnection for Down {
        fn request(&mut self, _: Action) -> Result<Reply> {
            Err(std::io::Error::other("provider down fixture").into())
        }
        fn receive(&mut self, _: Duration) -> Result<Option<Event>> {
            Ok(None)
        }
    }
    let error = match Down.start_capture(Action::RingRecovery {
        attachment: Attachment {
            seat: Ulid::from(1u128),
            generation: 1,
            writer_lease: Ulid::from(2u128),
        },
    }) {
        Ok(_) => panic!("provider refusal was converted into an admitted capture"),
        Err(error) => error,
    };
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().to_string(),
        "provider down fixture"
    );
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[test]
fn diagnostic_queue_observation_keeps_the_same_event_order() {
    let (mut client, _peer) = Memory::client();
    let first = Ulid::new();
    let second = Ulid::new();
    client.events.confirm_ring(first, 1, 1).unwrap();
    client.events.push(output(first, 1, 3, b'a')).unwrap();
    client.events.push(output(second, 1, 1, b'b')).unwrap();
    for _ in 0..2 {
        let observed = client.diagnostics(first, 1);
        assert_eq!(observed.queued_events, 2);
        assert_eq!(observed.queued_raw_events, 1);
        assert_eq!(observed.queued_output_sequence, Some(3));
        assert_eq!(client.diagnostics(first, 2).queued_output_sequence, None);
    }
    assert_eq!(
        client.receive(Duration::ZERO).unwrap(),
        Some(output(first, 1, 3, b'a'))
    );
    let observed = client.diagnostics(first, 1);
    assert_eq!(observed.queued_events, 1);
    assert_eq!(observed.queued_output_sequence, None);
    assert_eq!(observed.receive_calls, 1);
    assert_eq!(observed.returned_events, 1);
    assert_eq!(
        client.receive(Duration::ZERO).unwrap(),
        Some(output(second, 1, 1, b'b'))
    );
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn a_peer_closed_after_its_final_reply_does_not_hide_buffered_bytes_behind_timeout_setup() {
    let (stream, mut peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let expected = output(Ulid::new(), 1, 1, b'x');
    write_frame(
        &mut peer,
        &Frame::new(Message::Event {
            event: expected.clone(),
        }),
    )
    .unwrap();
    drop(peer); // deterministic peer close, before the client configures its read
    let mut client = SocketSupervisor {
        stream: stream.into(),
        decoder: Default::default(),
        events: Default::default(),
        replies: Default::default(),
        lease: None,
        pending_request: None,
        captures: BTreeMap::new(),
        capture_writes: Default::default(),
        broken: false,
        diagnostics: ClientDiagnostics::default(),
    };
    assert_eq!(
        client.receive(Duration::from_secs(30)).unwrap(),
        Some(expected)
    );
    let error = client.receive(Duration::from_secs(30)).unwrap_err();
    assert_eq!(
        error.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::InvalidData
    );
}

#[derive(Clone)]
pub(super) struct Memory(std::sync::Arc<std::sync::Mutex<MemoryState>>);
struct MemoryState {
    now: Instant,
    timeout: Duration,
    incoming: std::collections::VecDeque<u8>,
    held: Option<Ulid>,
    released: bool,
    closed: bool,
    requests: usize,
}
impl Memory {
    fn client() -> (SocketSupervisor, Self) {
        let memory = Self(std::sync::Arc::new(std::sync::Mutex::new(MemoryState {
            now: Instant::now(),
            timeout: Duration::ZERO,
            incoming: Default::default(),
            held: None,
            released: false,
            closed: false,
            requests: 0,
        })));
        let client = SocketSupervisor {
            stream: Wire::Memory(memory.clone()),
            decoder: Default::default(),
            events: Default::default(),
            replies: Default::default(),
            lease: None,
            pending_request: None,
            captures: BTreeMap::new(),
            capture_writes: Default::default(),
            broken: false,
            diagnostics: Default::default(),
        };
        (client, memory)
    }
    pub(super) fn now(&self) -> Instant {
        self.0.lock().unwrap().now
    }
    pub(super) fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.0.lock().unwrap().timeout = timeout.unwrap_or(Duration::ZERO);
        Ok(())
    }
    pub(super) fn shutdown(&self) -> std::io::Result<()> {
        self.0.lock().unwrap().closed = true;
        Ok(())
    }
    fn release(&self) {
        let mut state = self.0.lock().unwrap();
        state.released = true;
        if let Some(request_id) = state.held.take() {
            let bytes = crate::transport::encode(&Frame::new(Message::Reply {
                request_id,
                result: Ok(Reply::Seats { seats: vec![] }),
            }))
            .unwrap();
            state.incoming.extend(bytes);
        }
    }
}
impl Read for Memory {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        let mut state = self.0.lock().unwrap();
        if state.closed {
            return Ok(0);
        }
        if state.incoming.is_empty() {
            let timeout = state.timeout;
            state.now += timeout;
            return Err(std::io::ErrorKind::WouldBlock.into());
        }
        let count = bytes.len().min(state.incoming.len());
        for byte in &mut bytes[..count] {
            *byte = state.incoming.pop_front().unwrap();
        }
        Ok(count)
    }
}
impl Write for Memory {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let request = read_frame::<Request>(&mut std::io::Cursor::new(bytes))?.body;
        if !matches!(request.action, Action::ListOwnedSeats) {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        let mut state = self.0.lock().unwrap();
        state.requests += 1;
        if state.released {
            state
                .incoming
                .extend(crate::transport::encode(&Frame::new(Message::Reply {
                    request_id: request.request_id,
                    result: Ok(Reply::Seats { seats: vec![] }),
                }))?);
        } else {
            state.held = Some(request.request_id);
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// keeps: spec mux-daemon.md — Supervisor ↔ logic protocol — an ambiguous request fences writes until its original reply.
#[test]
fn a_timed_out_request_blocks_new_writes_until_its_late_reply_is_drained() {
    let (mut client, peer) = Memory::client();
    let started = peer.now();
    client.request(Action::ListOwnedSeats).unwrap_err();
    assert_eq!(peer.now() - started, REQUEST_TIMEOUT); // virtual clock, no wait
    let held = client.request(Action::ListOwnedSeats).unwrap_err();
    assert!(
        held.downcast_ref::<Failure>()
            .is_some_and(|failure| failure.code == crate::FailureCode::Busy)
    );
    peer.release();
    assert_eq!(client.receive(Duration::ZERO).unwrap(), None);
    assert!(
        matches!(client.request(Action::ListOwnedSeats).unwrap(), Reply::Seats { seats } if seats.is_empty())
    );
    assert_eq!(peer.0.lock().unwrap().requests, 2); // remote wire saw no refused write
}

// keeps: spec mux-daemon.md — Supervisor ↔ logic protocol — unresolved requests disconnect at their original recovery deadline.
#[test]
fn an_unanswered_request_becomes_disconnected_at_the_bounded_recovery_deadline() {
    let (mut client, peer) = Memory::client();
    let started = peer.now();
    client.request(Action::ListOwnedSeats).unwrap_err();
    client.receive(Duration::from_secs(60)).unwrap_err();
    assert_eq!(peer.now() - started, REQUEST_TIMEOUT * 2);
    client.request(Action::ListOwnedSeats).unwrap_err();
    assert_eq!(peer.0.lock().unwrap().requests, 1); // disconnect cannot admit another request
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[test]
fn pre_ring_bytes_remain_fifo_and_confirmation_covers_only_its_generation() {
    let seat = Ulid::new();
    let mut events = PendingEvents::default();
    events.push(output(seat, 1, 1, b'a')).unwrap();
    events.push(output(seat, 1, 2, b'b')).unwrap();
    // A captured event can precede its successful AttachRing reply in a read.
    events.push(output(seat, 1, 3, b'c')).unwrap();
    events.push(output(seat, 2, 1, b'd')).unwrap();
    events.confirm_ring(seat, 1, 3).unwrap();
    events.push(output(seat, 1, 4, b'e')).unwrap();
    events.push(output(seat, 2, 2, b'f')).unwrap();
    let actual: Vec<_> = std::iter::from_fn(|| events.pop_front()).collect();
    assert_eq!(
        actual,
        vec![
            output(seat, 1, 1, b'a'),
            output(seat, 1, 2, b'b'),
            output(seat, 2, 1, b'd'),
            output(seat, 1, 4, b'e'),
            output(seat, 2, 2, b'f'),
        ]
    );
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[test]
fn raw_fifo_keeps_its_named_limit_and_captured_slots_do_not_spend_it() {
    let raw = Ulid::new();
    let captured = Ulid::new();
    let mut events = PendingEvents::default();
    events.confirm_ring(captured, 1, 1).unwrap();
    for seq in 1..=1024 {
        events.push(output(raw, 1, seq, seq as u8)).unwrap();
        events.push(output(captured, 1, seq, b'x')).unwrap();
    }
    for seq in 1025..=10_000 {
        events.push(output(captured, 1, seq, b'x')).unwrap();
    }
    assert_eq!(events.ordered.len(), 1025);
    let error = events.push(output(raw, 1, 1025, 0)).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("raw supervisor event backlog exceeded")
    );
    for seq in 1..=1024 {
        assert_eq!(events.pop_front(), Some(output(raw, 1, seq, seq as u8)));
    }
    assert_eq!(events.pop_front(), Some(output(captured, 1, 10_000, b'x')));
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[test]
fn lifecycle_is_retained_per_generation_and_later_output_follows_resize() {
    let seat = Ulid::new();
    let mut events = PendingEvents::default();
    for generation in 1..=500 {
        events.confirm_ring(seat, generation, 1).unwrap();
        events.push(output(seat, generation, 1, b'a')).unwrap();
        events
            .push(Event::Resized {
                seat,
                generation,
                seq: 2,
                size: SeatSize::default(),
            })
            .unwrap();
        events.push(output(seat, generation, 3, b'b')).unwrap();
        events
            .push(Event::Exited {
                seat,
                generation,
                seq: 4,
                status: ExitInfo {
                    code: 0,
                    signal: None,
                },
            })
            .unwrap();
    }
    // More than the old global 1024 cap, bounded at three kinds per generation.
    assert_eq!(events.ordered.len(), 1500);
    for generation in 1..=500 {
        assert_eq!(
            event_key(&events.pop_front().unwrap()),
            (seat, generation, 1)
        );
        assert_eq!(events.pop_front(), Some(output(seat, generation, 3, b'b')));
        assert!(
            matches!(events.pop_front(), Some(Event::Exited { generation: g, seq: 4, .. }) if g == generation)
        );
    }
    assert!(events.captured.is_empty());
    assert!(events.slots.is_empty());
}

// keeps: spec mux-daemon.md — Ownership and failure boundaries
#[ignore = "night native boundary: exact selector in scripts/night-test-boundaries.sh"]
#[test]
fn ring_notifications_survive_ten_thousand_requests_without_an_event_receive() {
    let root = tempfile::Builder::new()
        .prefix("w12g-client-")
        .tempdir_in("/tmp")
        .unwrap();
    let path = root.path().join("supervisor.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let seat = Ulid::new();
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        let mut requests = 0;
        let mut seq = 0;
        while let Ok(frame) = read_frame::<Request>(&mut stream) {
            if matches!(&frame.body.action, Action::AttachRing { directory, .. } if directory.ends_with("refused"))
            {
                write_frame(
                    &mut stream,
                    &Frame::new(Message::Reply {
                        request_id: frame.body.request_id,
                        result: Err(Failure::new(
                            crate::FailureCode::Invalid,
                            "fixture ring refusal",
                        )),
                    }),
                )
                .unwrap();
                continue;
            }
            let reply = match frame.body.action {
                Action::AcquireLogic { .. } => Reply::Acquired {
                    logic_lease: Ulid::new(),
                    supervisor_boot: Ulid::new(),
                },
                Action::AttachRing { .. } | Action::Attach { .. } => Reply::Attached {
                    attachment: Attachment {
                        seat,
                        generation: 1,
                        writer_lease: Ulid::new(),
                    },
                    info: SeatInfo {
                        seat,
                        generation: 1,
                        pid: 1,
                        size: SeatSize::default(),
                        next_sequence: 1,
                        exited: None,
                        companion: None,
                    },
                    first_available_sequence: 1,
                },
                Action::ListOwnedSeats => {
                    requests += 1;
                    seq += 1;
                    let event = Event::Output {
                        seat,
                        generation: 1,
                        seq,
                        bytes: vec![b'x'],
                    };
                    if write_frame(&mut stream, &Frame::new(Message::Event { event })).is_err() {
                        break;
                    }
                    if requests == 5_000 {
                        seq += 1;
                        let event = Event::Resized {
                            seat,
                            generation: 1,
                            seq,
                            size: SeatSize {
                                cols: 100,
                                ..Default::default()
                            },
                        };
                        write_frame(&mut stream, &Frame::new(Message::Event { event })).unwrap();
                    }
                    if requests == 10_000 {
                        seq += 1;
                        let event = Event::Exited {
                            seat,
                            generation: 1,
                            seq,
                            status: ExitInfo {
                                code: 0,
                                signal: None,
                            },
                        };
                        write_frame(&mut stream, &Frame::new(Message::Event { event })).unwrap();
                    }
                    Reply::Seats { seats: vec![] }
                }
                action => panic!("unexpected fixture action: {action:?}"),
            };
            if write_frame(
                &mut stream,
                &Frame::new(Message::Reply {
                    request_id: frame.body.request_id,
                    result: Ok(reply),
                }),
            )
            .is_err()
            {
                break;
            }
            // Keep the peer alive until the client has drained its events and
            // closes. A supervisor shutdown is not part of this backlog test.
        }
    });
    let result = (|| -> Result<Vec<Event>> {
        let mut client = SocketSupervisor::connect(&path, [7; 32])?;
        client.request(Action::Attach {
            seat,
            generation: 1,
        })?;
        assert!(
            client.events.captured.is_empty(),
            "raw Attach must not enable coalescing"
        );
        assert!(
            client
                .request(Action::AttachRing {
                    seat,
                    generation: 1,
                    directory: root.path().join("refused"),
                    settings: Default::default(),
                })
                .is_err()
        );
        assert!(
            client.events.captured.is_empty(),
            "failed AttachRing must not enable coalescing"
        );
        client.request(Action::AttachRing {
            seat,
            generation: 1,
            directory: root.path().join("ring"),
            settings: Default::default(),
        })?;
        for index in 0..10_000 {
            client
                .request(Action::ListOwnedSeats)
                .map_err(|error| invalid(format!("fixture request {}: {error}", index + 1)))?;
        }
        let mut events = vec![];
        // Exactly one latest output, resize and exit, then close the client.
        for index in 0..3 {
            events.push(
                client
                    .receive(Duration::from_secs(30))
                    .map_err(|error| invalid(format!("fixture receive {index}: {error}")))?
                    .expect("missing retained event"),
            );
        }
        Ok(events)
    })();
    peer.join().unwrap();
    let events = result.expect("captured notifications must not kill the supervisor client");
    assert!(matches!(events[0], Event::Resized { seq: 5_001, .. }));
    assert!(matches!(events[1], Event::Output { seq: 10_001, .. }));
    assert!(matches!(events[2], Event::Exited { seq: 10_002, .. }));
}

// keeps: spec mux-daemon — independently correlated capture replies
#[test]
fn independent_capture_replies_pass_a_held_seat_on_the_same_socket() {
    let (stream, mut peer) = std::os::unix::net::UnixStream::pair().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let lease = Ulid::new();
    let mut client = SocketSupervisor {
        stream: stream.into(),
        decoder: Default::default(),
        events: Default::default(),
        replies: Default::default(),
        lease: Some(lease),
        pending_request: None,
        captures: Default::default(),
        capture_writes: Default::default(),
        broken: false,
        diagnostics: Default::default(),
    };
    let read = |n: u128| Action::RingRecovery {
        attachment: Attachment {
            seat: Ulid::from(n),
            generation: 1,
            writer_lease: Ulid::from(n + 100),
        },
    };
    let held = client.start_capture(read(1)).unwrap();
    let ready = client.start_capture(read(2)).unwrap();
    let first = read_frame::<Request>(&mut peer).unwrap().body;
    let second = read_frame::<Request>(&mut peer).unwrap().body;
    assert_ne!(first.request_id, second.request_id);
    assert_eq!(first.logic_lease, Some(lease));
    assert_eq!(second.logic_lease, Some(lease));
    write_frame(
        &mut peer,
        &Frame::new(Message::Reply {
            request_id: second.request_id,
            result: Ok(Reply::Seats { seats: vec![] }),
        }),
    )
    .unwrap();
    assert!(client.receive(Duration::ZERO).unwrap().is_none());
    assert!(
        ready.try_recv().unwrap().is_some(),
        "second seat waited for first"
    );
    assert!(held.try_recv().unwrap().is_none());
    write_frame(
        &mut peer,
        &Frame::new(Message::Reply {
            request_id: first.request_id,
            result: Ok(Reply::Seats { seats: vec![] }),
        }),
    )
    .unwrap();
    assert!(client.receive(Duration::ZERO).unwrap().is_none());
    assert!(held.try_recv().unwrap().is_some());
}

// keeps: spec mux-daemon — a full capture window refuses before sending
#[test]
fn capture_window_is_bounded_without_waiting_for_the_held_replies() {
    let (stream, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut client = SocketSupervisor {
        stream: stream.into(),
        decoder: Default::default(),
        events: Default::default(),
        replies: Default::default(),
        lease: Some(Ulid::new()),
        pending_request: None,
        captures: Default::default(),
        capture_writes: Default::default(),
        broken: false,
        diagnostics: Default::default(),
    };
    let mut pending = Vec::new();
    for n in 0..crate::transport::MAX_PENDING_CAPTURES {
        pending.push(
            client
                .start_capture(Action::RingRecovery {
                    attachment: Attachment {
                        seat: Ulid::from(n as u128 + 1),
                        generation: 1,
                        writer_lease: Ulid::new(),
                    },
                })
                .unwrap(),
        );
    }
    let error = match client.start_capture(Action::RingRecovery {
        attachment: Attachment {
            seat: Ulid::new(),
            generation: 1,
            writer_lease: Ulid::new(),
        },
    }) {
        Ok(_) => panic!("unbounded capture window"),
        Err(error) => error,
    };
    assert_eq!(
        error.downcast_ref::<Failure>().unwrap().code,
        crate::FailureCode::Busy
    );
    assert_eq!(
        client.captures.len(),
        crate::transport::MAX_PENDING_CAPTURES
    );
    assert!(
        pending
            .iter()
            .all(|reply| reply.try_recv().unwrap().is_none())
    );
}

// keeps: spec mux-daemon — a peer that stops reading cannot hold capture admission
#[test]
fn backpressured_capture_frames_are_bounded_and_do_not_block_control_admission() {
    let (stream, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut client = SocketSupervisor {
        stream: stream.into(),
        decoder: Default::default(),
        events: Default::default(),
        replies: Default::default(),
        lease: Some(Ulid::new()),
        pending_request: None,
        captures: Default::default(),
        capture_writes: Default::default(),
        broken: false,
        diagnostics: Default::default(),
    };
    let mut pending = Vec::new();
    for _ in 0..crate::transport::MAX_PENDING_CAPTURES {
        pending.push(
            client
                .start_capture(Action::PublishCheckpoint {
                    attachment: Attachment {
                        seat: Ulid::new(),
                        generation: 1,
                        writer_lease: Ulid::new(),
                    },
                    checkpoint: vec![127; 200 * 1024],
                })
                .unwrap(),
        );
    }
    assert!(
        !client.capture_writes.is_empty(),
        "fixture must fill the unread socket"
    );
    assert!(client.capture_writes.len() <= crate::transport::MAX_PENDING_CAPTURES);
    let error = client.request(Action::ListOwnedSeats).unwrap_err();
    assert_eq!(
        error.downcast_ref::<Failure>().unwrap().code,
        crate::FailureCode::Busy
    );
    assert!(
        client.pending_request.is_none(),
        "ordinary request must remain unsent"
    );
}

// keeps: spec mux-daemon — timeout and late completion are local to one seat
#[test]
fn expired_capture_keeps_its_bounded_slot_and_does_not_close_a_healthy_reply() {
    let (mut client, memory) = Memory::client();
    let expired = Ulid::new();
    let healthy = Ulid::new();
    let (old_tx, old_rx) = mpsc::sync_channel(1);
    let (tx, rx) = mpsc::sync_channel(1);
    client.captures.insert(
        expired,
        (
            PendingRequest {
                id: expired,
                expires: memory.now(),
            },
            Some(old_tx),
        ),
    );
    client.captures.insert(
        healthy,
        (
            PendingRequest {
                id: healthy,
                expires: memory.now() + REQUEST_TIMEOUT,
            },
            Some(tx),
        ),
    );
    client.check_connection().unwrap();
    assert_eq!(
        old_rx.try_recv().unwrap().unwrap_err().code,
        crate::FailureCode::Io
    );
    assert!(!client.broken);
    assert!(
        client.captures[&expired].1.is_none(),
        "late correlation remains charged"
    );
    for id in [healthy, expired] {
        memory.0.lock().unwrap().incoming.extend(
            crate::transport::encode(&Frame::new(Message::Reply {
                request_id: id,
                result: Ok(Reply::Seats { seats: vec![] }),
            }))
            .unwrap(),
        );
    }
    assert!(client.receive(Duration::ZERO).unwrap().is_none());
    assert!(rx.try_recv().unwrap().is_ok());
    assert!(client.captures.is_empty());
    assert!(
        client.replies.is_empty(),
        "late capture reply must not enter control correlation"
    );
}

// keeps: spec mux-daemon — a long event wait cannot extend a capture deadline
#[test]
fn capture_deadline_bounds_a_long_event_receive() {
    let (mut client, memory) = Memory::client();
    let id = Ulid::new();
    let (tx, rx) = mpsc::sync_channel(1);
    let start = memory.now();
    client.captures.insert(
        id,
        (
            PendingRequest {
                id,
                expires: start + Duration::from_millis(20),
            },
            Some(tx),
        ),
    );
    assert!(client.receive(Duration::from_secs(30)).unwrap().is_none());
    assert_eq!(
        memory.now().duration_since(start),
        Duration::from_millis(20)
    );
    assert_eq!(
        rx.try_recv().unwrap().unwrap_err().code,
        crate::FailureCode::Io
    );
    assert!(!client.broken);
}

// keeps: spec mux-daemon — ready-seat turns never spend an idle timer quantum
#[test]
fn nonblocking_receive_does_not_spend_a_timer_quantum() {
    let (mut client, memory) = Memory::client();
    let now = memory.now();
    assert!(client.receive(Duration::ZERO).unwrap().is_none());
    assert_eq!(memory.now(), now);
}

// keeps: spec mux-daemon — ready capture frames do not wait for an Event to finish
#[test]
fn independent_capture_reply_drains_across_eventless_frame_fragments() {
    let (mut client, memory) = Memory::client();
    let id = Ulid::new();
    let (tx, rx) = mpsc::sync_channel(1);
    client.captures.insert(
        id,
        (
            PendingRequest {
                id,
                expires: memory.now() + REQUEST_TIMEOUT,
            },
            Some(tx),
        ),
    );
    let bytes = crate::transport::encode(&Frame::new(Message::Reply {
        request_id: id,
        result: Err(Failure::new(crate::FailureCode::Busy, "x".repeat(9000))),
    }))
    .unwrap();
    assert!(bytes.len() > 8192 && bytes.len() < 2 * 8192);
    memory.0.lock().unwrap().incoming.extend(bytes);
    assert!(client.receive(Duration::ZERO).unwrap().is_none());
    assert_eq!(rx.try_recv().unwrap().unwrap_err().message.len(), 9000);
    assert!(client.captures.is_empty());
}
