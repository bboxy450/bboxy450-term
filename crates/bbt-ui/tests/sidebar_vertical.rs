// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    modules::composer::{Command, Composer, Controller, Conversation, Regions, SidebarLayout},
    text_edit::Command as Edit,
};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        text.chars().count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn named<'a>(layout: &'a c::Layout, key: &str) -> &'a c::Layout {
    fn find<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
        if matches!(&n.kind, c::LayoutKind::Named(name) if name == key) {
            return Some(n);
        }
        n.children.iter().find_map(|n| find(n, key))
    }
    find(layout, key).unwrap_or_else(|| panic!("missing {key}"))
}
#[test]
fn bounded_sidebar_drafts_keep_controls_and_history_visible_and_follow_the_caret() {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let text = "a long draft line with wrapped words\n".repeat(120);
    for width in [160., 300., 520.] {
        for height in [280., 420., 800.] {
            let mut view = Conversation {
                composer: Composer {
                    text: text.clone(),
                    focus: true,
                    edit: Some(c::Edit {
                        caret: Some(text.encode_utf16().count()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            };
            let layout = view
                .element_bounded(ctx, [width, height], &Metrics, Some(96.))
                .unwrap()
                .layout(Rect::new(0., 0., width, height), ctx, &Metrics)
                .unwrap();
            let regions = Regions::new(&layout).unwrap();
            assert!(regions.body_max_scroll > 0.);
            assert!(regions.body_offset > 0., "{width}/{height}: body={:?}, max={}, caret={:?}", regions.body, regions.body_max_scroll, named(&layout, "composer/text/caret").rect);
            assert!(regions.body.h <= 96.);
            assert!((layout.rect.h - height).abs() < 0.01);
            for key in [
                "composer/address",
                "composer/send",
                "composer/foot",
                "conversation/rows",
            ] {
                let r = named(&layout, key).rect;
                assert!(
                    r.y >= 0. && r.bottom() <= height + 0.01,
                    "{width}/{height}/{key}: {r:?}"
                );
            }
            let top = [regions.body.x + 3., regions.body.y];
            assert_eq!(
                regions.index_at(&layout, &text, [top[0], -1000.], &Metrics),
                regions.index_at(&layout, &text, top, &Metrics),
                "drag indices are clipped to the visible editor"
            );
            view.composer.body_scroll = Some(0.);
            let layout = view
                .element_bounded(ctx, [width, height], &Metrics, Some(96.))
                .unwrap()
                .layout(Rect::new(0., 0., width, height), ctx, &Metrics)
                .unwrap();
            assert_eq!(
                Regions::new(&layout).unwrap().body_offset,
                0.,
                "manual scroll overrides caret following"
            );
        }
    }
    let mut editor = Controller::new(Composer {
        text,
        body_scroll: Some(0.),
        ..Default::default()
    });
    editor.handle(Command::Edit(Edit::Insert("x".into())));
    assert_eq!(
        editor.view.body_scroll, None,
        "editing restores caret following"
    );
}
#[test]
fn sidebar_split_requests_reject_nonfinite_and_out_of_range_values() {
    assert!(SidebarLayout::default().valid());
    for fraction in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        assert!(
            !SidebarLayout {
                conversation_fraction: fraction,
                ..Default::default()
            }
            .valid()
        );
    }
    for height in [f32::NAN, f32::INFINITY, -1., 5000.] {
        assert!(
            !SidebarLayout {
                composer_body_height: Some(height),
                ..Default::default()
            }
            .valid()
        );
    }
}
