// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Plugin senders (G7 `messages.send`): host-signed, attributed to the
//! plugin, confined to granted destinations, never a seat or the operator.
use super::*;
use crate::messaging::{keys::LocalSeatKeys, wire::IssuerKind};
use ed25519_dalek::SigningKey;

const CANDIDATE: &str = "bboxy450-term-repo_graph-fixture@0123456789abcdef";

struct Fixture {
    _root: tempfile::TempDir,
    messaging: Messaging,
    host: SigningKey,
    seat: SeatRecord,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let account = SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap();
    let seat = SeatRecord::new(Ulid::new(), Ulid::new(), "recipient", "zsh");
    Registry::new(account).insert(&seat).unwrap();
    let keys = Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[3; 32])));
    let messaging = Messaging::open(root.path(), keys).unwrap();
    Fixture {
        _root: root,
        messaging,
        host: SigningKey::from_bytes(&[7; 32]),
        seat,
    }
}

fn sender(granted: &[String]) -> PluginSender {
    PluginSender {
        candidate: CANDIDATE.into(),
        name: "Fixture plugin".into(),
        generation: 2,
        sha256: "a".repeat(64),
        granted: granted.iter().cloned().collect(),
    }
}

impl Fixture {
    fn inbox(&self) -> String {
        format!("seats/{}/inbox", self.seat.id)
    }
    fn send(&mut self, granted: &[String], request: SendRequest) -> Result<StoredMessage> {
        let host = self.host.clone();
        match self
            .messaging
            .send_plugin(&host, &sender(granted), request, 1_000)?
        {
            super::super::api::Response::Stored { message } => Ok(*message),
            other => Err(invalid(format!("not stored: {other:?}"))),
        }
    }
}

// keeps: plugins §3c messages.send — the host signs as the plugin, and the
// envelope, ledger sender and display name it as a plugin.
#[test]
fn a_granted_plugin_message_is_host_signed_and_attributed_to_the_plugin() {
    let mut f = fixture();
    let inbox = f.inbox();
    let message = f
        .send(
            std::slice::from_ref(&inbox),
            SendRequest::text(&inbox, "build finished"),
        )
        .unwrap();
    let envelope = message.envelope().unwrap().clone();
    let host = public(&f.host.verifying_key().to_bytes());
    assert_eq!(envelope.issuer_kind, IssuerKind::Host);
    assert_eq!(
        envelope.issued_by.as_deref(),
        Some(&*format!("plugin:{CANDIDATE}"))
    );
    assert_eq!(envelope.seat, plugin_id(CANDIDATE));
    assert_ne!(envelope.seat, Ulid::nil());
    assert_ne!(envelope.seat, f.seat.id);
    assert_eq!(envelope.process_generation, 0);
    assert_eq!(envelope.role.name, "plugin");
    assert_eq!(envelope.role_epoch, 2);
    assert_eq!(envelope.door, None);
    for key in [
        &envelope.issuer,
        &envelope.account,
        &envelope.device,
        &envelope.author,
    ] {
        assert_eq!(key, &host);
    }
    envelope.verify_signature().unwrap();
    // Verified as a plugin: no seat record, worker class, never operator-direct.
    let (record, class, direct) = f.messaging.validate(&envelope).unwrap();
    assert!(record.is_none());
    assert_eq!(class, Class::Worker);
    assert!(!direct);
    assert_eq!(unseated(&message), Actor::Plugin);
    assert!(!Attention::signed(&envelope).operator);
    // The recipient holds it; the ledger and the display name the plugin.
    let listed = f.messaging.list(f.seat.id, None, None, 0, 16).unwrap();
    assert_eq!(listed.items.len(), 1);
    let swarm = f.seat.swarm;
    assert_eq!(
        f.messaging
            .swarm(swarm)
            .unwrap()
            .ledger
            .sender(envelope.id)
            .unwrap(),
        Recipient::Plugin(plugin_id(CANDIDATE))
    );
    let row = f
        .messaging
        .operator_message_row(swarm, envelope.id)
        .unwrap();
    assert_eq!(row.author.kind, bbt_proto::messaging::AuthorKind::Plugin);
    assert_eq!(row.author.id, format!("plugin:{CANDIDATE}"));
    assert_eq!(row.author.name, "Fixture plugin");
    // Idempotent by message ID.
    let mut again = SendRequest::text(&inbox, "build finished");
    again.id = envelope.id;
    assert_eq!(f.send(&[inbox], again).unwrap().view().id, envelope.id);
}

// keeps: plugins §3c messages.send — destinations are exactly the grant.
#[test]
fn a_plugin_posts_only_to_granted_destinations_as_plain_text() {
    let mut f = fixture();
    let inbox = f.inbox();
    let other = format!("swarms/{}/general", f.seat.swarm);
    for granted in [vec![], vec![other.clone()]] {
        let refused = f
            .send(&granted, SendRequest::text(&inbox, "hello"))
            .unwrap_err()
            .to_string();
        assert!(refused.contains("destination_not_granted"), "{refused}");
    }
    let mut threaded = SendRequest::text(&inbox, "hello");
    threaded.reply_to = Some(Ulid::new());
    let mut mentioning = SendRequest::text(&inbox, "hello");
    mentioning.mentions = vec![f.seat.id];
    for request in [threaded, mentioning] {
        let refused = f
            .send(std::slice::from_ref(&inbox), request)
            .unwrap_err()
            .to_string();
        assert!(refused.contains("invalid_request"), "{refused}");
    }
    assert!(
        f.messaging
            .list(f.seat.id, None, None, 0, 16)
            .unwrap()
            .items
            .is_empty()
    );
}

// keeps: plugins §3c messages.send — a plugin envelope cannot pose as a seat
// or the operator, and no other key can pose as the host.
#[test]
fn plugin_envelopes_cannot_impersonate_or_be_forged() {
    let mut f = fixture();
    let inbox = f.inbox();
    let message = f
        .send(
            std::slice::from_ref(&inbox),
            SendRequest::text(&inbox, "genuine"),
        )
        .unwrap();
    let genuine = message.envelope().unwrap().clone();
    let resign = |envelope: &mut Envelope, key: &SigningKey| {
        let public = public(&key.verifying_key().to_bytes());
        envelope.issuer = public.clone();
        envelope.account = public.clone();
        envelope.device = public.clone();
        envelope.author = public;
        envelope.sign(key)
    };
    // Relabelled attribution breaks the host signature.
    let mut relabelled = genuine.clone();
    relabelled.issued_by = Some("plugin:another".into());
    assert!(f.messaging.validate(&relabelled).is_err());
    // A well-formed plugin envelope signed by a key the host never used.
    let mut stranger = genuine.clone();
    resign(&mut stranger, &SigningKey::from_bytes(&[9; 32])).unwrap();
    let refused = f.messaging.validate(&stranger).unwrap_err().to_string();
    assert!(refused.contains("unknown host key"), "{refused}");
    // Posing as the operator (nil seat) or as a real seat is refused even
    // when re-signed by the genuine host key.
    for seat in [Ulid::nil(), f.seat.id] {
        let mut posing = genuine.clone();
        posing.seat = seat;
        assert!(
            resign(&mut posing, &f.host).is_err() || f.messaging.validate(&posing).is_err(),
            "{seat}"
        );
    }
    // The same bytes under a seat/operator issuer kind no longer verify.
    let mut seat_kind = genuine.clone();
    seat_kind.issuer_kind = IssuerKind::LocalDev;
    assert!(f.messaging.validate(&seat_kind).is_err());
    // A seat or operator envelope can never carry a plugin attribution.
    let mut operator = genuine;
    operator.issuer_kind = IssuerKind::LocalDev;
    operator.seat = Ulid::nil();
    operator.role.name = "operator".into();
    assert!(resign(&mut operator, &f.host).is_err() || f.messaging.validate(&operator).is_err());
}
