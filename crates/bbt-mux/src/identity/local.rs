// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The local-dev issuer is a machine/epoch custody item, never a seed file.
use std::{fmt, path::Path};

use ed25519_dalek::SigningKey;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ulid::Ulid;

use super::{Custody, DeviceKeyStore, PendingReason as E, Result, transition};
use crate::{
    messaging::wire::{Issuer, IssuerKind},
    store::{Store, object_dir, sqlite::SqliteStore},
};

pub enum LocalIssuerOutcome {
    Ready(Box<SigningKey>),
    Pending(E),
}
impl fmt::Debug for LocalIssuerOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready(_) => f.write_str("Ready(<custodied local issuer>)"),
            Self::Pending(reason) => f.debug_tuple("Pending").field(reason).finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
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
impl Record {
    fn same_issuer(&self, other: &Self) -> bool {
        self.version == other.version
            && self.epoch == other.epoch
            && self.key_id == other.key_id
            && self.public_key == other.public_key
            && self.custody == other.custody
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hold {
    epoch: String,
    custody: Custody,
    reason: E,
}
struct Context {
    epoch: u64,
    expected: Option<String>,
    custody: Option<Custody>,
    certified: bool,
}

/// IO-only production construction seam. A pending result must be surfaced as
/// a hold; callers must never substitute another signing key.
pub fn local_dev_issuer(run_dir: &Path) -> LocalIssuerOutcome {
    bbt_rt::assert_not_ui_thread();
    let result = (|| {
        let account = account(run_dir)?;
        let context = context(&account)?;
        let keys = super::io::process_store(context.custody)?;
        load(&account, keys.as_ref())
    })();
    outcome(result)
}

/// Same custody semantics for isolated Keychain or in-memory IO tests.
pub fn local_dev_issuer_with_store(
    run_dir: &Path,
    keys: &dyn DeviceKeyStore,
) -> LocalIssuerOutcome {
    bbt_rt::assert_not_ui_thread();
    outcome(account(run_dir).and_then(|account| load(&account, keys)))
}
fn outcome(result: Result<SigningKey>) -> LocalIssuerOutcome {
    match result {
        Ok(key) => LocalIssuerOutcome::Ready(Box::new(key)),
        Err(reason) => LocalIssuerOutcome::Pending(reason),
    }
}
fn account(run_dir: &Path) -> Result<SqliteStore> {
    SqliteStore::open(&object_dir(run_dir, "account").map_err(|_| E::StorageUnavailable)?)
        .map_err(|_| E::StorageUnavailable)
}
fn record(account: &SqliteStore) -> Result<Option<Record>> {
    account
        .get("identity/local-issuer")
        .map_err(|_| E::StorageUnavailable)?
        .map(|bytes| serde_json::from_slice::<Record>(&bytes).map_err(|_| E::InvalidProof))
        .transpose()
}

/// Private admission facts validated against the same epoch, issuer pins and history
/// as ordinary loading. A hold hides the public ID, not this existing journal fact.
pub(super) fn unlock_target(
    account: &SqliteStore,
) -> crate::store::Result<(Ulid, Ulid, [u8; 32], Custody)> {
    let context = context(account)?;
    let current = record(account)?
        .ok_or_else(|| crate::store::invalid("held: no persisted custody identity"))?;
    let device = current
        .device_id
        .filter(|id| *id != Ulid::nil())
        .ok_or_else(|| crate::store::invalid("held: no validated persisted device ID"))?;
    if !current.device_ready
        || context.epoch.to_string() != current.epoch
        || context.expected.as_deref() != Some(current.public_key.as_str())
    {
        return Err(crate::store::invalid(
            "held: custody identity journal mismatch",
        ));
    }
    let history: Record = serde_json::from_slice(
        &account
            .get(&format!("identity/local-issuer-history/{}", current.epoch))?
            .ok_or_else(|| crate::store::invalid("held: missing custody identity history"))?,
    )?;
    if history != current {
        return Err(crate::store::invalid(
            "held: custody identity history mismatch",
        ));
    }
    Ok((
        device,
        current.key_id,
        crate::messaging::wire::parse_public(&current.public_key)?,
        current.custody,
    ))
}

/// Read public continuity facts without selecting custody or claiming readiness.
pub(super) fn observed_device_id(account: &SqliteStore) -> crate::store::Result<Option<Ulid>> {
    let Some(current) = record(account)? else {
        return Ok(None);
    };
    if !current.device_ready {
        return Ok(None);
    }
    unlock_target(account).map(|(device, _, _, _)| Some(device))
}

/// Public custody metadata only; a hold never masquerades as a ready device.
pub(super) fn device_id(account: &SqliteStore) -> crate::store::Result<Option<Ulid>> {
    let Some(record) = record(account).map_err(|e| crate::store::invalid(e.to_string()))? else {
        return Ok(None);
    };
    if !record.device_ready {
        return Ok(None);
    }
    if let Some(hold) = account.get("identity/local-issuer-hold")? {
        let hold: Hold = serde_json::from_slice(&hold)?;
        if hold.epoch == record.epoch {
            return Ok(None);
        }
    }
    if record.device_id == Some(Ulid::nil()) {
        return Err(crate::store::invalid("invalid custody device id"));
    }
    Ok(record.device_id)
}

pub(super) fn first_device(account: &SqliteStore, keys: &dyn DeviceKeyStore) -> Result<Ulid> {
    match load(account, keys) {
        Ok(key) => {
            drop(key);
            device_id(account)
                .map_err(|_| E::StorageUnavailable)?
                .ok_or(E::LocalIssuerMissing)
        }
        Err(E::LocalIssuerMissing) => {
            // Primary bootstrap already permits a new device while preserving
            // an older public local pin. It must not replace that missing key.
            let context = context(account)?;
            let public_key = context.expected.ok_or(E::LocalIssuerMissing)?;
            let device = reserve_device(
                account,
                Record {
                    version: 1,
                    epoch: context.epoch.to_string(),
                    key_id: key_id(context.epoch),
                    public_key,
                    custody: keys.custody(),
                    device_id: None,
                    device_ready: false,
                },
            )?;
            ensure_device(account, keys, device)?;
            complete_device(account, device, false)?;
            Ok(device)
        }
        Err(reason) => Err(reason),
    }
}
pub(super) fn decorate(
    account: &SqliteStore,
    status: &mut super::IdentityStatus,
) -> crate::store::Result<()> {
    if !status.bootstrapped {
        let custody = if let Some(record) = record(account)? {
            Some(record.custody)
        } else {
            account
                .get("identity/local-issuer-hold")?
                .map(|bytes| serde_json::from_slice::<Hold>(&bytes).map(|hold| hold.custody))
                .transpose()?
        };
        status.custody = custody;
        status.custody_label = custody.map(|c| c.label().into());
    }
    Ok(())
}
fn context(account: &SqliteStore) -> Result<Context> {
    let old = record(account)?;
    if let Some(old) = &old {
        if old.version != 1
            || old.key_id != key_id(transition::counter(&old.epoch).map_err(|_| E::InvalidProof)?)
        {
            return Err(E::InvalidProof);
        }
        crate::messaging::wire::parse_public(&old.public_key).map_err(|_| E::InvalidProof)?;
    }
    let state = transition::read(account).map_err(|_| E::JournalConflict)?;
    let epoch = if let Some(state) = &state
        && state.kind == IssuerKind::LocalDev
    {
        transition::counter(&state.cutover_epoch).map_err(|_| E::InvalidProof)?
    } else {
        old.as_ref()
            .map(|r| transition::counter(&r.epoch).map_err(|_| E::InvalidProof))
            .transpose()?
            .unwrap_or(1)
    };
    let mut expected = None;
    let mut pin = |key: &str| -> Result<()> {
        crate::messaging::wire::parse_public(key).map_err(|_| E::InvalidProof)?;
        if expected.as_deref().is_some_and(|old| old != key) {
            return Err(E::LocalIssuerMismatch);
        }
        expected = Some(key.to_owned());
        Ok(())
    };
    if let Some(old) = &old {
        let old_epoch = transition::counter(&old.epoch).map_err(|_| E::InvalidProof)?;
        if old_epoch > epoch {
            return Err(E::JournalConflict);
        }
        if old_epoch == epoch {
            pin(&old.public_key)?;
        }
    }
    if let Some(state) = &state {
        if state.kind == IssuerKind::LocalDev {
            pin(&state.key)?;
        } else if old.is_none() {
            for legacy in &state.legacy {
                pin(&legacy.issuer)?;
            }
        }
    }
    if let Some(bytes) = account
        .get("messaging/issuer")
        .map_err(|_| E::StorageUnavailable)?
    {
        let issuer: Issuer = serde_json::from_slice(&bytes).map_err(|_| E::InvalidProof)?;
        if issuer.kind == IssuerKind::LocalDev {
            pin(&issuer.key)?;
        }
    }
    let custody = if let Some(old) = old.as_ref() {
        Some(old.custody)
    } else {
        account
            .get("identity/local-issuer-hold")
            .map_err(|_| E::StorageUnavailable)?
            .map(|bytes| serde_json::from_slice::<Hold>(&bytes).map_err(|_| E::InvalidProof))
            .transpose()?
            .filter(|hold| hold.epoch == epoch.to_string())
            .map(|hold| hold.custody)
    };
    Ok(Context {
        epoch,
        expected,
        custody,
        certified: state
            .as_ref()
            .is_some_and(|state| state.kind == IssuerKind::GpgRoot),
    })
}
fn key_id(epoch: u64) -> Ulid {
    // This is a public address in each Mac's own Keychain, not key derivation.
    let mut hash = Sha256::new();
    hash.update(b"bboxy450-term/local-dev-issuer/v1\0");
    hash.update(epoch.to_be_bytes());
    let hash = hash.finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    Ulid::from_bytes(id)
}
fn load(account: &SqliteStore, keys: &dyn DeviceKeyStore) -> Result<SigningKey> {
    let context = context(account)?;
    let result = obtain(account, keys, &context);
    if let Err(reason) = result.as_ref() {
        let hold = Hold {
            epoch: context.epoch.to_string(),
            custody: keys.custody(),
            reason: *reason,
        };
        account
            .put(
                "identity/local-issuer-hold",
                &serde_json::to_vec(&hold).map_err(|_| E::InvalidProof)?,
            )
            .map_err(|_| E::StorageUnavailable)?;
    }
    result
}
fn obtain(
    account: &SqliteStore,
    keys: &dyn DeviceKeyStore,
    context: &Context,
) -> Result<SigningKey> {
    let id = key_id(context.epoch);
    let key = match keys.load(id)? {
        Some(key) => key,
        None if context.expected.is_some() => return Err(E::LocalIssuerMissing),
        None => {
            let mut seed = [0; 32];
            getrandom::fill(&mut seed).map_err(|_| E::StorageUnavailable)?;
            let proposed = SigningKey::from_bytes(&seed);
            seed.fill(0);
            match keys.create(id, &proposed) {
                Ok(()) => {
                    let saved = keys.load(id)?.ok_or(E::KeychainUnavailable(-25300))?;
                    if saved.verifying_key() != proposed.verifying_key() {
                        return Err(E::InvalidProof);
                    }
                    saved
                }
                Err(E::JournalConflict | E::KeychainUnavailable(-25299)) => {
                    keys.load(id)?.ok_or(E::KeychainUnavailable(-25300))?
                }
                Err(reason) => return Err(reason),
            }
        }
    };
    let public_key = super::hex(&key.verifying_key().to_bytes());
    if context
        .expected
        .as_ref()
        .is_some_and(|expected| expected != &public_key)
    {
        return Err(E::LocalIssuerMismatch);
    }
    let proposed_record = Record {
        version: 1,
        epoch: context.epoch.to_string(),
        key_id: id,
        public_key,
        custody: keys.custody(),
        device_id: None,
        device_ready: false,
    };
    let device = reserve_device(account, proposed_record)?;
    // After certification the chain owns device-key validation. Opening it here
    // would turn its hold into another issuer hold and regrant the wrong item.
    if !context.certified
        || !record(account)?
            .is_some_and(|record| record.device_id == Some(device) && record.device_ready)
    {
        ensure_device(account, keys, device)?;
    }
    complete_device(account, device, true)?;
    Ok(key)
}

fn ensure_device(account: &SqliteStore, keys: &dyn DeviceKeyStore, device: Ulid) -> Result<()> {
    match keys.load(device)? {
        Some(_) => {}
        None => {
            let current = record(account)?.ok_or(E::JournalConflict)?;
            if current.device_ready {
                return Err(E::LocalIssuerMissing);
            }
            let mut seed = [0; 32];
            getrandom::fill(&mut seed).map_err(|_| E::StorageUnavailable)?;
            let proposed = SigningKey::from_bytes(&seed);
            seed.fill(0);
            let created = match keys.create(device, &proposed) {
                Ok(()) => true,
                Err(E::JournalConflict | E::KeychainUnavailable(-25299)) => false,
                Err(reason) => return Err(reason),
            };
            let saved = keys.load(device)?.ok_or(E::KeychainUnavailable(-25300))?;
            if created && saved.verifying_key() != proposed.verifying_key() {
                return Err(E::InvalidProof);
            }
        }
    }
    Ok(())
}

fn reserve_device(account: &SqliteStore, mut proposed: Record) -> Result<Ulid> {
    let status =
        super::status::read(account, crate::store::now_ms() / 1000).map_err(|_| E::InvalidProof)?;
    let enrolled = status.bootstrapped.then_some(status.device_id).flatten();
    #[derive(Deserialize)]
    struct Pending {
        id: Ulid,
    }
    let pending = account
        .get("identity/pending")
        .map_err(|_| E::StorageUnavailable)?
        .map(|bytes| {
            serde_json::from_slice::<Pending>(&bytes)
                .map(|v| v.id)
                .map_err(|_| E::InvalidProof)
        })
        .transpose()?;
    let mut db = account.connection().map_err(|_| E::StorageUnavailable)?;
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| E::StorageUnavailable)?;
    let state = transition::get(&tx).map_err(|_| E::JournalConflict)?;
    if let Some(state) = state
        && state.kind == IssuerKind::LocalDev
        && state.cutover_epoch != proposed.epoch
    {
        return Err(E::JournalConflict);
    }
    let history_key = format!("identity/local-issuer-history/{}", proposed.epoch);
    let old: Option<Vec<u8>> = tx
        .query_row("SELECT value FROM kv WHERE key=?1", [&history_key], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|_| E::StorageUnavailable)?;
    let old = old
        .map(|bytes| serde_json::from_slice::<Record>(&bytes).map_err(|_| E::InvalidProof))
        .transpose()?;
    if old.as_ref().is_some_and(|old| !old.same_issuer(&proposed)) {
        return Err(E::JournalConflict);
    }
    let current: Option<Vec<u8>> = tx
        .query_row(
            "SELECT value FROM kv WHERE key='identity/local-issuer'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| E::StorageUnavailable)?;
    let current = current
        .map(|bytes| serde_json::from_slice::<Record>(&bytes).map_err(|_| E::InvalidProof))
        .transpose()?;
    let candidates = [
        enrolled,
        current.as_ref().and_then(|r| r.device_id),
        old.as_ref().and_then(|r| r.device_id),
        pending,
    ];
    let mut chosen = None;
    for id in candidates.into_iter().flatten() {
        if id == Ulid::nil() || chosen.is_some_and(|old| old != id) {
            return Err(E::JournalConflict);
        }
        chosen = Some(id);
    }
    let id = chosen.unwrap_or_else(Ulid::new);
    proposed.device_id = Some(id);
    proposed.device_ready = enrolled == Some(id)
        || current
            .as_ref()
            .is_some_and(|r| r.device_id == Some(id) && r.device_ready)
        || old
            .as_ref()
            .is_some_and(|r| r.device_id == Some(id) && r.device_ready);
    let bytes = serde_json::to_vec(&proposed).map_err(|_| E::InvalidProof)?;
    tx.execute(
        "INSERT INTO kv VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![history_key, &bytes],
    )
    .map_err(|_| E::StorageUnavailable)?;
    tx.execute("INSERT INTO kv VALUES('identity/local-issuer',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [&bytes]).map_err(|_|E::StorageUnavailable)?;
    tx.commit().map_err(|_| E::StorageUnavailable)?;
    Ok(id)
}

fn complete_device(account: &SqliteStore, id: Ulid, clear_local_hold: bool) -> Result<()> {
    let mut db = account.connection().map_err(|_| E::StorageUnavailable)?;
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| E::StorageUnavailable)?;
    let bytes: Vec<u8> = tx
        .query_row(
            "SELECT value FROM kv WHERE key='identity/local-issuer'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| E::StorageUnavailable)?;
    let mut record: Record = serde_json::from_slice(&bytes).map_err(|_| E::InvalidProof)?;
    if record.device_id != Some(id) {
        return Err(E::JournalConflict);
    }
    record.device_ready = true;
    let bytes = serde_json::to_vec(&record).map_err(|_| E::InvalidProof)?;
    tx.execute(
        "UPDATE kv SET value=?1 WHERE key='identity/local-issuer'",
        [&bytes],
    )
    .map_err(|_| E::StorageUnavailable)?;
    tx.execute(
        "UPDATE kv SET value=?2 WHERE key=?1",
        params![
            format!("identity/local-issuer-history/{}", record.epoch),
            &bytes
        ],
    )
    .map_err(|_| E::StorageUnavailable)?;
    if clear_local_hold {
        tx.execute("DELETE FROM kv WHERE key='identity/local-issuer-hold'", [])
            .map_err(|_| E::StorageUnavailable)?;
    }
    tx.commit().map_err(|_| E::StorageUnavailable)?;
    Ok(())
}
