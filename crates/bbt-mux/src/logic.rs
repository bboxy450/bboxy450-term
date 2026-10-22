//! Restartable IO/logic owner. The UI gets an async bounded client, not a DB or PTY.

use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use bbt_core::{Checkpoint, Term};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use ulid::Ulid;

use crate::bus::{Broker, BusHandle};
use crate::index::{FileWatch, Index};
use crate::registry::{Lifecycle, Position, Registry, SeatRecord};
use crate::store::{
    Blobs, Result, Store, blobs::LocalBlobs, events::Draft, invalid, now_ms, object_dir,
    sqlite::SqliteStore,
};
use crate::streams::{HistoryGap, Screen, Snapshot, View};
use crate::{
    Action, Attachment, Client, Event, Failure, Frame, LaunchSpec, Message, Reply, SeatInfo,
};

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

#[derive(Clone)]
pub struct LogicConfig {
    pub run_dir: PathBuf,
    pub repository: PathBuf,
}

enum Command {
    Spawn(
        Box<SeatRecord>,
        LaunchSpec,
        oneshot::Sender<Result<SeatInfo>>,
    ),
    Snapshot(Ulid, u64, oneshot::Sender<Result<Snapshot>>),
    Close(Ulid, u64, Ulid, Position, u64, oneshot::Sender<Result<()>>),
    Undo(Ulid, u64, u64, u64, oneshot::Sender<Result<Position>>),
    Record(Ulid, oneshot::Sender<Result<SeatRecord>>),
    Endpoint(Ulid, oneshot::Sender<Result<EndpointInfo>>),
}

/// Private discovery result; never include the admission capability in logs.
#[derive(Clone)]
pub struct EndpointInfo {
    pub path: PathBuf,
    pub generation: u64,
    pub capability: [u8; 32],
}

#[derive(Clone)]
pub struct LogicClient {
    tx: mpsc::Sender<Command>,
    pub bus: BusHandle,
}
impl LogicClient {
    pub async fn spawn(&self, record: SeatRecord, spec: LaunchSpec) -> Result<SeatInfo> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Spawn(Box::new(record), spec, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await
            .map_err(|_| invalid("spawn outcome unknown; query original seat id"))?
    }
    pub async fn snapshot(&self, seat: Ulid, generation: u64) -> Result<Snapshot> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Snapshot(seat, generation, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await.map_err(|_| invalid("logic stopped"))?
    }
    pub async fn record(&self, seat: Ulid) -> Result<SeatRecord> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Record(seat, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await.map_err(|_| invalid("logic stopped"))?
    }
    pub async fn endpoint(&self, seat: Ulid) -> Result<EndpointInfo> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Endpoint(seat, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await.map_err(|_| invalid("logic stopped"))?
    }
    pub async fn close(
        &self,
        seat: Ulid,
        generation: u64,
        operation: Ulid,
        position: Position,
        grace_ms: u64,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Close(
                seat, generation, operation, position, grace_ms, tx,
            ))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await.map_err(|_| invalid("close outcome unknown"))?
    }
    pub async fn undo(
        &self,
        seat: Ulid,
        generation: u64,
        revision: u64,
        layout_revision: u64,
    ) -> Result<Position> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Undo(
                seat,
                generation,
                revision,
                layout_revision,
                tx,
            ))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await.map_err(|_| invalid("undo outcome unknown"))?
    }
    pub fn history_gap(&self, from: u64, to: u64) -> HistoryGap {
        HistoryGap {
            requested_from: from,
            requested_to: to,
            retained_from: None,
            retained_to: None,
            reason:
                "byte ring and semantic history are supplied by W1.2; no earlier range is claimed"
                    .into(),
        }
    }
}

pub struct Logic {
    pub client: LogicClient,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<Result<()>>>,
}
impl Logic {
    /// Bootstrap is blocking and belongs on an IO/bootstrap worker. Steady-state
    /// clients and seat sockets use bounded async messages on the supplied R1 handle.
    pub fn start(
        connection: impl SupervisorConnection + 'static,
        config: LogicConfig,
        key: SigningKey,
        handle: &tokio::runtime::Handle,
    ) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let broker = Broker::open(&config.run_dir, key.clone())?;
        let bus = BusHandle::start(handle, broker, 128)?;
        let (tx, rx) = mpsc::channel(128);
        let client = LogicClient {
            tx,
            bus: bus.clone(),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let runtime = handle.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let worker_client = client.clone();
        let thread = std::thread::Builder::new()
            .name("bbt-logic".into())
            .spawn(move || {
                let mut worker =
                    match Worker::open(Box::new(connection), config, bus, runtime, worker_client) {
                        Ok(worker) => worker,
                        Err(error) => {
                            let _ = ready_tx.send(Err(error.to_string()));
                            return Err(error);
                        }
                    };
                let _ = ready_tx.send(Ok(()));
                let result = worker.run(rx, stopping);
                let _ = worker.handle.block_on(worker.bus.shutdown());
                result
            })?;
        match ready_rx.recv_timeout(Duration::from_secs(30))? {
            Ok(()) => Ok(Self {
                client,
                stop,
                thread: Some(thread),
            }),
            Err(error) => {
                stop.store(true, Ordering::Release);
                let _ = thread.join();
                Err(invalid(error))
            }
        }
    }
    pub fn shutdown(mut self) -> Result<()> {
        bbt_rt::assert_not_ui_thread();
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| invalid("logic panicked"))??;
        }
        Ok(())
    }
}
impl Drop for Logic {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Saved {
    version: u32,
    generation: u64,
    sequence: u64,
    checkpoint: Checkpoint,
    title: String,
    cwd: Option<String>,
    gap: Option<String>,
}
struct ViewState {
    term: Term,
    sequence: u64,
    title: String,
    cwd: Option<String>,
    gap: Option<String>,
    generation: u64,
    view_generation: Ulid,
}

struct Worker {
    connection: Box<dyn SupervisorConnection>,
    config: LogicConfig,
    registry: Registry,
    index: Index,
    watch: FileWatch,
    bus: BusHandle,
    handle: tokio::runtime::Handle,
    views: BTreeMap<Ulid, ViewState>,
    attachments: BTreeMap<Ulid, Attachment>,
    client: LogicClient,
    endpoints: BTreeMap<Ulid, (EndpointInfo, crate::seat_socket::SeatEndpoint)>,
    endpoint_root: PathBuf,
}
impl Worker {
    fn open(
        connection: Box<dyn SupervisorConnection>,
        config: LogicConfig,
        bus: BusHandle,
        handle: tokio::runtime::Handle,
        client: LogicClient,
    ) -> Result<Self> {
        let registry = Registry::new(SqliteStore::open(&object_dir(&config.run_dir, "account")?)?);
        let index = Index::open(
            &config.repository,
            SqliteStore::open(&object_dir(&config.run_dir, "index")?)?,
        )?;
        let watch = FileWatch::start(&config.repository)?;
        // macOS sockaddr_un has a short path bound. A fresh private namespace
        // also avoids stealing any prior logic instance's listener on restart.
        use std::os::unix::fs::DirBuilderExt;
        // SAFETY: geteuid has no pointer arguments or side effects.
        let endpoint_root = PathBuf::from("/tmp").join(format!(
            "bbt-{}-{}",
            unsafe { libc::geteuid() },
            Ulid::new()
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&endpoint_root)?;
        let mut worker = Self {
            connection,
            config,
            registry,
            index,
            watch,
            bus,
            handle,
            views: BTreeMap::new(),
            attachments: BTreeMap::new(),
            client,
            endpoints: BTreeMap::new(),
            endpoint_root,
        };
        if let Reply::Seats { seats } = worker.connection.request(Action::ListOwnedSeats)? {
            for info in seats {
                worker.attach(info)?;
            }
        }
        worker.scan()?;
        Ok(worker)
    }
    fn publish(&self, stream: &str, event: Draft) -> Result<()> {
        self.handle.block_on(self.bus.publish(stream, event))?;
        Ok(())
    }
    fn scan(&mut self) -> Result<()> {
        for event in self.index.scan()? {
            let stream = if event.subject.starts_with("task/") {
                event.payload["swarm_id"]
                    .as_str()
                    .filter(|id| id.parse::<Ulid>().is_ok())
                    .map(|id| format!("swarm-{id}"))
                    .unwrap_or_else(|| "account".into())
            } else {
                "index".into()
            };
            let id = event.id;
            self.publish(&stream, event)?;
            self.index.acknowledge(id)?;
        }
        Ok(())
    }
    fn attach(&mut self, info: SeatInfo) -> Result<()> {
        check_size(info.size.cols as usize, info.size.rows as usize)?;
        let reply = self.connection.request(Action::Attach {
            seat: info.seat,
            generation: info.generation,
        })?;
        let Reply::Attached {
            attachment,
            first_available_sequence,
            ..
        } = reply
        else {
            return Err(invalid("attach failed"));
        };
        let directory = object_dir(&self.config.run_dir, &format!("seat-{}", info.seat))?;
        let store = SqliteStore::open(&directory)?;
        let mut term = Term::new(info.size.cols as usize, info.size.rows as usize, 0);
        let (mut sequence, mut title, mut cwd, mut gap) = (0, String::new(), None, None);
        if let Some(hash) = store.get("checkpoint")? {
            let hash = std::str::from_utf8(&hash)?;
            let saved: Saved = serde_json::from_slice(&LocalBlobs::open(&directory)?.get(hash)?)?;
            if saved.version != 1 {
                return Err(invalid("unsupported logic checkpoint version"));
            }
            if saved.generation == info.generation {
                term.restore(saved.checkpoint);
                sequence = saved.sequence;
                title = saved.title;
                cwd = saved.cwd;
                gap = saved.gap;
            }
        }
        if first_available_sequence > sequence + 1 {
            gap = Some(format!(
                "unavailable capture sequences {}..{}",
                sequence + 1,
                first_available_sequence - 1
            ));
            sequence = first_available_sequence - 1;
        }
        self.views.insert(
            info.seat,
            ViewState {
                term,
                sequence,
                title,
                cwd,
                gap,
                generation: info.generation,
                view_generation: Ulid::new(),
            },
        );
        self.attachments.insert(info.seat, attachment.clone());
        self.endpoints.remove(&info.seat);
        let path = self.endpoint_root.join(format!("{}.sock", info.seat));
        let mut capability = [0; 32];
        getrandom::fill(&mut capability).map_err(std::io::Error::other)?;
        let endpoint = crate::seat_socket::SeatEndpoint::bind(
            &self.handle,
            &path,
            self.client.clone(),
            info.seat,
            info.generation,
            capability,
        )?;
        self.endpoints.insert(
            info.seat,
            (
                EndpointInfo {
                    path,
                    generation: info.generation,
                    capability,
                },
                endpoint,
            ),
        );
        if let Ok(record) = self.registry.get(info.seat) {
            if record.generation != info.generation {
                self.registry.update(info.seat, |r| {
                    r.generation = info.generation;
                    Ok(())
                })?;
            }
            if info.exited.is_some() {
                self.registry.exited(info.seat, info.generation)?;
            } else if record.lifecycle == Lifecycle::ClosedGrace
                && let Some(close) = record.close
            {
                self.connection.request(Action::ScheduleClose {
                    attachment,
                    close_id: close.operation,
                    deadline_ms: close.deadline_ms,
                })?;
            }
        }
        Ok(())
    }
    fn spawn(&mut self, record: SeatRecord, spec: LaunchSpec) -> Result<SeatInfo> {
        check_size(spec.size.cols as usize, spec.size.rows as usize)?;
        if record.id != spec.seat || record.generation != spec.generation {
            return Err(invalid("launch and record identity mismatch"));
        }
        match self.registry.get(record.id) {
            Ok(old) if old != record => return Err(invalid("seat record conflicts")),
            Ok(_) => (),
            Err(_) => self.registry.insert(&record)?,
        }
        let Reply::Spawned { info } = self.connection.request(Action::Spawn { spec })? else {
            return Err(invalid("spawn reply mismatch"));
        };
        if self
            .views
            .get(&info.seat)
            .is_none_or(|view| view.generation != info.generation)
        {
            self.attach(info.clone())?;
        }
        self.publish(
            &format!("seat-{}", info.seat),
            Draft::new(
                format!("seat/{}/spawned", info.seat),
                "mux",
                serde_json::to_value(&info)?,
            ),
        )?;
        self.publish(
            "account",
            Draft::new(
                "mux/seat_spawned",
                "mux",
                serde_json::json!({"seat":info.seat,"generation":info.generation}),
            ),
        )?;
        self.publish(
            &format!("swarm-{}", record.swarm),
            Draft::new(
                format!("swarm/{}/joined", record.swarm),
                "mux",
                serde_json::json!({"seat":info.seat}),
            ),
        )?;
        Ok(info)
    }
    fn snapshot(&self, id: Ulid, generation: u64) -> Result<Snapshot> {
        let state = self
            .views
            .get(&id)
            .ok_or_else(|| invalid("unknown viewport"))?;
        if state.generation != generation {
            return Err(invalid("stale process generation"));
        }
        Ok(Snapshot {
            view: View {
                seat: id,
                generation,
                view_generation: state.view_generation,
            },
            sequence: state.sequence,
            screen: Screen::from_term(
                &state.term,
                state.title.clone(),
                state.cwd.clone(),
                state.gap.clone(),
            ),
        })
    }
    fn save(&self, id: Ulid) -> Result<()> {
        let view = &self.views[&id];
        let directory = object_dir(&self.config.run_dir, &format!("seat-{id}"))?;
        let bytes = serde_json::to_vec(&Saved {
            version: 1,
            generation: view.generation,
            sequence: view.sequence,
            checkpoint: view.term.checkpoint(),
            title: view.title.clone(),
            cwd: view.cwd.clone(),
            gap: view.gap.clone(),
        })?;
        let hash = LocalBlobs::open(&directory)?.put(&bytes)?;
        SqliteStore::open(&directory)?.put("checkpoint", hash.as_bytes())?;
        Ok(())
    }
    fn event(&mut self, event: Event) -> Result<()> {
        let id = event.seat();
        let seq = event.sequence().unwrap();
        let Some(view) = self.views.get_mut(&id) else {
            return Ok(());
        };
        let generation = match &event {
            Event::Output { generation, .. }
            | Event::Resized { generation, .. }
            | Event::Exited { generation, .. } => *generation,
        };
        if view.generation != generation || seq <= view.sequence {
            return Ok(());
        }
        if seq != view.sequence + 1 {
            view.gap = Some(format!(
                "unavailable capture sequences {}..{}",
                view.sequence + 1,
                seq - 1
            ));
        }
        match &event {
            Event::Output { bytes, .. } => {
                view.term.advance(bytes);
                for event in view.term.take_events() {
                    match event {
                        bbt_core::Event::Title(title) => view.title = title,
                        bbt_core::Event::ResetTitle => view.title.clear(),
                        _ => (),
                    }
                }
            }
            Event::Resized { size, .. } => {
                check_size(size.cols as usize, size.rows as usize)?;
                view.term.resize(size.cols as usize, size.rows as usize);
                view.view_generation = Ulid::new();
            }
            Event::Exited { .. } => {
                if self.registry.get(id).is_ok() {
                    self.registry.exited(id, generation)?;
                }
            }
        }
        view.sequence = seq;
        self.save(id)?;
        let kind = match event {
            Event::Output { .. } => "output",
            Event::Resized { .. } => "resized",
            Event::Exited { .. } => "exited",
        };
        let mut draft = Draft::new(
            format!("seat/{id}/{kind}"),
            "supervisor",
            serde_json::to_value(&event)?,
        );
        draft.seat = Some(id);
        draft.generation = Some(generation);
        self.publish(&format!("seat-{id}"), draft)?;
        Ok(())
    }
    fn run(&mut self, mut commands: mpsc::Receiver<Command>, stop: Arc<AtomicBool>) -> Result<()> {
        let mut scan_at = Instant::now();
        while !stop.load(Ordering::Acquire) {
            for _ in 0..32 {
                let Ok(command) = commands.try_recv() else {
                    break;
                };
                match command {
                    Command::Snapshot(id, generation, reply) => {
                        let _ = reply.send(self.snapshot(id, generation));
                    }
                    Command::Spawn(record, spec, reply) => {
                        let result = self.spawn(*record, spec);
                        let _ = reply.send(result);
                    }
                    Command::Record(id, reply) => {
                        let _ = reply.send(self.registry.get(id));
                    }
                    Command::Endpoint(id, reply) => {
                        let _ = reply.send(
                            self.endpoints
                                .get(&id)
                                .map(|(info, _)| info.clone())
                                .ok_or_else(|| invalid("seat endpoint unavailable")),
                        );
                    }
                    Command::Close(id, generation, operation, position, grace, reply) => {
                        let result = (|| {
                            let close = self.registry.close(
                                id,
                                generation,
                                operation,
                                position,
                                now_ms(),
                                grace,
                            )?;
                            let attachment = self
                                .attachments
                                .get(&id)
                                .ok_or_else(|| invalid("seat not attached"))?
                                .clone();
                            self.connection.request(Action::ScheduleClose {
                                attachment,
                                close_id: close.operation,
                                deadline_ms: close.deadline_ms,
                            })?;
                            Ok(())
                        })();
                        let _ = reply.send(result);
                    }
                    Command::Undo(id, generation, revision, layout_revision, reply) => {
                        let result = (|| {
                            let record = self.registry.get(id)?;
                            let close = record
                                .close
                                .as_ref()
                                .ok_or_else(|| invalid("no close record"))?;
                            if record.generation != generation
                                || close.position.layout_revision != layout_revision
                                || close.revision != revision
                            {
                                return Err(invalid("layout/close revision conflict"));
                            }
                            let attachment = self
                                .attachments
                                .get(&id)
                                .ok_or_else(|| invalid("seat not attached"))?
                                .clone();
                            let accepted_at = now_ms();
                            self.connection.request(Action::UndoClose {
                                attachment,
                                close_id: close.operation,
                            })?;
                            self.registry.undo(
                                id,
                                generation,
                                revision,
                                layout_revision,
                                accepted_at,
                            )
                        })();
                        let _ = reply.send(result);
                    }
                }
            }
            if let Some(event) = self.connection.receive(Duration::from_millis(10))? {
                self.event(event)?;
            }
            if scan_at.elapsed() >= Duration::from_millis(100) {
                if self.watch.failed() {
                    return Err(invalid(
                        "filesystem watch failed; index reconciliation required",
                    ));
                }
                if self.watch.changed() {
                    self.scan()?;
                }
                scan_at = Instant::now();
            }
        }
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.endpoints.clear();
        let _ = std::fs::remove_dir(&self.endpoint_root);
    }
}
fn check_size(cols: usize, rows: usize) -> Result<()> {
    if cols < 2 || rows == 0 || cols > 512 || rows > 256 || cols * rows > 32768 {
        return Err(invalid("viewport dimensions exceed bounded logic model"));
    }
    Ok(())
}
