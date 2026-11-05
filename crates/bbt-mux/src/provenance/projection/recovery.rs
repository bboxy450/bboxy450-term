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
pub(super) const STARTING: &str = "provenance/jsonl/recovery-starting/v1";
const TRIGGER: &str = "provenance/jsonl/recovery-trigger/v1";
const TEMPORARY: &str = ".projection-rebuild-v1";
// Amortize a durable sync across a bounded page on the background worker.
pub(super) const RECOVERY_PAGE: usize = 256;
const BUDGET: Duration = Duration::from_millis(32);

#[derive(Serialize, Deserialize)]
struct Recovery {
    head: u64,
    projection: Projection,
    published: usize,
}

impl Recovery {
    fn invalid_reason(&self, directory: &Path, events: &EventLog) -> Result<Option<Value>> {
        let temporary = directory.join(TEMPORARY);
        if !temporary
            .symlink_metadata()
            .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        {
            return Ok(Some(
                json!({"reason":"recovery_directory_missing_or_invalid"}),
            ));
        }
        let head = events.last_sequence()?;
        if self.projection.cursor > self.head || self.head > head {
            return Ok(Some(json!({"reason":"recovery_cursor_head_mismatch",
                "cursor":self.projection.cursor, "recovery_head":self.head, "head":head})));
        }
        if self.published > self.projection.files.len()
            || (self.published != 0 && self.projection.cursor != self.head)
        {
            return Ok(Some(json!({"reason":"recovery_publication_invalid",
                "published":self.published, "parts":self.projection.files.len()})));
        }
        self.projection.invalid_at(events, |name, position| {
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

/// Persist and emit one compact trigger per transition, including interrupted
/// rebuild cleanup. No document/envelope contents or untrusted hash bytes.
pub(super) fn enter(
    root: &Path,
    store: &SqliteStore,
    events: &EventLog,
    reason: Value,
) -> Result<Projection> {
    if store.get(STARTING)?.is_none() {
        let mut reason = reason;
        reason["root_id"] = json!(crate::store::digest(
            root.canonicalize()?.as_os_str().as_encoded_bytes()
        ));
        reason["transition"] = json!(ulid::Ulid::new().to_string());
        reason["observed_ms"] = json!(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis()
        );
        let bytes = serde_json::to_vec(&reason)?;
        let recorded = {
            let mut connection = store.connection()?;
            let transaction = connection.transaction()?;
            // Owner and admitted provider IO can encounter the same damage.
            // Only one caller records the transition before paged cleanup.
            let recorded = transaction.execute(
                "INSERT INTO kv VALUES(?1,?2) ON CONFLICT(key) DO NOTHING",
                rusqlite::params![STARTING, bytes],
            )? != 0;
            if recorded {
                transaction.execute(
                    "INSERT INTO kv VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    rusqlite::params![TRIGGER, bytes],
                )?;
            }
            transaction.commit()?;
            recorded
        };
        if recorded {
            eprintln!("ProjectionRecovery: {}", String::from_utf8_lossy(&bytes));
        }
    }
    step(root, store, events)
}

/// Each call verifies at most 256 canonical rows, then appends until 32 ms has
/// elapsed (at least one row), or publishes at most 256 completed part files.
/// There is no joined background job: shutdown abandons a durable page cursor.
pub(super) fn step(root: &Path, store: &SqliteStore, events: &EventLog) -> Result<Projection> {
    let directory = directory(root)?;
    let saved = store.get(REBUILD)?;
    let starting = store.get(STARTING)?.is_some();
    let saved = match saved {
        Some(bytes) if !starting => match serde_json::from_slice::<Recovery>(&bytes) {
            Ok(saved) => {
                if let Some(mut reason) = saved.invalid_reason(&directory, events)? {
                    reason["phase"] = json!("recovery");
                    return enter(root, store, events, reason);
                }
                Some(saved)
            }
            Err(_) => {
                return enter(
                    root,
                    store,
                    events,
                    json!({"reason":"recovery_state_invalid"}),
                );
            }
        },
        _ => None,
    };
    let mut recovery = match saved {
        Some(saved) => saved,
        None => {
            let Some(state) = Recovery::start(&directory, events)? else {
                return Ok(Projection::default());
            };
            store.put(REBUILD, &serde_json::to_vec(&state)?)?;
            store
                .connection()?
                .execute("DELETE FROM kv WHERE key=?1", [STARTING])?;
            state
        }
    };
    let temporary = directory.join(TEMPORARY);
    let started = Instant::now();
    if recovery.projection.cursor < recovery.head {
        // Verify SQL sequence/hash/prev AND envelope signature in the same
        // bounded read. This replaces recover's separate full-chain scan.
        let mut touched = BTreeSet::new();
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
            touched.insert(
                recovery
                    .projection
                    .append(&temporary, store, &row, PART_BYTES)?,
            );
            if started.elapsed() >= BUDGET {
                break;
            }
        }
        sync_page(&temporary, touched)?;
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
    File::open(&temporary)?.sync_all()?;
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
    const ROWS: usize = RECOVERY_PAGE * 3 + 1;
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
            for n in 0..ROWS {
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
                if projection.cursor == ROWS as u64 {
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
            assert_eq!(rows, self.events.after(0, ROWS).unwrap());
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
    fn legacy_state_repair_restarts_staging_and_interrupted_publication() {
        for (staged, published) in [(16, 0), (ROWS, 7)] {
            let mut f = Fixture::new();
            let directory = directory(&f.root).unwrap();
            let legacy = directory.join(format!(".projection-{}", crate::Ulid::new()));
            std::fs::create_dir(&legacy).unwrap();
            let mut projection = Projection::default();
            let mut touched = BTreeSet::new();
            for row in f.events.after(0, staged).unwrap() {
                touched.insert(projection.append(&legacy, &f.store, &row, 1).unwrap());
            }
            sync_page(&legacy, touched).unwrap();
            for name in projection.files.keys().take(published) {
                std::fs::rename(legacy.join(name), directory.join(name)).unwrap();
            }
            File::open(&legacy).unwrap().sync_all().unwrap();
            File::open(&directory).unwrap().sync_all().unwrap();
            let retained: BTreeMap<_, _> = std::fs::read_dir(&legacy)
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    (entry.file_name(), std::fs::read(entry.path()).unwrap())
                })
                .collect();
            let mut state = serde_json::to_value(&projection).unwrap();
            state["repair"] = json!({"directory":legacy.file_name().unwrap().to_str().unwrap().trim_start_matches(".projection-"), "head":ROWS});
            let bytes = serde_json::to_vec(&state).unwrap();
            f.store.put(STATE, &bytes).unwrap();
            // A deserialized legacy staged cursor never becomes a public one.
            let hidden: Projection = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(hidden.cursor(), 0);
            let last = f.events.at(ROWS as u64).unwrap().unwrap();
            assert!(hidden.reference(&last, &f.store).unwrap().is_none());
            assert_eq!(
                super::super::step(&f.root, &f.store, &f.events)
                    .unwrap()
                    .cursor(),
                0
            );
            let reason = f.store.get(TRIGGER).unwrap().unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&reason).unwrap()["reason"],
                "legacy_repair_state"
            );
            let saved = f.store.get(REBUILD).unwrap().unwrap();
            let cursor = serde_json::from_slice::<Value>(&saved).unwrap()["projection"]["cursor"]
                .as_u64()
                .unwrap();
            assert!((1..=RECOVERY_PAGE as u64).contains(&cursor));
            assert_eq!(f.store.get(STATE).unwrap().unwrap(), bytes);
            drop(f.store);
            f.store = SqliteStore::open(&f._temp.path().join("index")).unwrap();
            assert_eq!(
                super::super::step(&f.root, &f.store, &f.events)
                    .unwrap()
                    .cursor(),
                0
            );
            assert_eq!(f.store.get(TRIGGER).unwrap().unwrap(), reason);
            let complete = f.finish();
            f.assert_rows(&complete);
            assert!(complete.reference(&last, &f.store).unwrap().is_some());
            assert!(
                serde_json::from_slice::<Value>(&f.store.get(STATE).unwrap().unwrap())
                    .unwrap()
                    .get("repair")
                    .is_none()
            );
            let remaining: BTreeMap<_, _> = std::fs::read_dir(&legacy)
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    (entry.file_name(), std::fs::read(entry.path()).unwrap())
                })
                .collect();
            assert_eq!(
                remaining, retained,
                "unknown legacy trees must remain untouched"
            );
        }
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
        for row in f.events.after(0, ROWS).unwrap() {
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
    // keeps: spec provenance
    #[test]
    fn recovery_records_distinct_triggers_once_per_transition() {
        for expected in [
            "state_missing",
            "state_invalid",
            "cursor_head_mismatch",
            "empty_cursor_hash_mismatch",
            "cursor_hash_mismatch",
            "file_missing",
            "file_length_mismatch",
            "file_mtime_mismatch",
            "append_already_exists",
        ] {
            let f = Fixture::new();
            let mut projection = f.finish();
            let directory = directory(&f.root).unwrap();
            let name = projection.files.keys().next().unwrap().clone();
            let file = directory.join(&name);
            match expected {
                "state_missing" => {
                    f.store
                        .connection()
                        .unwrap()
                        .execute("DELETE FROM kv WHERE key=?1", [STATE])
                        .unwrap();
                }
                "state_invalid" => f.store.put(STATE, b"{private-invalid-json").unwrap(),
                "file_missing" => std::fs::remove_file(&file).unwrap(),
                "file_length_mismatch" => OpenOptions::new()
                    .append(true)
                    .open(&file)
                    .unwrap()
                    .write_all(b"{partial")
                    .unwrap(),
                "file_mtime_mismatch" => {
                    let (length, modified) = stamp(&file).unwrap();
                    File::options()
                        .write(true)
                        .open(&file)
                        .unwrap()
                        .set_modified(
                            std::time::UNIX_EPOCH
                                + Duration::from_nanos(
                                    (modified as u64).saturating_sub(1_000_000_000),
                                ),
                        )
                        .unwrap();
                    assert_eq!(stamp(&file).unwrap().0, length);
                }
                reason => {
                    match reason {
                        "cursor_head_mismatch" => projection.cursor = ROWS as u64 + 1,
                        "empty_cursor_hash_mismatch" => projection.cursor = 0,
                        "cursor_hash_mismatch" => projection.hash = "private-wrong-hash".into(),
                        "append_already_exists" => projection = Projection::default(),
                        _ => unreachable!(),
                    }
                    f.store
                        .put(STATE, &serde_json::to_vec(&projection).unwrap())
                        .unwrap();
                }
            }
            super::super::step(&f.root, &f.store, &f.events).unwrap();
            let trigger = f.store.get(TRIGGER).unwrap().unwrap();
            let reason: Value = serde_json::from_slice(&trigger).unwrap();
            assert_eq!(reason["reason"], expected);
            assert!(!String::from_utf8_lossy(&trigger).contains("private-"));
            if expected.starts_with("file_") {
                assert_eq!(reason["part"], name);
                assert!(reason["expected"]["length"].is_u64());
                if expected != "file_missing" {
                    assert!(reason["observed"].is_object());
                }
            }
            f.assert_rows(&f.finish());
            assert_eq!(
                f.store.get(TRIGGER).unwrap().unwrap(),
                trigger,
                "no per-page diagnostic"
            );
            if expected == "state_invalid" {
                f.store.put(STATE, b"{invalid-again").unwrap();
                super::super::step(&f.root, &f.store, &f.events).unwrap();
                assert_ne!(
                    f.store.get(TRIGGER).unwrap().unwrap(),
                    trigger,
                    "a later recovery must receive a new transition"
                );
                f.assert_rows(&f.finish());
            }
        }
    }

    // keeps: spec provenance
    #[test]
    fn torn_recovery_retains_original_stamp_through_paged_cleanup() {
        let f = Fixture::new();
        super::super::step(&f.root, &f.store, &f.events).unwrap();
        let state: Recovery =
            serde_json::from_slice(&f.store.get(REBUILD).unwrap().unwrap()).unwrap();
        let temporary = f.root.join("log").join(TEMPORARY);
        let name = state.projection.files.keys().next().unwrap();
        OpenOptions::new()
            .append(true)
            .open(temporary.join(name))
            .unwrap()
            .write_all(b"{torn")
            .unwrap();
        // Force more than one cleanup page; the trigger must retain the torn
        // stamp even after that file has been removed by the first page.
        for n in 0..ROWS {
            std::fs::write(temporary.join(format!("orphan-{n}")), b"").unwrap();
        }
        super::super::step(&f.root, &f.store, &f.events).unwrap();
        let bytes = f.store.get(TRIGGER).unwrap().unwrap();
        let trigger: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(trigger["reason"], "file_length_mismatch");
        assert_eq!(trigger["phase"], "recovery");
        assert_eq!(trigger["part"], *name);
        assert!(
            trigger["observed"]["length"].as_u64().unwrap()
                > trigger["expected"]["length"].as_u64().unwrap()
        );
        for _ in 0..3 {
            super::super::step(&f.root, &f.store, &f.events).unwrap();
        }
        assert_eq!(f.store.get(TRIGGER).unwrap().unwrap(), bytes);
        f.assert_rows(&f.finish());
    }
    // keeps: spec provenance
    #[test]
    fn invalid_recovery_state_and_head_have_distinct_triggers() {
        for expected in ["recovery_state_invalid", "recovery_cursor_head_mismatch"] {
            let f = Fixture::new();
            super::super::step(&f.root, &f.store, &f.events).unwrap();
            if expected == "recovery_state_invalid" {
                f.store.put(REBUILD, b"{invalid").unwrap();
            } else {
                let mut saved: Recovery =
                    serde_json::from_slice(&f.store.get(REBUILD).unwrap().unwrap()).unwrap();
                saved.head = ROWS as u64 + 1;
                f.store
                    .put(REBUILD, &serde_json::to_vec(&saved).unwrap())
                    .unwrap();
            }
            super::super::step(&f.root, &f.store, &f.events).unwrap();
            let bytes = f.store.get(TRIGGER).unwrap().unwrap();
            let trigger: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(trigger["reason"], expected);
            f.assert_rows(&f.finish());
            assert_eq!(f.store.get(TRIGGER).unwrap().unwrap(), bytes);
        }
    }
}
