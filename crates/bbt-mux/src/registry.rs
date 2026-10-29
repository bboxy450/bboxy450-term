// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Stable seat/session metadata. Activity and close lifecycle are separate axes.

use rusqlite::OptionalExtension;
use ulid::Ulid;

use crate::store::{Result, Store, invalid, sqlite::SqliteStore};

pub use bbt_proto::registry::{
    AttentionPolicy, AttentionRule, Class, CloseRecord, Lifecycle, Location, Position, Role,
    SeatRecord, SwarmRecord,
};

#[derive(Clone)]
pub struct Registry {
    store: SqliteStore,
}

fn valid_swarm(id: Ulid) -> Result<()> {
    if id == Ulid::nil() {
        Err(invalid("invalid swarm identity"))
    } else {
        Ok(())
    }
}

fn canonical_workdir(path: &str) -> Result<String> {
    if !bbt_proto::gui::valid_workdir(path) {
        return Err(invalid(
            "invalid_swarm_workdir: expected an absolute folder path",
        ));
    }
    let path = std::path::Path::new(path)
        .canonicalize()
        .map_err(|error| invalid(format!("swarm_workdir_unavailable: {error}")))?;
    if !path.is_dir() {
        return Err(invalid(
            "swarm_workdir_unavailable: expected an existing directory",
        ));
    }
    let path = path
        .to_str()
        .filter(|path| bbt_proto::gui::valid_workdir(path))
        .ok_or_else(|| invalid("invalid_swarm_workdir: expected a bounded UTF-8 folder path"))?;
    Ok(path.to_owned())
}

impl Registry {
    pub fn new(store: SqliteStore) -> Self {
        Self { store }
    }
    pub fn swarm(&self, id: Ulid) -> Result<Option<SwarmRecord>> {
        valid_swarm(id)?;
        self.store
            .get(&format!("swarm/{id}"))?
            .map(|bytes| serde_json::from_slice(&bytes).map_err(Into::into))
            .transpose()
    }
    pub fn swarms(&self) -> Result<Vec<SwarmRecord>> {
        self.store
            .list("swarm/")?
            .into_iter()
            .map(|(_, bytes)| serde_json::from_slice(&bytes).map_err(Into::into))
            .collect()
    }
    /// A no-op keeps its revision, but still records an explicit initial reset.
    fn change_swarm(&self, id: Ulid, change: impl FnOnce(&mut SwarmRecord)) -> Result<SwarmRecord> {
        valid_swarm(id)?;
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let key = format!("swarm/{id}");
        let bytes: Option<Vec<u8>> = tx
            .query_row("SELECT value FROM kv WHERE key=?1", [&key], |row| {
                row.get(0)
            })
            .optional()?;
        let mut record = bytes
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()?
            .unwrap_or_else(|| SwarmRecord::new(id));
        let previous = record.clone();
        change(&mut record);
        if record != previous {
            record.revision = record
                .revision
                .checked_add(1)
                .ok_or_else(|| invalid("swarm revision exhausted"))?;
        }
        tx.execute("INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", rusqlite::params![key, serde_json::to_vec(&record)?])?;
        tx.commit()?;
        Ok(record)
    }
    pub fn rename_swarm(&self, id: Ulid, name: &str) -> Result<SwarmRecord> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err(invalid("a swarm name is 1 to 64 printable characters"));
        }
        self.change_swarm(id, |record| record.name = Some(name.to_owned()))
    }
    pub fn update_swarm_workdir(&self, id: Ulid, path: Option<&str>) -> Result<SwarmRecord> {
        let path = path.map(canonical_workdir).transpose()?;
        self.change_swarm(id, |record| record.workdir = path)
    }
    /// Import only absent metadata. A missing legacy folder stays named, never
    /// silently becomes the startup repository. Explicit edits always win.
    pub(crate) fn import_swarm_workdir(&self, id: Ulid, path: &str) -> Result<()> {
        valid_swarm(id)?;
        if self.swarm(id)?.is_some() {
            return Ok(());
        }
        if !bbt_proto::gui::valid_workdir(path) {
            return Err(invalid(
                "invalid_swarm_workdir: expected an absolute folder path",
            ));
        }
        let record = SwarmRecord {
            workdir: Some(canonical_workdir(path).unwrap_or_else(|_| path.to_owned())),
            revision: 1,
            ..SwarmRecord::new(id)
        };
        self.store.connection()?.execute(
            "INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO NOTHING",
            rusqlite::params![format!("swarm/{id}"), serde_json::to_vec(&record)?],
        )?;
        Ok(())
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
    /// Only the admitted control owner supplies this seat and generation.
    pub fn set_attention(
        &self,
        id: Ulid,
        generation: u64,
        rule: AttentionRule,
    ) -> Result<AttentionPolicy> {
        self.update(id, |record| {
            if record.generation != generation {
                return Err(invalid("stale attention process generation"));
            }
            let policy = crate::messaging::attention::updated(record.attention.as_ref(), rule)?;
            record.attention = Some(policy.clone());
            Ok(policy)
        })
    }
    /// Trusted operator path, distinct from a seat's own MCP mutation.
    /// Only a confirmed publication advances the public registry view.
    pub fn change_attention(&self, id: Ulid, edit: &bbt_proto::gui::AttentionEdit) -> Result<()> {
        self.update(id, |record| {
            if record.generation != edit.generation {
                return Err(invalid("stale attention process generation"));
            }
            let destination = crate::messaging::attention::destination(&edit.destination)?;
            let defaults = crate::messaging::attention::defaults(&record.role);
            let default = if edit.class == bbt_proto::messaging::MessageClass::Event {
                defaults.event
            } else if destination == format!("seats/{id}/inbox") {
                defaults.direct_conversation
            } else {
                defaults.conversation
            };
            let mut policy = record.attention.clone().unwrap_or_default();
            crate::messaging::attention::validate(&policy)?;
            policy
                .rules
                .retain(|rule| rule.destination != destination || rule.class != edit.class);
            if let Some(mode) = edit.mode.filter(|mode| *mode != default) {
                policy = crate::messaging::attention::updated(
                    Some(&policy),
                    AttentionRule {
                        destination,
                        class: edit.class,
                        mode,
                    },
                )?;
            }
            record.attention = (!policy.rules.is_empty()).then_some(policy);
            Ok(())
        })
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
