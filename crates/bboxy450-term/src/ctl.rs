// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
mod e2e;
mod events;
mod identity;
mod knowledge;
mod message;
mod plugin;
mod records;
mod seat;
mod task;
mod ui;
mod workspace;
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

/// Shared GUI reply budget; control requests retain their existing bound.
const FRAME_LIMIT: usize = bbt_proto::gui::MAX_REPLY_BYTES;
const TIMEOUT: Duration = Duration::from_secs(15);
const PAGE: usize = 128;

pub const USAGE: &str = "usage: bboxy450-term ctl <noun> <verb> [args] [flags]   (ctl-cli rev. 2)
  ui open <graph|settings|launcher|docs <path>>
  ui paste-image <path>                  copy PNG/TIFF into the focused composer; returns owned path
  ui key <named-key>...                  cmd-t cmd-d cmd-shift-d cmd-g cmd-m cmd-comma esc enter tab arrows
  ui click <x> <y>                       Logical window pixels; use regions in ui layout
  ui drag <x0> <y0> <x1> <y1>            Drag a retained sidebar resize handle
  ui sidebar-scroll <lines>              Wheel input; positive scrolls toward the top
  ui launcher-scroll <lines>             Scroll the launcher; same wheel convention
  ui focus <seat>                        select its pane through the GUI owner
  ui scroll <seat> <top|bottom|page-up|page-down|row N>   zero-based GUI history
  ui chat-wheel <DY> / task-wheel <DY>    section scroll in logical pixels; positive moves down
  ui layout                              solved window tree, pane grids, focus and tab
  ui clock freeze|advance <ms>|resume     test-account animation time, acknowledged by its frame
  ui compose --to <seat|role:r|channel:c> --text <text>   send through the window composer
  seat ls
  seat attention <seat>                  read policy, defaults and pending digest summary
  seat attention <seat> <direct|destination> <conversation|event> <wake|inbox|digest|default>
  seat spawn <claude|codex|zsh> [--place right|down|tab|space] [--role oracle|architect|creativity|executor|shell]
             [--name NAME] [--cwd DIR] [--swarm ID]
  seat screen <seat> [--human]            the visible screen, cursor and title
  seat type <seat> <text>                 types the text as written
  seat key <seat> <key>...                enter esc tab backspace space up down left right home end
                                          pageup pagedown delete shift-tab ctrl-<letter>; Kitty when on
  seat paste <seat> <file|->              bracketed when the program asked
  seat wait <seat> (--state idle|active|waiting|exited | --text REGEX | --on-screen | --frame-seq N)
                                          [--subagent ID] [--grade reported] [--timeout MS]   default 30000; --on-screen: the seat has a
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
  plugin list [--swarm ID]                 workspace registry; never executes untrusted files
  plugin trust <path>                      pin and inspect a discovered binary; stays disabled
  plugin enable <candidate-id>
  plugin disable <candidate-id>
  plugin check <candidate-id> [--swarm ID]  disposable conformance check
  knowledge status [--swarm ID]
  knowledge select <root-id> <candidate-id> [--swarm ID]
  knowledge reset <root-id> [--swarm ID]    select the native local KB
  task list [--unassigned | --seat SEAT | --status STATUS]
  task get <#N|path|id>
  task status <#N|path|id>
  task report <#N|path|id> --summary TEXT [--commit SHA] [--verify-result JSON]
                                          JSON: command, exit_code, optional evidence entries
  task accept <#N|path|id>
  task reject <#N|path|id> --reason TEXT
  task reconcile <#N|path|id> --disposition implemented|superseded|cancelled --reason TEXT --evidence JSON
                                          historical done, never fresh acceptance; no wakes
  task import <manifest.json> --audit FILE --source-dir DIR --expected-root ROOT --owner-seat ID
                                          bounded pinned source, atomic publication; no wakes
  task import-context                    exact selected root for the import; no cwd inference
  task delegate <#N|path|id> <seat>
  task create <title> --verify CMD [--brief TEXT]
                                          an open, unassigned task, signed by the operator
  e2e list [--group GROUP]
  record fetch <#N|slug|path>              frontmatter, Markdown and confined resource links
  record search <query> [--type TYPE] [--cursor CURSOR]
  record changes --since CURSOR           changes and the cursor for the next read
  record commit <slug|path>... --message TEXT [--swarm ID]
  events streams                          retained stream names, no consumer attached
  events tail --stream NAME [--since SEQ] [--subject GLOB] [--timeout MS]
  debug logs <app|logic|supervisor> [--since BYTE_OFFSET]   bounded page, next_since
  debug errors [--since UNIX_MS] [--seat ID] failures; unfiltered reads mark retained errors read
  debug report                            offline hints for evidence gathering and gh issue reporting
  debug journal <account/OP|seat/SEAT/OP>  one retained operation; no SQL or key namespace
  debug metrics                           logic + GUI history heap/page counters
  debug history-admission COLSxROWS       test-account bounded core viewport/heap fixture
  debug fault ring-gap <seat> <from>..<to>  test-account retained-history gap mask
  debug fault spawn-hold                  test-account: hold the next admitted launch
  debug fault spawn-release <op>          release that exact launch once
  debug snapshot <file.json>              bounded R5 cursor/capture table; no tracing enabled
  debug ps                                this app's process tree; no argv or environment
  debug notifications [native]            test recording, or read-only local OS alert settings
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

/// Splices each noun's `Command` list (decision #320) into the flat enum and generates
/// `Command::noun`, the one exhaustive match a noun's handlers dispatch from. Each list is a
/// `macro_rules!` that takes this macro's name, the lists still to run and the variants so far,
/// appends `"noun": [variants]` and calls back; with no lists left the enum is emitted.
macro_rules! commands {
    ($(#[$meta:meta])* pub enum $name:ident from [$($list:ident),* $(,)?]) => {
        commands!(@next [$($list)*] [] $(#[$meta])* pub enum $name);
    };
    (@next [] [$($noun:literal: [$(
        $(#[$vmeta:meta])* $variant:ident $(($($tuple:tt)*))? $({$($field:tt)*})?
    ),* $(,)?])*] $(#[$meta:meta])* pub enum $name:ident) => {
        $(#[$meta])*
        pub enum $name {
            $($($(#[$vmeta])* $variant $(($($tuple)*))? $({$($field)*})?,)*)*
        }
        impl $name {
            /// The `ctl` noun whose module owns this command.
            pub fn noun(&self) -> &'static str {
                match self {
                    $($(Self::$variant { .. } => $noun,)*)*
                }
            }
        }
    };
    (@next [$list:ident $($rest:ident)*] [$($variants:tt)*] $($head:tt)*) => {
        $list!(commands, [$($rest)*] [$($variants)*] $($head)*);
    };
}

use app::app_commands;
use debug::debug_commands;
use e2e::e2e_commands;
use events::events_commands;
use identity::identity_commands;
use knowledge::knowledge_commands;
use message::message_commands;
use plugin::plugin_commands;
use records::record_commands;
use seat::seat_commands;
use task::task_commands;
use ui::ui_commands;

commands! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum Command from [
        ui_commands,
        identity_commands,
        seat_commands,
        message_commands,
        plugin_commands,
        task_commands,
        e2e_commands,
        record_commands,
        knowledge_commands,
        events_commands,
        debug_commands,
        app_commands,
    ]
}

/// One `ctl` verb: noun, verb, and the window action it issues (`-` for none).
pub type Verb = (&'static str, &'static str, &'static str);
/// Each noun's own table (decision #320): a verb is added in its noun's file only.
const NOUN_VERBS: &[&[Verb]] = &[
    ui::VERBS,
    identity::VERBS,
    seat::VERBS,
    message::VERBS,
    plugin::VERBS,
    task::VERBS,
    e2e::VERBS,
    records::VERBS,
    knowledge::VERBS,
    events::VERBS,
    debug::VERBS,
    app::VERBS,
];
const fn verb_count(parts: &[&[Verb]]) -> usize {
    let (mut n, mut i) = (0, 0);
    while i < parts.len() {
        n += parts[i].len();
        i += 1;
    }
    n
}
const ALL_VERBS: [Verb; verb_count(NOUN_VERBS)] = {
    let mut all = [("", "", ""); verb_count(NOUN_VERBS)];
    let (mut k, mut i) = (0, 0);
    while i < NOUN_VERBS.len() {
        let mut j = 0;
        while j < NOUN_VERBS[i].len() {
            all[k] = NOUN_VERBS[i][j];
            (k, j) = (k + 1, j + 1);
        }
        i += 1;
    }
    all
};
/// Every noun-verb `ctl` verb and the one window action it issues (ctl-cli §1). The parser
/// accepts exactly these; `app endpoint` reads the run directory and issues none.
pub const VERBS: &[(&str, &str, &str)] = &ALL_VERBS;
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

/// What a noun's `parse` may read of the command line besides its words.
pub(super) struct ParseCx<'a> {
    pub flags: &'a std::collections::BTreeMap<String, String>,
    pub swarm: Option<Ulid>,
    pub human_flag: bool,
    pub unassigned_flag: bool,
    pub on_screen_flag: bool,
    pub cells_flag: bool,
    pub headless: bool,
    pub ctl: bool,
    pub unattended: bool,
}
type NounParse = fn(&[&str], &ParseCx) -> Result<Option<Command>, String>;
/// Each noun's parser (decision #320), asked in turn; a verb is parsed in its noun's file.
const NOUN_PARSERS: &[NounParse] = &[
    ui::parse,
    debug::parse,
    seat::parse,
    message::parse,
    plugin::parse,
    task::parse,
    e2e::parse,
    records::parse,
    knowledge::parse,
    events::parse,
    identity::parse,
    app::parse,
];

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
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let cx = ParseCx {
        flags: &flags,
        swarm,
        human_flag,
        unassigned_flag,
        on_screen_flag,
        cells_flag,
        headless,
        ctl,
        unattended,
    };
    let parsed = NOUN_PARSERS
        .iter()
        .find_map(|parse| parse(&words, &cx).transpose());
    let command = match parsed {
        Some(command) => command?,
        None => {
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
        Command::UiCompose { .. } => Some(&["to", "text"]),
        Command::RecordFetch { .. } | Command::EventsStreams => Some(&[]),
        Command::RecordSearch { .. } => Some(&["type", "cursor"]),
        Command::RecordChanges { .. } => Some(&["since"]),
        Command::EventsTail { .. } => Some(&["stream", "since", "subject", "timeout"]),
        Command::Debug {
            request: bbt_proto::gui::DebugRequest::Errors { .. },
            ..
        } => Some(&["since", "seat"]),
        Command::Debug {
            request: bbt_proto::gui::DebugRequest::Logs { .. },
            ..
        } => Some(&["since"]),
        Command::Debug { .. } | Command::DebugFault { .. } | Command::DebugSpawnGate(_) => {
            Some(&[])
        }
        Command::DebugReport
        | Command::DebugNotifications
        | Command::DebugNativeNotifications
        | Command::UiNotificationClick(_) => Some(&[]),
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
    if flags.contains_key("reason")
        && !matches!(
            &command,
            Command::TaskReject { .. } | Command::TaskReconcile { .. }
        )
    {
        return Err(format!(
            "{USAGE_PREFIX}--reason applies only to task reject"
        ));
    }
    if flags
        .keys()
        .any(|key| matches!(key.as_str(), "disposition" | "evidence"))
        && !matches!(&command, Command::TaskReconcile { .. })
    {
        return Err(format!(
            "{USAGE_PREFIX}--disposition and --evidence apply only to task reconcile"
        ));
    }
    if flags.keys().any(|key| {
        matches!(
            key.as_str(),
            "audit" | "source-dir" | "expected-root" | "owner-seat"
        )
    }) && !matches!(&command, Command::TaskImport { .. })
    {
        return Err(format!(
            "{USAGE_PREFIX}import flags apply only to task import"
        ));
    }
    if flags.contains_key("to") && !matches!(&command, Command::UiCompose { .. }) {
        return Err(format!("{USAGE_PREFIX}--to applies only to ui compose"));
    }
    if flags.contains_key("message") && !matches!(&command, Command::RecordCommit { .. }) {
        return Err(format!(
            "{USAGE_PREFIX}--message applies only to record commit"
        ));
    }
    if (flags.contains_key("subagent") || flags.contains_key("grade"))
        && !matches!(&command, Command::Wait { .. })
    {
        return Err(format!(
            "{USAGE_PREFIX}--subagent and --grade apply only to seat wait"
        ));
    }
    if let Some(unknown) = flags.keys().find(|k| {
        ![
            "to",
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
            "subagent",
            "grade",
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
            "message",
            "verify-result",
            "reason",
            "disposition",
            "evidence",
            "audit",
            "source-dir",
            "expected-root",
            "owner-seat",
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
            return Err("gui.sock frame over the GUI reply limit".into());
        }
        serde_json::from_slice(&line).map_err(|e| format!("gui.sock reply: {e}"))
    }
    /// Sends `action` and returns the first reply `answer` accepts; an `Error` for this op
    /// is returned as `Err` with logic's reason.
    fn call(
        &mut self,
        op: Option<Ulid>,
        action: GuiAction,
        answer: impl FnMut(&GuiReply) -> bool,
    ) -> Result<GuiReply, String> {
        self.call_with_deadline(op, action, answer, None)
    }
    fn call_with_deadline(
        &mut self,
        op: Option<Ulid>,
        action: GuiAction,
        mut answer: impl FnMut(&GuiReply) -> bool,
        deadline: Option<std::time::Instant>,
    ) -> Result<GuiReply, String> {
        let sent = match deadline {
            Some(deadline) => workspace::send(self, action, deadline),
            None => self.send(action),
        };
        if let Err(write) = sent {
            // The door can refuse and close before the request is written; its reason is
            // already in our buffer, so a seat reads the refusal, not a broken pipe.
            let reply = match deadline {
                Some(deadline) => workspace::reply(self, deadline),
                None => self.reply(),
            };
            return match reply {
                Ok(GuiReply::Error { op: failed, reason })
                    if deadline.is_none() || op == Some(failed) || failed.is_nil() =>
                {
                    if deadline.is_some() && reason.starts_with("workspace_pending:") {
                        Err(format!(
                            "unknown: {write}; a buffered pending response does not confirm a failed write"
                        ))
                    } else {
                        Err(reason)
                    }
                }
                _ => Err(write),
            };
        }
        loop {
            let reply = match deadline {
                Some(deadline) => workspace::reply(self, deadline)?,
                None => self.reply()?,
            };
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
            .call_workspace(
                id,
                GuiAction::Knowledge {
                    scope: None,
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
        let history = matches!(request, bbt_proto::gui::DebugRequest::Metrics);
        let id = self.mint();
        let mut data = match self.call(
            Some(id),
            GuiAction::Debug { id, request },
            |reply| matches!(reply,GuiReply::Debug {op,..} if *op==id),
        )? {
            GuiReply::Debug { data, .. } => data,
            _ => unreachable!(),
        };
        if history {
            data["ui"] = match self.app(bbt_proto::gui::AppRequest::HistoryMetrics) {
                Ok(status) => json!({"available":!status.history.is_null(),
                    "frame_seq":status.frame_seq,"history":status.history,
                    "basis":"IO sample; frame_seq names the independently produced window frame"}),
                Err(reason) => json!({"available":false,"reason":reason}),
            };
            if serde_json::to_vec(&data).map_err(|e| e.to_string())?.len() > 256 * 1024 {
                return Err("debug_metrics_exceeds_bound: narrow history window".into());
            }
        }
        Ok(data)
    }
    pub fn knowledge_commit(
        &mut self,
        locators: Vec<String>,
        message: String,
        swarm: Option<Ulid>,
    ) -> Result<Value, String> {
        let scope = self.workspace_scope(swarm)?;
        let id = self.mint();
        match self.call_workspace(
            id,
            GuiAction::KnowledgeCommit {
                id,
                scope,
                locators,
                message,
            },
            |r| matches!(r,GuiReply::Knowledge{op,..} if *op==id),
        )? {
            GuiReply::Knowledge { response, .. } => {
                serde_json::from_value::<bbt_mux::knowledge::data_source::Result<Value>>(response)
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())
            }
            _ => unreachable!(),
        }
    }
    /// The operator's signed `e2e` record write: the data_source write result.
    pub fn record_write(&mut self, args: Value) -> Result<Value, String> {
        let id = self.mint();
        match self.call_workspace(
            id,
            GuiAction::RecordWrite {
                scope: None,
                id,
                args,
            },
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
        self.events_page_wait(stream, after, limit, None)
    }
    fn events_page_wait(
        &mut self,
        stream: &str,
        after: u64,
        limit: usize,
        wait_ms: Option<u64>,
    ) -> Result<(Vec<bbt_proto::gui::RetainedEvent>, Option<u64>, u64), String> {
        let id = self.mint();
        match self.call(
            Some(id),
            GuiAction::EventsRead {
                id,
                stream: stream.into(),
                after,
                limit,
                wait_ms,
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
        match self.call_workspace(
            id,
            GuiAction::Task {
                scope: None,
                id,
                request,
            },
            |r| matches!(r, GuiReply::Task { op, .. } if *op == id),
        )? {
            GuiReply::Task { response, .. } => Ok(response),
            _ => unreachable!(),
        }
    }
}

/// The state vocabulary stays in ctl; the bus only waits for a page append.
struct WaitPredicate {
    state: Option<String>,
    pattern: Option<regex::Regex>,
    subagent: Option<String>,
    grade: Option<String>,
}
impl WaitPredicate {
    fn new(
        state: Option<String>,
        text: Option<String>,
        subagent: Option<String>,
        grade: Option<String>,
    ) -> Result<Self, String> {
        if let Some(state) = state.as_deref()
            && !matches!(state, "idle" | "active" | "waiting" | "exited")
        {
            return Err(format!(
                "{USAGE_PREFIX}seat wait --state: idle, active, waiting or exited, not {state}"
            ));
        }
        if subagent.as_deref().is_some_and(str::is_empty)
            || (subagent.is_some() && state.as_deref() == Some("exited"))
        {
            return Err(format!(
                "{USAGE_PREFIX}--subagent requires a nonempty id and an activity state"
            ));
        }
        let pattern = text
            .map(|pattern| {
                regex::Regex::new(&pattern)
                    .map_err(|error| format!("{USAGE_PREFIX}seat wait --text: {error}"))
            })
            .transpose()?;
        Ok(Self {
            state,
            pattern,
            subagent,
            grade,
        })
    }
    fn matches(&self, seat: &GuiSeat, screen: Option<&bbt_proto::gui::SeatScreen>) -> bool {
        if let Some(pattern) = &self.pattern {
            return screen.is_some_and(|screen| pattern.is_match(&screen.rows.join("\n")));
        }
        use bbt_proto::seat_state::{Activity, Grade, ProcessStatus};
        let (activity, grade) = if let Some(id) = &self.subagent {
            let Some(row) = seat.state.subagents.iter().find(|row| row.id == *id) else {
                return false;
            };
            (row.activity, row.grade)
        } else {
            (seat.state.activity, seat.state.grade)
        };
        let met = match self.state.as_deref() {
            Some("exited") => matches!(seat.state.process, ProcessStatus::Exited { .. }),
            Some("idle") => activity == Activity::Idle,
            Some("active") => activity == Activity::Active,
            Some("waiting") => matches!(activity, Activity::Waiting(_)),
            _ => false,
        };
        met && (self.grade.is_none() || grade == Grade::Reported)
    }
}

fn wait_for_seat(
    client: &mut Client,
    selector: &str,
    predicate: &WaitPredicate,
    timeout: Duration,
    elapsed: impl Fn() -> Duration,
) -> Result<Vec<Value>, String> {
    let op = fresh();
    let screen_tail = std::cell::RefCell::new(None::<String>);
    let timed_out = || {
        if let Some(tail) = screen_tail.borrow().as_deref() {
            format!(
                "{TIMEOUT_PREFIX}{op}\nwaited {} ms; the screen ends:\n{tail}",
                timeout.as_millis()
            )
        } else {
            format!(
                "{TIMEOUT_PREFIX}{op}\nwaited {} ms; seat {selector} did not meet the requested predicate",
                timeout.as_millis()
            )
        }
    };
    let remaining = || {
        timeout
            .checked_sub(elapsed())
            .filter(|left| !left.is_zero())
            .ok_or_else(timed_out)
    };
    let set_budget = |client: &Client| -> Result<Duration, String> {
        let left = remaining()?;
        client
            .reader
            .get_ref()
            .set_read_timeout(Some(left))
            .map_err(|error| error.to_string())?;
        client
            .writer
            .set_write_timeout(Some(left))
            .map_err(|error| error.to_string())?;
        Ok(left)
    };
    let result = (|| {
        let mut after = 0;
        loop {
            set_budget(client)?;
            let failure = |error| {
                if elapsed() >= timeout {
                    timed_out()
                } else {
                    error
                }
            };
            // Recheck the current snapshot after every waiting page. Old retained
            // states must never satisfy a predicate that is false now.
            let seats = client.seats().map_err(failure)?;
            let seat = resolve(&seats, selector)?;
            let screen = if predicate.pattern.is_some() {
                set_budget(client)?;
                Some(client.screen(seat.record.id).map_err(failure)?.1)
            } else {
                None
            };
            if let Some(screen) = &screen {
                let rows: Vec<_> = screen
                    .rows
                    .iter()
                    .filter(|row| !row.is_empty())
                    .map(String::as_str)
                    .collect();
                *screen_tail.borrow_mut() = Some(rows[rows.len().saturating_sub(12)..].join("\n"));
            }
            remaining()?;
            if predicate.matches(seat, screen.as_ref()) {
                let mut row = json!({"ok":true,"op":op,"done":true,
                    "data":{"seat":seat.record.id,"state":predicate.state.clone(),"text":predicate.pattern.as_ref().map(|pattern|pattern.as_str())}});
                if let Some(id) = &predicate.subagent {
                    row["data"]["subagent"] = json!(id);
                }
                if let Some(grade) = &predicate.grade {
                    row["data"]["grade"] = json!(grade);
                }
                return Ok(vec![row]);
            }
            let left = set_budget(client)?;
            // No frame notice is guaranteed in the state stream. Text waits use
            // a named 200ms screen fallback by bounded waiting reads, not sleeps.
            let cap = if predicate.pattern.is_some() {
                200
            } else {
                60_000
            };
            let wait_ms = u64::try_from(left.as_millis().clamp(1, cap)).unwrap();
            let (_, _, through) = client
                .events_page_wait(
                    &format!("seat-{}", seat.record.id),
                    after,
                    PAGE,
                    Some(wait_ms),
                )
                .map_err(failure)?;
            if through < after {
                return Err("EventsRead cursor moved backwards".into());
            }
            after = through;
        }
    })();
    let _ = client.reader.get_ref().set_read_timeout(Some(TIMEOUT));
    let _ = client.writer.set_write_timeout(Some(TIMEOUT));
    result
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
/// Runs a flat (pre-grammar) command through its noun's `run` (decision #320); a noun with
/// no flat forms refuses, as each noun's own default does.
pub fn run(client: &mut Client, command: Command) -> Result<Vec<Value>, String> {
    match NOUN_RUNNERS
        .iter()
        .find(|(noun, _)| *noun == command.noun())
    {
        Some((_, run)) => run(client, command),
        None => Err(no_flat_form()),
    }
}
type NounRun = fn(&mut Client, Command) -> Result<Vec<Value>, String>;
/// Each noun with flat commands and its runner.
const NOUN_RUNNERS: &[(&str, NounRun)] = &[
    ("identity", identity::run),
    ("seat", seat::run),
    ("message", message::run),
    ("task", task::run),
    ("e2e", e2e::run),
    ("record", records::run),
];
/// The one refusal for a noun-verb-only command given in flat form.
fn no_flat_form() -> String {
    format!("{USAGE_PREFIX}this verb has no flat form; use the noun-verb grammar")
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
    if matches!(
        reason,
        "role_recipient_limit_32" | "ui_refused: role_recipient_limit_32"
    ) {
        return fail(
            "role_recipient_limit_32",
            "choose a role with at most 32 open seats in the current swarm",
            1,
        );
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

/// The CLI preserves a wait timeout's operation id before shaping its error.
fn operation_failure(op: Option<Ulid>, reason: &str) -> (Option<Ulid>, Fail) {
    match reason.strip_prefix(TIMEOUT_PREFIX) {
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
        None => (op, classify(reason)),
    }
}

/// Preserve stdout's error kind before noun runners turn I/O errors into strings.
/// A closed reader ends output; errors from backend sockets remain failures.
struct CliOutput {
    inner: std::io::StdoutLock<'static>,
    closed: bool,
}

impl CliOutput {
    fn new() -> Self {
        Self {
            inner: std::io::stdout().lock(),
            closed: false,
        }
    }

    fn observe<T>(&mut self, result: std::io::Result<T>) -> std::io::Result<T> {
        if let Err(error) = &result
            && error.kind() == std::io::ErrorKind::BrokenPipe
        {
            self.closed = true;
        }
        result
    }

    fn finish(&mut self, result: Result<(), String>) -> Result<(), String> {
        let result = result.and_then(|()| self.flush().map_err(|error| error.to_string()));
        if self.closed { Ok(()) } else { result }
    }
}

impl Write for CliOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let result = self.inner.write(bytes);
        self.observe(result)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let result = self.inner.flush();
        self.observe(result)
    }
}

fn with_stdout(run: impl FnOnce(&mut CliOutput) -> Result<(), String>) -> Result<(), String> {
    let mut out = CliOutput::new();
    let result = run(&mut out);
    out.finish(result)
}

/// Whether the line reached stdout. A closed reader suppresses semantic error exits.
fn print_line(line: std::fmt::Arguments<'_>) -> Result<bool, String> {
    let mut out = CliOutput::new();
    let result = writeln!(out, "{line}").map_err(|error| error.to_string());
    out.finish(result)?;
    Ok(!out.closed)
}

/// Preserve the selected diagnostics destination even when another CLI argument
/// fails parsing. Consume flag values so an issuer/value named `--run-dir` is
/// not mistaken for an actual routing selector.
fn diagnostic_endpoint(args: &[String]) -> Option<PathBuf> {
    let mut endpoint = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if !arg.starts_with("--")
            || matches!(
                arg.as_str(),
                "--json"
                    | "--human"
                    | "--unassigned"
                    | "--on-screen"
                    | "--cells"
                    | "--headless"
                    | "--ctl"
                    | "--unattended"
            )
        {
            continue;
        }
        let Some(value) = args.next() else { break };
        match arg.as_str() {
            "--endpoint" => endpoint = Some(PathBuf::from(value)),
            "--run-dir" => endpoint = Some(PathBuf::from(value).join(bbt_mux::ctl_door::ENDPOINT)),
            _ => {}
        }
    }
    endpoint
}

pub fn main(args: &[String], default_endpoint: &Path) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return print_line(format_args!("{USAGE}")).map(|_| ());
    }
    let parsed = parse_full(args);
    let diagnostic_endpoint = diagnostic_endpoint(args);
    let failure_root = parsed
        .as_ref()
        .ok()
        .and_then(|p| p.endpoint.as_deref())
        .or(diagnostic_endpoint.as_deref())
        .unwrap_or(default_endpoint)
        .parent()
        .map(Path::to_path_buf);
    let failure_operation = parsed
        .as_ref()
        .ok()
        .and_then(|p| p.verb)
        .map_or_else(|| "command".into(), |(n, v)| format!("{n}/{v}"));
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
        let result = run_main(args, default_endpoint);
        if let Err(reason) = &result
            && let Some(root) = &failure_root
        {
            bbt_rt::error_journal::report_to(
                root,
                bbt_rt::error_journal::Failure::new("ctl", &failure_operation, reason),
            );
        }
        return match result {
            Err(reason) if reason.starts_with(TIMEOUT_PREFIX) => {
                let (op, message) = reason[TIMEOUT_PREFIX.len()..]
                    .split_once('\n')
                    .unwrap_or(("", &reason));
                if print_line(format_args!(
                    "{}",
                    json!({ "ok": false, "op": op, "error": { "kind": "timeout", "message": message,
                        "hint": "raise --timeout, or wait for another --state or --text" } })
                ))? {
                    std::process::exit(3);
                }
                Ok(())
            }
            Err(reason) => {
                if print_line(format_args!(
                    "{}",
                    json!({"kind": "error", "reason": reason})
                ))? {
                    std::process::exit(1);
                }
                Ok(())
            }
            Ok(()) => Ok(()),
        };
    }
    let mut op = None;
    let mut tail = None;
    let mut capture_seat = None;
    let mut message_refusal = None;
    let result = parsed.and_then(|parsed| {
        op = parsed.op;
        if let Command::EventsTail { stream, since, .. } = &parsed.command {
            tail = Some(events::Progress::new(stream.clone(), *since));
        }
        run_verb(
            args,
            parsed,
            default_endpoint,
            &mut op,
            &mut tail,
            &mut capture_seat,
            &mut message_refusal,
        )
    });
    if let Err(reason) = result {
        if let Some(root) = &failure_root {
            bbt_rt::error_journal::report_to(
                root,
                bbt_rt::error_journal::Failure::new("ctl", &failure_operation, &reason),
            );
        }
        let (op, fail) = operation_failure(op, &reason);
        let mut line = json!({ "ok": false, "op": op.unwrap_or_else(fresh), "error": {
            "kind": fail.kind, "message": fail.message, "hint": fail.hint } });
        if fail.kind == "not_on_screen"
            && let Some(seat) = capture_seat
        {
            line["error"]["seat"] = json!(seat);
        }
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
        let exit = if message_refusal.is_some() {
            1
        } else {
            fail.exit
        };
        if let Some(error) = message_refusal {
            line["error"] = json!(error);
        }
        if print_line(format_args!("{line}"))? {
            std::process::exit(exit);
        }
    }
    Ok(())
}

/// One noun-verb verb: connect (unless it only reads the run directory), issue its action,
/// print its `data` in the §2 envelope, or `--human` as text.
/// What a noun's `run_verb` may read besides its command (decision #320).
struct VerbCx<'a> {
    client: Client,
    id: Ulid,
    human: bool,
    endpoint_file: PathBuf,
    tail: &'a mut Option<events::Progress>,
    capture_seat: &'a mut Option<Ulid>,
    message_refusal: &'a mut Option<bbt_proto::messaging::MessageRefusal>,
    out: &'a mut CliOutput,
    noun: &'static str,
    verb: &'static str,
}
type NounVerbRun = fn(VerbCx<'_>, Command) -> Result<(), String>;
/// Each noun's noun-verb runner; a verb is run in its noun's file.
const NOUN_VERB_RUNNERS: &[(&str, NounVerbRun)] = &[
    ("debug", debug::run_verb),
    ("plugin", plugin::run_verb),
    ("events", events::run_verb),
    ("record", records::run_verb),
    ("knowledge", knowledge::run_verb),
    ("ui", ui::run_verb),
    ("seat", seat::run_verb),
    ("app", app::run_verb),
    ("message", message::run_verb),
    ("task", task::run_verb),
];
/// Spawn, post, inbox, receipts, delegate, e2e list: a noun-verb command whose action the
/// flat form already runs answers with that reply (each runner's default arm).
pub(super) fn flat_reply(
    client: &mut Client,
    command: Command,
    verb: &str,
) -> Result<Value, String> {
    Ok({
        // Spawn, post, inbox, receipts, delegate, e2e list: the action's reply
        // as the flat form already carries it, minus the flat form's own `kind`.
        let mut rows: Vec<Value> = run(client, command)?
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
    })
}
/// The one reply line of a noun-verb command (ctl-cli §2).
pub(super) fn print_reply(
    out: &mut impl Write,
    noun: &str,
    verb: &str,
    op: Ulid,
    data: Value,
    human: bool,
) -> Result<(), String> {
    let line = if human {
        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?
    } else {
        json!({ "ok": true, "op": op, "noun": noun, "verb": verb, "data": data }).to_string()
    };
    writeln!(out, "{line}").map_err(|e| e.to_string())
}

fn run_verb(
    args: &[String],
    parsed: Parsed,
    default_endpoint: &Path,
    op: &mut Option<Ulid>,
    tail: &mut Option<events::Progress>,
    capture_seat: &mut Option<Ulid>,
    message_refusal: &mut Option<bbt_proto::messaging::MessageRefusal>,
) -> Result<(), String> {
    with_stdout(|out| {
        let (noun, verb) = parsed.verb.expect("a noun-verb form");
        if matches!(&parsed.command, Command::DebugReport) {
            return print_reply(
                out,
                noun,
                verb,
                parsed.op.unwrap_or_else(fresh),
                debug::report(),
                parsed.human,
            );
        }
        let endpoint_file = parsed
            .endpoint
            .clone()
            .unwrap_or_else(|| default_endpoint.to_path_buf());
        // File-only diagnostics remain readable after a backend exit.
        if matches!(parsed.command, Command::DebugNativeNotifications) {
            // Explicit local settings read: no GUI admission, permission request,
            // notification submission or dependency on a running frontend.
            return print_reply(
                out,
                noun,
                verb,
                op.unwrap_or_else(fresh),
                debug::native_settings()?,
                parsed.human,
            );
        }
        if let Command::Debug {
            request: bbt_proto::gui::DebugRequest::Errors { since, seat },
            ..
        } = &parsed.command
        {
            let root = endpoint_file
                .parent()
                .ok_or("diagnostics run directory unavailable")?;
            let data = bbt_rt::error_journal::read(
                root,
                *since,
                seat.as_ref().map(ToString::to_string).as_deref(),
                true,
            )
            .map_err(|e| e.to_string())?;
            return print_reply(
                out,
                noun,
                verb,
                op.unwrap_or_else(fresh),
                data,
                parsed.human,
            );
        }
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
            return print_reply(
                out,
                noun,
                verb,
                op.unwrap_or_else(fresh),
                data,
                parsed.human,
            );
        }
        if let Command::AppEndpoint = parsed.command {
            // The run directory and the generation, never the capability (#275).
            let text = std::fs::read_to_string(&endpoint_file)
                .map_err(|e| format!("endpoint {}: {e}", endpoint_file.display()))?;
            let value: Value = serde_json::from_str(&text).map_err(|e| format!("endpoint: {e}"))?;
            let id = op.unwrap_or_else(fresh);
            return print_reply(
                out,
                noun,
                verb,
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
        let cx = VerbCx {
            client,
            id,
            human,
            endpoint_file,
            tail,
            capture_seat,
            message_refusal,
            out,
            noun,
            verb,
        };
        let noun_of = parsed.command.noun();
        match NOUN_VERB_RUNNERS.iter().find(|(noun, _)| *noun == noun_of) {
            Some((_, run)) => run(cx, parsed.command),
            None => unreachable!("every noun with table verbs has a runner"),
        }
    })
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
    with_stdout(|out| {
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
        for line in run(&mut client, command)? {
            // `ctl screen` without --json prints the screen's own text, not a JSON string.
            match line {
                Value::String(text) => writeln!(out, "{text}"),
                line => writeln!(out, "{line}"),
            }
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
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
                wait_ms: None,
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
                scope: None,
                id,
                request: TaskRequest::Status {
                    task_path: None,
                    task_id: Some(id),
                },
            },
            GuiAction::Knowledge {
                scope: None,
                id,
                method: "list".into(),
                args: json!({}),
            },
            GuiAction::Plugin {
                scope: None,
                id,
                request: bbt_proto::gui::PluginRequest::List,
            },
            GuiAction::KnowledgeControl {
                scope: None,
                id,
                request: bbt_proto::gui::KnowledgeRequest::Status,
            },
            GuiAction::RecordWrite {
                scope: None,
                id,
                args: json!({}),
            },
            GuiAction::KnowledgeCommit {
                scope: None,
                id,
                locators: vec!["notes/example.md".into()],
                message: "explicit documents".into(),
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
        for (line, expected) in [
            (
                "ui select 10 20 30 40",
                bbt_proto::gui::UiRequest::TerminalSelect {
                    from: [10, 20],
                    to: [30, 40],
                    shift: false,
                },
            ),
            (
                "ui shift-select 10 20 30 40",
                bbt_proto::gui::UiRequest::TerminalSelect {
                    from: [10, 20],
                    to: [30, 40],
                    shift: true,
                },
            ),
            ("ui copy", bbt_proto::gui::UiRequest::Copy),
            (
                "ui clipboard-read",
                bbt_proto::gui::UiRequest::ClipboardRead,
            ),
        ] {
            let parsed = parse_full(&args(line)).unwrap();
            assert!(
                matches!(parsed.command, Command::Ui(actual) if actual == expected),
                "{line}"
            );
        }
        let parse_samples: Vec<&str> = [
            ui::PARSE_SAMPLES,
            identity::PARSE_SAMPLES,
            debug::PARSE_SAMPLES,
            seat::PARSE_SAMPLES,
            message::PARSE_SAMPLES,
            plugin::PARSE_SAMPLES,
            task::PARSE_SAMPLES,
            e2e::PARSE_SAMPLES,
            records::PARSE_SAMPLES,
            knowledge::PARSE_SAMPLES,
            events::PARSE_SAMPLES,
            app::PARSE_SAMPLES,
        ]
        .concat();
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
            // #320: the command a verb parses to is listed by that verb's own noun.
            assert_eq!(parsed.command.noun(), noun, "{line}");
        }
    }
    #[test]
    fn document_commit_parses_explicit_paths_message_and_swarm_without_widening_other_verbs() {
        let swarm = Ulid::new();
        let parsed = parse_full(
            &[
                "record",
                "commit",
                "notes/one.md",
                "tasks/two.md",
                "--message",
                "selected documents",
                "--swarm",
                &swarm.to_string(),
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert!(
            matches!(parsed.command, Command::RecordCommit { locators, message, swarm: Some(id) } if locators == ["notes/one.md", "tasks/two.md"] && message == "selected documents" && id == swarm)
        );
        assert!(parse_full(&args("record commit notes/one.md")).is_err());
        assert!(parse_full(&args("record fetch notes/one.md --message ignored")).is_err());
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
    fn task_migration_verbs_require_pinned_root_inputs_and_explicit_history() {
        let parsed=parse_full(&args("task reconcile #171 --disposition superseded --reason obsolete --evidence [\"audit:173\"]")).unwrap();
        assert!(
            matches!(parsed.command,Command::TaskReconcile{ref task,ref evidence,..} if task=="#171" && evidence==&["audit:173"])
        );
        let parsed=parse_full(&args("task import /tmp/manifest.json --audit /tmp/audit.json --source-dir /tmp/source --expected-root /tmp/project/.bboxy --owner-seat 01M630JKTP20TAX8FN05X4GMEX")).unwrap();
        assert!(
            matches!(parsed.command,Command::TaskImport{ref expected_root,..} if expected_root=="/tmp/project/.bboxy")
        );
        for line in [
            "task import /tmp/m",
            "task reconcile #171 --reason x",
            "task accept #171 --disposition implemented",
            "task accept #171 --expected-root /tmp/project/.bboxy",
            "task reconcile #171 --disposition implemented --reason x --evidence nope",
        ] {
            assert!(parse_full(&args(line)).is_err(), "{line}");
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
        for reason in [
            "role_recipient_limit_32",
            "ui_refused: role_recipient_limit_32",
        ] {
            let refusal = classify(reason);
            assert_eq!((refusal.kind, refusal.exit), ("role_recipient_limit_32", 1));
            assert!(refusal.hint.contains("32"));
        }
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
mod noun_verb_tests {
    use super::*;

    /// Decision #320's gate for verbs: every row sits in its own noun's table, and each noun
    /// and each noun-verb pair is listed exactly once.
    #[test]
    fn each_verb_is_listed_once_in_its_own_nouns_table() {
        let mut nouns = std::collections::BTreeSet::new();
        let mut pairs = std::collections::BTreeSet::new();
        for table in NOUN_VERBS {
            let noun = table[0].0;
            assert!(nouns.insert(noun), "noun {noun} has two tables");
            for (n, verb, _) in *table {
                assert_eq!(*n, noun, "{n} {verb} is listed in the {noun} table");
                assert!(pairs.insert((*n, *verb)), "{n} {verb} is listed twice");
            }
        }
        assert_eq!(pairs.len(), VERBS.len());
    }
}

#[cfg(test)]
mod wait_contract_tests {
    use super::*;
    use bbt_proto::seat_state::{Activity, Grade, SubagentRow};
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };

    fn seat() -> GuiSeat {
        include_str!("../../bbt-proto/tests/golden/gui_wire.tsv")
            .lines()
            .filter_map(|line| line.strip_prefix("reply\t"))
            .filter_map(|line| serde_json::from_str::<GuiReply>(line).ok())
            .find_map(|reply| match reply {
                GuiReply::Seats(seats) => seats.into_iter().next(),
                _ => None,
            })
            .expect("the wire fixture has a seat")
    }
    fn peer(
        handler: impl FnOnce(BufReader<UnixStream>, UnixStream) + Send + 'static,
    ) -> (Client, std::thread::JoinHandle<()>) {
        let (client, server) = UnixStream::pair().unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        server
            .set_write_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        let reader = BufReader::new(server.try_clone().unwrap());
        let task = std::thread::spawn(move || handler(reader, server));
        (
            Client {
                reader: BufReader::new(client.try_clone().unwrap()),
                writer: client,
                capability: [0; 32],
                op: None,
                identity: None,
            },
            task,
        )
    }
    fn request(reader: &mut BufReader<UnixStream>) -> GuiAction {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0, "missing request");
        serde_json::from_str::<GuiRequest>(&line).unwrap().action
    }
    fn reply(writer: &mut UnixStream, reply: GuiReply) {
        serde_json::to_writer(&mut *writer, &reply).unwrap();
        writer.write_all(b"\n").unwrap();
    }
    fn predicate(subagent: Option<&str>) -> WaitPredicate {
        WaitPredicate::new(
            Some("active".into()),
            None,
            subagent.map(str::to_owned),
            Some("reported".into()),
        )
        .unwrap()
    }

    // keeps: decision #328 (subagent and reported-grade qualifiers belong to state waits)
    #[test]
    fn wait_qualifiers_require_an_activity_predicate() {
        let args = |words: &str| {
            words
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert!(
            matches!(parse_full(&args("seat wait fixture --state active --subagent t8 --grade reported")).unwrap().command,
            Command::Wait {subagent:Some(id),grade:Some(grade),..} if id=="t8" && grade=="reported")
        );
        for words in [
            "seat wait fixture --text marker --grade reported",
            "seat wait fixture --on-screen --subagent t8",
            "seat wait fixture --state active --grade inferred",
            "seat ls --subagent t8",
        ] {
            assert!(
                parse_full(&args(words))
                    .unwrap_err()
                    .starts_with(USAGE_PREFIX)
            );
        }
        assert!(matches!(
            WaitPredicate::new(Some("exited".into()), None, Some("t8".into()), None),
            Err(error) if error.starts_with(USAGE_PREFIX)
        ));
    }

    // keeps: decision #328 (current state satisfies the wait without another append)
    #[test]
    fn an_already_matching_state_needs_no_new_page() {
        let mut seat = seat();
        seat.state.activity = Activity::Active;
        seat.state.grade = Grade::Reported;
        let selector = seat.record.id.to_string();
        let (mut client, task) = peer(move |mut reader, mut writer| {
            assert!(matches!(request(&mut reader), GuiAction::ListSeats));
            reply(&mut writer, GuiReply::Seats(vec![seat]));
            let mut extra = String::new();
            assert_eq!(
                reader.read_line(&mut extra).unwrap(),
                0,
                "matched wait requested another page"
            );
        });
        let rows = wait_for_seat(
            &mut client,
            &selector,
            &predicate(None),
            Duration::from_secs(30),
            || Duration::ZERO,
        )
        .unwrap();
        assert_eq!(rows[0]["data"]["grade"], "reported");
        drop(client);
        task.join().unwrap();
    }

    // keeps: decision #328 (recheck current state, exact subagent id, activity and grade)
    #[test]
    fn page_progress_rechecks_the_exact_current_subagent_predicate() {
        let mut seat = seat();
        let selector = seat.record.id.to_string();
        let (mut client, task) = peer(move |mut reader, mut writer| {
            for (index, (id, activity, grade)) in [
                ("other", Activity::Active, Grade::Reported),
                ("t8", Activity::Active, Grade::Inferred),
                ("t8", Activity::Idle, Grade::Reported),
                ("t8", Activity::Active, Grade::Reported),
            ]
            .into_iter()
            .enumerate()
            {
                assert!(matches!(request(&mut reader), GuiAction::ListSeats));
                seat.state.subagents = vec![SubagentRow {
                    id: id.into(),
                    title: "worker".into(),
                    activity,
                    grade,
                }];
                reply(&mut writer, GuiReply::Seats(vec![seat.clone()]));
                if index == 3 {
                    break;
                }
                let GuiAction::EventsRead {
                    id,
                    stream,
                    after,
                    wait_ms,
                    ..
                } = request(&mut reader)
                else {
                    panic!("expected waiting page")
                };
                assert_eq!(stream, format!("seat-{}", seat.record.id));
                assert_eq!(after, index as u64);
                assert_eq!(wait_ms, Some(30_000));
                reply(
                    &mut writer,
                    GuiReply::EventsRead {
                        op: id,
                        events: vec![],
                        stream,
                        next: Some(after + 1),
                        through: after + 1,
                    },
                );
            }
        });
        let rows = wait_for_seat(
            &mut client,
            &selector,
            &predicate(Some("t8")),
            Duration::from_secs(30),
            || Duration::ZERO,
        )
        .unwrap();
        assert_eq!(rows[0]["data"]["subagent"], "t8");
        drop(client);
        task.join().unwrap();
    }

    // keeps: decision #328 (empty waiting page preserves cursor; seat wait reports timeout)
    #[test]
    fn an_empty_page_at_the_injected_deadline_is_a_timeout() {
        let mut seat = seat();
        seat.state.activity = Activity::Idle;
        let selector = seat.record.id.to_string();
        let now = Arc::new(AtomicU64::new(0));
        let clock = now.clone();
        let (mut client, task) = peer(move |mut reader, mut writer| {
            assert!(matches!(request(&mut reader), GuiAction::ListSeats));
            reply(&mut writer, GuiReply::Seats(vec![seat.clone()]));
            let GuiAction::Screen { id, .. } = request(&mut reader) else {
                panic!("expected screen")
            };
            reply(
                &mut writer,
                GuiReply::Screen {
                    op: id,
                    screen: bbt_proto::gui::SeatScreen {
                        seat: seat.record.id,
                        generation: seat.state.generation,
                        rows: vec!["hello-42".into()],
                        cursor: (0, 0),
                        modes: 0,
                        title: String::new(),
                        cols: 80,
                        sequence: 1,
                        cursor_style: Value::Null,
                        cells: None,
                    },
                },
            );
            let GuiAction::EventsRead { id, after, .. } = request(&mut reader) else {
                panic!("expected waiting page")
            };
            clock.store(30_000, Ordering::SeqCst);
            reply(
                &mut writer,
                GuiReply::EventsRead {
                    op: id,
                    events: vec![],
                    stream: String::new(),
                    next: None,
                    through: after,
                },
            );
        });
        let predicate = WaitPredicate::new(None, Some("never-there".into()), None, None).unwrap();
        let error = wait_for_seat(
            &mut client,
            &selector,
            &predicate,
            Duration::from_secs(30),
            || Duration::from_millis(now.load(Ordering::SeqCst)),
        )
        .unwrap_err();
        let (op, failure) = operation_failure(None, &error);
        assert!(op.is_some());
        assert_eq!((failure.kind, failure.exit), ("timeout", 3));
        assert!(failure.message.contains("hello-42"));
        drop(client);
        task.join().unwrap();
    }

    // keeps: decision #328 (screen fallback uses a bounded page wait when no frame event exists)
    #[test]
    fn text_wait_rechecks_the_screen_after_an_empty_fallback_page() {
        let seat = seat();
        let selector = seat.record.id.to_string();
        let (mut client, task) = peer(move |mut reader, mut writer| {
            for found in [false, true] {
                assert!(matches!(request(&mut reader), GuiAction::ListSeats));
                reply(&mut writer, GuiReply::Seats(vec![seat.clone()]));
                let GuiAction::Screen { id, .. } = request(&mut reader) else {
                    panic!("expected screen")
                };
                reply(
                    &mut writer,
                    GuiReply::Screen {
                        op: id,
                        screen: bbt_proto::gui::SeatScreen {
                            seat: seat.record.id,
                            generation: 1,
                            rows: vec![if found { "marker" } else { "" }.into()],
                            cursor: (0, 0),
                            modes: 0,
                            title: String::new(),
                            cols: 80,
                            sequence: u64::from(found),
                            cursor_style: Value::Null,
                            cells: None,
                        },
                    },
                );
                if found {
                    break;
                }
                let GuiAction::EventsRead {
                    id, after, wait_ms, ..
                } = request(&mut reader)
                else {
                    panic!("expected waiting page")
                };
                assert_eq!(wait_ms, Some(200));
                reply(
                    &mut writer,
                    GuiReply::EventsRead {
                        op: id,
                        events: vec![],
                        stream: String::new(),
                        next: None,
                        through: after,
                    },
                );
            }
        });
        let predicate = WaitPredicate::new(None, Some("^marker$".into()), None, None).unwrap();
        wait_for_seat(
            &mut client,
            &selector,
            &predicate,
            Duration::from_secs(30),
            || Duration::ZERO,
        )
        .unwrap();
        drop(client);
        task.join().unwrap();
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
