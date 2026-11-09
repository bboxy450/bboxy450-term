// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Runtime of [crate::delivery::RemoteDelivery]: the local [Outbox] and the
//! remote [Inbox]. Model actions map one to one: submit = Submit, sendable =
//! Send, Inbox::arrive = Arrive (Fresh emits Apply), on_applied = AckArrive,
//! drop_link = Drop / RemoteCrash, reconnect = Reconnect.
//!
//! Abstraction recorded in verification/contracts.toml: the model makes
//! "journal the id" and "apply the turn" one atomic step. The runtime journals
//! first (fsync), then applies, so a crash between the two loses that turn and
//! the local side reports its outcome as unknown; it never applies twice.
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    AlreadySubmitted,
}

/// Local side: accepted messages until the remote acknowledges them.
#[derive(Debug, Default)]
pub struct Outbox<T> {
    unacked: BTreeMap<String, T>,
    inflight: BTreeSet<String>,
    acked: BTreeSet<String>,
    down: bool,
}

impl<T: Clone> Outbox<T> {
    pub fn new() -> Self {
        Self {
            unacked: BTreeMap::new(),
            inflight: BTreeSet::new(),
            acked: BTreeSet::new(),
            down: false,
        }
    }
    pub fn submit(&mut self, id: &str, payload: T) -> Result<(), Refused> {
        if self.unacked.contains_key(id) || self.acked.contains(id) {
            return Err(Refused::AlreadySubmitted);
        }
        self.unacked.insert(id.to_owned(), payload);
        Ok(())
    }
    /// Messages to put on the live link now; each is in flight once per link.
    pub fn sendable(&mut self) -> Vec<(String, T)> {
        if self.down {
            return Vec::new();
        }
        let fresh: Vec<_> = self
            .unacked
            .iter()
            .filter(|(id, _)| !self.inflight.contains(*id))
            .map(|(id, p)| (id.clone(), p.clone()))
            .collect();
        self.inflight.extend(fresh.iter().map(|(id, _)| id.clone()));
        fresh
    }
    /// The remote reported `Applied`; true when this retired an outbox entry.
    pub fn on_applied(&mut self, id: &str) -> bool {
        if self.down || !self.inflight.remove(id) {
            return false;
        }
        self.unacked.remove(id);
        self.acked.insert(id.to_owned());
        true
    }
    /// The link dropped or the remote crashed: nothing is in flight any more.
    pub fn drop_link(&mut self) {
        self.down = true;
        self.inflight.clear();
    }
    pub fn reconnect(&mut self) {
        self.down = false;
    }
    pub fn is_unacked(&self, id: &str) -> bool {
        self.unacked.contains_key(id)
    }
    pub fn is_acked(&self, id: &str) -> bool {
        self.acked.contains(id)
    }
}

/// The remote's durable record of applied message ids.
pub trait AppliedJournal {
    fn contains(&self, id: &str) -> bool;
    /// Persist before returning; the turn is applied only afterwards.
    fn record(&mut self, id: &str) -> io::Result<()>;
}

/// Append-only, fsynced journal: one id per line, reloaded on restart.
pub struct FileJournal {
    file: File,
    ids: BTreeSet<String>,
}

impl FileJournal {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut ids = BTreeSet::new();
        if let Ok(f) = File::open(path) {
            for line in BufReader::new(f).lines() {
                let line = line?;
                if !line.is_empty() {
                    ids.insert(line);
                }
            }
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { file, ids })
    }
}

impl AppliedJournal for FileJournal {
    fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }
    fn record(&mut self, id: &str) -> io::Result<()> {
        if id.contains('\n') || id.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "message id must be one non-empty line",
            ));
        }
        writeln!(self.file, "{id}")?;
        self.file.sync_data()?;
        self.ids.insert(id.to_owned());
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Arrival {
    /// Journalled now: apply the turn, then send `Applied`.
    Fresh,
    /// Already applied: send `Applied` again, apply nothing.
    Duplicate,
}

/// Remote side of delivery.
pub struct Inbox<J> {
    journal: J,
}

impl<J: AppliedJournal> Inbox<J> {
    pub fn new(journal: J) -> Self {
        Self { journal }
    }
    pub fn is_applied(&self, id: &str) -> bool {
        self.journal.contains(id)
    }
    pub fn arrive(&mut self, id: &str) -> io::Result<Arrival> {
        if self.journal.contains(id) {
            return Ok(Arrival::Duplicate);
        }
        self.journal.record(id)?;
        Ok(Arrival::Fresh)
    }
}
