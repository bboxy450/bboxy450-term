// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Local end of the link: an [Outbox] of ops and a [Cursor] per session, over a
//! [Transport] that is re-dialled after a drop. Reconnect re-sends hello, every
//! spawn (the daemon adopts live sessions), a resume per session and every
//! unacknowledged op; the remote's durable dedup makes the resend safe.
use crate::frame::{self, Frame, VERSION};
use crate::link::Outbox;
use crate::resume::{Accept, Cursor};
use std::collections::BTreeMap;
use std::io;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

/// How to reach a swarm's remote supervisor. Every variant is one local
/// command whose stdin/stdout carry the link unchanged and never prompt.
/// Inspired by herdr-core's `hc machine add --via` (Apache-2.0,
/// https://github.com/herdrdev/herdr-core, docs/machines.md).
#[derive(Debug, Clone)]
pub enum Transport {
    /// `ssh -T` with BatchMode, ConnectTimeout and ServerAliveInterval; the
    /// user's SSH config decides host, user, port and key.
    Ssh { host: String, state: String },
    /// Any carrier (`fly ssh console -C`, `wsl.exe -e sh -c`, a sandbox CLI):
    /// run as `sh -c 'VIA "exec bboxy450-term-remote serve --state STATE"'`.
    Via { command: String, state: String },
    /// A local program run with `serve --state STATE` (loopback and tests).
    Local { program: String, state: String },
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// `install` puts binaries in ~/.local/bin, which a non-login shell may lack.
const REMOTE_PATH: &str = "PATH=\"$HOME/.local/bin:$PATH\"; export PATH;";

/// Quote for the remote shell; a leading `~/` is the remote home.
fn quote_path(path: &str) -> String {
    match path.strip_prefix("~/") {
        Some(rest) => format!("\"$HOME\"/{}", quote(rest)),
        None => quote(path),
    }
}
fn remote_line(state: &str) -> String {
    format!(
        "{REMOTE_PATH} exec bboxy450-term-remote serve --state {}",
        quote_path(state)
    )
}

impl Transport {
    /// The remote state directory this transport serves.
    pub fn state(&self) -> &str {
        match self {
            Transport::Ssh { state, .. }
            | Transport::Via { state, .. }
            | Transport::Local { state, .. } => state,
        }
    }
    /// Run one shell line on the remote (install, probes) through the same carrier.
    pub fn shell(&self, line: &str) -> Command {
        let line = format!("{REMOTE_PATH} {line}");
        match self {
            Transport::Ssh { host, .. } => {
                let mut c = Command::new("ssh");
                c.args(["-T", "-o", "BatchMode=yes", "-o", "ConnectTimeout=10"]);
                c.arg(host).arg(line);
                c
            }
            Transport::Via { command, .. } => {
                let mut c = Command::new("sh");
                c.arg("-c").arg(format!("{command} {}", quote(&line)));
                c
            }
            Transport::Local { .. } => {
                let mut c = Command::new("sh");
                c.arg("-c").arg(line);
                c
            }
        }
    }
    fn command(&self) -> Command {
        match self {
            Transport::Ssh { host, state } => {
                let mut c = Command::new("ssh");
                c.args([
                    "-T",
                    "-o",
                    "BatchMode=yes",
                    "-o",
                    "ConnectTimeout=10",
                    "-o",
                    "ServerAliveInterval=15",
                ]);
                c.arg(host).arg(remote_line(state));
                c
            }
            Transport::Via { command, state } => {
                let mut c = Command::new("sh");
                c.arg("-c")
                    .arg(format!("{command} {}", quote(&remote_line(state))));
                c
            }
            Transport::Local { program, state } => {
                let mut c = Command::new(program);
                c.args(["serve", "--state", state]);
                c
            }
        }
    }
}

/// What a received frame meant to the client.
#[derive(Debug, PartialEq, Eq)]
pub enum Received {
    /// Show this event's envelope for the session.
    Show {
        session: String,
        envelope: String,
    },
    /// The view of this session was cleared for a new epoch.
    Cleared {
        session: String,
    },
    /// The op is applied remotely and left the outbox.
    Applied {
        message_id: String,
    },
    /// The op will never apply; the caller labels it failed.
    Refused {
        message_id: String,
        reason: String,
    },
    /// Duplicate/stale/out-of-order event, or a frame needing no action.
    Nothing,
    /// The link ended or carried no bytes for the dead-line timeout.
    Down,
    /// MCP relay: the remote opened, wrote to or closed a channel.
    McpOpen {
        session: String,
        channel: u64,
        name: String,
    },
    McpData {
        channel: u64,
        line: String,
    },
    McpClose {
        channel: u64,
    },
}

struct Live {
    child: Child,
    input: ChildStdin,
    frames: Receiver<io::Result<Option<Frame>>>,
    /// When the last frame arrived; silence past the dead line means gone.
    heard: std::time::Instant,
}

/// No bytes for this long means the line is dead (pings arrive every 15 s).
fn dead_line() -> Duration {
    Duration::from_millis(
        std::env::var("BBOXY450_REMOTE_DEAD_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(45_000),
    )
}

fn reader(mut output: ChildStdout) -> Receiver<io::Result<Option<Frame>>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        loop {
            let next = frame::read(&mut output);
            let end = !matches!(next, Ok(Some(_)));
            if tx.send(next).is_err() || end {
                break;
            }
        }
    });
    rx
}

pub struct Client {
    transport: Transport,
    swarm: String,
    live: Option<Live>,
    outbox: Outbox<Frame>,
    cursors: BTreeMap<String, Cursor>,
    spawns: BTreeMap<String, Frame>,
    supervisor: Option<String>,
}

impl Client {
    pub fn new(transport: Transport, swarm: &str) -> Self {
        Self {
            transport,
            swarm: swarm.into(),
            live: None,
            outbox: Outbox::new(),
            cursors: BTreeMap::new(),
            spawns: BTreeMap::new(),
            supervisor: None,
        }
    }

    /// Dial (or re-dial) the remote and bring every session up to date.
    pub fn connect(&mut self) -> io::Result<()> {
        self.disconnect();
        let mut child = self
            .transport
            .command()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut input = child.stdin.take().expect("piped");
        let mut output = child.stdout.take().expect("piped");
        if let Err(e) = frame::expect_magic(&mut output) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
        frame::write(
            &mut input,
            &Frame::Hello {
                version: VERSION,
                swarm_id: self.swarm.clone(),
                host: String::new(),
            },
        )?;
        match frame::read(&mut output)? {
            Some(Frame::Welcome {
                version,
                supervisor,
            }) if version == VERSION => {
                if self
                    .supervisor
                    .as_ref()
                    .is_some_and(|pinned| *pinned != supervisor)
                {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!(
                            "remote supervisor changed identity ({supervisor}); refusing to mix sessions"
                        ),
                    ));
                }
                self.supervisor = Some(supervisor);
            }
            other => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("bad welcome: {other:?}"),
                ));
            }
        }
        for spawn in self.spawns.values() {
            frame::write(&mut input, spawn)?;
        }
        for (session, cursor) in &self.cursors {
            frame::write(
                &mut input,
                &Frame::Resume {
                    session: session.clone(),
                    epoch: cursor.epoch,
                    after_seq: cursor.seq,
                },
            )?;
        }
        self.outbox.reconnect();
        for (_, op) in self.outbox.sendable() {
            frame::write(&mut input, &op)?;
        }
        self.live = Some(Live {
            child,
            input,
            frames: reader(output),
            heard: std::time::Instant::now(),
        });
        Ok(())
    }

    /// Redial until `deadline`: immediately first (most drops are a restart
    /// there), then 250 ms doubling to 30 s. An identity change is final.
    /// Backoff adapted in spirit from herdr-core's line reconnect (Apache-2.0).
    pub fn reconnect(&mut self, deadline: std::time::Instant) -> io::Result<()> {
        let mut wait = std::time::Duration::from_millis(250);
        loop {
            match self.connect() {
                Ok(()) => return Ok(()),
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => return Err(e),
                Err(e) if std::time::Instant::now() + wait > deadline => return Err(e),
                Err(_) => {
                    std::thread::sleep(wait);
                    wait = (wait * 2).min(std::time::Duration::from_secs(30));
                }
            }
        }
    }

    /// The link is gone (or being replaced): nothing is in flight any more.
    pub fn disconnect(&mut self) {
        if let Some(mut live) = self.live.take() {
            drop(live.input);
            let _ = live.child.kill();
            let _ = live.child.wait();
        }
        self.outbox.drop_link();
    }

    fn send(&mut self, f: &Frame) -> io::Result<()> {
        let live = self
            .live
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "link down"))?;
        frame::write(&mut live.input, f)
    }

    /// Start (or adopt) a session; its first frame back is a reset with the
    /// retained history. `spawn_live` instead starts the view "from now".
    pub fn spawn(&mut self, session: &str, cwd: &str) -> io::Result<()> {
        self.spawn_at(session, cwd, 0)
    }
    pub fn spawn_live(&mut self, session: &str, cwd: &str) -> io::Result<()> {
        self.spawn_at(session, cwd, u64::MAX)
    }
    fn spawn_at(&mut self, session: &str, cwd: &str, after_seq: u64) -> io::Result<()> {
        let f = Frame::Spawn {
            session: session.into(),
            profile: None,
            cwd: cwd.into(),
        };
        self.spawns.insert(session.into(), f.clone());
        self.cursors.entry(session.into()).or_default();
        self.send(&f)?;
        self.send(&Frame::Resume {
            session: session.into(),
            epoch: 0,
            after_seq,
        })
    }

    /// Queue one ACP message for a session; sent now if the link is up, else on reconnect.
    pub fn op(&mut self, session: &str, message_id: &str, envelope: &str) -> io::Result<()> {
        let f = Frame::Op {
            session: session.into(),
            message_id: message_id.into(),
            envelope: envelope.into(),
        };
        self.outbox.submit(message_id, f).map_err(|_| {
            io::Error::new(io::ErrorKind::AlreadyExists, "message id already submitted")
        })?;
        for (_, op) in self.outbox.sendable() {
            self.send(&op)?;
        }
        Ok(())
    }

    /// Send an MCP relay frame now; MCP is not queued across a drop.
    pub fn mcp(&mut self, f: &Frame) -> io::Result<()> {
        self.send(f)
    }

    /// Any op still waiting for its `applied`.
    pub fn has_pending(&self) -> bool {
        self.outbox.has_unacked()
    }
    pub fn is_pending(&self, message_id: &str) -> bool {
        self.outbox.is_unacked(message_id)
    }

    /// Block for the next frame and interpret it. `Ok(None)`: the link ended.
    pub fn recv(&mut self) -> io::Result<Option<Received>> {
        loop {
            match self.recv_timeout(dead_line())? {
                Some(Received::Down) => return Ok(None),
                Some(r) => return Ok(Some(r)),
                None => continue, // timed out just before the dead line
            }
        }
    }

    /// Wait up to `timeout` for a frame. `Ok(None)`: nothing yet (link alive);
    /// `Received::Down`: the link ended or stayed silent past the dead line.
    pub fn recv_timeout(&mut self, timeout: Duration) -> io::Result<Option<Received>> {
        let Some(live) = self.live.as_mut() else {
            return Ok(Some(Received::Down));
        };
        let f = match live.frames.recv_timeout(timeout) {
            Ok(Ok(Some(f))) => {
                live.heard = std::time::Instant::now();
                f
            }
            Err(RecvTimeoutError::Timeout) if live.heard.elapsed() < dead_line() => {
                return Ok(None);
            }
            _ => {
                self.disconnect();
                return Ok(Some(Received::Down));
            }
        };
        Ok(Some(match f {
            Frame::Event {
                session,
                epoch,
                seq,
                envelope,
            } => match self.cursors.get_mut(&session).map(|c| c.accept(epoch, seq)) {
                Some(Accept::Apply) => Received::Show { session, envelope },
                Some(Accept::Gap) => {
                    let c = self.cursors[&session];
                    self.send(&Frame::Resume {
                        session,
                        epoch: c.epoch,
                        after_seq: c.seq,
                    })?;
                    Received::Nothing
                }
                _ => Received::Nothing,
            },
            Frame::Reset {
                session,
                epoch,
                from_seq,
            } => {
                self.cursors
                    .entry(session.clone())
                    .or_default()
                    .reset(epoch, from_seq);
                Received::Cleared { session }
            }
            Frame::Applied { message_id } => {
                self.outbox.on_applied(&message_id);
                Received::Applied { message_id }
            }
            Frame::Refused { message_id, reason } => {
                self.outbox.on_applied(&message_id); // retire: it will never apply
                Received::Refused { message_id, reason }
            }
            Frame::McpOpen {
                session,
                channel,
                name,
            } => Received::McpOpen {
                session,
                channel,
                name,
            },
            Frame::McpData { channel, line } => Received::McpData { channel, line },
            Frame::McpClose { channel } => Received::McpClose { channel },
            _ => Received::Nothing,
        }))
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.disconnect();
    }
}
