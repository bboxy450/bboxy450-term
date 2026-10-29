// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-term signed MA-10 implementation 0885ac378f162c6dfddadb9a7117957d68751bbd.
use bbt_proto::messaging::{Envelope, PendingIntent, SendRequest, StoredContent, StoredMessage};
use serde_json::json;
use ulid::Ulid;

// keeps: ICD messaging-seats; absent mentions retain legacy bytes, present IDs survive every view.
#[test]
fn optional_mentions_round_trip_in_signed_and_pending_messages() {
    let mut request = SendRequest::text("channels/review/*", "@Peer review this");
    let old = serde_json::to_value(&request).unwrap();
    assert!(old.get("mentions").is_none());
    let mut explicit_empty = old.clone();
    explicit_empty["mentions"] = json!([]);
    assert_eq!(
        serde_json::from_value::<SendRequest>(explicit_empty).unwrap(),
        request
    );
    request.mentions = vec![Ulid::from(3)];
    let encoded = serde_json::to_value(&request).unwrap();
    assert_eq!(encoded["mentions"], json!(request.mentions));
    assert_eq!(
        serde_json::from_value::<SendRequest>(encoded).unwrap(),
        request
    );

    let value = include_str!("golden/gui_wire.tsv")
        .lines()
        .find_map(|line| {
            let (direction, json) = line.split_once('\t')?;
            if direction != "reply" {
                return None;
            }
            let value: serde_json::Value = serde_json::from_str(json).ok()?;
            value
                .pointer("/body/response/page/items/0/envelope")
                .cloned()
        })
        .unwrap();
    let mut envelope: Envelope = serde_json::from_value(value).unwrap();
    assert!(envelope.mentions.is_empty());
    envelope.mentions = request.mentions.clone();
    let signed = StoredMessage {
        sequence: 1,
        content: envelope.clone().into(),
        held_out: None,
        bus: None,
    };
    assert_eq!(signed.view().mentions, &request.mentions);
    let intent = PendingIntent {
        version: 1,
        id: request.id,
        sender: bbt_proto::messaging::LogicalSender::Seat {
            device: None,
            seat: envelope.seat,
            process_generation: 1,
        },
        swarm_id: envelope.swarm_id,
        role: envelope.role,
        role_epoch: Some(1),
        body: bbt_proto::messaging::Body::Text {
            text: request.text.clone(),
        },
        request,
        admission_order: 1,
        admitted_ms: 1,
        signature: bbt_proto::messaging::PendingSignature::Pending,
        hold_reason: "fixture".into(),
    };
    let pending = StoredMessage {
        sequence: 2,
        content: StoredContent::Pending {
            intent: Box::new(intent),
        },
        held_out: None,
        bus: None,
    };
    assert_eq!(pending.view().mentions, signed.view().mentions);
    for message in [signed, pending] {
        assert_eq!(
            serde_json::from_slice::<StoredMessage>(&serde_json::to_vec(&message).unwrap())
                .unwrap(),
            message
        );
    }
}
