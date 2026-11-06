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

// The C1 reader is host constructed and contains public pins only. It deliberately
// has no SeatKeys or DeviceKeyStore port; a signing hold is not a public proof.
const PUBLIC_ROW_LIMIT: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicFence {
    pub daemon_incarnation: Ulid,
    pub fence_rev: u64,
    pub publication_rev: u64,
    pub account: String,
    pub account_epoch: u64,
    pub issuer: crate::messaging::wire::Issuer,
    pub issuer_epoch: u64,
    pub device: Option<Ulid>,
    pub device_epoch: Option<u64>,
    pub proof: String,
    pub grant: Option<String>,
    pub not_before: u64,
    pub expires_at: Option<u64>,
    pub revoked: bool,
}

#[derive(Clone)]
pub(super) enum PublicPin {
    Local(crate::messaging::wire::Issuer),
    Certified {
        issuer: crate::messaging::wire::Issuer,
        pinned: String,
        device: Ulid,
        grant: String,
    },
}

#[derive(Clone)]
struct PublicRowDigest {
    key: String,
    digest: Option<[u8; 32]>,
}
struct VerifiedPublicFence {
    fence: PublicFence,
    verified_at: u64,
    file: (u64, u64),
    rows: Vec<PublicRowDigest>,
}

/// One job's read-only authority connection. It is transferred between that
/// job's sequential compute owners; try_lock never makes an owner wait. No
/// transaction or lock is retained between units. Compute and monitor share
/// this connection; one transient checkpoint connection fits the second slot.
pub(crate) struct PublicFenceSession {
    file: (u64, u64),
    db: std::sync::Mutex<Option<rusqlite::Connection>>,
    retiring: std::sync::atomic::AtomicBool,
}
impl PublicFenceSession {
    pub(crate) fn is_retiring(&self) -> bool {
        self.retiring.load(std::sync::atomic::Ordering::Acquire)
    }
    /// Retirement grants no authority. Only the exact owned connection can be
    /// taken, after current readers release their bounded borrow. Poisoned
    /// custody may be recovered for destruction, never for a public check.
    pub(crate) fn try_take_connection(
        &self,
    ) -> std::result::Result<Option<rusqlite::Connection>, bbt_proto::dream::Code> {
        self.retiring
            .store(true, std::sync::atomic::Ordering::Release);
        let mut db = match self.db.try_lock() {
            Ok(db) => db,
            Err(std::sync::TryLockError::WouldBlock) => return Err(bbt_proto::dream::Code::Busy),
            Err(std::sync::TryLockError::Poisoned(poison)) => poison.into_inner(),
        };
        Ok(db.take())
    }
}
enum PublicCheckConnection<'a> {
    Transient(rusqlite::Connection),
    Retained(std::sync::MutexGuard<'a, Option<rusqlite::Connection>>),
}
impl std::ops::Deref for PublicCheckConnection<'_> {
    type Target = rusqlite::Connection;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Transient(db) => db,
            Self::Retained(db) => db.as_ref().expect("checked open authority session"),
        }
    }
}
impl std::ops::DerefMut for PublicCheckConnection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Transient(db) => db,
            Self::Retained(db) => db.as_mut().expect("checked open authority session"),
        }
    }
}

struct PublicFenceSource {
    account: SqliteStore,
    daemon_incarnation: Ulid,
    publication: std::sync::atomic::AtomicU64,
    pin: std::sync::Mutex<Option<(PublicPin, (u64, u64))>>,
    installed: bool,
    verified: std::sync::Mutex<Option<Arc<VerifiedPublicFence>>>,
    #[cfg(test)]
    guard_profile_records: std::sync::atomic::AtomicU64,
    #[cfg(test)]
    public_clock: std::sync::atomic::AtomicU64,
}

/// Clone this on the IO owner and read only from the bounded C1 control/monitor
/// owner. Each call checks current public authority; no cached status can admit.
#[derive(Clone)]
pub(crate) struct PublicFenceReader(Arc<PublicFenceSource>);
impl PublicFenceReader {
    pub(super) fn new(account: SqliteStore, pin: Option<PublicPin>) -> Self {
        // A failure disables C1 only. Ordinary identity/custody behavior remains
        // unchanged and never depends on this optional consumer's availability.
        let installed = install_public_fence(&account).is_ok();
        let file = public_file(&account).ok();
        let pin = pin.zip(file);
        Self(Arc::new(PublicFenceSource {
            account,
            daemon_incarnation: Ulid::new(),
            publication: std::sync::atomic::AtomicU64::new(2),
            pin: std::sync::Mutex::new(pin),
            installed,
            verified: std::sync::Mutex::new(None),
            #[cfg(test)]
            guard_profile_records: std::sync::atomic::AtomicU64::new(0),
            #[cfg(test)]
            public_clock: std::sync::atomic::AtomicU64::new(0),
        }))
    }
    /// Lock/IO-free hint only. It never substitutes for current() at an effect.
    pub(crate) fn cached_revision(&self) -> u64 {
        self.0
            .publication
            .load(std::sync::atomic::Ordering::Acquire)
    }
    #[cfg(test)]
    pub(crate) fn current(&self) -> Result<PublicFence> {
        let prior = self.verified_snapshot()?;
        self.current_verified_at(self.public_now(), prior)
    }
    /// Public-only cold verification has explicit work boundaries. An existing
    /// verified proof is only a candidate: every current raw row/revision/inode
    /// and time check still runs before that candidate can leave the reader.
    #[cfg(test)]
    pub(crate) fn current_cooperative(
        &self,
        checkpoint: &mut dyn FnMut() -> Result<()>,
    ) -> Result<PublicFence> {
        let prior = self.verified_snapshot()?;
        // Sample AFTER capturing this exact candidate (forward verification
        // must not be mistaken for a wall-clock rollback).
        let now = self.public_now();
        if let Some(verified) = prior {
            self.with_current_verified_at(
                &verified.fence,
                verified.clone(),
                now,
                None,
                |_| Ok(()),
            )?;
            return Ok(verified.fence.clone());
        }
        self.current_verified_cooperative(now, None, checkpoint)
    }
    pub(crate) fn current_prepared_cooperative(
        &self,
        session: &PublicFenceSession,
        checkpoint: &mut dyn FnMut() -> Result<()>,
    ) -> Result<PublicFence> {
        let prior = self.verified_snapshot()?;
        let now = self.public_now();
        if let Some(verified) = prior {
            self.with_current_session(&verified.fence, session, |_| Ok(()))?;
            return Ok(verified.fence.clone());
        }
        self.current_verified_cooperative(now, None, checkpoint)
    }
    /// A bounded cache locator only. It cannot authorize a response; callers
    /// must run fresh public/actor/root/provider checks on the exact candidate.
    pub(crate) fn verified_candidate(&self) -> Result<Option<PublicFence>> {
        Ok(self
            .verified_snapshot()?
            .map(|verified| verified.fence.clone()))
    }
    fn verified_snapshot(&self) -> Result<Option<Arc<VerifiedPublicFence>>> {
        Ok(self
            .0
            .verified
            .try_lock()
            .map_err(|_| invalid("stale public identity cache"))?
            .clone())
    }
    fn public_now(&self) -> u64 {
        #[cfg(test)]
        {
            match self
                .0
                .public_clock
                .load(std::sync::atomic::Ordering::Acquire)
            {
                0 => crate::store::now_ms() / 1000,
                now => now,
            }
        }
        #[cfg(not(test))]
        {
            crate::store::now_ms() / 1000
        }
    }
    fn invalidate_publication(&self, expected: u64) {
        if expected & 1 == 0
            && let Some(invalid) = expected.checked_add(1)
        {
            let _ = self.0.publication.compare_exchange(
                expected,
                invalid,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            );
        }
    }
    /// A compute owner may compare only a proof this host actually verified.
    /// Every call reads fresh public revision and bounded row digests; no PGP,
    /// status, signing availability, custody or TTL-only permission is used.
    #[cfg(test)]
    pub(crate) fn still_current(&self, captured: &PublicFence) -> bool {
        self.with_current(captured, |_| Ok(())).is_ok()
    }
    /// Read bounded host metadata in the same fresh read-only transaction as
    /// the verified public-row checks. The callback cannot borrow the returned
    /// connection; revision/file/publication checks still run after its reads.
    pub(crate) fn with_current<T>(
        &self,
        captured: &PublicFence,
        read: impl FnOnce(&rusqlite::Connection) -> Result<T>,
    ) -> Result<T> {
        self.with_current_from(captured, || self.verified_snapshot(), read)
    }
    /// Preparation runs on the isolated admission owner and is separately CPU
    /// measured/enforced by its caller. Only a fixed prepared-statement cache
    /// and the native read-only connection survive; authority never does.
    pub(crate) fn prepare_session(
        &self,
        prepare_actor: impl FnOnce(&rusqlite::Connection) -> Result<()>,
    ) -> Result<PublicFenceSession> {
        self.prepare_session_cooperative(prepare_actor, &mut || Ok(()))
    }
    pub(crate) fn prepare_session_cooperative(
        &self,
        prepare_actor: impl FnOnce(&rusqlite::Connection) -> Result<()>,
        checkpoint: &mut dyn FnMut() -> Result<()>,
    ) -> Result<PublicFenceSession> {
        bbt_rt::assert_not_ui_thread();
        if !self.0.installed || self.cached_revision() & 1 != 0 {
            return Err(invalid("stale public authority preparation"));
        }
        let file = public_file(&self.0.account)?;
        checkpoint()?;
        let db = public_connection_cooperative(&self.0.account, checkpoint)?;
        db.set_prepared_statement_cache_capacity(3);
        drop(db.prepare_cached(PUBLIC_REVISION_SQL)?);
        checkpoint()?;
        drop(db.prepare_cached(PUBLIC_DIGEST_SQL)?);
        checkpoint()?;
        prepare_actor(&db)?;
        checkpoint()?;
        public_connection_current(&db)?;
        if public_file(&self.0.account)? != file {
            return Err(invalid("stale public account session"));
        }
        Ok(PublicFenceSession {
            file,
            db: std::sync::Mutex::new(Some(db)),
            retiring: std::sync::atomic::AtomicBool::new(false),
        })
    }
    pub(crate) fn with_current_session<T>(
        &self,
        captured: &PublicFence,
        session: &PublicFenceSession,
        read: impl FnOnce(&rusqlite::Connection) -> Result<T>,
    ) -> Result<T> {
        self.try_with_current_session(captured, session, read)?
            .ok_or_else(|| invalid("public authority session unavailable"))
    }
    /// None means only a concurrent bounded check owns this session. Callers
    /// defer without doing work; poisoned/stale sessions remain errors.
    pub(crate) fn try_with_current_session<T>(
        &self,
        captured: &PublicFence,
        session: &PublicFenceSession,
        read: impl FnOnce(&rusqlite::Connection) -> Result<T>,
    ) -> Result<Option<T>> {
        if session.is_retiring() {
            return Err(invalid("retiring public authority session"));
        }
        let db = match session.db.try_lock() {
            Ok(db) => db,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(invalid("poisoned public authority session"));
            }
        };
        if db.is_none() {
            return Err(invalid("closed public authority session"));
        }
        let verified = self
            .verified_snapshot()?
            .ok_or_else(|| invalid("missing verified public identity"))?;
        let now = self.public_now();
        self.with_current_verified_at(captured, verified, now, Some((session.file, db)), read)
            .map(Some)
    }
    fn with_current_from<T>(
        &self,
        captured: &PublicFence,
        snapshot: impl FnOnce() -> Result<Option<Arc<VerifiedPublicFence>>>,
        read: impl FnOnce(&rusqlite::Connection) -> Result<T>,
    ) -> Result<T> {
        let verified = snapshot()?.ok_or_else(|| invalid("missing verified public identity"))?;
        // The production clock read follows the cache Arc clone, never before.
        self.with_current_verified_at(captured, verified, self.public_now(), None, read)
    }
    #[cfg(test)]
    fn still_current_from(
        &self,
        captured: &PublicFence,
        snapshot: impl FnOnce() -> Result<Option<Arc<VerifiedPublicFence>>>,
    ) -> bool {
        self.with_current_from(captured, snapshot, |_| Ok(()))
            .is_ok()
    }
    #[cfg(test)]
    fn still_current_at(&self, captured: &PublicFence, now: u64) -> bool {
        let Ok(Some(verified)) = self.verified_snapshot() else {
            return false;
        };
        self.with_current_verified_at(captured, verified, now, None, |_| Ok(()))
            .is_ok()
    }
    fn with_current_verified_at<T>(
        &self,
        captured: &PublicFence,
        verified: Arc<VerifiedPublicFence>,
        now: u64,
        session: Option<(
            (u64, u64),
            std::sync::MutexGuard<'_, Option<rusqlite::Connection>>,
        )>,
        read: impl FnOnce(&rusqlite::Connection) -> Result<T>,
    ) -> Result<T> {
        use std::sync::atomic::Ordering;
        bbt_rt::assert_not_ui_thread();
        #[cfg(test)]
        let mut profile = if std::env::var_os("BBOXY_DREAM_GUARD_PROFILE").is_some() {
            self.0
                .guard_profile_records
                .try_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                    (count < 16).then(|| count + 1)
                })
                .ok()
                .map(|record| (record, [std::time::Duration::ZERO; 9]))
        } else {
            None
        };
        #[cfg(test)]
        guard_profile_sample(&mut profile, 0);
        if &verified.fence != captured
            || !self.0.installed
            || self.0.publication.load(Ordering::Acquire) != captured.publication_rev
            || captured.publication_rev & 1 != 0
        {
            return Err(invalid("stale verified public identity"));
        }
        if now < verified.verified_at
            || now < captured.not_before
            || captured.expires_at.is_some_and(|expiry| now >= expiry)
        {
            self.invalidate_publication(captured.publication_rev);
            return Err(invalid("expired or rolled-back public identity time"));
        }
        if public_file(&self.0.account)? != verified.file {
            self.invalidate_publication(captured.publication_rev);
            return Err(invalid("stale public account incarnation"));
        }
        #[cfg(test)]
        guard_profile_sample(&mut profile, 1);
        let mut db = match session {
            Some((file, db)) => {
                if file != verified.file {
                    return Err(invalid("stale public account session"));
                }
                public_connection_current(
                    db.as_ref()
                        .ok_or_else(|| invalid("closed public authority session"))?,
                )?;
                PublicCheckConnection::Retained(db)
            }
            None => PublicCheckConnection::Transient(public_connection(&self.0.account)?),
        };
        #[cfg(test)]
        guard_profile_sample(&mut profile, 2);
        let tx = db.transaction()?;
        if public_revision(&tx)? != captured.fence_rev {
            return Err(invalid("stale public identity revision"));
        }
        #[cfg(test)]
        guard_profile_sample(&mut profile, 3);
        for row in &verified.rows {
            if public_digest(&tx, &row.key)? != row.digest {
                self.invalidate_publication(captured.publication_rev);
                return Err(invalid("stale public identity row digest"));
            }
        }
        #[cfg(test)]
        guard_profile_sample(&mut profile, 4);
        let value = read(&tx)?;
        #[cfg(test)]
        guard_profile_sample(&mut profile, 5);
        tx.commit()?;
        if public_revision(&db)? != captured.fence_rev {
            return Err(invalid("stale public identity revision"));
        }
        #[cfg(test)]
        guard_profile_sample(&mut profile, 6);
        if public_file(&self.0.account)? != verified.file {
            self.invalidate_publication(captured.publication_rev);
            return Err(invalid("stale public account incarnation"));
        }
        if self.0.publication.load(Ordering::Acquire) != captured.publication_rev {
            return Err(invalid("stale public identity publication"));
        }
        public_connection_current(&db)?;
        #[cfg(test)]
        {
            guard_profile_sample(&mut profile, 7);
        }
        // Closing the private connection is part of the measured guard too.
        drop(db);
        #[cfg(test)]
        {
            guard_profile_sample(&mut profile, 8);
            if let Some((record, times)) = profile {
                let ns = |a: usize, b: usize| times[b].saturating_sub(times[a]).as_nanos();
                eprintln!(
                    "DREAM_GUARD_PROFILE record={record} thread={:?} pre_file_ns={} open_pragmas_ns={} begin_revision_ns={} public_digests_ns={} actor_callback_ns={} commit_revision_ns={} final_fence_ns={} close_ns={} measured_sum_ns={}",
                    std::thread::current().name(),
                    ns(0, 1),
                    ns(1, 2),
                    ns(2, 3),
                    ns(3, 4),
                    ns(4, 5),
                    ns(5, 6),
                    ns(6, 7),
                    ns(7, 8),
                    ns(0, 8)
                );
            }
        }
        Ok(value)
    }
    /// Private clock seam drives the actual public expiry checker through MCP/
    /// GUI; changing it invalidates old stamps even if a test restores time.
    #[cfg(test)]
    pub(crate) fn test_public_clock(&self, now: u64) {
        self.0
            .public_clock
            .store(now, std::sync::atomic::Ordering::Release);
        self.0
            .publication
            .fetch_add(2, std::sync::atomic::Ordering::AcqRel);
    }
    #[cfg(test)]
    fn current_at(&self, now: u64) -> Result<PublicFence> {
        self.current_verified_at(now, self.verified_snapshot()?)
    }
    #[cfg(test)]
    fn current_verified_at(
        &self,
        now: u64,
        prior: Option<Arc<VerifiedPublicFence>>,
    ) -> Result<PublicFence> {
        self.current_verified_cooperative(now, prior, &mut || Ok(()))
    }
    fn current_verified_cooperative(
        &self,
        now: u64,
        prior: Option<Arc<VerifiedPublicFence>>,
        checkpoint: &mut dyn FnMut() -> Result<()>,
    ) -> Result<PublicFence> {
        use std::sync::atomic::Ordering;
        bbt_rt::assert_not_ui_thread();
        let revision = self.0.publication.load(Ordering::Acquire);
        if !self.0.installed || revision & 1 != 0 {
            return Err(invalid("stale public identity fence"));
        }
        if let Some(prior) = prior
            && prior.fence.publication_rev == revision
            && (now < prior.verified_at
                || now < prior.fence.not_before
                || prior.fence.expires_at.is_some_and(|expiry| now >= expiry))
        {
            self.invalidate_publication(revision);
            return Err(invalid("expired or rolled-back public identity time"));
        }
        let (pin, file) = self
            .0
            .pin
            .try_lock()
            .map_err(|_| invalid("stale public identity publication"))?
            .clone()
            .ok_or_else(|| invalid("missing authenticated public identity"))?;
        if public_file(&self.0.account)? != file {
            return Err(invalid("stale public account incarnation"));
        }
        // This read-only connection belongs only to this C1 check. Its native
        // cache is advisory-bounded; it neither borrows the shared pool nor
        // changes another owner's PRAGMAs. Native allocations remain measured
        // against the parent headroom, not a claimed SQLite hard heap limit.
        checkpoint()?;
        let db = public_connection_cooperative(&self.0.account, checkpoint)?;
        let fence_rev = public_revision(&db)?;
        // Each indexed statement completes before a checkpoint. No read
        // transaction/lock survives the ordinary rest or public crypto work.
        // The initial revision plus final row digests bind this frozen proof
        // even when a same-inode write bypasses an ordinary trigger.
        let mut read_rows = Vec::new();
        read_rows
            .try_reserve_exact(8)
            .map_err(|_| invalid("public proof snapshot capacity"))?;
        let mut row = |key: &str| -> Result<Option<Vec<u8>>> {
            use sha2::{Digest, Sha256};
            let bytes = public_row(&db, key)?;
            let digest = bytes.as_ref().map(|bytes| Sha256::digest(bytes).into());
            if read_rows
                .iter()
                .any(|prior: &PublicRowDigest| prior.key == key && prior.digest != digest)
            {
                return Err(invalid("public proof changed during read"));
            }
            read_rows.push(PublicRowDigest {
                key: key.to_owned(),
                digest,
            });
            Ok(bytes)
        };
        checkpoint()?;
        let issuer_bytes =
            row("messaging/issuer")?.ok_or_else(|| invalid("missing public issuer pin"))?;
        let issuer: crate::messaging::wire::Issuer = serde_json::from_slice(&issuer_bytes)?;
        checkpoint()?;
        crate::messaging::wire::parse_public(&issuer.key)?;
        checkpoint()?;
        let state_bytes = row("identity/issuer")?;
        let state: Option<super::IssuerState> = state_bytes
            .as_ref()
            .map(|bytes| serde_json::from_slice(bytes))
            .transpose()?;
        let issuer_epoch = match &state {
            Some(state) => {
                if state.kind != issuer.kind || state.key != issuer.key {
                    return Err(invalid("stale public issuer journal"));
                }
                let epoch = transition::counter(&state.cutover_epoch)?;
                let history = row(&format!("identity/issuer-history/{epoch}"))?
                    .ok_or_else(|| invalid("missing public issuer history"))?;
                if Some(&history) != state_bytes.as_ref() {
                    return Err(invalid("stale public issuer history"));
                }
                epoch
            }
            None if issuer.kind == IssuerKind::LocalDev => 1,
            None => return Err(invalid("missing certified public issuer journal")),
        };
        let mut fence = PublicFence {
            daemon_incarnation: self.0.daemon_incarnation,
            fence_rev,
            publication_rev: revision,
            account: issuer.key.clone(),
            account_epoch: issuer_epoch,
            issuer: issuer.clone(),
            issuer_epoch,
            device: None,
            device_epoch: None,
            proof: crate::store::digest(&issuer_bytes),
            grant: None,
            not_before: 0,
            expires_at: None,
            revoked: false,
        };
        checkpoint()?;
        match pin {
            PublicPin::Local(expected) => {
                if issuer.kind != IssuerKind::LocalDev || issuer != expected {
                    return Err(invalid("stale host-authenticated local issuer"));
                }
                if let Some(bytes) = row("identity/local-issuer")? {
                    #[derive(Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct LocalProof {
                        version: u8,
                        epoch: String,
                        key_id: Ulid,
                        public_key: String,
                        custody: Custody,
                        #[serde(default)]
                        device_id: Option<Ulid>,
                        #[serde(default)]
                        device_ready: bool,
                    }
                    let proof: LocalProof = serde_json::from_slice(&bytes)?;
                    let epoch = transition::counter(&proof.epoch)?;
                    if proof.version != 1
                        || proof.public_key != issuer.key
                        || epoch != issuer_epoch
                        || proof.key_id == Ulid::nil()
                        || proof.device_id == Some(Ulid::nil())
                    {
                        return Err(invalid("stale local public device proof"));
                    }
                    let history = row(&format!("identity/local-issuer-history/{epoch}"))?
                        .ok_or_else(|| invalid("missing local public device history"))?;
                    if history != bytes {
                        return Err(invalid("stale local public device history"));
                    }
                    let _public_custody_kind = proof.custody;
                    fence.device = proof.device_id.filter(|_| proof.device_ready);
                    fence.device_epoch = fence.device.map(|_| epoch);
                    fence.proof = crate::store::digest(&bytes);
                    fence.grant = Some(fence.proof.clone());
                }
            }
            PublicPin::Certified {
                issuer: expected,
                pinned,
                device,
                grant,
            } => {
                if issuer.kind != IssuerKind::GpgRoot || issuer != expected {
                    return Err(invalid("stale host-authenticated certified issuer"));
                }
                let state = state.ok_or_else(|| invalid("missing public issuer state"))?;
                if state.root_fingerprint.as_deref() != Some(pinned.as_str()) {
                    return Err(invalid("stale public root fingerprint"));
                }
                let head_bytes =
                    row("identity/head")?.ok_or_else(|| invalid("missing public account head"))?;
                let head: Signed<AccountHead> = serde_json::from_slice(&head_bytes)?;
                let identity_bytes = row(&format!("identity/device/{device}"))?
                    .ok_or_else(|| invalid("missing public device proof"))?;
                let identity: DeviceIdentity = serde_json::from_slice(&identity_bytes)?;
                if identity.grant.payload.device_id != device
                    || identity.grant.digest != grant
                    || state.account_head_id.as_deref() != Some(head.digest.as_str())
                {
                    return Err(invalid("stale host-authenticated device grant"));
                }
                // Shared with normal device_current(), but this port has no key
                // loader and every input was bounded before allocation/serde.
                if now >= identity.grant.payload.expires_at {
                    self.invalidate_publication(revision);
                    return Err(invalid("expired public DeviceGrant"));
                }
                checkpoint()?;
                super::chain::verify_public_device_cooperative(
                    &identity, &pinned, &head, now, checkpoint,
                )?;
                checkpoint()?;
                let payload = &identity.grant.payload;
                fence.account_epoch = transition::counter(&payload.account_epoch)?;
                fence.device = Some(device);
                fence.device_epoch = Some(transition::counter(&payload.device_epoch)?);
                fence.proof = crate::store::digest(&head_bytes);
                fence.grant = Some(grant);
                fence.not_before = payload.not_before;
                fence.expires_at = Some(payload.expires_at);
            }
        }
        checkpoint()?;
        let rows = verified_public_rows(&db, &fence)?;
        if read_rows.iter().any(|read| {
            !rows
                .iter()
                .any(|row| row.key == read.key && row.digest == read.digest)
        }) {
            return Err(invalid("public proof changed during verification"));
        }
        if self.0.publication.load(Ordering::Acquire) != revision
            || public_file(&self.0.account)? != file
            || public_revision(&db)? != fence_rev
        {
            return Err(invalid("stale public identity publication"));
        }
        // Only this successful complete verification publishes a cache entry.
        let verified = Arc::new(VerifiedPublicFence {
            fence: fence.clone(),
            verified_at: now,
            file,
            rows,
        });
        let mut cache = self
            .0
            .verified
            .try_lock()
            .map_err(|_| invalid("stale public identity cache publication"))?;
        if self.0.publication.load(Ordering::Acquire) != revision {
            return Err(invalid("stale public identity publication"));
        }
        // A slower verifier may finish after a newer verification of identical
        // facts. Keep the later time so a real rollback is never forgotten.
        if cache
            .as_ref()
            .is_none_or(|prior| prior.fence != fence || prior.verified_at <= now)
        {
            *cache = Some(verified);
        }
        // No SQL borrow survives these retirement boundaries. The complete
        // verified candidate is published, but cannot escape a failed verdict.
        drop(cache);
        db.set_prepared_statement_cache_capacity(1);
        checkpoint()?;
        db.set_prepared_statement_cache_capacity(0);
        checkpoint()?;
        drop(db);
        checkpoint()?;
        Ok(fence)
    }
    fn republish_from(&self, replacement: &Self) {
        use std::sync::atomic::Ordering;
        // Odd means unavailable. A contended/poisoned publication remains stale
        // rather than blocking an identity owner or accepting an old snapshot.
        let Ok(previous) =
            self.0
                .publication
                .try_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                    value.checked_add(if value & 1 == 0 { 1 } else { 2 })
                })
        else {
            return;
        };
        let publishing = match previous.checked_add(if previous & 1 == 0 { 1 } else { 2 }) {
            Some(value) if value < u64::MAX => value,
            _ => return,
        };
        let replacement = if replacement.0.installed {
            replacement
                .0
                .pin
                .try_lock()
                .ok()
                .and_then(|pin| pin.clone())
        } else {
            None
        };
        let Ok(mut pin) = self.0.pin.try_lock() else {
            return;
        };
        if self.0.publication.load(Ordering::Acquire) != publishing {
            return;
        }
        *pin = replacement;
        let _ = self.0.publication.compare_exchange(
            publishing,
            publishing + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}

// Diagnostic-only fixed storage; logs are bounded to sixteen records per host
// reader. Logging/symbol-free formatting changes timings and is never gate proof.
#[cfg(test)]
fn guard_profile_sample(profile: &mut Option<(u64, [std::time::Duration; 9])>, index: usize) {
    if let Some((_, times)) = profile {
        times[index] = bbt_rt::fault::thread_cpu_time().unwrap_or(std::time::Duration::ZERO);
    }
}

fn public_connection(account: &SqliteStore) -> Result<rusqlite::Connection> {
    public_connection_cooperative(account, &mut || Ok(()))
}
fn public_connection_cooperative(
    account: &SqliteStore,
    checkpoint: &mut dyn FnMut() -> Result<()>,
) -> Result<rusqlite::Connection> {
    let db = rusqlite::Connection::open_with_flags(
        &account.path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    checkpoint()?;
    db.busy_timeout(std::time::Duration::ZERO)?;
    checkpoint()?;
    db.pragma_update(None, "cache_size", -1024i64)?;
    checkpoint()?;
    db.pragma_update(None, "mmap_size", 0i64)?;
    checkpoint()?;
    Ok(db)
}
fn public_connection_current(db: &rusqlite::Connection) -> Result<()> {
    // SQLite's native Unix VFS compares the opened file's inode to its path.
    // Path metadata alone cannot prove that a retained connection still reads
    // that file after a replacement. Unsupported VFS controls fail closed.
    let mut moved: std::ffi::c_int = 1;
    // SAFETY: the connection remains exclusively borrowed/alive for this call;
    // HAS_MOVED synchronously writes one c_int to the correctly typed pointer.
    let result = unsafe {
        rusqlite::ffi::sqlite3_file_control(
            db.handle(),
            c"main".as_ptr(),
            rusqlite::ffi::SQLITE_FCNTL_HAS_MOVED,
            (&mut moved as *mut std::ffi::c_int).cast(),
        )
    };
    if result != rusqlite::ffi::SQLITE_OK || moved != 0 {
        return Err(invalid("stale public account connection"));
    }
    Ok(())
}
const PUBLIC_REVISION_SQL: &str = "SELECT revision FROM dream_public_fence WHERE singleton=1";
const PUBLIC_DIGEST_SQL: &str = "SELECT length(value),CASE WHEN typeof(value)='blob' AND length(value)<=16384 THEN value ELSE NULL END FROM kv WHERE key=?1";
fn public_revision(db: &rusqlite::Connection) -> Result<u64> {
    let revision: i64 = db
        .prepare_cached(PUBLIC_REVISION_SQL)?
        .query_row([], |r| r.get(0))?;
    u64::try_from(revision)
        .ok()
        .filter(|value| *value > 0 && *value < 9_007_199_254_740_991)
        .ok_or_else(|| invalid("stale public identity revision"))
}
/// Hash the borrowed bounded cell: no row Vec/String is cloned on a unit check.
fn public_digest(db: &rusqlite::Connection, key: &str) -> Result<Option<[u8; 32]>> {
    use rusqlite::OptionalExtension;
    use sha2::{Digest, Sha256};
    Ok(db
        .prepare_cached(PUBLIC_DIGEST_SQL)?
        .query_row([key], |row| {
            let length: i64 = row.get(0)?;
            if !(0..=PUBLIC_ROW_LIMIT as i64).contains(&length) {
                return Err(rusqlite::Error::InvalidQuery);
            }
            let cell = row.get_ref(1)?.as_blob()?;
            if cell.len() != length as usize {
                return Err(rusqlite::Error::InvalidQuery);
            }
            Ok(Sha256::digest(cell).into())
        })
        .optional()?)
}
fn verified_public_rows(
    db: &rusqlite::Connection,
    fence: &PublicFence,
) -> Result<Vec<PublicRowDigest>> {
    let mut keys = Vec::new();
    keys.try_reserve_exact(8)
        .map_err(|_| invalid("public identity cache capacity"))?;
    for key in [
        "messaging/issuer",
        "identity/issuer",
        "identity/head",
        "identity/local-issuer",
    ] {
        keys.push(key.to_string());
    }
    keys.push(format!("identity/issuer-history/{}", fence.issuer_epoch));
    keys.push(format!(
        "identity/local-issuer-history/{}",
        fence.issuer_epoch
    ));
    if let Some(device) = fence.device {
        keys.push(format!("identity/device/{device}"));
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(8)
        .map_err(|_| invalid("public identity cache capacity"))?;
    for key in keys {
        let digest = public_digest(db, &key)?;
        rows.push(PublicRowDigest { key, digest });
    }
    Ok(rows)
}

fn public_file(account: &SqliteStore) -> Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(&account.path)?;
    if !metadata.file_type().is_file() {
        return Err(invalid("invalid public account file"));
    }
    Ok((metadata.dev(), metadata.ino()))
}

#[cfg(test)]
struct DreamSigningSpy {
    inner: Arc<dyn SeatKeys>,
    calls: Arc<std::sync::atomic::AtomicU64>,
    trace: Arc<std::sync::atomic::AtomicUsize>,
}
#[cfg(test)]
impl DreamSigningSpy {
    #[inline(never)]
    fn observed(&self, method: &'static str) {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        IdentityRuntime::test_dream_spy_trace(&self.trace, method);
    }
}
#[cfg(test)]
impl SeatKeys for DreamSigningSpy {
    fn test_account_label(&self) -> Option<&str> {
        self.inner.test_account_label()
    }
    fn issuer(&self) -> crate::messaging::wire::Issuer {
        self.inner.issuer()
    }
    fn ready_issuer(&self) -> Option<crate::messaging::wire::Issuer> {
        self.observed("seat_keys.ready_issuer");
        self.inner.ready_issuer()
    }
    fn signing_hold(&self) -> Option<String> {
        self.observed("seat_keys.signing_hold");
        self.inner.signing_hold()
    }
    fn leaf(&self, seat: Ulid, generation: u64) -> Result<ed25519_dalek::SigningKey> {
        self.observed("seat_keys.leaf");
        self.inner.leaf(seat, generation)
    }
    fn verify(&self, envelope: &crate::messaging::wire::Envelope) -> Result<()> {
        self.inner.verify(envelope)
    }
    fn sign_envelope(
        &self,
        envelope: &mut crate::messaging::wire::Envelope,
        key: &ed25519_dalek::SigningKey,
    ) -> Result<()> {
        self.observed("seat_keys.sign_envelope");
        self.inner.sign_envelope(envelope, key)
    }
    fn finalize(
        &self,
        envelope: &mut crate::messaging::wire::Envelope,
        record: &crate::registry::SeatRecord,
        tx: &rusqlite::Transaction<'_>,
    ) -> Result<()> {
        self.observed("seat_keys.finalize");
        self.inner.finalize(envelope, record, tx)
    }
}

/// SELECT length/value_ref rejects the row before any Vec clone or JSON parse.
/// This query touches one indexed key only; there is no account/table scan.
fn public_row(db: &rusqlite::Connection, key: &str) -> Result<Option<Vec<u8>>> {
    use rusqlite::OptionalExtension;
    Ok(db.query_row("SELECT length(value),CASE WHEN typeof(value)='blob' AND length(value)<=16384 THEN value ELSE NULL END FROM kv WHERE key=?1", [key], |row| {
        let length: i64 = row.get(0)?;
        if !(0..=PUBLIC_ROW_LIMIT as i64).contains(&length) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let value = row.get_ref(1)?.as_blob()?;
        if value.len() != length as usize || value.len() > PUBLIC_ROW_LIMIT {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(value.to_vec())
    }).optional()?)
}

fn install_public_fence(account: &SqliteStore) -> Result<()> {
    // Revision changes atomically with only public-authority rows. Unlike a
    // sampled digest or data_version this retains unseen A→B→A, without causing
    // unrelated message writes to invalidate every C1 job. Overflow fails the
    // authority mutation closed, never wraps to an earlier incarnation.
    let mut db = account.connection()?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS dream_public_fence(
           singleton INTEGER PRIMARY KEY CHECK(singleton=1),
           revision INTEGER NOT NULL CHECK(revision>0 AND revision<9007199254740991));
         INSERT OR IGNORE INTO dream_public_fence VALUES(1,1);
         CREATE TRIGGER IF NOT EXISTS dream_public_insert AFTER INSERT ON kv
         WHEN NEW.key IN ('identity/head','identity/issuer','messaging/issuer','identity/local-issuer')
           OR NEW.key GLOB 'identity/device/*' OR NEW.key GLOB 'identity/issuer-history/*'
           OR NEW.key GLOB 'identity/local-issuer-history/*'
         BEGIN UPDATE dream_public_fence SET revision=revision+1 WHERE singleton=1; END;
         CREATE TRIGGER IF NOT EXISTS dream_public_update AFTER UPDATE ON kv
         WHEN OLD.key IN ('identity/head','identity/issuer','messaging/issuer','identity/local-issuer')
           OR NEW.key IN ('identity/head','identity/issuer','messaging/issuer','identity/local-issuer')
           OR OLD.key GLOB 'identity/device/*' OR NEW.key GLOB 'identity/device/*'
           OR OLD.key GLOB 'identity/issuer-history/*' OR NEW.key GLOB 'identity/issuer-history/*'
           OR OLD.key GLOB 'identity/local-issuer-history/*' OR NEW.key GLOB 'identity/local-issuer-history/*'
         BEGIN UPDATE dream_public_fence SET revision=revision+1 WHERE singleton=1; END;
         CREATE TRIGGER IF NOT EXISTS dream_public_delete AFTER DELETE ON kv
         WHEN OLD.key IN ('identity/head','identity/issuer','messaging/issuer','identity/local-issuer')
           OR OLD.key GLOB 'identity/device/*' OR OLD.key GLOB 'identity/issuer-history/*'
           OR OLD.key GLOB 'identity/local-issuer-history/*'
         BEGIN UPDATE dream_public_fence SET revision=revision+1 WHERE singleton=1; END;"
    )?;
    tx.commit()?;
    Ok(())
}

/// Held by the logic IO owner. Neither device custody nor this provider is
/// exposed over the seat/MCP API; the GUI receives only IdentityStatus.
pub struct IdentityRuntime {
    pub(super) account: SqliteStore,
    pub(super) dream_fence: PublicFenceReader,
    pub(super) keys: Arc<dyn SeatKeys>,
    pub(super) chain: Option<Arc<ChainSeatKeys>>,
    pub(super) retry: Option<super::custodied::Retry>,
    pub(super) pending: Option<PendingReason>,
    pub(super) bus_key: Option<ed25519_dalek::SigningKey>,
    pub(super) observed_status: std::sync::Mutex<Option<IdentityFields>>,
    pub(super) reopen_failed: Option<bbt_proto::identity::ReopenFailure>,
}
impl IdentityRuntime {
    pub(crate) fn dream_fence(&self) -> PublicFenceReader {
        self.dream_fence.clone()
    }
    #[cfg(test)]
    pub(crate) fn test_dream_open_with_store(
        root: &std::path::Path,
        local: ed25519_dalek::SigningKey,
        handle: tokio::runtime::Handle,
        custody: Arc<dyn super::DeviceKeyStore + Send + Sync>,
    ) -> Result<Self> {
        Self::open_with_store(root, local, handle, Some(custody))
    }
    /// Counts real private/signing entry points after a disposable bootstrap.
    /// The normal provider remains installed; controls do not receive a fake key.
    #[cfg(test)]
    pub(crate) fn test_dream_signing_spy(
        &mut self,
        calls: Arc<std::sync::atomic::AtomicU64>,
        trace: Arc<std::sync::atomic::AtomicUsize>,
    ) {
        self.keys = Arc::new(DreamSigningSpy {
            inner: self.keys.clone(),
            calls,
            trace,
        });
    }
    /// Diagnostic only, never changes the zero-call counters. At most eight
    /// records of 8 KiB after fixture arm; std backtrace capture/symbolization
    /// is opaque test overhead and is not included in memory/latency evidence.
    #[cfg(test)]
    #[inline(never)]
    pub(crate) fn test_dream_spy_trace(
        trace: &std::sync::atomic::AtomicUsize,
        method: &'static str,
    ) {
        use std::{fmt::Write, sync::atomic::Ordering};
        if std::env::var_os("BBOXY_DREAM_SPY_TRACE").is_none() {
            return;
        }
        let Ok(record) = trace.try_update(Ordering::AcqRel, Ordering::Acquire, |count| {
            (count < 8).then(|| count + 1)
        }) else {
            return;
        };
        struct Bounded {
            bytes: [u8; 8192],
            len: usize,
        }
        impl std::fmt::Write for Bounded {
            fn write_str(&mut self, text: &str) -> std::fmt::Result {
                let remaining = self.bytes.len() - self.len;
                let mut count = remaining.min(text.len());
                while !text.is_char_boundary(count) {
                    count -= 1;
                }
                self.bytes[self.len..self.len + count].copy_from_slice(&text.as_bytes()[..count]);
                self.len += count;
                if count < text.len() {
                    Err(std::fmt::Error)
                } else {
                    Ok(())
                }
            }
        }
        let mut output = Bounded {
            bytes: [0; 8192],
            len: 0,
        };
        let backtrace = std::backtrace::Backtrace::force_capture();
        let _ = write!(&mut output, "{backtrace}");
        let thread = std::thread::current();
        eprintln!(
            "DREAM_SPY record={record} method={method} thread={:?} trace_bytes={}\n{}",
            thread.name(),
            output.len,
            String::from_utf8_lossy(&output.bytes[..output.len])
        );
    }
    /// Keep already-cloned readers connected across successful host reopen.
    pub(crate) fn inherit_dream_fence(&mut self, previous: &IdentityRuntime) {
        previous.dream_fence.republish_from(&self.dream_fence);
        self.dream_fence = previous.dream_fence.clone();
    }
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
            dream_fence: PublicFenceReader::new(account.clone(), chain.dream_public_pin().ok()),
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

#[cfg(test)]
mod dream_public_fence_tests {
    use super::*;
    use crate::identity::{DeviceKeyStore, TestAccount};
    use crate::messaging::{
        keys::LocalSeatKeys,
        wire::{Envelope, Issuer},
    };
    use ed25519_dalek::SigningKey;

    struct NoCustody;
    impl DeviceKeyStore for NoCustody {
        fn custody(&self) -> Custody {
            panic!("C1 must not query custody")
        }
        fn load(&self, _: Ulid) -> crate::identity::Result<Option<SigningKey>> {
            panic!("C1 must not load any private key")
        }
        fn create(&self, _: Ulid, _: &SigningKey) -> crate::identity::Result<()> {
            panic!("C1 must not create a private key")
        }
    }
    struct NoSigning;
    impl SeatKeys for NoSigning {
        fn issuer(&self) -> Issuer {
            panic!("reader must use host public pins")
        }
        fn ready_issuer(&self) -> Option<Issuer> {
            panic!("ready_issuer is not a public fence")
        }
        fn signing_hold(&self) -> Option<String> {
            panic!("signing_hold may load custody")
        }
        fn leaf(&self, _: Ulid, _: u64) -> Result<SigningKey> {
            panic!("no C1 signing")
        }
        fn verify(&self, _: &Envelope) -> Result<()> {
            panic!("no C1 envelope verification")
        }
    }
    fn certified() -> (tempfile::TempDir, SqliteStore, IdentityRuntime) {
        let root = tempfile::tempdir().unwrap();
        let canonical = root.path().canonicalize().unwrap();
        let run = canonical.join("run");
        let _generated = TestAccount::open(&canonical.join("keys"), &run).unwrap();
        let account = SqliteStore::open(&run.join("account")).unwrap();
        let pinned = transition::read(&account)
            .unwrap()
            .unwrap()
            .root_fingerprint
            .unwrap();
        let chain = Arc::new(
            ChainSeatKeys::open(
                account.clone(),
                Arc::new(NoCustody),
                &pinned,
                SigningKey::from_bytes(&[92; 32]),
            )
            .unwrap(),
        );
        let runtime = IdentityRuntime {
            dream_fence: PublicFenceReader::new(
                account.clone(),
                Some(chain.dream_public_pin().unwrap()),
            ),
            account: account.clone(),
            keys: Arc::new(NoSigning),
            chain: Some(chain),
            retry: None,
            pending: None,
            bus_key: None,
            observed_status: Default::default(),
            reopen_failed: None,
        };
        (root, account, runtime)
    }
    fn legacy() -> (tempfile::TempDir, SqliteStore, PublicFenceReader, Issuer) {
        let root = tempfile::tempdir().unwrap();
        let account = SqliteStore::open(&root.path().canonicalize().unwrap()).unwrap();
        let issuer = LocalSeatKeys::new(SigningKey::from_bytes(&[93; 32])).issuer();
        account
            .put("messaging/issuer", &serde_json::to_vec(&issuer).unwrap())
            .unwrap();
        let reader =
            PublicFenceReader::new(account.clone(), Some(PublicPin::Local(issuer.clone())));
        (root, account, reader, issuer)
    }

    #[test]
    fn dream_public_cooperative_prelude_rechecks_rows_and_aba_after_yield() {
        for raw_change in [false, true] {
            let (_root, account, reader, _) = legacy();
            let original = account.get("messaging/issuer").unwrap().unwrap();
            let mut checkpoints = 0;
            let result = reader.current_cooperative(&mut || {
                checkpoints += 1;
                if checkpoints == 9 {
                    // The issuer has already been captured, while every read
                    // statement has ended. This separate owner never waits for
                    // a cold-proof read transaction through a cooperative rest.
                    let db = account.connection()?;
                    db.busy_timeout(std::time::Duration::ZERO)?;
                    if raw_change {
                        db.execute_batch("DROP TRIGGER dream_public_update")?;
                        db.execute(
                            "UPDATE kv SET value=?1 WHERE key='messaging/issuer'",
                            [b"raw changed issuer".as_slice()],
                        )?;
                    } else {
                        db.execute(
                            "UPDATE kv SET value=?1 WHERE key='messaging/issuer'",
                            [b"temporary B issuer".as_slice()],
                        )?;
                        db.execute(
                            "UPDATE kv SET value=?1 WHERE key='messaging/issuer'",
                            [&original],
                        )?;
                    }
                }
                Ok(())
            });
            assert!(
                checkpoints >= 9,
                "cold proof did not reach the causal yield"
            );
            assert!(result.is_err(), "changed cold-proof input was published");
            assert!(reader.verified_snapshot().unwrap().is_none());
        }
    }

    // A real certified account, including its OpenPGP public certificates, is
    // checked while every custody/signing entry point is a panic spy.
    #[test]
    fn dream_public_fence_reads_certified_proof_without_custody_or_signing() {
        let (_root, _account, runtime) = certified();
        let reader = runtime.dream_fence();
        let first = reader.current().unwrap();
        assert_eq!(first.issuer.kind, IssuerKind::GpgRoot);
        assert!(first.device.is_some());
        assert_eq!(first.account_epoch, 1);
        assert_eq!(first.device_epoch, Some(1));
        assert_eq!(first.proof.len(), 64);
        assert!(first.grant.is_some());
        assert_eq!(reader.current().unwrap(), first);
        let serialized = serde_json::to_vec(&first).unwrap();
        assert_eq!(
            serde_json::from_slice::<PublicFence>(&serialized).unwrap(),
            first
        );
        assert!(
            reader.current_at(first.expires_at.unwrap()).is_err(),
            "expiry is public, not custody"
        );
    }
    #[test]
    fn dream_public_fence_rejects_revoked_invalid_head_and_retains_unseen_aba_revision() {
        let (_root, account, runtime) = certified();
        let reader = runtime.dream_fence();
        let initial = reader.current().unwrap();
        let original = account.get("identity/head").unwrap().unwrap();
        let mut head: Signed<AccountHead> = serde_json::from_slice(&original).unwrap();
        head.payload.revoked_devices.push(initial.device.unwrap());
        account
            .put("identity/head", &serde_json::to_vec(&head).unwrap())
            .unwrap();
        assert!(
            reader.current().is_err(),
            "revoked/invalid authority cannot be presented"
        );
        account.put("identity/head", &original).unwrap();
        let restored = reader.current().unwrap();
        assert!(restored.fence_rev > initial.fence_rev);
        assert_eq!(restored.proof, initial.proof);
        // No reader samples B here. Same-byte A cannot hide the two commits.
        account
            .put("identity/head", b"invalid intermediate B")
            .unwrap();
        account.put("identity/head", &original).unwrap();
        assert!(reader.current().unwrap().fence_rev >= restored.fence_rev + 2);
    }
    #[test]
    fn dream_public_fence_bounds_rows_before_clone_and_rejects_missing_publication() {
        let (_root, account, runtime) = certified();
        let reader = runtime.dream_fence();
        let original = account.get("identity/head").unwrap().unwrap();
        account
            .put("identity/head", &vec![b'x'; PUBLIC_ROW_LIMIT + 1])
            .unwrap();
        assert!(reader.current().is_err());
        let db = account.connection().unwrap();
        assert!(public_row(&db, "identity/head").is_err());
        drop(db);
        account.put("identity/head", &original).unwrap();
        assert!(reader.current().is_ok());
        let missing = PublicFenceReader::new(account, None);
        assert!(missing.current().is_err());
    }
    #[test]
    fn dream_public_fence_local_legacy_pin_and_epoch_are_explicit_and_aba_safe() {
        let (_root, account, reader, issuer) = legacy();
        let first = reader.current().unwrap();
        assert_eq!(first.account_epoch, 1);
        assert_eq!(first.device, None);
        assert_eq!(first.expires_at, None);
        account
            .put("unrelated/message", b"no identity invalidation")
            .unwrap();
        assert_eq!(reader.current().unwrap(), first);
        let foreign = LocalSeatKeys::new(SigningKey::from_bytes(&[94; 32])).issuer();
        account
            .put("messaging/issuer", &serde_json::to_vec(&foreign).unwrap())
            .unwrap();
        assert!(reader.current().is_err());
        account
            .put("messaging/issuer", &serde_json::to_vec(&issuer).unwrap())
            .unwrap();
        let restored = reader.current().unwrap();
        assert_eq!(restored.proof, first.proof);
        assert!(restored.fence_rev >= first.fence_rev + 2);
    }
    #[test]
    fn dream_public_fence_republish_keeps_cloned_reader_and_invalidates_old_stamp() {
        let (_root, account, reader, issuer) = legacy();
        let clone = reader.clone();
        let first = clone.current().unwrap();
        let replacement = PublicFenceReader::new(account.clone(), Some(PublicPin::Local(issuer)));
        reader.republish_from(&replacement);
        let next = clone.current().unwrap();
        assert_eq!(next.daemon_incarnation, first.daemon_incarnation);
        assert_eq!(next.proof, first.proof);
        assert!(next.publication_rev > first.publication_rev);
        assert_eq!(reader.cached_revision(), next.publication_rev);
        reader.republish_from(&PublicFenceReader::new(account, None));
        assert!(
            clone.current().is_err(),
            "a missing host proof revokes all old clones"
        );
    }
    #[test]
    fn dream_public_still_current_requires_verified_snapshot_and_rejects_sql_aba() {
        let (_root, account, runtime) = certified();
        let reader = runtime.dream_fence();
        let captured = reader.current().unwrap();
        assert!(reader.still_current(&captured));
        let mut forged = captured.clone();
        forged.proof = "0".repeat(64);
        assert!(
            !reader.still_current(&forged),
            "a journal stamp alone is not a verified proof"
        );
        let original = account.get("identity/head").unwrap().unwrap();
        account.put("identity/head", b"intermediate B").unwrap();
        account.put("identity/head", &original).unwrap();
        assert!(
            !reader.still_current(&captured),
            "unseen SQL A to B to A reused a stamp"
        );
        let next = reader.current().unwrap();
        assert!(next.fence_rev > captured.fence_rev);
        assert!(reader.still_current(&next));
        let verified = reader.0.verified.try_lock().unwrap().clone().unwrap();
        assert!(verified.rows.len() <= 8);
        assert!(
            verified
                .rows
                .iter()
                .any(|row| row.key == "identity/local-issuer")
        );
    }
    #[test]
    fn dream_public_still_current_hashes_raw_same_inode_rows_and_never_revives_old_stamp() {
        use std::os::unix::fs::MetadataExt;
        let (_root, account, runtime) = certified();
        let reader = runtime.dream_fence();
        let captured = reader.current().unwrap();
        let original = account.get("identity/head").unwrap().unwrap();
        let inode = std::fs::metadata(&account.path).unwrap().ino();
        let db = account.connection().unwrap();
        // Deliberately bypass normal authority triggers to model an external
        // raw-row writer. The unchanged revision alone must not bless the cache.
        db.execute_batch("DROP TRIGGER dream_public_update")
            .unwrap();
        db.execute(
            "UPDATE kv SET value=?1 WHERE key='identity/head'",
            [b"raw different head".as_slice()],
        )
        .unwrap();
        assert_eq!(public_revision(&db).unwrap(), captured.fence_rev);
        assert_eq!(std::fs::metadata(&account.path).unwrap().ino(), inode);
        assert!(!reader.still_current(&captured));
        assert_eq!(reader.cached_revision() & 1, 1);
        db.execute(
            "UPDATE kv SET value=?1 WHERE key='identity/head'",
            [&original],
        )
        .unwrap();
        assert!(
            !reader.still_current(&captured),
            "raw A bytes revived an invalidated publication"
        );
        assert!(reader.current().is_err());
        drop(db);
        // Only a separately host-constructed public pin can refresh the odd
        // publication. Its new revision still rejects the old retained stamp.
        let replacement = PublicFenceReader::new(
            account,
            runtime.chain.as_ref().unwrap().dream_public_pin().ok(),
        );
        reader.republish_from(&replacement);
        let refreshed = reader.current().unwrap();
        assert!(refreshed.publication_rev > captured.publication_rev);
        assert!(reader.still_current(&refreshed));
        assert!(!reader.still_current(&captured));
    }
    #[test]
    fn dream_public_still_current_observed_expiry_and_clock_rollback_are_monotonic() {
        let (_root, _account, runtime) = certified();
        let reader = runtime.dream_fence();
        let captured = reader.current().unwrap();
        let now = reader
            .0
            .verified
            .try_lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .verified_at;
        assert!(reader.still_current_at(&captured, now));
        assert!(!reader.still_current_at(&captured, captured.expires_at.unwrap()));
        assert_eq!(reader.cached_revision() & 1, 1);
        assert!(
            !reader.still_current_at(&captured, now),
            "clock rollback revived expired authority"
        );
        assert!(reader.current_at(now).is_err());

        let (_root, _account, runtime) = certified();
        let reader = runtime.dream_fence();
        let captured = reader.current().unwrap();
        let now = reader
            .0
            .verified
            .try_lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .verified_at;
        assert!(!reader.still_current_at(&captured, now - 1));
        assert_eq!(reader.cached_revision() & 1, 1);
        assert!(!reader.still_current_at(&captured, now));
    }
    #[test]
    fn dream_public_forward_clock_cache_publication_cannot_false_revoke_inflight_guard() {
        use std::sync::atomic::Ordering;
        let (_root, _account, runtime) = certified();
        let reader = runtime.dream_fence();
        let now = crate::store::now_ms() / 1000;
        reader.0.public_clock.store(now, Ordering::Release);
        let captured = reader.current().unwrap();
        let publication = reader.cached_revision();
        let (at_cache, waiting) = std::sync::mpsc::sync_channel(1);
        let (release, released) = std::sync::mpsc::sync_channel(1);
        std::thread::scope(|scope| {
            let guard_reader = reader.clone();
            let guard_fence = captured.clone();
            let guard = scope.spawn(move || {
                // Same snapshot loader and production clock path as
                // still_current(). Stop just before its cache read.
                guard_reader.still_current_from(&guard_fence, || {
                    at_cache.send(()).unwrap();
                    released
                        .recv_timeout(std::time::Duration::from_secs(5))
                        .unwrap();
                    guard_reader.verified_snapshot()
                })
            });
            waiting
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            // Advance the real clock source used by the production method,
            // then publish a successfully verified snapshot in that next second.
            // No test clock setter/publication invalidation is used here.
            reader.0.public_clock.store(now + 1, Ordering::Release);
            assert_eq!(reader.current().unwrap(), captured);
            release.send(()).unwrap();
            assert!(
                guard.join().unwrap(),
                "forward time was mistaken for rollback"
            );
        });
        assert_eq!(reader.cached_revision(), publication);
        assert_eq!(publication & 1, 0);
        assert!(reader.still_current(&captured));
        // The explicit-time seam still rejects an actual rollback against the
        // successfully published later verification.
        assert!(!reader.still_current_at(&captured, now));
        assert_eq!(reader.cached_revision() & 1, 1);
    }
    #[test]
    fn dream_public_with_current_reads_one_transaction_and_rechecks_after_callback() {
        let (_root, account, runtime) = certified();
        let reader = runtime.dream_fence();
        let captured = reader.current().unwrap();
        let revision = reader
            .with_current(&captured, |db| {
                assert!(
                    !db.is_autocommit(),
                    "metadata escaped the public read transaction"
                );
                public_revision(db)
            })
            .unwrap();
        assert_eq!(revision, captured.fence_rev);
        let original = account.get("identity/head").unwrap().unwrap();
        // A concurrent SQL owner commits after the callback's old-snapshot
        // reads. Its normal revision change must suppress the produced value.
        let result = reader.with_current(&captured, |db| {
            assert_eq!(public_revision(db)?, captured.fence_rev);
            account.put("identity/head", &original)?;
            Ok("this stale metadata must never leave the guard")
        });
        assert!(result.is_err());
        assert!(
            reader
                .with_current(&captured, |_| -> Result<()> {
                    panic!("a stale proof invoked the metadata callback")
                })
                .is_err()
        );
    }

    #[test]
    fn dream_public_prepared_session_releases_transaction_and_lock_between_fresh_units() {
        let (_root, account, reader, _) = legacy();
        let captured = reader.current().unwrap();
        let session = reader
            .prepare_session(|db| {
                drop(db.prepare_cached("SELECT 1")?);
                Ok(())
            })
            .unwrap();
        for _ in 0..2 {
            assert_eq!(
                reader
                    .with_current_session(&captured, &session, |db| {
                        assert!(!db.is_autocommit());
                        public_revision(db)
                    })
                    .unwrap(),
                captured.fence_rev
            );
            let db = session.db.try_lock().unwrap();
            assert!(
                db.as_ref().unwrap().is_autocommit(),
                "a unit retained its SQLite snapshot"
            );
        }
        let held = session.db.try_lock().unwrap();
        let start = std::time::Instant::now();
        assert!(
            reader
                .try_with_current_session(&captured, &session, |_| -> Result<()> {
                    panic!("busy session cannot execute an authority callback")
                })
                .unwrap()
                .is_none(),
            "contention has a typed unavailable outcome rather than stale permission"
        );
        assert!(
            reader
                .with_current_session(&captured, &session, |_| -> Result<()> {
                    panic!("contended session invoked the actor callback")
                })
                .is_err()
        );
        assert!(start.elapsed() < std::time::Duration::from_millis(250));
        drop(held);

        // The connection and prepared statements are reused, not their values.
        // A second actual SQL owner changes authority during this transaction;
        // the post-commit revision check must suppress the callback's result.
        let issuer = account.get("messaging/issuer").unwrap().unwrap();
        assert!(
            reader
                .with_current_session(&captured, &session, |db| {
                    assert_eq!(public_revision(db)?, captured.fence_rev);
                    account.put("messaging/issuer", &issuer)?;
                    Ok("old actor metadata")
                })
                .is_err()
        );
        assert!(
            session
                .db
                .try_lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .is_autocommit()
        );
    }

    #[test]
    fn dream_public_prepared_session_retirement_waits_for_real_borrow_and_closes_once() {
        let (_root, _account, reader, _) = legacy();
        let captured = reader.current().unwrap();
        let session = Arc::new(reader.prepare_session(|_| Ok(())).unwrap());
        let control = Arc::new(crate::logic::dream_c1::Control::default());
        let (was_busy, finished_early, refused_during_retire, result) =
            std::thread::scope(|scope| {
                let held = session.db.try_lock().unwrap();
                let was_busy = matches!(
                    session.try_take_connection(),
                    Err(bbt_proto::dream::Code::Busy)
                );
                let owned = session.clone();
                let owner_control = control.clone();
                let (sent, received) = std::sync::mpsc::sync_channel(1);
                let owner = scope.spawn(move || {
                    let result =
                        crate::logic::dream_c1::retire_public_session(&owned, &owner_control);
                    sent.send(()).unwrap();
                    result
                });
                std::thread::sleep(std::time::Duration::from_millis(20));
                let finished_early = received.try_recv().is_ok();
                let refused = reader
                    .try_with_current_session(&captured, &session, |_| Ok(()))
                    .is_err();
                // Release before asserting, so a failed causal premise never strands
                // the required settlement owner in this private test process.
                drop(held);
                (was_busy, finished_early, refused, owner.join().unwrap())
            });
        assert!(was_busy && !finished_early && refused_during_retire);
        assert!(
            result.is_ok(),
            "actual native retirement failed: {result:?}"
        );
        assert!(session.db.try_lock().unwrap().is_none());
        assert!(
            reader
                .with_current_session(&captured, &session, |_| Ok(()))
                .is_err()
        );
        let units = crate::logic::dream_c1::test_cleanup_units(&control);
        assert!(
            units >= 4,
            "cache finalization and native close were not distinct units"
        );
        assert!(crate::logic::dream_c1::retire_public_session(&session, &control).is_ok());
        assert_eq!(
            crate::logic::dream_c1::test_cleanup_units(&control),
            units,
            "a duplicate close performed another native retirement"
        );
    }

    #[test]
    fn dream_public_prepared_session_rechecks_raw_cells_and_opened_file_identity() {
        let (_root, account, reader, _) = legacy();
        let captured = reader.current().unwrap();
        let session = reader.prepare_session(|_| Ok(())).unwrap();
        assert!(
            reader
                .with_current_session(&captured, &session, |_| Ok(()))
                .is_ok()
        );
        let original = account.get("messaging/issuer").unwrap().unwrap();
        let db = account.connection().unwrap();
        db.execute_batch("DROP TRIGGER dream_public_update;")
            .unwrap();
        db.execute(
            "UPDATE kv SET value=?1 WHERE key='messaging/issuer'",
            [b"same-inode unjournaled authority bytes".as_slice()],
        )
        .unwrap();
        assert_eq!(public_revision(&db).unwrap(), captured.fence_rev);
        assert_eq!(public_file(&account).unwrap(), session.file);
        assert!(
            reader
                .with_current_session(&captured, &session, |_| -> Result<()> {
                    panic!("cached statement reused an old public cell")
                })
                .is_err()
        );
        db.execute(
            "UPDATE kv SET value=?1 WHERE key='messaging/issuer'",
            [&original],
        )
        .unwrap();
        assert!(
            reader
                .with_current_session(&captured, &session, |_| Ok(()))
                .is_err(),
            "restoring bytes revived the old publication"
        );
        drop(db);
        let retired = account.path.with_extension("private-retired");
        std::fs::rename(&account.path, &retired).unwrap();
        assert!(
            public_connection_current(session.db.try_lock().unwrap().as_ref().unwrap()).is_err()
        );
        std::fs::rename(retired, &account.path).unwrap();
        public_connection_current(session.db.try_lock().unwrap().as_ref().unwrap()).unwrap();
    }
}
