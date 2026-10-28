// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Token-resolved Style, stateless atoms and retained L6 component layout.
//! Renderer-free: the host supplies fonts, a viewport, paint execution and input.
pub mod atoms;
pub mod components;
pub mod cursor;
pub mod marks;
pub mod modules;
pub mod overlays;
pub mod style;
pub mod text_edit;
pub mod widgets;
pub use bbt_ui_model::{Density, Theme};
pub use style::*;

// Root-level so the invariants' `cargo test --lib <name> -- --exact` checks name them exactly.
#[cfg(test)]
// keeps: invariant rim-never-covers-text
#[test]
fn rim_never_covers_text() {
    modules::pane::rim_invariants::rim_never_covers_text();
}
#[cfg(test)]
// keeps: invariant glow-never-moves-layout
#[test]
fn glow_never_moves_layout() {
    modules::pane::rim_invariants::glow_never_moves_layout();
}
#[cfg(test)]
// keeps: invariant rim-size-is-layout
#[test]
fn rim_size_moves_only_gutter_and_inset() {
    modules::pane::rim_invariants::rim_size_moves_only_gutter_and_inset();
}
