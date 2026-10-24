//! A throwaway, persistent account root. It never opens native custody.
#![allow(
    clippy::disallowed_methods,
    reason = "explicit private test-account IO bootstrap"
)]
use std::{path::Path, sync::Arc};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use bboxy450_keychain::{Acl, Backend};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use ulid::Ulid;
use zeroize::{Zeroize, Zeroizing};

use super::{
    AccountHead, Custody, DeviceIdentity, DeviceKeyStore, PendingReason as E, Result, Signed,
    certificate::{self, Root},
};
use crate::store::{Store, invalid, object_dir, sqlite::SqliteStore};

const CONTEXT: &str = "identity/test-account";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootSeed {
    version: u8,
    run: Ulid,
    created: u32,
    seed: [u8; 32],
}
impl Drop for RootSeed {
    fn drop(&mut self) {
        self.seed.zeroize();
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunContext {
    version: u8,
    run: Ulid,
    run_dir: String,
    root_fingerprint: String,
    issued_by: String,
}

struct FileKeys(Arc<dyn bboxy450_keychain::Store>);
impl DeviceKeyStore for FileKeys {
    fn custody(&self) -> Custody {
        Custody::File
    }
    fn load(&self, device: Ulid) -> Result<Option<SigningKey>> {
        let secret = self
            .0
            .get(&format!("key_{device}"))
            .map_err(custody_error)?;
        secret
            .map(|secret| {
                let mut seed: [u8; 32] =
                    secret.as_bytes().try_into().map_err(|_| E::InvalidProof)?;
                let key = SigningKey::from_bytes(&seed);
                seed.zeroize();
                Ok(key)
            })
            .transpose()
    }
    fn create(&self, device: Ulid, key: &SigningKey) -> Result<()> {
        if device == Ulid::nil() {
            return Err(E::InvalidProof);
        }
        self.0
            .put(&format!("key_{device}"), key.as_bytes(), Acl::OwnerOnly)
            .map_err(custody_error)
    }
}
fn custody_error(error: bboxy450_keychain::Error) -> E {
    match error {
        bboxy450_keychain::Error::Conflict => E::JournalConflict,
        _ => E::FileCustodyUnavailable,
    }
}

/// Public context and an opaque custody handle, never a root or device secret.
pub struct TestAccount {
    keys: Arc<FileKeys>,
    label: String,
}
impl TestAccount {
    /// Generate once, then reopen the same root/device after a logic restart.
    /// Both directories are explicit, absolute and private. A foreign account
    /// is refused, and this function has no native or in-memory fallback.
    pub fn open(directory: &Path, run_dir: &Path) -> crate::store::Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let store: Arc<dyn bboxy450_keychain::Store> = bboxy450_keychain::open(Backend::File {
            directory: directory.into(),
        })?
        .into();
        // Reuse the same owner-only directory validation for the run boundary.
        let _run = bboxy450_keychain::open(Backend::File {
            directory: run_dir.into(),
        })?;
        let run_dir = run_dir.canonicalize()?;
        let account_path = run_dir.join("account");
        let _account_dir = bboxy450_keychain::open(Backend::File {
            directory: account_path,
        })?;
        let account = SqliteStore::open(&object_dir(&run_dir, "account")?)?;
        let old_context = account.get(CONTEXT)?;
        if old_context.is_none() {
            for key in ["identity/head", "identity/local-issuer", "messaging/issuer"] {
                if account.get(key)?.is_some() {
                    return Err(invalid(E::TestAccountMismatch.to_string()));
                }
            }
        }
        if store.get("test_root")?.is_none() {
            if old_context.is_some() {
                return Err(invalid(
                    "test_account_root_missing: existing run is never rekeyed",
                ));
            }
            if std::fs::read_dir(directory)?.next().transpose()?.is_some() {
                return Err(invalid(
                    "test_account_directory_not_empty: use a fresh private directory",
                ));
            }
            let mut fresh = RootSeed {
                version: 1,
                run: Ulid::new(),
                created: (crate::store::now_ms() / 1000).try_into()?,
                seed: [0; 32],
            };
            getrandom::fill(&mut fresh.seed)?;
            let bytes = Zeroizing::new(serde_json::to_vec(&fresh)?);
            match store.put("test_root", &bytes, Acl::OwnerOnly) {
                Ok(()) | Err(bboxy450_keychain::Error::Conflict) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let secret = store
            .get("test_root")?
            .ok_or_else(|| invalid("test account root missing"))?;
        let seed: RootSeed = serde_json::from_slice(secret.as_bytes())
            .map_err(|_| invalid("test_account_root_corrupt"))?;
        if seed.version != 1 || seed.run == Ulid::nil() {
            return Err(invalid(E::TestAccountMismatch.to_string()));
        }
        let root_key = SigningKey::from_bytes(&seed.seed);
        let label = format!("test-account:{}", seed.run);
        let (certificate, fingerprint) = certificate::test_root(&root_key, seed.created, &label)?;
        let context = Signed::issue_device(
            "TestRunContext",
            RunContext {
                version: 1,
                run: seed.run,
                run_dir: run_dir
                    .to_str()
                    .ok_or_else(|| invalid("test run path is not UTF-8"))?
                    .into(),
                root_fingerprint: fingerprint.clone(),
                issued_by: label.clone(),
            },
            &root_key,
        )?;
        let bytes = serde_json::to_vec(&context)?;
        // Per-account compare-and-create, not a global counter, lock or queue.
        account.connection()?.execute(
            "INSERT OR IGNORE INTO kv(key,value) VALUES(?1,?2)",
            rusqlite::params![CONTEXT, &bytes],
        )?;
        if account.get(CONTEXT)?.as_deref() != Some(bytes.as_slice()) {
            return Err(invalid(E::TestAccountMismatch.to_string()));
        }
        let keys = Arc::new(FileKeys(store));
        let now = crate::store::now_ms() / 1000;
        let identity = super::journal::install_certified(
            &account,
            keys.as_ref(),
            now,
            &fingerprint,
            &certificate,
            &root_key,
        )?;
        let head: Signed<AccountHead> = serde_json::from_slice(
            &account
                .get("identity/head")?
                .ok_or_else(|| invalid("test account head missing"))?,
        )?;
        if verified_label(&account, &identity, &head, &fingerprint)?.as_deref() != Some(&label) {
            return Err(invalid(E::TestAccountMismatch.to_string()));
        }
        Ok(Self { keys, label })
    }
    /// A verified label derived from this root, never from an action argument.
    pub fn issued_by(&self) -> &str {
        &self.label
    }
    /// Supply to the existing custodied logic constructor on its IO owner.
    pub fn key_store(&self) -> Arc<dyn DeviceKeyStore + Send + Sync> {
        self.keys.clone()
    }
}

/// Receivers derive the label from the verified certificate and signed context.
/// A real account stays on its existing wire rules and returns None.
pub(super) fn verified_label(
    account: &SqliteStore,
    identity: &DeviceIdentity,
    head: &Signed<AccountHead>,
    pinned: &str,
) -> crate::store::Result<Option<String>> {
    identity.verify(pinned, head, head.payload.issued_at)?;
    let root = Root::parse(
        &B64.decode(&identity.root_certificate)?,
        pinned,
        head.payload.issued_at,
    )?;
    let labels: Vec<_> = root
        .certificate
        .details
        .users
        .iter()
        .filter(|user| {
            user.signatures.iter().any(|signature| {
                signature.is_certification()
                    && signature.created().is_some_and(|created| {
                        u64::from(created.as_secs()) <= head.payload.issued_at
                    })
                    && signature
                        .verify_certification(
                            &root.certificate.primary_key,
                            pgp::types::Tag::UserId,
                            &user.id,
                        )
                        .is_ok()
            })
        })
        .filter_map(|user| {
            std::str::from_utf8(user.id.id())
                .ok()
                .filter(|value| value.starts_with("test-account:"))
        })
        .collect();
    let stored = account.get(CONTEXT)?;
    if labels.is_empty() && stored.is_none() {
        return Ok(None);
    }
    if labels.len() != 1 {
        return Err(invalid("test_account_label_mismatch: root UID"));
    }
    let context: Signed<RunContext> = serde_json::from_slice(
        &stored
            .ok_or_else(|| invalid("test_account_label_mismatch: signed run context missing"))?,
    )?;
    context.verify("TestRunContext", &root.public)?;
    let expected = format!("test-account:{}", context.payload.run);
    if context.payload.version != 1
        || context.payload.run == Ulid::nil()
        || labels[0] != expected
        || context.payload.issued_by != expected
        || context.payload.root_fingerprint != pinned
        || !Path::new(&context.payload.run_dir).is_absolute()
    {
        return Err(invalid(
            "test_account_label_mismatch: root and context disagree",
        ));
    }
    Ok(Some(expected))
}
