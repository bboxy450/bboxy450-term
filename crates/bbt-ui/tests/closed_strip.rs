// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{self as ui, components as c, modules::closed_strip::ClosedStrip};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
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
fn find<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&n.kind,c::LayoutKind::Named(s) if s==key) {
        Some(n)
    } else {
        n.children.iter().find_map(|n| find(n, key))
    }
}
// keeps: spec e2e-acceptance §1 T7
#[test]
fn t7_grace_pose_keeps_one_row_and_one_undo_target_until_the_deadline() {
    for theme in [ui::Theme::Dark, ui::Theme::Light] {
        for density in [
            ui::Density::Compact,
            ui::Density::Cozy,
            ui::Density::Comfortable,
            ui::Density::Touch,
        ] {
            let ctx = ui::Context::new(theme, density);
            let mut strip = ClosedStrip {
                key: "victim".into(),
                name: "Tamsin Voss".into(),
                harness: "codex".into(),
                remaining_ms: 52_000,
                grace_ms: 60_000,
            };
            let layout = strip
                .element(ctx, &Metrics, 260.)
                .layout(Rect::new(0., 0., 260., 0.), ctx, &Metrics)
                .unwrap();
            assert_eq!(layout.rect.h, density.tokens().tab_h as f32);
            let undo = find(&layout, "closed/victim/undo").unwrap();
            assert!(undo.rect.w > 0. && undo.rect.h > 0.);
            strip.remaining_ms = 1000;
            let last = strip
                .element(ctx, &Metrics, 260.)
                .layout(Rect::new(0., 0., 260., 0.), ctx, &Metrics)
                .unwrap();
            assert_eq!(last.rect, layout.rect);
            strip.remaining_ms = 0;
            let gone = strip
                .element(ctx, &Metrics, 260.)
                .layout(Rect::new(0., 0., 260., 0.), ctx, &Metrics)
                .unwrap();
            assert!(find(&gone, "closed/victim/undo").is_none());
        }
    }
}
