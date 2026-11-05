// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    modules::document_modal::controller::{Action, RecordLinks, Regions},
};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, s: &str, f: &TextStyle) -> f32 {
        s.chars().count() as f32 * f.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, f: &TextStyle) -> [f32; 2] {
        [f.size * 0.8, f.size * 0.2]
    }
}
// keeps: spec document-viewer §What it is and §Mermaid — shared native modal links and independent wide viewports
#[test]
fn native_body_keeps_semantic_targets_and_marks_missing_records_inert() {
    let markdown = bbt_doc::parse(
        "Read [[#44]] and [[old-slug|#17]].\n\n```mermaid\nflowchart LR\n A --> B\n```\n",
    );
    assert_eq!(markdown.link_targets(), ["#44", "old-slug"]);
    let links = RecordLinks::from([("#44".into(), Some(44)), ("old-slug".into(), None)]);
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let layout = markdown
        .body_element_with_links(None, ctx, &Metrics, 500., &[], &links)
        .layout(Rect::new(0., 0., 500., 0.), ctx, &Metrics)
        .unwrap();
    fn text(n: &c::Layout) -> String {
        let mut s = if let c::LayoutKind::Text { runs, .. } = &n.kind {
            runs.iter().map(|r| r.text.as_str()).collect()
        } else {
            String::new()
        };
        for child in &n.children {
            s += &text(child);
        }
        s
    }
    assert!(text(&layout).contains("#17 · no record"));
    let regions = Regions::retained(&layout);
    let mut saw = false;
    for y in 0..layout.rect.h as i32 {
        for x in 0..500 {
            if let Some(Action::Open(target)) = regions.hit([x as f32, y as f32]) {
                assert_eq!(target, "#44");
                saw = true;
            }
        }
    }
    assert!(saw);
    assert!(!c::paint(&layout, ctx).is_empty());
}
// keeps: spec document-viewer §What it is and §Mermaid — shared native modal links and independent wide viewports
#[test]
fn provider_metadata_has_no_fabricated_yaml_and_uses_the_shared_number_resolver() {
    let mut markdown = bbt_doc::parse("# Real body\n");
    markdown.set_metadata(&serde_json::json!({"number":44,"title":"Provider title","links":[{"to":"[[#47]]","rel":"derives"}]})).unwrap();
    assert_eq!(markdown.source, "# Real body\n");
    assert!(markdown.frontmatter.source.is_empty());
    assert_eq!(markdown.document("note.md").number, Some(44));
    assert_eq!(markdown.link_targets(), ["#47"]);
    assert_eq!(
        bbt_doc::links::resolve(
            [bbt_doc::links::Record {
                path: "note.md",
                markdown: &markdown
            }],
            "#44"
        )
        .unwrap()
        .record
        .path,
        "note.md"
    );
}

// keeps: spec document-viewer §What it is and §Mermaid — shared native modal links and independent wide viewports
#[test]
fn wide_mermaid_code_and_tables_pan_in_their_own_retained_viewports() {
    use bbt_ui::modules::document_modal::{DocumentModal, controller::Controller};
    fn view(markdown: &bbt_doc::Markdown, control: &Controller) -> c::Layout {
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let shell = DocumentModal {
            document: markdown.document("wide.md"),
            modal: true,
            viewport: [900., 600.],
            ..Default::default()
        };
        shell
            .element_with_body(ctx, &Metrics, [900., 600.], &|w| {
                markdown.hosted_body_element(
                    "Wide",
                    ctx,
                    &Metrics,
                    w,
                    &RecordLinks::new(),
                    control.horizontal(),
                )
            })
            .layout(Rect::new(0., 0., 846., 492.), ctx, &Metrics)
            .unwrap()
    }
    fn offsets<'a>(n: &'a c::Layout, scope: Option<&'a str>, out: &mut Vec<(String, f32)>) {
        let scope = if let c::LayoutKind::Named(name) = &n.kind {
            Some(name.as_str())
        } else {
            scope
        };
        if let c::LayoutKind::ScrollView { offset, .. } = &n.kind {
            out.push((scope.unwrap_or("").into(), offset[0]));
        }
        for child in &n.children {
            offsets(child, scope, out);
        }
    }
    let code = format!("```text\n{}\n```", "w".repeat(250));
    // Text words now wrap to fit; forty structural cell paddings cannot.
    let table = format!(
        "|{}\n|{}\n|{}",
        "X|".repeat(40),
        "---|".repeat(40),
        "v|".repeat(40)
    );
    let mermaid="```mermaid\nflowchart LR\n A[one] --> B[two] --> C[three] --> D[four] --> E[five] --> F[six] --> G[seven] --> H[eight] --> I[nine] --> J[ten]\n```".to_owned();
    for source in [code, table, mermaid] {
        let mut markdown = bbt_doc::parse(&source);
        markdown
            .set_metadata(&serde_json::json!({"type":"note","title":"Wide"}))
            .unwrap();
        let mut control = Controller::default();
        let before = view(&markdown, &control);
        let regions = Regions::retained(&before);
        let horizontal = regions
            .horizontal()
            .iter()
            .find(|r| !r.root)
            .expect("native overflow viewport");
        let delta = 40_f32.min(horizontal.max);
        assert!(control.pan(&horizontal.key, delta, horizontal.max));
        let after = view(&markdown, &control);
        let mut actual = vec![];
        offsets(&after, None, &mut actual);
        assert!(
            actual
                .iter()
                .any(|(key, x)| key == &horizontal.key && (*x - delta).abs() < 0.01),
            "{}: {actual:?}",
            horizontal.key
        );
        assert!(
            regions
                .horizontal_at([horizontal.rect.x + 1., horizontal.rect.y + 1.])
                .is_some()
        );
    }
}

#[test]
fn mermaid_expand_opens_the_full_vector_canvas_from_the_shared_body() {
    fn find<'a>(node: &'a c::Layout, suffix: &str) -> Option<&'a c::Layout> {
        if matches!(&node.kind, c::LayoutKind::Named(name) if name.ends_with(suffix)) { return Some(node); }
        node.children.iter().find_map(|child| find(child, suffix))
    }
    for theme in [ui::Theme::Light, ui::Theme::Dark] {
        let ctx = ui::Context::new(theme, ui::Density::Cozy);
        let layout = bbt_doc::parse("```mermaid\nflowchart LR\n A[Start] --> B[Finish]\n```\n")
            .body_element_with_links(None, ctx, &Metrics, 500., &[], &RecordLinks::new())
            .layout(Rect::new(0., 0., 500., 0.), ctx, &Metrics).unwrap();
        let expand = find(&layout, "/expand").expect("clickable expand control");
        let at = [expand.rect.x + expand.rect.w / 2., expand.rect.y + expand.rect.h / 2.];
        let regions = Regions::retained(&layout);
        let Some(Action::ExpandDiagram(key)) = regions.hit(at) else { panic!("expand action"); };
        assert_eq!(regions.cursor(at), ui::cursor::Cursor::Pointer);
        let canvas = regions.diagram(&key).expect("retained full canvas");
        assert!(canvas.rect.w > 0. && canvas.rect.h > 0.);
        let mut viewer = ui::modules::document_modal::viewer::Viewer::new(canvas, [1200., 800.]).unwrap();
        let (fullscreen, _) = viewer.layout([1200., 800.], ctx, &Metrics).unwrap();
        assert_eq!(fullscreen.rect, Rect::new(0., 0., 1200., 800.));
        assert!(!c::paint(&fullscreen, ctx).is_empty());
    }
}

#[test]
fn mermaid_expand_keeps_repeated_transcript_block_names_separate() {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let make = |source, y| bbt_doc::parse(source)
        .body_element_with_links(None, ctx, &Metrics, 500., &[], &RecordLinks::new())
        .layout(Rect::new(0., y, 500., 0.), ctx, &Metrics).unwrap();
    let first = make("```mermaid\nflowchart LR\n A --> B\n```\n", 0.);
    let second = make("```mermaid\nflowchart LR\n A --> B --> C --> D\n```\n", first.rect.h + 20.);
    fn expand(n: &c::Layout) -> Option<[f32; 2]> {
        if matches!(&n.kind, c::LayoutKind::Named(name) if name.ends_with("/expand")) { return Some([n.rect.x + 1., n.rect.y + 1.]); }
        n.children.iter().find_map(expand)
    }
    let at = [expand(&first).unwrap(), expand(&second).unwrap()];
    let root = c::Layout { rect: Rect::new(0., 0., 500., second.rect.bottom()), baseline: 0., kind: c::LayoutKind::Named("transcript".into()), children: vec![first, second] };
    let regions = Regions::retained(&root);
    let keys: Vec<_> = at.into_iter().map(|at| match regions.hit(at).unwrap() { Action::ExpandDiagram(key) => key, _ => panic!("expand") }).collect();
    assert_ne!(keys[0], keys[1]);
    assert_ne!(regions.diagram(&keys[0]).unwrap().rect.w, regions.diagram(&keys[1]).unwrap().rect.w);
}
