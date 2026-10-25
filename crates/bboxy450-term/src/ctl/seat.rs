//! Seats: listing, input, screens, capture and lifecycle: `ctl seat …`.

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("seat", "ls", "ListSeats"),
    ("seat", "spawn", "Spawn"),
    ("seat", "screen", "Screen"),
    ("seat", "type", "Input"),
    ("seat", "key", "Input"),
    ("seat", "paste", "Input"),
    ("seat", "wait", "Screen"),
    ("seat", "resize", "Resize"),
    ("seat", "watch", "EventsRead"),
    ("seat", "rename", "RenameSeat"),
    ("seat", "focus", "FocusSeat"),
    ("seat", "restart", "RestartSeat"),
    ("seat", "close", "CloseSeat"),
    ("seat", "undo-close", "UndoCloseSeat"),
    ("seat", "capture", "Capture"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! seat_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "seat": [
            Spawn {
                harness: String,
                name: Option<String>,
                cwd: Option<PathBuf>,
                swarm: Option<Ulid>,
                /// A fresh tab; without it the backend places the seat as the launcher's default.
                new_tab: bool,
                /// A role preset (#262); the backend resolves and guards it as the launcher does.
                role: Option<String>,
            },
            /// An explicit placement is captured by the attached GUI, never guessed here.
            SpawnAt(Box<bbt_proto::gui::UiSpawn>),
            Seats,
            Capture {
                path: PathBuf,
                seat: Option<String>,
            },
            /// `seat screen` (ctl-cli §1, decision #270): what the operator would see in its pane.
            Screen {
                seat: String,
                human: bool,
                /// `--scrollback A..B`: the history rows of that capture range (#27).
                scrollback: Option<(u64, u64)>,
                cells: bool,
            },
            /// `seat watch <seat> [--timeout MS]`: its state from the retained stream, its screen when
            /// the screen's sequence moves; event lines, then a done line.
            Watch {
                seat: String,
                timeout: Duration,
            },
            /// `seat rename <seat> <name>`.
            Rename {
                seat: String,
                name: String,
            },
            /// `seat restart <seat>`: the seat's next process generation.
            Restart {
                seat: String,
            },
            /// `seat focus <seat>`: the window focuses it.
            Focus {
                seat: String,
            },
            /// `seat resize <seat> <cols>x<rows>`.
            Resize {
                seat: String,
                cols: u16,
                rows: u16,
            },
            /// `seat type`, `seat key`, `seat paste` (ctl-cli rev. 2): bytes for the window's one
            /// input path, from literal text, named keys, or a bracketed paste.
            Type {
                seat: String,
                text: String,
            },
            Key {
                seat: String,
                keys: Vec<String>,
            },
            Paste {
                seat: String,
                source: String,
            },
            /// `seat wait`: until the seat is in a state or its screen matches, or the timeout.
            Wait {
                seat: String,
                state: Option<String>,
                text: Option<String>,
                /// `--on-screen`: the seat has a pane in the window's current frame (ctl-cli §1a).
                on_screen: bool,
                /// `--frame-seq N`: the window has produced frame N.
                frame_seq: Option<u64>,
                timeout: Duration,
            },
            /// `app endpoint`: the run directory, socket path and generation; never the capability.
            Close {
                seat: String,
            },
            UndoClose {
                seat: String,
            },
        ]] $($head)*);
    };
}
pub(super) use seat_commands;
