use bbt_proto::messaging::{
    Body, MessageClass, MessageKind, Receipt, ReceiptWake, Request, WakeReason,
};
use serde_json::json;

// keeps: #334 optional cancellation fields preserve legacy bytes and type new events.
#[test]
fn withdrawal_is_a_content_free_event_with_an_optional_signed_supersedes_target() {
    let id = "00000000000000000000000001";
    let value = json!({"op":"send","message":{"id":id,"destination":"seats/target/inbox","thread_id":null,"reply_to":null,"supersedes":id,"priority":0,"kind":"withdrawn","text":"","attachments":[]}});
    let request: Request = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), value);
    assert_eq!(MessageKind::Withdrawn.class(), MessageClass::Event);
    assert_eq!(
        serde_json::to_value(Body::Withdrawn {}).unwrap(),
        json!({"type":"withdrawn"})
    );
    assert!(
        serde_json::from_value::<Body>(json!({"type":"withdrawn","text":"not empty"})).is_err()
    );
}

// keeps: a cancellation reason belongs only to wake:none and unknown reasons refuse.
#[test]
fn receipt_reasons_are_typed_optional_and_cannot_claim_a_live_or_deferred_wake() {
    for (name, reason) in [
        ("superseded", WakeReason::Superseded),
        ("done", WakeReason::Done),
        ("policy", WakeReason::Policy),
    ] {
        let value = json!({"wake":"none","reason":name});
        assert_eq!(
            serde_json::from_value::<ReceiptWake>(value.clone()).unwrap(),
            ReceiptWake::None {
                reason: Some(reason)
            }
        );
        assert_eq!(
            serde_json::to_value(ReceiptWake::None {
                reason: Some(reason)
            })
            .unwrap(),
            value
        );
    }
    assert_eq!(
        serde_json::to_value(ReceiptWake::None { reason: None }).unwrap(),
        json!({"wake":"none"})
    );
    let line = include_str!("golden/gui_wire.tsv")
        .lines()
        .find(|line| line.contains("\"reason\":\"done\""))
        .unwrap();
    let wrapper: serde_json::Value =
        serde_json::from_str(line.split_once('\t').unwrap().1).unwrap();
    let receipt = wrapper["body"]["response"]["page"]["items"][0].clone();
    for wake in ["now", "deferred"] {
        let mut bad = receipt.clone();
        bad["wake"] = json!(wake);
        assert!(serde_json::from_value::<Receipt>(bad).is_err());
    }
    let mut bad = receipt;
    bad["reason"] = json!("guessed");
    assert!(serde_json::from_value::<Receipt>(bad).is_err());
}
