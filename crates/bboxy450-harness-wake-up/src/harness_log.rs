// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Read-only harness observation. Discovery pins the native socket's kernel peer
//! PID/uid and Hello session, never a newest-file guess. No operations are sent.
use crate::{Binding, Grade, Observation, Result, Session, State, WaitingKind, invalid};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const CHUNK: usize = 64 * 1024;
const LINE_LIMIT: usize = 256 * 1024;

/// Locate the socket bound by this exact supervisor-owned harness process.
/// Missing sockets, unrelated sessions and unresponsive peers confer no evidence.
///
/// # Errors
/// Returns discovery, ownership, framing or native identity failures.
pub async fn discover(root: &Path, binding: &Binding) -> Result<String> {
    let mut entries = std::fs::read_dir(root.join("run"))?;
    for entry in entries.by_ref().take(4096) {
        let path = entry?.path();
        let Some(session) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if path.extension().is_none_or(|s| s != "sock") || session.parse::<crate::Ulid>().is_err() {
            continue;
        }
        if hello(&path, session, binding).await.is_ok() {
            return Ok(session.into());
        }
    }
    Err(invalid("no harness socket matches the seat process"))
}

/// Read a fresh native Hello from the already selected session's socket.
/// This proves discovery/liveness, not a state heartbeat or fresh log observation.
///
/// # Errors
/// Refuses another PID/uid/session, oversized frames and unavailable peers.
pub async fn hello(path: &Path, session: &str, binding: &Binding) -> Result<()> {
    discover_socket(path, Some(session), binding)
        .await
        .map(|_| ())
}

/// Discover the session on the term-selected endpoint, fenced by kernel peer identity.
///
/// # Errors
/// Refuses another process, malformed identity or an unavailable endpoint.
pub async fn discover_socket(
    path: &Path,
    session: Option<&str>,
    binding: &Binding,
) -> Result<String> {
    let exchange = async {
        let meta = std::fs::symlink_metadata(path)?;
        use std::os::unix::fs::FileTypeExt;
        // SAFETY: geteuid has no arguments or side effects.
        if !meta.file_type().is_socket() || meta.uid() != unsafe { libc::geteuid() } {
            return Err(invalid("harness endpoint is not an owned socket"));
        }
        let mut stream = tokio::net::UnixStream::connect(path).await?;
        let peer = stream.peer_cred()?;
        if peer.pid().map(|pid| pid as u32) != Some(binding.pid) || peer.uid() != meta.uid() {
            return Err(invalid("harness socket process mismatch"));
        }
        stream
            .write_all(
                format!(
                    "{}\n",
                    json!({
                        "v":{"major":1,"minor":1},
                        "subscribe":{"after":null,"ephemeral":false}
                    })
                )
                .as_bytes(),
            )
            .await?;
        let mut line = Vec::new();
        let mut reader = BufReader::new(stream);
        // A bounded read, even if an unrelated same-uid process misbehaves.
        loop {
            let bytes = reader.fill_buf().await?;
            if bytes.is_empty() {
                return Err(invalid("harness Hello missing"));
            }
            let n = bytes
                .iter()
                .position(|b| *b == b'\n')
                .map_or(bytes.len(), |i| i + 1);
            if line.len() + n > LINE_LIMIT {
                return Err(invalid("harness Hello oversized"));
            }
            line.extend_from_slice(&bytes[..n]);
            reader.consume(n);
            if line.ends_with(b"\n") {
                break;
            }
        }
        let value: Value = serde_json::from_slice(&line)?;
        let actual = value["meta"]["session"]
            .as_str()
            .ok_or_else(|| invalid("missing Hello session"))?;
        actual.parse::<crate::Ulid>()?;
        if value["v"]["major"] != 1 || session.is_some_and(|expected| actual != expected) {
            return Err(invalid("harness Hello session/version mismatch"));
        }
        Ok(actual.to_owned())
    };
    tokio::time::timeout(std::time::Duration::from_millis(250), exchange).await?
}

/// Incremental bounded reader of one selected session. Reads run on an IO owner.
/// Partial final lines are retained; duplicates are ignored and gaps fail closed.
pub struct Log {
    path: PathBuf,
    session: String,
    file: std::fs::File,
    identity: (u64, u64),
    offset: u64,
    pending: Vec<u8>,
    dirty: bool,
    caught_up: bool,
    seq: u64,
    epoch: u64,
    observed_ms: u64,
    turns: BTreeSet<String>,
    items: Vec<(String, String, bool)>,
    error: Option<String>,
}
impl Log {
    /// Open an owned regular JSONL file for a socket-verified session.
    ///
    /// # Errors
    /// Refuses symlinks, another uid, missing files and invalid session identifiers.
    pub fn open(root: &Path, session: &str) -> Result<Self> {
        session.parse::<crate::Ulid>()?;
        let path = root.join("sessions").join(format!("{session}.jsonl"));
        let meta = std::fs::symlink_metadata(&path)?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)?;
        let opened = file.metadata()?;
        // SAFETY: geteuid has no arguments or side effects.
        if !opened.is_file()
            || opened.uid() != unsafe { libc::geteuid() }
            || (meta.dev(), meta.ino()) != (opened.dev(), opened.ino())
        {
            return Err(invalid("invalid harness session log"));
        }
        Ok(Self {
            path,
            session: session.into(),
            file,
            identity: (meta.dev(), meta.ino()),
            offset: 0,
            pending: Vec::new(),
            dirty: false,
            caught_up: false,
            seq: 0,
            epoch: 0,
            observed_ms: 0,
            turns: BTreeSet::new(),
            items: Vec::new(),
            error: None,
        })
    }
    /// Consume at most 64 KiB and return fresh reported projection only on new events.
    ///
    /// # Errors
    /// Refuses replacement/truncation, oversized lines, wrong sessions and sequence gaps.
    pub fn poll(&mut self) -> Result<Option<Value>> {
        let meta = std::fs::symlink_metadata(&self.path)?;
        if !meta.is_file() || (meta.dev(), meta.ino()) != self.identity || meta.len() < self.offset
        {
            return Err(invalid("harness log replaced or truncated"));
        }
        let mut bytes = [0; CHUNK];
        let n = self.file.read(&mut bytes)?;
        self.offset += n as u64;
        self.pending.extend_from_slice(&bytes[..n]);
        let mut consumed = 0;
        self.caught_up = false;
        while let Some(end) = self.pending[consumed..].iter().position(|b| *b == b'\n') {
            let end = consumed + end + 1;
            if end - consumed > LINE_LIMIT {
                return Err(invalid("harness log line oversized"));
            }
            let event: Value = serde_json::from_slice(&self.pending[consumed..end])?;
            self.dirty |= self.event(&event)?;
            consumed = end;
        }
        self.pending.drain(..consumed);
        if self.pending.len() > LINE_LIMIT {
            return Err(invalid("harness log line oversized"));
        }
        self.caught_up = self.offset == meta.len() && self.pending.is_empty();
        // Historical prefixes never replace live reports with an old idle.
        Ok((self.caught_up && std::mem::take(&mut self.dirty)).then(|| self.projection()))
    }
    /// Whether the last read reached the observed log head without a partial line.
    pub fn caught_up(&self) -> bool {
        self.caught_up
    }
    fn event(&mut self, envelope: &Value) -> Result<bool> {
        if envelope["session"] != self.session || envelope["v"]["major"] != 1 {
            return Err(invalid("harness log session/version mismatch"));
        }
        let seq = envelope["seq"]
            .as_u64()
            .ok_or_else(|| invalid("missing harness log sequence"))?;
        if seq <= self.seq {
            return Ok(false);
        }
        if seq != self.seq + 1 {
            return Err(invalid("harness log sequence gap"));
        }
        self.seq = seq;
        self.epoch = envelope["epoch"]
            .as_u64()
            .ok_or_else(|| invalid("missing harness epoch"))?;
        self.observed_ms = envelope["ts_ms"]
            .as_u64()
            .ok_or_else(|| invalid("missing harness timestamp"))?;
        let e = &envelope["event"];
        match e["type"].as_str() {
            Some("session_recovered" | "session_closed") => {
                self.turns.clear();
                self.items.clear();
                self.error = None;
            }
            Some("turn_started") => {
                self.turns.insert(e["turn"].to_string());
                self.error = None;
                if self.turns.len() > 128 {
                    return Err(invalid("too many harness turns"));
                }
            }
            Some("turn_ended") => {
                self.turns.remove(&e["turn"].to_string());
                self.items.clear();
                self.error = (e["reason"]["reason"] == "error").then(|| {
                    e["reason"]["message"]
                        .as_str()
                        .unwrap_or("turn failed")
                        .into()
                });
            }
            Some("item_started") => {
                let kind = e["kind"].as_str().unwrap_or("item");
                if matches!(
                    kind,
                    "tool_call" | "terminal" | "approval" | "reasoning" | "agent_message"
                ) {
                    let name = e["head"]["name"]
                        .as_str()
                        .or_else(|| e["head"]["command"].as_str())
                        .unwrap_or(kind);
                    self.items.push((
                        e["item"].to_string(),
                        crate::observe::short(name, 160),
                        kind == "approval",
                    ));
                    if self.items.len() > 128 {
                        return Err(invalid("too many open harness items"));
                    }
                }
            }
            Some("item_completed") => {
                self.items.retain(|(id, _, _)| id != &e["item"].to_string());
            }
            _ => {}
        }
        Ok(true)
    }
    fn projection(&self) -> Value {
        let waiting = self.items.iter().rev().find(|(_, _, approval)| *approval);
        let state = if self.error.is_some() {
            "errored"
        } else if waiting.is_some() {
            "waiting"
        } else if self.turns.is_empty() {
            "idle"
        } else {
            "active"
        };
        let detail = self.error.as_deref().or_else(|| {
            waiting
                .or_else(|| self.items.last())
                .map(|(_, name, _)| name.as_str())
        });
        json!({"session":self.session,"state":state,"detail":detail,"log_seq":self.seq,"epoch":self.epoch,"observed_ms":self.observed_ms})
    }
}
use std::os::unix::fs::OpenOptionsExt;

/// Apply private IO-owner reports only to their exact process/attachment identity.
///
/// # Errors
/// Refuses stale bindings, invalid sessions and malformed projections.
pub fn observe(
    session: &mut Session,
    event: &Value,
    sequence: u64,
    _now_ms: u64,
) -> Result<Vec<Observation>> {
    let binding: Binding = serde_json::from_value(event["binding"].clone())?;
    if binding.seat != session.binding.seat
        || binding.pid != session.binding.pid
        || binding.process_start != session.binding.process_start
        || binding.generation != session.binding.generation
        || binding.attachment_generation != session.binding.attachment_generation
    {
        return Err(invalid("stale harness log binding"));
    }
    if session.status.source == "exit" {
        return Ok(Vec::new());
    }
    let id = event["session"]
        .as_str()
        .ok_or_else(|| invalid("missing harness session"))?;
    id.parse::<crate::Ulid>()?;
    if !session.binding.session.starts_with("process-")
        && session.binding.session != id
        && event["previous_session"].as_str() != Some(&session.binding.session)
    {
        return Err(invalid("harness log session changed"));
    }
    session.binding.session = id.into();
    let state = match event["state"].as_str() {
        Some("active") => State::Active,
        Some("idle") => State::Idle,
        Some("waiting") => State::Waiting,
        Some("errored") => State::Errored,
        _ => return Err(invalid("invalid harness log state")),
    };
    let observed_ms = event["observed_ms"]
        .as_u64()
        .ok_or_else(|| invalid("missing harness report time"))?;
    let mut obs = session.observation(
        sequence,
        observed_ms,
        "sessions_file",
        Grade::Reported,
        state,
    );
    obs.resolves = state != State::Waiting;
    obs.detail = event["detail"].as_str().map(str::to_owned);
    obs.waiting_kind = (state == State::Waiting).then_some(WaitingKind::Permission);
    Ok(if session.apply(obs.clone()) {
        vec![obs]
    } else {
        Vec::new()
    })
}
