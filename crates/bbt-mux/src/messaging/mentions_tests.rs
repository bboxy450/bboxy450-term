// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-term signed MA-10 implementation 0885ac378f162c6dfddadb9a7117957d68751bbd.
use super::*;
use bbt_proto::messaging::{MessageClass, ReceiptWake, WakeReason};
use bbt_proto::registry::AttentionRule;
use ed25519_dalek::SigningKey;

// keeps: MA-10 recipient-specific signed IDs, pinned admission, receiver override and no prose parsing.
#[test]
fn channel_mentions_wake_only_named_members_and_remain_pinned_after_reopen() {
    let root = tempfile::tempdir().unwrap();
    let registry =
        Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
    let swarm = Ulid::new();
    let mut sender = SeatRecord::new(Ulid::new(), swarm, "Sender", "zsh");
    sender.role.class = Class::Oracle;
    let a = SeatRecord::new(Ulid::new(), swarm, "A", "zsh");
    let b = SeatRecord::new(Ulid::new(), swarm, "B", "zsh");
    for record in [&sender, &a, &b] {
        registry.insert(record).unwrap();
    }
    let keys = Arc::new(super::super::keys::LocalSeatKeys::new(
        SigningKey::from_bytes(&[89; 32]),
    ));
    let mut messaging = Messaging::open(root.path(), keys.clone()).unwrap();
    let channel = "channels/mention-proof";
    let tree = &mut messaging.swarm(swarm).unwrap().tree;
    tree.put(Actor::Operator, Node::folder(channel)).unwrap();
    for record in [&a, &b] {
        tree.put(
            Actor::Operator,
            Node::link(
                format!("{channel}/{}", record.id),
                format!("seats/{}/inbox", record.id),
            ),
        )
        .unwrap();
    }
    let mut request = SendRequest::text(
        format!("{channel}/*"),
        "@A is just prose on the receiver; signed IDs name B",
    );
    request.mentions = vec![b.id];
    let message = messaging
        .send(sender.id, 1, request.clone(), 1_000)
        .unwrap();
    assert_eq!(message.view().mentions, [b.id]);
    let wakes: BTreeMap<_, _> = messaging
        .receipts(sender.id, request.id)
        .unwrap()
        .into_iter()
        .map(|r| (r.recipient, r.wake))
        .collect();
    assert_eq!(wakes.len(), 2);
    assert!(matches!(
        wakes[&Recipient::Seat(a.id)],
        Some(ReceiptWake::Deferred { .. })
    ));
    assert_eq!(wakes[&Recipient::Seat(b.id)], Some(ReceiptWake::Now));
    let mut event = message.envelope().unwrap().clone();
    event.id = Ulid::new();
    event.kind = MessageKind::Presence;
    event.body = Body::Presence {
        event: "mute".into(),
        name: sender.name.clone(),
        harness: sender.harness_kind.clone(),
        state: bbt_proto::messaging::State::Idle,
        muted: true,
        policy_epoch: 1,
    };
    event.sign(&keys.leaf(sender.id, 1).unwrap()).unwrap();
    keys.verify(&event).unwrap();
    let routes: Vec<_> = [&a, &b]
        .into_iter()
        .map(|seat| Route {
            recipient: Recipient::Seat(seat.id),
            agent: None,
            arrivals: Vec::new(),
        })
        .collect();
    let modes = messaging
        .attention_modes(Attention::signed(&event), &routes)
        .unwrap();
    assert!(
        modes.values().all(|mode| *mode == WakeMode::Inbox),
        "signed mentions cannot turn a default event into a wake"
    );
    assert!(messaging.prepare(swarm, a.id, 1_001).unwrap().is_none());
    assert_eq!(
        messaging
            .prepare(swarm, b.id, 1_001)
            .unwrap()
            .unwrap()
            .messages[0]
            .view()
            .id,
        request.id
    );
    registry
        .set_attention(
            b.id,
            1,
            AttentionRule {
                destination: channel.into(),
                class: MessageClass::Conversation,
                mode: WakeMode::Inbox,
            },
        )
        .unwrap();
    registry
        .set_attention(
            a.id,
            1,
            AttentionRule {
                destination: channel.into(),
                class: MessageClass::Conversation,
                mode: WakeMode::Wake,
            },
        )
        .unwrap();
    drop(messaging);
    let mut messaging = Messaging::open(root.path(), keys).unwrap();
    let retry = messaging
        .send(sender.id, 1, request.clone(), 2_000)
        .unwrap();
    assert_eq!(retry.view().id, message.view().id);
    let after: BTreeMap<_, _> = messaging
        .receipts(sender.id, request.id)
        .unwrap()
        .into_iter()
        .map(|r| (r.recipient, r.wake))
        .collect();
    assert_eq!(after, wakes);
    let mut next = request;
    next.id = Ulid::new();
    let next = messaging.send(sender.id, 1, next, 3_000).unwrap();
    let after: BTreeMap<_, _> = messaging
        .receipts(sender.id, next.view().id)
        .unwrap()
        .into_iter()
        .map(|r| (r.recipient, r.wake))
        .collect();
    assert_eq!(after[&Recipient::Seat(a.id)], Some(ReceiptWake::Now));
    assert_eq!(
        after[&Recipient::Seat(b.id)],
        Some(ReceiptWake::None {
            reason: Some(WakeReason::Policy)
        })
    );
}
