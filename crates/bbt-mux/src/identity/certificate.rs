use std::{cell::Cell, fmt};

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use pgp::{
    composed::{Deserializable, SignedKeyDetails, SignedPublicKey},
    crypto::{hash::HashAlgorithm, public_key::PublicKeyAlgorithm},
    packet::{
        KeyFlags, PubKeyInner, PublicKey, SignatureConfig, SignatureType, Subpacket, SubpacketData,
        UserId,
    },
    ser::Serialize,
    types::{
        EddsaLegacyPublicParams, Fingerprint, KeyDetails, KeyId, KeyVersion, Mpi,
        PacketHeaderVersion, Password, PublicParams, SignatureBytes, SignedUser, Tag, Timestamp,
    },
};

use super::{
    PendingReason as E, Result,
    agent::{Input, RootSigner},
    proof,
};

pub(crate) struct Root {
    pub certificate: SignedPublicKey,
    pub public: VerifyingKey,
    pub fingerprint: String,
    pub valid_until: Option<u64>,
}
impl Root {
    pub fn parse(bytes: &[u8], expected: &str, now: u64) -> Result<Self> {
        if expected.len() != 40 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(E::FingerprintMismatch);
        }
        let certificate = parse(bytes)?;
        let fingerprint = certificate.fingerprint().to_string().to_uppercase();
        if fingerprint != expected {
            return Err(E::FingerprintMismatch);
        }
        if certificate.version() != KeyVersion::V4
            || !certificate.details.revocation_signatures.is_empty()
        {
            return Err(E::InvalidProof);
        }
        let public = raw(&certificate.primary_key)?;
        let primary = &certificate.primary_key;
        let self_sig = certificate
            .details
            .users
            .iter()
            .flat_map(|user| {
                user.signatures.iter().filter(move |sig| {
                    sig.is_certification()
                        && allowed_hash(sig.hash_alg())
                        && sig
                            .verify_certification(primary, Tag::UserId, &user.id)
                            .is_ok()
                })
            })
            .max_by_key(|sig| sig.created())
            .ok_or(E::InvalidProof)?;
        let created = u64::from(self_sig.created().ok_or(E::InvalidProof)?.as_secs());
        let key_created = u64::from(certificate.created_at().as_secs());
        let flags = self_sig.key_flags();
        let valid_until = self_sig
            .signature_expiration_time()
            .map(|d| created + u64::from(d.as_secs()))
            .into_iter()
            .chain(
                self_sig
                    .key_expiration_time()
                    .map(|d| key_created + u64::from(d.as_secs())),
            )
            .min();
        if created > now
            || key_created > now
            || !flags.certify()
            || !flags.sign()
            || self_sig
                .signature_expiration_time()
                .is_some_and(|d| created + u64::from(d.as_secs()) <= now)
            || self_sig
                .key_expiration_time()
                .is_some_and(|d| key_created + u64::from(d.as_secs()) <= now)
        {
            return Err(E::InvalidProof);
        }
        Ok(Self {
            certificate,
            public,
            fingerprint,
            valid_until,
        })
    }

    pub fn certify(
        &self,
        agent: &impl RootSigner,
        key: &SigningKey,
        device: ulid::Ulid,
        created: u32,
    ) -> Result<Vec<u8>> {
        let public = PublicKey::from_inner(proof(PubKeyInner::new(
            KeyVersion::V4,
            PublicKeyAlgorithm::EdDSALegacy,
            Timestamp::from_secs(created),
            None,
            PublicParams::EdDSALegacy(EddsaLegacyPublicParams::Ed25519 {
                key: proof(key.verifying_key().as_bytes().as_slice().try_into())?,
            }),
        ))?);
        let public = proof(public)?;
        let uid = proof(UserId::from_str(
            PacketHeaderVersion::New,
            format!("bboxy450-term device {device}"),
        ))?;
        let local = Adapter::new(public.clone(), |digest: &[u8]| {
            Ok(key.sign(digest).to_bytes())
        });
        let mut own = config(&local, created)?;
        own.hashed_subpackets
            .push(proof(Subpacket::regular(SubpacketData::KeyFlags(proof(
                KeyFlags::try_from_reader([3u8].as_slice()),
            )?)))?);
        let self_sig = proof(own.sign_certification_third_party(
            &local,
            &Password::empty(),
            &public,
            Tag::UserId,
            &uid,
        ))?;
        let external = Adapter::new(self.certificate.primary_key.clone(), |digest: &[u8]| {
            agent.sign_root(Input::Sha256(digest))
        });
        let cert = config(&external, created)?
            .sign_certification_third_party(
                &external,
                &Password::empty(),
                &public,
                Tag::UserId,
                &uid,
            )
            .map_err(|_| external.failure.get().unwrap_or(E::InvalidProof))?;
        proof(self_sig.verify_certification(&public, Tag::UserId, &uid))?;
        proof(cert.verify_third_party_certification(
            &public,
            &self.certificate.primary_key,
            Tag::UserId,
            &uid,
        ))?;
        let certificate = SignedPublicKey::new(
            public,
            SignedKeyDetails::new(
                vec![],
                vec![],
                vec![SignedUser::new(uid, vec![self_sig, cert])],
                vec![],
            ),
            vec![],
        );
        proof(certificate.to_bytes())
    }
}

/// A disposable root uses the same OpenPGP packet and self-certification rules.
pub(crate) fn test_root(key: &SigningKey, created: u32, label: &str) -> Result<(Vec<u8>, String)> {
    let public = proof(PublicKey::from_inner(proof(PubKeyInner::new(
        KeyVersion::V4,
        PublicKeyAlgorithm::EdDSALegacy,
        Timestamp::from_secs(created),
        None,
        PublicParams::EdDSALegacy(EddsaLegacyPublicParams::Ed25519 {
            key: proof(key.verifying_key().as_bytes().as_slice().try_into())?,
        }),
    ))?))?;
    let uid = proof(UserId::from_str(PacketHeaderVersion::New, label))?;
    let signer = Adapter::new(public.clone(), |digest: &[u8]| {
        Ok(key.sign(digest).to_bytes())
    });
    let mut config = config(&signer, created)?;
    config
        .hashed_subpackets
        .push(proof(Subpacket::regular(SubpacketData::KeyFlags(proof(
            KeyFlags::try_from_reader([3u8].as_slice()),
        )?)))?);
    let signature = proof(config.sign_certification_third_party(
        &signer,
        &Password::empty(),
        &public,
        Tag::UserId,
        &uid,
    ))?;
    proof(signature.verify_certification(&public, Tag::UserId, &uid))?;
    let certificate = SignedPublicKey::new(
        public,
        SignedKeyDetails::new(
            vec![],
            vec![],
            vec![SignedUser::new(uid, vec![signature])],
            vec![],
        ),
        vec![],
    );
    let fingerprint = certificate.fingerprint().to_string().to_uppercase();
    let bytes = proof(certificate.to_bytes())?;
    Root::parse(&bytes, &fingerprint, u64::from(created))?;
    Ok((bytes, fingerprint))
}

pub(crate) fn parse(bytes: &[u8]) -> Result<SignedPublicKey> {
    if bytes.is_empty() || bytes.len() > 64 * 1024 {
        return Err(E::InvalidProof);
    }
    // Do not use the convenience parser that filters malformed packets away.
    let packets = pgp::packet::PacketParser::new(bytes)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| E::InvalidProof)?;
    if packets.iter().any(|p| {
        !matches!(
            p,
            pgp::packet::Packet::PublicKey(_)
                | pgp::packet::Packet::Signature(_)
                | pgp::packet::Packet::UserId(_)
        )
    }) {
        return Err(E::InvalidProof);
    }
    let mut keys = SignedPublicKey::from_packets(packets.into_iter().map(Ok).peekable());
    let key = proof(keys.next().ok_or(E::InvalidProof)?)?;
    if keys.next().is_some() {
        return Err(E::InvalidProof);
    }
    Ok(key)
}
pub(crate) fn raw(key: &PublicKey) -> Result<VerifyingKey> {
    match key.public_params() {
        PublicParams::EdDSALegacy(EddsaLegacyPublicParams::Ed25519 { key }) => {
            proof(VerifyingKey::from_bytes(key.as_bytes()))
        }
        _ => Err(E::InvalidProof),
    }
}
pub(crate) fn verify_device(root: &Root, bytes: &[u8], expected: &[u8; 32]) -> Result<()> {
    let cert = parse(bytes)?;
    if raw(&cert.primary_key)?.to_bytes() != *expected || cert.details.users.len() != 1 {
        return Err(E::InvalidProof);
    }
    let user = &cert.details.users[0];
    let own = user.signatures.iter().any(|s| {
        s.is_certification()
            && allowed_hash(s.hash_alg())
            && s.verify_certification(&cert.primary_key, Tag::UserId, &user.id)
                .is_ok()
    });
    let issuer = user.signatures.iter().any(|s| {
        s.is_certification()
            && allowed_hash(s.hash_alg())
            && s.verify_third_party_certification(
                &cert.primary_key,
                &root.certificate.primary_key,
                Tag::UserId,
                &user.id,
            )
            .is_ok()
    });
    if !own || !issuer {
        return Err(E::InvalidProof);
    }
    Ok(())
}

/// Certify a fresh, in-memory seat key without involving the primary agent.
pub(crate) fn certify_seat(
    device: &[u8],
    device_key: &SigningKey,
    seat_key: &SigningKey,
    seat: ulid::Ulid,
    key_epoch: u64,
    created: u32,
) -> Result<Vec<u8>> {
    let device = parse(device)?;
    if raw(&device.primary_key)? != device_key.verifying_key() {
        return Err(E::InvalidProof);
    }
    let public = proof(PublicKey::from_inner(proof(PubKeyInner::new(
        KeyVersion::V4,
        PublicKeyAlgorithm::EdDSALegacy,
        Timestamp::from_secs(created),
        None,
        PublicParams::EdDSALegacy(EddsaLegacyPublicParams::Ed25519 {
            key: proof(seat_key.verifying_key().as_bytes().as_slice().try_into())?,
        }),
    ))?))?;
    let uid = proof(UserId::from_str(
        PacketHeaderVersion::New,
        format!("bboxy450-term seat {seat} key {key_epoch}"),
    ))?;
    let own = Adapter::new(public.clone(), |digest: &[u8]| {
        Ok(seat_key.sign(digest).to_bytes())
    });
    let issuer = Adapter::new(device.primary_key.clone(), |digest: &[u8]| {
        Ok(device_key.sign(digest).to_bytes())
    });
    let own_sig = proof(config(&own, created)?.sign_certification_third_party(
        &own,
        &Password::empty(),
        &public,
        Tag::UserId,
        &uid,
    ))?;
    let issuer_sig = proof(config(&issuer, created)?.sign_certification_third_party(
        &issuer,
        &Password::empty(),
        &public,
        Tag::UserId,
        &uid,
    ))?;
    proof(own_sig.verify_certification(&public, Tag::UserId, &uid))?;
    proof(issuer_sig.verify_third_party_certification(
        &public,
        &device.primary_key,
        Tag::UserId,
        &uid,
    ))?;
    proof(
        SignedPublicKey::new(
            public,
            SignedKeyDetails::new(
                vec![],
                vec![],
                vec![SignedUser::new(uid, vec![own_sig, issuer_sig])],
                vec![],
            ),
            vec![],
        )
        .to_bytes(),
    )
}

pub(crate) fn verify_seat(
    device: &[u8],
    certificate: &[u8],
    expected: &[u8; 32],
    seat: ulid::Ulid,
    key_epoch: u64,
    now: u64,
) -> Result<()> {
    let device = parse(device)?;
    let cert = parse(certificate)?;
    if raw(&cert.primary_key)?.to_bytes() != *expected
        || cert.details.users.len() != 1
        || !cert.details.revocation_signatures.is_empty()
    {
        return Err(E::InvalidProof);
    }
    let user = &cert.details.users[0];
    if user.id.id() != format!("bboxy450-term seat {seat} key {key_epoch}").as_bytes() {
        return Err(E::InvalidProof);
    }
    let current = |s: &pgp::packet::Signature| {
        s.is_certification()
            && allowed_hash(s.hash_alg())
            && s.created().is_some_and(|t| u64::from(t.as_secs()) <= now)
    };
    if !user.signatures.iter().any(|s| {
        current(s)
            && s.verify_certification(&cert.primary_key, Tag::UserId, &user.id)
                .is_ok()
    }) || !user.signatures.iter().any(|s| {
        current(s)
            && s.verify_third_party_certification(
                &cert.primary_key,
                &device.primary_key,
                Tag::UserId,
                &user.id,
            )
            .is_ok()
    }) {
        return Err(E::InvalidProof);
    }
    Ok(())
}
fn allowed_hash(hash: Option<HashAlgorithm>) -> bool {
    matches!(
        hash,
        Some(HashAlgorithm::Sha256 | HashAlgorithm::Sha384 | HashAlgorithm::Sha512)
    )
}
fn config(signer: &impl KeyDetails, time: u32) -> Result<SignatureConfig> {
    let mut config = SignatureConfig::v4(
        SignatureType::CertPositive,
        signer.algorithm(),
        HashAlgorithm::Sha256,
    );
    config.hashed_subpackets = vec![
        proof(Subpacket::regular(SubpacketData::SignatureCreationTime(
            Timestamp::from_secs(time),
        )))?,
        proof(Subpacket::regular(SubpacketData::IssuerFingerprint(
            signer.fingerprint(),
        )))?,
    ];
    config.unhashed_subpackets = vec![proof(Subpacket::regular(SubpacketData::IssuerKeyId(
        signer.legacy_key_id(),
    )))?];
    Ok(config)
}
struct Adapter<F> {
    public: PublicKey,
    sign: F,
    failure: Cell<Option<E>>,
}
impl<F> Adapter<F> {
    fn new(public: PublicKey, sign: F) -> Self {
        Self {
            public,
            sign,
            failure: Cell::new(None),
        }
    }
}
impl<F> fmt::Debug for Adapter<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CertificateSigner")
            .field("fingerprint", &self.public.fingerprint())
            .finish()
    }
}
impl<F> KeyDetails for Adapter<F> {
    fn version(&self) -> KeyVersion {
        self.public.version()
    }
    fn legacy_key_id(&self) -> KeyId {
        self.public.legacy_key_id()
    }
    fn fingerprint(&self) -> Fingerprint {
        self.public.fingerprint()
    }
    fn algorithm(&self) -> PublicKeyAlgorithm {
        self.public.algorithm()
    }
    fn created_at(&self) -> Timestamp {
        self.public.created_at()
    }
    fn legacy_v3_expiration_days(&self) -> Option<u16> {
        None
    }
    fn public_params(&self) -> &PublicParams {
        self.public.public_params()
    }
}
impl<F: Fn(&[u8]) -> Result<[u8; 64]>> pgp::types::SigningKey for Adapter<F> {
    fn sign(
        &self,
        _: &Password,
        hash: HashAlgorithm,
        data: &[u8],
    ) -> pgp::errors::Result<SignatureBytes> {
        if hash != HashAlgorithm::Sha256 || data.len() != 32 {
            return Err(std::io::Error::other("unsupported certification digest").into());
        }
        let sig = (self.sign)(data).map_err(|reason| {
            self.failure.set(Some(reason));
            pgp::errors::Error::from(std::io::Error::other(reason))
        })?;
        Ok(SignatureBytes::Mpis(vec![
            Mpi::from_slice(&sig[..32]),
            Mpi::from_slice(&sig[32..]),
        ]))
    }
    fn hash_alg(&self) -> HashAlgorithm {
        HashAlgorithm::Sha256
    }
}
