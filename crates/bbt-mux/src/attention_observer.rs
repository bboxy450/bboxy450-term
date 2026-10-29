// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Current waiting projection. No bus cursor, read watermark, terminal watch or
//! historical event can create an episode here.
use crate::{
    Ulid,
    store::{Result, invalid},
};
use bbt_proto::{Activity, GuiSeat, NotificationEpisode, ProcessStatus, gui::AttentionUpdate};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use tokio::sync::broadcast;

const CAPACITY: usize = 256;
struct State {
    epoch: Ulid,
    revision: u64,
    ordinal: u64,
    generations: BTreeMap<Ulid, u64>,
    waiting: BTreeMap<Ulid, GuiSeat>,
}
pub(crate) struct CurrentAttention {
    state: RwLock<State>,
    changed: broadcast::Sender<AttentionUpdate>,
}
impl Default for CurrentAttention {
    fn default() -> Self {
        Self {
            state: RwLock::new(State {
                epoch: Ulid::new(),
                revision: 0,
                ordinal: 0,
                generations: BTreeMap::new(),
                waiting: BTreeMap::new(),
            }),
            changed: broadcast::channel(CAPACITY).0,
        }
    }
}
impl CurrentAttention {
    pub(crate) fn remove(&self, id: Ulid) -> Result<bool> {
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        let known = state.generations.remove(&id).is_some();
        if state.waiting.remove(&id).is_none() && !known {
            return Ok(false);
        }
        state.revision = state
            .revision
            .checked_add(1)
            .ok_or_else(|| invalid("attention revision exhausted"))?;
        let _ = self.changed.send(AttentionUpdate {
            epoch: state.epoch,
            revision: state.revision,
            snapshot: false,
            seats: Vec::new(),
            removed: vec![id],
        });
        Ok(true)
    }
    pub(crate) fn episode(&self, seat: Ulid, generation: u64) -> Option<NotificationEpisode> {
        self.state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .waiting
            .get(&seat)
            .filter(|row| row.state.generation == generation)
            .and_then(|row| row.notification_episode)
    }
    /// Only a fresh validated observation may mint a key. Metadata can update
    /// or clear an existing key, but a replay-only waiting row stays unkeyed.
    pub(crate) fn update(&self, mut row: GuiSeat, live_wait: bool) -> Result<bool> {
        let id = row.record.id;
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        if row.record.generation != row.state.generation
            || state
                .generations
                .get(&id)
                .is_some_and(|generation| *generation > row.state.generation)
        {
            return Ok(false);
        }
        // Generation fences for retained/removed seats stay in the registry,
        // not an ever-growing observer tombstone map.
        let permanent = matches!(row.record.lifecycle, bbt_proto::Lifecycle::ClosedRetained)
            || matches!(row.state.process, ProcessStatus::Retained);
        let generation_changed = if permanent {
            state.generations.remove(&id).is_some()
        } else {
            state.generations.insert(id, row.state.generation) != Some(row.state.generation)
        };
        let waiting = matches!(row.record.lifecycle, bbt_proto::Lifecycle::Open)
            && matches!(row.state.process, ProcessStatus::Running { .. })
            && matches!(row.state.activity, Activity::Waiting(_));
        let mut removed = Vec::new();
        let mut seats = Vec::new();
        if waiting {
            let previous = state
                .waiting
                .get(&id)
                .and_then(|old| old.notification_episode)
                .filter(|episode| episode.generation == row.state.generation);
            row.notification_episode = if previous.is_some() || !live_wait {
                previous
            } else {
                state.ordinal = state
                    .ordinal
                    .checked_add(1)
                    .ok_or_else(|| invalid("attention episode ordinal exhausted"))?;
                Some(NotificationEpisode {
                    epoch: state.epoch,
                    generation: row.state.generation,
                    ordinal: state.ordinal,
                })
            };
            if state.waiting.get(&id).is_some_and(|old| {
                let mut comparable = old.clone();
                comparable.state.observed_ms = row.state.observed_ms;
                comparable == row
            }) {
                return Ok(false);
            }
            state.waiting.insert(id, row.clone());
            seats.push(row);
        } else if state.waiting.remove(&id).is_some() || generation_changed {
            removed.push(id);
        } else {
            return Ok(false);
        }
        state.revision = state
            .revision
            .checked_add(1)
            .ok_or_else(|| invalid("attention revision exhausted"))?;
        let update = AttentionUpdate {
            epoch: state.epoch,
            revision: state.revision,
            snapshot: false,
            seats,
            removed,
        };
        let _ = self.changed.send(update);
        Ok(true)
    }
    fn snapshot(&self) -> AttentionUpdate {
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        AttentionUpdate {
            epoch: state.epoch,
            revision: state.revision,
            snapshot: true,
            seats: state.waiting.values().cloned().collect(),
            removed: state
                .generations
                .keys()
                .filter(|id| !state.waiting.contains_key(id))
                .copied()
                .collect(),
        }
    }
    pub(crate) fn observe(self: &Arc<Self>) -> AttentionSubscription {
        // Subscribe while the producer's state is locked, then publish the
        // matching bootstrap. Revisions discard anything covered by a resync.
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        let receiver = self.changed.subscribe();
        let bootstrap = AttentionUpdate {
            epoch: state.epoch,
            revision: state.revision,
            snapshot: true,
            seats: state.waiting.values().cloned().collect(),
            removed: state
                .generations
                .keys()
                .filter(|id| !state.waiting.contains_key(id))
                .copied()
                .collect(),
        };
        AttentionSubscription {
            current: self.clone(),
            receiver,
            through: bootstrap.revision,
            bootstrap: Some(bootstrap),
        }
    }
}
pub(crate) struct AttentionSubscription {
    current: Arc<CurrentAttention>,
    receiver: broadcast::Receiver<AttentionUpdate>,
    through: u64,
    bootstrap: Option<AttentionUpdate>,
}
impl AttentionSubscription {
    pub(crate) async fn next(&mut self) -> Result<AttentionUpdate> {
        if let Some(snapshot) = self.bootstrap.take() {
            return Ok(snapshot);
        }
        loop {
            match self.receiver.recv().await {
                Ok(update) if update.revision > self.through => {
                    self.through = update.revision;
                    return Ok(update);
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let snapshot = self.current.snapshot();
                    self.through = snapshot.revision;
                    return Ok(snapshot);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(invalid("attention observer stopped"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seat(generation: u64, waiting: bool) -> GuiSeat {
        let mut record =
            bbt_proto::registry::SeatRecord::new(Ulid(1), Ulid(2), "question", "claude");
        record.generation = generation;
        GuiSeat {
            record,
            notification_episode: None,
            attention: None,
            state: bbt_proto::SeatState {
                generation,
                process: ProcessStatus::Running { pid: 7 },
                activity: if waiting {
                    Activity::Waiting(bbt_proto::WaitKind::Question)
                } else {
                    Activity::Active
                },
                grade: bbt_proto::Grade::Shim,
                title: String::new(),
                detail: Some("Which folder?".into()),
                turn_open: true,
                observed_ms: 1,
                subagents: Vec::new(),
            },
        }
    }
    #[test]
    fn live_episode_is_shared_stable_and_replay_cannot_mint() {
        let current = Arc::new(CurrentAttention::default());
        current.update(seat(1, true), false).unwrap();
        assert!(current.episode(Ulid(1), 1).is_none());
        current.update(seat(1, true), true).unwrap();
        let first = current.episode(Ulid(1), 1).unwrap();
        assert!(!current.update(seat(1, true), true).unwrap());
        assert_eq!(current.episode(Ulid(1), 1), Some(first));
        current.update(seat(1, false), false).unwrap();
        assert!(current.episode(Ulid(1), 1).is_none());
        current.update(seat(1, true), true).unwrap();
        assert_ne!(current.episode(Ulid(1), 1), Some(first));
        current.update(seat(2, true), true).unwrap();
        let second = current.episode(Ulid(1), 2).unwrap();
        assert!(!current.update(seat(1, true), true).unwrap());
        assert_eq!(current.episode(Ulid(1), 2), Some(second));
        let replacement = CurrentAttention::default();
        replacement.update(seat(2, true), true).unwrap();
        assert_ne!(replacement.episode(Ulid(1), 2).unwrap().epoch, second.epoch);
    }
    #[test]
    fn closed_catalogue_members_do_not_accumulate_generation_tombstones() {
        let current = CurrentAttention::default();
        for id in 1..=100 {
            let mut row = seat(1, true);
            row.record.id = Ulid(id);
            current.update(row.clone(), true).unwrap();
            row.record.lifecycle = bbt_proto::Lifecycle::ClosedRetained;
            row.state.process = ProcessStatus::Retained;
            current.update(row, false).unwrap();
        }
        let state = current.state.read().unwrap();
        assert!(state.generations.is_empty());
        assert!(state.waiting.is_empty());
    }
    #[tokio::test]
    async fn snapshots_cover_lag_without_replaying_stale_deltas() {
        let current = Arc::new(CurrentAttention::default());
        let mut observer = current.observe();
        let bootstrap = observer.next().await.unwrap();
        assert!(bootstrap.snapshot);
        assert_eq!(bootstrap.revision, 0);
        for _ in 0..=CAPACITY {
            current.update(seat(1, true), true).unwrap();
            current.update(seat(1, false), false).unwrap();
        }
        let recovered = observer.next().await.unwrap();
        assert!(recovered.snapshot);
        assert!(recovered.seats.is_empty());
        let through = recovered.revision;
        current.update(seat(1, true), true).unwrap();
        let next = observer.next().await.unwrap();
        assert!(!next.snapshot);
        assert_eq!(next.revision, through + 1);
        assert_eq!(next.seats.len(), 1);
        let mut second = current.observe();
        let initial = second.next().await.unwrap();
        assert_eq!(
            initial.seats[0].notification_episode,
            next.seats[0].notification_episode
        );
    }
}
