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

#[test]
fn signed_stored_messages_keep_their_legacy_bytes_without_a_new_tag() {
    use bbt_proto::messaging::{Body, IssuerKind, StoredMessage};
    let id = Ulid::new();
    let envelope = Envelope {
        version: 1,
        id,
        issuer_kind: IssuerKind::LocalDev,
        issuer: "serialization fixture".into(),
        account: "serialization fixture".into(),
        device: "serialization fixture".into(),
        seat: Ulid::new(),
        author: "serialization fixture".into(),
        swarm_id: Ulid::new(),
        process_generation: 1,
        context: "operator/inbox".into(),
        role: Role {
            name: "executor".into(),
            class: Class::Worker,
        },
        role_grant_id: "serialization fixture".into(),
        role_epoch: 1,
        priority_class: Class::Worker,
        sender_sequence: 1,
        thread_id: id,
        reply_to: None,
        timestamp: 1,
        priority: 0,
        kind: bbt_proto::messaging::MessageKind::Text,
        body: Body::Text {
            text: "wire".into(),
        },
        attachments: Vec::new(),
        door: None,
        issued_by: None,
        signature: "serialization fixture, not signature verification".into(),
    };
    #[derive(serde::Serialize)]
    struct Legacy<'a> {
        sequence: u64,
        envelope: &'a Envelope,
        held_out: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bus: Option<&'a bbt_proto::messaging::BusPosition>,
    }
    let expected = serde_json::to_vec(&Legacy {
        sequence: 7,
        envelope: &envelope,
        held_out: None,
        bus: None,
    })
    .unwrap();
    let stored = StoredMessage {
        sequence: 7,
        content: envelope.into(),
        held_out: None,
        bus: None,
    };
    assert_eq!(serde_json::to_vec(&stored).unwrap(), expected);
    assert_eq!(
        serde_json::from_slice::<StoredMessage>(&expected).unwrap(),
        stored
    );
    assert_eq!(
        stored.view().signature,
        bbt_proto::messaging::SignatureState::Signed
    );
    assert!(stored.intent().is_none());
}
