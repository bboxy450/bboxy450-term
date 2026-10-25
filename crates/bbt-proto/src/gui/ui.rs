//! Window chrome relayed to the GUI owner: `ctl ui …` and the frame fence's layouts.
use super::*;

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
    /// Send through the window's retained composer, not a direct messaging call.
    Compose(UiCompose),
}

impl UiRequest {
    /// Transport bounds; the window decides whether each chrome key is applicable.
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Compose(compose) => compose.validate(),
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSpawned {
    pub seat: Ulid,
    pub op: Ulid,
    pub swarm: Ulid,
    pub placement: crate::seat_state::Placement,
    pub layout_revision: u64,
    pub applied_layout_revision: u64,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composed: Option<UiComposed>,
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
                result: Result<UiLayout, String>,
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
