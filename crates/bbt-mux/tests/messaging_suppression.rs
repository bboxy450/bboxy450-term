// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pending wakes may be cancelled; their signed content and inbox rows stay.
use bbt_mux::{
    Ulid,
    adapters::{Evidence, State, WakeOutcome},
    messaging::{
        Messaging,
        api::{Request, Response},
        keys::LocalSeatKeys,
        tree::Actor,
        wire::{EnvelopeExt, SendRequest},
    },
    registry::{Class, Registry, SeatRecord},
    store::{object_dir, sqlite::SqliteStore},
};
use bbt_proto::{
    messaging::{
        MessageClass, MessageKind, Receipt, ReceiptWake, StoredMessage, WakeMode, WakeReason,
    },
    registry::AttentionRule,
};
use ed25519_dalek::SigningKey;
use std::sync::Arc;

struct Fixture {
    root: tempfile::TempDir,
    messaging: Messaging,
    a: SeatRecord,
    b: SeatRecord,
    c: SeatRecord,
}
impl Fixture {
    fn new(mode: WakeMode) -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry =
            Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
        let swarm = Ulid::new();
        let a = SeatRecord::new(Ulid::new(), swarm, "a", "zsh");
        let b = SeatRecord::new(Ulid::new(), swarm, "b", "zsh");
        let mut c = SeatRecord::new(Ulid::new(), swarm, "c", "zsh");
        c.role.class = Class::Oracle; // can audit the chain, but is not its recipient
        for record in [&a, &b, &c] {
            registry.insert(record).unwrap();
        }
        registry
            .set_attention(
                b.id,
                b.generation,
                AttentionRule {
                    destination: format!("seats/{}/inbox", b.id),
                    class: MessageClass::Conversation,
                    mode,
                },
            )
            .unwrap();
        let messaging = Messaging::open(
            root.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[74; 32]))),
        )
        .unwrap();
        Self {
            root,
            messaging,
            a,
            b,
            c,
        }
    }
    fn send(&mut self, from: Ulid, request: SendRequest, now: u64) -> StoredMessage {
        let Response::Stored { message } = self
            .messaging
            .request(
                from,
                1,
                Request::Send { message: request },
                false,
                None,
                now,
            )
            .unwrap()
        else {
            panic!("stored message")
        };
        *message
    }
    fn receipt(&mut self, id: Ulid) -> Receipt {
        self.messaging
            .receipts(self.a.id, id)
            .unwrap()
            .into_iter()
            .find(|r| r.recipient == self.b.id)
            .unwrap()
    }
    fn none(&mut self, id: Ulid, reason: WakeReason) {
        let receipt = self.receipt(id);
        assert_eq!(receipt.evidence, Evidence::RecipientStored);
        assert_eq!(
            receipt.wake,
            Some(ReceiptWake::None {
                reason: Some(reason)
            })
        );
    }
    fn reopen(&mut self) {
        self.messaging = Messaging::open(
            self.root.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[74; 32]))),
        )
        .unwrap();
    }
}

// keeps: #334 §4a chaining supersession and a content-free withdrawal never delete history.
#[test]
fn supersedes_chains_and_withdrawal_cancel_only_pending_wakes_across_restart() {
    let mut f = Fixture::new(WakeMode::Wake);
    let a = f.send(
        f.a.id,
        SendRequest::text(format!("seats/{}/inbox", f.b.id), "old"),
        1,
    );
    let mut b = SendRequest::text("", "replacement");
    b.supersedes = Some(a.view().id);
    let b = f.send(f.a.id, b, 2);
    let mut c = SendRequest::text("", "");
    c.kind = MessageKind::Withdrawn;
    c.supersedes = Some(b.view().id);
    let c = f.send(f.a.id, c, 3);
    assert_eq!(c.view().body, &bbt_proto::messaging::Body::Withdrawn {});
    f.none(a.view().id, WakeReason::Superseded);
    f.none(b.view().id, WakeReason::Superseded);
    f.none(c.view().id, WakeReason::Policy);
    f.reopen();
    assert!(
        f.messaging
            .prepare(f.a.swarm, f.b.id, 1_000_000)
            .unwrap()
            .is_none()
    );
    let inbox = f.messaging.list(f.b.id, None, None, 0, 128).unwrap();
    assert_eq!(inbox.items.len(), 3);
    assert_eq!(inbox.items[0].view().body.text(), Some("old"));
    for message in inbox.items {
        message.envelope().unwrap().verify_signature().unwrap();
    }
}

// keeps: only the original sender may supersede, before signing or storing an effect.
#[test]
fn another_sender_cannot_withdraw_or_replace_a_message_or_forge_its_signed_target() {
    let mut f = Fixture::new(WakeMode::Wake);
    let original = f.send(
        f.a.id,
        SendRequest::text(format!("seats/{}/inbox", f.b.id), "retained"),
        1,
    );
    for kind in [MessageKind::Text, MessageKind::Withdrawn] {
        let mut request = SendRequest::text("", "");
        request.kind = kind;
        request.supersedes = Some(original.view().id);
        let Response::Refused { error } = f
            .messaging
            .request(
                f.c.id,
                1,
                Request::Send { message: request },
                false,
                None,
                2,
            )
            .unwrap()
        else {
            panic!("ownership refusal")
        };
        assert_eq!(error.kind, "invalid_supersedes");
    }
    assert_eq!(
        f.messaging
            .list(f.b.id, None, None, 0, 128)
            .unwrap()
            .items
            .len(),
        1
    );
    let account = SqliteStore::open(&object_dir(f.root.path(), "account").unwrap()).unwrap();
    assert_eq!(
        account
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM msg_counters WHERE seat=?1",
                [f.c.id.to_string()],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    let mut changed = original.envelope().unwrap().clone();
    changed.supersedes = Some(Ulid::new());
    assert!(
        changed.verify_signature().is_err(),
        "the target is part of the signed bytes"
    );
    assert_eq!(f.receipt(original.view().id).wake, Some(ReceiptWake::Now));
}

// keeps: an actual recipient's in-chain answer cancels its question; someone else's does not.
#[test]
fn only_the_addressed_recipient_answers_the_question_through_a_reply_chain() {
    let mut f = Fixture::new(WakeMode::Wake);
    let question = f.send(
        f.a.id,
        SendRequest::text(format!("seats/{}/inbox", f.b.id), "question"),
        1,
    );
    let mut other = SendRequest::text(format!("seats/{}/inbox", f.b.id), "not an answer from b");
    other.reply_to = Some(question.view().id);
    let other = f.send(f.c.id, other, 2);
    assert_eq!(f.receipt(question.view().id).wake, Some(ReceiptWake::Now));
    let mut answer = SendRequest::text("", "b answered");
    answer.reply_to = Some(other.view().id);
    f.send(f.b.id, answer, 3);
    f.none(question.view().id, WakeReason::Done);
    assert!(
        f.messaging
            .prepare(f.a.swarm, f.b.id, 10_000)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        f.messaging
            .list(f.b.id, None, None, 0, 128)
            .unwrap()
            .items
            .len(),
        3
    );
}

// keeps: cancelling a digest member leaves neither its body nor an empty native summary.
#[test]
fn cancelled_digest_members_never_reappear_at_the_original_deadline() {
    let mut f = Fixture::new(WakeMode::Digest);
    let original = f.send(
        f.a.id,
        SendRequest::text(format!("seats/{}/inbox", f.b.id), "old digest member"),
        1,
    );
    assert_eq!(
        f.messaging.deferred_view(&f.b).unwrap(),
        Some(serde_json::json!({"deferred":{"count":1,"next_at_ms":600_001}}))
    );
    let mut withdrawal = SendRequest::text("", "");
    withdrawal.kind = MessageKind::Withdrawn;
    withdrawal.supersedes = Some(original.view().id);
    f.send(f.a.id, withdrawal, 2);
    f.none(original.view().id, WakeReason::Superseded);
    assert_eq!(f.messaging.deferred_view(&f.b).unwrap(), None);
    assert!(
        f.messaging
            .prepare(f.a.swarm, f.b.id, 600_001)
            .unwrap()
            .is_none()
    );
    f.reopen();
    assert!(
        f.messaging
            .prepare(f.a.swarm, f.b.id, 1_200_001)
            .unwrap()
            .is_none()
    );
    assert_eq!(f.messaging.deferred_view(&f.b).unwrap(), None);
}

// keeps: an immutable id delivered once is not re-woken by replay or a later replacement.
#[test]
fn native_written_evidence_survives_supersession_and_replayed_ids_never_wake_twice() {
    let mut f = Fixture::new(WakeMode::Wake);
    let request = SendRequest::text(format!("seats/{}/inbox", f.b.id), "deliver once");
    let original = f.send(f.a.id, request.clone(), 1);
    let work = f
        .messaging
        .prepare(f.a.swarm, f.b.id, 10_000)
        .unwrap()
        .unwrap();
    f.messaging
        .finish(
            f.a.swarm,
            &work,
            &WakeOutcome {
                cause: None,
                evidence: Evidence::NativeWritten,
                native_id: Some("one actual attempt".into()),
                note: "written".into(),
            },
            10_001,
            false,
        )
        .unwrap();
    let before = f.receipt(original.view().id);
    f.send(f.a.id, request.clone(), 10_002);
    let mut withdrawal = SendRequest::text("", "");
    withdrawal.kind = MessageKind::Withdrawn;
    withdrawal.supersedes = Some(original.view().id);
    f.send(f.a.id, withdrawal, 10_003);
    assert_eq!(f.receipt(original.view().id), before);
    f.reopen();
    f.send(f.a.id, request, 20_000);
    assert!(
        f.messaging
            .prepare(f.a.swarm, f.b.id, 30_000)
            .unwrap()
            .is_none()
    );
    assert_eq!(f.receipt(original.view().id), before);
}

// keeps: a sender-held message is still cancelled before its first inbox scheduling.
#[test]
fn a_superseded_held_message_cannot_rearm_when_the_sender_is_unmuted() {
    let mut f = Fixture::new(WakeMode::Wake);
    f.messaging
        .set_mute(Actor::Operator, f.a.id, true, State::Active, 1)
        .unwrap();
    let old = f.send(
        f.a.id,
        SendRequest::text(format!("seats/{}/inbox", f.b.id), "held"),
        2,
    );
    assert!(old.held_out.is_some());
    let mut withdrawal = SendRequest::text("", "");
    withdrawal.kind = MessageKind::Withdrawn;
    withdrawal.supersedes = Some(old.view().id);
    f.send(f.a.id, withdrawal, 3);
    f.messaging
        .set_mute(Actor::Operator, f.a.id, false, State::Active, 4)
        .unwrap();
    f.none(old.view().id, WakeReason::Superseded);
    assert!(
        f.messaging
            .prepare(f.a.swarm, f.b.id, 10_000)
            .unwrap()
            .is_none()
    );
}

// keeps: a no-effect native refusal may retry live work, but never a cancelled member.
#[test]
fn a_cancelled_inflight_attempt_does_not_rearm_when_the_adapter_reports_no_effect() {
    let mut f = Fixture::new(WakeMode::Wake);
    let old = f.send(
        f.a.id,
        SendRequest::text(format!("seats/{}/inbox", f.b.id), "cancel before retry"),
        1,
    );
    let work = f
        .messaging
        .prepare(f.a.swarm, f.b.id, 10_000)
        .unwrap()
        .unwrap();
    let mut next = SendRequest::text("", "the retained next message");
    next.supersedes = Some(old.view().id);
    let next = f.send(f.a.id, next, 10_001);
    f.messaging
        .finish(
            f.a.swarm,
            &work,
            &WakeOutcome {
                cause: None,
                evidence: Evidence::RecipientStored,
                native_id: None,
                note: "adapter refused before mutation".into(),
            },
            10_002,
            false,
        )
        .unwrap();
    f.none(old.view().id, WakeReason::Superseded);
    let next_work = f
        .messaging
        .prepare(f.a.swarm, f.b.id, 10_300)
        .unwrap()
        .unwrap();
    assert_eq!(
        next_work
            .messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        vec![next.view().id]
    );
}

// keeps: #334 certified seat continuity owns old messages after a leaf-key rotation.
#[test]
fn the_same_seat_can_withdraw_after_a_generation_and_signing_key_change() {
    let mut f = Fixture::new(WakeMode::Wake);
    let old = f.send(
        f.a.id,
        SendRequest::text(format!("seats/{}/inbox", f.b.id), "before rotation"),
        1,
    );
    let registry =
        Registry::new(SqliteStore::open(&object_dir(f.root.path(), "account").unwrap()).unwrap());
    registry
        .update(f.a.id, |current| {
            current.generation = 2;
            Ok(())
        })
        .unwrap();
    let mut withdrawal = SendRequest::text("", "");
    withdrawal.kind = MessageKind::Withdrawn;
    withdrawal.supersedes = Some(old.view().id);
    let Response::Stored { message: new } = f
        .messaging
        .request(
            f.a.id,
            2,
            Request::Send {
                message: withdrawal,
            },
            false,
            None,
            2,
        )
        .unwrap()
    else {
        panic!("same certified seat")
    };
    assert_ne!(
        new.envelope().unwrap().author,
        old.envelope().unwrap().author
    );
    f.none(old.view().id, WakeReason::Superseded);
}
