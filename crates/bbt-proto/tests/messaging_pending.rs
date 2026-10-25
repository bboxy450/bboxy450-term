use bbt_proto::{
    messaging::{Envelope, LogicalSender, PendingIntent, PendingSignature, SendRequest},
    registry::{Class, Role},
};
use ulid::Ulid;

#[test]
fn first_startup_intent_has_no_author_and_cannot_be_a_signed_envelope() {
    let request = SendRequest::text("operator/inbox", "local before custody");
    let intent = PendingIntent {
        version: 1,
        id: request.id,
        sender: LogicalSender::Seat {
            seat: Ulid::new(),
            process_generation: 1,
        },
        swarm_id: Ulid::new(),
        role: Role {
            name: "Executor".into(),
            class: Class::Worker,
        },
        role_epoch: 1,
        body: bbt_proto::messaging::Body::Text {
            text: request.text.clone(),
        },
        request,
        admission_order: 1,
        admitted_ms: 1,
        signature: PendingSignature::Pending,
        hold_reason: "fixture custody unavailable".into(),
    };
    let value = serde_json::to_value(&intent).unwrap();
    assert_eq!(value["signature"], "pending");
    assert_eq!(
        serde_json::from_value::<PendingIntent>(value.clone()).unwrap(),
        intent
    );
    for field in ["author", "issuer", "account", "device", "role_grant_id"] {
        assert!(value.get(field).is_none(), "invented key claim: {field}");
    }
    assert!(serde_json::from_value::<Envelope>(value.clone()).is_err());
    let mut forged = value.clone();
    forged["author"] = "not an author".into();
    assert!(serde_json::from_value::<PendingIntent>(forged).is_err());
    let mut signed = value;
    signed["signature"] = "signed".into();
    assert!(serde_json::from_value::<PendingIntent>(signed).is_err());
}
