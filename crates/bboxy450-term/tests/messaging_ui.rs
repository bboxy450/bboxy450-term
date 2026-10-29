// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxy450_term::{
    cockpit::UiMsg,
    messaging::{Client, Reply, Results, Status},
};
use bbt_mux::Ulid;
use bbt_proto::{
    SwarmId,
    messaging::{Request, Response},
};
use bbt_ui::modules::composer::{Address, Submission};

fn draft() -> Submission {
    Submission {
        delegate: false,
        to: vec![Address {
            kind: "seat".into(),
            id: Ulid::new().to_string(),
            label: "a misleading label".into(),
            destination: "seats/exact-path/inbox".into(),
            ..Default::default()
        }],
        attach: vec![],
        text: "decision text".into(),
        kind: "decision".into(),
        priority: 9,
    }
}
#[test]
fn send_captures_the_selected_swarm_exact_destination_and_stable_operation_id() {
    let c = Client::default();
    assert!(matches!(c.prepare(None, draft()), Err("swarm_required")));
    let swarm = SwarmId::from_u128(4);
    let batch = c.prepare(Some(swarm), draft()).unwrap();
    let UiMsg::Messaging {
        id,
        swarm_id,
        request: Request::Send { message },
    } = &batch.commands[0]
    else {
        panic!("send");
    };
    assert_eq!(*swarm_id, swarm);
    assert_eq!(*id, message.id);
    assert_eq!(message.destination, "seats/exact-path/inbox");
    assert_eq!(message.priority, 9);
    assert_eq!(message.kind, bbt_proto::messaging::MessageKind::Decision);
}
#[test]
fn unrelated_replies_cannot_clear_the_draft_and_durable_held_is_named() {
    let mut c = Client::default();
    let batch = c.prepare(Some(SwarmId::from_u128(1)), draft()).unwrap();
    let commands = c.admitted(batch);
    let UiMsg::Messaging { id, .. } = commands[0] else {
        panic!("send");
    };
    let mut results = Results::default();
    results.push(Ulid::new(), Reply::Refused("unrelated".into()));
    assert!(c.replies(&results).0.is_none());
    assert_eq!(c.status, Status::Pending);
    results.push(
        id,
        Reply::Messaging(Response::Held {
            id,
            reason: "identity pending".into(),
        }),
    );
    assert!(c.replies(&results).0.is_some());
    assert_eq!(c.status, Status::Held("identity pending".into()));
    assert!(c.replies(&results).0.is_none());
}

// keeps: a typed thread refusal leaves the composer draft intact and names its recovery.
#[test]
fn thread_refusal_keeps_the_draft_and_visible_thread_hint() {
    let mut client = Client::default();
    let batch = client
        .prepare(Some(SwarmId::from_u128(1)), draft())
        .unwrap();
    let commands = client.admitted(batch);
    let UiMsg::Messaging { id, .. } = commands[0] else {
        panic!("send");
    };
    let tip = Ulid::new();
    let mut results = Results::default();
    results.push(
        id,
        Reply::Messaging(Response::Refused {
            error: bbt_proto::messaging::MessageRefusal {
                kind: "unknown_thread".into(),
                message: "parent is not visible".into(),
                hint: format!("reply to {tip}"),
            },
        }),
    );
    let (cleared, follow_up) = client.replies(&results);
    assert!(cleared.is_none());
    assert!(follow_up.is_empty());
    assert!(matches!(&client.status, Status::Refused(reason)
        if reason.contains("unknown_thread") && reason.contains(&tip.to_string())));
}
#[test]
fn reply_window_is_bounded_and_replacements_keep_the_same_correlation() {
    let mut r = Results::default();
    for _ in 0..150 {
        r.push(Ulid::new(), Reply::Refused("x".repeat(20_000)));
    }
    assert!(r.rows().count() <= 64);
    assert!(r.bytes() <= 1024 * 1024);
    assert!(r.omitted() > 0);
    let id = Ulid::new();
    r.push(id, Reply::Refused("before".into()));
    r.push(id, Reply::Refused("after".into()));
    let rows = r.rows().filter(|row| row.op == id).collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert!(matches!(&rows[0].reply,Reply::Refused(reason) if reason=="after"));
}

#[test]
fn a_disconnect_retries_the_captured_ids_and_ignores_the_old_unknown_publication() {
    let mut client = Client::default();
    let batch = client
        .prepare(Some(SwarmId::from_u128(7)), draft())
        .unwrap();
    let commands = client.admitted(batch);
    let UiMsg::Messaging {
        id,
        swarm_id,
        request: Request::Send { message },
    } = &commands[0]
    else {
        panic!("send");
    };
    let mut results = Results::default();
    assert!(results.requested(*id));
    assert!(results.disconnected("logic stopped"));
    client.replies(&results);
    assert!(matches!(client.status, Status::Unknown(_)));
    let retry = client.retry();
    let UiMsg::Messaging {
        id: next,
        swarm_id: scope,
        request: Request::Send { message: again },
    } = &retry[0]
    else {
        panic!("retry");
    };
    assert_eq!(next, id);
    assert_eq!(scope, swarm_id);
    assert_eq!(again, message);
    client.retry_admitted();
    client.replies(&results);
    assert_eq!(client.status, Status::Pending);
    assert!(!results.complete(Ulid::new(), Reply::Refused("unrelated".into())));
    assert!(!results.complete(*id, Reply::Refused("old connection reply".into())));
    assert!(results.requested(*id)); // explicit retry on the replacement connection
    assert!(results.complete(
        *id,
        Reply::Messaging(Response::Held {
            id: *id,
            reason: "signing held".into()
        })
    ));
    assert!(client.replies(&results).0.is_some());
}

#[test]
fn role_recipients_are_captured_from_scoped_registry_membership_and_cap_at_32() {
    use bbt_ui_model::{Role, RoleClass, Seat, SeatId, Stage, Swarm};
    let swarm = SwarmId::from_u128(1);
    let mut model = bboxy450_term::model::Model::default();
    model
        .ui
        .window
        .sidebar
        .swarms
        .push(Swarm::new(swarm, "scope", vec![]));
    let stage = Stage {
        swarm: Some(swarm),
        tab: None,
        focused: None,
    };
    for i in 1..=33 {
        let mut seat = Seat::new(
            SeatId::from_u128(i),
            swarm,
            [0; 32],
            "same display name",
            "codex",
        );
        seat.role = Some(Role {
            name: "executor".into(),
            class: RoleClass::Worker,
        });
        model.ui.seats.insert(seat.id, seat);
    }
    let mut message = draft();
    message.to = vec![Address {
        kind: "role".into(),
        role: "executor".into(),
        label: "not a destination".into(),
        n: Some(1),
        ..Default::default()
    }];
    let client = Client::default();
    assert!(matches!(
        client.prepare_for(&model, &stage, message.clone()),
        Err("role_recipient_limit_32")
    ));
    model
        .ui
        .seats
        .get_mut(&SeatId::from_u128(33))
        .unwrap()
        .swarm = SwarmId::from_u128(2);
    let batch = client.prepare_for(&model, &stage, message).unwrap();
    assert_eq!(batch.commands.len(), 32);
    let mut ids = std::collections::BTreeSet::new();
    for command in batch.commands {
        let UiMsg::Messaging {
            id,
            swarm_id,
            request: Request::Send { message },
        } = command
        else {
            panic!("role send");
        };
        assert_eq!(swarm_id, swarm);
        assert_eq!(id, message.id);
        assert!(message.destination.starts_with("seats/"));
        assert!(ids.insert(id));
    }
}

#[test]
fn ui_role_compose_uses_one_real_composer_group_and_scoped_targets() {
    use bbt_proto::gui::{UiCompose, UiComposeTarget};
    use bbt_ui_model::{Role, RoleClass, Seat, SeatId, Stage, Swarm};
    let swarm = SwarmId::from_u128(1);
    let mut model = bboxy450_term::model::Model::default();
    model
        .ui
        .window
        .sidebar
        .swarms
        .push(Swarm::new(swarm, "scope", vec![]));
    let stage = Stage {
        swarm: Some(swarm),
        tab: None,
        focused: None,
    };
    for i in 1..=3 {
        let mut seat = Seat::new(SeatId::from_u128(i), swarm, [0; 32], "member", "zsh");
        seat.role = Some(Role {
            name: if i == 3 { "creativity" } else { "executor" }.into(),
            class: RoleClass::Worker,
        });
        model.ui.seats.insert(seat.id, seat);
    }
    let request = UiCompose {
        to: UiComposeTarget::Role("executor".into()),
        text: "one role send".into(),
    };
    let submission = bboxy450_term::messaging::ui_compose_draft(&model, &stage, &request).unwrap();
    assert_eq!(
        submission.to[0].n,
        Some(2),
        "the role chip keeps its actual membership count"
    );
    let mut client = Client::default();
    let batch = client.prepare_for(&model, &stage, submission).unwrap();
    let commands = client.admitted(batch);
    assert_eq!(commands.len(), 2);
    let destinations: std::collections::BTreeSet<_> = commands
        .iter()
        .map(|command| match command {
            UiMsg::Messaging {
                request: Request::Send { message },
                ..
            } => message.destination.clone(),
            _ => panic!("composer send"),
        })
        .collect();
    assert_eq!(
        destinations,
        [
            format!("seats/{}/inbox", Ulid::from(1)),
            format!("seats/{}/inbox", Ulid::from(2))
        ]
        .into()
    );
    assert_eq!(client.groups().len(), 1);
    assert_eq!(client.groups()[0].ids.len(), 2);
    assert_eq!(client.groups()[0].label, "executor");
    let absent = UiCompose {
        to: UiComposeTarget::Seat(Ulid::from(4)),
        text: "absent".into(),
    };
    assert!(matches!(
        bboxy450_term::messaging::ui_compose_draft(&model, &stage, &absent),
        Err("composer_seat_not_in_swarm")
    ));
    let channel = UiCompose {
        to: UiComposeTarget::Channel("work".into()),
        text: "channel send".into(),
    };
    assert_eq!(
        bboxy450_term::messaging::ui_compose_draft(&model, &stage, &channel)
            .unwrap()
            .to[0]
            .destination,
        "channels/work"
    );
}

#[test]
fn normal_model_build_preserves_correlated_results_across_grid_publications() {
    let mut model = bboxy450_term::model::Model::default();
    let id = Ulid::new();
    std::sync::Arc::make_mut(&mut model.operation_results).requested(id);
    std::sync::Arc::make_mut(&mut model.operation_results)
        .complete(id, Reply::Refused("named refusal".into()));
    let next = bboxy450_term::model::build(&[], &bboxy450_term::grids::Grids::new(), &model);
    assert!(std::sync::Arc::ptr_eq(
        &model.operation_results,
        &next.operation_results
    ));
    assert_eq!(next.operation_results.rows().next().unwrap().op, id);
}

#[test]
fn task_views_keep_canonical_status_and_require_the_actual_zero_exit_report() {
    use bboxy450_term::messaging::{task_attachment, task_item, task_report, task_row};
    use bbt_proto::messaging::{TaskCard, TaskDoc, TaskVerifySummary};
    use serde_json::json;
    let id = Ulid::new();
    let mut card = TaskCard {
        id,
        path: ".bboxy/tasks/check.md".into(),
        title: "Check".into(),
        status: "claimed".into(),
        assignee: Some("seat:missing".into()),
        delegated_by: Some("operator:actual-device".into()),
        updated: None,
        verify: None,
    };
    let view = task_item(&card, &bboxy450_term::model::Model::default());
    assert_eq!(view.state, "claimed");
    assert_eq!(view.assignee, "seat:missing");
    assert_eq!(view.delegated_by.as_deref(), Some("operator:actual-device"));
    assert_eq!(task_attachment(&card).path, card.path);
    assert!(!task_row(&card).0.may_accept);
    card.status = "reported".into();
    card.verify = Some(TaskVerifySummary { exit_code: 0 });
    assert!(task_row(&card).0.may_accept);
    let mut doc = TaskDoc {
        id,
        path: card.path.clone(),
        metadata: json!({"status":"reported","report":{"summary":"Checks run","commit":"abc123"},"verify_result":{"command":"cargo test","exit_code":1,"evidence":[]}}),
        body: String::new(),
        body_truncated: false,
    };
    let report = task_report(&card, &doc).unwrap().unwrap();
    assert!(!report.can_accept());
    assert_eq!(report.commit.as_deref(), Some("abc123"));
    doc.metadata["verify_result"]["exit_code"] = json!(0);
    assert!(task_report(&card, &doc).unwrap().unwrap().can_accept());
    doc.id = Ulid::new();
    assert!(matches!(
        task_report(&card, &doc),
        Err("task_document_mismatch")
    ));
}

#[test]
fn result_byte_bound_includes_correlation_metadata_and_names_oversize_reads() {
    let mut results = Results::default();
    results.push(Ulid::new(), Reply::Refused("x".repeat(1024 * 1024)));
    let rows: Vec<_> = results.rows().collect();
    assert!(serde_json::to_vec(&rows).unwrap().len() <= results.bytes());
    assert!(results.bytes() <= 1024 * 1024);
    assert!(
        matches!(&rows[0].reply,Reply::Refused(s) if s.starts_with("operation_result_window_limit:"))
    );
}

#[test]
fn repeated_disconnects_release_result_admission_without_accepting_old_socket_replies() {
    let mut results = Results::default();
    for _ in 0..100 {
        let id = Ulid::new();
        assert!(results.requested(id));
        assert!(results.disconnected("owned fixture reconnect"));
        assert!(!results.complete(id, Reply::Refused("old socket".into())));
    }
    assert!(results.rows().count() <= 64);
}

fn op_id(command: &UiMsg) -> Ulid {
    match command {
        UiMsg::Task { id, .. } | UiMsg::Messaging { id, .. } => *id,
        _ => panic!("operator request"),
    }
}
fn task_card(id: Ulid) -> bbt_proto::messaging::TaskCard {
    bbt_proto::messaging::TaskCard {
        id,
        path: ".bboxy/tasks/check.md".into(),
        title: "Check".into(),
        status: "reported".into(),
        assignee: None,
        delegated_by: Some("operator:device".into()),
        updated: None,
        verify: Some(bbt_proto::messaging::TaskVerifySummary { exit_code: 0 }),
    }
}
#[test]
fn read_pages_capture_scope_keep_actual_next_and_ignore_old_scope_replies() {
    use bboxy450_term::messaging::Records;
    use bbt_proto::messaging::{TaskRequest, TaskResponse};
    let mut records = Records::default();
    let swarm = SwarmId::from_u128(52);
    records.select(swarm);
    let commands = records.commands();
    assert_eq!(commands.len(), 4);
    let task = commands
        .iter()
        .find(|c| matches!(c, UiMsg::Task { .. }))
        .unwrap();
    assert!(
        matches!(task,UiMsg::Task {request:TaskRequest::List {swarm_id:Some(id),limit:Some(128),..},..} if id.0==52)
    );
    for c in &commands {
        if let UiMsg::Messaging { swarm_id, .. } = c {
            assert_eq!(*swarm_id, swarm);
        }
    }
    let next = Ulid::new();
    let mut replies = Results::default();
    replies.push(
        op_id(task),
        Reply::Task(TaskResponse::Items {
            items: vec![task_card(Ulid::new())],
            next: Some(next),
        }),
    );
    assert!(records.replies(&replies));
    assert_eq!(records.task_next, Some(next));
    records.more_tasks();
    let more = records.commands();
    assert!(more.iter().any(
        |c| matches!(c,UiMsg::Task {request:TaskRequest::List {after:Some(id),..},..} if *id==next)
    ));
    records.select(SwarmId::from_u128(53));
    assert!(!records.replies(&replies));
    assert!(records.tasks().is_empty());
}
#[test]
fn task_review_does_not_repeat_an_unknown_effect_with_a_fresh_id_or_changed_reason() {
    use bboxy450_term::messaging::Records;
    use bbt_proto::messaging::TaskResponse;
    let mut records = Records::default();
    records.select(SwarmId::from_u128(1));
    let requests = records.commands();
    let op = requests
        .iter()
        .find(|r| matches!(r, UiMsg::Task { .. }))
        .unwrap();
    let task = Ulid::new();
    let mut replies = Results::default();
    replies.push(
        op_id(op),
        Reply::Task(TaskResponse::Items {
            items: vec![task_card(task)],
            next: None,
        }),
    );
    records.replies(&replies);
    assert!(matches!(
        records.review(task, Some("  ".into())),
        Err("rebrief_reason_required")
    ));
    let accept = records.review(task, None).unwrap();
    let id = op_id(&accept);
    replies.push(id, Reply::Unknown("connection closed".into()));
    records.replies(&replies);
    assert!(matches!(
        records.review(task, Some("changed action".into())),
        Err("task_outcome_unknown_repeat_original_action")
    ));
    records.select(SwarmId::from_u128(2));
    assert_eq!(op_id(&records.review(task, None).unwrap()), id);
    assert!(matches!(
        records.review(task, None),
        Err("task_operation_in_flight")
    ));
}

fn projected_message(scope: SwarmId, sequence: u64) -> bbt_proto::messaging::MessageRow {
    use bbt_proto::messaging::*;
    MessageRow {
        id: Ulid::new(),
        bus: Some(BusPosition {
            stream: format!("swarm-{scope}"),
            seq: sequence,
        }),
        author: AuthorRef {
            kind: AuthorKind::Seat,
            id: "seat-author".into(),
            name: "Actual shared author".into(),
        },
        issuer_kind: Some(IssuerKind::LocalDev),
        signature: SignatureState::Signed,
        signature_reason: None,
        upgrade_of: None,
        kind: MessageKind::Decision,
        priority_class: bbt_proto::registry::Class::Worker,
        priority: 8,
        text: "shared projection".into(),
        ts: 1000,
        receipts: ReceiptSummary::default(),
    }
}

#[test]
fn pending_frame_has_no_issuer_and_a_grade_upgrade_keeps_the_message_position() {
    use bboxy450_term::messaging::History;
    use bbt_proto::messaging::{AuthorKind, IssuerKind, SignatureState};
    use bbt_ui::{self as ui, components as c, modules::stream_dock::Entry};
    use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
    struct Metrics;
    impl TextMeasure for Metrics {
        fn width(&self, value: &str, style: &TextStyle) -> f32 {
            value.chars().count() as f32 * style.size * 0.5
        }
    }
    impl c::Measure for Metrics {
        fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
            [style.size * 0.8, style.size * 0.2]
        }
    }
    fn labels(node: &c::Layout) -> String {
        let mut out = match &node.kind {
            c::LayoutKind::Text { runs, .. } => runs.iter().map(|run| run.text.clone()).collect(),
            _ => String::new(),
        };
        for child in &node.children {
            out += &labels(child);
        }
        out
    }
    let scope = SwarmId::from_u128(81);
    let mut pending = projected_message(scope, 1);
    pending.author.kind = AuthorKind::Operator;
    pending.issuer_kind = None;
    pending.signature = SignatureState::Pending;
    pending.signature_reason = Some("fixture custody unavailable".into());
    pending.receipts.delivered = 1;
    pending.text = "first message, retained in place".into();
    let mut later = projected_message(scope, 2);
    later.text = "later message".into();
    let mut history = History::default();
    history.select(scope);
    history.row(&pending, Some("operator/inbox"), 100);
    history.row(&later, Some("operator/inbox"), 100);
    let model = bboxy450_term::model::Model::default();
    let entries = history.entries(&model, &Default::default(), &[]);
    let Entry::Message { data, .. } = &entries[0] else {
        panic!("pending message");
    };
    assert!(data.issuer_kind.is_none());
    assert_eq!(data.signature.as_deref(), Some("pending"));
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let frame = data
        .element(ctx, "pending", true, false, &Metrics)
        .layout(Rect::new(0., 0., 900., 0.), ctx, &Metrics)
        .unwrap();
    let text = labels(&frame);
    assert!(text.contains("signature: pending · inferred · fixture custody unavailable"));
    assert_eq!(
        data.receipts.as_ref().unwrap().delivered,
        1,
        "delivery and signing remain separate evidence"
    );
    assert!(
        !text.contains("1 delivered"),
        "routine receipt counts stay in retained data, not message chrome"
    );
    assert!(
        !text.contains("local-dev"),
        "pending has no placeholder issuer"
    );

    let mut signed = pending.clone();
    signed.issuer_kind = Some(IssuerKind::LocalDev);
    signed.signature = SignatureState::Signed;
    signed.signature_reason = None;
    signed.upgrade_of = Some(bbt_proto::messaging::IntentBinding {
        id: signed.id,
        // Display-only projection fixture; signature verification is tested in mux.
        digest: "0".repeat(64),
    });
    history.row(&signed, None, 101);
    history.row(&pending, None, 100); // An old publication cannot undo a newer grade.
    history.row(&pending, None, 101); // Equal display clocks cannot revoke a completion.
    history.row(&pending, None, 102); // A later stale projection cannot either.
    let entries = history.entries(&model, &Default::default(), &[]);
    assert!(
        !entries
            .iter()
            .any(|entry| matches!(entry,Entry::Note {text,..} if text=="stream cursor pending"))
    );
    let entries: Vec<_> = entries
        .iter()
        .filter(|entry| matches!(entry, Entry::Message { .. }))
        .collect();
    assert_eq!(entries.len(), 2, "the grade upgrade must not add a message");
    let Entry::Message { data, .. } = &entries[0] else {
        panic!("signed message");
    };
    assert_eq!(data.text, signed.text);
    assert_eq!(data.signature.as_deref(), Some("signed"));
    let frame = data
        .element(ctx, "signed", true, false, &Metrics)
        .layout(Rect::new(0., 0., 900., 0.), ctx, &Metrics)
        .unwrap();
    assert!(
        !labels(&frame).contains("signature: signed"),
        "successful signing does not add another routine status line"
    );
    let Entry::Message { data, .. } = &entries[1] else {
        panic!("later message");
    };
    assert_eq!(data.text, later.text);
}
#[test]
fn receiving_does_not_mark_read_and_counts_require_complete_retained_coverage() {
    use bboxy450_term::messaging::{History, HistoryAction};
    use bbt_ui::modules::stream_dock::{Entry, UnreadCount};
    let scope = SwarmId::from_u128(75);
    let mut history = History::default();
    history.select(scope);
    assert!(
        history
            .actions()
            .iter()
            .any(|a| matches!(a, HistoryAction::Page { after: 0, .. }))
    );
    history.cursor(2, 8, 1);
    let row = projected_message(scope, 5);
    history.row(&row, Some("channels/review"), 100);
    assert_eq!(history.seat_unread("seat-author"), UnreadCount::More);
    history.page(1, 8, None);
    assert_eq!(history.seat_unread("seat-author"), UnreadCount::Exact(1));
    assert!(
        history.actions().is_empty(),
        "receive and page completion never prove viewed"
    );
    let model = bboxy450_term::model::Model::default();
    let entries = history.entries(&model, &Default::default(), &[]);
    assert!(entries.iter().any(|entry| matches!(entry, Entry::Message { data, .. } if data.unread && data.issuer_kind.as_deref()==Some("local-dev") && data.ch.as_deref()==Some("review"))));
    assert!(matches!(
        history.viewed_all(),
        Some(HistoryAction::Read { sequence: 8, .. })
    ));
    assert!(
        history
            .entries(&model, &Default::default(), &[])
            .iter()
            .any(|entry| matches!(entry, Entry::Message { data, .. } if data.unread)),
        "only the server cursor clears dots"
    );
    history.cursor(8, 8, 0);
    assert_eq!(history.seat_unread("seat-author"), UnreadCount::Exact(0));
}

#[test]
fn a_refused_retained_page_finishes_the_read_and_waits_for_an_explicit_retry() {
    use bboxy450_term::messaging::{History, HistoryAction};
    use bbt_ui::modules::stream_dock::Entry;
    let mut history = History::default();
    history.select(SwarmId::from_u128(79));
    history.actions();
    history.page_failed(1, "retained page refused by fixture");
    assert!(history.actions().is_empty(), "no automatic refusal loop");
    assert!(history.entries(&Default::default(),&Default::default(),&[]).iter().any(|entry| matches!(entry, Entry::Note {text,..} if text.contains("retained page refused"))));
    history.more();
    assert!(
        history
            .actions()
            .iter()
            .any(|action| matches!(action, HistoryAction::Page { after: 0, .. }))
    );
    history.page(2, 0, None);
    assert!(
        !history
            .entries(&Default::default(), &Default::default(), &[])
            .iter()
            .any(|entry| matches!(entry, Entry::Note {text,..} if text.contains("refused")))
    );
}

#[test]
fn a_refused_inbox_page_retries_its_actual_cursor_only_on_explicit_view() {
    use bboxy450_term::messaging::History;
    let mut history = History::default();
    history.select(SwarmId::from_u128(80));
    let UiMsg::Messaging { id, .. } = history.commands().pop().unwrap() else {
        unreachable!()
    };
    let mut results = Results::default();
    results.push(id, Reply::Refused("read refused".into()));
    assert!(history.replies(&results));
    assert!(history.commands().is_empty());
    history.more();
    assert!(matches!(
        history.commands().as_slice(),
        [UiMsg::Messaging {
            request: Request::List { after: 0, .. },
            ..
        }]
    ));
}
#[test]
fn a_newer_live_cursor_cannot_make_an_old_empty_page_prove_newer_coverage() {
    use bboxy450_term::messaging::{History, HistoryAction};
    use bbt_ui::modules::stream_dock::UnreadCount;
    let scope = SwarmId::from_u128(76);
    let mut history = History::default();
    history.select(scope);
    history.actions();
    history.cursor(0, 12, 1);
    history.row(&projected_message(scope, 5), None, 100);
    history.page(1, 8, None);
    assert_eq!(history.seat_unread("seat-author"), UnreadCount::More);
    history.more();
    assert!(
        history
            .actions()
            .iter()
            .any(|a| matches!(a, HistoryAction::Page { after: 8, .. }))
    );
    history.page(2, 12, None); // empty completion, advancing its own revision
    assert_eq!(history.seat_unread("seat-author"), UnreadCount::Exact(1));
    history.gap(3, 4);
    assert_eq!(history.seat_unread("seat-author"), UnreadCount::More);
}

#[test]
fn role_send_is_one_view_with_receipts_only_when_all_its_real_projections_exist() {
    use bboxy450_term::messaging::{SendGroup, message_entries};
    use bbt_proto::messaging::AuthorKind;
    use bbt_ui::modules::stream_dock::Entry;
    let scope = SwarmId::from_u128(77);
    let mut a = projected_message(scope, 11);
    a.author.kind = AuthorKind::Operator;
    a.receipts.delivered = 1;
    let mut b = projected_message(scope, 12);
    b.author.kind = AuthorKind::Operator;
    b.receipts.acked = 1;
    let groups = [SendGroup {
        swarm: scope,
        label: "executor".into(),
        ids: vec![a.id, b.id],
    }]
    .into();
    let model = bboxy450_term::model::Model::default();
    let missing = message_entries(
        std::slice::from_ref(&a),
        &Default::default(),
        &groups,
        scope,
        &model,
        None,
    );
    assert!(matches!(&missing[0], Entry::Message { data, .. } if data.receipts.is_none()));
    assert!(matches!(&missing[1], Entry::Note { text, .. } if text.contains("1 of 2")));
    let ids = vec![a.id.to_string(), b.id.to_string()];
    let complete = message_entries(&[a, b], &Default::default(), &groups, scope, &model, None);
    assert_eq!(complete.len(), 1);
    assert!(matches!(&complete[0], Entry::Message {data, ..} if data.message_ids == ids));
    assert!(
        matches!(&complete[0], Entry::Message { data, .. } if data.to_label.as_deref()==Some("executor · 2 recipients") && data.receipts.as_ref().is_some_and(|r|r.delivered==1 && r.acked==1))
    );
}

#[test]
fn older_display_projections_cannot_undo_new_receipts_or_release_evidence() {
    use bboxy450_term::messaging::History;
    use bbt_ui::modules::stream_dock::Entry;
    let scope = SwarmId::from_u128(78);
    let mut history = History::default();
    history.select(scope);
    let mut row = projected_message(scope, 5);
    row.author.kind = bbt_proto::messaging::AuthorKind::Operator;
    row.receipts.held = 1;
    history.row(&row, None, 100);
    row.receipts.held = 0;
    row.receipts.delivered = 1;
    history.row(&row, None, 101);
    history.row(&row, None, 200); // identical value must still retain its newer projection time
    row.receipts.held = 1;
    row.receipts.delivered = 0;
    history.row(&row, None, 199);
    assert!(history.entries(&Default::default(),&Default::default(),&[]).iter().any(|entry|matches!(entry,Entry::Message {data,..} if data.receipts.as_ref().is_some_and(|r|r.held==0&&r.delivered==1))));
}

#[test]
fn native_task_reads_and_unknown_mutations_remain_bound_across_colliding_roots() {
    use bboxy450_term::messaging::Records;
    use bbt_proto::{gui::SwarmScope, messaging::TaskResponse};
    let swarm = SwarmId::from_u128(61);
    let first = SwarmScope {
        swarm: Ulid(61),
        revision: 1,
    };
    let second = SwarmScope {
        swarm: Ulid(61),
        revision: 2,
    };
    let mut records = Records::default();
    records.select(swarm);
    records.bind_workspace(Err("workspace_metadata_pending"));
    assert!(
        records
            .commands()
            .iter()
            .all(|command| !matches!(command, UiMsg::Task { .. })),
        "native unknown metadata never emits unscoped tasks"
    );
    records.bind_workspace(Ok(first));
    let requests = records.commands();
    let old = requests
        .iter()
        .find(|command| matches!(command, UiMsg::Task { .. }))
        .unwrap();
    assert!(matches!(old,UiMsg::Task{scope:Some(scope),..}if *scope==first));
    let task = Ulid(1);
    let mut results = Results::default();
    results.push(
        op_id(old),
        Reply::Task(TaskResponse::Items {
            items: vec![task_card(task)],
            next: None,
        }),
    );
    records.replies(&results);
    let original = records.review(task, None).unwrap();
    let original_id = op_id(&original);
    results.push(original_id, Reply::Unknown("connection lost".into()));
    records.replies(&results);
    let replay = records.review(task, None).unwrap();
    assert_eq!(op_id(&replay), original_id);
    assert!(matches!(replay,UiMsg::Task{scope:Some(scope),..}if scope==first));
    results.push(original_id, Reply::Unknown("connection lost again".into()));
    records.replies(&results);
    records.bind_workspace(Ok(second));
    assert!(records.tasks().is_empty());
    assert!(
        matches!(records.review(task, None), Err("task_not_loaded")),
        "colliding task ID cannot replay an effect from the previous root"
    );
    let requests = records.commands();
    let new = requests
        .iter()
        .find(|command| matches!(command, UiMsg::Task { .. }))
        .unwrap();
    assert!(matches!(new,UiMsg::Task{scope:Some(scope),..}if *scope==second));
    let late = op_id(old);
    results.push(
        late,
        Reply::Task(TaskResponse::Items {
            items: vec![task_card(task)],
            next: None,
        }),
    );
    records.replies(&results);
    assert!(
        records.tasks().is_empty(),
        "old-root reply cannot repaint a new binding"
    );
    results.push(
        op_id(new),
        Reply::Task(TaskResponse::Items {
            items: vec![task_card(task)],
            next: None,
        }),
    );
    records.replies(&results);
    let new_review = records.review(task, None).unwrap();
    assert_ne!(op_id(&new_review), original_id);
    assert!(matches!(new_review,UiMsg::Task{scope:Some(scope),..}if scope==second));
}
