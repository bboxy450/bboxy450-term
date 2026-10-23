//! A stuck custody service cannot hold its caller or grow a worker per retry.
#![allow(
    clippy::disallowed_methods,
    reason = "bounded custody IO, never the UI thread"
)]

use std::{
    sync::{Arc, Condvar, Mutex, OnceLock, mpsc},
    time::{Duration, Instant},
};

use ed25519_dalek::SigningKey;
use ulid::Ulid;

use super::{Custody, DeviceKeyStore, PendingReason as E, Result};

const BOUND: Duration = Duration::from_secs(2);

#[derive(Default)]
pub(super) struct CustodyIo {
    active: Mutex<bool>,
    available: Condvar,
}
impl CustodyIo {
    pub(super) fn run<T: Send + 'static>(
        self: &Arc<Self>,
        job: impl FnOnce() -> Result<T> + Send + 'static,
    ) -> Result<T> {
        bbt_rt::assert_not_ui_thread();
        let start = Instant::now();
        let active = self.active.lock().map_err(|_| E::StorageUnavailable)?;
        let (mut active, _) = self
            .available
            .wait_timeout_while(active, BOUND, |active| *active)
            .map_err(|_| E::StorageUnavailable)?;
        if *active || start.elapsed() >= BOUND {
            return Err(E::KeychainTimedOut);
        }
        *active = true;
        drop(active);
        let permit = Active(self.clone());
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("bbt-custody-io".into())
            .spawn(move || {
                let _permit = permit;
                let _ = tx.send(job());
            })
            .map_err(|_| E::StorageUnavailable)?;
        match rx.recv_timeout(BOUND.saturating_sub(start.elapsed())) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(E::KeychainTimedOut),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(E::StorageUnavailable),
        }
    }
}
struct Active(Arc<CustodyIo>);
impl Drop for Active {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0.active.lock() {
            *active = false;
            self.0.available.notify_one();
        }
    }
}

/// All native Keychain instances share the gate, including new retry instances.
/// A stalled OS call leaves at most one native worker until it returns or the
/// process exits. We never cancel it midway through an atomic Keychain add.
pub(super) fn keychain_io() -> Arc<CustodyIo> {
    static IO: OnceLock<Arc<CustodyIo>> = OnceLock::new();
    IO.get_or_init(Default::default).clone()
}

struct BoundedStore {
    store: Arc<dyn DeviceKeyStore + Send + Sync>,
    io: Arc<CustodyIo>,
}
pub(super) fn bounded_store(
    store: Arc<dyn DeviceKeyStore + Send + Sync>,
) -> Arc<dyn DeviceKeyStore + Send + Sync> {
    Arc::new(BoundedStore {
        store,
        io: Arc::default(),
    })
}
impl DeviceKeyStore for BoundedStore {
    fn custody(&self) -> Custody {
        self.store.custody()
    }
    fn load(&self, id: Ulid) -> Result<Option<SigningKey>> {
        let store = self.store.clone();
        self.io.run(move || store.load(id))
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> Result<()> {
        let store = self.store.clone();
        let key = key.clone();
        self.io.run(move || store.create(id, &key))
    }
}

/// This switch and the process-owned test keys do not exist in release code.
pub(super) fn process_test_store() -> Option<Arc<dyn DeviceKeyStore + Send + Sync>> {
    #[cfg(debug_assertions)]
    if std::env::var_os("BBOXY_TEST_CUSTODY").as_deref() == Some(std::ffi::OsStr::new("memory")) {
        static KEYS: OnceLock<Arc<MemoryKeys>> = OnceLock::new();
        return Some(KEYS.get_or_init(Default::default).clone());
    }
    None
}

pub(super) fn process_store(
    custody: Option<Custody>,
) -> Result<Arc<dyn DeviceKeyStore + Send + Sync>> {
    if let Some(store) = process_test_store() {
        return Ok(store);
    }
    Ok(Arc::new(match custody {
        Some(custody) => super::KeychainDeviceKeys::resume(custody)?,
        None => super::KeychainDeviceKeys::default(),
    }))
}

#[cfg(debug_assertions)]
#[derive(Default)]
struct MemoryKeys(Mutex<std::collections::BTreeMap<Ulid, SigningKey>>);
#[cfg(debug_assertions)]
impl DeviceKeyStore for MemoryKeys {
    fn custody(&self) -> Custody {
        Custody::Memory
    }
    fn load(&self, id: Ulid) -> Result<Option<SigningKey>> {
        Ok(self
            .0
            .lock()
            .map_err(|_| E::StorageUnavailable)?
            .get(&id)
            .cloned())
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> Result<()> {
        let mut keys = self.0.lock().map_err(|_| E::StorageUnavailable)?;
        match keys.entry(id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(key.clone());
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(_) => Err(E::JournalConflict),
        }
    }
}

#[cfg(all(test, not(debug_assertions)))]
mod release_tests {
    #[test]
    fn release_has_no_process_custody_override() {
        // Run this with BBOXY_TEST_CUSTODY=memory; no native Keychain call is made.
        assert!(super::process_test_store().is_none());
        assert_eq!(
            super::process_store(None).unwrap().custody(),
            super::Custody::DataProtection
        );
    }
}

#[cfg(all(test, debug_assertions))]
mod debug_tests {
    #[test]
    #[ignore = "spawned-process fixture, invoked by debug_without_override_selects_keychain"]
    fn unset_process_fixture() {
        assert!(std::env::var_os("BBOXY_TEST_CUSTODY").is_none());
        // Selection only: this test must not open the operator's real Keychain.
        assert!(super::process_test_store().is_none());
        assert_eq!(
            super::process_store(None).unwrap().custody(),
            super::Custody::DataProtection
        );
    }

    #[test]
    fn debug_without_override_selects_keychain() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "identity::io::debug_tests::unset_process_fixture",
            ])
            .env_remove("BBOXY_TEST_CUSTODY")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
