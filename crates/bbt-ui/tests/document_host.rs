use bbt_ui::{
    self as ui, components as c,
    modules::document_modal::{Document, DocumentModal, controller::*},
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
fn ctx() -> ui::Context {
    ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)
}
#[test]
fn link_hit_regions_follow_the_scroll_clip_and_disable_missing_links() {
    let child = link(
        c::Element::Text(c::Text::new("#44", ui::Text::default())),
        "#44",
        true,
    );
    let element = c::Element::ScrollView(c::ScrollView {
        viewport: [200., 10.],
        offset: [0., 5.],
        content_width: None,
        content: Box::new(child),
    });
    let layout = element
        .layout(Rect::new(20., 30., 200., 10.), ctx(), &Metrics)
        .unwrap();
    let regions = Regions::retained(&layout);
    assert_eq!(regions.hit([22., 31.]), Some(Action::Open("#44".into())));
    assert_eq!(regions.hit([22., 29.]), None);
    assert_eq!(regions.hit([22., 41.]), None);
    let missing = link(
        c::Element::Text(c::Text::new("#999 · no record", ui::Text::default())),
        "#999",
        false,
    );
    let layout = missing
        .layout(Rect::new(0., 0., 200., 30.), ctx(), &Metrics)
        .unwrap();
    assert!(Regions::retained(&layout).hit([2., 2.]).is_none());
}
#[test]
fn modal_scroll_clamps_at_both_ends_and_the_footer_closes() {
    let shell = DocumentModal {
        document: Document {
            title: "A record".into(),
            body: (0..60).map(|_| "A full paragraph.\n\n").collect(),
            ..Default::default()
        },
        modal: true,
        viewport: [1100., 600.],
        ..Default::default()
    };
    let layout = shell
        .element(ctx(), &Metrics, [1100., 600.])
        .layout(Rect::new(0., 0., 980., 492.), ctx(), &Metrics)
        .unwrap();
    let regions = Regions::retained(&layout);
    assert!(regions.extent[1] > 0.);
    let mut control = Controller::default();
    control.scroll([0., f32::MAX], regions.extent);
    assert_eq!(control.offset[1], regions.extent[1]);
    control.scroll([0., -f32::MAX], regions.extent);
    assert_eq!(control.offset[1], 0.);
    fn close(n: &c::Layout) -> Option<Rect> {
        if matches!(&n.kind,c::LayoutKind::Named(s) if s=="action6") {
            Some(n.rect)
        } else {
            n.children.iter().find_map(close)
        }
    }
    let r = close(&layout).unwrap();
    assert_eq!(
        regions.hit([r.x + r.w / 2., r.y + r.h / 2.]),
        Some(Action::Close)
    );
}
#[test]
fn stream_numbered_links_and_attachments_are_measured_actions_without_paint_changes() {
    use ui::modules::{
        composer::Attachment,
        stream_dock::{self as dock, Action as StreamAction, Entry},
    };
    let stream = dock::StreamDock {
        entries: vec![Entry::Message {
            data: Box::new(dock::Message {
                text: "Read [[#44]] and the attachment".into(),
                attach: vec![Attachment {
                    path: ".bboxy/notes/record.md".into(),
                    title: "Record".into(),
                    kind: "artifact".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            show_channel: None,
        }],
        ..Default::default()
    };
    let layout = stream
        .element(ctx(), &Metrics)
        .layout(Rect::new(0., 0., 300., 500.), ctx(), &Metrics)
        .unwrap();
    let before = format!("{:?}", c::paint(&layout, ctx()));
    let mut regions = dock::Regions::dock(&layout, &stream);
    regions.record_text_links(&layout, &Metrics);
    assert_eq!(before, format!("{:?}", c::paint(&layout, ctx())));
    let mut number = false;
    let mut attachment = false;
    for y in 0..500 {
        for x in 0..300 {
            match regions.hit([x as f32, y as f32]) {
                Some(StreamAction::OpenRecord(s)) if s == "#44" => number = true,
                Some(StreamAction::OpenRecord(s)) if s == ".bboxy/notes/record.md" => {
                    attachment = true
                }
                _ => {}
            }
        }
    }
    assert!(number);
    assert!(attachment);
}

#[test]
fn a_host_read_error_is_visible_without_claiming_a_frontmatter_failure() {
    let view = DocumentModal {
        document: Document {
            title: "#999".into(),
            body: "Open again to retry.".into(),
            ..Default::default()
        },
        notice: Some("not_found: no record".into()),
        ..Default::default()
    };
    let layout = view
        .element(ctx(), &Metrics, [600., 500.])
        .layout(Rect::new(0., 0., 600., 0.), ctx(), &Metrics)
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
    let visible = text(&layout);
    assert!(visible.contains("not_found: no record"));
    assert!(!visible.contains("frontmatter does not parse"));
}
