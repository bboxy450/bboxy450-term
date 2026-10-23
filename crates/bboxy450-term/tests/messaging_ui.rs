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
    assert_eq!(commands.len(), 3);
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
