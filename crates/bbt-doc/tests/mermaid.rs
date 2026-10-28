// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use bbt_doc::mermaid;
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::Prim,
    layout::{Rect, TextMeasure, TextStyle},
};

struct Fonts;
impl TextMeasure for Fonts {
    fn width(&self, value: &str, style: &TextStyle) -> f32 {
        value.chars().filter(|c| !c.is_control()).count() as f32 * style.size * 0.53
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
fn texts(l: &c::Layout) -> String {
    let mut out = String::new();
    if let c::LayoutKind::Text { runs, .. } = &l.kind {
        for run in runs {
            out.push_str(&run.text);
        }
        out.push('\n');
    }
    for child in &l.children {
        out.push_str(&texts(child));
    }
    out
}
fn paths(l: &c::Layout) -> Vec<Prim> {
    fn primitives(items: Vec<Prim>, out: &mut Vec<Prim>) {
        for item in items {
            match item {
                Prim::Group {
                    primitives: children,
                    ..
                } => primitives(children, out),
                Prim::Path { .. } => out.push(item),
                _ => {}
            }
        }
    }
    fn paints(ops: Vec<c::Paint>, out: &mut Vec<Prim>) {
        for op in ops {
            match op {
                c::Paint::Group { ops, .. } => paints(ops, out),
                c::Paint::Draw(list) | c::Paint::Underlay(list) => {
                    list.validate().unwrap();
                    primitives(list.primitives, out);
                }
                _ => {}
            }
        }
    }
    let mut out = vec![];
    paints(c::paint(l, ctx()), &mut out);
    out
}

// keeps: spec document-viewer §Mermaid — native geometry, source fallback and lab fixtures
#[test]
fn geometry_uses_ui_fonts_and_true_size_and_retains_document_link_targets() {
    let g = mermaid::geometry(
        "flowchart LR\n A[worker] --> B{ready?}\n B -.-> C[[events]]",
        ctx(),
        &Fonts,
    )
    .unwrap();
    assert_eq!(
        g.nodes
            .iter()
            .find(|n| n.id == "C")
            .unwrap()
            .link
            .as_deref(),
        Some("events")
    );
    let e = mermaid::canvas(&g, ctx(), &Fonts);
    let l = e
        .layout(Rect::new(0., 0., g.size.w, g.size.h), ctx(), &Fonts)
        .unwrap();
    assert_eq!([l.rect.w, l.rect.h], [g.size.w, g.size.h]);
    let all = paths(&l);
    assert!(
        all.iter()
            .filter(|p| matches!(p, Prim::Path { fill: Some(_), stroke, .. } if stroke.width == 0.))
            .count()
            >= 2,
        "closed arrowheads have real fills"
    );
    assert!(
        all.iter()
            .any(|p| matches!(p, Prim::Path { stroke, .. } if !stroke.dash.is_empty()))
    );
    let t = texts(&l);
    assert!(t.contains("worker") && t.contains("ready?") && t.contains("events"));
}

// keeps: spec document-viewer §Mermaid — native geometry, source fallback and lab fixtures
#[test]
fn sequence_keeps_duplicate_participants_notes_activations_and_frame_dividers() {
    let source = "sequenceDiagram\n participant O as Oracle\n actor N as Nyx\n O->>+N: implement\n alt ready\n N-->>O: report\n else retry\n O-)N: again\n end\n Note right of N: input guarded\n deactivate N";
    let g = mermaid::geometry(source, ctx(), &Fonts).unwrap();
    assert_eq!(g.kind, bbt_mermaid::Kind::Sequence);
    assert_eq!(g.nodes.iter().filter(|n| n.id == "N").count(), 2);
    assert!(
        g.nodes
            .iter()
            .any(|n| n.shape == bbt_mermaid::Shape::Activation)
    );
    assert!(g.clusters.iter().any(|c| !c.dividers.is_empty()));
    assert!(g.edges.iter().any(|e| e.end == bbt_mermaid::Arrow::Open));
    let l = mermaid::canvas(&g, ctx(), &Fonts)
        .layout(Rect::new(0., 0., g.size.w, g.size.h), ctx(), &Fonts)
        .unwrap();
    let t = texts(&l);
    assert!(
        t.contains("Oracle")
            && t.contains("Nyx")
            && t.contains("report")
            && t.contains("input guarded")
    );
    assert!(!paths(&l).is_empty());
}

// keeps: spec document-viewer §Mermaid — native geometry, source fallback and lab fixtures
#[test]
fn circle_markers_are_hollow_at_both_ends() {
    let g = mermaid::geometry("flowchart LR\n A o--o B", ctx(), &Fonts).unwrap();
    assert_eq!(g.edges[0].start, bbt_mermaid::Arrow::Circle);
    assert_eq!(g.edges[0].end, bbt_mermaid::Arrow::Circle);
    let l = mermaid::canvas(&g, ctx(), &Fonts)
        .layout(Rect::new(0., 0., g.size.w, g.size.h), ctx(), &Fonts)
        .unwrap();
    assert_eq!(paths(&l).iter().filter(|p| matches!(p, Prim::Path { points, fill: None, .. } if points.iter().filter(|p| matches!(p, c::PathCommand::Cubic(_))).count() == 4)).count(), 2);
}

// keeps: spec document-viewer §Mermaid — native geometry, source fallback and lab fixtures
#[test]
fn figure_source_state_and_unsupported_syntax_keep_the_original_source_readable() {
    let src = "flowchart LR\n A[alpha] --> B[beta]";
    let view = |source, options| {
        mermaid::element(source, 220., ctx(), &Fonts, options)
            .layout(Rect::new(0., 0., 220., 0.), ctx(), &Fonts)
            .unwrap()
    };
    let drawing = view(src, mermaid::Options::default());
    let source = view(
        src,
        mermaid::Options {
            show_source: true,
            ..Default::default()
        },
    );
    assert!(texts(&drawing).contains("Drawing"));
    assert!(!texts(&drawing).contains("A[alpha]"));
    assert!(
        texts(&source).contains("flowchart LR") && texts(&source).contains("A[alpha] --> B[beta]")
    );
    let bad = view("gantt\n title source retained", Default::default());
    assert!(texts(&bad).contains("line 1: unsupported diagram type"));
    assert!(texts(&bad).contains("title source retained"));
}

// keeps: spec document-viewer §Mermaid — native geometry, source fallback and lab fixtures
#[test]
fn every_real_bboxy_mermaid_block_lays_out_on_the_ui_measurement_contract() {
    fn walk_blocks(blocks: &[bbt_doc::Block], out: &mut Vec<String>) {
        for b in blocks {
            match &b.kind {
                bbt_doc::BlockKind::Code { info, text }
                    if info.split_whitespace().next() == Some("mermaid") =>
                {
                    out.push(text.clone())
                }
                bbt_doc::BlockKind::List { items, .. } => {
                    for item in items {
                        walk_blocks(item, out)
                    }
                }
                bbt_doc::BlockKind::Quote(bs) | bbt_doc::BlockKind::Footnote { blocks: bs, .. } => {
                    walk_blocks(bs, out)
                }
                _ => {}
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    fn markdown_paths(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            let path = entry.path();
            if kind.is_dir() {
                markdown_paths(&path, out);
            } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                out.push(path);
            }
        }
    }
    let mut files = vec![];
    markdown_paths(&root.join(".bboxy"), &mut files);
    files.sort();
    let mut count = 0;
    for file in files {
        let path = file.strip_prefix(&root).unwrap().display().to_string();
        let doc = bbt_doc::parse(&std::fs::read_to_string(root.join(&path)).unwrap());
        let mut source = vec![];
        walk_blocks(&doc.blocks, &mut source);
        for (i, source) in source.iter().enumerate() {
            let g = mermaid::geometry(source, ctx(), &Fonts)
                .unwrap_or_else(|error| panic!("{path}, Mermaid block {}: {error}", i + 1));
            let l = mermaid::canvas(&g, ctx(), &Fonts)
                .layout(Rect::new(0., 0., g.size.w, g.size.h), ctx(), &Fonts)
                .unwrap();
            assert_eq!([l.rect.w, l.rect.h], [g.size.w, g.size.h]);
            for p in paths(&l) {
                if let Prim::Path { points, stroke, .. } = p {
                    stroke.validate(&points).unwrap();
                }
            }
            count += 1;
        }
    }
    assert!(
        count >= 11,
        "the required real corpus must be present, got {count}"
    );
    println!("Mermaid: {count} real .bboxy blocks, native geometry and retained layout pass");
}

// keeps: spec document-viewer §Mermaid — native geometry, source fallback and lab fixtures
#[test]
fn lab_diagram_inputs_stay_aligned_and_supported_cases_use_the_native_engine() {
    let reference = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lab/web/diagrams.html"),
    )
    .unwrap();
    for (name, fixture, supported) in [
        (
            "flowchart TD",
            include_str!("fixtures/lab/flowchart-td.md"),
            true,
        ),
        (
            "flowchart LR",
            include_str!("fixtures/lab/flowchart-lr.md"),
            true,
        ),
        ("sequence", include_str!("fixtures/lab/sequence.md"), true),
        ("state", include_str!("fixtures/lab/state.md"), true),
        (
            "unsupported syntax",
            include_str!("fixtures/lab/unsupported-syntax.md"),
            false,
        ),
    ] {
        let doc = bbt_doc::parse(fixture);
        let bbt_doc::BlockKind::Code { text, .. } = &doc.blocks[0].kind else {
            panic!("Mermaid fixture")
        };
        assert!(
            reference.contains(text.trim_end()),
            "lab input drift: {name}"
        );
        let geometry = mermaid::geometry(text, ctx(), &Fonts);
        assert_eq!(geometry.is_ok(), supported, "{name}: {geometry:?}");
        let layout = mermaid::element(text, 576., ctx(), &Fonts, Default::default())
            .layout(Rect::new(0., 0., 576., 0.), ctx(), &Fonts)
            .unwrap();
        if supported {
            assert!(!paths(&layout).is_empty());
        } else {
            assert!(texts(&layout).contains("shown as source"));
        }
    }
}
