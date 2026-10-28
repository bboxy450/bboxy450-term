// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{self as ui, components as c, widgets::text_area::TextArea};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, s: &str, _: &TextStyle) -> f32 {
        s.chars().count() as f32 * 5.
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, _: &TextStyle) -> [f32; 2] {
        [9., 3.]
    }
}
fn runs(n: &c::Layout, out: &mut Vec<(String, Rect)>) {
    if let c::LayoutKind::Text { runs, .. } = &n.kind {
        out.extend(runs.iter().map(|r| (r.text.clone(), r.rect)));
    }
    for child in &n.children {
        runs(child, out);
    }
}
fn node<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&n.kind,c::LayoutKind::Named(name) if name==key) {
        Some(n)
    } else {
        n.children.iter().find_map(|c| node(c, key))
    }
}
// keeps: spec gui-components §4.1
#[test]
fn editing_never_reflows_the_committed_paragraph_and_uses_shaped_positions() {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let style = ui::Text {
        size: Some(12.),
        line_height: 1.45,
        ..Default::default()
    };
    let rect = Rect::new(0., 0., 30., 0.);
    let value = "one two three";
    let paragraph = c::Element::Text(c::Text::new(value, style))
        .layout(rect, ctx, &Metrics)
        .unwrap();
    let area = TextArea {
        value: value.into(),
        placeholder: String::new(),
        style,
        edit: Some(c::Edit {
            caret: Some(6),
            ..Default::default()
        }),
        min_lines: 2,
        wrap: true,
        name: "body".into(),
    };
    let edited = area
        .element(30., ctx, &Metrics)
        .unwrap()
        .layout(rect, ctx, &Metrics)
        .unwrap();
    let (mut ordinary, mut focused) = (vec![], vec![]);
    runs(&paragraph, &mut ordinary);
    runs(&edited, &mut focused);
    assert_eq!(ordinary, focused);
    let caret = node(&edited, "body/caret").unwrap().rect;
    assert_eq!(caret.x, 9.);
    // Inline caret has vertical-align:-.2em; its top is one pixel above this line.
    assert_eq!(caret.y, 16.390625);
    assert_eq!(edited.rect.h, paragraph.rect.h);
}
// keeps: spec gui-components §4.1
#[test]
fn invalid_surrogate_boundaries_are_rejected_before_layout() {
    let area = TextArea {
        value: "🙂".into(),
        placeholder: String::new(),
        style: Default::default(),
        edit: Some(c::Edit {
            caret: Some(1),
            ..Default::default()
        }),
        min_lines: 2,
        wrap: true,
        name: "body".into(),
    };
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    assert!(area.element(200., ctx, &Metrics).is_err());
}

#[test]
fn trailing_newlines_and_blank_lines_keep_caret_paint_and_hit_boundaries_together() {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let style = ui::Text {
        size: Some(12.),
        line_height: 1.45,
        ..Default::default()
    };
    for value in ["a\n", "a\n\n", "🙂\n", "a\r\n"] {
        let end = value.encode_utf16().count();
        let area = TextArea {
            value: value.into(),
            placeholder: String::new(),
            style,
            edit: Some(c::Edit {
                caret: Some(end),
                ..Default::default()
            }),
            min_lines: 1,
            wrap: true,
            name: "body".into(),
        };
        let layout = area
            .element(120., ctx, &Metrics)
            .unwrap()
            .layout(Rect::new(0., 0., 120., 0.), ctx, &Metrics)
            .unwrap();
        let caret = node(&layout, "body/caret").unwrap().rect;
        assert!(caret.y > 12., "{value:?}: {caret:?}");
        assert!(caret.bottom() <= layout.rect.bottom());
        let position = TextArea::position_at(value, &layout, end, &Metrics);
        assert!(position[1] > caret.y && position[1] < caret.bottom());
        assert_eq!(TextArea::index_at(value, &layout, position, &Metrics), end);
        if value == "a\n\n" {
            let middle = TextArea::position_at(value, &layout, 2, &Metrics);
            assert!(middle[1] < position[1]);
            assert_eq!(TextArea::index_at(value, &layout, middle, &Metrics), 2);
        }
    }
    let area = TextArea {
        value: "a".into(),
        placeholder: String::new(),
        style,
        edit: Some(c::Edit {
            caret: Some(1),
            preedit: Some(c::Preedit {
                text: "\n".into(),
                caret: Some(1),
            }),
            ..Default::default()
        }),
        min_lines: 1,
        wrap: true,
        name: "body".into(),
    };
    let layout = area
        .element(120., ctx, &Metrics)
        .unwrap()
        .layout(Rect::new(0., 0., 120., 0.), ctx, &Metrics)
        .unwrap();
    assert!(
        node(&layout, "body/caret").unwrap().rect.y > 12.,
        "marked newline has its own empty line too"
    );
}

#[test]
fn caret_paint_agrees_with_pointer_positions_on_each_explicit_line_boundary() {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let style = ui::Text {
        size: Some(12.),
        line_height: 1.45,
        ..Default::default()
    };
    for value in ["a\nb", "a\n\n", "🙂\n"] {
        for at in 0..=value.encode_utf16().count() {
            if c::utf16_byte_index(value, at).is_none() {
                continue;
            }
            let area = TextArea {
                value: value.into(),
                placeholder: String::new(),
                style,
                edit: Some(c::Edit {
                    caret: Some(at),
                    ..Default::default()
                }),
                min_lines: 1,
                wrap: true,
                name: "body".into(),
            };
            let layout = area
                .element(120., ctx, &Metrics)
                .unwrap()
                .layout(Rect::new(0., 0., 120., 0.), ctx, &Metrics)
                .unwrap();
            let point = TextArea::position_at(value, &layout, at, &Metrics);
            let caret = node(&layout, "body/caret").unwrap().rect;
            assert!(
                point[1] >= caret.y && point[1] <= caret.bottom(),
                "{value:?}/{at}: {point:?}, {caret:?}"
            );
            assert_eq!(TextArea::index_at(value, &layout, point, &Metrics), at);
        }
    }
}
