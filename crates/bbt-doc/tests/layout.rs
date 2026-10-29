// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_doc::parse;
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    STYLE,
    layout::{Rect, TextMeasure, TextStyle},
};

struct Fonts;
impl TextMeasure for Fonts {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        text.chars().filter(|c| !c.is_control()).count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Fonts {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn ctx() -> ui::Context {
    ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)
}
fn texts(layout: &c::Layout) -> String {
    let mut out = String::new();
    if let c::LayoutKind::Text { runs, .. } = &layout.kind {
        for run in runs {
            out.push_str(&run.text);
        }
        out.push('\n');
    }
    for child in &layout.children {
        out.push_str(&texts(child))
    }
    out
}
fn named<'a>(layout: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&layout.kind,c::LayoutKind::Named(k) if k==key) {
        Some(layout)
    } else {
        layout.children.iter().find_map(|c| named(c, key))
    }
}

// keeps: document heading baseline — six levels retain the existing named type scale.
#[test]
fn all_six_heading_levels_keep_their_baselined_native_size_and_weight() {
    let style: serde_json::Value =
        serde_json::from_str(include_str!("../../../design/style.json")).unwrap();
    for (level, token) in [
        "doc_title",
        "heading",
        "subtitle",
        "doc_heading",
        "body",
        "section",
    ]
    .into_iter()
    .enumerate()
    {
        let source = format!("{} Heading{}\n", "#".repeat(level + 1), level + 1);
        let expected = style["type"][token].as_f64().unwrap() as f32;
        for width in [240., 576.] {
            let layout = parse(&source)
                .body_element(ctx(), &Fonts, width, &[])
                .layout(Rect::new(0., 0., width, 0.), ctx(), &Fonts)
                .unwrap();
            fn heading(node: &c::Layout) -> Option<&c::PositionedRun> {
                if let c::LayoutKind::Text { runs, .. } = &node.kind {
                    return runs.first();
                }
                node.children.iter().find_map(heading)
            }
            let run = heading(&layout).unwrap();
            assert_eq!(run.text, format!("Heading{}", level + 1));
            assert_eq!(run.style.size, expected, "H{} uses {token}", level + 1);
            assert_eq!(run.style.weight, 700);
            assert!(run.line_height >= expected && run.rect.h >= expected);
        }
    }
}

// keeps: spec document-viewer §Markdown — parser and native layout preserve source semantics
#[test]
fn document_body_uses_a_bounded_measure_and_real_wrapping_for_rich_tables() {
    let doc = parse(
        "# Measure\n\nA **bold** *italic* paragraph with [[events|Bus]]{derives:0.7}.\n\n| Name | Value |\n| --- | ---: |\n| a long description that must wrap inside its own cell and keeps going with several more words so the widest document still needs multiple lines | 123 |\n\n> quoted words\n\n- first\n- second\n",
    );
    for width in [240., 360., 1200.] {
        let layout = doc
            .body_element(ctx(), &Fonts, width, &[])
            .layout(Rect::new(0., 0., width, 0.), ctx(), &Fonts)
            .unwrap();
        let body = named(&layout, "body").unwrap();
        assert!(body.rect.w <= width.min(STYLE.spacing.doc_measure as f32) + 0.01);
        let table = named(&layout, "block2").unwrap();
        assert!(table.rect.w <= width.min(STYLE.spacing.doc_measure as f32) + 0.01);
        let c0 = named(&layout, "block2/row1/cell0").unwrap();
        let c1 = named(&layout, "block2/row1/cell1").unwrap();
        assert!(
            c0.rect.h > 30.,
            "wrapped cell, not a truncated fixed-height UI table"
        );
        assert!((c0.rect.y - c1.rect.y).abs() < 0.01);
        assert!((c0.rect.h - c1.rect.h).abs() < 0.01);
        assert!(c0.rect.x + c0.rect.w <= c1.rect.x + 0.01);
        let text = texts(&layout);
        assert!(text.contains("Bus") && text.contains("derives 0.7") && text.contains("123"));
    }
}

// keeps: spec document-viewer §Markdown — parser and native layout preserve source semantics
#[test]
fn table_columns_keep_unbroken_words_and_scroll_when_their_minima_do_not_fit() {
    let doc = parse(
        "| Term | Description |\n| --- | --- |\n| harness | A considerably longer description that wraps naturally over many lines. |\n| **front**matter | metadata |\n",
    );
    for width in [120., 240., 576.] {
        let layout = doc
            .body_element(ctx(), &Fonts, width, &[])
            .layout(Rect::new(0., 0., width, 0.), ctx(), &Fonts)
            .unwrap();
        let cell = named(&layout, "block0/row1/cell0").unwrap();
        let word_width = Fonts.width(
            "harness",
            &TextStyle {
                size: ui::TypeStep::Body.pixels(),
                weight: 400,
                mono: false,
                italic: false,
                tracking_em: 0.,
            },
        );
        assert!(cell.rect.w >= word_width + 16.);
        let rendered = texts(named(&layout, "block0/row1/cell0/text").unwrap());
        assert_eq!(rendered.trim(), "harness");
        let styled = named(&layout, "block0/row2/cell0/text").unwrap();
        let ys: Vec<_> = match &styled.kind {
            c::LayoutKind::Named(_) => match &styled.children[0].kind {
                c::LayoutKind::Text { runs, .. } => runs.iter().map(|run| run.rect.y).collect(),
                _ => panic!("styled cell text"),
            },
            _ => unreachable!(),
        };
        assert!(
            ys.iter().all(|y| (*y - ys[0]).abs() < 0.01),
            "a word spanning style runs stays together"
        );
        if width == 120. {
            assert!(
                named(&layout, "block0/rows").unwrap().rect.w > width,
                "min-content columns use the explicit horizontal viewport"
            );
        }
    }
}

// keeps: spec document-viewer §Markdown — parser and native layout preserve source semantics
#[test]
fn matching_leading_h1_is_hidden_only_with_a_header_and_original_source_stays_intact() {
    for dash in ["-", "‒", "–", "—", "―"] {
        let source = format!("\n \n# BBOXy450-term  {dash}  vision\n\n## Cockpit first\n\nBody\n");
        let doc = parse(&source);
        let with_header = doc
            .body_element_with_title(Some("bboxy450-term — vision"), ctx(), &Fonts, 576., &[])
            .layout(Rect::new(0., 0., 576., 0.), ctx(), &Fonts)
            .unwrap();
        assert!(!texts(&with_header).contains("BBOXy450"));
        assert!(texts(&with_header).contains("Cockpit first"));
        let alone = doc
            .body_element(ctx(), &Fonts, 576., &[])
            .layout(Rect::new(0., 0., 576., 0.), ctx(), &Fonts)
            .unwrap();
        assert!(texts(&alone).contains("BBOXy450"));
        assert_eq!(doc.source, source);
    }
    for source in [
        "# Why a cockpit\nBody",
        "## Vision\nBody",
        "[ref]: https://example.test\n\n# Vision\nBody",
        "# **Vision**\nBody",
        " # Vision\nBody",
        "# Vision ###\nBody",
    ] {
        let doc = parse(source);
        let view = doc
            .body_element_with_title(Some("Vision"), ctx(), &Fonts, 576., &[])
            .layout(Rect::new(0., 0., 576., 0.), ctx(), &Fonts)
            .unwrap();
        assert!(
            texts(&view).contains(if source.starts_with("# Why") {
                "Why a cockpit"
            } else {
                "Vision"
            }),
            "{source}"
        );
    }
    let source = "\u{feff}---\r\nid: fox\r\ntype: spec\r\ntitle: Vision\r\n---\r\n\r\n# VISION\r\n\r\n# Vision\r\nBody\r\n";
    let doc = parse(source);
    assert!(doc.source[doc.body_start..].starts_with("\r\n# VISION"));
    let view = doc
        .modal_element("vision.md", false, [1246., 900.], ctx(), &Fonts)
        .layout(Rect::new(0., 0., 1246., 0.), ctx(), &Fonts)
        .unwrap();
    let t = texts(&view);
    assert!(!t.contains("VISION"));
    assert_eq!(
        t.matches("Vision").count(),
        2,
        "header plus the second heading; remove only one H1"
    );
    assert_eq!(doc.source, source);
}

// keeps: spec document-viewer §Markdown — parser and native layout preserve source semantics
#[test]
fn real_mermaid_is_a_drawing_card_and_wide_code_scrolls_without_scaling() {
    let src = format!(
        "```mermaid\nflowchart TD\n A --> B\n```\n\n```rust\n{}\n```\n",
        "long_token_".repeat(100)
    );
    let doc = parse(&src);
    let layout = doc
        .body_element(ctx(), &Fonts, 300., &[])
        .layout(Rect::new(0., 0., 300., 0.), ctx(), &Fonts)
        .unwrap();
    let value = texts(&layout);
    assert!(value.contains("flowchart") && value.contains("Source") && value.contains("Drawing"));
    assert!(!value.contains("A --> B"));
    fn scroll(l: &c::Layout) -> bool {
        matches!(l.kind, c::LayoutKind::ScrollView { .. }) || l.children.iter().any(scroll)
    }
    assert!(scroll(&layout));
}

// keeps: spec document-viewer §Markdown — parser and native layout preserve source semantics
#[test]
fn modal_body_override_preserves_the_shell_and_does_not_claim_yaml_signatures_verify() {
    let doc = parse(
        "---\ntype: task\nid: test\ntitle: Header survives\nsignature: present-but-not-verified\ncustom: [one, two]\n---\nA new parsed body.\n",
    );
    let projected = doc.document("task.md");
    assert_eq!(projected.title, "Header survives");
    assert!(projected.prov.signed.is_none());
    let layout = doc
        .modal_element("task.md", true, [1246., 900.], ctx(), &Fonts)
        .layout(Rect::new(0., 0., 980., 900.), ctx(), &Fonts)
        .unwrap();
    let value = texts(&layout);
    assert!(
        value.contains("Header survives") && value.contains("A new parsed body."),
        "{value:?}"
    );
    assert!(value.contains("signature") && value.contains("present-but-not-verified"));
    assert!(value.contains("custom"));
    assert!(named(&layout, "footer").is_some() || value.contains("Feed into"));
}

// keeps: spec document-viewer §Markdown — parser and native layout preserve source semantics
#[test]
fn structured_frontmatter_links_project_to_chips_and_keep_the_original_mapping() {
    let doc = parse(
        "---\ntype: spec\nlinks:\n  - {to: '[[vision]]', rel: direction}\n  - {to: '[[events|Bus]]', rel: derives, weight: 0.7}\n  - {to: 'https://example.test', rel: reference}\n---\nBody\n",
    );
    assert_eq!(
        doc.document("spec.md").links,
        vec![
            "[[vision]]{direction}",
            "[[events|Bus]]{derives:0.7}",
            "https://example.test · reference"
        ]
    );
    assert!(matches!(
        &doc.frontmatter.fields["links"][0],
        serde_yaml::Value::Mapping(_)
    ));
}

// keeps: spec document-viewer §Markdown — parser and native layout preserve source semantics
#[test]
fn long_paths_and_emoji_wrap_at_grapheme_boundaries_without_rewriting_source() {
    let token = format!("/{}", "👩‍💻/folder-name/".repeat(20));
    let source = format!("{token} [[events]]");
    let doc = parse(&source);
    let layout = doc
        .body_element(ctx(), &Fonts, 240., &[])
        .layout(Rect::new(0., 0., 240., 0.), ctx(), &Fonts)
        .unwrap();
    fn check(l: &c::Layout) {
        if let c::LayoutKind::Text { runs, .. } = &l.kind {
            assert!(
                l.rect.w <= 240.01,
                "long token escaped the column: {:?}",
                l.rect
            );
            for run in runs {
                assert!(!run.text.starts_with('\u{200d}') && !run.text.ends_with('\u{200d}'));
            }
        }
        for c in &l.children {
            check(c);
        }
    }
    check(&layout);
    assert_eq!(doc.source, source);
    assert!(doc.source.contains(&token));
}
