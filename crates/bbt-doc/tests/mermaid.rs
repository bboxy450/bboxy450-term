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
    c::paint(l, ctx())
        .into_iter()
        .flat_map(|op| match op {
            c::Paint::Draw(list) => {
                list.validate().unwrap();
                list.primitives
            }
            _ => vec![],
        })
        .filter(|p| matches!(p, Prim::Path { .. }))
        .collect()
}

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
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-files", "-z", "--", ".bboxy"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut count = 0;
    for path in output
        .stdout
        .split(|b| *b == 0)
        .filter(|v| v.ends_with(b".md"))
    {
        let path = std::str::from_utf8(path).unwrap();
        let doc = bbt_doc::parse(&std::fs::read_to_string(root.join(path)).unwrap());
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
