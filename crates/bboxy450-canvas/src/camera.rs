// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub center: [f64; 2],
    pub zoom: f64,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            center: [0.; 2],
            zoom: 1.,
        }
    }
}
impl Camera {
    /// Fit world bounds into a logical-pixel viewport with padding on each edge.
    /// Empty content uses the default camera; invalid geometry never produces
    /// a partial camera or introduces a zoom limit.
    pub fn fit(
        bounds: impl IntoIterator<Item = crate::Bounds>,
        viewport: [f64; 2],
        padding: f64,
    ) -> Result<Self, &'static str> {
        if !padding.is_finite()
            || padding < 0.
            || viewport
                .iter()
                .any(|n| !n.is_finite() || *n <= padding * 2.)
        {
            return Err("invalid canvas fit viewport");
        }
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        let mut empty = true;
        for bounds in bounds {
            if !bounds.valid() {
                return Err("invalid canvas fit bounds");
            }
            empty = false;
            for axis in 0..2 {
                min[axis] = min[axis].min(bounds.origin[axis]);
                max[axis] = max[axis].max(bounds.origin[axis] + bounds.size[axis]);
            }
        }
        if empty {
            return Ok(Self::default());
        }
        let size = finite(std::array::from_fn(|axis| max[axis] - min[axis]))?;
        let next = Self {
            center: std::array::from_fn(|axis| min[axis] + size[axis] / 2.),
            zoom: ((viewport[0] - padding * 2.) / size[0])
                .min((viewport[1] - padding * 2.) / size[1]),
        };
        next.validate()?;
        Ok(next)
    }
    pub fn validate(self) -> Result<(), &'static str> {
        if self.center.iter().all(|v| v.is_finite()) && self.zoom.is_finite() && self.zoom > 0. {
            Ok(())
        } else {
            Err("camera requires finite center and positive finite zoom")
        }
    }
    /// Screen origin is the viewport center; hosts add their viewport offset.
    pub fn world_to_screen(self, world: [f64; 2]) -> Result<[f64; 2], &'static str> {
        self.validate()?;
        finite(std::array::from_fn(|i| {
            (world[i] - self.center[i]) * self.zoom
        }))
    }
    pub fn screen_to_world(self, screen: [f64; 2]) -> Result<[f64; 2], &'static str> {
        self.validate()?;
        finite(std::array::from_fn(|i| {
            screen[i] / self.zoom + self.center[i]
        }))
    }
    /// Drag in screen pixels. Invalid/overflowing changes leave the camera intact.
    pub fn pan(&mut self, delta: [f64; 2]) -> Result<(), &'static str> {
        self.validate()?;
        self.center = finite(std::array::from_fn(|i| {
            self.center[i] - delta[i] / self.zoom
        }))?;
        Ok(())
    }
    pub fn zoom_at(&mut self, factor: f64, screen: [f64; 2]) -> Result<(), &'static str> {
        let anchor = self.screen_to_world(screen)?;
        let mut next = Self {
            center: self.center,
            zoom: self.zoom * factor,
        };
        next.validate()?;
        next.center = finite(std::array::from_fn(|i| anchor[i] - screen[i] / next.zoom))?;
        *self = next;
        Ok(())
    }
}
fn finite(point: [f64; 2]) -> Result<[f64; 2], &'static str> {
    if point.iter().all(|n| n.is_finite()) {
        Ok(point)
    } else {
        Err("non-finite camera projection")
    }
}
pub const CULL_BELOW_PX: f64 = 1.;
pub const COLLAPSE_BELOW_PX: f64 = 24.;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    Full,
    Collapsed,
    Culled,
}
/// Size is the longest on-screen extent. Invalid/sub-pixel sizes have no ink.
pub fn semantic_zoom(size_px: f64) -> Detail {
    if !size_px.is_finite() || size_px < CULL_BELOW_PX {
        Detail::Culled
    } else if size_px < COLLAPSE_BELOW_PX {
        Detail::Collapsed
    } else {
        Detail::Full
    }
}
