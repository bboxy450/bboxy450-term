// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! L7 modules compose retained components; they never emit draw primitives.
pub mod accordion;
pub mod canvas;
pub mod closed_strip;
pub mod composer;
pub mod e2e_checklist;
pub mod graph_tab;
pub mod harness;
pub mod launcher;
pub mod launcher_banner;
pub mod link;
pub mod mini_grid;
pub mod pane;
pub mod perf_hud;
pub mod seat_row;
pub mod sidebar;
pub mod stream_dock;
pub mod strip;
pub mod swarm_card;
pub mod tab_row;
pub mod title_bar;
pub mod startup;

pub mod document_modal;
pub mod settings;

pub mod transcript;
pub mod diff;
pub mod tool_call;
pub mod approval;

// Moved from document_modal/diagram.rs::arrow; shared by document and canvas.
// bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
pub(crate) fn arrow_points(
    tip: [f32; 2],
    direction: [f32; 2],
    weight: f32,
) -> Vec<bbt_ui_model::draw::PathCommand> {
    use bbt_ui_model::draw::PathCommand as P;
    let u = weight * 0.7;
    let base = [
        tip[0] - direction[0] * 8. * u,
        tip[1] - direction[1] * 8. * u,
    ];
    let normal = [-direction[1] * 3.5 * u, direction[0] * 3.5 * u];
    let a = [base[0] + normal[0], base[1] + normal[1]];
    let b = [base[0] - normal[0], base[1] - normal[1]];
    vec![
        P::Move(a),
        P::Line(tip),
        P::Line(b),
        P::Line(a),
        P::Move(base),
        P::Line(tip),
    ]
}
