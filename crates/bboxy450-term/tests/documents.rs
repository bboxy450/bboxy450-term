// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxy450_term::documents::{Host, Read, Reader, Reply, Response};
use bbt_mux::{Ulid, knowledge::data_source as d};
use serde_json::json;
fn reference(n: u64) -> d::RecordRef {
    d::RecordRef {
        id: format!("id-{n}"),
        number: Some(n),
        slug: format!("record-{n}"),
        path: format!("notes/record-{n}.md"),
        digest: format!("digest-{n}"),
        external_id: None,
    }
}
fn result(reader: &mut Reader, id: Ulid, value: serde_json::Value) -> std::sync::Arc<Reply> {
    reader.response(id, Ok(value)).unwrap().prepare()
}
// keeps: spec l9-messaging-tasks §4
#[test]
fn resolve_fetch_and_numbered_links_use_the_provider_record_without_a_signature_claim() {
    let mut host = Host::default();
    let mut reader = Reader::default();
    host.show("#44".into());
    let (op, read) = host.next_read().unwrap();
    assert!(matches!(&read,Read::Resolve(s) if s=="#44"));
    let action = reader.admit(op, read).unwrap();
    assert!(matches!(action,bbt_proto::GuiAction::Knowledge {ref method,..} if method=="resolve"));
    assert!(host.apply(&result(&mut reader, op, json!(reference(44)))));
    let (op, read) = host.next_read().unwrap();
    reader.admit(op, read).unwrap();
    let record = d::Record {
        reference: reference(44),
        frontmatter: json!({"type":"decision","number":44,"title":"Real title","signed":"unverified YAML value"}),
        body: "# Real title\n\nRead [[#47]].\n\n```mermaid\nflowchart LR\n A --> B\n```\n".into(),
        etag: "etag".into(),
        resources: vec![],
        parse_error: None,
    };
    assert!(host.apply(&result(&mut reader, op, json!(record))));
    let prepared = host.prepared.as_ref().unwrap();
    assert_eq!(prepared.document.number, Some(44));
    assert_eq!(prepared.document.title, "Real title");
    assert_eq!(prepared.document.prov.signed, None);
    assert!(prepared.markdown.source.contains("A --> B"));
    let (op, read) = host.next_read().unwrap();
    assert!(matches!(&read,Read::Resolve(s) if s=="#47"));
    reader.admit(op, read).unwrap();
    host.apply(&result(&mut reader, op, json!(reference(47))));
    assert_eq!(host.links["#47"], Some(47));
    assert!(host.settled());
}
// keeps: spec l9-messaging-tasks §4
#[test]
fn the_provenance_card_displays_test_account_attribution_without_certifying_metadata() {
    for label in [Some("test-account:01M5QXCFMCR0Z4ZFYZM5JY8QBZ"), None] {
        let mut reader = Reader::default();
        let id = Ulid::new();
        reader.admit(id, Read::Fetch(reference(1)));
        let record = d::Record {
            reference: reference(1),
            frontmatter: json!({"type":"note","created_by":"original author",
                "updated_by":"device:fixture","authorship":{"issued_by":label}}),
            body: "Public attribution from the provider.\n".into(),
            etag: "etag".into(),
            resources: vec![],
            parse_error: None,
        };
        let reply = result(&mut reader, id, json!(record));
        let Response::Fetched(prepared) = &reply.response else {
            panic!("record was not projected: {reply:?}");
        };
        let expected = label.map_or_else(
            || "device:fixture".to_owned(),
            |label| format!("device:fixture · {label}"),
        );
        assert_eq!(
            prepared.document.prov.updated_by.as_deref(),
            Some(expected.as_str())
        );
        assert_eq!(prepared.document.prov.created_by, "original author");
        assert_eq!(prepared.document.prov.signed, None);
    }
}
// keeps: spec l9-messaging-tasks §4
#[test]
fn reopening_while_pending_supersedes_the_old_reply_without_waiting_for_it() {
    let mut host = Host::default();
    host.show("#1".into());
    let (old, _) = host.next_read().unwrap();
    host.show("#2".into());
    let (next, read) = host
        .next_read()
        .expect("held old read cannot block a new document");
    assert!(matches!(read,Read::Resolve(s) if s=="#2"));
    assert!(!host.apply(&Reply {
        id: old,
        response: Response::Resolved(reference(1))
    }));
    assert_eq!(host.target, "#2");
    assert!(host.prepared.is_none());
    host.close();
    assert!(!host.apply(&Reply {
        id: next,
        response: Response::Failed("not_found: missing".into())
    }));
    assert!(!host.open);
    assert!(host.next_read().is_none());
}
// keeps: spec l9-messaging-tasks §4
#[test]
fn errors_and_disconnect_finish_once_and_retry_only_on_a_new_open() {
    let mut host = Host::default();
    let mut reader = Reader::default();
    host.show("#999".into());
    let (op, read) = host.next_read().unwrap();
    reader.admit(op, read);
    let reply = reader
        .response(op, Err(d::CommandError::new("not_found", "no such record")))
        .unwrap()
        .prepare();
    host.apply(&reply);
    assert!(host.error.as_ref().unwrap().starts_with("not_found:"));
    assert!(
        host.view().broken.is_none(),
        "a failed read does not diagnose frontmatter"
    );
    assert!(host.next_read().is_none());
    assert!(!host.apply(&reply));
    host.show("#1".into());
    let (op, read) = host.next_read().unwrap();
    reader.admit(op, read);
    let reply = reader.disconnected("logic stopped").unwrap();
    assert_eq!(reply.id, op);
    host.apply(&reply);
    assert!(
        host.error
            .as_ref()
            .unwrap()
            .starts_with("document_read_disconnected:")
    );
    assert!(reader.disconnected("again").is_none());
}
// keeps: spec l9-messaging-tasks §4
#[test]
fn oversize_and_stale_results_refuse_instead_of_clipping_or_picking_another_file() {
    let mut reader = Reader::default();
    let id = Ulid::new();
    reader.admit(id, Read::Fetch(reference(1)));
    let record = d::Record {
        reference: reference(1),
        frontmatter: json!({}),
        body: "x".repeat(d::MAX_PAGE_BYTES),
        etag: "".into(),
        resources: vec![],
        parse_error: None,
    };
    assert!(
        matches!(&result(&mut reader,id,json!(record)).response,Response::Failed(s) if s.starts_with("response_too_large:"))
    );
    let id = Ulid::new();
    reader.admit(id, Read::Fetch(reference(1)));
    let record = d::Record {
        reference: reference(2),
        frontmatter: json!({}),
        body: "wrong record".into(),
        etag: "".into(),
        resources: vec![],
        parse_error: None,
    };
    assert!(
        matches!(&result(&mut reader,id,json!(record)).response,Response::Failed(s) if s.starts_with("stale_reference:"))
    );
}
// keeps: spec l9-messaging-tasks §4
#[test]
fn metadata_publications_retain_an_unconsumed_document_reply() {
    use std::{sync::Arc, time::Instant};
    let record = bbt_proto::SeatRecord::new(Ulid::new(), Ulid::new(), "Anchor", "zsh");
    let seat = bbt_ui_model::SeatId::from_u128(record.id.0);
    let generation = record.generation;
    let grids = bboxy450_term::grids::Grids::new();
    let mut model =
        bboxy450_term::model::build(std::slice::from_ref(&record), &grids, &Default::default());
    let landed = bboxy450_term::model::FocusSwell {
        at: Instant::now(),
        where_: bbt_ui_model::event::LocateWhere::Pane,
    };
    assert!(model.attention_focus(seat, generation, None, Some(landed)));
    let mut model = Arc::new(model);
    for _ in 0..3 {
        let published = model.clone();
        let reply = Arc::new(Reply {
            id: Ulid::new(),
            response: Response::Failed("not_found: retained".into()),
        });
        Arc::make_mut(&mut model).document_reply = Some(reply.clone());
        let next = bboxy450_term::model::build(std::slice::from_ref(&record), &grids, &model);
        assert!(Arc::ptr_eq(next.document_reply.as_ref().unwrap(), &reply));
        assert_eq!(next.focus_swell, Some((seat, landed)));
        assert_eq!(published.focus_swell, next.focus_swell);
        model = Arc::new(next);
    }
}

// keeps: spec l9-messaging-tasks §4
#[test]
fn retained_attachment_targets_are_bounded_as_whole_references() {
    let references: Vec<_> = (1..=34).map(|n| format!("[[#{n}]]")).collect();
    let bounded = bboxy450_term::messaging::MessageAttachments::bounded(&references);
    assert_eq!(bounded.references, references[..32]);
    assert_eq!(bounded.omitted, 2);
    let long = format!("[[{}]]", "x".repeat(512));
    let bounded = bboxy450_term::messaging::MessageAttachments::bounded(&["[[#44]]".into(), long]);
    assert_eq!(bounded.references, ["[[#44]]"]);
    assert_eq!(bounded.omitted, 1);
}

// keeps: spec l9-messaging-tasks §4
#[test]
fn attachment_overflow_is_named_on_the_message_without_changing_its_projection() {
    use bboxy450_term::messaging::{self, MessageAttachments};
    use bbt_proto::messaging as wire;
    use std::collections::{BTreeMap, VecDeque};
    let id = Ulid::new();
    let row:wire::MessageRow=serde_json::from_value(json!({"id":id,"bus":null,"author":{"kind":"operator","id":"operator:device","name":"Operator"},"issuer_kind":"local-dev","kind":"decision","priority_class":"oracle","priority":7,"text":"Original text","ts":1,"receipts":{"delivered":0,"acked":0,"held":0}})).unwrap();
    let references: Vec<_> = (1..=34).map(|n| format!("[[#{n}]]")).collect();
    let entries = messaging::message_entries_with_attachments(
        &[row],
        &BTreeMap::new(),
        &VecDeque::new(),
        bbt_proto::SwarmId::from_u128(1),
        &Default::default(),
        None,
        &BTreeMap::from([(id, MessageAttachments::bounded(&references))]),
    );
    assert!(entries.iter().any(|entry|matches!(entry,bbt_ui::modules::stream_dock::Entry::Message {data,..} if data.attach.len()==32 && data.text=="Original text" && data.priority==7 && data.kind.as_deref()==Some("decision"))));
    assert!(entries.iter().any(|entry|matches!(entry,bbt_ui::modules::stream_dock::Entry::Note {text,..} if text.contains("attachment_view_limit: 2"))));
}

fn attachment_draft(path: &str) -> bbt_ui::modules::composer::Submission {
    use bbt_ui::modules::composer::{Address, Attachment, Submission};
    Submission {
        delegate: false,
        to: vec![Address {
            kind: "channel".into(),
            destination: "swarms/one".into(),
            ..Default::default()
        }],
        attach: vec![Attachment {
            kind: "task".into(),
            path: path.into(),
            ..Default::default()
        }],
        text: "Review this record".into(),
        kind: "text".into(),
        priority: 0,
    }
}
// keeps: spec l9-messaging-tasks §4
#[test]
fn composer_wraps_task_paths_once_and_preserves_existing_wire_references() {
    use bboxy450_term::{cockpit::UiMsg, messaging::Client};
    use bbt_proto::messaging::Request;
    let client = Client::default();
    let swarm = Some(bbt_proto::SwarmId::from_u128(1));
    for (input, expected) in [
        (".bboxy/tasks/one.md", "[[.bboxy/tasks/one.md]]"),
        ("#44", "[[#44]]"),
        ("[[#44]]", "[[#44]]"),
        ("[[task-slug|#44]]", "[[task-slug|#44]]"),
    ] {
        let batch = client.prepare(swarm, attachment_draft(input)).unwrap();
        let UiMsg::Messaging {
            request: Request::Send { message: args },
            ..
        } = &batch.commands[0]
        else {
            panic!("one message");
        };
        assert_eq!(args.attachments, [expected]);
    }
}
// keeps: spec l9-messaging-tasks §4
#[test]
fn composer_refuses_unknown_attachment_shapes_before_any_send_is_admitted() {
    let client = bboxy450_term::messaging::Client::default();
    for input in [
        "https://example.test/record.md",
        "/etc/passwd",
        "../record.md",
        "[[#44]]tail",
        "[[[[#44]]]]",
        "#0",
        "#044",
        "[[#44|]]",
        "not\na\nreference",
    ] {
        let error = client
            .prepare(
                Some(bbt_proto::SwarmId::from_u128(1)),
                attachment_draft(input),
            )
            .err()
            .expect("named pre-send refusal");
        assert!(
            error.starts_with("attachment_reference_"),
            "{input}: {error}"
        );
    }
}
// keeps: spec l9-messaging-tasks §4
#[test]
fn task_delegate_keeps_its_provider_path() {
    use bboxy450_term::messaging::{self, Client};
    let card = bbt_proto::messaging::TaskCard {
        id: Ulid::new(),
        path: ".bboxy/tasks/one.md".into(),
        title: "A task".into(),
        status: "open".into(),
        assignee: None,
        delegated_by: None,
        updated: None,
        verify: None,
    };
    let attachment = messaging::task_attachment(&card);
    assert_eq!(attachment.path, card.path);
    let mut draft = attachment_draft(&attachment.path);
    draft.attach = vec![attachment];
    draft.delegate = true;
    draft.to = vec![bbt_ui::modules::composer::Address {
        kind: "seat".into(),
        id: Ulid::new().to_string(),
        destination: "swarms/one/seats/two/inbox".into(),
        ..Default::default()
    }];
    let batch = Client::default()
        .prepare(Some(bbt_proto::SwarmId::from_u128(1)), draft)
        .unwrap();
    assert!(
        matches!(&batch.commands[0],bboxy450_term::cockpit::UiMsg::Task {request:bbt_proto::messaging::TaskRequest::Delegate {task_path,..},..} if task_path==&card.path)
    );
}

// keeps: spec l9-messaging-tasks §4
#[test]
fn known_backend_loss_is_a_visible_final_outcome_and_late_data_cannot_restart_the_read() {
    let mut host = Host::default();
    host.show("#44".into());
    let (id, _) = host.next_read().unwrap();
    assert!(host.unavailable("document_backend_unavailable: connection lost"));
    assert!(host.settled());
    assert!(!host.apply(&Reply {
        id,
        response: Response::Resolved(reference(44))
    }));
    assert!(host.next_read().is_none());
    assert!(host.prepared.is_none());
    assert!(
        host.error
            .as_deref()
            .unwrap()
            .starts_with("document_backend_unavailable:")
    );
    host.show("#44".into());
    assert!(host.error.is_none());
    assert!(host.next_read().is_some());
}

fn scoped_value(
    mut value: serde_json::Value,
    scope: bbt_proto::gui::SwarmScope,
    root: &str,
    provider_revision: u64,
) -> serde_json::Value {
    value["knowledge_root_id"] = json!(root);
    value["knowledge_root_revision"] = json!(provider_revision);
    value["swarm_workspace_id"] = json!(scope.swarm.to_string());
    value["swarm_workspace_revision"] = json!(scope.revision);
    value
}
#[test]
fn scoped_document_chain_retains_flat_root_evidence_and_rejects_cross_root_collisions() {
    let scope = bbt_proto::gui::SwarmScope {
        swarm: Ulid(44),
        revision: 2,
    };
    let mut host = Host::default();
    let mut reader = Reader::default();
    host.show_scoped("#44".into(), Some(scope));
    let (op, read) = host.next_read().unwrap();
    let action = reader.admit_scoped(op, read, host.scope()).unwrap();
    assert!(
        matches!(action,bbt_proto::GuiAction::Knowledge{scope:Some(actual),..} if actual==scope)
    );
    let value = scoped_value(json!(reference(44)), scope, "root-a", 7);
    // Portable provider DTOs intentionally tolerate the agreed flat GUI evidence.
    assert_eq!(
        serde_json::from_value::<d::RecordRef>(value.clone()).unwrap(),
        reference(44)
    );
    host.apply(&result(&mut reader, op, value));
    let (op, read) = host.next_read().unwrap();
    reader.admit_scoped(op, read, host.scope()).unwrap();
    let record = d::Record {
        reference: reference(44),
        frontmatter: json!({"title":"wrong root same ID"}),
        body: "foreign contents".into(),
        etag: "same".into(),
        resources: vec![],
        parse_error: None,
    };
    host.apply(&result(
        &mut reader,
        op,
        scoped_value(json!(record), scope, "root-b", 7),
    ));
    assert!(host.prepared.is_none());
    assert!(
        host.error
            .as_deref()
            .unwrap()
            .starts_with("document_source_changed:")
    );
    assert!(host.next_read().is_none());
}
#[test]
fn pending_workspace_retry_is_scheduled_bounded_scoped_and_cancel_invalidates_its_wake() {
    let scope = bbt_proto::gui::SwarmScope {
        swarm: Ulid(45),
        revision: 3,
    };
    let mut host = Host::default();
    let mut reader = Reader::default();
    host.show_scoped("#1".into(), Some(scope));
    let (op, read) = host.next_read().unwrap();
    reader.admit_scoped(op, read, host.scope()).unwrap();
    let mut pending = d::CommandError::new("workspace_pending", "mounting");
    pending.retry_after_ms = Some(100);
    host.apply(&reader.response(op, Err(pending)).unwrap().prepare());
    assert!(host.error.is_none());
    assert!(host.next_read().is_none());
    let (epoch, delay) = host.take_retry().unwrap();
    assert_eq!(delay, std::time::Duration::from_millis(100));
    assert!(host.take_retry().is_none());
    assert!(host.retry_ready(epoch));
    let (op, read) = host.next_read().unwrap();
    assert!(
        matches!(reader.admit_scoped(op,read,host.scope()).unwrap(),bbt_proto::GuiAction::Knowledge{scope:Some(actual),..}if actual==scope)
    );
    host.close();
    assert!(!host.retry_ready(epoch));
    host.apply(&result(
        &mut reader,
        op,
        scoped_value(json!(reference(1)), scope, "root", 1),
    ));
    assert!(!host.open);
    assert!(host.prepared.is_none());
}

#[test]
fn following_a_link_keeps_captured_workspace_and_provider_generation() {
    let scope = bbt_proto::gui::SwarmScope {
        swarm: Ulid(46),
        revision: 4,
    };
    let mut host = Host::default();
    let mut reader = Reader::default();
    host.show_scoped("#1".into(), Some(scope));
    let (op, read) = host.next_read().unwrap();
    reader.admit_scoped(op, read, host.scope()).unwrap();
    host.apply(&result(
        &mut reader,
        op,
        scoped_value(json!(reference(1)), scope, "root-a", 7),
    ));
    host.follow("#2".into());
    assert_eq!(
        host.scope(),
        Some(scope),
        "follow never recaptures the selected swarm"
    );
    let (op, read) = host.next_read().unwrap();
    assert!(matches!(&read, Read::Resolve(target) if target=="#2"));
    reader.admit_scoped(op, read, host.scope()).unwrap();
    host.apply(&result(
        &mut reader,
        op,
        scoped_value(json!(reference(2)), scope, "root-a", 8),
    ));
    assert!(host.prepared.is_none());
    assert!(
        host.error
            .as_deref()
            .unwrap()
            .starts_with("document_source_changed:"),
        "follow cannot reinterpret a retained link with a replaced provider"
    );
    assert!(host.next_read().is_none());
}
#[test]
fn changed_workspace_invalidates_the_held_reply_without_admitting_a_fallback_read() {
    let scope = bbt_proto::gui::SwarmScope {
        swarm: Ulid(47),
        revision: 5,
    };
    let mut host = Host::default();
    let mut reader = Reader::default();
    host.show_scoped("#1".into(), Some(scope));
    let (op, read) = host.next_read().unwrap();
    reader.admit_scoped(op, read, host.scope()).unwrap();
    host.unavailable("swarm_root_changed: working folder changed");
    assert!(!host.apply(&result(
        &mut reader,
        op,
        scoped_value(json!(reference(1)), scope, "old-root", 1)
    )));
    assert!(host.next_read().is_none());
    assert!(host.prepared.is_none());
    assert_eq!(host.scope(), Some(scope));
    assert!(
        host.error
            .as_deref()
            .unwrap()
            .starts_with("swarm_root_changed:")
    );
}

#[test]
fn latest_document_reader_supersedes_held_roots_and_bounds_retired_replies() {
    let scope = bbt_proto::gui::SwarmScope {
        swarm: Ulid(80),
        revision: 1,
    };
    let mut reader = Reader::default();
    let mut old = Vec::new();
    for _ in 0..8 {
        let id = Ulid::new();
        reader
            .admit_latest_scoped(id, Read::Resolve("#1".into()), Some(scope))
            .unwrap();
        old.push(id);
    }
    assert!(
        reader
            .admit_latest_scoped(Ulid::new(), Read::Resolve("#2".into()), Some(scope))
            .unwrap_err()
            .starts_with("document_read_limit:")
    );
    assert!(
        reader.response(old[0], Ok(json!(reference(1)))).is_none(),
        "retired read releases accounting without preparation"
    );
    let fresh = Ulid::new();
    reader
        .admit_latest_scoped(fresh, Read::Resolve("#2".into()), Some(scope))
        .unwrap();
    for id in old.into_iter().skip(1) {
        assert!(reader.response(id, Ok(json!(reference(1)))).is_none());
    }
    let reply = reader
        .response(
            fresh,
            Ok(scoped_value(json!(reference(2)), scope, "ready-root", 1)),
        )
        .unwrap()
        .prepare();
    assert_eq!(reply.id, fresh);
    assert!(matches!(reply.response, Response::Bound { .. }));
}
