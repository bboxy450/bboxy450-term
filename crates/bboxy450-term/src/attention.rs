//! Waiting episodes derived from current typed state, never terminal text or
//! read-through history. This module changes no layout, focus or bus cursor.
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

use bboxy450_desktop_notifications::{Notification, NotificationId};
use bbt_proto::{Activity, GuiSeat, SeatId};

/// G5's paint port: a muted waiting seat retains only its ordinary quiet rim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attention {
    pub since: Instant,
    pub muted: bool,
}

/// Supplied settings, independent of the future Settings host and persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notifications {
    pub enabled: bool,
    pub disabled_harnesses: BTreeSet<String>,
    pub only_when_not_frontmost: bool,
}
impl Default for Notifications {
    fn default() -> Self {
        Self {
            enabled: true,
            disabled_harnesses: BTreeSet::new(),
            only_when_not_frontmost: false,
        }
    }
}

/// An exact current episode; an old focus intent cannot handle a later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpisodeKey {
    pub generation: u64,
    pub ordinal: u64,
}

/// A desired native notice. The ordinal distinguishes consecutive episodes
/// even when their title/body and stable native identifier are identical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub episode: EpisodeKey,
    pub notification: Notification,
}
pub type Desired = BTreeMap<NotificationId, Notice>;

#[derive(Debug, Clone)]
struct Episode {
    key: EpisodeKey,
    since: Instant,
    handled: bool,
    muted: bool,
    harness: String,
    notification: Notification,
}

/// IO-owned episode bookkeeping, bounded by the current seat catalogue.
#[derive(Debug, Clone, Default)]
pub struct Episodes {
    ordinal: u64,
    seats: BTreeMap<SeatId, Episode>,
}

impl Episodes {
    /// Apply a live current-state snapshot. Callers must not feed history pages.
    pub fn observe(&mut self, seat: &GuiSeat, now: Instant) {
        let id = SeatId::from_u128(seat.record.id.0);
        if seat.record.generation != seat.state.generation {
            return;
        }
        if !matches!(seat.record.lifecycle, bbt_proto::registry::Lifecycle::Open)
            || !matches!(seat.state.activity, Activity::Waiting(_))
        {
            self.seats.remove(&id);
            return;
        }
        if self
            .seats
            .get(&id)
            .is_some_and(|old| old.key.generation != seat.state.generation)
        {
            self.seats.remove(&id);
        }
        let notification = Notification {
            id: NotificationId(seat.record.id.to_string()),
            title: format!("{} · {}", seat.record.name, seat.record.harness_kind),
            body: first_line(seat.state.detail.as_deref()).unwrap_or_else(|| {
                match seat.state.activity {
                    Activity::Waiting(bbt_proto::WaitKind::Permission) => {
                        "Permission requested".into()
                    }
                    Activity::Waiting(bbt_proto::WaitKind::Auth) => {
                        "Authentication requested".into()
                    }
                    _ => "Your answer is needed".into(),
                }
            }),
        };
        let episode = self.seats.entry(id).or_insert_with(|| {
            self.ordinal += 1;
            Episode {
                key: EpisodeKey {
                    generation: seat.state.generation,
                    ordinal: self.ordinal,
                },
                since: now,
                handled: false,
                muted: seat.record.muted,
                harness: seat.record.harness_kind.clone(),
                notification: notification.clone(),
            }
        });
        episode.muted = seat.record.muted;
        episode.harness = seat.record.harness_kind.clone();
        episode.notification = notification;
    }

    /// Catalogue removal ends an episode even without a final state notice.
    pub fn retain(&mut self, seats: impl Iterator<Item = SeatId>) {
        let present: BTreeSet<_> = seats.collect();
        self.seats.retain(|id, _| present.contains(id));
    }

    pub fn key(&self, seat: SeatId) -> Option<EpisodeKey> {
        self.seats.get(&seat).map(|episode| episode.key)
    }

    /// Only an explicit operator action handles an episode, including one
    /// directed at an already-focused pane. A state push never does this.
    pub fn handle(&mut self, seat: SeatId, key: EpisodeKey) -> bool {
        let Some(episode) = self.seats.get_mut(&seat) else {
            return false;
        };
        if episode.key != key || episode.handled {
            return false;
        }
        episode.handled = true;
        true
    }

    pub fn paint(&self) -> BTreeMap<SeatId, Attention> {
        self.seats
            .iter()
            .filter(|(_, episode)| !episode.handled)
            .map(|(&id, episode)| {
                (
                    id,
                    Attention {
                        since: episode.since,
                        muted: episode.muted,
                    },
                )
            })
            .collect()
    }

    /// A focused frontmost seat never posts. The optional background-only
    /// setting additionally suppresses other seats while our window is active.
    pub fn notifications(
        &self,
        settings: &Notifications,
        frontmost: bool,
        focused: Option<SeatId>,
    ) -> Desired {
        self.seats
            .iter()
            .filter(|(id, episode)| {
                settings.enabled
                    && !episode.handled
                    && !episode.muted
                    && !settings.disabled_harnesses.contains(&episode.harness)
                    && !(frontmost && (focused == Some(**id) || settings.only_when_not_frontmost))
            })
            .map(|(_, episode)| {
                (
                    episode.notification.id.clone(),
                    Notice {
                        episode: episode.key,
                        notification: episode.notification.clone(),
                    },
                )
            })
            .collect()
    }
}

fn first_line(text: Option<&str>) -> Option<String> {
    let line = text?.lines().map(str::trim).find(|line| !line.is_empty())?;
    // Current detail is bounded by the adapter already. Keep this port bounded
    // too and strip control characters before showing an OS notification.
    let line: String = line.chars().filter(|c| !c.is_control()).take(192).collect();
    (!line.is_empty()).then_some(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_mux::Ulid;
    use bbt_proto::{Grade, ProcessStatus, SeatState, WaitKind, registry::SeatRecord};

    fn fixture(activity: Activity) -> GuiSeat {
        GuiSeat {
            record: SeatRecord::new(Ulid(1), Ulid(2), "fixture", "claude"),
            state: SeatState {
                generation: 1,
                process: ProcessStatus::Running { pid: 9 },
                activity,
                grade: Grade::Shim,
                title: String::new(),
                detail: Some("\n  Allow this action?\nprivate second line".into()),
                turn_open: true,
                observed_ms: 10,
                subagents: vec![],
            },
        }
    }

    #[test]
    fn one_episode_loops_until_explicit_focus_or_leaving_and_repeat_keeps_its_key() {
        let now = Instant::now();
        let id = SeatId::from_u128(1);
        let mut state = Episodes::default();
        let mut seat = fixture(Activity::Waiting(WaitKind::Question));
        state.observe(&seat, now);
        let key = state.key(id).unwrap();
        let desired = state.notifications(&Notifications::default(), false, None);
        let notice = desired.values().next().unwrap();
        assert_eq!(notice.notification.id.0, seat.record.id.to_string());
        assert_eq!(notice.notification.title, "fixture · claude");
        assert_eq!(notice.notification.body, "Allow this action?");
        state.observe(&seat, now + std::time::Duration::from_secs(1));
        assert_eq!(state.key(id), Some(key));
        assert_eq!(state.paint()[&id].since, now);
        assert!(state.handle(id, key));
        assert!(state.paint().is_empty());
        assert!(
            state
                .notifications(&Notifications::default(), false, None)
                .is_empty()
        );
        state.observe(&seat, now);
        assert!(
            state.paint().is_empty(),
            "repeated current state must not undo focus handling"
        );
        seat.state.activity = Activity::Idle;
        state.observe(&seat, now);
        assert!(state.key(id).is_none());
        seat.state.activity = Activity::Waiting(WaitKind::Permission);
        state.observe(&seat, now);
        assert_ne!(state.key(id), Some(key));
        assert!(
            !state.handle(id, key),
            "old focus cannot clear a new episode"
        );
        assert!(!state.paint().is_empty());
    }

    #[test]
    fn all_three_wait_reasons_and_generation_are_typed() {
        let now = Instant::now();
        for kind in [WaitKind::Question, WaitKind::Permission, WaitKind::Auth] {
            let mut state = Episodes::default();
            let mut seat = fixture(Activity::Waiting(kind));
            state.observe(&seat, now);
            let key = state.key(SeatId::from_u128(1)).unwrap();
            seat.record.generation = 2;
            seat.state.generation = 2;
            state.observe(&seat, now);
            assert_ne!(state.key(SeatId::from_u128(1)), Some(key));
            assert!(!state.handle(SeatId::from_u128(1), key));
            state.retain(std::iter::empty());
            assert!(state.paint().is_empty());
        }
    }

    #[test]
    fn muted_and_supplied_notification_preferences_never_post_or_change_focus() {
        let now = Instant::now();
        let id = SeatId::from_u128(1);
        let mut seat = fixture(Activity::Waiting(WaitKind::Question));
        let mut state = Episodes::default();
        seat.record.muted = true;
        state.observe(&seat, now);
        assert!(state.paint()[&id].muted);
        assert!(
            state
                .notifications(&Notifications::default(), false, None)
                .is_empty()
        );
        seat.record.muted = false;
        state.observe(&seat, now);
        assert!(
            state
                .notifications(&Notifications::default(), true, Some(id))
                .is_empty()
        );
        assert_eq!(
            state
                .notifications(&Notifications::default(), true, Some(SeatId::from_u128(9)))
                .len(),
            1
        );
        let mut settings = Notifications {
            only_when_not_frontmost: true,
            ..Default::default()
        };
        assert!(state.notifications(&settings, true, None).is_empty());
        settings.only_when_not_frontmost = false;
        settings.disabled_harnesses.insert("claude".into());
        assert!(state.notifications(&settings, false, None).is_empty());
        settings.disabled_harnesses.clear();
        settings.enabled = false;
        assert!(state.notifications(&settings, false, None).is_empty());
    }

    #[test]
    fn live_state_updates_preserve_stage_and_handling_survives_model_rebuild() {
        let seat = fixture(Activity::Waiting(WaitKind::Auth));
        let grids = crate::grids::Grids::new();
        let mut model = crate::model::build(
            std::slice::from_ref(&seat.record),
            &grids,
            &crate::model::Model::default(),
        );
        let stage = model.ui.window.stage.clone();
        model.apply_seat_states(std::slice::from_ref(&seat));
        assert_eq!(model.ui.window.stage, stage);
        let id = SeatId::from_u128(1);
        let key = model.attention_key(id);
        assert_eq!(model.attention.len(), 1);
        let landed = Instant::now();
        assert!(model.attention_focus(id, 1, key, Some(landed)));
        model = crate::model::build(std::slice::from_ref(&seat.record), &grids, &model);
        model.apply_seat_states(&[seat]);
        assert!(model.attention.is_empty());
        assert_eq!(model.focus_swell, Some((id, landed)));
        assert_eq!(model.ui.window.stage, stage);
    }
}
