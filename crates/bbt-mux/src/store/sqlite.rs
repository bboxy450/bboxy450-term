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

impl SqliteStore {
    pub fn open(directory: &Path) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        std::fs::create_dir_all(directory)?;
        let store = Self {
            path: directory.join("object.sqlite"),
        };
        let mut db = open(&store.path)?;
        let tx = db.transaction()?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS version (version INTEGER NOT NULL);
            INSERT INTO version SELECT 1 WHERE NOT EXISTS(SELECT 1 FROM version);
            CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS alarms (key TEXT PRIMARY KEY, deadline INTEGER NOT NULL);",
        )?;
        let version: i64 = tx.query_row("SELECT version FROM version", [], |r| r.get(0))?;
        if version != 1 {
            return Err(invalid("unsupported object schema"));
        }
        tx.commit()?;
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
