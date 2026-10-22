//! Prototype custody seam. This is never presented as a GPG certification chain.
use ed25519_dalek::SigningKey;
use hkdf::Hkdf;
use sha2::Sha256;
use ulid::Ulid;

use super::wire::{Envelope, Issuer, IssuerKind, public};
use crate::store::{Result, invalid};

pub trait SeatKeys: Send + Sync {
    fn issuer(&self) -> Issuer;
    fn leaf(&self, seat: Ulid, generation: u64) -> Result<SigningKey>;
    fn verify(&self, envelope: &Envelope) -> Result<()>;
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
