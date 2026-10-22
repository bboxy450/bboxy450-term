//! Token-resolved Style, stateless atoms and retained L6 component layout.
//! Renderer-free: the host supplies fonts, a viewport, paint execution and input.
pub mod atoms;
pub mod components;
pub mod style;
pub mod widgets;
pub use bbt_ui_model::{Density, Theme};
pub use style::*;
