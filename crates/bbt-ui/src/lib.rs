//! L3 token-resolved Style and L5 stateless atom painters.
//! No renderer, layout tree, window, clock or input ownership.
pub mod atoms;
pub mod style;
pub use bbt_ui_model::{Density, Theme};
pub use style::*;
