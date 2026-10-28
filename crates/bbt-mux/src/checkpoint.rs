// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Parser checkpoint budgets and the bounded set of immutable submitted saves.
//!
//! A slot belongs to the submitted process generation and cursor until its
//! completion arrives. Replacing a view never frees a still-running save or
//! allows its reply to advance the replacement view's checkpoint.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use crate::Ulid;

/// Decision 253 policy data, separate from the checkpoint scheduling loop.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Policy {
    pub recovery_bytes: u64,
    pub force_bytes: u64,
    pub max_age: Duration,
    pub idle: Duration,
    pub max_saves: usize,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            recovery_bytes: 8 * 1024 * 1024,
            force_bytes: 8 * 1024 * 1024,
            max_age: Duration::from_secs(60),
            idle: Duration::from_millis(200),
            max_saves: 4,
        }
    }
}
impl Policy {
    /// Observe idle expiry on the caller's clock, without waiting in a test.
    pub fn idle_due(&self, arrived: Instant, now: Instant) -> bool {
        now.saturating_duration_since(arrived) >= self.idle
    }

    pub fn for_ring(settings: &crate::ring::Settings) -> Self {
        let mut policy = Self::default();
        // D253 replaces the old default cadence. Keep a caller's non-default
        // tighter checkpoint hint (including existing congestion fixtures),
        // without shrinking the absolute R/2R debt budgets below one record.
        if settings.checkpoint_bytes != crate::ring::Settings::default().checkpoint_bytes {
            policy.force_bytes = settings.checkpoint_bytes.min(policy.recovery_bytes);
        }
        policy
    }
}

/// Arrival clocks survive replay pages and an immutable image in flight.
#[derive(Default)]
pub(crate) struct Age {
    oldest: Option<Instant>,
    after_image: Option<Instant>,
    submitted: bool,
}
impl Age {
    pub fn arrived(&mut self, now: Instant) {
        self.oldest.get_or_insert(now);
        if self.submitted {
            self.after_image.get_or_insert(now);
        }
    }
    pub fn submitted(&mut self) {
        self.submitted = true;
        self.after_image = None;
    }
    pub fn completed(&mut self, committed: bool, dirty: bool) {
        if committed {
            self.oldest = if dirty {
                self.after_image.or(self.oldest)
            } else {
                None
            };
        }
        self.submitted = false;
        self.after_image = None;
    }
    pub fn expired(&self, now: Instant, policy: Policy) -> bool {
        self.oldest
            .is_some_and(|oldest| now.saturating_duration_since(oldest) >= policy.max_age)
    }
}

/// Metadata frozen with the encoded parser image, never read from a newer view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cursor {
    pub seat: Ulid,
    pub generation: u64,
    pub sequence: u64,
    pub byte_end: u64,
}

pub(crate) struct Saves {
    limit: usize,
    generations: BTreeMap<Ulid, u64>,
    submitted: BTreeMap<Ulid, Cursor>,
    queued: VecDeque<Ulid>,
    active: Option<Ulid>,
}

impl Saves {
    pub fn new(policy: Policy) -> Self {
        assert!(policy.max_saves > 0);
        Self {
            limit: policy.max_saves,
            generations: BTreeMap::new(),
            submitted: BTreeMap::new(),
            queued: VecDeque::new(),
            active: None,
        }
    }

    pub fn generation(&mut self, seat: Ulid, generation: u64) {
        self.generations.insert(seat, generation);
    }

    pub fn retire(&mut self, seat: Ulid) {
        self.generations.remove(&seat);
    }

    pub fn start(&mut self, cursor: Cursor) -> Option<Ulid> {
        if self.submitted.len() >= self.limit
            || self.generations.get(&cursor.seat) != Some(&cursor.generation)
            || self
                .submitted
                .values()
                .any(|old| old.seat == cursor.seat && old.generation == cursor.generation)
        {
            return None;
        }
        let token = Ulid::new();
        self.submitted.insert(token, cursor);
        self.queued.push_back(token);
        Some(token)
    }

    /// V1 retains exactly one unresolved request on the leased stream. K is
    /// the admitted save-work bound, never the number of concurrent wire calls.
    pub fn dispatch(&mut self) -> Option<(Ulid, Cursor)> {
        if self.active.is_some() {
            return None;
        }
        let token = self.queued.pop_front()?;
        self.active = Some(token);
        Some((token, self.submitted[&token]))
    }

    /// Release exactly one completed task. Only its matching confirmed cursor
    /// can update a still-current view; rejection/unknown never acknowledge it.
    pub fn complete(&mut self, token: Ulid, confirmed_sequence: Option<u64>) -> Option<Cursor> {
        if self.active != Some(token) {
            return None;
        }
        self.active = None;
        let cursor = self.submitted.remove(&token)?;
        (confirmed_sequence == Some(cursor.sequence)
            && self.generations.get(&cursor.seat) == Some(&cursor.generation))
        .then_some(cursor)
    }

    pub fn in_flight(&self) -> usize {
        self.submitted.len()
    }

    pub fn active(&self) -> bool {
        self.active.is_some()
    }

    pub fn contains(&self, seat: Ulid, generation: u64) -> bool {
        self.submitted
            .values()
            .any(|c| c.seat == seat && c.generation == generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor(seat: u128, generation: u64) -> Cursor {
        Cursor {
            seat: Ulid::from(seat),
            generation,
            sequence: 7,
            byte_end: 99,
        }
    }

    // keeps: decision #253, idle expiry follows the last byte on a controlled clock.
    #[test]
    fn idle_checkpoint_requires_two_hundred_milliseconds_since_the_last_arrival() {
        let policy = Policy::default();
        let start = Instant::now();
        assert!(!policy.idle_due(start, start + Duration::from_millis(199)));
        assert!(policy.idle_due(start, start + Duration::from_millis(200)));
        let later_byte = start + Duration::from_millis(150);
        assert!(!policy.idle_due(later_byte, start + Duration::from_millis(200)));
        assert!(policy.idle_due(later_byte, start + Duration::from_millis(350)));
    }

    // keeps: decision #253
    #[test]
    fn an_explicit_earlier_checkpoint_hint_does_not_shrink_the_record_debt_budget() {
        let policy = Policy::for_ring(&crate::ring::Settings {
            checkpoint_bytes: 256,
            ..Default::default()
        });
        assert_eq!(policy.force_bytes, 256);
        assert_eq!(policy.recovery_bytes, 8 * 1024 * 1024);
        assert_eq!(
            Policy::for_ring(&crate::ring::Settings::default()).force_bytes,
            policy.recovery_bytes
        );
    }

    // keeps: decision #253
    #[test]
    fn replay_pages_do_not_reset_the_sixty_second_oldest_byte_clock() {
        let policy = Policy::default();
        let start = Instant::now();
        let mut age = Age::default();
        age.arrived(start);
        for seconds in 1..60 {
            let now = start + Duration::from_secs(seconds);
            age.arrived(now);
            assert!(!age.expired(now, policy));
        }
        assert!(age.expired(start + Duration::from_secs(60), policy));
        age.submitted();
        age.arrived(start + Duration::from_secs(61));
        // A failed/unknown save cannot rejuvenate the unconfirmed prefix.
        age.completed(false, true);
        assert!(age.expired(start + Duration::from_secs(61), policy));
        age.submitted();
        age.arrived(start + Duration::from_secs(62));
        age.completed(true, true);
        assert!(!age.expired(start + Duration::from_secs(121), policy));
        assert!(age.expired(start + Duration::from_secs(122), policy));
        age.completed(true, false);
        assert!(!age.expired(start + Duration::from_secs(1000), policy));
    }

    // keeps: decision #253
    #[test]
    fn four_occupied_slots_keep_the_next_save_queued_until_completion() {
        let mut saves = Saves::new(Policy::default());
        let cursors: Vec<_> = (1..=6).map(|seat| cursor(seat, 1)).collect();
        for c in &cursors {
            saves.generation(c.seat, c.generation);
        }
        let tokens: Vec<_> = cursors[..4]
            .iter()
            .map(|c| saves.start(*c).unwrap())
            .collect();
        assert_eq!(saves.in_flight(), 4);
        assert!(saves.start(cursors[4]).is_none());
        assert!(saves.start(cursors[5]).is_none());
        assert_eq!(saves.dispatch().unwrap().0, tokens[0]);
        assert!(saves.dispatch().is_none());
        assert_eq!(saves.complete(tokens[0], Some(7)), Some(cursors[0]));
        assert!(saves.start(cursors[4]).is_some());
        assert_eq!(saves.in_flight(), 4);
        assert!(saves.complete(tokens[0], Some(7)).is_none());
        assert_eq!(saves.in_flight(), 4);
    }

    // keeps: decision #253
    #[test]
    fn a_generation_has_only_one_immutable_submitted_cursor() {
        let mut saves = Saves::new(Policy::default());
        let old = cursor(1, 1);
        saves.generation(old.seat, 1);
        let token = saves.start(old).unwrap();
        saves.dispatch().unwrap();
        let newer = Cursor {
            sequence: 8,
            byte_end: 1234,
            ..old
        };
        assert!(saves.start(newer).is_none());
        assert_eq!(saves.complete(token, Some(7)), Some(old));
        assert!(saves.start(newer).is_some());
    }

    // keeps: decision #253
    #[test]
    fn late_old_generation_ack_does_not_commit_or_release_a_new_save() {
        let mut saves = Saves::new(Policy::default());
        let old = cursor(1, 1);
        saves.generation(old.seat, 1);
        let old_token = saves.start(old).unwrap();
        saves.dispatch().unwrap();
        let new = cursor(1, 2);
        saves.generation(new.seat, 2);
        let new_token = saves.start(new).unwrap();
        assert_eq!(saves.in_flight(), 2);
        assert!(saves.complete(old_token, Some(7)).is_none());
        assert_eq!(saves.in_flight(), 1);
        assert_eq!(saves.dispatch().unwrap().0, new_token);
        assert_eq!(saves.complete(new_token, Some(7)), Some(new));
    }

    // keeps: decision #253
    #[test]
    fn unknown_or_mismatching_completion_never_acknowledges_a_cursor() {
        let mut saves = Saves::new(Policy::default());
        let c = cursor(1, 1);
        assert!(saves.start(c).is_none());
        saves.generation(c.seat, 1);
        let unknown = saves.start(c).unwrap();
        saves.dispatch().unwrap();
        assert!(saves.complete(unknown, None).is_none());
        let mismatch = saves.start(c).unwrap();
        saves.dispatch().unwrap();
        assert!(saves.complete(mismatch, Some(8)).is_none());
        let retired = saves.start(c).unwrap();
        saves.dispatch().unwrap();
        saves.retire(c.seat);
        assert_eq!(saves.in_flight(), 1);
        assert!(saves.complete(retired, Some(7)).is_none());
        assert_eq!(saves.in_flight(), 0);
    }
}
