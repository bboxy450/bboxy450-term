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
