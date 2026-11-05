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
use serde_json::{Value, json};

use crate::store::{
    Events, Result, Store,
    events::{Envelope, EventLog},
    invalid, sqlint,
    sqlite::SqliteStore,
};

const STATE: &str = "provenance/jsonl/v1";
const PART_BYTES: u64 = 64 * 1024 * 1024;
const PAGE: usize = 128;
const BUDGET: Duration = Duration::from_millis(8);

mod recovery;

/// A kernel-owned, nonblocking claim for one canonical root. Never unlink the
/// lock inode: descriptor lifetime, including a still-running blocking job,
/// owns release. Dropping an old descriptor cannot unlock a newer claim.
pub(crate) struct WriterLease {
    pub(crate) root: PathBuf,
    _file: File,
}
impl WriterLease {
    pub(crate) fn acquire(root: &Path) -> Result<Option<std::sync::Arc<Self>>> {
        let root = root.canonicalize()?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(directory(&root)?.join(".writer-lock"))?;
        if !file.metadata()?.is_file() {
            return Err(invalid("unsafe projection writer lock"));
        }
        match file.try_lock() {
            Ok(()) => Ok(Some(std::sync::Arc::new(Self { root, _file: file }))),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(error)) => Err(error.into()),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct Projection {
    cursor: u64,
    hash: String,
    files: BTreeMap<String, Part>,
    // Upstream 2e481e4f staged this cursor in STATE. Detect, never publish it.
    #[serde(default, rename = "repair", skip_serializing)]
    legacy_repair: Option<Value>,
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
    #[cfg(test)]
    pub(crate) fn cursor(&self) -> u64 {
        if self.legacy_repair.is_some() {
            0
        } else {
            self.cursor
        }
    }
    /// The physical daily part containing a canonical stream sequence.
    #[cfg(test)]
    pub(crate) fn reference(
        &self,
        envelope: &Envelope,
        store: &SqliteStore,
    ) -> Result<Option<String>> {
        if self.legacy_repair.is_some() {
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
    #[cfg(test)]
    fn valid(&self, directory: &Path, events: &EventLog) -> Result<bool> {
        Ok(self
            .invalid_at(events, |name, _| directory.join(name))?
            .is_none())
    }
    fn invalid_at(
        &self,
        events: &EventLog,
        mut path: impl FnMut(&str, usize) -> PathBuf,
    ) -> Result<Option<Value>> {
        if self.legacy_repair.is_some() {
            return Ok(Some(json!({"reason":"legacy_repair_state"})));
        }
        if self.files.len() > 4096 {
            return Ok(Some(
                json!({"reason":"part_budget", "observed":self.files.len(), "expected_max":4096}),
            ));
        }
        let head = events.last_sequence()?;
        if self.cursor > head {
            return Ok(Some(
                json!({"reason":"cursor_head_mismatch", "cursor":self.cursor, "head":head}),
            ));
        }
        if self.cursor == 0 && !self.hash.is_empty() {
            return Ok(Some(
                json!({"reason":"empty_cursor_hash_mismatch", "cursor":0}),
            ));
        }
        if self.cursor != 0 {
            let Some(row) = events.at(self.cursor)? else {
                return Ok(Some(
                    json!({"reason":"cursor_row_missing", "cursor":self.cursor}),
                ));
            };
            if row.hash()?.as_str() != self.hash {
                return Ok(Some(
                    json!({"reason":"cursor_hash_mismatch", "cursor":self.cursor}),
                ));
            }
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
                return Ok(Some(
                    json!({"reason":"invalid_part_name", "position":position}),
                ));
            }
            let expected =
                json!({"length":part.length, "modified_ns":part.modified_ns.to_string()});
            let observed = match stamp(&path(name, position)) {
                Ok((length, modified_ns)) => (length, modified_ns),
                Err(error) => {
                    let reason = if error
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
                    {
                        "file_missing"
                    } else {
                        "file_unavailable"
                    };
                    return Ok(Some(
                        json!({"reason":reason, "part":name, "expected":expected}),
                    ));
                }
            };
            let reason = if observed.0 != part.length {
                "file_length_mismatch"
            } else if observed.1 != part.modified_ns {
                "file_mtime_mismatch"
            } else {
                continue;
            };
            return Ok(Some(
                json!({"reason":reason, "part":name, "expected":expected,
                "observed":{"length":observed.0, "modified_ns":observed.1.to_string()}}),
            ));
        }
        Ok(None)
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

/// Adapted from bboxy450-term 2e481e4f projection batching; keep file contents
/// durable before directory entries and the corresponding SQLite cursor.
fn sync_page(directory: &Path, touched: BTreeSet<PathBuf>) -> Result<()> {
    for path in touched {
        File::open(path)?.sync_all()?;
    }
    File::open(directory)?.sync_all()?;
    Ok(())
}

/// Append at most one bounded page; callers keep their cursor and yield between
/// pages, including during verification/recovery. A crash after an append but
/// before the cursor commit restarts the damaged page's temporary projection.
pub(crate) fn step(root: &Path, store: &SqliteStore, events: &EventLog) -> Result<Projection> {
    let directory = directory(root)?;
    if store.get(recovery::REBUILD)?.is_some() || store.get(recovery::STARTING)?.is_some() {
        return recovery::step(root, store, events);
    }
    let Some(bytes) = store.get(STATE)? else {
        return recovery::enter(root, store, events, json!({"reason":"state_missing"}));
    };
    let Ok(mut state) = serde_json::from_slice::<Projection>(&bytes) else {
        return recovery::enter(root, store, events, json!({"reason":"state_invalid"}));
    };
    if let Some(reason) = state.invalid_at(events, |name, _| directory.join(name))? {
        return recovery::enter(root, store, events, reason);
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
                    let day = day(store, row.event.ts)?;
                    let part = if state.files.values().any(|part| part.day == day) {
                        format!("{day}.part-{}.jsonl", row.seq)
                    } else {
                        format!("{day}.jsonl")
                    };
                    return recovery::enter(
                        root,
                        store,
                        events,
                        json!({"reason":"append_already_exists", "sequence":row.seq, "part":part}),
                    );
                }
                return Err(error);
            }
        }
        if started.elapsed() >= BUDGET {
            break;
        }
    }
    sync_page(&directory, touched)?;
    store.put(STATE, &serde_json::to_vec(&state)?)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::events::Draft;
    use ed25519_dalek::SigningKey;
    use serde_json::json;

    // Adapted from bboxy450-term 2e481e4f: same yield/restart/signature/visibility
    // properties, reading the canonical separate rebuild record.
    fn catch_up(root: &Path, store: &SqliteStore, events: &EventLog) -> Projection {
        for _ in 0..1024 {
            let state = step(root, store, events).unwrap();
            if store.get(recovery::REBUILD).unwrap().is_none()
                && state.cursor() == events.last_sequence().unwrap()
            {
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
        for _ in 0..(recovery::RECOVERY_PAGE * 3 + 1) {
            last = Some(
                events
                    .append(Draft::new("fixture", "test", json!({})), &key)
                    .unwrap(),
            );
        }
        let first = step(&root, &store, &events).unwrap();
        let pending = |store: &SqliteStore| -> u64 {
            let bytes = store.get(recovery::REBUILD).unwrap().unwrap();
            serde_json::from_slice::<Value>(&bytes).unwrap()["projection"]["cursor"]
                .as_u64()
                .unwrap()
        };
        assert!((1..=recovery::RECOVERY_PAGE as u64).contains(&pending(&store)));
        assert_eq!(first.cursor(), 0);
        assert!(
            first
                .reference(last.as_ref().unwrap(), &store)
                .unwrap()
                .is_none()
        );
        use std::os::unix::fs::MetadataExt;
        let temporary = root.join("log/.projection-rebuild-v1");
        let directory = std::fs::metadata(&temporary).unwrap().ino();
        let cursor = pending(&store);
        drop(store);
        drop(events);
        let store = SqliteStore::open(&index).unwrap();
        let events = EventLog::open(&index).unwrap();
        let resumed = step(&root, &store, &events).unwrap();
        assert_eq!(std::fs::metadata(&temporary).unwrap().ino(), directory);
        assert_eq!(resumed.cursor(), 0);
        assert!(
            pending(&store) > cursor && pending(&store) <= cursor + recovery::RECOVERY_PAGE as u64
        );
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
        assert_eq!(count, (recovery::RECOVERY_PAGE * 3 + 1) as u64);
        assert!(!temporary.exists());
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
