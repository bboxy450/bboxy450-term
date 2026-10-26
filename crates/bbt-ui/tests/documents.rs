use bbt_ui::{self as ui, components as c, modules::document_modal::*};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, v: &str, s: &TextStyle) -> f32 {
        v.chars().count() as f32 * s.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, s: &TextStyle) -> [f32; 2] {
        [s.size * 0.8, s.size * 0.2]
    }
}
fn layout(view: &DocumentModal, w: f32) -> c::Layout {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    view.element(ctx, &Metrics, [w, 664.])
        .layout(
            Rect::new(
                0.,
                0.,
                if view.modal { 980_f32.min(w) } else { w },
                if view.modal { 664. } else { 0. },
            ),
            ctx,
            &Metrics,
        )
        .unwrap()
}
fn find<'a>(l: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&l.kind,c::LayoutKind::Named(n)if n==key) {
        Some(l)
    } else {
        l.children.iter().find_map(|l| find(l, key))
    }
}
fn labels(l: &c::Layout) -> String {
    let mut out = if let c::LayoutKind::Text { runs, .. } = &l.kind {
        runs.iter().map(|r| r.text.as_str()).collect()
    } else {
        String::new()
    };
    for c in &l.children {
        out += &labels(c)
    }
    out
}
// keeps: spec document-viewer §Markdown
#[test]
fn record_number_shares_the_type_chip_and_absent_numbers_keep_the_old_label() {
    let mut view = DocumentModal {
        document: Document {
            kind: Some("decision".into()),
            title: "A numbered record".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let tree = layout(&view, 1218.);
    assert_eq!(
        labels(find(&tree, "type/text").unwrap()).to_lowercase(),
        "decision"
    );
    view.document.number = Some(47);
    let tree = layout(&view, 1218.);
    assert_eq!(
        labels(find(&tree, "type/text").unwrap()).to_lowercase(),
        "decision #47"
    );
}
// keeps: spec document-viewer §Markdown
#[test]
fn document_metadata_and_unsigned_or_broken_evidence_remain_visible() {
    let mut view = DocumentModal {
        document: Document {
            kind: Some("note".into()),
            title: "Host supplied title".into(),
            path: ".bboxy/notes/one.md".into(),
            broken: Some("line 4: invalid depends_on".into()),
            body: "A supplied body with [[real-target|display]]{constrains:2}".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let text = labels(&layout(&view, 1218.));
    for term in [
        "Host supplied title",
        "line 4: invalid depends_on",
        "unsigned · no write is anonymous",
        "display",
        "constrains 2",
    ] {
        assert!(text.contains(term), "{term}")
    }
    view.document.prov.signed = Some("operator fingerprint".into());
    assert!(labels(&layout(&view, 1218.)).contains("✓ signed · operator fingerprint"));
}
// keeps: spec document-viewer §Markdown
#[test]
fn modal_retains_explicit_scroll_and_leaves_actions_as_visual_data() {
    let view = DocumentModal {
        modal: true,
        seat_name: Some("Receiver".into()),
        scroll: [0., 100000.],
        document: Document {
            title: "Long read".into(),
            path: "README.md".into(),
            body: (0..60).map(|i| format!("Paragraph {i}.\n")).collect(),
            ..Default::default()
        },
        ..Default::default()
    };
    let tree = layout(&view, 1162.);
    let scroller = find(&tree, "scroller").unwrap();
    let scroll = &scroller.children[0];
    if let c::LayoutKind::ScrollView { offset, content } = &scroll.kind {
        assert!(offset[1] > 0.);
        assert!(offset[1] < 100000.);
        assert!(content[1] > scroll.rect.h)
    } else {
        panic!("missing retained scroll viewport")
    }
    let text = labels(&tree);
    for label in [
        "Feed into Receiver",
        "Feed selection",
        "Copy as Markdown",
        "Copy path",
        "Pin as tab",
        "Close",
    ] {
        assert!(text.contains(label))
    }
}
fn graph() -> Diagram {
    Diagram {
        source: "flowchart LR\n a[A] --> b[B]".into(),
        graph: Some(FlowGraph {
            kind: "flow".into(),
            dir: "LR".into(),
            nodes: vec![
                FlowNode {
                    id: "a".into(),
                    label: "A".into(),
                    shape: "rect".into(),
                    ..Default::default()
                },
                FlowNode {
                    id: "b".into(),
                    label: "B".into(),
                    shape: "rect".into(),
                    link: Some("B".into()),
                    ..Default::default()
                },
            ],
            edges: vec![FlowEdge {
                from: "a".into(),
                to: "b".into(),
                ..Default::default()
            }],
        }),
        ..Default::default()
    }
}
// keeps: spec document-viewer §Markdown
#[test]
fn diagram_state_is_serializable_and_drawing_never_scales_to_the_document() {
    let diagram = graph();
    let mut view = DocumentModal {
        document: Document {
            body: format!("```mermaid\n{}\n```", diagram.source),
            ..Default::default()
        },
        diagrams: vec![diagram],
        ..Default::default()
    };
    let wide = layout(&view, 1218.);
    let narrow = layout(&view, 240.);
    let a = find(&wide, "block0/canvas").unwrap().rect;
    let b = find(&narrow, "block0/canvas").unwrap().rect;
    assert_eq!([a.w, a.h], [b.w, b.h]);
    view.diagrams[0].show_source = true;
    let round: DocumentModal =
        serde_json::from_str(&serde_json::to_string(&view).unwrap()).unwrap();
    assert_eq!(
        round.diagrams[0].graph.as_ref().unwrap().nodes[1]
            .link
            .as_deref(),
        Some("B")
    );
    assert!(find(&layout(&round, 1218.), "block0/source").is_some());
    assert!(find(&layout(&round, 1218.), "block0/canvas").is_none());
}
// keeps: spec document-viewer §Markdown
#[test]
fn unsupported_diagram_preserves_source_instead_of_a_partial_drawing() {
    let source = "not-yet-supported syntax";
    let view = DocumentModal {
        document: Document {
            body: format!("```mermaid\n{source}\n```"),
            ..Default::default()
        },
        diagrams: vec![Diagram {
            source: source.into(),
            error: Some("unsupported diagram: not-yet-supported".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let l = layout(&view, 1218.);
    let text = labels(&l);
    assert!(text.contains(source));
    assert!(text.contains("not drawn"));
    assert!(find(&l, "block0/canvas").is_none());
}
// keeps: spec document-viewer §Markdown
#[test]
fn short_code_cells_center_within_the_keys_padded_grid_row() {
    let view = DocumentModal {
        document: Document {
            kind: Some("note".into()),
            id: "a public id".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let tree = layout(&view, 1218.);
    let key = find(&tree, "fm0/key").unwrap();
    let value = find(&tree, "fm0/value0").unwrap();
    assert!((value.rect.y - key.rect.y - 1.).abs() < 0.001);
}
