use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};

use super::{Result, Store, invalid};

/// Connections are leased for one operation from a small process-wide pool:
/// a hot object reuses its open connection (pragmas once, WAL kept open), while
/// at most `POOL` idle connections stay open in all, so 1000 cold objects do
/// not retain thousands of database/WAL descriptors. One logic owner writes
/// each object.
#[derive(Debug, Clone)]
pub struct SqliteStore {
    pub(crate) path: PathBuf,
}

/// Idle connections kept open across all objects, least recently used dropped first.
const POOL: usize = 64;

struct Idle {
    path: PathBuf,
    /// (device, inode) of the database file when opened: a replaced file is never reused.
    file: (u64, u64),
    db: Connection,
}

static IDLE: Mutex<VecDeque<Idle>> = Mutex::new(VecDeque::new());

fn identity(path: &Path) -> Option<(u64, u64)> {
    std::fs::metadata(path).ok().map(|m| (m.dev(), m.ino()))
}

/// An open connection on loan; it returns to the pool when dropped.
pub struct Lease {
    path: PathBuf,
    file: (u64, u64),
    db: Option<Connection>,
}

impl Deref for Lease {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.db.as_ref().expect("leased connection")
    }
}

impl DerefMut for Lease {
    fn deref_mut(&mut self) -> &mut Connection {
        self.db.as_mut().expect("leased connection")
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        let Some(db) = self.db.take() else { return };
        // Only a connection at rest goes back; anything mid-transaction is closed.
        if !db.is_autocommit() {
            return;
        }
        let evicted = {
            let mut idle = IDLE.lock().unwrap_or_else(|e| e.into_inner());
            idle.push_back(Idle {
                path: std::mem::take(&mut self.path),
                file: self.file,
                db,
            });
            (idle.len() > POOL).then(|| idle.pop_front())
        };
        drop(evicted); // close outside the lock
    }
}

pub(crate) fn open(path: &Path) -> Result<Lease> {
    bbt_rt::assert_not_ui_thread();
    let file = identity(path);
    let reused = {
        let mut idle = IDLE.lock().unwrap_or_else(|e| e.into_inner());
        idle.iter()
            .rposition(|c| c.path == path)
            .and_then(|i| idle.remove(i))
    };
    if let Some(c) = reused
        && Some(c.file) == file
    {
        return Ok(Lease {
            path: c.path,
            file: c.file,
            db: Some(c.db),
        });
    }
    let db = Connection::open(path)?;
    db.busy_timeout(Duration::from_secs(2))?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.pragma_update(None, "synchronous", "FULL")?;
    db.pragma_update(None, "foreign_keys", true)?;
    // Reads map the file (shared OS page cache) instead of pread per page.
    db.pragma_update(None, "mmap_size", 256i64 << 20)?;
    let file = identity(path).unwrap_or_default();
    Ok(Lease {
        path: path.to_path_buf(),
        file,
        db: Some(db),
    })
}

/// The pragma is a read-only fast path; only a missing schema marker reserves
/// the writer. Existing version-table databases receive the marker once.
pub(super) fn ensure_schema(db: &mut Connection, ddl: &str, unsupported: &str) -> Result<()> {
    let version: i64 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version != 0 && version != 1 {
        return Err(invalid(unsupported));
    }
    if version == 0 {
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        // Another opener may have initialized it while we waited for the writer.
        let current: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
        match current {
            0 => {
                tx.execute_batch(ddl)?;
                let legacy: i64 = tx.query_row("SELECT version FROM version", [], |r| r.get(0))?;
                if legacy != 1 {
                    return Err(invalid(unsupported));
                }
                tx.pragma_update(None, "user_version", 1)?;
            }
            1 => {}
            _ => return Err(invalid(unsupported)),
        }
        tx.commit()?;
    }
    // Preserve the existing version-table compatibility check on the fast path.
    let legacy: i64 = db.query_row("SELECT version FROM version", [], |row| row.get(0))?;
    if legacy != 1 {
        return Err(invalid(unsupported));
    }
    Ok(())
}

impl SqliteStore {
    pub fn open(directory: &Path) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        std::fs::create_dir_all(directory)?;
        let store = Self {
            path: directory.join("object.sqlite"),
        };
        let mut db = open(&store.path)?;
        ensure_schema(
            &mut db,
            "CREATE TABLE IF NOT EXISTS version (version INTEGER NOT NULL);
            INSERT INTO version SELECT 1 WHERE NOT EXISTS(SELECT 1 FROM version);
            CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS alarms (key TEXT PRIMARY KEY, deadline INTEGER NOT NULL);",
            "unsupported object schema",
        )?;
        Ok(store)
    }

    pub fn connection(&self) -> Result<Lease> {
        open(&self.path)
    }
}

impl Store for SqliteStore {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self
            .connection()?
            .query_row("SELECT value FROM kv WHERE key=?1", [key], |r| r.get(0))
            .optional()?)
    }
    fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.connection()?.execute(
            "INSERT INTO kv VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
    fn list(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let db = self.connection()?;
        let mut query =
            db.prepare("SELECT key,value FROM kv WHERE substr(key,1,length(?1))=?1 ORDER BY key")?;
        Ok(query
            .query_map([prefix], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?)
    }
    fn sql(&self, query: &str, args: &[Value]) -> Result<Vec<Vec<Value>>> {
        let db = self.connection()?;
        let mut stmt = db.prepare(query)?;
        let columns = stmt.column_count();
        if columns == 0 {
            stmt.execute(params_from_iter(args))?;
            return Ok(Vec::new());
        }
        Ok(stmt
            .query_map(params_from_iter(args), |row| {
                (0..columns).map(|i| row.get(i)).collect()
            })?
            .collect::<std::result::Result<_, _>>()?)
    }
    fn alarm(&self, key: &str, deadline_ms: u64) -> Result<()> {
        self.connection()?.execute("INSERT INTO alarms VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET deadline=excluded.deadline", params![key,super::sqlint(deadline_ms)?])?;
        Ok(())
    }
    fn due_alarms(&self, now_ms: u64) -> Result<Vec<String>> {
        let db = self.connection()?;
        let mut stmt =
            db.prepare("SELECT key FROM alarms WHERE deadline<=?1 ORDER BY deadline,key")?;
        Ok(stmt
            .query_map([super::sqlint(now_ms)?], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;

    #[test]
    fn schema_markers_migrate_legacy_databases_and_preserve_version_refusals() {
        use crate::store::events::EventLog;

        let root = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(root.path()).unwrap();
        store.put("retained", b"original").unwrap();
        let log = EventLog::open(root.path()).unwrap();
        for path in [&store.path, &log.path] {
            let db = open(path).unwrap();
            db.pragma_update(None, "user_version", 0).unwrap();
        }
        SqliteStore::open(root.path()).unwrap();
        EventLog::open(root.path()).unwrap();
        assert_eq!(store.get("retained").unwrap().unwrap(), b"original");
        for path in [&store.path, &log.path] {
            let db = open(path).unwrap();
            assert_eq!(
                db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                    .unwrap(),
                1
            );
            db.pragma_update(None, "user_version", 2).unwrap();
        }
        assert_eq!(
            SqliteStore::open(root.path()).unwrap_err().to_string(),
            "unsupported object schema"
        );
        assert_eq!(
            EventLog::open(root.path()).unwrap_err().to_string(),
            "unsupported event schema"
        );
        for path in [&store.path, &log.path] {
            let db = open(path).unwrap();
            db.pragma_update(None, "user_version", 0).unwrap();
            db.execute("UPDATE version SET version=2", []).unwrap();
        }
        assert_eq!(
            SqliteStore::open(root.path()).unwrap_err().to_string(),
            "unsupported object schema"
        );
        assert_eq!(
            EventLog::open(root.path()).unwrap_err().to_string(),
            "unsupported event schema"
        );
        for path in [&store.path, &log.path] {
            let db = open(path).unwrap();
            assert_eq!(
                db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }

    #[test]
    fn fifty_current_schema_opens_do_not_take_either_database_write_lock() {
        use crate::store::events::EventLog;

        const CALLERS: usize = 50;
        let root = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(root.path()).unwrap();
        let log = EventLog::open(root.path()).unwrap();
        let mut object_db = store.connection().unwrap();
        let mut events_db = open(&log.path).unwrap();
        // Keep both writers reserved until every open returns. A read-only open
        // can finish; any attempted schema write must fail rather than waiting
        // for a writer we release later and accidentally passing the test.
        let object_writer = object_db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        let events_writer = events_db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .unwrap();
        let start = Barrier::new(CALLERS);
        let failures = std::thread::scope(|scope| {
            let calls: Vec<_> = (0..CALLERS)
                .map(|caller| {
                    let start = &start;
                    let directory = root.path();
                    scope.spawn(move || {
                        start.wait();
                        let mut failures = Vec::new();
                        if let Err(error) = SqliteStore::open(directory) {
                            failures.push(format!("caller={caller} object: {error}"));
                        }
                        if let Err(error) = EventLog::open(directory) {
                            failures.push(format!("caller={caller} events: {error}"));
                        }
                        failures
                    })
                })
                .collect();
            calls
                .into_iter()
                .flat_map(|call| call.join().unwrap())
                .collect::<Vec<_>>()
        });
        drop((object_writer, events_writer));
        assert!(
            failures.is_empty(),
            "schema opens requested a writer: {failures:?}"
        );
    }

    #[test]
    fn concurrent_open_and_read_then_write_share_one_database_without_busy() {
        const ROUNDS: usize = 50;
        const CALLERS: usize = 4;
        let root = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(root.path()).unwrap();
        store.put("counter", &0u64.to_le_bytes()).unwrap();
        // Start with enough configured connections to isolate schema transaction
        // admission from first-connection WAL setup. All callers use the real pool.
        let leases: Vec<_> = (0..CALLERS).map(|_| store.connection().unwrap()).collect();
        drop(leases);
        let start = Barrier::new(CALLERS);
        let failures = std::thread::scope(|scope| {
            let calls: Vec<_> = (0..CALLERS)
                .map(|caller| {
                    let start = &start;
                    let directory = root.path();
                    scope.spawn(move || {
                        let mut failures = Vec::new();
                        for round in 0..ROUNDS {
                            start.wait();
                            let result = (|| -> Result<()> {
                                let reopened = SqliteStore::open(directory)?;
                                let mut db = reopened.connection()?;
                                let tx = db.transaction_with_behavior(
                                    rusqlite::TransactionBehavior::Immediate,
                                )?;
                                let bytes: Vec<u8> = tx.query_row(
                                    "SELECT value FROM kv WHERE key='counter'",
                                    [],
                                    |row| row.get(0),
                                )?;
                                let value = u64::from_le_bytes(
                                    bytes.try_into().map_err(|_| invalid("invalid counter"))?,
                                );
                                // Let competing schema opens observe this writer;
                                // no time-based delay or retry hides an error.
                                std::thread::yield_now();
                                tx.execute(
                                    "UPDATE kv SET value=?1 WHERE key='counter'",
                                    [value.checked_add(1).unwrap().to_le_bytes().as_slice()],
                                )?;
                                tx.commit()?;
                                Ok(())
                            })();
                            if let Err(error) = result {
                                failures.push(format!("caller={caller} round={round}: {error}"));
                            }
                        }
                        failures
                    })
                })
                .collect();
            calls
                .into_iter()
                .flat_map(|call| call.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert!(failures.is_empty(), "concurrent store errors: {failures:?}");
        let bytes = store.get("counter").unwrap().unwrap();
        assert_eq!(
            u64::from_le_bytes(bytes.try_into().unwrap()),
            (CALLERS * ROUNDS) as u64,
            "every successful read-then-write must be retained"
        );
    }
}
