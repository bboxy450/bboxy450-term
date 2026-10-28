//! Pure interaction state. The caller supplies solved geometry and monotonic
//! event times; physical key translation and all effects belong to the app.
//! Origin: bboxy450-term, MIT OR Apache-2.0. Retain credit on reuse:
//! https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

mod focus;
mod keys;
mod sidebar;

pub use focus::{Direction, Focus, FocusOrigin};
pub use keys::{FocusContext, KeyRoute, Modifiers, Navigation, NavigationKey, route_navigation};
pub use sidebar::{MAX_WIDTH, MIN_WIDTH, SidebarResize};
