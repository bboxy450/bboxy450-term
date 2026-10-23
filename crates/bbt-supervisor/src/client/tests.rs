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

#[test]
fn a_peer_closed_after_its_final_reply_does_not_hide_buffered_bytes_behind_timeout_setup() {
    let (stream, mut peer) = std::os::unix::net::UnixStream::pair().unwrap();
    let request_id = Ulid::new();
    write_frame(
        &mut peer,
        &Frame::new(Message::Reply {
            request_id,
            result: Ok(Reply::Seats { seats: vec![] }),
        }),
    )
    .unwrap();
    drop(peer); // deterministic peer close, before the client configures its read
    let mut client = SocketSupervisor {
        stream,
        decoder: Default::default(),
        events: Default::default(),
        replies: Default::default(),
        lease: None,
    };
    let native = client
        .stream
        .set_read_timeout(Some(Duration::from_millis(20)));
    eprintln!("closed-peer native SO_RCVTIMEO: {native:?}");
    client.read(Duration::from_millis(20)).unwrap();
    assert!(
        matches!(client.replies.remove(&request_id), Some(Ok(Reply::Seats { seats })) if seats.is_empty())
    );
    let error = client.receive(Duration::from_millis(20)).unwrap_err();
    assert!(
        error.to_string().contains("supervisor disconnected"),
        "EOF misreported: {error:?}"
    );
}

struct ReleasePeer(std::sync::mpsc::Sender<()>);
impl Drop for ReleasePeer {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

fn admit_peer(stream: &mut std::os::unix::net::UnixStream) {
    let frame = read_frame::<Request>(stream).unwrap();
    assert!(matches!(frame.body.action, Action::AcquireLogic { .. }));
    write_frame(
        stream,
        &Frame::new(Message::Reply {
            request_id: frame.body.request_id,
            result: Ok(Reply::Acquired {
                logic_lease: Ulid::new(),
                supervisor_boot: Ulid::new(),
            }),
        }),
    )
    .unwrap();
}

#[test]
fn a_timed_out_request_blocks_new_writes_until_its_late_reply_is_drained() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("serial.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (release, released) = std::sync::mpsc::channel();
    let (identify, identified) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .unwrap();
        admit_peer(&mut stream);
        let old = read_frame::<Request>(&mut stream).unwrap();
        identify.send(old.body.request_id).unwrap();
        let _ = released.recv_timeout(Duration::from_secs(20));
        if write_frame(
            &mut stream,
            &Frame::new(Message::Reply {
                request_id: old.body.request_id,
                result: Ok(Reply::Seats { seats: vec![] }),
            }),
        )
        .is_err()
        {
            return;
        }
        let new = read_frame::<Request>(&mut stream).unwrap();
        assert_ne!(old.body.request_id, new.body.request_id);
        assert!(matches!(new.body.action, Action::ListOwnedSeats));
        let _ = write_frame(
            &mut stream,
            &Frame::new(Message::Reply {
                request_id: new.body.request_id,
                result: Ok(Reply::Seats { seats: vec![] }),
            }),
        );
    });
    let mut client = SocketSupervisor::connect(&path, [11; 32]).unwrap();
    let release = ReleasePeer(release);
    let error = client.request(Action::ListOwnedSeats).unwrap_err();
    assert!(error.to_string().contains("outcome unknown"), "{error}");
    let old = identified.recv_timeout(Duration::from_secs(1)).unwrap();
    let held = client.request(Action::ListOwnedSeats).unwrap_err();
    assert!(
        held.downcast_ref::<Failure>()
            .is_some_and(|f| f.code == crate::FailureCode::Busy),
        "not retryable/no-effect: {held:?}"
    );
    release.0.send(()).unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    while !client.replies.contains_key(&old) {
        client.receive(Duration::from_millis(20)).unwrap();
        assert!(Instant::now() < until, "late reply not drained");
    }
    assert!(matches!(
        client.replies.remove(&old),
        Some(Ok(Reply::Seats { .. }))
    ));
    assert!(matches!(
        client.request(Action::ListOwnedSeats).unwrap(),
        Reply::Seats { .. }
    ));
    peer.join().unwrap();
}

#[test]
fn an_unanswered_request_becomes_disconnected_at_the_bounded_recovery_deadline() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("unanswered.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let (release, released) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        admit_peer(&mut stream);
        let _old = read_frame::<Request>(&mut stream).unwrap();
        let _ = released.recv_timeout(Duration::from_secs(20));
    });
    let mut client = SocketSupervisor::connect(&path, [12; 32]).unwrap();
    let release = ReleasePeer(release);
    let started = Instant::now();
    let error = client.request(Action::ListOwnedSeats).unwrap_err();
    assert!(error.to_string().contains("outcome unknown"), "{error}");
    // A long requested receive must still be clipped by the old request's
    // second deadline (10 s from its original write, not another 60 s).
    let error = client.receive(Duration::from_secs(60)).unwrap_err();
    let elapsed = started.elapsed();
    eprintln!("unanswered request disconnected after {elapsed:?}: {error}");
    assert!(error.to_string().contains("supervisor disconnected"));
    assert!(
        elapsed < Duration::from_secs(12),
        "recovery deadline did not bound read: {elapsed:?}"
    );
    let later = client.request(Action::ListOwnedSeats).unwrap_err();
    assert!(later.to_string().contains("supervisor disconnected"));
    drop(release);
    peer.join().unwrap();
}

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
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
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
                    .receive(Duration::from_secs(1))
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
