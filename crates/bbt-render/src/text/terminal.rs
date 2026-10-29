// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Cell-aligned CoreText shaping with styled faces and colour emoji.
#[cfg(not(target_os = "macos"))]
use crate::{FontConfig, FontMetrics, Result};

pub use super::Format;

pub struct Glyph {
    pub width: u32,
    pub height: u32,
    pub left: i16,
    pub top: i16,
    pub format: Format,
    pub pixels: Vec<u8>,
    pub faces: Vec<String>,
}

#[cfg(target_os = "macos")]
#[path = "terminal_macos.rs"]
mod platform;
#[cfg(target_os = "macos")]
pub use platform::Fonts;

#[cfg(not(target_os = "macos"))]
pub struct Fonts;
#[cfg(not(target_os = "macos"))]
impl Fonts {
    pub fn new(_: &FontConfig, _: Option<f32>, _: bool) -> Result<Self> {
        Err("terminal fonts require CoreText on macOS".into())
    }
    pub fn metrics(&self) -> FontMetrics {
        unreachable!()
    }
    pub fn rasterize(&self, _: &str, _: bool, _: bool, _: u8) -> Result<Option<Glyph>> {
        unreachable!()
    }
}
