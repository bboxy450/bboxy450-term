// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::super::keys::LocalSeatKeys;
use super::*;
use bbt_proto::{
    messaging::{ReceiptWake, WakeReason},
    registry::AttentionRule,
};
use ed25519_dalek::SigningKey;

fn fixture() -> (tempfile::TempDir, Messaging, SeatRecord, SeatRecord) {
    let root = tempfile::tempdir().unwrap();
    let registry =
        Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
    let swarm = Ulid::new();
    let mut caller = SeatRecord::new(Ulid::new(), swarm, "oracle", "zsh");
    caller.role.class = Class::Oracle;
    let receiver = SeatRecord::new(Ulid::new(), swarm, "executor", "zsh");
    registry.insert(&caller).unwrap();
    registry.insert(&receiver).unwrap();
    registry
        .set_attention(
            receiver.id,
            1,
            AttentionRule {
                destination: format!("seats/{}/inbox", receiver.id),
                class: bbt_proto::messaging::MessageClass::Conversation,
                mode: WakeMode::Wake,
            },
        )
        .unwrap();
    let messaging = Messaging::open(
        root.path(),
        Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[75; 32]))),
    )
    .unwrap();
    (root, messaging, caller, receiver)
}
fn delegate(
    messaging: &mut Messaging,
    caller: &SeatRecord,
    receiver: &SeatRecord,
    task: Ulid,
    now: u64,
) -> StoredMessage {
    let mut request = SendRequest::text(format!("seats/{}/inbox", receiver.id), "delegated task");
    request.kind = MessageKind::Task;
    messaging
        .task_event(
            caller.id,
            1,
            request,
            Body::Task {
                task_id: task,
                event: "delegate".into(),
                path: "tasks/one.md".into(),
                summary: "delegated task".into(),
            },
            now,
        )
        .unwrap()
}
fn wake(messaging: &mut Messaging, caller: &SeatRecord, id: Ulid) -> Option<ReceiptWake> {
    messaging.receipts(caller.id, id).unwrap().remove(0).wake
}

#[test]
fn amendments_wake_inbox_only_receivers_and_join_only_their_root_delegation() {
    let (_root, mut messaging, caller, receiver) = fixture();
    messaging
        .registry
        .set_attention(
            receiver.id,
            1,
            AttentionRule {
                destination: format!("seats/{}/inbox", receiver.id),
                class: bbt_proto::messaging::MessageClass::Conversation,
                mode: WakeMode::Inbox,
            },
        )
        .unwrap();
    let task = Ulid::new();
    let mut send = |root: &str, event: &str, now| {
        let summary = format!("task brief {event}");
        let mut request = SendRequest::text(format!("seats/{}/inbox", receiver.id), &summary);
        request.kind = MessageKind::Task;
        messaging.task_root(caller.swarm, request.id, root).unwrap();
        messaging
            .task_event(
                caller.id,
                1,
                request,
                Body::Task {
                    task_id: task,
                    event: event.into(),
                    path: "tasks/one.md".into(),
                    summary,
                },
                now,
            )
            .unwrap()
    };
    let a = send("root-a", "delegate", 1);
    let b = send("root-b", "delegate", 2);
    let amendment = send("root-a", "amend", 3);
    let later = send("root-a", "delegate", 4);
    let legacy = send("root-legacy", "amend", 5);
    let legacy_next = send("root-legacy", "amend", 6);
    assert_eq!(legacy.view().thread_id, legacy.view().id);
    assert_eq!(legacy_next.view().thread_id, legacy.view().thread_id);
    assert_eq!(amendment.view().thread_id, a.view().thread_id);
    assert_ne!(amendment.view().thread_id, b.view().thread_id);
    assert_ne!(amendment.view().thread_id, later.view().thread_id);
    amendment.envelope().unwrap().verify_signature().unwrap();
    assert_eq!(
        wake(&mut messaging, &caller, amendment.view().id),
        Some(ReceiptWake::Now)
    );
    let mut retry = SendRequest::text(format!("seats/{}/inbox", receiver.id), "task brief amend");
    retry.id = amendment.view().id;
    retry.kind = MessageKind::Task;
    let replay = messaging
        .task_event(caller.id, 1, retry, amendment.view().body.clone(), 7)
        .unwrap();
    assert_eq!(replay.envelope(), amendment.envelope());
    assert_eq!(replay.sequence, amendment.sequence);
    let mut reply = SendRequest::text("", "using the amended legacy brief");
    reply.reply_to = Some(legacy_next.view().id);
    let reply = messaging.send(receiver.id, 1, reply, 8).unwrap();
    assert_eq!(reply.view().thread_id, legacy.view().thread_id);
    let mut request = SendRequest::text(format!("seats/{}/inbox", caller.id), "forged amendment");
    request.kind = MessageKind::Task;
    assert!(
        messaging
            .task_event(
                receiver.id,
                1,
                request,
                Body::Task {
                    task_id: task,
                    event: "amend".into(),
                    path: "tasks/one.md".into(),
                    summary: "forged amendment".into(),
                },
                4
            )
            .unwrap_err()
            .to_string()
            .contains("oracle-class")
    );
}

// keeps: #334 terminal authority is exactly accepted/rejected, never report prose or a reply.
#[test]
fn terminal_task_records_cancel_delegations_and_never_rearm_their_old_messages() {
    for terminal in ["accepted", "rejected"] {
        let (_root, mut messaging, caller, receiver) = fixture();
        let task = Ulid::new();
        let message = delegate(&mut messaging, &caller, &receiver, task, 1);
        for status in ["reported", "done", "closed"] {
            messaging.task_state(task, status, 2).unwrap();
            assert_eq!(
                wake(&mut messaging, &caller, message.view().id),
                Some(ReceiptWake::Now)
            );
        }
        let mut reply = SendRequest::text("", "report is a reply, not acceptance");
        reply.reply_to = Some(message.view().id);
        messaging.send(receiver.id, 1, reply, 3).unwrap();
        assert_eq!(
            wake(&mut messaging, &caller, message.view().id),
            Some(ReceiptWake::Now)
        );
        messaging.task_state(task, terminal, 4).unwrap();
        assert_eq!(
            wake(&mut messaging, &caller, message.view().id),
            Some(ReceiptWake::None {
                reason: Some(WakeReason::Done)
            })
        );
        assert!(
            messaging
                .prepare(caller.swarm, receiver.id, 10_000)
                .unwrap()
                .is_none()
        );
        messaging.task_state(task, "claimed", 5).unwrap();
        let new = delegate(&mut messaging, &caller, &receiver, task, 6);
        let work = messaging
            .prepare(caller.swarm, receiver.id, 10_000)
            .unwrap()
            .unwrap();
        assert_eq!(
            work.messages
                .iter()
                .map(|m| m.view().id)
                .collect::<Vec<_>>(),
            vec![new.view().id]
        );
    }
}

// keeps: an already terminal task is stored and receipted without admitting a wake.
#[test]
fn a_terminal_task_is_done_at_admission_and_a_text_claim_cannot_set_its_status() {
    let (_root, mut messaging, caller, receiver) = fixture();
    let task = Ulid::new();
    messaging.task_state(task, "accepted", 1).unwrap();
    let message = delegate(&mut messaging, &caller, &receiver, task, 2);
    assert_eq!(
        wake(&mut messaging, &caller, message.view().id),
        Some(ReceiptWake::None {
            reason: Some(WakeReason::Done)
        })
    );
    assert_eq!(
        messaging
            .list(receiver.id, None, None, 0, 16)
            .unwrap()
            .items
            .len(),
        1
    );
    let other = Ulid::new();
    let pending = delegate(&mut messaging, &caller, &receiver, other, 3);
    messaging
        .send(
            caller.id,
            1,
            SendRequest::text(
                format!("seats/{}/inbox", receiver.id),
                format!("Task {other} accepted"),
            ),
            4,
        )
        .unwrap();
    assert_eq!(
        wake(&mut messaging, &caller, pending.view().id),
        Some(ReceiptWake::Now)
    );
}

// keeps: local withdrawal remains available without an operator signing key (#286).
#[test]
fn local_operator_withdrawal_cancels_an_unsigned_intent_without_signing() {
    use super::super::api::{Request, Response};
    let (_root, mut messaging, caller, receiver) = fixture();
    let Response::Stored { message: old } = messaging
        .request_operator_pending(
            None,
            &Default::default(),
            caller.swarm,
            Request::Send {
                message: SendRequest::text(
                    format!("seats/{}/inbox", receiver.id),
                    "local pending original",
                ),
            },
            1,
        )
        .unwrap()
    else {
        panic!("local stored intent")
    };
    assert!(old.envelope().is_none());
    let mut request = SendRequest::text("", "");
    request.kind = MessageKind::Withdrawn;
    request.supersedes = Some(old.view().id);
    let Response::Stored { message: new } = messaging
        .request_operator_pending(
            None,
            &Default::default(),
            caller.swarm,
            Request::Send { message: request },
            2,
        )
        .unwrap()
    else {
        panic!("local withdrawal")
    };
    assert!(new.envelope().is_none());
    assert_eq!(new.intent().unwrap().body, Body::Withdrawn {});
    let receipt = messaging
        .swarm(caller.swarm)
        .unwrap()
        .ledger
        .receipts(old.view().id)
        .unwrap()
        .remove(0);
    assert_eq!(
        receipt.wake,
        Some(ReceiptWake::None {
            reason: Some(WakeReason::Superseded)
        })
    );
    assert!(
        messaging
            .prepare(caller.swarm, receiver.id, 10_000)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        messaging
            .account
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM msg_counters", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

// keeps: cwd selects records; copied task IDs cannot cancel another root's wakes.
#[test]
fn workspace_task_status_isolated_across_roots_and_same_swarm_cwd_changes() {
    let (_root, mut messaging, caller, receiver) = fixture();
    let task = Ulid::new();
    let mut send = |root: &str, now: u64| {
        let mut request =
            SendRequest::text(format!("seats/{}/inbox", receiver.id), "root-bound task");
        request.kind = MessageKind::Task;
        messaging.task_root(caller.swarm, request.id, root).unwrap();
        messaging
            .task_event(
                caller.id,
                1,
                request,
                Body::Task {
                    task_id: task,
                    event: "delegate".into(),
                    path: "tasks/same.md".into(),
                    summary: "root-bound task".into(),
                },
                now,
            )
            .unwrap()
            .view()
            .id
    };
    let first = send("project-a", 1);
    let second = send("project-b", 2);
    messaging.bind_workspace(caller.swarm, "project-b").unwrap();
    messaging
        .task_state_root("project-a", &[], task, "accepted", 3)
        .unwrap();
    assert_eq!(
        wake(&mut messaging, &caller, first),
        Some(ReceiptWake::None {
            reason: Some(WakeReason::Done)
        })
    );
    assert_eq!(
        wake(&mut messaging, &caller, second),
        Some(ReceiptWake::Now)
    );
    // The compatibility projection cannot suppress a root-tagged delegation.
    messaging.task_state(task, "rejected", 4).unwrap();
    assert_eq!(
        wake(&mut messaging, &caller, second),
        Some(ReceiptWake::Now)
    );
    messaging
        .task_state_root("project-b", &[caller.swarm], task, "accepted", 5)
        .unwrap();
    assert_eq!(
        wake(&mut messaging, &caller, second),
        Some(ReceiptWake::None {
            reason: Some(WakeReason::Done)
        })
    );
}

#[test]
fn workspace_pending_task_reopen_hydrates_its_source_root_after_cwd_changes() {
    let (root, mut messaging, caller, receiver) = fixture();
    let task = Ulid::new();
    let mut request = SendRequest::text(
        format!("seats/{}/inbox", receiver.id),
        "scoped pending task",
    );
    request.kind = MessageKind::Task;
    messaging
        .task_root(caller.swarm, request.id, "project-a")
        .unwrap();
    let id = messaging
        .task_event(
            caller.id,
            1,
            request,
            Body::Task {
                task_id: task,
                event: "delegate".into(),
                path: "tasks/same.md".into(),
                summary: "scoped pending task".into(),
            },
            1,
        )
        .unwrap()
        .view()
        .id;
    messaging.bind_workspace(caller.swarm, "project-b").unwrap();
    // Crash after account projection, before applying the per-ledger suppression.
    messaging
        .account
        .put(
            &format!("messaging/task-state-root/project-a/{task}"),
            b"accepted",
        )
        .unwrap();
    drop(messaging);
    let mut reopened = Messaging::open(
        root.path(),
        Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[75; 32]))),
    )
    .unwrap();
    assert_eq!(
        wake(&mut reopened, &caller, id),
        Some(ReceiptWake::None {
            reason: Some(WakeReason::Done)
        })
    );
}

#[test]
fn workspace_legacy_terminal_state_migrates_only_into_trusted_startup_root() {
    let (root, mut messaging, caller, receiver) = fixture();
    let task = Ulid::new();
    let id = delegate(&mut messaging, &caller, &receiver, task, 1)
        .view()
        .id;
    messaging
        .account
        .put(&format!("messaging/task-state/{task}"), b"accepted")
        .unwrap();
    drop(messaging);
    let mut reopened = Messaging::open(
        root.path(),
        Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[75; 32]))),
    )
    .unwrap();
    reopened.startup_workspace("startup").unwrap();
    assert_eq!(
        wake(&mut reopened, &caller, id),
        Some(ReceiptWake::None {
            reason: Some(WakeReason::Done)
        })
    );
}
