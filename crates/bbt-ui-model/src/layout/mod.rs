// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
pub mod sidebar;
pub mod size;
pub mod solve;
pub mod split;

pub use sidebar::*;
pub use size::*;
pub use solve::{Box_, Child, TextMeasure, TextStyle, solve_box};
pub use split::{Divider, SplitLayout, solve_split, stage_split};
