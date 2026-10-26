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
    // This boundary proves native delivery during custody hold; receiver defaults
    // are exercised separately by the attention tests below.
    set_attention(
        &registry,
        &recipient,
        bbt_proto::messaging::MessageClass::Conversation,
        format!("seats/{}/inbox", recipient.id),
        bbt_proto::messaging::WakeMode::Wake,
    );
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
        assert_eq!(
            receipt.wake, None,
            "no recipient admission while signature is pending remotely"
        );
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
    set_attention(
        &registry,
        &remote,
        bbt_proto::messaging::MessageClass::Conversation,
        format!("seats/{}/inbox", remote.id),
        bbt_proto::messaging::WakeMode::Wake,
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
        assert_eq!(receipt.wake, Some(bbt_proto::messaging::ReceiptWake::Now));
        assert_eq!(receipt.received_ms, 20_000);
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

fn set_attention(
    registry: &Registry,
    receiver: &SeatRecord,
    class: bbt_proto::messaging::MessageClass,
    destination: String,
    mode: bbt_proto::messaging::WakeMode,
) {
    registry
        .set_attention(
            receiver.id,
            receiver.generation,
            bbt_proto::registry::AttentionRule {
                destination,
                class,
                mode,
            },
        )
        .unwrap();
}

struct AttentionFixture {
    root: tempfile::TempDir,
    registry: Registry,
    messaging: Messaging,
    sender: SeatRecord,
    receiver: SeatRecord,
}
impl AttentionFixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry =
            Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
        let swarm = Ulid::new();
        let sender = SeatRecord::new(Ulid::new(), swarm, "sender", "zsh");
        let receiver = SeatRecord::new(Ulid::new(), swarm, "receiver", "zsh");
        registry.insert(&sender).unwrap();
        registry.insert(&receiver).unwrap();
        let messaging = Messaging::open(
            root.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[66; 32]))),
        )
        .unwrap();
        Self {
            root,
            registry,
            messaging,
            sender,
            receiver,
        }
    }
    fn mode(&self, mode: bbt_proto::messaging::WakeMode) {
        set_attention(
            &self.registry,
            &self.receiver,
            bbt_proto::messaging::MessageClass::Conversation,
            format!("seats/{}/inbox", self.receiver.id),
            mode,
        );
    }
    fn send(&mut self, now: u64) -> Ulid {
        self.messaging
            .send(
                self.sender.id,
                1,
                SendRequest::text(
                    format!("seats/{}/inbox", self.receiver.id),
                    "attention fixture",
                ),
                now,
            )
            .unwrap()
            .view()
            .id
    }
    fn receipt(&mut self, id: Ulid) -> bbt_proto::messaging::Receipt {
        self.messaging
            .receipts(self.sender.id, id)
            .unwrap()
            .into_iter()
            .find(|r| r.recipient == self.receiver.id)
            .unwrap()
    }
    fn reopen(&mut self) {
        self.messaging = Messaging::open(
            self.root.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[66; 32]))),
        )
        .unwrap();
    }
    fn ledger_store(&self) -> SqliteStore {
        SqliteStore::open(
            &object_dir(self.root.path(), &format!("swarm-{}", self.sender.swarm)).unwrap(),
        )
        .unwrap()
    }
    fn finish(&mut self, work: &bbt_mux::messaging::ledger::Work, now: u64) {
        self.messaging
            .finish(
                self.receiver.swarm,
                work,
                &WakeOutcome {
                    evidence: Evidence::NativeWritten,
                    native_id: Some("attention-fixture".into()),
                    note: "written once".into(),
                },
                now,
                false,
            )
            .unwrap();
    }
}

// keeps: messaging-attention §3 admission pinning, all three modes and restart.
#[test]
fn admission_pins_wake_and_digest_while_inbox_history_leaves_the_native_scan() {
    use bbt_proto::messaging::{ReceiptWake, WakeMode};
    let mut f = AttentionFixture::new();
    f.mode(WakeMode::Inbox);
    let archived: Vec<_> = (0..32).map(|n| f.send(1_000 + n)).collect();
    f.mode(WakeMode::Wake);
    let immediate = f.send(2_000);
    f.mode(WakeMode::Digest);
    let deferred = f.send(3_000);
    let decision = f.receipt(deferred);
    assert_eq!(
        decision.wake,
        Some(ReceiptWake::Deferred {
            until: "1970-01-01T00:10:03.000Z".into()
        })
    );
    assert_eq!(decision.held, None);
    f.mode(WakeMode::Wake);
    f.reopen();
    for id in archived {
        assert_eq!(f.receipt(id).wake, Some(ReceiptWake::None));
    }
    assert_eq!(f.receipt(deferred).wake, decision.wake);
    let db = f.ledger_store().connection().unwrap();
    let active: i64 = db.query_row("SELECT count(*) FROM msg_deliveries INDEXED BY msg_attention_pending WHERE evidence='recipient-stored' AND (json_extract(CAST(data AS TEXT),'$.wake') IS NOT 'none' OR held IS NOT NULL)", [], |row| row.get(0)).unwrap();
    assert_eq!(active, 2, "inbox history must not enter the native scan");
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 4_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [immediate]
    );
    f.finish(&work, 4_001);
    assert!(
        f.messaging
            .prepare(f.receiver.swarm, f.receiver.id, 602_999)
            .unwrap()
            .is_none()
    );
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 603_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [deferred]
    );
    f.finish(&work, 603_001);
    assert!(f.messaging.ready(603_002).unwrap().is_empty());
}

// keeps: messaging-attention §3 actual thread-root author exception.
#[test]
fn own_thread_reply_wakes_its_root_author_despite_inbox_policy() {
    use bbt_proto::messaging::{ReceiptWake, WakeMode};
    let mut f = AttentionFixture::new();
    f.mode(WakeMode::Inbox);
    let root = f
        .messaging
        .send(
            f.receiver.id,
            1,
            SendRequest::text(
                format!("seats/{}/inbox", f.sender.id),
                "the receiver started this thread",
            ),
            1_000,
        )
        .unwrap();
    let mut reply = SendRequest::text("", "answer to your thread");
    reply.reply_to = Some(root.view().id);
    let reply = f.messaging.send(f.sender.id, 1, reply, 1_001).unwrap();
    assert_eq!(f.receipt(reply.view().id).wake, Some(ReceiptWake::Now));
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 1_002)
        .unwrap()
        .unwrap();
    assert_eq!(work.messages[0].view().id, reply.view().id);
}

// keeps: a sender-supplied thread_id cannot manufacture an always-wake exception.
#[test]
fn thread_id_alias_cannot_claim_another_seats_root_exception() {
    use bbt_proto::messaging::{ReceiptWake, WakeMode};
    let mut f = AttentionFixture::new();
    f.mode(WakeMode::Inbox);
    let root = f
        .messaging
        .send(
            f.receiver.id,
            1,
            SendRequest::text(
                format!("seats/{}/inbox", f.sender.id),
                "a real receiver-owned thread",
            ),
            1_000,
        )
        .unwrap();
    let destination = format!("seats/{}/inbox", f.receiver.id);
    let mut alias = SendRequest::text(&destination, "a different root by the sender");
    alias.thread_id = Some(root.view().id);
    let alias = f.messaging.send(f.sender.id, 1, alias, 2_000).unwrap();
    let mut reply = SendRequest::text(&destination, "still the sender's thread");
    reply.reply_to = Some(alias.view().id);
    let reply = f.messaging.send(f.sender.id, 1, reply, 2_001).unwrap();
    assert_eq!(f.receipt(reply.view().id).wake, Some(ReceiptWake::None));
    f.ledger_store()
        .connection()
        .unwrap()
        .execute("DELETE FROM msg_thread_roots", [])
        .unwrap();
    f.reopen();
    let mut another = SendRequest::text(&destination, "legacy chain still belongs to the sender");
    another.reply_to = Some(reply.view().id);
    let another = f.messaging.send(f.sender.id, 1, another, 2_002).unwrap();
    assert_eq!(f.receipt(another.view().id).wake, Some(ReceiptWake::None));
    assert!(
        f.messaging
            .prepare(f.receiver.swarm, f.receiver.id, 2_002)
            .unwrap()
            .is_none()
    );
}

// keeps: event opt-in is receiver-owned; operator mute remains live.
#[test]
fn opted_in_event_wake_obeys_live_mute_and_carries_structured_content() {
    use bbt_mux::{adapters::State, messaging::tree::Actor};
    use bbt_proto::messaging::{MessageClass, ReceiptWake, WakeMode};
    let mut f = AttentionFixture::new();
    set_attention(
        &f.registry,
        &f.receiver,
        MessageClass::Event,
        format!("swarms/{}", f.receiver.swarm),
        WakeMode::Wake,
    );
    f.messaging
        .set_mute(Actor::Operator, f.receiver.id, true, State::Idle, 1_000)
        .unwrap();
    let notice = f
        .messaging
        .set_mute(Actor::Operator, f.sender.id, true, State::Idle, 2_000)
        .unwrap()
        .unwrap();
    let receipt = f.receipt(notice);
    assert_eq!(receipt.wake, Some(ReceiptWake::Now));
    assert_eq!(receipt.held.as_deref(), Some("recipient muted"));
    assert!(
        f.messaging
            .prepare(f.receiver.swarm, f.receiver.id, 2_001)
            .unwrap()
            .is_none()
    );
    f.messaging
        .set_mute(Actor::Operator, f.receiver.id, false, State::Idle, 3_000)
        .unwrap();
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 3_001)
        .unwrap()
        .unwrap();
    assert!(work.messages.iter().any(|m| m.view().id == notice));
    assert!(
        work.text().contains("\"event\":\"mute\""),
        "structured status must have content when explicitly requested"
    );
    f.finish(&work, 3_002);
    assert!(
        f.messaging
            .prepare(f.receiver.swarm, f.receiver.id, 3_003)
            .unwrap()
            .is_none()
    );
}

// keeps: additive receipt migration never replays a completed native attempt.
#[test]
fn legacy_pending_receipts_migrate_without_replaying_completed_work() {
    use bbt_mux::{adapters::State, messaging::tree::Actor};
    use bbt_proto::messaging::{ReceiptWake, WakeMode};
    let mut f = AttentionFixture::new();
    f.mode(WakeMode::Wake);
    let completed = f.send(1_000);
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 1_001)
        .unwrap()
        .unwrap();
    f.finish(&work, 1_002);
    let pending = f.send(2_000);
    let notice = f
        .messaging
        .set_mute(Actor::Operator, f.sender.id, true, State::Idle, 3_000)
        .unwrap()
        .unwrap();
    let db = f.ledger_store().connection().unwrap();
    // Recreate the rows written before receipt wake existed, including the old
    // metadata-only pseudo-hold. Signed messages and native journal are intact.
    db.execute("UPDATE msg_deliveries SET data=CAST(json_remove(CAST(data AS TEXT),'$.wake','$.until') AS BLOB)", []).unwrap();
    db.execute("UPDATE msg_deliveries SET held='metadata only; no harness wake',data=CAST(json_set(CAST(data AS TEXT),'$.held','metadata only; no harness wake') AS BLOB) WHERE message=(SELECT seq FROM msg_messages WHERE id=?1)", [notice.to_string()]).unwrap();
    f.reopen();
    assert_eq!(f.receipt(completed).evidence, Evidence::NativeWritten);
    assert_eq!(
        f.receipt(completed).wake,
        None,
        "completed legacy evidence is historical"
    );
    assert_eq!(f.receipt(pending).wake, Some(ReceiptWake::Now));
    assert_eq!(f.receipt(notice).wake, Some(ReceiptWake::None));
    assert_eq!(f.receipt(notice).held, None);
    f.messaging
        .set_mute(Actor::Operator, f.sender.id, false, State::Idle, 4_000)
        .unwrap();
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 4_001)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [pending]
    );
    f.finish(&work, 4_002);
    f.reopen();
    assert!(f.messaging.ready(4_003).unwrap().is_empty());
}

// keeps: messaging-attention §3 window boundary on the injected service clock.
#[test]
fn digest_window_delivers_one_summary_at_its_first_deadline() {
    let mut f = AttentionFixture::new();
    let first = f.send(1_000);
    let second = f.send(2_000);
    assert_eq!(
        f.receipt(first).wake,
        f.receipt(second).wake,
        "one window has one deadline"
    );
    f.reopen();
    assert!(f.messaging.ready(600_999).unwrap().is_empty());
    assert!(
        f.messaging
            .prepare(f.receiver.swarm, f.receiver.id, 600_999)
            .unwrap()
            .is_none()
    );
    let next_window = f.send(601_000);
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 601_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [first, second]
    );
    let text = work.text();
    assert_eq!(text.matches("Bboxy message digest").count(), 1);
    assert_eq!(text.matches(&first.to_string()).count(), 1);
    assert_eq!(text.matches(&second.to_string()).count(), 1);
    assert!(text.find(&first.to_string()).unwrap() < text.find(&second.to_string()).unwrap());
    f.finish(&work, 601_001);
    f.reopen();
    assert!(
        f.messaging
            .prepare(f.receiver.swarm, f.receiver.id, 1_200_999)
            .unwrap()
            .is_none()
    );
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 1_201_000)
        .unwrap()
        .unwrap();
    assert_eq!(work.messages[0].view().id, next_window);
    f.finish(&work, 1_201_001);
    assert!(f.messaging.ready(1_201_002).unwrap().is_empty());
}

// keeps: messaging-attention §3 fifty-message cap without advancing wall time.
#[test]
fn digest_cap_flushes_fifty_once_while_the_clock_is_frozen() {
    let mut f = AttentionFixture::new();
    let mut ids: Vec<_> = (0..49).map(|_| f.send(1_000)).collect();
    assert!(
        f.messaging
            .prepare(f.receiver.swarm, f.receiver.id, 1_000)
            .unwrap()
            .is_none()
    );
    ids.push(f.send(1_000));
    f.reopen();
    let next_window = f.send(1_000);
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 1_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        ids
    );
    let text = work.text();
    assert_eq!(text.matches("Bboxy message digest").count(), 1);
    assert!(
        !text.contains("attention fixture"),
        "digest lists stored ids instead of repasting fifty bodies"
    );
    for id in &ids {
        assert_eq!(text.matches(&id.to_string()).count(), 1);
    }
    f.finish(&work, 1_001);
    assert!(
        f.messaging
            .prepare(f.receiver.swarm, f.receiver.id, 601_000 - 1)
            .unwrap()
            .is_none()
    );
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 601_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [next_window]
    );
    f.finish(&work, 601_001);
    f.reopen();
    assert!(f.messaging.ready(601_002).unwrap().is_empty());
}

// keeps: receiver digests preserve per-destination order and independent windows.
#[test]
fn digest_summaries_do_not_merge_destinations() {
    use bbt_mux::messaging::tree::{Actor, Node, Right};
    let mut f = AttentionFixture::new();
    let mut folder = Node::folder("channels/updates");
    folder.grants.insert(f.sender.id, vec![Right::Write]);
    f.messaging
        .tree(f.sender.swarm)
        .unwrap()
        .put(Actor::Operator, folder)
        .unwrap();
    let link = Node::link(
        "channels/updates/receiver",
        format!("seats/{}/inbox", f.receiver.id),
    );
    f.messaging
        .tree(f.sender.swarm)
        .unwrap()
        .put(Actor::Operator, link)
        .unwrap();
    let first = f.send(1_000);
    let channel_first = f
        .messaging
        .send(
            f.sender.id,
            1,
            SendRequest::text("#updates/*", "channel one"),
            1_000,
        )
        .unwrap()
        .view()
        .id;
    let second = f.send(2_000);
    let channel_second = f
        .messaging
        .send(
            f.sender.id,
            1,
            SendRequest::text("#updates/*", "channel two"),
            2_000,
        )
        .unwrap()
        .view()
        .id;
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 602_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [first, second]
    );
    f.finish(&work, 602_001);
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 602_002)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [channel_first, channel_second]
    );
    f.finish(&work, 602_003);
    assert!(f.messaging.ready(602_004).unwrap().is_empty());
}

// keeps: messaging-attention §3 eligible messages never wait on fenced members.
#[test]
fn digest_releases_eligible_members_then_the_fenced_remainder_without_retiming() {
    use bbt_mux::messaging::tree::{Actor, Right};
    let mut f = AttentionFixture::new();
    let peer = SeatRecord::new(Ulid::new(), f.receiver.swarm, "peer", "zsh");
    f.registry.insert(&peer).unwrap();
    let destination = format!("seats/{}/inbox", f.receiver.id);
    f.messaging
        .tree(f.receiver.swarm)
        .unwrap()
        .grant(Actor::Operator, &destination, peer.id, vec![Right::Write])
        .unwrap();
    let eligible = f.send(1_000);
    let held = f
        .messaging
        .send(
            peer.id,
            1,
            SendRequest::text(&destination, "fenced later"),
            2_000,
        )
        .unwrap()
        .view()
        .id;
    let original = f.messaging.receipts(peer.id, held).unwrap()[0].wake.clone();
    f.messaging
        .tree(f.receiver.swarm)
        .unwrap()
        .grant(Actor::Operator, &destination, peer.id, vec![])
        .unwrap();
    f.messaging.ready(601_000).unwrap();
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 601_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [eligible]
    );
    f.finish(&work, 601_001);
    assert!(
        f.messaging
            .prepare(f.receiver.swarm, f.receiver.id, 601_002)
            .unwrap()
            .is_none()
    );
    f.messaging
        .tree(f.receiver.swarm)
        .unwrap()
        .grant(Actor::Operator, &destination, peer.id, vec![Right::Write])
        .unwrap();
    f.messaging.ready(601_003).unwrap();
    assert_eq!(
        f.messaging.receipts(peer.id, held).unwrap()[0].wake,
        original
    );
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 601_003)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [held]
    );
    assert_eq!(work.text().matches("Bboxy message digest").count(), 1);
    f.finish(&work, 601_004);
    f.reopen();
    assert!(f.messaging.ready(601_005).unwrap().is_empty());
    let db = f.ledger_store().connection().unwrap();
    for table in ["msg_digest_members", "msg_digest_windows"] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

// keeps: one credit per summary, with ids ordered across sender priority classes.
#[test]
fn digest_mixed_priorities_keep_id_order_and_cost_one_native_credit() {
    let mut f = AttentionFixture::new();
    let mut oracle = SeatRecord::new(Ulid::new(), f.receiver.swarm, "oracle", "zsh");
    oracle.role.class = bbt_mux::registry::Class::Oracle;
    f.registry.insert(&oracle).unwrap();
    let first = f.send(1_000);
    let middle = f
        .messaging
        .send(
            oracle.id,
            1,
            SendRequest::text(
                format!("seats/{}/inbox", f.receiver.id),
                "oracle-class prose",
            ),
            1_500,
        )
        .unwrap()
        .view()
        .id;
    let last = f.send(2_000);
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 601_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [first, middle, last]
    );
    let db = f.ledger_store().connection().unwrap();
    let (credit, streak): (i64, i64) = db
        .query_row(
            "SELECT credit,streak FROM msg_scheduler WHERE recipient=?1",
            [f.receiver.id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((credit, streak), (29_000, 1));
    f.finish(&work, 601_001);
    assert!(f.messaging.ready(601_002).unwrap().is_empty());
}

// keeps: MA-3 deferred receipts retain their deadlines after the window upgrade.
#[test]
fn legacy_deferred_rows_keep_their_own_deadlines_and_summarize_once() {
    let mut f = AttentionFixture::new();
    let first = f.send(1_000);
    let second = f.send(2_000);
    let db = f.ledger_store().connection().unwrap();
    db.execute("DELETE FROM msg_digest_members", []).unwrap();
    db.execute("DELETE FROM msg_digest_windows", []).unwrap();
    db.execute("UPDATE msg_deliveries SET eligible_ms=602000,data=CAST(json_set(CAST(data AS TEXT),'$.eligible_ms',602000,'$.until','1970-01-01T00:10:02.000Z') AS BLOB) WHERE message=(SELECT seq FROM msg_messages WHERE id=?1)", [second.to_string()]).unwrap();
    f.reopen();
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 601_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [first]
    );
    assert!(work.text().starts_with("Bboxy message digest\n"));
    f.finish(&work, 601_001);
    assert!(
        f.messaging
            .prepare(f.receiver.swarm, f.receiver.id, 601_999)
            .unwrap()
            .is_none()
    );
    let work = f
        .messaging
        .prepare(f.receiver.swarm, f.receiver.id, 602_000)
        .unwrap()
        .unwrap();
    assert_eq!(
        work.messages
            .iter()
            .map(|m| m.view().id)
            .collect::<Vec<_>>(),
        [second]
    );
    assert!(work.text().starts_with("Bboxy message digest\n"));
    f.finish(&work, 602_001);
    assert!(f.messaging.ready(602_002).unwrap().is_empty());
}
