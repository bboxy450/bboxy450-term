//! `bboxy450-term ctl`: the operator's channel into a running app from outside it
//! (e2e-acceptance §3). It speaks `gui.sock` exactly as the GUI does, the same `GuiRequest`
//! frames under the same admission, so logic attaches the same operator principal
//! (decision #31) and dispatches through the same `GuiAction::Spawn`, `Messaging` and `Task`
//! paths, journaled as `control/op` (decisions #37, #38). It adds no authority: every command
//! is something the operator can already do in the window. Output is JSON lines.
#![allow(
    clippy::disallowed_methods,
    reason = "ctl is its own CLI process; it has no UI thread"
)]
use bbt_mux::Ulid;
mod app;
mod debug;
mod events;
mod records;
use bbt_proto::{
    GuiAction, GuiReply, GuiRequest, GuiSeat, SpawnSpec, SwarmId, Where,
    messaging::{Request, Response, SendRequest, TaskRequest, TaskResponse, TaskVerification},
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    time::Duration,
};

/// The GUI socket's frame cap (bbt-mux `seat_socket::read_line`).
const FRAME_LIMIT: usize = 1 << 20;
const TIMEOUT: Duration = Duration::from_secs(15);
const PAGE: usize = 128;

pub const USAGE: &str = "usage: bboxy450-term ctl <noun> <verb> [args] [flags]   (ctl-cli rev. 2)
  ui open <graph|settings|launcher|docs <path>>
  ui key <named-key>...                  cmd-t cmd-g cmd-m cmd-comma esc enter tab arrows
  ui focus <seat>                        select its pane through the GUI owner
  ui scroll <seat> <top|bottom|page-up|page-down|row N>   zero-based GUI history
  ui layout                              solved window tree, pane grids, focus and tab
  ui clock freeze|advance <ms>|resume     test-account animation time, acknowledged by its frame
  seat ls
  seat spawn <claude|codex|zsh> [--place right|down|tab|space] [--role oracle|architect|creativity|executor|shell]
             [--name NAME] [--cwd DIR] [--swarm ID]
  seat screen <seat> [--human]            the visible screen, cursor and title
  seat type <seat> <text>                 types the text as written
  seat key <seat> <key>...                enter esc tab backspace space up down left right home end
                                          pageup pagedown delete shift-tab ctrl-<letter>; Kitty when on
  seat paste <seat> <file|->              bracketed when the program asked
  seat wait <seat> (--state idle|waiting|exited | --text REGEX | --on-screen | --frame-seq N)
                                          [--timeout MS]   default 30000; --on-screen: the seat has a
                                          pane in the window's current frame; --frame-seq: the window
                                          has produced frame N (both read the owner's ui layout)
  seat capture <seat|window> <file.png>   with stale, occluded, frame_presented_at, frame_seq
  seat screen <seat> --cells | --scrollback FROM..TO   styled runs, or history rows
  seat resize <seat> <cols>x<rows>
  seat rename <seat> <name>
  seat restart <seat>                     stop the seat's process and start its next generation
  seat focus <seat>                       the window focuses it
  seat close <seat>                       close in the window, with 60 s undo
  seat undo-close <seat>                  restore the same PTY in its saved position
  seat watch <seat> [--timeout MS]        state and screen changes as event lines, then done
  message send (<seat|destination> | --reply-to ID) <text> [--swarm ID]
  message post <destination> <text> [--swarm ID]
  message inbox [--since SEQ]
  message receipts <message-id>
  message ack <message-id>
  task list [--unassigned | --seat SEAT | --status STATUS]
  task get <#N|path|id>
  task status <#N|path|id>
  task report <#N|path|id> --summary TEXT [--commit SHA] [--verify-result JSON]
                                          JSON: command, exit_code, optional evidence entries
  task accept <#N|path|id>
  task reject <#N|path|id> --reason TEXT
  task delegate <#N|path|id> <seat>
  task create <title> --verify CMD [--brief TEXT]
                                          an open, unassigned task, signed by the operator
  e2e list [--group GROUP]
  record fetch <#N|slug|path>              frontmatter, Markdown and confined resource links
  record search <query> [--type TYPE] [--cursor CURSOR]
  record changes --since CURSOR           changes and the cursor for the next read
  events streams                          retained stream names, no consumer attached
  events tail --stream NAME [--since SEQ] [--subject GLOB] [--timeout MS]
  debug logs <app|logic|supervisor> [--since BYTE_OFFSET]   bounded page, next_since
  debug journal <account/OP|seat/SEAT/OP>  one retained operation; no SQL or key namespace
  debug metrics                           existing numeric counters, with availability
  debug snapshot <file.json>              bounded R5 cursor/capture table; no tracing enabled
  debug ps                                this app's process tree; no argv or environment
  debug notifications                     bounded test recording; no OS presentation
  ui notification-click <seat>            recording callback, acknowledged by its pane frame
  record write <file|->                   an e2e record, signed by the operator device: the
                                          knowledge_write fields (path or ref, frontmatter,
                                          body, etag) as one JSON object
  app endpoint                            the run directory, socket path and generation
  app start [--headless] [--ctl] [--run-dir DIR]
            [--test-account DIR --notifications record]   debug fixtures only
  app status                              actual backend, GUI, events and custody readiness
  app resize <cols>x<rows>                the window (offscreen or real) to that cell grid
  app stop                                orderly shutdown through the admitted app relay
Flags on every verb: --json (default) | --human, --op ULID (an idempotent retry), --as LABEL or
BBOXY_CTL_ISSUER (who issues it, #269), --run-dir DIR or BBOXY_RUN_DIR, --endpoint FILE.
A <seat> is a seat id or a unique seat name. Each reply is one JSON line
{ok, op, noun, verb, data}; an error is {ok: false, op, error: {kind, message, hint}} with exit
1 refused, 2 usage, 3 timeout, 4 endpoint unavailable.
Deprecated for one release, with their old output: spawn, seats, dm, post, inbox, receipts,
delegate, reports, capture, e2e.";

/// Where the app's `gui.sock` is and the capability that admits a client.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub path: PathBuf,
    pub capability: [u8; 32],
}
impl Endpoint {
    /// `{"path": "...", "capability": "<64 hex digits>"}`.
    pub fn read(file: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(file)
            .map_err(|e| format!("endpoint {}: {e}", file.display()))?;
        let value: Value = serde_json::from_str(&text).map_err(|e| format!("endpoint: {e}"))?;
        let path = value["path"].as_str().ok_or("endpoint: missing path")?;
        let hex = value["capability"]
            .as_str()
            .ok_or("endpoint: missing capability")?;
        let mut capability = [0; 32];
        if hex.len() != 64 {
            return Err("endpoint: capability must be 64 hex digits".into());
        }
        for (i, byte) in capability.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                .map_err(|_| "endpoint: capability must be hex")?;
        }
        Ok(Self {
            path: path.into(),
            capability,
        })
    }
    /// The file form `read` accepts.
    pub fn to_json(&self) -> String {
        let hex: String = self.capability.iter().map(|b| format!("{b:02x}")).collect();
        json!({ "path": self.path, "capability": hex }).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ui(bbt_proto::gui::UiRequest),
    UiFocus(String),
    UiScroll {
        seat: String,
        motion: bbt_proto::gui::UiScroll,
    },
    Identity(bbt_proto::identity::IdentityRequest),
    UiNotificationClick(String),
    DebugNotifications,
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
    Dm {
        seat: String,
        text: String,
    },
    Post {
        destination: String,
        text: String,
        swarm: Option<Ulid>,
    },
    Inbox {
        since: u64,
    },
    Receipts {
        message: Ulid,
    },
    Delegate {
        task: String,
        seat: String,
    },
    Reports {
        since: Option<String>,
    },
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
    E2e {
        group: Option<String>,
    },
    /// `record write <file|->`: the knowledge_write JSON from a file or stdin.
    RecordWrite {
        source: String,
    },
    /// Fetch a record through the window's portable knowledge read path.
    RecordFetch {
        locator: String,
    },
    RecordSearch {
        query: String,
        cursor: Option<String>,
    },
    RecordChanges {
        since: String,
    },
    EventsStreams,
    EventsTail {
        stream: String,
        since: u64,
        subject: Option<String>,
        timeout: Duration,
    },
    Debug {
        request: bbt_proto::gui::DebugRequest,
        snapshot: Option<PathBuf>,
    },
    /// `message send (<destination|seat> | --reply-to <id>) <text>` (#264 routing).
    Send {
        to: Option<String>,
        text: String,
        reply_to: Option<Ulid>,
        swarm: Option<Ulid>,
    },
    /// `message ack <id>`: the operator acknowledges a message addressed to it.
    Ack {
        message: Ulid,
    },
    /// `task list [--unassigned | --seat <s> | --status <st>]`.
    TaskList {
        unassigned: bool,
        seat: Option<String>,
        status: Option<String>,
    },
    /// `task create <title> --verify CMD [--brief TEXT]`.
    TaskCreate {
        title: String,
        verify: String,
        brief: Option<String>,
    },
    /// `task get` and `task status <#N|path|id>` use the same task read.
    TaskGet {
        task: String,
    },
    /// `task report <#N|path|id> --summary TEXT [--commit SHA] [--verify-result JSON]`.
    TaskReport {
        task: String,
        summary: String,
        commit: Option<String>,
        verify_result: Option<TaskVerification>,
    },
    TaskAccept {
        task: String,
    },
    TaskReject {
        task: String,
        reason: String,
    },
    /// `app endpoint`: the run directory, socket path and generation; never the capability.
    Close {
        seat: String,
    },
    UndoClose {
        seat: String,
    },
    AppEndpoint,
    /// Start the same app binary and wait for its GUI render-readiness line.
    AppStart {
        headless: bool,
        ctl: bool,
        unattended: bool,
        test_account: Option<PathBuf>,
        record_notifications: bool,
    },
    /// Read liveness through the attached GUI owner.
    AppStatus,
    /// `app resize <cols>x<rows>`: the window to that grid of cells; the layout sizes panes.
    AppResize {
        cols: u16,
        rows: u16,
    },
    /// Stop through the authenticated app relay, never by PID.
    AppStop,
}

/// Every noun-verb `ctl` verb and the one window action it issues (ctl-cli §1). The parser
/// accepts exactly these; `app endpoint` reads the run directory and issues none.
pub const VERBS: &[(&str, &str, &str)] = &[
    ("ui", "open", "Ui"),
    ("ui", "key", "Ui"),
    ("ui", "focus", "Ui"),
    ("ui", "scroll", "Ui"),
    ("ui", "layout", "Ui"),
    ("ui", "clock", "Ui"),
    ("identity", "status", "Identity"),
    ("identity", "unlock", "Identity"),
    ("ui", "notification-click", "Ui"),
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
    ("message", "send", "Messaging"),
    ("message", "post", "Messaging"),
    ("message", "inbox", "Messaging"),
    ("message", "receipts", "Messaging"),
    ("message", "ack", "Messaging"),
    ("task", "list", "Task"),
    ("task", "get", "Task"),
    ("task", "status", "Task"),
    ("task", "report", "Task"),
    ("task", "accept", "Task"),
    ("task", "reject", "Task"),
    ("task", "delegate", "Task"),
    ("task", "create", "RecordWrite"),
    ("e2e", "list", "Knowledge"),
    ("record", "write", "RecordWrite"),
    ("record", "fetch", "Knowledge"),
    ("record", "search", "Knowledge"),
    ("record", "changes", "Knowledge"),
    ("events", "streams", "EventsStreams"),
    ("events", "tail", "EventsRead"),
    ("debug", "logs", "Debug"),
    ("debug", "journal", "Debug"),
    ("debug", "metrics", "Debug"),
    ("debug", "snapshot", "Debug"),
    ("debug", "ps", "Debug"),
    ("debug", "notifications", "App"),
    ("app", "endpoint", "-"),
    ("app", "start", "-"),
    ("app", "status", "App"),
    ("app", "resize", "Ui"),
    ("app", "stop", "App"),
];
/// The pre-grammar flat commands, kept one release as aliases (ctl-cli rev. 2, C-0).
const LEGACY: &[(&str, &str)] = &[
    ("spawn", "seat spawn"),
    ("seats", "seat ls"),
    ("dm", "message send <seat> <text>"),
    ("post", "message post"),
    ("inbox", "message inbox"),
    ("receipts", "message receipts"),
    ("delegate", "task delegate"),
    ("reports", "task list --status reported"),
    ("capture", "seat capture <seat|window> <path>"),
    ("e2e", "e2e list"),
];

/// A parsed command line: the command, and for a noun-verb form its table entry and the
/// shared flags (ctl-cli §1).
#[derive(Debug)]
pub struct Parsed {
    pub endpoint: Option<PathBuf>,
    pub command: Command,
    pub verb: Option<(&'static str, &'static str)>,
    pub op: Option<Ulid>,
    pub human: bool,
}

/// Parses everything after `ctl`; returns the endpoint file override and the command.
pub fn parse(args: &[String]) -> Result<(Option<PathBuf>, Command), String> {
    parse_full(args).map(|p| (p.endpoint, p.command))
}

/// Parses everything after `ctl`, the shared flags included.
pub fn parse_full(args: &[String]) -> Result<Parsed, String> {
    let mut endpoint = None;
    let mut flags = std::collections::BTreeMap::new();
    let mut words = vec![];
    let mut human_flag = false;
    let mut unassigned_flag = false;
    let mut on_screen_flag = false;
    let mut cells_flag = false;
    let (mut headless, mut ctl, mut unattended) = (false, false, false);
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "--json" {
            // The default reply form (ctl-cli §1).
        } else if arg == "--human" {
            human_flag = true;
        } else if arg == "--unassigned" {
            unassigned_flag = true;
        } else if arg == "--on-screen" {
            on_screen_flag = true;
        } else if arg == "--cells" {
            cells_flag = true;
        } else if arg == "--headless" {
            headless = true;
        } else if arg == "--ctl" {
            ctl = true;
        } else if arg == "--unattended" {
            unattended = true;
        } else if let Some(flag) = arg.strip_prefix("--") {
            let value = it.next().ok_or_else(|| {
                // Keep deprecated flat-form failures unchanged. The new read
                // grammar reports malformed flags as its named usage error.
                let prefix = if words
                    .first()
                    .is_some_and(|s: &String| matches!(s.as_str(), "record" | "events" | "debug"))
                {
                    USAGE_PREFIX
                } else {
                    ""
                };
                format!("{prefix}--{flag} needs a value")
            })?;
            match flag {
                "endpoint" => endpoint = Some(PathBuf::from(value)),
                // --run-dir finds the endpoint as BBOXY_RUN_DIR does (#260).
                "run-dir" => {
                    endpoint = Some(PathBuf::from(value).join(bbt_mux::ctl_door::ENDPOINT))
                }
                _ => {
                    flags.insert(flag.to_owned(), value.clone());
                }
            }
        } else {
            words.push(arg.clone());
        }
    }
    let verb = match words.as_slice() {
        [noun, verb, ..] => VERBS
            .iter()
            .find(|(n, v, _)| n == noun && v == verb)
            .map(|(n, v, _)| (*n, *v)),
        _ => None,
    };
    if verb.is_none()
        && let Some(noun) = words.first()
        && VERBS.iter().any(|(n, ..)| n == noun)
        && !LEGACY.iter().any(|(old, _)| old == noun)
    {
        let verbs: Vec<_> = VERBS
            .iter()
            .filter(|(n, ..)| n == noun)
            .map(|(_, v, _)| *v)
            .collect();
        return Err(format!("{USAGE_PREFIX}{noun} verbs: {}", verbs.join(", ")));
    }
    let ulid = |v: &String| v.parse::<Ulid>().map_err(|_| format!("not an id: {v}"));
    let flag = |name: &str| flags.get(name).cloned();
    let swarm = flag("swarm").as_ref().map(ulid).transpose()?;
    let command = match words.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["ui", "clock", "freeze"] => Command::Ui(bbt_proto::gui::UiRequest::Clock(
            bbt_proto::gui::UiClockCommand::Freeze,
        )),
        ["ui", "clock", "advance", ms] => Command::Ui(bbt_proto::gui::UiRequest::Clock(
            bbt_proto::gui::UiClockCommand::Advance {
                ms: ms.parse::<u64>().map_err(|_| {
                    format!("{USAGE_PREFIX}ui clock advance requires unsigned integer milliseconds")
                })?,
            },
        )),
        ["ui", "clock", "resume"] => Command::Ui(bbt_proto::gui::UiRequest::Clock(
            bbt_proto::gui::UiClockCommand::Resume,
        )),
        ["ui", "layout"] => Command::Ui(bbt_proto::gui::UiRequest::Layout),
        ["ui", "focus", seat] => Command::UiFocus(seat.into()),
        ["ui", "scroll", seat, motion] => Command::UiScroll {
            seat: seat.into(),
            motion: match motion {
                "top" => bbt_proto::gui::UiScroll::Top,
                "bottom" => bbt_proto::gui::UiScroll::Bottom,
                "page-up" => bbt_proto::gui::UiScroll::PageUp,
                "page-down" => bbt_proto::gui::UiScroll::PageDown,
                _ => return Err(format!("{USAGE_PREFIX}ui scroll <pane> top|bottom|page-up|page-down|row N")),
            },
        },
        ["ui", "scroll", seat, "row", row] => Command::UiScroll {
            seat: seat.into(),
            motion: bbt_proto::gui::UiScroll::Row(row.parse().map_err(|_| format!("{USAGE_PREFIX}ui scroll row requires an unsigned zero-based row"))?),
        },
        ["ui", "notification-click", seat] => Command::UiNotificationClick(seat.into()),
        ["debug", "notifications"] => Command::DebugNotifications,
        ["ui", "open", target] => Command::Ui(bbt_proto::gui::UiRequest::Open(match target {
            "graph" => bbt_proto::gui::UiTarget::Graph,
            "settings" => bbt_proto::gui::UiTarget::Settings,
            "launcher" => bbt_proto::gui::UiTarget::Launcher,
            _ => return Err(format!("{USAGE_PREFIX}ui open graph|settings|launcher|docs <path>")),
        })),
        ["ui", "open", "docs", path] => {
            let request = bbt_proto::gui::UiRequest::Open(bbt_proto::gui::UiTarget::Docs(path.into()));
            request.validate().map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Ui(request)
        },
        ["ui", "key", ref keys @ ..] => {
            let request = bbt_proto::gui::UiRequest::Key(keys.iter().map(|s| s.to_string()).collect());
            request.validate().map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Ui(request)
        },

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
        ["dm", seat, text] => Command::Dm {
            seat: seat.into(),
            text: text.into(),
        },
        ["post", destination, text] => Command::Post {
            destination: destination.into(),
            text: text.into(),
            swarm,
        },
        ["inbox"] => Command::Inbox {
            since: flag("since")
                .map(|v| {
                    v.parse()
                        .map_err(|_| format!("--since: not a sequence: {v}"))
                })
                .transpose()?
                .unwrap_or(0),
        },
        ["receipts", message] => Command::Receipts {
            message: ulid(&message.to_owned())?,
        },
        ["delegate", task, seat] => Command::Delegate {
            task: task.into(),
            seat: seat.into(),
        },
        ["reports"] => Command::Reports {
            since: flag("since"),
        },
        ["e2e"] => Command::E2e {
            group: flag("group"),
        },
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
            Command::Wait {
                seat: seat.into(),
                state,
                text,
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
        ["message", "send", to, text] => Command::Send {
            to: Some(to.into()),
            text: text.into(),
            reply_to: flag("reply-to").as_ref().map(ulid).transpose()?,
            swarm,
        },
        ["message", "send", text] if flags.contains_key("reply-to") => Command::Send {
            to: None,
            text: text.into(),
            reply_to: flag("reply-to").as_ref().map(ulid).transpose()?,
            swarm,
        },
        ["message", "post", destination, text] => Command::Post {
            destination: destination.into(),
            text: text.into(),
            swarm,
        },
        ["message", "inbox"] => Command::Inbox {
            since: flag("since")
                .map(|v| {
                    v.parse()
                        .map_err(|_| format!("--since: not a sequence: {v}"))
                })
                .transpose()?
                .unwrap_or(0),
        },
        ["message", "receipts", message] => Command::Receipts {
            message: ulid(&message.to_owned())?,
        },
        ["message", "ack", message] => Command::Ack {
            message: ulid(&message.to_owned())?,
        },
        ["task", "delegate", task, seat] => Command::Delegate {
            task: task.into(),
            seat: seat.into(),
        },
        ["task", "list"] => Command::TaskList {
            unassigned: unassigned_flag,
            seat: flag("seat"),
            status: flag("status"),
        },
        ["task", "get" | "status", task] => Command::TaskGet { task: task.into() },
        ["task", "report", task] => Command::TaskReport {
            task: task.into(),
            summary: flag("summary")
                .ok_or_else(|| format!("{USAGE_PREFIX}task report needs --summary TEXT"))?,
            commit: flag("commit"),
            verify_result: flag("verify-result")
                .map(|value| serde_json::from_str(&value).map_err(|e| {
                    format!("{USAGE_PREFIX}task report --verify-result: {e}")
                }))
                .transpose()?,
        },
        ["task", "accept", task] => Command::TaskAccept { task: task.into() },
        ["task", "reject", task] => Command::TaskReject {
            task: task.into(),
            reason: flag("reason")
                .ok_or_else(|| format!("{USAGE_PREFIX}task reject needs --reason TEXT"))?,
        },
        ["task", "create", title] => Command::TaskCreate {
            title: title.into(),
            verify: flag("verify")
                .ok_or_else(|| format!("{USAGE_PREFIX}task create needs --verify CMD"))?,
            brief: flag("brief"),
        },
        ["e2e", "list"] => Command::E2e {
            group: flag("group"),
        },
        ["record", "write", source] => Command::RecordWrite {
            source: source.into(),
        },
        ["record", "fetch", locator] => Command::RecordFetch {
            locator: locator.into(),
        },
        ["record", "search", query] => Command::RecordSearch {
            query: records::search_query(query, flag("type").as_deref())?,
            cursor: flag("cursor"),
        },
        ["record", "changes"] => Command::RecordChanges {
            since: flag("since")
                .filter(|s| !s.is_empty())
                .ok_or_else(|| format!("{USAGE_PREFIX}record changes requires --since CURSOR"))?,
        },
        ["events", "streams"] => Command::EventsStreams,
        ["debug", "logs", process] => Command::Debug {
            request: bbt_proto::gui::DebugRequest::Logs {
                process: match process {
                    "app" => bbt_proto::gui::DebugProcess::App,
                    "logic" => bbt_proto::gui::DebugProcess::Logic,
                    "supervisor" => bbt_proto::gui::DebugProcess::Supervisor,
                    _ => return Err(format!("{USAGE_PREFIX}debug logs: app, logic or supervisor")),
                },
                since: flag("since").map(|s| s.parse::<u64>().map_err(|_|format!("{USAGE_PREFIX}debug logs --since: byte offset"))).transpose()?.unwrap_or(0),
            }, snapshot: None,
        },
        ["debug", "journal", query] => {
            bbt_mux::debug::journal_query(query).map_err(|e|format!("{USAGE_PREFIX}{e}"))?;
            Command::Debug {request:bbt_proto::gui::DebugRequest::Journal {query:query.into()},snapshot:None}
        },
        ["debug", "metrics"] => Command::Debug {request:bbt_proto::gui::DebugRequest::Metrics,snapshot:None},
        ["debug", "ps"] => Command::Debug {request:bbt_proto::gui::DebugRequest::Ps,snapshot:None},
        ["debug", "snapshot", path] => {
            let path = std::path::absolute(path).map_err(|_|format!("{USAGE_PREFIX}debug snapshot requires a local file path"))?;
            let destination = path.to_str().filter(|s|s.len()<=4096).ok_or_else(||format!("{USAGE_PREFIX}debug snapshot path must be UTF-8, at most 4096 bytes"))?.to_owned();
            Command::Debug {request:bbt_proto::gui::DebugRequest::Snapshot {destination},snapshot:Some(path)}
        },
        ["events", "tail"] => {
            let stream = flag("stream").filter(|s| !s.is_empty() && s.len() <= 128)
                .ok_or_else(|| format!("{USAGE_PREFIX}events tail requires --stream NAME (events streams lists names)"))?;
            let since = flag("since").map(|s| s.parse::<u64>()
                .map_err(|_| format!("{USAGE_PREFIX}--since requires a stream sequence")))
                .transpose()?.unwrap_or(0);
            let subject = flag("subject");
            if subject.as_ref().is_some_and(|s| s.is_empty() || s.len() > 256) {
                return Err(format!("{USAGE_PREFIX}--subject requires a nonempty subject glob up to 256 bytes"));
            }
            let timeout = flag("timeout").map(|s| s.parse::<u64>().ok()
                .filter(|ms| (1..=3_600_000).contains(ms))
                .ok_or_else(|| format!("{USAGE_PREFIX}--timeout: 1 to 3600000 ms")))
                .transpose()?.unwrap_or(30_000);
            Command::EventsTail {stream,since,subject,timeout:Duration::from_millis(timeout)}
        },
        ["identity", "status"] => Command::Identity(bbt_proto::identity::IdentityRequest::Status),
        ["identity", "unlock"] => Command::Identity(bbt_proto::identity::IdentityRequest::Unlock),
        ["app", "endpoint"] => Command::AppEndpoint,
        ["app", "start"] => Command::AppStart {
            headless,
            ctl,
            unattended,
            test_account: flag("test-account").map(PathBuf::from),
            record_notifications: match flag("notifications").as_deref() {
                None => false,
                Some("record") if flags.contains_key("test-account") => true,
                _ => return Err(format!("{USAGE_PREFIX}--notifications record requires --test-account DIR")),
            },
        },
        ["app", "status"] => Command::AppStatus,
        ["app", "resize", size] => {
            let (cols, rows) = size
                .split_once('x')
                .and_then(|(c, r)| Some((c.parse::<u16>().ok()?, r.parse::<u16>().ok()?)))
                .filter(|&(c, r)| (2..=512).contains(&c) && (1..=256).contains(&r))
                .ok_or(format!(
                    "{USAGE_PREFIX}app resize: COLSxROWS, 2-512 by 1-256, not {size}"
                ))?;
            Command::AppResize { cols, rows }
        }
        ["app", "stop"] => Command::AppStop,
        ["capture", path] => Command::Capture {
            path: std::path::absolute(path).map_err(|e| format!("capture {path}: {e}"))?,
            seat: flag("seat"),
        },
        _ => {
            return Err(match verb {
                Some((noun, v)) => {
                    format!("{USAGE_PREFIX}wrong arguments for {noun} {v}; see ctl --help")
                }
                None => USAGE.into(),
            });
        }
    };
    if (headless
        || ctl
        || unattended
        || flags.contains_key("test-account")
        || flags.contains_key("notifications"))
        && !matches!(&command, Command::AppStart { .. })
    {
        return Err(format!(
            "{USAGE_PREFIX}--headless, --ctl, --unattended and --test-account apply only to app start"
        ));
    }
    let read_flags: Option<&[&str]> = match &command {
        Command::RecordFetch { .. } | Command::EventsStreams => Some(&[]),
        Command::RecordSearch { .. } => Some(&["type", "cursor"]),
        Command::RecordChanges { .. } => Some(&["since"]),
        Command::EventsTail { .. } => Some(&["stream", "since", "subject", "timeout"]),
        Command::Debug {
            request: bbt_proto::gui::DebugRequest::Logs { .. },
            ..
        } => Some(&["since"]),
        Command::Debug { .. } => Some(&[]),
        Command::DebugNotifications | Command::UiNotificationClick(_) => Some(&[]),
        _ => None,
    };
    if let Some(allowed) = read_flags {
        if let Some(unknown) = flags
            .keys()
            .find(|k| !allowed.contains(&k.as_str()) && !["op", "as"].contains(&k.as_str()))
        {
            return Err(format!(
                "{USAGE_PREFIX}--{unknown} does not apply to this read verb"
            ));
        }
        if cells_flag || unassigned_flag {
            return Err(format!(
                "{USAGE_PREFIX}--cells and --unassigned do not apply to record/events reads"
            ));
        }
    }
    if flags
        .keys()
        .any(|key| matches!(key.as_str(), "summary" | "commit" | "verify-result"))
        && !matches!(&command, Command::TaskReport { .. })
    {
        return Err(format!(
            "{USAGE_PREFIX}--summary, --commit and --verify-result apply only to task report"
        ));
    }
    if flags.contains_key("reason") && !matches!(&command, Command::TaskReject { .. }) {
        return Err(format!(
            "{USAGE_PREFIX}--reason applies only to task reject"
        ));
    }
    if let Some(unknown) = flags.keys().find(|k| {
        ![
            "swarm",
            "test-account",
            "notifications",
            "name",
            "cwd",
            "since",
            "placement",
            "seat",
            "group",
            "role",
            "timeout",
            "state",
            "text",
            "op",
            "as",
            "place",
            "reply-to",
            "status",
            "scrollback",
            "frame-seq",
            "verify",
            "brief",
            "type",
            "cursor",
            "stream",
            "subject",
            "summary",
            "commit",
            "verify-result",
            "reason",
        ]
        .contains(&k.as_str())
    }) {
        return Err(format!("{USAGE_PREFIX}unknown flag --{unknown}"));
    }
    let op = flag("op").as_ref().map(ulid).transpose()?;
    Ok(Parsed {
        endpoint,
        command,
        verb,
        op,
        human: human_flag,
    })
}
/// A usage error (ctl-cli §3, exit 2).
const USAGE_PREFIX: &str = "usage: ";

/// One admitted connection to `gui.sock`.
pub struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    capability: [u8; 32],
    /// `--op`: the id the next effect uses, for an idempotent retry (#266 rule 1).
    pub op: Option<Ulid>,
    /// The identity logic announced on admission (its verified custody label).
    pub identity: Option<bbt_proto::gui::GuiIdentity>,
}
impl Client {
    pub fn connect(endpoint: &Endpoint) -> Result<Self, String> {
        let stream = UnixStream::connect(&endpoint.path)
            .map_err(|e| format!("gui.sock {}: {e}", endpoint.path.display()))?;
        stream
            .set_read_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            reader: BufReader::new(stream.try_clone().map_err(|e| e.to_string())?),
            writer: stream,
            capability: endpoint.capability,
            op: None,
            identity: None,
        })
    }
    /// The id for the next action: `--op` once, then fresh ones.
    pub fn mint(&mut self) -> Ulid {
        self.op.take().unwrap_or_else(fresh)
    }
    fn send(&mut self, action: GuiAction) -> Result<(), String> {
        let mut frame = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: self.capability,
            action,
        })
        .map_err(|e| e.to_string())?;
        frame.push(b'\n');
        self.writer
            .write_all(&frame)
            .map_err(|e| format!("gui.sock write: {e}"))
    }
    fn reply(&mut self) -> Result<GuiReply, String> {
        let mut line = vec![];
        let read = Read::by_ref(&mut self.reader)
            .take(FRAME_LIMIT as u64 + 1)
            .read_until(b'\n', &mut line)
            .map_err(|e| format!("gui.sock read: {e}"))?;
        if read == 0 {
            return Err("gui.sock closed the connection (version or capability refused)".into());
        }
        if line.len() > FRAME_LIMIT {
            return Err("gui.sock frame over the 1 MiB limit".into());
        }
        serde_json::from_slice(&line).map_err(|e| format!("gui.sock reply: {e}"))
    }
    /// Sends `action` and returns the first reply `answer` accepts; an `Error` for this op
    /// is returned as `Err` with logic's reason.
    fn call(
        &mut self,
        op: Option<Ulid>,
        action: GuiAction,
        mut answer: impl FnMut(&GuiReply) -> bool,
    ) -> Result<GuiReply, String> {
        if let Err(write) = self.send(action) {
            // The door can refuse and close before the request is written; its reason is
            // already in our buffer, so a seat reads the refusal, not a broken pipe.
            return match self.reply() {
                Ok(GuiReply::Error { reason, .. }) => Err(reason),
                _ => Err(write),
            };
        }
        loop {
            let reply = self.reply()?;
            if let GuiReply::Identity(identity) = &reply {
                self.identity = Some(identity.clone());
            }
            if let GuiReply::Error { op: failed, reason } = &reply
                && (op == Some(*failed) || failed.is_nil())
            {
                return Err(reason.clone());
            }
            if answer(&reply) {
                return Ok(reply);
            }
        }
    }
    /// Names who issues this connection's operator requests (decision #269).
    pub fn issue(&mut self, label: String) -> Result<(), String> {
        self.call(None, GuiAction::Issuer { label }, |r| {
            matches!(r, GuiReply::Ok { .. })
        })
        .map(drop)
    }
    pub fn seats(&mut self) -> Result<Vec<GuiSeat>, String> {
        match self.call(None, GuiAction::ListSeats, |r| {
            matches!(r, GuiReply::Seats(_))
        })? {
            GuiReply::Seats(seats) => Ok(seats),
            _ => unreachable!(),
        }
    }
    pub fn spawn(&mut self, spec: SpawnSpec) -> Result<Ulid, String> {
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::Spawn { id, spec },
            |r| matches!(r, GuiReply::Spawned { op, .. } if *op == id),
        )? {
            GuiReply::Spawned { seat, .. } => Ok(Ulid::from(seat.0.0)),
            _ => unreachable!(),
        }
    }
    pub fn messaging(&mut self, swarm: Ulid, request: Request) -> Result<Response, String> {
        let id = match &request {
            Request::Send { message } => message.id,
            _ => self.mint(),
        };
        let action = GuiAction::Messaging {
            id,
            swarm_id: SwarmId::from_u128(swarm.0),
            request,
        };
        match self.call(
            Some(id),
            action,
            |r| matches!(r, GuiReply::Messaging { op, .. } if *op == id),
        )? {
            GuiReply::Messaging { response, .. } => Ok(response),
            _ => unreachable!(),
        }
    }
    /// Asks the window for a PNG of itself, or of one seat's pane; returns the written path.
    pub fn capture(
        &mut self,
        path: &Path,
        seat: Option<Ulid>,
    ) -> Result<(String, bbt_proto::gui::CaptureFrame), String> {
        let id = self.mint();
        let path = path.to_str().ok_or("capture path is not UTF-8")?.to_owned();
        match self.call(
            Some(id),
            GuiAction::Capture { id, path, seat },
            |r| matches!(r, GuiReply::Captured { op, .. } if *op == id),
        )? {
            GuiReply::Captured { path, frame, .. } => Ok((path, frame)),
            _ => unreachable!(),
        }
    }
    pub fn knowledge(
        &mut self,
        call: &crate::knowledge::ReadCall,
    ) -> bbt_mux::knowledge::data_source::Result<Value> {
        use bbt_mux::knowledge::data_source::CommandError;
        let id = self.mint();
        let reply = self
            .call(
                Some(id),
                GuiAction::Knowledge {
                    id,
                    method: call.method.into(),
                    args: call.args.clone(),
                },
                |r| matches!(r, GuiReply::Knowledge {op,..} if *op == id),
            )
            .map_err(|e| CommandError::new("transport", e))?;
        match reply {
            GuiReply::Knowledge { response, .. } => serde_json::from_value(response)
                .map_err(|e| CommandError::new("invalid_provider_result", e.to_string()))?,
            _ => unreachable!(),
        }
    }
    /// Read custody or admit one unlock through the same private operator connection.
    pub fn identity(
        &mut self,
        request: bbt_proto::identity::IdentityRequest,
    ) -> Result<(Ulid, bbt_proto::identity::IdentityControl), String> {
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::Identity(bbt_proto::identity::IdentityCall { id, request }),
            |reply| matches!(reply, GuiReply::IdentityControl { op, .. } if *op == id),
        )? {
            GuiReply::IdentityControl { response, .. } => Ok((id, response)),
            _ => unreachable!(),
        }
    }
    pub fn debug(&mut self, request: bbt_proto::gui::DebugRequest) -> Result<Value, String> {
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::Debug { id, request },
            |reply| matches!(reply,GuiReply::Debug {op,..} if *op==id),
        )? {
            GuiReply::Debug { data, .. } => Ok(data),
            _ => unreachable!(),
        }
    }
    /// The operator's signed `e2e` record write: the data_source write result.
    pub fn record_write(&mut self, args: Value) -> Result<Value, String> {
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::RecordWrite { id, args },
            |r| matches!(r, GuiReply::Knowledge { op, .. } if *op == id),
        )? {
            GuiReply::Knowledge { response, .. } => {
                serde_json::from_value::<bbt_mux::knowledge::data_source::Result<Value>>(response)
                    .map_err(|e| e.to_string())?
                    .map_err(|e| format!("{}: {}", e.code, e.message))
            }
            _ => unreachable!(),
        }
    }
    /// The seat's screen, with the op id of the read.
    pub fn screen(&mut self, seat: Ulid) -> Result<(Ulid, bbt_proto::gui::SeatScreen), String> {
        self.screen_with(seat, false)
    }
    /// The seat's screen, with every cell's style as runs when `cells`.
    pub fn screen_with(
        &mut self,
        seat: Ulid,
        cells: bool,
    ) -> Result<(Ulid, bbt_proto::gui::SeatScreen), String> {
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::Screen { id, seat, cells },
            |r| matches!(r, GuiReply::Screen { op, .. } if *op == id),
        )? {
            GuiReply::Screen { screen, .. } => Ok((id, screen)),
            _ => unreachable!(),
        }
    }
    /// One page of a retained stream after `after`, read-only (#272 `EventsRead`).
    pub fn events_read(
        &mut self,
        stream: &str,
        after: u64,
        limit: usize,
    ) -> Result<(Vec<bbt_proto::gui::RetainedEvent>, Option<u64>), String> {
        self.events_page(stream, after, limit)
            .map(|(events, next, _)| (events, next))
    }
    fn events_page(
        &mut self,
        stream: &str,
        after: u64,
        limit: usize,
    ) -> Result<(Vec<bbt_proto::gui::RetainedEvent>, Option<u64>, u64), String> {
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::EventsRead {
                id,
                stream: stream.into(),
                after,
                limit,
            },
            |r| matches!(r, GuiReply::EventsRead { op, .. } if *op == id),
        )? {
            GuiReply::EventsRead {
                events,
                next,
                through,
                ..
            } => Ok((events, next, through)),
            _ => unreachable!(),
        }
    }
    /// Read-only discovery under the same GUI admission as EventsRead.
    pub fn event_streams(&mut self) -> Result<Vec<String>, String> {
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::EventsStreams { id },
            |r| matches!(r, GuiReply::EventsStreams {op,..} if *op==id),
        )? {
            GuiReply::EventsStreams { streams, .. } => Ok(streams),
            _ => unreachable!(),
        }
    }
    /// The capture records of `from..=to`, every page of them (#27's lazy pages).
    pub fn history(
        &mut self,
        seat: Ulid,
        from: u64,
        to: u64,
    ) -> Result<Vec<bbt_proto::gui::HistoryRecord>, String> {
        let mut records = vec![];
        let mut at = from;
        loop {
            let wire = bbt_proto::SeatId::from_u128(seat.0);
            let reply = self.call(
                None,
                GuiAction::GetHistory {
                    seat: wire,
                    from: at,
                    to,
                },
                |r| matches!(r, GuiReply::History { seat: s, .. } if *s == wire),
            )?;
            let GuiReply::History {
                records: page,
                next,
                ..
            } = reply
            else {
                unreachable!()
            };
            records.extend(page);
            match next {
                Some(next) if next > at && next <= to => at = next,
                _ => break,
            }
        }
        Ok(records)
    }
    /// The window's resize; returns the op the app journaled it under.
    pub fn resize(
        &mut self,
        seat: Ulid,
        generation: u64,
        cols: u16,
        rows: u16,
    ) -> Result<Ulid, String> {
        match self.call(
            None,
            GuiAction::Resize {
                seat: bbt_proto::SeatId::from_u128(seat.0),
                generation,
                cols,
                rows,
            },
            // Resize carries no op id, so the door tags its reply, and a refusal, with an op
            // of its own: this one-request client takes whichever comes next.
            |r| matches!(r, GuiReply::Ok { .. } | GuiReply::Error { .. }),
        )? {
            GuiReply::Ok { op } => Ok(op),
            GuiReply::Error { reason, .. } => Err(reason),
            _ => unreachable!(),
        }
    }
    /// The window's input action, under this connection's door and issuer; returns its op.
    pub fn input(&mut self, seat: Ulid, generation: u64, bytes: Vec<u8>) -> Result<Ulid, String> {
        let id = self.mint();
        self.call(
            Some(id),
            GuiAction::Input {
                seat: bbt_proto::SeatId::from_u128(seat.0),
                generation,
                id,
                bytes,
            },
            |r| matches!(r, GuiReply::Ok { op } if *op == id),
        )
        .map(|_| id)
    }
    /// Query or stop the attached app owner through the existing admitted endpoint.
    pub fn ui(
        &mut self,
        request: bbt_proto::gui::UiRequest,
    ) -> Result<bbt_proto::gui::UiLayout, String> {
        request.validate().map_err(str::to_owned)?;
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::Ui { id, request },
            |r| matches!(r, GuiReply::Ui { op, .. } if *op == id),
        )? {
            GuiReply::Ui { layout, .. } => Ok(layout),
            _ => unreachable!(),
        }
    }
    pub fn app(
        &mut self,
        request: bbt_proto::gui::AppRequest,
    ) -> Result<bbt_proto::gui::AppStatus, String> {
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::App { id, request },
            |r| matches!(r,GuiReply::App {op,..} if *op==id),
        )? {
            GuiReply::App { status, .. } => Ok(status),
            _ => unreachable!(),
        }
    }
    pub fn task(&mut self, request: TaskRequest) -> Result<TaskResponse, String> {
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::Task { id, request },
            |r| matches!(r, GuiReply::Task { op, .. } if *op == id),
        )? {
            GuiReply::Task { response, .. } => Ok(response),
            _ => unreachable!(),
        }
    }
}

fn seat_line(seat: &GuiSeat) -> Value {
    let r = &seat.record;
    // The wire form of the state, so activity and process read as the GUI receives them.
    let state = serde_json::to_value(&seat.state).unwrap_or(Value::Null);
    json!({
        "kind": "seat",
        "seat": r.id,
        "name": r.name,
        "harness": r.harness_kind,
        "swarm": r.swarm,
        "role": r.role,
        "where": r.location,
        "generation": state["generation"],
        "process": state["process"],
        "activity": state["activity"],
        "title": state["title"],
        "muted": r.muted,
    })
}
/// A seat by id, or by a name that names exactly one seat.
fn resolve<'a>(seats: &'a [GuiSeat], seat: &str) -> Result<&'a GuiSeat, String> {
    if let Ok(id) = seat.parse::<Ulid>()
        && let Some(found) = seats.iter().find(|s| s.record.id == id)
    {
        return Ok(found);
    }
    let named: Vec<_> = seats.iter().filter(|s| s.record.name == seat).collect();
    match named[..] {
        [one] => Ok(one),
        [] => Err(format!("no seat {seat}")),
        _ => Err(format!("seat name {seat} is ambiguous; use its id")),
    }
}
/// The distinct swarms of the live seats, in roster order.
fn swarms(seats: &[GuiSeat]) -> Vec<Ulid> {
    let mut out = vec![];
    for s in seats {
        if !out.contains(&s.record.swarm) {
            out.push(s.record.swarm);
        }
    }
    out
}

/// Runs one command against an admitted client; returns its JSON lines.
pub fn run(client: &mut Client, command: Command) -> Result<Vec<Value>, String> {
    let to_value = |v: &dyn erased::Serialize| v.to_json();
    Ok(match command {
        Command::Identity(request) => {
            let (op, response) = client.identity(request)?;
            vec![json!({"kind":"identity", "op":op, "response":response})]
        }
        Command::Spawn {
            harness,
            name,
            cwd,
            swarm,
            new_tab,
            role,
        } => {
            // The launcher's defaults (live.rs `spawn_spec`): the first seat's swarm, else a
            // new one; the backend validates the spec as it does for the GUI.
            let swarm = match swarm {
                Some(swarm) => swarm,
                None => client
                    .seats()?
                    .first()
                    .map_or_else(Ulid::new, |s| s.record.swarm),
            };
            let cwd = match cwd {
                Some(cwd) => cwd,
                None => std::env::current_dir().map_err(|e| e.to_string())?,
            };
            let seat = client.spawn(SpawnSpec {
                swarm: SwarmId::from_u128(swarm.0),
                harness_kind: harness.clone(),
                name,
                role,
                cwd: cwd.to_string_lossy().into_owned(),
                location: Where::Local,
                resume: None,
                placement: new_tab.then(|| bbt_proto::seat_state::Placement::NewTab {
                    tab: bbt_proto::TabId::from_u128(Ulid::new().0),
                }),
            })?;
            vec![json!({ "kind": "spawned", "seat": seat, "harness": harness, "swarm": swarm })]
        }
        Command::Seats => client.seats()?.iter().map(seat_line).collect(),
        Command::Dm { seat, text } => {
            let seats = client.seats()?;
            let seat = resolve(&seats, &seat)?;
            let message = SendRequest::text(format!("seats/{}/inbox", seat.record.id), text);
            let id = message.id;
            let response = client.messaging(seat.record.swarm, Request::Send { message })?;
            vec![
                json!({ "kind": "sent", "message": id, "seat": seat.record.id, "response": to_value(&response) }),
            ]
        }
        Command::Post {
            destination,
            text,
            swarm,
        } => {
            let swarm = match swarm {
                Some(swarm) => swarm,
                None => *swarms(&client.seats()?)
                    .first()
                    .ok_or("no seats: pass --swarm")?,
            };
            let message = SendRequest::text(destination.clone(), text);
            let id = message.id;
            let response = client.messaging(swarm, Request::Send { message })?;
            vec![
                json!({ "kind": "sent", "message": id, "destination": destination, "response": to_value(&response) }),
            ]
        }
        Command::Inbox { since } => {
            // The operator's inbox: an operator List with no channel lists what is addressed
            // to this device's operator sink (decision #38 (e)), one swarm at a time (#38 (f)).
            let mut out = vec![];
            for swarm in swarms(&client.seats()?) {
                let mut after = since;
                loop {
                    let request = Request::List {
                        thread: None,
                        channel: None,
                        after,
                        limit: PAGE,
                    };
                    let Response::Messages { page } = client.messaging(swarm, request)? else {
                        return Err("inbox: unexpected messaging response".into());
                    };
                    for message in &page.items {
                        out.push(json!({ "kind": "message", "swarm": swarm, "message": to_value(message) }));
                    }
                    match page.next {
                        Some(next) if next > after && !page.items.is_empty() => after = next,
                        _ => break,
                    }
                }
            }
            out
        }
        Command::Receipts { message } => {
            // A message lives in one swarm (decision #38 (f)); ask each until one answers.
            let mut last = String::from("no seats, so no swarm to ask");
            for swarm in swarms(&client.seats()?) {
                let request = Request::Receipts {
                    message,
                    after: 0,
                    limit: PAGE,
                };
                match client.messaging(swarm, request) {
                    Ok(Response::Receipts { page }) => {
                        return Ok(page
                            .items
                            .iter()
                            .map(|r| json!({ "kind": "receipt", "message": message, "swarm": swarm, "receipt": to_value(r) }))
                            .collect());
                    }
                    Ok(_) => return Err("receipts: unexpected messaging response".into()),
                    Err(reason) => last = reason,
                }
            }
            return Err(format!("receipts for {message}: {last}"));
        }
        Command::Delegate { task, seat } => {
            let seats = client.seats()?;
            let seat = resolve(&seats, &seat)?;
            let response = client.task(TaskRequest::Delegate {
                task_path: task.clone(),
                seat_id: seat.record.id,
                role: None,
            })?;
            vec![
                json!({ "kind": "delegated", "task": task, "seat": seat.record.id, "response": to_value(&response) }),
            ]
        }
        Command::Capture { path, seat } => {
            let seat = match seat {
                Some(seat) => Some(resolve(&client.seats()?, &seat)?.record.id),
                None => None,
            };
            // A stale frame says so, so a judge never takes it for the current one.
            let (path, frame) = client.capture(&path, seat)?;
            vec![json!({ "kind": "captured", "path": path, "seat": seat,
                "stale": frame.stale, "occluded": frame.occluded,
                "frame_presented_at": frame.presented_at_ms, "offscreen":frame.offscreen,
                "frame_seq":frame.frame_seq,"rendered_at_ms":frame.rendered_at_ms })]
        }
        Command::E2e { group } => {
            let mut loader = crate::knowledge::E2eLoader::new(crate::knowledge::promises()?);
            // Validate the group before sending any provider request. The
            // projection still supplies the global summary for filtered rows.
            crate::knowledge::e2e_lines(loader.snapshot(), group.as_deref())?;
            while let Some(call) = loader.next_request() {
                loader.respond(client.knowledge(&call));
            }
            crate::knowledge::e2e_lines(loader.snapshot(), group.as_deref())?
        }
        Command::Screen {
            seat,
            human,
            scrollback,
            cells,
        } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let (op, mut screen) = client.screen_with(seat, cells)?;
            if let Some((from, to)) = scrollback {
                // The history rows of the range: its capture records replayed into a fresh
                // terminal the seat's size, gaps marked where the ring has none (#27).
                let records = client.history(seat, from, to)?;
                screen.rows = replay_rows(&records, screen.cols, screen.rows.len());
                screen.cells = None;
            }
            if human {
                let mut lines: Vec<Value> = screen.rows.into_iter().map(Value::String).collect();
                lines.push(Value::String(format!(
                    "-- cursor {},{} · {}",
                    screen.cursor.0, screen.cursor.1, screen.title
                )));
                lines
            } else {
                vec![
                    json!({ "ok": true, "op": op, "noun": "seat", "verb": "screen", "data": screen }),
                ]
            }
        }
        Command::Resize { seat, cols, rows } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let (_, screen) = client.screen(seat)?;
            let op = client.resize(seat, screen.generation, cols, rows)?;
            vec![
                json!({ "ok": true, "op": op, "noun": "seat", "verb": "resize",
                "data": { "seat": seat, "generation": screen.generation, "cols": cols, "rows": rows } }),
            ]
        }
        Command::Type { seat, text } => input(client, &seat, "type", |_| Ok(text.into_bytes()))?,
        Command::Key { seat, keys } => input(client, &seat, "key", |modes| {
            let mut bytes = vec![];
            for key in &keys {
                bytes.extend(key_bytes(key, modes)?);
            }
            Ok(bytes)
        })?,
        Command::Paste { seat, source } => {
            let mut text = vec![];
            if source == "-" {
                std::io::stdin().read_to_end(&mut text)
            } else {
                std::fs::File::open(&source).and_then(|mut f| f.read_to_end(&mut text))
            }
            .map_err(|e| format!("seat paste {source}: {e}"))?;
            paste(client, &seat, &text)?
        }
        Command::TaskCreate {
            title,
            verify,
            brief,
        } => {
            let mut row = client.record_write(task_create(&title, &verify, brief.as_deref()))?;
            row["kind"] = json!("task");
            vec![row]
        }
        Command::RecordWrite { source } => {
            let mut text = String::new();
            if source == "-" {
                std::io::stdin().read_to_string(&mut text)
            } else {
                std::fs::File::open(&source).and_then(|mut f| f.read_to_string(&mut text))
            }
            .map_err(|e| format!("record write {source}: {e}"))?;
            let args = serde_json::from_str(&text)
                .map_err(|e| format!("record write {source}: not JSON: {e}"))?;
            let mut row = client.record_write(args)?;
            row["kind"] = json!("record");
            vec![row]
        }
        Command::Wait {
            seat,
            on_screen,
            frame_seq,
            timeout,
            ..
        } if on_screen || frame_seq.is_some() => {
            // Window truth, not the model: the owner's latest produced layout (ctl-cli §1a).
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let op = Ulid::new();
            let deadline = std::time::Instant::now() + timeout;
            loop {
                let layout = match client.ui(bbt_proto::gui::UiRequest::Layout) {
                    Ok(layout) => Some(layout),
                    Err(reason) if reason.starts_with("ui_not_ready") => None,
                    Err(reason) => return Err(reason),
                };
                if let Some(layout) = &layout {
                    let pane = layout.panes.iter().find(|pane| pane.seat == seat);
                    let met = match frame_seq {
                        Some(n) => layout.frame_seq >= n,
                        None => pane.is_some(),
                    };
                    if met {
                        break vec![json!({ "ok": true, "op": op, "done": true,
                            "data": { "seat": seat, "frame_seq": layout.frame_seq,
                                "pane": pane.map(|p| json!({ "rect": p.rect, "cols": p.cols,
                                    "rows": p.rows, "focused": p.focused })) } })];
                    }
                }
                if std::time::Instant::now() >= deadline {
                    let seen = layout.map_or("no produced frame yet".into(), |l| {
                        format!("the window is at frame {}", l.frame_seq)
                    });
                    return Err(format!(
                        "{TIMEOUT_PREFIX}{op}\nwaited {} ms; {seen}",
                        timeout.as_millis()
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        Command::Wait {
            seat,
            state,
            text,
            timeout,
            ..
        } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let pattern = text
                .as_deref()
                .map(|p| {
                    regex::Regex::new(p).map_err(|e| format!("{USAGE_PREFIX}seat wait --text: {e}"))
                })
                .transpose()?;
            if let Some(state) = &state
                && !matches!(state.as_str(), "idle" | "waiting" | "exited")
            {
                return Err(format!(
                    "{USAGE_PREFIX}seat wait --state: idle, waiting or exited, not {state}"
                ));
            }
            let op = Ulid::new();
            let deadline = std::time::Instant::now() + timeout;
            loop {
                let (_, screen) = client.screen(seat)?;
                let met = match (&pattern, state.as_deref()) {
                    (Some(pattern), _) => pattern.is_match(&screen.rows.join("\n")),
                    (None, Some(state)) => {
                        let seats = client.seats()?;
                        let line = seat_line(resolve(&seats, &seat.to_string())?);
                        match state {
                            "exited" => line["process"]["kind"] == "exited",
                            kind => line["activity"]["kind"] == kind,
                        }
                    }
                    (None, None) => unreachable!("parse requires one condition"),
                };
                if met {
                    break vec![json!({ "ok": true, "op": op, "done": true,
                        "data": { "seat": seat, "state": state, "text": text } })];
                }
                if std::time::Instant::now() >= deadline {
                    let tail: Vec<_> = screen.rows.iter().filter(|r| !r.is_empty()).collect();
                    let tail = &tail[tail.len().saturating_sub(12)..];
                    return Err(format!(
                        "{TIMEOUT_PREFIX}{op}\nwaited {} ms; the screen ends:\n{}",
                        timeout.as_millis(),
                        tail.iter()
                            .map(|r| r.as_str())
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }
        Command::Ui(_)
        | Command::UiFocus(_)
        | Command::UiScroll { .. }
        | Command::SpawnAt(_)
        | Command::UiNotificationClick(_)
        | Command::DebugNotifications
        | Command::Watch { .. }
        | Command::Rename { .. }
        | Command::Restart { .. }
        | Command::Close { .. }
        | Command::UndoClose { .. }
        | Command::Focus { .. }
        | Command::Send { .. }
        | Command::Ack { .. }
        | Command::TaskList { .. }
        | Command::TaskGet { .. }
        | Command::RecordFetch { .. }
        | Command::RecordSearch { .. }
        | Command::RecordChanges { .. }
        | Command::EventsStreams
        | Command::Debug { .. }
        | Command::EventsTail { .. }
        | Command::TaskReport { .. }
        | Command::TaskAccept { .. }
        | Command::TaskReject { .. }
        | Command::AppEndpoint
        | Command::AppStart { .. }
        | Command::AppStatus
        | Command::AppResize { .. }
        | Command::AppStop => {
            return Err(format!(
                "{USAGE_PREFIX}this verb has no flat form; use the noun-verb grammar"
            ));
        }
        Command::Reports { since } => {
            let mut cards = vec![];
            let mut after = None;
            loop {
                let TaskResponse::Items { items, next } = client.task(TaskRequest::List {
                    unassigned: false,
                    swarm_id: None,
                    status: None,
                    assignee: None,
                    after,
                    limit: Some(PAGE),
                })?
                else {
                    return Err("reports: unexpected task response".into());
                };
                cards.extend(items);
                match next {
                    Some(next) if Some(next) != after => after = Some(next),
                    _ => break,
                }
            }
            let mut out = vec![];
            for card in cards {
                if since
                    .as_deref()
                    .is_some_and(|since| card.updated.as_deref().is_none_or(|at| at < since))
                {
                    continue;
                }
                let TaskResponse::Item { item } = client.task(TaskRequest::Status {
                    task_path: Some(card.path.clone()),
                    task_id: None,
                })?
                else {
                    return Err("reports: unexpected task response".into());
                };
                let report = &item.metadata["report"];
                if report.is_null() {
                    continue;
                }
                out.push(json!({
                    "kind": "report",
                    "task": card.id,
                    "path": card.path,
                    "title": card.title,
                    "status": card.status,
                    "assignee": card.assignee,
                    "updated": card.updated,
                    "summary": report["summary"],
                    "commit": report["commit"],
                    "reported_by": report["reported_by"],
                    "verify_result": item.metadata["verify_result"],
                }));
            }
            out
        }
    })
}

/// Object-safe serialisation for the reply payloads `run` echoes.
mod erased {
    pub trait Serialize {
        fn to_json(&self) -> serde_json::Value;
    }
    impl<T: serde::Serialize> Serialize for T {
        fn to_json(&self) -> serde_json::Value {
            serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
        }
    }
}

/// The `ctl` subcommand: parse, connect, run, print one JSON object per line.
/// A `seat wait` timeout: printed in the ctl-cli §3 error shape with exit 3.
const TIMEOUT_PREFIX: &str = "timeout: ";

/// A fresh id. Not `Ulid::default()`, which is the nil id.
fn fresh() -> Ulid {
    Ulid::new()
}

/// A ctl-cli §3 error: a kind the specs already name, one sentence, what would be valid,
/// and the exit code.
#[derive(Debug)]
pub struct Fail {
    pub kind: &'static str,
    pub message: String,
    pub hint: String,
    pub exit: i32,
}
/// Names a failure by the kinds the specs use (ctl-cli §3); every one carries a hint.
pub fn classify(reason: &str) -> Fail {
    let fail = |kind, hint: &str, exit| Fail {
        kind,
        message: reason.to_owned(),
        hint: hint.to_owned(),
        exit,
    };
    if let Some(rest) = reason.strip_prefix(USAGE_PREFIX) {
        return Fail {
            kind: "usage",
            message: rest.to_owned(),
            hint: "bboxy450-term ctl --help lists every noun and verb".into(),
            exit: 2,
        };
    }
    if reason.starts_with("not_on_screen:") {
        return fail("not_on_screen", "ctl seat focus <seat>", 1);
    }
    if reason.starts_with("focus_timeout:") {
        return fail(
            "focus_timeout",
            "inspect ctl app status and the run's app.log",
            1,
        );
    }
    if reason.starts_with("focus_superseded:") {
        return fail(
            "focus_superseded",
            "a newer UI action replaced this focus; inspect ctl app status",
            1,
        );
    }
    if reason == bbt_mux::ctl_door::REFUSAL {
        return fail(
            "ancestry_refused",
            "a seat uses its MCP tools; ctl is the operator's door",
            1,
        );
    }
    if reason.starts_with("unknown: timeout:") {
        return fail(
            "timeout",
            "the GUI owner did not answer; retain --op and inspect app status",
            3,
        );
    }
    if reason.starts_with("ui_refused: endpoint_unavailable:") {
        return fail(
            "endpoint_unavailable",
            "start a native or headless app owner with --ctl",
            4,
        );
    }
    if reason.starts_with("layout_owned") {
        return fail(
            "layout_owned",
            "ctl app resize <cols>x<rows> resizes the window; its layout sizes each pane",
            1,
        );
    }
    if reason.starts_with("app endpoint unavailable:") {
        return fail(
            "endpoint_unavailable",
            "inspect ctl debug ps and the run's app.log; no PID-kill fallback is used",
            4,
        );
    }
    if reason.starts_with("endpoint ") || reason.starts_with("gui.sock ") && !reason.contains(": ")
    {
        return fail(
            "endpoint_unavailable",
            "start the app with --ctl, or point --run-dir or BBOXY_RUN_DIR at its run directory",
            4,
        );
    }
    if reason.starts_with("gui.sock") || reason == "GUI socket admission rejected" {
        return fail(
            "endpoint_unavailable",
            "the endpoint is stale or the app stopped: restart it with --ctl, or point --run-dir at the live run",
            4,
        );
    }
    if let Some((_, hint)) = reason
        .split_once("unknown destination ")
        .and_then(|(_, r)| r.split_once(": "))
    {
        return Fail {
            kind: "dangling_destination",
            message: reason.to_owned(),
            hint: hint.to_owned(),
            exit: 1,
        };
    }
    if reason.contains("op_id") || reason.contains("conflict") {
        return fail(
            "op_id_reused",
            "a retry reuses --op with the same arguments; new work takes a new --op",
            1,
        );
    }
    if reason.starts_with("no seat ") || reason.contains("seats are named") {
        return fail(
            "refused",
            "bboxy450-term ctl seat ls lists the seats by id and name",
            1,
        );
    }
    fail(
        "refused",
        "the message is the app's own reason; correct the request it names",
        1,
    )
}

pub fn main(args: &[String], default_endpoint: &Path) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let parsed = parse_full(args);
    // The noun-verb grammar answers in the ctl-cli §2 and §3 shapes; the flat names keep
    // their own output for one release.
    let noun_verb = parsed
        .as_ref()
        .map(|p| p.verb.is_some())
        .unwrap_or_else(|reason| reason.starts_with(USAGE_PREFIX));
    if !noun_verb {
        if let Some(word) = args.iter().find(|a| !a.starts_with("--"))
            && let Some((_, new)) = LEGACY.iter().find(|(old, _)| old == word)
        {
            eprintln!("bboxy450-term ctl {word}: deprecated, use: ctl {new}");
        }
        return match run_main(args, default_endpoint) {
            Err(reason) if reason.starts_with(TIMEOUT_PREFIX) => {
                let (op, message) = reason[TIMEOUT_PREFIX.len()..]
                    .split_once('\n')
                    .unwrap_or(("", &reason));
                println!(
                    "{}",
                    json!({ "ok": false, "op": op, "error": { "kind": "timeout", "message": message,
                        "hint": "raise --timeout, or wait for another --state or --text" } })
                );
                std::process::exit(3);
            }
            result => result,
        };
    }
    let mut op = None;
    let mut tail = None;
    let result = parsed.and_then(|parsed| {
        op = parsed.op;
        if let Command::EventsTail { stream, since, .. } = &parsed.command {
            tail = Some(events::Progress::new(stream.clone(), *since));
        }
        run_verb(args, parsed, default_endpoint, &mut op, &mut tail)
    });
    if let Err(reason) = result {
        let (op, fail) = match reason.strip_prefix(TIMEOUT_PREFIX) {
            Some(rest) => {
                let (op, message) = rest.split_once('\n').unwrap_or(("", rest));
                (
                    op.parse().ok(),
                    Fail {
                        kind: "timeout",
                        message: message.to_owned(),
                        hint: "raise --timeout, or wait for another --state or --text".into(),
                        exit: 3,
                    },
                )
            }
            None => (op, classify(&reason)),
        };
        let mut line = json!({ "ok": false, "op": op.unwrap_or_else(fresh), "error": {
            "kind": fail.kind, "message": fail.message, "hint": fail.hint } });
        // not_on_screen names the frame it checked: before a relayed verb's reply that frame
        // means early, after it hidden (ctl-cli §1a).
        if let Some(frame) = fail
            .message
            .split_once(" in frame ")
            .and_then(|(_, n)| n.split_whitespace().next()?.parse::<u64>().ok())
            .filter(|_| fail.kind == "not_on_screen")
        {
            line["error"]["frame_seq"] = json!(frame);
        }
        if let Some(tail) = tail {
            tail.error(&mut line);
        }
        println!("{line}");
        std::process::exit(fail.exit);
    }
    Ok(())
}

/// One noun-verb verb: connect (unless it only reads the run directory), issue its action,
/// print its `data` in the §2 envelope, or `--human` as text.
fn run_verb(
    args: &[String],
    parsed: Parsed,
    default_endpoint: &Path,
    op: &mut Option<Ulid>,
    tail: &mut Option<events::Progress>,
) -> Result<(), String> {
    let (noun, verb) = parsed.verb.expect("a noun-verb form");
    let endpoint_file = parsed
        .endpoint
        .clone()
        .unwrap_or_else(|| default_endpoint.to_path_buf());
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut print = |op: Ulid, data: Value, human: bool| -> Result<(), String> {
        let line = if human {
            serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
        } else {
            json!({ "ok": true, "op": op, "noun": noun, "verb": verb, "data": data }).to_string()
        };
        writeln!(out, "{line}").map_err(|e| e.to_string())
    };
    if let Command::AppStart {
        headless,
        ctl,
        unattended,
        test_account,
        record_notifications,
    } = &parsed.command
    {
        let data = app::start(
            &endpoint_file,
            *headless,
            *ctl,
            *unattended,
            test_account.as_deref(),
            *record_notifications,
        )?;
        return print(op.unwrap_or_else(fresh), data, parsed.human);
    }
    if let Command::AppEndpoint = parsed.command {
        // The run directory and the generation, never the capability (#275).
        let text = std::fs::read_to_string(&endpoint_file)
            .map_err(|e| format!("endpoint {}: {e}", endpoint_file.display()))?;
        let value: Value = serde_json::from_str(&text).map_err(|e| format!("endpoint: {e}"))?;
        let id = op.unwrap_or_else(fresh);
        return print(
            id,
            json!({ "run_dir": endpoint_file.parent(), "path": value["path"],
                "generation": value["generation"] }),
            parsed.human,
        );
    }
    let mut client = connect(args, &endpoint_file).map_err(|reason| {
        if matches!(&parsed.command, Command::AppStop) {
            format!("app endpoint unavailable: {reason}")
        } else {
            reason
        }
    })?;
    client.op = parsed.op;
    let id = parsed.op.unwrap_or_else(fresh);
    *op = Some(id);
    client.op = Some(id);
    let human = parsed.human;
    let data = match parsed.command {
        Command::Debug { request, snapshot } => {
            debug::read(&mut client, request, snapshot.as_deref())?
        }
        Command::EventsStreams => json!({"streams":client.event_streams()?}),
        Command::EventsTail {
            subject, timeout, ..
        } => {
            return events::tail(
                &mut client,
                id,
                tail.as_mut().expect("tail progress"),
                subject.as_deref(),
                timeout,
                &mut out,
            );
        }
        Command::RecordFetch { locator } => records::fetch(&mut client, locator)?,
        Command::RecordSearch { query, cursor } => records::search(&mut client, query, cursor)?,
        Command::RecordChanges { since } => records::changes(&mut client, since)?,
        Command::Ui(request) => {
            serde_json::to_value(client.ui(request)?).map_err(|e| e.to_string())?
        }
        Command::SpawnAt(pick) => {
            let harness = pick.harness.clone();
            let layout = client.ui(bbt_proto::gui::UiRequest::Spawn(pick))?;
            let placed = layout
                .spawned
                .as_ref()
                .ok_or("ui_frame_refused: spawn reply has no placement receipt")?;
            json!({"seat":placed.seat,"swarm":placed.swarm,"harness":harness,
                "spawn_op":placed.op,"placement":placed.placement,"layout_revision":placed.layout_revision,
                "applied_layout_revision":placed.applied_layout_revision,
                "frame_seq":layout.frame_seq,"layout":layout})
        }
        Command::UiFocus(seat) => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            serde_json::to_value(client.ui(bbt_proto::gui::UiRequest::Focus { seat })?)
                .map_err(|e| e.to_string())?
        }
        // The window replies with its first produced layout at the new size (ctl-cli §1a).
        Command::AppResize { cols, rows } => {
            serde_json::to_value(client.ui(bbt_proto::gui::UiRequest::Resize { cols, rows })?)
                .map_err(|e| e.to_string())?
        }
        Command::UiScroll { seat, motion } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            serde_json::to_value(client.ui(bbt_proto::gui::UiRequest::Scroll { seat, motion })?)
                .map_err(|e| e.to_string())?
        }
        Command::UiNotificationClick(seat) => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            serde_json::to_value(client.ui(bbt_proto::gui::UiRequest::NotificationClick { seat })?)
                .map_err(|e| e.to_string())?
        }
        Command::DebugNotifications => {
            client
                .app(bbt_proto::gui::AppRequest::Notifications)?
                .notifications
        }
        Command::AppStatus => serde_json::to_value(client.app(bbt_proto::gui::AppRequest::Status)?)
            .map_err(|e| e.to_string())?,
        Command::AppStop => serde_json::to_value(app::stop(&mut client, &endpoint_file)?)
            .map_err(|e| e.to_string())?,
        Command::Seats => serde_json::to_value(client.seats()?).map_err(|e| e.to_string())?,
        Command::Screen { .. }
        | Command::Resize { .. }
        | Command::Type { .. }
        | Command::Key { .. }
        | Command::Paste { .. }
        | Command::Wait { .. } => {
            // Already in the §2 shape (and --human for screen).
            for line in run(&mut client, parsed.command)? {
                match line {
                    Value::String(text) => writeln!(out, "{text}"),
                    line => writeln!(out, "{line}"),
                }
                .map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        Command::Rename { seat, name } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let op = client.mint();
            client.call(
                Some(op),
                GuiAction::RenameSeat {
                    id: op,
                    seat,
                    name: name.clone(),
                },
                |r| matches!(r, GuiReply::Ok { op: o } if *o == op),
            )?;
            json!({ "seat": seat, "name": name })
        }
        Command::Restart { seat } => {
            let seat = resolve(&client.seats()?, &seat)?.record.clone();
            let op = client.mint();
            client.call(
                Some(op),
                GuiAction::RestartSeat {
                    id: op,
                    seat: seat.id,
                    generation: seat.generation,
                },
                |r| matches!(r, GuiReply::Ok { op: o } if *o == op),
            )?;
            json!({ "seat": seat.id, "from_generation": seat.generation })
        }
        command @ (Command::Close { .. } | Command::UndoClose { .. }) => {
            let (seat, undo) = match command {
                Command::Close { seat } => (seat, false),
                Command::UndoClose { seat } => (seat, true),
                _ => unreachable!(),
            };
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let op = client.mint();
            let reply = client.call(
                Some(op),
                if undo {
                    GuiAction::UndoCloseSeat { id: op, seat }
                } else {
                    GuiAction::CloseSeat { id: op, seat }
                },
                |r| matches!(r,GuiReply::Ui {op:o,..} if *o==op),
            )?;
            let GuiReply::Ui { layout, .. } = reply else {
                unreachable!()
            };
            if layout.frame_seq == 0 {
                return Err(
                    "close_frame_incomplete: the GUI owner returned no produced frame".into(),
                );
            }
            json!({"seat":seat,"relayed":true,"applied":true,"action":if undo {"undo-close"} else {"close"},"frame_seq":layout.frame_seq,"layout":layout})
        }
        Command::Focus { seat } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let op = client.mint();
            let reply = client.call(
                Some(op),
                GuiAction::FocusSeat { id: op, seat },
                |r| matches!(r, GuiReply::Focused { op: o, .. } if *o == op),
            )?;
            let GuiReply::Focused { frame, .. } = reply else {
                unreachable!()
            };
            if frame.seat.0.0 != seat.0 {
                return Err("focus_superseded: GUI acknowledged a different seat".into());
            }
            json!({"seat":seat,"applied":true,"generation":frame.generation,"frame_seq":frame.frame_seq})
        }
        Command::Watch { seat, timeout } => {
            // State: the seat's retained stream paged by our own cursor, never the window's
            // lane. Screen: polled, a line only when its capture sequence moves (#272).
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let stream = format!("seat-{seat}");
            let deadline = std::time::Instant::now() + timeout;
            let (mut after, mut state, mut seen, mut events) = (0, None, None, 0u64);
            loop {
                while let (page, Some(next)) = client.events_read(&stream, after, 128)? {
                    state = page
                        .into_iter()
                        .rev()
                        .find(|e| e.subject.ends_with("/state"))
                        .or(state);
                    after = next;
                }
                let (page, _) = client.events_read(&stream, after, 128)?;
                if let Some(last) = page.last() {
                    after = last.seq;
                }
                for event in page.into_iter().filter(|e| e.subject.ends_with("/state")) {
                    state = Some(event);
                }
                if let Some(event) = state.take() {
                    events += 1;
                    writeln!(
                        out,
                        "{}",
                        json!({ "ok": true, "op": id, "event": {
                        "kind": "state", "seat": seat, "seq": event.seq, "state": event.payload } })
                    )
                    .map_err(|e| e.to_string())?;
                }
                let (_, screen) = client.screen(seat)?;
                if seen != Some(screen.sequence) {
                    seen = Some(screen.sequence);
                    events += 1;
                    writeln!(
                        out,
                        "{}",
                        json!({ "ok": true, "op": id, "event": {
                        "kind": "screen", "seat": seat, "sequence": screen.sequence,
                        "cursor": screen.cursor, "rows": screen.rows } })
                    )
                    .map_err(|e| e.to_string())?;
                }
                out.flush().map_err(|e| e.to_string())?;
                if std::time::Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            writeln!(
                out,
                "{}",
                json!({ "ok": true, "op": id, "done": true,
                "data": { "seat": seat, "events": events } })
            )
            .map_err(|e| e.to_string())?;
            return Ok(());
        }
        Command::Send {
            to,
            text,
            reply_to,
            swarm,
        } => {
            let seats = client.seats()?;
            // A seat name or id addresses its inbox; anything else is a destination path.
            let (destination, seat_swarm) = match to.as_deref() {
                Some(to) if !to.contains('/') => {
                    let seat = resolve(&seats, to)?;
                    (
                        format!("seats/{}/inbox", seat.record.id),
                        Some(seat.record.swarm),
                    )
                }
                Some(path) => (path.to_owned(), None),
                None => (String::new(), None),
            };
            let swarm = swarm
                .or(seat_swarm)
                .or_else(|| swarms(&seats).first().copied())
                .ok_or("no seats: pass --swarm")?;
            let mut message = SendRequest::text(destination, text);
            message.id = id;
            message.reply_to = reply_to;
            serde_json::to_value(client.messaging(swarm, Request::Send { message })?)
                .map_err(|e| e.to_string())?
        }
        Command::Ack { message } => {
            let mut last = String::from("no seats, so no swarm to ask");
            let mut answer = None;
            for swarm in swarms(&client.seats()?) {
                match client.messaging(
                    swarm,
                    Request::Acknowledge {
                        message,
                        agent: None,
                    },
                ) {
                    Ok(response) => {
                        answer = Some(response);
                        break;
                    }
                    Err(reason) => last = reason,
                }
            }
            serde_json::to_value(answer.ok_or(format!("ack {message}: {last}"))?)
                .map_err(|e| e.to_string())?
        }
        Command::TaskList {
            unassigned,
            seat,
            status,
        } => {
            let assignee = match seat {
                Some(seat) => Some(resolve(&client.seats()?, &seat)?.record.id.to_string()),
                None => None,
            };
            let mut cards = vec![];
            let mut after = None;
            loop {
                let TaskResponse::Items { items, next } = client.task(TaskRequest::List {
                    unassigned,
                    swarm_id: None,
                    status: status.clone(),
                    assignee: assignee.clone(),
                    after,
                    limit: Some(PAGE),
                })?
                else {
                    return Err("task list: unexpected task response".into());
                };
                cards.extend(items);
                match next {
                    Some(next) if Some(next) != after => after = Some(next),
                    _ => break,
                }
            }
            serde_json::to_value(cards).map_err(|e| e.to_string())?
        }
        Command::TaskGet { task } => {
            let request = match task.parse::<Ulid>() {
                Ok(id) => TaskRequest::Status {
                    task_path: None,
                    task_id: Some(id),
                },
                Err(_) => TaskRequest::Status {
                    task_path: Some(task),
                    task_id: None,
                },
            };
            serde_json::to_value(client.task(request)?).map_err(|e| e.to_string())?
        }
        Command::TaskReport {
            task,
            summary,
            commit,
            verify_result,
        } => serde_json::to_value(client.task(TaskRequest::Report {
            task_path: task,
            summary,
            commit,
            verify_result,
        })?)
        .map_err(|e| e.to_string())?,
        Command::TaskAccept { task } => {
            serde_json::to_value(client.task(TaskRequest::Accept { task_path: task })?)
                .map_err(|e| e.to_string())?
        }
        Command::TaskReject { task, reason } => {
            serde_json::to_value(client.task(TaskRequest::Reject {
                task_path: task,
                reason,
            })?)
            .map_err(|e| e.to_string())?
        }
        command => {
            // Spawn, post, inbox, receipts, delegate, capture, e2e list: the action's reply
            // as the flat form already carries it, minus the flat form's own `kind`.
            let mut rows: Vec<Value> = run(&mut client, command)?
                .into_iter()
                .map(|mut row| {
                    if let Some(object) = row.as_object_mut() {
                        object.remove("kind");
                    }
                    row
                })
                .collect();
            if rows.len() == 1 && !matches!(verb, "inbox" | "receipts" | "list") {
                rows.remove(0)
            } else {
                Value::Array(rows)
            }
        }
    };
    print(id, data, human)
}

/// Connects and names the issuer (`--as` or `BBOXY_CTL_ISSUER`, #269).
fn connect(args: &[String], endpoint_file: &Path) -> Result<Client, String> {
    let issuer = args
        .windows(2)
        .find(|w| w[0] == "--as")
        .map(|w| w[1].clone())
        .or_else(|| std::env::var("BBOXY_CTL_ISSUER").ok());
    let endpoint = Endpoint::read(endpoint_file)?;
    let mut client = Client::connect(&endpoint)?;
    if let Some(label) = issuer.filter(|label| !label.is_empty()) {
        client.issue(label)?;
    }
    Ok(client)
}

fn run_main(args: &[String], default_endpoint: &Path) -> Result<(), String> {
    // --as names the issuer (decision #269); it is not a command flag.
    let original = args;
    let mut args = args.to_vec();
    if let Some(at) = args.iter().position(|arg| arg == "--as") {
        args.remove(at);
        if at >= args.len() {
            return Err("--as needs a value".into());
        }
        args.remove(at);
    }
    let (endpoint, command) = parse(&args)?;
    let mut client = connect(original, endpoint.as_deref().unwrap_or(default_endpoint))?;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in run(&mut client, command)? {
        // `ctl screen` without --json prints the screen's own text, not a JSON string.
        match line {
            Value::String(text) => writeln!(out, "{text}"),
            line => writeln!(out, "{line}"),
        }
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The widest `--scrollback` range, in capture sequences: bounded so a reply fits the frame.
const SCROLLBACK_SEQUENCES: u64 = 4096;
/// The longest history a `--scrollback` replay keeps, in rows.
const SCROLLBACK_ROWS: usize = 10_000;

/// Replays capture records into a fresh terminal `cols` wide and `rows` high and returns
/// every row it holds, history first, as text with trailing blanks trimmed. A gap in the
/// ring becomes one marker row.
pub fn replay_rows(
    records: &[bbt_proto::gui::HistoryRecord],
    cols: usize,
    rows: usize,
) -> Vec<String> {
    use bbt_proto::gui::HistoryRecord;
    let mut term =
        bbt_core::Term::new_with_spare_rows(cols.max(2), rows.max(1), SCROLLBACK_ROWS, 0);
    for record in records {
        match record {
            HistoryRecord::Bytes { bytes, .. } => term.advance(bytes),
            HistoryRecord::Gap { from, to, reason } => {
                term.advance(format!("\r\n[gap {from}..{to}: {reason}]\r\n").as_bytes())
            }
        }
        drop(term.take_events());
    }
    let bounds = term.history_bounds();
    term.history_window(bounds.start, (bounds.end - bounds.start) as usize)
        .unwrap_or_default()
        .iter()
        .map(|row| {
            row.iter()
                .filter(|cell| {
                    !cell.flags.intersects(
                        bbt_core::term::cell::Flags::WIDE_CHAR_SPACER
                            | bbt_core::term::cell::Flags::LEADING_WIDE_CHAR_SPACER,
                    )
                })
                .map(|cell| cell.c)
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// One input through the window's path: the seat's current generation and modes, the
/// bytes from `make`, and the ctl-cli reply.
fn input(
    client: &mut Client,
    seat: &str,
    verb: &str,
    make: impl FnOnce(u64) -> Result<Vec<u8>, String>,
) -> Result<Vec<Value>, String> {
    let seat = resolve(&client.seats()?, seat)?.record.id;
    let (_, screen) = client.screen(seat)?;
    let bytes = make(screen.modes)?;
    if bytes.is_empty() {
        return Err(format!("seat {verb}: nothing to send"));
    }
    let count = bytes.len();
    let op = client.input(seat, screen.generation, bytes)?;
    Ok(vec![
        json!({ "ok": true, "op": op, "noun": "seat", "verb": verb,
        "data": { "seat": seat, "generation": screen.generation, "bytes": count } }),
    ])
}

/// The most one input carries: the supervisor refuses more than 64 KiB in one write.
const PASTE_CHUNK: usize = 32 * 1024;

/// A paste of any size: bracketed once around the whole text when the program asked, sent
/// as inputs of at most `PASTE_CHUNK` bytes in order, each its own journaled op.
fn paste(client: &mut Client, seat: &str, text: &[u8]) -> Result<Vec<Value>, String> {
    let seat = resolve(&client.seats()?, seat)?.record.id;
    let (_, screen) = client.screen(seat)?;
    let bytes = bracketed(text, screen.modes);
    if bytes.is_empty() {
        return Err("seat paste: nothing to send".into());
    }
    let mut ops = vec![];
    for chunk in bytes.chunks(PASTE_CHUNK) {
        // The supervisor takes one input at a time and refuses the next while one is in
        // flight, before applying it: wait the chunk out, never drop or reorder it.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        ops.push(loop {
            match client.input(seat, screen.generation, chunk.to_vec()) {
                Err(e) if e.contains("Busy:") && std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                result => break result?,
            }
        });
    }
    Ok(vec![
        json!({ "ok": true, "op": ops[0], "noun": "seat", "verb": "paste",
        "data": { "seat": seat, "generation": screen.generation, "bytes": bytes.len(),
            "inputs": ops.len() } }),
    ])
}

/// The record write of a new task: open and unassigned at `tasks/<date>-<slug>.md`; logic
/// adds the operator as its delegator and the Index its number.
pub fn task_create(title: &str, verify: &str, brief: Option<&str>) -> Value {
    let now = utc_now();
    let mut slug = String::new();
    for c in title.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_end_matches('-').chars().take(48).collect();
    let slug = slug.trim_end_matches('-');
    json!({ "path": format!("tasks/{}-{slug}.md", &now[..10]),
        "frontmatter": { "type": "task", "title": title, "verify": verify, "status": "open",
            "assignee": "none", "role": "executor" },
        "body": format!("# {title}\n\n{}\n", brief.unwrap_or("")) })
}

/// Now as RFC 3339 UTC, whole seconds.
pub fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Civil date from days since 1970-01-01 (Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

/// A paste as the window sends one: bracketed when the program enabled bracketed paste,
/// with any end marker inside the text removed so the paste cannot end itself early.
pub fn bracketed(text: &[u8], modes: u64) -> Vec<u8> {
    use bbt_core::term::TermMode;
    if !TermMode::from_bits_retain(modes as u32).contains(TermMode::BRACKETED_PASTE) {
        return text.to_vec();
    }
    const END: &[u8] = b"\x1b[201~";
    let mut out = b"\x1b[200~".to_vec();
    let mut rest = text;
    while let Some(at) = rest.windows(END.len()).position(|w| w == END) {
        out.extend(&rest[..at]);
        rest = &rest[at + END.len()..];
    }
    out.extend(rest);
    out.extend(END);
    out
}

/// A named key, encoded by the window's keyboard adapter against the seat's current modes,
/// so a seat with the Kitty protocol on gets Kitty sequences.
pub fn key_bytes(name: &str, modes: u64) -> Result<Vec<u8>, String> {
    crate::keyboard::named_key_bytes(name, modes)
        .ok_or_else(|| format!("seat key: unknown key {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(s: &str) -> Vec<String> {
        s.split(' ').map(Into::into).collect()
    }
    /// ctl-cli §1: one table, each verb once, each naming one real window action, and every
    /// verb in it parses.
    #[test]
    fn every_verb_maps_to_exactly_one_window_action() {
        let mut seen = std::collections::BTreeSet::new();
        for (noun, verb, _) in VERBS {
            assert!(seen.insert((noun, verb)), "{noun} {verb} twice");
        }
        let id = Ulid::new();
        let seat = bbt_proto::SeatId::from_u128(1);
        let samples = [
            GuiAction::Ui {
                id,
                request: bbt_proto::gui::UiRequest::Layout,
            },
            GuiAction::Identity(bbt_proto::identity::IdentityCall {
                id,
                request: bbt_proto::identity::IdentityRequest::Status,
            }),
            GuiAction::ListSeats,
            GuiAction::Debug {
                id,
                request: bbt_proto::gui::DebugRequest::Metrics,
            },
            GuiAction::App {
                id,
                request: bbt_proto::gui::AppRequest::Status,
            },
            GuiAction::Screen {
                id,
                seat: id,
                cells: false,
            },
            GuiAction::Resize {
                seat,
                generation: 1,
                cols: 80,
                rows: 24,
            },
            GuiAction::EventsRead {
                id,
                stream: "s".into(),
                after: 0,
                limit: 1,
            },
            GuiAction::EventsStreams { id },
            GuiAction::RenameSeat {
                id,
                seat: id,
                name: "n".into(),
            },
            GuiAction::CloseSeat { id, seat: id },
            GuiAction::UndoCloseSeat { id, seat: id },
            GuiAction::FocusSeat { id, seat: id },
            GuiAction::Input {
                seat,
                generation: 1,
                id,
                bytes: vec![],
            },
            GuiAction::Capture {
                id,
                path: "/p".into(),
                seat: None,
            },
            GuiAction::Messaging {
                id,
                swarm_id: SwarmId::from_u128(1),
                request: Request::List {
                    thread: None,
                    channel: None,
                    after: 0,
                    limit: 1,
                },
            },
            GuiAction::Task {
                id,
                request: TaskRequest::Status {
                    task_path: None,
                    task_id: Some(id),
                },
            },
            GuiAction::Knowledge {
                id,
                method: "list".into(),
                args: json!({}),
            },
            GuiAction::RecordWrite {
                id,
                args: json!({}),
            },
            GuiAction::RestartSeat {
                id,
                seat: id,
                generation: 1,
            },
        ];
        let kinds: Vec<String> = samples
            .iter()
            .map(|a| {
                serde_json::to_value(a).unwrap()["kind"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .chain(["Spawn".to_owned()])
            .collect();
        for (noun, verb, action) in VERBS {
            assert!(
                *action == "-" || kinds.iter().any(|k| k == action),
                "{noun} {verb}: {action}"
            );
        }
        let parse_samples = [
            "ui open graph",
            "ui key cmd-t esc",
            "ui focus s",
            "ui scroll s top",
            "ui layout",
            "ui clock freeze",
            "identity status",
            "identity unlock",
            "ui notification-click s",
            "debug notifications",
            "seat ls",
            "seat spawn zsh",
            "seat screen s",
            "seat type s x",
            "seat key s enter",
            "seat paste s -",
            "seat wait s --state idle",
            "seat capture window /tmp/x.png",
            "message send s hi",
            "message post swarms/x hi",
            "message inbox",
            "message receipts 01M5Q8A7DQ7EQFKBZH4NXBW3QQ",
            "message ack 01M5Q8A7DQ7EQFKBZH4NXBW3QQ",
            "task list",
            "task get #1",
            "task status #1",
            "task report #1 --summary proof",
            "task accept #1",
            "task reject #1 --reason incomplete",
            "task delegate #1 s",
            "task create t --verify true",
            "e2e list",
            "record write -",
            "record fetch #1",
            "record search example",
            "record changes --since 0",
            "events streams",
            "events tail --stream account",
            "debug logs app",
            "debug journal account/01M5Q8A7DQ7EQFKBZH4NXBW3QQ",
            "debug metrics",
            "debug snapshot /tmp/debug.json",
            "debug ps",
            "app endpoint",
            "seat watch s",
            "seat rename s n",
            "seat close s",
            "seat undo-close s",
            "seat focus s",
            "seat restart s",
            "seat resize s 80x24",
            "app start --headless --ctl",
            "app status",
            "app resize 100x30",
            "app stop",
        ];
        assert_eq!(
            VERBS.len(),
            parse_samples.len(),
            "a new verb needs a parse line above"
        );
        for line in parse_samples {
            let parsed = parse_full(&args(line)).unwrap_or_else(|e| panic!("{line}: {e}"));
            let (noun, verb) = parsed
                .verb
                .unwrap_or_else(|| panic!("{line}: not a table verb"));
            assert!(line.starts_with(&format!("{noun} {verb}")));
        }
    }
    #[test]
    fn task_verbs_parse_typed_evidence_and_require_their_own_flags() {
        let verification = TaskVerification {
            command: "cargo test --locked".into(),
            exit_code: 9,
            evidence: vec![bbt_proto::messaging::TaskEvidence {
                criterion: "M4".into(),
                reference: "fixture.log:1".into(),
            }],
        };
        let encoded = serde_json::to_string(&verification).unwrap();
        let op = Ulid::new();
        let op_text = op.to_string();
        let parsed = parse_full(
            &[
                "task",
                "report",
                "#1",
                "--summary",
                "actual output",
                "--commit",
                "abc123",
                "--verify-result",
                &encoded,
                "--op",
                &op_text,
            ]
            .map(String::from),
        )
        .unwrap();
        assert_eq!(parsed.op, Some(op));
        assert_eq!(parsed.verb, Some(("task", "report")));
        assert_eq!(
            parsed.command,
            Command::TaskReport {
                task: "#1".into(),
                summary: "actual output".into(),
                commit: Some("abc123".into()),
                verify_result: Some(verification),
            }
        );
        for line in [
            "task status",
            "task accept",
            "task report #1",
            "task reject #1",
            "task report #1 --summary x --verify-result nope",
            "task report #1 --summary x --verify-result {}",
            "task accept #1 --verify-result {}",
            "task status #1 --reason x",
            "task reject #1 --reason x --summary unused",
        ] {
            let error = parse_full(&args(line)).unwrap_err();
            assert_eq!(classify(&error).exit, 2, "{line}: {error}");
        }
    }
    #[test]
    fn ui_scroll_parses_only_the_typed_window_motions() {
        use bbt_proto::gui::UiScroll;
        for (text, expected) in [
            ("top", UiScroll::Top),
            ("bottom", UiScroll::Bottom),
            ("page-up", UiScroll::PageUp),
            ("page-down", UiScroll::PageDown),
            ("row 18446744073709551615", UiScroll::Row(u64::MAX)),
        ] {
            let parsed = parse_full(&args(&format!("ui scroll pane {text}"))).unwrap();
            assert!(
                matches!(parsed.command, Command::UiScroll {seat, motion} if seat == "pane" && motion == expected)
            );
        }
        for line in [
            "ui scroll pane",
            "ui scroll pane up",
            "ui scroll pane row",
            "ui scroll pane row -1",
            "ui scroll pane row 18446744073709551616",
            "ui scroll pane top extra",
        ] {
            assert!(parse_full(&args(line)).is_err(), "{line}");
        }
    }
    #[test]
    fn ui_keys_are_bounded_chrome_names_and_never_terminal_text() {
        for line in [
            "ui key",
            "ui key hello",
            "ui open docs",
            "ui open terminal",
            "ui scroll down",
        ] {
            assert!(parse_full(&args(line)).is_err(), "{line}");
        }
        let line = format!("ui key {}", vec!["cmd-t"; 33].join(" "));
        assert!(parse_full(&args(&line)).is_err());
        assert!(
            matches!(parse_full(&args("ui key cmd-t esc")).unwrap().command, Command::Ui(bbt_proto::gui::UiRequest::Key(keys)) if keys == ["cmd-t", "esc"])
        );
    }

    #[test]
    fn ui_clock_commands_use_the_existing_relay_and_reject_non_integer_advances() {
        use bbt_proto::gui::{UiClockCommand, UiRequest};
        for (line, expected) in [
            ("ui clock freeze", UiClockCommand::Freeze),
            ("ui clock advance 450", UiClockCommand::Advance { ms: 450 }),
            ("ui clock resume", UiClockCommand::Resume),
        ] {
            assert!(
                matches!(parse_full(&args(line)).unwrap().command, Command::Ui(UiRequest::Clock(actual)) if actual == expected)
            );
        }
        for line in [
            "ui clock advance -1",
            "ui clock advance 1.5",
            "ui clock advance 18446744073709551616",
            "ui clock advance",
            "ui clock stop",
        ] {
            assert!(parse_full(&args(line)).is_err(), "{line}");
        }
    }
    #[test]
    fn errors_name_a_kind_a_hint_and_an_exit() {
        let hidden = classify("not_on_screen: seat fixture has no pane on screen");
        assert_eq!(hidden.kind, "not_on_screen");
        assert_eq!(hidden.hint, "ctl seat focus <seat>");
        assert_eq!(
            classify("focus_timeout: no frame completed").kind,
            "focus_timeout"
        );
        assert_eq!(
            (
                classify("unknown: timeout: GUI owner did not answer").kind,
                classify("unknown: timeout: GUI owner did not answer").exit
            ),
            ("timeout", 3)
        );
        assert_eq!(
            classify("ui_refused: endpoint_unavailable: no GUI owner").exit,
            4
        );
        let usage = classify(&parse_full(&args("seat fly s")).unwrap_err());
        assert_eq!((usage.kind, usage.exit), ("usage", 2));
        assert!(usage.message.contains("ls"), "{usage:?}");
        let gone = classify("endpoint /x/gui.endpoint: No such file or directory (os error 2)");
        assert_eq!((gone.kind, gone.exit), ("endpoint_unavailable", 4));
        let seat = classify(bbt_mux::ctl_door::REFUSAL);
        assert_eq!((seat.kind, seat.exit), ("ancestry_refused", 1));
        let dangling = classify("unknown destination seats/x/inbox: send to operator/inbox");
        assert_eq!(dangling.kind, "dangling_destination");
        assert_eq!(dangling.hint, "send to operator/inbox");
        for reason in ["no seat x", "stale close operation", "anything"] {
            let fail = classify(reason);
            assert!(
                !fail.hint.is_empty() && fail.exit != 0,
                "{reason}: {fail:?}"
            );
        }
    }
    #[test]
    fn a_paste_is_bracketed_only_when_asked_and_cannot_end_itself() {
        let on = u64::from(bbt_core::term::TermMode::BRACKETED_PASTE.bits());
        assert_eq!(bracketed(b"ab", 0), b"ab");
        assert_eq!(bracketed(b"a\x1b[201~b", on), b"\x1b[200~ab\x1b[201~");
        assert_eq!(key_bytes("ctrl-c", 0).unwrap(), b"\x03");
        assert_eq!(key_bytes("up", 0).unwrap(), b"\x1b[A");
        assert!(key_bytes("hyper-z", 0).is_err());
    }
    /// The door refuses and closes before the request is written (a seat under load):
    /// the write fails, and the refusal already buffered is still what the caller reads.
    #[test]
    fn a_refusal_that_beats_the_request_is_still_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gui.sock");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let mut client = Client::connect(&Endpoint {
            path: path.clone(),
            capability: [0; 32],
        })
        .unwrap();
        let (mut door, _) = listener.accept().unwrap();
        let refusal = serde_json::to_string(&GuiReply::Error {
            op: Ulid::nil(),
            reason: bbt_mux::ctl_door::REFUSAL.into(),
        })
        .unwrap();
        door.write_all(format!("{refusal}\n").as_bytes()).unwrap();
        drop(door);
        assert_eq!(client.seats().unwrap_err(), bbt_mux::ctl_door::REFUSAL);
    }
    #[test]
    fn commands_parse_in_the_spec_s_shapes() {
        assert_eq!(parse(&args("seats")).unwrap(), (None, Command::Seats));
        let (endpoint, cmd) = parse(&args("--endpoint /tmp/e spawn claude --name Rook")).unwrap();
        assert_eq!(endpoint, Some(PathBuf::from("/tmp/e")));
        assert_eq!(
            cmd,
            Command::Spawn {
                harness: "claude".into(),
                name: Some("Rook".into()),
                cwd: None,
                swarm: None,
                new_tab: false,
                role: None,
            }
        );
        assert_eq!(
            parse(&args("inbox --since 7")).unwrap().1,
            Command::Inbox { since: 7 }
        );
        assert_eq!(
            parse(&args("delegate #12 Rook")).unwrap().1,
            Command::Delegate {
                task: "#12".into(),
                seat: "Rook".into()
            }
        );
        assert!(parse(&args("receipts not-an-id")).is_err());
        assert!(
            parse(&args("seats --bogus 1"))
                .unwrap_err()
                .contains("--bogus")
        );
        assert!(
            parse(&args("frobnicate"))
                .unwrap_err()
                .starts_with("usage:")
        );
    }
    #[test]
    fn the_endpoint_file_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("gui.endpoint");
        let endpoint = Endpoint {
            path: "/tmp/bbt-501-x/gui.sock".into(),
            capability: std::array::from_fn(|i| i as u8),
        };
        std::fs::write(&file, endpoint.to_json()).unwrap();
        let back = Endpoint::read(&file).unwrap();
        assert_eq!(
            (back.path, back.capability),
            (endpoint.path, endpoint.capability)
        );
    }
}

#[cfg(test)]
mod placement_parser_tests {
    use super::*;
    #[test]
    fn explicit_placements_relay_intents_without_choosing_an_anchor_in_the_cli() {
        use bbt_proto::gui::SpawnPlace;
        for (name, expected) in [
            ("right", SpawnPlace::Right),
            ("down", SpawnPlace::Down),
            ("tab", SpawnPlace::Tab),
            ("new-tab", SpawnPlace::Tab),
            ("space", SpawnPlace::Space),
        ] {
            let args = [
                "seat",
                "spawn",
                "zsh",
                "--place",
                name,
                "--cwd",
                "/chosen cwd",
                "--name",
                "chosen",
                "--role",
                "shell",
            ]
            .map(str::to_owned);
            let Command::SpawnAt(pick) = parse_full(&args).unwrap().command else {
                panic!("explicit placement bypassed the owner")
            };
            assert_eq!(pick.place, expected);
            assert_eq!(pick.swarm, None);
            assert_eq!(pick.cwd, "/chosen cwd");
            assert_eq!(pick.name.as_deref(), Some("chosen"));
            assert_eq!(pick.role.as_deref(), Some("shell"));
        }
        let args = ["seat", "spawn", "zsh", "--place", "diagonal"].map(str::to_owned);
        assert!(parse_full(&args).unwrap_err().starts_with(USAGE_PREFIX));
        let args = ["seat", "spawn", "zsh"].map(str::to_owned);
        assert!(matches!(
            parse_full(&args).unwrap().command,
            Command::Spawn { new_tab: false, .. }
        ));
    }
}
