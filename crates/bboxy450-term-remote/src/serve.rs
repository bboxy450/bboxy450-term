// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The remote supervisor's relay (ICD §4–5): one [Server] per swarm keeps its
//! harness sessions across link connections. Each session is one
//! `bboxy450-harness acp` process; ops are ACP client messages written to its
//! stdin exactly once (durable [FileJournal] dedup), and every line it writes
//! is logged as an event for resume. Only bboxy450-harness is launched;
//! `BBOXY450_REMOTE_AGENT` replaces the program for unattended tests only.
use crate::frame::{self, Frame, VERSION};
use crate::link::{Arrival, FileJournal, Inbox};
use crate::resume::SessionLog;
use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

struct Session {
    log: SessionLog,
    stdin: ChildStdin,
    child: Child,
    /// The agent closed its stdout; ops are refused with its last stderr line.
    exited: bool,
    /// Each session follows its own link; a reconnect replaces only this owner.
    out: Option<(u64, Sender<Frame>)>,
}

struct McpChannel {
    session: String,
    connection: u64,
    to: Sender<Frame>,
}

struct Inner {
    sessions: BTreeMap<String, Session>,
    /// Sessions whose agent could not start, with the reason.
    failed: BTreeMap<String, String>,
    connections: u64,
    /// MCP relay channels: remote `mcp` processes, by channel id.
    mcp: BTreeMap<u64, McpChannel>,
    channels: u64,
}

impl Inner {
    fn retire_mcp(&mut self, matches: impl Fn(&McpChannel) -> bool) {
        self.mcp.retain(|channel, route| {
            if matches(route) {
                let _ = route.to.send(Frame::McpClose { channel: *channel });
                false
            } else {
                true
            }
        });
    }
}

pub struct Server {
    epoch: u64,
    /// Stable identity of this state directory; a client pins it, so a cloned
    /// disk or a different host is refused rather than mixed in.
    id: String,
    state: std::path::PathBuf,
    inbox: Mutex<Inbox<FileJournal>>,
    inner: Arc<Mutex<Inner>>,
}

fn agent() -> (String, Vec<String>) {
    match std::env::var("BBOXY450_REMOTE_AGENT") {
        Ok(program) => (program, Vec::new()),
        Err(_) => ("bboxy450-harness".into(), vec!["acp".into()]),
    }
}

impl Server {
    /// Open the swarm's state directory; each open is a new epoch.
    pub fn open(state: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(state)?;
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        let id_path = state.join("supervisor.id");
        let id = match std::fs::read_to_string(&id_path) {
            Ok(id) if !id.trim().is_empty() => id.trim().to_owned(),
            _ => {
                let id = format!("{:x}-{:x}", epoch, std::process::id());
                std::fs::write(&id_path, format!("{id}\n"))?;
                id
            }
        };
        Ok(Self {
            epoch,
            id,
            state: state.to_owned(),
            inbox: Mutex::new(Inbox::new(FileJournal::open(&state.join("applied.log"))?)),
            inner: Arc::new(Mutex::new(Inner {
                sessions: BTreeMap::new(),
                failed: BTreeMap::new(),
                connections: 0,
                mcp: BTreeMap::new(),
                channels: 0,
            })),
        })
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn id(&self) -> &str {
        &self.id
    }

    fn spawn(&self, session: &str, cwd: &str, id: u64, out: &Sender<Frame>) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(s) = inner.sessions.get_mut(session) {
            if s.out.as_ref().map(|(owner, _)| *owner) != Some(id) {
                s.out = Some((id, out.clone()));
                inner.retire_mcp(|route| route.session == session);
            }
            return Ok(()); // adopt: a reconnecting client re-sends Spawn
        }
        let (program, args) = agent();
        // Each agent's stderr goes to agent-<session>.log in the state dir, so
        // a missing feature or a crash is diagnosable instead of a silent hang.
        let log = self.agent_log(session);
        let spawned = std::fs::File::create(&log).and_then(|err| {
            // A `~/` directory is this user's home on this machine.
            let cwd = match (cwd.strip_prefix("~/"), std::env::var("HOME")) {
                (Some(rest), Ok(home)) => format!("{home}/{rest}"),
                _ => cwd.to_owned(),
            };
            Command::new(&program)
                .args(&args)
                .current_dir(cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::from(err))
                .spawn()
        });
        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => {
                inner.failed.insert(
                    session.to_owned(),
                    format!("agent {program} failed to start: {e}"),
                );
                return Ok(());
            }
        };
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        inner.sessions.insert(
            session.to_owned(),
            Session {
                log: SessionLog::new(session, self.epoch),
                stdin,
                child,
                exited: false,
                out: Some((id, out.clone())),
            },
        );
        let shared = Arc::clone(&self.inner);
        let name = session.to_owned();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let mut inner = shared.lock().unwrap();
                let Some(s) = inner.sessions.get_mut(&name) else {
                    break;
                };
                let event = s.log.push(line);
                if let Some((_, out)) = &s.out
                    && out.send(event).is_err()
                {
                    s.out = None;
                }
            }
            if let Some(s) = shared.lock().unwrap().sessions.get_mut(&name) {
                s.exited = true;
            }
        });
        Ok(())
    }

    fn agent_log(&self, session: &str) -> std::path::PathBuf {
        let safe: String = session
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.state.join(format!("agent-{safe}.log"))
    }

    fn last_error(&self, session: &str) -> String {
        std::fs::read_to_string(self.agent_log(session))
            .ok()
            .and_then(|t| {
                t.lines()
                    .rev()
                    .find(|l| !l.trim().is_empty())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "no stderr".into())
    }

    fn op(&self, session: &str, message_id: &str, envelope: &str, id: u64) -> io::Result<Frame> {
        let mut inner = self.inner.lock().unwrap();
        if inner
            .sessions
            .get(session)
            .is_some_and(|s| s.out.as_ref().map(|(owner, _)| *owner) != Some(id))
        {
            return Ok(Frame::Refused {
                message_id: message_id.into(),
                reason: "session belongs to another link".into(),
            });
        }
        let mut inbox = self.inbox.lock().unwrap();
        // Durable dedup first: an applied id is acknowledged even after restart.
        if inbox.is_applied(message_id) {
            return Ok(Frame::Applied {
                message_id: message_id.into(),
            });
        }
        if let Some(reason) = inner.failed.get(session) {
            return Ok(Frame::Refused {
                message_id: message_id.into(),
                reason: reason.clone(),
            });
        }
        if inner.sessions.get(session).is_some_and(|s| s.exited) {
            return Ok(Frame::Refused {
                message_id: message_id.into(),
                reason: format!("agent exited: {}", self.last_error(session)),
            });
        }
        let Some(s) = inner.sessions.get_mut(session) else {
            return Ok(Frame::Refused {
                message_id: message_id.into(),
                reason: "unknown session".into(),
            });
        };
        if envelope.contains('\n') {
            return Ok(Frame::Refused {
                message_id: message_id.into(),
                reason: "envelope must be one line".into(),
            });
        }
        match inbox.arrive(message_id)? {
            Arrival::Fresh => {
                // Journalled before the write: a crash here loses this op (unknown), never doubles it.
                writeln!(s.stdin, "{envelope}")?;
                s.stdin.flush()?;
            }
            Arrival::Duplicate => {}
        }
        Ok(Frame::Applied {
            message_id: message_id.into(),
        })
    }

    fn close(&self, session: &str, owner: Option<u64>) {
        let closed = {
            let mut inner = self.inner.lock().unwrap();
            if owner.is_some_and(|owner| {
                inner
                    .sessions
                    .get(session)
                    .is_none_or(|s| s.out.as_ref().map(|(id, _)| *id) != Some(owner))
            }) {
                return;
            }
            inner.retire_mcp(|route| route.session == session);
            inner.sessions.remove(session)
        };
        if let Some(mut s) = closed {
            let _ = s.child.kill();
            let _ = s.child.wait();
        }
    }

    /// Serve one link connection until it ends; sessions outlive it.
    pub fn serve(
        &self,
        mut reader: impl Read,
        writer: impl Write + Send + 'static,
    ) -> io::Result<()> {
        let (tx, rx) = mpsc::channel::<Frame>();
        let writer = std::thread::spawn(move || {
            let mut writer = writer;
            for frame in rx {
                if frame::write(&mut writer, &frame).is_err() {
                    break;
                }
            }
        });
        let id = {
            let mut inner = self.inner.lock().unwrap();
            inner.connections += 1;
            inner.connections
        };
        let result = self.connection(&mut reader, &tx, id);
        {
            let mut inner = self.inner.lock().unwrap();
            for session in inner.sessions.values_mut() {
                if session
                    .out
                    .as_ref()
                    .is_some_and(|(current, _)| *current == id)
                {
                    session.out = None;
                }
            }
            inner.retire_mcp(|route| route.connection == id);
        }
        drop(tx);
        let _ = writer.join();
        result
    }

    fn connection(&self, reader: &mut impl Read, tx: &Sender<Frame>, id: u64) -> io::Result<()> {
        match frame::read(reader)? {
            Some(Frame::Hello { version, .. }) if version == VERSION => {}
            Some(Frame::Hello { .. }) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "link major version mismatch",
                ));
            }
            Some(Frame::McpOpen { session, name, .. }) => {
                return self.mcp_channel(reader, tx, &session, &name);
            }
            None => return Ok(()),
            Some(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "expected hello")),
        }
        let send = |f: Frame| {
            tx.send(f)
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "link writer gone"))
        };
        send(Frame::Welcome {
            version: VERSION,
            supervisor: self.id.clone(),
        })?;
        let pinger = tx.clone();
        std::thread::spawn(move || {
            let every = std::env::var("BBOXY450_REMOTE_PING_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15_000);
            while pinger.send(Frame::Ping).is_ok() {
                std::thread::sleep(std::time::Duration::from_millis(every));
            }
        });

        while let Some(frame) = frame::read(reader)? {
            match frame {
                Frame::Spawn { session, cwd, .. } => self.spawn(&session, &cwd, id, tx)?,
                Frame::Op {
                    session,
                    message_id,
                    envelope,
                } => send(self.op(&session, &message_id, &envelope, id)?)?,
                Frame::Resume {
                    session,
                    epoch,
                    after_seq,
                } => {
                    let frames = match self.inner.lock().unwrap().sessions.get(&session) {
                        Some(s) => s.log.replay(epoch, after_seq),
                        None => vec![Frame::Reset {
                            session: session.clone(),
                            epoch: self.epoch,
                            from_seq: 1,
                        }],
                    };
                    for f in frames {
                        send(f)?;
                    }
                }
                Frame::Close { session } => self.close(&session, Some(id)),
                Frame::McpData { channel, line } => {
                    if let Some(route) = self.inner.lock().unwrap().mcp.get(&channel)
                        && route.connection == id
                    {
                        let _ = route.to.send(Frame::McpData { channel, line });
                    }
                }
                Frame::McpClose { channel } => {
                    let mut inner = self.inner.lock().unwrap();
                    if inner
                        .mcp
                        .get(&channel)
                        .is_some_and(|route| route.connection == id)
                        && let Some(route) = inner.mcp.remove(&channel)
                    {
                        let _ = route.to.send(Frame::McpClose { channel });
                    }
                }
                _ => {} // remote→local frames and unknown types are ignored
            }
        }
        Ok(())
    }

    /// One remote `bboxy450-term-remote mcp` process: its lines go to the live
    /// link as McpData on a fresh channel, and the link's replies come back.
    fn mcp_channel(
        &self,
        reader: &mut impl Read,
        tx: &Sender<Frame>,
        session: &str,
        name: &str,
    ) -> io::Result<()> {
        let channel = {
            let mut inner = self.inner.lock().unwrap();
            let Some((connection, out)) = inner.sessions.get(session).and_then(|s| s.out.clone())
            else {
                let _ = tx.send(Frame::McpClose { channel: 0 });
                return Ok(()); // no live link: the MCP server ends at once
            };
            inner.channels += 1;
            let channel = inner.channels;
            inner.mcp.insert(
                channel,
                McpChannel {
                    session: session.into(),
                    connection,
                    to: tx.clone(),
                },
            );
            let _ = out.send(Frame::McpOpen {
                session: session.into(),
                channel,
                name: name.into(),
            });
            channel
        };
        let forward = |f: Frame| {
            let inner = self.inner.lock().unwrap();
            if let Some(route) = inner.mcp.get(&channel)
                && let Some((owner, out)) = inner.sessions.get(session).and_then(|s| s.out.as_ref())
                && *owner == route.connection
            {
                let _ = out.send(f);
            }
        };
        let result = (|| {
            while let Some(frame) = frame::read(reader)? {
                if let Frame::McpData { line, .. } = frame {
                    forward(Frame::McpData { channel, line });
                }
            }
            Ok(())
        })();
        forward(Frame::McpClose { channel });
        self.inner.lock().unwrap().mcp.remove(&channel);
        result
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let names: Vec<_> = self
            .inner
            .lock()
            .unwrap()
            .sessions
            .keys()
            .cloned()
            .collect();
        for name in names {
            self.close(&name, None);
        }
    }
}
