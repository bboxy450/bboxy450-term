//! The App's one animation timeline. Operational deadlines keep their real clock.
use bbt_proto::gui::{UiClockCommand, UiClockState};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub now: Instant,
    pub time_ms: f32,
    pub state: UiClockState,
}

pub struct Clock {
    epoch: Instant,
    wall_anchor: Instant,
    logical_anchor: Instant,
    frozen: Option<Instant>,
    controlled: bool,
    revision: u64,
}
impl Default for Clock {
    fn default() -> Self {
        Self::new(Instant::now())
    }
}
impl Clock {
    fn new(now: Instant) -> Self {
        Self {
            epoch: now,
            wall_anchor: now,
            logical_anchor: now,
            frozen: None,
            controlled: false,
            revision: 0,
        }
    }
    pub fn is_frozen(&self) -> bool {
        self.frozen.is_some()
    }
    pub fn controlled(&self) -> bool {
        self.controlled
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// A frozen paint does not even sample the wall clock.
    pub fn now(&self) -> Instant {
        self.frozen.unwrap_or_else(|| self.at(Instant::now()))
    }
    fn at(&self, wall: Instant) -> Instant {
        self.frozen.unwrap_or_else(|| {
            self.logical_anchor + wall.saturating_duration_since(self.wall_anchor)
        })
    }
    pub fn frame(&self) -> Frame {
        self.frame_at(self.now())
    }
    fn frame_at(&self, now: Instant) -> Frame {
        let elapsed = now.saturating_duration_since(self.epoch);
        Frame {
            now,
            time_ms: elapsed.as_secs_f32() * 1000.,
            state: UiClockState {
                frozen: self.is_frozen(),
                time_ms: elapsed.as_millis().try_into().unwrap_or(u64::MAX),
                revision: self.revision,
            },
        }
    }
    pub fn apply(&mut self, command: UiClockCommand) -> Result<Frame, String> {
        // Advance and repeated freeze have no reason to consult real time.
        let wall = if self.is_frozen() && command != UiClockCommand::Resume {
            self.wall_anchor
        } else {
            Instant::now()
        };
        self.apply_at(command, wall)
    }
    fn apply_at(&mut self, command: UiClockCommand, wall: Instant) -> Result<Frame, String> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or("invalid_ui_clock: clock revision exhausted")?;
        match command {
            UiClockCommand::Freeze => self.frozen = Some(self.at(wall)),
            UiClockCommand::Advance { ms } => {
                let frozen = self
                    .frozen
                    .ok_or("clock_not_frozen: freeze the animation clock before advancing")?;
                let current_ms: u64 = frozen
                    .saturating_duration_since(self.epoch)
                    .as_millis()
                    .try_into()
                    .map_err(|_| "invalid_ui_clock: animation time exceeds the wire range")?;
                current_ms
                    .checked_add(ms)
                    .ok_or("invalid_ui_clock: advance exceeds the animation time range")?;
                self.frozen = Some(
                    frozen
                        .checked_add(Duration::from_millis(ms))
                        .ok_or("invalid_ui_clock: advance exceeds the animation clock range")?,
                );
            }
            UiClockCommand::Resume => {
                self.logical_anchor = self.at(wall);
                self.wall_anchor = wall;
                self.frozen = None;
            }
        }
        self.controlled = true;
        self.revision = revision;
        Ok(self.frame_at(self.at(wall)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use UiClockCommand::{Advance, Freeze, Resume};

    #[test]
    fn frozen_frames_ignore_wall_time_and_advance_exactly_to_the_swell_peak() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let frozen = clock.apply_at(Freeze, start).unwrap();
        let much_later = start + Duration::from_secs(3600);
        assert_eq!(clock.at(much_later), frozen.now);
        let peak = clock.apply_at(Advance { ms: 450 }, much_later).unwrap();
        assert_eq!(peak.now - frozen.now, Duration::from_millis(450));
        assert_eq!(peak.state.time_ms, 450);
        assert_eq!(peak.time_ms, 450.);
        let envelope = (std::f32::consts::PI * peak.time_ms / 900.).sin();
        assert!((envelope - 1.).abs() < 1e-6);
        assert_eq!(clock.at(much_later), peak.now);
    }

    #[test]
    fn resume_keeps_the_virtual_instant_and_then_follows_live_time() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        clock.apply_at(Freeze, start).unwrap();
        clock.apply_at(Advance { ms: 450 }, start).unwrap();
        let wall = start + Duration::from_secs(20);
        let resumed = clock.apply_at(Resume, wall).unwrap();
        assert!(!resumed.state.frozen);
        assert_eq!(resumed.state.time_ms, 450);
        assert_eq!(
            clock.at(wall + Duration::from_millis(23)) - resumed.now,
            Duration::from_millis(23)
        );
        let pinned = clock
            .apply_at(Freeze, wall + Duration::from_millis(23))
            .unwrap();
        assert_eq!(pinned.state.time_ms, 473);
    }

    #[test]
    fn an_unfrozen_advance_is_refused_without_changing_the_timeline() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        assert!(
            clock
                .apply_at(Advance { ms: 450 }, start)
                .unwrap_err()
                .starts_with("clock_not_frozen:")
        );
        assert!(!clock.controlled());
        assert_eq!(clock.revision(), 0);
        assert_eq!(
            clock.at(start + Duration::from_secs(1)) - start,
            Duration::from_secs(1)
        );
    }

    #[test]
    fn an_overflowing_advance_does_not_change_the_frozen_frame_or_revision() {
        let start = Instant::now();
        let mut clock = Clock::new(start);
        let before = clock
            .apply_at(Freeze, start + Duration::from_millis(1))
            .unwrap();
        assert!(
            clock
                .apply_at(Advance { ms: u64::MAX }, start)
                .unwrap_err()
                .starts_with("invalid_ui_clock:")
        );
        assert_eq!(clock.frame().state, before.state);
    }
}
