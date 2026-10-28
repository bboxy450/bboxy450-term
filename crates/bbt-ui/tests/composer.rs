// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{modules::composer::*, text_edit::Command as Edit};

// keeps: spec ux-prototype — 160–520px resizable sidebar
#[test]
fn sidebar_footer_keeps_send_delegate_and_kind_controls_inside_its_width() {
    use bbt_ui::{self as ui, components as c};
    use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
    struct Metrics;
    impl TextMeasure for Metrics {
        fn width(&self, text: &str, style: &TextStyle) -> f32 {
            text.chars().count() as f32 * style.size * 0.5
        }
    }
    impl c::Measure for Metrics {
        fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
            [style.size * 0.8, style.size * 0.2]
        }
    }
    fn find<'a>(layout: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
        if matches!(&layout.kind, c::LayoutKind::Named(name) if name == key) {
            return Some(layout);
        }
        layout.children.iter().find_map(|child| find(child, key))
    }
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    for mode in ["", "delegate"] {
        for kind in ["text", "decision"] {
            for width in (160..=520).step_by(4) {
                let view = Composer {
                    mode: mode.into(),
                    kind: kind.into(),
                    ..Default::default()
                };
                let layout = view
                    .element(ctx, width as f32, &Metrics)
                    .unwrap()
                    .layout(Rect::new(0., 0., width as f32, 0.), ctx, &Metrics)
                    .unwrap();
                for key in ["composer/send", "composer/priority", "composer/kpick"] {
                    let rect = find(&layout, key).unwrap().rect;
                    assert!(
                        rect.x >= 0. && rect.x + rect.w <= width as f32 + 0.01,
                        "{mode}/{kind}/{width}/{key}: {rect:?}"
                    );
                }
            }
        }
    }
}

fn address(id: &str) -> Address {
    Address {
        kind: "seat".into(),
        label: id.into(),
        id: id.into(),
        destination: format!("seats/{id}/inbox"),
        ..Default::default()
    }
}
fn composer() -> Controller {
    Controller::new(Composer {
        to: vec![address("seat-a")],
        focus: true,
        ..Default::default()
    })
}

// keeps: spec gui-components §4.1
#[test]
fn preedit_does_not_send_or_filter_and_escape_restores_the_selection() {
    let mut c = composer();
    c.replace_choices(vec![PickGroup {
        title: "Seats".into(),
        items: vec![address("seat-a"), address("seat-b")],
    }]);
    c.handle(Command::Focus(Field::Address));
    c.handle(Command::Edit(Edit::Insert("seat".into())));
    assert_eq!(c.view.pick[0].items.len(), 2);
    c.handle(Command::Edit(Edit::Preedit {
        text: "🙂".into(),
        caret: Some(2),
    }));
    assert_eq!(c.view.typed, "seat");
    assert_eq!(c.view.pick[0].items.len(), 2);
    assert_eq!(c.handle(Command::Choose(1)), Outcome::Local);
    assert_eq!(c.handle(Command::Send), Outcome::Local);
    c.handle(Command::Escape);
    assert_eq!(c.view.typed, "seat");
    c.handle(Command::Choose(1));
    assert_eq!(c.view.to[1].destination, "seats/seat-b/inbox");
}

// keeps: spec gui-components §4.1
#[test]
fn submission_is_a_snapshot_and_a_stored_reply_does_not_erase_new_typing() {
    let mut c = composer();
    c.handle(Command::Edit(Edit::Insert("first".into())));
    let Outcome::Submit(sent) = c.handle(Command::Send) else {
        panic!("send");
    };
    assert_eq!(c.view.text, "first");
    c.handle(Command::Edit(Edit::Insert(" second".into())));
    c.stored(&sent);
    assert_eq!(c.view.text, "first second");
    let Outcome::Submit(next) = c.handle(Command::Send) else {
        panic!("send");
    };
    c.stored(&next);
    assert_eq!(c.view.text, "");
    assert_eq!(c.view.to.len(), 1);
}

// keeps: spec gui-components §4.1
#[test]
fn unresolved_labels_and_invalid_priority_are_named_refusals() {
    let mut c = composer();
    c.view.to[0].destination.clear();
    c.handle(Command::Edit(Edit::Insert("payload".into())));
    assert_eq!(
        c.handle(Command::Send),
        Outcome::Refused("destination_required")
    );
    assert_eq!(
        c.handle(Command::Priority(128)),
        Outcome::Refused("priority_out_of_range")
    );
    assert_eq!(c.view.priority, 0);
}

// keeps: spec gui-components §4.1
#[test]
fn kind_chooser_is_local_and_offers_only_the_three_sendable_kinds() {
    let mut c = composer();
    c.handle(Command::Edit(Edit::Insert("a decision".into())));
    c.handle(Command::ToggleKindMenu);
    assert!(c.view.kind_menu);
    c.handle(Command::Next { backwards: false });
    assert_eq!(c.view.kind, "chunk");
    c.handle(Command::Next { backwards: false });
    assert_eq!(c.view.kind, "decision");
    assert_eq!(c.handle(Command::Send), Outcome::Changed);
    assert!(!c.view.kind_menu);
    assert_eq!(c.view.text, "a decision");
    let Outcome::Submit(snapshot) = c.handle(Command::Send) else {
        panic!("send after choosing");
    };
    assert_eq!(snapshot.kind, "decision");
    assert_eq!(
        c.handle(Command::Kind("task".into())),
        Outcome::Refused("message_kind_not_sendable")
    );
}
// keeps: spec gui-components §4.1
#[test]
fn accepting_a_report_requires_reported_status_and_zero_exit() {
    let mut report = Report {
        status: "reported".into(),
        ..Default::default()
    };
    assert!(!report.can_accept());
    report.verify_result = Some(Verification {
        exit_code: 1,
        ..Default::default()
    });
    assert!(!report.can_accept());
    report.verify_result.as_mut().unwrap().exit_code = 0;
    assert!(report.can_accept());
    report.status = "running".into();
    assert!(!report.can_accept());
}

// keeps: spec gui-components §4.1
#[test]
fn attachment_query_is_uncommitted_until_pick_and_preedit_never_filters_or_submits() {
    let mut c = composer();
    c.replace_attachments(vec![
        Attachment {
            id: "task-a".into(),
            path: ".bboxy/tasks/a.md".into(),
            kind: "task".into(),
            title: "Adapters".into(),
            status: "open".into(),
            ..Default::default()
        },
        Attachment {
            id: "artifact-b".into(),
            path: ".bboxy/artifacts/b.md".into(),
            kind: "artifact".into(),
            title: "Adapter image".into(),
            ..Default::default()
        },
    ]);
    c.handle(Command::Edit(Edit::Insert("hello ".into())));
    c.handle(Command::Edit(Edit::Insert("[".into())));
    c.handle(Command::Edit(Edit::Insert("[".into())));
    assert_eq!(c.view.text, "hello ");
    assert!(c.view.attach_pick.is_some());
    c.handle(Command::Edit(Edit::Insert("adap".into())));
    c.handle(Command::Edit(Edit::Preedit {
        text: "no match".into(),
        caret: Some(8),
    }));
    assert_eq!(c.view.attach_pick.as_ref().unwrap().query, "adap");
    assert_eq!(c.view.attach_pick.as_ref().unwrap().rows().len(), 2);
    assert_eq!(c.handle(Command::ChooseSelected), Outcome::Local);
    assert_eq!(c.handle(Command::Send), Outcome::Local);
    c.handle(Command::Edit(Edit::CancelPreedit));
    c.handle(Command::ChooseSelected);
    assert!(c.view.attach_pick.is_none());
    assert_eq!(c.view.attach[0].path, ".bboxy/tasks/a.md");
    assert_eq!(c.view.text, "hello ");
}
// keeps: spec gui-components §4.1
#[test]
fn reject_reason_requires_committed_nonempty_text_and_survives_refusal() {
    let mut reject = RejectController::default();
    assert_eq!(reject.submission(), Err("reject_reason_required"));
    reject.handle(Edit::Preedit {
        text: "add a test".into(),
        caret: Some(10),
    });
    assert_eq!(reject.submission(), Err("preedit_pending"));
    reject.handle(Edit::Commit("add a test".into()));
    assert_eq!(reject.submission().unwrap(), "add a test");
    reject.refused("task changed");
    assert_eq!(reject.view.reason, "add a test");
    assert_eq!(reject.view.notice.as_ref().unwrap().text, "task changed");
}

// keeps: spec gui-components §4.1
#[test]
fn task_status_is_canonical_in_graph_records_without_renaming_seat_activity() {
    use bbt_ui::modules::graph_tab::{Seat, Work};
    let seat: Seat = serde_json::from_value(
        serde_json::json!({"state":"active","tasks":[{"title":"Task","status":"reported"}]}),
    )
    .unwrap();
    assert_eq!(seat.state, "active");
    assert_eq!(seat.tasks[0].state, "reported");
    let old: Work = serde_json::from_value(serde_json::json!({"state":"running"})).unwrap();
    assert_eq!(old.state, "running");
    assert_eq!(serde_json::to_value(&old).unwrap()["status"], "running");
}
