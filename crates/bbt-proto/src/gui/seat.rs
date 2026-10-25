//! Seats: their state, input, watched views, history, capture and lifecycle.
use super::*;

/// Ordered capture history. Sequence numbers are capture cursors, never row IDs.
/// Non-output capture records have empty bytes, preserving their cursor slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum HistoryRecord {
    Bytes { sequence: u64, bytes: Vec<u8> },
    Gap { from: u64, to: u64, reason: String },
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

/// A run of cells in one row with the same colours and attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyledRun {
    pub text: String,
    pub fg: bbt_core::Color,
    pub bg: bbt_core::Color,
    /// bbt-core cell flags (bold, italic, underline, inverse, …) as their bits.
    pub flags: u16,
}

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! seat_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            ListSeats,
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
            /// The window's answer to [`GuiReply::CaptureAsk`]: written, or why not.
            Captured {
                id: Ulid,
                #[serde(default)]
                error: Option<String>,
                #[serde(default)]
                frame: CaptureFrame,
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
        ] $($head)*);
    };
}
pub(super) use seat_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! seat_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
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
            /// One seat's visible screen: text rows (trailing blanks trimmed), the cursor, the
            /// terminal modes (keyboard protocol included) and the process generation input targets.
            Screen {
                op: Ulid,
                screen: SeatScreen,
            },
        ] $($head)*);
    };
}
pub(super) use seat_replies;
