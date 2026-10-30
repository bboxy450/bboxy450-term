// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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

// keeps: spec document-viewer — provenance details are an explicit, clipped
// display action; no source reads or record authority are attached to the link.
#[test]
fn provenance_disclosure_defaults_collapsed_and_uses_clipped_retained_actions() {
    fn layout(name: &str, offset: f32) -> c::Layout {
        let toggle = c::Element::Named {
            name: name.into(),
            content: Box::new(c::Element::Text(c::Text::new(
                "Provenance details",
                ui::Text::default(),
            ))),
        };
        c::Element::ScrollView(c::ScrollView {
            viewport: [240., 10.],
            offset: [0., offset],
            content_width: None,
            content: Box::new(c::Element::Stack(c::Stack {
                direction: bbt_ui_model::layout::Axis::Column,
                gap: ui::Length::Px(0.),
                align: c::Alignment::Start,
                children: vec![
                    toggle.into(),
                    c::Element::Text(c::Text::new(
                        "Other document content.\n".repeat(20),
                        ui::Text::default(),
                    ))
                    .into(),
                ],
            })),
        })
        .layout(Rect::new(20., 30., 240., 10.), ctx(), &Metrics)
        .unwrap()
    }
    fn named(node: &c::Layout, name: &str) -> Rect {
        fn find(node: &c::Layout, name: &str) -> Option<Rect> {
            if matches!(&node.kind, c::LayoutKind::Named(key) if key == name) {
                Some(node.rect)
            } else {
                node.children.iter().find_map(|child| find(child, name))
            }
        }
        find(node, name).unwrap()
    }
    assert!(!Controller::default().provenance_expanded);
    for name in [
        "metadata/provenance/toggle",
        "document/body/metadata/provenance/toggle",
    ] {
        let visible = layout(name, 5.);
        let control = named(&visible, name);
        let clipped = intersection(control, visible.rect);
        assert!(clipped.w > 0. && clipped.h > 0.);
        let point = [clipped.x + clipped.w * 0.5, clipped.y + clipped.h * 0.5];
        let regions = Regions::retained(&visible);
        assert_eq!(regions.hit(point), Some(Action::ToggleProvenance));
        assert_eq!(regions.cursor(point), ui::cursor::Cursor::Pointer);
        let outside = [clipped.x + 1., visible.rect.y - 1.];
        assert_eq!(regions.hit(outside), None);
        assert_eq!(regions.cursor(outside), ui::cursor::Cursor::Default);
        let modal = Regions::retained_modal(&visible);
        assert_eq!(modal.hit(point), Some(Action::ToggleProvenance));
        assert_eq!(modal.hit(outside), Some(Action::Close));

        let hidden = layout(name, 100.);
        let control = named(&hidden, name);
        assert!(control.bottom() <= hidden.rect.y);
        let regions = Regions::retained(&hidden);
        let viewport_center = [hidden.rect.x + 1., hidden.rect.y + hidden.rect.h * 0.5];
        assert_eq!(regions.hit(viewport_center), None);
        assert_eq!(regions.cursor(viewport_center), ui::cursor::Cursor::Default);
    }
    let unrelated = layout("metadata/provenance/toggle-extra", 0.);
    assert_eq!(Regions::retained(&unrelated).hit([22., 31.]), None);
}

// keeps: spec document-viewer §Markdown
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
    assert_eq!(regions.cursor([22., 31.]), ui::cursor::Cursor::Pointer);
    assert_eq!(regions.hit([22., 29.]), None);
    assert_eq!(regions.cursor([22., 29.]), ui::cursor::Cursor::Default);
    assert_eq!(regions.hit([22., 41.]), None);
    assert_eq!(regions.cursor([22., 41.]), ui::cursor::Cursor::Default);
    let missing = link(
        c::Element::Text(c::Text::new("#999 · no record", ui::Text::default())),
        "#999",
        false,
    );
    let layout = missing
        .layout(Rect::new(0., 0., 200., 30.), ctx(), &Metrics)
        .unwrap();
    assert!(Regions::retained(&layout).hit([2., 2.]).is_none());
    assert_eq!(
        Regions::retained(&layout).cursor([2., 2.]),
        ui::cursor::Cursor::Default
    );
}
// keeps: spec document-viewer §Markdown
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
    assert_eq!(
        regions.cursor([r.x + r.w / 2., r.y + r.h / 2.]),
        ui::cursor::Cursor::Pointer
    );
    fn unavailable(n: &c::Layout) -> Option<Rect> {
        if matches!(&n.kind, c::LayoutKind::Named(s) if s == "action0") {
            Some(n.rect)
        } else {
            n.children.iter().find_map(unavailable)
        }
    }
    let r = unavailable(&layout).unwrap();
    assert!(matches!(
        regions.hit([r.x + r.w / 2., r.y + r.h / 2.]),
        Some(Action::Unavailable(_))
    ));
    assert_eq!(
        regions.cursor([r.x + r.w / 2., r.y + r.h / 2.]),
        ui::cursor::Cursor::Default
    );
}
// keeps: spec document-viewer §What it is — only the modal backdrop dismisses.
#[test]
fn modal_backdrop_closes_but_interior_gaps_controls_and_embedded_documents_do_not() {
    let shell = DocumentModal {
        document: Document {
            title: "Backdrop test".into(),
            body: "Selectable body".into(),
            ..Default::default()
        },
        modal: true,
        viewport: [1100., 600.],
        ..Default::default()
    };
    let layout = shell
        .element(ctx(), &Metrics, [1100., 600.])
        .layout(Rect::new(60., 54., 980., 492.), ctx(), &Metrics)
        .unwrap();
    let modal = Regions::retained_modal(&layout);
    let embedded = Regions::retained(&layout);
    let r = layout.rect;
    for point in [
        [r.x - 1., r.y + 1.],
        [r.right(), r.y + 1.],
        [r.x + 1., r.y - 1.],
        [r.x + 1., r.bottom()],
    ] {
        assert_eq!(modal.hit(point), Some(Action::Close));
        assert_eq!(embedded.hit(point), None);
    }
    for point in [
        [r.x + 1., r.y + 1.],
        [r.x + 1., r.y + r.h * 0.5],
        [r.x + 1., r.bottom() - 1.],
    ] {
        assert_eq!(modal.hit(point), None, "inside padding is not the backdrop");
    }
    for point in [[f32::NAN, r.y], [r.x, f32::INFINITY]] {
        assert_eq!(modal.hit(point), None);
    }
    fn action(node: &c::Layout, key: &str) -> Option<Rect> {
        if matches!(&node.kind,c::LayoutKind::Named(s) if s==key) {
            Some(node.rect)
        } else {
            node.children.iter().find_map(|n| action(n, key))
        }
    }
    let close = action(&layout, "action6").unwrap();
    assert_eq!(
        modal.hit([close.x + close.w * 0.5, close.y + close.h * 0.5]),
        Some(Action::Close)
    );
    let find = action(&layout, "action4").unwrap();
    assert!(matches!(
        modal.hit([find.x + find.w * 0.5, find.y + find.h * 0.5]),
        Some(Action::Unavailable(_))
    ));
    assert_eq!(
        modal.extent, embedded.extent,
        "dismissal metadata never changes scroll bounds"
    );
}
// keeps: spec document-viewer §Markdown
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

// keeps: spec document-viewer §Markdown
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

// keeps: spec document-viewer §Markdown
#[test]
fn a_nested_diagram_viewport_never_replaces_the_document_scroll_owner() {
    let shell = DocumentModal {
        modal: true,
        viewport: [900., 600.],
        ..Default::default()
    };
    let layout = shell
        .element_with_body(ctx(), &Metrics, [900., 600.], &|_| {
            let mut style = ui::Style::new(ui::Preset::Card);
            style.overrides.height = Some(ui::Length::Px(1000.));
            c::Element::Box(c::Box::new(
                style,
                vec![
                    c::Element::ScrollView(c::ScrollView {
                        viewport: [100., 20.],
                        offset: [0.; 2],
                        content_width: Some(1000.),
                        content: Box::new(c::Element::Text(c::Text::new(
                            "a wide diagram",
                            ui::Text::default(),
                        ))),
                    })
                    .into(),
                ],
            ))
        })
        .layout(Rect::new(0., 0., 846., 492.), ctx(), &Metrics)
        .unwrap();
    let regions = Regions::retained(&layout);
    assert!(regions.viewport.unwrap().w > 800.);
    assert!(regions.extent[1] > 500.);
    let mut scroll = Controller::default();
    assert!(scroll.scroll([0., 300.], regions.extent));
    assert_eq!(scroll.offset[1], 300.);
}

#[test]
fn missing_slug_aliases_and_numbers_are_broken_and_inert() {
    let mut links=RecordLinks::new();
    links.insert("missing".into(),None);links.insert("#12".into(),None);
    for (target,display,label) in [("missing","friendly alias","friendly alias · no record"),("#12","friendly alias","#12 · no record")] {
        let (actual,enabled)=presentation(target,display,Some(&links));
        assert_eq!(actual,label);assert!(!enabled);
        let linked=link(c::Element::Spacer,target,enabled);
        assert!(!matches!(linked,c::Element::Named { .. }));
    }
}
