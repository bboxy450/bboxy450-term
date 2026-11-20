// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded SVG interpretation. Input never acquires a filesystem or URL resolver.
mod fonts;
mod path;
mod prepare;
mod render;
pub use fonts::{Face, FontSet};
pub use path::parse_path;
pub use prepare::{Scene, prepare};
pub use render::{Capture, Geometry, Layer, RenderRequest};

use serde::Serialize;

pub const MAX_SVG_BYTES: usize = 16_777_216;
pub const MAX_XML_NODES: usize = 50_000;
pub const MAX_DEPTH: usize = 64;
pub const MAX_SCENE_COMMANDS: usize = 262_144;
pub const MAX_GLYPHS: usize = 65_536;
pub const MAX_PNGS: usize = 16;
pub const MAX_PNG_PIXELS: u64 = 4_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Refused,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub disposition: Disposition,
    pub code: &'static str,
    pub detail: String,
}
impl Diagnostic {
    fn refused(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            disposition: Disposition::Refused,
            code,
            detail: detail.into(),
        }
    }
    fn unsupported(detail: impl Into<String>) -> Self {
        Self {
            disposition: Disposition::Unsupported,
            code: "unsupported_svg",
            detail: detail.into(),
        }
    }
}
impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}
impl std::error::Error for Diagnostic {}
pub type Result<T> = std::result::Result<T, Diagnostic>;

#[cfg(test)]
mod tests;

mod viewport;
pub use viewport::{CaptureMapping, capture_viewport};
