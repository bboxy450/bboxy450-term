// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Native custody's explicit operator act; no file-store or synthetic identity path.
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::store::{Store, invalid, sqlite::SqliteStore};

/// Secret-free progress is durably recorded before each migration effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Step {
    ReadCurrent,
    ReadLegacy,
    ValidateKey,
    WriteReplacement,
    VerifyReplacement,
    DeleteLegacy,
}

/// A callback failure stops the next effect, including after a caller deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Started,
    Applied,
    Refused,
}

/// OS status codes are safe to record; OS response bodies and secrets are not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressRow {
    pub step: Step,
    pub phase: Phase,
    pub status: Option<i32>,
}

/// The existing control journal owns this callback and its bounded history.
pub type Progress = Arc<dyn Fn(ProgressRow) -> super::Result<()> + Send + Sync>;

pub const CLEAR_ACTIONS: [&str; 2] = ["ctl identity unlock", "Settings › Identity › Unlock"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum TargetKind {
    Local,
    Certified,
}

/// Public journal facts retained with the original control operation, never a caller DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Reference {
    pub device: Ulid,
    pub key: Ulid,
    pub expected: [u8; 32],
    pub signer: [u8; 32],
    pub custody: super::Custody,
    kind: TargetKind,
}

/// The IO task revalidates these public journal facts before its native attempt.
#[derive(Clone)]
pub(crate) struct Target {
    pub device: Ulid,
    pub key: Ulid,
    pub expected: [u8; 32],
    pub custody: super::Custody,
    signer: [u8; 32],
    pub store: Arc<dyn super::DeviceKeyStore + Send + Sync>,
    account: SqliteStore,
    kind: TargetKind,
    retry: Option<super::custodied::Retry>,
}
fn facts(
    account: &SqliteStore,
    kind: TargetKind,
) -> crate::store::Result<(Ulid, Ulid, [u8; 32], super::Custody)> {
    let status = super::status::read(account, crate::store::now_ms() / 1000)?;
    if status.state == super::IdentityState::Held {
        return Err(invalid(
            "held: native custody authority is no longer current",
        ));
    }
    if kind == TargetKind::Local {
        let (local_device, key, public, custody) = super::local::unlock_target(account)?;
        let device = if status.bootstrapped {
            status
                .device_id
                .ok_or_else(|| invalid("held: missing certified device ID"))?
        } else {
            local_device
        };
        return Ok((device, key, public, custody));
    }
    let device = status
        .device_id
        .filter(|id| *id != Ulid::nil())
        .ok_or_else(|| invalid("held: missing validated device ID"))?;
    if !status.bootstrapped {
        return Err(invalid("held: certified device identity unavailable"));
    }
    // status::read already verified the root, head and this device grant.
    let identity: super::DeviceIdentity = serde_json::from_slice(
        &account
            .get(&format!("identity/device/{device}"))?
            .ok_or_else(|| invalid("held: missing device identity"))?,
    )?;
    Ok((
        device,
        device,
        crate::messaging::wire::parse_public(&identity.grant.payload.device_pubkey)?,
        identity.custody,
    ))
}
fn signer_fact(account: &SqliteStore, local: [u8; 32]) -> crate::store::Result<[u8; 32]> {
    let status = super::status::read(account, crate::store::now_ms() / 1000)?;
    if status.state == super::IdentityState::Held {
        return Err(invalid(
            "held: native completion authority is no longer current",
        ));
    }
    if !status.bootstrapped {
        return Ok(local);
    }
    let device = status
        .device_id
        .ok_or_else(|| invalid("held: missing verified device"))?;
    let identity: super::DeviceIdentity = serde_json::from_slice(
        &account
            .get(&format!("identity/device/{device}"))?
            .ok_or_else(|| invalid("held: missing device identity"))?,
    )?;
    crate::messaging::wire::parse_public(&identity.grant.payload.device_pubkey)
}
impl Target {
    pub(crate) fn reference(&self) -> Reference {
        Reference {
            device: self.device,
            key: self.key,
            expected: self.expected,
            signer: self.signer,
            custody: self.custody,
            kind: self.kind,
        }
    }
    pub(crate) fn revalidate(&self) -> crate::store::Result<()> {
        if facts(&self.account, self.kind)? != (self.device, self.key, self.expected, self.custody)
            || signer_fact(&self.account, self.expected)? != self.signer
        {
            return Err(invalid(
                "held: native unlock target changed before access; nothing executed",
            ));
        }
        Ok(())
    }
    pub(crate) fn run(&self, progress: Progress, kind: JobKind) -> crate::store::Result<Finished> {
        self.revalidate()?;
        let live = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let guard = live.clone();
        let authority = self.clone();
        let progress: Progress = Arc::new(move |row| {
            if !guard.load(std::sync::atomic::Ordering::Acquire) {
                return Err(super::PendingReason::KeychainTimedOut);
            }
            authority
                .revalidate()
                .map_err(|_| super::PendingReason::InvalidProof)?;
            progress(row)
        });
        let outcome = match kind {
            JobKind::Attempt => self.store.unlock(self.key, self.expected, progress),
            JobKind::Cleanup => self
                .store
                .finish_unlock(self.key, self.expected, progress)
                .map(|_| bbt_proto::identity::UnlockOutcome::Unknown),
            JobKind::Sign => Ok(bbt_proto::identity::UnlockOutcome::Unknown),
        };
        // A timed-out OS call can finish later. It must not proceed to a new migration
        // effect after the caller has already retained its unknown result.
        live.store(false, std::sync::atomic::Ordering::Release);
        let outcome = outcome.unwrap_or(bbt_proto::identity::UnlockOutcome::Unknown);
        let acl = self
            .store
            .native_acl(self.key)
            .unwrap_or(bbt_proto::identity::CustodyAcl::Unknown);
        // Reopening and signer checks stay on this IO task, not the terminal owner.
        let (runtime, reopen_failed) = match self.retry.as_ref().map(|retry| retry.reopen()) {
            Some(Ok(runtime)) => (Some(runtime), None),
            Some(Err(error)) => (None, Some(reopen_failure(&error))),
            None => (None, None),
        };
        let signer = runtime
            .as_ref()
            .and_then(|runtime| runtime.operator_signing().ok())
            .map(|authority| authority.key);
        Ok(Finished {
            outcome,
            acl,
            runtime,
            signer,
            reopen_failed,
        })
    }
}
pub(crate) struct Finished {
    pub outcome: bbt_proto::identity::UnlockOutcome,
    pub acl: bbt_proto::identity::CustodyAcl,
    pub runtime: Option<super::IdentityRuntime>,
    pub signer: Option<ed25519_dalek::SigningKey>,
    pub reopen_failed: Option<bbt_proto::identity::ReopenFailure>,
}

/// Retain a safe cause code, never arbitrary error bodies containing journal data.
pub(super) fn reopen_failure(error: &crate::store::Error) -> bbt_proto::identity::ReopenFailure {
    let reason = if let Some(rusqlite::Error::SqliteFailure(code, _)) =
        error.downcast_ref::<rusqlite::Error>()
    {
        format!("SQLite {:?} ({})", code.code, code.extended_code)
    } else if let Some(error) = error.downcast_ref::<std::io::Error>() {
        format!("identity IO {:?}", error.kind())
    } else {
        "identity journal or proof could not be reopened".into()
    };
    bbt_proto::identity::ReopenFailure { reason }
}
#[derive(Clone, Copy)]
pub(crate) enum JobKind {
    Attempt,
    Cleanup,
    Sign,
}
impl super::IdentityRuntime {
    pub(crate) fn has_native_custody_hold(&self) -> bool {
        if self.reopening_failed() {
            return false;
        }
        let reason = self.pending.or_else(|| {
            self.observed_status
                .lock()
                .ok()
                .and_then(|value| value.as_ref().and_then(|status| status.pending_reason))
        });
        matches!(
            reason,
            Some(
                super::PendingReason::KeychainUnavailable(_)
                    | super::PendingReason::KeychainInteractionRequired
                    | super::PendingReason::KeychainTimedOut
                    | super::PendingReason::KeychainSigningIdentityRequired
            )
        )
    }
    /// Admission-only ID: this never decorates or changes the public held status.
    pub(crate) fn custody_operator_device(&self) -> crate::store::Result<Ulid> {
        if let Some(id) =
            super::status::read(&self.account, crate::store::now_ms() / 1000)?.device_id
        {
            return Ok(id);
        }
        let target = self.native_unlock_target()?;
        Ok(target.device)
    }
    pub(crate) fn native_unlock_target(&self) -> crate::store::Result<Target> {
        let kind = if self.pending.is_some() || self.chain.is_none() {
            TargetKind::Local
        } else {
            TargetKind::Certified
        };
        let (device, key, expected, custody) = facts(&self.account, kind)?;
        if !matches!(
            custody,
            super::Custody::DataProtection | super::Custody::LoginKeychain
        ) {
            return Err(invalid(
                "native unlock unavailable for file or memory custody",
            ));
        }
        let retry = self.retry.clone().ok_or_else(|| {
            invalid("native unlock unavailable for explicit non-custodied identity")
        })?;
        let store = retry.store(custody)?;
        let signer = signer_fact(&self.account, expected)?;
        Ok(Target {
            device,
            key,
            expected,
            signer,
            custody,
            store,
            account: self.account.clone(),
            kind,
            retry: Some(retry),
        })
    }
    pub(crate) fn resume_unlock_target(
        &self,
        reference: &Reference,
    ) -> crate::store::Result<Target> {
        if facts(&self.account, reference.kind)?
            != (
                reference.device,
                reference.key,
                reference.expected,
                reference.custody,
            )
            || signer_fact(&self.account, reference.expected)? != reference.signer
        {
            return Err(invalid(
                "held: recorded unlock authority is no longer current",
            ));
        }
        let retry = self
            .retry
            .clone()
            .ok_or_else(|| invalid("native unlock unavailable"))?;
        let store = retry.store(reference.custody)?;
        Ok(Target {
            device: reference.device,
            key: reference.key,
            expected: reference.expected,
            signer: reference.signer,
            custody: reference.custody,
            store,
            account: self.account.clone(),
            kind: reference.kind,
            retry: Some(retry),
        })
    }
    pub(crate) fn custody_control_status(
        &self,
    ) -> crate::store::Result<bbt_proto::identity::IdentityControl> {
        // This is a public metadata read, not an implicit attempt to open custody.
        let cached = self
            .observed_status
            .lock()
            .map_err(|_| invalid("identity status cache poisoned"))?
            .clone();
        let mut status = match cached {
            Some(status) => status,
            None => super::status::read(&self.account, crate::store::now_ms() / 1000)?,
        };
        super::local::decorate(&self.account, &mut status)?;
        if let Some(reason) = self.pending {
            status.hold(reason);
        }
        self.decorate_reopen_failure(&mut status);
        let native = matches!(
            status.custody,
            Some(super::Custody::DataProtection | super::Custody::LoginKeychain)
        );
        Ok(bbt_proto::identity::IdentityControl {
            custody: status.custody_label,
            acl: if native {
                bbt_proto::identity::CustodyAcl::Unknown
            } else {
                bbt_proto::identity::CustodyAcl::NotNative
            },
            hold: status.reason_label,
            clear_actions: if native && status.pending_reason.is_some() {
                CLEAR_ACTIONS.iter().map(|s| (*s).into()).collect()
            } else {
                Vec::new()
            },
            outcome: None,
            pending: Vec::new(),
            reopen_failed: status.reopen_failed,
        })
    }
}

/// Private native seam: only the explicit legacy read may allow interaction.
pub(super) trait Vault {
    fn read(
        &self,
        account: &str,
        interactive: bool,
    ) -> super::Result<Option<ed25519_dalek::SigningKey>>;
    fn add(&self, account: &str, key: &ed25519_dalek::SigningKey) -> super::Result<()>;
    fn remove(&self, account: &str) -> super::Result<()>;
}

fn note(progress: &Progress, step: Step, phase: Phase) -> super::Result<()> {
    progress(ProgressRow {
        step,
        phase,
        status: None,
    })
}
fn checked<T>(progress: &Progress, step: Step, result: super::Result<T>) -> super::Result<T> {
    let phase = if result.is_ok() {
        Phase::Applied
    } else {
        Phase::Refused
    };
    let status = match &result {
        Err(super::PendingReason::KeychainUnavailable(code)) => Some(*code),
        Err(super::PendingReason::Cancelled) => Some(-128),
        _ => None,
    };
    progress(ProgressRow {
        step,
        phase,
        status,
    })?;
    result
}

/// Read the fixed replacement first. A refused replacement never falls back to legacy.
pub(super) fn read_preferred(
    vault: &impl Vault,
    account: &str,
) -> super::Result<Option<ed25519_dalek::SigningKey>> {
    match vault.read(&format!("{account}.dr1"), false)? {
        Some(key) => Ok(Some(key)),
        None => vault.read(account, false),
    }
}

fn validate(key: &ed25519_dalek::SigningKey, expected: &[u8; 32]) -> super::Result<()> {
    if &key.verifying_key().to_bytes() == expected {
        Ok(())
    } else {
        Err(super::PendingReason::InvalidProof)
    }
}

/// Crash recovery is noninteractive and cannot create, replace or rekey an item.
pub(super) fn finish_migration(
    vault: &impl Vault,
    account: &str,
    expected: &[u8; 32],
    progress: &Progress,
) -> super::Result<bool> {
    let replacement = format!("{account}.dr1");
    note(progress, Step::VerifyReplacement, Phase::Started)?;
    let Some(key) = checked(
        progress,
        Step::VerifyReplacement,
        vault.read(&replacement, false),
    )?
    else {
        return Ok(false);
    };
    checked(progress, Step::ValidateKey, validate(&key, expected))?;
    note(progress, Step::DeleteLegacy, Phase::Started)?;
    checked(progress, Step::DeleteLegacy, vault.remove(account))?;
    Ok(true)
}

/// The add/read/verify/delete ordering leaves a verified copy across every failure.
pub(super) fn migrate(
    vault: &impl Vault,
    account: &str,
    expected: &[u8; 32],
    progress: &Progress,
) -> super::Result<bbt_proto::identity::UnlockOutcome> {
    use super::PendingReason as E;
    use bbt_proto::identity::UnlockOutcome as O;
    let replacement = format!("{account}.dr1");
    note(progress, Step::ReadCurrent, Phase::Started)?;
    let existing = checked(progress, Step::ReadCurrent, vault.read(&replacement, false));
    let (key, already_current) = match existing {
        Ok(Some(key)) => (key, true),
        Ok(None) => {
            note(progress, Step::ReadLegacy, Phase::Started)?;
            match checked(progress, Step::ReadLegacy, vault.read(account, true)) {
                Ok(Some(key)) => (key, false),
                Ok(None) => return Err(E::KeychainUnavailable(-25300)),
                Err(E::Cancelled) => return Ok(O::Cancelled),
                Err(error) => return Err(error),
            }
        }
        // An already-migrated item's locked vault may need the one explicit access.
        Err(E::KeychainInteractionRequired) => {
            note(progress, Step::ReadCurrent, Phase::Started)?;
            match checked(progress, Step::ReadCurrent, vault.read(&replacement, true)) {
                Ok(Some(key)) => (key, true),
                Ok(None) => return Err(E::KeychainUnavailable(-25300)),
                Err(E::Cancelled) => return Ok(O::Cancelled),
                Err(error) => return Err(error),
            }
        }
        Err(error) => return Err(error),
    };
    note(progress, Step::ValidateKey, Phase::Started)?;
    checked(progress, Step::ValidateKey, validate(&key, expected))?;
    if !already_current {
        note(progress, Step::WriteReplacement, Phase::Started)?;
        match vault.add(&replacement, &key) {
            Ok(()) | Err(E::JournalConflict | E::KeychainUnavailable(-25299)) => {}
            Err(error) => return checked(progress, Step::WriteReplacement, Err(error)),
        }
        note(progress, Step::WriteReplacement, Phase::Applied)?;
    }
    note(progress, Step::VerifyReplacement, Phase::Started)?;
    let replacement_key = checked(
        progress,
        Step::VerifyReplacement,
        vault
            .read(&replacement, false)
            .and_then(|key| key.ok_or(E::InvalidProof)),
    )?;
    checked(
        progress,
        Step::ValidateKey,
        validate(&replacement_key, expected),
    )?;
    note(progress, Step::DeleteLegacy, Phase::Started)?;
    checked(progress, Step::DeleteLegacy, vault.remove(account))?;
    Ok(O::Allowed)
}

#[cfg(test)]
mod tests {
    use super::super::PendingReason as E;
    use super::*;
    use ed25519_dalek::SigningKey;
    use std::{collections::BTreeMap, sync::Mutex};

    #[derive(Default)]
    struct Fake {
        items: Mutex<BTreeMap<String, SigningKey>>,
        calls: Mutex<Vec<(String, bool)>>,
        fault: Mutex<Option<&'static str>>,
    }
    impl Fake {
        fn legacy() -> Self {
            let value = Self::default();
            value
                .items
                .lock()
                .unwrap()
                .insert("device".into(), SigningKey::from_bytes(&[1; 32]));
            value
        }
        fn expected() -> [u8; 32] {
            SigningKey::from_bytes(&[1; 32]).verifying_key().to_bytes()
        }
        fn has(&self, name: &str) -> bool {
            self.items.lock().unwrap().contains_key(name)
        }
    }
    impl Vault for Fake {
        fn read(
            &self,
            account: &str,
            interactive: bool,
        ) -> super::super::Result<Option<SigningKey>> {
            self.calls
                .lock()
                .unwrap()
                .push((account.into(), interactive));
            let fault = *self.fault.lock().unwrap();
            if fault == Some("cancel") && interactive {
                return Err(E::Cancelled);
            }
            if fault == Some("replacement_refused") && account.ends_with(".dr1") {
                return Err(E::KeychainUnavailable(-25293));
            }
            if fault == Some("verify") && account.ends_with(".dr1") && self.has(account) {
                return Ok(Some(SigningKey::from_bytes(&[2; 32])));
            }
            Ok(self.items.lock().unwrap().get(account).cloned())
        }
        fn add(&self, account: &str, key: &SigningKey) -> super::super::Result<()> {
            if *self.fault.lock().unwrap() == Some("add") {
                return Err(E::StorageUnavailable);
            }
            let mut items = self.items.lock().unwrap();
            if items.contains_key(account) {
                return Err(E::JournalConflict);
            }
            items.insert(account.into(), key.clone());
            Ok(())
        }
        fn remove(&self, account: &str) -> super::super::Result<()> {
            if *self.fault.lock().unwrap() == Some("delete") {
                return Err(E::KeychainInteractionRequired);
            }
            self.items.lock().unwrap().remove(account);
            Ok(())
        }
    }
    fn progress() -> Progress {
        Arc::new(|_| Ok(()))
    }

    // keeps: decision #280
    #[test]
    fn native_regrant_uses_one_interactive_read_then_prompt_free_verification() {
        let vault = Fake::legacy();
        assert_eq!(
            migrate(&vault, "device", &Fake::expected(), &progress()).unwrap(),
            bbt_proto::identity::UnlockOutcome::Allowed
        );
        assert!(!vault.has("device"));
        assert!(vault.has("device.dr1"));
        assert_eq!(
            vault
                .calls
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, allow)| *allow)
                .count(),
            1
        );
        assert_eq!(
            read_preferred(&vault, "device")
                .unwrap()
                .unwrap()
                .verifying_key()
                .to_bytes(),
            Fake::expected()
        );
        assert_eq!(
            vault.calls.lock().unwrap().last().unwrap(),
            &("device.dr1".into(), false)
        );
    }

    // keeps: decision #280
    #[test]
    fn add_verify_or_delete_failure_never_loses_the_legacy_secret() {
        for fault in ["add", "verify", "delete"] {
            let vault = Fake::legacy();
            *vault.fault.lock().unwrap() = Some(fault);
            assert!(migrate(&vault, "device", &Fake::expected(), &progress()).is_err());
            assert!(vault.has("device"), "lost old item on {fault}");
            assert_eq!(
                vault.items.lock().unwrap()["device"]
                    .verifying_key()
                    .to_bytes(),
                Fake::expected()
            );
        }
    }

    // keeps: decision #280
    #[test]
    fn a_wrong_key_and_a_cancelled_native_read_have_no_writes() {
        let vault = Fake::legacy();
        assert_eq!(
            migrate(&vault, "device", &[9; 32], &progress()),
            Err(E::InvalidProof)
        );
        assert!(vault.has("device"));
        assert!(!vault.has("device.dr1"));
        *vault.fault.lock().unwrap() = Some("cancel");
        assert_eq!(
            migrate(&vault, "device", &Fake::expected(), &progress()).unwrap(),
            bbt_proto::identity::UnlockOutcome::Cancelled
        );
        assert!(vault.has("device"));
        assert!(!vault.has("device.dr1"));
    }

    // keeps: decision #280
    #[test]
    fn an_audit_refusal_before_delete_leaves_both_and_restart_cleanup_never_prompts() {
        let vault = Fake::legacy();
        let stop: Progress = Arc::new(|row| {
            if row.step == Step::DeleteLegacy && row.phase == Phase::Started {
                Err(E::StorageUnavailable)
            } else {
                Ok(())
            }
        });
        assert!(migrate(&vault, "device", &Fake::expected(), &stop).is_err());
        assert!(vault.has("device") && vault.has("device.dr1"));
        vault.calls.lock().unwrap().clear();
        assert!(finish_migration(&vault, "device", &Fake::expected(), &progress()).unwrap());
        assert!(!vault.has("device") && vault.has("device.dr1"));
        assert!(
            vault
                .calls
                .lock()
                .unwrap()
                .iter()
                .all(|(_, interactive)| !interactive)
        );
    }

    // keeps: decision #280
    #[test]
    fn an_unreadable_replacement_never_silently_falls_back_to_legacy() {
        let vault = Fake::legacy();
        *vault.fault.lock().unwrap() = Some("replacement_refused");
        assert!(read_preferred(&vault, "device").is_err());
        assert_eq!(
            vault.calls.lock().unwrap().as_slice(),
            &[("device.dr1".into(), false)]
        );
        assert!(vault.has("device"));
    }
}
