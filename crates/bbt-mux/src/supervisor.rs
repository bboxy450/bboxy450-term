//! One actor owns the PTY masters, child handles, sequence counters and writers.

use std::collections::{BTreeMap, HashMap};
use std::io::{self, Read};
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token, Waker};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use sha2::{Digest, Sha256};
use ulid::Ulid;

use crate::protocol::*;
use crate::transport::{BoundSocket, SocketPeer};

const WAKE: Token = Token(0);
const LISTENER: Token = Token(1);
const CHANNEL_CAPACITY: usize = 128;
const MAX_CLIENTS: usize = 8;
const MAX_INPUT_IDS: usize = 4096;
const TICK: Duration = Duration::from_millis(20);
const CALL_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Config {
    /// None is the v0.1 in-process transport; Some enables the same protocol on Unix IPC.
    pub socket_path: Option<PathBuf>,
    pub max_seats: usize,
    /// A partially written input times out as unknown; it is never resubmitted.
    pub write_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket_path: None,
            max_seats: 1024,
            write_timeout: Duration::from_secs(5),
        }
    }
}

/// Owning this value keeps the single supervisor thread alive. Dropping clients
/// does not kill seats. Dropping the supervisor explicitly terminates/reaps them.
pub struct Supervisor {
    tx: mpsc::SyncSender<Control>,
    waker: Arc<Waker>,
    thread: Option<JoinHandle<io::Result<()>>>,
    capability: [u8; 32],
}

impl Supervisor {
    pub fn start(config: Config) -> io::Result<Self> {
        if config.max_seats == 0 || config.max_seats > 1024 || config.write_timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid supervisor limits",
            ));
        }
        let poll = Poll::new()?;
        let waker = Arc::new(Waker::new(poll.registry(), WAKE)?);
        let mut socket = config
            .socket_path
            .as_deref()
            .map(BoundSocket::bind)
            .transpose()?;
        if let Some(socket) = socket.as_mut() {
            poll.registry()
                .register(&mut socket.listener, LISTENER, Interest::READABLE)?;
        }
        let mut capability = [0; 32];
        getrandom::fill(&mut capability).map_err(io::Error::other)?;
        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let mut actor = Actor {
            poll,
            socket,
            config,
            capability,
            boot: Ulid::new(),
            rx,
            clients: HashMap::new(),
            routes: HashMap::new(),
            seats: BTreeMap::new(),
            active: None,
            next_token: 2,
        };
        let thread = thread::Builder::new()
            .name("bbt-supervisor".into())
            .spawn(move || actor.run())?;
        Ok(Self {
            tx,
            waker,
            thread: Some(thread),
            capability,
        })
    }

    /// The startup owner passes this capability out of band, never via argv/logs.
    pub fn socket_capability(&self) -> [u8; 32] {
        self.capability
    }

    pub fn connect(&self) -> Result<Client, Failure> {
        let id = Ulid::new();
        let (events_tx, events_rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        self.tx
            .send(Control::Register {
                id,
                events: events_tx,
                reply: reply_tx,
            })
            .map_err(|_| disconnected())?;
        self.waker.wake()?;
        reply_rx
            .recv_timeout(CALL_TIMEOUT)
            .map_err(|_| disconnected())??;
        let mut client = Client {
            id,
            lease: Ulid::nil(),
            tx: self.tx.clone(),
            waker: self.waker.clone(),
            events: events_rx,
        };
        match client.request(Action::AcquireLogic {
            capability: self.capability,
        })? {
            Reply::Acquired { logic_lease, .. } => client.lease = logic_lease,
            _ => return Err(disconnected()),
        }
        Ok(client)
    }

    pub fn shutdown(mut self) -> io::Result<()> {
        self.stop()
    }

    fn stop(&mut self) -> io::Result<()> {
        if let Some(thread) = self.thread.take() {
            let _ = self.tx.send(Control::Shutdown);
            let _ = self.waker.wake();
            thread
                .join()
                .map_err(|_| io::Error::other("supervisor thread panicked"))??;
        }
        Ok(())
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

pub struct Client {
    id: Ulid,
    lease: Ulid,
    tx: mpsc::SyncSender<Control>,
    waker: Arc<Waker>,
    events: mpsc::Receiver<Event>,
}

impl Client {
    pub fn logic_lease(&self) -> Ulid {
        self.lease
    }

    pub fn request(&self, action: Action) -> Result<Reply, Failure> {
        self.request_frame(Frame::new(Request::new(Some(self.lease), action)))
    }

    pub fn request_frame(&self, frame: Frame<Request>) -> Result<Reply, Failure> {
        let (reply, rx) = mpsc::sync_channel(1);
        self.tx
            .send(Control::Request {
                client: self.id,
                frame,
                reply,
            })
            .map_err(|_| disconnected())?;
        self.waker.wake()?;
        rx.recv_timeout(CALL_TIMEOUT).map_err(|_| {
            Failure::new(
                FailureCode::Io,
                "reply unavailable; operation outcome unknown, query before retry",
            )
        })?
    }

    pub fn spawn(&self, spec: LaunchSpec) -> Result<SeatInfo, Failure> {
        match self.request(Action::Spawn { spec })? {
            Reply::Spawned { info } => Ok(info),
            _ => Err(disconnected()),
        }
    }

    pub fn attach(&self, seat: Ulid, generation: u64) -> Result<Attachment, Failure> {
        match self.request(Action::Attach { seat, generation })? {
            Reply::Attached { attachment, .. } => Ok(attachment),
            _ => Err(disconnected()),
        }
    }

    pub fn input(
        &self,
        attachment: &Attachment,
        input_id: Ulid,
        bytes: &[u8],
    ) -> Result<InputReceipt, Failure> {
        match self.request(Action::Input {
            attachment: attachment.clone(),
            input_id,
            bytes: bytes.into(),
        })? {
            Reply::Input { receipt } => Ok(receipt),
            _ => Err(disconnected()),
        }
    }

    pub fn resize(&self, attachment: &Attachment, size: SeatSize) -> Result<Reply, Failure> {
        self.request(Action::Resize {
            attachment: attachment.clone(),
            size,
        })
    }

    /// Asynchronously requests SIGHUP, escalating to SIGKILL after 250 ms.
    pub fn kill(&self, attachment: &Attachment) -> Result<Reply, Failure> {
        self.request(Action::Kill {
            attachment: attachment.clone(),
        })
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<Event, mpsc::RecvTimeoutError> {
        self.events.recv_timeout(timeout)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Never block a caller's drop. A disconnected event receiver is also
        // detected by the next attempted publication if this queue is full.
        let _ = self.tx.try_send(Control::Disconnect(self.id));
        let _ = self.waker.wake();
    }
}

enum Control {
    Register {
        id: Ulid,
        events: mpsc::SyncSender<Event>,
        reply: mpsc::SyncSender<Result<(), Failure>>,
    },
    Request {
        client: Ulid,
        frame: Frame<Request>,
        reply: mpsc::SyncSender<Result<Reply, Failure>>,
    },
    Disconnect(Ulid),
    Shutdown,
}

enum Target {
    Local(mpsc::SyncSender<Event>),
    Socket { token: Token, peer: SocketPeer },
}

#[derive(Clone, Copy)]
enum Route {
    Seat(Ulid),
    Socket(Ulid),
}

struct Actor {
    poll: Poll,
    socket: Option<BoundSocket>,
    config: Config,
    capability: [u8; 32],
    boot: Ulid,
    rx: mpsc::Receiver<Control>,
    clients: HashMap<Ulid, Target>,
    routes: HashMap<Token, Route>,
    seats: BTreeMap<Ulid, Seat>,
    active: Option<(Ulid, Ulid)>,
    next_token: usize,
}

impl Actor {
    fn token(&mut self) -> Token {
        let token = Token(self.next_token);
        self.next_token = self
            .next_token
            .checked_add(1)
            .expect("poll token exhausted");
        token
    }

    fn run(&mut self) -> io::Result<()> {
        let mut events = Events::with_capacity(256);
        loop {
            for _ in 0..64 {
                match self.rx.try_recv() {
                    Ok(Control::Shutdown) => return Ok(()),
                    Ok(Control::Disconnect(id)) => self.disconnect(id),
                    Ok(Control::Register { id, events, reply }) => {
                        let result = if self.clients.len() < MAX_CLIENTS {
                            self.clients.insert(id, Target::Local(events));
                            Ok(())
                        } else {
                            Err(Failure::new(
                                FailureCode::Limit,
                                "too many logic connections",
                            ))
                        };
                        let _ = reply.send(result);
                    }
                    Ok(Control::Request {
                        client,
                        frame,
                        reply,
                    }) => {
                        let result = self.dispatch(client, frame);
                        let _ = reply.send(result);
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                }
            }
            // Every seat is driven here; no reader/writer/wait thread per seat.
            let ids: Vec<_> = self.seats.keys().copied().collect();
            for id in ids {
                let mut seat = self.seats.remove(&id).unwrap();
                seat.tick(self)?;
                self.seats.insert(id, seat);
            }
            self.flush_sockets();
            let ready = self.seats.values().any(|seat| {
                seat.read_ready
                    && seat.pending_event.is_none()
                    && seat
                        .ring
                        .as_ref()
                        .map_or(seat.writer.is_some(), crate::ring::Ring::can_capture)
                    && !seat.exit_published
            });
            match self
                .poll
                .poll(&mut events, Some(if ready { Duration::ZERO } else { TICK }))
            {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
            for event in &events {
                match event.token() {
                    WAKE => {}
                    LISTENER => self.accept_sockets()?,
                    token => match self.routes.get(&token).copied() {
                        Some(Route::Seat(id)) => {
                            if let Some(seat) = self.seats.get_mut(&id) {
                                if event.is_readable() || event.is_read_closed() || event.is_error()
                                {
                                    seat.read_ready = true;
                                }
                                if event.is_writable() {
                                    seat.write_ready = true;
                                }
                            }
                        }
                        Some(Route::Socket(id)) => self.read_socket(id),
                        None => {}
                    },
                }
            }
        }
    }

    fn dispatch(&mut self, client: Ulid, frame: Frame<Request>) -> Result<Reply, Failure> {
        if frame.version != VERSION {
            return Err(Failure::new(FailureCode::Version, "unsupported version"));
        }
        if !self.clients.contains_key(&client) {
            return Err(disconnected());
        }
        let request = frame.body;
        if request.request_id == Ulid::nil() {
            return Err(Failure::new(FailureCode::Invalid, "nil request id"));
        }
        if let Action::AcquireLogic { capability } = request.action {
            // Constant-time comparison of the fixed-width startup capability.
            let difference = capability
                .iter()
                .zip(self.capability)
                .fold(0, |acc, (a, b)| acc | (a ^ b));
            if difference != 0 {
                return Err(Failure::new(
                    FailureCode::Unauthorized,
                    "invalid startup capability",
                ));
            }
            let lease = Ulid::new();
            self.active = Some((client, lease));
            for seat in self.seats.values_mut() {
                seat.writer = None;
            }
            return Ok(Reply::Acquired {
                logic_lease: lease,
                supervisor_boot: self.boot,
            });
        }
        if self.active.is_none() || self.active != request.logic_lease.map(|lease| (client, lease))
        {
            return Err(Failure::new(
                FailureCode::StaleLease,
                "logic lease was replaced or disconnected",
            ));
        }
        match request.action {
            Action::AcquireLogic { .. } => unreachable!(),
            Action::ListOwnedSeats => Ok(Reply::Seats {
                seats: self.seats.values().map(Seat::info).collect(),
            }),
            Action::Spawn { spec } => self.spawn_seat(spec),
            Action::Attach { seat, generation } => {
                let seat = self.seat_mut(seat, generation)?;
                let attachment = Attachment {
                    seat: seat.spec.seat,
                    generation,
                    writer_lease: Ulid::new(),
                };
                seat.writer = Some((client, attachment.writer_lease));
                Ok(Reply::Attached {
                    attachment,
                    info: seat.info(),
                    first_available_sequence: seat
                        .pending_event
                        .as_ref()
                        .and_then(Event::sequence)
                        .unwrap_or(seat.next_seq),
                })
            }
            Action::AttachRing {
                seat,
                generation,
                directory,
                settings,
            } => {
                let seat = self.seat_mut(seat, generation)?;
                if !directory.is_absolute() {
                    return Err(Failure::new(
                        FailureCode::Invalid,
                        "ring directory must be absolute",
                    ));
                }
                if let Some(ring) = &seat.ring {
                    if ring.directory() != directory || ring.settings() != &settings {
                        return Err(Failure::new(
                            FailureCode::Invalid,
                            "ring owner/configuration conflicts",
                        ));
                    }
                } else {
                    let ring = crate::ring::Ring::open(
                        &directory,
                        seat.spec.seat,
                        generation,
                        seat.next_seq - 1,
                        settings,
                    )
                    .map_err(pty_error)?;
                    if ring.status().committed_sequence != seat.next_seq - 1 {
                        return Err(Failure::new(
                            FailureCode::StaleGeneration,
                            "retained ring is from an earlier supervisor/process; resume needs a new generation",
                        ));
                    }
                    seat.ring = Some(ring);
                }
                let attachment = Attachment {
                    seat: seat.spec.seat,
                    generation,
                    writer_lease: Ulid::new(),
                };
                seat.writer = Some((client, attachment.writer_lease));
                Ok(Reply::Attached {
                    attachment,
                    info: seat.info(),
                    first_available_sequence: seat.ring.as_ref().unwrap().status().first_sequence,
                })
            }
            Action::RingRecovery { attachment } => {
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                let ring = seat
                    .ring
                    .as_ref()
                    .ok_or_else(|| Failure::new(FailureCode::NotFound, "no ring configured"))?;
                Ok(Reply::RingRecovery {
                    status: ring.status(),
                    checkpoint: ring.latest_checkpoint().map_err(pty_error)?,
                })
            }
            Action::ReadRing {
                attachment,
                from,
                to,
            } => {
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                let ring = seat
                    .ring
                    .as_ref()
                    .ok_or_else(|| Failure::new(FailureCode::NotFound, "no ring configured"))?;
                Ok(Reply::RingHistory {
                    history: Box::new(ring.history(from, to).map_err(pty_error)?),
                })
            }
            Action::PublishCheckpoint {
                attachment,
                checkpoint,
            } => {
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                let ring = seat
                    .ring
                    .as_mut()
                    .ok_or_else(|| Failure::new(FailureCode::NotFound, "no ring configured"))?;
                let sequence = crate::ring::Saved::decode(&checkpoint)
                    .map_err(pty_error)?
                    .sequence;
                ring.checkpoint(checkpoint).map_err(pty_error)?;
                Ok(Reply::CheckpointCommitted { sequence })
            }
            Action::Input {
                attachment,
                input_id,
                bytes,
            } => {
                let timeout = self.config.write_timeout;
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                let receipt = seat.accept_input(input_id, bytes, timeout)?;
                Ok(Reply::Input { receipt })
            }
            Action::InputStatus {
                seat,
                generation,
                input_id,
            } => {
                let seat = self.seat_mut(seat, generation)?;
                let record = seat
                    .inputs
                    .get(&input_id)
                    .ok_or_else(|| Failure::new(FailureCode::NotFound, "input id not accepted"))?;
                Ok(Reply::Input {
                    receipt: record.receipt.clone(),
                })
            }
            Action::Resize { attachment, size } => {
                size.validate()?;
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                seat.check_live()?;
                if seat.pending_event.is_some() {
                    return Err(Failure::new(
                        FailureCode::Busy,
                        "drain pending records before resize",
                    ));
                }
                if seat.ring.as_ref().is_some_and(|ring| !ring.can_control()) {
                    return Err(Failure::new(
                        FailureCode::Busy,
                        "ring full: checkpoint required before resize",
                    ));
                }
                seat.master.resize(pty_size(size)).map_err(pty_error)?;
                seat.size = size;
                let sequence = seat.next_seq;
                let event = Event::Resized {
                    seat: attachment.seat,
                    generation: attachment.generation,
                    seq: sequence,
                    size,
                };
                seat.capture(event).map_err(pty_error)?;
                Ok(Reply::Resized { size, sequence })
            }
            Action::Kill { attachment } => {
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                seat.check_live()?;
                if seat.terminate_at.is_none() {
                    signal(seat.pid, libc::SIGHUP)?;
                    seat.terminate_at = Some(Instant::now() + Duration::from_millis(250));
                }
                Ok(Reply::Terminating)
            }
            Action::ScheduleClose {
                attachment,
                close_id,
                deadline_ms,
            } => {
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                if let Some((deadline, canceled)) = seat.close_ids.get(&close_id) {
                    if *canceled || *deadline != deadline_ms {
                        return Err(Failure::new(
                            FailureCode::Invalid,
                            "close id was canceled or conflicts",
                        ));
                    }
                    return Ok(Reply::CloseScheduled {
                        close_id,
                        deadline_ms: *deadline,
                    });
                }
                seat.check_live()?;
                let now = crate::store::now_ms();
                if close_id == Ulid::nil()
                    || deadline_ms.saturating_sub(now) > 86_400_000
                    || seat.close_ids.len() >= 4096
                    || seat.grace.is_some()
                {
                    return Err(Failure::new(
                        FailureCode::Invalid,
                        "invalid or conflicting close",
                    ));
                }
                seat.grace = Some((
                    close_id,
                    Instant::now() + Duration::from_millis(deadline_ms.saturating_sub(now)),
                ));
                seat.close_ids.insert(close_id, (deadline_ms, false));
                Ok(Reply::CloseScheduled {
                    close_id,
                    deadline_ms,
                })
            }
            Action::UndoClose {
                attachment,
                close_id,
            } => {
                let seat = self.seat_mut(attachment.seat, attachment.generation)?;
                seat.check_writer(client, &attachment)?;
                if seat
                    .close_ids
                    .get(&close_id)
                    .is_some_and(|(_, canceled)| *canceled)
                {
                    return Ok(Reply::CloseUndone { close_id });
                }
                seat.check_live()?;
                match seat.grace {
                    Some((id, deadline)) if id == close_id && Instant::now() < deadline => {
                        seat.grace = None;
                        if let Some((_, canceled)) = seat.close_ids.get_mut(&close_id) {
                            *canceled = true;
                        }
                        Ok(Reply::CloseUndone { close_id })
                    }
                    _ => Err(Failure::new(
                        FailureCode::Exited,
                        "close expired or revision changed",
                    )),
                }
            }
        }
    }

    fn seat_mut(&mut self, id: Ulid, generation: u64) -> Result<&mut Seat, Failure> {
        let seat = self
            .seats
            .get_mut(&id)
            .ok_or_else(|| Failure::new(FailureCode::NotFound, "seat not owned"))?;
        if seat.spec.generation != generation {
            return Err(Failure::new(
                FailureCode::StaleGeneration,
                "process generation changed",
            ));
        }
        Ok(seat)
    }

    fn spawn_seat(&mut self, spec: LaunchSpec) -> Result<Reply, Failure> {
        spec.validate()?;
        if let Some(old) = self.seats.get(&spec.seat) {
            // The exact retry returns the original child, even after its exit.
            if old.spec == spec {
                return Ok(Reply::Spawned { info: old.info() });
            }
            if spec.generation != old.spec.generation.checked_add(1).unwrap_or(0) {
                return Err(Failure::new(
                    FailureCode::StaleGeneration,
                    "replacement needs the next generation",
                ));
            }
            if !old.exit_published || old.pending_event.is_some() {
                return Err(Failure::new(
                    FailureCode::Busy,
                    "old process or its final records are still live",
                ));
            }
        } else if spec.generation != 1 {
            return Err(Failure::new(
                FailureCode::StaleGeneration,
                "first process generation must be 1",
            ));
        } else if self.seats.len() >= self.config.max_seats {
            return Err(Failure::new(FailureCode::Limit, "machine seat cap reached"));
        }
        let token = self.token();
        let seat = Seat::spawn(spec, token, self.poll.registry())?;
        let info = seat.info();
        if let Some(old) = self.seats.insert(info.seat, seat) {
            self.routes.remove(&old.token);
        }
        self.routes.insert(token, Route::Seat(info.seat));
        Ok(Reply::Spawned { info })
    }

    fn notify(&mut self, client: Ulid, event: &Event) -> bool {
        match self.clients.get_mut(&client) {
            Some(Target::Local(tx)) => match tx.try_send(event.clone()) {
                Ok(()) => true,
                Err(mpsc::TrySendError::Full(_)) => false,
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    self.disconnect(client);
                    false
                }
            },
            Some(Target::Socket { peer, .. }) => peer
                .enqueue(Message::Event {
                    event: event.clone(),
                })
                .unwrap_or(false),
            None => false,
        }
    }

    fn disconnect(&mut self, id: Ulid) {
        if let Some(Target::Socket { token, mut peer }) = self.clients.remove(&id) {
            let _ = self.poll.registry().deregister(&mut peer.stream);
            self.routes.remove(&token);
        }
        if self.active.is_some_and(|(client, _)| client == id) {
            self.active = None;
            for seat in self.seats.values_mut() {
                seat.writer = None;
            }
        }
    }

    fn accept_sockets(&mut self) -> io::Result<()> {
        loop {
            let accepted = self.socket.as_mut().unwrap().listener.accept();
            let (mut stream, _) = match accepted {
                Ok(pair) => pair,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            if self.clients.len() >= MAX_CLIENTS {
                continue;
            }
            let id = Ulid::new();
            let token = self.token();
            self.poll
                .registry()
                .register(&mut stream, token, Interest::READABLE)?;
            self.clients.insert(
                id,
                Target::Socket {
                    token,
                    peer: SocketPeer::new(stream),
                },
            );
            self.routes.insert(token, Route::Socket(id));
        }
    }

    fn read_socket(&mut self, id: Ulid) {
        // Drain to WouldBlock; decoder retains partial headers/payloads.
        loop {
            let mut bytes = [0; 8192];
            let frames = match self.clients.get_mut(&id) {
                Some(Target::Socket { peer, .. }) => match peer.stream.read(&mut bytes) {
                    Ok(0) => {
                        self.disconnect(id);
                        return;
                    }
                    Ok(count) => peer.decoder.push(&bytes[..count]),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => Err(error),
                },
                _ => return,
            };
            let frames = match frames {
                Ok(frames) => frames,
                Err(_) => {
                    self.disconnect(id);
                    return;
                }
            };
            for frame in frames {
                let request_id = frame.body.request_id;
                let result = self.dispatch(id, frame);
                let queued = match self.clients.get_mut(&id) {
                    Some(Target::Socket { peer, .. }) => peer
                        .enqueue(Message::Reply { request_id, result })
                        .unwrap_or(false),
                    _ => false,
                };
                if !queued {
                    self.disconnect(id);
                    return;
                }
            }
        }
    }

    fn flush_sockets(&mut self) {
        let mut failed = Vec::new();
        for (&id, target) in &mut self.clients {
            if let Target::Socket { token, peer } = target {
                let result = peer.flush().and_then(|()| {
                    let interest = if peer.wants_write() {
                        Interest::READABLE | Interest::WRITABLE
                    } else {
                        Interest::READABLE
                    };
                    self.poll
                        .registry()
                        .reregister(&mut peer.stream, *token, interest)
                });
                if result.is_err() {
                    failed.push(id);
                }
            }
        }
        for id in failed {
            self.disconnect(id);
        }
    }
}

struct StoredInput {
    digest: [u8; 32],
    receipt: InputReceipt,
}

struct PendingWrite {
    id: Ulid,
    bytes: Vec<u8>,
    deadline: Instant,
}

struct Seat {
    spec: LaunchSpec,
    size: SeatSize,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    pid: u32,
    fd: RawFd,
    token: Token,
    registered: bool,
    read_ready: bool,
    write_ready: bool,
    eof: bool,
    next_seq: u64,
    writer: Option<(Ulid, Ulid)>,
    pending_event: Option<Event>,
    pending_write: Option<PendingWrite>,
    inputs: HashMap<Ulid, StoredInput>,
    exited: Option<ExitInfo>,
    exit_published: bool,
    terminate_at: Option<Instant>,
    termination_escalated: bool,
    grace: Option<(Ulid, Instant)>,
    close_ids: HashMap<Ulid, (u64, bool)>,
    ring: Option<crate::ring::Ring>,
}

impl Seat {
    fn spawn(spec: LaunchSpec, token: Token, registry: &mio::Registry) -> Result<Self, Failure> {
        let pair = native_pty_system()
            .openpty(pty_size(spec.size))
            .map_err(pty_error)?;
        let fd = pair
            .master
            .as_raw_fd()
            .ok_or_else(|| Failure::new(FailureCode::Io, "PTY has no raw fd"))?;
        set_nonblocking(fd)?;
        registry.register(&mut SourceFd(&fd), token, Interest::READABLE)?;
        let mut command = CommandBuilder::new(&spec.program);
        command.args(&spec.args);
        command.cwd(&spec.cwd);
        command.env("TERM", "xterm-256color");
        for (key, value) in &spec.env {
            command.env(key, value);
        }
        command.env("BBOXY_SEAT_ID", spec.seat.to_string());
        command.env("BBOXY_PROCESS_GENERATION", spec.generation.to_string());
        let child = pair.slave.spawn_command(command).map_err(pty_error)?;
        drop(pair.slave);
        let pid = child
            .process_id()
            .ok_or_else(|| Failure::new(FailureCode::Io, "child has no pid"))?;
        Ok(Self {
            size: spec.size,
            spec,
            master: pair.master,
            child,
            pid,
            fd,
            token,
            registered: true,
            read_ready: false,
            write_ready: true,
            eof: false,
            next_seq: 1,
            writer: None,
            pending_event: None,
            pending_write: None,
            inputs: HashMap::new(),
            exited: None,
            exit_published: false,
            terminate_at: None,
            termination_escalated: false,
            grace: None,
            close_ids: HashMap::new(),
            ring: None,
        })
    }

    fn info(&self) -> SeatInfo {
        SeatInfo {
            seat: self.spec.seat,
            generation: self.spec.generation,
            pid: self.pid,
            size: self.size,
            next_sequence: self.next_seq,
            exited: self.exited.clone(),
        }
    }

    fn sequence(&mut self) -> u64 {
        let seq = self.next_seq;
        self.next_seq = self
            .next_seq
            .checked_add(1)
            .expect("seat sequence exhausted");
        seq
    }

    fn check_live(&self) -> Result<(), Failure> {
        if self.exited.is_some() || self.eof || self.terminate_at.is_some() {
            return Err(Failure::new(
                FailureCode::Exited,
                "child exited or termination is pending",
            ));
        }
        Ok(())
    }

    fn check_writer(&self, client: Ulid, attachment: &Attachment) -> Result<(), Failure> {
        if self.writer != Some((client, attachment.writer_lease)) {
            return Err(Failure::new(
                FailureCode::StaleWriter,
                "writer attachment was replaced",
            ));
        }
        Ok(())
    }

    fn accept_input(
        &mut self,
        id: Ulid,
        bytes: Vec<u8>,
        timeout: Duration,
    ) -> Result<InputReceipt, Failure> {
        if id == Ulid::nil() || bytes.is_empty() || bytes.len() > MAX_INPUT_BYTES {
            return Err(Failure::new(
                FailureCode::Invalid,
                "input needs a nonnil id and 1..65536 bytes",
            ));
        }
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        if let Some(stored) = self.inputs.get(&id) {
            if stored.digest != digest {
                return Err(Failure::new(
                    FailureCode::InputConflict,
                    "input id reused for different bytes",
                ));
            }
            return Ok(stored.receipt.clone());
        }
        self.check_live()?;
        if self.pending_write.is_some() {
            return Err(Failure::new(
                FailureCode::Busy,
                "a previous input is still in flight",
            ));
        }
        if self.inputs.len() >= MAX_INPUT_IDS {
            return Err(Failure::new(
                FailureCode::Limit,
                "input deduplication capacity reached; ids are never silently evicted",
            ));
        }
        let receipt = InputReceipt {
            input_id: id,
            state: InputState::Pending,
            bytes_written: 0,
            bytes_total: bytes.len(),
        };
        self.inputs.insert(id, StoredInput { digest, receipt });
        self.pending_write = Some(PendingWrite {
            id,
            bytes,
            deadline: Instant::now() + timeout,
        });
        self.write_ready = true;
        self.write_pending();
        Ok(self.inputs[&id].receipt.clone())
    }

    fn write_pending(&mut self) {
        let Some(pending) = self.pending_write.as_ref() else {
            return;
        };
        let stored = self.inputs.get_mut(&pending.id).unwrap();
        if self.exited.is_some() || self.eof || Instant::now() >= pending.deadline {
            stored.receipt.state = InputState::PartialUnknown;
            self.pending_write = None;
            return;
        }
        if !self.write_ready {
            return;
        }
        while stored.receipt.bytes_written < pending.bytes.len() {
            let remaining = &pending.bytes[stored.receipt.bytes_written..];
            // SAFETY: master owns fd for this call; remaining is a valid slice.
            let count = unsafe { libc::write(self.fd, remaining.as_ptr().cast(), remaining.len()) };
            if count > 0 {
                stored.receipt.bytes_written += count as usize;
                continue;
            }
            let error = io::Error::last_os_error();
            if count < 0 && error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if count < 0 && error.kind() == io::ErrorKind::WouldBlock {
                self.write_ready = false;
                return;
            }
            stored.receipt.state = InputState::PartialUnknown;
            self.pending_write = None;
            return;
        }
        stored.receipt.state = InputState::PtyWritten;
        self.pending_write = None;
    }

    fn publish_pending(&mut self, actor: &mut Actor) {
        if let Some((client, _)) = self.writer
            && actor.active.is_some_and(|(active, _)| active == client)
            && let Some(event) = self.pending_event.as_ref()
            && actor.notify(client, event)
        {
            self.pending_event = None;
        }
        if self.ring.is_some() && self.writer.is_none() {
            // Already committed; the replacement logic replays from the ring.
            self.pending_event = None;
        }
    }

    fn capture(&mut self, event: Event) -> io::Result<()> {
        if let Some(ring) = self.ring.as_mut() {
            ring.append_event(&event).map_err(io::Error::other)?;
        }
        self.sequence();
        self.pending_event = Some(event);
        Ok(())
    }

    fn tick(&mut self, actor: &mut Actor) -> io::Result<()> {
        if self.exited.is_none()
            && self.terminate_at.is_none()
            && self
                .grace
                .is_some_and(|(_, deadline)| Instant::now() >= deadline)
        {
            signal(self.pid, libc::SIGHUP).map_err(io::Error::other)?;
            self.terminate_at = Some(Instant::now() + Duration::from_millis(250));
        }
        if self
            .writer
            .is_some_and(|(client, _)| !actor.active.is_some_and(|(active, _)| active == client))
        {
            self.writer = None;
        }
        if self.exited.is_none() {
            if let Some(status) = self.child.try_wait()? {
                self.exited = Some(ExitInfo {
                    code: status.exit_code(),
                    signal: status.signal().map(str::to_owned),
                });
                self.read_ready = true;
            } else if !self.termination_escalated
                && self.terminate_at.is_some_and(|at| Instant::now() >= at)
            {
                signal(self.pid, libc::SIGKILL)?;
                self.termination_escalated = true;
            }
        }
        self.write_pending();
        self.publish_pending(actor);
        // One pending output chunk per seat bounds capture without a ring. A
        // stalled/detached logic applies kernel PTY backpressure, not silent loss.
        for _ in 0..4 {
            if let Some(ring) = self.ring.as_mut() {
                ring.prepare_capture().map_err(io::Error::other)?;
            }
            if !self.read_ready
                || self.pending_event.is_some()
                || self
                    .ring
                    .as_ref()
                    .map_or(self.writer.is_none(), |ring| !ring.can_capture())
                || self.exit_published
            {
                break;
            }
            let mut bytes = [0; OUTPUT_CHUNK_BYTES];
            let capacity = self
                .ring
                .as_ref()
                .map_or(OUTPUT_CHUNK_BYTES, crate::ring::Ring::capture_capacity);
            // SAFETY: master owns fd; bytes is writable for its stated length.
            let count = unsafe { libc::read(self.fd, bytes.as_mut_ptr().cast(), capacity) };
            if count > 0 {
                let seq = self.next_seq;
                self.capture(Event::Output {
                    seat: self.spec.seat,
                    generation: self.spec.generation,
                    seq,
                    bytes: bytes[..count as usize].into(),
                })?;
                self.publish_pending(actor);
                continue;
            }
            let error = io::Error::last_os_error();
            if count < 0 && error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            self.read_ready = false;
            if count == 0 || error.raw_os_error() == Some(libc::EIO) {
                self.eof = true;
            } else if error.kind() != io::ErrorKind::WouldBlock {
                return Err(error);
            }
            break;
        }
        if let Some(status) = self.exited.clone()
            && !self.read_ready
            && self.pending_event.is_none()
            && !self.exit_published
            && self
                .ring
                .as_ref()
                .is_none_or(crate::ring::Ring::can_control)
        {
            let seq = self.next_seq;
            self.capture(Event::Exited {
                seat: self.spec.seat,
                generation: self.spec.generation,
                seq,
                status,
            })?;
            self.exit_published = true;
            self.publish_pending(actor);
        }
        if self.eof || self.exit_published {
            if self.registered {
                actor.poll.registry().deregister(&mut SourceFd(&self.fd))?;
                self.registered = false;
            }
        } else if self.registered {
            actor.poll.registry().reregister(
                &mut SourceFd(&self.fd),
                self.token,
                if self.pending_write.is_some() {
                    Interest::READABLE | Interest::WRITABLE
                } else {
                    Interest::READABLE
                },
            )?;
        }
        Ok(())
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        if self.exited.is_none() {
            let _ = signal(self.pid, libc::SIGKILL);
            let _ = self.child.wait();
        }
    }
}

fn pty_size(size: SeatSize) -> PtySize {
    PtySize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: size.pixel_width,
        pixel_height: size.pixel_height,
    }
}

fn pty_error(error: impl std::fmt::Display) -> Failure {
    Failure::new(FailureCode::Io, error.to_string())
}

fn disconnected() -> Failure {
    Failure::new(FailureCode::Io, "supervisor connection unavailable")
}

pub(crate) fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    // SAFETY: callers keep the owning PTY/socket alive. F_GETFL/F_SETFL operate
    // on this descriptor only; no pointers or borrowed ownership are created.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn signal(pid: u32, signal: libc::c_int) -> io::Result<()> {
    // SAFETY: pid comes from this actor's unreaped child, never from a request.
    if unsafe { libc::kill(pid as libc::pid_t, signal) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
