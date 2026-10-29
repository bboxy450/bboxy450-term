// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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

// keeps: operator role-first identity across seat lists, pane strips and messages.
#[test]
fn message_identity_puts_explicit_role_before_first_name_without_changing_sender_data() {
    for (kind, role, name, expected_role, expected_name) in [
        ("seat", "oracle", "Vesper Greywind", "ORACLE", "Vesper"),
        (
            "seat",
            "architect",
            "Nyx\u{a0}Copperwing",
            "ARCHITECT",
            "Nyx",
        ),
        (
            "seat",
            "creativity",
            "  Wren Starling ",
            "CREATIVITY",
            "Wren",
        ),
        ("seat", "executor", "Dax Riven", "EXECUTOR", "Dax"),
        ("seat", "reviewer", "🦊 Boxy", "REVIEWER", "🦊"),
        ("operator", "oracle", "Boxy User", "OPERATOR", "Boxy"),
        ("operator-local", "oracle", "Boxy User", "OPERATOR", "Boxy"),
        ("seat", "", "Dax Riven", "UNKNOWN", "Dax"),
        ("seat", " \u{a0} ", "Dax Riven", "UNKNOWN", "Dax"),
    ] {
        let message = Message {
            author: Some(Author {
                kind: kind.into(),
                id: "stable-actor-id".into(),
                name: name.into(),
            }),
            from: "legacy sender".into(),
            role: role.into(),
            text: "Complete message body is unchanged.".into(),
            ..Default::default()
        };
        let tree = layout(messages(vec![message.clone()]), 320.);
        let role_node = node(&tree, "e0/body/head/role").unwrap();
        let name_node = node(&tree, "e0/body/head/who").unwrap();
        assert_eq!(labels(role_node), expected_role);
        assert_eq!(labels(name_node), expected_name);
        assert!(role_node.rect.right() <= name_node.rect.x);
        assert_eq!(message.author.as_ref().unwrap().name, name);
        assert_eq!(message.author.as_ref().unwrap().id, "stable-actor-id");
        assert_eq!(message.from, "legacy sender");
        assert!(labels(&tree).contains(&message.text));
    }
    let tree = layout(
        messages(vec![Message {
            from: "Legacy Sender".into(),
            ..Default::default()
        }]),
        320.,
    );
    assert_eq!(
        labels(node(&tree, "e0/body/head/role").unwrap()),
        "UNSIGNED"
    );
}

#[test]
fn narrow_message_identity_keeps_role_width_while_name_yields() {
    let message = Message {
        from: "AnUnreasonablyLongGivenNameThatMustYield Surname".into(),
        role: "creativity".into(),
        time: "11:06".into(),
        text: "body".into(),
        ..Default::default()
    };
    for width in [180., 240., 360.] {
        let tree = layout(messages(vec![message.clone()]), width);
        let head = node(&tree, "e0/body/head").unwrap().rect;
        let role = node(&tree, "e0/body/head/role").unwrap();
        let name = node(&tree, "e0/body/head/who").unwrap();
        assert_eq!(labels(role), "CREATIVITY");
        assert!(
            role.rect.w
                >= "CREATIVITY".len() as f32 * bbt_ui_model::STYLE.r#type.caption as f32 * 0.5
                    - 0.01
        );
        assert!(role.rect.x >= head.x && role.rect.right() <= head.right());
        assert!(name.rect.x >= role.rect.right());
        assert!(name.rect.right() <= head.right() + 0.01);
    }
}

#[test]
fn rejected_role_first_identity_retains_the_wrapped_flex_allocation() {
    let message = Message {
        from: "Rook Ashwhisker".into(),
        role: "executor".into(),
        rejected: true,
        time: "14:36".into(),
        delegator: "Vesper Greywind".into(),
        ..Default::default()
    };
    for width in [220., 260., 300.] {
        let tree = layout(messages(vec![message.clone()]), width);
        let identity = node(&tree, "e0/rj-t").unwrap();
        let time = node(&tree, "e0/time").unwrap();
        assert!(labels(identity).contains("EXECUTOR · Rook"));
        assert!(!labels(identity).contains("Ashwhisker"));
        assert!((identity.rect.right() - time.rect.right()).abs() < 0.01);
    }
}
// keeps: spec gui-components §1.3
#[test]
fn task_review_hits_follow_the_painted_clip_and_zero_exit_gate() {
    use bbt_ui::modules::composer::{Report, Verification};
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let mut view = StreamDock {
        dock: true,
        framed: false,
        entries: vec![Entry::Report {
            id: "actual-task-id".into(),
            data: Report {
                task: "Real task".into(),
                status: "reported".into(),
                summary: "done".into(),
                verify_result: Some(Verification {
                    exit_code: 1,
                    ..Default::default()
                }),
                ..Default::default()
            },
            review: true,
        }],
    };
    let tree = layout(view.clone(), 300.);
    let accept = node(&tree, "e0/accept").unwrap().rect;
    let at = [accept.x + accept.w / 2., accept.y + accept.h / 2.];
    assert_eq!(Regions::dock(&tree, &view).hit(at), None);
    assert_eq!(
        Regions::dock(&tree, &view).cursor(at),
        ui::cursor::Cursor::Default
    );
    let Entry::Report { data, .. } = &mut view.entries[0] else {
        unreachable!();
    };
    data.verify_result.as_mut().unwrap().exit_code = 0;
    let tree = layout(view.clone(), 300.);
    assert_eq!(
        Regions::dock(&tree, &view).hit(at),
        Some(Action::Accept("actual-task-id".into()))
    );
    assert_eq!(
        Regions::dock(&tree, &view).cursor(at),
        ui::cursor::Cursor::Pointer
    );
    let scroll = c::Element::ScrollView(c::ScrollView {
        viewport: [300., 12.],
        offset: [0.; 2],
        content_width: Some(300.),
        content: Box::new(view.element(ctx, &Metrics)),
    })
    .layout(Rect::new(0., 0., 300., 12.), ctx, &Metrics)
    .unwrap();
    assert_eq!(Regions::dock(&scroll, &view).hit(at), None);
    assert_eq!(
        Regions::dock(&scroll, &view).cursor(at),
        ui::cursor::Cursor::Default
    );
}
// keeps: spec gui-components §1.3
#[test]
fn report_rim_surrounds_the_clipped_card_without_changing_its_geometry() {
    use bbt_ui::modules::composer::Report;
    let view = messages(vec![Message {
        text: "a report".into(),
        report: Some(Report {
            task: "real task key".into(),
            summary: "summary inside the clipped report".into(),
            ..Default::default()
        }),
        ..Default::default()
    }]);
    for theme in [ui::Theme::Dark, ui::Theme::Light] {
        let ctx = ui::Context::new(theme, ui::Density::Cozy);
        let tree = view
            .element(ctx, &Metrics)
            .layout(Rect::new(0., 0., 300., 0.), ctx, &Metrics)
            .unwrap();
        let card = node(&tree, "e0/body/report").unwrap();
        assert_eq!(
            node(&tree, "e0/body/report/wrapper").unwrap().rect,
            card.rect
        );
        let ops = c::paint(&tree, ctx);
        let (rim_index, state, clips) = ops
            .iter()
            .enumerate()
            .find_map(|(i, op)| match op {
                c::Paint::Material {
                    rect,
                    rim: Some(rim),
                    clips,
                    ..
                } if *rect == card.rect => Some((i, rim, clips)),
                _ => None,
            })
            .unwrap();
        assert_eq!(state.activity, ui::GlowActivity::Idle);
        assert_eq!(state.seed, ui::rim_phase("real task key"));
        assert_eq!(
            state.tint,
            Some(ui::Paint::solid(ui::Color::Accent).resolve(ctx).unwrap())
        );
        assert_eq!(state.depth, Some(0.));
        assert_eq!(
            state.outer,
            Some(bbt_ui_model::TOKENS.glow.rim as f32 * 0.5)
        );
        assert!(!clips.iter().any(|clip| clip.rect == card.rect));
        assert!(ops[..rim_index].iter().any(|op| matches!(op, c::Paint::Draw(list) if list.primitives.iter().any(|p| matches!(p, bbt_ui_model::draw::Prim::Text { .. })))));
    }
}
// keeps: spec gui-components §1.3
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
// keeps: spec gui-components §1.3
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
// keeps: spec gui-components §1.3
#[test]
fn collapsed_epics_and_bounded_newest_first_external_lane_use_model_state() {
    let view = StreamDock {
        dock: true,
        framed: true,
        entries: vec![
            Entry::Epic {
                data: Epic {
                    title: "closed epic".into(),
                    delegate: false,
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
// keeps: spec gui-components §1.3
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
// keeps: spec gui-components §1.3
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
// keeps: spec gui-components §1.3
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

// keeps: spec gui-components §1.3
#[test]
fn incomplete_channel_counts_keep_more_and_explicit_swarm_total() {
    use bbt_ui::modules::stream_dock::UnreadCount;
    let view:StreamDock=serde_json::from_value(serde_json::json!({"dock":true,"entries":[{"kind":"tabs","selected":"all","total":12,"channels":[{"name":"render","unread":"more"},{"name":"decisions","unread":3}]}]})).unwrap();
    let text = labels(&layout(view, 300.));
    assert!(text.contains("more"));
    assert!(text.contains("12"));
    assert!(serde_json::from_value::<UnreadCount>(serde_json::json!("12")).is_err());
    assert!(serde_json::from_value::<UnreadCount>(serde_json::json!(-1)).is_err());
    assert_eq!(serde_json::to_value(UnreadCount::More).unwrap(), "more");
}

// keeps: spec gui-components §1.3
#[test]
fn a_changed_task_keeps_its_reject_draft_visible_without_an_enabled_mutation() {
    use bbt_ui::modules::composer::{RejectController, Report};
    let reason = RejectController::new("keep this rebrief".into());
    let view = StreamDock {
        dock: true,
        framed: false,
        entries: vec![Entry::Report {
            id: "actual-task".into(),
            data: Report {
                task: "Task changed".into(),
                status: "accepted".into(),
                reject: Some(reason.view),
                ..Default::default()
            },
            review: false,
        }],
    };
    let tree = layout(view.clone(), 300.);
    assert!(node(&tree, "e0/reject/input").is_some());
    assert!(labels(&tree).contains("keep this rebrief"));
    let submit = node(&tree, "e0/reject/submit").unwrap().rect;
    assert_eq!(
        Regions::dock(&tree, &view).hit([submit.x + submit.w / 2., submit.y + submit.h / 2.]),
        None
    );
    assert_eq!(
        Regions::dock(&tree, &view).cursor([submit.x + submit.w / 2., submit.y + submit.h / 2.]),
        ui::cursor::Cursor::Default
    );
    let input = node(&tree, "e0/reject/input").unwrap().rect;
    assert_eq!(
        Regions::dock(&tree, &view).cursor([input.x + input.w / 2., input.y + input.h / 2.]),
        ui::cursor::Cursor::Text
    );
    let cancel = node(&tree, "e0/reject/cancel").unwrap().rect;
    assert_eq!(
        Regions::dock(&tree, &view).hit([cancel.x + cancel.w / 2., cancel.y + cancel.h / 2.]),
        Some(Action::CancelReject("actual-task".into()))
    );
    assert_eq!(
        Regions::dock(&tree, &view).cursor([cancel.x + cancel.w / 2., cancel.y + cancel.h / 2.]),
        ui::cursor::Cursor::Pointer
    );
}

// keeps: operator chat cleanup — readable cards retain messages and exceptional state.
#[test]
fn ordinary_messages_are_quiet_cards_and_explicit_local_times_win() {
    let ordinary = Message {
        author: Some(Author {
            kind: "operator".into(),
            id: "operator".into(),
            name: "Boxy".into(),
        }),
        role: "oracle".into(),
        issuer_kind: Some("local-dev".into()),
        signature: Some("signed".into()),
        priority_class: Some("oracle".into()),
        kind: Some("chunk".into()),
        time: "11:06".into(),
        ts: Some(18 * 3_600_000 + 6 * 60_000),
        text: "The complete message stays readable and can wrap across the card.".into(),
        receipts: Some(Receipts {
            delivered: 2,
            acked: 2,
            held: 0,
        }),
        ..Default::default()
    };
    let held = Message {
        priority: 8,
        kind: Some("decision".into()),
        receipts: Some(Receipts {
            delivered: 1,
            acked: 0,
            held: 1,
        }),
        ..ordinary.clone()
    };
    let tree = layout(messages(vec![ordinary.clone(), held]), 300.);
    let first = node(&tree, "e0").unwrap();
    let text = labels(first);
    assert!(text.contains("Boxy") && text.contains(&ordinary.text));
    assert!(text.contains("11:06") && !text.contains("18:06"));
    for noise in [
        "local-dev",
        "<OPERATOR>",
        "p0",
        "chunk",
        "delivered",
        "acked",
        "signature: signed",
    ] {
        assert!(!text.contains(noise), "{noise}");
    }
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    assert!(
        matches!(&first.children[0].kind, c::LayoutKind::Box { style, .. } if style.fill == ui::Paint::solid(ui::Color::Surface).resolve(ctx).unwrap())
    );
    let head = node(&tree, "e0/body/head").unwrap().rect;
    let body = node(&tree, "e0/body/text").unwrap().rect;
    assert!(body.y >= head.bottom() + bbt_ui_model::STYLE.spacing.s2 as f32 - 0.01);
    assert!(
        node(&tree, "e1").unwrap().rect.y
            >= first.rect.bottom() + bbt_ui_model::STYLE.spacing.s2 as f32 - 0.01
    );
    let exceptional = labels(node(&tree, "e1").unwrap());
    assert!(
        exceptional.contains("Priority 8")
            && exceptional.contains("DECISION")
            && exceptional.contains("1 held")
    );
}
