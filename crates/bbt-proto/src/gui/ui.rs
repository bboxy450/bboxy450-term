// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Window chrome relayed to the GUI owner: `ctl ui …` and the frame fence's layouts.
use super::*;

/// Logical input verbs; old logic is refreshed without replacing live PTYs.
/// Includes the explicit operator image-paste relay added in version 3.
/// Version 5 adds terminal selection/copy and private test clipboard receipts.
pub const UI_CONTROL_VERSION: u16 = 5;

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
    /// Visible retained input regions in logical window coordinates.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub regions: std::collections::BTreeMap<String, [f32; 4]>,
    /// Present only on an explicit verified test-account ClipboardRead receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_clipboard: Option<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub swarm_workdirs: std::collections::BTreeMap<Ulid, String>,
    /// Produced-frame expansion choices: swarm:<ULID>, providers/Hosts and providers/Cloud.
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
