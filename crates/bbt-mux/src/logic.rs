//! Restartable IO/logic owner. The UI gets an async bounded client, not a DB or PTY.

#[cfg(test)]
#[path = "knowledge/responsiveness_tests.rs"]
mod knowledge_responsiveness_tests;
#[cfg(test)]
#[path = "identity/reopen_tests.rs"]
mod native_reopen_tests;

#[cfg(test)]
#[path = "logic/ring_batch_tests.rs"]
mod ring_batch_tests;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::{
    Arc, OnceLock, Weak,
    atomic::{AtomicBool, AtomicU64, Ordering},
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
use crate::checkpoint::{Age, Cursor as CheckpointCursor, Policy as CheckpointPolicy, Saves};
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

fn first_available_seat_name(swarm: Ulid, records: &[SeatRecord]) -> String {
    use bbt_proto::seat_names::POOL;
    let occupied: BTreeSet<_> = records
        .iter()
        .filter(|record| record.swarm == swarm)
        .map(|record| record.name.as_str())
        .collect();
    // One more complete pool tier than the occupied set can fill guarantees a name.
    for tier in 0..=occupied.len() / POOL.len() {
        for base in POOL {
            let name = if tier == 0 {
                base.to_owned()
            } else {
                format!("{base} {}", tier + 1)
            };
            if !occupied.contains(name.as_str()) {
                return name;
            }
        }
    }
    unreachable!("the finite occupied set cannot fill the extra name tier")
}

#[cfg(test)]
mod seat_name_tests {
    use super::*;
    use bbt_proto::seat_names::POOL;

    // keeps: spec mux-daemon
    #[test]
    fn automatic_names_are_unique_in_the_swarm_for_every_harness() {
        let swarm = Ulid::from(1);
        let mut records = Vec::new();
        for kind in ["claude", "codex", "pi", "omp", "zsh", "generic"] {
            let name = first_available_seat_name(swarm, &records);
            assert!(POOL.contains(&name.as_str()));
            records.push(SeatRecord::new(Ulid::new(), swarm, name, kind));
        }
        let names: BTreeSet<_> = records.iter().map(|record| &record.name).collect();
        assert_eq!(names.len(), records.len());
        assert_eq!(first_available_seat_name(Ulid::from(2), &records), POOL[0]);
        // A retained closed seat still owns its name, including while it can be undone.
        records[0].lifecycle = Lifecycle::ClosedGrace;
        assert_ne!(first_available_seat_name(swarm, &records), POOL[0]);
    }

    // keeps: spec mux-daemon
    #[test]
    fn exhausted_pool_uses_the_first_unused_numbered_name() {
        let swarm = Ulid::from(1);
        let mut records: Vec<_> = POOL
            .into_iter()
            .map(|name| SeatRecord::new(Ulid::new(), swarm, name, "zsh"))
            .collect();
        records.push(SeatRecord::new(
            Ulid::new(),
            swarm,
            format!("{} 2", POOL[0]),
            "codex",
        ));
        assert_eq!(
            first_available_seat_name(swarm, &records),
            format!("{} 2", POOL[1])
        );
    }
}

struct CodexDiscovery {
    binding: Binding,
    result: std::result::Result<(String, serde_json::Value), String>,
}
struct SharedCodexDiscovery {
    binding: Binding,
    result: std::result::Result<bboxy450_harness_wake_up::RuntimeBinding, String>,
}

struct UiAdmission {
    retained: serde_json::Value,
    fresh: bool,
    journal: crate::mcp::journal::Journal,
    principal: crate::mcp::Principal,
    call: crate::mcp::Call,
}

enum Command {
    CtlUi(
        Ulid,
        bbt_proto::gui::UiRequest,
        crate::mcp::Origin,
        oneshot::Sender<Result<UiAdmission>>,
    ),
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
        crate::mcp::Origin,
        oneshot::Sender<Result<crate::messaging::api::Response>>,
    ),
    GuiTask(
        Ulid,
        bbt_proto::messaging::TaskRequest,
        crate::mcp::Origin,
        oneshot::Sender<Result<bbt_proto::messaging::TaskResponse>>,
    ),
    GuiRecordWrite(
        Ulid,
        serde_json::Value,
        crate::mcp::Origin,
        oneshot::Sender<crate::knowledge::data_source::Result<serde_json::Value>>,
    ),
    GuiPlugin(
        Ulid,
        bbt_proto::gui::PluginRequest,
        crate::mcp::Origin,
        oneshot::Sender<Result<Vec<bbt_plugin::registry::Entry>>>,
    ),
    GuiKnowledgeControl(
        Ulid,
        bbt_proto::gui::KnowledgeRequest,
        crate::mcp::Origin,
        oneshot::Sender<Result<bbt_proto::gui::KnowledgeStatus>>,
    ),
    GuiDebug(
        Ulid,
        bbt_proto::gui::DebugRequest,
        crate::mcp::Origin,
        oneshot::Sender<Result<serde_json::Value>>,
    ),
    GuiKnowledge(
        String,
        serde_json::Value,
        oneshot::Sender<crate::knowledge::data_source::Result<serde_json::Value>>,
    ),
    #[cfg(test)]
    KnowledgeFixture(Arc<dyn DataSource + Send + Sync>, oneshot::Sender<()>),
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
    GuiScreen(
        Ulid,
        bool,
        oneshot::Sender<Result<bbt_proto::gui::SeatScreen>>,
    ),
    GuiAttention(
        Ulid,
        Option<u64>,
        oneshot::Sender<Result<crate::seat_socket::GuiSeat>>,
    ),
    CtlAttention(
        Ulid,
        bbt_proto::gui::AttentionEdit,
        Ulid,
        crate::mcp::Origin,
        oneshot::Sender<Result<()>>,
    ),
    CtlRename(
        Ulid,
        String,
        Ulid,
        crate::mcp::Origin,
        oneshot::Sender<Result<()>>,
    ),
    CtlRestart(
        Ulid,
        u64,
        Ulid,
        crate::mcp::Origin,
        oneshot::Sender<Result<()>>,
    ),
    CtlRecord(
        Ulid,
        String,
        serde_json::Value,
        crate::mcp::Origin,
        oneshot::Sender<Result<()>>,
    ),
    CtlResize(
        Ulid,
        u64,
        u16,
        u16,
        Ulid,
        crate::mcp::Origin,
        oneshot::Sender<Result<()>>,
    ),
    CtlInput(
        Ulid,
        u64,
        Ulid,
        Vec<u8>,
        crate::mcp::Origin,
        oneshot::Sender<Result<()>>,
    ),
    GuiEndpoint(oneshot::Sender<Result<EndpointInfo>>),
    GuiHello(oneshot::Sender<Result<bbt_proto::gui::GuiHello>>),
    GuiSeats(oneshot::Sender<Result<Vec<crate::seat_socket::GuiSeat>>>),
    GuiSeat(Ulid, oneshot::Sender<Result<crate::seat_socket::GuiSeat>>),
    GuiIdentity(oneshot::Sender<Result<bbt_proto::GuiIdentity>>),
    OperatorIdentity(
        Ulid,
        bbt_proto::identity::IdentityRequest,
        crate::mcp::Origin,
        oneshot::Sender<Result<bbt_proto::identity::IdentityControl>>,
    ),
    GuiDevice(oneshot::Sender<Result<bbt_proto::messaging::OperatorReference>>),
    GuiWatchInterest {
        session: Ulid,
        life: Weak<GuiWatchLife>,
        seat: Ulid,
        generation: Option<u64>,
        reply: oneshot::Sender<Result<()>>,
    },
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
    CodexDiscovery(Box<CodexDiscovery>, oneshot::Sender<Result<bool>>),
    SharedCodexDiscovery(Box<SharedCodexDiscovery>, oneshot::Sender<Result<bool>>),
    NativeHookFailure(
        Box<(
            crate::adapters::Binding,
            crate::adapters::native::HookFailure,
        )>,
    ),
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
        bool,
        oneshot::Sender<Result<(crate::ring::History, Vec<crate::history_fault::Missing>)>>,
    ),
    Close(
        Ulid,
        u64,
        Ulid,
        Position,
        u64,
        Option<u64>,
        oneshot::Sender<Result<()>>,
    ),
    Undo(Ulid, u64, u64, u64, oneshot::Sender<Result<Position>>),
    Record(Ulid, oneshot::Sender<Result<SeatRecord>>),
    Endpoint(Ulid, oneshot::Sender<Result<EndpointInfo>>),
    IdentityStatus(oneshot::Sender<Result<crate::identity::IdentityStatus>>),
    #[cfg(test)]
    RefreshIdentity(oneshot::Sender<Result<()>>),
}
impl Command {
    fn local_while_checkpoint_pending(&self) -> bool {
        matches!(
            self,
            Self::GuiHosts(_)
                | Self::GuiEndpoint(_)
                | Self::GuiHello(_)
                | Self::GuiSeats(_)
                | Self::GuiSeat(..)
                | Self::GuiAttention(..)
                | Self::GuiIdentity(_)
                | Self::OperatorIdentity(..)
                | Self::IdentityStatus(_)
                | Self::Snapshot(..)
                | Self::Record(..)
                | Self::Endpoint(..)
                | Self::Status(..)
                | Self::NativeHookFailure(..)
                | Self::GuiWatchInterest { .. }
                | Self::GuiDebug(..)
                | Self::GuiPlugin(..)
                | Self::GuiKnowledgeControl(..)
        )
    }
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
fn messaging_response(result: serde_json::Value) -> Result<crate::messaging::api::Response> {
    let error = &result["structuredContent"]["error"];
    if result["isError"] == true
        && let (Some(kind), Some(message), Some(hint)) = (
            error["kind"].as_str(),
            error["message"].as_str(),
            error["hint"].as_str(),
        )
    {
        return Ok(crate::messaging::api::Response::Refused {
            error: bbt_proto::messaging::MessageRefusal {
                kind: kind.into(),
                message: message.into(),
                hint: hint.into(),
            },
        });
    }
    operator_response(result)
}
fn identity_response(
    mut result: serde_json::Value,
) -> Result<bbt_proto::identity::IdentityControl> {
    // MCP's retained result adds this wrapper field; it is not part of the strict DTO.
    if let Some(data) = result["structuredContent"].as_object_mut() {
        data.remove("op_id");
    }
    operator_response(result)
}

/// Private discovery result; never include the admission capability in logs.
#[derive(Clone)]
pub struct EndpointInfo {
    pub path: PathBuf,
    pub generation: u64,
    pub capability: [u8; 32],
}

/// Process-tree admission belongs to a seat generation, not a Logic process.
/// Never Debug/log this private durable value.
#[derive(Serialize, Deserialize)]
struct SeatAdmission {
    generation: u64,
    capability: [u8; 32],
}

struct GuiWatchLife {
    epoch: Arc<AtomicU64>,
}
impl Drop for GuiWatchLife {
    fn drop(&mut self) {
        // The last strong reference is gone before this notification. Cleanup
        // cannot be lost behind a full command channel or a cancelled socket.
        self.epoch.fetch_add(1, Ordering::Release);
    }
}

/// Ephemeral interest belongs to one GUI connection, never to Snapshot calls.
pub(crate) struct GuiWatchSession {
    id: Ulid,
    life: Arc<GuiWatchLife>,
}

struct GuiWatchOwner {
    life: Weak<GuiWatchLife>,
    seats: BTreeMap<Ulid, u64>,
}

#[derive(Default)]
struct GuiWatchInterests {
    sessions: BTreeMap<Ulid, GuiWatchOwner>,
    references: BTreeMap<(Ulid, u64), usize>,
}
impl GuiWatchInterests {
    fn remove_reference(&mut self, key: (Ulid, u64)) -> bool {
        let count = self.references.get_mut(&key).unwrap();
        *count -= 1;
        if *count == 0 {
            self.references.remove(&key);
            true
        } else {
            false
        }
    }

    fn update(
        &mut self,
        session: Ulid,
        life: Weak<GuiWatchLife>,
        seat: Ulid,
        generation: Option<u64>,
    ) -> Result<bool> {
        let _live = life
            .upgrade()
            .ok_or_else(|| invalid("GUI watch session closed"))?;
        if self
            .sessions
            .get(&session)
            .is_some_and(|owner| !Weak::ptr_eq(&owner.life, &life))
        {
            return Err(invalid("GUI watch session token changed"));
        }
        let old = self
            .sessions
            .get(&session)
            .and_then(|owner| owner.seats.get(&seat))
            .copied();
        if old == generation {
            return Ok(false);
        }
        let mut changed = false;
        if let Some(old) = old {
            self.sessions.get_mut(&session).unwrap().seats.remove(&seat);
            changed |= self.remove_reference((seat, old));
        }
        if let Some(generation) = generation {
            self.sessions
                .entry(session)
                .or_insert_with(|| GuiWatchOwner {
                    life,
                    seats: BTreeMap::new(),
                })
                .seats
                .insert(seat, generation);
            let references = self.references.entry((seat, generation)).or_default();
            changed |= *references == 0;
            *references += 1;
        } else if self.sessions[&session].seats.is_empty() {
            self.sessions.remove(&session);
        }
        Ok(changed)
    }

    fn prune_closed(&mut self) -> bool {
        let closed: Vec<_> = self
            .sessions
            .iter()
            .filter(|(_, owner)| owner.life.strong_count() == 0)
            .map(|(&session, _)| session)
            .collect();
        let mut changed = false;
        for session in closed {
            for key in self.sessions.remove(&session).unwrap().seats {
                changed |= self.remove_reference(key);
            }
        }
        changed
    }

    fn watched(&self) -> BTreeSet<(Ulid, u64)> {
        self.references.keys().copied().collect()
    }
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
    gui_watch_epoch: Arc<AtomicU64>,
    pub(crate) window_journal: Option<Arc<crate::window_journal::Journal>>,
}
impl LogicClient {
    pub(crate) fn gui_watch_session(&self) -> GuiWatchSession {
        GuiWatchSession {
            id: Ulid::new(),
            life: Arc::new(GuiWatchLife {
                epoch: self.gui_watch_epoch.clone(),
            }),
        }
    }

    pub(crate) async fn gui_watch_interest(
        &self,
        session: &GuiWatchSession,
        seat: Ulid,
        generation: Option<u64>,
    ) -> Result<()> {
        let (reply, receive) = oneshot::channel();
        self.tx
            .send(Command::GuiWatchInterest {
                session: session.id,
                life: Arc::downgrade(&session.life),
                seat,
                generation,
                reply,
            })
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        receive
            .await
            .map_err(|_| self.unavailable("logic stopped"))?
    }

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

    /// Lossless audit for recovery-fixture preparation, before its controlled kill.
    /// The ordinary product recorder remains nonblocking by default.
    #[doc(hidden)]
    pub fn enable_fixture_capture_diagnostics(&self) {
        self.capture_diagnostics.enable_fixture_lossless();
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
        if self.exit_cause.set(cause).is_ok() {
            if log {
                eprintln!("logic worker exited: {}", self.exit_cause.get().unwrap());
            }
            bbt_rt::error_journal::report(bbt_rt::error_journal::Failure::new(
                "logic",
                "worker_exit",
                self.exit_cause.get().unwrap(),
            ));
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
    /// The admitted operator connection is the authority; the peer supplies no identity.
    pub async fn gui_identity_control(
        &self,
        op: Ulid,
        request: bbt_proto::identity::IdentityRequest,
        origin: crate::mcp::Origin,
    ) -> Result<bbt_proto::identity::IdentityControl> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::OperatorIdentity(op, request, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    /// Logic resolves the immutable local operator reference for this GUI;
    /// custody and device attribution cannot move its durable consumer.
    pub(crate) async fn gui_device(&self) -> Result<bbt_proto::messaging::OperatorReference> {
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
        origin: crate::mcp::Origin,
    ) -> Result<crate::messaging::api::Response> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiMessaging(id, swarm, request, origin, tx))
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
        origin: crate::mcp::Origin,
    ) -> Result<bbt_proto::messaging::TaskResponse> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiTask(id, request, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| {
            self.unavailable("task outcome unknown; retry the original operation ID")
        })?
    }
    /// `ctl record write`: the operator's signed `e2e` record write (e2e acceptance §2).
    pub async fn gui_record_write(
        &self,
        id: Ulid,
        args: serde_json::Value,
        origin: crate::mcp::Origin,
    ) -> Result<crate::knowledge::data_source::Result<serde_json::Value>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiRecordWrite(id, args, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| {
            self.unavailable("record write outcome unknown; retry the original operation ID")
        })
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
    /// One seat's visible screen as text, from the snapshot the window watches.
    pub async fn gui_screen(&self, seat: Ulid, cells: bool) -> Result<bbt_proto::gui::SeatScreen> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiScreen(seat, cells, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    pub async fn gui_plugin(
        &self,
        id: Ulid,
        request: bbt_proto::gui::PluginRequest,
        origin: crate::mcp::Origin,
    ) -> Result<Vec<bbt_plugin::registry::Entry>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiPlugin(id, request, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| {
            self.unavailable("plugin outcome unknown; retain the original operation ID")
        })?
    }
    pub async fn gui_knowledge_control(
        &self,
        id: Ulid,
        request: bbt_proto::gui::KnowledgeRequest,
        origin: crate::mcp::Origin,
    ) -> Result<bbt_proto::gui::KnowledgeStatus> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiKnowledgeControl(id, request, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| {
            self.unavailable("knowledge outcome unknown; retain the original operation ID")
        })?
    }
    pub async fn gui_debug(
        &self,
        id: Ulid,
        request: bbt_proto::gui::DebugRequest,
        origin: crate::mcp::Origin,
    ) -> Result<serde_json::Value> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiDebug(id, request, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    /// Input through the ctl door: the window's input path, journaled and audited as a
    /// `ctl` operation with its door and issuer, since nobody sits at that keyboard.
    pub async fn ctl_input(
        &self,
        seat: Ulid,
        generation: u64,
        id: Ulid,
        bytes: Vec<u8>,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::CtlInput(seat, generation, id, bytes, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("input outcome unknown; keep the original input id"))?
    }
    pub async fn gui_attention(
        &self,
        seat: Ulid,
        generation: Option<u64>,
    ) -> Result<crate::seat_socket::GuiSeat> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiAttention(seat, generation, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("attention read unavailable"))?
    }
    /// Operator-authority policy setter; never impersonates the target seat.
    pub async fn ctl_attention(
        &self,
        seat: Ulid,
        edit: bbt_proto::gui::AttentionEdit,
        op: Ulid,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::CtlAttention(seat, edit, op, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("attention outcome unknown; retain the operation id"))?
    }
    /// Rename a seat through the ctl door: a journaled registry write and a state event.
    pub async fn ctl_rename(
        &self,
        seat: Ulid,
        name: String,
        op: Ulid,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::CtlRename(seat, name, op, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("rename outcome unknown"))?
    }
    /// Restart a seat through the ctl door: stop its process, relaunch the next generation.
    pub async fn ctl_restart(
        &self,
        seat: Ulid,
        generation: u64,
        op: Ulid,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::CtlRestart(seat, generation, op, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("restart outcome unknown"))?
    }
    /// Admit on logic, then wait for the window outside the logic worker. A replay
    /// returns its retained layout and never repeats a chrome action.
    pub async fn ctl_ui(
        &self,
        op: Ulid,
        request: bbt_proto::gui::UiRequest,
        origin: crate::mcp::Origin,
    ) -> Result<bbt_proto::gui::UiLayout> {
        request.validate().map_err(invalid)?;
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::CtlUi(op, request.clone(), origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        let admission = rx
            .await
            .map_err(|_| self.unavailable("UI admission outcome unknown"))??;
        if !admission.fresh {
            return operator_response(admission.retained);
        }
        let result = match self.captures.ui(op, request).await {
            Ok(layout) => crate::mcp::success(op, serde_json::to_value(layout)?),
            Err(reason) => crate::mcp::error(
                op,
                if reason.starts_with("timeout:") || reason.starts_with("ui_frame_refused:") {
                    "unknown"
                } else {
                    "ui_refused"
                },
                &reason,
            ),
        };
        operator_response(admission.journal.complete(
            &admission.principal,
            &admission.call,
            result,
        )?)
    }
    /// Journals a ctl operation whose effect happens in the window (a relay), so it is
    /// recorded and audited like every other ctl op.
    pub async fn ctl_record(
        &self,
        op: Ulid,
        tool: &str,
        args: serde_json::Value,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::CtlRecord(op, tool.into(), args, origin, tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("outcome unknown"))?
    }
    /// Resize through the ctl door: the window's resize, journaled once per id like input.
    pub async fn ctl_resize(
        &self,
        seat: Ulid,
        generation: u64,
        cols: u16,
        rows: u16,
        id: Ulid,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::CtlResize(
                seat, generation, cols, rows, id, origin, tx,
            ))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await
            .map_err(|_| self.unavailable("resize outcome unknown"))?
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
    pub(crate) async fn gui_hello(&self) -> Result<bbt_proto::gui::GuiHello> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::GuiHello(tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    pub(crate) fn claim_gui(&self, pid: u32) -> Result<()> {
        let mut slot = self.gui_peers.write().unwrap_or_else(|e| e.into_inner());
        if let Some(peers) = slot.as_mut() {
            if peers.gui != pid && crate::ctl_door::parent(peers.gui).is_some() {
                return Err(invalid(
                    "app_already_running: native GUI owner is still alive",
                ));
            }
            if matches!(
                crate::ctl_door::classify(Some(pid as i32), *peers),
                crate::ctl_door::Admit::Refused(_)
            ) {
                return Err(invalid("native GUI claim refused for a seat descendant"));
            }
            peers.gui = pid;
        }
        Ok(())
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
    async fn codex_discovery(&self, discovery: CodexDiscovery) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::CodexDiscovery(Box::new(discovery), tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    async fn shared_codex_discovery(&self, discovery: SharedCodexDiscovery) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::SharedCodexDiscovery(Box::new(discovery), tx))
            .await
            .map_err(|_| self.unavailable("logic stopped"))?;
        rx.await.map_err(|_| self.unavailable("logic stopped"))?
    }
    fn native_hook_failure(
        &self,
        binding: crate::adapters::Binding,
        failure: crate::adapters::native::HookFailure,
    ) -> bool {
        self.tx
            .try_send(Command::NativeHookFailure(Box::new((binding, failure))))
            .is_ok()
    }
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
                seat, generation, operation, position, grace_ms, None, tx,
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
        self.retained_history(seat, generation, from, to, false)
            .await
            .map(|(history, _)| history)
    }
    pub(crate) async fn history_for_gui(
        &self,
        seat: Ulid,
        generation: u64,
        from: u64,
        to: u64,
    ) -> Result<(crate::ring::History, Vec<crate::history_fault::Missing>)> {
        self.retained_history(seat, generation, from, to, true)
            .await
    }
    async fn retained_history(
        &self,
        seat: Ulid,
        generation: u64,
        from: u64,
        to: u64,
        masks: bool,
    ) -> Result<(crate::ring::History, Vec<crate::history_fault::Missing>)> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::History(seat, generation, from, to, masks, tx))
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
    #[cfg(test)]
    Prepared(Box<crate::identity::IdentityRuntime>),
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
        Self::start_custodied_with_fixture_store(connection, config, None, handle)
    }
    /// The same fixture-host ownership path with an explicit persistent store.
    pub fn start_custodied_with_fixture_store(
        connection: impl SupervisorConnection + 'static,
        config: LogicConfig,
        store: Option<Arc<dyn crate::identity::DeviceKeyStore + Send + Sync>>,
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
            IdentityMode::Custodied(store),
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
    /// Explicit host deadlines for private IO fixtures; production defaults stay at two seconds.
    pub fn start_custodied_with_plugin_config(
        connection: impl SupervisorConnection + 'static,
        config: LogicConfig,
        store: Arc<dyn crate::identity::DeviceKeyStore + Send + Sync>,
        plugins: crate::plugins::HostConfig,
        handle: &tokio::runtime::Handle,
    ) -> Result<Self> {
        let mut seed = [0; 32];
        getrandom::fill(&mut seed)?;
        let bus_key = SigningKey::from_bytes(&seed);
        seed.fill(0);
        Self::start_inner_with_plugins(
            vec![Box::new(LocalProvider::new(connection))],
            config,
            bus_key,
            IdentityMode::Custodied(Some(store)),
            handle,
            plugins,
        )
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
        Self::start_inner_with_plugins(providers, config, key, identity, handle, Default::default())
    }
    fn start_inner_with_plugins(
        providers: Vec<Box<dyn Provider>>,
        config: LogicConfig,
        key: SigningKey,
        identity: IdentityMode,
        handle: &tokio::runtime::Handle,
        plugins: crate::plugins::HostConfig,
    ) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let custodied = !matches!(&identity, IdentityMode::Explicit(_));
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
            #[cfg(test)]
            IdentityMode::Prepared(identity) => *identity,
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
        broker.set_issued_by(identity.issued_by().map(str::to_owned));
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
            gui_watch_epoch: Arc::default(),
            window_journal: Some(Arc::new(crate::window_journal::Journal::open(
                &config.run_dir,
            )?)),
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
                let mut worker = match Worker::open_configured(
                    providers,
                    config,
                    bus,
                    runtime,
                    worker_client,
                    identity,
                    bus_window,
                    plugins,
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
    ring_refusal: Option<crate::Failure>,
    tap: OscTap,
    replies: VecDeque<(Ulid, Vec<u8>)>,
    native_reader: Option<NativeReader>,
    shared_codex_runtime: Option<PathBuf>,
    state: Playback,
    view_generation: Ulid,
    checkpoint_bytes: u64,
    checkpoint_sequence: u64,
    changed_at: Age,
    last_parsed_at: Instant,
    last_arrival_at: Instant,
    captured_sequence: u64,
    checkpoint_pending: Option<CheckpointCursor>,
    checkpoint_requested: bool,
    checkpoint_paused: bool,
    checkpoint_pauses: u64,
    idle_checkpoints: u64,
    forced_checkpoints: u64,
    checkpoint_retry_at: Instant,
}

struct CheckpointImage {
    cursor: CheckpointCursor,
    saved: RingSaved,
    attachment: Attachment,
    idle: bool,
}
enum CheckpointResult {
    Committed(u64),
    NotIdle(u64),
}
type CheckpointCompletion = (Ulid, CheckpointCursor, bool, Result<CheckpointResult>);

// A replay target belongs to a seat, not its entire supervisor host. Keep the
// active target fixed until it finishes; subsequent captured notifications use
// the same bounded coalescer as the supervisor connection.
#[derive(Default)]
struct PendingSeats {
    seats: BTreeMap<(HostId, Ulid), PendingSeat>,
    ready_watched: VecDeque<(HostId, Ulid)>,
    ready_background: VecDeque<(HostId, Ulid)>,
    watched: BTreeSet<(Ulid, u64)>,
    watched_streak: usize,
    parked: BTreeSet<HostId>,
    watched_index: ReadyIndex,
    background_index: ReadyIndex,
}

#[derive(Default)]
struct HostReady {
    order: VecDeque<(u128, Ulid)>,
    ring: BTreeMap<u128, Ulid>,
}

#[derive(Default)]
struct ReadyIndex {
    hosts: BTreeMap<HostId, HostReady>,
    head: u128,
    tail: u128,
}

impl ReadyIndex {
    fn runnable_len(&self, total: usize, parked: &BTreeSet<HostId>) -> usize {
        total
            - parked
                .iter()
                .filter_map(|host| self.hosts.get(host))
                .map(|host| host.order.len())
                .sum::<usize>()
    }
    fn runnable_rank(&self, ticket: u128, parked: &BTreeSet<HostId>) -> usize {
        let skipped: usize = parked
            .iter()
            .filter_map(|host| self.hosts.get(host))
            .map(|host| host.order.partition_point(|(t, _)| *t < ticket))
            .sum();
        (ticket - self.head) as usize - skipped
    }
    fn push(&mut self, key: &(HostId, Ulid), needs_ring: bool) {
        let host = self.hosts.entry(key.0.clone()).or_default();
        host.order.push_back((self.tail, key.1));
        if needs_ring {
            host.ring.insert(self.tail, key.1);
        }
        self.tail += 1;
    }
    fn pop(&mut self, key: &(HostId, Ulid)) {
        let host = self.hosts.get_mut(&key.0).expect("ready host indexed");
        let (ticket, seat) = host.order.pop_front().expect("ready seat indexed");
        debug_assert_eq!(seat, key.1);
        debug_assert_eq!(ticket, self.head);
        host.ring.remove(&ticket);
        self.head += 1;
        if host.order.is_empty() {
            self.hosts.remove(&key.0);
        }
    }
}

struct PendingSeat {
    active: Event,
    generation: u64,
    latest: u64,
    exited: bool,
    queued: bbt_supervisor::client::PendingEvents,
    first_unparsed: u64,
    ring_blocked: bool,
}
impl PendingSeat {
    fn needs_ring(&self) -> bool {
        !self.ring_blocked && self.active.sequence().unwrap() > self.first_unparsed
    }
}
impl PendingSeats {
    const WATCHED_QUANTA: usize = 3;

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.seats.is_empty()
    }
    fn has_ready(&self) -> bool {
        self.ready_watched
            .iter()
            .chain(&self.ready_background)
            .any(|(host, _)| !self.parked.contains(host))
    }
    fn ready_len(&self) -> usize {
        self.ready_watched.len() + self.ready_background.len()
    }
    fn is_watched(&self, key: &(HostId, Ulid)) -> bool {
        self.seats
            .get(key)
            .is_some_and(|slot| self.watched.contains(&(key.1, slot.generation)))
    }
    fn enqueue(&mut self, key: (HostId, Ulid)) {
        let needs_ring = self.seats[&key].needs_ring();
        if self.is_watched(&key) {
            self.watched_index.push(&key, needs_ring);
            self.ready_watched.push_back(key);
        } else {
            self.background_index.push(&key, needs_ring);
            self.ready_background.push_back(key);
        }
        // Each pending seat has one queue entry, apart from the page currently
        // checked out by next(). Interest changes never append tombstones.
        debug_assert!(self.ready_len() <= self.seats.len());
    }
    fn set_watched(&mut self, watched: BTreeSet<(Ulid, u64)>) {
        if self.watched == watched {
            return;
        }
        self.watched = watched;
        let old_watched = std::mem::take(&mut self.ready_watched);
        let old_background = std::mem::take(&mut self.ready_background);
        let mut promoted = VecDeque::new();
        let mut demoted = VecDeque::new();
        for key in old_watched {
            if self.is_watched(&key) {
                self.ready_watched.push_back(key);
            } else {
                demoted.push_back(key);
            }
        }
        for key in old_background {
            if self.is_watched(&key) {
                promoted.push_back(key);
            } else {
                self.ready_background.push_back(key);
            }
        }
        // New lane members join behind its existing waiters. Keep the current
        // quota: repeated Watch/Unwatch cannot reset background service.
        self.ready_watched.extend(promoted);
        self.ready_background.extend(demoted);
        self.rebuild_ready_indices();
    }
    fn rebuild_ready_indices(&mut self) {
        // Interest/generation changes already rebuild the ready lanes. Hot
        // prefetches use the per-host indices, never this full-lane walk.
        self.watched_index = ReadyIndex::default();
        self.background_index = ReadyIndex::default();
        for key in &self.ready_watched {
            self.watched_index.push(key, self.seats[key].needs_ring());
        }
        for key in &self.ready_background {
            self.background_index
                .push(key, self.seats[key].needs_ring());
        }
    }

    fn parsed(&mut self, key: &(HostId, Ulid), sequence: u64) {
        if let Some(slot) = self.seats.get_mut(key) {
            slot.first_unparsed = sequence.saturating_add(1);
        }
    }

    fn ring_candidates(&self, host: &HostId, limit: usize) -> Vec<Ulid> {
        // Tickets are contiguous within each lane; subtraction gives its rank
        // without scanning other hosts. Merge at most 2*limit indexed rows in
        // the exact future global 3:1 order, with empty-lane donation intact.
        if self.parked.contains(host) {
            return vec![];
        }
        let remaining = Self::WATCHED_QUANTA.saturating_sub(self.watched_streak);
        let watched = self
            .watched_index
            .runnable_len(self.ready_watched.len(), &self.parked);
        let background = self
            .background_index
            .runnable_len(self.ready_background.len(), &self.parked);
        let mut rows = Vec::with_capacity(limit * 2);
        if let Some(index) = self.watched_index.hosts.get(host) {
            for (&ticket, &seat) in index.ring.iter().take(limit) {
                let rank = self.watched_index.runnable_rank(ticket, &self.parked);
                let before = if rank < remaining {
                    0
                } else {
                    1 + (rank - remaining) / Self::WATCHED_QUANTA
                };
                rows.push((rank + before.min(background), seat));
            }
        }
        if let Some(index) = self.background_index.hosts.get(host) {
            for (&ticket, &seat) in index.ring.iter().take(limit) {
                let rank = self.background_index.runnable_rank(ticket, &self.parked);
                let before = remaining
                    .saturating_add(Self::WATCHED_QUANTA * rank)
                    .min(watched);
                rows.push((rank + before, seat));
            }
        }
        rows.sort_unstable_by_key(|row| row.0);
        rows.into_iter().take(limit).map(|row| row.1).collect()
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
        let old_priority = self
            .seats
            .get(&key)
            .map(|slot| self.watched.contains(&(key.1, slot.generation)));
        let replaced = self.seats.insert(
            key.clone(),
            PendingSeat {
                active: event,
                generation,
                latest: sequence,
                exited,
                queued,
                first_unparsed: first,
                ring_blocked: false,
            },
        );
        if replaced.is_none() {
            self.enqueue(key);
        } else if old_priority != Some(self.is_watched(&key)) {
            let before = self.ready_len();
            self.ready_watched.retain(|queued| queued != &key);
            self.ready_background.retain(|queued| queued != &key);
            if self.ready_len() < before {
                self.rebuild_ready_indices();
                self.enqueue(key);
            }
        } else {
            // A replacement generation can change ring eligibility while its
            // priority and position stay the same. This is a cold rebuild.
            self.rebuild_ready_indices();
        }
        Ok(())
    }
    fn pop_lane(&mut self, watched: bool) -> Option<(HostId, Ulid)> {
        let (ready, index) = if watched {
            (&mut self.ready_watched, &mut self.watched_index)
        } else {
            (&mut self.ready_background, &mut self.background_index)
        };
        for _ in 0..ready.len() {
            let key = ready.pop_front()?;
            index.pop(&key);
            if self.parked.contains(&key.0) {
                index.push(&key, self.seats[&key].needs_ring());
                ready.push_back(key);
            } else {
                return Some(key);
            }
        }
        None
    }
    fn next(&mut self) -> Option<((HostId, Ulid), Event)> {
        if self.watched_streak < Self::WATCHED_QUANTA
            && let Some(key) = self.pop_lane(true)
        {
            self.watched_streak += 1;
            return Some((key.clone(), self.seats[&key].active.clone()));
        }
        if let Some(key) = self.pop_lane(false) {
            self.watched_streak = 0;
            return Some((key.clone(), self.seats[&key].active.clone()));
        }
        let key = self.pop_lane(true)?;
        // Empty background capacity is donated, never borrowed from its next
        // runnable turn. At most three watched pages precede that turn.
        self.watched_streak = Self::WATCHED_QUANTA;
        Some((key.clone(), self.seats[&key].active.clone()))
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
        self.enqueue(key);
    }
}
fn native_hook_row(
    current: Option<&crate::adapters::Binding>,
    binding: &crate::adapters::Binding,
    failure: crate::adapters::native::HookFailure,
) -> Option<bbt_rt::error_journal::Failure> {
    if current != Some(binding) {
        return None;
    }
    let message = format!(
        "generation {} · {} · {} · {}",
        binding.generation,
        failure.source,
        crate::adapters::observe::short(&failure.source_path, 256),
        crate::adapters::observe::short(&failure.message, 512)
    );
    let mut row = bbt_rt::error_journal::Failure::new("hook", &failure.event, message)
        .seat(binding.seat)
        .harness("codex")
        .exit(failure.exit_code)
        .stderr(failure.stderr.as_bytes());
    if let Some(ms) = failure.duration_ms {
        row = row.duration(Duration::from_millis(ms));
    }
    Some(row)
}
struct NativeReader {
    binding: crate::adapters::Binding,
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
#[derive(Clone)]
struct RingPage {
    read: bbt_supervisor::protocol::RingRead,
    result: std::result::Result<Box<crate::ring::History>, crate::Failure>,
}
impl RingPage {
    fn can_serve(&self, read: &bbt_supervisor::protocol::RingRead) -> bool {
        self.read.attachment == read.attachment
            && self.read.from <= read.from
            && match &self.result {
                Err(_) => self.read.from == read.from,
                Ok(history) => history
                    .records
                    .iter()
                    .any(|r| r.event.sequence() == Some(read.from)),
            }
    }
    fn page(
        &self,
        read: &bbt_supervisor::protocol::RingRead,
    ) -> Option<std::result::Result<Box<crate::ring::History>, crate::Failure>> {
        if !self.can_serve(read) {
            return None;
        }
        match &self.result {
            Err(error) if self.read.from == read.from => Some(Err(error.clone())),
            Err(_) => None,
            Ok(history) => {
                if !history
                    .records
                    .iter()
                    .any(|r| r.event.sequence() == Some(read.from))
                {
                    return None;
                }
                let mut page = history.clone();
                page.records.retain(|r| {
                    r.event
                        .sequence()
                        .is_some_and(|n| n >= read.from && n <= read.to)
                });
                page.next = page
                    .records
                    .last()
                    .and_then(|r| r.event.sequence())
                    .filter(|last| *last < read.to)
                    .map(|last| last + 1);
                Some(Ok(page))
            }
        }
    }
}

struct Worker {
    connections: Connections,
    config: LogicConfig,
    registry: Registry,
    // Reads retain an immutable catalogue without holding the logic worker.
    // Scans and provenance updates use copy-on-write while an older read is active.
    index: Arc<Index>,
    knowledge_reads: Arc<tokio::sync::Semaphore>,
    knowledge_host: crate::knowledge::roots::Host,
    knowledge_jobs: tokio::task::JoinSet<crate::knowledge::seat_reads::Completion>,
    knowledge_seen: u64,
    debug_reads: Arc<tokio::sync::Semaphore>,
    plugins: crate::plugins::control::Control,
    plugin_jobs: tokio::task::JoinSet<()>,
    watch: FileWatch,
    bus: BusHandle,
    handle: tokio::runtime::Handle,
    views: BTreeMap<Ulid, ViewState>,
    attachments: BTreeMap<Ulid, Attachment>,
    client: LogicClient,
    endpoints: BTreeMap<Ulid, (EndpointInfo, crate::seat_socket::SeatEndpoint)>,
    pending_endpoints: VecDeque<(Ulid, u64)>,
    endpoint_root: PathBuf,
    gui_info: EndpointInfo,
    gui_endpoint: Option<crate::seat_socket::SeatEndpoint>,
    messaging: crate::messaging::Messaging,
    identity: crate::identity::IdentityRuntime,
    bus_window: Option<Ulid>,
    last_identity: Option<Vec<u8>>,
    deliveries: tokio::task::JoinSet<Delivery>,
    control_deliveries: tokio::task::JoinSet<ControlDelivery>,
    identity_jobs: tokio::task::JoinSet<(
        crate::mcp::journal::NativePending,
        Result<crate::identity::unlock::Finished>,
    )>,
    identity_active: Option<Ulid>,
    identity_seen: BTreeSet<Ulid>,
    identity_probe_at: Instant,
    identity_acl: bbt_proto::identity::CustodyAcl,
    control_journals: BTreeMap<Ulid, crate::mcp::journal::Journal>,
    provenance: crate::provenance::engine::Engine,
    operator_journal: crate::mcp::journal::Journal,
    history_faults: crate::history_fault::Masks,
    stale_gui_requests: u64,
    #[cfg(test)]
    knowledge_fixture: Option<Arc<dyn DataSource + Send + Sync>>,
    checkpoint_policy: CheckpointPolicy,
    checkpoint_saves: Saves,
    /// Seats `ctl seat restart` stopped, by the generation that is exiting.
    restarts: BTreeMap<Ulid, u64>,
    checkpoint_images: BTreeMap<Ulid, CheckpointImage>,
    checkpoint_completion: Option<std::sync::mpsc::Receiver<CheckpointCompletion>>,
    checkpoint_cursor: Option<Ulid>,
    checkpoint_progress: u64,
    gui_watches: GuiWatchInterests,
    gui_watch_epoch_seen: u64,
    ring_pages: BTreeMap<HostId, BTreeMap<Ulid, RingPage>>,
}
impl Worker {
    #[cfg(test)]
    fn open(
        providers: Vec<Box<dyn Provider>>,
        config: LogicConfig,
        bus: BusHandle,
        handle: tokio::runtime::Handle,
        client: LogicClient,
        identity: crate::identity::IdentityRuntime,
        bus_window: Option<Ulid>,
    ) -> Result<Self> {
        Self::open_configured(
            providers,
            config,
            bus,
            handle,
            client,
            identity,
            bus_window,
            Default::default(),
        )
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "the IO bootstrap owns the existing worker ports and host-only plugin budgets"
    )]
    fn open_configured(
        providers: Vec<Box<dyn Provider>>,
        config: LogicConfig,
        bus: BusHandle,
        handle: tokio::runtime::Handle,
        client: LogicClient,
        identity: crate::identity::IdentityRuntime,
        bus_window: Option<Ulid>,
        plugin_config: crate::plugins::HostConfig,
    ) -> Result<Self> {
        let connections = Connections::open(providers)?;
        let account = SqliteStore::open(&object_dir(&config.run_dir, "account")?)?;
        let registry = Registry::new(account.clone());
        // The selected support/run namespace keeps custom and test runs private.
        let plugin_roots = crate::plugins::DiscoveryRoots::new(
            config
                .run_dir
                .parent()
                .ok_or_else(|| invalid("run directory has no support parent"))?,
            &config.repository,
        );
        let plugin_registry = crate::plugins::Registry::with_config(
            account.clone(),
            plugin_roots,
            plugin_config.registry,
        )
        .map_err(|error| invalid(format!("{}: {}", error.code, error.message)))?;
        let plugins = crate::plugins::control::Control::new(plugin_registry.clone());
        let mut index = Index::open(
            &config.repository.join(".bboxy"),
            SqliteStore::open(&object_dir(&config.run_dir, "index")?)?,
        )?;
        index.set_issued_by(identity.issued_by().map(str::to_owned));
        index.route_activity(bus.clone(), handle.clone());
        // Markdown is authority even if a crash preceded the mirror update.
        // scan retains its durable outbox; the normal startup scan publishes it.
        let _ = index.scan()?;
        let knowledge_host = crate::knowledge::roots::Host::with_config(
            index.knowledge_root(),
            plugin_registry,
            handle.clone(),
            plugin_config.provider,
        )?;
        let knowledge_seen = knowledge_host.revision()?;
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
        messaging.operator_reference(identity.observed_operator_device()?)?;
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
        let checkpoint_policy = CheckpointPolicy::for_ring(&config.ring);
        let mut worker = Self {
            connections,
            config,
            registry,
            index: Arc::new(index),
            knowledge_reads: Arc::new(tokio::sync::Semaphore::new(2)),
            knowledge_host,
            knowledge_jobs: tokio::task::JoinSet::new(),
            knowledge_seen,
            debug_reads: Arc::new(tokio::sync::Semaphore::new(2)),
            plugins,
            plugin_jobs: tokio::task::JoinSet::new(),
            watch,
            bus,
            handle,
            views: BTreeMap::new(),
            attachments: BTreeMap::new(),
            client,
            endpoints: BTreeMap::new(),
            pending_endpoints: VecDeque::new(),
            endpoint_root,
            gui_info,
            gui_endpoint: Some(gui_endpoint),
            messaging,
            identity,
            bus_window,
            last_identity: None,
            deliveries: tokio::task::JoinSet::new(),
            control_deliveries: tokio::task::JoinSet::new(),
            identity_jobs: tokio::task::JoinSet::new(),
            identity_active: None,
            identity_seen: BTreeSet::new(),
            identity_probe_at: Instant::now(),
            identity_acl: bbt_proto::identity::CustodyAcl::Unknown,
            control_journals: BTreeMap::new(),
            provenance,
            history_faults: crate::history_fault::Masks::new(account.clone()),
            operator_journal: crate::mcp::journal::Journal::new(account),
            stale_gui_requests: 0,
            #[cfg(test)]
            knowledge_fixture: None,
            checkpoint_policy,
            checkpoint_saves: Saves::new(checkpoint_policy),
            restarts: BTreeMap::new(),
            checkpoint_images: BTreeMap::new(),
            checkpoint_completion: None,
            checkpoint_cursor: None,
            checkpoint_progress: 0,
            gui_watches: GuiWatchInterests::default(),
            gui_watch_epoch_seen: 0,
            ring_pages: BTreeMap::new(),
        };
        for (host, info) in worker.connections.owned_seats()? {
            // Several fixture links can wrap one local supervisor. Recover a
            // seat only through the link selected by its durable location.
            if worker.host_for_seat(info.seat)? == host {
                worker.attach(info, false)?;
            }
        }
        for (task, status) in worker.index.task_states() {
            worker.messaging.task_state(task, status, now_ms())?;
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
            let pending = event.payload["tool"] == "identity_unlock"
                && event.payload["signature"] == "pending";
            let op = event.payload["op_id"]
                .as_str()
                .and_then(|value| value.parse::<Ulid>().ok());
            let envelope = self.handle.block_on(self.bus.publish("account", event))?;
            if pending {
                self.operator_journal.bind_native_pending(
                    op.ok_or_else(|| invalid("native audit has no operation ID"))?,
                    &envelope,
                )?;
            }
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
    fn identity_control_status(&self) -> Result<bbt_proto::identity::IdentityControl> {
        let mut status = self.identity.custody_control_status()?;
        if status.acl != bbt_proto::identity::CustodyAcl::NotNative {
            status.acl = self.identity_acl;
        }
        status.pending = self
            .operator_journal
            .native_pending(128)?
            .into_iter()
            .map(|pending| bbt_proto::identity::PendingUnlock {
                op: pending.op,
                pending_digest: pending.attempt.position.map(|position| position.digest),
                outcome: pending.attempt.outcome,
                signature: "pending".into(),
                in_flight: self.identity_active == Some(pending.op),
            })
            .collect();
        Ok(status)
    }
    fn operator_identity(
        &mut self,
        op: Ulid,
        request: bbt_proto::identity::IdentityRequest,
        origin: crate::mcp::Origin,
    ) -> Result<bbt_proto::identity::IdentityControl> {
        use bbt_proto::identity::{IdentityRequest as R, UnlockOutcome as O};
        let origin = self.identity.bind_origin(origin);
        let principal = crate::mcp::Principal::Operator {
            device_id: self.identity.custody_operator_device()?,
            door: origin.door,
            issued_by: origin.issued_by,
        };
        let call = crate::mcp::Call {
            op_id: op,
            tool: match request {
                R::Status => "identity_status",
                R::Unlock => "identity_unlock",
            }
            .into(),
            args: serde_json::json!({"request":request}),
        };
        call.validate()?;
        let journal = self.operator_journal.clone();
        if request == R::Status {
            let status = self.identity_control_status()?;
            return identity_response(journal.native_status(
                &principal,
                &call,
                crate::mcp::success(op, serde_json::to_value(status)?),
            )?);
        }
        if journal.retained(&principal, &call)?.is_some() {
            return identity_response(journal.call(&principal, &call, || {
                crate::mcp::error(op, "unknown", "retained identity result unavailable")
            })?);
        }
        let mut status = self.identity_control_status()?;
        if status.hold.is_none() {
            return identity_response(journal.call(&principal, &call, || {
                crate::mcp::success(op, serde_json::to_value(status).unwrap())
            })?);
        }
        if let Some(failure) = &status.reopen_failed {
            return identity_response(journal.checked(
                &principal,
                &call,
                crate::mcp::error(
                    op,
                    "reopen_failed",
                    &format!("{}; native access will not be repeated", failure.reason),
                ),
            )?);
        }
        if !self.identity.has_native_custody_hold() {
            return identity_response(journal.checked(
                &principal,
                &call,
                crate::mcp::error(
                    op,
                    "held",
                    "unlock is only available for a native custody hold",
                ),
            )?);
        }
        let target = self.identity.native_unlock_target()?;
        if !self.identity_jobs.is_empty() || target.store.native_busy() {
            return identity_response(journal.checked(
                &principal,
                &call,
                crate::mcp::error(
                    op,
                    "Busy",
                    "one native custody attempt is already in flight; nothing executed",
                ),
            )?);
        }
        status.outcome = Some(O::Unknown);
        status.pending.push(bbt_proto::identity::PendingUnlock {
            op,
            pending_digest: None,
            outcome: O::Unknown,
            signature: "pending".into(),
            in_flight: true,
        });
        let initial = crate::mcp::success(op, serde_json::to_value(&status)?);
        let (retained, fresh) = journal.admit_native_unlock(
            &principal,
            &call,
            status.hold.clone().unwrap(),
            target.reference(),
            initial,
        )?;
        if !fresh {
            return identity_response(retained);
        }
        // Publish in the existing audit order before permitting the native attempt.
        // A bounded backlog cannot turn into unbounded work on the terminal owner.
        for _ in 0..4 {
            self.publish_control_audits()?;
            if journal
                .native_pending(128)?
                .iter()
                .any(|entry| entry.op == op && entry.attempt.position.is_some())
            {
                break;
            }
        }
        let pending = journal
            .native_pending(128)?
            .into_iter()
            .find(|entry| entry.op == op)
            .ok_or_else(|| invalid("native admission disappeared"))?;
        if pending.attempt.position.is_none() {
            return Err(invalid(
                "Busy: native audit is not yet published; native access did not start; retain the operation ID",
            ));
        }
        journal.start_native_unlock(op, &principal)?;
        self.start_identity_job(pending, target, crate::identity::unlock::JobKind::Attempt);
        let mut response = self.identity_control_status()?;
        response.outcome = Some(O::Unknown);
        journal.native_snapshot(
            op,
            &principal,
            crate::mcp::success(op, serde_json::to_value(&response)?),
        )?;
        Ok(response)
    }
    fn start_identity_job(
        &mut self,
        pending: crate::mcp::journal::NativePending,
        target: crate::identity::unlock::Target,
        kind: crate::identity::unlock::JobKind,
    ) {
        let journal = self.operator_journal.clone();
        let actor = pending.actor.clone();
        let op = pending.op;
        self.identity_active = Some(op);
        self.identity_seen.insert(op);
        let progress: crate::identity::UnlockProgress = Arc::new(move |row| {
            journal
                .native_progress(op, &actor, row)
                .map_err(|_| crate::identity::PendingReason::StorageUnavailable)
        });
        let work = self
            .handle
            .spawn_blocking(move || target.run(progress, kind));
        self.identity_jobs.spawn_on(
            async move {
                let result = work
                    .await
                    .map_err(|_| invalid("native custody task lost; outcome remains unknown"))
                    .and_then(|result| result);
                (pending, result)
            },
            &self.handle,
        );
    }
    fn service_identity_jobs(&mut self) -> Result<()> {
        use bbt_proto::identity::UnlockOutcome as O;
        while let Some(joined) = self.identity_jobs.try_join_next() {
            self.identity_active = None;
            let (pending, result) = match joined {
                Ok(value) => value,
                Err(_) => {
                    eprintln!("native custody task lost; its durable operation remains unknown");
                    continue;
                }
            };
            let (outcome, signer) = match result {
                Ok(finished) => {
                    self.identity_acl = finished.acl;
                    if let Some(runtime) = finished.runtime {
                        self.identity = runtime;
                    } else if let Some(failure) = finished.reopen_failed {
                        self.identity.note_reopen_failed(failure);
                    }
                    (finished.outcome, finished.signer)
                }
                Err(error) => {
                    eprintln!("native custody outcome unknown: {error}");
                    (O::Unknown, None)
                }
            };
            let mut status = self.identity_control_status()?;
            status.outcome = Some(if pending.attempt.finished {
                pending.attempt.outcome
            } else {
                outcome
            });
            for entry in &mut status.pending {
                if entry.op == pending.op {
                    entry.outcome = status.outcome.unwrap();
                    entry.in_flight = false;
                }
            }
            self.operator_journal.finish_native_unlock(
                pending.op,
                &pending.actor,
                outcome,
                crate::mcp::success(pending.op, serde_json::to_value(status)?),
            )?;
            if let Some(key) = signer {
                let public = key.verifying_key().to_bytes();
                let ranges = self.handle.block_on(self.bus.identity_signer(key, None))?;
                if let Some(run) = self.bus_window.take() {
                    crate::identity::seal_bus_window(&self.config.run_dir, run, ranges)?;
                }
                self.messaging
                    .resume_signing(self.identity.keys(), now_ms())?;
                // The immutable completion is joined to the original pending position.
                // One legitimate key can drain a bounded batch of its own completions.
                let mut completed = 0;
                for entry in self.operator_journal.native_pending(128)? {
                    if completed == 32 {
                        break;
                    }
                    if self
                        .identity
                        .resume_unlock_target(&entry.attempt.reference)
                        .is_err()
                    {
                        continue;
                    }
                    if let Some(completion) =
                        self.operator_journal.native_completion(entry.op, public)?
                    {
                        let envelope = self
                            .handle
                            .block_on(self.bus.publish("account", completion.clone()))?;
                        if envelope.provenance.signer != public {
                            return Err(invalid("native completion signer mismatch"));
                        }
                        self.operator_journal
                            .native_completion_published(entry.op, completion.id)?;
                        completed += 1;
                        self.identity_seen.remove(&entry.op);
                    }
                }
            }
        }
        if self.identity_probe_at.elapsed() >= Duration::from_secs(30) {
            self.identity_seen.clear();
            self.identity_probe_at = Instant::now();
        }
        if self.identity_jobs.is_empty()
            && let Some(pending) =
                self.operator_journal
                    .native_pending(128)?
                    .into_iter()
                    .find(|entry| {
                        entry.attempt.position.is_some() && !self.identity_seen.contains(&entry.op)
                    })
        {
            self.identity_seen.insert(pending.op);
            if let Ok(target) = self
                .identity
                .resume_unlock_target(&pending.attempt.reference)
            {
                // Recovery never repeats the interactive access, even for an unknown.
                let kind = if pending.attempt.started && !pending.attempt.finished {
                    crate::identity::unlock::JobKind::Cleanup
                } else {
                    crate::identity::unlock::JobKind::Sign
                };
                self.start_identity_job(pending, target, kind);
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
        if crate::mcp::attention::handles(&call.tool) {
            return self.attention_call(&actor, &call, &journal);
        }
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
                    Ok(None) => self.control_read(&actor, &record, &call, None),
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
    fn attention_call(
        &self,
        actor: &crate::mcp::SeatActor,
        call: &crate::mcp::Call,
        journal: &crate::mcp::journal::Journal,
    ) -> Result<serde_json::Value> {
        use crate::mcp::{self, attention as tool};
        let read = || {
            let value = self
                .registry
                .get(actor.seat_id)
                .and_then(|record| tool::view(&record).map_err(invalid));
            match value {
                Ok(value) => mcp::success(call.op_id, value),
                Err(error) => mcp::error(call.op_id, "unavailable", &error.to_string()),
            }
        };
        match tool::prepare(call) {
            Ok(tool::Request::Get) => journal.checked(actor, call, read()),
            Ok(tool::Request::Set(rule)) => journal.effect(actor, call, || {
                match self.registry.set_attention(actor.seat_id, actor.process_generation, rule) {
                    Ok(policy) => mcp::success(
                        call.op_id,
                        serde_json::json!({"seat_id":actor.seat_id,"policy":policy}),
                    ),
                    Err(error) if error.downcast_ref::<std::io::Error>()
                        .is_some_and(|e| e.kind() == std::io::ErrorKind::InvalidData) => {
                        mcp::error(call.op_id, "rejected", &error.to_string())
                    }
                    Err(_) => mcp::error(call.op_id, "unknown",
                        "attention outcome unavailable; retain the operation ID and read your policy"),
                }
            }),
            Err(reason) => journal.checked(actor, call, mcp::error(call.op_id, "rejected", &reason)),
        }
    }
    fn start_knowledge_read(
        &mut self,
        id: Ulid,
        generation: u64,
        call: crate::mcp::Call,
        reply: oneshot::Sender<Result<serde_json::Value>>,
    ) {
        let prepared = (|| -> Result<_> {
            let (actor, record) = self.control_actor(id, generation)?;
            call.validate()?;
            self.scan()?;
            let journal = self.control_journal(id)?;
            let revision = self.knowledge_host.revision()?;
            Ok((actor, record, journal, revision))
        })();
        let (actor, record, journal, revision) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                let _ = reply.send(Err(error));
                return;
            }
        };
        let orient = if call.tool == "orient" {
            match self.control_read(
                &actor,
                &record,
                &call,
                Some(serde_json::json!({"items":[],"truncated":false})),
            ) {
                Ok(value) => Some(value),
                Err(reason) => {
                    let _ = reply.send(journal.checked(
                        &actor,
                        &call,
                        crate::mcp::error(call.op_id, "rejected", &reason),
                    ));
                    return;
                }
            }
        } else {
            None
        };
        if crate::mcp::knowledge::handles(&call.tool)
            && let Err(reason) = crate::mcp::knowledge::prepare(&call)
        {
            let _ = reply.send(journal.checked(
                &actor,
                &call,
                crate::mcp::error(call.op_id, "rejected", &reason),
            ));
            return;
        }
        let permit = match self.knowledge_reads.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                let _ = reply.send(journal.checked(
                    &actor,
                    &call,
                    crate::mcp::error(
                        call.op_id,
                        "provider_busy",
                        "Knowledge read capacity is occupied; retry the read.",
                    ),
                ));
                return;
            }
        };
        let job = crate::knowledge::seat_reads::Job {
            host: self.knowledge_host.clone(),
            index: self.index.clone(),
            registry: self.registry.clone(),
            actor,
            record,
            call,
            journal,
            orient,
            revision,
            reply,
            permit,
        };
        self.knowledge_jobs
            .spawn_blocking_on(move || job.run(), &self.handle);
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
                    Ok(record) => tool::bounded(
                        call.op_id,
                        mcp::success(call.op_id, serde_json::to_value(record)?),
                        false,
                    ),
                    Err(reason) => mcp::error(call.op_id, "rejected", &reason),
                };
                journal.checked(actor, call, value)
            }
            Ok(tool::Request::Search(request)) => {
                let source = BboxySource::new(&self.index);
                let value = match source.search(request) {
                    Ok(page) => {
                        let empty = page.items.is_empty();
                        let data = mcp::tools::scoped_list(
                            actor,
                            serde_json::to_value(page)?,
                            empty.then_some("nothing_yet"),
                        );
                        tool::bounded(call.op_id, mcp::success(call.op_id, data), false)
                    }
                    Err(error) => mcp::error(call.op_id, &error.code, "knowledge search refused"),
                };
                journal.checked(actor, call, value)
            }
            Ok(tool::Request::Write(request)) => {
                journal.effect(actor, call, || self.knowledge_write(actor, call, *request))
            }
            Err(reason) => {
                journal.checked(actor, call, mcp::error(call.op_id, "rejected", &reason))
            }
        }
    }
    fn knowledge_write(
        &mut self,
        actor: &crate::mcp::SeatActor,
        call: &crate::mcp::Call,
        request: crate::knowledge::data_source::WriteRequest,
    ) -> serde_json::Value {
        use crate::{
            knowledge::data_source as d,
            mcp::{self, knowledge as tool},
        };
        if self.knowledge_host.external() {
            return mcp::error(
                call.op_id,
                "signed_writer_unavailable",
                "Select bboxy450-term-data_source-bboxy in Settings > Knowledge to write until the host signed-writer bridge is available.",
            );
        }
        let source = BboxySource::new(&self.index);
        let current = match request.reference.clone() {
            Some(reference) => match source.fetch(d::FetchRequest { reference }) {
                Ok(record) => {
                    if request
                        .etag
                        .as_ref()
                        .is_some_and(|etag| etag != &record.etag)
                    {
                        return mcp::error(
                            call.op_id,
                            "etag_mismatch",
                            "etag does not match the current record",
                        );
                    }
                    Some(record)
                }
                Err(error) => {
                    return mcp::error(
                        call.op_id,
                        &error.code,
                        "knowledge update reference is unavailable or stale",
                    );
                }
            },
            None => None,
        };
        let record_id = request.frontmatter["id"]
            .as_str()
            .expect("prepared knowledge id")
            .to_owned();
        let plan = match tool::write_plan(actor, call, request, current.as_ref()) {
            Ok(plan) => plan,
            Err(error) => return mcp::error(call.op_id, &error.code, &error.message),
        };
        let authority = crate::mcp::Actor::from(actor);
        let result = match plan {
            tool::WritePlan::Report(report) => {
                let effect = match crate::mcp::work_items::prepare(
                    &self.index,
                    &self.registry,
                    &authority,
                    &report,
                ) {
                    Ok(crate::mcp::work_items::Prepared::Effect(effect)) => effect,
                    Ok(_) => {
                        return mcp::error(
                            call.op_id,
                            "task_field_protected",
                            "task result write requires the existing Report transition",
                        );
                    }
                    Err(reason) => return mcp::error(call.op_id, "task_field_protected", &reason),
                };
                if !matches!(effect.as_ref(), crate::mcp::work_items::Effect::Task { document, .. }
                    if current.as_ref().is_some_and(|old| document.id == old.reference.id && document.path == old.reference.path && document.digest == old.reference.digest))
                {
                    return mcp::error(
                        call.op_id,
                        "stale_reference",
                        "task changed before Report; fetch it again",
                    );
                }
                let value = self.control_work_item(&authority, &report, *effect);
                if value["isError"] == true {
                    return value;
                }
                if self.scan().is_err() {
                    return mcp::error(
                        call.op_id,
                        "unknown",
                        "task Report applied; activity publication pending, inspect original operation",
                    );
                }
                let source = BboxySource::new(&self.index);
                let record = source
                    .resolve(d::ResolveRequest { locator: record_id })
                    .and_then(|reference| source.fetch(d::FetchRequest { reference }));
                match record {
                    Ok(record) => d::WriteResult {
                        record,
                        lossy: false,
                    },
                    Err(_) => {
                        return mcp::error(
                            call.op_id,
                            "unknown",
                            "task Report applied; record readback unavailable, never redispatch it",
                        );
                    }
                }
            }
            tool::WritePlan::Record(request) => {
                if request.frontmatter["type"] == "task" {
                    let document = crate::index::Document {
                        id: record_id,
                        path: request
                            .reference
                            .as_ref()
                            .map(|reference| reference.path.as_str())
                            .or(request.path.as_deref())
                            .expect("prepared knowledge path")
                            .to_owned(),
                        metadata: request.frontmatter.clone(),
                        body: request.body.clone(),
                        digest: current
                            .as_ref()
                            .map(|old| old.reference.digest.clone())
                            .unwrap_or_default(),
                        error: current.as_ref().and_then(|old| old.parse_error.clone()),
                        links: Vec::new(),
                    };
                    if let Err(reason) =
                        crate::mcp::work_items::authorize(&document, &self.registry, &authority)
                    {
                        return mcp::error(call.op_id, "task_field_protected", &reason);
                    }
                }
                let keys = self.identity.keys();
                if let Some(reason) = keys.signing_hold() {
                    return mcp::error(
                        call.op_id,
                        "held",
                        &format!("knowledge signing held: {reason}"),
                    );
                }
                let key = match keys.leaf(actor.seat_id, actor.process_generation) {
                    Ok(key) if keys.ready_issuer().is_some() => key,
                    _ => {
                        return mcp::error(
                            call.op_id,
                            "held",
                            "knowledge signing held: identity pending",
                        );
                    }
                };
                let context = crate::knowledge::WriteContext::seat(
                    actor.seat_id,
                    Some(actor.process_generation),
                    &key,
                );
                let source = BboxySource::new(&self.index);
                if request.reference.is_none() {
                    match source.resolve(d::ResolveRequest {
                        locator: call.op_id.to_string(),
                    }) {
                        Err(error) if error.code == "not_found" => (),
                        _ => {
                            return mcp::error(
                                call.op_id,
                                "rejected",
                                "knowledge create id already exists or cannot be resolved uniquely",
                            );
                        }
                    }
                }
                let result = match source.write(*request, &context) {
                    Ok(result) => result,
                    Err(error) if error.code == "decision_filename_required" => {
                        return mcp::error(call.op_id, &error.code, &error.message);
                    }
                    Err(error)
                        if matches!(
                            error.code.as_str(),
                            "write_not_confirmed" | "write_applied_result_unavailable"
                        ) =>
                    {
                        return mcp::error(
                            call.op_id,
                            "unknown",
                            "knowledge write outcome is not confirmed; inspect original operation, never redispatch it",
                        );
                    }
                    Err(error) => {
                        return mcp::error(
                            call.op_id,
                            &error.code,
                            "knowledge write refused before application",
                        );
                    }
                };
                if self.scan().is_err() {
                    return mcp::error(
                        call.op_id,
                        "unknown",
                        "knowledge write applied; activity publication pending, inspect original operation",
                    );
                }
                result
            }
        };
        match serde_json::to_value(result) {
            Ok(value) => tool::bounded(call.op_id, mcp::success(call.op_id, value), true),
            Err(_) => mcp::error(
                call.op_id,
                "unknown",
                "knowledge write applied; result serialization unavailable",
            ),
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
            crate::mcp::Actor::OperatorLocal { .. } => {
                return crate::mcp::error(call.op_id, "held", "work-item signing requires custody");
            }
            crate::mcp::Actor::Operator {
                device_id,
                door,
                issued_by,
            } => match self.identity.operator_signing() {
                Ok(mut authority) if authority.device_id == *device_id => {
                    // The Delegate message records who issued it, through which door (#269).
                    authority.origin = crate::mcp::Origin {
                        door: *door,
                        issued_by: issued_by.clone(),
                    };
                    Some(authority)
                }
                Ok(_) => return crate::mcp::error(call.op_id, "held", "operator device changed"),
                Err(error) => return crate::mcp::error(call.op_id, "held", &error.to_string()),
            },
        };
        let key = match actor {
            crate::mcp::Actor::OperatorLocal { .. } => {
                return crate::mcp::error(call.op_id, "held", "work-item signing requires custody");
            }
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
            crate::mcp::Actor::OperatorLocal { .. } => {
                return crate::mcp::error(call.op_id, "held", "work-item signing requires custody");
            }
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
                bbt_proto::messaging::Recipient::Operator(_)
                | bbt_proto::messaging::Recipient::OperatorLocal(_) => match task_swarm {
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
                    bbt_proto::messaging::Recipient::Operator(_)
                    | bbt_proto::messaging::Recipient::OperatorLocal(_) => "operator/inbox".into(),
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
                crate::mcp::Actor::OperatorLocal { .. } => {
                    Err(invalid("work-item signing requires custody"))
                }
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
                Ok(message) => completed.push(message.view().id),
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
    /// Prefer the saved user layout, then follow durable launch anchors. A
    /// pre-placement legacy seat belongs to its swarm's bootstrap tab.
    fn seat_tab(&self, id: Ulid) -> Result<Ulid> {
        let saved = self.client.window_journal.as_ref().and_then(|j| j.latest());
        let mut current = id;
        for _ in 0..64 {
            if let Some(tab) = saved.as_ref().and_then(|state| {
                state
                    .swarms
                    .iter()
                    .flat_map(|s| &s.tabs)
                    .find(|tab| tab.seats.contains(&current))
            }) {
                return Ok(tab.id);
            }
            let record = self.registry.get(current)?;
            match record.launch_placement {
                Some(bbt_proto::seat_state::Placement::NewTab { tab }) => {
                    return Ok(Ulid::from(tab.0.0));
                }
                Some(bbt_proto::seat_state::Placement::Split { target, tab, .. }) => {
                    if self.registry.get(Ulid::from(target.0.0)).is_err() {
                        return Ok(Ulid::from(tab.0.0));
                    }
                    current = Ulid::from(target.0.0);
                }
                None => return Ok(record.swarm),
            }
        }
        Err(invalid("placement anchor chain exceeds bound"))
    }
    fn control_placement(
        &self,
        args: &crate::mcp::seats::Spawn,
        id: Ulid,
    ) -> Result<bbt_proto::seat_state::Placement> {
        use bbt_proto::seat_state::Placement;
        if let Some(target) = args.target() {
            let tab = self.seat_tab(target.seat)?;
            if args.tab_id.is_some_and(|expected| expected != tab) {
                return Err(invalid("placement anchor tab changed"));
            }
            Ok(Placement::Split {
                tab: bbt_proto::TabId::from_u128(tab.0),
                target: bbt_proto::SeatId::from_u128(target.seat.0),
                target_generation: target.generation,
                side: args.side.ok_or_else(|| invalid("missing split side"))?,
            })
        } else {
            let tab_id = args.tab_id.unwrap_or(id);
            let occupied = self.client.window_journal.as_ref().and_then(|j| j.latest()).is_some_and(|state| {
                state.swarms.iter().flat_map(|s| &s.tabs).any(|tab| tab.id == tab_id && !tab.seats.contains(&id))
            }) || self.registry.list()?.iter().any(|r| r.id != id && matches!(&r.launch_placement, Some(Placement::NewTab { tab }) if tab.0.0 == tab_id.0));
            if occupied {
                return Err(invalid("new tab_id is already occupied"));
            }
            Ok(Placement::NewTab {
                tab: bbt_proto::TabId::from_u128(tab_id.0),
            })
        }
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
        if matches!(request, crate::mcp::seats::Request::Focus { .. })
            && target
                .as_ref()
                .is_some_and(|r| r.lifecycle != Lifecycle::Open)
        {
            return Err("focus target is closed".into());
        }
        if let crate::mcp::seats::Request::Spawn(args) = &request {
            if !args.cwd.is_dir() {
                return Err("spawn cwd is not an available directory".into());
            }
            if args.target().is_some_and(|target| {
                self.registry
                    .get(target.seat)
                    .is_ok_and(|r| r.lifecycle != Lifecycle::Open)
            }) {
                return Err("split anchor is closed".into());
            }
            self.control_placement(
                args,
                crate::mcp::seats::effect_id(actor, call, args.target()),
            )
            .map_err(|e| e.to_string())?;
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
            Request::Layout {
                swarm,
                after,
                limit,
            } => {
                let mut records: Vec<_> = self
                    .registry
                    .list()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .filter(|r| r.swarm == *swarm && after.is_none_or(|id| r.id > id))
                    .collect();
                records.sort_by_key(|r| r.id);
                let more = records.len() > *limit;
                records.truncate(*limit);
                let next = more.then(|| records.last().unwrap().id);
                let seats: Vec<_> = records.iter().map(|r| json!({
                    "seat_id":r.id,"name":r.name,"process_generation":r.generation.to_string(),"lifecycle":r.lifecycle,
                    "tab_id":self.seat_tab(r.id).ok(),"launch_placement":r.launch_placement
                })).collect();
                Ok(
                    json!({"swarm_id":swarm,"ui_attached":self.client.captures.attached(),"layout_source":"saved_window_or_launch_placement","seats":seats,"next":next}),
                )
            }
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
                    rows.push(row_text(cells));
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
                crate::adapters::mapping::literal(id, &text, submit),
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
                        args.name
                            .clone()
                            .map(Ok)
                            .unwrap_or_else(|| self.default_seat_name(args.swarm_id))?,
                        harness.kind.clone(),
                    );
                    record.launch_placement = Some(self.control_placement(&args, id)?);
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
                            env: vec![],
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
                        json!({"status":"applied","seat_id":info.seat,"process_generation":info.generation.to_string(),"pid":info.pid,"placement":self.registry.get(info.seat).ok().and_then(|r| r.launch_placement)}),
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
    fn start_control_focus(
        &mut self,
        id: Ulid,
        generation: u64,
        call: crate::mcp::Call,
        reply: oneshot::Sender<Result<serde_json::Value>>,
    ) {
        use crate::mcp::seats::{self, Request};
        let mut reply = Some(reply);
        let started = (|| -> Result<Option<serde_json::Value>> {
            let (actor, caller) = self.control_actor(id, generation)?;
            call.validate()?;
            let journal = self.control_journal(id)?;
            let target = match self.prepare_seat_control(&actor, &caller, &call) {
                Ok(Request::Focus { target }) => target,
                other => {
                    let reason = other
                        .err()
                        .unwrap_or_else(|| "invalid focus dispatch".into());
                    return journal
                        .checked(
                            &actor,
                            &call,
                            crate::mcp::error(call.op_id, "rejected", &reason),
                        )
                        .map(Some);
                }
            };
            let (retained, fresh) = journal.admit(&actor, &call)?;
            if !fresh {
                return Ok(Some(retained));
            }
            if !self.client.captures.attached() || self.control_deliveries.len() >= 8 {
                return journal
                    .complete(
                        &actor,
                        &call,
                        crate::mcp::error(
                            call.op_id,
                            "unavailable",
                            "no window attached or focus capacity full",
                        ),
                    )
                    .map(Some);
            }
            let captures = self.client.captures.clone();
            let run_dir = self.config.run_dir.clone();
            let harness = self.views[&target.seat].adapter.record.kind.clone();
            let input_id = seats::effect_id(&actor, &call, Some(target));
            let call = call.clone();
            let sender = reply.take().unwrap();
            self.control_deliveries.spawn_on(async move {
                let check_actor = actor.clone();
                let checked = tokio::task::spawn_blocking(move || seats::revalidate_focus(&run_dir,&check_actor,target,&harness)).await;
                let result = match checked {
                    Ok(Ok(())) => match captures.focus(input_id,bbt_proto::SeatId::from_u128(target.seat.0)).await {
                        Ok(frame) => crate::mcp::success(call.op_id,serde_json::json!({"status":"applied","seat_id":target.seat,"process_generation":target.generation.to_string(),"frame":frame})),
                        Err(reason) => crate::mcp::error(call.op_id,"unavailable",&reason),
                    },
                    Ok(Err(reason)) => crate::mcp::error(call.op_id,"rejected",&reason),
                    Err(_) => crate::mcp::error(call.op_id,"unknown","focus validation interrupted"),
                };
                ControlDelivery { actor,call,journal,reply:sender,target,input_id,outcome:None,result }
            }, &self.handle);
            Ok(None)
        })();
        match started {
            Ok(None) => (),
            Ok(Some(value)) => {
                let _ = reply.take().unwrap().send(Ok(value));
            }
            Err(error) => {
                if let Some(reply) = reply {
                    let _ = reply.send(Err(error));
                }
            }
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
            Ok(crate::messaging::api::Response::Refused { error }) => {
                crate::mcp::messaging::refused(call.op_id, error)
            }
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
        documents: Option<serde_json::Value>,
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
                let tasks = match documents {
                    Some(tasks) => tasks,
                    None => tools::orient_documents(&source, record)?,
                };
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
                    "attention":crate::mcp::attention::view(record)?,
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
        for event in Arc::make_mut(&mut self.index).scan()? {
            if let Some(task) = event
                .subject
                .strip_prefix("task/")
                .and_then(|s| s.strip_suffix("/status"))
                .and_then(|s| s.parse::<Ulid>().ok())
                && let Some(status) = event.payload["to"].as_str()
            {
                self.messaging.task_state(task, status, now_ms())?;
            }
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
        let host = self.host_for_seat(info.seat)?;
        if let Some(pages) = self.ring_pages.get_mut(&host) {
            pages.remove(&info.seat);
        }
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
        let mut checkpoint_bytes = state.byte_end;
        let mut checkpoint_sequence = state.sequence;
        let mut changed_at = Age::default();
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
                let next_bytes = match &record.event {
                    Event::Output { bytes, .. } => bytes.len() as u64,
                    _ => 0,
                };
                if state.byte_end > checkpoint_bytes
                    && (state.byte_end - checkpoint_bytes + next_bytes
                        > self.checkpoint_policy.force_bytes
                        || changed_at.expired(Instant::now(), self.checkpoint_policy))
                {
                    // Rule 4's explicit bootstrap/recovery exception: the
                    // forced #11 boundary commits before attachment readiness.
                    // Incremental ingestion below never waits for these saves.
                    self.capture_request(
                        info.seat,
                        Action::PublishCheckpoint {
                            attachment: attachment.clone(),
                            checkpoint: state.saved().encode()?,
                        },
                    )?;
                    checkpoint_bytes = state.byte_end;
                    checkpoint_sequence = state.sequence;
                    changed_at.completed(true, false);
                }
                state.apply(&record.event)?;
                if next_bytes > 0 {
                    changed_at.arrived(Instant::now());
                }
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
        let shared_codex_runtime = SqliteStore::open(&directory)?
            .get("adapter_shared_runtime_endpoint")?
            .filter(|bytes| !bytes.is_empty())
            .map(|bytes| serde_json::from_slice::<Option<PathBuf>>(&bytes))
            .transpose()?
            .flatten();
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
                runtime: info
                    .companion
                    .as_ref()
                    .filter(|_| shared_codex_runtime.is_none())
                    .map(|c| bboxy450_harness_wake_up::RuntimeBinding {
                        endpoint: c.ready_socket.clone(),
                        pid: c.pid,
                        process_start: crate::adapters::native::process_start(c.pid)
                            .unwrap_or_else(|_| "exited".into()),
                    }),
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
                ring_refusal: None,
                tap,
                replies,
                native_reader: None,
                shared_codex_runtime,
                state,
                view_generation: Ulid::new(),
                checkpoint_bytes,
                checkpoint_sequence,
                changed_at,
                last_parsed_at: Instant::now(),
                last_arrival_at: Instant::now(),
                captured_sequence: status.captured_sequence,
                checkpoint_pending: None,
                checkpoint_requested: false,
                checkpoint_paused: false,
                checkpoint_pauses: 0,
                idle_checkpoints: 0,
                forced_checkpoints: 0,
                checkpoint_retry_at: Instant::now(),
            },
        );
        self.checkpoint_saves.retire(info.seat);
        self.checkpoint_saves.generation(info.seat, info.generation);
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
        if let Err(error) = self.ensure_endpoint(info.seat, info.generation) {
            if !error.is::<crate::seat_socket::SeatListenerPending>() {
                return Err(error);
            }
            // A child forked before the old logic died can retain its listener
            // until exec. Keep the recovered view live; retry only the bind.
            self.pending_endpoints
                .push_back((info.seat, info.generation));
        }
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
        // Surviving harness/MCP processes retain their launch environment.
        // Keep both the short address and admission across a Logic restart;
        // a new seat generation gets a new capability and rejects old peers.
        use std::os::unix::ffi::OsStrExt;
        let run = std::fs::canonicalize(&self.config.run_dir)?;
        let digest = crate::store::digest(run.as_os_str().as_bytes());
        // SAFETY: geteuid has no pointer arguments or side effects.
        let root = PathBuf::from("/tmp").join(format!(
            "bbt-seat-{}-{}",
            unsafe { libc::geteuid() },
            &digest[..20],
        ));
        let path = root.join(format!("{id}.sock"));
        let directory = object_dir(&self.config.run_dir, &format!("seat-{id}"))?;
        let store = SqliteStore::open(&directory)?;
        let saved = store
            .get("seat_admission")?
            .map(|bytes| serde_json::from_slice::<SeatAdmission>(&bytes))
            .transpose()?;
        let admission = match saved {
            Some(saved) if saved.generation == generation => saved,
            _ => {
                let mut capability = [0; 32];
                getrandom::fill(&mut capability).map_err(std::io::Error::other)?;
                SeatAdmission {
                    generation,
                    capability,
                }
            }
        };
        let capability = admission.capability;
        let endpoint = crate::seat_socket::SeatEndpoint::bind(
            &self.handle,
            &path,
            self.client.clone(),
            id,
            generation,
            capability,
        )?;
        store.put("seat_admission", &serde_json::to_vec(&admission)?)?;
        let info = EndpointInfo {
            path,
            generation,
            capability,
        };
        self.endpoints.insert(id, (info.clone(), endpoint));
        Ok(info)
    }
    fn retry_endpoint(&mut self) -> Result<()> {
        let Some((id, generation)) = self.pending_endpoints.pop_front() else {
            return Ok(());
        };
        if self
            .views
            .get(&id)
            .is_none_or(|view| view.generation != generation)
        {
            return Ok(());
        }
        if let Err(error) = self.ensure_endpoint(id, generation) {
            if !error.is::<crate::seat_socket::SeatListenerPending>() {
                return Err(error);
            }
            self.pending_endpoints.push_back((id, generation));
        }
        Ok(())
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
                let endpoint = store
                    .get("adapter_runtime_endpoint")?
                    .filter(|b| !b.is_empty())
                    .map(|b| serde_json::from_slice::<Option<PathBuf>>(&b))
                    .transpose()?
                    .flatten();
                bboxy450_harness_wake_up::launch::cleanup_overlay(&path, endpoint.as_deref())?;
            }
            store.put("adapter_overlay", b"")?;
            store.put("adapter_runtime_endpoint", b"")?;
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
        crate::adapters::launch::with_environment(&mut plan.spec, &options.env);
        record.harness_session_id = plan.harness_session_id;
        crate::adapters::launch::in_login_shell(
            &mut plan.spec,
            &harness.kind,
            &options.user_home,
            options.user_zdotdir.as_deref(),
        );
        // Explicit PATH also works with a supervisor retained from a Finder launch.
        if let Ok(path) = std::env::var("PATH") {
            plan.spec
                .env
                .entry("PATH".into())
                .or_insert_with(|| path.clone());
            if let Some(companion) = &mut plan.spec.companion {
                companion.env.entry("PATH".into()).or_insert(path);
            }
        }
        let diagnostics_root = self.config.run_dir.to_string_lossy().into_owned();
        plan.spec
            .env
            .insert("BBOXY_RUN_DIR".into(), diagnostics_root.clone());
        if let Some(companion) = &mut plan.spec.companion {
            companion
                .env
                .insert("BBOXY_RUN_DIR".into(), diagnostics_root);
        }
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
        store.put(
            "adapter_runtime_endpoint",
            &serde_json::to_vec(&plan.spec.companion.as_ref().map(|c| &c.ready_socket))?,
        )?;
        store.put(
            "adapter_shared_runtime_endpoint",
            &serde_json::to_vec(&plan.shared_runtime)?,
        )?;
        // On an ambiguous spawn keep the overlay; deleting it could break a child
        // that actually started. Reconciliation uses the original durable seat ID.
        self.spawn(record, plan.spec)
    }
    fn recover_seats(&mut self) -> Result<()> {
        for record in self.registry.list()? {
            if record.location != crate::registry::Location::Local
                || record.lifecycle != Lifecycle::Open
                || self.views.contains_key(&record.id)
            {
                continue;
            }
            let store = SqliteStore::open(&object_dir(
                &self.config.run_dir,
                &format!("seat-{}", record.id),
            )?)?;
            if store.get("process_running")?.as_deref() != Some(b"1")
                || store.get("recovery_launch")?.is_none()
            {
                continue;
            }
            self.relaunch(record)?;
        }
        Ok(())
    }
    /// The next process generation of a seat whose process is gone (#11): its committed
    /// ring replayed as the restored screen with the gap named, a harness seat resumed by
    /// its session id, any other seat a fresh zsh.
    fn relaunch(&mut self, mut record: SeatRecord) -> Result<()> {
        {
            let root = object_dir(&self.config.run_dir, &format!("seat-{}", record.id))?;
            let store = SqliteStore::open(&root)?;
            let bytes = store.get("recovery_launch")?.ok_or_else(|| {
                invalid(format!(
                    "RecoveryUnavailable: seat {} has no launch context",
                    record.id
                ))
            })?;
            let context: RecoveryLaunch = serde_json::from_slice(&bytes)?;
            let previous = record.generation;
            let mut state = if let Some(view) = self.views.get(&record.id) {
                // A restart under a live supervisor: it still holds this generation's ring,
                // so logic's own playback of it is the restored screen, never a second
                // writer on that ring.
                crate::ring::Playback::restore(view.state.saved())
            } else {
                let ring_root = root.join(format!("ring-{previous}"));
                // The old supervisor is already reaped. Reopening its ring rejects
                // torn/uncommitted suffixes and replays only durable capture.
                if !ring_root.join("committed").is_file() && !ring_root.join("previous").is_file() {
                    return Err(invalid(format!(
                        "RecoveryUnavailable: seat {} has no committed predecessor generation {previous}",
                        record.id
                    )));
                }
                crate::ring::Ring::open(
                    &ring_root,
                    record.id,
                    previous,
                    0,
                    self.config.ring.clone(),
                )?
                .replay()?
            };
            let retained = format!(
                "generation {previous}, committed through capture sequence {} ({} bytes)",
                state.sequence, state.byte_end
            );
            // Resolve the old launch only through this seat's private store,
            // before launch() replaces its retained paths. Never scan /tmp or
            // remove a still-present socket or the predecessor's overlay.
            if let Some(bytes) = store
                .get("adapter_overlay")?
                .filter(|bytes| !bytes.is_empty())
            {
                let name = std::str::from_utf8(&bytes)?;
                if !name.starts_with("launch-") || name[7..].parse::<Ulid>().is_err() {
                    return Err(invalid("invalid retained overlay name"));
                }
                let endpoint = store
                    .get("adapter_runtime_endpoint")?
                    .filter(|bytes| !bytes.is_empty())
                    .map(|bytes| serde_json::from_slice::<Option<PathBuf>>(&bytes))
                    .transpose()?
                    .flatten();
                bboxy450_harness_wake_up::launch::cleanup_stale_runtime_alias(
                    &root.join("overlays").join(name),
                    endpoint.as_deref(),
                )?;
            }
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
                    env: vec![],
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
    /// `ctl seat restart` (Oracle ruling, 2026-10-24): a live seat's process is stopped
    /// (SIGHUP, SIGKILL after 250 ms) and relaunched when its exit lands; an exited seat
    /// relaunches at once. Journaled once per op id; `generation` is the one the caller saw.
    fn ctl_restart(
        &mut self,
        id: Ulid,
        generation: u64,
        op: Ulid,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        let args = serde_json::json!({"seat":id,"generation":generation});
        self.ctl_journaled(op, "seat_restart", args, origin, |this| {
            let record = this.registry.get(id)?;
            if record.generation != generation {
                return Err(invalid(format!(
                    "StaleGeneration: seat {id} is at generation {}",
                    record.generation
                )));
            }
            if record.location != crate::registry::Location::Local
                || record.lifecycle != Lifecycle::Open
            {
                return Err(invalid("only an open local seat restarts"));
            }
            let store =
                SqliteStore::open(&object_dir(&this.config.run_dir, &format!("seat-{id}"))?)?;
            if store.get("process_running")?.as_deref() != Some(b"1") {
                return this.relaunch(record);
            }
            let attachment = this
                .attachments
                .get(&id)
                .ok_or_else(|| invalid("seat not attached"))?
                .clone();
            this.restarts.insert(id, generation);
            let stopped = this.request_for_seat(id, Action::Kill { attachment });
            if stopped.is_err() {
                this.restarts.remove(&id);
            }
            stopped.map(|_| ())
        })
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
        let Reply::Spawned { info } = self
            .request_for_seat(record.id, Action::Spawn { spec })
            .inspect_err(|e| {
                bbt_rt::error_journal::report(
                    bbt_rt::error_journal::Failure::new("spawn", "seat", e)
                        .seat(record.id)
                        .harness(&record.harness_kind),
                );
            })?
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
    /// The window's input path: the displayed generation, then the seat's attachment.
    fn seat_input(
        &mut self,
        id: Ulid,
        generation: u64,
        input_id: Ulid,
        bytes: Vec<u8>,
    ) -> Result<crate::InputReceipt> {
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
    }
    /// The window's resize path: the displayed generation, clamped to the grid's bounds.
    fn seat_resize(&mut self, id: Ulid, generation: u64, cols: u16, rows: u16) -> Result<()> {
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
    }
    fn ctl_ui_admit(
        &self,
        op: Ulid,
        request: bbt_proto::gui::UiRequest,
        origin: crate::mcp::Origin,
    ) -> Result<UiAdmission> {
        request.validate().map_err(invalid)?;
        if matches!(&request, bbt_proto::gui::UiRequest::Clock(_))
            && !self
                .identity
                .issued_by()
                .is_some_and(|label| label.starts_with("test-account:"))
        {
            return Err(invalid(
                "test_account_required: UI clock control requires a verified test account",
            ));
        }
        let origin = self.identity.bind_origin(origin);
        let device_id = self.operator_device_for_effect()?;
        let principal = crate::mcp::Principal::Operator {
            device_id,
            door: origin.door,
            issued_by: origin.issued_by,
        };
        let call = crate::mcp::Call {
            op_id: op,
            tool: "ui_relay".into(),
            args: serde_json::to_value(request)?,
        };
        call.validate()?;
        let (retained, fresh) = self.operator_journal.admit(&principal, &call)?;
        Ok(UiAdmission {
            retained,
            fresh,
            journal: self.operator_journal.clone(),
            principal,
            call,
        })
    }
    fn operator_device_for_effect(&self) -> Result<Ulid> {
        let status = self.identity.status()?;
        status.device_id.ok_or_else(|| {
            invalid(format!(
                "held: {}; clear with {} or {}",
                status
                    .reason_label
                    .as_deref()
                    .unwrap_or("operator device custody is unavailable"),
                crate::identity::unlock::CLEAR_ACTIONS[0],
                crate::identity::unlock::CLEAR_ACTIONS[1]
            ))
        })
    }
    /// One `ctl` effect under the operator principal with its door and issuer, journaled
    /// once per op id (a retry replays, never repeats) and audited on `account/control`.
    /// `args` is what the journal keeps: never typed bytes, only their count and digest.
    fn ctl_journaled(
        &mut self,
        op: Ulid,
        tool: &str,
        args: serde_json::Value,
        origin: crate::mcp::Origin,
        effect: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<()> {
        let origin = self.identity.bind_origin(origin);
        let device_id = self.operator_device_for_effect()?;
        let principal = crate::mcp::Principal::Operator {
            device_id,
            door: origin.door,
            issued_by: origin.issued_by,
        };
        let call = crate::mcp::Call {
            op_id: op,
            tool: tool.into(),
            args,
        };
        call.validate()?;
        let (retained, fresh) = self.operator_journal.admit(&principal, &call)?;
        let result = if fresh {
            let result = match effect(self) {
                Ok(()) => crate::mcp::success(op, serde_json::json!({"ok":true})),
                Err(error) => crate::mcp::error(op, "rejected", &error.to_string()),
            };
            self.operator_journal.complete(&principal, &call, result)?
        } else {
            retained
        };
        if result["isError"] == true {
            let message = result["structuredContent"]["error"]["message"]
                .as_str()
                .unwrap_or("outcome unavailable");
            return Err(invalid(message.to_owned()));
        }
        Ok(())
    }
    /// Input through `ctl` (#270): the window's input path, journaled as `gui_input`.
    fn ctl_input(
        &mut self,
        id: Ulid,
        generation: u64,
        input_id: Ulid,
        bytes: Vec<u8>,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        let args = serde_json::json!({"seat":id,"generation":generation,
            "bytes":bytes.len(),"digest":crate::store::digest(&bytes)});
        self.ctl_journaled(input_id, "gui_input", args, origin, |this| {
            this.seat_input(id, generation, input_id, bytes).map(drop)
        })
    }
    fn ctl_attention(
        &mut self,
        id: Ulid,
        mut edit: bbt_proto::gui::AttentionEdit,
        op: Ulid,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        edit.destination = if edit.destination == "direct" {
            format!("seats/{id}/inbox")
        } else {
            crate::messaging::attention::destination(&edit.destination)?
        };
        let args = serde_json::json!({"seat": id, "edit": edit});
        self.ctl_journaled(op, "seat_attention", args, origin, |this| {
            this.check_gui_generation(id, edit.generation)?;
            this.registry.change_attention(id, &edit)?;
            this.publish(
                &format!("seat-{id}"),
                Draft::new(
                    format!("seat/{id}/state"),
                    "attention",
                    serde_json::json!({"seat":id}),
                ),
            )
        })
    }
    /// Rename through `ctl`: the registry name, then a state event so the window refreshes
    /// the seat (ctl-cli §1a). A name is 1 to 64 printable characters.
    fn ctl_rename(
        &mut self,
        id: Ulid,
        name: String,
        op: Ulid,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        let name = name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err(invalid("a seat name is 1 to 64 printable characters"));
        }
        let args = serde_json::json!({"seat":id,"name":name});
        self.ctl_journaled(op, "seat_rename", args, origin, |this| {
            this.registry.update(id, |record| {
                record.name = name.clone();
                Ok(())
            })?;
            this.publish(
                &format!("seat-{id}"),
                Draft::new(
                    format!("seat/{id}/state"),
                    "rename",
                    serde_json::json!({"name": name}),
                ),
            )
        })
    }
    /// Resize through `ctl`: the window's resize path, journaled as `gui_resize`.
    fn ctl_resize(
        &mut self,
        id: Ulid,
        generation: u64,
        cols: u16,
        rows: u16,
        op: Ulid,
        origin: crate::mcp::Origin,
    ) -> Result<()> {
        // Under a window owner the layout owns each pane's PTY size (Oracle ruling
        // 2026-10-24): a seat a window watches is laid out by it, so ctl resizes the window.
        // A bare Session watches nothing and keeps the direct resize.
        if self
            .gui_watches
            .watched()
            .iter()
            .any(|(seat, _)| *seat == id)
        {
            return Err(invalid(
                "layout_owned: a window lays out this seat and owns its PTY size; resize the window with ctl app resize <cols>x<rows>",
            ));
        }
        let args = serde_json::json!({"seat":id,"generation":generation,"cols":cols,"rows":rows});
        self.ctl_journaled(op, "gui_resize", args, origin, |this| {
            this.seat_resize(id, generation, cols, rows)
        })
    }
    /// One seat's visible screen as text rows, trailing blanks trimmed.
    fn seat_screen(&self, id: Ulid, cells: bool) -> Result<bbt_proto::gui::SeatScreen> {
        let generation = self
            .views
            .get(&id)
            .ok_or_else(|| invalid("unknown seat"))?
            .generation;
        let snapshot = self.snapshot(id, generation)?;
        Ok(bbt_proto::gui::SeatScreen {
            seat: id,
            generation,
            rows: snapshot
                .screen
                .cells
                .iter()
                .map(|cells| row_text(cells).trim_end().to_owned())
                .collect(),
            cursor: snapshot.screen.cursor,
            modes: snapshot.screen.modes,
            cols: snapshot.screen.cols,
            sequence: snapshot.sequence,
            cursor_style: snapshot.screen.cursor_style.clone(),
            cells: cells.then(|| {
                snapshot
                    .screen
                    .cells
                    .iter()
                    .map(|row| style_runs(row))
                    .collect()
            }),
            title: snapshot.screen.title,
        })
    }
    fn snapshot(&self, id: Ulid, generation: u64) -> Result<Snapshot> {
        let _publish =
            bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::LogicViewPublish).work(1, 0);
        let state = self
            .views
            .get(&id)
            .ok_or_else(|| invalid("unknown viewport"))?;
        if state.generation != generation {
            return Err(invalid("stale process generation"));
        }
        self.record_checkpoint_state(id);
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
        if let Some(view) = self
            .views
            .get_mut(&id)
            .filter(|v| v.generation == generation)
        {
            view.captured_sequence = history.status.captured_sequence;
        }
        Ok(*history)
    }
    fn save(&mut self, id: Ulid) -> Result<()> {
        self.views.get_mut(&id).unwrap().checkpoint_requested = true;
        self.prepare_checkpoint(id, false)
    }
    fn prepare_checkpoint(&mut self, id: Ulid, idle: bool) -> Result<()> {
        let view = &self.views[&id];
        if view.sequence == view.checkpoint_sequence {
            self.views.get_mut(&id).unwrap().checkpoint_requested = false;
            return Ok(());
        }
        if view.checkpoint_retry_at > Instant::now() {
            return Ok(());
        }
        if self.checkpoint_saves.contains(id, view.generation) {
            return Ok(());
        }
        let cursor = CheckpointCursor {
            seat: id,
            generation: view.generation,
            sequence: view.sequence,
            byte_end: view.byte_end,
        };
        let Some(token) = self.checkpoint_saves.start(cursor) else {
            return Ok(());
        };
        self.checkpoint_images.insert(
            token,
            CheckpointImage {
                cursor,
                saved: view.saved(),
                attachment: self.attachments[&id].clone(),
                idle,
            },
        );
        let view = self.views.get_mut(&id).unwrap();
        view.checkpoint_pending = Some(cursor);
        view.changed_at.submitted();
        view.checkpoint_requested = false;
        Ok(())
    }

    fn poll_checkpoint(&mut self) -> Result<()> {
        let Some(receiver) = &self.checkpoint_completion else {
            return Ok(());
        };
        let (token, cursor, idle, result) = match receiver.try_recv() {
            Ok(done) => done,
            Err(std::sync::mpsc::TryRecvError::Empty) => return Ok(()),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Err(invalid(
                    "checkpoint side task stopped; publication outcome unknown",
                ));
            }
        };
        self.checkpoint_completion = None;
        let confirmed = match &result {
            Ok(CheckpointResult::Committed(sequence)) => Some(*sequence),
            _ => None,
        };
        let committed = self.checkpoint_saves.complete(token, confirmed);
        let Some(view) = self
            .views
            .get_mut(&cursor.seat)
            .filter(|v| v.generation == cursor.generation && v.checkpoint_pending == Some(cursor))
        else {
            return Ok(());
        };
        view.checkpoint_pending = None;
        let was_paused = view.checkpoint_paused;
        match result {
            Ok(CheckpointResult::Committed(_)) => {
                let committed =
                    committed.ok_or_else(|| invalid("checkpoint reply cursor mismatch"))?;
                view.checkpoint_sequence = committed.sequence;
                view.checkpoint_bytes = committed.byte_end;
                view.idle_checkpoints += u64::from(idle);
                view.forced_checkpoints += u64::from(!idle);
                self.checkpoint_progress += 1;
            }
            Ok(CheckpointResult::NotIdle(tail)) => {
                view.captured_sequence = view.captured_sequence.max(tail);
                view.last_arrival_at = Instant::now();
            }
            Err(error)
                if is_capture_busy(&error) || crate::provider::fake::is_link_down(&error) =>
            {
                view.checkpoint_requested = true;
                view.checkpoint_retry_at = Instant::now() + Duration::from_millis(200);
            }
            Err(error) => return Err(error),
        }
        view.changed_at.completed(
            committed.is_some(),
            view.byte_end > view.checkpoint_bytes
                || view.captured_sequence > view.checkpoint_sequence,
        );
        if view.byte_end.saturating_sub(view.checkpoint_bytes)
            < self.checkpoint_policy.recovery_bytes * 2
            && !view.checkpoint_requested
        {
            view.checkpoint_paused = false;
        }
        let changed_pause = was_paused != view.checkpoint_paused;
        self.record_checkpoint_state(cursor.seat);
        if changed_pause {
            self.publish_adapter(cursor.seat, self.views[&cursor.seat].adapter.status.clone())?;
        }
        Ok(())
    }

    fn service_checkpoints(&mut self, dispatch: bool) -> Result<()> {
        self.poll_checkpoint()?;
        // A fixed scan quantum, not a full synchronous checkpoint sweep.
        let ids: Vec<_> = self.views.keys().copied().collect();
        let start = self
            .checkpoint_cursor
            .and_then(|old| ids.iter().position(|id| *id > old))
            .unwrap_or(0);
        for id in ids
            .iter()
            .cycle()
            .skip(start)
            .take(ids.len().min(32))
            .copied()
        {
            self.checkpoint_cursor = Some(id);
            let view = &self.views[&id];
            let dirty = view.sequence != view.checkpoint_sequence;
            let force = view.checkpoint_requested
                || (view.checkpoint_pending.is_none()
                    && dirty
                    && (view.byte_end.saturating_sub(view.checkpoint_bytes)
                        >= self.checkpoint_policy.force_bytes
                        || view
                            .changed_at
                            .expired(Instant::now(), self.checkpoint_policy)));
            let idle = dirty
                && view.sequence == view.captured_sequence
                && self
                    .checkpoint_policy
                    .idle_due(view.last_arrival_at, Instant::now());
            if force || idle {
                self.prepare_checkpoint(id, !force)?;
            }
        }
        if !dispatch || self.checkpoint_saves.active() {
            return Ok(());
        }
        let Some((token, cursor)) = self.checkpoint_saves.dispatch() else {
            return Ok(());
        };
        let image = self.checkpoint_images.remove(&token).unwrap();
        debug_assert_eq!(image.cursor, cursor);
        let link = self.connections.link(&self.host_for_seat(cursor.seat)?)?;
        let recorder = self.client.capture_diagnostics.clone();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        self.checkpoint_completion = Some(rx);
        let owner = std::thread::current();
        let _task = self.handle.spawn_blocking(move || {
            let span = recorder.start(
                Phase::Checkpoint,
                cursor.seat,
                cursor.generation,
                0,
                cursor.sequence,
            );
            let result = (|| {
                let bytes = image.saved.encode()?;
                if image.idle {
                    let recovery = link
                        .lock()
                        .map_err(|_| invalid("supervisor link owner stopped"))?
                        .request(Action::RingRecovery {
                            attachment: image.attachment.clone(),
                        })?;
                    let Reply::RingRecovery { status, .. } = recovery else {
                        return Err(invalid("idle checkpoint recovery reply mismatch"));
                    };
                    if status.captured_sequence != cursor.sequence {
                        return Ok(CheckpointResult::NotIdle(status.captured_sequence));
                    }
                }
                let reply = retry_capture(|| {
                    let rpc = recorder.start(
                        Phase::CheckpointRpc,
                        cursor.seat,
                        cursor.generation,
                        0,
                        cursor.sequence,
                    );
                    let result = link
                        .lock()
                        .map_err(|_| invalid("supervisor link owner stopped"))?
                        .request(Action::PublishCheckpoint {
                            attachment: image.attachment.clone(),
                            checkpoint: bytes.clone(),
                        });
                    rpc.finish(capture_outcome(&result), 0, 0);
                    result
                })?;
                match reply {
                    Reply::CheckpointCommitted { sequence } => {
                        Ok(CheckpointResult::Committed(sequence))
                    }
                    _ => Err(invalid("checkpoint publication reply mismatch")),
                }
            })();
            span.finish(capture_outcome(&result), 0, 0);
            let _ = tx.send((token, cursor, image.idle, result));
            owner.unpark();
        });
        Ok(())
    }

    fn record_checkpoint_state(&self, id: Ulid) {
        let view = &self.views[&id];
        self.client
            .capture_diagnostics
            .seat(id, view.generation, |row, now| {
                row.progress(view.sequence, now);
                row.checkpoint_sequence = view.checkpoint_sequence;
                row.checkpoint_byte_end = view.checkpoint_bytes;
                row.parsed_byte_end = view.byte_end;
                row.checkpoint_pauses = view.checkpoint_pauses;
                row.checkpoint_paused = view.checkpoint_paused;
                row.idle_checkpoints = view.idle_checkpoints;
                row.forced_checkpoints = view.forced_checkpoints;
            });
    }

    fn allow_record(&mut self, id: Ulid, event: &Event) -> Result<bool> {
        let bytes = match event {
            Event::Output { bytes, .. } => bytes.len() as u64,
            _ => 0,
        };
        let view = &self.views[&id];
        let debt = view.byte_end.saturating_sub(view.checkpoint_bytes);
        let prepared = view
            .checkpoint_pending
            .map_or(view.checkpoint_bytes, |c| c.byte_end);
        let interval = view.byte_end.saturating_sub(prepared);
        let pause = debt >= self.checkpoint_policy.recovery_bytes * 2
            || debt.saturating_add(bytes) > self.checkpoint_policy.recovery_bytes * 2
            || (interval > 0
                && interval.saturating_add(bytes) > self.checkpoint_policy.recovery_bytes)
            || view.checkpoint_requested;
        if pause {
            self.save(id)?;
        }
        let view = self.views.get_mut(&id).unwrap();
        if view.checkpoint_paused != pause {
            view.checkpoint_paused = pause;
            view.checkpoint_pauses += u64::from(pause);
            self.record_checkpoint_state(id);
            self.publish_adapter(id, self.views[&id].adapter.status.clone())?;
        }
        Ok(!pause)
    }

    fn parsed_bytes(&mut self, id: Ulid, previous_end: u64) {
        let view = self.views.get_mut(&id).unwrap();
        // Input-context recency is separate from the oldest dirty-byte age.
        view.last_parsed_at = Instant::now();
        if view.byte_end > previous_end {
            let now = Instant::now();
            view.changed_at.arrived(now);
        }
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
        if let Event::Output { bytes, .. } = &event {
            bbt_rt::diagnostics::probe_live_output(id.0, generation, seq, bytes);
        }
        if !self.allow_record(id, &event)? {
            return Ok(false);
        }
        let view = self.views.get_mut(&id).unwrap();
        let previous_end = view.byte_end;
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
        if let Event::Resized { size, .. } = &event {
            check_size(size.cols as usize, size.rows as usize)?;
            view.view_generation = Ulid::new();
        }
        self.parsed_bytes(id, previous_end);
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
            if self.restarts.get(&id) == Some(&generation) {
                self.restarts.remove(&id);
                if let Err(error) = self
                    .registry
                    .get(id)
                    .and_then(|record| self.relaunch(record))
                {
                    eprintln!("seat {id} restart failed after its exit: {error}");
                }
            }
        }
        if !self.checkpoint_saves.active() {
            self.flush_protocol_replies(id)?;
        }
        let view = &self.views[&id];
        if (view.checkpoint_pending.is_none()
            && view.byte_end.saturating_sub(view.checkpoint_bytes)
                >= self.checkpoint_policy.force_bytes)
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
        let result = self.catch_up_page(id, through);
        // Check every return, including Busy, a held stream, and a page that
        // made no progress. Parsing is not required for an old byte to age.
        let view = &self.views[&id];
        if view.checkpoint_pending.is_none()
            && view.sequence != view.checkpoint_sequence
            && (view.byte_end.saturating_sub(view.checkpoint_bytes)
                >= self.checkpoint_policy.force_bytes
                || view
                    .changed_at
                    .expired(Instant::now(), self.checkpoint_policy))
        {
            self.save(id)?;
        }
        result
    }
    fn ring_read(&self, id: Ulid, through: u64) -> Option<bbt_supervisor::protocol::RingRead> {
        let view = self.views.get(&id)?;
        if view.sequence >= through
            || view.ring_refusal.is_some()
            || (view.checkpoint_paused
                && view.checkpoint_requested
                && view.checkpoint_pending.is_none())
        {
            return None;
        }
        Some(bbt_supervisor::protocol::RingRead {
            attachment: self.attachments.get(&id)?.clone(),
            from: view.sequence + 1,
            to: through,
        })
    }
    fn prefetch_ring(
        &mut self,
        key: &(HostId, Ulid),
        event: &Event,
        pending: &PendingSeats,
    ) -> Result<()> {
        use bbt_supervisor::protocol::{
            MAX_RING_BATCH_BYTES, MAX_RING_BATCH_MEMBERS, RingReadResult,
        };
        let through = event.sequence().unwrap().saturating_sub(1);
        let Some(selected) = self.ring_read(key.1, through) else {
            return Ok(());
        };
        if self
            .ring_pages
            .get(&key.0)
            .and_then(|pages| pages.get(&key.1))
            .is_some_and(|page| page.can_serve(&selected))
        {
            return Ok(());
        }
        let mut reads = Vec::with_capacity(MAX_RING_BATCH_MEMBERS);
        let mut retained = BTreeMap::new();
        let ids = std::iter::once(key.1)
            .chain(pending.ring_candidates(&key.0, MAX_RING_BATCH_MEMBERS - 1));
        for id in ids {
            let Some(slot) = pending.seats.get(&(key.0.clone(), id)) else {
                continue;
            };
            let Some(view) = self.views.get(&id) else {
                continue;
            };
            if view.generation != slot.generation {
                continue;
            }
            let Some(read) = self.ring_read(id, slot.active.sequence().unwrap().saturating_sub(1))
            else {
                continue;
            };
            if let Some(old) = self.ring_pages.get(&key.0).and_then(|pages| pages.get(&id))
                && old.can_serve(&read)
            {
                retained.insert(id, old.clone());
            } else {
                reads.push(read);
            }
        }
        if reads.is_empty() {
            self.ring_pages.insert(key.0.clone(), retained);
            return Ok(());
        }
        let link = self.connections.link(&key.0)?;
        let mut connection = match link.try_lock() {
            Ok(connection) => connection,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(()),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(invalid("supervisor link owner stopped"));
            }
        };
        let rpc = self.client.capture_diagnostics.start(
            Phase::ReadRingRpc,
            key.1,
            selected.attachment.generation,
            selected.from,
            selected.to,
        );
        let measurement = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::ReadRingRequest);
        let reply = connection.request(Action::ReadRingBatch {
            reads: reads.clone(),
            max_bytes: MAX_RING_BATCH_BYTES as u32,
        });
        drop(connection);
        rpc.finish(capture_outcome(&reply), 0, 0);
        let reply = match reply {
            Err(error) if is_capture_busy(&error) => {
                measurement.finish(1, 0);
                return Ok(());
            }
            reply => reply?,
        };
        let Reply::RingHistoryBatch { results } = reply else {
            return Err(invalid("ring batch reply mismatch"));
        };
        if results.len() != reads.len() {
            return Err(invalid("ring batch member count mismatch"));
        }
        let mut expected: BTreeMap<_, _> = reads
            .into_iter()
            .map(|read| {
                (
                    (
                        read.attachment.seat,
                        read.attachment.generation,
                        read.from,
                        read.to,
                    ),
                    read,
                )
            })
            .collect();
        let mut bytes = 0;
        for RingReadResult {
            seat,
            generation,
            from,
            to,
            mut result,
        } in results
        {
            let Some(read) = expected.remove(&(seat, generation, from, to)) else {
                return Err(invalid("ring batch member identity mismatch"));
            };
            if let Ok(history) = &result {
                let mut next = from;
                let valid = history.status.generation == generation
                    && history.gap.is_none()
                    && !history.records.is_empty()
                    && history.records.iter().all(|record| {
                        let current = record.event.sequence();
                        let record_generation = match &record.event {
                            Event::Output { generation, .. }
                            | Event::Resized { generation, .. }
                            | Event::Exited { generation, .. } => *generation,
                        };
                        let okay = record.event.seat() == seat
                            && record_generation == generation
                            && current == Some(next)
                            && next <= to
                            && next <= history.status.captured_sequence;
                        next = next.saturating_add(1);
                        okay
                    });
                if !valid {
                    result = Err(crate::Failure {
                        code: crate::FailureCode::Invalid,
                        message: "ring batch history continuity mismatch".into(),
                    });
                }
            }
            if let Ok(history) = &result {
                bytes += history
                    .records
                    .iter()
                    .map(|r| match &r.event {
                        Event::Output { bytes, .. } => bytes.len() as u64,
                        _ => 0,
                    })
                    .sum::<u64>();
            }
            retained.insert(seat, RingPage { read, result });
        }
        measurement.finish(1, bytes);
        self.ring_pages.insert(key.0.clone(), retained);
        Ok(())
    }
    fn replay_page(&mut self, id: Ulid, through: u64) -> Result<Option<Box<crate::ring::History>>> {
        let Some(read) = self.ring_read(id, through) else {
            return Ok(None);
        };
        let host = self.host_for_seat(id)?;
        let result = self
            .ring_pages
            .get(&host)
            .and_then(|pages| pages.get(&id))
            .and_then(|page| page.page(&read));
        match result {
            None => Ok(None),
            Some(Ok(history)) => Ok(Some(history)),
            Some(Err(error)) => {
                if let Some(pages) = self.ring_pages.get_mut(&host) {
                    pages.remove(&id);
                }
                if error.code != crate::FailureCode::Busy {
                    eprintln!(
                        "ring read refused: seat={id} generation={} code={:?}",
                        read.attachment.generation, error.code
                    );
                    self.views.get_mut(&id).unwrap().ring_refusal = Some(error);
                }
                Ok(None)
            }
        }
    }

    fn catch_up_page(&mut self, id: Ulid, through: u64) -> Result<bool> {
        let current = &self.views[&id];
        if current.sequence >= through {
            return Ok(true);
        }
        let generation = current.generation;
        let from = current.sequence + 1;
        if current.checkpoint_paused
            && current.checkpoint_requested
            && current.checkpoint_pending.is_none()
        {
            return Ok(false);
        }
        let page =
            self.client
                .capture_diagnostics
                .start(Phase::ReplayPage, id, generation, from, through);
        let Some(history) = self.replay_page(id, through)? else {
            page.finish(Outcome::Busy, 0, 0);
            return Ok(false);
        };
        let view = self.views.get_mut(&id).unwrap();
        if history.status.captured_sequence > view.captured_sequence
            && history.status.byte_end > view.byte_end
        {
            view.last_arrival_at = Instant::now();
            view.changed_at.arrived(view.last_arrival_at);
        }
        view.captured_sequence = view.captured_sequence.max(history.status.captured_sequence);
        self.client
            .capture_diagnostics
            .seat(id, generation, |row, now| {
                row.ring_observed_ns = now;
                row.captured_sequence = history.status.captured_sequence;
                row.committed_sequence = history.status.committed_sequence;
            });
        if history.gap.is_some() || history.records.is_empty() {
            return Err(invalid("capture recovery gap"));
        }
        let mut events = 0;
        let mut bytes = 0;
        let parse = self.client.capture_diagnostics.start(
            Phase::Parse,
            id,
            generation,
            self.views[&id].sequence + 1,
            through,
        );
        for record in history.records {
            if let Event::Output { bytes, .. } = &record.event {
                bbt_rt::diagnostics::probe_ring_output(
                    id.0,
                    generation,
                    record.event.sequence().unwrap_or(0),
                    bytes,
                );
            }
            if !self.allow_record(id, &record.event)? {
                break;
            }
            let view = self.views.get_mut(&id).unwrap();
            let previous_end = view.byte_end;
            if matches!(record.event, Event::Resized { .. }) {
                view.view_generation = Ulid::new();
            }
            let decode =
                bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::ReadRingTerminalDecode);
            view.state.apply(&record.event)?;
            decode.finish(
                1,
                match &record.event {
                    Event::Output { bytes, .. } => bytes.len() as u64,
                    _ => 0,
                },
            );
            if let Event::Output { bytes: output, .. } = &record.event {
                bytes += output.len() as u64;
            }
            // Recovery must not replay probe responses or hook/native effects.
            let _ = view.tap.drain();
            events += 1;
            let boundary = !matches!(record.event, Event::Output { .. });
            self.parsed_bytes(id, previous_end);
            let view = &self.views[&id];
            if boundary
                || (view.checkpoint_pending.is_none()
                    && (view.byte_end.saturating_sub(view.checkpoint_bytes)
                        >= self.checkpoint_policy.force_bytes
                        || view
                            .changed_at
                            .expired(Instant::now(), self.checkpoint_policy)))
            {
                self.save(id)?;
            }
            if boundary {
                break;
            }
        }
        parse.finish(Outcome::Ok, events, bytes);
        let view = &self.views[&id];
        self.client
            .capture_diagnostics
            .seat(id, generation, |row, now| {
                row.progress(view.sequence, now);
                row.target_sequence = through;
                row.pages += 1;
                row.parsed_events += events;
                row.parsed_bytes += bytes;
            });
        self.record_checkpoint_state(id);
        let view = &self.views[&id];
        if view.sequence >= through {
            // An output notification's target is not a forced checkpoint
            // boundary. R/age above and caught-up idle service own that policy;
            // lifecycle records and #11 readiness still force their images.
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
                if view.shared_codex_runtime.is_some()
                    && view.adapter.binding.runtime.is_some()
                    && view.native_reader.as_ref().is_some_and(|reader| {
                        reader.binding == view.adapter.binding && reader.task.is_finished()
                    })
                {
                    // The daemon owns its lifecycle. Re-pin a replacement peer
                    // through the same exact thread; never restart/stop it here.
                    let mut binding = view.adapter.binding.clone();
                    binding.runtime = None;
                    binding.attachment_generation = binding
                        .attachment_generation
                        .checked_add(1)
                        .ok_or_else(|| invalid("native attachment generation exhausted"))?;
                    view.adapter.rebind(binding)?;
                    view.delivery_endpoint = crate::adapters::Endpoint::None;
                }
                if view.adapter.record.kind == "codex"
                    && !view.adapter.binding.session.starts_with("process-")
                    && view.adapter.binding.runtime.is_none()
                    && view.native_reader.as_ref().is_none_or(|reader| {
                        reader.binding != view.adapter.binding || reader.task.is_finished()
                    })
                    && let Some(path) = view.shared_codex_runtime.clone()
                {
                    let binding = view.adapter.binding.clone();
                    let reader_binding = binding.clone();
                    let client = self.client.clone();
                    let task = self.handle.spawn(async move {
                        let result = crate::adapters::native::shared_codex_runtime(&binding, &path)
                            .await.map_err(|error| crate::adapters::observe::short(
                                &format!("shared Codex runtime binding failed: {error}; no mutation attempted"), 512));
                        if result.is_err() { tokio::time::sleep(Duration::from_millis(200)).await; }
                        let _ = client.shared_codex_discovery(SharedCodexDiscovery { binding, result }).await;
                    });
                    view.native_reader = Some(NativeReader {
                        binding: reader_binding,
                        task,
                    });
                }
                if view.adapter.record.kind == "codex"
                    && view.adapter.binding.session.starts_with("process-")
                    && view.native_reader.is_none()
                    && view.adapter.binding.runtime.is_some()
                {
                    let binding = view.adapter.binding.clone();
                    let reader_binding = binding.clone();
                    let client = self.client.clone();
                    let task = self.handle.spawn(async move {
                        loop {
                            match crate::adapters::native::discover_codex_thread(&binding).await {
                                Ok(None) => tokio::time::sleep(Duration::from_millis(200)).await,
                                result => {
                                    let result = result
                                        .and_then(|thread| thread.ok_or_else(|| invalid("native thread not loaded")))
                                        .map_err(|error| crate::adapters::observe::short(
                                            &format!("native bootstrap failed: {error}; no mutation attempted"), 512,
                                        ));
                                    let _ = client.codex_discovery(CodexDiscovery { binding, result }).await;
                                    break;
                                }
                            }
                        }
                    });
                    view.native_reader = Some(NativeReader {
                        binding: reader_binding,
                        task,
                    });
                }
                if view.adapter.record.kind == "codex"
                    && !view.adapter.binding.session.starts_with("process-")
                    && view
                        .native_reader
                        .as_ref()
                        .is_none_or(|r| r.binding != view.adapter.binding || r.task.is_finished())
                    && let Some(runtime) = &view.adapter.binding.runtime
                {
                    let path = runtime.endpoint.clone();
                    let session = view.adapter.binding.session.clone();
                    view.delivery_endpoint = crate::adapters::Endpoint::Codex {
                        path: path.clone(),
                        thread: session.clone(),
                        turn: None,
                    };
                    let thread = session.clone();
                    let generation = view.generation;
                    let client = self.client.clone();
                    let binding = view.adapter.binding.clone();
                    let reader_binding = binding.clone();
                    let diagnostics_root = self.config.run_dir.clone();
                    let observe_hooks = view.shared_codex_runtime.is_none();
                    let task = self.handle.spawn(async move {
                        let mut observer = if !observe_hooks { None } else { match crate::adapters::native::codex_hook_observer(&binding).await {
                            Ok(observer) => Some(observer),
                            Err(error) => {
                                bbt_rt::error_journal::report_to(&diagnostics_root, bbt_rt::error_journal::Failure::new("native", "hook_observer", error).seat(id).harness("codex"));
                                None
                            }
                        }};
                        let mut recent = VecDeque::<String>::new();
                        let mut tick = tokio::time::interval(Duration::from_millis(200));
                        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                        loop {
                            if let Some(reader) = &mut observer {
                                tokio::select! {
                                    failure = reader.next_failure() => {
                                        match failure {
                                            Ok(failure) => {
                                                let run = failure.id.clone();
                                                if !recent.contains(&run) && client.native_hook_failure(binding.clone(), failure) {
                                                    if recent.len() == 64 { recent.pop_front(); }
                                                    recent.push_back(run);
                                                }
                                            }
                                            Err(error) => {
                                                bbt_rt::error_journal::report_to(&diagnostics_root, bbt_rt::error_journal::Failure::new("native", "hook_observer_closed", error).seat(id).harness("codex"));
                                                observer = None;
                                            }
                                        }
                                        continue;
                                    }
                                    _ = tick.tick() => {}
                                }
                            } else { tick.tick().await; }
                            // No blind rejoin retries: existing status observations remain live.
                            if let Ok(status) = crate::adapters::native::codex_owned_status(&binding).await {
                                let event = serde_json::json!({"source":"snapshot","method":"thread/status/changed","params":{"threadId":thread,"status":status}});
                                if client.native_event(id, generation, event).await.is_err() { break; }
                            } else if !observe_hooks { break; }
                        }
                    });
                    view.native_reader = Some(NativeReader {
                        binding: reader_binding,
                        task,
                    });
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
        self.service_identity_jobs()?;
        if let Ok(revision) = self.knowledge_host.revision()
            && revision != self.knowledge_seen
        {
            // One coalesced view invalidation; root authority lives in the account,
            // and this notice never enters the messaging wake/delivery path.
            let event = Draft::new(
                "activity/knowledge-provider",
                "logic",
                serde_json::json!({"kind":"knowledge_provider_changed","revision":revision,"title":"Knowledge provider changed"}),
            );
            if self.publish("index", event).is_ok() {
                self.knowledge_seen = revision;
            }
        }
        while let Some(result) = self.knowledge_jobs.try_join_next() {
            match result {
                Ok(done) => {
                    let current = self
                        .control_actor(done.actor.seat_id, done.actor.process_generation)
                        .and_then(|(actor, _)| {
                            if actor == done.actor {
                                Ok(())
                            } else {
                                Err(invalid("seat authority changed during knowledge read"))
                            }
                        })
                        .and_then(|_| {
                            self.knowledge_host
                                .current(done.revision)
                                .map_err(|e| invalid(e.to_string()))
                        });
                    let _ = done.reply.send(current.and(done.result));
                }
                Err(_) => eprintln!("knowledge read worker interrupted"),
            }
        }
        while let Some(result) = self.plugin_jobs.try_join_next() {
            if result.is_err() {
                eprintln!("plugin task interrupted; admitted operation remains unknown");
            }
        }
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
            let mut value = delivery.result;
            if delivery.call.tool == "seat_focus" && value["isError"] == false {
                let same_actor = self
                    .control_actor(delivery.actor.seat_id, delivery.actor.process_generation)
                    .is_ok_and(|(actor, _)| actor == delivery.actor);
                let same_target = self.registry.get(delivery.target.seat).is_ok_and(|r| {
                    r.generation == delivery.target.generation
                        && r.swarm == delivery.actor.swarm_id
                        && r.lifecycle == Lifecycle::Open
                });
                if !same_actor || !same_target {
                    value = crate::mcp::error(
                        delivery.call.op_id,
                        "unknown",
                        "focus binding changed while producing the frame",
                    );
                }
            }
            let result = delivery
                .journal
                .complete(&delivery.actor, &delivery.call, value);
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
                    let harness = view.adapter.record.kind.clone();
                    let endpoint = view.delivery_endpoint.clone();
                    let delivery_failure = view.delivery_failure.clone();
                    let priority = if work.messages.iter().all(|m| m.view().priority > 0) {
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
                            crate::messaging::wake_cost::note(
                                &work, &binding, &harness, &outcome, now_ms(),
                            );
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
                    let binding = view.adapter.binding.clone();
                    let harness = view.adapter.record.kind.clone();
                    let outcome = self.deliver_pty(seat, &work, &text)?;
                    crate::messaging::wake_cost::note(
                        &work,
                        &binding,
                        &harness,
                        &outcome,
                        now_ms(),
                    );
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
            crate::adapters::mapping::literal(work.id, text, true),
        )
    }
    fn guarded_input(
        &mut self,
        seat: Ulid,
        generation: u64,
        input: crate::adapters::guard::LiteralInput<'_>,
    ) -> Result<crate::adapters::WakeOutcome> {
        use crate::adapters::{Evidence, WakeOutcome};
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
                .readiness_observed_ms()
                .is_none_or(|observed_ms| observed_ms <= context.last_input_ms)
            || now.saturating_sub(context.last_input_ms) < 500
        {
            return Ok(pending(if view.adapter.readiness_observed_ms().is_none() {
                "no positive composer-ready indication"
            } else {
                "pending for operator: fresh readiness after the last input and a caught-up PTY are required"
            }));
        }
        let arbiter = crate::adapters::mapping::arbiter(
            view.adapter.binding.clone(),
            &context,
            view.adapter.status.turn_open,
            view.state.term.sync_pending(),
            view.last_parsed_at.elapsed() < Duration::from_millis(50),
            view.state
                .term
                .mode()
                .contains(bbt_core::term::TermMode::BRACKETED_PASTE),
        );
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
    fn refresh_identity_due(
        &mut self,
        last: &mut Instant,
        clock: impl Fn() -> Instant,
    ) -> Result<bool> {
        const INTERVAL: Duration = Duration::from_secs(30);
        if clock().duration_since(*last) < INTERVAL {
            return Ok(false);
        }
        let phase =
            self.client
                .capture_diagnostics
                .start(Phase::MaintenanceIdentity, Ulid::nil(), 0, 0, 0);
        self.refresh_identity()?;
        phase.finish(Outcome::Ok, 0, 0);
        *last = clock();
        Ok(true)
    }
    fn refresh_identity(&mut self) -> Result<()> {
        let reopening_failed = self.identity.reopening_failed();
        self.identity.retry_custody()?;
        if reopening_failed && !self.identity.reopening_failed() {
            // The existing completion outbox can now sign; replay only the
            // noninteractive recovery jobs, never the native attempt.
            self.identity_seen.clear();
        }
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
        let device = self.identity.observed_operator_device()?;
        self.messaging.operator_reference(device)?;
        let authority = self.identity.operator_signing().ok();
        self.messaging.resume_signing_with_operator(
            self.identity.keys(),
            authority.as_ref(),
            now_ms(),
        )?;
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
            reason: match (self.identity.issued_by(), status.reason_label) {
                (Some(label), Some(reason)) => Some(format!("{label} · {reason}")),
                (Some(label), None) => Some(label.into()),
                (None, reason) => reason,
            },
        })
    }
    fn gui_device(&mut self) -> Result<bbt_proto::messaging::OperatorReference> {
        self.messaging
            .operator_reference(self.identity.observed_operator_device()?)
    }
    fn gui_attention(
        &mut self,
        id: Ulid,
        generation: Option<u64>,
    ) -> Result<crate::seat_socket::GuiSeat> {
        if let Some(generation) = generation {
            self.check_gui_generation(id, generation)?;
        }
        let mut seat = self.gui_seat(id)?;
        seat.attention = Some(self.messaging.attention_view(&seat.record)?);
        Ok(seat)
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
        let mut title = if !view.title.is_empty() {
            view.title.clone()
        } else {
            status
                .detail
                .clone()
                .unwrap_or_else(|| view.adapter.record.name.clone())
        };
        if view.checkpoint_paused {
            title.push_str(" · Replay paused for checkpoint");
        }
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
            attention: self
                .messaging
                .deferred_view(&record)
                .unwrap_or_else(|error| {
                    Some(serde_json::json!({"deferred_error":error.to_string()}))
                }),
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
        origin: crate::mcp::Origin,
    ) -> Result<crate::messaging::api::Response> {
        let origin = self.identity.bind_origin(origin);
        let reference = self.gui_device()?;
        let device = self.identity.observed_operator_device()?;
        let actor = match reference.kind {
            bbt_proto::messaging::OperatorReferenceKind::DeviceDerived => {
                crate::mcp::Actor::Operator {
                    device_id: reference.id,
                    door: origin.door,
                    issued_by: origin.issued_by.clone(),
                }
            }
            bbt_proto::messaging::OperatorReferenceKind::OperatorLocal => {
                crate::mcp::Actor::OperatorLocal {
                    reference,
                    door: origin.door,
                    issued_by: origin.issued_by.clone(),
                }
            }
        };
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
        let mut authority = self.identity.operator_signing().ok();
        if let Some(authority) = &mut authority {
            authority.origin = origin.clone();
            if Some(authority.device_id) != device {
                return Err(invalid("held: operator device changed"));
            }
        } else if !matches!(
            &request,
            crate::messaging::api::Request::Send { .. }
                | crate::messaging::api::Request::List { .. }
                | crate::messaging::api::Request::Receipts { .. }
                | crate::messaging::api::Request::Acknowledge { .. }
                | crate::messaging::api::Request::Ls { .. }
        ) {
            return Err(invalid(
                "held: this operator mutation requires signing custody",
            ));
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
        let dispatch = || match match authority.as_ref() {
            Some(authority) => {
                self.messaging
                    .request_operator(authority, swarm, request, mute_state, now_ms())
            }
            None => {
                self.messaging
                    .request_operator_pending(device, &origin, swarm, request, now_ms())
            }
        } {
            Ok(crate::messaging::api::Response::Refused { error }) => {
                crate::mcp::messaging::refused(id, error)
            }
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
        messaging_response(result)
    }
    fn gui_task(
        &mut self,
        id: Ulid,
        request: bbt_proto::messaging::TaskRequest,
        origin: crate::mcp::Origin,
    ) -> Result<bbt_proto::messaging::TaskResponse> {
        let origin = self.identity.bind_origin(origin);
        let device_id = self.operator_device_for_effect()?;
        let actor = crate::mcp::Actor::Operator {
            device_id,
            door: origin.door,
            issued_by: origin.issued_by.clone(),
        };
        let call = crate::mcp::work_items::call(id, request).map_err(invalid)?;
        call.validate()?;
        if crate::mcp::work_items::mutation(&call.tool) {
            self.identity.operator_signing()?;
        }
        let journal = self.operator_journal.clone();
        let result = self.work_item_call(&actor, &call, &journal)?;
        crate::mcp::work_items::response(&call.tool, result).map_err(invalid)
    }
    /// `ctl record write` and `ctl task create` (e2e acceptance §2, #266 addendum): the
    /// operator's write of an `e2e` record or a new open task, signed by the operator device
    /// under its issuer and journaled like every operator mutation. Other record types and
    /// task edits keep their seat and task_* paths.
    fn gui_record_write(
        &mut self,
        id: Ulid,
        args: serde_json::Value,
        origin: crate::mcp::Origin,
    ) -> crate::knowledge::data_source::Result<serde_json::Value> {
        use crate::{
            knowledge::data_source as d,
            mcp::{self, knowledge as tool},
        };
        let origin = self.identity.bind_origin(origin);
        let device_id = self
            .identity
            .status()
            .map_err(|e| d::CommandError::new("held", e.to_string()))?
            .device_id
            .ok_or_else(|| {
                d::CommandError::new("held", "held: operator device custody is unavailable")
            })?;
        let actor = crate::mcp::Actor::Operator {
            device_id,
            door: origin.door,
            issued_by: origin.issued_by.clone(),
        };
        let call = crate::mcp::Call {
            op_id: id,
            tool: "knowledge_write".into(),
            args,
        };
        call.validate()
            .map_err(|e| d::CommandError::new("rejected", e.to_string()))?;
        let journal = self.operator_journal.clone();
        let value = match tool::prepare(&call) {
            Ok(tool::Request::Write(request)) => journal.effect(&actor, &call, || {
                self.operator_record_write(&call, *request)
            }),
            Ok(_) => unreachable!("knowledge_write prepares a write"),
            Err(reason) => journal.checked(&actor, &call, mcp::error(id, "rejected", &reason)),
        }
        .map_err(|e| d::CommandError::new("unknown", e.to_string()))?;
        if value["isError"] == true {
            let error = &value["structuredContent"]["error"];
            return Err(d::CommandError::new(
                error["code"].as_str().unwrap_or("unknown"),
                error["message"].as_str().unwrap_or("record write refused"),
            ));
        }
        Ok(value["structuredContent"].clone())
    }
    fn operator_record_write(
        &mut self,
        call: &crate::mcp::Call,
        mut request: crate::knowledge::data_source::WriteRequest,
    ) -> serde_json::Value {
        use crate::{
            knowledge::data_source as d,
            mcp::{self, knowledge as tool},
        };
        if self.knowledge_host.external() {
            return mcp::error(
                call.op_id,
                "signed_writer_unavailable",
                "Select bboxy450-term-data_source-bboxy in Settings > Knowledge to write until the host signed-writer bridge is available.",
            );
        }
        let task = request.frontmatter["type"] == "task";
        if !task && request.frontmatter["type"] != "e2e" {
            return mcp::error(
                call.op_id,
                "knowledge_write_denied",
                "operator record write covers e2e records and new tasks",
            );
        }
        let signing = match self.identity.operator_signing() {
            Ok(signing) => signing,
            Err(error) => {
                return mcp::error(
                    call.op_id,
                    "held",
                    &format!("knowledge signing held: {error}"),
                );
            }
        };
        if task && let Err(error) = tool::operator_task(&mut request, signing.device_id) {
            return mcp::error(call.op_id, &error.code, &error.message);
        }
        let context = crate::knowledge::WriteContext::operator(
            signing.device_id,
            signing.issuer.kind,
            &signing.key,
        );
        let source = BboxySource::new(&self.index);
        match request.reference.clone() {
            Some(reference) => match source.fetch(d::FetchRequest { reference }) {
                Ok(record) if record.frontmatter["type"] != "e2e" => {
                    return mcp::error(
                        call.op_id,
                        "type_immutable",
                        "an update must retain the current record type",
                    );
                }
                Ok(record)
                    if request
                        .etag
                        .as_ref()
                        .is_some_and(|etag| etag != &record.etag) =>
                {
                    return mcp::error(
                        call.op_id,
                        "etag_mismatch",
                        "etag does not match the current record",
                    );
                }
                Ok(_) => (),
                Err(error) => {
                    return mcp::error(
                        call.op_id,
                        &error.code,
                        "knowledge update reference is unavailable or stale",
                    );
                }
            },
            None => match source.resolve(d::ResolveRequest {
                locator: call.op_id.to_string(),
            }) {
                Err(error) if error.code == "not_found" => (),
                _ => {
                    return mcp::error(
                        call.op_id,
                        "rejected",
                        "knowledge create id already exists or cannot be resolved uniquely",
                    );
                }
            },
        }
        let result = match source.write(request, &context) {
            Ok(result) => result,
            Err(error)
                if matches!(
                    error.code.as_str(),
                    "write_not_confirmed" | "write_applied_result_unavailable"
                ) =>
            {
                return mcp::error(
                    call.op_id,
                    "unknown",
                    "knowledge write outcome is not confirmed; inspect original operation, never redispatch it",
                );
            }
            Err(error) => return mcp::error(call.op_id, &error.code, &error.message),
        };
        if self.scan().is_err() {
            return mcp::error(
                call.op_id,
                "unknown",
                "knowledge write applied; activity publication pending, inspect original operation",
            );
        }
        match serde_json::to_value(result) {
            Ok(value) => tool::bounded(call.op_id, mcp::success(call.op_id, value), true),
            Err(_) => mcp::error(
                call.op_id,
                "unknown",
                "knowledge write applied; result serialization unavailable",
            ),
        }
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
        let device_id = self.operator_device_for_effect()?;
        let principal = crate::mcp::Principal::Operator {
            device_id,
            door,
            issued_by: self.identity.issued_by().map(str::to_owned),
        };
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

    fn default_seat_name(&self, swarm: Ulid) -> Result<String> {
        Ok(first_available_seat_name(swarm, &self.registry.list()?))
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
        let mut harness = HarnessRecord::builtin(&spec.harness_kind)?;
        if let Some(command) = spec.command {
            if command.is_empty() || command.contains('\0') {
                return Err(invalid("invalid_spawn_command"));
            }
            harness.command = vec![command];
        }
        harness.command.extend(spec.args);
        if spec.env.iter().any(|(key, value)| {
            key.starts_with("BBOXY_")
                || key.is_empty()
                || key.starts_with(|c: char| c.is_ascii_digit())
                || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                || value.contains('\0')
        }) {
            return Err(invalid("reserved_spawn_environment"));
        }
        let swarm = Ulid::from(spec.swarm.0.0);
        let name = match spec.name {
            Some(name) => name,
            None => self.default_seat_name(swarm)?,
        };
        let mut record = SeatRecord::new(Ulid::new(), swarm, name, harness.kind.clone());
        record.location = match spec.location {
            bbt_proto::Where::Local => crate::registry::Location::Local,
            bbt_proto::Where::Ssh { host } => crate::registry::Location::Ssh { host },
            bbt_proto::Where::Cloud { provider, host } => {
                crate::registry::Location::Cloud { provider, host }
            }
        };
        record.launch_placement = spec.placement;
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
                env: spec.env,
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
            let ring = matches!(action, Action::ReadRing { .. })
                .then(|| bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::ReadRingRequest));
            let result = self.request_for_seat(seat, action.clone());
            if let Some(ring) = ring {
                let bytes = if ring.enabled() {
                    match &result {
                        Ok(Reply::RingHistory { history }) => history
                            .records
                            .iter()
                            .map(|record| match &record.event {
                                Event::Output { bytes, .. } => bytes.len() as u64,
                                _ => 0,
                            })
                            .sum(),
                        _ => 0,
                    }
                } else {
                    0
                };
                ring.finish(1, bytes);
            }
            span.finish(capture_outcome(&result), 0, 0);
            result
        });
        request.finish(capture_outcome(&result), 0, 0);
        result
    }
    fn refresh_gui_watch_interest(&mut self, pending: &mut PendingSeats) {
        let epoch = self.client.gui_watch_epoch.load(Ordering::Acquire);
        if epoch != self.gui_watch_epoch_seen {
            self.gui_watch_epoch_seen = epoch;
            if self.gui_watches.prune_closed() {
                pending.set_watched(self.gui_watches.watched());
            }
        }
    }

    fn ingest_one(
        &mut self,
        pending_events: &mut PendingSeats,
        made_progress: &mut bool,
    ) -> Result<()> {
        let Some((key, event)) = pending_events.next() else {
            return Ok(());
        };
        let before = self.views.get(&key.1).map(|view| view.sequence);
        if let Some(view) = self.views.get(&key.1) {
            self.client
                .capture_diagnostics
                .seat(key.1, view.generation, |row, _| {
                    row.target_sequence = event.sequence().unwrap();
                });
        }
        let result = self
            .prefetch_ring(&key, &event, pending_events)
            .and_then(|()| self.event(event));
        let complete = match result {
            Ok(complete) => {
                *made_progress =
                    complete || self.views.get(&key.1).map(|view| view.sequence) != before;
                complete
            }
            Err(error) if crate::provider::fake::is_link_down(&error) => {
                pending_events.park(&key.0);
                self.ring_pages.remove(&key.0);
                false
            }
            Err(error) if is_capture_busy(&error) => false,
            Err(error) => return Err(error),
        };
        if let Some(view) = self.views.get(&key.1) {
            pending_events.parsed(&key, view.sequence);
            pending_events.seats.get_mut(&key).unwrap().ring_blocked = view.ring_refusal.is_some();
        }
        pending_events.finish(key, complete);
        Ok(())
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
        let mut deferred_commands = VecDeque::new();
        while !stop.load(Ordering::Acquire) {
            // One pending bind per turn, rotating on refusal. No sleep, replay
            // of attachment effects, global seat scan, or idle redraw wake.
            self.retry_endpoint()?;
            let mut turn =
                self.client
                    .capture_diagnostics
                    .start(Phase::LogicTurn, Ulid::nil(), 0, 0, 0);
            self.refresh_gui_watch_interest(&mut pending_events);
            let checkpoint_service = self.client.capture_diagnostics.start(
                Phase::CheckpointService,
                Ulid::nil(),
                0,
                0,
                0,
            );
            self.service_checkpoints(false)?;
            checkpoint_service.finish(Outcome::Ok, 0, 0);
            let poll_ready = made_progress && pending_events.has_ready();
            made_progress = false;
            let command_span =
                self.client
                    .capture_diagnostics
                    .start(Phase::Commands, Ulid::nil(), 0, 0, 0);
            let mut command_count = 0;
            for _ in 0..32 {
                // Pending RPC-dependent commands retain their admission order;
                // local snapshots/status still run while a save owns the link.
                let command = if !self.checkpoint_saves.active() && !deferred_commands.is_empty() {
                    deferred_commands.pop_front().unwrap()
                } else {
                    if deferred_commands.len() >= 256 {
                        break;
                    }
                    let Ok(command) = commands.try_recv() else {
                        break;
                    };
                    command
                };
                if self.checkpoint_saves.active() && !command.local_while_checkpoint_pending() {
                    deferred_commands.push_back(command);
                    continue;
                }
                command_count += 1;
                match command {
                    Command::GuiWatchInterest {
                        session,
                        life,
                        seat,
                        generation,
                        reply,
                    } => {
                        let result = (|| {
                            if let Some(generation) = generation
                                && self
                                    .views
                                    .get(&seat)
                                    .is_none_or(|view| view.info.generation != generation)
                            {
                                return Err(invalid("stale GUI watch generation"));
                            }
                            if self.gui_watches.update(session, life, seat, generation)? {
                                pending_events.set_watched(self.gui_watches.watched());
                            }
                            Ok(())
                        })();
                        let _ = reply.send(result);
                    }
                    Command::GuiHosts(reply) => {
                        let _ = reply.send(Ok(self.connections.gui_hosts()));
                    }
                    Command::IdentityStatus(reply) => {
                        let _ = reply.send(self.identity.status());
                    }
                    #[cfg(test)]
                    Command::RefreshIdentity(reply) => {
                        let _ = reply.send(self.refresh_identity());
                    }
                    Command::OperatorIdentity(op, request, origin, reply) => {
                        let result = self.operator_identity(op, request, origin);
                        let _ = reply.send(result);
                    }
                    Command::ControlCatalogue(id, generation, reply) => {
                        let result = self
                            .control_actor(id, generation)
                            .map(|(actor, _)| crate::mcp::tools::catalogue(&actor));
                        let _ = reply.send(result);
                    }
                    Command::ControlCall(id, generation, call, reply) => {
                        let call = if call.tool == "ctl" {
                            match self.control_actor(id, generation).and_then(|(actor, _)| {
                                crate::mcp::seats::canonical(&actor, &call).map_err(invalid)
                            }) {
                                Ok(canonical) => canonical,
                                Err(_) => {
                                    let result = self.control_call(id, generation, call);
                                    let _ = reply.send(result);
                                    continue;
                                }
                            }
                        } else {
                            call
                        };
                        if crate::knowledge::seat_reads::handles(&call) {
                            self.start_knowledge_read(id, generation, call, reply);
                        } else if call.tool == "seat_focus" {
                            self.start_control_focus(id, generation, call, reply);
                        } else if call.tool == "seat_interrupt" {
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
                    Command::GuiMessaging(id, swarm, request, origin, reply) => {
                        let result = self.gui_messaging(id, swarm, request, origin);
                        let _ = reply.send(result);
                    }
                    Command::GuiTask(id, request, origin, reply) => {
                        let result = self.gui_task(id, request, origin);
                        let _ = reply.send(result);
                    }
                    Command::GuiRecordWrite(id, args, origin, reply) => {
                        let result = self.gui_record_write(id, args, origin);
                        let _ = reply.send(result);
                    }
                    Command::GuiPlugin(id, request, origin, reply) => {
                        let prepared = (|| -> Result<_> {
                            let origin = self.identity.bind_origin(origin);
                            let device_id = self.operator_device_for_effect()?;
                            if request.mutation() {
                                self.identity.operator_signing()?;
                            }
                            let actor = crate::mcp::Actor::Operator {
                                device_id,
                                door: origin.door,
                                issued_by: origin.issued_by,
                            };
                            let call = crate::mcp::Call {
                                op_id: id,
                                tool: "plugin_control".into(),
                                args: serde_json::to_value(&request)?,
                            };
                            call.validate()?;
                            Ok((actor, call))
                        })();
                        match prepared {
                            Err(error) => {
                                let _ = reply.send(Err(error));
                            }
                            Ok((actor, call)) => {
                                let plugins = self.plugins.clone();
                                let journal = self.operator_journal.clone();
                                self.plugin_jobs.spawn_on(
                                    async move {
                                        let result =
                                            plugins.request(actor, journal, call, request).await;
                                        let _ = reply.send(result);
                                    },
                                    &self.handle,
                                );
                            }
                        }
                    }
                    Command::GuiKnowledgeControl(id, request, origin, reply) => {
                        let prepared = (|| -> Result<_> {
                            let origin = self.identity.bind_origin(origin);
                            let device_id = self.operator_device_for_effect()?;
                            if request.mutation() {
                                self.identity.operator_signing()?;
                            }
                            let actor = crate::mcp::Actor::Operator {
                                device_id,
                                door: origin.door,
                                issued_by: origin.issued_by,
                            };
                            let call = crate::mcp::Call {
                                op_id: id,
                                tool: "knowledge_control".into(),
                                args: serde_json::to_value(&request)?,
                            };
                            call.validate()?;
                            Ok((actor, call))
                        })();
                        match prepared {
                            Err(error) => {
                                let _ = reply.send(Err(error));
                            }
                            Ok((actor, call)) => {
                                let host = self.knowledge_host.clone();
                                let index = self.index.clone();
                                let journal = self.operator_journal.clone();
                                self.plugin_jobs.spawn_on(
                                    async move {
                                        let result = host
                                            .request(index, actor, journal, call, request)
                                            .await;
                                        let _ = reply.send(result);
                                    },
                                    &self.handle,
                                );
                            }
                        }
                    }
                    Command::GuiDebug(id, request, origin, reply) => {
                        if let bbt_proto::gui::DebugRequest::RingGap { seat, from, to } = request {
                            let prepared = (|| -> Result<_> {
                                if origin.door != crate::mcp::Door::Ctl {
                                    return Err(invalid("debug uses the ctl operator door"));
                                }
                                if !self
                                    .identity
                                    .issued_by()
                                    .is_some_and(|s| s.starts_with("test-account:"))
                                {
                                    return Err(invalid(
                                        "test_account_required: ring-gap is refused on real accounts",
                                    ));
                                }
                                let view = self
                                    .views
                                    .get(&seat)
                                    .ok_or_else(|| invalid("ring_gap_unknown_seat"))?;
                                if from == 0
                                    || to < from
                                    || to - from >= crate::history_fault::INTERVAL
                                    || to > view.sequence
                                {
                                    return Err(invalid(
                                        "ring_gap_invalid_range: expected 1..4096 already applied capture sequences",
                                    ));
                                }
                                let generation = view.info.generation;
                                let origin = self.identity.bind_origin(origin);
                                let device_id =
                                    self.identity.status()?.device_id.ok_or_else(|| {
                                        invalid("held: operator device custody is unavailable")
                                    })?;
                                let actor = crate::mcp::Actor::Operator {
                                    device_id,
                                    door: origin.door,
                                    issued_by: origin.issued_by,
                                };
                                let call = crate::mcp::Call {
                                    op_id: id,
                                    tool: "debug_fault_ring_gap".into(),
                                    args: serde_json::json!({"seat":seat,"generation":generation,"from":from,"to":to}),
                                };
                                call.validate()?;
                                let permit =
                                    self.debug_reads.clone().try_acquire_owned().map_err(|_| {
                                        invalid("Busy: two debug operations already active; retry")
                                    })?;
                                Ok((actor, call, generation, permit))
                            })();
                            match prepared {
                                Err(error) => {
                                    let _ = reply.send(Err(error));
                                }
                                Ok((actor, call, generation, permit)) => {
                                    let journal = self.operator_journal.clone();
                                    let masks = self.history_faults.clone();
                                    self.handle.spawn_blocking(move || {
                                        let _permit = permit;
                                        if reply.is_closed() { return; }
                                        let result = journal.effect(&actor, &call, || match masks.install(seat,generation,from,to) {
                                            Ok(()) => crate::mcp::success(id,serde_json::json!({"seat":seat,"generation":generation,"from":from,"to":to,
                                                "scope":"test-account retained History projection; original ring bytes and live state untouched"})),
                                            Err(error) => crate::mcp::error(id,"ring_gap_refused",&error.to_string()),
                                        }).and_then(operator_response);
                                        let _ = reply.send(result);
                                    });
                                }
                            }
                            continue;
                        }
                        // The worker copies only cached numeric state. Files, SQLite,
                        // process queries and journal publication run on the read task.
                        let prepared = (|| -> Result<_> {
                            if origin.door != crate::mcp::Door::Ctl {
                                return Err(invalid("debug uses the ctl operator door"));
                            }
                            let permit =
                                self.debug_reads.clone().try_acquire_owned().map_err(|_| {
                                    invalid("Busy: two debug reads are already active; retry")
                                })?;
                            let origin = self.identity.bind_origin(origin);
                            let device_id = self.identity.status()?.device_id.ok_or_else(|| {
                                invalid("held: operator device custody is unavailable")
                            })?;
                            let actor = crate::mcp::Actor::Operator {
                                device_id,
                                door: origin.door,
                                issued_by: origin.issued_by,
                            };
                            let call = crate::mcp::Call {
                                op_id: id,
                                tool: "debug_read".into(),
                                args: serde_json::to_value(&request)?,
                            };
                            call.validate()?;
                            let seats = self.views.iter().take(crate::debug::ROWS).map(|(seat,view)| {
                                serde_json::json!({"seat":seat,"generation":view.info.generation,
                                    "logic_current_snapshot":{"sequence":view.sequence,"view_generation":view.view_generation},
                                    "captured_sequence":view.captured_sequence,
                                    "checkpoint_sequence":view.checkpoint_sequence,
                                    "checkpoint_pending":view.checkpoint_pending.is_some(),
                                    "checkpoint_paused":view.checkpoint_paused,
                                    "gui_at_timeout":null,"applied_written_to_socket":null,"supervisor_ring_tail":null})
                            }).collect();
                            let mut processes = BTreeMap::new();
                            for view in self
                                .views
                                .values()
                                .filter(|v| v.info.exited.is_none())
                                .take(2048)
                            {
                                processes.insert(view.info.pid, "seat");
                                if let Some(companion) = &view.info.companion {
                                    processes.insert(companion.pid, "companion");
                                }
                            }
                            let context = crate::debug::Context {
                                root: self.config.run_dir.clone(),
                                peers: self.client.gui_peers(),
                                seats,
                                seat_count: self.views.len(),
                                watched: self.gui_watches.watched().len(),
                                stale_gui_requests: self.stale_gui_requests,
                                processes,
                            };
                            Ok((permit, actor, call, context))
                        })();
                        match prepared {
                            Err(error) => {
                                let _ = reply.send(Err(error));
                            }
                            Ok((permit, actor, call, context)) => {
                                let journal = self.operator_journal.clone();
                                let recorder = self.client.clone();
                                self.handle.spawn_blocking(move || {
                                    let _permit = permit;
                                    if reply.is_closed() {
                                        return;
                                    }
                                    let result = (|| -> Result<_> {
                                        let capture = if matches!(
                                            &request,
                                            bbt_proto::gui::DebugRequest::Metrics
                                                | bbt_proto::gui::DebugRequest::Snapshot { .. }
                                        ) {
                                            crate::debug::capture(
                                                serde_json::to_value(
                                                    recorder.capture_diagnostics(),
                                                )?,
                                                matches!(
                                                    &request,
                                                    bbt_proto::gui::DebugRequest::Snapshot { .. }
                                                ),
                                            )
                                        } else {
                                            serde_json::Value::Null
                                        };
                                        let value = match context.read(&request, capture) {
                                            Ok(data) => crate::mcp::success(id, data),
                                            Err(error) => crate::mcp::error(
                                                id,
                                                "refused",
                                                crate::debug::sanitize(serde_json::json!(
                                                    error.to_string()
                                                ))
                                                .as_str()
                                                .unwrap_or("debug source unavailable"),
                                            ),
                                        };
                                        // Do not hold a journal transaction across diagnostic IO.
                                        operator_response(journal.checked(&actor, &call, value)?)
                                    })();
                                    let _ = reply.send(result);
                                });
                            }
                        }
                    }
                    Command::GuiKnowledge(method, args, reply) => {
                        let Ok(permit) = self.knowledge_reads.clone().try_acquire_owned() else {
                            let mut error = crate::knowledge::data_source::CommandError::new(
                                "provider_busy",
                                "knowledge read capacity is occupied",
                            );
                            error.retry_after_ms = Some(100);
                            let _ = reply.send(Err(error));
                            continue;
                        };
                        let index = self.index.clone();
                        let registry = self.registry.clone();
                        let host = self.knowledge_host.clone();
                        #[cfg(test)]
                        let fixture = self.knowledge_fixture.clone();
                        self.handle.spawn_blocking(move || {
                            // Cancellation does not free capacity until the blocking read ends.
                            let _permit = permit;
                            if reply.is_closed() {
                                return;
                            }
                            #[cfg(test)]
                            if let Some(source) = fixture {
                                let _ = reply.send(crate::knowledge::read(
                                    source.as_ref(),
                                    &method,
                                    args,
                                ));
                                return;
                            }
                            let _ = reply.send(host.with_source(&index, &registry, |source| {
                                crate::knowledge::read(source, &method, args)
                            }));
                        });
                    }
                    #[cfg(test)]
                    Command::KnowledgeFixture(source, reply) => {
                        self.knowledge_fixture = Some(source);
                        let _ = reply.send(());
                    }
                    Command::GuiMessageRow(swarm, message, reply) => {
                        let result = self
                            .gui_device()
                            .and_then(|_| self.messaging.operator_message_row(swarm, message));
                        let _ = reply.send(result);
                    }
                    Command::GuiMessagingSubjects(reply) => {
                        let result = self
                            .gui_device()
                            .and_then(|_| self.messaging.operator_subjects());
                        let _ = reply.send(result);
                    }
                    Command::GuiInput(id, generation, input_id, bytes, reply) => {
                        let result = self.seat_input(id, generation, input_id, bytes);
                        let _ = reply.send(result);
                    }
                    Command::GuiScreen(id, cells, reply) => {
                        let result = self.seat_screen(id, cells);
                        let _ = reply.send(result);
                    }
                    Command::GuiAttention(id, generation, reply) => {
                        let _ = reply.send(self.gui_attention(id, generation));
                    }
                    Command::CtlAttention(id, edit, op, origin, reply) => {
                        let _ = reply.send(self.ctl_attention(id, edit, op, origin));
                    }
                    Command::CtlRename(id, name, op, origin, reply) => {
                        let result = self.ctl_rename(id, name, op, origin);
                        let _ = reply.send(result);
                    }
                    Command::CtlRestart(id, generation, op, origin, reply) => {
                        let result = self.ctl_restart(id, generation, op, origin);
                        let _ = reply.send(result);
                    }
                    Command::CtlUi(op, request, origin, reply) => {
                        let _ = reply.send(self.ctl_ui_admit(op, request, origin));
                    }
                    Command::CtlRecord(op, tool, args, origin, reply) => {
                        let result = self.ctl_journaled(op, &tool, args, origin, |_| Ok(()));
                        let _ = reply.send(result);
                    }
                    Command::CtlResize(id, generation, cols, rows, op, origin, reply) => {
                        let result = self.ctl_resize(id, generation, cols, rows, op, origin);
                        let _ = reply.send(result);
                    }
                    Command::CtlInput(id, generation, input_id, bytes, origin, reply) => {
                        let result = self.ctl_input(id, generation, input_id, bytes, origin);
                        let _ = reply.send(result);
                    }
                    Command::GuiResize(id, generation, cols, rows, reply) => {
                        let result = self.seat_resize(id, generation, cols, rows);
                        let _ = reply.send(result);
                    }
                    Command::GuiHello(reply) => {
                        let result =
                            self.identity
                                .status()
                                .map(|status| bbt_proto::gui::GuiHello {
                                    run_dir: self.config.run_dir.clone(),
                                    account_root: status.root_fingerprint,
                                    window: None,
                                });
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
                                if event.raw["bboxy_proxy"] == true
                                    && view.shared_codex_runtime.is_some()
                                    && view.adapter.binding.runtime.is_some()
                                {
                                    let mut binding = view.adapter.binding.clone();
                                    binding.runtime = None;
                                    binding.attachment_generation =
                                        binding.attachment_generation.checked_add(1).ok_or_else(
                                            || invalid("native attachment generation exhausted"),
                                        )?;
                                    view.adapter.rebind(binding)?;
                                    view.delivery_endpoint = crate::adapters::Endpoint::None;
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
                    Command::CodexDiscovery(discovery, reply) => {
                        let id = discovery.binding.seat;
                        let result = (|| {
                            let Some(view) = self.views.get_mut(&id) else {
                                return Ok(false);
                            };
                            if view.generation != discovery.binding.generation
                                || view.adapter.binding != discovery.binding
                            {
                                return Ok(false);
                            }
                            let (thread, status) = match discovery.result {
                                Ok(thread) => thread,
                                Err(reason) => {
                                    view.delivery_failure = Some(reason);
                                    return Ok(false);
                                }
                            };
                            let bound = crate::adapters::native::bind_discovered_codex(
                                &mut view.adapter,
                                &discovery.binding,
                                &thread,
                                status,
                                view.state.sequence,
                                now_ms(),
                            )?;
                            if bound {
                                view.delivery_failure = None;
                            }
                            Ok(bound)
                        })();
                        if matches!(result, Ok(true)) {
                            self.persist_native_session(id)?;
                            self.publish_adapter(id, self.views[&id].adapter.status.clone())?;
                        }
                        let _ = reply.send(result);
                    }
                    Command::SharedCodexDiscovery(discovery, reply) => {
                        let id = discovery.binding.seat;
                        let result = (|| {
                            let Some(view) = self.views.get_mut(&id) else {
                                return Ok(false);
                            };
                            if view.adapter.binding != discovery.binding
                                || view.adapter.status.source == "exit"
                            {
                                return Ok(false);
                            }
                            let runtime = match discovery.result {
                                Ok(runtime) => runtime,
                                Err(reason) => {
                                    view.delivery_failure = Some(reason);
                                    return Ok(false);
                                }
                            };
                            let mut binding = discovery.binding;
                            binding.runtime = Some(runtime);
                            binding.attachment_generation = binding
                                .attachment_generation
                                .checked_add(1)
                                .ok_or_else(|| invalid("native attachment generation exhausted"))?;
                            view.adapter.rebind(binding)?;
                            view.delivery_failure = None;
                            Ok(true)
                        })();
                        let _ = reply.send(result);
                    }
                    Command::NativeHookFailure(payload) => {
                        let (binding, failure) = *payload;
                        let current = self
                            .views
                            .get(&binding.seat)
                            .filter(|view| {
                                view.info.exited.is_none()
                                    && view.info.generation == binding.generation
                            })
                            .map(|view| &view.adapter.binding);
                        if let Some(row) = native_hook_row(current, &binding, failure) {
                            bbt_rt::error_journal::report_to(&self.config.run_dir, row);
                        }
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
                    Command::History(id, generation, from, to, masks, reply) => {
                        let result = self.history(id, generation, from, to).and_then(|history| {
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
                                .ok_or_else(|| {
                                    if self.pending_endpoints.iter().any(|(seat, _)| *seat == id) {
                                        crate::seat_socket::SeatListenerPending.into()
                                    } else {
                                        invalid("seat endpoint unavailable")
                                    }
                                }),
                        );
                    }
                    Command::Close(id, generation, operation, position, grace, target, reply) => {
                        let Some(view) = self.views.get(&id).filter(|v| v.generation == generation)
                        else {
                            let _ = reply.send(Err(invalid("stale close generation")));
                            continue;
                        };
                        let target = target.unwrap_or(view.sequence);
                        if view.checkpoint_sequence < target {
                            if let Err(error) = self.save(id) {
                                let _ = reply.send(Err(error));
                            } else {
                                // Keep the original cursor and RPC-command order
                                // until its strong publication completes. The
                                // Worker returns to ingestion instead of waiting.
                                deferred_commands.push_front(Command::Close(
                                    id,
                                    generation,
                                    operation,
                                    position,
                                    grace,
                                    Some(target),
                                    reply,
                                ));
                                break;
                            }
                            continue;
                        }
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
                            if let Some(position) = restored_close_position(
                                &record,
                                generation,
                                revision,
                                layout_revision,
                                self.views
                                    .get(&id)
                                    .is_some_and(|view| view.info.exited.is_none()),
                            )? {
                                return Ok(position);
                            }
                            let close = record.close.as_ref().expect("validated close record");
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
            self.refresh_gui_watch_interest(&mut pending_events);
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
                        let seat = event.seat();
                        let view = self.views.get_mut(&seat).unwrap();
                        view.captured_sequence =
                            view.captured_sequence.max(event.sequence().unwrap());
                        if matches!(&event, Event::Output { bytes, .. } if !bytes.is_empty()) {
                            let now = Instant::now();
                            view.last_arrival_at = now;
                            view.changed_at.arrived(now);
                        }
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
            // One parsed page per turn, with fresh commands/notifications between.
            self.ingest_one(&mut pending_events, &mut made_progress)?;
            let maintenance =
                self.client
                    .capture_diagnostics
                    .start(Phase::Maintenance, Ulid::nil(), 0, 0, 0);
            if !self.checkpoint_saves.active() {
                if providers_at.elapsed() >= Duration::from_millis(200) {
                    let phase = self.client.capture_diagnostics.start(
                        Phase::MaintenanceProviders,
                        Ulid::nil(),
                        0,
                        0,
                        0,
                    );
                    self.refresh_providers(&mut provider_recoveries, &mut pending_events)?;
                    phase.finish(Outcome::Ok, 0, 0);
                    providers_at = Instant::now();
                }
                if adapters_at.elapsed() >= Duration::from_millis(200) {
                    let phase = self.client.capture_diagnostics.start(
                        Phase::MaintenanceAdapters,
                        Ulid::nil(),
                        0,
                        0,
                        0,
                    );
                    self.refresh_adapters()?;
                    phase.finish(Outcome::Ok, 0, 0);
                    adapters_at = Instant::now();
                }
                self.refresh_identity_due(&mut identity_at, Instant::now)?;
                if messages_at.elapsed() >= Duration::from_millis(100) {
                    let phase = self.client.capture_diagnostics.start(
                        Phase::MaintenanceMessages,
                        Ulid::nil(),
                        0,
                        0,
                        0,
                    );
                    self.pump_messages()?;
                    self.publish_control_audits()?;
                    phase.finish(Outcome::Ok, 0, 0);
                    messages_at = Instant::now();
                }
                let replies: Vec<_> = self
                    .views
                    .iter()
                    .filter(|(_, v)| !v.replies.is_empty())
                    .map(|(id, _)| *id)
                    .take(32)
                    .collect();
                for id in replies {
                    self.flush_protocol_replies(id)?;
                }
                if scan_at.elapsed() >= Duration::from_millis(100) {
                    let phase = self.client.capture_diagnostics.start(
                        Phase::MaintenanceProvenance,
                        Ulid::nil(),
                        0,
                        0,
                        0,
                    );
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
                        .map(|(seat, view)| {
                            (*seat, view.generation, view.adapter.status.observed_ms)
                        })
                        .collect();
                    let changed = match self.provenance.step(
                        Arc::make_mut(&mut self.index),
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
                    phase.finish(Outcome::Ok, active.len() as u64, 0);
                    scan_at = Instant::now();
                }
            }
            // Each completed save gets an ordinary ingest/command turn before
            // the next is dispatched, interleaving the single leased stream.
            let checkpoint_service = self.client.capture_diagnostics.start(
                Phase::CheckpointService,
                Ulid::nil(),
                0,
                0,
                0,
            );
            self.service_checkpoints(true)?;
            checkpoint_service.finish(Outcome::Ok, 0, 0);
            maintenance.finish(Outcome::Ok, 0, 0);
            if self.checkpoint_saves.active() && !made_progress {
                // The side-task completion unparks this thread. Contended
                // nonblocking receive must not create a no-progress spin.
                std::thread::park_timeout(Duration::from_millis(1));
            }
            turn.pending(
                pending_events.seats.len(),
                pending_events.ready_len(),
                pending_events.parked.len(),
            );
            turn.finish(Outcome::Ok, 0, 0);
        }
        for id in self.views.keys().copied().collect::<Vec<_>>() {
            self.save(id)?;
        }
        // Shutdown is outside ingestion: all forced publications must finish
        // before handing over the lease, with a named bounded failure if not.
        let mut progress = self.checkpoint_progress;
        let mut progressed_at = Instant::now();
        loop {
            self.service_checkpoints(true)?;
            if self.checkpoint_saves.in_flight() == 0
                && self
                    .views
                    .values()
                    .all(|v| v.checkpoint_sequence == v.sequence)
            {
                break;
            }
            if self.checkpoint_progress != progress {
                progress = self.checkpoint_progress;
                progressed_at = Instant::now();
            }
            // Per-request deadlines still bound IO. Do not impose a total
            // throughput deadline on a shutdown draining many healthy seats.
            if progressed_at.elapsed() >= Duration::from_secs(20) {
                return Err(invalid(
                    "checkpoint shutdown drain unavailable; publication outcome unknown",
                ));
            }
            std::thread::park_timeout(Duration::from_millis(1));
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
        self.knowledge_host.close();
        let roots: BTreeSet<_> = self
            .endpoints
            .values()
            .filter_map(|(info, _)| info.path.parent().map(|path| path.to_path_buf()))
            .collect();
        self.endpoints.clear();
        for root in roots {
            let _ = std::fs::remove_dir(root);
        }
        drop(self.gui_endpoint.take());
        let _ = std::fs::remove_dir(&self.endpoint_root);
    }
}
fn gui_activity(
    state: crate::adapters::State,
    waiting: Option<crate::adapters::WaitingKind>,
) -> bbt_proto::Activity {
    use crate::adapters::State;
    use bbt_proto::{Activity, WaitKind};
    match state {
        State::Active => Activity::Active,
        State::Idle => Activity::Idle,
        State::Errored => Activity::Errored,
        State::Waiting => Activity::Waiting(
            waiting
                .map(crate::adapters::mapping::waiting)
                .unwrap_or(WaitKind::Question),
        ),
    }
}
fn gui_grade(grade: crate::adapters::Grade) -> bbt_proto::Grade {
    crate::adapters::mapping::grade(grade)
}

fn check_size(cols: usize, rows: usize) -> Result<()> {
    if cols < 2 || rows == 0 || cols > 512 || rows > 256 || cols * rows > 32768 {
        return Err(invalid("viewport dimensions exceed bounded logic model"));
    }
    Ok(())
}

/// One screen row as runs of equal style (`ctl seat screen --cells`).
fn style_runs(cells: &[bbt_core::Cell]) -> Vec<bbt_proto::gui::StyledRun> {
    let mut runs: Vec<bbt_proto::gui::StyledRun> = Vec::new();
    for cell in cells {
        if cell.flags.intersects(
            bbt_core::term::cell::Flags::WIDE_CHAR_SPACER
                | bbt_core::term::cell::Flags::LEADING_WIDE_CHAR_SPACER,
        ) {
            continue;
        }
        let flags = cell.flags.bits();
        let mut text = cell.c.to_string();
        if let Some(extra) = cell.zerowidth() {
            text.extend(extra);
        }
        match runs.last_mut() {
            Some(run) if run.fg == cell.fg && run.bg == cell.bg && run.flags == flags => {
                run.text.push_str(&text)
            }
            _ => runs.push(bbt_proto::gui::StyledRun {
                text,
                fg: cell.fg,
                bg: cell.bg,
                flags,
            }),
        }
    }
    runs
}
/// One screen row as text: wide-glyph spacers skipped, zero-width marks kept.
fn row_text(cells: &[bbt_core::Cell]) -> String {
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
    text
}

// The close reply's restoration handle is valid only for the same live process.
fn restored_close_position(
    record: &SeatRecord,
    generation: u64,
    revision: u64,
    layout_revision: u64,
    process_live: bool,
) -> Result<Option<crate::registry::Position>> {
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
        if !process_live {
            return Err(invalid("process exited"));
        }
        return Ok(Some(close.position.clone()));
    }
    Ok(None)
}

#[cfg(test)]
mod pending_tests {
    use super::*;
    fn session(epoch: &Arc<AtomicU64>) -> GuiWatchSession {
        GuiWatchSession {
            id: Ulid::new(),
            life: Arc::new(GuiWatchLife {
                epoch: epoch.clone(),
            }),
        }
    }
    fn interest(
        watches: &mut GuiWatchInterests,
        session: &GuiWatchSession,
        seat: Ulid,
        generation: Option<u64>,
    ) -> bool {
        watches
            .update(session.id, Arc::downgrade(&session.life), seat, generation)
            .unwrap()
    }
    // keeps: invariant watched-replay-priority
    #[test]
    fn watch_refcounts_are_idempotent_and_disconnect_releases_only_its_session() {
        let epoch = Arc::new(AtomicU64::new(0));
        let one = session(&epoch);
        let two = session(&epoch);
        let seat = Ulid::from(1);
        let mut watches = GuiWatchInterests::default();
        assert!(interest(&mut watches, &one, seat, Some(1)));
        assert!(!interest(&mut watches, &one, seat, Some(1)));
        assert!(!interest(&mut watches, &two, seat, Some(1)));
        assert_eq!(watches.references[&(seat, 1)], 2);
        assert!(!interest(&mut watches, &one, seat, None));
        assert_eq!(watches.references[&(seat, 1)], 1);
        drop(one);
        assert!(!watches.prune_closed());
        assert_eq!(watches.watched(), BTreeSet::from([(seat, 1)]));
        drop(two);
        assert_eq!(epoch.load(Ordering::Acquire), 2);
        assert!(watches.prune_closed());
        assert!(watches.references.is_empty());
        assert!(watches.sessions.is_empty());
    }
    // keeps: invariant watched-replay-priority
    #[test]
    fn closed_watch_admission_cannot_resurrect_interest_or_change_its_token() {
        let epoch = Arc::new(AtomicU64::new(0));
        let one = session(&epoch);
        let seat = Ulid::from(1);
        let mut watches = GuiWatchInterests::default();
        interest(&mut watches, &one, seat, Some(1));
        assert!(interest(&mut watches, &one, seat, Some(2)));
        assert_eq!(watches.watched(), BTreeSet::from([(seat, 2)]));
        let other = session(&epoch);
        assert!(
            watches
                .update(one.id, Arc::downgrade(&other.life), seat, Some(3))
                .is_err()
        );
        let id = one.id;
        let late = Arc::downgrade(&one.life);
        drop(one);
        assert!(watches.update(id, late, seat, Some(3)).is_err());
        assert!(watches.prune_closed());
        assert!(watches.watched().is_empty());
    }
    // keeps: invariant watched-replay-priority
    #[test]
    fn watched_quota_is_three_to_one_and_each_lane_rotates() {
        let mut pending = PendingSeats::default();
        let seats = [Ulid::from(1), Ulid::from(2), Ulid::from(3), Ulid::from(4)];
        pending.set_watched(BTreeSet::from([(seats[0], 1), (seats[1], 1)]));
        for seat in seats {
            pending
                .push(HostId::Local, output(seat, 1, 1000), 1, 1)
                .unwrap();
        }
        let mut watched_turn = 0;
        let mut background_turn = 0;
        let mut classes = Vec::new();
        for turn in 0..128 {
            let (key, event) = pending.next().unwrap();
            let watched = turn % 4 != 3;
            let expected = if watched {
                let seat = seats[watched_turn % 2];
                watched_turn += 1;
                seat
            } else {
                let seat = seats[2 + background_turn % 2];
                background_turn += 1;
                seat
            };
            assert_eq!(event.seat(), expected);
            classes.push(watched);
            pending.finish(key, false);
        }
        assert!(
            classes
                .windows(8)
                .all(|window| window.iter().filter(|&&w| w).count() >= 4)
        );
        assert_eq!(background_turn, 32);
    }
    // keeps: invariant watched-replay-priority
    #[test]
    fn empty_lanes_donate_without_borrowing_the_next_background_turn() {
        let watched = Ulid::from(1);
        let background = Ulid::from(2);
        let mut pending = PendingSeats::default();
        pending.set_watched(BTreeSet::from([(watched, 1)]));
        pending
            .push(HostId::Local, output(watched, 1, 1000), 1, 1)
            .unwrap();
        for _ in 0..100 {
            let (key, event) = pending.next().unwrap();
            assert_eq!(event.seat(), watched);
            pending.finish(key, false);
        }
        pending
            .push(HostId::Local, output(background, 1, 1000), 1, 1)
            .unwrap();
        let (key, event) = pending.next().unwrap();
        assert_eq!(event.seat(), background);
        pending.finish(key, true);
        pending.set_watched(BTreeSet::new());
        for _ in 0..100 {
            let (key, event) = pending.next().unwrap();
            assert_eq!(event.seat(), watched);
            pending.finish(key, false);
        }
    }
    // keeps: invariant watched-replay-priority
    #[test]
    fn ended_interest_demotes_next_round_without_tombstones_or_generation_leaks() {
        let watched = Ulid::from(1);
        let background = Ulid::from(2);
        let mut pending = PendingSeats::default();
        pending.set_watched(BTreeSet::from([(watched, 1)]));
        for seat in [watched, background] {
            pending
                .push(HostId::Local, output(seat, 1, 1000), 1, 1)
                .unwrap();
        }
        for _ in 0..10_000 {
            pending.set_watched(BTreeSet::new());
            pending.set_watched(BTreeSet::from([(watched, 1)]));
            assert_eq!(pending.ready_len(), 2);
            assert_eq!(
                pending
                    .ready_watched
                    .iter()
                    .chain(&pending.ready_background)
                    .collect::<BTreeSet<_>>()
                    .len(),
                2
            );
        }
        pending.set_watched(BTreeSet::new());
        let (key, event) = pending.next().unwrap();
        assert_eq!(event.seat(), background);
        pending.finish(key, false);
        pending.set_watched(BTreeSet::from([(watched, 1)]));
        pending
            .push(HostId::Local, output(watched, 2, 1), 2, 1)
            .unwrap();
        assert!(pending.ready_watched.is_empty());
        assert_eq!(pending.ready_background.len(), 2);
    }
    // keeps: spec mux-daemon
    #[test]
    fn watched_and_background_work_stays_parked_without_a_ready_spin() {
        let mut pending = PendingSeats::default();
        let watched = Ulid::from(1);
        pending.set_watched(BTreeSet::from([(watched, 1)]));
        for seat in [watched, Ulid::from(2)] {
            pending
                .push(HostId::Local, output(seat, 1, 1000), 1, 1)
                .unwrap();
        }
        pending.park(&HostId::Local);
        for _ in 0..100 {
            assert!(!pending.has_ready());
            assert!(pending.next().is_none());
            assert_eq!(pending.ready_len(), 2);
        }
        pending.resume(&HostId::Local);
        assert!(pending.has_ready());
        assert_eq!(pending.next().unwrap().1.seat(), watched);
    }
    fn output(seat: Ulid, generation: u64, seq: u64) -> Event {
        Event::Output {
            seat,
            generation,
            seq,
            bytes: vec![b'x'],
        }
    }
    // keeps: spec mux-daemon
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
        assert_eq!(pending.ready_len(), 4);
    }
    // keeps: invariant no-chokepoints (bounded captured backlog, order, Exit-last)
    #[test]
    fn pending_notifications_stay_bounded_and_preserve_resize_and_exit_order() {
        let mut pending = PendingSeats::default();
        let seat = Ulid::from(1);
        pending.set_watched(BTreeSet::from([(seat, 1)]));
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
        assert_eq!(pending.ready_len(), 1);
        let mut sequences = Vec::new();
        while let Some((key, event)) = pending.next() {
            sequences.push(event.sequence().unwrap());
            pending.finish(key, true);
        }
        assert_eq!(sequences, [1, 10_001, 20_000, 20_001]);
        assert!(pending.is_empty());
    }
    // keeps: invariant no-chokepoints (raw cap is separate from captured spans)
    #[test]
    fn raw_backlog_cap_preserves_fifo_while_captured_output_coalesces_until_exit() {
        let raw = Ulid::from(1);
        let captured = Ulid::from(2);
        let mut pending = bbt_supervisor::client::PendingEvents::default();
        pending.confirm_ring(captured, 1, 1).unwrap();
        for seq in 1..=1024 {
            pending.push(output(raw, 1, seq)).unwrap();
        }
        for seq in 1..=10_000 {
            pending.push(output(captured, 1, seq)).unwrap();
        }
        let refused = pending.push(output(raw, 1, 1025)).unwrap_err();
        assert_eq!(
            refused.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::InvalidData
        );
        pending
            .push(Event::Exited {
                seat: captured,
                generation: 1,
                seq: 10_001,
                status: crate::protocol::ExitInfo {
                    code: 0,
                    signal: None,
                },
            })
            .unwrap();
        for seq in 1..=1024 {
            assert_eq!(pending.pop_front(), Some(output(raw, 1, seq)));
        }
        assert_eq!(pending.pop_front(), Some(output(captured, 1, 10_000)));
        assert!(matches!(
            pending.pop_front(),
            Some(Event::Exited { seq: 10_001, .. })
        ));
        assert!(pending.pop_front().is_none());
    }
    // keeps: spec mux-daemon
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
        assert_eq!(pending.ready_len(), 0);
    }
    // keeps: spec mux-daemon
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
        assert_eq!(pending.ready_len(), 1);
        pending.resume(&HostId::Local);
        assert!(pending.has_ready());
        let (key, event) = pending.next().unwrap();
        assert_eq!(event.sequence(), Some(1000));
        pending.finish(key, true);
        assert!(pending.is_empty());
    }
}

#[cfg(test)]
mod close_restoration_tests {
    use super::*;
    use crate::registry::Position;
    // keeps: spec c1-live-seats §4 (idempotent undo is generation/liveness bound)
    #[test]
    fn undo_restores_the_exact_position_once_and_refuses_stale_process_handles() {
        let mut record = SeatRecord::new(Ulid::new(), Ulid::new(), "close fixture", "zsh");
        let position = Position {
            swarm: record.swarm,
            tab: Ulid::new(),
            split_path: vec![1, 0],
            ratios: vec![0.35, 0.65],
            layout_revision: 7,
        };
        record.lifecycle = Lifecycle::ClosedGrace;
        record.lifecycle_revision = 1;
        record.close = Some(crate::registry::CloseRecord {
            operation: Ulid::new(),
            revision: 1,
            deadline_ms: 60_000,
            position: position.clone(),
        });
        assert_eq!(
            restored_close_position(&record, 1, 1, 7, true).unwrap(),
            None
        );
        assert!(restored_close_position(&record, 1, 1, 8, true).is_err());
        assert!(restored_close_position(&record, 1, 2, 7, true).is_err());
        assert!(restored_close_position(&record, 2, 1, 7, true).is_err());
        record.lifecycle = Lifecycle::Open;
        record.lifecycle_revision = 2;
        for _ in 0..2 {
            assert_eq!(
                restored_close_position(&record, 1, 1, 7, true).unwrap(),
                Some(position.clone())
            );
        }
        assert!(restored_close_position(&record, 1, 1, 7, false).is_err());
        record.generation = 2;
        record.close = None;
        assert!(restored_close_position(&record, 2, 1, 7, true).is_err());
    }
}

#[cfg(test)]
mod identity_retry_clock_tests {
    #![allow(
        clippy::disallowed_methods,
        reason = "private fixture IO and memory custody"
    )]
    use super::*;
    use crate::{
        identity::{Custody, DeviceKeyStore, IdentityRuntime, IdentityState, PendingReason},
        messaging::{
            api::{Request, Response},
            wire::SendRequest,
        },
    };
    use std::sync::Mutex;

    struct Link;
    impl SupervisorConnection for Link {
        fn request(&mut self, action: Action) -> Result<Reply> {
            match action {
                Action::ListOwnedSeats => Ok(Reply::Seats { seats: vec![] }),
                _ => panic!("identity clock fixture must not perform terminal effects"),
            }
        }
        fn receive(&mut self, _: Duration) -> Result<Option<Event>> {
            Ok(None)
        }
    }
    struct Keys {
        held: AtomicBool,
        values: Mutex<BTreeMap<Ulid, SigningKey>>,
        clock: Arc<Mutex<Instant>>,
        finished: Instant,
    }
    impl DeviceKeyStore for Keys {
        fn custody(&self) -> Custody {
            Custody::Memory
        }
        fn load(&self, id: Ulid) -> crate::identity::Result<Option<SigningKey>> {
            if self.held.load(Ordering::Acquire) {
                return Err(PendingReason::KeychainUnavailable(-25308));
            }
            Ok(self.values.lock().unwrap().get(&id).cloned())
        }
        fn create(&self, id: Ulid, key: &SigningKey) -> crate::identity::Result<()> {
            let mut values = self.values.lock().unwrap();
            let std::collections::btree_map::Entry::Vacant(entry) = values.entry(id) else {
                return Err(PendingReason::JournalConflict);
            };
            entry.insert(key.clone());
            // A completed refresh takes three logical seconds. Its next interval
            // must begin after the work, exactly as in the production loop.
            *self.clock.lock().unwrap() = self.finished;
            Ok(())
        }
    }

    // keeps: spec id1-seat-chain §4 (30-second custody retry, no replacement or replay)
    #[test]
    fn thirty_second_identity_retry_releases_held_messages_and_provenance_on_a_controlled_clock() {
        let root = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let start = Instant::now();
        let clock = Arc::new(Mutex::new(start));
        let keys = Arc::new(Keys {
            held: AtomicBool::new(true),
            values: Mutex::default(),
            clock: clock.clone(),
            finished: start + Duration::from_secs(33),
        });
        let config = LogicConfig {
            run_dir: root.path().join("run"),
            repository: root.path().join("repository"),
            ring: Default::default(),
        };
        std::fs::create_dir_all(config.repository.join(".bboxy")).unwrap();
        let identity = IdentityRuntime::open_custodied_with_store(
            &config.run_dir,
            keys.clone(),
            runtime.handle().clone(),
        )
        .unwrap();
        let signing = SigningKey::from_bytes(&[56; 32]);
        let window = crate::identity::begin_bus_window(&config.run_dir, &signing).unwrap();
        let mut broker = Broker::open(&config.run_dir, signing.clone()).unwrap();
        broker.identity_signer(signing, Some(window));
        let bus = BusHandle::start(runtime.handle(), broker, 128).unwrap();
        let (tx, _rx) = mpsc::channel(128);
        let client = LogicClient {
            tx,
            bus: bus.clone(),
            exit_cause: Arc::new(OnceLock::new()),
            capture_diagnostics: Arc::new(Recorder::default()),
            gui_peers: Arc::default(),
            captures: Arc::default(),
            gui_watch_epoch: Arc::default(),
            window_journal: None,
        };
        let mut worker = Worker::open(
            vec![Box::new(LocalProvider::new(Link))],
            config,
            bus.clone(),
            runtime.handle().clone(),
            client,
            identity,
            Some(window),
        )
        .unwrap();
        let seat = SeatRecord::new(Ulid::new(), Ulid::new(), "clock fixture", "zsh");
        worker.registry.insert(&seat).unwrap();
        worker.messaging.sync_seat(&seat).unwrap();
        let mut ids = Vec::new();
        for text in ["first held", "second held"] {
            let request = SendRequest::text(format!("seats/{}/inbox", seat.id), text);
            ids.push(request.id);
            let Response::Stored { message } = worker
                .messaging
                .request(
                    seat.id,
                    1,
                    Request::Send { message: request },
                    false,
                    None,
                    1,
                )
                .unwrap()
            else {
                panic!("local delivery was held");
            };
            assert!(message.envelope().is_none());
            assert_eq!(message.view().id, *ids.last().unwrap());
        }
        // Feed a completed observation at the public bus boundary. Projection
        // waits for the same controlled retry; the original signer never changes.
        use crate::provenance::artifacts::{Attribution, Change};
        use crate::store::Events;
        let artifact = Ulid::new();
        let path = format!("artifacts/{artifact}.md");
        let observation = Change {
            artifact_id: artifact,
            kind: "created".into(),
            reference: "held.txt".into(),
            previous_ref: None,
            digest: Some(format!(
                "sha256:{:x}",
                <sha2::Sha256 as sha2::Digest>::digest(b"held-content")
            )),
            before_digest: None,
            bytes: Some(12),
            attribution: Attribution::inferred(vec![]),
            source: "filesystem".into(),
            reason: None,
            commit: None,
        };
        let observed = runtime
            .block_on(bus.publish(
                "index",
                Draft::new(
                    format!("artifact/{artifact}/created"),
                    "provenance/filesystem",
                    serde_json::json!({"observation": observation}),
                ),
            ))
            .unwrap();
        observed.verify().unwrap();
        assert!(observed.event.source.starts_with("ephemeral-hold/"));
        let original = serde_json::to_vec(&observed).unwrap();
        worker
            .provenance
            .step(
                Arc::make_mut(&mut worker.index),
                &bus,
                runtime.handle(),
                &worker.identity,
                &[],
            )
            .unwrap();
        assert!(worker.index.current(&path).unwrap().is_none());
        assert!(
            worker
                .index
                .activity()
                .after(0, 128)
                .unwrap()
                .iter()
                .any(|row| row.event.payload["reason"] == "artifact-projection-signing-held")
        );
        let mut last = start;
        keys.held.store(false, Ordering::Release);
        *clock.lock().unwrap() = start + Duration::from_secs(30) - Duration::from_nanos(1);
        assert!(
            !worker
                .refresh_identity_due(&mut last, || *clock.lock().unwrap())
                .unwrap()
        );
        assert_eq!(
            worker.identity.status().unwrap().state,
            IdentityState::Pending
        );
        *clock.lock().unwrap() = start + Duration::from_secs(30);
        assert!(
            worker
                .refresh_identity_due(&mut last, || *clock.lock().unwrap())
                .unwrap()
        );
        assert_eq!(
            worker.identity.status().unwrap().state,
            IdentityState::LocalDev
        );
        assert!(
            worker
                .provenance
                .step(
                    Arc::make_mut(&mut worker.index),
                    &bus,
                    runtime.handle(),
                    &worker.identity,
                    &[]
                )
                .unwrap()
        );
        let doc = worker.index.current(&path).unwrap().unwrap();
        assert_eq!(doc.metadata["authorship"]["kind"], "local-dev");
        assert!(
            doc.metadata["history"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["source"] == observed.event.source)
        );
        let restored = worker.index.activity().at(observed.seq).unwrap().unwrap();
        assert_eq!(serde_json::to_vec(&restored).unwrap(), original);
        use base64::Engine;
        let author = base64::engine::general_purpose::STANDARD
            .decode(doc.metadata["authorship"]["public_key"].as_str().unwrap())
            .unwrap();
        assert_ne!(restored.provenance.signer.as_slice(), author);
        let mut tampered = restored;
        tampered.event.payload["observation"]["digest"] = serde_json::json!("sha256:forged");
        assert!(tampered.verify().is_err());
        let page = worker.messaging.list(seat.id, None, None, 0, 16).unwrap();
        let envelopes: Vec<_> = page
            .items
            .iter()
            .map(|row| row.envelope().unwrap())
            .collect();
        assert_eq!(envelopes.iter().map(|m| m.id).collect::<Vec<_>>(), ids);
        assert_eq!(
            envelopes
                .iter()
                .map(|m| m.sender_sequence)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        *clock.lock().unwrap() = start + Duration::from_secs(63) - Duration::from_nanos(1);
        assert!(
            !worker
                .refresh_identity_due(&mut last, || *clock.lock().unwrap())
                .unwrap()
        );
        *clock.lock().unwrap() = start + Duration::from_secs(63);
        assert!(
            worker
                .refresh_identity_due(&mut last, || *clock.lock().unwrap())
                .unwrap()
        );
        assert_eq!(
            worker
                .messaging
                .list(seat.id, None, None, 0, 16)
                .unwrap()
                .items
                .len(),
            2
        );
        drop(worker);
        runtime.block_on(bus.shutdown()).unwrap();
    }
}

#[cfg(test)]
mod native_hook_tests {
    use super::*;
    fn binding() -> crate::adapters::Binding {
        crate::adapters::Binding {
            seat: Ulid::new(),
            session: "private-thread".into(),
            pid: 42,
            process_start: "fixture-start".into(),
            generation: 1,
            attachment_generation: 1,
            runtime: None,
        }
    }
    fn failure() -> crate::adapters::native::HookFailure {
        crate::adapters::native::HookFailure {
            id: "run".into(),
            event: "sessionStart".into(),
            source: "user".into(),
            source_path: "/fixture/hooks.json".into(),
            message: "Hook exited with code 1".into(),
            duration_ms: Some(7),
            exit_code: Some(1),
            stderr: "ambiguous inherited hook session".into(),
        }
    }
    #[test]
    fn a_hook_failure_is_journaled_only_for_its_exact_live_attachment() {
        let bound = binding();
        let row = native_hook_row(Some(&bound), &bound, failure()).unwrap();
        assert_eq!(row.seat, Some(bound.seat.to_string()));
        assert_eq!(row.harness.as_deref(), Some("codex"));
        assert_eq!(row.exit_code, Some(1));
        assert_eq!(row.duration_ms, Some(7));
        assert_eq!(row.stderr, "ambiguous inherited hook session");
        assert!(native_hook_row(None, &bound, failure()).is_none());
        for field in 0..5 {
            let mut newer = bound.clone();
            match field {
                0 => newer.generation += 1,
                1 => newer.attachment_generation += 1,
                2 => newer.session = "replacement-thread".into(),
                3 => newer.pid += 1,
                _ => newer.process_start = "reused-pid".into(),
            }
            assert!(native_hook_row(Some(&newer), &bound, failure()).is_none());
        }
    }
    #[test]
    fn retiring_a_native_reader_cancels_its_observer_task() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                struct Done(Option<oneshot::Sender<()>>);
                impl Drop for Done {
                    fn drop(&mut self) {
                        let _ = self.0.take().unwrap().send(());
                    }
                }
                let (started, ready) = oneshot::channel();
                let (ended, done) = oneshot::channel();
                let task = tokio::spawn(async move {
                    let _done = Done(Some(ended));
                    let _ = started.send(());
                    std::future::pending::<()>().await;
                });
                let reader = NativeReader {
                    binding: binding(),
                    task,
                };
                ready.await.unwrap();
                drop(reader);
                tokio::time::timeout(Duration::from_secs(30), done)
                    .await
                    .unwrap()
                    .unwrap();
            });
    }
}
