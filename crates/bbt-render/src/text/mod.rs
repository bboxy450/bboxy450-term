// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Shared font settings and the legacy ASCII fixture font.
//! Current terminal cluster shaping and colour glyphs are in [`terminal`].

/// Portable glyph storage contract shared by cell text and proportional chrome.
/// Color pixels are native premultiplied BGRA, without a UI foreground tint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Mask,
    Bgra,
}

impl Format {
    pub fn channels(self) -> usize {
        match self {
            Self::Mask => 1,
            Self::Bgra => 4,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub scale: u32,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "Menlo".into(),
            size: 13.0,
            scale: 1,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FontMetrics {
    pub cell_width: u32,
    pub cell_height: u32,
    pub baseline: i16,
}

pub struct RasterizedGlyph {
    pub left: i16,
    pub top: i16,
    pub width: u32,
    pub height: u32,
    pub mask: Vec<u8>,
}

pub mod terminal;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{Font, register_font_file};

#[cfg(not(target_os = "macos"))]
pub struct Font;
#[cfg(not(target_os = "macos"))]
impl Font {
    pub fn new(_: &FontConfig) -> crate::Result<Self> {
        Err("CoreText requires macOS".into())
    }
    pub fn metrics(&self) -> FontMetrics {
        unreachable!()
    }
    pub fn effective_name(&self) -> String {
        unreachable!()
    }
    pub fn rasterize(&self, _: char) -> crate::Result<Option<RasterizedGlyph>> {
        unreachable!()
    }
}
