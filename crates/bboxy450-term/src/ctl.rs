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
use bbt_proto::{
    GuiAction, GuiReply, GuiRequest, GuiSeat, SpawnSpec, SwarmId, Where,
    messaging::{Request, Response, SendRequest, TaskRequest, TaskResponse},
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

pub const USAGE: &str = "usage: bboxy450-term ctl [--endpoint FILE] [--as LABEL] <command>
  spawn <claude|codex|zsh> [--placement new-tab] [--name NAME] [--cwd DIR] [--swarm ID]
        [--role oracle|architect|creativity|executor|shell]
  seats
  dm <seat> <text>        questions and feedback; work goes by delegate
  post <destination> <text> [--swarm ID]
  inbox [--since SEQ]
  receipts <message-id>
  delegate <#N|path|id> <seat>
  reports [--since TIME]
  capture <file.png> [--seat SEAT]
  seat screen <seat> [--human]    the seat's visible screen, its cursor and title (#270)
  seat type <seat> <text>         types the text as written
  seat key <seat> <key>...        enter esc tab backspace space up down left right home end
                                  pageup pagedown delete shift-tab ctrl-<letter>; Kitty when on
  seat paste <seat> <file|->      pastes a file or stdin, bracketed when the program asked
  seat wait <seat> (--state idle|waiting|exited | --text REGEX) [--timeout MS]  default 30000
  e2e [--group GROUP]
A <seat> is a seat id or a unique seat name. Output is one JSON object per line.
--as LABEL (or BBOXY_CTL_ISSUER) names who issues the messages, e.g. 'Vesper (Oracle)';
seats read 'the operator principal, via ctl, issued by LABEL'. It is attribution only.";

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
        timeout: Duration,
    },
    E2e {
        group: Option<String>,
    },
}

/// Parses everything after `ctl`; returns the endpoint file override and the command.
pub fn parse(args: &[String]) -> Result<(Option<PathBuf>, Command), String> {
    let mut endpoint = None;
    let mut flags = std::collections::BTreeMap::new();
    let mut words = vec![];
    let mut human_flag = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "--json" {
            // The default reply form (ctl-cli §1).
        } else if arg == "--human" {
            human_flag = true;
        } else if let Some(flag) = arg.strip_prefix("--") {
            let value = it.next().ok_or(format!("--{flag} needs a value"))?;
            if flag == "endpoint" {
                endpoint = Some(PathBuf::from(value));
            } else {
                flags.insert(flag.to_owned(), value.clone());
            }
        } else {
            words.push(arg.clone());
        }
    }
    let ulid = |v: &String| v.parse::<Ulid>().map_err(|_| format!("not an id: {v}"));
    let flag = |name: &str| flags.get(name).cloned();
    let swarm = flag("swarm").as_ref().map(ulid).transpose()?;
    let command = match words.iter().map(String::as_str).collect::<Vec<_>>()[..] {
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
        },
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
            if state.is_some() == text.is_some() {
                return Err(format!("seat wait: one of --state or --text\n{USAGE}"));
            }
            Command::Wait {
                seat: seat.into(),
                state,
                text,
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
        ["capture", path] => Command::Capture {
            path: std::path::absolute(path).map_err(|e| format!("capture {path}: {e}"))?,
            seat: flag("seat"),
        },
        _ => return Err(USAGE.into()),
    };
    if let Some(unknown) = flags.keys().find(|k| {
        ![
            "swarm",
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
        ]
        .contains(&k.as_str())
    }) {
        return Err(format!("unknown flag --{unknown}\n{USAGE}"));
    }
    Ok((endpoint, command))
}

/// One admitted connection to `gui.sock`.
pub struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    capability: [u8; 32],
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
        })
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
        let id = Ulid::new();
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
            _ => Ulid::new(),
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
        let id = Ulid::new();
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
        let id = Ulid::new();
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
    /// The seat's screen, with the op id of the read.
    pub fn screen(&mut self, seat: Ulid) -> Result<(Ulid, bbt_proto::gui::SeatScreen), String> {
        let id = Ulid::new();
        match self.call(
            Some(id),
            GuiAction::Screen { id, seat },
            |r| matches!(r, GuiReply::Screen { op, .. } if *op == id),
        )? {
            GuiReply::Screen { screen, .. } => Ok((id, screen)),
            _ => unreachable!(),
        }
    }
    /// The window's input action, under this connection's door and issuer; returns its op.
    pub fn input(&mut self, seat: Ulid, generation: u64, bytes: Vec<u8>) -> Result<Ulid, String> {
        let id = Ulid::new();
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
    pub fn task(&mut self, request: TaskRequest) -> Result<TaskResponse, String> {
        let id = Ulid::new();
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
                "frame_presented_at": frame.presented_at_ms })]
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
        Command::Screen { seat, human } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let (op, screen) = client.screen(seat)?;
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
            input(client, &seat, "paste", |modes| Ok(bracketed(&text, modes)))?
        }
        Command::Wait {
            seat,
            state,
            text,
            timeout,
        } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            let pattern = text
                .as_deref()
                .map(|p| regex::Regex::new(p).map_err(|e| format!("seat wait --text: {e}")))
                .transpose()?;
            if let Some(state) = &state
                && !matches!(state.as_str(), "idle" | "waiting" | "exited")
            {
                return Err(format!(
                    "seat wait --state: idle, waiting or exited, not {state}"
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

pub fn main(args: &[String], default_endpoint: &Path) -> Result<(), String> {
    match run_main(args, default_endpoint) {
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
    }
}
fn run_main(args: &[String], default_endpoint: &Path) -> Result<(), String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    // --as names the issuer (decision #269); it is not a command flag.
    let mut args = args.to_vec();
    let mut issuer = std::env::var("BBOXY_CTL_ISSUER").ok();
    if let Some(at) = args.iter().position(|arg| arg == "--as") {
        args.remove(at);
        if at >= args.len() {
            return Err("--as needs a value".into());
        }
        issuer = Some(args.remove(at));
    }
    let (endpoint, command) = parse(&args)?;
    let endpoint = Endpoint::read(endpoint.as_deref().unwrap_or(default_endpoint))?;
    let mut client = Client::connect(&endpoint)?;
    if let Some(label) = issuer.filter(|label| !label.is_empty()) {
        client.issue(label)?;
    }
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
