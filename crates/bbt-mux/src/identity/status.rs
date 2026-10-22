//! Public identity-card data. Reading it never asks the primary to sign.
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::{
    AccountHead, ChainSeatKeys, Custody, DeviceIdentity, PendingReason, Signed, transition,
};
use crate::{
    messaging::{keys::SeatKeys, wire::IssuerKind},
    registry::{Lifecycle, Registry},
    store::{Result, Store, invalid, sqlite::SqliteStore},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityState {
    LocalDev,
    Pending,
    Certified,
    Held,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityStatus {
    pub state: IdentityState,
    pub label: String,
    pub issuer_kind: IssuerKind,
    pub bootstrapped: bool,
    pub authenticated_signing_ready: bool,
    pub root_fingerprint: Option<String>,
    pub pending_reason: Option<PendingReason>,
    pub reason_label: Option<String>,
    pub custody: Option<Custody>,
    pub custody_label: Option<String>,
    pub device_id: Option<Ulid>,
    pub device_grant_expires_at: Option<u64>,
    pub device_renewal_due_at: Option<u64>,
    pub device_renewal_required: bool,
    pub seats_still_local_dev: usize,
}
impl IdentityStatus {
    pub(super) fn hold(&mut self, reason: PendingReason) {
        self.state = if self.bootstrapped {
            IdentityState::Held
        } else {
            IdentityState::Pending
        };
        self.label = if self.bootstrapped {
            "signing held"
        } else {
            "identity pending"
        }
        .into();
        self.authenticated_signing_ready = false;
        self.pending_reason = Some(reason);
        self.reason_label = Some(reason.label().into());
    }
}

/// Held by the logic IO owner. Neither device custody nor this provider is
/// exposed over the seat/MCP API; the GUI receives only IdentityStatus.
pub struct IdentityRuntime {
    pub(super) account: SqliteStore,
    pub(super) keys: Arc<dyn SeatKeys>,
    pub(super) chain: Option<Arc<ChainSeatKeys>>,
}
impl IdentityRuntime {
    pub fn keys(&self) -> Arc<dyn SeatKeys> {
        self.keys.clone()
    }
    pub fn status(&self) -> Result<IdentityStatus> {
        bbt_rt::assert_not_ui_thread();
        self.status_at(crate::store::now_ms() / 1000)
    }
    /// IO clock seam; timestamps use the same seconds as signed grants.
    pub fn status_at(&self, now: u64) -> Result<IdentityStatus> {
        bbt_rt::assert_not_ui_thread();
        if let Some(chain) = &self.chain {
            return chain.status_at(now);
        }
        let mut status = read(&self.account, now)?;
        if status.bootstrapped && status.pending_reason.is_none() {
            // Bootstrap is an explicit operator operation. If it ran after
            // this worker opened, report the pending activation honestly.
            status.hold(PendingReason::ActivationRequired);
        }
        Ok(status)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Attempt {
    pub root_fingerprint: Option<String>,
    pub reason: PendingReason,
    pub at: u64,
}
pub(super) fn attempted(
    account: &SqliteStore,
    fingerprint: &str,
    reason: PendingReason,
    now: u64,
) -> super::Result<()> {
    let fingerprint = (fingerprint.len() == 40
        && fingerprint
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_lowercase()))
    .then(|| fingerprint.to_owned());
    let attempt = Attempt {
        root_fingerprint: fingerprint,
        reason,
        at: now,
    };
    account
        .put(
            "identity/attempt",
            &serde_json::to_vec(&attempt).map_err(|_| PendingReason::InvalidProof)?,
        )
        .map_err(|_| PendingReason::StorageUnavailable)
}
pub(super) fn local_count(
    account: &SqliteStore,
    state: Option<&super::IssuerState>,
) -> Result<usize> {
    Ok(Registry::new(account.clone())
        .list()?
        .iter()
        .filter(|record| {
            record.lifecycle != Lifecycle::ClosedRetained
                && state.is_none_or(|state| {
                    state.kind == IssuerKind::LocalDev
                        || state.legacy.iter().any(|entry| {
                            entry.seat == record.id
                                && entry.generation == record.generation
                                && entry.before.is_none()
                        })
                })
        })
        .count())
}
pub(super) fn read(account: &SqliteStore, now: u64) -> Result<IdentityStatus> {
    let state = transition::read(account)?;
    let mut status = IdentityStatus {
        state: IdentityState::LocalDev,
        label: "unsigned, local only".into(),
        issuer_kind: IssuerKind::LocalDev,
        bootstrapped: false,
        authenticated_signing_ready: false,
        root_fingerprint: None,
        pending_reason: None,
        reason_label: None,
        custody: None,
        custody_label: None,
        device_id: None,
        device_grant_expires_at: None,
        device_renewal_due_at: None,
        device_renewal_required: false,
        seats_still_local_dev: local_count(account, state.as_ref())?,
    };
    if state
        .as_ref()
        .is_some_and(|s| s.kind == IssuerKind::LocalDev)
    {
        return Ok(status); // Explicit reset retains public history, never re-pins it.
    }
    let Some(state) = state else {
        if let Some(bytes) = account.get("identity/attempt")? {
            let attempt: Attempt = serde_json::from_slice(&bytes)?;
            status.root_fingerprint = attempt.root_fingerprint;
            status.hold(attempt.reason);
        }
        return Ok(status);
    };
    let head: Signed<AccountHead> = serde_json::from_slice(
        &account
            .get("identity/head")?
            .ok_or_else(|| invalid("missing identity head"))?,
    )?;
    let id = head
        .payload
        .active_devices
        .first()
        .ok_or_else(|| invalid("missing active device"))?
        .device_id;
    let identity: DeviceIdentity = serde_json::from_slice(
        &account
            .get(&format!("identity/device/{id}"))?
            .ok_or_else(|| invalid("missing device identity"))?,
    )?;
    let pinned = state
        .root_fingerprint
        .as_deref()
        .ok_or_else(|| invalid("missing pinned fingerprint"))?;
    identity.verify(pinned, &head, head.payload.issued_at)?;
    status.state = IdentityState::Certified;
    status.label = "certified".into();
    status.issuer_kind = IssuerKind::GpgRoot;
    status.bootstrapped = true;
    status.authenticated_signing_ready = true;
    status.root_fingerprint = Some(pinned.into());
    status.custody = Some(identity.custody);
    status.custody_label = Some(identity.custody.label().into());
    status.device_id = Some(id);
    let grant = &identity.grant.payload;
    let due = grant
        .expires_at
        .saturating_sub(7 * 24 * 3600)
        .max(grant.not_before);
    status.device_grant_expires_at = Some(grant.expires_at);
    status.device_renewal_due_at = Some(due);
    status.device_renewal_required = now >= due;
    if now >= grant.expires_at {
        status.hold(PendingReason::DeviceGrantExpired);
    }
    Ok(status)
}
