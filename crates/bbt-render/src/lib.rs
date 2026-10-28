// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Offscreen terminal rendering. The renderer consumes scenes, never a daemon `Term`.

pub mod atlas;
pub mod cells;
pub mod chrome;
pub mod device;
pub mod headless;
pub mod passes;
pub mod text;

pub use headless::{HeadlessRenderer, render_to_rgba, write_png};
pub use text::{FontConfig, FontMetrics};

pub type Color = [u8; 4];
pub type Result<T> = std::result::Result<T, RenderError>;

#[derive(Debug, Clone)]
pub struct RenderError(pub String);

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for RenderError {}
impl From<&str> for RenderError {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl From<String> for RenderError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug)]
pub struct TextRun {
    pub column: u16,
    pub row: u16,
    pub text: String,
    pub foreground: Color,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub columns: u16,
    pub rows: u16,
    /// Physical pixels per cell, after applying the scale factor.
    pub cell_size: [u32; 2],
    pub backgrounds: Vec<Color>,
    pub text: Vec<TextRun>,
    pub font: FontConfig,
}

impl Scene {
    pub fn solid(color: Color, width: u32, height: u32) -> Self {
        Self {
            columns: 1,
            rows: 1,
            cell_size: [width, height],
            backgrounds: vec![color],
            text: Vec::new(),
            font: FontConfig::default(),
        }
    }

    /// An 80×24 coloured grid with the ASCII fixture on row two.
    pub fn smoke(font: FontConfig, light: bool) -> Result<Self> {
        let metrics = text::Font::new(&font)?.metrics();
        let background = if light {
            [242, 244, 248, 255]
        } else {
            [15, 20, 29, 255]
        };
        let foreground = if light {
            [23, 31, 43, 255]
        } else {
            [232, 237, 247, 255]
        };
        let mut backgrounds = vec![background; 80 * 24];
        for row in 6..18 {
            for col in 2..78 {
                backgrounds[row * 80 + col] = [
                    (25 + col * 2) as u8,
                    (30 + row * 9) as u8,
                    (40 + (col + row) * 2) as u8,
                    255,
                ];
            }
        }
        Ok(Self {
            columns: 80,
            rows: 24,
            cell_size: [metrics.cell_width, metrics.cell_height],
            backgrounds,
            text: vec![TextRun {
                column: 3,
                row: 2,
                text: "bboxy450-term".into(),
                foreground,
            }],
            font,
        })
    }

    pub fn pixel_size(&self) -> [u32; 2] {
        [
            u32::from(self.columns) * self.cell_size[0],
            u32::from(self.rows) * self.cell_size[1],
        ]
    }
}
