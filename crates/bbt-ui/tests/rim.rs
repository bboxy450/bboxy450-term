// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The alive rim as a kit capability (rim-glow §8): blocks that are not panes carry it by
//! default, with the lab's rules, and their phase is the lab's hash of the block's key.
use bbt_ui::{
    GlowActivity, Rim,
    components::{self as c, Paint},
    modules::{
        mini_grid::Node,
        seat_row::SeatRow,
        tab_row::{TabRow, TabSeat},
    },
    rim_phase,
};
use bbt_ui_model::{
    Activity, TOKENS,
    layout::{Rect, TextMeasure, TextStyle},
};

struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, t: &str, s: &TextStyle) -> f32 {
        t.chars().count() as f32 * s.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, s: &TextStyle) -> [f32; 2] {
        [s.size * 0.8, s.size * 0.2]
    }
}

fn ctx() -> bbt_ui::Context {
    bbt_ui::Context::new(bbt_ui::Theme::Dark, bbt_ui::Density::Cozy)
}
fn rims(element: c::Element) -> Vec<Rim> {
    let layout = element
        .layout(Rect::new(0., 0., 300., 0.), ctx(), &Metrics)
        .unwrap();
    c::paint(&layout, ctx())
        .into_iter()
        .filter_map(|op| match op {
            Paint::Material {
                kind: bbt_ui::Material::Rim,
                rim: Some(r),
                ..
            } => Some(r),
            _ => None,
        })
        .collect()
}
fn row(key: &str, activity: Activity, focused: bool) -> SeatRow {
    let mut r = SeatRow::new(key, "Rook Dusk", "pi");
    r.activity = activity;
    r.focused = focused;
    r
}

// keeps: spec rim-glow §6
#[test]
fn the_phase_is_the_labs_hash32_of_the_key() {
    // hash32 (lab/web/components.js:8) % 10000, computed independently.
    assert_eq!(rim_phase("k16fcaae3"), 2400);
    assert_eq!(rim_phase("gpu research"), 1670);
    assert_eq!(rim_phase("sw0"), 7373);
    assert_eq!(rim_phase("Vesper Greywind"), 9681);
}

// keeps: spec rim-glow §6
#[test]
fn seat_rows_carry_the_rim_edge_to_edge_and_idle_rows_are_bare_at_rest() {
    for (activity, focused, alpha) in [
        (Activity::Active, false, 1.),
        (Activity::Idle, true, 1.),
        (Activity::Errored, false, 1.),
        (
            Activity::Idle,
            false,
            if TOKENS.glow.row_idle { 1. } else { 0. },
        ),
    ] {
        let found = rims(row("k16fcaae3", activity, focused).element(ctx()));
        assert_eq!(found.len(), 1, "{activity:?}: one rim per row");
        let r = found[0];
        assert_eq!(r.rim_alpha, Some(alpha), "{activity:?} focused={focused}");
        assert_eq!(
            (r.outer, r.depth),
            (Some(0.), Some(0.)),
            "no outer half, no bloom"
        );
        assert_eq!(r.seed, 2400);
        assert_eq!(r.activity, GlowActivity::from(activity));
    }
    // The host's swell and attention reach the row's rim.
    let mut r = row("k16fcaae3", Activity::Idle, false);
    (r.swell, r.attention) = (0.7, true);
    let found = rims(r.element(ctx()));
    assert_eq!((found[0].swell, found[0].attention), (0.7, true));
}

// keeps: spec rim-glow §6
#[test]
fn a_tab_reads_its_worst_seat_and_swells_with_its_seats() {
    let seat = |key: &str, activity| TabSeat {
        row: row(key, activity, false),
        children: vec![],
    };
    let mut tab = TabRow {
        name: "gpu research".into(),
        shortcut: None,
        tree: Node::Cell {
            key: "a".into(),
            name: "a".into(),
            activity: Activity::Active,
        },
        seats: vec![
            seat("a", Activity::Active),
            seat("b", Activity::Errored),
            seat("c", Activity::Idle),
        ],
        expanded: false,
        focused_key: None,
    };
    tab.seats[2].row.swell = 0.5;
    let found = rims(tab.element(ctx()));
    let header = found[0];
    assert_eq!(header.activity, GlowActivity::Errored, "the worst seat");
    assert_eq!(header.seed, 1670, "hash32 of the tab name");
    assert_eq!(header.swell, 0.5, "a seat's swell is the tab's");
    assert_eq!((header.outer, header.depth), (Some(0.), Some(0.)));
}
