// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Window chrome relayed to the GUI owner: `ctl ui …` and the frame fence's layouts.
use super::*;

/// Logical input verbs; old logic is refreshed without replacing live PTYs.
/// Versions 3/4 add image paste and captured seat controls; version 5 adds
/// terminal selection/copy and private test clipboard receipts. Version 6 adds
/// independent chat/task section wheel controls and produced-frame scroll receipts.
/// Version 7 adds the headless recording account's logical notification foreground control.
/// Version 8 adds bounded private native document paint observations.
/// Version 10 adds saved Tasks/Chat folds; version 11 adds the Cmd-V logical chord.
pub const UI_CONTROL_VERSION: u16 = 11;
pub const MAX_SECTION_WHEEL_DY: f32 = 32768.;

/// The completed frame which acknowledged a focus command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusFrame {
    #[serde(with = "text_id")]
    pub seat: SeatId,
    pub generation: u64,
    pub frame_seq: u64,
}

/// Chrome input, relayed to the GUI owner; never terminal bytes or OS input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body", rename_all = "snake_case")]
pub enum UiRequest {
    /// Set a swarm's default launch folder; None restores the app default.
    SwarmWorkdir {
        swarm: Ulid,
        path: Option<String>,
    },
    Open(UiTarget),
    Key(Vec<String>),
    /// Logical window coordinates through the native pointer handler.
    Click {
        x: u32,
        y: u32,
    },
    /// Logical secondary-button click; existing Click remains the left-button wire.
    RightClick {
        x: u32,
        y: u32,
    },
    /// Logical pointer drag through a retained supported resize handle.
    Drag {
        from: [u32; 2],
        to: [u32; 2],
    },
    /// Terminal drag through the real pointer selection owner. Shift overrides PTY mouse mode.
    TerminalSelect {
        from: [u32; 2],
        to: [u32; 2],
        shift: bool,
    },
    /// Same action as the native Edit > Copy menu.
    Copy,
    /// Private test-account clipboard provider only; never reads the system clipboard.
    ClipboardRead,
    /// Wheel input in the retained sidebar viewport.
    SidebarWheel {
        lines: i32,
    },
    LauncherWheel {
        lines: i32,
    },
    /// Logical pixels in the chat section; positive moves toward newer messages.
    ChatWheel { dy: f32 },
    /// Logical pixels in the independent task section; positive moves downward.
    TaskWheel { dy: f32 },
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
    /// Debug headless recording test accounts only; never changes OS focus.
    NotificationForeground {
        frontmost: bool,
    },
    /// The GUI's retained real-seat metadata and operation-keyed starting panes.
    /// Reading this never waits for a harness launch or a logic-worker query.
    Seats,
    Layout,
    /// `ctl app resize`: the window (offscreen or real) to this grid of terminal cells; the
    /// reply is the first produced layout at that size (ctl-cli §1a), its layout sizing panes.
    Resize {
        cols: u16,
        rows: u16,
    },
    /// Deterministic animation time, admitted only for a verified test account.
    Clock(UiClockCommand),
    /// One launch-dispatch barrier for a verified test-account owner.
    SpawnGate(UiSpawnGateCommand),
    /// Send through the window's retained composer, not a direct messaging call.
    Compose(UiCompose),
    /// Copy PNG/TIFF bytes from an explicit operator path into private composer custody.
    PasteImage { path: String },
}

impl UiRequest {
    /// Transport bounds; the window decides whether each chrome key is applicable.
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::ChatWheel { dy } | Self::TaskWheel { dy }
                if !dy.is_finite() || dy.abs() > MAX_SECTION_WHEEL_DY =>
            {
                Err("invalid_ui_section_wheel: expected finite logical pixels within +/-32768")
            }
            Self::SidebarWheel { lines } | Self::LauncherWheel { lines }
                if lines.unsigned_abs() > 1000 =>
            {
                Err("invalid_ui_wheel: expected at most 1000 lines")
            }
            Self::Click { x, y } | Self::RightClick { x, y } if *x > 32768 || *y > 32768 => {
                Err("invalid_ui_click: coordinates exceed window bounds")
            }
            Self::Drag { from, to } | Self::TerminalSelect { from, to, .. }
                if from.iter().chain(to).any(|v| *v > 32768) =>
            {
                Err("invalid_ui_drag: coordinates exceed window bounds")
            }
            Self::SwarmWorkdir { swarm, path }
                if *swarm == Ulid::nil()
                    || path.as_deref().is_some_and(|path| !valid_workdir(path)) =>
            {
                Err("invalid_swarm_workdir: expected a swarm and an absolute folder path")
            }
            Self::Compose(compose) => compose.validate(),
            Self::PasteImage { path } if !valid_workdir(path) => {
                Err("invalid_ui_image_path: expected a bounded absolute printable image path")
            },
            Self::Spawn(pick)
                if pick.harness.is_empty()
                    || pick.harness.len() > 256
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
                Err("invalid_ui_keys: expected 1-32 chrome keys")
            }
            Self::Key(keys)
                if keys
                    .iter()
                    .any(|key| !UI_KEYS.contains(&key.as_str()) && !ui_text_key(key)) =>
            {
                Err("invalid_ui_key: unsupported chrome key; use seat key for PTY input")
            }
            Self::Focus { seat } | Self::NotificationClick { seat } if *seat == Ulid::nil() => {
                Err("invalid_ui_focus: expected a seat ID")
            }
            Self::Scroll { seat, .. } if *seat == Ulid::nil() => {
                Err("invalid_ui_scroll: expected a seat ID")
            }
            Self::SpawnGate(UiSpawnGateCommand::Release { op }) if *op == Ulid::nil() => {
                Err("invalid_spawn_gate: expected the held operation ID")
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

pub fn valid_workdir(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && std::path::Path::new(path).is_absolute()
        && !path.chars().any(char::is_control)
}

/// Controls only the next admitted launch; it never pauses an IO owner or changes time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiSpawnGateCommand {
    Arm,
    Release { op: Ulid },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum UiComposeTarget {
    Seat(Ulid),
    Role(String),
    Channel(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiCompose {
    pub to: UiComposeTarget,
    pub text: String,
}

impl UiCompose {
    pub fn validate(&self) -> Result<(), &'static str> {
        let invalid_target = match &self.to {
            UiComposeTarget::Seat(id) => *id == Ulid::nil(),
            UiComposeTarget::Role(name) => {
                name.trim().is_empty() || name.len() > 128 || name.chars().any(char::is_control)
            }
            UiComposeTarget::Channel(name) => {
                name.trim().is_empty() || name.len() > 512 || name.chars().any(char::is_control)
            }
        };
        if invalid_target {
            return Err("invalid_ui_compose_target: expected a seat, role or channel");
        }
        if self.text.trim().is_empty() || self.text.len() > crate::messaging::MAX_TEXT {
            return Err("invalid_ui_compose_text: expected nonempty bounded message text");
        }
        Ok(())
    }
}

/// The sent row in this produced frame, bound to the composer's real message IDs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiComposed {
    pub to: UiComposeTarget,
    pub text: String,
    pub messages: Vec<Ulid>,
    pub label: String,
    pub rect: [f32; 4],
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiSpawned {
    pub seat: Ulid,
    pub op: Ulid,
    pub swarm: Ulid,
    pub placement: crate::seat_state::Placement,
    pub layout_revision: u64,
    pub applied_layout_revision: u64,
    /// Observed at completion of the first rendered starting pane, never predicted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting: Option<UiStartingFrame>,
}

/// Decision #340: an operation's presentation has no seat identity or authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiStartingPane {
    pub op: Ulid,
    pub name: String,
    pub harness_kind: String,
    pub swarm: Ulid,
    pub placement: crate::seat_state::Placement,
    pub state: UiStartingState,
    pub rect: [f32; 4],
    pub focused: bool,
    pub admitted_frame_seq: u64,
    pub first_frame_seq: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiStartingState {
    Starting,
    Refused { reason: String },
}

/// The first completed starting frame retained until the real-seat spawn reply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiStartingFrame {
    pub admitted_frame_seq: u64,
    pub frame_seq: u64,
    pub rect: [f32; 4],
    pub name: String,
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
    "cmd-c",
    "cmd-v",
    "cmd-t",
    "cmd-d",
    "cmd-shift-d",
    "cmd-a",
    "cmd-shift-enter",
    "alt-enter",
    "cmd-g",
    "cmd-m",
    "cmd-minus",
    "cmd-equal",
    "cmd-plus",
    "cmd-shift-equal",
    "cmd-shift-plus",
    "cmd-w",
    "cmd-z",
    "cmd-r",
    "cmd-comma",
    "esc",
    "enter",
    "tab",
    "menu",
    "shift-f10",
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

/// A literal query keystroke is one non-control Unicode scalar. Named chords
/// remain in [`UI_KEYS`]; neither form grants permission to type into a PTY.
pub fn ui_text_key(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|c| !c.is_control()) && chars.next().is_none()
}

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
    /// Private recording account state sampled with this produced frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notification_frontmost: Option<bool>,
    /// Visible retained input regions in logical window coordinates.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub regions: std::collections::BTreeMap<String, [f32; 4]>,
    /// Present only on an explicit verified test-account ClipboardRead receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_clipboard: Option<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub swarm_workdirs: std::collections::BTreeMap<Ulid, String>,
    /// Produced-frame expansion choices: swarm:<ULID>, providers/Hosts, providers/Cloud, panels/Tasks and panels/Chat.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub sidebar_open: std::collections::BTreeMap<String, bool>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launcher_params: Option<UiLauncherParams>,
    pub page: Option<String>,
    pub document: Option<String>,
    /// Debug/headless private-fixture document facts from this produced frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_observation: Option<UiDocumentObservation>,
    pub panes: Vec<UiPane>,
    pub tree: Vec<UiLayoutNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawned: Option<UiSpawned>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composed: Option<UiComposed>,
    /// Only the explicit paste-image reply carries its newly owned file reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pasted_image: Option<UiImagePaste>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub starting: Vec<UiStartingPane>,
    /// Included only for `UiRequest::Seats`; ordinary layout replies keep it absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats: Option<Vec<GuiSeat>>,
    /// Actual clamped section geometry sampled from this produced frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sidebar_sections: Option<UiSidebarSections>,
}

/// A section's actual pixel offset and extent; following is a chat behavior flag.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UiSectionScrollFields")]
pub struct UiSectionScroll { pub offset: f32, pub max: f32, pub following: bool }
impl UiSectionScroll {
    pub fn valid(self) -> bool { self.offset.is_finite() && self.max.is_finite() && self.offset >= 0. && self.max >= 0. && self.offset <= self.max }
}
#[derive(Deserialize)]
struct UiSectionScrollFields { offset: f32, max: f32, following: bool }
impl TryFrom<UiSectionScrollFields> for UiSectionScroll {
    type Error = &'static str;
    fn try_from(fields: UiSectionScrollFields) -> Result<Self,Self::Error> {
        let value = Self {offset:fields.offset,max:fields.max,following:fields.following};
        if value.valid() {Ok(value)} else {Err("invalid_ui_section_scroll: expected finite clamped offsets")}
    }
}
/// Independent chat/task section receipts. Tasks never inherit chat tail following.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UiSidebarSectionsFields")]
pub struct UiSidebarSections { pub chat: UiSectionScroll, pub tasks: UiSectionScroll }
impl UiSidebarSections {
    pub fn valid(self) -> bool { self.chat.valid() && self.tasks.valid() && !self.tasks.following }
}
#[derive(Deserialize)]
struct UiSidebarSectionsFields { chat: UiSectionScroll, tasks: UiSectionScroll }
impl TryFrom<UiSidebarSectionsFields> for UiSidebarSections {
    type Error = &'static str;
    fn try_from(fields: UiSidebarSectionsFields) -> Result<Self,Self::Error> {
        let value = Self {chat:fields.chat,tasks:fields.tasks};
        if value.valid() {Ok(value)} else {Err("invalid_ui_sidebar_sections: tasks do not follow chat")}
    }
}

/// Completed private PNG copy, bound to the marker in the produced composer frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiImagePaste { pub id: String, pub marker: String, pub path: String }

/// Resolved launcher values, sampled with the produced frame for ctl assertions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiLauncherParams {
    pub launch: String,
    pub fields: Vec<UiSpawnField>,
    pub runs: String,
    pub editing: Option<String>,
    pub choosing: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSpawnField {
    pub label: String,
    pub value: String,
    pub source: String,
    pub fixed: bool,
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
    /// The exact live grid encoded in this produced frame; absent for old hosts,
    /// unavailable grids and historical displays. This is not a logic screen read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendered: Option<UiRenderedScreen>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiRenderedScreen {
    pub view: crate::View,
    pub sequence: u64,
    /// Omitted when the shared text budget cannot hold the complete grid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<String>>,
    /// Device-pixel dimensions of the exact prepared grid, before pane clipping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pixel_size: Option<[u32; 2]>,
    pub ingested: UiScreenIngestion,
}

/// UI-owner ingestion, counted against successful frame submissions. The first
/// frame keeps the source it actually drew even if a later frame catches up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiScreenIngestion {
    pub view: crate::View,
    pub sequence: u64,
    pub after_frame_seq: u64,
    pub first_frame_seq: u64,
    pub first_drawn_view: crate::View,
    pub first_drawn_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiLayoutNode {
    pub kind: String,
    pub rect: [f32; 4],
    pub children: Vec<UiLayoutNode>,
    /// Positioned text that paints in this produced node. Older hosts and
    /// observations exceeding the text budget omit it; absence is not empty text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

pub const DOCUMENT_TEXT_RUNS: usize = 128;
pub const DOCUMENT_PATHS: usize = 128;
pub const DOCUMENT_PATH_COMMANDS: usize = 2048;
pub const DOCUMENT_CLIPS: usize = 128;
pub const DOCUMENT_OBSERVATION_BYTES: usize = 64 * 1024;

/// Provider-bound identity of the document whose retained paint was submitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiDocumentSource {
    pub record_id: String,
    pub number: Option<u64>,
    pub path: String,
    pub digest: String,
    pub etag: String,
    pub workspace: SwarmScope,
    pub root: String,
    pub provider_revision: u64,
}

/// Coordinates are logical window coordinates, after document scrolling.
/// The clip is the intersection of the production paint's ancestor clips.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiDocumentText {
    pub key: String,
    pub text: String,
    pub origin: [f32; 2],
    pub rect: [f32; 4],
    pub baseline: Option<f32>,
    pub font_size: f32,
    pub line_height: f32,
    pub weight: u16,
    pub mono: bool,
    pub italic: bool,
    #[serde(default)]
    pub tracking_em: f32,
    pub max_width: Option<f32>,
    pub clip: Option<[f32; 4]>,
    /// Indices into this observation's shared actual rounded-clip table.
    #[serde(default, deserialize_with = "document_clip_indices")]
    pub clips: Vec<u16>,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", content = "points", rename_all = "kebab-case")]
pub enum UiDocumentPathCommand {
    Move([f32; 2]),
    Line([f32; 2]),
    Cubic([[f32; 2]; 3]),
    Close,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiDocumentClip {
    pub rect: [f32; 4],
    pub radii: [f32; 4],
}

/// Commands are the submitted native path in local coordinates; the effective
/// window point is origin + scale * point, exactly as for production painting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiDocumentPath {
    pub key: String,
    pub origin: [f32; 2],
    pub scale: f32,
    #[serde(deserialize_with = "document_commands")]
    pub commands: Vec<UiDocumentPathCommand>,
    pub stroke_width: f32,
    pub clip: Option<[f32; 4]>,
    #[serde(default, deserialize_with = "document_clip_indices")]
    pub clips: Vec<u16>,
    pub color: [f32; 4],
    pub fill: Option<[f32; 4]>,
}

/// Counts are omitted facts encountered within the bounded walk. If the walk
/// itself was cut short, these are lower bounds, never invented exact totals.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiDocumentOmitted {
    pub text_runs: u32,
    pub paths: u32,
    pub path_commands: u32,
    pub bytes: u32,
    pub walk_truncated: bool,
    pub unsupported_paint: bool,
}
impl UiDocumentOmitted {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "DocumentObservationWire")]
pub struct UiDocumentObservation {
    pub frame_seq: u64,
    pub private_test_run: Ulid,
    pub source: UiDocumentSource,
    pub clips: Vec<UiDocumentClip>,
    pub texts: Vec<UiDocumentText>,
    pub paths: Vec<UiDocumentPath>,
    pub omitted: UiDocumentOmitted,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentObservationWire {
    frame_seq: u64,
    private_test_run: Ulid,
    source: UiDocumentSource,
    #[serde(deserialize_with = "document_clips")]
    clips: Vec<UiDocumentClip>,
    #[serde(deserialize_with = "document_texts")]
    texts: Vec<UiDocumentText>,
    #[serde(deserialize_with = "document_paths")]
    paths: Vec<UiDocumentPath>,
    omitted: UiDocumentOmitted,
}
fn document_bounded<'de, D, T, const N: usize>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Bounded<T, const N: usize>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const N: usize> serde::de::Visitor<'de> for Bounded<T, N> {
        type Value = Vec<T>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "at most {N} document facts")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(N));
            while let Some(value) = seq.next_element()? {
                if values.len() == N {
                    return Err(serde::de::Error::custom(
                        "document observation count exceeds its bound",
                    ));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(Bounded::<T, N>(std::marker::PhantomData))
}
fn document_texts<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<UiDocumentText>, D::Error> {
    document_bounded::<D, _, DOCUMENT_TEXT_RUNS>(d)
}
fn document_paths<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<UiDocumentPath>, D::Error> {
    document_bounded::<D, _, DOCUMENT_PATHS>(d)
}
fn document_commands<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Vec<UiDocumentPathCommand>, D::Error> {
    document_bounded::<D, _, DOCUMENT_PATH_COMMANDS>(d)
}
fn document_clips<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<UiDocumentClip>, D::Error> {
    document_bounded::<D, _, DOCUMENT_CLIPS>(d)
}
fn document_clip_indices<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u16>, D::Error> {
    document_bounded::<D, _, 16>(d)
}
impl TryFrom<DocumentObservationWire> for UiDocumentObservation {
    type Error = String;
    fn try_from(value: DocumentObservationWire) -> Result<Self, Self::Error> {
        let observation = Self {
            frame_seq: value.frame_seq,
            private_test_run: value.private_test_run,
            source: value.source,
            clips: value.clips,
            texts: value.texts,
            paths: value.paths,
            omitted: value.omitted,
        };
        observation.validate()?;
        Ok(observation)
    }
}
impl UiDocumentObservation {
    pub fn validate(&self) -> Result<(), String> {
        let bad = || "invalid or oversized document observation".to_owned();
        let word = |value: &str, max: usize| {
            !value.is_empty() && value.len() <= max && !value.contains('\0')
        };
        let finite = |values: &[f32]| values.iter().all(|value| value.is_finite());
        let rect = |r: &[f32; 4]| finite(r) && r[2] >= 0. && r[3] >= 0.;
        let clip = |r: Option<[f32; 4]>| r.is_none_or(|r| rect(&r));
        let s = &self.source;
        if self.frame_seq == 0
            || self.private_test_run == Ulid::nil()
            || !word(&s.record_id, 128)
            || !word(&s.path, 1024)
            || !word(&s.digest, 128)
            || !word(&s.etag, 128)
            || !word(&s.root, 128)
            || s.workspace.swarm == Ulid::nil()
            || s.number == Some(0)
            || self.texts.len() > DOCUMENT_TEXT_RUNS
            || self.paths.len() > DOCUMENT_PATHS
            || self.clips.len() > DOCUMENT_CLIPS
            || self.paths.iter().map(|p| p.commands.len()).sum::<usize>() > DOCUMENT_PATH_COMMANDS
        {
            return Err(bad());
        }
        if self
            .clips
            .iter()
            .any(|c| !rect(&c.rect) || !finite(&c.radii) || c.radii.iter().any(|r| *r < 0.))
        {
            return Err(bad());
        }
        for t in &self.texts {
            if !word(&t.key, 512)
                || t.text.len() > 16 * 1024
                || !finite(&t.origin)
                || !rect(&t.rect)
                || !clip(t.clip)
                || !finite(&t.color)
                || t.baseline.is_some_and(|v| !v.is_finite())
                || !t.font_size.is_finite()
                || t.font_size <= 0.
                || !t.line_height.is_finite()
                || t.line_height <= 0.
                || !t.tracking_em.is_finite()
                || t.clips.len() > 16
                || t.clips
                    .iter()
                    .any(|index| usize::from(*index) >= self.clips.len())
                || t.max_width.is_some_and(|v| !v.is_finite() || v < 0.)
            {
                return Err(bad());
            }
        }
        for p in &self.paths {
            if !word(&p.key, 512)
                || !finite(&p.origin)
                || !clip(p.clip)
                || !finite(&p.color)
                || p.fill.is_some_and(|c| !finite(&c))
                || !p.scale.is_finite()
                || p.scale <= 0.
                || !p.stroke_width.is_finite()
                || p.stroke_width < 0.
                || p.clips.len() > 16
                || p.clips
                    .iter()
                    .any(|index| usize::from(*index) >= self.clips.len())
            {
                return Err(bad());
            }
            for command in &p.commands {
                let valid = match command {
                    UiDocumentPathCommand::Move(v) | UiDocumentPathCommand::Line(v) => finite(v),
                    UiDocumentPathCommand::Cubic(v) => v.iter().all(|v| finite(v)),
                    UiDocumentPathCommand::Close => true,
                };
                if !valid {
                    return Err(bad());
                }
            }
        }
        if serde_json::to_vec(self).map_err(|_| bad())?.len() > DOCUMENT_OBSERVATION_BYTES {
            return Err(bad());
        }
        Ok(())
    }
}

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! ui_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Ui {
                id: Ulid,
                request: UiRequest,
            },
            /// Only the admitted GUI owner can complete a UI relay.
            UiReply {
                id: Ulid,
                result: Box<Result<UiLayout, String>>,
            },
        ] $($head)*);
    };
}
pub(super) use ui_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! ui_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            UiAsk {
                op: Ulid,
                request: UiRequest,
            },
            Ui {
                op: Ulid,
                layout: UiLayout,
            },
        ] $($head)*);
    };
}
pub(super) use ui_replies;

#[cfg(test)]
mod sidebar_layout_compatibility {
    use super::*;

    #[test]
    fn old_layouts_omit_expansion_choices_and_new_choices_roundtrip() {
        let mut layout: UiLayout = serde_json::from_value(serde_json::json!({
            "frame_seq": 1,
            "size": [1280., 720.],
            "swarm": null,
            "tab": null,
            "focused": null,
            "launcher_open": false,
            "page": null,
            "document": null,
            "panes": [],
            "tree": []
        }))
        .unwrap();
        assert!(layout.sidebar_open.is_empty());
        assert!(
            serde_json::to_value(&layout)
                .unwrap()
                .get("sidebar_open")
                .is_none()
        );
        layout.sidebar_open.insert("providers/Hosts".into(), false);
        layout.sidebar_open.insert("providers/Cloud".into(), true);
        layout
            .sidebar_open
            .insert(format!("swarm:{}", Ulid::from(1u128)), false);
        let decoded: UiLayout =
            serde_json::from_slice(&serde_json::to_vec(&layout).unwrap()).unwrap();
        assert_eq!(decoded.sidebar_open, layout.sidebar_open);
    }
}
