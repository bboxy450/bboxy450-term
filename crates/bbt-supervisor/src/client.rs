//! Blocking supervisor clients, shared by in-process and separate-process logic.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::store::{Result, invalid};
use crate::{Action, Client, Event, Failure, Frame, Message, Reply, Ulid};

#[cfg(test)]
mod tests;

pub trait SupervisorConnection: Send {
    fn request(&mut self, action: Action) -> Result<Reply>;
    fn receive(&mut self, timeout: Duration) -> Result<Option<Event>>;
}
impl SupervisorConnection for Client {
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
    /// Coalesce captured seat/generation/kind slots; raw prefixes stay FIFO and exits are never replaced.
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

/// The same logic can run in a separate process against W2.1's private socket.
pub struct SocketSupervisor {
    stream: std::os::unix::net::UnixStream,
    decoder: crate::transport::Decoder<Message>,
    events: PendingEvents,
    replies: BTreeMap<Ulid, std::result::Result<Reply, Failure>>,
    lease: Option<Ulid>,
}
impl SocketSupervisor {
    pub fn connect(path: &Path, capability: [u8; 32]) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let stream = std::os::unix::net::UnixStream::connect(path)?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut client = Self {
            stream,
            decoder: Default::default(),
            events: PendingEvents::default(),
            replies: BTreeMap::new(),
            lease: None,
        };
        match client.request(Action::AcquireLogic { capability })? {
            Reply::Acquired { logic_lease, .. } => client.lease = Some(logic_lease),
            _ => return Err(invalid("supervisor did not admit logic")),
        }
        Ok(client)
    }
    fn read(&mut self, timeout: Duration) -> Result<()> {
        self.stream
            .set_read_timeout(Some(timeout.max(Duration::from_millis(1))))?;
        let mut bytes = [0; 8192];
        let count = match self.stream.read(&mut bytes) {
            Ok(0) => return Err(invalid("supervisor disconnected")),
            Ok(n) => n,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };
        for frame in self.decoder.push(&bytes[..count])? {
            match frame.body {
                Message::Event { event } => self.events.push(event)?,
                Message::Reply { request_id, result } => {
                    if self.replies.len() >= 32 {
                        return Err(invalid("unexpected reply backlog"));
                    }
                    self.replies.insert(request_id, result);
                }
            }
        }
        Ok(())
    }
}
impl SupervisorConnection for SocketSupervisor {
    fn request(&mut self, action: Action) -> Result<Reply> {
        bbt_rt::assert_not_ui_thread();
        let ring = match &action {
            Action::AttachRing {
                seat, generation, ..
            } => Some((*seat, *generation)),
            _ => None,
        };
        let request = crate::Request::new(self.lease, action);
        let id = request.request_id;
        self.stream
            .write_all(&crate::transport::encode(&Frame::new(request))?)?;
        let until = Instant::now() + Duration::from_secs(5);
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
            if Instant::now() >= until {
                return Err(invalid(
                    "supervisor operation outcome unknown; query original id",
                ));
            }
            self.read(Duration::from_millis(20))?;
        }
    }
    fn receive(&mut self, timeout: Duration) -> Result<Option<Event>> {
        bbt_rt::assert_not_ui_thread();
        if let Some(event) = self.events.pop_front() {
            return Ok(Some(event));
        }
        self.read(timeout)?;
        Ok(self.events.pop_front())
    }
}
