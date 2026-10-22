//! Blocking supervisor clients, shared by in-process and separate-process logic.

use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::store::{Result, invalid};
use crate::{Action, Client, Event, Failure, Frame, Message, Reply, Ulid};

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

/// The same logic can run in a separate process against W2.1's private socket.
pub struct SocketSupervisor {
    stream: std::os::unix::net::UnixStream,
    decoder: crate::transport::Decoder<Message>,
    events: VecDeque<Event>,
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
            events: VecDeque::new(),
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
                Message::Event { event } => {
                    if self.events.len() >= 1024 {
                        return Err(invalid(
                            "supervisor event backlog exceeded; continuity unavailable",
                        ));
                    }
                    self.events.push_back(event);
                }
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
        let request = crate::Request::new(self.lease, action);
        let id = request.request_id;
        self.stream
            .write_all(&crate::transport::encode(&Frame::new(request))?)?;
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(reply) = self.replies.remove(&id) {
                return Ok(reply?);
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
