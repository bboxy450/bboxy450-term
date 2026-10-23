//! Restartable IO/logic owner. The UI gets an async bounded client, not a DB or PTY.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::{
    Arc, OnceLock,
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
use crate::provider::{Connections, HostId, LocalProvider, Provider};
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
    RecoverSeats(oneshot::Sender<Result<()>>),
    GuiHosts(oneshot::Sender<Result<Vec<bbt_proto::GuiHost>>>),
    ControlCatalogue(Ulid, u64, oneshot::Sender<Result<serde_json::Value>>),
    ControlCall(
        Ulid,
        u64,
        crate::mcp::Call,
        oneshot::Sender<Result<serde_json::Value>>,
    ),
    GuiSpawn(
        bbt_proto::SpawnSpec,
        oneshot::Sender<Result<bbt_proto::GuiSeat>>,
    ),
    GuiInput(
        Ulid,
        u64,
        Ulid,
        Vec<u8>,
        oneshot::Sender<Result<crate::InputReceipt>>,
    ),
    GuiResize(Ulid, u64, u16, u16, oneshot::Sender<Result<()>>),
    GuiEndpoint(oneshot::Sender<Result<EndpointInfo>>),
    GuiSeats(oneshot::Sender<Result<Vec<crate::seat_socket::GuiSeat>>>),
    GuiSeat(Ulid, oneshot::Sender<Result<crate::seat_socket::GuiSeat>>),
    GuiIdentity(oneshot::Sender<Result<bbt_proto::GuiIdentity>>),
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
    IdentityStatus(oneshot::Sender<Result<crate::identity::IdentityStatus>>),
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
    exit_cause: Arc<OnceLock<String>>,
}
impl LogicClient {
    /// Resume missing local processes after the app has replaced the supervisor.
    /// A logic-only restart must not call this: its PTYs are still owned.
    pub async fn recover_seats(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::RecoverSeats(tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    pub async fn gui_hosts(&self) -> Result<Vec<bbt_proto::GuiHost>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiHosts(tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }

    fn unavailable(&self, context: &str) -> crate::store::Error {
        match self.exit_cause.get() {
            Some(cause) => invalid(format!("{context}; worker exit: {cause}")),
            None => invalid(context),
        }
    }

    fn record_exit(&self, cause: String, log: bool) {
        if self.exit_cause.set(cause).is_ok() && log {
            eprintln!("logic worker exited: {}", self.exit_cause.get().unwrap());
        }
    }

    /// Portable status only; key custody stays inside the identity runtime.
    pub async fn gui_identity(&self) -> Result<bbt_proto::GuiIdentity> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiIdentity(tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    /// Read-only identity-card data for the in-process GUI. Primary signing is
    /// deliberately absent from this client and from the seat/MCP surface.
    pub async fn identity_status(&self) -> Result<crate::identity::IdentityStatus> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::IdentityStatus(tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    pub async fn control_catalogue(
        &self,
        seat: Ulid,
        generation: u64,
    ) -> Result<serde_json::Value> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::ControlCatalogue(seat, generation, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    pub async fn control_call(
        &self,
        seat: Ulid,
        generation: u64,
        call: crate::mcp::Call,
    ) -> Result<serde_json::Value> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::ControlCall(seat, generation, call, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| {
            self.unavailable("control outcome unavailable; query the original operation ID")
        })?
    }
    pub async fn gui_spawn(&self, spec: bbt_proto::SpawnSpec) -> Result<bbt_proto::GuiSeat> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiSpawn(spec, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("spawn outcome unknown; inspect the seat list"))?
    }
    /// Enqueue operator input fenced to the process generation shown by the GUI.
    pub async fn gui_input(
        &self,
        seat: Ulid,
        generation: u64,
        id: Ulid,
        bytes: Vec<u8>,
    ) -> Result<crate::InputReceipt> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiInput(seat, generation, id, bytes, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("input outcome unknown; keep the original input id"))?
    }
    /// Resize only the displayed process generation; validation runs on the worker.
    pub async fn gui_resize(
        &self,
        seat: Ulid,
        generation: u64,
        cols: u16,
        rows: u16,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiResize(seat, generation, cols, rows, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("resize outcome unknown"))?
    }
    pub async fn gui_endpoint(&self) -> Result<EndpointInfo> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiEndpoint(tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    pub async fn gui_seats(&self) -> Result<Vec<crate::seat_socket::GuiSeat>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiSeats(tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    pub async fn gui_seat(&self, id: Ulid) -> Result<crate::seat_socket::GuiSeat> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiSeat(id, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
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
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| {
            self.unavailable("message operation outcome unknown; query original message ID")
        })?
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
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
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
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("launch outcome unknown; query original seat ID"))?
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
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
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
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
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
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    pub async fn spawn(&self, record: SeatRecord, spec: LaunchSpec) -> Result<SeatInfo> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Spawn(Box::new(record), spec, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("spawn outcome unknown; query original seat id"))?
    }
    pub async fn snapshot(&self, seat: Ulid, generation: u64) -> Result<Snapshot> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Snapshot(seat, generation, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    pub async fn record(&self, seat: Ulid) -> Result<SeatRecord> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Record(seat, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    pub async fn endpoint(&self, seat: Ulid) -> Result<EndpointInfo> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Endpoint(seat, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
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
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("close outcome unknown"))?
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
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("undo outcome unknown"))?
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
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
}

pub struct Logic {
    pub client: LogicClient,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<Result<()>>>,
}
enum IdentityMode {
    Explicit(Box<SigningKey>),
    Custodied(Option<Arc<dyn crate::identity::DeviceKeyStore + Send + Sync>>),
}
impl Logic {
    /// Production construction: custody may be pending while PTYs, the GUI
    /// and public history remain live. No caller substitutes a fresh issuer.
    pub fn start_custodied(
        connection: impl SupervisorConnection + 'static,
        config: LogicConfig,
        handle: &tokio::runtime::Handle,
    ) -> Result<Self> {
        Self::start_with_custody(connection, config, None, handle)
    }
    /// Production dummy hosts share the one admitted supervisor and journal.
    pub fn start_custodied_with_fixtures(
        connection: impl SupervisorConnection + 'static,
        config: LogicConfig,
        handle: &tokio::runtime::Handle,
    ) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let registry = Registry::new(SqliteStore::open(&object_dir(&config.run_dir, "account")?)?);
        let owners = registry
            .list()?
            .into_iter()
            .map(|seat| (seat.id, HostId::from(&seat.location)));
        let providers = crate::provider::shared::fixture_providers(
            connection,
            owners,
            Arc::new(crate::provider::fake::RealClock::default()),
        )?;
        let mut seed = [0; 32];
        getrandom::fill(&mut seed)?;
        let bus_key = SigningKey::from_bytes(&seed);
        seed.fill(0);
        Self::start_inner(
            providers,
            config,
            bus_key,
            IdentityMode::Custodied(None),
            handle,
        )
    }
    /// Isolated custody seam for IO tests; production uses start_custodied.
    pub fn start_custodied_with_store(
        connection: impl SupervisorConnection + 'static,
        config: LogicConfig,
        store: Arc<dyn crate::identity::DeviceKeyStore + Send + Sync>,
        handle: &tokio::runtime::Handle,
    ) -> Result<Self> {
        Self::start_with_custody(connection, config, Some(store), handle)
    }
    fn start_with_custody(
        connection: impl SupervisorConnection + 'static,
        config: LogicConfig,
        store: Option<Arc<dyn crate::identity::DeviceKeyStore + Send + Sync>>,
        handle: &tokio::runtime::Handle,
    ) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let mut seed = [0; 32];
        getrandom::fill(&mut seed)?;
        let bus_key = SigningKey::from_bytes(&seed);
        seed.fill(0);
        Self::start_inner(
            vec![Box::new(LocalProvider::new(connection))],
            config,
            bus_key,
            IdentityMode::Custodied(store),
            handle,
        )
    }
    /// Bootstrap is blocking and belongs on an IO/bootstrap worker. Steady-state
    /// clients and seat sockets use bounded async messages on the supplied R1 handle.
    pub fn start(
        connection: impl SupervisorConnection + 'static,
        config: LogicConfig,
        key: SigningKey,
        handle: &tokio::runtime::Handle,
    ) -> Result<Self> {
        Self::start_with_providers(
            vec![Box::new(LocalProvider::new(connection))],
            config,
            key,
            handle,
        )
    }
    /// IO/bootstrap seam for providers. Local callers keep their admitted link.
    pub fn start_with_providers(
        providers: Vec<Box<dyn Provider>>,
        config: LogicConfig,
        key: SigningKey,
        handle: &tokio::runtime::Handle,
    ) -> Result<Self> {
        Self::start_inner(
            providers,
            config,
            key.clone(),
            IdentityMode::Explicit(Box::new(key)),
            handle,
        )
    }
    fn start_inner(
        providers: Vec<Box<dyn Provider>>,
        config: LogicConfig,
        key: SigningKey,
        identity: IdentityMode,
        handle: &tokio::runtime::Handle,
    ) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let custodied = matches!(&identity, IdentityMode::Custodied(_));
        let identity = match identity {
            IdentityMode::Explicit(local) => {
                crate::identity::IdentityRuntime::open(&config.run_dir, *local, handle.clone())?
            }
            IdentityMode::Custodied(Some(store)) => {
                crate::identity::IdentityRuntime::open_custodied_with_store(
                    &config.run_dir,
                    store,
                    handle.clone(),
                )?
            }
            IdentityMode::Custodied(None) => {
                crate::identity::IdentityRuntime::open_custodied(&config.run_dir, handle.clone())?
            }
        };
        let ready = if custodied {
            identity.bus_signer()?
        } else {
            Some(key.clone())
        };
        let bus_window = if ready.is_none() {
            Some(crate::identity::begin_bus_window(&config.run_dir, &key)?)
        } else {
            None
        };
        let key = ready.unwrap_or(key);
        let mut broker = Broker::open(&config.run_dir, key.clone())?;
        if let Some(run) = bus_window {
            broker.identity_signer(key, Some(run));
        } else if custodied {
            broker.identity_signer(key, None);
        }
        let bus = BusHandle::start(handle, broker, 128)?;
        let (tx, rx) = mpsc::channel(128);
        let client = LogicClient {
            tx,
            bus: bus.clone(),
            exit_cause: Arc::new(OnceLock::new()),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let runtime = handle.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let worker_client = client.clone();
        let reporter = client.clone();
        let thread = std::thread::Builder::new()
            .name("bbt-logic".into())
            .spawn(move || {
                // Keep the receiver until the exit cause is published. A
                // blocked caller must not observe channel closure first.
                let mut rx = rx;
                let mut worker = match Worker::open(
                    providers,
                    config,
                    bus,
                    runtime,
                    worker_client,
                    identity,
                    bus_window,
                ) {
                    Ok(worker) => worker,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.to_string()));
                        reporter.record_exit(error.to_string(), true);
                        return Err(error);
                    }
                };
                let _ = ready_tx.send(Ok(()));
                let execution = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let result = worker.run(&mut rx, stopping);
                    let seal = worker.seal_active_hold();
                    result.and(seal)
                }));
                let result = match execution {
                    Ok(result) => {
                        if let Err(error) = &result {
                            reporter.record_exit(error.to_string(), true);
                        }
                        result
                    }
                    Err(payload) => {
                        let cause = payload
                            .downcast_ref::<String>()
                            .map(String::as_str)
                            .or_else(|| payload.downcast_ref::<&str>().copied())
                            .unwrap_or("non-string panic payload");
                        // The existing panic hook has logged once. Preserve
                        // its cause for callers, then retain panic semantics.
                        reporter.record_exit(format!("panic: {cause}"), false);
                        std::panic::resume_unwind(payload);
                    }
                };
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
            thread
                .join()
                .map_err(|_| self.client.unavailable("logic panicked"))??;
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
// Only explicit launch context is persisted; inherited environment is never
// captured. A new overlay supplies fresh endpoint capabilities on every launch.
#[derive(Serialize, Deserialize)]
struct RecoveryLaunch {
    cwd: PathBuf,
    role_prompt: String,
    user_home: PathBuf,
    user_zdotdir: Option<PathBuf>,
}
struct ViewState {
    info: SeatInfo,
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
    connections: Connections,
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
    gui_info: EndpointInfo,
    gui_endpoint: Option<crate::seat_socket::SeatEndpoint>,
    messaging: crate::messaging::Messaging,
    identity: crate::identity::IdentityRuntime,
    bus_window: Option<Ulid>,
    last_identity: Option<Vec<u8>>,
    deliveries: tokio::task::JoinSet<Delivery>,
    control_journals: BTreeMap<Ulid, crate::mcp::journal::Journal>,
    stale_gui_requests: u64,
}
impl Worker {
    fn open(
        providers: Vec<Box<dyn Provider>>,
        config: LogicConfig,
        bus: BusHandle,
        handle: tokio::runtime::Handle,
        client: LogicClient,
        identity: crate::identity::IdentityRuntime,
        bus_window: Option<Ulid>,
    ) -> Result<Self> {
        let connections = Connections::open(providers)?;
        let registry = Registry::new(SqliteStore::open(&object_dir(&config.run_dir, "account")?)?);
        let index = Index::open(
            &config.repository.join(".bboxy"),
            SqliteStore::open(&object_dir(&config.run_dir, "index")?)?,
        )?;
        let watch = FileWatch::start(&config.repository.join(".bboxy"))?;
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
        let messaging = crate::messaging::Messaging::open(&config.run_dir, identity.keys())?;
        let mut capability = [0; 32];
        getrandom::fill(&mut capability).map_err(std::io::Error::other)?;
        let gui_info = EndpointInfo {
            path: endpoint_root.join("gui.sock"),
            generation: 1,
            capability,
        };
        let gui_endpoint = crate::seat_socket::SeatEndpoint::bind_gui(
            &handle,
            &gui_info.path,
            client.clone(),
            capability,
        )?;
        let mut worker = Self {
            connections,
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
            gui_info,
            gui_endpoint: Some(gui_endpoint),
            messaging,
            identity,
            bus_window,
            last_identity: None,
            deliveries: tokio::task::JoinSet::new(),
            control_journals: BTreeMap::new(),
            stale_gui_requests: 0,
        };
        for (host, info) in worker.connections.owned_seats()? {
            // Several fixture links can wrap one local supervisor. Recover a
            // seat only through the link selected by its durable location.
            if worker.host_for_seat(info.seat)? == host {
                worker.attach(info, false)?;
            }
        }
        worker.scan()?;
        // Recover audit outboxes after a logic crash, even without a new call.
        for record in worker.registry.list()? {
            worker.control_journal(record.id)?;
        }
        worker.publish_control_audits()?;
        worker.refresh_identity()?;
        Ok(worker)
    }
    fn host_for_seat(&self, seat: Ulid) -> Result<HostId> {
        match self.registry.get(seat) {
            Ok(record) => Ok(HostId::from(&record.location)),
            // Preserve the existing recovery of supervisor-owned local seats
            // predating registry metadata. Other storage failures propagate.
            Err(error) if error.to_string() == "unknown seat" => Ok(HostId::Local),
            Err(error) => Err(error),
        }
    }
    fn request_for_seat(&mut self, seat: Ulid, action: Action) -> Result<Reply> {
        let host = self.host_for_seat(seat)?;
        // The registry read/lease has ended before a potentially long request.
        self.connections.request(&host, action)
    }
    fn control_actor(
        &mut self,
        id: Ulid,
        generation: u64,
    ) -> Result<(crate::mcp::Actor, SeatRecord)> {
        self.snapshot(id, generation)?;
        let Reply::Seats { seats } = self.request_for_seat(id, Action::ListOwnedSeats)? else {
            return Err(invalid("supervisor ownership unavailable"));
        };
        if !seats
            .iter()
            .any(|seat| seat.seat == id && seat.generation == generation)
        {
            return Err(invalid("stale control process generation"));
        }
        let (record, grant_epoch, issuer_kind) = self.messaging.authority(id, generation)?;
        Ok((
            crate::mcp::Actor {
                seat_id: id,
                process_generation: generation,
                swarm_id: record.swarm,
                role: record.role.clone(),
                grant_epoch,
                issuer_kind,
            },
            record,
        ))
    }
    fn control_journal(&mut self, id: Ulid) -> Result<crate::mcp::journal::Journal> {
        if let Some(journal) = self.control_journals.get(&id) {
            return Ok(journal.clone());
        }
        let journal = crate::mcp::journal::Journal::new(SqliteStore::open(&object_dir(
            &self.config.run_dir,
            &format!("seat-{id}"),
        )?)?);
        self.control_journals.insert(id, journal.clone());
        Ok(journal)
    }
    fn publish_control_audits(&self) -> Result<()> {
        for (id, journal) in &self.control_journals {
            for event in journal.pending(32)? {
                let event_id = event.id;
                self.publish(&format!("seat-{id}"), event)?;
                journal.published(event_id)?;
            }
        }
        Ok(())
    }
    fn control_call(
        &mut self,
        id: Ulid,
        generation: u64,
        call: crate::mcp::Call,
    ) -> Result<serde_json::Value> {
        let (actor, record) = self.control_actor(id, generation)?;
        call.validate()?;
        let journal = self.control_journal(id)?;
        journal.call(&actor, &call, || {
            match self.control_read(&actor, &record, &call) {
                Ok(value) => crate::mcp::success(call.op_id, value),
                Err(reason) => crate::mcp::error(call.op_id, "rejected", &reason),
            }
        })
    }
    fn control_seat(&self, record: &SeatRecord) -> serde_json::Value {
        serde_json::json!({"record":record,"adapter":self.views.get(&record.id).filter(|v|v.generation==record.generation).map(|v|&v.adapter.status)})
    }
    fn control_read(
        &mut self,
        actor: &crate::mcp::Actor,
        record: &SeatRecord,
        call: &crate::mcp::Call,
    ) -> std::result::Result<serde_json::Value, String> {
        use crate::mcp::tools;
        use crate::registry::Class;
        use serde_json::json;
        match call.tool.as_str() {
            "orient" => {
                let _: tools::Empty = tools::parse(&call.args)?;
                let channels = self
                    .messaging
                    .request(
                        record.id,
                        record.generation,
                        crate::messaging::api::Request::Ls {
                            path: format!("seats/{}", record.id),
                            after: None,
                            limit: 32,
                        },
                        false,
                        None,
                        now_ms(),
                    )
                    .map_err(|e| e.to_string())?;
                Ok(
                    json!({"actor":actor,"seat":self.control_seat(record),"tasks":tools::orient_documents(&self.index,record),
                    "execution":tools::graph(&self.index,actor,record,&tools::GraphRead{graph:"execution".into(),after:None,limit:Some(32)},true)?,
                    "channels":channels,"unread_count":null,"unread_unavailable":"no read cursor exists; delivery and application acknowledgment are separate evidence",
                    "pending_application_ack":self.messaging.pending_application_ack(record.id,record.generation).map_err(|e|e.to_string())?}),
                )
            }
            "self_status" => {
                let _: tools::Empty = tools::parse(&call.args)?;
                Ok(json!({"actor":actor,"seat":self.control_seat(record)}))
            }
            "seat_get" => {
                let args: tools::SeatGet = tools::parse(&call.args)?;
                let target = self
                    .registry
                    .get(args.seat_id)
                    .map_err(|_| "seat unavailable")?;
                if actor.role.class != Class::Oracle && target.swarm != record.swarm {
                    return Err("seat outside caller swarm".into());
                }
                Ok(self.control_seat(&target))
            }
            "seat_list" => {
                let args: tools::SeatList = tools::parse(&call.args)?;
                let limit = tools::page_limit(args.limit)?;
                let scope = args.scope.as_deref().unwrap_or("swarm");
                if !matches!(scope, "swarm" | "account") {
                    return Err("invalid seat scope".into());
                }
                let swarm = args.swarm_id.unwrap_or(record.swarm);
                if actor.role.class != Class::Oracle
                    && (scope == "account" || swarm != record.swarm)
                {
                    return Err("cross-swarm reads require oracle-class authority".into());
                }
                if scope == "account" && args.swarm_id.is_some() {
                    return Err("account scope cannot also select a swarm".into());
                }
                let mut records = self.registry.list().map_err(|e| e.to_string())?;
                records.retain(|r| {
                    (scope == "account" || r.swarm == swarm)
                        && args.after.is_none_or(|after| r.id > after)
                });
                records.sort_by_key(|r| r.id);
                let more = records.len() > limit;
                records.truncate(limit);
                Ok(
                    json!({"items":records.iter().map(|r|self.control_seat(r)).collect::<Vec<_>>(),"next":if more{records.last().map(|r|r.id)}else{None}}),
                )
            }
            "graph_read" => {
                let args: tools::GraphRead = tools::parse(&call.args)?;
                tools::graph(&self.index, actor, record, &args, false)
            }
            _ => Err("tool is unavailable for this actor and catalogue".into()),
        }
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
        let reply = self.request_for_seat(
            info.seat,
            Action::AttachRing {
                seat: info.seat,
                generation: info.generation,
                directory: directory.join(format!("ring-{}", info.generation)),
                settings: self.config.ring.clone(),
            },
        )?;
        let Reply::Attached {
            attachment,
            first_available_sequence,
            ..
        } = reply
        else {
            return Err(invalid("ring attach failed"));
        };
        let Reply::RingRecovery { status, checkpoint } = self.capture_request(
            info.seat,
            Action::RingRecovery {
                attachment: attachment.clone(),
            },
        )?
        else {
            return Err(invalid("ring recovery reply mismatch"));
        };
        let mut state = if let Some(bytes) = checkpoint {
            Playback::restore(RingSaved::decode(&bytes)?)
        } else {
            let mut state = Playback::new(info.seat, info.generation, info.size);
            if let Some(bytes) = SqliteStore::open(&directory)?.get("restart_checkpoint")?
                && !bytes.is_empty()
            {
                let saved = RingSaved::decode(&bytes)?;
                if saved.generation == info.generation {
                    state = Playback::restore(saved);
                }
            }
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
            self.capture_request(
                info.seat,
                Action::PublishCheckpoint {
                    attachment: attachment.clone(),
                    checkpoint: state.saved().encode()?,
                },
            )?;
            state
        };
        let checkpoint_bytes = state.byte_end;
        let checkpoint_sequence = state.sequence;
        let tap = OscTap::install(&mut state.term);
        let mut live_frames = Vec::new();
        while state.sequence < status.captured_sequence {
            let Reply::RingHistory { history } = self.capture_request(
                info.seat,
                Action::ReadRing {
                    attachment: attachment.clone(),
                    from: state.sequence + 1,
                    to: status.captured_sequence,
                },
            )?
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
                info: info.clone(),
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
        SqliteStore::open(&directory)?.put(
            "process_running",
            if info.exited.is_some() { b"0" } else { b"1" },
        )?;
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
                self.request_for_seat(
                    info.seat,
                    Action::ScheduleClose {
                        attachment,
                        close_id: close.operation,
                        deadline_ms: close.deadline_ms,
                    },
                )?;
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
        let mut plan = crate::adapters::launch::plan(
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
        if let Some(bytes) = store.get("restart_checkpoint")?
            && !bytes.is_empty()
        {
            let saved = RingSaved::decode(&bytes)?;
            if saved.generation == record.generation {
                plan.spec.size = crate::SeatSize {
                    cols: saved.cols.try_into()?,
                    rows: saved.rows.try_into()?,
                    ..Default::default()
                };
            }
        }
        store.put(
            "recovery_launch",
            &serde_json::to_vec(&RecoveryLaunch {
                cwd: options.cwd,
                role_prompt: options.role_prompt,
                user_home: options.user_home,
                user_zdotdir: options.user_zdotdir,
            })?,
        )?;
        store.put("adapter_record", &serde_json::to_vec(&harness)?)?;
        store.put(
            "adapter_overlay",
            plan.overlay.file_name().unwrap().as_encoded_bytes(),
        )?;
        // On an ambiguous spawn keep the overlay; deleting it could break a child
        // that actually started. Reconciliation uses the original durable seat ID.
        self.spawn(record, plan.spec)
    }
    fn recover_seats(&mut self) -> Result<()> {
        for mut record in self.registry.list()? {
            if record.location != crate::registry::Location::Local
                || record.lifecycle != Lifecycle::Open
                || self.views.contains_key(&record.id)
            {
                continue;
            }
            let root = object_dir(&self.config.run_dir, &format!("seat-{}", record.id))?;
            let store = SqliteStore::open(&root)?;
            if store.get("process_running")?.as_deref() != Some(b"1") {
                continue;
            }
            let Some(bytes) = store.get("recovery_launch")? else {
                continue;
            };
            let context: RecoveryLaunch = serde_json::from_slice(&bytes)?;
            let previous = record.generation;
            let ring_root = root.join(format!("ring-{previous}"));
            // The old supervisor is already reaped. Reopening its ring rejects
            // torn/uncommitted suffixes and replays only durable capture.
            if !ring_root.join("committed").is_file() && !ring_root.join("previous").is_file() {
                return Err(invalid(format!(
                    "RecoveryUnavailable: seat {} has no committed predecessor generation {previous}",
                    record.id
                )));
            }
            let mut state = crate::ring::Ring::open(
                &ring_root,
                record.id,
                previous,
                0,
                self.config.ring.clone(),
            )?
            .replay()?;
            let retained = format!(
                "generation {previous}, committed through capture sequence {} ({} bytes)",
                state.sequence, state.byte_end
            );
            record.generation = previous
                .checked_add(1)
                .ok_or_else(|| invalid("generation exhausted"))?;
            // Open seats may still retain an already-undone close handle.
            // Its restoration operation belongs to the dead process only.
            record.close = None;
            let resume = record.harness_session_id.is_some()
                && matches!(record.harness_kind.as_str(), "claude" | "codex");
            let harness = if resume {
                serde_json::from_slice(
                    &store
                        .get("adapter_record")?
                        .ok_or_else(|| invalid("missing resume harness"))?,
                )?
            } else {
                record.harness_kind = "zsh".into();
                record.harness_session_id = None;
                HarnessRecord::builtin("zsh")?
            };
            state.generation = record.generation;
            state.gap = Some(format!(
                "restarted; retained {retained}; uncommitted output unavailable{}",
                state
                    .gap
                    .as_ref()
                    .map_or(String::new(), |gap| format!("; {gap}"))
            ));
            state.sequence = 0;
            state.byte_end = 0;
            state.index.clear();
            store.put("restart_checkpoint", &state.saved().encode()?)?;
            store.put("previous_ring", &previous.to_le_bytes())?;
            self.registry.update(record.id, |saved| {
                *saved = record.clone();
                Ok(())
            })?;
            let cwd = state
                .cwd
                .as_ref()
                .map(PathBuf::from)
                .filter(|path| path.is_absolute() && path.is_dir())
                .unwrap_or(context.cwd);
            self.launch(
                record,
                harness,
                crate::adapters::launch::LaunchOptions {
                    cwd,
                    role_prompt: context.role_prompt,
                    executable: std::env::current_exe()?,
                    user_home: context.user_home,
                    user_zdotdir: context.user_zdotdir,
                    resume,
                },
            )?;
        }
        Ok(())
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
        let Reply::Spawned { info } = self.request_for_seat(record.id, Action::Spawn { spec })?
        else {
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
        // Other GUI connections must learn about a new unwatched seat even
        // when its adapter remains in the initial state until the first output.
        self.publish_adapter(info.seat, self.views[&info.seat].adapter.status.clone())?;
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
        let Reply::RingHistory { history } = self.capture_request(
            id,
            Action::ReadRing {
                attachment,
                from,
                to,
            },
        )?
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
        let result = self.capture_request(
            id,
            Action::PublishCheckpoint {
                attachment: self.attachments[&id].clone(),
                checkpoint: view.saved().encode()?,
            },
        );
        if result.as_ref().is_err_and(|error| {
            is_capture_busy(error) || crate::provider::fake::is_link_down(error)
        }) {
            // Keep the previous checkpoint cursor and retry on the next idle
            // pass. The supervisor still owns the raw records; Busy or a down
            // host is not an acknowledged checkpoint and must not stop logic.
            return Ok(());
        }
        result?;
        let view = self.views.get_mut(&id).unwrap();
        view.checkpoint_bytes = view.byte_end;
        view.checkpoint_sequence = sequence;
        Ok(())
    }
    // false retains this event while replay yields to the command queue.
    fn event(&mut self, event: Event) -> Result<bool> {
        let id = event.seat();
        let seq = event.sequence().unwrap();
        let generation = match &event {
            Event::Output { generation, .. }
            | Event::Resized { generation, .. }
            | Event::Exited { generation, .. } => *generation,
        };
        let Some(view) = self.views.get(&id) else {
            return Ok(true);
        };
        if view.generation != generation || seq <= view.sequence {
            return Ok(true);
        }
        if seq > view.sequence + 1 && !self.catch_up(id, seq - 1)? {
            return Ok(false);
        }
        let view = self.views.get_mut(&id).unwrap();
        let before = view.adapter.status.clone();
        view.state.apply(&event)?;
        view.info.next_sequence = seq.saturating_add(1);
        match &event {
            Event::Resized { size, .. } => view.info.size = *size,
            Event::Exited { status, .. } => view.info.exited = Some(status.clone()),
            Event::Output { .. } => {}
        }
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
            SqliteStore::open(&object_dir(&self.config.run_dir, &format!("seat-{id}"))?)?
                .put("process_running", b"0")?;
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
        Ok(true)
    }
    fn catch_up(&mut self, id: Ulid, through: u64) -> Result<bool> {
        let current = &self.views[&id];
        if current.sequence >= through {
            self.save(id)?;
            return Ok(true);
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
        if view.sequence >= through {
            self.save(id)?;
            Ok(true)
        } else {
            // The caller keeps the final live event. Its sequence and this
            // applied cursor survive the command-service turn between pages.
            Ok(false)
        }
    }
    fn flush_protocol_replies(&mut self, id: Ulid) -> Result<()> {
        loop {
            let Some((input_id, bytes)) = self.views[&id].replies.front().cloned() else {
                return Ok(());
            };
            match self.request_for_seat(
                id,
                Action::Input {
                    attachment: self.attachments[&id].clone(),
                    input_id,
                    bytes,
                },
            ) {
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
        let host = self.host_for_seat(seat)?;
        let context = match self.connections.request(
            &host,
            Action::InputContext {
                attachment: attachment.clone(),
            },
        ) {
            Ok(Reply::InputContext { context }) => context,
            _ => {
                return Ok(pending(
                    "pending for operator: live PTY arbiter unavailable",
                ));
            }
        };
        let connections = &mut self.connections;
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
        view.adapter.submit_pty(
            &arbiter,
            now,
            work.id,
            text,
            &journal,
            |bytes| match connections.request(
                &host,
                Action::GuardedInput {
                    attachment,
                    input_id: work.id,
                    bytes,
                    expected: context,
                },
            )? {
                Reply::Input { receipt } => Ok(receipt),
                _ => Err(invalid("guarded input result unknown")),
            },
        )
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
    fn seal_active_hold(&mut self) -> Result<()> {
        if let Some(run) = self.bus_window.take() {
            let ranges = self.handle.block_on(self.bus.hold_ranges())?;
            crate::identity::seal_bus_window(&self.config.run_dir, run, ranges)?;
        }
        Ok(())
    }
    fn refresh_identity(&mut self) -> Result<()> {
        self.identity.retry_custody()?;
        if let Some(key) = self.identity.bus_signer()? {
            if let Some(run) = self.bus_window.take() {
                let ranges = self.handle.block_on(self.bus.identity_signer(key, None))?;
                crate::identity::seal_bus_window(&self.config.run_dir, run, ranges)?;
            }
            // Attribution can stay visibly ephemeral while device custody is
            // briefly unavailable; retry it on the next identity pass.
            let _ = self.identity.close_open_holds(&self.config.run_dir);
        } else if self.bus_window.is_none() {
            let mut seed = [0; 32];
            getrandom::fill(&mut seed)?;
            let key = SigningKey::from_bytes(&seed);
            seed.fill(0);
            let run = crate::identity::begin_bus_window(&self.config.run_dir, &key)?;
            self.handle
                .block_on(self.bus.identity_signer(key, Some(run)))?;
            self.bus_window = Some(run);
        }
        self.messaging
            .resume_signing(self.identity.keys(), now_ms())?;
        let status = self.identity.status()?;
        let bytes = serde_json::to_vec(&status)?;
        if self.last_identity.as_ref() != Some(&bytes) {
            self.publish(
                "account",
                Draft::new(
                    "account/identity/state",
                    "logic",
                    serde_json::to_value(status)?,
                ),
            )?;
            self.last_identity = Some(bytes);
        }
        Ok(())
    }
    fn gui_identity(&self) -> Result<bbt_proto::GuiIdentity> {
        use crate::identity::IdentityState;
        use bbt_proto::GuiIdentityState;
        let status = self.identity.status()?;
        Ok(bbt_proto::GuiIdentity {
            state: match status.state {
                IdentityState::LocalDev => GuiIdentityState::LocalDev,
                IdentityState::Pending => GuiIdentityState::Pending,
                IdentityState::Certified => GuiIdentityState::Certified,
                IdentityState::Held => GuiIdentityState::Held,
            },
            reason: status.reason_label,
        })
    }
    fn gui_seat(&self, id: Ulid) -> Result<crate::seat_socket::GuiSeat> {
        let view = self
            .views
            .get(&id)
            .ok_or_else(|| invalid("seat has no live process view"))?;
        use bbt_proto::{ProcessStatus, SeatState, SubagentRow};
        let record = self.registry.get(id)?;
        let status = &view.adapter.status;
        let process = match record.lifecycle {
            Lifecycle::ClosedRetained => ProcessStatus::Retained,
            Lifecycle::ClosedGrace => ProcessStatus::ClosedGrace {
                until_ms: record
                    .close
                    .as_ref()
                    .ok_or_else(|| invalid("close grace has no deadline"))?
                    .deadline_ms,
            },
            Lifecycle::Open => match &view.info.exited {
                Some(exit) => ProcessStatus::Exited {
                    status: i32::try_from(exit.code)?,
                },
                None => ProcessStatus::Running { pid: view.info.pid },
            },
        };
        let title = if !view.title.is_empty() {
            view.title.clone()
        } else {
            status
                .detail
                .clone()
                .unwrap_or_else(|| view.adapter.record.name.clone())
        };
        let subagents = status
            .subagents
            .iter()
            .filter(|agent| {
                matches!(
                    agent.state,
                    crate::adapters::State::Active | crate::adapters::State::Waiting
                )
            })
            .map(|agent| SubagentRow {
                id: agent.id.clone(),
                title: agent.title.clone(),
                activity: gui_activity(agent.state, agent.waiting_kind),
                grade: gui_grade(agent.grade),
            })
            .collect();
        Ok(bbt_proto::GuiSeat {
            record,
            state: SeatState {
                generation: view.info.generation,
                process,
                activity: gui_activity(status.state, status.waiting_kind),
                grade: gui_grade(status.grade),
                title,
                detail: status.detail.clone(),
                turn_open: status.turn_open,
                observed_ms: status.observed_ms,
                subagents,
            },
        })
    }
    // Runs on the serialized worker immediately before attachment/provider
    // lookup, so queued GUI work cannot target a replacement process.
    fn check_gui_generation(&mut self, seat: Ulid, displayed: u64) -> Result<()> {
        let current = self.registry.get(seat)?.generation;
        if displayed == current {
            return Ok(());
        }
        self.stale_gui_requests = self.stale_gui_requests.saturating_add(1);
        let reason = format!(
            "displayed generation {displayed}, current {current}; dropped={}",
            self.stale_gui_requests,
        );
        eprintln!("StaleGeneration: GUI request for seat {seat} rejected: {reason}");
        Err(Box::new(crate::Failure {
            code: crate::FailureCode::StaleGeneration,
            message: reason,
        }))
    }
    fn gui_spawn(&mut self, spec: bbt_proto::SpawnSpec) -> Result<bbt_proto::GuiSeat> {
        let id = HostId::from(&spec.location);
        let host = self
            .connections
            .hosts()
            .into_iter()
            .find(|host| host.id == id)
            .ok_or_else(|| invalid("unknown provider host"))?;
        if !matches!(
            host.state,
            crate::provider::LinkState::Connected { .. }
                | crate::provider::LinkState::Degraded { .. }
        ) {
            return Err(Box::new(crate::provider::fake::LinkDown {
                host: id,
                state: host.state,
            }));
        }
        let harness = HarnessRecord::builtin(&spec.harness_kind)?;
        let mut record = SeatRecord::new(
            Ulid::new(),
            Ulid::from(spec.swarm.0.0),
            spec.name.unwrap_or_else(|| harness.name.clone()),
            harness.kind.clone(),
        );
        record.location = match spec.location {
            bbt_proto::Where::Local => crate::registry::Location::Local,
            bbt_proto::Where::Ssh { host } => crate::registry::Location::Ssh { host },
            bbt_proto::Where::Cloud { provider, host } => {
                crate::registry::Location::Cloud { provider, host }
            }
        };
        record.role = harness.default_role.clone();
        if let Some(role) = spec.role {
            record.role.name = role;
        }
        let resume = spec.resume.is_some();
        record.harness_session_id = spec.resume;
        let info = self.launch(
            record,
            harness,
            crate::adapters::launch::LaunchOptions {
                cwd: spec.cwd.into(),
                role_prompt: String::new(),
                executable: std::env::current_exe()?,
                user_home: std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .ok_or_else(|| invalid("missing user home"))?,
                user_zdotdir: std::env::var_os("ZDOTDIR").map(PathBuf::from),
                resume,
            },
        )?;
        self.gui_seat(info.seat)
    }
    fn refresh_providers(&mut self, recoveries: &mut BTreeSet<HostId>) -> Result<()> {
        use crate::provider::fake::{STATE_SUBJECT, is_link_down};
        use crate::provider::{LinkState, ProviderEvent};
        for event in self.connections.poll() {
            let ProviderEvent::HostState { host, state } = &event;
            if matches!(
                state,
                LinkState::Connected { .. } | LinkState::Degraded { .. }
            ) {
                recoveries.insert(host.clone());
            } else {
                recoveries.remove(host);
            }
            self.publish(
                "account",
                Draft::new(STATE_SUBJECT, "provider", serde_json::to_value(&event)?),
            )?;
        }
        for host in recoveries.clone() {
            let result = (|| {
                let Reply::Seats { seats } =
                    self.connections.request(&host, Action::ListOwnedSeats)?
                else {
                    return Err(invalid("provider ownership reply mismatch"));
                };
                for info in seats {
                    if self.host_for_seat(info.seat)? == host
                        && self
                            .views
                            .get(&info.seat)
                            .is_none_or(|view| view.generation != info.generation)
                    {
                        self.attach(info, false)?;
                    }
                }
                Ok(())
            })();
            match result {
                Ok(()) => {
                    recoveries.remove(&host);
                }
                Err(error) if is_link_down(&error) || is_capture_busy(&error) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
    fn capture_request(&mut self, seat: Ulid, action: Action) -> Result<Reply> {
        debug_assert!(matches!(
            action,
            Action::RingRecovery { .. }
                | Action::ReadRing { .. }
                | Action::PublishCheckpoint { .. }
        ));
        retry_capture(|| self.request_for_seat(seat, action.clone()))
    }
    fn run(&mut self, commands: &mut mpsc::Receiver<Command>, stop: Arc<AtomicBool>) -> Result<()> {
        let mut scan_at = Instant::now();
        let mut adapters_at = Instant::now();
        let mut messages_at = Instant::now();
        let mut identity_at = Instant::now();
        let mut providers_at = Instant::now();
        let mut pending_events = BTreeMap::new();
        let mut provider_recoveries = BTreeSet::new();
        while !stop.load(Ordering::Acquire) {
            for _ in 0..32 {
                let Ok(command) = commands.try_recv() else {
                    break;
                };
                match command {
                    Command::GuiHosts(reply) => {
                        let _ = reply.send(Ok(self.connections.gui_hosts()));
                    }
                    Command::IdentityStatus(reply) => {
                        let _ = reply.send(self.identity.status());
                    }
                    Command::ControlCatalogue(id, generation, reply) => {
                        let result = self
                            .control_actor(id, generation)
                            .map(|(actor, _)| crate::mcp::tools::catalogue(&actor));
                        let _ = reply.send(result);
                    }
                    Command::ControlCall(id, generation, call, reply) => {
                        let result = self.control_call(id, generation, call);
                        let _ = reply.send(result);
                    }
                    Command::RecoverSeats(reply) => {
                        let result = self.recover_seats();
                        let _ = reply.send(result);
                    }
                    Command::GuiSpawn(spec, reply) => {
                        let result = self.gui_spawn(spec);
                        let _ = reply.send(result);
                    }
                    Command::GuiInput(id, generation, input_id, bytes, reply) => {
                        let result = (|| {
                            self.check_gui_generation(id, generation)?;
                            let attachment = self
                                .attachments
                                .get(&id)
                                .ok_or_else(|| invalid("seat not attached"))?
                                .clone();
                            match self.request_for_seat(
                                id,
                                Action::Input {
                                    attachment,
                                    input_id,
                                    bytes,
                                },
                            )? {
                                Reply::Input { receipt } => Ok(receipt),
                                _ => Err(invalid("input receipt mismatch")),
                            }
                        })();
                        let _ = reply.send(result);
                    }
                    Command::GuiResize(id, generation, cols, rows, reply) => {
                        let result = (|| {
                            self.check_gui_generation(id, generation)?;
                            let attachment = self
                                .attachments
                                .get(&id)
                                .ok_or_else(|| invalid("seat not attached"))?
                                .clone();
                            let mut size = self
                                .views
                                .get(&id)
                                .ok_or_else(|| invalid("seat unavailable"))?
                                .info
                                .size;
                            size.cols = cols.clamp(2, 512);
                            size.rows = rows.clamp(1, 256).min(32768 / size.cols);
                            check_size(size.cols as usize, size.rows as usize)?;
                            match self.request_for_seat(id, Action::Resize { attachment, size })? {
                                Reply::Resized { .. } => Ok(()),
                                _ => Err(invalid("resize receipt mismatch")),
                            }
                        })();
                        let _ = reply.send(result);
                    }
                    Command::GuiEndpoint(reply) => {
                        let _ = reply.send(Ok(self.gui_info.clone()));
                    }
                    Command::GuiIdentity(reply) => {
                        let _ = reply.send(self.gui_identity());
                    }
                    Command::GuiSeats(reply) => {
                        let seats = self.views.keys().map(|id| self.gui_seat(*id)).collect();
                        let _ = reply.send(seats);
                    }
                    Command::GuiSeat(id, reply) => {
                        let _ = reply.send(self.gui_seat(id));
                    }
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
                            self.request_for_seat(
                                id,
                                Action::ScheduleClose {
                                    attachment,
                                    close_id: close.operation,
                                    deadline_ms: close.deadline_ms,
                                },
                            )?;
                            self.publish_adapter(id, self.views[&id].adapter.status.clone())?;
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
                            if record.lifecycle == Lifecycle::Open
                                && record.lifecycle_revision == revision.saturating_add(1)
                            {
                                if self
                                    .views
                                    .get(&id)
                                    .is_none_or(|view| view.info.exited.is_some())
                                {
                                    return Err(invalid("process exited"));
                                }
                                // The matching undo already committed. Replaying
                                // its ID returns the same restoration handle.
                                return Ok(close.position.clone());
                            }
                            let attachment = self
                                .attachments
                                .get(&id)
                                .ok_or_else(|| invalid("seat not attached"))?
                                .clone();
                            let accepted_at = now_ms();
                            self.request_for_seat(
                                id,
                                Action::UndoClose {
                                    attachment,
                                    close_id: close.operation,
                                },
                            )?;
                            let position = self.registry.undo(
                                id,
                                generation,
                                revision,
                                layout_revision,
                                accepted_at,
                            )?;
                            self.publish_adapter(id, self.views[&id].adapter.status.clone())?;
                            Ok(position)
                        })();
                        let _ = reply.send(result);
                    }
                }
            }
            for host in self.connections.host_ids() {
                let event = match pending_events.remove(&host) {
                    Some(event) => Some(event),
                    None => self.connections.receive_from(&host)?,
                };
                if let Some(event) = event {
                    if self.host_for_seat(event.seat())? != host {
                        continue;
                    }
                    match self.event(event.clone()) {
                        Ok(true) => {}
                        Ok(false) => {
                            // One page applied; retain the live event and cursor
                            // while commands and other hosts get their turn.
                            pending_events.insert(host, event);
                        }
                        Err(error)
                            if is_capture_busy(&error)
                                || crate::provider::fake::is_link_down(&error) =>
                        {
                            pending_events.insert(host, event);
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            if providers_at.elapsed() >= Duration::from_millis(200) {
                self.refresh_providers(&mut provider_recoveries)?;
                providers_at = Instant::now();
            }
            if adapters_at.elapsed() >= Duration::from_millis(200) {
                self.refresh_adapters()?;
                adapters_at = Instant::now();
            }
            if identity_at.elapsed() >= Duration::from_secs(30) {
                self.refresh_identity()?;
                identity_at = Instant::now();
            }
            if messages_at.elapsed() >= Duration::from_millis(100) {
                self.pump_messages()?;
                self.publish_control_audits()?;
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
fn is_capture_busy(error: &crate::store::Error) -> bool {
    error
        .downcast_ref::<crate::Failure>()
        .is_some_and(|error| error.code == crate::FailureCode::Busy)
}

#[allow(clippy::disallowed_methods)] // Bounded backoff on the blocking logic IO owner, never the UI.
fn retry_capture(mut request: impl FnMut() -> Result<Reply>) -> Result<Reply> {
    for attempt in 0..6 {
        let result = request();
        if !result.as_ref().is_err_and(is_capture_busy) || attempt == 5 {
            return result;
        }
        std::thread::sleep(Duration::from_millis(2 << attempt));
    }
    unreachable!()
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.endpoints.clear();
        drop(self.gui_endpoint.take());
        let _ = std::fs::remove_dir(&self.endpoint_root);
    }
}
fn gui_activity(
    state: crate::adapters::State,
    waiting: Option<crate::adapters::WaitingKind>,
) -> bbt_proto::Activity {
    use crate::adapters::{State, WaitingKind};
    use bbt_proto::{Activity, WaitKind};
    match state {
        State::Active => Activity::Active,
        State::Idle => Activity::Idle,
        State::Errored => Activity::Errored,
        State::Waiting => Activity::Waiting(match waiting {
            Some(WaitingKind::Permission) => WaitKind::Permission,
            Some(WaitingKind::Auth) => WaitKind::Auth,
            _ => WaitKind::Question,
        }),
    }
}
fn gui_grade(grade: crate::adapters::Grade) -> bbt_proto::Grade {
    match grade {
        crate::adapters::Grade::Reported => bbt_proto::Grade::Reported,
        crate::adapters::Grade::Shim => bbt_proto::Grade::Shim,
        crate::adapters::Grade::Inferred => bbt_proto::Grade::Inferred,
    }
}
fn check_size(cols: usize, rows: usize) -> Result<()> {
    if cols < 2 || rows == 0 || cols > 512 || rows > 256 || cols * rows > 32768 {
        return Err(invalid("viewport dimensions exceed bounded logic model"));
    }
    Ok(())
}
