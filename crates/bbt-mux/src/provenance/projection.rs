// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! JSONL is a recoverable view of the signed SQLite stream, never its authority.
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::store::{
    Events, Result, Store,
    events::{Envelope, EventLog},
    invalid, sqlint,
    sqlite::SqliteStore,
};

const STATE: &str = "provenance/jsonl/v1";
const PART_BYTES: u64 = 64 * 1024 * 1024;
const PAGE: usize = 128;

mod recovery;

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Projection {
    cursor: u64,
    hash: String,
    files: BTreeMap<String, Part>,
}
#[derive(Serialize, Deserialize)]
struct Part {
    day: String,
    first: u64,
    last: u64,
    length: u64,
    modified_ns: u128,
}

fn directory(root: &Path) -> Result<PathBuf> {
    let directory = root.join("log");
    if directory
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err(invalid("symlink in activity projection directory"));
    }
    std::fs::create_dir_all(&directory)?;
    if !directory.canonicalize()?.starts_with(root.canonicalize()?) {
        return Err(invalid("activity projection escapes index root"));
    }
    Ok(directory)
}
fn stamp(path: &Path) -> Result<(u64, u128)> {
    let metadata = path.symlink_metadata()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(invalid("activity projection must be a regular file"));
    }
    Ok((
        metadata.len(),
        metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos(),
    ))
}
fn day(store: &SqliteStore, timestamp: u64) -> Result<String> {
    let day: String = store.connection()?.query_row(
        "SELECT date(?1 / 1000,'unixepoch')",
        [sqlint(timestamp)?],
        |row| row.get(0),
    )?;
    if day.len() != 10
        || day.bytes().enumerate().any(|(i, b)| {
            if matches!(i, 4 | 7) {
                b != b'-'
            } else {
                !b.is_ascii_digit()
            }
        })
    {
        return Err(invalid("activity timestamp is outside the UTC date bound"));
    }
    Ok(day)
}

impl Projection {
    pub(crate) fn cursor(&self) -> u64 {
        self.cursor
    }
    /// The physical daily part containing a canonical stream sequence.
    pub(crate) fn reference(
        &self,
        envelope: &Envelope,
        store: &SqliteStore,
    ) -> Result<Option<String>> {
        let day = day(store, envelope.event.ts)?;
        let sequence = envelope.seq;
        Ok(self
            .files
            .iter()
            .find(|(_, part)| part.day == day && part.first <= sequence && sequence <= part.last)
            .map(|(name, _)| format!(".bboxy/log/{name}#{sequence}")))
    }
    fn valid(&self, directory: &Path, events: &EventLog) -> Result<bool> {
        self.valid_at(events, |name, _| directory.join(name))
    }
    fn valid_at(
        &self,
        events: &EventLog,
        mut path: impl FnMut(&str, usize) -> PathBuf,
    ) -> Result<bool> {
        if self.files.len() > 4096 {
            return Ok(false);
        }
        if (self.cursor == 0 && !self.hash.is_empty()) || self.cursor > events.last_sequence()? {
            return Ok(false);
        }
        if self.cursor != 0
            && events
                .at(self.cursor)?
                .is_none_or(|row| row.hash().ok().as_deref() != Some(&self.hash))
        {
            return Ok(false);
        }
        for (position, (name, part)) in self.files.iter().enumerate() {
            let expected = format!("{}.jsonl", part.day);
            let split = format!("{}.part-{}.jsonl", part.day, part.first);
            if part.day.len() != 10
                || part.day.bytes().enumerate().any(|(i, b)| {
                    if matches!(i, 4 | 7) {
                        b != b'-'
                    } else {
                        !b.is_ascii_digit()
                    }
                })
                || (name != &expected && name != &split)
            {
                return Ok(false);
            }
            if stamp(&path(name, position)).ok() != Some((part.length, part.modified_ns)) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn append(
        &mut self,
        directory: &Path,
        store: &SqliteStore,
        envelope: &Envelope,
        limit: u64,
    ) -> Result<()> {
        envelope.verify()?;
        if envelope.seq != self.cursor + 1 || envelope.prev != self.hash {
            return Err(invalid("activity projection sequence or hash gap"));
        }
        let day = day(store, envelope.event.ts)?;
        let mut bytes = serde_json::to_vec(envelope)?;
        bytes.push(b'\n');
        let previous = self
            .files
            .iter()
            .filter(|(_, part)| part.day == day)
            .max_by_key(|(_, part)| part.last)
            .map(|(name, part)| (name.clone(), part.length));
        let (name, new) = match previous {
            Some((name, length)) if length + bytes.len() as u64 <= limit => (name, false),
            Some(_) => (format!("{day}.part-{}.jsonl", envelope.seq), true),
            None => (format!("{day}.jsonl"), true),
        };
        if new && self.files.len() >= 4096 {
            return Err(invalid("activity projection part budget exceeded"));
        }
        let path = directory.join(&name);
        let mut file = OpenOptions::new()
            .write(true)
            .append(!new)
            .create_new(new)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)?;
        if !file.metadata()?.is_file() {
            return Err(invalid("activity projection is not a regular file"));
        }
        file.write_all(&bytes)?;
        file.sync_all()?;
        let (length, modified_ns) = stamp(&path)?;
        self.files
            .entry(name)
            .and_modify(|part| {
                part.last = envelope.seq;
                part.length = length;
                part.modified_ns = modified_ns;
            })
            .or_insert(Part {
                day,
                first: envelope.seq,
                last: envelope.seq,
                length,
                modified_ns,
            });
        self.cursor = envelope.seq;
        self.hash = envelope.hash()?;
        Ok(())
    }
}

/// Append at most one bounded page; callers keep their cursor and yield between
/// pages, including during verification/recovery. A crash after an append but
/// before the cursor commit restarts the damaged page's temporary projection.
pub(crate) fn step(root: &Path, store: &SqliteStore, events: &EventLog) -> Result<Projection> {
    let directory = directory(root)?;
    if store.get(recovery::REBUILD)?.is_some() {
        return recovery::step(root, store, events);
    }
    let Some(bytes) = store.get(STATE)? else {
        return recovery::step(root, store, events);
    };
    let Ok(mut state) = serde_json::from_slice::<Projection>(&bytes) else {
        return recovery::step(root, store, events);
    };
    if !state.valid(&directory, events)? {
        return recovery::step(root, store, events);
    }
    let rows = events.after(state.cursor, PAGE)?;
    if rows.is_empty() {
        return Ok(state);
    }
    for row in rows {
        if let Err(error) = state.append(&directory, store, &row, PART_BYTES) {
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists)
            {
                return recovery::step(root, store, events);
            }
            return Err(error);
        }
    }
    store.put(STATE, &serde_json::to_vec(&state)?)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::events::Draft;
    use ed25519_dalek::SigningKey;
    use serde_json::json;

    // keeps: spec provenance
    #[test]
    fn jsonl_recovers_partial_and_duplicate_rows_without_resigning_history() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        std::fs::create_dir(&root).unwrap();
        let store = SqliteStore::open(&temp.path().join("index")).unwrap();
        let events = EventLog::open(&temp.path().join("index")).unwrap();
        let key = SigningKey::from_bytes(&[71; 32]);
        let first = events
            .append(
                Draft::new(
                    "artifact/fixture/created",
                    "filesystem",
                    json!({"digest":"fixture"}),
                ),
                &key,
            )
            .unwrap();
        let state = step(&root, &store, &events).unwrap();
        let reference = state.reference(&first, &store).unwrap().unwrap();
        let name = reference
            .strip_prefix(".bboxy/log/")
            .unwrap()
            .split('#')
            .next()
            .unwrap();
        let path = root.join("log").join(name);
        let original = std::fs::read(&path).unwrap();
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"{partial")
            .unwrap();
        step(&root, &store, &events).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), original);
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(&original)
            .unwrap();
        step(&root, &store, &events).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let second = events
            .append(
                Draft::new(
                    "artifact/fixture/modified",
                    "filesystem",
                    json!({"digest":"changed"}),
                ),
                &key,
            )
            .unwrap();
        step(&root, &store, &events).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let rows: Vec<Envelope> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows, vec![first, second]);
        for row in rows {
            row.verify().unwrap();
        }
        store
            .connection()
            .unwrap()
            .execute("DELETE FROM kv WHERE key=?1", [STATE])
            .unwrap();
        step(&root, &store, &events).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), text);
    }
    // keeps: spec provenance
    #[test]
    fn projection_refuses_symlink_destinations() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        std::fs::create_dir(&root).unwrap();
        let outside = temp.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("log")).unwrap();
        let store = SqliteStore::open(&temp.path().join("index")).unwrap();
        let events = EventLog::open(&temp.path().join("index")).unwrap();
        assert!(step(&root, &store, &events).is_err());
        assert_eq!(std::fs::read_dir(outside).unwrap().count(), 0);
    }
}
