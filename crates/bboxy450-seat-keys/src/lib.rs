// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Offline verification of seat, node and client authority. Callers supply time,
//! pinned trust and observed revocations, and keep custody of signing keys.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::VerifyingKey;
pub use ed25519_dalek::{Signature, Signer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub use ulid::Ulid;

const MAX_CHAIN: usize = 3;
const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;
const DAY: u64 = 24 * 60 * 60;

/// A raw Ed25519 public key. Its JSON representation is lowercase hex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Key(#[serde(with = "hex32")] pub [u8; 32]);

/// The SHA-256 of a record's domain-prefixed signing bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecordDigest(#[serde(with = "hex32")] pub [u8; 32]);

mod ulid_wire {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
    use ulid::Ulid;

    // ulid's serde feature needs its std feature, which also enables OS RNG.
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    struct Wire(#[serde(with = "self")] Ulid);

    pub(super) fn serialize<S: Serializer>(id: &Ulid, writer: S) -> Result<S::Ok, S::Error> {
        writer.collect_str(id)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(reader: D) -> Result<Ulid, D::Error> {
        let encoded = String::deserialize(reader)?;
        let id = Ulid::from_string(&encoded).map_err(D::Error::custom)?;
        if id.to_string() != encoded {
            return Err(D::Error::custom("expected a canonical ULID"));
        }
        Ok(id)
    }

    pub(super) mod optional {
        use super::*;

        pub(crate) fn serialize<S: Serializer>(
            id: &Option<Ulid>,
            writer: S,
        ) -> Result<S::Ok, S::Error> {
            id.map(Wire).serialize(writer)
        }

        pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
            reader: D,
        ) -> Result<Option<Ulid>, D::Error> {
            Ok(Option::<Wire>::deserialize(reader)?.map(|wire| wire.0))
        }
    }
}

mod hex32 {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub(super) fn serialize<S: Serializer>(raw: &[u8; 32], writer: S) -> Result<S::Ok, S::Error> {
        writer.serialize_str(&hex::encode(raw))
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(reader: D) -> Result<[u8; 32], D::Error> {
        let encoded = String::deserialize(reader)?;
        let mut raw = [0; 32];
        hex::decode_to_slice(&encoded, &mut raw).map_err(D::Error::custom)?;
        if encoded.bytes().any(|byte| byte.is_ascii_uppercase()) {
            return Err(D::Error::custom("expected lowercase hex"));
        }
        Ok(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Node,
    Client,
    Seat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoleClass {
    Oracle,
    Worker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    pub class: RoleClass,
    pub name: String,
}

// Keep serialized fields in JCS order, including nested records. These fixed
// schemas contain only ASCII property names, strings, safe unsigned integers,
// arrays and null; serde_json can emit their canonical form directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub epoch: u64,
    pub expires_at: u64,
    #[serde(with = "ulid_wire")]
    pub id: Ulid,
    pub issuer: Key,
    pub kind: Kind,
    pub name: String,
    pub not_before: u64,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    pub parent: Option<RecordDigest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    pub scopes: Vec<String>,
    pub subject: Key,
    #[serde(
        default,
        with = "ulid_wire::optional",
        skip_serializing_if = "Option::is_none"
    )]
    pub swarm: Option<Ulid>,
    pub v: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revoke {
    pub below_epoch: u64,
    pub issued_at: u64,
    pub issuer: Key,
    pub subject: Key,
    pub v: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rotate {
    pub issued_at: u64,
    pub new_root: Key,
    pub old_root: Key,
    pub v: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signed<T> {
    pub record: T,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authority {
    pub leaf: Key,
    pub kind: Kind,
    pub id: Ulid,
    pub scopes: Vec<String>,
    pub swarm: Option<Ulid>,
    pub role: Option<Role>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    Signature,
    Issuer,
    Scope,
    Expiry,
    Revoked,
    Root,
    Length,
}

impl fmt::Display for RefusalReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Signature => "signature",
            Self::Issuer => "issuer",
            Self::Scope => "scope",
            Self::Expiry => "expiry",
            Self::Revoked => "revoked",
            Self::Root => "root",
            Self::Length => "length",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refusal {
    /// Zero-based index in the supplied root-to-leaf grant slice.
    pub link: usize,
    pub reason: RefusalReason,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "grant link {}: {}", self.link, self.reason)
    }
}

impl std::error::Error for Refusal {}

// Inspired by bboxy450-term's fixed-schema signing in
// https://github.com/bboxy450/bboxy450-term/blob/main/crates/bbt-mux/src/identity/journal.rs
// The field order above removes the need to build and recursively sort a JSON value.
mod record {
    use super::*;

    pub trait Sealed: Serialize {
        const DOMAIN: &[u8];
        fn v(&self) -> u8;
        fn safe_numbers(&self) -> bool;
    }

    impl Sealed for Grant {
        const DOMAIN: &[u8] = b"bboxy450/grant/v1\0";
        fn v(&self) -> u8 {
            self.v
        }
        fn safe_numbers(&self) -> bool {
            [self.epoch, self.not_before, self.expires_at]
                .into_iter()
                .all(|n| n <= MAX_SAFE_INTEGER)
        }
    }

    impl Sealed for Revoke {
        const DOMAIN: &[u8] = b"bboxy450/revoke/v1\0";
        fn v(&self) -> u8 {
            self.v
        }
        fn safe_numbers(&self) -> bool {
            self.below_epoch <= MAX_SAFE_INTEGER && self.issued_at <= MAX_SAFE_INTEGER
        }
    }

    impl Sealed for Rotate {
        const DOMAIN: &[u8] = b"bboxy450/rotate/v1\0";
        fn v(&self) -> u8 {
            self.v
        }
        fn safe_numbers(&self) -> bool {
            self.issued_at <= MAX_SAFE_INTEGER
        }
    }
}

fn signing_bytes<T: record::Sealed>(record: &T) -> Result<Vec<u8>, RefusalReason> {
    if record.v() != 1 || !record.safe_numbers() {
        return Err(RefusalReason::Signature);
    }
    let mut bytes = T::DOMAIN.to_vec();
    serde_json::to_writer(&mut bytes, record).map_err(|_| RefusalReason::Signature)?;
    Ok(bytes)
}

fn verifying_key(key: Key) -> Result<VerifyingKey, RefusalReason> {
    let public = VerifyingKey::from_bytes(&key.0).map_err(|_| RefusalReason::Signature)?;
    if public.is_weak() {
        return Err(RefusalReason::Signature);
    }
    Ok(public)
}

fn verify_signature(key: Key, bytes: &[u8], encoded: &str) -> Result<(), RefusalReason> {
    // A 64-byte detached signature has exactly 86 unpadded base64url characters.
    if encoded.len() != 86 {
        return Err(RefusalReason::Signature);
    }
    let mut raw = [0; 64];
    URL_SAFE_NO_PAD
        .decode_slice(encoded, &mut raw)
        .map_err(|_| RefusalReason::Signature)?;
    verifying_key(key)?
        .verify_strict(bytes, &Signature::from_bytes(&raw))
        .map_err(|_| RefusalReason::Signature)
}

impl<T: record::Sealed> Signed<T> {
    pub fn digest(&self) -> Result<RecordDigest, RefusalReason> {
        Ok(RecordDigest(
            Sha256::digest(signing_bytes(&self.record)?).into(),
        ))
    }
}

/// Sign one record using an Ed25519 signing capability supplied by the caller.
pub fn sign<T: record::Sealed>(
    record: T,
    signer: &dyn Signer<Signature>,
) -> Result<Signed<T>, RefusalReason> {
    let bytes = signing_bytes(&record)?;
    let signature = signer
        .try_sign(&bytes)
        .map_err(|_| RefusalReason::Signature)?;
    Ok(Signed {
        record,
        signature: URL_SAFE_NO_PAD.encode(signature.to_bytes()),
    })
}

/// Issue a grant and check that the supplied signer owns the declared issuer.
pub fn issue(grant: Grant, signer: &dyn Signer<Signature>) -> Result<Signed<Grant>, RefusalReason> {
    let signed = sign(grant, signer)?;
    let bytes = signing_bytes(&signed.record)?;
    verify_signature(signed.record.issuer, &bytes, &signed.signature)?;
    Ok(signed)
}

/// A pinned root and its cryptographically authorized successors.
#[derive(Debug, Clone)]
pub struct RootSet {
    active: Key,
    retired: BTreeSet<Key>,
}

impl RootSet {
    pub fn new(pinned: Key) -> Result<Self, RefusalReason> {
        verifying_key(pinned).map_err(|_| RefusalReason::Root)?;
        Ok(Self {
            active: pinned,
            retired: BTreeSet::new(),
        })
    }

    fn contains(&self, key: Key) -> bool {
        self.active == key || self.retired.contains(&key)
    }

    /// Install a caller-attested successor after checking both signatures over
    /// the same rotation record. Apply rotations in order from the pinned root.
    /// Previous roots remain valid anchors for their expiring grants.
    pub fn accept_rotation(
        &mut self,
        old: &Signed<Rotate>,
        new: &Signed<Rotate>,
        now: u64,
    ) -> Result<(), RefusalReason> {
        let rotation = &old.record;
        if rotation.old_root != self.active || self.contains(rotation.new_root) {
            return Err(RefusalReason::Root);
        }
        if rotation.issued_at > now {
            return Err(RefusalReason::Expiry);
        }
        if &new.record != rotation {
            return Err(RefusalReason::Signature);
        }
        let bytes = signing_bytes(rotation)?;
        verify_signature(rotation.old_root, &bytes, &old.signature)?;
        verify_signature(rotation.new_root, &bytes, &new.signature)?;
        self.retired.insert(self.active);
        self.active = rotation.new_root;
        Ok(())
    }
}

/// Monotone epoch floors learned from authenticated root revocations.
#[derive(Debug, Default)]
pub struct Revocations(BTreeMap<Key, u64>);

impl Revocations {
    pub fn accept(
        &mut self,
        signed: &Signed<Revoke>,
        roots: &RootSet,
        now: u64,
    ) -> Result<(), RefusalReason> {
        let revoke = &signed.record;
        if !roots.contains(revoke.issuer) {
            return Err(RefusalReason::Root);
        }
        if revoke.issued_at > now {
            return Err(RefusalReason::Expiry);
        }
        verify_signature(revoke.issuer, &signing_bytes(revoke)?, &signed.signature)?;
        let floor = self.0.entry(revoke.subject).or_default();
        *floor = (*floor).max(revoke.below_epoch);
        Ok(())
    }

    fn refuses(&self, grant: &Grant) -> bool {
        self.0
            .get(&grant.subject)
            .is_some_and(|floor| grant.epoch < *floor)
    }
}

/// Verify grants ordered from the issuer nearest the pinned root to the leaf.
/// The trust anchor itself is not a grant in the slice.
pub fn verify(
    chain: &[Signed<Grant>],
    root: &RootSet,
    revoked: &Revocations,
    now: u64,
) -> Result<Authority, Refusal> {
    use RefusalReason::*;
    let Some(leaf) = chain.last() else {
        return Err(Refusal {
            link: 0,
            reason: Length,
        });
    };
    if chain.len() > MAX_CHAIN {
        return Err(Refusal {
            link: MAX_CHAIN,
            reason: Length,
        });
    }

    let mut parent: Option<(&Grant, RecordDigest)> = None;
    for (link, signed) in chain.iter().enumerate() {
        let grant = &signed.record;
        let refuse = |reason| Refusal { link, reason };
        if let Some((previous, digest)) = parent {
            if previous.kind != Kind::Node
                || grant.kind != Kind::Seat
                || previous.subject != grant.issuer
                || grant.parent != Some(digest)
            {
                return Err(refuse(Issuer));
            }
        } else {
            if !root.contains(grant.issuer) {
                return Err(refuse(Root));
            }
            if grant.parent.is_some() || grant.kind == Kind::Seat {
                return Err(refuse(Issuer));
            }
        }

        let bytes = signing_bytes(grant).map_err(refuse)?;
        verify_signature(grant.issuer, &bytes, &signed.signature).map_err(refuse)?;
        verifying_key(grant.subject).map_err(refuse)?;

        if let Some((previous, _)) = parent {
            if !previous.scopes.iter().any(|scope| scope == "issue-seats")
                || grant
                    .scopes
                    .iter()
                    .any(|scope| scope == "issue-seats" || !previous.scopes.contains(scope))
            {
                return Err(refuse(Scope));
            }
            if grant.expires_at > previous.expires_at {
                return Err(refuse(Expiry));
            }
        } else if grant.scopes.iter().any(|scope| match grant.kind {
            Kind::Node => matches!(scope.as_str(), "issue-node" | "issue-client" | "revoke"),
            Kind::Client => !matches!(scope.as_str(), "view" | "answer" | "control"),
            Kind::Seat => unreachable!("seat chains have a checked parent"),
        }) {
            return Err(refuse(Scope));
        }
        let max_age = match grant.kind {
            Kind::Node | Kind::Client => 365 * DAY,
            Kind::Seat => 7 * DAY,
        };
        if now < grant.not_before
            || now >= grant.expires_at
            || grant.expires_at - grant.not_before > max_age
        {
            return Err(refuse(Expiry));
        }
        if revoked.refuses(grant) {
            return Err(refuse(Revoked));
        }
        parent = Some((grant, RecordDigest(Sha256::digest(bytes).into())));
    }

    let grant = &leaf.record;
    Ok(Authority {
        leaf: grant.subject,
        kind: grant.kind,
        id: grant.id,
        scopes: grant.scopes.clone(),
        swarm: grant.swarm,
        role: grant.role.clone(),
    })
}
