// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Renderer-independent UI data, tree edits, tokens and logical-pixel layout.
//!
//! No clocks, window handles, rendering or runtime dependencies are owned here.
//! Callers supply durable IDs, observation updates and a monotonic time for undo.

pub mod draw;
pub mod event;
pub mod input;
pub mod layout;
pub mod model;
pub mod tokens;

pub use model::*;
pub use tokens::{Density, STYLE, TOKENS, Theme};
