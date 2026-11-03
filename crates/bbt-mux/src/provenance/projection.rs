// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! JSONL is a recoverable view of the signed SQLite stream, never its authority.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
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
const REPAIR_PAGE: usize = 128;
const BUDGET: Duration = Duration::from_millis(8);

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Projection {
    cursor: u64,
    hash: String,
    files: BTreeMap<String, Part>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    repair: Option<Repair>,
}
#[derive(Serialize, Deserialize)]
struct Repair {
    directory: crate::Ulid,
    head: u64,
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
        if self.repair.is_some() {
            0
        } else {
            self.cursor
        }
    }
    /// The physical daily part containing a canonical stream sequence.
    pub(crate) fn reference(
        &self,
        envelope: &Envelope,
        store: &SqliteStore,
    ) -> Result<Option<String>> {
        // Staged files are not published references until the whole repair is durable.
        if self.repair.is_some() {
            return Ok(None);
        }
        let day = day(store, envelope.event.ts)?;
        let sequence = envelope.seq;
        Ok(self
            .files
            .iter()
            .find(|(_, part)| part.day == day && part.first <= sequence && sequence <= part.last)
            .map(|(name, _)| format!(".bboxy/log/{name}#{sequence}")))
    }
    fn valid(&self, directory: &Path, events: &EventLog) -> Result<bool> {
        if self.cursor > events.last_sequence()? {
            return Ok(false);
        }
        if self.cursor != 0
            && events
                .at(self.cursor)?
                .is_none_or(|row| row.hash().ok().as_deref() != Some(&self.hash))
        {
            return Ok(false);
        }
        for (name, part) in &self.files {
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
            if stamp(&directory.join(name)).ok() != Some((part.length, part.modified_ns)) {
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
    ) -> Result<PathBuf> {
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
        Ok(path)
    }
}

/// Start a bounded repair from canonical bytes. The staged cursor is durable,
/// so subsequent logic turns (and restarts) continue instead of blocking the owner.
pub(crate) fn recover(root: &Path, store: &SqliteStore, events: &EventLog) -> Result<Projection> {
    let directory = directory(root)?;
    let id = crate::Ulid::new();
    let temporary = directory.join(format!(".projection-{id}"));
    std::fs::create_dir(&temporary)?;
    let state = Projection {
        repair: Some(Repair {
            directory: id,
            head: events.last_sequence()?,
        }),
        ..Projection::default()
    };
    repair_page(&directory, &temporary, store, events, state)
}

fn repair_page(
    directory: &Path,
    temporary: &Path,
    store: &SqliteStore,
    events: &EventLog,
    mut state: Projection,
) -> Result<Projection> {
    let head = state.repair.as_ref().unwrap().head;
    if state.cursor < head {
        let started = Instant::now();
        let rows = events.after(state.cursor, REPAIR_PAGE)?;
        let mut touched = BTreeSet::new();
        if rows.is_empty() {
            return Err(invalid("activity projection canonical rows unavailable"));
        }
        for row in rows.into_iter().take_while(|row| row.seq <= head) {
            // append verifies both the signature and the exact sequence/hash link.
            touched.insert(state.append(temporary, store, &row, PART_BYTES)?);
            if started.elapsed() >= BUDGET {
                break;
            }
        }
        // Staged files are synced once per page before the cursor is committed.
        for path in touched {
            File::open(path)?.sync_all()?;
        }
    }
    if state.cursor == head {
        for (name, part) in &mut state.files {
            let destination = directory.join(name);
            if destination
                .symlink_metadata()
                .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
            {
                return Err(invalid("unsafe activity projection replacement"));
            }
            std::fs::rename(temporary.join(name), &destination)?;
            (part.length, part.modified_ns) = stamp(&destination)?;
        }
        File::open(directory)?.sync_all()?;
        state.repair = None;
        std::fs::remove_dir(temporary)?;
    }
    store.put(STATE, &serde_json::to_vec(&state)?)?;
    Ok(state)
}

/// Append at most one bounded page; callers keep their cursor and yield between
/// pages. A crash after an append but before the cursor commit forces repair.
pub(crate) fn step(root: &Path, store: &SqliteStore, events: &EventLog) -> Result<Projection> {
    let directory = directory(root)?;
    let Some(bytes) = store.get(STATE)? else {
        return recover(root, store, events);
    };
    let mut state: Projection = serde_json::from_slice(&bytes)?;
    if let Some(repair) = &state.repair {
        let temporary = directory.join(format!(".projection-{}", repair.directory));
        let safe = temporary
            .symlink_metadata()
            .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink());
        if safe
            && state.cursor <= repair.head
            && repair.head <= events.last_sequence()?
            && state.valid(&temporary, events)?
        {
            return repair_page(&directory, &temporary, store, events, state);
        }
        return recover(root, store, events);
    }
    if !state.valid(&directory, events)? {
        return recover(root, store, events);
    }
    let rows = events.after(state.cursor, PAGE)?;
    if rows.is_empty() {
        return Ok(state);
    }
    let started = Instant::now();
    let mut touched = BTreeSet::new();
    for row in rows {
        match state.append(&directory, store, &row, PART_BYTES) {
            Ok(path) => {
                touched.insert(path);
            }
            Err(error) => {
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists)
                {
                    return recover(root, store, events);
                }
                return Err(error);
            }
        }
        if started.elapsed() >= BUDGET {
            break;
        }
    }
    for path in touched {
        File::open(path)?.sync_all()?;
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

    fn catch_up(root: &Path, store: &SqliteStore, events: &EventLog) -> Projection {
        for _ in 0..1024 {
            let state = step(root, store, events).unwrap();
            if state.repair.is_none() && state.cursor() == events.last_sequence().unwrap() {
                return state;
            }
        }
        panic!("projection did not drain");
    }

    #[test]
    fn repair_yields_and_resumes_its_durable_cursor_before_publishing() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        std::fs::create_dir(&root).unwrap();
        let index = temp.path().join("index");
        let store = SqliteStore::open(&index).unwrap();
        let events = EventLog::open(&index).unwrap();
        let key = SigningKey::from_bytes(&[72; 32]);
        let mut last = None;
        for _ in 0..(REPAIR_PAGE * 3) {
            last = Some(
                events
                    .append(Draft::new("fixture", "test", json!({})), &key)
                    .unwrap(),
            );
        }
        let first = step(&root, &store, &events).unwrap();
        assert!((1..=REPAIR_PAGE as u64).contains(&first.cursor));
        assert_eq!(first.cursor(), 0);
        assert!(
            first
                .reference(last.as_ref().unwrap(), &store)
                .unwrap()
                .is_none()
        );
        let directory = first.repair.as_ref().unwrap().directory;
        let cursor = first.cursor;
        drop(store);
        drop(events);
        let store = SqliteStore::open(&index).unwrap();
        let events = EventLog::open(&index).unwrap();
        let resumed = step(&root, &store, &events).unwrap();
        assert_eq!(resumed.repair.as_ref().unwrap().directory, directory);
        assert!(resumed.cursor > cursor && resumed.cursor <= cursor + REPAIR_PAGE as u64);
        let complete = catch_up(&root, &store, &events);
        assert!(
            complete
                .reference(last.as_ref().unwrap(), &store)
                .unwrap()
                .is_some()
        );
        let mut count = 0;
        for name in complete.files.keys() {
            let text = std::fs::read_to_string(root.join("log").join(name)).unwrap();
            for line in text.lines() {
                let row: Envelope = serde_json::from_str(line).unwrap();
                row.verify().unwrap();
                count += 1;
                assert_eq!(row.seq, count);
            }
        }
        assert_eq!(count, (REPAIR_PAGE * 3) as u64);
        assert!(
            !root
                .join("log")
                .join(format!(".projection-{directory}"))
                .exists()
        );
    }

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
        let state = catch_up(&root, &store, &events);
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
        catch_up(&root, &store, &events);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(&original)
            .unwrap();
        catch_up(&root, &store, &events);
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
        catch_up(&root, &store, &events);
        let text = std::fs::read_to_string(&path).unwrap();
        let rows: Vec<Envelope> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows, vec![first, second]);
        for row in rows {
            row.verify().unwrap();
        }
        recover(&root, &store, &events).unwrap();
        catch_up(&root, &store, &events);
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
        assert!(recover(&root, &store, &events).is_err());
        assert_eq!(std::fs::read_dir(outside).unwrap().count(), 0);
    }
}
