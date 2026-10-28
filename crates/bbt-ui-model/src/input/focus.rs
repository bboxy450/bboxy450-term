//! Origin: bboxy450-term, MIT OR Apache-2.0. Retain credit on reuse:
//! https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use std::time::{Duration, Instant};

use crate::{
    EditError, Lifecycle, Model, SeatId,
    event::{Event, LocateWhere},
    layout::Rect,
};

const DWELL: Duration = Duration::from_millis(150);
const TYPING_GUARD: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusOrigin {
    Row,
    Pane,
    Keyboard,
}

/// Stage remains the sole seat-focus authority. This controller retains only
/// input timing and one Locate intent waiting for the new stage's layout.
#[derive(Debug, Clone, Default)]
pub struct Focus {
    pointer: Option<[f32; 2]>,
    hovered: Option<SeatId>,
    dwell_since: Option<Instant>,
    last_key: Option<Instant>,
    pending_locate: Option<(SeatId, LocateWhere)>,
}

fn current_seats(model: &Model) -> Vec<SeatId> {
    model
        .window
        .sidebar
        .swarms
        .iter()
        .find(|s| Some(s.id) == model.window.stage.swarm)
        .and_then(|s| s.tabs.iter().find(|t| Some(t.id) == model.window.stage.tab))
        .map_or_else(Vec::new, |tab| tab.tree.leaves())
}

fn contains(rect: Rect, at: [f32; 2]) -> bool {
    rect.w > 0.
        && rect.h > 0.
        && at[0] >= rect.x
        && at[0] < rect.right()
        && at[1] >= rect.y
        && at[1] < rect.bottom()
}

impl Focus {
    /// Record every key, including keys passed through to a terminal. This
    /// function consumes no key and never changes Stage or text focus.
    pub fn key_pressed(&mut self, now: Instant) {
        self.last_key = Some(now);
    }

    pub fn activate(
        &mut self,
        model: &mut Model,
        seat: SeatId,
        origin: FocusOrigin,
        now: Instant,
    ) -> Result<Vec<Event>, EditError> {
        let where_ = match origin {
            FocusOrigin::Row => LocateWhere::Pane,
            FocusOrigin::Pane => LocateWhere::Row,
            FocusOrigin::Keyboard => LocateWhere::Pane,
        };
        let events = self.change(model, seat, Some(where_))?;
        self.dwell_since = None;
        if origin == FocusOrigin::Keyboard {
            self.key_pressed(now);
        }
        Ok(events)
    }

    fn change(
        &mut self,
        model: &mut Model,
        seat: SeatId,
        locate: Option<LocateWhere>,
    ) -> Result<Vec<Event>, EditError> {
        let record = model.seats.get(&seat).ok_or(EditError::MissingSeat)?;
        if record.lifecycle != Lifecycle::Open {
            return Err(EditError::NotOpen);
        }
        let awaiting_layout = self.pending_locate.is_some();
        let previous = model.window.stage.clone();
        model.focus(seat)?;
        let stage_switched =
            previous.tab != model.window.stage.tab || previous.swarm != model.window.stage.swarm;
        let changed = previous.focused != Some(seat) || stage_switched;
        self.pending_locate = None;
        let mut events = vec![];
        if changed {
            events.push(Event::FocusChanged {
                seat,
                prev: previous.focused,
                stage_switched,
            });
        }
        // Explicit activation reads the seat even if it was already focused.
        // Dwell only reads it when focus actually changes.
        if (changed || locate.is_some())
            && model.seats.get_mut(&seat).unwrap().unread.take().is_some()
        {
            events.push(Event::UnreadCleared { seat });
        }
        if let Some(where_) = locate {
            if stage_switched || awaiting_layout {
                self.pending_locate = Some((seat, where_));
            } else {
                events.push(Event::Locate { seat, where_ });
            }
        }
        Ok(events)
    }

    /// Semantic pane cycling. The app chooses its physical chord; terminals
    /// keep ordinary Tab. Order comes from the split tree, not insertion order.
    pub fn cycle(
        &mut self,
        model: &mut Model,
        backwards: bool,
        now: Instant,
    ) -> Result<Vec<Event>, EditError> {
        self.key_pressed(now);
        let seats = current_seats(model);
        if seats.is_empty() {
            return Ok(vec![]);
        }
        let index = model
            .window
            .stage
            .focused
            .and_then(|s| seats.iter().position(|id| *id == s));
        let next = match (index, backwards) {
            (Some(i), false) => (i + 1) % seats.len(),
            (Some(i), true) => (i + seats.len() - 1) % seats.len(),
            (None, false) => 0,
            (None, true) => seats.len() - 1,
        };
        self.activate(model, seats[next], FocusOrigin::Keyboard, now)
    }

    /// Same edge eligibility and along + 2*across score as the lab. Equal scores
    /// keep split-tree order even if the caller's rectangle map is reordered.
    pub fn navigate(
        &mut self,
        model: &mut Model,
        panes: &[(SeatId, Rect)],
        direction: Direction,
        now: Instant,
    ) -> Result<Vec<Event>, EditError> {
        self.key_pressed(now);
        let Some(current) = model.window.stage.focused else {
            return Ok(vec![]);
        };
        let Some((_, me)) = panes.iter().find(|(id, _)| *id == current) else {
            return Ok(vec![]);
        };
        let centre = [me.x + me.w * 0.5, me.y + me.h * 0.5];
        let mut best = None;
        let mut score = f32::INFINITY;
        for id in current_seats(model) {
            if id == current {
                continue;
            }
            let Some((_, r)) = panes.iter().find(|(seat, _)| *seat == id) else {
                continue;
            };
            let dx = r.x + r.w * 0.5 - centre[0];
            let dy = r.y + r.h * 0.5 - centre[1];
            let ahead = match direction {
                Direction::Right => dx > 0. && r.x >= me.right() - 2.,
                Direction::Left => dx < 0. && r.right() <= me.x + 2.,
                Direction::Down => dy > 0. && r.y >= me.bottom() - 2.,
                Direction::Up => dy < 0. && r.bottom() <= me.y + 2.,
            };
            let next = match direction {
                Direction::Left | Direction::Right => dx.abs() + 2. * dy.abs(),
                Direction::Up | Direction::Down => dy.abs() + 2. * dx.abs(),
            };
            if ahead && next < score {
                best = Some(id);
                score = next;
            }
        }
        match best {
            Some(id) => self.activate(model, id, FocusOrigin::Keyboard, now),
            None => Ok(vec![]),
        }
    }

    pub fn pointer_moved(&mut self, panes: &[(SeatId, Rect)], at: [f32; 2], now: Instant) {
        self.pointer = Some(at);
        self.refresh_hover(panes, now);
    }

    pub fn pointer_left(&mut self) {
        self.pointer = None;
        self.hovered = None;
        self.dwell_since = None;
    }

    fn refresh_hover(&mut self, panes: &[(SeatId, Rect)], now: Instant) {
        let next = self.pointer.and_then(|at| {
            panes
                .iter()
                .rev()
                .find(|(_, rect)| contains(*rect, at))
                .map(|(id, _)| *id)
        });
        if next != self.hovered {
            self.hovered = next;
            self.dwell_since = next.map(|_| now);
        }
    }

    /// Call after solving the current stage. A cross-tab click cannot Locate a
    /// pane until that pane exists in this layout. Daemon focus changes cancel
    /// stale intent rather than creating a Locate of their own.
    pub fn layout_ready(
        &mut self,
        model: &Model,
        panes: &[(SeatId, Rect)],
        now: Instant,
    ) -> Vec<Event> {
        self.refresh_hover(panes, now);
        let Some((seat, where_)) = self.pending_locate else {
            return vec![];
        };
        if model.window.stage.focused != Some(seat) || !current_seats(model).contains(&seat) {
            self.pending_locate = None;
            return vec![];
        }
        if panes
            .iter()
            .any(|(id, r)| *id == seat && r.w > 0. && r.h > 0.)
        {
            self.pending_locate = None;
            vec![Event::Locate { seat, where_ }]
        } else {
            vec![]
        }
    }

    pub fn tick(
        &mut self,
        model: &mut Model,
        panes: &[(SeatId, Rect)],
        now: Instant,
    ) -> Result<Vec<Event>, EditError> {
        self.refresh_hover(panes, now);
        let Some(since) = self.dwell_since else {
            return Ok(vec![]);
        };
        if now
            .checked_duration_since(since)
            .is_none_or(|elapsed| elapsed < DWELL)
        {
            return Ok(vec![]);
        }
        self.dwell_since = None; // A blocked dwell is not retried without re-entry.
        if self.last_key.is_some_and(|key| {
            now.checked_duration_since(key)
                .is_none_or(|elapsed| elapsed <= TYPING_GUARD)
        }) {
            return Ok(vec![]);
        }
        let Some(seat) = self.hovered.filter(|id| current_seats(model).contains(id)) else {
            return Ok(vec![]);
        };
        self.change(model, seat, None)
    }
}
