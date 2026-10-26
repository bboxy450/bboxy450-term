//! Compatibility checks for the existing W2.4 JSON carried by the GUI wrappers.
use bbt_proto::messaging::{Envelope, Request, Response};
use bbt_proto::{GuiAction, GuiReply};
use serde_json::{Value, json};

const ID: &str = "00000000000000000000000001";

fn envelope() -> Value {
    json!({
        "version":1,"id":ID,"issuer_kind":"local-dev","issuer":"issuer",
        "account":"account","device":"device","seat":ID,"author":"author",
        "swarm_id":ID,"process_generation":"9007199254740993","context":"seats/target/inbox",
        "role":{"name":"executor","class":"worker"},"role_grant_id":"grant",
        "role_epoch":"1","priority_class":"worker","sender_sequence":"18446744073709551615",
        "thread_id":ID,"reply_to":null,"timestamp":1,"priority":0,"kind":"text",
        "body":{"type":"text","text":"hello"},"attachments":["[[task]]"],"signature":"signature"
    })
}

// keeps: spec l9-messaging-tasks §2
#[test]
fn existing_messaging_requests_keep_their_tags_and_nested_fields() {
    let node = json!({"path":"channels/render/link","kind":"link","target":"seats/target/inbox",
        "filter":{"kinds":["decision"],"classes":["oracle"],"priorities":[0]},
        "rate":{"batch_ms":50,"coalesce":true},"paused":false,"grants":{}});
    let requests = [
        json!({"op":"send","message":{"id":ID,"destination":"seats/target/inbox",
            "thread_id":null,"reply_to":null,"priority":0,"kind":"chunk","text":"hello","attachments":["[[task]]"]}}),
        json!({"op":"list","thread":ID,"channel":null,"after":7,"limit":128}),
        json!({"op":"receipts","message":ID,"after":2,"limit":12}),
        json!({"op":"acknowledge","message":ID,"agent":"main"}),
        json!({"op":"ls","path":"channels","after":"channels/render","limit":12}),
        json!({"op":"subscribe","folder":"channels/render","name":"mine",
            "filter":{"kinds":[],"classes":[],"priorities":[]},"rate":{"batch_ms":0,"coalesce":false}}),
        json!({"op":"unsubscribe","path":"channels/render/mine"}),
        json!({"op":"link","node":node}),
        json!({"op":"pause","path":"channels/render/mine","paused":true}),
        json!({"op":"grant","path":"channels/render","seat":ID,"rights":["read","write","link","admin"]}),
        json!({"op":"mute","seat":ID,"muted":true}),
    ];
    for value in requests {
        let decoded: Request = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
    }
    assert!(
        serde_json::from_value::<Request>(
            json!({"op":"mute","seat":ID,"muted":true,"operator":true})
        )
        .is_err()
    );
}

// keeps: spec l9-messaging-tasks §2
#[test]
fn carried_messages_preserve_canonical_decimal_counters() {
    let value = envelope();
    let decoded: Envelope = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(decoded.process_generation, 9_007_199_254_740_993);
    assert_eq!(decoded.sender_sequence, u64::MAX);
    assert_eq!(serde_json::to_value(decoded).unwrap(), value);
    for bad in [
        json!(1),
        json!("01"),
        json!("+1"),
        json!("18446744073709551616"),
    ] {
        let mut value = envelope();
        value["sender_sequence"] = bad;
        assert!(serde_json::from_value::<Envelope>(value).is_err());
    }
    let mut value = envelope();
    value["injected_authority"] = json!(true);
    assert!(serde_json::from_value::<Envelope>(value).is_err());
}

// keeps: spec l9-messaging-tasks §2
#[test]
fn existing_stored_message_and_receipt_pages_keep_their_wire_shape() {
    let message = json!({"sequence":3,"envelope":envelope(),"held_out":null});
    let receipt = json!({"message_sequence":3,"message":ID,"sender":ID,"recipient":ID,
        "agent":"review","evidence":"native-unknown","note":"unknown outcome","held":null,
        "arrivals":[{"path":"channels/render","links":[],"paused":false,"batch_ms":0,"coalesce":false}],
        "received_ms":1,"eligible_ms":2,"batch_key":"channels/render","class":"worker",
        "attempt":ID,"native_id":null,"relayed_via_main":true});
    let responses = [
        json!({"kind":"held","id":ID,"reason":"custody unavailable"}),
        json!({"kind":"stored","message":message.clone()}),
        json!({"kind":"messages","page":{"items":[message],"next":3}}),
        json!({"kind":"receipts","page":{"items":[receipt],"next":null}}),
        json!({"kind":"tree","nodes":[],"next":null}),
        json!({"kind":"muted","notice":ID}),
        json!({"kind":"applied"}),
    ];
    for value in responses {
        let decoded: Response = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
    }
}

// keeps: spec l9-messaging-tasks §2
#[test]
fn gui_messaging_wraps_the_existing_payload_and_correlates_its_reply() {
    let value = json!({"kind":"Messaging","body":{"id":ID,"swarm_id":ID,
        "request":{"op":"list","thread":null,"channel":null,"after":3,"limit":16}}});
    let action: GuiAction = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(action).unwrap(), value);

    let value = json!({"kind":"Messaging","body":{"op":ID,
        "response":{"kind":"messages","page":{"items":[],"next":3}}}});
    let reply: GuiReply = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(reply).unwrap(), value);
}

// keeps: spec l9-messaging-tasks §2
#[test]
fn operator_recipients_are_named_while_legacy_seat_receipts_remain_ulids() {
    use bbt_proto::messaging::Recipient;
    for value in [ID.to_owned(), format!("operator:{ID}")] {
        let recipient: Recipient = serde_json::from_value(json!(value)).unwrap();
        assert_eq!(serde_json::to_value(recipient).unwrap(), value);
    }
    for value in [
        "00000000000000000000000000",
        "operator:00000000000000000000000000",
        "operator:missing",
        "observer:unknown",
    ] {
        assert!(serde_json::from_value::<Recipient>(json!(value)).is_err());
    }
}

// keeps: spec l9-messaging-tasks §2
#[test]
fn task_wire_matches_as_built_tools_and_never_accepts_a_spawn_or_authority_override() {
    use bbt_proto::messaging::TaskRequest;
    let value = json!({"kind":"Task","body":{"id":ID,"request":{
        "op":"report","task_path":".bboxy/tasks/fixture.md","summary":"verified fixture",
        "commit":null,"verify_result":{"command":"true","exit_code":0,"evidence":[]}
    }}});
    let decoded: GuiAction = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), value);
    for request in [
        json!({"op":"report","task_path":".bboxy/tasks/fixture.md","verify_result":null}),
        json!({"op":"delegate","task_path":".bboxy/tasks/fixture.md","seat_id":ID,"spawn_spec":{}}),
        json!({"op":"accept","task_path":".bboxy/tasks/fixture.md","device_id":ID}),
    ] {
        assert!(serde_json::from_value::<TaskRequest>(request).is_err());
    }
}

// keeps: spec l9-messaging-tasks §2
#[test]
fn unassigned_inventory_is_optional_and_keeps_existing_task_list_bytes() {
    use bbt_proto::messaging::TaskRequest;
    let old =
        json!({"op":"list","swarm_id":ID,"status":null,"assignee":null,"after":null,"limit":128});
    let request: TaskRequest = serde_json::from_value(old.clone()).unwrap();
    assert!(matches!(
        request,
        TaskRequest::List {
            unassigned: false,
            ..
        }
    ));
    assert_eq!(serde_json::to_value(request).unwrap(), old);
    let inventory = json!({"op":"list","unassigned":true,"swarm_id":null,"status":"open","assignee":null,"after":null,"limit":128});
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<TaskRequest>(inventory.clone()).unwrap())
            .unwrap(),
        inventory
    );
}

fn attention_receipt() -> Value {
    json!({
        "message_sequence":3,"message":ID,"sender":ID,"recipient":ID,
        "agent":null,"evidence":"recipient-stored","note":"stored","held":null,
        "arrivals":[],"received_ms":1,"eligible_ms":2,"batch_key":"seats/target/inbox",
        "class":"worker","attempt":null,"native_id":null,"relayed_via_main":false
    })
}

// keeps: spec messaging-attention §2 — class is signed through kind, never chosen separately
#[test]
fn message_class_is_derived_from_kind_and_cannot_be_supplied_by_the_sender() {
    use bbt_proto::messaging::{MessageClass, MessageKind, WakeMode};
    for kind in ["text", "chunk", "decision", "task"] {
        let kind: MessageKind = serde_json::from_value(json!(kind)).unwrap();
        assert_eq!(kind.class(), MessageClass::Conversation);
    }
    for kind in [
        "presence",
        "slot_claimed",
        "slot_released",
        "cut_claimed",
        "landed",
        "gate_result",
        "task_transition",
    ] {
        let kind: MessageKind = serde_json::from_value(json!(kind)).unwrap();
        assert_eq!(kind.class(), MessageClass::Event);
    }
    let mut forged = envelope();
    forged["class"] = json!("conversation");
    assert!(serde_json::from_value::<Envelope>(forged).is_err());
    for mode in ["wake", "inbox", "digest"] {
        let decoded: WakeMode = serde_json::from_value(json!(mode)).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), mode);
    }
    assert!(serde_json::from_value::<WakeMode>(json!("mute")).is_err());
}

// keeps: spec messaging-attention §2 — every event has the pinned structured fields
#[test]
fn structured_event_bodies_keep_the_pinned_fields_and_have_no_model_text() {
    use bbt_proto::messaging::Body;
    let cases = [
        json!({"type":"slot_claimed","slot":2,"sha":"abc"}),
        json!({"type":"slot_released","slot":2,"sha":"abc","exit":-15}),
        json!({"type":"cut_claimed","branch":"topic","paths":["crates/bbt-proto/src/messaging.rs"]}),
        json!({"type":"landed","sha":"abc","tier":"narrow","presubmit_seconds":12}),
        json!({"type":"gate_result","sha":"abc","stage":"presubmit","verdict":"green"}),
        json!({"type":"task_transition","task":337,"from":"open","to":"delegated"}),
    ];
    for value in cases {
        let decoded: Body = serde_json::from_value(value.clone()).unwrap();
        assert!(decoded.text().is_none());
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
        let mut extra = value;
        extra["text"] = json!("untyped prose");
        assert!(serde_json::from_value::<Body>(extra).is_err());
    }
    for tier in ["lab", "narrow", "wide"] {
        let value = json!({"type":"landed","sha":"abc","tier":tier,"presubmit_seconds":12});
        assert_eq!(
            serde_json::to_value(serde_json::from_value::<Body>(value.clone()).unwrap()).unwrap(),
            value
        );
    }
    for verdict in ["green", "red", "flake", "manual-green", "manual-revert"] {
        let value = json!({"type":"gate_result","sha":"abc","stage":"postsubmit","verdict":verdict,"predicate":"rust-tests"});
        assert_eq!(
            serde_json::to_value(serde_json::from_value::<Body>(value.clone()).unwrap()).unwrap(),
            value
        );
    }
    for bad in [
        json!({"type":"slot_claimed","slot":256,"sha":"abc"}),
        json!({"type":"slot_released","slot":1,"sha":"abc","exit":"0"}),
        json!({"type":"cut_claimed","branch":"topic","paths":"one path"}),
        json!({"type":"landed","sha":"abc","tier":"trusted","presubmit_seconds":12}),
        json!({"type":"gate_result","sha":"abc","stage":"other","verdict":"green"}),
        json!({"type":"gate_result","sha":"abc","stage":"presubmit","verdict":"manual_green"}),
        json!({"type":"task_transition","task":4294967296u64,"from":"open","to":"delegated"}),
    ] {
        assert!(serde_json::from_value::<Body>(bad).is_err());
    }
}

// keeps: spec messaging-attention §2 — optional legacy data and exact flat wake states
#[test]
fn receipt_wake_keeps_legacy_bytes_and_uses_the_pinned_flat_states() {
    use bbt_proto::messaging::{Receipt, ReceiptWake};
    let original = attention_receipt();
    let legacy: Receipt = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(legacy.wake, None);
    assert_eq!(serde_json::to_value(legacy).unwrap(), original);
    let until = "2026-10-26T00:10:00Z";
    for (fields, state) in [
        (json!({"wake":"now"}), ReceiptWake::Now),
        (json!({"wake":"none"}), ReceiptWake::None),
        (
            json!({"wake":"deferred","until":until}),
            ReceiptWake::Deferred {
                until: until.into(),
            },
        ),
    ] {
        let mut value = original.clone();
        value
            .as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        let decoded: Receipt = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(decoded.wake, Some(state));
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
    }
}

// keeps: spec messaging-attention §2 — partial or unknown wake states cannot masquerade as absent
#[test]
fn malformed_receipt_wake_states_are_refused() {
    use bbt_proto::messaging::Receipt;
    for fields in [
        json!({"wake":"digest"}),
        json!({"wake":"wake"}),
        json!({"wake":true}),
        json!({"wake":"deferred"}),
        json!({"wake":"deferred","until":""}),
        json!({"wake":"deferred","until":12}),
        json!({"wake":"now","until":"2026-10-26T00:10:00Z"}),
        json!({"until":"2026-10-26T00:10:00Z"}),
    ] {
        let mut value = attention_receipt();
        value
            .as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        assert!(serde_json::from_value::<Receipt>(value).is_err());
    }
}
