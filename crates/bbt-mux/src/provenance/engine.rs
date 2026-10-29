// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The IO owner publishes one observation at a time and replays its Markdown
//! projection independently. A hold never changes the original event's signer.
use super::{
    artifacts::{self, Change},
    author::Author,
    detector::{Detector, Hashed},
};
use crate::{
    Ulid,
    bus::BusHandle,
    identity::IdentityRuntime,
    index::Index,
    store::{
        Events, Result, Store,
        events::{Draft, Envelope},
        sqlite::SqliteStore,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};

const OUTBOX: &str = "provenance/observation-outbox";
const CURSOR: &str = "provenance/artifact-projection";
#[derive(Clone)]
struct Entry {
    id: Ulid,
    digest: Option<String>,
    deleted: bool,
    observed: bool,
}
#[derive(Serialize, Deserialize)]
struct Pending {
    event: Draft,
    change: Change,
}

pub(crate) struct Engine {
    detector: Detector,
    store: SqliteStore,
    entries: BTreeMap<String, Entry>,
    ready: VecDeque<Hashed>,
    diagnostics: VecDeque<String>,
    recovery_cursor: u64,
    recovery_head: u64,
    recovering: bool,
    projection_cursor: u64,
    pending: Option<Pending>,
    holding: Option<bool>,
}
impl Engine {
    pub fn open(
        index: &Index,
        repository: &std::path::Path,
        handle: tokio::runtime::Handle,
    ) -> Result<Self> {
        let store = index.mirror();
        let pending = store
            .get(OUTBOX)?
            .map(|bytes| serde_json::from_slice(&bytes))
            .transpose()?;
        let projection_cursor = store
            .get(CURSOR)?
            .map(|bytes| serde_json::from_slice(&bytes))
            .transpose()?
            .unwrap_or(0);
        let head = index.activity().last_sequence()?;
        let mut engine = Self {
            detector: Detector::new(repository, handle)?,
            store,
            entries: BTreeMap::new(),
            ready: VecDeque::new(),
            diagnostics: VecDeque::from([
                "artifact-watch-started: offline changes require reconciliation".into(),
            ]),
            recovery_cursor: 0,
            recovery_head: head,
            recovering: head != 0,
            projection_cursor,
            pending,
            holding: None,
        };
        for doc in index
            .documents
            .values()
            .filter(|doc| doc.metadata["type"] == "artifact")
        {
            engine.note_claim(&doc.metadata);
        }
        Ok(engine)
    }
    pub fn register(&mut self, seat: Ulid, generation: u64, cwd: &std::path::Path, local: bool) {
        if self
            .detector
            .register(seat, generation, cwd, local)
            .is_err()
        {
            self.diagnostic("artifact-worktree-unavailable".into());
        }
    }
    pub fn hook(&mut self, seat: Ulid, generation: u64, event: &crate::adapters::hook::ShimEvent) {
        self.detector.hook(seat, generation, event);
    }
    pub fn osc(&mut self, seat: Ulid, generation: u64, params: &[Vec<u8>]) {
        self.detector.osc(seat, generation, params);
    }
    pub fn normalize(&self, seat: Ulid, generation: u64, reference: &str) -> Option<String> {
        if self.entries.contains_key(reference) && self.detector.owns(seat, generation, reference) {
            return Some(reference.into());
        }
        self.detector.normalize(seat, generation, reference)
    }
    pub fn recovering(&self) -> bool {
        self.recovering
    }
    fn diagnostic(&mut self, reason: String) {
        if self.diagnostics.len() < 16 && !self.diagnostics.contains(&reason) {
            self.diagnostics.push_back(reason);
        }
    }
    fn apply_catalog(&mut self, change: &Change) {
        let entry = Entry {
            id: change.artifact_id,
            digest: change.digest.clone(),
            deleted: change.kind == "deleted",
            observed: true,
        };
        if let Some(previous) = &change.previous_ref {
            self.entries.remove(previous);
        }
        self.entries.insert(change.reference.clone(), entry);
    }
    fn change(envelope: &Envelope) -> Option<Change> {
        let change: Change =
            serde_json::from_value(envelope.event.payload.get("observation")?.clone()).ok()?;
        (envelope.event.subject == format!("artifact/{}/{}", change.artifact_id, change.kind))
            .then_some(change)
    }
    fn draft(&self, row: Hashed) -> Option<Pending> {
        let old = row
            .previous_ref
            .as_ref()
            .and_then(|p| self.entries.get(p))
            .or_else(|| self.entries.get(&row.reference));
        if row.kind == "deleted" && old.is_none() {
            return None;
        }
        if row.source.starts_with("filesystem")
            && row.kind != "renamed"
            && old.is_some_and(|old| {
                old.observed && old.digest == row.digest && old.deleted == (row.kind == "deleted")
            })
            && row.reason.is_none()
        {
            return None;
        }
        // OS hints can arrive after a second edit. Kind follows the known
        // revision state; a repeated Create is not a second birth.
        let kind = match (old, row.kind.as_str()) {
            (None, "modified") => "created".into(),
            (Some(entry), "created") if !entry.deleted => "modified".into(),
            (Some(entry), "modified") if entry.deleted => "created".into(),
            _ => row.kind,
        };
        let id = if let Some(entry) = old.filter(|old| !old.deleted || kind != "created") {
            entry.id
        } else {
            Ulid::new()
        };
        let change = Change {
            artifact_id: id,
            kind,
            reference: row.reference,
            previous_ref: row.previous_ref,
            digest: row.digest,
            before_digest: old
                .filter(|old| old.observed)
                .and_then(|old| old.digest.clone()),
            bytes: row.bytes,
            attribution: row.attribution,
            source: row.source,
            reason: row.reason,
            commit: row.commit,
        };
        let mut event = Draft::new(
            format!("artifact/{id}/{}", change.kind),
            format!("provenance/{}", change.source),
            json!({"observation":change}),
        );
        event.ts = row.at;
        event.seat = change.attribution.seat;
        event.generation = change.attribution.generation;
        Some(Pending { event, change })
    }
    /// One bounded catalog page, one observation and one artifact projection.
    /// All hashes have already completed on the four bounded IO jobs.
    pub fn step(
        &mut self,
        index: &mut Index,
        bus: &BusHandle,
        handle: &tokio::runtime::Handle,
        identity: &IdentityRuntime,
        active: &[(Ulid, u64, u64)],
    ) -> Result<bool> {
        if self.recovering {
            let rows = index.activity().after(self.recovery_cursor, 128)?;
            let head = self.recovery_head;
            for row in rows.into_iter().take_while(|row| row.seq <= head) {
                if let Some(change) = Self::change(&row) {
                    self.apply_catalog(&change);
                }
                self.recovery_cursor = row.seq;
            }
            if self.recovery_cursor >= self.recovery_head {
                self.recovering = false;
                self.detector.reconcile_known(
                    self.entries
                        .iter()
                        .filter(|(_, entry)| entry.observed)
                        .map(|(reference, _)| reference.clone()),
                );
            }
            index.project_activity()?;
            return Ok(false);
        }
        // Detector yields at most 132 rows, so this queue never exceeds 139.
        if self.ready.len() < 8 {
            let (rows, gaps) = self.detector.poll(active);
            self.ready.extend(rows);
            if gaps.iter().any(|gap| gap.contains("overflow")) {
                self.detector.reconcile_known(
                    self.entries
                        .iter()
                        .filter(|(_, entry)| entry.observed)
                        .map(|(reference, _)| reference.clone()),
                );
            }
            for gap in gaps {
                self.diagnostic(gap);
            }
        }
        if let Some(reason) = self.diagnostics.pop_front() {
            let event = Draft::new(
                "activity/provenance/gap",
                "provenance",
                json!({"reason":reason,"complete":false}),
            );
            handle.block_on(bus.publish(index.activity_stream(), event))?;
        }
        if self.pending.is_none()
            && let Some(row) = self.ready.pop_front()
        {
            if row.reason.as_deref() == Some("artifact-path-unavailable-or-changed") {
                self.diagnostic(
                    "artifact-path-unavailable-or-changed: reconciliation required".into(),
                );
            } else if let Some(pending) = self.draft(row) {
                self.store.put(OUTBOX, &serde_json::to_vec(&pending)?)?;
                self.pending = Some(pending);
            }
        }
        if let Some(pending) = &self.pending {
            let envelope =
                handle.block_on(bus.publish(index.activity_stream(), pending.event.clone()))?;
            envelope.verify()?;
            let change = pending.change.clone();
            self.apply_catalog(&change);
            self.store
                .connection()?
                .execute("DELETE FROM kv WHERE key=?1", [OUTBOX])?;
            self.pending = None;
        }
        let mut changed = false;
        let signer = identity.bus_signer()?;
        let held = signer.is_none();
        if self.holding != Some(held) {
            let event = Draft::new(
                "activity/provenance/state",
                "provenance",
                json!({"state":if held{"held"}else{"live"},"reason":held.then_some("artifact-projection-signing-held"),"checkpoint":"pending-root-held"}),
            );
            handle.block_on(bus.publish(index.activity_stream(), event))?;
            self.holding = Some(held);
        }
        if let Some(key) = signer {
            let status = identity.status()?;
            let before_projection = self.projection_cursor;
            let rows = index.activity().after(self.projection_cursor, 128)?;
            for row in rows {
                if let Some(change) = Self::change(&row) {
                    let update = artifacts::materialize(index, &row, &change)?;
                    // A prior completed write of this observation is already a projection.
                    let already = index.current(&update.path)?.is_some_and(|doc| {
                        doc.metadata["last_observation_sequence"]
                            .as_str()
                            .and_then(|value| value.parse::<u64>().ok())
                            .is_some_and(|sequence| sequence >= row.seq)
                            || doc.metadata["history"].as_array().is_some_and(|history| {
                                history
                                    .iter()
                                    .any(|entry| entry["id"] == json!(row.event.id))
                            })
                    });
                    if !already {
                        index.apply_author(update, &Author::observer(&status, &key))?;
                        changed = true;
                    }
                    self.projection_cursor = row.seq;
                    break;
                }
                self.projection_cursor = row.seq;
            }
            if self.projection_cursor != before_projection {
                self.store
                    .put(CURSOR, &serde_json::to_vec(&self.projection_cursor)?)?;
            }
        } // No legitimate observer key: durable hold events remain, projection is pending.
        index.project_activity()?;
        Ok(changed)
    }
    pub fn note_claim(&mut self, metadata: &serde_json::Value) {
        let Some(reference) = metadata["ref"].as_str() else {
            return;
        };
        let Some(id) = metadata["id"].as_str().and_then(|id| id.parse().ok()) else {
            return;
        };
        self.entries
            .entry(reference.into())
            .or_insert_with(|| Entry {
                id,
                digest: metadata["digest"].as_str().map(str::to_owned),
                deleted: metadata["deleted"] == true,
                observed: metadata["observed"] == true,
            });
    }
    pub fn claim_ready(&self, index: &Index, reference: &str) -> Result<bool> {
        let Some(entry) = self.entries.get(reference) else {
            return Ok(true);
        };
        Ok(index
            .current(&format!("artifacts/{}.md", entry.id))?
            .is_some_and(|doc| {
                doc.metadata["ref"] == reference
                    && doc.metadata["digest"].as_str() == entry.digest.as_deref()
                    && (doc.metadata["deleted"] == true) == entry.deleted
            }))
    }
}
