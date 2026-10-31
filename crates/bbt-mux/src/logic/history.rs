// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded GUI/MCP history completions, independent of live seat parsing.
use super::*;

type GuiReply = oneshot::Sender<Result<(crate::ring::History, Vec<crate::history_fault::Missing>)>>;
pub(super) enum Destination {
    Gui { masks: bool, reply: GuiReply },
    Mcp(Box<McpRead>),
}
pub(super) struct McpRead {
    actor: crate::mcp::SeatActor,
    call: crate::mcp::Call,
    request: crate::mcp::seats::Request,
    reply: oneshot::Sender<Result<serde_json::Value>>,
}
pub(super) struct Read {
    attachment: Attachment,
    from: u64,
    to: u64,
    pending: Option<bbt_supervisor::client::PendingCapture>,
    admitted: Instant,
    destination: Destination,
}

pub(super) fn range(
    target: &crate::mcp::seats::Target,
    cursor: Option<&str>,
    sequence: u64,
) -> std::result::Result<(u64, u64), String> {
    if let Some(cursor) = cursor {
        let cursor: crate::mcp::seats::CaptureCursor = crate::mcp::seats::decode_cursor(cursor)?;
        if cursor.seat != target.seat
            || cursor.generation != target.generation
            || cursor.next == 0
            || cursor.next > cursor.to
            || cursor.to > sequence
        {
            return Err("stale or invalid capture cursor".into());
        }
        Ok((cursor.next, cursor.to))
    } else {
        Ok((1, sequence))
    }
}

impl Worker {
    pub(super) fn start_history(
        &mut self,
        id: Ulid,
        generation: u64,
        from: u64,
        to: u64,
        destination: Destination,
    ) {
        let result = (|| {
            if self.history_reads.contains_key(&id) {
                return Err(crate::Failure {
                    code: crate::FailureCode::Busy,
                    message: "one outstanding history page per seat".into(),
                }
                .into());
            }
            let attachment = self
                .attachments
                .get(&id)
                .filter(|a| a.generation == generation)
                .ok_or_else(|| invalid("stale history generation"))?
                .clone();
            Ok(attachment)
        })();
        match result {
            Ok(attachment) => {
                self.history_reads.insert(
                    id,
                    Read {
                        attachment,
                        from,
                        to,
                        pending: None,
                        admitted: Instant::now(),
                        destination,
                    },
                );
            }
            Err(error) => self.finish_history(id, generation, from, to, destination, Err(error)),
        }
    }

    pub(super) fn start_control_history(
        &mut self,
        id: Ulid,
        generation: u64,
        call: crate::mcp::Call,
        reply: oneshot::Sender<Result<serde_json::Value>>,
    ) {
        let prepared = (|| -> Result<_> {
            call.validate()?;
            let (actor, caller) = self.control_actor(id, generation)?;
            let request = self
                .prepare_seat_control(&actor, &caller, &call)
                .map_err(invalid)?;
            let crate::mcp::seats::Request::History { target, cursor, .. } = &request else {
                return Err(invalid("not a history read"));
            };
            let sequence = self
                .views
                .get(&target.seat)
                .ok_or_else(|| invalid("target viewport unavailable"))?
                .sequence;
            let (from, to) = range(target, cursor.as_deref(), sequence).map_err(invalid)?;
            Ok((actor, target.seat, target.generation, from, to, request))
        })();
        match prepared {
            Ok((actor, target, generation, from, to, request)) => {
                let destination = Destination::Mcp(Box::new(McpRead {
                    actor,
                    call,
                    request,
                    reply,
                }));
                if to == 0 {
                    // The empty range needs no storage read; use the ordinary bounded encoder.
                    self.finish_history(
                        target,
                        generation,
                        from,
                        to,
                        destination,
                        Err(invalid("empty history range")),
                    );
                } else {
                    self.start_history(target, generation, from, to, destination);
                }
            }
            Err(error) => {
                let _ = reply.send(Ok(crate::mcp::error(
                    call.op_id,
                    "rejected",
                    &error.to_string(),
                )));
            }
        }
    }

    pub(super) fn poll_history(&mut self) {
        let waiting: Vec<_> = self
            .history_reads
            .iter()
            .filter(|(_, read)| read.pending.is_none())
            .take(32)
            .map(|(id, _)| *id)
            .collect();
        let mut completed = Vec::new();
        for id in waiting {
            let read = &self.history_reads[&id];
            let result = (|| {
                let link = self.connections.link(&self.host_for_seat(id)?)?;
                let mut link = match link.try_lock() {
                    Ok(link) => link,
                    Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
                    Err(std::sync::TryLockError::Poisoned(_)) => {
                        return Err(invalid("supervisor link owner stopped"));
                    }
                };
                link.start_capture(Action::ReadRing {
                    attachment: read.attachment.clone(),
                    from: read.from,
                    to: read.to,
                })
                .map(Some)
            })();
            match result {
                Ok(pending) => self.history_reads.get_mut(&id).unwrap().pending = pending,
                Err(error) if is_capture_busy(&error) => {}
                Err(error) => completed.push((id, Err(error))),
            }
        }
        for (id, read) in &mut self.history_reads {
            if completed.iter().any(|(done, _)| done == id) {
                continue;
            }
            if read.admitted.elapsed() >= Duration::from_secs(10) {
                completed.push((*id, Err(invalid("history page deadline expired"))));
            } else if let Some(pending) = &read.pending {
                match pending.try_recv() {
                    Ok(None) => {}
                    Err(error) if is_capture_busy(&error) => read.pending = None,
                    result => completed.push((
                        *id,
                        result.and_then(|reply| match reply {
                            Some(Reply::RingHistory { history }) => Ok(*history),
                            _ => Err(invalid("history reply mismatch")),
                        }),
                    )),
                }
            }
            if completed.len() == 32 {
                break;
            }
        }
        for (id, result) in completed {
            let read = self.history_reads.remove(&id).unwrap();
            let result = if self.attachments.get(&id) == Some(&read.attachment) {
                result
            } else {
                Err(invalid("stale history completion"))
            };
            if let Ok(history) = &result
                && let Some(view) = self.views.get_mut(&id)
            {
                view.captured_sequence =
                    view.captured_sequence.max(history.status.captured_sequence);
                self.client
                    .capture_diagnostics
                    .seat(id, read.attachment.generation, |row, now| {
                        row.ring_observed_ns = now;
                        row.captured_sequence =
                            row.captured_sequence.max(history.status.captured_sequence);
                        row.committed_sequence = row
                            .committed_sequence
                            .max(history.status.committed_sequence);
                    });
            }
            self.finish_history(
                id,
                read.attachment.generation,
                read.from,
                read.to,
                read.destination,
                result,
            );
        }
    }

    fn finish_history(
        &mut self,
        id: Ulid,
        generation: u64,
        from: u64,
        to: u64,
        destination: Destination,
        result: Result<crate::ring::History>,
    ) {
        match destination {
            Destination::Gui { masks, reply } => {
                let result = result.and_then(|history| {
                    let masks = if masks
                        && self
                            .identity
                            .issued_by()
                            .is_some_and(|s| s.starts_with("test-account:"))
                    {
                        self.history_faults.ranges(id, generation)?
                    } else {
                        Vec::new()
                    };
                    Ok((history, masks))
                });
                let _ = reply.send(result);
            }
            Destination::Mcp(read) => {
                let value = (|| -> Result<_> {
                    let (current, caller) =
                        self.control_actor(read.actor.seat_id, read.actor.process_generation)?;
                    if current != read.actor {
                        return Err(invalid("history caller authority changed"));
                    }
                    self.prepare_seat_control(&current, &caller, &read.call)
                        .map_err(invalid)?;
                    let value = if to == 0 {
                        self.read_seat_control(&read.request, None)
                    } else {
                        result.map_err(|e| e.to_string()).and_then(|history| {
                            self.read_seat_control(&read.request, Some((history, from, to)))
                        })
                    };
                    let value = match value {
                        Ok(value) => {
                            let empty = value
                                .get("records")
                                .and_then(serde_json::Value::as_array)
                                .is_some_and(Vec::is_empty);
                            crate::mcp::success(
                                read.call.op_id,
                                crate::mcp::tools::scoped_list(
                                    &current,
                                    value,
                                    empty.then_some("nothing_yet"),
                                ),
                            )
                        }
                        Err(reason) => crate::mcp::error(read.call.op_id, "rejected", &reason),
                    };
                    self.control_journal(read.actor.seat_id)?
                        .checked(&current, &read.call, value)
                })();
                let _ = read.reply.send(value);
            }
        }
    }
}
