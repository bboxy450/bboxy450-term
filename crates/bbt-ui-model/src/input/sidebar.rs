// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure pointer capture for the sidebar's retained resize handle.

use crate::layout::Rect;

pub const MIN_WIDTH: f32 = 160.;
pub const MAX_WIDTH: f32 = 520.;

#[derive(Debug, Default)]
pub struct SidebarResize {
    grip: Option<f32>,
}

impl SidebarResize {
    /// Begin only inside the caller's solved eight-pixel handle. The grip
    /// offset keeps the sidebar edge from jumping to the press position.
    pub fn begin(&mut self, point: [f32; 2], handle: Rect, visible_width: f32) -> bool {
        self.finish();
        if ![
            point[0],
            point[1],
            visible_width,
            handle.x,
            handle.y,
            handle.w,
            handle.h,
        ]
        .into_iter()
        .all(f32::is_finite)
            || handle.w <= 0.
            || handle.h <= 0.
            || point[0] < handle.x
            || point[0] >= handle.right()
            || point[1] < handle.y
            || point[1] >= handle.bottom()
        {
            return false;
        }
        let grip = point[0] - visible_width;
        if !grip.is_finite() {
            return false;
        }
        self.grip = Some(grip);
        true
    }

    /// Captured movement may leave the original handle and window bounds.
    pub fn update(&self, x: f32) -> Option<f32> {
        if !x.is_finite() {
            return None;
        }
        Some((x - self.grip?).clamp(MIN_WIDTH, MAX_WIDTH))
    }

    pub fn finish(&mut self) {
        self.grip = None;
    }

    pub fn active(&self) -> bool {
        self.grip.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle() -> Rect {
        Rect::new(296., 36., 8., 740.)
    }

    #[test]
    fn drag_preserves_the_grip_and_captures_movement_outside_the_handle() {
        let mut drag = SidebarResize::default();
        assert!(drag.begin([298., 100.], handle(), 300.));
        assert!(drag.active());
        assert_eq!(drag.update(298.), Some(300.));
        assert_eq!(drag.update(398.), Some(400.));
        assert_eq!(drag.update(250.), Some(252.));
        drag.finish();
        assert!(!drag.active());
        assert_eq!(drag.update(400.), None);
    }

    #[test]
    fn captured_width_clamps_and_returns_from_either_limit_without_a_new_press() {
        let mut drag = SidebarResize::default();
        assert!(drag.begin([300., 100.], handle(), 300.));
        assert_eq!(drag.update(-100.), Some(MIN_WIDTH));
        assert_eq!(drag.update(1000.), Some(MAX_WIDTH));
        assert_eq!(drag.update(260.), Some(260.));
    }

    #[test]
    fn only_the_retained_handle_starts_a_drag() {
        let mut drag = SidebarResize::default();
        for point in [[295., 100.], [304., 100.], [300., 35.], [300., 776.]] {
            assert!(!drag.begin(point, handle(), 300.));
            assert!(!drag.active());
            assert_eq!(drag.update(400.), None);
        }
        assert!(drag.begin([296., 36.], handle(), 300.));
        assert_eq!(drag.update(296.), Some(300.));
    }

    #[test]
    fn non_finite_input_never_produces_a_width() {
        let mut drag = SidebarResize::default();
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(!drag.begin([invalid, 100.], handle(), 300.));
            assert!(!drag.begin([300., invalid], handle(), 300.));
            assert!(!drag.begin([300., 100.], handle(), invalid));
            assert!(!drag.active());
        }
        assert!(drag.begin([300., 100.], handle(), 300.));
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(drag.update(invalid), None);
        }
        assert!(drag.active());
        assert_eq!(drag.update(310.), Some(310.));
    }
}
