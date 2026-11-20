// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

use bboxy450_seat_keys::*;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn public(key: &SigningKey) -> Key {
    Key(key.verifying_key().to_bytes())
}

fn grant(kind: Kind, issuer: &SigningKey, subject: &SigningKey, scopes: &[&str]) -> Grant {
    Grant {
        epoch: 1,
        expires_at: 100,
        id: Ulid::from(1_u128),
        issuer: public(issuer),
        kind,
        name: "fixture".into(),
        not_before: 10,
        parent: None,
        role: None,
        scopes: scopes.iter().map(|scope| (*scope).into()).collect(),
        subject: public(subject),
        swarm: None,
        v: 1,
    }
}

struct Fixture {
    root: SigningKey,
    node: SigningKey,
    roots: RootSet,
    revoked: Revocations,
}

impl Fixture {
    fn new() -> Self {
        let root = key(1);
        Self {
            roots: RootSet::new(public(&root)).unwrap(),
            root,
            node: key(2),
            revoked: Revocations::default(),
        }
    }

    fn node(&self, epoch: u64) -> Signed<Grant> {
        let mut record = grant(
            Kind::Node,
            &self.root,
            &self.node,
            &["issue-seats", "kb-write"],
        );
        record.epoch = epoch;
        issue(record, &self.root).unwrap()
    }

    fn seat(&self, parent: &Signed<Grant>) -> Grant {
        let mut record = grant(Kind::Seat, &self.node, &key(3), &["kb-write"]);
        record.parent = Some(parent.digest().unwrap());
        record.expires_at = 90;
        record.swarm = Some(Ulid::from(2_u128));
        record.role = Some(Role {
            name: "executor".into(),
            class: RoleClass::Worker,
        });
        record
    }

    fn check(&self, chain: &[Signed<Grant>]) -> Result<Authority, Refusal> {
        verify(chain, &self.roots, &self.revoked, 20)
    }

    fn revoke(&self, below_epoch: u64) -> Revoke {
        Revoke {
            below_epoch,
            issued_at: 20,
            issuer: public(&self.root),
            subject: public(&self.node),
            v: 1,
        }
    }
}

fn refusal(link: usize, reason: RefusalReason) -> Result<Authority, Refusal> {
    Err(Refusal { link, reason })
}

#[test]
fn root_node_seat() {
    let f = Fixture::new();
    let node = f.node(1);
    let seat = f.seat(&node);
    let signed = issue(seat.clone(), &f.node).unwrap();
    assert_eq!(
        f.check(&[node, signed]),
        Ok(Authority {
            leaf: seat.subject,
            kind: Kind::Seat,
            id: seat.id,
            scopes: seat.scopes,
            swarm: seat.swarm,
            role: seat.role,
        })
    );
}

#[test]
fn root_client() {
    let f = Fixture::new();
    let client = grant(Kind::Client, &f.root, &key(4), &["view", "answer"]);
    let signed = issue(client.clone(), &f.root).unwrap();
    assert_eq!(
        f.check(&[signed]),
        Ok(Authority {
            leaf: client.subject,
            kind: Kind::Client,
            id: client.id,
            scopes: client.scopes,
            swarm: None,
            role: None,
        })
    );
}

#[test]
fn bad_signature_names_first_failure() {
    let f = Fixture::new();
    let mut node = f.node(1);
    let mut seat = f.seat(&node);
    seat.scopes.push("issue-node".into());
    let child = issue(seat, &f.node).unwrap();
    node.signature = "A".repeat(86);
    assert_eq!(
        f.check(&[node, child]),
        refusal(0, RefusalReason::Signature)
    );
}

#[test]
fn wrong_issuer() {
    let f = Fixture::new();
    let node = f.node(1);
    let wrong_signer = key(5);
    let mut seat = f.seat(&node);
    seat.issuer = public(&wrong_signer);
    let child = issue(seat, &wrong_signer).unwrap();
    assert_eq!(f.check(&[node, child]), refusal(1, RefusalReason::Issuer));
}

#[test]
fn wrong_parent_digest_after_renewal() {
    let f = Fixture::new();
    let child = issue(f.seat(&f.node(1)), &f.node).unwrap();
    assert_eq!(
        f.check(&[f.node(2), child]),
        refusal(1, RefusalReason::Issuer)
    );
}

#[test]
fn scope_escalation() {
    let f = Fixture::new();
    let node = f.node(1);
    let mut seat = f.seat(&node);
    seat.scopes.push("mesh".into());
    let child = issue(seat, &f.node).unwrap();
    assert_eq!(f.check(&[node, child]), refusal(1, RefusalReason::Scope));
}

#[test]
fn a_seat_cannot_gain_root_scope() {
    let f = Fixture::new();
    let node = f.node(1);
    let mut seat = f.seat(&node);
    seat.scopes.push("issue-node".into());
    let child = issue(seat, &f.node).unwrap();
    assert_eq!(f.check(&[node, child]), refusal(1, RefusalReason::Scope));
}

#[test]
fn a_node_needs_issue_seats_to_delegate() {
    let f = Fixture::new();
    let mut record = f.node(1).record;
    record.scopes.retain(|scope| scope != "issue-seats");
    let node = issue(record, &f.root).unwrap();
    let child = issue(f.seat(&node), &f.node).unwrap();
    assert_eq!(f.check(&[node, child]), refusal(1, RefusalReason::Scope));
}

#[test]
fn expiry_beyond_parent() {
    let f = Fixture::new();
    let node = f.node(1);
    let mut seat = f.seat(&node);
    seat.expires_at = node.record.expires_at + 1;
    let child = issue(seat, &f.node).unwrap();
    assert_eq!(f.check(&[node, child]), refusal(1, RefusalReason::Expiry));
}

#[test]
fn validity_window_is_half_open() {
    let f = Fixture::new();
    let node = f.node(1);
    let chain = [node];
    assert_eq!(
        verify(&chain, &f.roots, &f.revoked, 9),
        refusal(0, RefusalReason::Expiry)
    );
    assert!(verify(&chain, &f.roots, &f.revoked, 10).is_ok());
    assert!(verify(&chain, &f.roots, &f.revoked, 99).is_ok());
    assert_eq!(
        verify(&chain, &f.roots, &f.revoked, 100),
        refusal(0, RefusalReason::Expiry)
    );
}

#[test]
fn revoked_parent_stays_revoked_after_epoch_aba() {
    let mut f = Fixture::new();
    let node = f.node(1);
    let child = issue(f.seat(&node), &f.node).unwrap();
    assert!(f.check(&[node.clone(), child.clone()]).is_ok());

    let notice = sign(f.revoke(2), &f.root).unwrap();
    f.revoked.accept(&notice, &f.roots, 20).unwrap();
    let renewed = f.node(2);
    let renewed_child = issue(f.seat(&renewed), &f.node).unwrap();
    assert!(f.check(&[renewed, renewed_child]).is_ok());

    let stale = sign(f.revoke(1), &f.root).unwrap();
    f.revoked.accept(&stale, &f.roots, 20).unwrap();
    f.revoked.accept(&notice, &f.roots, 20).unwrap();
    let mut returned = node.record;
    returned.name = "renewed with old epoch".into();
    let returned = issue(returned, &f.root).unwrap();
    let child = issue(f.seat(&returned), &f.node).unwrap();
    assert_eq!(
        f.check(&[returned, child]),
        refusal(0, RefusalReason::Revoked)
    );
}

#[test]
fn unrelated_signature_does_not_revoke() {
    let mut f = Fixture::new();
    let notice = sign(f.revoke(2), &f.node).unwrap();
    assert_eq!(
        f.revoked.accept(&notice, &f.roots, 20),
        Err(RefusalReason::Signature)
    );
    assert!(f.check(&[f.node(1)]).is_ok());
}

#[test]
fn unknown_root() {
    let f = Fixture::new();
    let unpinned = key(5);
    let client = grant(Kind::Client, &unpinned, &key(4), &["view"]);
    let signed = issue(client, &unpinned).unwrap();
    assert_eq!(f.check(&[signed]), refusal(0, RefusalReason::Root));
}

#[test]
fn too_many_links() {
    let f = Fixture::new();
    let chain = vec![f.node(1); 4];
    assert_eq!(f.check(&chain), refusal(3, RefusalReason::Length));
    assert_eq!(f.check(&[]), refusal(0, RefusalReason::Length));
}

#[test]
fn root_rotation_requires_both_signatures_and_keeps_old_grants() {
    let mut f = Fixture::new();
    let prior_grant = f.node(1);
    let successor = key(6);
    let client = grant(Kind::Client, &successor, &key(4), &["view"]);
    let client = issue(client, &successor).unwrap();
    let rotation = Rotate {
        issued_at: 20,
        new_root: public(&successor),
        old_root: public(&f.root),
        v: 1,
    };
    let old = sign(rotation.clone(), &f.root).unwrap();
    let wrong = sign(rotation.clone(), &key(7)).unwrap();
    let new = sign(rotation, &successor).unwrap();
    assert_eq!(
        f.roots.accept_rotation(&old, &wrong, 20),
        Err(RefusalReason::Signature)
    );
    assert_eq!(
        f.check(std::slice::from_ref(&client)),
        refusal(0, RefusalReason::Root)
    );
    f.roots.accept_rotation(&old, &new, 20).unwrap();
    assert!(f.check(&[client]).is_ok());
    assert!(f.check(std::slice::from_ref(&prior_grant)).is_ok());
    assert_eq!(
        verify(&[prior_grant], &f.roots, &f.revoked, 100),
        refusal(0, RefusalReason::Expiry)
    );
}

#[test]
fn jcs_bytes_and_digest_are_stable() {
    let f = Fixture::new();
    let mut client = grant(Kind::Client, &f.root, &key(4), &["view", "answer"]);
    client.name = "a\u{2028}é\"\0".into();
    let expected = format!(
        concat!(
            "bboxy450/grant/v1\0",
            "{{\"epoch\":1,\"expires_at\":100,\"id\":\"00000000000000000000000001\",",
            "\"issuer\":\"{}\",\"kind\":\"client\",\"name\":\"a\u{2028}é\\\"\\u0000\",",
            "\"not_before\":10,\"parent\":null,\"scopes\":[\"view\",\"answer\"],",
            "\"subject\":\"{}\",\"v\":1}}"
        ),
        hex::encode(client.issuer.0),
        hex::encode(client.subject.0),
    );
    let signed = issue(client, &f.root).unwrap();
    assert_eq!(
        signed.digest().unwrap().0,
        <[u8; 32]>::from(Sha256::digest(expected.as_bytes()))
    );
    assert_eq!(signed.signature, {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        URL_SAFE_NO_PAD.encode(f.root.sign(expected.as_bytes()).to_bytes())
    });

    let wire = serde_json::to_string(&signed).unwrap();
    let decoded: Signed<Grant> = serde_json::from_str(&wire).unwrap();
    assert_eq!(signed, decoded);
    assert!(f.check(&[decoded]).is_ok());
}

#[test]
fn issuing_rejects_wrong_key_version_and_unsafe_integer() {
    let f = Fixture::new();
    let g = f.node(1).record;
    assert_eq!(issue(g.clone(), &f.node), Err(RefusalReason::Signature));
    assert_eq!(
        issue(Grant { v: 2, ..g.clone() }, &f.root),
        Err(RefusalReason::Signature)
    );
    assert_eq!(
        issue(
            Grant {
                epoch: 1 << 53,
                ..g
            },
            &f.root
        ),
        Err(RefusalReason::Signature)
    );
}
