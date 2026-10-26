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
