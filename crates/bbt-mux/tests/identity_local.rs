#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use bbt_mux::{
    Ulid,
    identity::{
        Custody, DeviceKeyStore, LocalIssuerOutcome, PendingReason, local_dev_issuer_with_store,
    },
    store::{Store, sqlite::SqliteStore},
};
use ed25519_dalek::SigningKey;

#[derive(Default)]
struct Memory {
    keys: Mutex<BTreeMap<Ulid, SigningKey>>,
    creates: AtomicUsize,
    blocked: Mutex<Option<PendingReason>>,
}
impl DeviceKeyStore for Memory {
    fn custody(&self) -> Custody {
        Custody::Memory
    }
    fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        if let Some(reason) = *self.blocked.lock().unwrap() {
            return Err(reason);
        }
        Ok(self.keys.lock().unwrap().get(&id).cloned())
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
        self.creates.fetch_add(1, Ordering::Relaxed);
        let mut keys = self.keys.lock().unwrap();
        if keys.contains_key(&id) {
            return Err(PendingReason::JournalConflict);
        }
        keys.insert(id, key.clone());
        Ok(())
    }
}
fn ready(outcome: LocalIssuerOutcome) -> SigningKey {
    match outcome {
        LocalIssuerOutcome::Ready(key) => key,
        LocalIssuerOutcome::Pending(reason) => panic!("unexpected hold: {reason}"),
    }
}
fn public(key: &SigningKey) -> String {
    bbt_mux::messaging::wire::public(&key.verifying_key().to_bytes())
}
fn store(root: &std::path::Path) -> SqliteStore {
    SqliteStore::open(&root.join("account")).unwrap()
}

#[test]
fn stable_machine_key_survives_logic_relaunch_and_is_shared_across_run_dirs() {
    use bbt_mux::{
        Config, Supervisor,
        logic::{Logic, LogicConfig},
    };
    let machine = Memory::default();
    let root = tempfile::tempdir().unwrap();
    let repository = root.path().join(".bboxy");
    std::fs::create_dir(&repository).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let config = LogicConfig {
        run_dir: root.path().to_owned(),
        repository,
        ring: Default::default(),
    };
    let first = ready(local_dev_issuer_with_store(root.path(), &machine));
    let issuer = public(&first);
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config.clone(),
        first,
        runtime.handle(),
    )
    .unwrap();
    let before = store(root.path()).get("messaging/issuer").unwrap().unwrap();
    logic.shutdown().unwrap();
    let second = ready(local_dev_issuer_with_store(root.path(), &machine));
    assert_eq!(public(&second), issuer);
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        config,
        second,
        runtime.handle(),
    )
    .unwrap();
    assert_eq!(
        store(root.path()).get("messaging/issuer").unwrap().unwrap(),
        before
    );
    assert_eq!(
        runtime
            .block_on(logic.client.identity_status())
            .unwrap()
            .issuer_kind,
        bbt_mux::messaging::wire::IssuerKind::LocalDev
    );
    logic.shutdown().unwrap();
    let other = tempfile::tempdir().unwrap();
    assert_eq!(
        public(&ready(local_dev_issuer_with_store(other.path(), &machine))),
        issuer
    );
    assert_eq!(machine.creates.load(Ordering::Relaxed), 1);
    let record = store(root.path())
        .get("identity/local-issuer")
        .unwrap()
        .unwrap();
    let record: serde_json::Value = serde_json::from_slice(&record).unwrap();
    assert_eq!(record["epoch"], "1");
    assert_eq!(record["public_key"], issuer);
    assert!(
        store(root.path())
            .get("identity/local-issuer-history/1")
            .unwrap()
            .is_some()
    );
    println!(
        "ID1d: same custodied machine issuer across two logic starts and distinct run dirs; messaging pin unchanged; one creation"
    );
}

#[test]
fn unavailable_and_missing_custody_hold_without_replacement_or_secret_file() {
    let machine = Memory::default();
    let root = tempfile::tempdir().unwrap();
    *machine.blocked.lock().unwrap() = Some(PendingReason::KeychainUnavailable(-25308));
    assert!(matches!(
        local_dev_issuer_with_store(root.path(), &machine),
        LocalIssuerOutcome::Pending(PendingReason::KeychainUnavailable(-25308))
    ));
    assert_eq!(machine.creates.load(Ordering::Relaxed), 0);
    assert!(
        store(root.path())
            .get("identity/local-issuer")
            .unwrap()
            .is_none()
    );
    *machine.blocked.lock().unwrap() = None;
    let key = ready(local_dev_issuer_with_store(root.path(), &machine));
    assert!(
        store(root.path())
            .get("identity/local-issuer-hold")
            .unwrap()
            .is_none()
    );
    let seed = key.to_bytes();
    machine.keys.lock().unwrap().clear();
    assert!(matches!(
        local_dev_issuer_with_store(root.path(), &machine),
        LocalIssuerOutcome::Pending(PendingReason::LocalIssuerMissing)
    ));
    assert_eq!(machine.creates.load(Ordering::Relaxed), 1);
    fn scan(root: &std::path::Path, seed: &[u8; 32]) {
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                scan(&path, seed);
            } else {
                let bytes = std::fs::read(path).unwrap();
                assert!(!bytes.windows(seed.len()).any(|w| w == seed));
                assert!(
                    !String::from_utf8_lossy(&bytes)
                        .contains(&seed.iter().map(|b| format!("{b:02x}")).collect::<String>())
                );
            }
        }
    }
    scan(root.path(), &seed);
    assert!(
        !format!("{:?}", LocalIssuerOutcome::Ready(key))
            .contains(&seed.iter().map(|b| format!("{b:02x}")).collect::<String>())
    );
    println!(
        "ID1d: locked store and missing key are named holds; creation count stays fixed; no seed in run dir or Debug"
    );
}

#[test]
fn old_random_pin_is_not_silently_replaced() {
    use bbt_mux::messaging::{Messaging, keys::LocalSeatKeys};
    let machine = Memory::default();
    let old = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[7; 32]);
    Messaging::open(old.path(), Arc::new(LocalSeatKeys::new(key))).unwrap();
    let before = store(old.path()).get("messaging/issuer").unwrap().unwrap();
    assert!(matches!(
        local_dev_issuer_with_store(old.path(), &machine),
        LocalIssuerOutcome::Pending(PendingReason::LocalIssuerMissing)
    ));
    assert_eq!(machine.creates.load(Ordering::Relaxed), 0);
    assert_eq!(
        store(old.path()).get("messaging/issuer").unwrap().unwrap(),
        before
    );
    let fresh = tempfile::tempdir().unwrap();
    let _ = ready(local_dev_issuer_with_store(fresh.path(), &machine));
    assert!(matches!(
        local_dev_issuer_with_store(old.path(), &machine),
        LocalIssuerOutcome::Pending(PendingReason::LocalIssuerMismatch)
    ));
    assert_eq!(
        store(old.path()).get("messaging/issuer").unwrap().unwrap(),
        before
    );
    println!(
        "ID1d: lost pre-custody random pin remains unchanged; a different machine key is refused by name"
    );
}
