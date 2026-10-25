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
    pub(super) fn busy(&self) -> bool {
        self.active.lock().map(|value| *value).unwrap_or(true)
    }
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
    fn custody_label(&self) -> &'static str {
        self.store.custody_label()
    }
    fn native_busy(&self) -> bool {
        self.io.busy() || self.store.native_busy()
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
    fn native_acl(&self, id: Ulid) -> Result<bbt_proto::identity::CustodyAcl> {
        let store = self.store.clone();
        self.io.run(move || store.native_acl(id))
    }
    fn unlock(
        &self,
        id: Ulid,
        expected: [u8; 32],
        progress: super::unlock::Progress,
    ) -> Result<bbt_proto::identity::UnlockOutcome> {
        let store = self.store.clone();
        self.io.run(move || store.unlock(id, expected, progress))
    }
    fn finish_unlock(
        &self,
        id: Ulid,
        expected: [u8; 32],
        progress: super::unlock::Progress,
    ) -> Result<bool> {
        let store = self.store.clone();
        self.io
            .run(move || store.finish_unlock(id, expected, progress))
    }
}

/// Test keys exist only in debug builds. Release refuses explicit fixtures.
pub(super) fn process_test_store() -> Option<Arc<dyn DeviceKeyStore + Send + Sync>> {
    selected_test_store(std::env::var_os("BBOXY_TEST_CUSTODY").as_deref())
}

fn selected_test_store(
    selector: Option<&std::ffi::OsStr>,
) -> Option<Arc<dyn DeviceKeyStore + Send + Sync>> {
    if let Some(directory) = selector?.to_string_lossy().strip_prefix("fixture:") {
        #[cfg(debug_assertions)]
        return Some(Arc::new(FixtureKeys(std::path::PathBuf::from(directory))));
        #[cfg(not(debug_assertions))]
        {
            let _ = directory;
            eprintln!("refused: fixture custody is debug-only");
            return Some(Arc::new(RefusedFixture));
        }
    }
    #[cfg(debug_assertions)]
    if selector == Some(std::ffi::OsStr::new("memory")) {
        static KEYS: OnceLock<Arc<MemoryKeys>> = OnceLock::new();
        return Some(KEYS.get_or_init(Default::default).clone());
    }
    None
}

// A release never falls through from an explicit fixture selector to the real
// Keychain. Its unavailable store exposes the named refusal and holds signing.
#[cfg(not(debug_assertions))]
struct RefusedFixture;
#[cfg(not(debug_assertions))]
impl DeviceKeyStore for RefusedFixture {
    fn custody(&self) -> Custody {
        Custody::Memory
    }
    fn custody_label(&self) -> &'static str {
        "refused: fixture custody is debug-only"
    }
    fn load(&self, _: Ulid) -> Result<Option<SigningKey>> {
        Err(E::StorageUnavailable)
    }
    fn create(&self, _: Ulid, _: &SigningKey) -> Result<()> {
        Err(E::StorageUnavailable)
    }
}

/// Explicit private test directory; normal first-launch code generates the
/// keys. Atomic publication keeps a restarted child on the same fixture keys.
#[cfg(debug_assertions)]
struct FixtureKeys(std::path::PathBuf);
#[cfg(debug_assertions)]
impl FixtureKeys {
    fn path(&self, id: Ulid) -> Result<std::path::PathBuf> {
        if !self.0.is_absolute() {
            return Err(E::StorageUnavailable);
        }
        Ok(self.0.join(id.to_string()))
    }
}
#[cfg(debug_assertions)]
impl DeviceKeyStore for FixtureKeys {
    fn custody(&self) -> Custody {
        Custody::Memory
    }
    fn custody_label(&self) -> &'static str {
        "fixture custody (debug)"
    }
    fn load(&self, id: Ulid) -> Result<Option<SigningKey>> {
        use std::io::Read;
        let mut file = match std::fs::File::open(self.path(id)?) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(E::StorageUnavailable),
        };
        let mut seed = [0; 32];
        file.read_exact(&mut seed)
            .map_err(|_| E::StorageUnavailable)?;
        let mut extra = [0];
        if file.read(&mut extra).map_err(|_| E::StorageUnavailable)? != 0 {
            seed.fill(0);
            return Err(E::StorageUnavailable);
        }
        let key = SigningKey::from_bytes(&seed);
        seed.fill(0);
        Ok(Some(key))
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> Result<()> {
        use std::{
            io::Write,
            os::unix::fs::{DirBuilderExt, OpenOptionsExt},
        };
        let path = self.path(id)?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.0)
            .map_err(|_| E::StorageUnavailable)?;
        let temporary = self.0.join(format!(".{}", Ulid::new()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| E::StorageUnavailable)?;
        let result = (|| {
            file.write_all(key.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(|_| E::StorageUnavailable)?;
            std::fs::hard_link(&temporary, path).map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    E::JournalConflict
                } else {
                    E::StorageUnavailable
                }
            })
        })();
        let _ = std::fs::remove_file(temporary);
        result
    }
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
    fn custody_label(&self) -> &'static str {
        "memory custody (debug)"
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
    fn release_refuses_fixture_custody_by_name_without_selecting_keychain() {
        let store = super::selected_test_store(Some(std::ffi::OsStr::new("fixture:/unused")))
            .expect("an explicit fixture request cannot fall through to Keychain");
        assert_eq!(
            store.custody_label(),
            "refused: fixture custody is debug-only"
        );
        assert!(matches!(
            store.load(super::Ulid::new()),
            Err(super::E::StorageUnavailable)
        ));
    }

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
    fn fixture_generates_private_keys_once_and_reopens_without_a_known_key() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("custody");
        let selector = format!("fixture:{}", directory.display());
        let store = super::selected_test_store(Some(std::ffi::OsStr::new(&selector))).unwrap();
        let account = temp.path().join("account");
        let super::super::LocalIssuerOutcome::Ready(first) =
            super::super::local_dev_issuer_with_store(&account, store.as_ref())
        else {
            panic!("fixture first launch failed")
        };
        assert_eq!(store.custody_label(), "fixture custody (debug)");
        let reopened = super::selected_test_store(Some(std::ffi::OsStr::new(&selector))).unwrap();
        let super::super::LocalIssuerOutcome::Ready(second) =
            super::super::local_dev_issuer_with_store(&account, reopened.as_ref())
        else {
            panic!("fixture restart failed")
        };
        assert_eq!(first.verifying_key(), second.verifying_key());
        assert_eq!(
            std::fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
            0o700
        );
        let entries: Vec<_> = std::fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(entries.len(), 2); // independent generated issuer and device keys
        for path in entries {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let other = temp.path().join("other");
        let other_selector = format!("fixture:{}", other.display());
        let other =
            super::selected_test_store(Some(std::ffi::OsStr::new(&other_selector))).unwrap();
        let super::super::LocalIssuerOutcome::Ready(third) =
            super::super::local_dev_issuer_with_store(
                &temp.path().join("other-account"),
                other.as_ref(),
            )
        else {
            panic!("second fixture failed")
        };
        assert_ne!(first.verifying_key(), third.verifying_key());
    }

    #[test]
    fn fixture_never_overwrites_a_key_or_accepts_a_relative_directory() {
        use super::{DeviceKeyStore, E, FixtureKeys, Ulid};
        let temp = tempfile::tempdir().unwrap();
        let store = FixtureKeys(temp.path().join("keys"));
        let id = Ulid::new();
        let mut seed = [0; 32];
        getrandom::fill(&mut seed).unwrap();
        let key = super::SigningKey::from_bytes(&seed);
        store.create(id, &key).unwrap();
        assert!(matches!(store.create(id, &key), Err(E::JournalConflict)));
        assert_eq!(
            store.load(id).unwrap().unwrap().verifying_key(),
            key.verifying_key()
        );
        assert!(matches!(
            FixtureKeys("relative".into()).load(id),
            Err(E::StorageUnavailable)
        ));
    }

    #[test]
    fn labels_describe_the_actual_store_and_survive_the_io_wrapper() {
        use super::{Custody, DeviceKeyStore, E, Result};
        struct Fixture;
        impl DeviceKeyStore for Fixture {
            fn custody(&self) -> Custody {
                Custody::Memory
            }
            fn load(&self, _: super::Ulid) -> Result<Option<super::SigningKey>> {
                Err(E::InvalidProof)
            }
            fn create(&self, _: super::Ulid, _: &super::SigningKey) -> Result<()> {
                Err(E::InvalidProof)
            }
        }
        assert_eq!(Custody::Memory.label(), "test custody");
        let fixture = super::bounded_store(super::Arc::new(Fixture));
        assert_eq!(fixture.custody_label(), "test custody");
        let debug = super::bounded_store(super::Arc::new(super::MemoryKeys::default()));
        assert_eq!(debug.custody_label(), "memory custody (debug)");
    }

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
