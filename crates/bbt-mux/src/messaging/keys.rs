//! Prototype custody seam. This is never presented as a GPG certification chain.
use ed25519_dalek::SigningKey;
use hkdf::Hkdf;
use sha2::Sha256;
use ulid::Ulid;

use super::wire::{Envelope, EnvelopeExt, Issuer, IssuerKind, public};
use crate::store::{Result, invalid};

/// IO-owned device authority for an admitted local operator. Never a wire DTO.
pub(crate) struct OperatorSigning {
    pub device_id: Ulid,
    pub issuer: Issuer,
    pub device: String,
    pub grant: String,
    pub epoch: u64,
    pub key: SigningKey,
    /// The door and issuer this envelope records (decision #269); the window by default.
    pub origin: crate::mcp::Origin,
}

/// Public fields of the persisted ID1d custody record, not IssuerState.
pub(crate) fn local_operator_grant(bytes: &[u8]) -> Result<(String, u64)> {
    #[derive(serde::Deserialize)]
    struct Grant {
        public_key: String,
        epoch: String,
    }
    let grant: Grant = serde_json::from_slice(bytes)?;
    let epoch: u64 = grant.epoch.parse()?;
    if epoch == 0 || epoch.to_string() != grant.epoch {
        return Err(invalid("noncanonical local operator epoch"));
    }
    Ok((grant.public_key, epoch))
}

pub trait SeatKeys: Send + Sync {
    /// Receiver-owned context authenticated by its root certificate and journal.
    fn test_account_label(&self) -> Option<&str> {
        None
    }
    fn issuer(&self) -> Issuer;
    /// A pending custody provider has no issuer to pin. Its public hint from
    /// issuer() is never authority for signing or activation.
    fn ready_issuer(&self) -> Option<Issuer> {
        Some(self.issuer())
    }
    fn signing_hold(&self) -> Option<String> {
        None
    }
    fn leaf(&self, seat: Ulid, generation: u64) -> Result<SigningKey>;
    fn verify(&self, envelope: &Envelope) -> Result<()>;
    /// Sign only after finalize has bound the current account's public context.
    fn sign_envelope(&self, envelope: &mut Envelope, key: &SigningKey) -> Result<()> {
        envelope.sign(key)
    }
    /// The IO owner supplies the current registry record and existing account
    /// transaction. Local-dev keeps its exact v1 bytes; certified providers can
    /// bind the device and journal a signed SeatGrant before envelope signing.
    fn finalize(
        &self,
        _envelope: &mut Envelope,
        _record: &crate::registry::SeatRecord,
        _tx: &rusqlite::Transaction<'_>,
    ) -> Result<()> {
        Ok(())
    }
}
pub struct LocalSeatKeys {
    broker: SigningKey,
}
impl LocalSeatKeys {
    pub fn new(broker: SigningKey) -> Self {
        Self { broker }
    }
}
impl SeatKeys for LocalSeatKeys {
    fn issuer(&self) -> Issuer {
        Issuer {
            kind: IssuerKind::LocalDev,
            key: public(&self.broker.verifying_key().to_bytes()),
        }
    }
    fn leaf(&self, seat: Ulid, generation: u64) -> Result<SigningKey> {
        if seat == Ulid::nil() || generation == 0 {
            return Err(invalid("invalid leaf identity"));
        }
        let hkdf = Hkdf::<Sha256>::new(None, &self.broker.to_bytes());
        let mut info = b"bboxy450/seat-leaf/v1".to_vec();
        info.extend_from_slice(&seat.to_bytes());
        info.extend_from_slice(&generation.to_be_bytes());
        let mut seed = [0; 32];
        hkdf.expand(&info, &mut seed)
            .map_err(|_| invalid("leaf key derivation failed"))?;
        Ok(SigningKey::from_bytes(&seed))
    }
    fn verify(&self, envelope: &Envelope) -> Result<()> {
        let issuer = self.issuer();
        if envelope.seat == Ulid::nil() {
            if envelope.issuer_kind != issuer.kind
                || envelope.issuer != issuer.key
                || envelope.account != issuer.key
                || envelope.device != issuer.key
                || envelope.author != issuer.key
            {
                return Err(invalid("untrusted local operator device key"));
            }
            // The messaging service also checks the persisted issuer digest/epoch.
            return envelope.verify_signature();
        }
        if envelope.issuer_kind != issuer.kind
            || envelope.issuer != issuer.key
            || envelope.account != envelope.issuer
            || envelope.device != envelope.issuer
            || envelope.author
                != public(
                    &self
                        .leaf(envelope.seat, envelope.process_generation)?
                        .verifying_key()
                        .to_bytes(),
                )
        {
            return Err(invalid("untrusted local issuer or seat key"));
        }
        envelope.verify_signature()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_operator_grant_requires_a_key_field_and_canonical_positive_epoch() {
        let bytes = br#"{"public_key":"fixture","epoch":"1","device_id":"retained by ID1d"}"#;
        assert_eq!(local_operator_grant(bytes).unwrap(), ("fixture".into(), 1));
        for epoch in ["0", "01", "+1", "18446744073709551616"] {
            let record = serde_json::json!({"public_key":"fixture","epoch":epoch});
            assert!(local_operator_grant(&serde_json::to_vec(&record).unwrap()).is_err());
        }
        assert!(local_operator_grant(br#"{"epoch":"1"}"#).is_err());
        assert!(local_operator_grant(br#"{"public_key":"fixture","epoch":1}"#).is_err());
    }
}
