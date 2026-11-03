// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Durable, bounded recovery. The request owner never walks history in a turn.
use super::*;
use std::time::{Duration, Instant};

pub(super) const REBUILD: &str = "provenance/jsonl/rebuild/v1";
const TEMPORARY: &str = ".projection-rebuild-v1";
const RECOVERY_PAGE: usize = 16;
const BUDGET: Duration = Duration::from_millis(8);

#[derive(Serialize, Deserialize)]
struct Recovery {
    head: u64,
    projection: Projection,
    published: usize,
}

impl Recovery {
    fn valid(&self, directory: &Path, events: &EventLog) -> Result<bool> {
        let temporary = directory.join(TEMPORARY);
        if !temporary
            .symlink_metadata()
            .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
            || self.projection.cursor > self.head
            || self.head > events.last_sequence()?
            || self.published > self.projection.files.len()
            || (self.published != 0 && self.projection.cursor != self.head)
        {
            return Ok(false);
        }
        self.projection.valid_at(events, |name, position| {
            if position < self.published {
                directory.join(name)
            } else {
                temporary.join(name)
            }
        })
    }

    fn start(directory: &Path, events: &EventLog) -> Result<Option<Self>> {
        let temporary = directory.join(TEMPORARY);
        if let Ok(metadata) = temporary.symlink_metadata() {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(invalid("unsafe activity recovery directory"));
            }
            // Even discarding an interrupted rebuild is paged. Never walk all
            // retained parts in a single request-owner turn.
            let mut removed = false;
            for entry in std::fs::read_dir(&temporary)?.take(RECOVERY_PAGE) {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    return Err(invalid("unsafe activity recovery entry"));
                }
                std::fs::remove_file(entry.path())?;
                removed = true;
            }
            if removed {
                return Ok(None);
            }
            std::fs::remove_dir(&temporary)?;
        }
        std::fs::create_dir(&temporary)?;
        File::open(directory)?.sync_all()?;
        Ok(Some(Self {
            head: events.last_sequence()?,
            projection: Projection::default(),
            published: 0,
        }))
    }
}

/// Each call verifies at most 16 canonical rows, then appends until 8 ms has
/// elapsed (at least one row), or publishes at most 16 completed part files.
/// There is no joined background job: shutdown abandons a durable page cursor.
pub(super) fn step(root: &Path, store: &SqliteStore, events: &EventLog) -> Result<Projection> {
    let directory = directory(root)?;
    let saved = store.get(REBUILD)?;
    let saved = saved.and_then(|bytes| serde_json::from_slice::<Recovery>(&bytes).ok());
    let mut recovery = match saved {
        Some(saved) if saved.valid(&directory, events)? => saved,
        _ => {
            let Some(state) = Recovery::start(&directory, events)? else {
                return Ok(Projection::default());
            };
            store.put(REBUILD, &serde_json::to_vec(&state)?)?;
            state
        }
    };
    let temporary = directory.join(TEMPORARY);
    let started = Instant::now();
    if recovery.projection.cursor < recovery.head {
        // Verify SQL sequence/hash/prev AND envelope signature in the same
        // bounded read. This replaces recover's separate full-chain scan.
        for _ in 0..RECOVERY_PAGE {
            if recovery.projection.cursor >= recovery.head {
                break;
            }
            // Fetch only the row that this turn will consume. Pre-verifying
            // a whole page and then yielding after its first fsync would
            // repeatedly verify the discarded suffix on every short turn.
            let row = events
                .verified_page(recovery.projection.cursor, &recovery.projection.hash, 1)?
                .pop()
                .ok_or_else(|| invalid("activity projection canonical rows unavailable"))?;
            recovery
                .projection
                .append(&temporary, store, &row, PART_BYTES)?;
            if started.elapsed() >= BUDGET {
                break;
            }
        }
        File::open(&temporary)?.sync_all()?;
        store.put(REBUILD, &serde_json::to_vec(&recovery)?)?;
        if recovery.projection.cursor < recovery.head {
            // No partial rebuild is advertised as a published projection.
            return Ok(Projection::default());
        }
    }
    let mut count = 0;
    for (name, part) in recovery
        .projection
        .files
        .iter_mut()
        .skip(recovery.published)
        .take(RECOVERY_PAGE)
    {
        let destination = directory.join(name);
        if destination
            .symlink_metadata()
            .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err(invalid("unsafe activity projection replacement"));
        }
        std::fs::rename(temporary.join(name), &destination)?;
        (part.length, part.modified_ns) = stamp(&destination)?;
        count += 1;
        if started.elapsed() >= BUDGET {
            break;
        }
    }
    recovery.published += count;
    File::open(&directory)?.sync_all()?;
    if recovery.published < recovery.projection.files.len() {
        store.put(REBUILD, &serde_json::to_vec(&recovery)?)?;
        return Ok(Projection::default());
    }
    // All signatures and chain links passed before the first rename. As in
    // the original repair, each replacement is atomic; a crash before this
    // cursor commit causes an interrupted publication to resume or restart.
    store.put(STATE, &serde_json::to_vec(&recovery.projection)?)?;
    store
        .connection()?
        .execute("DELETE FROM kv WHERE key=?1", [REBUILD])?;
    std::fs::remove_dir(&temporary)?;
    Ok(recovery.projection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::events::Draft;
    use ed25519_dalek::SigningKey;

    struct Fixture {
        root: PathBuf,
        store: SqliteStore,
        events: EventLog,
        _temp: tempfile::TempDir,
    }
    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join(".bboxy");
            std::fs::create_dir(&root).unwrap();
            let store = SqliteStore::open(&temp.path().join("index")).unwrap();
            let events = EventLog::open(&temp.path().join("index")).unwrap();
            let key = SigningKey::from_bytes(&[78; 32]);
            for n in 0..40 {
                events
                    .append(
                        Draft::new("fixture/row", "fixture", serde_json::json!(n)),
                        &key,
                    )
                    .unwrap();
            }
            Self {
                root,
                store,
                events,
                _temp: temp,
            }
        }
        fn finish(&self) -> Projection {
            for _ in 0..200 {
                let projection = step(&self.root, &self.store, &self.events).unwrap();
                if projection.cursor == 40 {
                    return projection;
                }
            }
            panic!("recovery did not make progress");
        }
        fn assert_rows(&self, projection: &Projection) {
            let rows: Vec<Envelope> = projection
                .files
                .keys()
                .flat_map(|name| {
                    std::fs::read_to_string(self.root.join("log").join(name))
                        .unwrap()
                        .lines()
                        .map(|line| serde_json::from_str(line).unwrap())
                        .collect::<Vec<_>>()
                })
                .collect();
            assert_eq!(rows, self.events.after(0, 40).unwrap());
            assert!(
                projection
                    .valid(&self.root.join("log"), &self.events)
                    .unwrap()
            );
        }
    }

    // keeps: spec provenance
    #[test]
    fn recovery_resumes_a_durable_page_and_restarts_a_torn_page() {
        let f = Fixture::new();
        assert_eq!(step(&f.root, &f.store, &f.events).unwrap().cursor, 0);
        let saved: Recovery =
            serde_json::from_slice(&f.store.get(REBUILD).unwrap().unwrap()).unwrap();
        assert!(saved.projection.cursor > 0 && saved.projection.cursor <= RECOVERY_PAGE as u64);
        assert!(f.store.get(STATE).unwrap().is_none());
        // New store handles stand in for a restart, with no in-memory cursor.
        let reopened = SqliteStore::open(f.store.path.parent().unwrap()).unwrap();
        step(&f.root, &reopened, &f.events).unwrap();
        let next: Recovery =
            serde_json::from_slice(&reopened.get(REBUILD).unwrap().unwrap()).unwrap();
        assert!(next.projection.cursor > saved.projection.cursor);
        let file = f
            .root
            .join("log")
            .join(TEMPORARY)
            .join(next.projection.files.keys().next().unwrap());
        OpenOptions::new()
            .append(true)
            .open(file)
            .unwrap()
            .write_all(b"{torn")
            .unwrap();
        let projection = f.finish();
        f.assert_rows(&projection);
        assert!(f.store.get(REBUILD).unwrap().is_none());
    }

    // keeps: spec provenance
    #[test]
    fn recovery_checks_sql_hashes_before_publishing_any_history() {
        let f = Fixture::new();
        let db = rusqlite::Connection::open(&f.events.path).unwrap();
        db.execute_batch(
            "DROP TRIGGER immutable_update; UPDATE events SET hash='tampered' WHERE seq=20",
        )
        .unwrap();
        let mut refused = false;
        for _ in 0..40 {
            match step(&f.root, &f.store, &f.events) {
                Err(_) => {
                    refused = true;
                    break;
                }
                Ok(projection) => assert_eq!(projection.cursor, 0),
            }
        }
        assert!(refused);
        assert!(f.store.get(STATE).unwrap().is_none());
        assert!(std::fs::read_dir(f.root.join("log")).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".jsonl")
        }));
    }

    // keeps: spec provenance
    #[test]
    fn recovery_refuses_a_symlink_in_its_reserved_temporary_path() {
        let f = Fixture::new();
        std::fs::create_dir(f.root.join("log")).unwrap();
        let outside = f._temp.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, f.root.join("log").join(TEMPORARY)).unwrap();
        assert!(step(&f.root, &f.store, &f.events).is_err());
        assert_eq!(std::fs::read_dir(outside).unwrap().count(), 0);
    }

    // keeps: spec provenance
    #[test]
    fn an_uncommitted_part_replacement_restarts_without_duplicate_rows() {
        let f = Fixture::new();
        let directory = directory(&f.root).unwrap();
        let mut recovery = Recovery::start(&directory, &f.events).unwrap().unwrap();
        for row in f.events.after(0, 40).unwrap() {
            recovery
                .projection
                .append(&directory.join(TEMPORARY), &f.store, &row, 1)
                .unwrap();
        }
        f.store
            .put(REBUILD, &serde_json::to_vec(&recovery).unwrap())
            .unwrap();
        // Simulate death after a rename and before its durable progress update.
        let first = recovery.projection.files.keys().next().unwrap();
        std::fs::rename(directory.join(TEMPORARY).join(first), directory.join(first)).unwrap();
        let projection = f.finish();
        f.assert_rows(&projection);
        assert!(!directory.join(TEMPORARY).exists());
    }

    // keeps: spec provenance
    #[test]
    fn invalid_state_and_append_collision_take_the_paged_recovery_path() {
        for collision in [false, true] {
            let f = Fixture::new();
            if collision {
                f.store
                    .put(STATE, &serde_json::to_vec(&Projection::default()).unwrap())
                    .unwrap();
                let directory = directory(&f.root).unwrap();
                let row = f.events.at(1).unwrap().unwrap();
                let day = day(&f.store, row.event.ts).unwrap();
                std::fs::write(directory.join(format!("{day}.jsonl")), b"{partial").unwrap();
            } else {
                f.store.put(STATE, b"{invalid-json").unwrap();
            }
            assert_eq!(
                super::super::step(&f.root, &f.store, &f.events)
                    .unwrap()
                    .cursor,
                0
            );
            assert!(f.store.get(REBUILD).unwrap().is_some());
            f.assert_rows(&f.finish());
        }
    }
}
