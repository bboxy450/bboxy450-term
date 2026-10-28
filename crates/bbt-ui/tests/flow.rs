// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, s: &str, t: &TextStyle) -> f32 {
        s.chars().count() as f32 * t.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, t: &TextStyle) -> [f32; 2] {
        [t.size * 0.8, t.size * 0.2]
    }
}
fn ctx() -> ui::Context {
    ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)
}
fn text(value: &str, size: f32) -> c::Element {
    c::Element::Text(c::Text::new(
        value,
        ui::Text {
            size: Some(size),
            ..Default::default()
        },
    ))
}
fn layout(e: c::Element, w: f32) -> c::Layout {
    e.layout(Rect::new(0., 0., w, 0.), ctx(), &Metrics).unwrap()
}
// keeps: spec gui-components §2.2
#[test]
fn flow_wraps_in_source_order_with_independent_gaps_and_shared_row_baselines() {
    let flow = c::Flow {
        children: vec![
            text("aaaa", 20.).into(),
            text("bbbbbbbb", 10.).into(),
            text("ccccc", 10.).into(),
        ],
        wrap: true,
        justify: c::Justify::Center,
        align: c::Alignment::Baseline,
        column_gap: ui::Length::Px(5.),
        row_gap: ui::Length::Px(2.),
    };
    let a = layout(c::Element::Flow(flow), 100.);
    let n = &a.children;
    assert_eq!(n.len(), 3);
    assert_eq!(n[0].rect.x, 7.5);
    assert_eq!(n[1].rect.x, 52.5);
    assert_eq!(n[0].rect.y + n[0].baseline, n[1].rect.y + n[1].baseline);
    assert_eq!(n[2].rect.x, 37.5);
    assert_eq!(n[2].rect.y, 29.);
}
// keeps: spec gui-components §2.2
#[test]
fn nowrap_space_between_and_exact_fit_do_not_create_extra_rows() {
    let mut flow = c::Flow::new(vec![
        text("aa", 10.).into(),
        text("bb", 10.).into(),
        text("cc", 10.).into(),
    ]);
    flow.wrap = false;
    flow.justify = c::Justify::SpaceBetween;
    flow.column_gap = ui::Length::Px(5.);
    let a = layout(c::Element::Flow(flow.clone()), 150.);
    assert_eq!(
        a.children.iter().map(|c| c.rect.x).collect::<Vec<_>>(),
        vec![0., 70., 140.]
    );
    flow.wrap = true;
    flow.justify = c::Justify::Start;
    let b = layout(c::Element::Flow(flow), 40.);
    assert!(b.children.iter().all(|c| c.rect.y == 0.));
}
// keeps: spec gui-components §2.2
#[test]
fn text_centers_or_ends_each_wrapped_line_and_ignores_collapsible_trailing_spaces() {
    let mut t = c::Text::new(
        "one two three",
        ui::Text {
            size: Some(10.),
            ..Default::default()
        },
    );
    t.align = c::Alignment::Center;
    let a = layout(c::Element::Text(t.clone()), 40.);
    let c::LayoutKind::Text { runs, .. } = &a.kind else {
        panic!("text")
    };
    assert_eq!(runs[0].rect.x, 2.5);
    assert_eq!(runs[2].rect.x, 7.5);
    t.align = c::Alignment::End;
    let b = layout(c::Element::Text(t), 40.);
    let c::LayoutKind::Text { runs, .. } = &b.kind else {
        panic!("text")
    };
    assert_eq!(runs[0].rect.x, 5.);
    assert_eq!(runs[2].rect.x, 15.);
}
// keeps: spec gui-components §2.2
#[test]
fn default_text_and_fitting_flow_keep_the_stack_geometry() {
    let children = vec![text("one", 10.).into(), text("two", 10.).into()];
    let a = layout(
        c::Element::Stack(c::Stack {
            direction: bbt_ui_model::layout::Axis::Row,
            gap: ui::Length::Px(6.),
            align: c::Alignment::Center,
            children: children.clone(),
        }),
        100.,
    );
    let b = layout(
        c::Element::Flow(c::Flow {
            children,
            wrap: true,
            justify: c::Justify::Start,
            align: c::Alignment::Center,
            row_gap: ui::Length::Px(2.),
            column_gap: ui::Length::Px(6.),
        }),
        100.,
    );
    assert_eq!(a.rect, b.rect);
    assert_eq!(a.baseline, b.baseline);
    assert_eq!(format!("{:?}", a.children), format!("{:?}", b.children));
}
