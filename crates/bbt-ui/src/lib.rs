//! Token-resolved Style, stateless atoms and retained L6 component layout.
//! Renderer-free: the host supplies fonts, a viewport, paint execution and input.
pub mod atoms;
pub mod components;
pub mod modules;
pub mod overlays;
pub mod style;
pub mod widgets;
pub use bbt_ui_model::{Density, Theme};
pub use style::*;

// Root-level so the invariants' `cargo test --lib <name> -- --exact` checks name them exactly.
#[cfg(test)]
#[test]
fn rim_never_covers_text() {
    modules::pane::rim_invariants::rim_never_covers_text();
}
#[cfg(test)]
#[test]
fn glow_never_moves_layout() {
    modules::pane::rim_invariants::glow_never_moves_layout();
}
#[cfg(test)]
#[test]
fn rim_size_moves_only_gutter_and_inset() {
    modules::pane::rim_invariants::rim_size_moves_only_gutter_and_inset();
}
