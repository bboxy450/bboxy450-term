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
}

struct Inner {
    sessions: BTreeMap<String, Session>,
    /// The live connection's writer, tagged so a closing older connection
    /// never clears a newer one's.
    out: Option<(u64, Sender<Frame>)>,
    connections: u64,
}

pub struct Server {
    epoch: u64,
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
        Ok(Self {
            epoch,
            inbox: Mutex::new(Inbox::new(FileJournal::open(&state.join("applied.log"))?)),
            inner: Arc::new(Mutex::new(Inner {
                sessions: BTreeMap::new(),
                out: None,
                connections: 0,
            })),
        })
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    fn spawn(&self, session: &str, cwd: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if inner.sessions.contains_key(session) {
            return Ok(()); // adopt: a reconnecting client re-sends Spawn
        }
        let (program, args) = agent();
        let mut child = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        inner.sessions.insert(
            session.to_owned(),
            Session {
                log: SessionLog::new(session, self.epoch),
                stdin,
                child,
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
                if let Some((_, out)) = &inner.out
                    && out.send(event).is_err()
                {
                    inner.out = None;
                }
            }
        });
        Ok(())
    }

    fn op(&self, session: &str, message_id: &str, envelope: &str) -> io::Result<Frame> {
        let mut inner = self.inner.lock().unwrap();
        let mut inbox = self.inbox.lock().unwrap();
        // Durable dedup first: an applied id is acknowledged even after restart.
        if inbox.is_applied(message_id) {
            return Ok(Frame::Applied {
                message_id: message_id.into(),
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

    fn close(&self, session: &str) {
        if let Some(mut s) = self.inner.lock().unwrap().sessions.remove(session) {
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
            if inner
                .out
                .as_ref()
                .is_some_and(|(current, _)| *current == id)
            {
                inner.out = None;
            }
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
            None => return Ok(()),
            Some(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "expected hello")),
        }
        let send = |f: Frame| {
            tx.send(f)
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "link writer gone"))
        };
        send(Frame::Welcome {
            version: VERSION,
            supervisor: format!("bboxy450-term-remote epoch {}", self.epoch),
        })?;
        self.inner.lock().unwrap().out = Some((id, tx.clone()));
        while let Some(frame) = frame::read(reader)? {
            match frame {
                Frame::Spawn { session, cwd, .. } => self.spawn(&session, &cwd)?,
                Frame::Op {
                    session,
                    message_id,
                    envelope,
                } => send(self.op(&session, &message_id, &envelope)?)?,
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
                Frame::Close { session } => self.close(&session),
                _ => {} // remote→local frames and unknown types are ignored
            }
        }
        Ok(())
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
            self.close(&name);
        }
    }
}
