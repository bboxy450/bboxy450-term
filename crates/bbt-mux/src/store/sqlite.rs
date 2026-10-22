use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};

use super::{Result, Store, invalid};

/// Connections are leased for one operation, so 1000 cold objects do not retain
/// thousands of database/WAL descriptors. One logic owner writes each object.
#[derive(Debug, Clone)]
pub struct SqliteStore {
    pub(crate) path: PathBuf,
}

pub(crate) fn open(path: &Path) -> Result<Connection> {
    bbt_rt::assert_not_ui_thread();
    let db = Connection::open(path)?;
    db.busy_timeout(Duration::from_secs(2))?;
    db.pragma_update(None, "journal_mode", "WAL")?;
    db.pragma_update(None, "synchronous", "FULL")?;
    db.pragma_update(None, "foreign_keys", true)?;
    Ok(db)
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

    pub fn connection(&self) -> Result<Connection> {
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
