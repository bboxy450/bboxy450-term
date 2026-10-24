//! Identity bootstrap, public proofs and seat signing on logic/IO workers.
//! Primary signing remains confined to explicit operator bootstrap operations.
//!
//! Primary secrets stay in gpg-agent. Only public certificates and verified
//! authority records reach the account object; device seeds stay in Keychain.

mod agent;
mod certificate;
mod chain;
mod custodied;
mod hold;
mod io;
mod journal;
mod keychain;
mod local;
mod seats;
mod status;
mod test_account;
mod transition;

pub use chain::{ChainSeatKeys, seat_keys};
pub use hold::{HoldClosure, HoldWindow};
pub(crate) use hold::{begin_bus_window, seal_bus_window};
pub use journal::{AccountHead, Bootstrap, BootstrapOutcome, DeviceGrant, DeviceIdentity, Signed};
pub use keychain::{Custody, DeviceKeyStore, KeychainDeviceKeys};
pub use local::{LocalIssuerOutcome, local_dev_issuer, local_dev_issuer_with_store};
pub use seats::{KeyStatus, RosterEntry, SeatGrant, SeatProof, SeatRoster};
pub use status::{IdentityRuntime, IdentityState, IdentityStatus};
pub use test_account::TestAccount;
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
    KeychainInteractionRequired,
    KeychainTimedOut,
    JournalConflict,
    StorageUnavailable,
    BootstrapInProgress,
    ActivationRequired,
    DeviceGrantExpired,
    SeatRenewalUnavailable,
    LocalIssuerMissing,
    LocalIssuerMismatch,
    FileCustodyUnavailable,
    TestAccountMismatch,
}
impl PendingReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::AgentUnavailable => "gpg-agent unavailable",
            Self::UnsupportedAgent => "unsupported gpg-agent",
            Self::AgentRejected(_) => "gpg-agent rejected the request",
            Self::Cancelled => "pinentry cancelled",
            Self::TimedOut => "gpg-agent timed out",
            Self::FingerprintMismatch => "primary fingerprint mismatch",
            Self::InvalidProof => "identity proof invalid",
            Self::KeychainUnavailable(_) => "device Keychain unavailable",
            Self::KeychainInteractionRequired => "Keychain interaction required; signing held",
            Self::KeychainTimedOut => "Keychain did not respond within 2 seconds; signing held",
            Self::JournalConflict => "identity journal conflict",
            Self::StorageUnavailable => "identity storage unavailable",
            Self::BootstrapInProgress => "bootstrap has not completed",
            Self::ActivationRequired => "restart logic to activate the certified identity",
            Self::DeviceGrantExpired => "device grant expired; operator renewal required",
            Self::SeatRenewalUnavailable => "seat grant renewal held",
            Self::LocalIssuerMissing => {
                "local-dev issuer key unavailable; use a fresh run directory or explicitly reset identity"
            }
            Self::LocalIssuerMismatch => {
                "local-dev issuer differs from pinned history; explicit identity reset required"
            }
            Self::FileCustodyUnavailable => "selected file custody unavailable; no native fallback",
            Self::TestAccountMismatch => {
                "test account root, run context or private directory mismatch"
            }
        }
    }
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
