// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
use unicode_segmentation::UnicodeSegmentation;

struct Fonts;
impl TextMeasure for Fonts {
    fn width(&self, text: &str, font: &TextStyle) -> f32 {
        text.graphemes(true).count() as f32 * font.size * 0.6
    }
}
impl c::Measure for Fonts {
    fn font_metrics(&self, font: &TextStyle) -> [f32; 2] {
        [font.size * 0.8, font.size * 0.2]
    }
}
fn capsules<'a>(node: &'a c::Layout, out: &mut Vec<&'a c::Layout>) {
    if let c::LayoutKind::Named(name) = &node.kind
        && name.ends_with("/capsule")
    {
        out.push(node);
        return;
    }
    for child in &node.children {
        capsules(child, out);
    }
}
fn runs<'a>(node: &'a c::Layout, out: &mut Vec<&'a c::PositionedRun>) {
    if let c::LayoutKind::Text { runs, .. } = &node.kind {
        out.extend(runs);
    }
    for child in &node.children {
        runs(child, out);
    }
}

#[test]
fn markdown_links_keep_targets_through_emphasis_code_and_wrapping() {
    fn links<'a>(node: &'a c::Layout, out: &mut Vec<&'a c::Layout>) {
        if matches!(&node.kind, c::LayoutKind::Named(name) if name.starts_with("record-link:")) {
            out.push(node);
        }
        for child in &node.children {
            links(child, out);
        }
    }
    let source = "Before [**bold label** and `code` café](https://example.com/docs) after.";
    for theme in [ui::Theme::Light, ui::Theme::Dark] {
        for width in [120., 320.] {
            let ctx = ui::Context::new(theme, ui::Density::Cozy);
            let layout = bbt_doc::parse(source)
                .body_element(ctx, &Fonts, width, &[])
                .layout(Rect::new(0., 0., width, 400.), ctx, &Fonts)
                .unwrap();
            let regions = ui::modules::document_modal::controller::Regions::retained(&layout);
            let mut targets = Vec::new();
            links(&layout, &mut targets);
            assert!(targets.len() > 2);
            let mut visible = 0;
            for link in targets {
                // Flow removes whitespace at a wrap boundary; it has no clickable pixels.
                if link.rect.w <= 0. || link.rect.h <= 0. { continue; }
                visible += 1;
                assert!(link.rect.x >= 0. && link.rect.right() <= width + 0.01);
                let at = [
                    link.rect.x + link.rect.w / 2.,
                    link.rect.y + link.rect.h / 2.,
                ];
                assert_eq!(
                    regions.hit(at),
                    Some(ui::modules::document_modal::controller::Action::Open(
                        "https://example.com/docs".into()
                    )),
                    "link {:?}, root {:?}, kind {:?}", link.rect, layout.rect, link.kind
                );
                assert_eq!(regions.cursor(at), ui::cursor::Cursor::Pointer);
            }
            assert!(visible >= 3);
        }
    }
}

// keeps: rich-text-safety — one semantic capsule owns padding, not copied text.
#[test]
fn inline_code_capsules_wrap_without_fake_spaces_or_split_graphemes() {
    let value = "make verify-acp-controls_é👩🏽‍💻許";
    let source = format!("Run `{value}`, then continue.");
    for width in [64., 120., 240., 640.] {
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let layout = bbt_doc::parse(&source)
            .body_element(ctx, &Fonts, width, &[])
            .layout(Rect::new(0., 0., width, 0.), ctx, &Fonts)
            .unwrap();
        let mut code = vec![];
        capsules(&layout, &mut code);
        assert!(!code.is_empty());
        if width == 640. {
            assert_eq!(code.len(), 1, "one code span, one background");
        }
        let mut copied = String::new();
        for capsule in code {
            assert!(capsule.rect.x >= 0. && capsule.rect.right() <= width + 0.01);
            let mut content = vec![];
            runs(capsule, &mut content);
            assert!(!content.is_empty());
            assert!((content[0].baseline - capsule.rect.y - capsule.baseline).abs() < 0.01);
            assert!(
                value.grapheme_indices(true).any(|(i, _)| i == copied.len()),
                "wrap starts on a native extended cluster boundary"
            );
            for run in content {
                assert!(run.rect.x > capsule.rect.x && run.rect.right() < capsule.rect.right());
                copied.push_str(&run.text);
            }
        }
        assert_eq!(copied, value, "source bytes survive layout and copy");
        let mut all = vec![];
        runs(&layout, &mut all);
        assert!(
            all.iter().any(|r| r.text.contains(',')),
            "adjacent punctuation survives"
        );
    }
}

// keeps: rich-text-safety — code padding participates in table width allocation.
#[test]
fn inline_code_capsules_participate_in_narrow_table_geometry() {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let doc = bbt_doc::parse(
        "| Command | Next |\n| --- | --- |\n| `make verify-acp-controls` | `é👩🏽‍💻` |\n",
    );
    for width in [160., 240., 640.] {
        let layout = doc
            .body_element(ctx, &Fonts, width, &[])
            .layout(Rect::new(0., 0., width, 0.), ctx, &Fonts)
            .unwrap();
        let mut code = vec![];
        capsules(&layout, &mut code);
        assert!(code.len() >= 2);
        for capsule in code {
            assert!(capsule.rect.right() <= width + 0.01);
        }
    }
}
