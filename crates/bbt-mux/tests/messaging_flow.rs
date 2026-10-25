//! A custody hold changes message evidence, never same-device delivery.
use std::sync::Arc;

use bbt_mux::{
    Ulid,
    messaging::{
        Messaging,
        api::{Request, Response},
        keys::SeatKeys,
        wire::{Envelope, Issuer, IssuerKind, SendRequest},
    },
    registry::{Registry, SeatRecord},
    store::{Result, invalid, object_dir, sqlite::SqliteStore},
};
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
}
