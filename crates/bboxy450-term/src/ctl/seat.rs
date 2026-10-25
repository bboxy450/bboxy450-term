//! Seats: listing, input, screens, capture and lifecycle: `ctl seat …`.
use super::*;

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
                subagent: Option<String>,
                grade: Option<String>,
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

/// `ctl seat …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], cx: &super::ParseCx) -> Result<Option<Command>, String> {
    let flags = cx.flags;
    let flag = |name: &str| flags.get(name).cloned();
    let swarm = cx.swarm;
    let human_flag = cx.human_flag;
    let on_screen_flag = cx.on_screen_flag;
    let cells_flag = cx.cells_flag;
    Ok(Some(match *words {
    ["spawn", harness] => Command::Spawn {
        harness: harness.into(),
        name: flag("name"),
        cwd: flag("cwd").map(PathBuf::from),
        swarm,
        role: flag("role"),
        new_tab: match flag("placement").as_deref() {
            None => false,
            Some("new-tab") => true,
            Some(other) => {
                return Err(format!(
                    "--placement: only new-tab is supported, not {other}"
                ));
            }
        },
    },
    ["seats"] => Command::Seats,
    ["seat", "screen", seat] => Command::Screen {
        seat: seat.into(),
        human: human_flag,
        scrollback: flag("scrollback")
            .map(|range| {
                range
                    .split_once("..")
                    .and_then(|(a, b)| Some((a.parse::<u64>().ok()?, b.parse::<u64>().ok()?)))
                    .filter(|(a, b)| a <= b && b - a <= SCROLLBACK_SEQUENCES)
                    .ok_or(format!(
                        "{USAGE_PREFIX}--scrollback FROM..TO, capture sequences, at most {SCROLLBACK_SEQUENCES} apart, not {range}"
                    ))
            })
            .transpose()?,
        cells: cells_flag,
    },
    ["seat", "rename", seat, name] => Command::Rename {
        seat: seat.into(),
        name: name.into(),
    },
    ["seat", "focus", seat] => Command::Focus { seat: seat.into() },
    ["seat", "restart", seat] => Command::Restart { seat: seat.into() },
    ["seat", "close", seat] => Command::Close { seat: seat.into() },
    ["seat", "undo-close", seat] => Command::UndoClose { seat: seat.into() },
    ["seat", "watch", seat] => Command::Watch {
        seat: seat.into(),
        timeout: Duration::from_millis(
            flag("timeout")
                .map(|v| {
                    v.parse::<u64>()
                        .ok()
                        .filter(|ms| (1..=3_600_000).contains(ms))
                        .ok_or(format!("{USAGE_PREFIX}--timeout: 1 to 3600000 ms, not {v}"))
                })
                .transpose()?
                .unwrap_or(30_000),
        ),
    },
    ["seat", "resize", seat, size] => {
        let (cols, rows) = size
            .split_once('x')
            .and_then(|(c, r)| Some((c.parse::<u16>().ok()?, r.parse::<u16>().ok()?)))
            .ok_or(format!("{USAGE_PREFIX}seat resize: COLSxROWS, not {size}"))?;
        Command::Resize {
            seat: seat.into(),
            cols,
            rows,
        }
    }
    ["seat", "type", seat, text] => Command::Type {
        seat: seat.into(),
        text: text.into(),
    },
    ["seat", "key", seat, ref keys @ ..] if !keys.is_empty() => Command::Key {
        seat: seat.into(),
        keys: keys.iter().map(|k| k.to_string()).collect(),
    },
    ["seat", "paste", seat, source] => Command::Paste {
        seat: seat.into(),
        source: source.into(),
    },
    ["seat", "wait", seat] => {
        let (state, text) = (flag("state"), flag("text"));
        let frame_seq = flag("frame-seq")
            .map(|v| {
                v.parse::<u64>()
                    .map_err(|_| format!("{USAGE_PREFIX}--frame-seq: a frame number, not {v}"))
            })
            .transpose()?;
        let conditions = [
            state.is_some(),
            text.is_some(),
            on_screen_flag,
            frame_seq.is_some(),
        ];
        if conditions.iter().filter(|set| **set).count() != 1 {
            return Err(format!(
                "seat wait: one of --state, --text, --on-screen or --frame-seq\n{USAGE}"
            ));
        }
        if (flag("subagent").is_some() || flag("grade").is_some()) && state.is_none() {
            return Err(format!("{USAGE_PREFIX}--subagent and --grade require --state"));
        }
        if flag("grade").as_deref().is_some_and(|grade| grade != "reported") {
            return Err(format!("{USAGE_PREFIX}--grade: reported"));
        }
        Command::Wait {
            seat: seat.into(),
            state,
            text,
            subagent: flag("subagent"),
            grade: flag("grade"),
            on_screen: on_screen_flag,
            frame_seq,
            timeout: Duration::from_millis(
                flag("timeout")
                    .map(|v| {
                        v.parse::<u64>()
                            .ok()
                            .filter(|ms| (1..=3_600_000).contains(ms))
                            .ok_or(format!("--timeout: 1 to 3600000 ms, not {v}"))
                    })
                    .transpose()?
                    .unwrap_or(30_000),
            ),
        }
    }
    ["seat", "ls"] => Command::Seats,
    ["seat", "spawn", harness] => {
        if let Some(place) = flag("place").or(flag("placement")) {
            use bbt_proto::gui::{SpawnPlace, UiSpawn};
            let place = match place.as_str() {
                "right" => SpawnPlace::Right,
                "down" => SpawnPlace::Down,
                "tab" | "new-tab" => SpawnPlace::Tab,
                "space" => SpawnPlace::Space,
                _ => return Err(format!("{USAGE_PREFIX}--place: expected right|down|tab|space")),
            };
            let pick = UiSpawn {
                harness: harness.into(), name: flag("name"), role: flag("role"), swarm, place,
                cwd: std::path::absolute(flag("cwd").unwrap_or_else(|| ".".into()))
                    .map_err(|e| format!("{USAGE_PREFIX}--cwd: {e}"))?.to_string_lossy().into_owned(),
            };
            let request = bbt_proto::gui::UiRequest::Spawn(Box::new(pick));
            request.validate().map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            let bbt_proto::gui::UiRequest::Spawn(pick) = request else { unreachable!() };
            Command::SpawnAt(pick)
        } else {
            // The no-placement backend lane also serves bare Session callers.
            Command::Spawn {harness:harness.into(),name:flag("name"),cwd:flag("cwd").map(PathBuf::from),swarm,role:flag("role"),new_tab:false}
        }
    },
    ["seat", "capture", target, path] => Command::Capture {
        path: std::path::absolute(path).map_err(|e| format!("capture {path}: {e}"))?,
        seat: (target != "window").then(|| target.to_string()),
    },
    ["capture", path] => Command::Capture {
        path: std::path::absolute(path).map_err(|e| format!("capture {path}: {e}"))?,
        seat: flag("seat"),
    },
        _ => return Ok(None),
    }))
}
