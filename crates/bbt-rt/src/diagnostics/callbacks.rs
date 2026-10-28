// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! UI-thread CPU accounting cut at the same endpoints as a measured frame.
//! Fixed arrays retain attribution even when thousands of callbacks occur
//! between frames. Nested callbacks contribute exclusive CPU exactly once.
use std::cell::RefCell;

use super::{PHASES, Phase};

/// The fixed number of instrumented UI callback kinds.
pub const CALLBACK_KINDS: usize = Phase::WindowOther as usize + 1;
const DEPTH: usize = 16;

/// The portion of a uniquely identified callback inside a frame interval.
#[derive(Clone, Copy, Debug)]
pub struct CallbackCpuSlice {
    /// Process-thread-local callback identifier, retained across frame cuts.
    pub id: u64,
    /// Beginning of this interval in enabled-recorder-relative wall nanoseconds.
    pub from_ns: u64,
    /// End of this interval in enabled-recorder-relative wall nanoseconds.
    pub to_ns: u64,
    /// Calling thread CPU-clock reading at the interval beginning, in nanoseconds.
    pub from_cpu_ns: u64,
    /// Calling thread CPU-clock reading at the interval end, in nanoseconds.
    pub to_cpu_ns: u64,
    /// Exclusive CPU; nested callbacks lie inside the timestamps but are excluded.
    pub cpu_ns: u64,
    /// Whether the callback began before this frame interval.
    pub clipped_start: bool,
    /// Whether the callback remained open at this frame interval end.
    pub clipped_end: bool,
}

/// One callback kind's contribution to an interval, not cumulative totals.
#[derive(Clone, Copy, Debug)]
pub struct CallbackKindCpu {
    /// Stable callback phase name corresponding to its fixed kind index.
    pub phase: &'static str,
    /// Number of exclusive CPU segments accumulated in this interval.
    pub segments: u64,
    /// Thread CPU charged to this record, in nanoseconds.
    pub cpu_ns: u64,
    /// Largest callback slice of this kind observed in the interval.
    pub largest: Option<CallbackCpuSlice>,
}

/// Thread CPU between frame endpoints, including an explicit uncovered part.
#[derive(Clone, Copy, Debug)]
pub struct CallbackFrameCpu {
    /// Beginning of this interval in enabled-recorder-relative wall nanoseconds.
    pub from_ns: u64,
    /// End of this interval in enabled-recorder-relative wall nanoseconds.
    pub to_ns: u64,
    /// Calling thread CPU-clock reading at the interval beginning, in nanoseconds.
    pub from_cpu_ns: u64,
    /// Calling thread CPU-clock reading at the interval end, in nanoseconds.
    pub to_cpu_ns: u64,
    /// Thread CPU charged to this record, in nanoseconds.
    pub cpu_ns: u64,
    /// Thread CPU not attributed to one of the instrumented callback kinds.
    pub unattributed_ns: u64,
    /// Tracker or clock-read errors observed since the preceding frame cut.
    pub errors: u64,
    /// Fixed callback-kind contributions in phase-discriminant order.
    pub kinds: [CallbackKindCpu; CALLBACK_KINDS],
}

#[derive(Clone, Copy, Default)]
struct Aggregate {
    segments: u64,
    cpu_ns: u64,
    largest: Option<CallbackCpuSlice>,
}

#[derive(Clone, Copy)]
struct Active {
    id: u64,
    kind: usize,
    from_ns: u64,
    from_cpu_ns: u64,
    segment_cpu_ns: u64,
    interval_cpu_ns: u64,
    clipped_start: bool,
}

#[derive(Default)]
struct Tracker {
    next_id: u64,
    stack: [Option<Active>; DEPTH],
    depth: usize,
    totals: [Aggregate; CALLBACK_KINDS],
    previous: Option<(u64, u64)>,
    errors: u64,
    previous_errors: u64,
}

impl Tracker {
    fn pause(&mut self, cpu: u64) {
        let Some(active) = self
            .depth
            .checked_sub(1)
            .and_then(|at| self.stack[at].as_mut())
        else {
            return;
        };
        if cpu < active.segment_cpu_ns {
            self.errors = self.errors.saturating_add(1);
        }
        let elapsed = cpu.saturating_sub(active.segment_cpu_ns);
        active.interval_cpu_ns = active.interval_cpu_ns.saturating_add(elapsed);
        active.segment_cpu_ns = cpu;
        let total = &mut self.totals[active.kind];
        total.cpu_ns = total.cpu_ns.saturating_add(elapsed);
        total.segments = total.segments.saturating_add(1);
    }
    fn enter(&mut self, kind: usize, at: u64, cpu: u64) -> Option<u64> {
        if kind >= CALLBACK_KINDS || self.depth == DEPTH {
            self.errors = self.errors.saturating_add(1);
            return None;
        }
        self.pause(cpu);
        self.next_id = self.next_id.saturating_add(1);
        self.stack[self.depth] = Some(Active {
            id: self.next_id,
            kind,
            from_ns: at,
            from_cpu_ns: cpu,
            segment_cpu_ns: cpu,
            interval_cpu_ns: 0,
            clipped_start: false,
        });
        self.depth += 1;
        Some(self.next_id)
    }
    fn largest(&mut self, active: Active, at: u64, cpu: u64, clipped_end: bool) {
        let row = CallbackCpuSlice {
            id: active.id,
            from_ns: active.from_ns,
            to_ns: at,
            from_cpu_ns: active.from_cpu_ns,
            to_cpu_ns: cpu,
            cpu_ns: active.interval_cpu_ns,
            clipped_start: active.clipped_start,
            clipped_end,
        };
        let largest = &mut self.totals[active.kind].largest;
        if largest.is_none_or(|old| row.cpu_ns > old.cpu_ns) {
            *largest = Some(row);
        }
    }
    fn leave(&mut self, id: u64, at: u64, cpu: Option<u64>) {
        let Some(top) = self.depth.checked_sub(1) else {
            self.errors = self.errors.saturating_add(1);
            return;
        };
        let active = self.stack[top].expect("occupied callback stack");
        if active.id != id {
            self.errors = self.errors.saturating_add(1);
            return;
        }
        let cpu = cpu.unwrap_or_else(|| {
            self.errors = self.errors.saturating_add(1);
            active.segment_cpu_ns
        });
        self.pause(cpu);
        let active = self.stack[top].take().expect("occupied callback stack");
        self.largest(active, at, cpu, false);
        self.depth = top;
        if let Some(parent) = self
            .depth
            .checked_sub(1)
            .and_then(|at| self.stack[at].as_mut())
        {
            parent.segment_cpu_ns = cpu;
        }
    }
    fn cut(&mut self, at: u64, cpu: u64) -> Option<CallbackFrameCpu> {
        self.pause(cpu);
        for slot in 0..self.depth {
            let active = self.stack[slot].expect("occupied callback stack");
            self.largest(active, at, cpu, true);
            let active = self.stack[slot].as_mut().expect("occupied callback stack");
            active.from_ns = at;
            active.from_cpu_ns = cpu;
            active.segment_cpu_ns = cpu;
            active.interval_cpu_ns = 0;
            active.clipped_start = true;
        }
        let totals = std::mem::take(&mut self.totals);
        let previous = self.previous.replace((at, cpu));
        let mut errors = self.errors.saturating_sub(self.previous_errors);
        self.previous_errors = self.errors;
        let (from_ns, from_cpu_ns) = previous?;
        let elapsed = cpu.saturating_sub(from_cpu_ns);
        let attributed = totals
            .iter()
            .fold(0_u64, |sum, row| sum.saturating_add(row.cpu_ns));
        if attributed > elapsed || cpu < from_cpu_ns || at < from_ns {
            errors = errors.saturating_add(1);
        }
        Some(CallbackFrameCpu {
            from_ns,
            to_ns: at,
            from_cpu_ns,
            to_cpu_ns: cpu,
            cpu_ns: elapsed,
            unattributed_ns: elapsed.saturating_sub(attributed),
            errors,
            kinds: std::array::from_fn(|kind| CallbackKindCpu {
                phase: PHASES[kind].name(),
                segments: totals[kind].segments,
                cpu_ns: totals[kind].cpu_ns,
                largest: totals[kind].largest,
            }),
        })
    }
}

thread_local! {
    static TRACKER: RefCell<Tracker> = RefCell::new(Tracker::default());
}
pub(super) fn enter(phase: Phase, at: u64, cpu: u64) -> Option<u64> {
    TRACKER.with_borrow_mut(|tracker| tracker.enter(phase as usize, at, cpu))
}
pub(super) fn leave(id: u64, at: u64, cpu: Option<u64>) {
    TRACKER.with_borrow_mut(|tracker| tracker.leave(id, at, cpu));
}
pub(super) fn cut(at: u64, cpu: u64) -> Option<CallbackFrameCpu> {
    TRACKER.with_borrow_mut(|tracker| tracker.cut(at, cpu))
}
pub(super) fn failed() {
    TRACKER.with_borrow_mut(|tracker| tracker.errors = tracker.errors.saturating_add(1));
}

#[cfg(test)]
mod tests {
    use super::*;

    // keeps: spec runtime §5 and §6 — bounded opt-in measurements and callback/probe attribution
    #[test]
    fn frame_endpoints_clip_running_callbacks_and_preserve_uncovered_cpu() {
        let mut tracker = Tracker::default();
        let redraw = tracker.enter(Phase::WindowRedraw as usize, 10, 5).unwrap();
        assert!(tracker.cut(20, 10).is_none());
        tracker.leave(redraw, 30, Some(14));
        let wake = tracker.enter(Phase::UserWake as usize, 40, 16).unwrap();
        tracker.leave(wake, 50, Some(20));
        let next = tracker.enter(Phase::WindowRedraw as usize, 60, 22).unwrap();
        let frame = tracker.cut(70, 30).unwrap();
        assert_eq!(
            (frame.from_cpu_ns, frame.to_cpu_ns, frame.cpu_ns),
            (10, 30, 20)
        );
        assert_eq!(frame.unattributed_ns, 4);
        assert_eq!(frame.errors, 0);
        let row = frame.kinds[Phase::WindowRedraw as usize];
        assert_eq!(row.cpu_ns, 12);
        let largest = row.largest.unwrap();
        assert_eq!(
            (largest.id, largest.from_ns, largest.to_ns, largest.cpu_ns),
            (next, 60, 70, 8)
        );
        assert!(!largest.clipped_start && largest.clipped_end);
        assert_eq!(frame.kinds[Phase::UserWake as usize].cpu_ns, 4);
        tracker.leave(next, 80, Some(33));
        let tail = tracker.cut(90, 35).unwrap();
        assert_eq!(tail.kinds[Phase::WindowRedraw as usize].cpu_ns, 3);
        assert!(
            tail.kinds[Phase::WindowRedraw as usize]
                .largest
                .unwrap()
                .clipped_start
        );
        assert_eq!(tail.unattributed_ns, 2);
    }

    // keeps: spec runtime §5 and §6 — bounded opt-in measurements and callback/probe attribution
    #[test]
    fn nested_callbacks_count_exclusive_cpu_once() {
        let mut tracker = Tracker::default();
        tracker.cut(1, 0);
        let outer = tracker.enter(Phase::WindowRedraw as usize, 2, 0).unwrap();
        let inner = tracker.enter(Phase::UserWake as usize, 3, 2).unwrap();
        tracker.leave(inner, 4, Some(5));
        tracker.leave(outer, 5, Some(8));
        let frame = tracker.cut(6, 10).unwrap();
        assert_eq!(frame.kinds[Phase::WindowRedraw as usize].cpu_ns, 5);
        assert_eq!(frame.kinds[Phase::UserWake as usize].cpu_ns, 3);
        assert_eq!(frame.unattributed_ns, 2);
        assert_eq!(frame.errors, 0);
    }

    // keeps: spec runtime §5 and §6 — bounded opt-in measurements and callback/probe attribution
    #[test]
    fn excessive_callback_nesting_is_bounded_and_explicit() {
        let mut tracker = Tracker::default();
        tracker.cut(1, 0);
        let ids: Vec<_> = (0..DEPTH)
            .map(|n| {
                tracker
                    .enter(Phase::UserWake as usize, n as u64 + 2, 0)
                    .unwrap()
            })
            .collect();
        assert!(tracker.enter(Phase::UserWake as usize, 30, 0).is_none());
        for id in ids.into_iter().rev() {
            tracker.leave(id, 31, Some(0));
        }
        assert_eq!(tracker.cut(32, 0).unwrap().errors, 1);
        assert_eq!(tracker.depth, 0);
    }
}
