//! Operator custody controls carry public status and outcomes, never key material.
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Both actions use the already admitted operator connection, never seat authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityRequest {
    Status,
    Unlock,
}

/// Status with an earlier unlock ID reads its result; a fresh ID reads current status.
/// Unknown authority or key-target fields are refused before dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityCall {
    pub id: Ulid,
    pub request: IdentityRequest,
}

/// A native result is distinct from a missing or interrupted response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnlockOutcome {
    Allowed,
    Cancelled,
    Unknown,
}

/// Public access metadata; unknown is never presented as a verified policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CustodyAcl {
    NotNative,
    TeamSigned,
    Legacy,
    Unknown,
}

/// One pending completion joins to its original audit by operation ID and digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingUnlock {
    pub op: Ulid,
    pub pending_digest: Option<String>,
    pub outcome: UnlockOutcome,
    pub signature: String,
    pub in_flight: bool,
}

/// Native access and reopening its journal are separate results. No secret or
/// native response body is carried in this recovery failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReopenFailure {
    pub reason: String,
}

/// The held view deliberately has no device ID or native credential address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityControl {
    pub custody: Option<String>,
    pub acl: CustodyAcl,
    pub hold: Option<String>,
    pub clear_actions: Vec<String>,
    pub outcome: Option<UnlockOutcome>,
    pub pending: Vec<PendingUnlock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reopen_failed: Option<ReopenFailure>,
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: decision #315
    #[test]
    fn reopen_failure_is_typed_and_absent_from_unchanged_control_responses() {
        let original = serde_json::json!({
            "custody": null, "acl": "unknown", "hold": null,
            "clear_actions": [], "outcome": null, "pending": []
        });
        let mut control: IdentityControl = serde_json::from_value(original.clone()).unwrap();
        assert_eq!(serde_json::to_value(&control).unwrap(), original);
        control.outcome = Some(UnlockOutcome::Allowed);
        control.reopen_failed = Some(ReopenFailure {
            reason: "SQLite DatabaseBusy (5)".into(),
        });
        let value = serde_json::to_value(&control).unwrap();
        assert_eq!(value["outcome"], "allowed");
        assert_eq!(value["reopen_failed"]["reason"], "SQLite DatabaseBusy (5)");
        assert_eq!(
            serde_json::from_value::<IdentityControl>(value).unwrap(),
            control
        );
    }
    // keeps: decision #315
    #[test]
    fn native_control_never_accepts_a_caller_identity_or_key_target() {
        let call = IdentityCall {
            id: Ulid::new(),
            request: IdentityRequest::Unlock,
        };
        for field in ["device_id", "key", "authority", "signature", "capability"] {
            let mut value = serde_json::to_value(&call).unwrap();
            value[field] = serde_json::json!("caller supplied");
            assert!(
                serde_json::from_value::<IdentityCall>(value).is_err(),
                "{field}"
            );
        }
        assert_eq!(
            serde_json::from_value::<IdentityCall>(serde_json::to_value(&call).unwrap()).unwrap(),
            call
        );
    }
}
