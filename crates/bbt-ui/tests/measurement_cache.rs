// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{Context, Density, Length, Theme, components as c};
use bbt_ui_model::layout::{Axis, Rect, TextMeasure, TextStyle};
use std::cell::Cell;

#[derive(Default)]
struct Metrics {
    calls: Cell<usize>,
    extra_width: Cell<f32>,
}
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        self.calls.set(self.calls.get() + 1);
        text.chars().count() as f32 * style.size * 0.5 + self.extra_width.get()
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn context() -> Context {
    Context::new(Theme::Dark, Density::Cozy)
}
fn text(value: &str) -> c::Element {
    let mut text = c::Text::new(value, bbt_ui::style::Text::default());
    text.wrap = false;
    c::Element::Text(text)
}

// keeps: spec gui-components §2.2
#[test]
fn nesting_does_not_repeat_leaf_measurement_at_the_same_constraint() {
    let measure = Metrics::default();
    let mut root = text("leaf");
    let bounds = Rect::new(0., 0., 200., 0.);
    let leaf = root.layout(bounds, context(), &measure).unwrap();
    let leaf_calls = measure.calls.replace(0);
    for _ in 0..32 {
        root = c::Element::Stack(c::Stack {
            direction: Axis::Column,
            gap: Length::Px(0.),
            align: c::Alignment::Stretch,
            children: vec![root.into()],
        });
    }
    let nested = root.layout(bounds, context(), &measure).unwrap();
    assert_eq!(nested.rect, leaf.rect);
    assert_eq!(measure.calls.get(), leaf_calls);
}

// keeps: spec gui-components §2.2
#[test]
fn independent_passes_observe_changed_measurements_and_nodes() {
    let measure = Metrics::default();
    let mut root = text("first");
    let before = root.natural(200., context(), &measure).unwrap();
    measure.extra_width.set(7.);
    let after = root.natural(200., context(), &measure).unwrap();
    assert_eq!(after.0[0], before.0[0] + 7.);
    if let c::Element::Text(t) = &mut root {
        t.runs[0].value.push_str(" more");
    }
    assert!(root.natural(200., context(), &measure).unwrap().0[0] > after.0[0]);
}

// keeps: spec gui-components §2.2
#[test]
fn flow_keeps_distinct_node_sizes_while_reusing_min_content_queries() {
    let mut flow = c::Flow::new(vec![
        text("a").into(),
        text("longer").into(),
        text("xyz").into(),
    ]);
    flow.align = c::Alignment::Start;
    flow.row_gap = Length::Px(2.);
    flow.column_gap = Length::Px(0.);
    let layout = c::Element::Flow(flow)
        .layout(Rect::new(0., 0., 20., 0.), context(), &Metrics::default())
        .unwrap();
    assert_eq!(layout.children.len(), 3);
    assert!(layout.children[0].rect.w < layout.children[2].rect.w);
    assert!(layout.children[2].rect.w < layout.children[1].rect.w);
    assert!(layout.children[0].rect.y < layout.children[1].rect.y);
    assert!(layout.children[1].rect.y < layout.children[2].rect.y);
}
