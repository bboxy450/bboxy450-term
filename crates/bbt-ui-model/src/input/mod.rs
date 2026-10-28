// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure interaction state. The caller supplies solved geometry and monotonic
//! event times; physical key translation and all effects belong to the app.

mod focus;
mod keys;
mod sidebar;

pub use focus::{Direction, Focus, FocusOrigin};
pub use keys::{FocusContext, KeyRoute, Modifiers, Navigation, NavigationKey, route_navigation};
pub use sidebar::{MAX_WIDTH, MIN_WIDTH, SidebarResize};
