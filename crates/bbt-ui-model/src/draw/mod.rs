// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Retained, renderer-independent chrome commands in logical pixels.
pub mod atoms;
pub mod ctx;
pub mod list;
pub mod path;
pub mod prim;
pub use atoms::*;
pub use ctx::*;
pub use list::*;
pub use path::*;
pub use prim::*;
