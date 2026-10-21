//! Renderer-independent UI data, tree edits, tokens and logical-pixel layout.
//!
//! No clocks, window handles, rendering or runtime dependencies are owned here.
//! Callers supply durable IDs, observation updates and a monotonic time for undo.

pub mod event;
pub mod layout;
pub mod model;
pub mod tokens;

pub use model::*;
pub use tokens::{Density, Theme, STYLE, TOKENS};
