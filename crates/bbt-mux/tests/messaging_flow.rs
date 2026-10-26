//! A custody hold changes message evidence, never same-device delivery.
use std::sync::Arc;

use bbt_mux::{
    Ulid,
    adapters::{Evidence, WakeOutcome},
    messaging::{
        Messaging,
        api::{Request, Response},
        keys::{LocalSeatKeys, SeatKeys},
        wire::{Envelope, EnvelopeExt, Issuer, IssuerKind, SendRequest},
    },
    registry::{Registry, SeatRecord},
    store::{Result, invalid, object_dir, sqlite::SqliteStore},
};
use bbt_proto::messaging::SignatureState;
use ed25519_dalek::SigningKey;

#[path = "messaging_flow/gui.rs"]
mod gui;

/// First startup has neither a public author nor any available signing secret.
struct FirstHeld;
impl SeatKeys for FirstHeld {
    fn issuer(&self) -> Issuer {
        Issuer {
            kind: IssuerKind::LocalDev,
            key: String::new(),
        }
    }
    fn ready_issuer(&self) -> Option<Issuer> {
        None
    }
    fn signing_hold(&self) -> Option<String> {
        Some("fixture custody unavailable".into())
    }
    fn leaf(&self, _: Ulid, _: u64) -> Result<SigningKey> {
        Err(invalid("fixture has no signing author"))
    }
    fn verify(&self, _: &Envelope) -> Result<()> {
        Err(invalid("a pending intent is never a signed envelope"))
    }
}

// keeps: invariant messaging-never-held
#[test]
fn custody_hold_never_holds_local_delivery() {
    let root = tempfile::tempdir().unwrap();
    let account = SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap();
    let registry = Registry::new(account);
    let swarm = Ulid::new();
    let sender = SeatRecord::new(Ulid::new(), swarm, "sender", "zsh");
    let recipient = SeatRecord::new(Ulid::new(), swarm, "recipient", "zsh");
    registry.insert(&sender).unwrap();
    registry.insert(&recipient).unwrap();
    let mut messaging = Messaging::open(root.path(), Arc::new(FirstHeld)).unwrap();
    let request = SendRequest::text(
        format!("seats/{}/inbox", recipient.id),
        "local delivery precedes cryptographic author availability",
    );
    let outcome = messaging
        .request(
            sender.id,
            sender.generation,
            Request::Send { message: request },
            false,
            None,
            1,
        )
        .unwrap();
    assert!(
        matches!(outcome, Response::Stored { .. }),
        "custody gated a local first-startup delivery: {outcome:?}"
    );
    let inbox = messaging.list(recipient.id, None, None, 0, 16).unwrap();
    assert_eq!(
        inbox.items.len(),
        1,
        "recipient must already have its message"
    );
    let pending = &inbox.items[0];
    assert!(pending.envelope().is_none());
    assert_eq!(pending.view().signature, SignatureState::Pending);
    let intent = pending.intent().unwrap();
    let id = intent.id;
    let binding = bbt_mux::messaging::pending::binding(intent).unwrap();
    let value = serde_json::to_value(intent).unwrap();
    assert!(value.get("author").is_none());
    assert!(value.get("issuer").is_none());
    let receipt = messaging.receipts(sender.id, id).unwrap().remove(0);
    assert_eq!(receipt.evidence, Evidence::RecipientStored);
    assert_eq!(receipt.signature, SignatureState::Pending);
    assert_eq!(
        receipt.signature_reason.as_deref(),
        Some("fixture custody unavailable")
    );

    let work = messaging
        .prepare(swarm, recipient.id, 10000)
        .unwrap()
        .unwrap();
    assert!(work.text().contains("signature: pending"));
    assert!(work.text().contains("Local inferred content"));
    assert!(!work.text().contains("Authenticated seat content"));
    messaging
        .finish(
            swarm,
            &work,
            &WakeOutcome {
                evidence: Evidence::NativeWritten,
                native_id: Some("fixture-single-native-attempt".into()),
                note: "fixture wrote once".into(),
            },
            10001,
            false,
        )
        .unwrap();

    let ledger =
        SqliteStore::open(&object_dir(root.path(), &format!("swarm-{swarm}")).unwrap()).unwrap();
    let retained = || {
        let db = ledger.connection().unwrap();
        let intent: Vec<u8> = db
            .query_row(
                "SELECT envelope FROM msg_messages WHERE id=?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        let draft: Vec<u8> = db
            .query_row(
                "SELECT draft FROM msg_bus_outbox WHERE id=?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        (intent, draft)
    };
    let original = retained();
    drop(messaging);
    let mut messaging = Messaging::open(root.path(), Arc::new(FirstHeld)).unwrap();
    assert_eq!(
        messaging
            .list(recipient.id, None, None, 0, 16)
            .unwrap()
            .items[0]
            .view()
            .signature,
        SignatureState::Pending
    );
    let keys = Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[42; 32])));
    assert_eq!(messaging.resume_signing(keys.clone(), 20000).unwrap(), 1);
    let signed = messaging
        .list(recipient.id, None, None, 0, 16)
        .unwrap()
        .items
        .remove(0);
    assert_eq!(signed.view().id, id);
    assert_eq!(signed.upgrade_of(), Some(&binding));
    signed.envelope().unwrap().verify_signature().unwrap();
    assert_eq!(signed.envelope().unwrap().sender_sequence, 1);
    assert_eq!(
        retained(),
        original,
        "neither original intent nor delivered bus draft may change"
    );
    let receipt = messaging.receipts(sender.id, id).unwrap().remove(0);
    assert_eq!(receipt.signature, SignatureState::Signed);
    assert_eq!(receipt.evidence, Evidence::NativeWritten);
    assert_eq!(receipt.attempt, Some(work.id));
    assert_eq!(
        receipt.native_id.as_deref(),
        Some("fixture-single-native-attempt")
    );
    assert!(
        messaging
            .prepare(swarm, recipient.id, 20001)
            .unwrap()
            .is_none(),
        "signature completion must not deliver twice"
    );
    assert_eq!(messaging.resume_signing(keys, 20002).unwrap(), 0);
    assert_eq!(
        ledger
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM msg_signatures", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

// keeps: decision #285, pinned remote delivery and admission-order signatures.
#[test]
fn pending_remote_delivery_waits_and_signed_completions_follow_admission_order() {
    remote_pending_fixture();
}

fn remote_pending_fixture() {
    let root = tempfile::tempdir().unwrap();
    let account = SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap();
    let registry = Registry::new(account);
    let swarm = Ulid::new();
    let sender = SeatRecord::new(Ulid::new(), swarm, "sender", "zsh");
    let mut remote = SeatRecord::new(Ulid::new(), swarm, "remote", "zsh");
    remote.location = bbt_mux::registry::Location::Ssh {
        host: "fixture-only".into(),
    };
    registry.insert(&sender).unwrap();
    registry.insert(&remote).unwrap();
    let mut messaging = Messaging::open(root.path(), Arc::new(FirstHeld)).unwrap();
    let mut ids = vec![];
    for n in 0..3 {
        let request = SendRequest::text(
            format!("seats/{}/inbox", remote.id),
            format!("held remote {n}"),
        );
        ids.push(request.id);
        let stored = messaging.send(sender.id, 1, request, n + 1000).unwrap();
        assert!(stored.envelope().is_none());
        let receipt = messaging
            .receipts(sender.id, stored.view().id)
            .unwrap()
            .remove(0);
        assert_eq!(receipt.evidence, Evidence::SenderStored);
        assert_eq!(receipt.signature, SignatureState::Pending);
        assert_eq!(
            receipt.held.as_deref(),
            Some("signature pending; pinned remote delivery waits")
        );
    }
    assert!(messaging.ready(10000).unwrap().is_empty());
    assert!(
        messaging
            .prepare(swarm, remote.id, 10000)
            .unwrap()
            .is_none()
    );
    assert!(
        messaging
            .list(remote.id, None, None, 0, 16)
            .unwrap()
            .items
            .is_empty(),
        "unsigned intents must not enter a pinned remote inbox"
    );
    assert_eq!(
        messaging
            .resume_signing(
                Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[43; 32]))),
                20000
            )
            .unwrap(),
        3
    );
    let rows = messaging.list(remote.id, None, None, 0, 16).unwrap().items;
    assert_eq!(rows.len(), 3);
    for (index, message) in rows.iter().enumerate() {
        assert_eq!(message.view().id, ids[index]);
        let envelope = message.envelope().unwrap();
        envelope.verify_signature().unwrap();
        assert_eq!(envelope.sender_sequence, index as u64 + 1);
        assert_eq!(envelope.timestamp, 1);
        let receipt = messaging.receipts(sender.id, ids[index]).unwrap().remove(0);
        assert_eq!(receipt.evidence, Evidence::RecipientStored);
        assert_eq!(receipt.signature, SignatureState::Signed);
    }
}

// keeps: invariant messaging-never-held, delivery beyond 128 rows and bounded signing batches.
#[test]
fn local_delivery_outlives_the_old_signing_queue_cap_and_resume_is_bounded() {
    let root = tempfile::tempdir().unwrap();
    let account = SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap();
    let registry = Registry::new(account.clone());
    let swarm = Ulid::new();
    let sender = SeatRecord::new(Ulid::new(), swarm, "sender", "zsh");
    let recipient = SeatRecord::new(Ulid::new(), swarm, "recipient", "zsh");
    registry.insert(&sender).unwrap();
    registry.insert(&recipient).unwrap();
    let mut messaging = Messaging::open(root.path(), Arc::new(FirstHeld)).unwrap();
    let mut ids = vec![];
    for n in 0..130 {
        let request = SendRequest::text(
            format!("seats/{}/inbox", recipient.id),
            format!("local during hold {n}"),
        );
        ids.push(request.id);
        let stored = messaging.send(sender.id, 1, request, 1000 + n).unwrap();
        assert_eq!(stored.view().signature, SignatureState::Pending);
    }
    let first = messaging.list(recipient.id, None, None, 0, 128).unwrap();
    assert_eq!(first.items.len(), 128);
    let second = messaging
        .list(recipient.id, None, None, first.next.unwrap(), 128)
        .unwrap();
    assert_eq!(second.items.len(), 2);
    assert_eq!(
        first
            .items
            .iter()
            .chain(&second.items)
            .map(|message| message.view().id)
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        account
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM msg_counters", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0,
        "no placeholder signing sequence while custody is held"
    );
    let keys = Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[44; 32])));
    assert_eq!(messaging.resume_signing(keys.clone(), 20000).unwrap(), 64);
    assert_eq!(messaging.resume_signing(keys.clone(), 20001).unwrap(), 64);
    assert_eq!(messaging.resume_signing(keys.clone(), 20002).unwrap(), 2);
    assert_eq!(messaging.resume_signing(keys, 20003).unwrap(), 0);
    let first = messaging.list(recipient.id, None, None, 0, 128).unwrap();
    let second = messaging
        .list(recipient.id, None, None, first.next.unwrap(), 128)
        .unwrap();
    for (index, message) in first.items.iter().chain(&second.items).enumerate() {
        let signed = message.envelope().unwrap();
        signed.verify_signature().unwrap();
        assert_eq!(signed.id, ids[index]);
        assert_eq!(signed.sender_sequence, index as u64 + 1);
    }
}

// keeps: invariant messaging-never-held, actual GUI door and durable event lane.
#[test]
#[ignore = "#327 B2: real GUI/PTY/custody retry boundary; scripts/night-test-boundaries.sh"]
fn custody_hold_gui_boundary() {
    gui::exercise_held_startup();
}

// keeps: invariant notices-never-wake; spec messaging-attention §3
#[test]
fn events_are_stored_and_never_wake_by_default() {
    use bbt_mux::{adapters::State, messaging::tree::Actor, registry::Class};
    use bbt_proto::messaging::{MessageClass, ReceiptWake};
    let root = tempfile::tempdir().unwrap();
    let registry = Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
    let swarm = Ulid::new();
    let mut subject = SeatRecord::new(Ulid::new(), swarm, "subject", "zsh");
    subject.muted = true;
    registry.insert(&subject).unwrap();
    let mut receivers = Vec::new();
    for role in ["executor", "architect", "creativity", "oracle"] {
        let mut record = SeatRecord::new(Ulid::new(), swarm, role, "zsh");
        record.role.name = role.into();
        if role == "oracle" { record.role.class = Class::Oracle; }
        registry.insert(&record).unwrap();
        receivers.push(record);
    }
    let keys = Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[65; 32])));
    let mut messaging = Messaging::open(root.path(), keys).unwrap();
    let notice = messaging.set_mute(Actor::Operator, subject.id, false, State::Idle, 1_000).unwrap().unwrap();
    for receiver in &receivers {
        let inbox = messaging.list(receiver.id, None, None, 0, 16).unwrap();
        let message = inbox.items.iter().find(|message| message.view().id == notice).unwrap();
        assert_eq!(message.view().kind.class(), MessageClass::Event);
        message.envelope().unwrap().verify_signature().unwrap();
        let receipt = messaging.receipts(receiver.id, notice).unwrap().into_iter().find(|r| r.recipient == receiver.id).unwrap();
        assert_eq!(receipt.evidence, Evidence::RecipientStored);
        assert_eq!(receipt.wake, Some(ReceiptWake::None), "{} default must not wake on operator status", receiver.role.name);
        assert_eq!(receipt.held, None, "metadata is not a delivery hold");
        assert!(messaging.prepare(swarm, receiver.id, 1_001).unwrap().is_none());
    }
    assert!(messaging.ready(1_002).unwrap().is_empty());
    let oracle = receivers.last().unwrap();
    let request = SendRequest::text(format!("seats/{}/inbox", oracle.id), "conversation to a wake-default receiver");
    let id = request.id;
    messaging.request(subject.id, 1, Request::Send { message: request }, false, None, 2_000).unwrap();
    let receipt = messaging.receipts(subject.id, id).unwrap().remove(0);
    assert_eq!(receipt.wake, Some(ReceiptWake::Now));
    let work = messaging.prepare(swarm, oracle.id, 2_001).unwrap().unwrap();
    assert_eq!(work.messages.len(), 1);
    assert_eq!(work.messages[0].view().id, id);
    messaging.finish(swarm, &work, &WakeOutcome { evidence: Evidence::NativeWritten, native_id: Some("one-attention-wake".into()), note: "fixture native delivery".into() }, 2_002, false).unwrap();
    assert!(messaging.prepare(swarm, oracle.id, 2_003).unwrap().is_none());
}
