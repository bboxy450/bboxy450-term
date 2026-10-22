//! Stable seat/session metadata. Activity and close lifecycle are separate axes.

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::store::{Result, Store, invalid, sqlite::SqliteStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub class: Class,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Oracle,
    Worker,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Location {
    Local,
    Ssh { host: String },
    Cloud { provider: String, host: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub swarm: Ulid,
    pub tab: Ulid,
    pub split_path: Vec<usize>,
    pub ratios: Vec<f64>,
    pub layout_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloseRecord {
    pub operation: Ulid,
    pub revision: u64,
    pub deadline_ms: u64,
    pub position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Open,
    ClosedGrace,
    ClosedRetained,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeatRecord {
    pub id: Ulid,
    pub swarm: Ulid,
    pub name: String,
    pub harness_kind: String,
    pub harness_session_id: Option<String>,
    pub role: Role,
    #[serde(rename = "where")]
    pub location: Location,
    pub muted: bool,
    pub generation: u64,
    pub lifecycle: Lifecycle,
    pub lifecycle_revision: u64,
    pub close: Option<CloseRecord>,
    pub tasks: Vec<Ulid>,
}

impl SeatRecord {
    pub fn new(
        id: Ulid,
        swarm: Ulid,
        name: impl Into<String>,
        harness_kind: impl Into<String>,
    ) -> Self {
        Self {
            id,
            swarm,
            name: name.into(),
            harness_kind: harness_kind.into(),
            harness_session_id: None,
            role: Role {
                name: "executor".into(),
                class: Class::Worker,
            },
            location: Location::Local,
            muted: false,
            generation: 1,
            lifecycle: Lifecycle::Open,
            lifecycle_revision: 0,
            close: None,
            tasks: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct Registry {
    store: SqliteStore,
}

impl Registry {
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }
    pub fn insert(&self, record: &SeatRecord) -> Result<()> {
        if record.id == Ulid::nil() || record.swarm == Ulid::nil() || record.generation == 0 {
            return Err(invalid("invalid seat identity"));
        }
        let db = self.store.connection()?;
        db.execute(
            "INSERT INTO kv VALUES(?1,?2)",
            rusqlite::params![format!("seat/{}", record.id), serde_json::to_vec(record)?],
        )?;
        Ok(())
    }
    pub fn get(&self, id: Ulid) -> Result<SeatRecord> {
        let bytes = self
            .store
            .get(&format!("seat/{id}"))?
            .ok_or_else(|| invalid("unknown seat"))?;
        Ok(serde_json::from_slice(&bytes)?)
    }
    pub fn list(&self) -> Result<Vec<SeatRecord>> {
        self.store
            .list("seat/")?
            .into_iter()
            .map(|(_, b)| Ok(serde_json::from_slice(&b)?))
            .collect()
    }
    pub fn update<T>(&self, id: Ulid, f: impl FnOnce(&mut SeatRecord) -> Result<T>) -> Result<T> {
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let key = format!("seat/{id}");
        let bytes: Vec<u8> =
            tx.query_row("SELECT value FROM kv WHERE key=?1", [&key], |r| r.get(0))?;
        let mut record: SeatRecord = serde_json::from_slice(&bytes)?;
        let result = f(&mut record)?;
        tx.execute(
            "UPDATE kv SET value=?2 WHERE key=?1",
            rusqlite::params![key, serde_json::to_vec(&record)?],
        )?;
        tx.commit()?;
        Ok(result)
    }
    pub fn close(
        &self,
        id: Ulid,
        generation: u64,
        operation: Ulid,
        position: Position,
        now: u64,
        grace_ms: u64,
    ) -> Result<CloseRecord> {
        if grace_ms > 86_400_000
            || operation == Ulid::nil()
            || position
                .ratios
                .iter()
                .any(|r| !r.is_finite() || *r <= 0. || *r >= 1.)
        {
            return Err(invalid("invalid close window or layout"));
        }
        self.update(id, |record| {
            if record.generation != generation {
                return Err(invalid("process replaced"));
            }
            if let Some(close) = &record.close
                && close.operation == operation
            {
                return Ok(close.clone());
            }
            if record.lifecycle != Lifecycle::Open {
                return Err(invalid("seat already closed"));
            }
            record.lifecycle_revision = record
                .lifecycle_revision
                .checked_add(1)
                .ok_or_else(|| invalid("lifecycle exhausted"))?;
            let close = CloseRecord {
                operation,
                revision: record.lifecycle_revision,
                deadline_ms: now
                    .checked_add(grace_ms)
                    .ok_or_else(|| invalid("deadline overflow"))?,
                position,
            };
            record.lifecycle = Lifecycle::ClosedGrace;
            record.close = Some(close.clone());
            Ok(close)
        })
    }
    pub fn undo(
        &self,
        id: Ulid,
        generation: u64,
        revision: u64,
        layout_revision: u64,
        now: u64,
    ) -> Result<Position> {
        self.update(id, |record| {
            let close = record
                .close
                .as_ref()
                .ok_or_else(|| invalid("seat has no close record"))?;
            if record.generation != generation {
                return Err(invalid("process replaced"));
            }
            if close.revision != revision {
                return Err(invalid("stale close revision"));
            }
            if close.position.layout_revision != layout_revision {
                return Err(invalid("layout conflict"));
            }
            if record.lifecycle != Lifecycle::ClosedGrace || now >= close.deadline_ms {
                return Err(invalid("close window expired"));
            }
            let position = close.position.clone();
            record.lifecycle = Lifecycle::Open;
            record.lifecycle_revision += 1;
            Ok(position)
        })
    }
    /// Supervisor exit receipt, not a speculative clock tick, makes this retained.
    pub fn exited(&self, id: Ulid, generation: u64) -> Result<()> {
        self.update(id, |record| {
            if record.generation != generation {
                return Err(invalid("stale exit generation"));
            }
            if record.lifecycle == Lifecycle::ClosedGrace {
                record.lifecycle = Lifecycle::ClosedRetained;
            }
            Ok(())
        })
    }
}
