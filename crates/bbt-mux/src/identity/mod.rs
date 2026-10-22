//! Identity bootstrap, public proofs and seat signing on logic/IO workers.
//! Primary signing remains confined to explicit operator bootstrap operations.
//!
//! Primary secrets stay in gpg-agent. Only public certificates and verified
//! authority records reach the account object; device seeds stay in Keychain.

mod agent;
mod certificate;
mod chain;
mod journal;
mod keychain;
mod seats;
mod transition;

pub use chain::{ChainSeatKeys, seat_keys};
pub use journal::{AccountHead, Bootstrap, BootstrapOutcome, DeviceGrant, DeviceIdentity, Signed};
pub use keychain::{Custody, DeviceKeyStore, KeychainDeviceKeys};
pub use seats::{KeyStatus, RosterEntry, SeatGrant, SeatProof, SeatRoster};
pub use transition::{IssuerState, reset_identity};

use std::fmt;

pub const OPERATOR_ROOT: &str = "9B07E849480C68328CA20684844ECC0587DE7AB0";

/// Safe to display: never contains agent responses, payloads or secret bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PendingReason {
    AgentUnavailable,
    UnsupportedAgent,
    AgentRejected(u32),
    Cancelled,
    TimedOut,
    FingerprintMismatch,
    InvalidProof,
    KeychainUnavailable(i32),
    JournalConflict,
    StorageUnavailable,
}
impl fmt::Display for PendingReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "identity pending: {self:?}")
    }
}
impl std::error::Error for PendingReason {}
pub type Result<T> = std::result::Result<T, PendingReason>;

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(crate) fn proof<T, E>(result: std::result::Result<T, E>) -> Result<T> {
    result.map_err(|_| PendingReason::InvalidProof)
}
