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
        assert!((body.rect.w - width).abs() < 0.01);
        assert!(
            named(&layout, "block1").unwrap().rect.w <= STYLE.spacing.doc_measure as f32 + 0.01
        );
        let table = named(&layout, "block2").unwrap();
        assert!((table.rect.w - width).abs() < 0.01);
        let c0 = named(&layout, "block2/row1/cell0").unwrap();
        let c1 = named(&layout, "block2/row1/cell1").unwrap();
        if width <= 360. {
            assert!(c0.rect.h > 30., "the fitted cell wraps without truncation");
        }
        assert!((c0.rect.y - c1.rect.y).abs() < 0.01);
        assert!((c0.rect.h - c1.rect.h).abs() < 0.01);
        assert!(c0.rect.x + c0.rect.w <= c1.rect.x + 0.01);
        let text = texts(&layout);
        assert!(text.contains("Bus") && text.contains("derives 0.7") && text.contains("123"));
    }
}

// keeps: spec document-viewer §Markdown — parser and native layout preserve source semantics
#[test]
fn table_columns_wrap_oversized_words_and_preserve_styled_source() {
    let source = "| Term | Description |\n| --- | --- |\n| harness | A considerably longer description that wraps naturally over many lines. |\n| **front**matter | metadata |\n";
    let doc = parse(source);
    for width in [120., 240., 576.] {
        let layout = doc
            .body_element(ctx(), &Fonts, width, &[])
            .layout(Rect::new(0., 0., width, 0.), ctx(), &Fonts)
            .unwrap();
        assert!(named(&layout, "block0/rows").unwrap().rect.w <= width + 0.01);
        for (row, value) in [(1, "harness"), (2, "frontmatter")] {
            let cell = named(&layout, &format!("block0/row{row}/cell0/text")).unwrap();
            let runs = runs(cell);
            assert_eq!(
                runs.iter().map(|r| r.text.as_str()).collect::<String>(),
                value
            );
            assert!(
                runs.iter().all(|r| r.rect.x >= cell.rect.x - 0.01
                    && r.rect.right() <= cell.rect.right() + 0.01)
            );
            if width == 576. {
                assert!(
                    runs.iter()
                        .all(|r| (r.rect.y - runs[0].rect.y).abs() < 0.01),
                    "words that fit remain together"
                );
            }
        }
    }
    assert_eq!(doc.source, source);
}

fn runs(node: &c::Layout) -> Vec<&c::PositionedRun> {
    let mut result = vec![];
    if let c::LayoutKind::Text { runs, .. } = &node.kind {
        result.extend(runs);
    }
    for child in &node.children {
        result.extend(runs(child));
    }
    result
}

// keeps: spec document-viewer §Markdown — available width, true-size rich cells and requested alignment.
#[test]
fn fitted_tables_use_the_offered_width_and_align_short_and_wrapped_cells() {
    let source = "Readable prose stays on its own measure.\n\n| Center | Right |\n| :---: | ---: |\n| **long**CenterIdentifierThatWrapsAcrossSeveralLines | `longRightIdentifierThatWrapsAcrossSeveralLines` |\n| 123 | 45 |\n";
    let doc = parse(source);
    for width in [240., 360., 944.] {
        let layout = doc
            .body_element(ctx(), &Fonts, width, &[])
            .layout(Rect::new(0., 0., width, 0.), ctx(), &Fonts)
            .unwrap();
        let prose = named(&layout, "block0").unwrap();
        assert!(prose.rect.w <= width.min(STYLE.spacing.doc_measure as f32) + 0.01);
        let table = named(&layout, "block1").unwrap();
        assert!((table.rect.w - width).abs() < 0.01);
        let rows = named(&layout, "block1/rows").unwrap();
        assert!(rows.rect.right() <= table.rect.right() + 0.01);
        for row in [1, 2] {
            let left = named(&layout, &format!("block1/row{row}/cell0")).unwrap();
            let right = named(&layout, &format!("block1/row{row}/cell1")).unwrap();
            assert!((left.rect.h - right.rect.h).abs() < 0.01);
            for col in [0, 1] {
                let cell = named(&layout, &format!("block1/row{row}/cell{col}/text")).unwrap();
                let runs = runs(cell);
                assert!(
                    runs.iter()
                        .all(|r| r.style.size == ui::TypeStep::Body.pixels())
                );
                for first in &runs {
                    let line: Vec<_> = runs
                        .iter()
                        .filter(|r| (r.rect.y - first.rect.y).abs() < 0.01)
                        .collect();
                    let x = line.iter().map(|r| r.rect.x).fold(f32::INFINITY, f32::min);
                    let end = line
                        .iter()
                        .map(|r| r.rect.right())
                        .fold(f32::NEG_INFINITY, f32::max);
                    assert!(x >= cell.rect.x - 0.01 && end <= cell.rect.right() + 0.01);
                    if col == 0 {
                        assert!(
                            ((x + end) * 0.5 - (cell.rect.x + cell.rect.w * 0.5)).abs() < 0.01,
                            "centered row {row}, width {width}"
                        );
                    } else {
                        assert!(
                            (end - cell.rect.right()).abs() < 0.01,
                            "right-aligned row {row}, width {width}"
                        );
                    }
                }
                let expected = match (row, col) {
                    (1, 0) => "longCenterIdentifierThatWrapsAcrossSeveralLines",
                    (1, 1) => "longRightIdentifierThatWrapsAcrossSeveralLines",
                    (2, 0) => "123",
                    _ => "45",
                };
                assert_eq!(
                    runs.iter().map(|r| r.text.as_str()).collect::<String>(),
                    expected
                );
            }
        }
    }
    assert_eq!(doc.source, source);
}

// keeps: spec document-viewer §Markdown — fitting body tables preserves generic metadata policy.
#[test]
fn narrow_gfm_tables_fit_while_generated_metadata_keeps_its_original_word_viewport() {
    use bbt_ui::modules::document_modal::controller::{RecordLinks, Regions};
    let token = "0123456789abcdef".repeat(4);
    let source = format!("| Left | Right |\n| --- | --- |\n| {token} | {token} |\n");
    let mut doc = parse(&source);
    let metadata = serde_json::json!({"type":"note", "custom_metadata":{"public_key":token}});
    doc.set_metadata(&metadata).unwrap();
    let layout = doc
        .hosted_body_element(
            "Note",
            ctx(),
            &Fonts,
            240.,
            &RecordLinks::new(),
            &Default::default(),
        )
        .layout(Rect::new(0., 0., 240., 0.), ctx(), &Fonts)
        .unwrap();
    let gfm = named(&layout, "block0/rows").unwrap();
    assert!(
        gfm.rect.w <= 240.01,
        "source Markdown table wraps to its offered width"
    );
    let generated = named(&layout, "metadata/grid").unwrap();
    let generated_rows = named(&layout, "metadata/grid/rows").unwrap();
    assert!(
        generated_rows.rect.w > generated.rect.w,
        "generated key/value metadata retains min-word pan"
    );
    let key = runs(named(&layout, "metadata/grid/row1/cell0/text").unwrap());
    assert_eq!(
        key.len(),
        1,
        "generated metadata key must not enter the new per-cell fragmentation path"
    );
    assert_eq!(key[0].text, "custom_metadata");
    let value = runs(named(&layout, "metadata/grid/row1/cell1/text").unwrap())
        .iter()
        .map(|r| r.text.as_str())
        .collect::<String>();
    assert!(
        value.contains(&token),
        "full cryptographic value is retained"
    );
    assert!(
        Regions::retained(&layout)
            .horizontal()
            .iter()
            .any(|r| r.key == "metadata/grid" && r.max > 0.)
    );
    assert_eq!(
        doc.frontmatter.fields["custom_metadata"],
        serde_yaml::to_value(&metadata["custom_metadata"]).unwrap()
    );
    assert_eq!(doc.source, source);
}

// keeps: spec document-viewer §Markdown — evidence is a display disclosure, never a source rewrite.
#[test]
fn task_provenance_is_quiet_until_opened_and_keeps_every_evidence_value() {
    use bbt_ui::modules::document_modal::controller::{Action, Controller, RecordLinks, Regions};
    let digest = format!("sha256:{}", "0123456789abcdef".repeat(4));
    let signature = "fixture-signature-".repeat(12);
    let metadata = serde_json::json!({
        "type":"task", "title":"Retained chat visibility", "swarm_id":"fixture-swarm",
        "custom":"keep this field visible",
        "provenance": {"digest":digest, "matter_digest":"fedcba9876543210".repeat(4),
            "log":".bboxy/log/2026-10-29.jsonl#1302", "sig":signature, "signer":"seat:fixture-executor"},
        "authorship":{"issued_by":"fixture-device", "public_key":"fixture-public-key"}
    });
    for provider in [false, true] {
        let source = if provider {
            "## Purpose\nKeep the task readable.\n".to_owned()
        } else {
            format!(
                "---\n{}---\n## Purpose\nKeep the task readable.\n",
                serde_yaml::to_string(&metadata).unwrap()
            )
        };
        let mut doc = parse(&source);
        if provider {
            doc.set_metadata(&metadata).unwrap();
            assert!(doc.frontmatter.source.is_empty());
        }
        let fields = doc.frontmatter.fields.clone();
        for theme in [ui::Theme::Light, ui::Theme::Dark] {
            let ctx = ui::Context::new(theme, ui::Density::Cozy);
            for width in [240., 640.] {
                let render = |expanded| {
                    let mut control = Controller::default();
                    control.provenance_expanded = expanded;
                    doc.hosted_body_element_with_controller(
                        "Retained chat visibility",
                        ctx,
                        &Fonts,
                        width,
                        &RecordLinks::new(),
                        &control,
                    )
                    .layout(Rect::new(0., 0., width, 0.), ctx, &Fonts)
                    .unwrap()
                };
                let closed = render(false);
                let text = texts(&closed);
                assert!(
                    text.contains("Provenance details") && text.contains("Keep the task readable.")
                );
                assert!(text.contains("keep this field visible") && text.contains("fixture-swarm"));
                for value in [
                    digest.as_str(),
                    signature.as_str(),
                    "fixture-device",
                    "fixture-public-key",
                    "seat:fixture-executor",
                    ".bboxy/log/2026-10-29.jsonl#1302",
                ] {
                    assert!(!text.contains(value), "closed disclosure leaked {value}");
                }
                let toggle = named(&closed, "metadata/provenance/toggle").unwrap().rect;
                assert_eq!(
                    Regions::retained(&closed).hit([toggle.x + 1., toggle.y + 1.]),
                    Some(Action::ToggleProvenance)
                );
                let opened = render(true);
                let expanded = texts(&opened);
                for value in [
                    digest.as_str(),
                    signature.as_str(),
                    "fixture-device",
                    "fixture-public-key",
                    "seat:fixture-executor",
                    ".bboxy/log/2026-10-29.jsonl#1302",
                ] {
                    assert!(expanded.contains(value), "expanded disclosure lost {value}");
                }
                assert!(expanded.contains("Hide provenance details"));
                assert!(opened.rect.h > closed.rect.h);
                assert!(!texts(&render(false)).contains(&signature));
            }
        }
        assert_eq!(doc.frontmatter.fields, fields);
        assert_eq!(doc.source, source);
        assert_eq!(doc.document("task.md").body, source);
        assert!(doc.document("task.md").prov.signed.is_none());
    }
}

// keeps: spec document-viewer §Markdown — fitting never splits extended grapheme clusters.
#[test]
fn narrow_table_tokens_preserve_unicode_clusters_and_exact_text() {
    use unicode_segmentation::UnicodeSegmentation;
    let word = "e\u{301}👩\u{200d}💻漢字".repeat(5);
    let source = format!("| Left | Right |\n| --- | --- |\n| {word} | {word} |\n");
    let doc = parse(&source);
    let layout = doc
        .body_element(ctx(), &Fonts, 150., &[])
        .layout(Rect::new(0., 0., 150., 0.), ctx(), &Fonts)
        .unwrap();
    for col in [0, 1] {
        let cell = named(&layout, &format!("block0/row1/cell{col}/text")).unwrap();
        let rendered = runs(cell);
        assert_eq!(
            rendered.iter().map(|r| r.text.as_str()).collect::<String>(),
            word
        );
        let mut end = 0;
        let boundaries: Vec<_> = word
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .chain(std::iter::once(word.len()))
            .collect();
        for run in rendered {
            end += run.text.len();
            assert!(
                boundaries.contains(&end),
                "fragment ends inside a grapheme cluster"
            );
            assert!(run.rect.right() <= cell.rect.right() + 0.01);
        }
    }
    assert_eq!(doc.source, source);
}

// keeps: spec document-viewer §Markdown — nested tables cannot escape their prose parent.
#[test]
fn nested_list_quote_and_footnote_tables_use_the_actual_enclosing_column() {
    use bbt_doc::markdown::{Block, BlockKind};
    let table = parse("| Left | Right |\n| --- | --- |\n| value | other |\n")
        .blocks
        .remove(0);
    for (kind, key) in [
        (
            BlockKind::List {
                start: None,
                items: vec![vec![table.clone()]],
            },
            "block0/li0/block0",
        ),
        (BlockKind::Quote(vec![table.clone()]), "block0/quote/block0"),
        (
            BlockKind::Footnote {
                label: "n".into(),
                blocks: vec![table.clone()],
            },
            "block0/note/block0",
        ),
    ] {
        let mut doc = parse("parent");
        doc.blocks = vec![Block { source: 0..6, kind }];
        for top_level_table in [false, true] {
            if top_level_table {
                doc.blocks.push(table.clone());
            }
            let layout = doc
                .body_element(ctx(), &Fonts, 944., &[])
                .layout(Rect::new(0., 0., 944., 0.), ctx(), &Fonts)
                .unwrap();
            let parent = named(&layout, "block0").unwrap();
            let nested = named(&layout, key).unwrap();
            assert!(parent.rect.w <= STYLE.spacing.doc_measure as f32 + 0.01);
            assert!(
                nested.rect.x >= parent.rect.x && nested.rect.right() <= parent.rect.right() + 0.01
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
