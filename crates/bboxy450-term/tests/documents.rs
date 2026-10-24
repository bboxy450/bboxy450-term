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
#[test]
fn reopening_while_pending_drains_the_old_reply_without_replacing_the_new_target() {
    let mut host = Host::default();
    host.show("#1".into());
    let (old, _) = host.next_read().unwrap();
    host.show("#2".into());
    assert!(host.next_read().is_none());
    assert!(!host.apply(&Reply {
        id: Ulid::new(),
        response: Response::Resolved(reference(1))
    }));
    assert!(host.apply(&Reply {
        id: old,
        response: Response::Resolved(reference(1))
    }));
    assert_eq!(host.target, "#2");
    assert!(host.prepared.is_none());
    let (next, read) = host.next_read().unwrap();
    assert!(matches!(read,Read::Resolve(s) if s=="#2"));
    host.close();
    host.apply(&Reply {
        id: next,
        response: Response::Failed("not_found: missing".into()),
    });
    assert!(!host.open);
    assert!(host.next_read().is_none());
}
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
#[test]
fn metadata_publications_retain_an_unconsumed_document_reply() {
    use std::{sync::Arc, time::Instant};
    let record = bbt_proto::SeatRecord::new(Ulid::new(), Ulid::new(), "Anchor", "zsh");
    let seat = bbt_ui_model::SeatId::from_u128(record.id.0);
    let generation = record.generation;
    let grids = bboxy450_term::grids::Grids::new();
    let mut model =
        bboxy450_term::model::build(std::slice::from_ref(&record), &grids, &Default::default());
    let landed = Instant::now();
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

#[test]
fn known_backend_loss_is_a_visible_final_outcome_and_late_data_cannot_restart_the_read() {
    let mut host = Host::default();
    host.show("#44".into());
    let (id, _) = host.next_read().unwrap();
    assert!(host.unavailable("document_backend_unavailable: connection lost"));
    assert!(host.settled());
    assert!(host.apply(&Reply {
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
