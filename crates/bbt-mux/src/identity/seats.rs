//! Public proofs and private seat authority. No seat secret is serializable.
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::Signed;
use crate::registry::{Location, Role};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatGrant {
    pub version: u8,
    pub root_pubkey: String,
    pub device_grant_id: String,
    pub device_id: Ulid,
    pub device_epoch: String,
    pub seat_id: Ulid,
    pub seat_pubkey: String,
    pub key_epoch: String,
    pub grant_epoch: String,
    pub previous_grant_id: Option<String>,
    pub swarm_id: Ulid,
    pub harness_kind: String,
    pub role: Role,
    pub destinations: Vec<String>,
    pub control_scopes: Vec<String>,
    pub muted: bool,
    #[serde(rename = "where")]
    pub location: Location,
    pub not_before: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatProof {
    pub certificate: String,
    pub process_generation: u64,
    pub grant: Signed<SeatGrant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyStatus {
    Current,
    Retired,
    Revoked,
}

/// This index has no destinations, where/host, cwd, conversation IDs or tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterEntry {
    pub seat_id: Ulid,
    pub seat_pubkey: String,
    pub key_epoch: String,
    pub character_name: String,
    pub harness_kind: String,
    pub role: Role,
    pub valid_from: u64,
    pub valid_until: u64,
    pub status: KeyStatus,
    pub certificate: String,
    pub certificate_fingerprint: String,
    pub grant_reference: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatRoster {
    pub version: u8,
    pub root_pubkey: String,
    pub device_grant_id: String,
    pub device_epoch: String,
    pub roster_sequence: String,
    pub previous_roster_id: Option<String>,
    pub issued_at: u64,
    pub entries: Vec<RosterEntry>,
}
