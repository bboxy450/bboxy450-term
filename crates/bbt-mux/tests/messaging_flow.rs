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
