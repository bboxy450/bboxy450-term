//! The one admitted GUI connection multiplexes state and watched view updates.
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::seat_state::{GuiSeat, SpawnSpec, text_id};
use crate::{SeatId, Update, View};
pub type Row = Vec<bbt_core::Cell>;

/// The admitted GUI schema, including explicit pending message grades and logical operators.
pub const VERSION: u16 = 3;

/// Ordered capture history. Sequence numbers are capture cursors, never row IDs.
/// Non-output capture records have empty bytes, preserving their cursor slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum HistoryRecord {
    Bytes { sequence: u64, bytes: Vec<u8> },
    Gap { from: u64, to: u64, reason: String },
}

/// Public signing state; custody and secret key material never cross the GUI socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GuiIdentityState {
    LocalDev,
    Pending,
    Certified,
    Held,
}

/// The compact identity note kept in the app's immutable model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiIdentity {
    pub state: GuiIdentityState,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GuiRequest {
    pub version: u16,
    pub capability: [u8; 32],
    pub action: GuiAction,
}

/// Lifecycle operations relayed to the attached GUI owner, including headless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppRequest {
    /// Read the GUI owner's actual readiness, without any capability material.
    Status,
    /// Read the explicitly selected test notification backend, never the OS.
    Notifications,
    /// On-demand sample by the GUI IO owner, with no layout or paint work.
    HistoryMetrics,
    /// Acknowledge the request before beginning orderly app shutdown.
    Stop,
}

/// Public app readiness. No process credential or endpoint capability belongs here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStatus {
    pub running: bool,
    pub mode: String,
    pub unattended: bool,
    pub backend_ready: bool,
    pub gui_ready: bool,
    pub events_ready: bool,
    pub custody_backend: String,
    pub generation: u64,
    pub frame_seq: u64,
    pub windows: usize,
    /// Bounded readback of the retained close/grace UI, absent on older hosts.
    #[serde(default)]
    pub close: serde_json::Value,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub notifications: serde_json::Value,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub history: serde_json::Value,
}
/// The completed frame which acknowledged a focus command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusFrame {
    #[serde(with = "text_id")]
    pub seat: SeatId,
    pub generation: u64,
    pub frame_seq: u64,
}

/// Chrome input, relayed to the GUI owner; never terminal bytes or OS input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body", rename_all = "snake_case")]
pub enum UiRequest {
    Open(UiTarget),
    Key(Vec<String>),
    Focus {
        seat: Ulid,
    },
    Scroll {
        seat: Ulid,
        motion: UiScroll,
    },
    /// Capture a placement in the GUI owner before any seat is launched.
    Spawn(Box<UiSpawn>),
    /// Debug recording backend only; invokes its real response callback.
    NotificationClick {
        seat: Ulid,
    },
    Layout,
    /// `ctl app resize`: the window (offscreen or real) to this grid of terminal cells; the
    /// reply is the first produced layout at that size (ctl-cli §1a), its layout sizing panes.
    Resize {
        cols: u16,
        rows: u16,
    },
    /// Deterministic animation time, admitted only for a verified test account.
    Clock(UiClockCommand),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UiClockCommand {
    Freeze,
    Advance { ms: u64 },
    Resume,
}
/// The animation clock sampled for this produced frame, not a wall timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiClockState {
    pub frozen: bool,
    pub time_ms: u64,
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnPlace {
    Right,
    Down,
    Tab,
    Space,
}

/// Operator inputs only; the GUI supplies the actual tab/anchor/generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSpawn {
    pub harness: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub cwd: String,
    pub swarm: Option<Ulid>,
    pub place: SpawnPlace,
}

/// The captured target of a spawn whose pane the reply's frame contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSpawned {
    pub seat: Ulid,
    pub op: Ulid,
    pub swarm: Ulid,
    pub placement: crate::seat_state::Placement,
    pub layout_revision: u64,
    pub applied_layout_revision: u64,
}
impl UiRequest {
    /// Transport bounds; the window decides whether each chrome key is applicable.
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Spawn(pick)
                if pick.harness.is_empty()
                    || pick.harness.len() > 256
                    || pick.cwd.is_empty()
                    || pick.cwd.len() > 4096
                    || pick.name.as_ref().is_some_and(|name| name.len() > 1024)
                    || pick.role.as_ref().is_some_and(|role| role.len() > 128)
                    || pick.swarm == Some(Ulid::nil()) =>
            {
                Err("invalid_ui_spawn: invalid launch inputs")
            }
            Self::Open(UiTarget::Docs(path))
                if path.is_empty() || path.len() > 4096 || path.chars().any(char::is_control) =>
            {
                Err("invalid_ui_path: expected a printable record locator")
            }
            Self::Key(keys) if keys.is_empty() || keys.len() > 32 => {
                Err("invalid_ui_keys: expected 1-32 named chrome keys")
            }
            Self::Key(keys) if keys.iter().any(|key| !UI_KEYS.contains(&key.as_str())) => {
                Err("invalid_ui_key: unsupported chrome key; use seat key for PTY input")
            }
            Self::Focus { seat } | Self::NotificationClick { seat } if *seat == Ulid::nil() => {
                Err("invalid_ui_focus: expected a seat ID")
            }
            Self::Scroll { seat, .. } if *seat == Ulid::nil() => {
                Err("invalid_ui_scroll: expected a seat ID")
            }
            Self::Resize { cols, rows }
                if !(2..=512).contains(cols) || !(1..=256).contains(rows) =>
            {
                Err("invalid_ui_resize: expected 2-512 columns by 1-256 rows")
            }
            _ => Ok(()),
        }
    }
}
/// The window's existing history navigation, in absolute zero-based row coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body", rename_all = "kebab-case")]
pub enum UiScroll {
    Top,
    Bottom,
    PageUp,
    PageDown,
    Row(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiScrollPosition {
    /// Unknown for an unindexed live view; never a guessed zero.
    pub top_row: Option<u64>,
    pub live: bool,
}
impl Default for UiScrollPosition {
    fn default() -> Self {
        Self {
            top_row: None,
            live: true,
        }
    }
}
/// Named chrome chords accepted by ctl and independently checked by the window.
pub const UI_KEYS: &[&str] = &[
    "cmd-t",
    "cmd-g",
    "cmd-m",
    "cmd-w",
    "cmd-z",
    "cmd-r",
    "cmd-comma",
    "esc",
    "enter",
    "tab",
    "shift-tab",
    "up",
    "down",
    "left",
    "right",
    "cmd-left",
    "cmd-right",
    "cmd-up",
    "cmd-down",
    "cmd-enter",
    "backspace",
    "pageup",
    "pagedown",
    "home",
    "end",
    "cmd-alt-left",
    "cmd-alt-right",
    "cmd-alt-up",
    "cmd-alt-down",
    "cmd-alt-tab",
    "cmd-alt-shift-tab",
];
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "path", rename_all = "snake_case")]
pub enum UiTarget {
    Graph,
    Settings,
    Launcher,
    Docs(String),
}
/// The GUI's solved tree, not a reconstruction by logic or ctl.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiLayout {
    /// The window composition this solved layout describes (not a bus cursor).
    pub frame_seq: u64,
    /// The tree revision encoded in this frame; absent on older hosts.
    #[serde(default)]
    pub layout_revision: Option<u64>,
    /// Absent on an older owner. A retained reply keeps its original frame clock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock: Option<UiClockState>,
    pub size: [f32; 2],
    pub swarm: Option<Ulid>,
    pub tab: Option<Ulid>,
    pub focused: Option<Ulid>,
    pub launcher_open: bool,
    pub page: Option<String>,
    pub document: Option<String>,
    pub panes: Vec<UiPane>,
    pub tree: Vec<UiLayoutNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawned: Option<UiSpawned>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiPane {
    pub seat: Ulid,
    pub rect: [f32; 4],
    pub cols: u16,
    pub rows: u16,
    pub focused: bool,
    #[serde(default)]
    pub scroll: UiScrollPosition,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiLayoutNode {
    pub kind: String,
    pub rect: [f32; 4],
    pub children: Vec<UiLayoutNode>,
}

/// Fixed diagnostic sources; a caller cannot use the debug door to read a path or SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugProcess {
    App,
    Logic,
    Supervisor,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebugRequest {
    Logs {
        process: DebugProcess,
        since: u64,
    },
    /// `account/<op>` or `seat/<seat-id>/<op>`; never an SQL statement.
    Journal {
        query: String,
    },
    Metrics,
    /// Test-account-only retained History projection; original ring/live bytes remain intact.
    RingGap {
        seat: Ulid,
        from: u64,
        to: u64,
    },
    /// Destination is part of the journaled arguments; only the CLI writes the file.
    Snapshot {
        destination: String,
    },
    Ps,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body")]
pub enum GuiAction {
    Ui {
        id: Ulid,
        request: UiRequest,
    },
    /// Only the admitted GUI owner can complete a UI relay.
    UiReply {
        id: Ulid,
        result: Result<UiLayout, String>,
    },
    ListHosts,
    ListSeats,
    /// Bounded diagnostics and test-account fixtures, admitted only through the ctl door.
    Debug {
        id: Ulid,
        request: DebugRequest,
    },
    /// Ask the app owner through the same admitted operator endpoint.
    App {
        id: Ulid,
        request: AppRequest,
    },
    /// Only the GUI owner may answer an app lifecycle relay.
    AppReply {
        id: Ulid,
        status: AppStatus,
    },
    /// The hold's two operator controls; target identity is derived in logic.
    Identity(crate::identity::IdentityCall),
    /// Start the one device-bound durable events drain on this connection.
    AttachEvents {
        id: Ulid,
    },
    GetEvents {
        stream: String,
        #[serde(with = "crate::events::decimal")]
        after: u64,
        limit: usize,
    },
    GetEventCursor {
        stream: String,
    },
    /// ACK follows publication to the model, not a socket write.
    EventAck {
        stream: String,
        #[serde(with = "crate::events::decimal")]
        seq: u64,
    },
    /// Only an operator view/mark-read advances this independent watermark.
    MarkEventsRead {
        stream: String,
        #[serde(with = "crate::events::decimal")]
        seq: u64,
    },
    Watch {
        #[serde(with = "text_id")]
        seat: SeatId,
        view: View,
    },
    Unwatch {
        #[serde(with = "text_id")]
        seat: SeatId,
    },
    Applied {
        #[serde(with = "text_id")]
        seat: SeatId,
        view: View,
        sequence: u64,
    },
    Input {
        #[serde(with = "text_id")]
        seat: SeatId,
        /// Process generation displayed when the operator produced this input.
        generation: u64,
        id: Ulid,
        bytes: Vec<u8>,
    },
    Resize {
        #[serde(with = "text_id")]
        seat: SeatId,
        generation: u64,
        cols: u16,
        rows: u16,
    },
    Spawn {
        id: Ulid,
        spec: SpawnSpec,
    },
    Messaging {
        id: Ulid,
        #[serde(with = "text_id")]
        swarm_id: crate::SwarmId,
        request: crate::messaging::Request,
    },
    Task {
        id: Ulid,
        request: crate::messaging::TaskRequest,
    },
    ScheduleClose {
        #[serde(with = "text_id")]
        seat: SeatId,
        /// Stable operation ID, retained across retries and used by UndoClose.
        id: Ulid,
        /// Exact layout restoration handle captured when close was requested.
        position: crate::registry::Position,
    },
    UndoClose {
        #[serde(with = "text_id")]
        seat: SeatId,
        close_id: Ulid,
        /// Current layout revision; a changed layout must reject restoration.
        layout_revision: u64,
    },
    GetHistory {
        #[serde(with = "text_id")]
        seat: SeatId,
        /// Inclusive capture sequence range, not semantic terminal row numbers.
        from: u64,
        to: u64,
    },
    /// The operator asks for a PNG of the window, or of one seat's pane, written to `path`
    /// (absolute). Answered with [`GuiReply::Captured`] once the window has written it.
    Capture {
        id: Ulid,
        path: String,
        #[serde(default)]
        seat: Option<Ulid>,
    },
    /// The operator reads one seat's visible screen as text (`ctl screen`), from the same
    /// snapshot the window watches.
    Screen {
        id: Ulid,
        seat: Ulid,
        /// With every cell's style, as runs of equal style per row (`ctl seat screen --cells`).
        #[serde(default)]
        cells: bool,
    },
    /// Rename a seat (ctl-cli §1a): a registry write the window reconciles from the state lane.
    RenameSeat {
        id: Ulid,
        seat: Ulid,
        name: String,
    },
    /// Restart a seat (ctl `seat restart`): stop the process of `generation` and relaunch
    /// the next one, as recovery does; harness seats resume by session id.
    RestartSeat {
        id: Ulid,
        seat: Ulid,
        generation: u64,
    },
    /// Close through the window, which captures its solved layout before scheduling grace.
    CloseSeat {
        id: Ulid,
        seat: Ulid,
    },
    /// Restore a window close during grace, through its retained restoration handle.
    UndoCloseSeat {
        id: Ulid,
        seat: Ulid,
    },
    /// Focus in the window, as its own row click does.
    FocusSeat {
        id: Ulid,
        seat: Ulid,
    },
    /// Only the GUI owner may acknowledge an applied focus.
    Focused {
        id: Ulid,
        result: Result<FocusFrame, String>,
    },
    /// Page a retained stream by the caller's cursor (ctl-cli §1a, #272): read-only, no
    /// attachment, no ack, never a second consumer of the operator's lane; filtered to the
    /// subjects the operator's lane may see.
    EventsRead {
        id: Ulid,
        stream: String,
        after: u64,
        limit: usize,
    },
    /// Discover retained stream names through the same operator admission.
    /// This creates no consumer; EventsRead still filters every returned event.
    EventsStreams {
        id: Ulid,
    },
    /// An operator-signed knowledge write of an `e2e` record (`ctl record write`, e2e
    /// acceptance §2): `args` is the knowledge_write DTO (`path` or `ref`, `frontmatter`,
    /// `body`, `etag`). Logic signs it with the operator device under its issuer and journals
    /// it; the reply is `Knowledge` with the serialized data_source result.
    RecordWrite {
        id: Ulid,
        args: serde_json::Value,
    },
    /// Who issues this connection's operator requests (decision #269): `ctl --as LABEL`.
    /// Attribution only, recorded on the envelopes the operator principal signs; it grants
    /// nothing.
    Issuer {
        label: String,
    },
    /// The window's answer to [`GuiReply::CaptureAsk`]: written, or why not.
    Captured {
        id: Ulid,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        frame: CaptureFrame,
    },
    /// Read-only portable data_source call. `args` is the existing method DTO;
    /// admission is the same operator GUI admission as every other view read.
    Knowledge {
        id: Ulid,
        method: String,
        args: serde_json::Value,
    },
}
/// Which frame a capture holds. A stale frame (the last one drawn before the window stopped
/// presenting) is never passed off as current: a judge reads `stale` and refuses it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureFrame {
    pub stale: bool,
    pub occluded: bool,
    /// When the captured frame was presented, in Unix milliseconds.
    pub presented_at_ms: Option<u64>,
    /// The window's presented-frame count for that frame (ctl-cli §1 `frame_seq`).
    #[serde(default)]
    pub frame_seq: u64,
    /// Offscreen frames are GPU-completed, never described as window presentations.
    #[serde(default)]
    pub offscreen: bool,
    /// Completion time of an offscreen composition, in Unix milliseconds.
    #[serde(default)]
    pub rendered_at_ms: Option<u64>,
}
/// The text of one seat's screen, for `ctl screen` and `ctl wait`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatScreen {
    pub seat: Ulid,
    pub generation: u64,
    pub rows: Vec<String>,
    /// Row and column of the cursor in the visible screen.
    pub cursor: (i32, usize),
    pub modes: u64,
    pub title: String,
    #[serde(default)]
    pub cols: usize,
    /// The capture sequence this screen shows; it changes when the screen does.
    #[serde(default)]
    pub sequence: u64,
    /// The cursor shape the program asked for (DECSCUSR), as the window draws it.
    #[serde(default)]
    pub cursor_style: serde_json::Value,
    /// With `cells`: each row as runs of equal style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cells: Option<Vec<Vec<StyledRun>>>,
}
/// One retained event as `EventsRead` pages it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetainedEvent {
    pub seq: u64,
    pub subject: String,
    pub source: String,
    pub ts: u64,
    pub payload: serde_json::Value,
}
/// A run of cells in one row with the same colours and attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyledRun {
    pub text: String,
    pub fg: bbt_core::Color,
    pub bg: bbt_core::Color,
    /// bbt-core cell flags (bold, italic, underline, inverse, …) as their bits.
    pub flags: u16,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body")]
#[allow(
    clippy::large_enum_variant,
    reason = "keep the decided C1 shared API; transport bounds each message"
)]
pub enum GuiReply {
    UiAsk {
        op: Ulid,
        request: UiRequest,
    },
    Ui {
        op: Ulid,
        layout: UiLayout,
    },
    Debug {
        op: Ulid,
        data: serde_json::Value,
    },
    /// A lifecycle query for the attached GUI owner.
    AppAsk {
        op: Ulid,
        request: AppRequest,
    },
    /// The stop request was written back to its requester; shutdown may now begin.
    AppStopping {
        op: Ulid,
    },
    /// The app owner's public readiness snapshot.
    App {
        op: Ulid,
        status: AppStatus,
    },
    /// Asynchronous attachment transitions, never a terminal action result.
    EventStatus {
        attached: bool,
        reason: Option<String>,
    },
    Event {
        stream: String,
        #[serde(with = "crate::events::decimal")]
        seq: u64,
        subject: String,
        envelope: Box<crate::events::GuiEventEnvelope>,
    },
    Gap {
        stream: String,
        #[serde(with = "crate::events::decimal")]
        first: u64,
        #[serde(with = "crate::events::decimal")]
        last: u64,
        reason: String,
    },
    Events {
        page: crate::events::GuiEventPage,
    },
    EventCursor {
        cursor: crate::events::GuiEventCursor,
    },
    Hosts(Vec<crate::GuiHost>),
    HostState {
        host: crate::GuiHost,
    },
    Identity(GuiIdentity),
    Seats(Vec<GuiSeat>),
    Update {
        #[serde(with = "text_id")]
        seat: SeatId,
        update: Update,
    },
    Seat(GuiSeat),
    Spawned {
        op: Ulid,
        #[serde(with = "text_id")]
        seat: SeatId,
    },
    Messaging {
        op: Ulid,
        response: crate::messaging::Response,
    },
    Task {
        op: Ulid,
        response: crate::messaging::TaskResponse,
    },
    /// A correlated control response, separate from the asynchronous identity note.
    IdentityControl {
        op: Ulid,
        response: crate::identity::IdentityControl,
    },
    Ok {
        op: Ulid,
    },
    Error {
        op: Ulid,
        reason: String,
    },
    History {
        #[serde(with = "text_id")]
        seat: SeatId,
        records: Vec<HistoryRecord>,
        /// First capture sequence not included in this bounded page.
        next: Option<u64>,
    },
    /// Sent to the window: capture the next presented frame to `path`, cropped to `seat`'s
    /// pane when named, and answer with [`GuiAction::Captured`].
    CaptureAsk {
        op: Ulid,
        path: String,
        seat: Option<Ulid>,
    },
    /// The PNG the window wrote for [`GuiAction::Capture`], and which frame it holds.
    Captured {
        op: Ulid,
        path: String,
        #[serde(default)]
        frame: CaptureFrame,
    },
    /// Sent to the window: a seat spawned through another operator door (`ctl spawn`).
    /// The window places it as it places its own launcher pick.
    OperatorSpawned {
        op: Ulid,
        #[serde(with = "text_id")]
        seat: SeatId,
        spec: SpawnSpec,
    },
    /// Sent to the window: close this seat using its solved Position.
    OperatorClose {
        op: Ulid,
        #[serde(with = "text_id")]
        seat: SeatId,
    },
    /// Sent to the window: undo this seat's close during grace.
    OperatorUndoClose {
        op: Ulid,
        #[serde(with = "text_id")]
        seat: SeatId,
    },
    /// Sent to the window: focus this seat, as its own row click does.
    OperatorFocus {
        op: Ulid,
        #[serde(with = "text_id")]
        seat: SeatId,
    },
    Focused {
        op: Ulid,
        frame: FocusFrame,
    },
    /// A page of retained events, oldest first; `next` is the cursor to continue from.
    EventsRead {
        op: Ulid,
        stream: String,
        events: Vec<RetainedEvent>,
        next: Option<u64>,
        /// Last scanned sequence, including rows hidden by the subject filter.
        #[serde(default)]
        through: u64,
    },
    EventsStreams {
        op: Ulid,
        streams: Vec<String>,
    },
    /// One seat's visible screen: text rows (trailing blanks trimmed), the cursor, the
    /// terminal modes (keyboard protocol included) and the process generation input targets.
    Screen {
        op: Ulid,
        screen: SeatScreen,
    },
    /// Serialized data_source Result<Value, CommandError>, bounded by the
    /// provider page limit. It carries no new document or error DTO.
    Knowledge {
        op: Ulid,
        response: serde_json::Value,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{registry::SeatRecord, seat_state::*};
    #[test]
    fn identity_status_is_portable_and_contains_only_state_and_reason() {
        for (state, name) in [
            (GuiIdentityState::LocalDev, "local-dev"),
            (GuiIdentityState::Pending, "pending"),
            (GuiIdentityState::Certified, "certified"),
            (GuiIdentityState::Held, "held"),
        ] {
            let identity = GuiIdentity {
                state,
                reason: Some("Keychain unavailable".into()),
            };
            let value = serde_json::to_value(GuiReply::Identity(identity.clone())).unwrap();
            assert_eq!(value["body"]["state"], name);
            assert_eq!(value["body"].as_object().unwrap().len(), 2);
            let GuiReply::Identity(decoded) = serde_json::from_value(value).unwrap() else {
                panic!("identity reply changed kind")
            };
            assert_eq!(identity, decoded);
        }
    }
    #[test]
    fn gui_state_round_trips_portable_enums_without_a_mux_dependency() {
        for activity in [
            Activity::Active,
            Activity::Idle,
            Activity::Errored,
            Activity::Waiting(WaitKind::Permission),
            Activity::Waiting(WaitKind::Question),
            Activity::Waiting(WaitKind::Auth),
        ] {
            for grade in [Grade::Reported, Grade::Shim, Grade::Inferred] {
                for process in [
                    ProcessStatus::Running { pid: 7 },
                    ProcessStatus::Exited { status: 137 },
                    ProcessStatus::ClosedGrace { until_ms: 99 },
                    ProcessStatus::Retained,
                ] {
                    let state = SeatState {
                        generation: 1,
                        process,
                        activity,
                        grade,
                        title: "zsh".into(),
                        detail: None,
                        turn_open: false,
                        observed_ms: 3,
                        subagents: vec![],
                    };
                    let seat = GuiSeat {
                        record: SeatRecord::new(Ulid::from(1), Ulid::from(2), "zsh", "zsh"),
                        state,
                    };
                    let wire = serde_json::to_vec(&GuiReply::Seat(seat.clone())).unwrap();
                    let GuiReply::Seat(decoded) = serde_json::from_slice(&wire).unwrap() else {
                        panic!()
                    };
                    assert_eq!(decoded, seat);
                }
            }
        }
    }
    #[test]
    fn close_readback_is_optional_for_older_app_status_replies() {
        let mut wire = serde_json::json!({
            "running":true,"mode":"headless","unattended":false,
            "backend_ready":true,"gui_ready":true,"events_ready":true,
            "custody_backend":"memory","generation":1,"frame_seq":1,"windows":0
        });
        let legacy: AppStatus = serde_json::from_value(wire.clone()).unwrap();
        assert!(legacy.close.is_null());
        wire["close"] = serde_json::json!({"slots":[],"grace":[],"notices":[],"complete":true});
        let current: AppStatus = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(current).unwrap(), wire);
    }
    #[test]
    fn gui_ids_keep_canonical_ulid_text_and_reject_malformed_ids() {
        let seat = SeatId::from_u128(42);
        let value = serde_json::to_value(GuiAction::Unwatch { seat }).unwrap();
        assert_eq!(value["body"]["seat"], seat.to_string());
        let decoded: GuiAction = serde_json::from_value(value.clone()).unwrap();
        assert!(matches!(decoded,GuiAction::Unwatch {seat:id} if id==seat));
        let mut bad = value;
        bad["body"]["seat"] = serde_json::json!("not-a-seat");
        assert!(serde_json::from_value::<GuiAction>(bad).is_err());
    }
    #[test]
    fn input_and_resize_require_an_explicit_displayed_generation() {
        let seat = SeatId::from_u128(42);
        for action in [
            GuiAction::Input {
                seat,
                generation: 7,
                id: Ulid::new(),
                bytes: b"hi\r".to_vec(),
            },
            GuiAction::Resize {
                seat,
                generation: 7,
                cols: 80,
                rows: 24,
            },
        ] {
            let mut value = serde_json::to_value(action).unwrap();
            assert_eq!(value["body"]["generation"], 7);
            assert!(serde_json::from_value::<GuiAction>(value.clone()).is_ok());
            value["body"].as_object_mut().unwrap().remove("generation");
            assert!(serde_json::from_value::<GuiAction>(value).is_err());
        }
    }
}

#[cfg(test)]
mod ui_tests {
    use super::*;
    #[test]
    fn scroll_rows_stay_unsigned_and_live_positions_can_be_unindexed() {
        let seat = Ulid::new();
        for motion in [
            UiScroll::Top,
            UiScroll::Bottom,
            UiScroll::PageUp,
            UiScroll::PageDown,
            UiScroll::Row(u64::MAX),
        ] {
            let request = UiRequest::Scroll { seat, motion };
            assert!(request.validate().is_ok());
            let value = serde_json::to_value(&request).unwrap();
            assert_eq!(serde_json::from_value::<UiRequest>(value).unwrap(), request);
        }
        assert!(
            UiRequest::Scroll {
                seat: Ulid::nil(),
                motion: UiScroll::Top
            }
            .validate()
            .is_err()
        );
        let legacy: UiPane = serde_json::from_value(serde_json::json!({"seat":seat,"rect":[0.,0.,80.,24.],"cols":80,"rows":24,"focused":true})).unwrap();
        assert_eq!(legacy.scroll, UiScrollPosition::default());
        assert_eq!(
            serde_json::to_value(UiScrollPosition::default()).unwrap(),
            serde_json::json!({"top_row":null,"live":true})
        );
        assert!(
            serde_json::from_value::<UiScroll>(serde_json::json!({"kind":"row","body":-1}))
                .is_err()
        );
    }
    #[test]
    fn ui_requests_keep_their_scope_and_reject_unbounded_input() {
        let request = UiRequest::Key(vec!["cmd-t".into(), "esc".into()]);
        assert_eq!(
            serde_json::from_value::<UiRequest>(serde_json::to_value(&request).unwrap()).unwrap(),
            request
        );
        assert!(request.validate().is_ok());
        for request in [
            UiRequest::Key(vec![]),
            UiRequest::Key(vec!["cmd-t".into(); 33]),
            UiRequest::Key(vec!["text into terminal".into()]),
            UiRequest::Focus { seat: Ulid::nil() },
            UiRequest::Open(UiTarget::Docs("x\n".into())),
        ] {
            assert!(request.validate().is_err());
        }
    }
}

#[cfg(test)]
mod spawn_pick_tests {
    use super::*;
    #[test]
    fn spawn_pick_wire_is_bounded_and_old_layouts_keep_revision_unknown() {
        let pick = UiSpawn {
            harness: "zsh".into(),
            name: None,
            role: None,
            cwd: "/fixture".into(),
            swarm: None,
            place: SpawnPlace::Right,
        };
        let request = UiRequest::Spawn(Box::new(pick.clone()));
        request.validate().unwrap();
        assert_eq!(
            serde_json::from_value::<UiRequest>(serde_json::to_value(&request).unwrap()).unwrap(),
            request
        );
        let mut oversized = pick.clone();
        oversized.cwd = "x".repeat(4097);
        assert!(UiRequest::Spawn(Box::new(oversized)).validate().is_err());
        let mut invalid = pick;
        invalid.swarm = Some(Ulid::nil());
        assert!(UiRequest::Spawn(Box::new(invalid)).validate().is_err());
        let old:UiLayout=serde_json::from_value(serde_json::json!({"frame_seq":4,"size":[100,100],"swarm":null,"tab":null,"focused":null,"launcher_open":false,"page":null,"document":null,"panes":[],"tree":[]})).unwrap();
        assert_eq!(old.layout_revision, None);
        assert_eq!(old.spawned, None);
    }
}
