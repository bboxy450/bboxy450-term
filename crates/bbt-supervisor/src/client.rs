// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Blocking supervisor clients, shared by in-process and separate-process logic.

use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::store::{Result, invalid};
use crate::{Action, Client, Event, Failure, Frame, Message, Reply, Ulid};

mod wire;
use wire::Wire;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// A single correlated capture completion. Dropping it cancels observation,
/// never a write or the supervisor's lease. No IO lock is held while waiting.
pub struct PendingCapture {
    receiver: mpsc::Receiver<std::result::Result<Reply, Failure>>,
    expires: Instant,
}
impl PendingCapture {
    /// Wrap one bounded correlated reply from a connection adapter.
    pub fn new(receiver: mpsc::Receiver<std::result::Result<Reply, Failure>>) -> Self {
        Self {
            receiver,
            expires: Instant::now() + REQUEST_TIMEOUT * 2,
        }
    }
    /// Construct an already completed reply, for in-memory connection adapters.
    pub fn ready(result: Result<Reply>) -> Self {
        let (tx, rx) = mpsc::sync_channel(1);
        let _ = tx.send(result.map_err(|error| {
            error
                .downcast_ref::<Failure>()
                .cloned()
                .unwrap_or_else(|| Failure::new(crate::FailureCode::Io, error.to_string()))
        }));
        Self::new(rx)
    }
    /// Observe completion without waiting for another seat or reading a socket.
    /// Socket owners must keep servicing `receive` to route correlated replies.
    ///
    /// # Errors
    /// Returns the original refusal, a closed owner, or an expired read.
    pub fn try_recv(&self) -> Result<Option<Reply>> {
        match self.receiver.try_recv() {
            Ok(result) => result.map(Some).map_err(Into::into),
            Err(mpsc::TryRecvError::Empty) if Instant::now() < self.expires => Ok(None),
            Err(mpsc::TryRecvError::Empty) => Err(Failure::new(
                crate::FailureCode::Io,
                "capture reply deadline expired; outcome unknown",
            )
            .into()),
            Err(mpsc::TryRecvError::Disconnected) => Err(Failure::new(
                crate::FailureCode::Io,
                "capture reply owner stopped; outcome unknown",
            )
            .into()),
        }
    }
    /// Wait on an IO side task without retaining the supervisor connection lock.
    ///
    /// # Errors
    /// Returns the original refusal, disconnection or deadline expiration.
    pub fn wait(self) -> Result<Reply> {
        self.receiver
            .recv_timeout(self.expires.saturating_duration_since(Instant::now()))
            .map_err(|_| invalid("capture reply unavailable; outcome unknown"))?
            .map_err(Into::into)
    }
}

pub(crate) fn capture_action(action: &Action) -> Result<()> {
    if matches!(
        action,
        Action::ReadRing { .. }
            | Action::ReadRingBatch { .. }
            | Action::RingRecovery { .. }
            | Action::PublishCheckpoint { .. }
    ) {
        Ok(())
    } else {
        Err(invalid(
            "only capture operations may use the independent completion port",
        ))
    }
}

type CaptureCompletion = mpsc::SyncSender<std::result::Result<Reply, Failure>>;

struct PendingRequest {
    id: Ulid,
    expires: Instant,
}

#[cfg(test)]
mod tests;

/// Blocking control and event port shared by admitted in-process and socket clients.
pub trait SupervisorConnection: Send {
    /// Submit one action on its IO owner and return the correlated reply.
    ///
    /// # Errors
    /// Returns named host refusals, framing, disconnection or request-completion failures.
    fn request(&mut self, action: Action) -> Result<Reply>;
    /// Admit capture work and release the connection before its completion.
    /// Native and socket clients pipeline independently correlated requests;
    /// the compatibility default is for synchronous in-memory test adapters.
    ///
    /// # Errors
    /// Returns admission/refusal without advancing an output cursor.
    fn start_capture(&mut self, action: Action) -> Result<PendingCapture> {
        capture_action(&action)?;
        // Preserve typed admission failures (including provider LinkDown), so
        // the owner can park and resume that host instead of fencing a seat.
        let reply = self.request(action)?;
        Ok(PendingCapture::ready(Ok(reply)))
    }
    /// Receive one observed event, returning `None` when the supplied wait expires.
    ///
    /// # Errors
    /// Returns transport, continuity or disconnection failures.
    fn receive(&mut self, timeout: Duration) -> Result<Option<Event>>;
}
impl SupervisorConnection for Client {
    fn start_capture(&mut self, action: Action) -> Result<PendingCapture> {
        Ok(Client::start_capture(self, action)?)
    }
    fn request(&mut self, action: Action) -> Result<Reply> {
        Ok(Client::request(self, action)?)
    }
    fn receive(&mut self, timeout: Duration) -> Result<Option<Event>> {
        match self.recv_timeout(timeout) {
            Ok(event) => Ok(Some(event)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

type EventKey = (Ulid, u64, u8);

fn event_key(event: &Event) -> EventKey {
    match event {
        Event::Output {
            seat, generation, ..
        } => (*seat, *generation, 0),
        Event::Resized {
            seat, generation, ..
        } => (*seat, *generation, 1),
        Event::Exited {
            seat, generation, ..
        } => (*seat, *generation, 2),
    }
}

struct Pending {
    event: Event,
    raw: bool,
}

/// Ring-backed notifications are bounded by distinct pending
/// (seat, generation, kind), not output volume. Each process exits once; exits
/// are retained, never replaced by output or by a newer process generation.
/// Moving a replaced notification to the back preserves per-process ordering:
/// an intervening resize is received before a later output that replays its gap.
/// Reusable per-seat/generation/kind queue; raw prefixes stay FIFO and exits are never replaced.
#[derive(Default)]
pub struct PendingEvents {
    captured: BTreeMap<(Ulid, u64), u64>,
    ordered: BTreeMap<u64, Pending>,
    slots: BTreeMap<EventKey, u64>,
    next: u64,
    raw_count: usize,
}
impl PendingEvents {
    /// Coalesce captured process/generation/kind slots; raw prefixes stay FIFO and exits are never replaced.
    ///
    /// # Errors
    /// Returns a continuity failure when the uncaptured FIFO backlog reaches 1024 events.
    pub fn push(&mut self, event: Event) -> Result<()> {
        let key = event_key(&event);
        let raw = !self
            .captured
            .get(&(key.0, key.1))
            .is_some_and(|first| event.sequence().unwrap() >= *first);
        if raw {
            if self.raw_count >= 1024 {
                return Err(invalid(
                    "raw supervisor event backlog exceeded; continuity unavailable without AttachRing",
                ));
            }
            self.raw_count += 1;
        } else if key.2 != 2
            && let Some(old) = self.slots.insert(key, self.next)
        {
            self.ordered.remove(&old);
        }
        self.ordered.insert(self.next, Pending { event, raw });
        self.next += 1;
        Ok(())
    }

    /// Promote only the confirmed captured range, preserving FIFO raw prefixes and all exits.
    ///
    /// # Errors
    /// Returns a continuity failure if a retained raw prefix exceeds the bounded backlog.
    pub fn confirm_ring(&mut self, seat: Ulid, generation: u64, first: u64) -> Result<()> {
        self.captured.insert((seat, generation), first);
        // Events and the attach reply can share a socket read. Promote only
        // records covered by the confirmed ring; pre-ring bytes stay FIFO.
        let pending = std::mem::take(&mut self.ordered);
        self.slots.clear();
        self.raw_count = 0;
        for queued in pending.into_values() {
            self.push(queued.event)?;
        }
        Ok(())
    }

    /// Remove the oldest notification; coalescing never replaces an exit or a raw prefix.
    pub fn pop_front(&mut self) -> Option<Event> {
        let (_, queued) = self.ordered.pop_first()?;
        let key = event_key(&queued.event);
        if queued.raw {
            self.raw_count -= 1;
        } else if key.2 != 2 {
            self.slots.remove(&key);
        }
        if key.2 == 2 {
            self.captured.remove(&(key.0, key.1));
        }
        Some(queued.event)
    }
}

/// Fixed-size, numeric observations of the serial client. Reading these does
/// not drain events, issue a request, or acquire another logic lease.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct ClientDiagnostics {
    /// Number of socket-read attempts made by the serial client.
    pub read_calls: u64,
    /// Total bytes successfully read from the control socket.
    pub read_bytes: u64,
    /// Number of asynchronous event frames decoded from the socket.
    pub decoded_event_frames: u64,
    /// Number of event-receive calls made by the caller.
    pub receive_calls: u64,
    /// Number of events returned to the caller.
    pub returned_events: u64,
    /// Number of receive calls that returned without an event.
    pub empty_receives: u64,
    /// Currently retained event notifications.
    pub queued_events: usize,
    /// FIFO event prefix not yet covered by confirmed retained capture.
    pub queued_raw_events: usize,
    /// Latest queued Output for the caller's exact seat/generation, if any.
    pub queued_output_sequence: Option<u64>,
}

/// The same logic can run in a separate process against W2.1's private socket.
pub struct SocketSupervisor {
    stream: Wire,
    decoder: crate::transport::Decoder<Message>,
    events: PendingEvents,
    replies: BTreeMap<Ulid, std::result::Result<Reply, Failure>>,
    lease: Option<Ulid>,
    pending_request: Option<PendingRequest>,
    captures: BTreeMap<Ulid, (PendingRequest, Option<CaptureCompletion>)>,
    capture_writes: VecDeque<(Vec<u8>, usize)>,
    broken: bool,
    diagnostics: ClientDiagnostics,
}
impl SocketSupervisor {
    /// Connect to the explicit Unix socket and acquire a logic lease with the owner capability.
    ///
    /// # Errors
    /// Returns IO, framing, disconnection or admission failures.
    ///
    /// # Panics
    /// In debug builds, panics when called on the registered UI thread.
    pub fn connect(path: &Path, capability: [u8; 32]) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let stream = std::os::unix::net::UnixStream::connect(path)?;
        stream.set_write_timeout(Some(REQUEST_TIMEOUT))?;
        let mut client = Self {
            stream: stream.into(),
            decoder: Default::default(),
            events: PendingEvents::default(),
            replies: BTreeMap::new(),
            lease: None,
            pending_request: None,
            captures: BTreeMap::new(),
            capture_writes: VecDeque::new(),
            broken: false,
            diagnostics: ClientDiagnostics::default(),
        };
        match client.request(Action::AcquireLogic { capability })? {
            Reply::Acquired { logic_lease, .. } => client.lease = Some(logic_lease),
            _ => return Err(invalid("supervisor did not admit logic")),
        }
        Ok(client)
    }
    /// Copy counters and one focused queue observation without consuming data.
    /// The queue observation costs two tree lookups, not a scan across seats.
    pub fn diagnostics(&self, seat: Ulid, generation: u64) -> ClientDiagnostics {
        ClientDiagnostics {
            queued_events: self.events.ordered.len(),
            queued_raw_events: self.events.raw_count,
            queued_output_sequence: self
                .events
                .slots
                .get(&(seat, generation, 0))
                .and_then(|slot| self.events.ordered.get(slot))
                .and_then(|pending| pending.event.sequence()),
            ..self.diagnostics
        }
    }

    fn check_connection(&mut self) -> Result<()> {
        if self.broken {
            return Err(invalid("supervisor disconnected"));
        }
        let now = self.stream.now();
        for (pending, sender) in self.captures.values_mut() {
            if now >= pending.expires
                && let Some(sender) = sender.take()
            {
                // Keep the bounded correlation slot until its late reply is
                // drained. One failed seat never closes healthy seat reads.
                let _ = sender.try_send(Err(Failure::new(
                    crate::FailureCode::Io,
                    "capture reply deadline expired; outcome unknown",
                )));
            }
        }
        if let Some(pending) = &self.pending_request
            && self.stream.now() >= pending.expires
        {
            let id = pending.id;
            self.broken = true;
            let _ = self.stream.shutdown(std::net::Shutdown::Both);
            return Err(invalid(format!(
                "supervisor disconnected: request {id} stayed unresolved beyond its recovery deadline; outcome remains unknown"
            )));
        }
        Ok(())
    }

    fn read(&mut self, timeout: Duration) -> Result<bool> {
        self.check_connection()?;
        self.flush_capture_writes()?;
        self.diagnostics.read_calls = self.diagnostics.read_calls.saturating_add(1);
        let timeout = self.pending_request.as_ref().map_or(timeout, |pending| {
            timeout.min(pending.expires.saturating_duration_since(self.stream.now()))
        });
        let timeout = self
            .captures
            .values()
            .filter(|(_, sender)| sender.is_some())
            .fold(timeout, |timeout, (pending, _)| {
                timeout.min(pending.expires.saturating_duration_since(self.stream.now()))
            });
        if !timeout.is_zero()
            && let Err(error) = self
                .stream
                .set_read_timeout(Some(timeout.max(Duration::from_millis(1))))
            && !(cfg!(target_os = "macos") && error.raw_os_error() == Some(libc::EINVAL))
        {
            return Err(error.into());
        }
        // Darwin can reject SO_RCVTIMEO after peer close, with buffered bytes
        // still unread. Consume those bytes, then report the actual read/EOF.
        let mut bytes = [0; 8192];
        let result = if timeout.is_zero() {
            self.stream.try_read(&mut bytes)
        } else {
            self.stream.read(&mut bytes)
        };
        let count = match result {
            Ok(0) => {
                self.broken = true;
                return Err(invalid("supervisor disconnected"));
            }
            Ok(n) => n,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return self.check_connection().map(|()| false);
            }
            Err(e) => return Err(e.into()),
        };
        self.check_connection()?;
        self.diagnostics.read_bytes = self.diagnostics.read_bytes.saturating_add(count as u64);
        for frame in self.decoder.push(&bytes[..count])? {
            match frame.body {
                Message::Event { event } => {
                    self.diagnostics.decoded_event_frames =
                        self.diagnostics.decoded_event_frames.saturating_add(1);
                    self.events.push(event)?;
                }
                Message::Reply { request_id, result } => {
                    if let Some((_, sender)) = self.captures.remove(&request_id) {
                        if let Some(sender) = sender {
                            let _ = sender.try_send(result);
                        }
                        continue;
                    }
                    if self
                        .pending_request
                        .as_ref()
                        .is_some_and(|pending| pending.id == request_id)
                    {
                        self.pending_request = None;
                    }
                    if self.replies.len() >= 32 {
                        return Err(invalid("unexpected reply backlog"));
                    }
                    self.replies.insert(request_id, result);
                }
            }
        }
        Ok(true)
    }

    fn flush_capture_writes(&mut self) -> Result<()> {
        while let Some((bytes, offset)) = self.capture_writes.front_mut() {
            match self.stream.try_write(&bytes[*offset..]) {
                Ok(0) => return Err(invalid("supervisor capture write closed")),
                Ok(count) => {
                    *offset += count;
                    if *offset == bytes.len() {
                        self.capture_writes.pop_front();
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}
impl SupervisorConnection for SocketSupervisor {
    fn start_capture(&mut self, action: Action) -> Result<PendingCapture> {
        capture_action(&action)?;
        self.check_connection()?;
        if self.captures.len() >= crate::transport::MAX_PENDING_CAPTURES {
            return Err(Failure::new(
                crate::FailureCode::Busy,
                "capture request window full; not sent",
            )
            .into());
        }
        let request = crate::Request::new(self.lease, action);
        let id = request.request_id;
        let bytes = crate::transport::encode(&Frame::new(request))?;
        let (sender, receiver) = mpsc::sync_channel(1);
        self.captures.insert(
            id,
            (
                PendingRequest {
                    id,
                    expires: self.stream.now() + REQUEST_TIMEOUT * 2,
                },
                Some(sender),
            ),
        );
        self.capture_writes.push_back((bytes, 0));
        if let Err(error) = self.flush_capture_writes() {
            self.captures.remove(&id);
            self.broken = true;
            return Err(error);
        }
        Ok(PendingCapture::new(receiver))
    }
    fn request(&mut self, action: Action) -> Result<Reply> {
        bbt_rt::assert_not_ui_thread();
        self.check_connection()?;
        self.flush_capture_writes()?;
        if !self.capture_writes.is_empty() {
            return Err(Failure::new(
                crate::FailureCode::Busy,
                "capture frame write pending; control request not sent",
            )
            .into());
        }
        if self.pending_request.is_some() {
            // A timed-out call may still have a reply on the wire. A small
            // drain permits recovery even while logic retains a replay event.
            self.read(Duration::ZERO)?;
            if let Some(pending) = &self.pending_request {
                return Err(Failure::new(
                    crate::FailureCode::Busy,
                    format!(
                        "request {} is still unresolved; new request not sent",
                        pending.id
                    ),
                )
                .into());
            }
        }
        let ring = match &action {
            Action::AttachRing {
                seat, generation, ..
            } => Some((*seat, *generation)),
            _ => None,
        };
        let request = crate::Request::new(self.lease, action);
        let id = request.request_id;
        let bytes = crate::transport::encode(&Frame::new(request))?;
        let started = self.stream.now();
        // One ordinary control request remains outstanding, including after an
        // ambiguous timeout. Independent capture requests have their own bounded
        // correlation slots and largest-frame reply reserves.
        let previous = self.pending_request.replace(PendingRequest {
            id,
            expires: started + REQUEST_TIMEOUT * 2,
        });
        debug_assert!(
            previous.is_none(),
            "multiple outstanding supervisor requests"
        );
        self.stream.write_all(&bytes)?;
        let until = started + REQUEST_TIMEOUT;
        loop {
            if let Some(reply) = self.replies.remove(&id) {
                let reply = reply?;
                if let (
                    Some((seat, generation)),
                    Reply::Attached {
                        attachment,
                        first_available_sequence,
                        ..
                    },
                ) = (ring, &reply)
                {
                    if attachment.seat != seat || attachment.generation != generation {
                        return Err(invalid("ring attach reply mismatches seat/generation"));
                    }
                    self.events
                        .confirm_ring(seat, generation, *first_available_sequence)?;
                }
                return Ok(reply);
            }
            if self.stream.now() >= until {
                return Err(invalid(
                    "supervisor operation outcome unknown; query original id",
                ));
            }
            self.read(Duration::from_millis(20))?;
        }
    }
    fn receive(&mut self, timeout: Duration) -> Result<Option<Event>> {
        bbt_rt::assert_not_ui_thread();
        self.check_connection()?;
        self.diagnostics.receive_calls = self.diagnostics.receive_calls.saturating_add(1);
        let event = if let Some(event) = self.events.pop_front() {
            Some(event)
        } else if timeout.is_zero() {
            // A partial frame or a capture reply is IO progress even without
            // an Event. Drain available bytes instead of one 8 KiB fragment
            // per logic turn; retain a bounded byte/time service quantum.
            let until = Instant::now() + Duration::from_millis(1);
            let mut event = None;
            for turn in 0..128 {
                if !self.read(Duration::ZERO)? {
                    break;
                }
                event = self.events.pop_front();
                if event.is_some() || (turn > 0 && Instant::now() >= until) {
                    break;
                }
            }
            event
        } else {
            self.read(timeout)?;
            self.events.pop_front()
        };
        if event.is_some() {
            self.diagnostics.returned_events = self.diagnostics.returned_events.saturating_add(1);
        } else {
            self.diagnostics.empty_receives = self.diagnostics.empty_receives.saturating_add(1);
        }
        Ok(event)
    }
}
