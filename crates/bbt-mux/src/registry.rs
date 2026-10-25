//! Stable seat/session metadata. Activity and close lifecycle are separate axes.

use ulid::Ulid;

use crate::store::{Result, Store, invalid, sqlite::SqliteStore};

pub use bbt_proto::registry::{
    Class, CloseRecord, Lifecycle, Location, Position, Role, SeatRecord,
};

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

#[cfg(test)]
mod close_policy_tests {
    use super::*;
    use std::time::Duration;

    // keeps: spec c1-live-seats §4 (durable close/undo, exact grace and Position)
    #[tokio::test(start_paused = true)]
    async fn close_keeps_its_deadline_and_position_across_reopen_and_checks_revisions() {
        let temp = tempfile::tempdir().unwrap();
        let registry = Registry::new(SqliteStore::open(temp.path()).unwrap());
        let mut record = SeatRecord::new(Ulid::new(), Ulid::new(), "fixture", "zsh");
        record.harness_session_id = Some("same-process-session".into());
        registry.insert(&record).unwrap();
        let position = Position {
            swarm: record.swarm,
            tab: Ulid::new(),
            split_path: vec![1, 0],
            ratios: vec![0.35, 0.65],
            layout_revision: 7,
        };
        let origin = tokio::time::Instant::now();
        let now = || 1_000 + (tokio::time::Instant::now() - origin).as_millis() as u64;
        let operation = Ulid::new();
        let close = registry
            .close(record.id, 1, operation, position.clone(), now(), 60_000)
            .unwrap();
        assert_eq!(close.deadline_ms, 61_000);
        assert_eq!(close.operation, operation);
        assert_eq!(close.position, position);
        assert_eq!(
            registry.get(record.id).unwrap().lifecycle,
            Lifecycle::ClosedGrace
        );
        tokio::time::advance(Duration::from_secs(30)).await;
        assert_eq!(
            registry
                .close(record.id, 1, operation, position.clone(), now(), 60_000)
                .unwrap(),
            close
        );
        let registry = Registry::new(SqliteStore::open(temp.path()).unwrap());
        assert_eq!(registry.get(record.id).unwrap().close, Some(close.clone()));
        assert!(
            registry
                .undo(record.id, 1, close.revision, 8, now())
                .is_err()
        );
        assert!(
            registry
                .undo(record.id, 2, close.revision, 7, now())
                .is_err()
        );
        assert!(
            registry
                .undo(record.id, 1, close.revision + 1, 7, now())
                .is_err()
        );
        assert_eq!(
            registry.get(record.id).unwrap().lifecycle,
            Lifecycle::ClosedGrace
        );
        assert_eq!(
            registry
                .undo(record.id, 1, close.revision, 7, now())
                .unwrap(),
            position
        );
        let restored = registry.get(record.id).unwrap();
        assert_eq!(restored.lifecycle, Lifecycle::Open);
        assert_eq!(restored.generation, 1);
        assert_eq!(restored.harness_session_id, record.harness_session_id);
        assert_eq!(restored.close, Some(close));
    }

    // keeps: invariant restart-keeps-state (retained close survives reopening)
    #[tokio::test(start_paused = true)]
    async fn expired_undo_is_refused_and_only_the_exit_receipt_makes_the_seat_retained() {
        let temp = tempfile::tempdir().unwrap();
        let registry = Registry::new(SqliteStore::open(temp.path()).unwrap());
        let record = SeatRecord::new(Ulid::new(), Ulid::new(), "fixture", "zsh");
        registry.insert(&record).unwrap();
        let position = Position {
            swarm: record.swarm,
            tab: Ulid::new(),
            split_path: vec![1, 0],
            ratios: vec![0.35, 0.65],
            layout_revision: 7,
        };
        let origin = tokio::time::Instant::now();
        let close = registry
            .close(record.id, 1, Ulid::new(), position, 0, 60_000)
            .unwrap();
        tokio::time::advance(Duration::from_secs(60)).await;
        let elapsed = (tokio::time::Instant::now() - origin).as_millis() as u64;
        assert!(
            registry
                .undo(record.id, 1, close.revision, 7, elapsed)
                .is_err()
        );
        assert_eq!(
            registry.get(record.id).unwrap().lifecycle,
            Lifecycle::ClosedGrace
        );
        registry.exited(record.id, 1).unwrap();
        let registry = Registry::new(SqliteStore::open(temp.path()).unwrap());
        let retained = registry.get(record.id).unwrap();
        assert_eq!(retained.lifecycle, Lifecycle::ClosedRetained);
        assert_eq!(retained.generation, 1);
        assert_eq!(retained.close, Some(close));
        assert!(registry.undo(record.id, 1, 1, 7, elapsed).is_err());
    }
}
