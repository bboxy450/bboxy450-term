//! Pure interaction state. The caller supplies solved geometry and monotonic
//! event times; physical key translation and all effects belong to the app.

mod focus;
mod keys;

pub use focus::{Direction, Focus, FocusOrigin};
pub use keys::{FocusContext, KeyRoute, Modifiers, Navigation, NavigationKey, route_navigation};
