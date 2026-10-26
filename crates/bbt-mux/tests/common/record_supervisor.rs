//! An empty-capture supervisor for MCP record/authority tests.
//! Unexpected terminal effects fail the caller; real input/capture has an explicit fixture.
use bbt_mux::logic::SupervisorConnection;
use bbt_mux::{Action, Event, Reply, Ulid};
use bbt_supervisor::protocol::{Attachment, SeatInfo};
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Clone, Default)]
pub struct RecordLink(Arc<(Mutex<BTreeMap<Ulid, Seat>>, Condvar)>);
struct Seat {
    info: SeatInfo,
    attachment: Attachment,
    checkpoint: Option<Vec<u8>>,
}
impl Seat {
    fn status(&self) -> bbt_mux::ring::Status {
        bbt_mux::ring::Status {
            generation: self.info.generation,
            first_sequence: 1,
            committed_sequence: 0,
            captured_sequence: 0,
            byte_start: 0,
            byte_end: 0,
            hot_bytes: 0,
            paused: false,
            recovery_notice: None,
            capture: Default::default(),
        }
    }
    fn history(&self) -> Box<bbt_mux::ring::History> {
        Box::new(bbt_mux::ring::History {
            status: self.status(),
            records: vec![],
            gap: None,
            next: None,
        })
    }
}
impl SupervisorConnection for RecordLink {
    fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
        let mut seats = self.0.0.lock().unwrap();
        match action {
            Action::ListOwnedSeats => Ok(Reply::Seats {
                seats: seats.values().map(|seat| seat.info.clone()).collect(),
            }),
            Action::Spawn { spec } => {
                let info = SeatInfo {
                    seat: spec.seat,
                    generation: spec.generation,
                    // The test process hosts this in-memory fixture. No child
                    // is launched; kernel-boundary cases select the real fixture.
                    pid: std::process::id(),
                    size: spec.size,
                    next_sequence: 1,
                    exited: None,
                    companion: None,
                };
                assert!(!seats.contains_key(&spec.seat), "duplicate fixture seat");
                seats.insert(
                    spec.seat,
                    Seat {
                        info: info.clone(),
                        attachment: Attachment {
                            seat: spec.seat,
                            generation: spec.generation,
                            writer_lease: Ulid::new(),
                        },
                        checkpoint: None,
                    },
                );
                Ok(Reply::Spawned { info })
            }
            Action::AttachRing {
                seat, generation, ..
            } => {
                let seat = seats.get_mut(&seat).unwrap();
                assert_eq!(seat.info.generation, generation);
                seat.attachment.writer_lease = Ulid::new();
                Ok(Reply::Attached {
                    attachment: seat.attachment.clone(),
                    info: seat.info.clone(),
                    first_available_sequence: 1,
                })
            }
            Action::RingRecovery { attachment } => {
                let seat = &seats[&attachment.seat];
                assert_eq!(seat.attachment, attachment);
                Ok(Reply::RingRecovery {
                    status: Box::new(seat.status()),
                    checkpoint: seat.checkpoint.clone(),
                })
            }
            Action::PublishCheckpoint {
                attachment,
                checkpoint,
            } => {
                let seat = seats.get_mut(&attachment.seat).unwrap();
                assert_eq!(seat.attachment, attachment);
                let saved = bbt_mux::ring::Saved::decode(&checkpoint)?;
                assert_eq!(saved.sequence, 0);
                seat.checkpoint = Some(checkpoint);
                Ok(Reply::CheckpointCommitted {
                    sequence: saved.sequence,
                })
            }
            Action::InputContext { attachment } => {
                let seat = &seats[&attachment.seat];
                assert_eq!(seat.attachment, attachment);
                Ok(Reply::InputContext {
                    context: bbt_supervisor::protocol::InputContext {
                        epoch: 1,
                        last_input_ms: 0,
                        sequence: 0,
                        draft: false,
                        foreground_pid: None,
                        canonical: true,
                        echo: true,
                        settled: true,
                    },
                })
            }
            Action::ReadRing { attachment, .. } => {
                let seat = &seats[&attachment.seat];
                assert_eq!(seat.attachment, attachment);
                Ok(Reply::RingHistory {
                    history: seat.history(),
                })
            }
            Action::ReadRingBatch { reads, .. } => {
                let results = reads
                    .into_iter()
                    .map(|read| {
                        let seat = &seats[&read.attachment.seat];
                        assert_eq!(seat.attachment, read.attachment);
                        bbt_supervisor::protocol::RingReadResult {
                            seat: read.attachment.seat,
                            generation: read.attachment.generation,
                            from: read.from,
                            to: read.to,
                            result: Ok(seat.history()),
                        }
                    })
                    .collect();
                Ok(Reply::RingHistoryBatch { results })
            }
            action => panic!("record-only MCP fixture reached a terminal effect: {action:?}"),
        }
    }
    fn receive(&mut self, timeout: Duration) -> bbt_mux::store::Result<Option<Event>> {
        // This is the transport's requested blocking receive, with no producer.
        // It asserts no time bound and never runs a child process.
        drop(
            self.0
                .1
                .wait_timeout(self.0.0.lock().unwrap(), timeout)
                .unwrap(),
        );
        Ok(None)
    }
}
