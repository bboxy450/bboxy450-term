// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private real messaging/SQLite witnesses; no model inference or operator keys.
use super::super::keys::LocalSeatKeys;
use super::*;
use crate::adapters::Evidence;
use bbt_proto::messaging::ReceiptWake;
use ed25519_dalek::SigningKey;

fn fixture() -> (tempfile::TempDir, Messaging, SeatRecord, SeatRecord) {
    let root = tempfile::tempdir().unwrap();
    let registry =
        Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
    let mut a = SeatRecord::new(Ulid::new(), Ulid::new(), "solo", "zsh");
    a.role.class = Class::Oracle;
    let b = SeatRecord::new(Ulid::new(), a.swarm, "peer", "zsh");
    registry.insert(&a).unwrap();
    registry.insert(&b).unwrap();
    let messaging = Messaging::open(
        root.path(),
        Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[79; 32]))),
    )
    .unwrap();
    (root, messaging, a, b)
}
fn send(m: &mut Messaging, a: &SeatRecord, b: &SeatRecord, now: u64) -> StoredMessage {
    m.send(
        a.id,
        1,
        SendRequest::text(format!("seats/{}/inbox", b.id), "peer wake"),
        now,
    )
    .unwrap()
}
fn finish(m: &mut Messaging, a: &SeatRecord, work: &Work, evidence: Evidence, now: u64) {
    m.finish(
        a.swarm,
        work,
        &WakeOutcome {
            cause: None,
            evidence,
            native_id: None,
            note: "private circuit fixture".into(),
        },
        now,
        false,
    )
    .unwrap();
}
fn status(m: &Messaging, b: &SeatRecord) -> serde_json::Value {
    m.attention_view(b).unwrap()["circuit_breaker"].clone()
}

#[test]
fn circuit_self_task_history_stays_readable_without_waking() {
    let (_root, mut m, a, _b) = fixture();
    for event in std::iter::once("delegate")
        .chain(std::iter::repeat_n("amend", 22))
        .chain(["report"])
    {
        let mut request = SendRequest::text(format!("seats/{}/inbox", a.id), event);
        request.kind = MessageKind::Task;
        let message = m
            .task_event(
                a.id,
                1,
                request,
                Body::Task {
                    task_id: Ulid::new(),
                    event: event.into(),
                    path: "tasks/self.md".into(),
                    summary: event.into(),
                },
                1000,
            )
            .unwrap();
        assert!(
            m.receipts(a.id, message.view().id)
                .unwrap()
                .iter()
                .all(|r| matches!(r.wake, Some(ReceiptWake::None { .. })))
        );
    }
    assert!(m.ready(1000).unwrap().is_empty());
    assert!(m.prepare(a.swarm, a.id, 1000).unwrap().is_none());
    assert_eq!(
        m.swarm(a.swarm)
            .unwrap()
            .ledger
            .list(Some(a.id), None, None, 0, 128)
            .unwrap()
            .items
            .len(),
        24
    );
    assert_eq!(status(&m, &a)["admissions"], 0);
}

#[test]
fn circuit_latches_at_eight_and_keeps_messages_and_other_seats_live_after_restart() {
    let (root, mut m, a, b) = fixture();
    for i in 0..8 {
        send(&mut m, &a, &b, 1000 + i);
        let w = m.prepare(a.swarm, b.id, 1000 + i).unwrap().unwrap();
        finish(&mut m, &a, &w, Evidence::NativeQueued, 1000 + i);
    }
    let ninth = send(&mut m, &a, &b, 1100);
    assert!(m.prepare(a.swarm, b.id, 1100).unwrap().is_none());
    assert_eq!(status(&m, &b)["open"], true);
    let receipt = m.receipts(a.id, ninth.view().id).unwrap().remove(0);
    assert!(matches!(receipt.wake, Some(ReceiptWake::None { .. })));
    assert!(receipt.note.contains("circuit open"));
    drop(m);
    let mut m = Messaging::open(
        root.path(),
        Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[79; 32]))),
    )
    .unwrap();
    send(&mut m, &a, &b, 600_000);
    assert!(m.prepare(a.swarm, b.id, 600_000).unwrap().is_none());
    assert_eq!(
        status(&m, &b)["open"],
        true,
        "time/restart cannot rearm a latched loop"
    );
    send(&mut m, &b, &a, 600_001);
    assert!(
        m.prepare(a.swarm, a.id, 600_001).unwrap().is_some(),
        "breaker is recipient-local"
    );
    assert_eq!(
        m.swarm(a.swarm)
            .unwrap()
            .ledger
            .list(Some(b.id), None, None, 0, 128)
            .unwrap()
            .items
            .len(),
        11
    );
}

#[test]
fn circuit_refunds_pre_effect_failures_but_not_unknown_or_acknowledgements() {
    let (_root, mut m, a, b) = fixture();
    for i in 0..12 {
        send(&mut m, &a, &b, 1000 + i * 1000);
        let w = m.prepare(a.swarm, b.id, 1000 + i * 1000).unwrap().unwrap();
        finish(&mut m, &a, &w, Evidence::RecipientStored, 1000 + i * 1000);
        assert_eq!(status(&m, &b)["admissions"], 0);
    }
    let w = m.prepare(a.swarm, b.id, 20_000).unwrap().unwrap();
    finish(&mut m, &a, &w, Evidence::NativeUnknown, 20_001);
    assert_eq!(status(&m, &b)["admissions"], 1);
    for message in &w.messages {
        m.acknowledge(b.id, message.view().id, None, 20_002)
            .unwrap();
    }
    assert_eq!(status(&m, &b)["admissions"], 1);
}

#[test]
fn circuit_fresh_operator_message_rearms_once_without_replaying_blocked_backlog() {
    let (_root, mut m, a, b) = fixture();
    for i in 0..9 {
        send(&mut m, &a, &b, 1000 + i);
        if let Some(w) = m.prepare(a.swarm, b.id, 1000 + i).unwrap() {
            finish(&mut m, &a, &w, Evidence::NativeQueued, 1000 + i);
        }
    }
    assert_eq!(status(&m, &b)["open"], true);
    let request = SendRequest::text(format!("seats/{}/inbox", b.id), "human resumes");
    let response = m
        .request_operator_pending(
            None,
            &crate::mcp::Origin::default(),
            a.swarm,
            super::super::api::Request::Send {
                message: request.clone(),
            },
            2000,
        )
        .unwrap();
    assert!(matches!(
        response,
        super::super::api::Response::Stored { .. } | super::super::api::Response::Held { .. }
    ));
    assert_eq!(status(&m, &b)["open"], false);
    let w = m.prepare(a.swarm, b.id, 2000).unwrap().unwrap();
    assert_eq!(w.messages.len(), 1);
    assert_eq!(w.messages[0].view().seat, Ulid::nil());
    finish(&mut m, &a, &w, Evidence::NativeQueued, 2000);
    for i in 0..2 {
        send(&mut m, &a, &b, 2001 + i);
        let w = m.prepare(a.swarm, b.id, 2001 + i).unwrap().unwrap();
        finish(&mut m, &a, &w, Evidence::NativeQueued, 2001 + i);
    }
    m.request_operator_pending(
        None,
        &crate::mcp::Origin::default(),
        a.swarm,
        super::super::api::Request::Send { message: request },
        2005,
    )
    .unwrap();
    assert_eq!(
        status(&m, &b)["admissions"],
        2,
        "operator replay cannot reset credit"
    );
}

#[test]
fn circuit_window_rollover_and_old_completion_cannot_refund_new_epoch() {
    let (_root, mut m, a, b) = fixture();
    send(&mut m, &a, &b, 0);
    let old = m.prepare(a.swarm, b.id, 0).unwrap().unwrap();
    // Operator admission changes the epoch while the old native call is in flight.
    m.request_operator_pending(
        None,
        &crate::mcp::Origin::default(),
        a.swarm,
        super::super::api::Request::Send {
            message: SendRequest::text(format!("seats/{}/inbox", b.id), "reset"),
        },
        1,
    )
    .unwrap();
    finish(&mut m, &a, &old, Evidence::RecipientStored, 2);
    assert_eq!(status(&m, &b)["admissions"], 0);
    let w = m.prepare(a.swarm, b.id, 300).unwrap().unwrap();
    finish(&mut m, &a, &w, Evidence::NativeQueued, 300);
    assert_eq!(status(&m, &b)["admissions"], 1);
    send(&mut m, &a, &b, 60_300);
    let w = m.prepare(a.swarm, b.id, 60_300).unwrap().unwrap();
    finish(&mut m, &a, &w, Evidence::NativeWritten, 60_300);
    assert_eq!(status(&m, &b)["admissions"], 1);
}

#[test]
fn circuit_legacy_self_backlog_larger_than_a_suppression_page_cannot_escape() {
    let (_root, mut m, a, _b) = fixture();
    for _ in 0..300 {
        send(&mut m, &a, &a, 1000);
    }
    {
        // Simulate old stored wake decisions without changing signed envelopes.
        let db = m.swarm(a.swarm).unwrap().ledger.store.connection().unwrap();
        db.execute("UPDATE msg_deliveries SET data=CAST(json_set(json_remove(CAST(data AS TEXT),'$.reason'),'$.wake','now') AS BLOB) WHERE recipient=?1",[a.id.to_string()]).unwrap();
    }
    for _ in 0..3 {
        assert!(m.prepare(a.swarm, a.id, 1000).unwrap().is_none());
    }
    assert!(m.ready(1000).unwrap().is_empty());
    assert_eq!(status(&m, &a)["admissions"], 0);
}

#[test]
fn circuit_reset_does_not_replay_a_large_tripped_backlog() {
    let (_root, mut m, a, b) = fixture();
    for i in 0..8 {
        send(&mut m, &a, &b, 1000 + i);
        let w = m.prepare(a.swarm, b.id, 1000 + i).unwrap().unwrap();
        finish(&mut m, &a, &w, Evidence::NativeQueued, 1000 + i);
    }
    for _ in 0..300 {
        send(&mut m, &a, &b, 1100);
    }
    assert!(m.prepare(a.swarm, b.id, 1100).unwrap().is_none());
    assert_eq!(status(&m, &b)["open"], true);
    m.request_operator_pending(
        None,
        &crate::mcp::Origin::default(),
        a.swarm,
        super::super::api::Request::Send {
            message: SendRequest::text(format!("seats/{}/inbox", b.id), "human resumes"),
        },
        1200,
    )
    .unwrap();
    let w = m.prepare(a.swarm, b.id, 1200).unwrap().unwrap();
    assert_eq!(w.messages.len(), 1);
    assert_eq!(w.messages[0].view().seat, Ulid::nil());
    finish(&mut m, &a, &w, Evidence::NativeQueued, 1200);
    assert!(m.ready(1200).unwrap().is_empty());
    assert_eq!(status(&m, &b)["admissions"], 0);
}
