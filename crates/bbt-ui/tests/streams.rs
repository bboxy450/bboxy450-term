use bbt_ui::{self as ui, components as c, modules::stream_dock::*};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, s: &str, t: &TextStyle) -> f32 {
        s.chars().count() as f32 * t.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, t: &TextStyle) -> [f32; 2] {
        [t.size * 0.8, t.size * 0.2]
    }
}
fn layout(view: StreamDock, width: f32) -> c::Layout {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    view.element(ctx, &Metrics)
        .layout(Rect::new(0., 0., width, 0.), ctx, &Metrics)
        .unwrap()
}
fn labels(n: &c::Layout) -> String {
    let mut out = if let c::LayoutKind::Text { runs, .. } = &n.kind {
        runs.iter().map(|r| r.text.clone()).collect::<String>()
    } else {
        String::new()
    };
    for c in &n.children {
        out += &labels(c);
    }
    out
}
fn node<'a>(tree: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&tree.kind, c::LayoutKind::Named(name) if name == key) {
        Some(tree)
    } else {
        tree.children.iter().find_map(|n| node(n, key))
    }
}
fn messages(rows: Vec<Message>) -> StreamDock {
    StreamDock {
        dock: true,
        framed: true,
        entries: rows
            .into_iter()
            .map(|data| Entry::Message {
                data: Box::new(data),
                show_channel: None,
            })
            .collect(),
    }
}
#[test]
fn chronology_foreign_authority_and_unknown_delivery_remain_independent() {
    let a = Message {
        from: "first worker".into(),
        role: "executor".into(),
        text: "first payload".into(),
        evidence: Some("native-unknown".into()),
        ..Default::default()
    };
    let b = Message {
        from: "foreign oracle".into(),
        role: "oracle".into(),
        foreign: true,
        text: "second payload".into(),
        ..Default::default()
    };
    let tree = layout(messages(vec![a, b]), 300.);
    let text = labels(&tree);
    assert!(text.find("first payload").unwrap() < text.find("second payload").unwrap());
    let ring = node(&tree, "e0/body/head/ticks/hollow").unwrap();
    assert_eq!([ring.rect.w, ring.rect.h], [8., 8.]);
    assert!(!text.contains("p0"));
    assert!(!text.contains("failed"));
}
#[test]
fn ended_subagent_holds_the_target_and_hides_the_delivery_ladder() {
    let row = Message {
        from: "operator".into(),
        role: "oracle".into(),
        text: "review this".into(),
        to: Some("Review W3".into()),
        held: Some("agent_ended".into()),
        evidence: Some("application-acknowledged".into()),
        ..Default::default()
    };
    let text = labels(&layout(messages(vec![row]), 520.));
    assert!(text.contains("Review W3"));
    assert!(text.contains("held · agent ended"));
    assert!(!text.contains('✓'));
    assert!(!text.contains("via main"));
}
#[test]
fn collapsed_epics_and_bounded_newest_first_external_lane_use_model_state() {
    let view = StreamDock {
        dock: true,
        framed: true,
        entries: vec![
            Entry::Epic {
                data: Epic {
                    title: "closed epic".into(),
                    open: Some(false),
                    items: vec![WorkItem {
                        title: "hidden child".into(),
                        state: "accepted".into(),
                        ..Default::default()
                    }],
                },
            },
            Entry::External {
                events: (0..7)
                    .map(|i| External {
                        source: "ci".into(),
                        text: format!("event-{i}"),
                        time: "12:00".into(),
                    })
                    .collect(),
                open: true,
            },
        ],
    };
    let text = labels(&layout(view, 300.));
    assert!(text.contains("1/1"));
    assert!(!text.contains("hidden child"));
    assert!(!text.contains("event-0"));
    assert!(!text.contains("event-1"));
    assert!(text.find("event-6").unwrap() < text.find("event-2").unwrap());
}
#[test]
fn message_body_reflows_with_width_without_changing_the_model() {
    let view = messages(vec![Message {
        from: "worker".into(),
        role: "executor".into(),
        text: "many words in a narrow stream must reflow instead of being clipped away".into(),
        ..Default::default()
    }]);
    let narrow = layout(view.clone(), 250.);
    let wide = layout(view, 700.);
    assert!(narrow.rect.h > wide.rect.h);
    assert!(labels(&narrow).contains("many words"));
}
#[test]
fn routed_chips_keep_the_oracle_tint_visible_and_unknown_is_a_dashed_hollow_mark() {
    let tree = layout(
        messages(vec![Message {
            role: "oracle".into(),
            from: "operator".into(),
            to: Some("review".into()),
            evidence: Some("native-unknown".into()),
            text: "inspect this".into(),
            ..Default::default()
        }]),
        520.,
    );
    let chip = node(&tree, "e0/body/head/agchip").unwrap();
    let c::LayoutKind::Box { style, .. } = &chip.children[0].kind else {
        panic!("chip box")
    };
    assert_eq!(style.fill[3], 0.);
    let ring = node(&tree, "e0/body/head/ticks/hollow").unwrap();
    let c::LayoutKind::Box { style, .. } = &ring.children[0].kind else {
        panic!("unknown ring")
    };
    assert!(style.dashed);
    assert_eq!(style.fill[3], 0.);
}
#[test]
fn unread_marks_stay_round_and_compressed_chips_clip_their_text() {
    for dock in [false, true] {
        let mut view = messages(vec![Message {
            role: "oracle".into(),
            from: "Vesper Greywind".into(),
            unread: true,
            group: Some(Group {
                name: "an exceptionally long rendering group name".into(),
                n: 12,
            }),
            time: "14:35".into(),
            text: "payload".into(),
            ..Default::default()
        }]);
        view.dock = dock;
        let tree = layout(view, 300.);
        let dot = node(&tree, "e0/unread-dot").unwrap();
        let size = if dock { 5. } else { 7. };
        assert_eq!([dot.rect.w, dot.rect.h], [size, size]);
        let chip = node(&tree, "e0/body/head/agchip").unwrap();
        let c::LayoutKind::Box { clip, .. } = &chip.children[0].kind else {
            panic!("chip box")
        };
        assert!(clip);
        let text = node(&tree, "e0/body/head/agchip/text").unwrap();
        assert!(text.rect.right() <= chip.rect.right());
    }
}
