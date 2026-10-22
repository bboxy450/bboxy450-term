//! Restartable IO/logic owner. The UI gets an async bounded client, not a DB or PTY.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use crate::adapters::{Adapter, Binding, HarnessRecord, OscTap, Session};
use crate::ring::{Playback, Saved as RingSaved};
use bbt_core::Checkpoint;
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
use crate::streams::{Screen, Snapshot, View};
use crate::{Action, Attachment, Event, LaunchSpec, Reply, SeatInfo};

pub use bbt_supervisor::{SocketSupervisor, SupervisorConnection};

#[derive(Clone)]
pub struct LogicConfig {
    pub run_dir: PathBuf,
    pub repository: PathBuf,
    pub ring: crate::ring::Settings,
}

enum Command {
    Messaging(
        Ulid,
        u64,
        crate::messaging::api::Request,
        bool,
        oneshot::Sender<Result<crate::messaging::api::Response>>,
    ),
    DeliveryEndpoint(
        Ulid,
        u64,
        crate::adapters::Endpoint,
        oneshot::Sender<Result<()>>,
    ),
    Launch(
        Box<SeatRecord>,
        HarnessRecord,
        crate::adapters::launch::LaunchOptions,
        oneshot::Sender<Result<SeatInfo>>,
    ),
    Hook(
        Ulid,
        u64,
        crate::adapters::hook::ShimEvent,
        oneshot::Sender<Result<crate::adapters::Status>>,
    ),
    Status(Ulid, u64, oneshot::Sender<Result<crate::adapters::Status>>),
    Native(
        Ulid,
        u64,
        serde_json::Value,
        oneshot::Sender<Result<crate::adapters::Status>>,
    ),
    Spawn(
        Box<SeatRecord>,
        LaunchSpec,
        oneshot::Sender<Result<SeatInfo>>,
    ),
    Snapshot(Ulid, u64, oneshot::Sender<Result<Snapshot>>),
    History(
        Ulid,
        u64,
        u64,
        u64,
        oneshot::Sender<Result<crate::ring::History>>,
    ),
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
    /// Admitted seat identity; the broker derives its class from the registry.
    pub async fn messaging(
        &self,
        seat: Ulid,
        generation: u64,
        request: crate::messaging::api::Request,
    ) -> Result<crate::messaging::api::Response> {
        self.message_request(seat, generation, request, false).await
    }
    /// Trusted in-process GUI composer. This is never a seat socket operation.
    pub async fn messaging_operator(
        &self,
        seat: Ulid,
        generation: u64,
        request: crate::messaging::api::Request,
    ) -> Result<crate::messaging::api::Response> {
        self.message_request(seat, generation, request, true).await
    }
    async fn message_request(
        &self,
        seat: Ulid,
        generation: u64,
        request: crate::messaging::api::Request,
        operator: bool,
    ) -> Result<crate::messaging::api::Response> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Messaging(seat, generation, request, operator, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await
            .map_err(|_| invalid("message operation outcome unknown; query original message ID"))?
    }
    /// Trusted discovery/fixture seam. Native delivery still checks process
    /// start and native session on every attempt. Peers cannot set this path.
    pub async fn bind_delivery_endpoint(
        &self,
        seat: Ulid,
        generation: u64,
        endpoint: crate::adapters::Endpoint,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::DeliveryEndpoint(seat, generation, endpoint, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await.map_err(|_| invalid("logic stopped"))?
    }

    pub async fn spawn_harness(
        &self,
        record: SeatRecord,
        harness: HarnessRecord,
        options: crate::adapters::launch::LaunchOptions,
    ) -> Result<SeatInfo> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Launch(Box::new(record), harness, options, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await
            .map_err(|_| invalid("launch outcome unknown; query original seat ID"))?
    }
    pub async fn hook(
        &self,
        seat: Ulid,
        generation: u64,
        event: crate::adapters::hook::ShimEvent,
    ) -> Result<crate::adapters::Status> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Hook(seat, generation, event, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await.map_err(|_| invalid("logic stopped"))?
    }
    pub async fn adapter_status(
        &self,
        seat: Ulid,
        generation: u64,
    ) -> Result<crate::adapters::Status> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Status(seat, generation, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await.map_err(|_| invalid("logic stopped"))?
    }
    /// Called by the bound native reader, never interpreted as a new operation.
    pub async fn native_event(
        &self,
        seat: Ulid,
        generation: u64,
        event: serde_json::Value,
    ) -> Result<crate::adapters::Status> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Native(seat, generation, event, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await.map_err(|_| invalid("logic stopped"))?
    }
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
    pub async fn history(
        &self,
        seat: Ulid,
        generation: u64,
        from: u64,
        to: u64,
    ) -> Result<crate::ring::History> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::History(seat, generation, from, to, tx))
            .await
            .map_err(|_| invalid("logic stopped"))?;
        rx.await.map_err(|_| invalid("logic stopped"))?
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
                let mut worker = match Worker::open(
                    Box::new(connection),
                    config,
                    bus,
                    runtime,
                    worker_client,
                    key,
                ) {
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
struct LegacySaved {
    version: u32,
    generation: u64,
    sequence: u64,
    checkpoint: Checkpoint,
    title: String,
    cwd: Option<String>,
    gap: Option<String>,
}
struct ViewState {
    adapter: Session,
    delivery_endpoint: crate::adapters::Endpoint,
    tap: OscTap,
    replies: VecDeque<(Ulid, Vec<u8>)>,
    native_reader: Option<NativeReader>,
    state: Playback,
    view_generation: Ulid,
    checkpoint_bytes: u64,
    checkpoint_sequence: u64,
    changed_at: Instant,
}
struct NativeReader {
    session: String,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for NativeReader {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl std::ops::Deref for ViewState {
    type Target = Playback;
    fn deref(&self) -> &Playback {
        &self.state
    }
}
impl std::ops::DerefMut for ViewState {
    fn deref_mut(&mut self) -> &mut Playback {
        &mut self.state
    }
}

struct Delivery {
    swarm: Ulid,
    work: crate::messaging::ledger::Work,
    outcome: crate::adapters::WakeOutcome,
    relayed: bool,
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
    messaging: crate::messaging::Messaging,
    deliveries: tokio::task::JoinSet<Delivery>,
}
impl Worker {
    fn open(
        connection: Box<dyn SupervisorConnection>,
        config: LogicConfig,
        bus: BusHandle,
        handle: tokio::runtime::Handle,
        client: LogicClient,
        key: SigningKey,
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
        let messaging = crate::messaging::Messaging::open(
            &config.run_dir,
            crate::identity::seat_keys(&config.run_dir, key, handle.clone())?,
        )?;
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
            messaging,
            deliveries: tokio::task::JoinSet::new(),
        };
        if let Reply::Seats { seats } = worker.connection.request(Action::ListOwnedSeats)? {
            for info in seats {
                worker.attach(info, false)?;
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
    fn attach(&mut self, info: SeatInfo, live_start: bool) -> Result<()> {
        check_size(info.size.cols as usize, info.size.rows as usize)?;
        let directory = object_dir(&self.config.run_dir, &format!("seat-{}", info.seat))?;
        let reply = self.connection.request(Action::AttachRing {
            seat: info.seat,
            generation: info.generation,
            directory: directory.join(format!("ring-{}", info.generation)),
            settings: self.config.ring.clone(),
        })?;
        let Reply::Attached {
            attachment,
            first_available_sequence,
            ..
        } = reply
        else {
            return Err(invalid("ring attach failed"));
        };
        let Reply::RingRecovery { status, checkpoint } =
            self.connection.request(Action::RingRecovery {
                attachment: attachment.clone(),
            })?
        else {
            return Err(invalid("ring recovery reply mismatch"));
        };
        let mut state = if let Some(bytes) = checkpoint {
            Playback::restore(RingSaved::decode(&bytes)?)
        } else {
            let mut state = Playback::new(info.seat, info.generation, info.size);
            // Migrate W2.2's bridge without claiming its missing byte history.
            if let Some(hash) = SqliteStore::open(&directory)?.get("checkpoint")? {
                let old: LegacySaved = serde_json::from_slice(
                    &LocalBlobs::open(&directory)?.get(std::str::from_utf8(&hash)?)?,
                )?;
                if old.version != 1 {
                    return Err(invalid("unsupported legacy checkpoint version"));
                }
                if old.generation == info.generation {
                    state.term.restore(old.checkpoint);
                    state.sequence = old.sequence;
                    state.title = old.title;
                    state.cwd = old.cwd;
                    state.gap = old.gap;
                }
            }
            if first_available_sequence > state.sequence + 1 {
                state.gap = Some(format!(
                    "unavailable pre-ring capture sequences {}..{}",
                    state.sequence + 1,
                    first_available_sequence - 1
                ));
            }
            state.sequence = first_available_sequence - 1;
            state.byte_end = status.byte_start;
            self.connection.request(Action::PublishCheckpoint {
                attachment: attachment.clone(),
                checkpoint: state.saved().encode()?,
            })?;
            state
        };
        let checkpoint_bytes = state.byte_end;
        let checkpoint_sequence = state.sequence;
        let tap = OscTap::install(&mut state.term);
        let mut live_frames = Vec::new();
        while state.sequence < status.captured_sequence {
            let Reply::RingHistory { history } = self.connection.request(Action::ReadRing {
                attachment: attachment.clone(),
                from: state.sequence + 1,
                to: status.captured_sequence,
            })?
            else {
                return Err(invalid("ring replay reply mismatch"));
            };
            if history.gap.is_some() || history.records.is_empty() {
                return Err(invalid("ring recovery continuity gap"));
            }
            for record in history.records {
                state.apply(&record.event)?;
                if let Ok(frames) = tap.drain()
                    && live_start
                {
                    for frame in frames {
                        if live_frames.len() < 128 {
                            live_frames.push((state.sequence, frame));
                        }
                    }
                }
            }
        }
        if state.sequence != status.captured_sequence {
            return Err(invalid("ring recovery cursor mismatch"));
        }
        self.attachments.insert(info.seat, attachment.clone());
        let record = self.registry.get(info.seat).ok();
        let kind = record
            .as_ref()
            .map_or("unknown", |r| r.harness_kind.as_str());
        let harness = if let Some(bytes) = SqliteStore::open(&directory)?.get("adapter_record")? {
            serde_json::from_slice(&bytes)?
        } else {
            HarnessRecord::builtin(kind).unwrap_or_else(|_| HarnessRecord {
                kind: kind.into(),
                name: kind.into(),
                command: vec![kind.into()],
                tile: String::new(),
                icon: String::new(),
                default_role: crate::registry::Role {
                    name: "executor".into(),
                    class: crate::registry::Class::Worker,
                },
                wake_method: crate::adapters::WakeMethod::Operator,
                state_source: crate::adapters::StateSource::Cadence,
                hooks: false,
            })
        };
        let mut adapter = Session::new(
            harness,
            Binding {
                seat: info.seat,
                generation: info.generation,
                pid: info.pid,
                process_start: crate::adapters::native::process_start(info.pid)
                    .unwrap_or_else(|_| "exited".into()),
                session: record
                    .as_ref()
                    .and_then(|r| r.harness_session_id.clone())
                    .unwrap_or_else(|| format!("process-{}", info.pid)),
                attachment_generation: 1,
            },
        )?;
        for (sequence, params) in live_frames {
            adapter.on_osc(&params, sequence, now_ms());
        }
        if let Some(exit) = &info.exited {
            adapter.exit(state.sequence, now_ms(), exit.code);
        }
        let replies = adapter
            .take_replies()
            .into_iter()
            .map(|bytes| (Ulid::new(), bytes))
            .collect();
        self.views.insert(
            info.seat,
            ViewState {
                adapter,
                delivery_endpoint: crate::adapters::Endpoint::None,
                tap,
                replies,
                native_reader: None,
                state,
                view_generation: Ulid::new(),
                checkpoint_bytes,
                checkpoint_sequence,
                changed_at: Instant::now(),
            },
        );
        self.persist_native_session(info.seat)?;
        // Historical frames have no live effects or visible intermediate state.
        // Cold capture no longer needs checkpoint progress to keep reading, so
        // publish one recovered tail instead of syncing every replay page.
        self.flush_protocol_replies(info.seat)?;
        self.save(info.seat)?;
        self.ensure_endpoint(info.seat, info.generation)?;
        if let Ok(record) = self.registry.get(info.seat) {
            if record.generation != info.generation {
                self.registry.update(info.seat, |r| {
                    r.generation = info.generation;
                    Ok(())
                })?;
            }
            if info.exited.is_some() {
                self.registry.exited(info.seat, info.generation)?;
                self.cleanup_overlay(info.seat)?;
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
    fn ensure_endpoint(&mut self, id: Ulid, generation: u64) -> Result<EndpointInfo> {
        if let Some((info, _)) = self.endpoints.get(&id)
            && info.generation == generation
        {
            return Ok(info.clone());
        }
        self.endpoints.remove(&id);
        let path = self.endpoint_root.join(format!("{id}.sock"));
        let mut capability = [0; 32];
        getrandom::fill(&mut capability).map_err(std::io::Error::other)?;
        let endpoint = crate::seat_socket::SeatEndpoint::bind(
            &self.handle,
            &path,
            self.client.clone(),
            id,
            generation,
            capability,
        )?;
        let info = EndpointInfo {
            path,
            generation,
            capability,
        };
        self.endpoints.insert(id, (info.clone(), endpoint));
        Ok(info)
    }
    fn cleanup_overlay(&self, id: Ulid) -> Result<()> {
        let root = object_dir(&self.config.run_dir, &format!("seat-{id}"))?;
        let store = SqliteStore::open(&root)?;
        if let Some(bytes) = store.get("adapter_overlay")?
            && !bytes.is_empty()
        {
            let name = std::str::from_utf8(&bytes)?;
            if !name.starts_with("launch-") || name[7..].parse::<Ulid>().is_err() {
                return Err(invalid("invalid retained overlay name"));
            }
            let path = root.join("overlays").join(name);
            if path.exists() {
                std::fs::remove_dir_all(path)?;
            }
            store.put("adapter_overlay", b"")?;
        }
        Ok(())
    }
    fn persist_native_session(&self, id: Ulid) -> Result<()> {
        let adapter = &self.views[&id].adapter;
        if matches!(adapter.record.kind.as_str(), "claude" | "codex")
            && !adapter.binding.session.starts_with("process-")
        {
            let session = &adapter.binding.session;
            if let Ok(record) = self.registry.get(id)
                && record.harness_session_id.as_ref() != Some(session)
            {
                self.registry.update(id, |record| {
                    record.harness_session_id = Some(session.clone());
                    Ok(())
                })?;
            }
        }
        Ok(())
    }
    fn launch(
        &mut self,
        mut record: SeatRecord,
        harness: HarnessRecord,
        options: crate::adapters::launch::LaunchOptions,
    ) -> Result<SeatInfo> {
        if record.harness_kind != harness.kind {
            return Err(invalid("seat/harness kind mismatch"));
        }
        let endpoint = self.ensure_endpoint(record.id, record.generation)?;
        let root = object_dir(&self.config.run_dir, &format!("seat-{}", record.id))?;
        let plan = crate::adapters::launch::plan(
            &harness,
            &crate::adapters::LaunchContext {
                seat: &record,
                cwd: &options.cwd,
                role_prompt: &options.role_prompt,
                executable: &options.executable,
                overlay_parent: &root.join("overlays"),
                user_home: &options.user_home,
                user_zdotdir: options.user_zdotdir.as_deref(),
                socket: &endpoint.path,
                capability: endpoint.capability,
                resume: options.resume,
            },
        )?;
        record.harness_session_id = plan.harness_session_id;
        let store = SqliteStore::open(&root)?;
        store.put("adapter_record", &serde_json::to_vec(&harness)?)?;
        store.put(
            "adapter_overlay",
            plan.overlay.file_name().unwrap().as_encoded_bytes(),
        )?;
        // On an ambiguous spawn keep the overlay; deleting it could break a child
        // that actually started. Reconciliation uses the original durable seat ID.
        self.spawn(record, plan.spec)
    }
    fn spawn(&mut self, record: SeatRecord, spec: LaunchSpec) -> Result<SeatInfo> {
        check_size(spec.size.cols as usize, spec.size.rows as usize)?;
        if record.id != spec.seat || record.generation != spec.generation {
            return Err(invalid("launch and record identity mismatch"));
        }
        let new_record = self.registry.get(record.id).is_err();
        match self.registry.get(record.id) {
            Ok(old) if old != record => return Err(invalid("seat record conflicts")),
            Ok(_) => (),
            Err(_) => self.registry.insert(&record)?,
        }
        self.messaging.sync_seat(&record)?;
        let Reply::Spawned { info } = self.connection.request(Action::Spawn { spec })? else {
            return Err(invalid("spawn reply mismatch"));
        };
        if self
            .views
            .get(&info.seat)
            .is_none_or(|view| view.generation != info.generation)
        {
            self.attach(info.clone(), new_record)?;
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
    fn history(
        &mut self,
        id: Ulid,
        generation: u64,
        from: u64,
        to: u64,
    ) -> Result<crate::ring::History> {
        let attachment = self
            .attachments
            .get(&id)
            .filter(|a| a.generation == generation)
            .ok_or_else(|| invalid("stale history generation"))?
            .clone();
        let Reply::RingHistory { history } = self.connection.request(Action::ReadRing {
            attachment,
            from,
            to,
        })?
        else {
            return Err(invalid("history reply mismatch"));
        };
        Ok(*history)
    }
    fn save(&mut self, id: Ulid) -> Result<()> {
        let view = &self.views[&id];
        let sequence = view.sequence;
        if sequence == view.checkpoint_sequence {
            return Ok(());
        }
        self.connection.request(Action::PublishCheckpoint {
            attachment: self.attachments[&id].clone(),
            checkpoint: view.saved().encode()?,
        })?;
        let view = self.views.get_mut(&id).unwrap();
        view.checkpoint_bytes = view.byte_end;
        view.checkpoint_sequence = sequence;
        Ok(())
    }
    fn event(&mut self, event: Event) -> Result<()> {
        let id = event.seat();
        let seq = event.sequence().unwrap();
        let generation = match &event {
            Event::Output { generation, .. }
            | Event::Resized { generation, .. }
            | Event::Exited { generation, .. } => *generation,
        };
        let Some(view) = self.views.get(&id) else {
            return Ok(());
        };
        if view.generation != generation || seq <= view.sequence {
            return Ok(());
        }
        if seq > view.sequence + 1 {
            self.catch_up(id, seq - 1)?;
        }
        let view = self.views.get_mut(&id).unwrap();
        let before = view.adapter.status.clone();
        view.state.apply(&event)?;
        let now = now_ms();
        view.adapter.output(now);
        match view.tap.drain() {
            Ok(frames) => {
                for params in frames {
                    view.adapter.on_osc(&params, seq, now);
                }
            }
            Err(_) => view.adapter.invalidate(),
        }
        for bytes in view.adapter.take_replies() {
            if view.replies.len() < 32 {
                view.replies.push_back((Ulid::new(), bytes));
            }
        }
        view.changed_at = Instant::now();
        if let Event::Resized { size, .. } = &event {
            check_size(size.cols as usize, size.rows as usize)?;
            view.view_generation = Ulid::new();
        }
        self.persist_native_session(id)?;
        if matches!(event, Event::Exited { .. }) && self.registry.get(id).is_ok() {
            self.registry.exited(id, generation)?;
        }
        if let Event::Exited { status, .. } = &event {
            self.views
                .get_mut(&id)
                .unwrap()
                .adapter
                .exit(seq, now, status.code);
            self.views.get_mut(&id).unwrap().native_reader = None;
            self.views.get_mut(&id).unwrap().replies.clear();
            self.cleanup_overlay(id)?;
        }
        self.flush_protocol_replies(id)?;
        let view = &self.views[&id];
        if view.byte_end.saturating_sub(view.checkpoint_bytes) >= self.config.ring.checkpoint_bytes
            || view.sequence.saturating_sub(view.checkpoint_sequence) >= 256
            || !matches!(event, Event::Output { .. })
        {
            self.save(id)?;
        }
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
        let status = self.views[&id].adapter.status.clone();
        if !status.same_value(&before) {
            self.publish_adapter(id, status)?;
        }
        Ok(())
    }
    fn catch_up(&mut self, id: Ulid, through: u64) -> Result<()> {
        loop {
            let current = &self.views[&id];
            if current.sequence >= through {
                return self.save(id);
            }
            let history = self.history(id, current.generation, current.sequence + 1, through)?;
            if history.gap.is_some() || history.records.is_empty() {
                return Err(invalid("capture recovery gap"));
            }
            let view = self.views.get_mut(&id).unwrap();
            for record in history.records {
                if matches!(record.event, Event::Resized { .. }) {
                    view.view_generation = Ulid::new();
                }
                view.state.apply(&record.event)?;
                // Recovery must not replay probe responses or hook/native effects.
                let _ = view.tap.drain();
            }
            view.changed_at = Instant::now();
        }
    }
    fn flush_protocol_replies(&mut self, id: Ulid) -> Result<()> {
        loop {
            let Some((input_id, bytes)) = self.views[&id].replies.front().cloned() else {
                return Ok(());
            };
            match self.connection.request(Action::Input {
                attachment: self.attachments[&id].clone(),
                input_id,
                bytes,
            }) {
                Ok(Reply::Input { receipt }) if receipt.state != crate::InputState::Pending => {
                    self.views.get_mut(&id).unwrap().replies.pop_front();
                }
                // Stable IDs query/dedupe an in-flight reply. No new paste or
                // protocol response can interleave the supervisor's active write.
                _ => return Ok(()),
            }
        }
    }
    fn refresh_adapters(&mut self) -> Result<()> {
        let now = now_ms();
        for id in self.views.keys().copied().collect::<Vec<_>>() {
            let view = self.views.get_mut(&id).unwrap();
            let before = view.adapter.status.clone();
            if view.adapter.status.source != "exit" {
                if view.adapter.record.kind == "claude" {
                    let root = std::env::var_os("CLAUDE_CONFIG_DIR")
                        .map(PathBuf::from)
                        .or_else(|| {
                            std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".claude"))
                        });
                    if let Some(root) = root {
                        let path = root
                            .join("sessions")
                            .join(format!("{}.json", view.adapter.binding.pid));
                        match crate::adapters::native::read_claude_bound(
                            &path,
                            &view.adapter.binding,
                        ) {
                            Ok(file) => {
                                if let Some(path) = &file.messaging_socket_path {
                                    view.delivery_endpoint =
                                        crate::adapters::Endpoint::Claude { path: path.clone() };
                                }
                                let _ = view.adapter.observe_claude_session(
                                    &file,
                                    view.state.sequence,
                                    now,
                                );
                            }
                            Err(_) => view.adapter.invalidate(),
                        }
                    }
                }
                if view.adapter.record.kind == "codex"
                    && !view.adapter.binding.session.starts_with("process-")
                    && view
                        .native_reader
                        .as_ref()
                        .is_none_or(|r| r.session != view.adapter.binding.session)
                {
                    let root = std::env::var_os("CODEX_HOME")
                        .map(PathBuf::from)
                        .or_else(|| {
                            std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".codex"))
                        });
                    if let Some(root) = root {
                        let path = root.join("app-server-control/app-server-control.sock");
                        let session = view.adapter.binding.session.clone();
                        view.delivery_endpoint = crate::adapters::Endpoint::Codex {
                            path: path.clone(),
                            thread: session.clone(),
                            turn: None,
                        };
                        let thread = session.clone();
                        let generation = view.generation;
                        let client = self.client.clone();
                        let task=self.handle.spawn(async move {
                            loop {
                                if let Ok(status)=crate::adapters::native::codex_status(&path,&thread).await {
                                    let event=serde_json::json!({"source":"snapshot","method":"thread/status/changed","params":{"threadId":thread,"status":status}});
                                    if client.native_event(id,generation,event).await.is_err(){break;}
                                }
                                tokio::time::sleep(Duration::from_millis(200)).await;
                            }
                        });
                        view.native_reader = Some(NativeReader { session, task });
                    }
                }
                view.adapter.tick(view.state.sequence, now);
            }
            let status = view.adapter.status.clone();
            if !status.same_value(&before) {
                self.publish_adapter(id, status)?;
            }
            self.flush_protocol_replies(id)?;
        }
        Ok(())
    }
    fn pump_messages(&mut self) -> Result<()> {
        use crate::adapters::{Evidence, Priority, WakeMethod, WakeOutcome};
        while let Some(result) = self.deliveries.try_join_next() {
            let delivery = result.map_err(|_| {
                invalid("delivery task lost; restart to mark the journaled attempt unknown")
            })?;
            self.messaging.finish(
                delivery.swarm,
                &delivery.work,
                &delivery.outcome,
                now_ms(),
                delivery.relayed,
            )?;
        }
        let targets = self.messaging.ready(now_ms())?;
        // Stable durable outbox records publish before native/PTY effects. A
        // publish followed by a crash is deduplicated by its original bus id.
        for (stream, draft) in self.messaging.events()? {
            let id = draft.id;
            self.publish(&stream, draft)?;
            self.messaging.event_sent(&stream, id)?;
        }
        for (swarm, seat) in targets {
            if self.deliveries.len() >= 32 {
                break;
            }
            let Some(view) = self
                .views
                .get(&seat)
                .filter(|v| v.adapter.status.source != "exit")
            else {
                continue;
            };
            let Some(work) = self.messaging.prepare(swarm, seat, now_ms())? else {
                continue;
            };
            let mut text = work.text();
            let relayed = if let Some(agent) = &work.agent {
                if !view.adapter.status.subagents.iter().any(|a| &a.id == agent) {
                    self.messaging.finish(swarm,&work,&WakeOutcome{evidence:Evidence::RecipientStored,native_id:None,note:"held: target subagent ended or is not currently reported; never redirect".into()},now_ms(),false)?;
                    continue;
                }
                text =
                    format!("Target subagent {agent}; relay via main to this target only.\n{text}");
                true
            } else {
                false
            };
            match view.adapter.record.wake_method {
                WakeMethod::ClaudeSocket | WakeMethod::CodexRpc => {
                    let binding = view.adapter.binding.clone();
                    let endpoint = view.delivery_endpoint.clone();
                    let priority = if work.messages.iter().all(|m| m.envelope.priority > 0) {
                        Priority::Later
                    } else {
                        Priority::Next
                    };
                    self.deliveries.spawn_on(
                        async move {
                            let outcome = crate::adapters::native::wake(
                                &binding, &endpoint, work.id, &text, priority, false,
                            )
                            .await;
                            Delivery {
                                swarm,
                                work,
                                outcome,
                                relayed,
                            }
                        },
                        &self.handle,
                    );
                }
                WakeMethod::Pty => {
                    let outcome = self.deliver_pty(seat, &work, &text)?;
                    self.messaging
                        .finish(swarm, &work, &outcome, now_ms(), relayed)?;
                }
                WakeMethod::Operator => {
                    self.messaging.finish(
                        swarm,
                        &work,
                        &WakeOutcome {
                            evidence: Evidence::RecipientStored,
                            native_id: None,
                            note: "pending for operator: harness has no autonomous input path"
                                .into(),
                        },
                        now_ms(),
                        false,
                    )?;
                }
            }
        }
        Ok(())
    }
    fn deliver_pty(
        &mut self,
        seat: Ulid,
        work: &crate::messaging::ledger::Work,
        text: &str,
    ) -> Result<crate::adapters::WakeOutcome> {
        use crate::adapters::{ArbiterView, Evidence, WakeOutcome};
        let pending = |note: &str| WakeOutcome {
            evidence: Evidence::RecipientStored,
            native_id: None,
            note: note.into(),
        };
        let attachment = self
            .attachments
            .get(&seat)
            .ok_or_else(|| invalid("recipient has no writer lease"))?
            .clone();
        let context = match self.connection.request(Action::InputContext {
            attachment: attachment.clone(),
        }) {
            Ok(Reply::InputContext { context }) => context,
            _ => {
                return Ok(pending(
                    "pending for operator: live PTY arbiter unavailable",
                ));
            }
        };
        let view = self.views.get_mut(&seat).unwrap();
        let now = now_ms();
        if !context.settled
            || context.sequence != view.state.sequence
            || view
                .adapter
                .proof
                .as_ref()
                .is_none_or(|p| p.observed_ms <= context.last_input_ms)
            || now.saturating_sub(context.last_input_ms) < 500
        {
            return Ok(pending(
                "pending for operator: fresh readiness after the last input and a caught-up PTY are required",
            ));
        }
        let arbiter = ArbiterView {
            binding: view.adapter.binding.clone(),
            operator_epoch: context.epoch,
            foreground_pid: context.foreground_pid,
            operator_draft: context.draft,
            active_tool: view.adapter.status.turn_open,
            sync_pending: view.state.term.sync_pending(),
            screen_switch_pending: view.changed_at.elapsed() < Duration::from_millis(50),
            canonical: context.canonical,
            echo: context.echo,
            bracketed_paste: view
                .state
                .term
                .mode()
                .contains(bbt_core::term::TermMode::BRACKETED_PASTE),
        };
        let directory = object_dir(&self.config.run_dir, &format!("seat-{seat}"))?;
        let journal = SqliteStore::open(&directory)?;
        view.adapter
            .submit_pty(&arbiter, now, work.id, text, &journal, |bytes| {
                match self.connection.request(Action::GuardedInput {
                    attachment,
                    input_id: work.id,
                    bytes,
                    expected: context,
                })? {
                    Reply::Input { receipt } => Ok(receipt),
                    _ => Err(invalid("guarded input result unknown")),
                }
            })
    }
    fn publish_adapter(&self, id: Ulid, status: crate::adapters::Status) -> Result<()> {
        self.publish(
            &format!("seat-{id}"),
            Draft::new(
                format!("seat/{id}/state"),
                "adapter",
                serde_json::to_value(status)?,
            ),
        )
    }
    fn run(&mut self, mut commands: mpsc::Receiver<Command>, stop: Arc<AtomicBool>) -> Result<()> {
        let mut scan_at = Instant::now();
        let mut adapters_at = Instant::now();
        let mut messages_at = Instant::now();
        while !stop.load(Ordering::Acquire) {
            for _ in 0..32 {
                let Ok(command) = commands.try_recv() else {
                    break;
                };
                match command {
                    Command::Messaging(id, generation, request, operator, reply) => {
                        let result = (|| {
                            self.snapshot(id, generation)?;
                            let state = match &request {
                                crate::messaging::api::Request::Mute { seat, .. } => {
                                    self.views.get(seat).map(|v| v.adapter.status.state)
                                }
                                _ => None,
                            };
                            self.messaging.request(
                                id,
                                generation,
                                request,
                                operator,
                                state,
                                now_ms(),
                            )
                        })();
                        let _ = reply.send(result);
                    }
                    Command::DeliveryEndpoint(id, generation, endpoint, reply) => {
                        let result = (|| {
                            let view = self
                                .views
                                .get_mut(&id)
                                .filter(|v| v.generation == generation)
                                .ok_or_else(|| invalid("stale delivery generation"))?;
                            match (&endpoint, view.adapter.record.kind.as_str()) {
                                (crate::adapters::Endpoint::Claude { .. }, "claude")
                                | (crate::adapters::Endpoint::Codex { .. }, "codex")
                                | (crate::adapters::Endpoint::None, _) => {}
                                _ => {
                                    return Err(invalid(
                                        "native endpoint does not match harness kind",
                                    ));
                                }
                            }
                            view.delivery_endpoint = endpoint;
                            Ok(())
                        })();
                        let _ = reply.send(result);
                    }
                    Command::Launch(record, harness, options, reply) => {
                        let result = self.launch(*record, harness, options);
                        let _ = reply.send(result);
                    }
                    Command::Status(id, generation, reply) => {
                        let result = self
                            .views
                            .get(&id)
                            .filter(|v| v.generation == generation)
                            .map(|v| v.adapter.status.clone())
                            .ok_or_else(|| invalid("stale adapter generation"));
                        let _ = reply.send(result);
                    }
                    Command::Hook(id, generation, event, reply) => {
                        let before = self.views.get(&id).map(|v| v.adapter.status.clone());
                        let result = self
                            .views
                            .get_mut(&id)
                            .filter(|v| v.generation == generation)
                            .ok_or_else(|| invalid("stale hook generation"))
                            .and_then(|view| {
                                if serde_json::to_vec(&event)?.len()
                                    > crate::adapters::hook::MAX_RAW
                                {
                                    return Err(invalid("hook payload bound"));
                                }
                                if let Some(obs) = crate::adapters::hook::project(
                                    &mut view.adapter,
                                    &event,
                                    view.state.sequence,
                                    now_ms(),
                                ) {
                                    view.adapter.apply(obs);
                                }
                                Ok(view.adapter.status.clone())
                            });
                        if let (Some(before), Ok(status)) = (&before, &result)
                            && !status.same_value(before)
                        {
                            self.publish_adapter(id, status.clone())?;
                        }
                        if result.is_ok() {
                            self.persist_native_session(id)?;
                        }
                        let _ = reply.send(result);
                    }
                    Command::Native(id, generation, event, reply) => {
                        let before = self.views.get(&id).map(|v| v.adapter.status.clone());
                        let result = self
                            .views
                            .get_mut(&id)
                            .filter(|v| v.generation == generation)
                            .ok_or_else(|| invalid("stale native generation"))
                            .and_then(|view| {
                                view.adapter
                                    .on_native(&event, view.state.sequence, now_ms())?;
                                Ok(view.adapter.status.clone())
                            });
                        if let (Some(before), Ok(status)) = (&before, &result)
                            && !status.same_value(before)
                        {
                            self.publish_adapter(id, status.clone())?;
                        }
                        let _ = reply.send(result);
                    }
                    Command::History(id, generation, from, to, reply) => {
                        let result = self.history(id, generation, from, to);
                        let _ = reply.send(result);
                    }
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
            if adapters_at.elapsed() >= Duration::from_millis(200) {
                self.refresh_adapters()?;
                adapters_at = Instant::now();
            }
            if messages_at.elapsed() >= Duration::from_millis(100) {
                self.pump_messages()?;
                messages_at = Instant::now();
            }
            let idle: Vec<_> = self
                .views
                .iter()
                .filter(|(_, v)| {
                    v.sequence > v.checkpoint_sequence
                        && v.changed_at.elapsed() >= Duration::from_millis(self.config.ring.idle_ms)
                })
                .map(|(id, _)| *id)
                .collect();
            for id in idle {
                self.save(id)?;
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
        for id in self.views.keys().copied().collect::<Vec<_>>() {
            self.save(id)?;
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
