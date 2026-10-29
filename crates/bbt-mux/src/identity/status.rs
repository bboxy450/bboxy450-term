// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
    #[serde(default)]
    pub custody_held_since: Option<u64>,
    #[serde(default)]
    pub open_ephemeral_holds: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reopen_failed: Option<bbt_proto::identity::ReopenFailure>,
}
// Hold/reopen transitions are shared by the internal facts and the count-bearing card.
pub(super) trait IdentityView {
    fn hold(&mut self, reason: PendingReason);
    fn reopen(&mut self, failure: &bbt_proto::identity::ReopenFailure);
    fn bootstrapped(&self) -> bool;
    fn set_custody(&mut self, custody: Option<Custody>);
}
macro_rules! identity_view {
    ($view:ty) => {
        impl IdentityView for $view {
            fn bootstrapped(&self) -> bool {
                self.bootstrapped
            }
            fn set_custody(&mut self, custody: Option<Custody>) {
                self.custody = custody;
                self.custody_label = custody.map(|kind| kind.label().into());
            }
            fn hold(&mut self, reason: PendingReason) {
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
            fn reopen(&mut self, failure: &bbt_proto::identity::ReopenFailure) {
                self.hold(PendingReason::ReopenFailed);
                self.reason_label = Some(format!(
                    "reopen_failed: {}; retrying without another native attempt",
                    failure.reason
                ));
                self.reopen_failed = Some(failure.clone());
                self.device_id = None;
            }
        }
    };
}
identity_view!(IdentityStatus);
impl IdentityStatus {
    pub(super) fn hold(&mut self, reason: PendingReason) {
        IdentityView::hold(self, reason);
    }
}

/// Only the current verified fields consumed by admission; never a status-card placeholder.
pub(crate) struct IdentityAdmission {
    pub state: IdentityState,
    pub reason_label: Option<String>,
    pub device_id: Option<Ulid>,
}

/// Shared verified facts. Display counts are absent and are collected only for a full card.
#[derive(Clone)]
pub(super) struct IdentityFields {
    pub(super) state: IdentityState,
    pub(super) label: String,
    pub(super) issuer_kind: IssuerKind,
    pub(super) bootstrapped: bool,
    pub(super) authenticated_signing_ready: bool,
    pub(super) root_fingerprint: Option<String>,
    pub(super) pending_reason: Option<PendingReason>,
    pub(super) reason_label: Option<String>,
    pub(super) custody: Option<Custody>,
    pub(super) custody_label: Option<String>,
    pub(super) device_id: Option<Ulid>,
    pub(super) device_grant_expires_at: Option<u64>,
    pub(super) device_renewal_due_at: Option<u64>,
    pub(super) device_renewal_required: bool,
    pub(super) reopen_failed: Option<bbt_proto::identity::ReopenFailure>,
}
identity_view!(IdentityFields);
impl IdentityFields {
    fn from_status(status: &IdentityStatus) -> Self {
        Self {
            state: status.state,
            label: status.label.clone(),
            issuer_kind: status.issuer_kind,
            bootstrapped: status.bootstrapped,
            authenticated_signing_ready: status.authenticated_signing_ready,
            root_fingerprint: status.root_fingerprint.clone(),
            pending_reason: status.pending_reason,
            reason_label: status.reason_label.clone(),
            custody: status.custody,
            custody_label: status.custody_label.clone(),
            device_id: status.device_id,
            device_grant_expires_at: status.device_grant_expires_at,
            device_renewal_due_at: status.device_renewal_due_at,
            device_renewal_required: status.device_renewal_required,
            reopen_failed: status.reopen_failed.clone(),
        }
    }
    pub(super) fn hold(&mut self, reason: PendingReason) {
        IdentityView::hold(self, reason);
    }
    pub(super) fn with_count(self, count: usize) -> IdentityStatus {
        IdentityStatus {
            state: self.state,
            label: self.label,
            issuer_kind: self.issuer_kind,
            bootstrapped: self.bootstrapped,
            authenticated_signing_ready: self.authenticated_signing_ready,
            root_fingerprint: self.root_fingerprint,
            pending_reason: self.pending_reason,
            reason_label: self.reason_label,
            custody: self.custody,
            custody_label: self.custody_label,
            device_id: self.device_id,
            device_grant_expires_at: self.device_grant_expires_at,
            device_renewal_due_at: self.device_renewal_due_at,
            device_renewal_required: self.device_renewal_required,
            seats_still_local_dev: count,
            custody_held_since: None,
            open_ephemeral_holds: 0,
            reopen_failed: self.reopen_failed,
        }
    }
}

/// Held by the logic IO owner. Neither device custody nor this provider is
/// exposed over the seat/MCP API; the GUI receives only IdentityStatus.
pub struct IdentityRuntime {
    pub(super) account: SqliteStore,
    pub(super) keys: Arc<dyn SeatKeys>,
    pub(super) chain: Option<Arc<ChainSeatKeys>>,
    pub(super) retry: Option<super::custodied::Retry>,
    pub(super) pending: Option<PendingReason>,
    pub(super) bus_key: Option<ed25519_dalek::SigningKey>,
    pub(super) observed_status: std::sync::Mutex<Option<IdentityFields>>,
    pub(super) reopen_failed: Option<bbt_proto::identity::ReopenFailure>,
}
impl IdentityRuntime {
    pub(crate) fn note_reopen_failed(&mut self, failure: bbt_proto::identity::ReopenFailure) {
        self.reopen_failed = Some(failure);
    }
    pub(super) fn decorate_reopen_failure(&self, status: &mut impl IdentityView) {
        if let Some(failure) = &self.reopen_failed {
            status.reopen(failure);
        }
    }
    pub(crate) fn reopening_failed(&self) -> bool {
        self.reopen_failed.is_some()
    }
    pub(crate) fn issued_by(&self) -> Option<&str> {
        self.chain.as_ref().and_then(|chain| chain.test_label())
    }
    /// Test-account attribution is an account fact, never a ctl --as value.
    pub(crate) fn bind_origin(&self, mut origin: crate::mcp::Origin) -> crate::mcp::Origin {
        if let Some(label) = self.issued_by() {
            origin.issued_by = Some(label.into());
        }
        origin
    }
    pub fn keys(&self) -> Arc<dyn SeatKeys> {
        self.keys.clone()
    }
    pub fn status(&self) -> Result<IdentityStatus> {
        bbt_rt::assert_not_ui_thread();
        let status = self.status_at(crate::store::now_ms() / 1000)?;
        self.observe(IdentityFields::from_status(&status))?;
        Ok(status)
    }
    /// IO clock seam; timestamps use the same seconds as signed grants.
    pub fn status_at(&self, now: u64) -> Result<IdentityStatus> {
        bbt_rt::assert_not_ui_thread();
        let mut fields = read_fields(&self.account, now)?;
        // Collect the real count before custody reads, as the full card always did.
        let count = seat_count(&self.account)?;
        self.decorate_fields(&mut fields, now)?;
        let mut status = fields.with_count(count);
        super::hold::decorate(&self.account, &mut status)?;
        self.decorate_reopen_failure(&mut status);
        Ok(status)
    }
    pub(crate) fn admission(&self) -> Result<IdentityAdmission> {
        bbt_rt::assert_not_ui_thread();
        let now = crate::store::now_ms() / 1000;
        let mut fields = read_fields(&self.account, now)?;
        self.decorate_fields(&mut fields, now)?;
        self.decorate_reopen_failure(&mut fields);
        self.observe(fields.clone())?;
        Ok(IdentityAdmission {
            state: fields.state,
            reason_label: fields.reason_label,
            device_id: fields.device_id,
        })
    }
    fn decorate_fields(&self, fields: &mut IdentityFields, now: u64) -> Result<()> {
        if let Some(chain) = &self.chain {
            chain.decorate_fields(fields, now);
            if matches!(
                fields.pending_reason,
                Some(
                    PendingReason::KeychainUnavailable(_)
                        | PendingReason::KeychainInteractionRequired
                        | PendingReason::KeychainTimedOut
                        | PendingReason::KeychainSigningIdentityRequired
                )
            ) {
                fields.device_id = None;
            }
        } else {
            super::local::decorate(&self.account, fields)?;
            if let Some(label) = self
                .retry
                .as_ref()
                .and_then(|retry| retry.custody_label(fields.custody))
            {
                fields.custody_label = Some(label.into());
            }
            if let Some(reason) = self.pending {
                fields.hold(reason);
                fields.device_id = None;
            }
            if fields.bootstrapped && fields.pending_reason.is_none() {
                fields.hold(PendingReason::ActivationRequired);
            }
        }
        Ok(())
    }
    fn observe(&self, fields: IdentityFields) -> Result<()> {
        *self
            .observed_status
            .lock()
            .map_err(|_| invalid("identity status cache poisoned"))? = Some(fields);
        Ok(())
    }
    pub(crate) fn bus_signer(&self) -> Result<Option<ed25519_dalek::SigningKey>> {
        if self.reopening_failed() {
            return Ok(None);
        }
        if self.keys.signing_hold().is_some() {
            return Ok(None);
        }
        if let Some(chain) = &self.chain {
            return Ok(chain.event_signer().ok());
        }
        Ok(self.bus_key.clone())
    }
    /// Public attribution survives a hold; this performs no key-store operation
    /// and never changes the held/readiness status exposed to the operator.
    pub(crate) fn observed_operator_device(&self) -> Result<Option<Ulid>> {
        if let Some(device) = read_fields(&self.account, crate::store::now_ms() / 1000)?.device_id {
            return Ok(Some(device));
        }
        super::local::observed_device_id(&self.account)
    }
    pub(crate) fn operator_signing(&self) -> Result<crate::messaging::keys::OperatorSigning> {
        let device_id = self
            .admission()?
            .device_id
            .ok_or_else(|| invalid("held: operator device custody is unavailable"))?;
        if let Some(chain) = &self.chain {
            return chain.operator_signing();
        }
        let key = self
            .bus_signer()?
            .ok_or_else(|| invalid("held: operator device signing is unavailable"))?;
        let issuer = self
            .keys
            .ready_issuer()
            .ok_or_else(|| invalid("held: operator issuer is unavailable"))?;
        let bytes = self
            .account
            .get("identity/local-issuer")?
            .ok_or_else(|| invalid("held: missing local issuer custody record"))?;
        let (public_key, epoch) = crate::messaging::keys::local_operator_grant(&bytes)?;
        let device = crate::messaging::wire::public(&key.verifying_key().to_bytes());
        if issuer.kind != IssuerKind::LocalDev || issuer.key != device || public_key != device {
            return Err(invalid(
                "held: local operator issuer does not match custody",
            ));
        }
        Ok(crate::messaging::keys::OperatorSigning {
            device_id,
            issuer,
            device,
            grant: crate::store::digest(&bytes),
            epoch,
            key,
            origin: Default::default(),
        })
    }
    pub(crate) fn close_open_holds(&self, root: &std::path::Path) -> Result<usize> {
        self.chain
            .as_ref()
            .map(|chain| chain.close_open_holds(root))
            .unwrap_or(Ok(0))
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
// Existing count-bearing readers retain their API and propagate count corruption.
pub(super) fn read(account: &SqliteStore, now: u64) -> Result<IdentityStatus> {
    let fields = read_fields(account, now)?;
    Ok(fields.with_count(seat_count(account)?))
}
pub(super) fn seat_count(account: &SqliteStore) -> Result<usize> {
    let state = transition::read(account)?;
    local_count(account, state.as_ref())
}
pub(super) fn read_fields(account: &SqliteStore, now: u64) -> Result<IdentityFields> {
    let state = transition::read(account)?;
    let mut status = IdentityFields {
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
        device_id: super::local::device_id(account)?,
        device_grant_expires_at: None,
        device_renewal_due_at: None,
        device_renewal_required: false,
        reopen_failed: None,
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

#[cfg(test)]
mod mentions_admission_tests {
    use super::super::{DeviceKeyStore, TestAccount};
    use super::*;
    use ed25519_dalek::SigningKey;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    struct CustodySwitch {
        store: Arc<dyn DeviceKeyStore + Send + Sync>,
        held: AtomicBool,
        loads: AtomicU64,
    }
    impl DeviceKeyStore for CustodySwitch {
        fn custody(&self) -> Custody {
            self.store.custody()
        }
        fn load(&self, id: Ulid) -> crate::identity::Result<Option<SigningKey>> {
            self.loads.fetch_add(1, Ordering::Relaxed);
            if self.held.load(Ordering::Relaxed) {
                return Err(PendingReason::KeychainUnavailable(-25308));
            }
            self.store.load(id)
        }
        fn create(&self, id: Ulid, key: &SigningKey) -> crate::identity::Result<()> {
            self.store.create(id, key)
        }
    }

    // keeps: admission shares proof/expiry/hold/reopen gates without fabricating a card count.
    #[test]
    fn mentions_admission_ignores_bad_counts_but_keeps_current_authority_guards() {
        let root = tempfile::tempdir().unwrap();
        let run = root.path().join("run");
        let generated = TestAccount::open(&root.path().join("keys"), &run).unwrap();
        let account = SqliteStore::open(&run.join("account")).unwrap();
        let pinned = transition::read(&account)
            .unwrap()
            .unwrap()
            .root_fingerprint
            .unwrap();
        let custody = Arc::new(CustodySwitch {
            store: generated.key_store(),
            held: AtomicBool::new(false),
            loads: AtomicU64::new(0),
        });
        let now = crate::store::now_ms() / 1000;
        let clock = Arc::new(AtomicU64::new(now));
        let injected = clock.clone();
        let chain = Arc::new(
            ChainSeatKeys::with_clock(
                account.clone(),
                custody.clone(),
                &pinned,
                SigningKey::from_bytes(&[91; 32]),
                Arc::new(move || injected.load(Ordering::Relaxed)),
            )
            .unwrap(),
        );
        let mut runtime = IdentityRuntime {
            account: account.clone(),
            keys: chain.clone(),
            chain: Some(chain),
            retry: None,
            pending: None,
            bus_key: None,
            observed_status: std::sync::Mutex::new(None),
            reopen_failed: None,
        };
        let device = runtime.admission().unwrap().device_id.unwrap();
        let key = format!("identity/device/{device}");
        let original = account.get(&key).unwrap().unwrap();
        let identity: DeviceIdentity = serde_json::from_slice(&original).unwrap();
        let expires = identity.grant.payload.expires_at;
        account
            .put("seat/corrupt-non-recipient", b"{invalid-json")
            .unwrap();
        assert!(
            runtime.status().is_err(),
            "a card must not invent a successful zero count"
        );
        assert_eq!(
            runtime
                .observed_status
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .device_id,
            Some(device),
            "only verified count-free facts are observed; no card placeholder exists"
        );
        assert_eq!(runtime.observed_operator_device().unwrap(), Some(device));
        assert_eq!(runtime.operator_signing().unwrap().device_id, device);
        let reads = custody.loads.load(Ordering::Relaxed);
        runtime.custody_control_status().unwrap();
        *runtime.observed_status.lock().unwrap() = None;
        runtime.custody_control_status().unwrap();
        assert_eq!(
            custody.loads.load(Ordering::Relaxed),
            reads,
            "cached and initially empty custody metadata must not attempt a key read"
        );

        custody.held.store(true, Ordering::Relaxed);
        let held = runtime.admission().unwrap();
        assert_eq!(held.state, IdentityState::Held);
        assert_eq!(held.device_id, None);
        let reads = custody.loads.load(Ordering::Relaxed);
        assert!(runtime.custody_control_status().unwrap().hold.is_some());
        assert_eq!(
            custody.loads.load(Ordering::Relaxed),
            reads,
            "held custody metadata is an observation, never a new key attempt"
        );
        assert_eq!(
            runtime.observed_operator_device().unwrap(),
            Some(device),
            "public attribution survives custody hold"
        );
        assert!(runtime.operator_signing().is_err());
        custody.held.store(false, Ordering::Relaxed);

        runtime.note_reopen_failed(bbt_proto::identity::ReopenFailure {
            reason: "private interrupted reopen".into(),
        });
        assert_eq!(runtime.admission().unwrap().device_id, None);
        assert!(
            runtime.operator_signing().is_err(),
            "outer reopen guard must run before chain signing"
        );
        assert_eq!(runtime.observed_operator_device().unwrap(), Some(device));
        runtime.reopen_failed = None;

        let mut forged = identity.clone();
        forged.grant.signature = "A".repeat(forged.grant.signature.len());
        account
            .put(&key, &serde_json::to_vec(&forged).unwrap())
            .unwrap();
        assert!(
            runtime.admission().is_err(),
            "own device proof is still verified"
        );
        assert!(runtime.operator_signing().is_err());
        account.put(&key, &original).unwrap();
        let head = account.get("identity/head").unwrap().unwrap();
        let mut forged_head: Signed<AccountHead> = serde_json::from_slice(&head).unwrap();
        forged_head.signature = "A".repeat(forged_head.signature.len());
        account
            .put("identity/head", &serde_json::to_vec(&forged_head).unwrap())
            .unwrap();
        assert!(
            runtime.admission().is_err(),
            "own account head is still verified"
        );
        assert!(runtime.operator_signing().is_err());
        account.put("identity/head", &head).unwrap();

        let mut expired = read_fields(&account, expires).unwrap();
        runtime.decorate_fields(&mut expired, expires).unwrap();
        assert_eq!(
            expired.pending_reason,
            Some(PendingReason::DeviceGrantExpired)
        );
        clock.store(expires, Ordering::Relaxed);
        assert!(
            runtime.operator_signing().is_err(),
            "expired current grant cannot sign"
        );
        clock.store(now, Ordering::Relaxed);
        account
            .connection()
            .unwrap()
            .execute("DELETE FROM kv WHERE key='seat/corrupt-non-recipient'", [])
            .unwrap();
        assert_eq!(runtime.status().unwrap().state, IdentityState::Certified);
        assert_eq!(runtime.operator_signing().unwrap().device_id, device);
    }
}
