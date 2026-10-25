//! Production startup may have public history but no signing secret yet.
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use ed25519_dalek::SigningKey;
use ulid::Ulid;

use super::{
    DeviceKeyStore, IdentityRuntime, LocalIssuerOutcome, PendingReason, local_dev_issuer,
    local_dev_issuer_with_store,
};
use crate::{
    messaging::{
        keys::SeatKeys,
        wire::{Envelope, Issuer, IssuerKind},
    },
    store::{Result, Store, invalid, object_dir, sqlite::SqliteStore},
};

#[derive(Clone)]
pub(super) struct Retry {
    root: PathBuf,
    store: Option<Arc<dyn DeviceKeyStore + Send + Sync>>,
    handle: tokio::runtime::Handle,
}
impl Retry {
    pub(super) fn store(
        &self,
        custody: super::Custody,
    ) -> super::Result<Arc<dyn DeviceKeyStore + Send + Sync>> {
        self.store
            .clone()
            .map(Ok)
            .unwrap_or_else(|| super::io::process_store(Some(custody)))
    }
    pub(super) fn reopen(&self) -> Result<IdentityRuntime> {
        self.open()
    }
    pub(super) fn custody_label(&self, recorded: Option<super::Custody>) -> Option<&'static str> {
        self.store
            .as_ref()
            .filter(|store| Some(store.custody()) == recorded)
            .map(|store| store.custody_label())
    }
    fn load(&self) -> LocalIssuerOutcome {
        match &self.store {
            Some(store) => local_dev_issuer_with_store(&self.root, store.as_ref()),
            None => local_dev_issuer(&self.root),
        }
    }
    fn open(&self) -> Result<IdentityRuntime> {
        let account = SqliteStore::open(&object_dir(&self.root, "account")?)?;
        let reason = match self.load() {
            LocalIssuerOutcome::Ready(key) => {
                match IdentityRuntime::open_with_store(
                    &self.root,
                    *key,
                    self.handle.clone(),
                    self.store.clone(),
                ) {
                    Ok(mut runtime) => {
                        runtime.retry = Some(self.clone());
                        return Ok(runtime);
                    }
                    Err(_) => PendingReason::InvalidProof,
                }
            }
            LocalIssuerOutcome::Pending(reason) => reason,
        };
        let hint = if let Some(bytes) = account.get("messaging/issuer")? {
            serde_json::from_slice(&bytes)?
        } else {
            Issuer {
                kind: IssuerKind::LocalDev,
                key: String::new(),
            }
        };
        Ok(IdentityRuntime {
            account,
            keys: Arc::new(Unavailable { hint, reason }),
            chain: None,
            retry: Some(self.clone()),
            pending: Some(reason),
            bus_key: None,
            observed_status: Default::default(),
        })
    }
}
impl IdentityRuntime {
    pub fn open_custodied(root: &Path, handle: tokio::runtime::Handle) -> Result<Self> {
        Retry {
            root: root.into(),
            store: super::io::process_test_store().map(super::io::bounded_store),
            handle,
        }
        .open()
    }
    /// Explicit custody, including a persistent per-run file store. A selected
    /// store's failure is held; it never selects native or replacement keys.
    pub fn open_custodied_with_store(
        root: &Path,
        store: Arc<dyn DeviceKeyStore + Send + Sync>,
        handle: tokio::runtime::Handle,
    ) -> Result<Self> {
        Retry {
            root: root.into(),
            store: Some(super::io::bounded_store(store)),
            handle,
        }
        .open()
    }
    /// Called by the IO owner every 30 seconds. A still-pending retry leaves
    /// the terminal owner and its public history intact.
    pub fn retry_custody(&mut self) -> Result<()> {
        bbt_rt::assert_not_ui_thread();
        if self.pending.is_some()
            && let Some(retry) = self.retry.clone()
        {
            *self = retry.open()?;
        }
        Ok(())
    }
}
struct Unavailable {
    hint: Issuer,
    reason: PendingReason,
}
impl SeatKeys for Unavailable {
    fn issuer(&self) -> Issuer {
        self.hint.clone()
    }
    fn ready_issuer(&self) -> Option<Issuer> {
        None
    }
    fn signing_hold(&self) -> Option<String> {
        Some(self.reason.to_string())
    }
    fn leaf(&self, _: Ulid, _: u64) -> Result<SigningKey> {
        Err(invalid(self.reason.label()))
    }
    fn verify(&self, _: &Envelope) -> Result<()> {
        Err(invalid(self.reason.label()))
    }
}
