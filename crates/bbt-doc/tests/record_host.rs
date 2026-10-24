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
