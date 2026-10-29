// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Scoped chat tail following. Layout never overwrites a requested manual offset.
use bbt_ui_model::SwarmId;
use std::collections::VecDeque;

pub const MAX_VIEWS: usize = 128;
const MAX_CHANNEL_BYTES: usize = 512;

#[derive(Debug, Clone, Copy)]
struct Position {
    requested: f32,
    following: bool,
}
impl Default for Position {
    fn default() -> Self {
        Self {
            requested: 0.,
            following: true,
        }
    }
}
#[derive(Debug, Clone)]
struct View {
    scope: SwarmId,
    channel: String,
    position: Position,
}
/// The active view is the most recently bound entry. No historic message data is retained.
#[derive(Debug, Default)]
pub struct State {
    views: VecDeque<View>,
    active: bool,
}
impl State {
    /// Returns whether the selected view changed. An invalid key clears the active
    /// binding, so another swarm's manual offset can never leak into that view.
    pub fn bind(&mut self, scope: SwarmId, channel: &str) -> Result<bool, &'static str> {
        if scope == SwarmId::from_u128(0)
            || channel.len() > MAX_CHANNEL_BYTES
            || channel.chars().any(char::is_control)
        {
            self.active = false;
            return Err("invalid_chat_scroll_scope");
        }
        if self.active
            && self
                .views
                .back()
                .is_some_and(|view| view.scope == scope && view.channel == channel)
        {
            return Ok(false);
        }
        let view = self
            .views
            .iter()
            .position(|view| view.scope == scope && view.channel == channel)
            .and_then(|index| self.views.remove(index))
            .unwrap_or_else(|| View {
                scope,
                channel: channel.into(),
                position: Position::default(),
            });
        if self.views.len() == MAX_VIEWS {
            self.views.pop_front();
        }
        self.views.push_back(view);
        self.active = true;
        Ok(true)
    }
    fn position(&self) -> Position {
        self.views
            .back()
            .filter(|_| self.active)
            .map_or_else(Position::default, |view| view.position)
    }
    pub fn following(&self) -> bool {
        self.position().following
    }
    pub fn requested_offset(&self) -> f32 {
        self.position().requested
    }
    /// The retained ScrollView clamps this finite sentinel to its newly solved maximum.
    /// Manual positions remain requested through empty history, loading and reflow.
    pub fn layout_offset(&self) -> f32 {
        let position = self.position();
        if position.following {
            f32::MAX
        } else {
            position.requested
        }
    }
    /// Positive logical pixels move down. Use the actual clamped offset and maximum
    /// from the produced ScrollView, not a previously requested loading/reflow offset.
    pub fn wheel(&mut self, actual: f32, max: f32, dy: f32) -> bool {
        if !self.active || !actual.is_finite() || !max.is_finite() || !dy.is_finite() || dy == 0. {
            return false;
        }
        let max = max.max(0.);
        let requested = (actual.clamp(0., max) + dy).clamp(0., max);
        let following = requested == max;
        let position = &mut self.views.back_mut().expect("active view exists").position;
        let changed = position.requested != requested || position.following != following;
        *position = Position {
            requested,
            following,
        };
        changed
    }
    /// Any explicit send returns this view to following its latest message.
    pub fn follow_latest(&mut self) -> bool {
        if !self.active {
            return false;
        }
        let view = self.views.back_mut().expect("active view exists");
        let changed = !view.position.following || view.position.requested != 0.;
        view.position = Position::default();
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope(id: u128) -> SwarmId {
        SwarmId::from_u128(id)
    }
    #[test]
    fn tail_follows_append_and_manual_scroll_stays_put_until_bottom_or_send() {
        let mut state = State::default();
        state.bind(scope(1), "all").unwrap();
        assert!(state.following());
        assert_eq!(state.layout_offset().clamp(0., 100.), 100.);
        assert_eq!(
            state.layout_offset().clamp(0., 200.),
            200.,
            "a larger message tail follows"
        );
        assert!(state.wheel(200., 200., -25.));
        assert_eq!(state.layout_offset(), 175.);
        assert!(!state.following());
        assert_eq!(
            state.layout_offset().clamp(0., 300.),
            175.,
            "an incoming append cannot pull a manual view down"
        );
        state.wheel(175., 300., 125.);
        assert!(state.following());
        state.wheel(300., 300., -50.);
        assert!(state.follow_latest());
        assert!(state.following());
    }
    #[test]
    fn swarm_and_channel_positions_are_isolated_and_restored() {
        let mut state = State::default();
        state.bind(scope(1), "all").unwrap();
        state.wheel(100., 100., -25.);
        state.bind(scope(2), "all").unwrap();
        assert!(state.following());
        state.wheel(100., 100., -80.);
        state.bind(scope(1), "channels/review").unwrap();
        assert!(state.following());
        state.wheel(100., 100., -55.);
        state.bind(scope(1), "all").unwrap();
        assert_eq!(state.requested_offset(), 75.);
        state.bind(scope(2), "all").unwrap();
        assert_eq!(state.requested_offset(), 20.);
        state.bind(scope(1), "channels/review").unwrap();
        assert_eq!(state.requested_offset(), 45.);
        assert!(!state.bind(scope(1), "channels/review").unwrap());
    }
    #[test]
    fn loading_and_reflow_clamp_only_the_display_and_zero_wheel_does_not_reset_intent() {
        let mut state = State::default();
        state.bind(scope(1), "all").unwrap();
        state.wheel(100., 100., -20.);
        assert_eq!(state.layout_offset().clamp(0., 0.), 0.);
        assert_eq!(state.requested_offset(), 80.);
        assert!(!state.wheel(0., 0., 0.));
        assert!(!state.following());
        assert_eq!(state.layout_offset().clamp(0., 50.), 50.);
        assert_eq!(state.requested_offset(), 80.);
        assert_eq!(state.layout_offset().clamp(0., 200.), 80.);
        state.wheel(50., 50., -10.);
        assert_eq!(
            state.requested_offset(),
            40.,
            "a new gesture starts from displayed geometry"
        );
    }
    #[test]
    fn wheel_bounds_and_nonfinite_input_cannot_corrupt_scope_state() {
        let mut state = State::default();
        assert!(!state.wheel(0., 100., 1.));
        state.bind(scope(1), "all").unwrap();
        state.wheel(100., 100., -0.25);
        assert_eq!(state.requested_offset(), 99.75);
        assert!(!state.following());
        for (actual, max, dy) in [
            (f32::NAN, 100., 1.),
            (0., f32::INFINITY, 1.),
            (0., 100., f32::NAN),
            (0., 100., f32::NEG_INFINITY),
        ] {
            assert!(!state.wheel(actual, max, dy));
            assert_eq!(state.requested_offset(), 99.75);
        }
        state.wheel(200., 100., -150.);
        assert_eq!(state.requested_offset(), 0.);
        state.wheel(0., 100., f32::MAX);
        assert!(state.following());
        assert_eq!(state.requested_offset(), 100.);
    }
    #[test]
    fn invalid_bind_clears_active_scope_without_erasing_its_saved_position() {
        let mut state = State::default();
        state.bind(scope(1), "all").unwrap();
        state.wheel(100., 100., -30.);
        for (scope, channel) in [
            (scope(0), "all".into()),
            (scope(2), "x".repeat(513)),
            (scope(2), "bad\nchannel".into()),
        ] {
            assert!(state.bind(scope, &channel).is_err());
            assert!(state.following());
            assert_eq!(state.requested_offset(), 0.);
            assert!(!state.wheel(0., 100., 10.));
        }
        state.bind(scope(1), "all").unwrap();
        assert_eq!(state.requested_offset(), 70.);
    }
    #[test]
    fn retained_128_view_lru_is_bounded_and_recent_scope_survives_eviction() {
        let mut state = State::default();
        for id in 1..=MAX_VIEWS as u128 {
            state.bind(scope(id), "all").unwrap();
            state.wheel(10., 100., -1.);
        }
        state.bind(scope(1), "all").unwrap();
        state.bind(scope(129), "all").unwrap();
        assert_eq!(state.views.len(), MAX_VIEWS);
        state.bind(scope(1), "all").unwrap();
        assert_eq!(state.requested_offset(), 9.);
        state.bind(scope(2), "all").unwrap();
        assert!(state.following(), "least recently used scope was evicted");
        for id in 0..1024 {
            state.bind(scope(7), &format!("channels/{id}")).unwrap();
        }
        assert_eq!(state.views.len(), MAX_VIEWS);
    }
}
