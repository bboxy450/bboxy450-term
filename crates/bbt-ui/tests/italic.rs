// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::Prim,
    layout::{Rect, TextMeasure, TextStyle},
};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, value: &str, style: &TextStyle) -> f32 {
        assert!(style.italic);
        value.len() as f32 * style.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        assert!(style.italic);
        [style.size * 0.8, style.size * 0.2]
    }
}
// keeps: spec gui-components §1.5
#[test]
fn italic_is_backward_compatible_and_reaches_measurement_and_draw_records() {
    let old = serde_json::to_value(ui::Text::default()).unwrap();
    assert!(old.get("italic").is_none());
    let mut text: ui::Text = serde_json::from_value(old).unwrap();
    assert!(!text.italic);
    text.italic = true;
    let value = serde_json::to_value(text).unwrap();
    assert_eq!(value["italic"], true);
    assert!(serde_json::from_value::<ui::Text>(value).unwrap().italic);
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let layout = c::Element::Text(c::Text::new("italic", text))
        .layout(Rect::new(0., 0., 300., 0.), ctx, &Metrics)
        .unwrap();
    let plan = c::paint(&layout, ctx);
    assert!(plan.iter().any(|op|matches!(op,c::Paint::Draw(d) if d.primitives.iter().any(|p|matches!(p,Prim::Text{style,..} if style.italic)))));
}
