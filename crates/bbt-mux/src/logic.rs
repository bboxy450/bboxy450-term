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

use bbt_supervisor::supervisor::capture_diagnostics::{Outcome, Phase, Recorder};

use crate::bus::{Broker, BusHandle};
use crate::index::{FileWatch, Index};
use crate::knowledge::{BboxySource, DataSource};
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
        Ulid,
        bbt_proto::SpawnSpec,
        crate::mcp::Door,
        oneshot::Sender<Result<bbt_proto::SeatId>>,
    ),
    GuiMessaging(
        Ulid,
        Ulid,
        crate::messaging::api::Request,
        crate::mcp::Door,
        oneshot::Sender<Result<crate::messaging::api::Response>>,
    ),
    GuiTask(
        Ulid,
        bbt_proto::messaging::TaskRequest,
        crate::mcp::Door,
        oneshot::Sender<Result<bbt_proto::messaging::TaskResponse>>,
    ),
    GuiKnowledge(
        String,
        serde_json::Value,
        oneshot::Sender<crate::knowledge::data_source::Result<serde_json::Value>>,
    ),
    GuiMessageRow(
        Ulid,
        Ulid,
        oneshot::Sender<Result<bbt_proto::messaging::MessageRow>>,
    ),
    GuiMessagingSubjects(oneshot::Sender<Result<Vec<String>>>),
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
    GuiDevice(oneshot::Sender<Result<Ulid>>),
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

fn operator_response<T: serde::de::DeserializeOwned>(result: serde_json::Value) -> Result<T> {
    if result["isError"] == true {
        let error = &result["structuredContent"]["error"];
        return Err(invalid(format!(
            "{}: {}",
            error["code"].as_str().unwrap_or("unknown"),
            error["message"]
                .as_str()
                .unwrap_or("operation outcome unavailable")
        )));
    }
    Ok(serde_json::from_value(result["structuredContent"].clone())?)
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
    capture_diagnostics: Arc<Recorder>,
    /// Set while `ctl` is open (decision #260): who may come through `gui.sock` and by which
    /// door. None: the window is the only client, as before.
    gui_peers: Arc<std::sync::RwLock<Option<crate::ctl_door::Peers>>>,
    /// Captures asked of the window on the operator's behalf.
    pub(crate) captures: Arc<crate::gui_capture::Captures>,
}
impl LogicClient {
    pub(crate) fn gui_peers(&self) -> Option<crate::ctl_door::Peers> {
        *self.gui_peers.read().unwrap_or_else(|e| e.into_inner())
    }
    pub(crate) fn set_gui_peers(&self, peers: Option<crate::ctl_door::Peers>) {
        *self.gui_peers.write().unwrap_or_else(|e| e.into_inner()) = peers;
    }
    /// Enable fixed-capacity, numeric capture diagnostics on IO before a run.
    /// Disabled by default; never changes replay, admission or retry policy.
    pub fn enable_capture_diagnostics(&self) {
        self.capture_diagnostics.enable();
    }

    /// Snapshot last-observed cursors and inclusive phase spans without an RPC.
    /// Copy on IO, before diagnostic requests that would affect the window.
    pub fn capture_diagnostics(&self) -> bbt_supervisor::supervisor::capture_diagnostics::Snapshot {
        self.capture_diagnostics.snapshot()
    }

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
    /// Logic resolves the device for the admitted GUI; the peer cannot choose
    /// another device's durable consumer or substitute an ephemeral identity.
    pub(crate) async fn gui_device(&self) -> Result<Ulid> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiDevice(tx))
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
    pub async fn gui_spawn(
        &self,
        id: Ulid,
        spec: bbt_proto::SpawnSpec,
        door: crate::mcp::Door,
    ) -> Result<bbt_proto::SeatId> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiSpawn(id, spec, door, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("spawn outcome unknown; inspect the seat list"))?
    }
    pub async fn gui_messaging(
        &self,
        id: Ulid,
        swarm: Ulid,
        request: crate::messaging::api::Request,
        door: crate::mcp::Door,
    ) -> Result<crate::messaging::api::Response> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiMessaging(id, swarm, request, door, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| {
            self.unavailable("messaging outcome unknown; retry the original operation ID")
        })?
    }
    pub async fn gui_task(
        &self,
        id: Ulid,
        request: bbt_proto::messaging::TaskRequest,
        door: crate::mcp::Door,
    ) -> Result<bbt_proto::messaging::TaskResponse> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiTask(id, request, door, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| {
            self.unavailable("task outcome unknown; retry the original operation ID")
        })?
    }
    /// Only the admitted GUI session calls this read port. The IO owner keeps
    /// the existing Index; hydration reads create no journal, watcher or ACK.
    pub async fn gui_knowledge(
        &self,
        method: String,
        args: serde_json::Value,
    ) -> Result<crate::knowledge::data_source::Result<serde_json::Value>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiKnowledge(method, args, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("knowledge read interrupted"))
    }
    pub async fn gui_message_row(
        &self,
        swarm: Ulid,
        message: Ulid,
    ) -> Result<bbt_proto::messaging::MessageRow> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiMessageRow(swarm, message, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("message projection unavailable"))?
    }
    pub async fn gui_messaging_subjects(&self) -> Result<Vec<String>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiMessagingSubjects(tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("message subjects unavailable"))?
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
            capture_diagnostics: Arc::new(Recorder::default()),
            gui_peers: Arc::default(),
            captures: Arc::default(),
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
    delivery_failure: Option<String>,
    tap: OscTap,
    replies: VecDeque<(Ulid, Vec<u8>)>,
    native_reader: Option<NativeReader>,
    state: Playback,
    view_generation: Ulid,
    checkpoint_bytes: u64,
    checkpoint_sequence: u64,
    changed_at: Instant,
}

// A replay target belongs to a seat, not its entire supervisor host. Keep the
// active target fixed until it finishes; subsequent captured notifications use
// the same bounded coalescer as the supervisor connection.
#[derive(Default)]
struct PendingSeats {
    seats: BTreeMap<(HostId, Ulid), PendingSeat>,
    ready: VecDeque<(HostId, Ulid)>,
    parked: BTreeSet<HostId>,
}
struct PendingSeat {
    active: Event,
    generation: u64,
    latest: u64,
    exited: bool,
    queued: bbt_supervisor::client::PendingEvents,
}
impl PendingSeats {
    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.seats.is_empty()
    }
    fn has_ready(&self) -> bool {
        self.ready
            .iter()
            .any(|(host, _)| !self.parked.contains(host))
    }
    fn park(&mut self, host: &HostId) {
        self.parked.insert(host.clone());
    }
    fn resume(&mut self, host: &HostId) {
        self.parked.remove(host);
    }
    fn push(&mut self, host: HostId, event: Event, generation: u64, first: u64) -> Result<()> {
        let key = (host, event.seat());
        let sequence = event.sequence().unwrap();
        let exited = matches!(event, Event::Exited { .. });
        if let Some(pending) = self.seats.get_mut(&key)
            && pending.generation == generation
        {
            if sequence <= pending.latest {
                return Ok(());
            }
            if pending.exited {
                return Err(invalid("supervisor event follows process exit"));
            }
            pending.queued.push(event)?;
            pending.latest = sequence;
            pending.exited = exited;
            return Ok(());
        }
        let mut queued = bbt_supervisor::client::PendingEvents::default();
        queued.confirm_ring(key.1, generation, first)?;
        let replaced = self.seats.insert(
            key.clone(),
            PendingSeat {
                active: event,
                generation,
                latest: sequence,
                exited,
                queued,
            },
        );
        if replaced.is_none() {
            self.ready.push_back(key);
        }
        Ok(())
    }
    fn next(&mut self) -> Option<((HostId, Ulid), Event)> {
        for _ in 0..self.ready.len() {
            let key = self.ready.pop_front()?;
            if self.parked.contains(&key.0) {
                self.ready.push_back(key);
            } else {
                return Some((key.clone(), self.seats[&key].active.clone()));
            }
        }
        None
    }
    fn finish(&mut self, key: (HostId, Ulid), complete: bool) {
        if complete {
            let pending = self.seats.get_mut(&key).unwrap();
            match pending.queued.pop_front() {
                Some(event) => pending.active = event,
                None => {
                    self.seats.remove(&key);
                    return;
                }
            }
        }
        self.ready.push_back(key);
    }
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
struct ControlDelivery {
    actor: crate::mcp::SeatActor,
    call: crate::mcp::Call,
    journal: crate::mcp::journal::Journal,
    reply: oneshot::Sender<Result<serde_json::Value>>,
    target: crate::mcp::seats::Target,
    input_id: Ulid,
    outcome: Option<crate::adapters::WakeOutcome>,
    result: serde_json::Value,
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
    control_deliveries: tokio::task::JoinSet<ControlDelivery>,
    control_journals: BTreeMap<Ulid, crate::mcp::journal::Journal>,
    provenance: crate::provenance::engine::Engine,
    operator_journal: crate::mcp::journal::Journal,
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
        let account = SqliteStore::open(&object_dir(&config.run_dir, "account")?)?;
        let registry = Registry::new(account.clone());
        let mut index = Index::open(
            &config.repository.join(".bboxy"),
            SqliteStore::open(&object_dir(&config.run_dir, "index")?)?,
        )?;
        index.route_activity(bus.clone(), handle.clone());
        // Markdown is authority even if a crash preceded the mirror update.
        // scan retains its durable outbox; the normal startup scan publishes it.
        let _ = index.scan()?;
        let provenance =
            crate::provenance::engine::Engine::open(&index, &config.repository, handle.clone())?;
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
        let mut messaging = crate::messaging::Messaging::open(&config.run_dir, identity.keys())?;
        if let Some(device_id) = identity.status()?.device_id {
            messaging.set_operator_device(device_id)?;
        }
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
            control_deliveries: tokio::task::JoinSet::new(),
            control_journals: BTreeMap::new(),
            provenance,
            operator_journal: crate::mcp::journal::Journal::new(account),
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
    ) -> Result<(crate::mcp::SeatActor, SeatRecord)> {
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
            crate::mcp::SeatActor {
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
        for event in self.operator_journal.pending(32)? {
            let event_id = event.id;
            self.publish("account", event)?;
            self.operator_journal.published(event_id)?;
        }
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
        let authority = crate::mcp::Actor::from(&actor);
        call.validate()?;
        let journal = self.control_journal(id)?;
        if crate::mcp::seats::handles(&call.tool) {
            return match self.prepare_seat_control(&actor, &record, &call) {
                Ok(request) if request.is_read() => {
                    let value = match self.read_seat_control(&request) {
                        Ok(value) => {
                            let empty = value
                                .get("records")
                                .or_else(|| value.get("rows"))
                                .and_then(serde_json::Value::as_array)
                                .is_some_and(Vec::is_empty);
                            crate::mcp::success(
                                call.op_id,
                                crate::mcp::tools::scoped_list(
                                    &actor,
                                    value,
                                    empty.then_some("nothing_yet"),
                                ),
                            )
                        }
                        Err(reason) => crate::mcp::error(call.op_id, "rejected", &reason),
                    };
                    journal.checked(&actor, &call, value)
                }
                Ok(request) => journal.effect(&actor, &call, || {
                    self.effect_seat_control(&actor, &call, request)
                }),
                Err(reason) => journal.checked(
                    &actor,
                    &call,
                    crate::mcp::error(call.op_id, "rejected", &reason),
                ),
            };
        }
        if crate::mcp::work_items::handles(&call.tool) {
            return self.work_item_call(&authority, &call, &journal);
        }
        if crate::mcp::knowledge::handles(&call.tool) {
            return self.knowledge_call(&actor, &call, &journal);
        }
        if matches!(call.tool.as_str(), "orient" | "graph_read") {
            self.scan()?;
        }
        match crate::mcp::messaging::request(&authority, &call) {
            Ok(Some(request)) if crate::mcp::messaging::mutation(&call.tool) => {
                journal.effect(&actor, &call, || {
                    self.control_message(&actor, &record, &call, request)
                })
            }
            Ok(Some(request)) => {
                // Destination grants can change without changing the seat's
                // role epoch. Recheck W2.4 before revealing a retained read.
                let value = self.control_message(&actor, &record, &call, request);
                journal.checked(&actor, &call, value)
            }
            prepared => {
                let result = match prepared {
                    Err(reason) => Err(reason),
                    Ok(None) => self.control_read(&actor, &record, &call),
                    Ok(Some(_)) => unreachable!(),
                };
                let value = match result {
                    Ok(value) => crate::mcp::success(call.op_id, value),
                    Err(reason) => crate::mcp::error(call.op_id, "rejected", &reason),
                };
                journal.checked_authorized(&actor, &call, value, |old| {
                    let source = BboxySource::new(&self.index).with_registry(&self.registry);
                    crate::mcp::tools::retained_documents(&source, &actor, &record, &call, old)
                })
            }
        }
    }
    fn knowledge_call(
        &mut self,
        actor: &crate::mcp::SeatActor,
        call: &crate::mcp::Call,
        journal: &crate::mcp::journal::Journal,
    ) -> Result<serde_json::Value> {
        use crate::mcp::{self, knowledge as tool};
        self.scan()?;
        match tool::prepare(call) {
            Ok(tool::Request::Fetch(locator)) => {
                let source = BboxySource::new(&self.index);
                let value = match tool::fetch(&source, locator) {
                    Ok(record) => tool::bounded(call.op_id, mcp::success(call.op_id,serde_json::to_value(record)?),false),
                    Err(reason) => mcp::error(call.op_id,"rejected",&reason),
                };
                journal.checked(actor,call,value)
            }
            Ok(tool::Request::Write(request)) => journal.effect(actor,call,|| {
                let keys = self.identity.keys();
                if let Some(reason) = keys.signing_hold() {
                    return mcp::error(call.op_id,"held",&format!("knowledge signing held: {reason}"));
                }
                let key = match keys.leaf(actor.seat_id,actor.process_generation) {
                    Ok(key) if keys.ready_issuer().is_some() => key,
                    _ => return mcp::error(call.op_id,"held","knowledge signing held: identity pending"),
                };
                let context = crate::knowledge::WriteContext::seat(actor.seat_id,Some(actor.process_generation),&key);
                let source = BboxySource::new(&self.index);
                if request.reference.is_none() {
                    match source.resolve(crate::knowledge::data_source::ResolveRequest { locator:call.op_id.to_string() }) {
                        Err(error) if error.code == "not_found" => (),
                        _ => return mcp::error(call.op_id,"rejected","knowledge create id already exists or cannot be resolved uniquely"),
                    }
                }
                let result = source.write(*request,&context);
                match result {
                    Ok(result) => {
                        if self.scan().is_err() {
                            return mcp::error(call.op_id,"unknown","knowledge write applied; activity publication pending, inspect original operation");
                        }
                        match serde_json::to_value(result) {
                            Ok(value) => tool::bounded(call.op_id,mcp::success(call.op_id,value),true),
                            Err(_) => mcp::error(call.op_id,"unknown","knowledge write applied; result serialization unavailable"),
                        }
                    }
                    Err(error) if matches!(error.code.as_str(),"write_not_confirmed"|"write_applied_result_unavailable") => mcp::error(call.op_id,"unknown","knowledge write outcome is not confirmed; inspect original operation, never redispatch it"),
                    Err(error) => mcp::error(call.op_id,&error.code,"knowledge write refused before application"),
                }
            }),
            Err(reason) => journal.checked(actor,call,mcp::error(call.op_id,"rejected",&reason)),
        }
    }
    fn work_item_call(
        &mut self,
        actor: &crate::mcp::Actor,
        call: &crate::mcp::Call,
        journal: &crate::mcp::journal::Journal,
    ) -> Result<serde_json::Value> {
        self.scan()?;
        match crate::mcp::work_items::prepare(&self.index, &self.registry, actor, call) {
            Ok(crate::mcp::work_items::Prepared::Read(value)) => {
                let value = if call.tool == "task_list"
                    && let Some(seat) = actor.seat()
                {
                    let empty = value["items"].as_array().is_some_and(Vec::is_empty);
                    crate::mcp::tools::scoped_list(seat, value, empty.then_some("nothing_assigned"))
                } else {
                    value
                };
                journal.checked_authorized(
                    actor,
                    call,
                    crate::mcp::success(call.op_id, value),
                    |old| {
                        crate::mcp::work_items::retained_allowed(
                            &self.index,
                            &self.registry,
                            actor,
                            call,
                            old,
                        )
                    },
                )
            }
            Ok(crate::mcp::work_items::Prepared::Effect(effect)) => {
                journal.effect(actor, call, || self.control_work_item(actor, call, *effect))
            }
            Err(reason) => journal.checked(
                actor,
                call,
                crate::mcp::error(call.op_id, "rejected", &reason),
            ),
        }
    }
    fn control_work_item(
        &mut self,
        actor: &crate::mcp::Actor,
        call: &crate::mcp::Call,
        effect: crate::mcp::work_items::Effect,
    ) -> serde_json::Value {
        let mut plan = match crate::mcp::work_items::plan(effect, actor, call) {
            Ok(plan) => plan,
            Err(reason) => return crate::mcp::error(call.op_id, "rejected", &reason),
        };
        if call.tool == "artifact_claim" {
            if self.provenance.recovering() {
                return crate::mcp::error(call.op_id, "held", "artifact catalog recovery pending");
            }
            let Some(seat_actor) = actor.seat() else {
                return crate::mcp::error(call.op_id, "rejected", "artifact claim requires a seat");
            };
            let original = call.args["ref"].as_str().unwrap_or_default();
            let normalized = self.provenance.normalize(
                seat_actor.seat_id,
                seat_actor.process_generation,
                original,
            );
            let reference = normalized.as_deref().unwrap_or(original);
            if !self
                .provenance
                .claim_ready(&self.index, reference)
                .unwrap_or(false)
            {
                return crate::mcp::error(call.op_id, "held", "artifact record projection pending");
            }
            if let Err(reason) = crate::provenance::artifacts::reconcile_claim(
                &self.index,
                &mut plan,
                seat_actor,
                call,
                reference,
            ) {
                return crate::mcp::error(call.op_id, "rejected", &reason);
            }
        }
        let keys = self.identity.keys();
        if let Some(reason) = keys.signing_hold() {
            return crate::mcp::error(
                call.op_id,
                "held",
                &format!("work-item signing held: {reason}"),
            );
        }
        let operator = match actor {
            crate::mcp::Actor::Seat(_) => None,
            crate::mcp::Actor::Operator { device_id, .. } => match self.identity.operator_signing()
            {
                Ok(authority) if authority.device_id == *device_id => Some(authority),
                Ok(_) => return crate::mcp::error(call.op_id, "held", "operator device changed"),
                Err(error) => return crate::mcp::error(call.op_id, "held", &error.to_string()),
            },
        };
        let key = match actor {
            crate::mcp::Actor::Seat(actor) => {
                match keys.leaf(actor.seat_id, actor.process_generation) {
                    Ok(key) if keys.ready_issuer().is_some() => key,
                    _ => {
                        return crate::mcp::error(
                            call.op_id,
                            "held",
                            "work-item signing held: identity pending",
                        );
                    }
                }
            }
            crate::mcp::Actor::Operator { .. } => match &operator {
                Some(authority) => authority.key.clone(),
                None => return crate::mcp::error(call.op_id, "held", "operator signer missing"),
            },
        };
        let author = match actor {
            crate::mcp::Actor::Seat(actor) => crate::provenance::author::Author::seat(
                actor.seat_id,
                Some(actor.process_generation),
                &key,
            ),
            crate::mcp::Actor::Operator { device_id, .. } => match &operator {
                Some(authority) => crate::provenance::author::Author::operator(
                    *device_id,
                    authority.issuer.kind,
                    &key,
                ),
                None => return crate::mcp::error(call.op_id, "held", "operator signer missing"),
            },
        };
        for wake in &plan.wakes {
            if let bbt_proto::messaging::Recipient::Operator(device) = wake.to {
                let current = match self.identity.status() {
                    Ok(status) => status.device_id,
                    Err(error) => return crate::mcp::error(call.op_id, "held", &error.to_string()),
                };
                if current != Some(device) {
                    return crate::mcp::error(
                        call.op_id,
                        "held",
                        "task delegator operator device is unavailable",
                    );
                }
                if let Err(error) = self.messaging.set_operator_device(device) {
                    return crate::mcp::error(call.op_id, "held", &error.to_string());
                }
            }
        }
        let task_swarm = plan.writes.iter().find_map(|update| {
            update.metadata["assignee"]
                .as_str()
                .and_then(|reference| {
                    reference
                        .strip_prefix("seat:")
                        .unwrap_or(reference)
                        .parse::<Ulid>()
                        .ok()
                })
                .and_then(|id| self.registry.get(id).ok())
                .map(|seat| seat.swarm)
        });
        let mut wake_swarms = Vec::with_capacity(plan.wakes.len());
        for wake in &plan.wakes {
            // A reassignment can notify both the old and the new assignee.
            // Scope each notice by its actual recipient, never by a default swarm.
            let swarm = match wake.to {
                bbt_proto::messaging::Recipient::Seat(seat) => match self.registry.get(seat) {
                    Ok(record) => record.swarm,
                    Err(_) => {
                        return crate::mcp::error(
                            call.op_id,
                            "rejected",
                            "task recipient is unavailable",
                        );
                    }
                },
                bbt_proto::messaging::Recipient::Operator(_) => match task_swarm {
                    Some(swarm) => swarm,
                    None => {
                        return crate::mcp::error(
                            call.op_id,
                            "rejected",
                            "task swarm requires an assignee",
                        );
                    }
                },
            };
            wake_swarms.push(swarm);
        }
        // Validate every source before the first write of a multi-file claim.
        for update in &plan.writes {
            match self.index.current(&update.path) {
                Ok(current) if current.as_ref().map(|d| &d.digest) == update.expected.as_ref() => {}
                Ok(_) => {
                    return crate::mcp::error(
                        call.op_id,
                        "conflict",
                        "document changed before write",
                    );
                }
                Err(error) => return crate::mcp::error(call.op_id, "rejected", &error.to_string()),
            }
        }
        let mut completed = Vec::new();
        let paths: Vec<_> = plan
            .writes
            .iter()
            .map(|w| format!(".bboxy/{}", w.path))
            .collect();
        for update in plan.writes {
            let artifact = (update.metadata["type"] == "artifact").then(|| update.metadata.clone());
            if self.index.apply_author(update, &author).is_err() {
                return crate::mcp::error(
                    call.op_id,
                    "unknown",
                    &format!(
                        "work-item write outcome unknown; inspect paths {paths:?}; no automatic redispatch"
                    ),
                );
            }
            if let Some(artifact) = artifact {
                self.provenance.note_claim(&artifact);
            }
        }
        for (wake, wake_swarm) in plan.wakes.into_iter().zip(wake_swarms) {
            let mut request = crate::messaging::wire::SendRequest::text(
                match wake.to {
                    bbt_proto::messaging::Recipient::Seat(seat) => format!("seats/{seat}/inbox"),
                    bbt_proto::messaging::Recipient::Operator(_) => "operator/inbox".into(),
                },
                wake.summary.clone(),
            );
            request.id = wake.id;
            request.kind = crate::messaging::wire::MessageKind::Task;
            request.attachments = vec![format!("[[{}]]", wake.task_id)];
            let body = crate::messaging::wire::Body::Task {
                task_id: wake.task_id,
                event: wake.event,
                path: wake.path,
                summary: wake.summary,
            };
            let sent = match actor {
                crate::mcp::Actor::Seat(actor) => self.messaging.task_event(
                    actor.seat_id,
                    actor.process_generation,
                    request,
                    body,
                    now_ms(),
                ),
                crate::mcp::Actor::Operator { .. } => match &operator {
                    Some(authority) => self.messaging.task_event_operator(
                        authority,
                        wake_swarm,
                        request,
                        body,
                        now_ms(),
                    ),
                    _ => Err(invalid("operator task scope/signing is unavailable")),
                },
            };
            match sent {
                Ok(message) => completed.push(message.envelope.id),
                Err(_) => {
                    return crate::mcp::error(
                        call.op_id,
                        "unknown",
                        &format!(
                            "frontmatter written; Task wake {} outcome unknown; completed {completed:?}; inspect {paths:?} and receipts; no automatic redispatch",
                            wake.id
                        ),
                    );
                }
            }
        }
        if self.scan().is_err() {
            return crate::mcp::error(
                call.op_id,
                "unknown",
                &format!(
                    "work-item projection unavailable; inspect {paths:?} and messages {completed:?}"
                ),
            );
        }
        crate::mcp::success(call.op_id, plan.data)
    }
    fn prepare_seat_control(
        &self,
        actor: &crate::mcp::SeatActor,
        caller: &SeatRecord,
        call: &crate::mcp::Call,
    ) -> std::result::Result<crate::mcp::seats::Request, String> {
        let request = crate::mcp::seats::request(actor, call)?;
        let target = request
            .target()
            .map(|target| {
                self.registry
                    .get(target.seat)
                    .map_err(|_| "target unavailable".to_string())
            })
            .transpose()?;
        crate::mcp::seats::authorize(actor, caller, &request, target.as_ref())?;
        if let Some(expected) = request.target() {
            let view = self
                .views
                .get(&expected.seat)
                .ok_or("target viewport unavailable")?;
            if view.generation != expected.generation
                || target
                    .as_ref()
                    .is_none_or(|r| r.harness_kind != view.adapter.record.kind)
            {
                return Err("target process binding changed".into());
            }
        }
        if let crate::mcp::seats::Request::Spawn(args) = &request {
            if !args.cwd.is_dir() {
                return Err("spawn cwd is not an available directory".into());
            }
            HarnessRecord::builtin(&args.approved_profile)
                .and_then(|harness| harness.resolve_role(args.role.as_deref()))
                .map_err(|error| error.to_string())?;
        }
        Ok(request)
    }
    fn read_seat_control(
        &mut self,
        request: &crate::mcp::seats::Request,
    ) -> std::result::Result<serde_json::Value, String> {
        use crate::mcp::seats::{self, Request};
        use serde_json::json;
        match request {
            Request::Screen {
                target,
                cursor,
                limit,
            } => {
                let snapshot = self
                    .snapshot(target.seat, target.generation)
                    .map_err(|e| e.to_string())?;
                let row = if let Some(cursor) = cursor {
                    let cursor: seats::ViewCursor = seats::decode_cursor(cursor)?;
                    if cursor.seat != target.seat
                        || cursor.generation != target.generation
                        || cursor.view != snapshot.view.view_generation
                        || cursor.sequence != snapshot.sequence
                        || cursor.row >= snapshot.screen.rows
                    {
                        return Err("stale viewport cursor".into());
                    }
                    cursor.row
                } else {
                    0
                };
                let end = (row + limit).min(snapshot.screen.rows);
                let mut rows = Vec::new();
                for cells in &snapshot.screen.cells[row..end] {
                    let mut text = String::new();
                    for cell in cells {
                        if cell.flags.intersects(
                            bbt_core::term::cell::Flags::WIDE_CHAR_SPACER
                                | bbt_core::term::cell::Flags::LEADING_WIDE_CHAR_SPACER,
                        ) {
                            continue;
                        }
                        text.push(cell.c);
                        if let Some(extra) = cell.zerowidth() {
                            text.extend(extra);
                        }
                    }
                    rows.push(text);
                }
                if serde_json::to_vec(&rows)
                    .map_err(|_| "viewport encoding unavailable")?
                    .len()
                    > 128 * 1024
                {
                    return Err("viewport page exceeds MCP byte bound".into());
                }
                let next = if end < snapshot.screen.rows {
                    Some(seats::encode_cursor(&seats::ViewCursor {
                        seat: target.seat,
                        generation: target.generation,
                        view: snapshot.view.view_generation,
                        sequence: snapshot.sequence,
                        row: end,
                    })?)
                } else {
                    None
                };
                Ok(
                    json!({"seat_id":target.seat,"process_generation":target.generation.to_string(),"view_generation":snapshot.view.view_generation,"sequence":snapshot.sequence.to_string(),"format":"text","row_start":row,"rows":rows,"cols":snapshot.screen.cols,"total_rows":snapshot.screen.rows,"cursor":snapshot.screen.cursor,"modes":snapshot.screen.modes.to_string(),"title":crate::mcp::tools::clip(&snapshot.screen.title,512),"gap":snapshot.screen.gap,"next":next}),
                )
            }
            Request::History {
                target,
                cursor,
                limit,
            } => {
                use base64::Engine;
                let sequence = self
                    .views
                    .get(&target.seat)
                    .ok_or("target viewport unavailable")?
                    .sequence;
                let (from, to) = if let Some(cursor) = cursor {
                    let cursor: seats::CaptureCursor = seats::decode_cursor(cursor)?;
                    if cursor.seat != target.seat
                        || cursor.generation != target.generation
                        || cursor.next == 0
                        || cursor.next > cursor.to
                        || cursor.to > sequence
                    {
                        return Err("stale or invalid capture cursor".into());
                    }
                    (cursor.next, cursor.to)
                } else {
                    (1, sequence)
                };
                if to == 0 {
                    return Ok(
                        json!({"seat_id":target.seat,"process_generation":target.generation.to_string(),"records":[],"gaps":[],"next":null,"capture_to":"0"}),
                    );
                }
                let history = self
                    .history(target.seat, target.generation, from, to)
                    .map_err(|e| e.to_string())?;
                let mut records = Vec::new();
                let mut bytes = 0;
                let mut next = history.next;
                for record in history.records {
                    let seq = record
                        .event
                        .sequence()
                        .ok_or("capture record has no sequence")?;
                    let value = match record.event {
                        Event::Output { bytes, .. } => {
                            json!({"sequence":seq.to_string(),"kind":"output","encoding":"base64","bytes":base64::engine::general_purpose::STANDARD.encode(&bytes),"byte_length":bytes.len(),"index":record.row})
                        }
                        Event::Resized { size, .. } => {
                            json!({"sequence":seq.to_string(),"kind":"resized","size":size,"index":record.row})
                        }
                        Event::Exited { status, .. } => {
                            json!({"sequence":seq.to_string(),"kind":"exited","status":status,"index":record.row})
                        }
                    };
                    let weight = serde_json::to_vec(&value)
                        .map_err(|_| "capture encoding unavailable")?
                        .len();
                    if records.len() == *limit || bytes + weight > 128 * 1024 {
                        if records.is_empty() {
                            return Err("capture record exceeds MCP page byte bound".into());
                        }
                        next = Some(seq);
                        break;
                    }
                    bytes += weight;
                    records.push(value);
                }
                let next = next
                    .map(|next| {
                        seats::encode_cursor(&seats::CaptureCursor {
                            seat: target.seat,
                            generation: target.generation,
                            next,
                            to,
                        })
                    })
                    .transpose()?;
                Ok(
                    json!({"seat_id":target.seat,"process_generation":target.generation.to_string(),"capture_from":from.to_string(),"capture_to":to.to_string(),"format":"capture_records","status":history.status,"records":records,"gaps":history.gap.into_iter().collect::<Vec<_>>(),"next":next}),
                )
            }
            _ => Err("seat control is not a read".into()),
        }
    }
    fn effect_seat_control(
        &mut self,
        actor: &crate::mcp::SeatActor,
        call: &crate::mcp::Call,
        request: crate::mcp::seats::Request,
    ) -> serde_json::Value {
        use crate::mcp::seats::{self, Request};
        use serde_json::json;
        let (current, caller) = match self.control_actor(actor.seat_id, actor.process_generation) {
            Ok(value) => value,
            Err(_) => {
                return crate::mcp::error(
                    call.op_id,
                    "rejected",
                    "caller binding unavailable before effect",
                );
            }
        };
        if current != *actor {
            return crate::mcp::error(
                call.op_id,
                "rejected",
                "caller authority changed before effect",
            );
        }
        if let Err(reason) = self.prepare_seat_control(actor, &caller, call) {
            return crate::mcp::error(call.op_id, "rejected", &reason);
        }
        let id = seats::effect_id(actor, call, request.target());
        match request {
            Request::Input {
                target,
                text,
                submit,
            } => match self.guarded_input(
                target.seat,
                target.generation,
                crate::adapters::guard::LiteralInput {
                    id,
                    text: &text,
                    submit,
                },
            ) {
                Ok(outcome) => seats::input_result(call, id, outcome),
                Err(error) => seats::failure(
                    call.op_id,
                    "not_submitted",
                    &format!("input guard refused: {error}"),
                    json!({"input_id":id}),
                ),
            },
            Request::Spawn(args) => {
                let result = (|| -> Result<SeatInfo> {
                    let harness = HarnessRecord::builtin(&args.approved_profile)?;
                    let mut record = SeatRecord::new(
                        id,
                        args.swarm_id,
                        harness.name.clone(),
                        harness.kind.clone(),
                    );
                    record.role = harness.resolve_role(args.role.as_deref())?;
                    let role_prompt = if record.role.name == "shell" {
                        String::new()
                    } else {
                        format!("ROLE: {}", record.role.name.to_ascii_uppercase())
                    };
                    self.launch(
                        record,
                        harness,
                        crate::adapters::launch::LaunchOptions {
                            cwd: args.cwd.canonicalize()?,
                            role_prompt,
                            executable: std::env::current_exe()?,
                            user_home: std::env::var_os("HOME")
                                .map(PathBuf::from)
                                .ok_or_else(|| invalid("missing user home"))?,
                            user_zdotdir: std::env::var_os("ZDOTDIR").map(PathBuf::from),
                            resume: false,
                        },
                    )
                })();
                match result {
                    Ok(info) => crate::mcp::success(
                        call.op_id,
                        json!({"status":"applied","seat_id":info.seat,"process_generation":info.generation.to_string(),"pid":info.pid}),
                    ),
                    Err(_) => seats::failure(
                        call.op_id,
                        "unknown",
                        "spawn outcome unavailable; inspect the reserved seat ID; no automatic relaunch",
                        json!({"seat_id":id,"process_generation":"1"}),
                    ),
                }
            }
            Request::Mute {
                target,
                epoch,
                muted,
            } => {
                let state = self.views[&target.seat].adapter.status.state;
                let result = self.messaging.set_mute_checked(
                    crate::messaging::tree::Actor::Seat {
                        seat: actor.seat_id,
                        class: actor.role.class,
                    },
                    target.seat,
                    muted,
                    state,
                    now_ms(),
                    (target.generation, epoch),
                );
                match result {
                    Ok(notice) => match self.messaging.authority(target.seat, target.generation) {
                        Ok((record, epoch, _)) => crate::mcp::success(
                            call.op_id,
                            json!({"status":"applied","seat_id":target.seat,"process_generation":target.generation.to_string(),"muted":record.muted,"policy_epoch":epoch.to_string(),"notice_id":notice}),
                        ),
                        Err(_) => seats::failure(
                            call.op_id,
                            "unknown",
                            "mute outcome committed; current policy unavailable",
                            json!({"seat_id":target.seat}),
                        ),
                    },
                    Err(error)
                        if matches!(
                            error.to_string().as_str(),
                            "stale target process generation" | "stale target policy epoch"
                        ) =>
                    {
                        crate::mcp::error(call.op_id, "stale_policy", &error.to_string())
                    }
                    Err(_) => seats::failure(
                        call.op_id,
                        "unknown",
                        "mute outcome unavailable; inspect current policy and presence receipts",
                        json!({"seat_id":target.seat}),
                    ),
                }
            }
            _ => crate::mcp::error(
                call.op_id,
                "rejected",
                "control requires its native asynchronous path",
            ),
        }
    }
    fn start_control_interrupt(
        &mut self,
        id: Ulid,
        generation: u64,
        call: crate::mcp::Call,
        reply: oneshot::Sender<Result<serde_json::Value>>,
    ) {
        use crate::mcp::seats::{self, Request};
        use serde_json::json;
        let mut reply = Some(reply);
        let started = (|| -> Result<Option<serde_json::Value>> {
            let (actor, caller) = self.control_actor(id, generation)?;
            call.validate()?;
            let journal = self.control_journal(id)?;
            let request = match self.prepare_seat_control(&actor, &caller, &call) {
                Ok(value) => value,
                Err(reason) => {
                    return journal
                        .checked(
                            &actor,
                            &call,
                            crate::mcp::error(call.op_id, "rejected", &reason),
                        )
                        .map(Some);
                }
            };
            let Request::Interrupt { target, text } = request else {
                return Err(invalid("invalid interrupt dispatch"));
            };
            let input_id = seats::effect_id(&actor, &call, Some(target));
            let (retained, fresh) = journal.admit(&actor, &call)?;
            if !fresh {
                return Ok(Some(retained));
            }
            let view = &self.views[&target.seat];
            let harness = view.adapter.record.kind.clone();
            let endpoint = view.delivery_endpoint.clone();
            let binding = view.adapter.binding.clone();
            let available = match (&endpoint, harness.as_str()) {
                (crate::adapters::Endpoint::Claude { .. }, "claude") => true,
                (
                    crate::adapters::Endpoint::Codex {
                        thread,
                        turn: Some(turn),
                        ..
                    },
                    "codex",
                ) => thread == &binding.session && !turn.is_empty(),
                _ => false,
            };
            if !available {
                return journal
                    .complete(
                        &actor,
                        &call,
                        seats::failure(
                            call.op_id,
                            "unavailable",
                            &format!("interrupt unavailable for {harness}"),
                            json!({"input_id":input_id}),
                        ),
                    )
                    .map(Some);
            }
            if self.control_deliveries.len() >= 8 {
                return journal
                    .complete(
                        &actor,
                        &call,
                        seats::failure(
                            call.op_id,
                            "busy",
                            "native interrupt capacity is full",
                            json!({"input_id":input_id}),
                        ),
                    )
                    .map(Some);
            }
            let run_dir = self.config.run_dir.clone();
            let handle = self.handle.clone();
            let call = call.clone();
            let sender = reply.take().unwrap();
            self.control_deliveries.spawn_on(
                async move {
                    let checked_actor = actor.clone();
                    let checked_harness = harness.clone();
                    let native = tokio::task::spawn_blocking(move || {
                        seats::revalidate_native(
                            &run_dir,
                            &checked_actor,
                            target,
                            &checked_harness,
                        )?;
                        Ok::<_, String>(handle.block_on(crate::adapters::native::wake(
                            &binding,
                            &endpoint,
                            input_id,
                            &text,
                            crate::adapters::Priority::Now,
                            true,
                        )))
                    })
                    .await;
                    let (outcome, result) = match native {
                        Ok(Ok(outcome)) => {
                            let value =
                                if outcome.evidence == crate::adapters::Evidence::RecipientStored {
                                    seats::failure(
                                        call.op_id,
                                        "unavailable",
                                        &format!(
                                            "interrupt unavailable for {harness}: {}",
                                            outcome.note
                                        ),
                                        json!({"input_id":input_id}),
                                    )
                                } else {
                                    seats::input_result(&call, input_id, outcome.clone())
                                };
                            (Some(outcome), value)
                        }
                        Ok(Err(reason)) => (
                            None,
                            seats::failure(
                                call.op_id,
                                "rejected",
                                &reason,
                                json!({"input_id":input_id}),
                            ),
                        ),
                        Err(_) => (
                            None,
                            seats::failure(
                                call.op_id,
                                "unknown",
                                "native task lost; retain the input ID and never redispatch",
                                json!({"input_id":input_id}),
                            ),
                        ),
                    };
                    ControlDelivery {
                        actor,
                        call,
                        journal,
                        reply: sender,
                        target,
                        input_id,
                        outcome,
                        result,
                    }
                },
                &self.handle,
            );
            Ok(None)
        })();
        if let Some(reply) = reply {
            match started {
                Ok(Some(value)) => {
                    let _ = reply.send(Ok(value));
                }
                Err(error) => {
                    let _ = reply.send(Err(error));
                }
                Ok(None) => {}
            }
        }
    }
    fn control_message(
        &mut self,
        actor: &crate::mcp::SeatActor,
        record: &SeatRecord,
        call: &crate::mcp::Call,
        request: crate::messaging::api::Request,
    ) -> serde_json::Value {
        // A peer is never the trusted in-process operator composer.
        match self
            .messaging
            .request(record.id, record.generation, request, false, None, now_ms())
        {
            Ok(response) => {
                let mut value = crate::mcp::messaging::result_for_seat(actor, response);
                if call.tool == "message_list" {
                    let summary = match self.messaging.inbox_summary(record.id, record.generation) {
                        Ok(summary) => summary,
                        Err(_) => {
                            return crate::mcp::error(
                                call.op_id,
                                "unavailable",
                                "inbox counts are unavailable; no count was substituted",
                            );
                        }
                    };
                    value = crate::mcp::messaging::with_counts(value, summary);
                }
                crate::mcp::success(call.op_id, value)
            }
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::InvalidData) =>
            {
                crate::mcp::error(call.op_id, "rejected", &error.to_string())
            }
            // The backend may already have committed before a storage failure.
            // Preserve uncertainty rather than imply a safe automatic retry.
            Err(_) => crate::mcp::error(
                call.op_id,
                "unknown",
                "messaging outcome unavailable; retain the operation ID and check receipts",
            ),
        }
    }
    fn control_seat(&self, record: &SeatRecord) -> serde_json::Value {
        serde_json::json!({"record":record,"adapter":self.views.get(&record.id).filter(|v|v.generation==record.generation).map(|v|&v.adapter.status)})
    }
    fn control_read(
        &mut self,
        actor: &crate::mcp::SeatActor,
        record: &SeatRecord,
        call: &crate::mcp::Call,
    ) -> std::result::Result<serde_json::Value, String> {
        use crate::mcp::tools;
        use crate::registry::Class;
        use serde_json::json;
        let source = BboxySource::new(&self.index).with_registry(&self.registry);
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
                let tasks = tools::orient_documents(&source, record)?;
                let empty = tasks["items"].as_array().is_some_and(Vec::is_empty);
                let tasks = tools::scoped_list(actor, tasks, empty.then_some("nothing_assigned"));
                let destinations = crate::mcp::messaging::destination_list(
                    self.messaging
                        .destinations(record.id, record.generation)
                        .map_err(|e| e.to_string())?,
                );
                let summary = self
                    .messaging
                    .inbox_summary(record.id, record.generation)
                    .map_err(|_| "inbox counts are unavailable; no count was substituted")?;
                let catalogue = tools::catalogue(actor);
                let tools = catalogue["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|tool| json!({"name":tool["name"],"description":tool["description"]}))
                    .collect::<Vec<_>>();
                let value = json!({
                    "actor":actor,"seat":self.control_seat(record),"tasks":tasks,"tools":tools,
                    "destinations":destinations,
                    "operator":"to reach the operator, reply with reply_to, or send to operator/inbox",
                    "op_id_guidance":"ids are minted for you unless you retry",
                    "execution":tools::graph(&self.index,&source,actor,record,&tools::GraphRead{graph:"execution".into(),after:None,limit:Some(32)},true)?,
                    "channels":crate::mcp::messaging::result_for_seat(actor, channels)
                });
                Ok(tools::scoped_list(
                    actor,
                    crate::mcp::messaging::with_counts(value, summary),
                    None,
                ))
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
                let empty = records.is_empty();
                Ok(tools::scoped_list(
                    actor,
                    json!({"items":records.iter().map(|r|self.control_seat(r)).collect::<Vec<_>>(),"next":if more{records.last().map(|r|r.id)}else{None}}),
                    empty.then_some("nothing_yet"),
                ))
            }
            "graph_read" => {
                let args: tools::GraphRead = tools::parse(&call.args)?;
                tools::graph(&self.index, &source, actor, record, &args, false)
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
        if let (Ok(record), Some(bytes)) = (
            self.registry.get(info.seat),
            SqliteStore::open(&directory)?.get("provenance_cwd")?,
        ) && let Ok(cwd) = serde_json::from_slice::<PathBuf>(&bytes)
        {
            self.provenance.register(
                info.seat,
                info.generation,
                &cwd,
                record.location == crate::registry::Location::Local,
            );
        }
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
                    name: "shell".into(),
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
        let (delivery_endpoint, delivery_failure) = if kind == "claude" {
            match SqliteStore::open(&directory)?.get("claude_socket_registration")? {
                Some(bytes) => match serde_json::from_slice::<
                    crate::adapters::native::ClaudeRegistration,
                >(&bytes)
                .map_err(|error| error.to_string())
                .and_then(|registration| {
                    registration
                        .validate(&adapter.binding)
                        .map_err(|error| error.to_string())?;
                    Ok(registration.path)
                }) {
                    Ok(path) => (crate::adapters::Endpoint::Claude { path }, None),
                    Err(reason) => (
                        crate::adapters::Endpoint::None,
                        Some(crate::adapters::observe::short(
                            &format!("stored Claude socket registration: {reason}"),
                            512,
                        )),
                    ),
                },
                None => (
                    crate::adapters::Endpoint::None,
                    Some("Claude messaging socket has not been registered by a hook".into()),
                ),
            }
        } else {
            (crate::adapters::Endpoint::None, None)
        };
        self.views.insert(
            info.seat,
            ViewState {
                info: info.clone(),
                adapter,
                delivery_endpoint,
                delivery_failure,
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
    fn register_claude_socket(
        &mut self,
        id: Ulid,
        generation: u64,
        event: &crate::adapters::hook::ShimEvent,
    ) -> Result<()> {
        let view = self
            .views
            .get(&id)
            .filter(|view| view.generation == generation)
            .ok_or_else(|| invalid("stale Claude socket registration generation"))?;
        let Some(registration) =
            crate::adapters::native::claude_registration(&view.adapter, event)?
        else {
            return Ok(());
        };
        if event.event != "SessionStart"
            && matches!(&view.delivery_endpoint, crate::adapters::Endpoint::Claude {path} if path == &registration.path)
        {
            return Ok(());
        }
        let directory = object_dir(&self.config.run_dir, &format!("seat-{id}"))?;
        SqliteStore::open(&directory)?.put(
            "claude_socket_registration",
            &serde_json::to_vec(&registration)?,
        )?;
        let view = self.views.get_mut(&id).unwrap();
        view.delivery_endpoint = crate::adapters::Endpoint::Claude {
            path: registration.path,
        };
        view.delivery_failure = None;
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
        harness.validate()?;
        crate::adapters::validate_role(&harness.kind, &record.role)?;
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
        crate::adapters::validate_role(&record.harness_kind, &record.role)?;
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
        SqliteStore::open(&object_dir(
            &self.config.run_dir,
            &format!("seat-{}", record.id),
        )?)?
        .put("provenance_cwd", &serde_json::to_vec(&spec.cwd)?)?;
        self.provenance.register(
            record.id,
            record.generation,
            &spec.cwd,
            record.location == crate::registry::Location::Local,
        );
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
        self.client
            .capture_diagnostics
            .seat(id, generation, |row, now| {
                row.ring_observed_ns = now;
                row.captured_sequence = history.status.captured_sequence;
                row.committed_sequence = history.status.committed_sequence;
            });
        Ok(*history)
    }
    fn save(&mut self, id: Ulid) -> Result<()> {
        let span = self.client.capture_diagnostics.start(
            Phase::Checkpoint,
            id,
            self.views[&id].generation,
            0,
            self.views[&id].sequence,
        );
        let result = self.save_inner(id);
        span.finish(capture_outcome(&result), 0, 0);
        result
    }
    fn save_inner(&mut self, id: Ulid) -> Result<()> {
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
        let generation = match &event {
            Event::Output { generation, .. }
            | Event::Resized { generation, .. }
            | Event::Exited { generation, .. } => *generation,
        };
        let span = self.client.capture_diagnostics.start(
            Phase::Event,
            event.seat(),
            generation,
            0,
            event.sequence().unwrap(),
        );
        let result = self.event_inner(event);
        span.finish(capture_outcome(&result), 0, 0);
        result
    }
    fn event_inner(&mut self, event: Event) -> Result<bool> {
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
        let span = self
            .client
            .capture_diagnostics
            .start(Phase::Parse, id, generation, seq, seq);
        view.state.apply(&event)?;
        let bytes = match &event {
            Event::Output { bytes, .. } => bytes.len() as u64,
            _ => 0,
        };
        span.finish(Outcome::Ok, 1, bytes);
        self.client
            .capture_diagnostics
            .seat(id, generation, |row, now| {
                row.progress(seq, now);
                row.parsed_events += 1;
                row.parsed_bytes += bytes;
            });
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
                    self.provenance.osc(id, generation, &params);
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
        let generation = current.generation;
        let page = self.client.capture_diagnostics.start(
            Phase::ReplayPage,
            id,
            generation,
            current.sequence + 1,
            through,
        );
        let history = self.history(id, current.generation, current.sequence + 1, through)?;
        if history.gap.is_some() || history.records.is_empty() {
            return Err(invalid("capture recovery gap"));
        }
        let events = history.records.len() as u64;
        let mut bytes = 0;
        let parse = self.client.capture_diagnostics.start(
            Phase::Parse,
            id,
            generation,
            self.views[&id].sequence + 1,
            through,
        );
        let view = self.views.get_mut(&id).unwrap();
        for record in history.records {
            if matches!(record.event, Event::Resized { .. }) {
                view.view_generation = Ulid::new();
            }
            view.state.apply(&record.event)?;
            if let Event::Output { bytes: output, .. } = &record.event {
                bytes += output.len() as u64;
            }
            // Recovery must not replay probe responses or hook/native effects.
            let _ = view.tap.drain();
        }
        parse.finish(Outcome::Ok, events, bytes);
        self.client
            .capture_diagnostics
            .seat(id, generation, |row, now| {
                row.progress(view.sequence, now);
                row.target_sequence = through;
                row.pages += 1;
                row.parsed_events += events;
                row.parsed_bytes += bytes;
            });
        view.changed_at = Instant::now();
        if view.sequence >= through {
            self.save(id)?;
            page.finish(Outcome::Ok, events, bytes);
            Ok(true)
        } else {
            // The caller keeps the final live event. Its sequence and this
            // applied cursor survive the command-service turn between pages.
            page.finish(Outcome::Ok, events, bytes);
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
                                    view.delivery_failure = None;
                                }
                                let _ = view.adapter.observe_claude_session(
                                    &file,
                                    view.state.sequence,
                                    now,
                                );
                            }
                            Err(error) => {
                                view.adapter.invalidate();
                                if matches!(view.delivery_endpoint, crate::adapters::Endpoint::None)
                                {
                                    view.delivery_failure = Some(crate::adapters::observe::short(
                                        &format!("Claude session-file discovery failed: {error}"),
                                        512,
                                    ));
                                }
                            }
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
        while let Some(result) = self.control_deliveries.try_join_next() {
            let delivery = match result {
                Ok(delivery) => delivery,
                Err(_) => {
                    eprintln!("native control task lost; admitted outcome remains unknown");
                    continue;
                }
            };
            if let Some(outcome) = &delivery.outcome
                && let Some(view) = self
                    .views
                    .get_mut(&delivery.target.seat)
                    .filter(|view| view.generation == delivery.target.generation)
            {
                view.adapter.record_outcome(delivery.input_id, outcome);
            }
            let result =
                delivery
                    .journal
                    .complete(&delivery.actor, &delivery.call, delivery.result);
            let _ = delivery.reply.send(result);
        }
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
            let published = self.handle.block_on(self.bus.publish(&stream, draft))?;
            self.messaging.event_published(&stream, id, published.seq)?;
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
                    let delivery_failure = view.delivery_failure.clone();
                    let priority = if work.messages.iter().all(|m| m.envelope.priority > 0) {
                        Priority::Later
                    } else {
                        Priority::Next
                    };
                    self.deliveries.spawn_on(
                        async move {
                            let mut outcome = crate::adapters::native::wake(
                                &binding, &endpoint, work.id, &text, priority, false,
                            )
                            .await;
                            if matches!(endpoint, crate::adapters::Endpoint::None)
                                && outcome.evidence == Evidence::RecipientStored
                                && let Some(reason) = delivery_failure
                            {
                                outcome.note = crate::adapters::observe::short(
                                    &format!("native attachment unavailable: {reason}; no mutation attempted"),
                                    640,
                                );
                            }
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
        let generation = self
            .views
            .get(&seat)
            .ok_or_else(|| invalid("recipient viewport unavailable"))?
            .generation;
        self.guarded_input(
            seat,
            generation,
            crate::adapters::guard::LiteralInput {
                id: work.id,
                text,
                submit: true,
            },
        )
    }
    fn guarded_input(
        &mut self,
        seat: Ulid,
        generation: u64,
        input: crate::adapters::guard::LiteralInput<'_>,
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
            .filter(|attachment| attachment.generation == generation)
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
        let view = self
            .views
            .get_mut(&seat)
            .filter(|view| view.generation == generation)
            .ok_or_else(|| invalid("stale target viewport generation"))?;
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
            return Ok(pending(if view.adapter.proof.is_none() {
                "no positive composer-ready indication"
            } else {
                "pending for operator: fresh readiness after the last input and a caught-up PTY are required"
            }));
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
            .submit_literal(&arbiter, now, input, &journal, |bytes| {
                match connections.request(
                    &host,
                    Action::GuardedInput {
                        attachment,
                        input_id: input.id,
                        bytes,
                        expected: context,
                    },
                )? {
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
    fn gui_device(&self) -> Result<Ulid> {
        let status = self.identity.status()?;
        status.device_id.ok_or_else(|| {
            invalid(format!(
                "GuiEventsHeld: {}",
                status
                    .reason_label
                    .as_deref()
                    .unwrap_or("device custody unavailable")
            ))
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
    fn gui_messaging(
        &mut self,
        id: Ulid,
        swarm: Ulid,
        request: crate::messaging::api::Request,
        door: crate::mcp::Door,
    ) -> Result<crate::messaging::api::Response> {
        let device_id = self
            .identity
            .status()?
            .device_id
            .ok_or_else(|| invalid("held: operator device custody is unavailable"))?;
        let actor = crate::mcp::Actor::Operator { device_id, door };
        let call = crate::mcp::Call {
            op_id: id,
            tool: "gui_messaging".into(),
            args: serde_json::json!({"swarm_id":swarm,"request":request}),
        };
        call.validate()?;
        let journal = self.operator_journal.clone();
        let request = match crate::mcp::messaging::gui_request(&actor, id, request) {
            Ok(request) => request,
            Err(reason) => {
                let result =
                    journal.checked(&actor, &call, crate::mcp::error(id, "rejected", &reason))?;
                return operator_response(result);
            }
        };
        let authority = self.identity.operator_signing()?;
        if authority.device_id != device_id {
            return Err(invalid("held: operator device changed"));
        }
        let mute_state = match &request {
            crate::messaging::api::Request::Mute { seat, .. } => {
                self.views.get(seat).map(|view| view.adapter.status.state)
            }
            _ => None,
        };
        let mutation = !matches!(
            request,
            crate::messaging::api::Request::List { .. }
                | crate::messaging::api::Request::Receipts { .. }
                | crate::messaging::api::Request::Ls { .. }
        );
        let dispatch = || match self.messaging.request_operator(
            &authority,
            swarm,
            request,
            mute_state,
            now_ms(),
        ) {
            Ok(response) => match serde_json::to_value(response) {
                Ok(value) => crate::mcp::success(id, value),
                Err(error) => crate::mcp::error(id, "unknown", &error.to_string()),
            },
            Err(error) => {
                let reason = error.to_string();
                let code = if reason.starts_with("unknown:") {
                    "unknown"
                } else if reason.starts_with("held:") {
                    "held"
                } else {
                    "rejected"
                };
                crate::mcp::error(id, code, &reason)
            }
        };
        let result = if mutation {
            journal.effect(&actor, &call, dispatch)?
        } else {
            journal.call(&actor, &call, dispatch)?
        };
        operator_response(result)
    }
    fn gui_task(
        &mut self,
        id: Ulid,
        request: bbt_proto::messaging::TaskRequest,
        door: crate::mcp::Door,
    ) -> Result<bbt_proto::messaging::TaskResponse> {
        let device_id = self
            .identity
            .status()?
            .device_id
            .ok_or_else(|| invalid("held: operator device custody is unavailable"))?;
        let actor = crate::mcp::Actor::Operator { device_id, door };
        let call = crate::mcp::work_items::call(id, request).map_err(invalid)?;
        call.validate()?;
        if crate::mcp::work_items::mutation(&call.tool) {
            self.identity.operator_signing()?;
        }
        let journal = self.operator_journal.clone();
        let result = self.work_item_call(&actor, &call, &journal)?;
        crate::mcp::work_items::response(&call.tool, result).map_err(invalid)
    }
    fn gui_spawn(
        &mut self,
        id: Ulid,
        spec: bbt_proto::SpawnSpec,
        door: crate::mcp::Door,
    ) -> Result<bbt_proto::SeatId> {
        let call = crate::mcp::Call {
            op_id: id,
            tool: "gui_spawn".into(),
            args: serde_json::to_value(&spec)?,
        };
        call.validate()?;
        let device_id = self
            .identity
            .status()?
            .device_id
            .ok_or_else(|| invalid("held: operator device custody is unavailable"))?;
        let principal = crate::mcp::Principal::Operator { device_id, door };
        let (retained, fresh) = self.operator_journal.admit(&principal, &call)?;
        let result = if fresh {
            let result = match self.prepare_gui_spawn(spec) {
                Ok((record, harness, options)) => match self.launch(record, harness, options) {
                    Ok(info) => crate::mcp::success(id, serde_json::json!({"seat":info.seat})),
                    // Admission is durable before the native effect. A lost
                    // supervisor reply cannot authorize a second launch.
                    Err(error) => crate::mcp::error(id, "unknown", &error.to_string()),
                },
                Err(error) => crate::mcp::error(id, "rejected", &error.to_string()),
            };
            self.operator_journal.complete(&principal, &call, result)?
        } else {
            retained
        };
        if result["isError"] == true {
            let error = &result["structuredContent"]["error"];
            let code = error["code"].as_str().unwrap_or("unknown");
            let message = error["message"]
                .as_str()
                .unwrap_or("spawn outcome unavailable");
            // Existing named validation outcomes (notably LinkDown) retain
            // their wire spelling. Unknown/conflict also carry journal status.
            return Err(invalid(if code == "rejected" {
                message.to_owned()
            } else {
                format!("{code}: {message}")
            }));
        }
        let seat: Ulid = serde_json::from_value(result["structuredContent"]["seat"].clone())?;
        Ok(bbt_proto::SeatId::from_u128(seat.0))
    }

    fn prepare_gui_spawn(
        &self,
        spec: bbt_proto::SpawnSpec,
    ) -> Result<(
        SeatRecord,
        HarnessRecord,
        crate::adapters::launch::LaunchOptions,
    )> {
        use bbt_proto::seat_state::Placement;
        if spec.swarm.0.0 == 0 {
            return Err(invalid("spawn_swarm_missing"));
        }
        if let Some(placement) = &spec.placement {
            let tab = match placement {
                Placement::NewTab { tab } | Placement::Split { tab, .. } => tab,
            };
            if tab.0.0 == 0 {
                return Err(invalid("placement_tab_missing"));
            }
            if let Placement::Split {
                target,
                target_generation,
                ..
            } = placement
            {
                let anchor = self
                    .registry
                    .get(Ulid(target.0.0))
                    .map_err(|_| invalid("placement_anchor_missing"))?;
                if anchor.lifecycle != Lifecycle::Open {
                    return Err(invalid("placement_anchor_missing"));
                }
                if anchor.generation != *target_generation {
                    return Err(invalid("placement_anchor_changed"));
                }
                if anchor.swarm.0 != spec.swarm.0.0 {
                    return Err(invalid("placement_swarm_changed"));
                }
            }
        }
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
        record.role = harness.resolve_role(spec.role.as_deref())?;
        let role_prompt = if record.role.name == "shell" {
            String::new()
        } else {
            format!("ROLE: {}", record.role.name.to_ascii_uppercase())
        };
        let resume = spec.resume.is_some();
        record.harness_session_id = spec.resume;
        Ok((
            record,
            harness,
            crate::adapters::launch::LaunchOptions {
                cwd: spec.cwd.into(),
                role_prompt,
                executable: std::env::current_exe()?,
                user_home: std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .ok_or_else(|| invalid("missing user home"))?,
                user_zdotdir: std::env::var_os("ZDOTDIR").map(PathBuf::from),
                resume,
            },
        ))
    }
    fn refresh_providers(
        &mut self,
        recoveries: &mut BTreeSet<HostId>,
        pending: &mut PendingSeats,
    ) -> Result<()> {
        use crate::provider::fake::{STATE_SUBJECT, is_link_down};
        use crate::provider::{LinkState, ProviderEvent};
        for event in self.connections.poll() {
            let ProviderEvent::HostState { host, state } = &event;
            if matches!(
                state,
                LinkState::Connected { .. } | LinkState::Degraded { .. }
            ) {
                pending.resume(host);
                recoveries.insert(host.clone());
            } else {
                pending.park(host);
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
        let (phase, generation, from, to) = match &action {
            Action::ReadRing {
                attachment,
                from,
                to,
            } => (Phase::ReadRingRpc, attachment.generation, *from, *to),
            Action::RingRecovery { attachment } => {
                (Phase::RecoveryRpc, attachment.generation, 0, 0)
            }
            Action::PublishCheckpoint { attachment, .. } => {
                (Phase::CheckpointRpc, attachment.generation, 0, 0)
            }
            // Diagnostics must not add a release-mode admission policy.
            _ => return retry_capture(|| self.request_for_seat(seat, action.clone())),
        };
        // The parent includes bounded Busy backoff; attempt spans contain only
        // the actual RPC wait. Subtract nested intervals, never add them.
        let request = self.client.capture_diagnostics.start(
            Phase::CaptureRequest,
            seat,
            generation,
            from,
            to,
        );
        let result = retry_capture(|| {
            let span = self
                .client
                .capture_diagnostics
                .start(phase, seat, generation, from, to);
            let result = self.request_for_seat(seat, action.clone());
            span.finish(capture_outcome(&result), 0, 0);
            result
        });
        request.finish(capture_outcome(&result), 0, 0);
        result
    }
    fn run(&mut self, commands: &mut mpsc::Receiver<Command>, stop: Arc<AtomicBool>) -> Result<()> {
        let mut scan_at = Instant::now();
        let mut adapters_at = Instant::now();
        let mut messages_at = Instant::now();
        let mut identity_at = Instant::now();
        let mut providers_at = Instant::now();
        let mut pending_events = PendingSeats::default();
        let mut provider_recoveries = BTreeSet::new();
        let mut made_progress = false;
        while !stop.load(Ordering::Acquire) {
            let mut turn =
                self.client
                    .capture_diagnostics
                    .start(Phase::LogicTurn, Ulid::nil(), 0, 0, 0);
            let poll_ready = made_progress && pending_events.has_ready();
            made_progress = false;
            let command_span =
                self.client
                    .capture_diagnostics
                    .start(Phase::Commands, Ulid::nil(), 0, 0, 0);
            let mut command_count = 0;
            for _ in 0..32 {
                let Ok(command) = commands.try_recv() else {
                    break;
                };
                command_count += 1;
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
                        if call.tool == "seat_interrupt" {
                            self.start_control_interrupt(id, generation, call, reply);
                        } else {
                            let result = self.control_call(id, generation, call);
                            let _ = reply.send(result);
                        }
                    }
                    Command::RecoverSeats(reply) => {
                        let result = self.recover_seats();
                        let _ = reply.send(result);
                    }
                    Command::GuiSpawn(id, spec, door, reply) => {
                        let result = self.gui_spawn(id, spec, door);
                        let _ = reply.send(result);
                    }
                    Command::GuiMessaging(id, swarm, request, door, reply) => {
                        let result = self.gui_messaging(id, swarm, request, door);
                        let _ = reply.send(result);
                    }
                    Command::GuiTask(id, request, door, reply) => {
                        let result = self.gui_task(id, request, door);
                        let _ = reply.send(result);
                    }
                    Command::GuiKnowledge(method, args, reply) => {
                        let source = crate::knowledge::BboxySource::new(&self.index)
                            .with_registry(&self.registry);
                        let _ = reply.send(crate::knowledge::read(&source, &method, args));
                    }
                    Command::GuiMessageRow(swarm, message, reply) => {
                        let result = self.identity.status().and_then(|status| {
                            let device = status.device_id.ok_or_else(|| {
                                invalid("held: operator device custody is unavailable")
                            })?;
                            self.messaging.set_operator_device(device)?;
                            self.messaging.operator_message_row(swarm, message)
                        });
                        let _ = reply.send(result);
                    }
                    Command::GuiMessagingSubjects(reply) => {
                        let result = self.identity.status().and_then(|status| {
                            let device = status.device_id.ok_or_else(|| {
                                invalid("held: operator device custody is unavailable")
                            })?;
                            self.messaging.set_operator_device(device)?;
                            self.messaging.operator_subjects()
                        });
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
                    Command::GuiDevice(reply) => {
                        let _ = reply.send(self.gui_device());
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
                        let result = result.and_then(|status| {
                            self.register_claude_socket(id, generation, &event)?;
                            Ok(status)
                        });
                        if let (Some(before), Ok(status)) = (&before, &result)
                            && !status.same_value(before)
                        {
                            self.publish_adapter(id, status.clone())?;
                        }
                        if result.is_ok() {
                            self.provenance.hook(id, generation, &event);
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
            command_span.finish(Outcome::Ok, command_count, 0);
            let receive =
                self.client
                    .capture_diagnostics
                    .start(Phase::Receive, Ulid::nil(), 0, 0, 0);
            for host in self.connections.host_ids() {
                let event = if poll_ready {
                    self.connections.receive_ready_from(&host)?
                } else {
                    self.connections.receive_from(&host)?
                };
                if let Some(event) = event {
                    if self.host_for_seat(event.seat())? != host {
                        continue;
                    }
                    let generation = match &event {
                        Event::Output { generation, .. }
                        | Event::Resized { generation, .. }
                        | Event::Exited { generation, .. } => *generation,
                    };
                    if let Some(view) = self.views.get(&event.seat())
                        && generation == view.generation
                        && event.sequence().unwrap() > view.sequence
                    {
                        self.client.capture_diagnostics.seat(
                            event.seat(),
                            generation,
                            |row, now| {
                                row.notified_sequence =
                                    row.notified_sequence.max(event.sequence().unwrap());
                                row.progress(view.sequence, now);
                            },
                        );
                        pending_events.push(host, event, generation, view.sequence + 1)?;
                    }
                }
            }
            receive.finish(Outcome::Ok, 0, 0);
            if let Some((key, event)) = pending_events.next() {
                let before = self.views.get(&key.1).map(|view| view.sequence);
                if let Some(view) = self.views.get(&key.1) {
                    self.client
                        .capture_diagnostics
                        .seat(key.1, view.generation, |row, _| {
                            row.target_sequence = event.sequence().unwrap();
                        });
                }
                let complete = match self.event(event) {
                    Ok(complete) => {
                        made_progress =
                            complete || self.views.get(&key.1).map(|view| view.sequence) != before;
                        complete
                    }
                    Err(error) if crate::provider::fake::is_link_down(&error) => {
                        pending_events.park(&key.0);
                        false
                    }
                    Err(error) if is_capture_busy(&error) => false,
                    Err(error) => return Err(error),
                };
                // One page per seat per round, with command service and fresh
                // notifications between pages. A long gap cannot hold a host.
                pending_events.finish(key, complete);
            }
            let maintenance =
                self.client
                    .capture_diagnostics
                    .start(Phase::Maintenance, Ulid::nil(), 0, 0, 0);
            if providers_at.elapsed() >= Duration::from_millis(200) {
                self.refresh_providers(&mut provider_recoveries, &mut pending_events)?;
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
                let active: Vec<_> = self
                    .views
                    .iter()
                    .filter(|(_, view)| {
                        view.info.exited.is_none()
                            && view.adapter.status.state == crate::adapters::State::Active
                    })
                    .map(|(seat, view)| (*seat, view.generation, view.adapter.status.observed_ms))
                    .collect();
                let changed = match self.provenance.step(
                    &mut self.index,
                    &self.bus,
                    &self.handle,
                    &self.identity,
                    &active,
                ) {
                    Ok(changed) => changed,
                    Err(error) => {
                        eprintln!("ProvenancePending: {error}");
                        false
                    }
                };
                if self.watch.changed() || changed {
                    self.scan()?;
                }
                scan_at = Instant::now();
            }
            maintenance.finish(Outcome::Ok, 0, 0);
            turn.pending(
                pending_events.seats.len(),
                pending_events.ready.len(),
                pending_events.parked.len(),
            );
            turn.finish(Outcome::Ok, 0, 0);
        }
        for id in self.views.keys().copied().collect::<Vec<_>>() {
            self.save(id)?;
        }
        Ok(())
    }
}
fn capture_outcome<T>(result: &Result<T>) -> Outcome {
    match result {
        Ok(_) => Outcome::Ok,
        Err(error) if is_capture_busy(error) => Outcome::Busy,
        Err(error) if crate::provider::fake::is_link_down(error) => Outcome::Unavailable,
        Err(_) => Outcome::Error,
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

#[cfg(test)]
mod pending_tests {
    use super::*;
    fn output(seat: Ulid, generation: u64, seq: u64) -> Event {
        Event::Output {
            seat,
            generation,
            seq,
            bytes: vec![b'x'],
        }
    }
    #[test]
    fn pending_seats_rotate_without_changing_an_unfinished_target() {
        let mut pending = PendingSeats::default();
        let seats = [Ulid::from(1), Ulid::from(2), Ulid::from(3), Ulid::from(4)];
        for seat in seats {
            pending
                .push(HostId::Local, output(seat, 1, 1000), 1, 1)
                .unwrap();
        }
        let mut quantum = 0;
        let mut last = BTreeMap::new();
        for _ in 0..3 {
            for expected in seats {
                let (key, event) = pending.next().unwrap();
                quantum += 1;
                let previous = last.insert(event.seat(), quantum).unwrap_or(0);
                assert!(
                    quantum - previous <= seats.len(),
                    "a seat missed its N-quanta bound"
                );
                assert_eq!(event.seat(), expected);
                assert_eq!(event.sequence(), Some(1000));
                pending.finish(key, false);
            }
        }
        assert_eq!(pending.seats.len(), 4);
        assert_eq!(pending.ready.len(), 4);
    }
    #[test]
    fn pending_notifications_stay_bounded_and_preserve_resize_and_exit_order() {
        let mut pending = PendingSeats::default();
        let seat = Ulid::from(1);
        for seq in 1..10_001 {
            pending
                .push(HostId::Local, output(seat, 1, seq), 1, 1)
                .unwrap();
        }
        pending
            .push(
                HostId::Local,
                Event::Resized {
                    seat,
                    generation: 1,
                    seq: 10_001,
                    size: Default::default(),
                },
                1,
                1,
            )
            .unwrap();
        for seq in 10_002..20_001 {
            pending
                .push(HostId::Local, output(seat, 1, seq), 1, 1)
                .unwrap();
        }
        pending
            .push(
                HostId::Local,
                Event::Exited {
                    seat,
                    generation: 1,
                    seq: 20_001,
                    status: crate::protocol::ExitInfo {
                        code: 0,
                        signal: None,
                    },
                },
                1,
                1,
            )
            .unwrap();
        assert_eq!(pending.ready.len(), 1);
        let mut sequences = Vec::new();
        while let Some((key, event)) = pending.next() {
            sequences.push(event.sequence().unwrap());
            pending.finish(key, true);
        }
        assert_eq!(sequences, [1, 10_001, 20_000, 20_001]);
        assert!(pending.is_empty());
    }
    #[test]
    fn replacement_generation_discards_only_the_old_generation_work() {
        let mut pending = PendingSeats::default();
        let seat = Ulid::from(1);
        pending
            .push(HostId::Local, output(seat, 1, 1000), 1, 1)
            .unwrap();
        pending
            .push(HostId::Local, output(seat, 1, 2000), 1, 1)
            .unwrap();
        pending
            .push(HostId::Local, output(seat, 2, 1), 2, 1)
            .unwrap();
        let (key, event) = pending.next().unwrap();
        assert!(matches!(
            event,
            Event::Output {
                generation: 2,
                seq: 1,
                ..
            }
        ));
        pending.finish(key, true);
        assert!(pending.is_empty());
        assert!(pending.ready.is_empty());
    }
    #[test]
    fn down_host_work_is_parked_until_a_provider_up_transition() {
        let mut pending = PendingSeats::default();
        pending
            .push(HostId::Local, output(Ulid::from(1), 1, 1000), 1, 1)
            .unwrap();
        pending.park(&HostId::Local);
        assert!(!pending.has_ready());
        for _ in 0..100 {
            assert!(pending.next().is_none());
        }
        assert_eq!(pending.ready.len(), 1);
        pending.resume(&HostId::Local);
        assert!(pending.has_ready());
        let (key, event) = pending.next().unwrap();
        assert_eq!(event.sequence(), Some(1000));
        pending.finish(key, true);
        assert!(pending.is_empty());
    }
}
