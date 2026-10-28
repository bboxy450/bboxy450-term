// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
mod common;
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
        LocalIssuerOutcome::Ready(key) => *key,
        LocalIssuerOutcome::Pending(reason) => panic!("unexpected hold: {reason}"),
    }
}
fn public(key: &SigningKey) -> String {
    bbt_mux::messaging::wire::public(&key.verifying_key().to_bytes())
}
fn store(root: &std::path::Path) -> SqliteStore {
    SqliteStore::open(&root.join("account")).unwrap()
}

fn custody_record(root: &std::path::Path) -> serde_json::Value {
    serde_json::from_slice(&store(root).get("identity/local-issuer").unwrap().unwrap()).unwrap()
}
fn device_id(root: &std::path::Path) -> Ulid {
    serde_json::from_value(custody_record(root)["device_id"].clone()).unwrap()
}
fn identity_status(root: &std::path::Path, keys: Arc<Memory>) -> bbt_mux::identity::IdentityStatus {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let identity = bbt_mux::identity::IdentityRuntime::open_custodied_with_store(
        root,
        keys,
        runtime.handle().clone(),
    )
    .unwrap();
    identity.status().unwrap()
}

// keeps: spec id1-seat-chain
#[test]
fn first_launch_device_is_stable_and_local_or_pending_is_still_uncertified() {
    let root = tempfile::tempdir().unwrap();
    let keys = Arc::new(Memory::default());
    ready(local_dev_issuer_with_store(root.path(), keys.as_ref()));
    let first = device_id(root.path());
    assert_ne!(first, Ulid::nil());
    assert_ne!(
        custody_record(root.path())["key_id"],
        serde_json::json!(first)
    );
    let local = identity_status(root.path(), keys.clone());
    assert_eq!(local.device_id, Some(first));
    assert_eq!(local.state, bbt_mux::identity::IdentityState::LocalDev);
    assert!(!local.bootstrapped);
    assert_eq!(
        local.issuer_kind,
        bbt_mux::messaging::wire::IssuerKind::LocalDev
    );
    store(root.path())
        .put(
            "identity/attempt",
            &serde_json::to_vec(&serde_json::json!({
                "root_fingerprint":null,"reason":PendingReason::AgentUnavailable,"at":1,
            }))
            .unwrap(),
        )
        .unwrap();
    let pending = identity_status(root.path(), keys.clone());
    assert_eq!(pending.device_id, Some(first));
    assert_eq!(pending.state, bbt_mux::identity::IdentityState::Pending);
    assert!(!pending.bootstrapped);
    assert_eq!(
        pending.issuer_kind,
        bbt_mux::messaging::wire::IssuerKind::LocalDev
    );
    ready(local_dev_issuer_with_store(root.path(), keys.as_ref()));
    assert_eq!(device_id(root.path()), first);
    assert_eq!(keys.creates.load(Ordering::Relaxed), 2); // issuer plus device
}

// keeps: spec id1-seat-chain
#[test]
fn legacy_custody_record_upgrades_once_without_changing_the_issuer_or_its_address() {
    let root = tempfile::tempdir().unwrap();
    let keys = Memory::default();
    let first = public(&ready(local_dev_issuer_with_store(root.path(), &keys)));
    let mut legacy = custody_record(root.path());
    let retired = device_id(root.path());
    keys.keys.lock().unwrap().remove(&retired);
    legacy.as_object_mut().unwrap().remove("device_id");
    legacy.as_object_mut().unwrap().remove("device_ready");
    let bytes = serde_json::to_vec(&legacy).unwrap();
    for name in ["identity/local-issuer", "identity/local-issuer-history/1"] {
        store(root.path()).put(name, &bytes).unwrap();
    }
    assert_eq!(
        public(&ready(local_dev_issuer_with_store(root.path(), &keys))),
        first
    );
    let upgraded = custody_record(root.path());
    let id = device_id(root.path());
    assert_ne!(id, retired);
    assert_eq!(upgraded["key_id"], legacy["key_id"]);
    let creates = keys.creates.load(Ordering::Relaxed);
    ready(local_dev_issuer_with_store(root.path(), &keys));
    assert_eq!(device_id(root.path()), id);
    assert_eq!(custody_record(root.path()), upgraded);
    assert_eq!(keys.creates.load(Ordering::Relaxed), creates);
}

// keeps: spec id1-seat-chain
#[test]
fn fresh_memory_custody_gets_a_fresh_device_and_lost_custody_stays_held() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let first = Arc::new(Memory::default());
    let second = Arc::new(Memory::default());
    let a_status = identity_status(a.path(), first);
    let b_status = identity_status(b.path(), second);
    assert_ne!(a_status.device_id, b_status.device_id);
    assert!(a_status.device_id.is_some() && b_status.device_id.is_some());
    let lost = Arc::new(Memory::default());
    let held = identity_status(a.path(), lost.clone());
    assert_eq!(held.device_id, None);
    assert_eq!(held.pending_reason, Some(PendingReason::LocalIssuerMissing));
    assert_eq!(lost.creates.load(Ordering::Relaxed), 0);
}

// keeps: spec id1-seat-chain
#[test]
fn held_device_creation_retains_its_reserved_id_and_releases_on_recovery() {
    struct DeviceHold {
        memory: Memory,
        issuer: Mutex<Option<Ulid>>,
        blocked: std::sync::atomic::AtomicBool,
    }
    impl DeviceKeyStore for DeviceHold {
        fn custody(&self) -> Custody {
            Custody::Memory
        }
        fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
            if self.blocked.load(Ordering::SeqCst)
                && self
                    .issuer
                    .lock()
                    .unwrap()
                    .is_some_and(|issuer| issuer != id)
            {
                return Err(PendingReason::KeychainInteractionRequired);
            }
            self.memory.load(id)
        }
        fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
            self.memory.create(id, key)?;
            self.issuer.lock().unwrap().get_or_insert(id);
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let keys = Arc::new(DeviceHold {
        memory: Memory::default(),
        issuer: Mutex::new(None),
        blocked: std::sync::atomic::AtomicBool::new(true),
    });
    assert!(matches!(
        local_dev_issuer_with_store(root.path(), keys.as_ref()),
        LocalIssuerOutcome::Pending(PendingReason::KeychainInteractionRequired)
    ));
    let reserved = device_id(root.path());
    assert_eq!(custody_record(root.path())["device_ready"], false);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut identity = bbt_mux::identity::IdentityRuntime::open_custodied_with_store(
        root.path(),
        keys.clone(),
        runtime.handle().clone(),
    )
    .unwrap();
    assert_eq!(identity.status().unwrap().device_id, None);
    keys.blocked.store(false, Ordering::SeqCst);
    identity.retry_custody().unwrap();
    let status = identity.status().unwrap();
    assert_eq!(status.device_id, Some(reserved));
    assert!(!status.bootstrapped);
    assert_eq!(custody_record(root.path())["device_ready"], true);
}

// keeps: spec id1-seat-chain
#[test]
fn stable_machine_key_survives_logic_relaunch_and_is_shared_across_run_dirs() {
    use bbt_mux::{
        Config, Supervisor,
        logic::{Logic, LogicConfig},
    };
    let machine = Arc::new(Memory::default());
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
    let first = ready(local_dev_issuer_with_store(root.path(), machine.as_ref()));
    let issuer = public(&first);
    let logic = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        config.clone(),
        machine.clone(),
        runtime.handle(),
    )
    .unwrap();
    let before = store(root.path()).get("messaging/issuer").unwrap().unwrap();
    logic.shutdown().unwrap();
    let second = ready(local_dev_issuer_with_store(root.path(), machine.as_ref()));
    assert_eq!(public(&second), issuer);
    let logic = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        config,
        machine.clone(),
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
    assert!(
        store(root.path())
            .list("identity/bus-signers/")
            .unwrap()
            .is_empty(),
        "ready startup creates no fake hold"
    );
    let other = tempfile::tempdir().unwrap();
    assert_eq!(
        public(&ready(local_dev_issuer_with_store(
            other.path(),
            machine.as_ref()
        ))),
        issuer
    );
    assert_eq!(machine.creates.load(Ordering::Relaxed), 3);
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
        "ID1d: same custodied issuer across two logic starts/run dirs; messaging pin unchanged; issuer plus two account device items"
    );
}

// keeps: spec id1-seat-chain
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
    assert_eq!(machine.creates.load(Ordering::Relaxed), 2);
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
        !format!("{:?}", LocalIssuerOutcome::Ready(Box::new(key)))
            .contains(&seed.iter().map(|b| format!("{b:02x}")).collect::<String>())
    );
    println!(
        "ID1d: locked store and missing key are named holds; creation count stays fixed; no seed in run dir or Debug"
    );
}

// keeps: spec id1-seat-chain
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

// keeps: spec id1-seat-chain
#[test]
fn concurrent_creators_read_one_atomic_winner() {
    struct Racing {
        memory: Memory,
        first_loads: AtomicUsize,
        both: std::sync::Barrier,
    }
    impl DeviceKeyStore for Racing {
        fn custody(&self) -> Custody {
            Custody::Memory
        }
        fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
            if self.first_loads.fetch_add(1, Ordering::SeqCst) < 2 {
                // Both creators must observe absence before either adds.
                self.both.wait();
                return Ok(None);
            }
            self.memory.load(id)
        }
        fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
            self.memory.create(id, key)
        }
    }
    let machine = Arc::new(Racing {
        memory: Memory::default(),
        first_loads: AtomicUsize::new(0),
        both: std::sync::Barrier::new(2),
    });
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let run = |root: std::path::PathBuf, machine: Arc<Racing>| {
        std::thread::spawn(move || {
            public(&ready(local_dev_issuer_with_store(&root, machine.as_ref())))
        })
    };
    let x = run(a.path().into(), machine.clone());
    let y = run(b.path().into(), machine.clone());
    assert_eq!(x.join().unwrap(), y.join().unwrap());
    assert_eq!(machine.memory.creates.load(Ordering::Relaxed), 4);
    assert_eq!(machine.memory.keys.lock().unwrap().len(), 3);
    println!(
        "ID1d: concurrent creators share the issuer; each account retains its own device item"
    );
}

// keeps: spec id1-seat-chain
#[test]
fn held_queue_survives_reopen_and_refuses_changed_generation_role_epoch_and_routes() {
    use bbt_mux::{
        identity::IdentityRuntime,
        messaging::{
            Messaging,
            api::{Request, Response},
            tree::{Actor, Node},
            wire::SendRequest,
        },
        registry::{Class, Registry, SeatRecord},
    };
    let machine = Arc::new(Memory::default());
    *machine.blocked.lock().unwrap() = Some(PendingReason::KeychainUnavailable(-25308));
    let root = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut identity = IdentityRuntime::open_custodied_with_store(
        root.path(),
        machine.clone(),
        runtime.handle().clone(),
    )
    .unwrap();
    let registry = Registry::new(store(root.path()));
    let swarm = Ulid::new();
    let mut seats = Vec::new();
    for name in [
        "changed role",
        "changed generation",
        "changed route",
        "valid",
    ] {
        let seat = SeatRecord::new(Ulid::new(), swarm, name, "zsh");
        registry.insert(&seat).unwrap();
        seats.push(seat);
    }
    let mut messaging = Messaging::open(root.path(), identity.keys()).unwrap();
    let mut ids = Vec::new();
    for seat in &seats {
        messaging.sync_seat(seat).unwrap();
        let message = SendRequest::text(format!("seats/{}/inbox", seat.id), seat.name.clone());
        ids.push(message.id);
        assert!(matches!(
            messaging
                .request(seat.id, 1, Request::Send { message }, false, None, 1)
                .unwrap(),
            Response::Stored { message } if message.envelope().is_none() && message.intent().is_some()
        ));
    }
    assert_eq!(
        store(root.path())
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM msg_counters", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0,
        "no unsigned placeholder envelope or sender sequence"
    );
    drop(messaging);
    let mut messaging = Messaging::open(root.path(), identity.keys()).unwrap();
    registry
        .update(seats[0].id, |s| {
            s.role.class = Class::Oracle;
            s.role.name = "oracle".into();
            Ok(())
        })
        .unwrap();
    registry
        .update(seats[1].id, |s| {
            s.generation = 2;
            Ok(())
        })
        .unwrap();
    // A changed route remains readable but must not acquire new delivery power.
    let path = format!("seats/{}/inbox", seats[2].id);
    let mut sink = messaging.tree(swarm).unwrap().get(&path).unwrap().clone();
    sink.paused = true;
    messaging
        .tree(swarm)
        .unwrap()
        .put(Actor::Operator, sink)
        .unwrap();
    // Touch a folder as well to prove no accidental exact-tree revision reliance.
    messaging
        .tree(swarm)
        .unwrap()
        .put(Actor::Operator, Node::folder("channels/unrelated"))
        .unwrap();
    *machine.blocked.lock().unwrap() = None;
    identity.retry_custody().unwrap();
    assert_eq!(messaging.resume_signing(identity.keys(), 2).unwrap(), 1);
    let db = store(root.path());
    let db = db.connection().unwrap();
    let rows: Vec<(String, String)> = db
        .prepare("SELECT state,reason FROM msg_held ORDER BY position")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        rows.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
        vec!["refused", "refused", "refused", "released"]
    );
    assert!(rows[0].1.contains("authority changed"));
    assert!(rows[1].1.contains("stale messaging process generation"));
    assert!(rows[2].1.contains("authority changed"));
    let page = messaging.list(seats[3].id, None, None, 0, 10).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].envelope().expect("signed fixture envelope").id, ids[3]);
    println!(
        "ID1d: unsigned queue survives owner reopen; changed role/epoch, generation and routes are refused; later valid work releases in order"
    );
}

// keeps: spec id1-seat-chain
#[test]
fn bus_seals_exact_hash_range_before_rekey_and_replays_retained_signed_bytes() {
    use bbt_mux::{
        bus::Broker,
        store::{
            Events,
            events::{Draft, EventLog},
        },
    };
    let root = tempfile::tempdir().unwrap();
    let run = Ulid::new();
    let ephemeral = SigningKey::from_bytes(&[31; 32]);
    let mut bus = Broker::open(root.path(), ephemeral.clone()).unwrap();
    bus.identity_signer(ephemeral.clone(), Some(run));
    let draft = Draft::new(
        "account/test/state",
        "fixture",
        serde_json::json!({"value":1}),
    );
    let first = bus.publish("account", draft.clone()).unwrap();
    let last = bus
        .publish(
            "account",
            Draft::new(
                "account/test/state",
                "fixture",
                serde_json::json!({"value":2}),
            ),
        )
        .unwrap();
    let ranges = bus.identity_signer(SigningKey::from_bytes(&[32; 32]), None);
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].first, first.seq);
    assert_eq!(ranges[0].first_hash, first.hash().unwrap());
    assert_eq!(ranges[0].last, last.seq);
    assert_eq!(ranges[0].last_hash, last.hash().unwrap());
    assert!(
        first
            .event
            .source
            .starts_with(&format!("ephemeral-hold/{run}/"))
    );
    first.verify().unwrap();
    assert_eq!(
        bus.publish("account", draft.clone()).unwrap(),
        first,
        "replay cannot rewrite the old signer after recovery"
    );
    let mut forged = draft;
    forged.payload = serde_json::json!({"value":999});
    assert!(bus.publish("account", forged).is_err());
    let normal = bus
        .publish(
            "account",
            Draft::new(
                "account/test/state",
                "fixture",
                serde_json::json!({"value":3}),
            ),
        )
        .unwrap();
    assert_eq!(normal.event.source, "fixture");
    assert_ne!(
        normal.provenance.signer,
        ephemeral.verifying_key().to_bytes()
    );
    assert_eq!(
        EventLog::open(&root.path().join("account"))
            .unwrap()
            .verify_chain()
            .unwrap(),
        3
    );
    println!(
        "ID1d: signed hold markers and exact stream endpoints; atomic rekey; old bytes deduplicate; conflicting replay refused; mixed-signer hash chain verifies"
    );
}

// keeps: spec id1-seat-chain §4; controlled retry/order proof is in identity_retry_clock_tests.
#[test]
#[ignore = "#327 B2: real PTY/custody retry boundary; scripts/night-test-boundaries.sh"]
fn pending_logic_keeps_pty_live_retries_at_30s_and_releases_durable_sends_in_order() {
    use bbt_mux::{
        LaunchSpec, Supervisor,
        bus::{Consumer, Delivery},
        identity::{HoldWindow, IdentityState},
        logic::{Logic, LogicConfig},
        messaging::{
            api::{Request, Response},
            wire::SendRequest,
        },
        registry::SeatRecord,
    };
    use std::time::{Duration, Instant};
    let machine = Arc::new(Memory::default());
    *machine.blocked.lock().unwrap() = Some(PendingReason::KeychainUnavailable(-25308));
    let root = tempfile::tempdir().unwrap();
    let repository = root.path().join(".bboxy");
    std::fs::create_dir(&repository).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(common::capture_config(root.path())).unwrap();
    let logic = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: root.path().into(),
            repository,
            ring: Default::default(),
        },
        machine.clone(),
        runtime.handle(),
    )
    .unwrap();
    let initial = runtime.block_on(logic.client.identity_status()).unwrap();
    assert_eq!(initial.state, IdentityState::Pending);
    assert!(initial.custody_held_since.is_some());
    assert_eq!(initial.open_ephemeral_holds, 1);
    assert_eq!(machine.creates.load(Ordering::Relaxed), 0);
    assert!(
        store(root.path())
            .get("messaging/issuer")
            .unwrap()
            .is_none()
    );
    let sub = runtime
        .block_on(
            logic
                .client
                .bus
                .subscribe(Consumer::gui("account/identity/state")),
        )
        .unwrap();
    let mut spec = LaunchSpec::new("/bin/zsh", root.path());
    spec.args=vec!["-f".into(),"-c".into(),"printf 'HELD-PTY-ALIVE\\n'; read -r line; printf 'RECOVERED-%s\\n' \"$line\"; read -r done".into()];
    let seat = SeatRecord::new(spec.seat, Ulid::new(), "held fixture", "zsh");
    runtime
        .block_on(logic.client.spawn(seat.clone(), spec))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let snap = runtime.block_on(logic.client.snapshot(seat.id, 1)).unwrap();
        let text: String = snap.screen.cells.iter().flatten().map(|c| c.c).collect();
        if text.contains("HELD-PTY-ALIVE") {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut ids = Vec::new();
    for text in ["first held", "second held"] {
        let message = SendRequest::text(format!("seats/{}/inbox", seat.id), text);
        ids.push(message.id);
        let response = runtime
            .block_on(
                logic
                    .client
                    .messaging(seat.id, 1, Request::Send { message }),
            )
            .unwrap();
        assert!(matches!(response,Response::Stored{message} if Some(&message.view().id)==ids.last() && message.envelope().is_none()));
    }
    assert_eq!(
        store(root.path())
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM msg_held WHERE state='held'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
    *machine.blocked.lock().unwrap() = None;
    let until = Instant::now() + Duration::from_secs(35);
    let mut saw_ready = false;
    while Instant::now() < until {
        let deliveries = runtime.block_on(logic.client.bus.poll(&sub, 16)).unwrap();
        for event in deliveries {
            if let Delivery::Event { envelope, .. } = event {
                let status: bbt_mux::identity::IdentityStatus =
                    serde_json::from_value(envelope.event.payload).unwrap();
                if status.state == IdentityState::LocalDev {
                    saw_ready = true;
                }
            }
        }
        if saw_ready {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(saw_ready, "30s custody recovery transition missing");
    assert!(
        store(root.path())
            .get("identity/local-issuer-hold")
            .unwrap()
            .is_none()
    );
    let Response::Messages { page } = runtime
        .block_on(logic.client.messaging(
            seat.id,
            1,
            Request::List {
                thread: None,
                channel: None,
                after: 0,
                limit: 16,
            },
        ))
        .unwrap()
    else {
        panic!("messages")
    };
    assert_eq!(
        page.items.iter().map(|m| m.envelope().expect("signed fixture envelope").id).collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        page.items
            .iter()
            .map(|m| m.envelope().expect("signed fixture envelope").sender_sequence)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    let windows = store(root.path()).list("identity/bus-signers/").unwrap();
    let window: HoldWindow = serde_json::from_slice(&windows[0].1).unwrap();
    assert_eq!(window.kind, "ephemeral-hold");
    assert!(!window.boot_id.is_empty());
    assert!(window.ended_at.is_some());
    let ranges = window.ranges.unwrap();
    assert!(
        ranges
            .iter()
            .any(|r| r.stream == "account" && r.last >= r.first)
    );
    assert!(
        store(root.path())
            .list("identity/hold-closures/")
            .unwrap()
            .is_empty(),
        "no primary means no device attribution"
    );
    let status = runtime.block_on(logic.client.identity_status()).unwrap();
    assert_eq!(status.open_ephemeral_holds, 1);
    assert_eq!(status.custody_held_since, None);
    logic.shutdown().unwrap();
    println!(
        "ID1d: live PTY during pending custody; unsigned sends durable; actual 30s retry, status event, ordered release; no-primary range stays visibly ephemeral"
    );
}
