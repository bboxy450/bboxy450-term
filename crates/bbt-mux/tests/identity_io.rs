#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]

use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use bbt_mux::{
    Ulid,
    identity::{Custody, DeviceKeyStore, IdentityRuntime, IdentityState, PendingReason},
};
use ed25519_dalek::SigningKey;

struct InteractionRequired;
impl DeviceKeyStore for InteractionRequired {
    fn custody(&self) -> Custody {
        Custody::LoginKeychain
    }
    fn load(&self, _: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        Err(PendingReason::KeychainInteractionRequired)
    }
    fn create(&self, _: Ulid, _: &SigningKey) -> bbt_mux::identity::Result<()> {
        panic!("denied custody must never create a replacement key")
    }
}

#[test]
fn interaction_required_is_pending_without_a_prompt_or_replacement() {
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let start = Instant::now();
    let identity = IdentityRuntime::open_custodied_with_store(
        root.path(),
        Arc::new(InteractionRequired),
        runtime.handle().clone(),
    )
    .unwrap();
    assert!(start.elapsed() < Duration::from_secs(2));
    let status = identity.status().unwrap();
    assert_eq!(status.state, IdentityState::Pending);
    assert_eq!(
        status.pending_reason,
        Some(PendingReason::KeychainInteractionRequired)
    );
    assert!(!status.authenticated_signing_ready);
    assert!(
        identity
            .keys()
            .signing_hold()
            .unwrap()
            .contains("KeychainInteractionRequired")
    );
    assert!(
        status
            .reason_label
            .unwrap()
            .contains("Keychain interaction required")
    );
}

#[derive(Default)]
struct Stuck {
    released: Mutex<bool>,
    changed: Condvar,
    calls: AtomicUsize,
}
impl Stuck {
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}
impl DeviceKeyStore for Stuck {
    fn custody(&self) -> Custody {
        Custody::LoginKeychain
    }
    fn load(&self, _: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let released = self.released.lock().unwrap();
        let _released = self
            .changed
            .wait_while(released, |released| !*released)
            .unwrap();
        Err(PendingReason::KeychainInteractionRequired)
    }
    fn create(&self, _: Ulid, _: &SigningKey) -> bbt_mux::identity::Result<()> {
        panic!("stalled custody must never create a replacement key")
    }
}
struct Release(Arc<Stuck>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}

#[test]
fn stuck_custody_has_a_two_second_bound_and_retry_does_not_add_workers() {
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let store = Arc::new(Stuck::default());
    let _release = Release(store.clone());
    let start = Instant::now();
    let mut identity = IdentityRuntime::open_custodied_with_store(
        root.path(),
        store.clone(),
        runtime.handle().clone(),
    )
    .unwrap();
    assert!(start.elapsed() >= Duration::from_millis(1800));
    assert!(start.elapsed() < Duration::from_millis(2500));
    assert_eq!(
        identity.status().unwrap().pending_reason,
        Some(PendingReason::KeychainTimedOut)
    );
    let start = Instant::now();
    identity.retry_custody().unwrap();
    assert!(start.elapsed() < Duration::from_millis(2500));
    assert_eq!(
        store.calls.load(Ordering::Relaxed),
        1,
        "a blocked retry must not add a worker"
    );
    store.release();
    identity.retry_custody().unwrap();
    assert_eq!(
        identity.status().unwrap().pending_reason,
        Some(PendingReason::KeychainInteractionRequired)
    );
    assert_eq!(store.calls.load(Ordering::Relaxed), 2);
}

#[cfg(debug_assertions)]
#[test]
#[ignore = "spawned-process fixture, invoked by debug_process_override_uses_only_memory_custody"]
fn memory_custody_process_fixture() {
    use bbt_mux::identity::{LocalIssuerOutcome, local_dev_issuer};
    assert_eq!(std::env::var("BBOXY_TEST_CUSTODY").unwrap(), "memory");
    let root = std::path::PathBuf::from(std::env::var_os("BBOXY_ID1E_FIXTURE_ROOT").unwrap());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let identity = IdentityRuntime::open_custodied(&root, runtime.handle().clone()).unwrap();
    let status = identity.status().unwrap();
    assert_eq!(status.state, IdentityState::LocalDev);
    assert_eq!(status.custody, Some(Custody::Memory));
    let LocalIssuerOutcome::Ready(first) = local_dev_issuer(&root) else {
        panic!("memory helper held")
    };
    let LocalIssuerOutcome::Ready(second) = local_dev_issuer(&root) else {
        panic!("memory helper held")
    };
    assert_eq!(first.verifying_key(), second.verifying_key());
    assert_eq!(
        identity.keys().issuer().key,
        bbt_mux::messaging::wire::public(&first.verifying_key().to_bytes())
    );
    println!("ID1e child: production runtime and public helper use one memory custody store");
}

#[cfg(debug_assertions)]
#[test]
fn debug_process_override_uses_only_memory_custody() {
    let root = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "memory_custody_process_fixture",
            "--nocapture",
        ])
        .env("BBOXY_TEST_CUSTODY", "memory")
        .env("BBOXY_ID1E_FIXTURE_ROOT", root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("one memory custody store"));
}
