#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
//! Disposable interoperability/custody checks. Every GnuPG command has an
//! explicit throwaway homedir; these tests never query the operator keyring.
mod common;
use bbt_mux::messaging::wire::EnvelopeExt;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bbt_mux::{
    identity::{
        AccountHead, Bootstrap, BootstrapOutcome, Custody, DeviceKeyStore, KeychainDeviceKeys,
        PendingReason, Signed,
    },
    store::{Store, sqlite::SqliteStore},
};
use ed25519_dalek::SigningKey;
use pgp::{
    composed::{Deserializable, SignedPublicKey},
    types::KeyDetails,
};
use tempfile::TempDir;
use ulid::Ulid;

#[derive(Default)]
struct MemoryKeys(RefCell<BTreeMap<Ulid, SigningKey>>);
impl DeviceKeyStore for MemoryKeys {
    fn custody(&self) -> Custody {
        Custody::Memory
    }
    fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        Ok(self.0.borrow().get(&id).cloned())
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
        let mut keys = self.0.borrow_mut();
        if keys.contains_key(&id) {
            return Err(PendingReason::JournalConflict);
        }
        keys.insert(id, key.clone());
        Ok(())
    }
}
struct Fixture {
    home: TempDir,
    bootstrap: Bootstrap,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
fn gpg(home: &Path, args: &[&str]) -> Output {
    Command::new("gpg")
        .args(["--no-options", "--homedir"])
        .arg(home)
        .args(["--batch", "--no-auto-key-retrieve"])
        .args(args)
        .env("GNUPGHOME", home)
        .output()
        .unwrap()
}
fn success(output: Output) -> Vec<u8> {
    assert!(
        output.status.success(),
        "GnuPG exit {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}
impl Fixture {
    fn new() -> Self {
        let home = tempfile::Builder::new()
            .prefix("id1a-gpg-")
            .tempdir_in("/tmp")
            .unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(home.path(), fs::Permissions::from_mode(0o700)).unwrap();
        success(gpg(
            home.path(),
            &[
                "--pinentry-mode",
                "loopback",
                "--passphrase",
                "",
                "--quick-generate-key",
                "ID1a disposable <fixture@invalid.test>",
                "ed25519",
                "cert,sign",
                "0",
            ],
        ));
        let metadata = String::from_utf8(success(gpg(
            home.path(),
            &["--with-colons", "--with-keygrip", "--list-keys"],
        )))
        .unwrap();
        let field = |kind: &str| {
            metadata
                .lines()
                .find(|l| l.starts_with(kind))
                .unwrap()
                .split(':')
                .nth(9)
                .unwrap()
                .to_owned()
        };
        let fingerprint = field("fpr:");
        let keygrip = field("grp:");
        let certificate = success(gpg(home.path(), &["--export", &fingerprint]));
        let socket = Command::new("gpgconf")
            .args(["--homedir"])
            .arg(home.path())
            .args(["--list-dirs", "agent-socket"])
            .env("GNUPGHOME", home.path())
            .output()
            .unwrap();
        let socket = PathBuf::from(String::from_utf8(success(socket)).unwrap().trim());
        Self {
            home,
            bootstrap: Bootstrap {
                fingerprint,
                public_certificate: certificate,
                keygrip,
                agent_socket: socket,
                timeout: Duration::from_secs(10),
            },
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let status = Command::new("gpgconf")
            .arg("--homedir")
            .arg(self.home.path())
            .args(["--kill", "gpg-agent"])
            .env("GNUPGHOME", self.home.path())
            .status()
            .unwrap();
        assert!(status.success());
    }
}
fn account() -> (TempDir, SqliteStore) {
    let dir = tempfile::Builder::new()
        .prefix("id1a-account-")
        .tempdir_in("/tmp")
        .unwrap();
    let store = SqliteStore::open(dir.path()).unwrap();
    (dir, store)
}
fn snapshot(home: &Path) -> Vec<Vec<u8>> {
    ["pubring.kbx", "trustdb.gpg"]
        .map(|p| fs::read(home.join(p)).unwrap())
        .to_vec()
}

#[test]
fn identity_bootstrap_external_certification_and_custody() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.home.path());
    let (directory, account) = account();
    let keys = MemoryKeys::default();
    let time = now();
    let outcome = fixture.bootstrap.run(&account, &keys, time);
    let identity = match outcome {
        BootstrapOutcome::Certified(identity) => identity,
        pending => panic!("{pending:?}"),
    };
    let head: Signed<AccountHead> =
        serde_json::from_slice(&account.get("identity/head").unwrap().unwrap()).unwrap();
    identity
        .verify(&fixture.bootstrap.fingerprint, &head, time)
        .unwrap();
    assert!(account.get("identity/pending").unwrap().is_none());
    assert_eq!(account.get("identity/roster").unwrap().unwrap(), b"[]");
    assert_eq!(keys.0.borrow().len(), 2); // local issuer plus the first-launch device
    let local: serde_json::Value =
        serde_json::from_slice(&account.get("identity/local-issuer").unwrap().unwrap()).unwrap();
    assert_eq!(
        local["device_id"],
        serde_json::json!(identity.grant.payload.device_id)
    );

    let verifier = tempfile::Builder::new()
        .prefix("id1a-verify-")
        .tempdir_in("/tmp")
        .unwrap();
    let root_file = verifier.path().join("root.pgp");
    let device_file = verifier.path().join("device.pgp");
    fs::write(&root_file, &fixture.bootstrap.public_certificate).unwrap();
    let certificate = identity.public_device_certificate().unwrap();
    fs::write(&device_file, &certificate).unwrap();
    let device = SignedPublicKey::from_bytes(&certificate[..]).unwrap();
    success(gpg(
        verifier.path(),
        &[
            "--import",
            root_file.to_str().unwrap(),
            device_file.to_str().unwrap(),
        ],
    ));
    let check = success(gpg(
        verifier.path(),
        &[
            "--with-colons",
            "--check-sigs",
            &device.fingerprint().to_string(),
        ],
    ));
    let check = String::from_utf8(check).unwrap();
    let key_id = &fixture.bootstrap.fingerprint[24..];
    assert!(
        check
            .lines()
            .any(|line| line.starts_with("sig:!:") && line.split(':').nth(4) == Some(key_id)),
        "{check}"
    );
    println!(
        "external gpg --check-sigs: good disposable-root certification; fingerprint {}",
        fixture.bootstrap.fingerprint
    );
    assert_eq!(
        snapshot(fixture.home.path()),
        before,
        "source public keyring/trustdb changed"
    );

    // Retry reads verified durable authority and Keychain; agent can be absent.
    let mut retry = Bootstrap {
        fingerprint: fixture.bootstrap.fingerprint.clone(),
        public_certificate: fixture.bootstrap.public_certificate.clone(),
        keygrip: fixture.bootstrap.keygrip.clone(),
        agent_socket: PathBuf::from("/nonexistent-id1a-agent"),
        timeout: Duration::from_millis(200),
    };
    assert!(matches!(
        retry.run(&account, &keys, time),
        BootstrapOutcome::Certified(_)
    ));
    retry.fingerprint = "0".repeat(40);
    assert!(matches!(
        retry.run(&account, &keys, time),
        BootstrapOutcome::Pending(PendingReason::FingerprintMismatch)
    ));
    let mut changed = (*identity).clone();
    changed.grant.payload.role_ceiling.clear();
    assert!(
        changed
            .verify(&fixture.bootstrap.fingerprint, &head, time)
            .is_err()
    );
    assert!(
        identity
            .verify(&fixture.bootstrap.fingerprint, &head, time + 30 * 24 * 3600)
            .is_err()
    );

    // No secret in the account journal/files, environment or child arguments.
    fn files(path: &Path) -> Vec<Vec<u8>> {
        fs::read_dir(path)
            .unwrap()
            .flat_map(|entry| {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    files(&path)
                } else {
                    vec![fs::read(path).unwrap()]
                }
            })
            .collect()
    }
    for key in keys.0.borrow().values() {
        let seed = key.to_bytes();
        let encoded: String = seed.iter().map(|b| format!("{b:02x}")).collect();
        for bytes in files(directory.path()) {
            assert!(!bytes.windows(seed.len()).any(|b| b == seed));
            assert!(
                !bytes
                    .windows(encoded.len())
                    .any(|b| b == encoded.as_bytes())
            );
        }
        for (_, value) in std::env::vars_os() {
            assert!(!value.to_string_lossy().contains(&encoded));
        }
        assert!(!std::env::args().any(|arg| arg.contains(&encoded)));
    }
}

#[test]
fn identity_pending_failure_installs_no_authority_and_retries_same_device() {
    let mut fixture = Fixture::new();
    let agent_socket = fixture.bootstrap.agent_socket.clone();
    fixture.bootstrap.agent_socket = PathBuf::from("/nonexistent-id1a-agent");
    let (_directory, account) = account();
    let keys = MemoryKeys::default();
    assert!(matches!(
        fixture.bootstrap.run(&account, &keys, now()),
        BootstrapOutcome::Pending(PendingReason::AgentUnavailable)
    ));
    assert!(account.get("identity/head").unwrap().is_none());
    assert!(account.list("identity/device/").unwrap().is_empty());
    assert!(account.get("identity/roster").unwrap().is_none());
    let local: serde_json::Value =
        serde_json::from_slice(&account.get("identity/local-issuer").unwrap().unwrap()).unwrap();
    let id: Ulid = serde_json::from_value(local["device_id"].clone()).unwrap();
    fixture.bootstrap.agent_socket = agent_socket;
    match fixture.bootstrap.run(&account, &keys, now()) {
        BootstrapOutcome::Certified(identity) => assert_eq!(identity.grant.payload.device_id, id),
        pending => panic!("{pending:?}"),
    }
    assert_eq!(keys.0.borrow().len(), 2);
}

#[test]
fn identity_locked_keychain_and_wrong_keygrip_install_nothing() {
    let mut fixture = Fixture::new();
    let (_directory, store) = account();
    struct Locked;
    impl DeviceKeyStore for Locked {
        fn custody(&self) -> Custody {
            Custody::Memory
        }
        fn load(&self, _: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
            Err(PendingReason::KeychainUnavailable(-25308))
        }
        fn create(&self, _: Ulid, _: &SigningKey) -> bbt_mux::identity::Result<()> {
            unreachable!()
        }
    }
    assert!(matches!(
        fixture.bootstrap.run(&store, &Locked, now()),
        BootstrapOutcome::Pending(PendingReason::KeychainUnavailable(-25308))
    ));
    assert!(store.get("identity/head").unwrap().is_none());
    fixture.bootstrap.keygrip = "0".repeat(40);
    assert!(matches!(
        fixture.bootstrap.run(&store, &MemoryKeys::default(), now()),
        BootstrapOutcome::Pending(PendingReason::AgentRejected(_))
    ));
    assert!(store.get("identity/head").unwrap().is_none());
    assert!(store.list("identity/device/").unwrap().is_empty());
}

#[test]
#[cfg(target_os = "macos")]
#[ignore = "uses the current login Keychain with an isolated, deleted test item; run for ID1a acceptance"]
fn identity_keychain_device_local_item_and_cleanup() {
    let service = format!("bboxy450-term.test.{}", Ulid::new());
    let store = KeychainDeviceKeys::isolated_test(&service).unwrap();
    let fixture = Fixture::new();
    let (_directory, account) = account();
    let outcome = fixture.bootstrap.run(&account, &store, now());
    let identity = match outcome {
        BootstrapOutcome::Certified(identity) => identity,
        pending => {
            if let Some(bytes) = account.get("identity/pending").unwrap() {
                let record: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                let id: Ulid = record["id"].as_str().unwrap().parse().unwrap();
                store.delete_test_item(id).unwrap();
            }
            panic!("{pending:?}");
        }
    };
    let id = identity.grant.payload.device_id;
    assert_eq!(identity.custody, store.custody());
    let stored: serde_json::Value = serde_json::from_slice(
        &account
            .get(&format!("identity/device/{id}"))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        stored["custody"],
        serde_json::to_value(store.custody()).unwrap()
    );
    struct Cleanup<'a>(&'a KeychainDeviceKeys, Ulid);
    impl Drop for Cleanup<'_> {
        fn drop(&mut self) {
            self.0.delete_test_item(self.1).unwrap();
        }
    }
    let _cleanup = Cleanup(&store, id);
    let key = store.load(id).unwrap().unwrap();
    let public: String = key
        .verifying_key()
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(public, identity.grant.payload.device_pubkey);
    use security_framework::item::{ItemClass, ItemSearchOptions, SearchResult};
    let mut query = ItemSearchOptions::new();
    query
        .class(ItemClass::generic_password())
        .service(&service)
        .account(&id.to_string())
        .load_attributes(true);
    if store.custody() == Custody::DataProtection {
        query.ignore_legacy_keychains().cloud_sync(Some(false));
    } else {
        query
            .keychains(&[security_framework::os::macos::keychain::SecKeychain::default().unwrap()]);
    }
    let attrs = query.search().unwrap();
    let SearchResult::Dict(attrs) = &attrs[0] else {
        panic!("missing Keychain attributes")
    };
    if store.custody() == Custody::DataProtection {
        assert!(
            format!("{attrs:?}").contains("aku"),
            "device-only unlocked protection missing: {attrs:?}"
        );
    } else {
        assert_eq!(store.custody().label(), "dev custody");
        println!(
            "login Keychain: isolated service/account attributes present; dev custody, no sync API"
        );
    }
    store.delete_test_item(id).unwrap();
    assert!(store.load(id).unwrap().is_none());
    println!(
        "Keychain tier {:?}: isolated item created, read, attributes checked and deleted",
        store.custody()
    );
}

// ID1b uses shared, memory-only test custody. Only public proofs reach SQLite.
#[derive(Default)]
struct SharedKeys(std::sync::Mutex<BTreeMap<Ulid, SigningKey>>);
impl DeviceKeyStore for SharedKeys {
    fn custody(&self) -> Custody {
        Custody::Memory
    }
    fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
        Ok(self.0.lock().unwrap().get(&id).cloned())
    }
    fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
        let mut keys = self.0.lock().unwrap();
        if keys.contains_key(&id) {
            return Err(PendingReason::JournalConflict);
        }
        keys.insert(id, key.clone());
        Ok(())
    }
}
struct SeatFixture {
    root: Fixture,
    _directory: TempDir,
    account: SqliteStore,
    custody: std::sync::Arc<SharedKeys>,
    registry: bbt_mux::registry::Registry,
    chain: std::sync::Arc<bbt_mux::identity::ChainSeatKeys>,
    messaging: bbt_mux::messaging::Messaging,
    clock: std::sync::Arc<std::sync::atomic::AtomicU64>,
    seat: bbt_mux::registry::SeatRecord,
    peer: bbt_mux::registry::SeatRecord,
    prior: Option<bbt_mux::messaging::wire::Envelope>,
}
impl SeatFixture {
    fn new(cutover: bool) -> Self {
        use bbt_mux::{
            identity::ChainSeatKeys,
            messaging::{Messaging, keys::LocalSeatKeys, wire::SendRequest},
            registry::{Registry, SeatRecord},
        };
        use std::sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        };
        let root = Fixture::new();
        let directory = tempfile::Builder::new()
            .prefix("id1b-")
            .tempdir_in("/tmp")
            .unwrap();
        let account = SqliteStore::open(&directory.path().join("account")).unwrap();
        let registry = Registry::new(account.clone());
        let time = now();
        let clock = Arc::new(AtomicU64::new(time));
        let custody = Arc::new(SharedKeys::default());
        let swarm = Ulid::new();
        let mut seat = SeatRecord::new(Ulid::new(), swarm, "Kestrel fixture", "codex");
        seat.location = bbt_mux::registry::Location::Ssh {
            host: "private-host.invalid".into(),
        };
        let peer = SeatRecord::new(Ulid::new(), swarm, "Peer fixture", "zsh");
        let prior = if cutover {
            registry.insert(&seat).unwrap();
            registry.insert(&peer).unwrap();
            let mut local = Messaging::open(
                directory.path(),
                Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[42; 32]))),
            )
            .unwrap();
            Some(
                local
                    .send(
                        seat.id,
                        1,
                        SendRequest::text(format!("seats/{}/inbox", peer.id), "before cutover"),
                        time * 1000,
                    )
                    .unwrap()
                    .envelope,
            )
        } else {
            None
        };
        assert!(matches!(
            root.bootstrap.run(&account, custody.as_ref(), time),
            BootstrapOutcome::Certified(_)
        ));
        if !cutover {
            registry.insert(&seat).unwrap();
            registry.insert(&peer).unwrap();
        }
        let clock_copy = clock.clone();
        let chain = Arc::new(
            ChainSeatKeys::with_clock(
                account.clone(),
                custody.clone(),
                &root.bootstrap.fingerprint,
                SigningKey::from_bytes(&[42; 32]),
                Arc::new(move || clock_copy.load(Ordering::Relaxed)),
            )
            .unwrap(),
        );
        let messaging = Messaging::open(directory.path(), chain.clone()).unwrap();
        Self {
            root,
            _directory: directory,
            account,
            custody,
            registry,
            chain,
            messaging,
            clock,
            seat,
            peer,
            prior,
        }
    }
    fn time(&self) -> u64 {
        self.clock.load(std::sync::atomic::Ordering::Relaxed)
    }
    fn send(&mut self, text: &str) -> bbt_mux::messaging::ledger::StoredMessage {
        self.messaging
            .send(
                self.seat.id,
                self.seat.generation,
                bbt_mux::messaging::wire::SendRequest::text(
                    format!("seats/{}/inbox", self.peer.id),
                    text,
                ),
                self.time() * 1000,
            )
            .unwrap()
    }
}
#[test]
fn identity_operator_uses_the_current_device_grant_and_refuses_seat_or_local_authority() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use bbt_mux::{
        identity::DeviceIdentity,
        messaging::{
            keys::SeatKeys,
            wire::{IssuerKind, public},
        },
        registry::{Class, Role},
    };
    use ed25519_dalek::Signer;
    let mut f = SeatFixture::new(false);
    let mut operator = f.send("device-signed operator").envelope;
    let head: Signed<AccountHead> =
        serde_json::from_slice(&f.account.get("identity/head").unwrap().unwrap()).unwrap();
    let device_id = head.payload.active_devices[0].device_id;
    let identity: DeviceIdentity = serde_json::from_slice(
        &f.account
            .get(&format!("identity/device/{device_id}"))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    let key = f.custody.load(device_id).unwrap().unwrap();
    operator.seat = Ulid::nil();
    operator.process_generation = 0;
    operator.author = identity.grant.payload.device_pubkey.clone();
    operator.role = Role {
        name: "operator".into(),
        class: Class::Oracle,
    };
    operator.priority_class = Class::Oracle;
    operator.role_grant_id = identity.grant.digest.clone();
    operator.role_epoch = identity.grant.payload.device_epoch.parse().unwrap();
    operator.sign(&key).unwrap();
    f.chain.verify(&operator).unwrap();
    for which in 0..5 {
        let mut forged = operator.clone();
        match which {
            0 => forged.role_grant_id = "11".repeat(32),
            1 => forged.role_epoch += 1,
            2 => forged.timestamp = identity.grant.payload.not_before - 1,
            3 => forged.role.name = "executor".into(),
            _ => forged.process_generation = 1,
        }
        forged.signature =
            URL_SAFE_NO_PAD.encode(key.sign(&forged.signing_bytes().unwrap()).to_bytes());
        assert!(f.chain.verify(&forged).is_err(), "forgery {which}");
    }
    let local_key = SigningKey::from_bytes(&[42; 32]);
    let mut local = operator.clone();
    local.issuer_kind = IssuerKind::LocalDev;
    local.issuer = public(&local_key.verifying_key().to_bytes());
    local.account = local.issuer.clone();
    local.device = local.issuer.clone();
    local.author = local.issuer.clone();
    local.sign(&local_key).unwrap();
    assert!(
        f.chain
            .verify(&local)
            .unwrap_err()
            .to_string()
            .contains("refuse local-dev operator")
    );
    f.clock.store(
        identity.grant.payload.expires_at,
        std::sync::atomic::Ordering::Relaxed,
    );
    assert!(
        f.chain
            .verify(&operator)
            .unwrap_err()
            .to_string()
            .contains("expired DeviceGrant")
    );
}

#[test]
fn identity_seat_chain_checks_certificates_scope_expiry_and_revoked_epochs() {
    use bbt_mux::messaging::{keys::SeatKeys, wire::IssuerKind};
    let mut f = SeatFixture::new(false);
    let message = f.send("root-certified seat").envelope;
    assert_eq!(message.issuer_kind, IssuerKind::GpgRoot);
    assert_ne!(message.device, message.issuer);
    f.chain.verify(&message).unwrap();
    let proof = f.chain.proof(&message.role_grant_id).unwrap();
    assert_eq!(proof.grant.payload.role, f.seat.role);
    assert_eq!(
        proof.grant.payload.destinations.as_slice(),
        std::slice::from_ref(&message.context)
    );
    assert_eq!(
        proof.grant.payload.expires_at - proof.grant.payload.not_before,
        24 * 3600
    );
    let leaf = f.chain.leaf(f.seat.id, 1).unwrap();
    assert_eq!(
        leaf.to_bytes(),
        f.chain.leaf(f.seat.id, 1).unwrap().to_bytes()
    );
    assert_ne!(
        leaf.to_bytes(),
        f.chain.leaf(f.seat.id, 2).unwrap().to_bytes()
    );
    let mut body = message.clone();
    body.body = bbt_mux::messaging::wire::Body::Text {
        text: "tampered".into(),
    };
    assert!(f.chain.verify(&body).is_err());
    let mut wrong = message.clone();
    wrong.device = "11".repeat(32);
    wrong.sign(&leaf).unwrap();
    assert!(
        f.chain
            .verify(&wrong)
            .unwrap_err()
            .to_string()
            .contains("wrong device")
    );
    wrong = message.clone();
    wrong.issuer = "11".repeat(32);
    wrong.account = wrong.issuer.clone();
    wrong.sign(&leaf).unwrap();
    assert!(
        f.chain
            .verify(&wrong)
            .unwrap_err()
            .to_string()
            .contains("foreign root")
    );
    wrong = message.clone();
    wrong.role_epoch = 2;
    wrong.sign(&leaf).unwrap();
    assert!(
        f.chain
            .verify(&wrong)
            .unwrap_err()
            .to_string()
            .contains("stale grant_epoch")
    );
    assert_eq!(f.chain.revoke_seat(f.seat.id).unwrap(), 2);
    assert_eq!(f.chain.revoke_seat(f.seat.id).unwrap(), 3);
    assert!(
        f.chain
            .verify(&message)
            .unwrap_err()
            .to_string()
            .contains("revoked grant_epoch")
    );
    f.chain.verify_history(&message).unwrap();
    f.clock
        .store(f.time() + 24 * 3600, std::sync::atomic::Ordering::Relaxed);
    assert!(
        f.chain
            .verify(&message)
            .unwrap_err()
            .to_string()
            .contains("expired SeatGrant")
    );
    println!(
        "ID1b: gpg-root envelope verified; tamper, wrong device, foreign root, stale/revoked grant and expiry rejected"
    );
}
#[test]
fn identity_seat_renewal_never_uses_the_primary_and_roster_keeps_public_history() {
    use bbt_mux::messaging::keys::SeatKeys;
    let mut f = SeatFixture::new(false);
    let old = f.send("first").envelope;
    let old_key = f.chain.leaf(f.seat.id, 1).unwrap();
    let stop = Command::new("gpgconf")
        .arg("--homedir")
        .arg(f.root.home.path())
        .args(["--kill", "gpg-agent"])
        .env("GNUPGHOME", f.root.home.path())
        .status()
        .unwrap();
    assert!(stop.success());
    f.clock
        .store(f.time() + 12 * 3600, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        f.chain.renew_due().unwrap(),
        1,
        "idle seat renews before another send"
    );
    let renewed = f.send("renew at half-life").envelope;
    assert_ne!(old.role_grant_id, renewed.role_grant_id);
    assert_eq!(old.author, renewed.author);
    let proof = f.chain.proof(&renewed.role_grant_id).unwrap();
    assert_eq!(
        proof.grant.payload.previous_grant_id,
        Some(old.role_grant_id.clone())
    );
    f.chain.verify(&renewed).unwrap();
    f.seat.generation = 2;
    f.registry
        .update(f.seat.id, |r| {
            r.generation = 2;
            Ok(())
        })
        .unwrap();
    let next = f.send("new process generation").envelope;
    assert_ne!(old.author, next.author);
    assert!(
        f.chain
            .verify(&old)
            .unwrap_err()
            .to_string()
            .contains("stale key_epoch")
    );
    f.chain.verify_history(&old).unwrap();
    let roster = f.chain.roster().unwrap().unwrap();
    assert_eq!(roster.payload.entries.len(), 2);
    assert!(
        roster
            .payload
            .entries
            .iter()
            .any(|e| e.status == bbt_mux::identity::KeyStatus::Retired)
    );
    assert!(roster.payload.previous_roster_id.is_some());
    let public = serde_json::to_string(&roster).unwrap();
    assert!(!public.contains("private-host.invalid"));
    assert!(!public.contains("seats/"));
    assert!(!public.contains(&f.seat.swarm.to_string()));
    fn files(path: &Path) -> Vec<Vec<u8>> {
        fs::read_dir(path)
            .unwrap()
            .flat_map(|e| {
                let p = e.unwrap().path();
                if p.is_dir() {
                    files(&p)
                } else {
                    vec![fs::read(p).unwrap()]
                }
            })
            .collect()
    }
    for bytes in files(f._directory.path()) {
        assert!(!bytes.windows(32).any(|b| b == old_key.to_bytes()));
    }
    println!(
        "ID1b: half-life renewal after primary agent stopped; generation key rotated; signed roster/history retained without private scope or seed"
    );
}
#[test]
fn identity_cutover_preserves_live_local_generation_and_fences_newer_local_sequences() {
    use bbt_mux::messaging::{
        keys::{LocalSeatKeys, SeatKeys},
        wire::IssuerKind,
    };
    let mut f = SeatFixture::new(true);
    let old = f.prior.clone().unwrap();
    let current = f.send("same running generation").envelope;
    assert_eq!(current.issuer_kind, IssuerKind::LocalDev);
    assert_eq!(current.author, old.author);
    f.chain.verify(&old).unwrap();
    f.chain.verify(&current).unwrap();
    assert_eq!(f.chain.still_local().unwrap(), 2);
    f.seat.generation = 2;
    f.registry
        .update(f.seat.id, |r| {
            r.generation = 2;
            Ok(())
        })
        .unwrap();
    let mut late_before_first_chain = current.clone();
    late_before_first_chain.sender_sequence += 1;
    late_before_first_chain
        .sign(
            &LocalSeatKeys::new(SigningKey::from_bytes(&[42; 32]))
                .leaf(f.seat.id, 1)
                .unwrap(),
        )
        .unwrap();
    assert!(
        f.chain
            .verify(&late_before_first_chain)
            .unwrap_err()
            .to_string()
            .contains("after cutover")
    );
    let next = f.send("new generation uses chain").envelope;
    assert_eq!(next.issuer_kind, IssuerKind::GpgRoot);
    f.chain.verify(&next).unwrap();
    assert_eq!(f.chain.still_local().unwrap(), 1);
    f.chain.verify_history(&old).unwrap();
    f.chain.verify(&current).unwrap();
    let mut late = current.clone();
    late.sender_sequence += 1;
    late.sign(
        &LocalSeatKeys::new(SigningKey::from_bytes(&[42; 32]))
            .leaf(f.seat.id, 1)
            .unwrap(),
    )
    .unwrap();
    assert!(
        f.chain
            .verify(&late)
            .unwrap_err()
            .to_string()
            .contains("after cutover")
    );
    println!(
        "ID1b cutover: live generation kept its local key; next generation gpg-root; pre-fence local history verified, newer local sequence refused"
    );
}
#[test]
fn identity_reset_is_explicit_monotonic_and_does_not_rewrite_old_chain_history() {
    use bbt_mux::{
        identity::{ChainSeatKeys, reset_identity},
        messaging::{
            keys::{LocalSeatKeys, SeatKeys},
            wire::IssuerKind,
        },
    };
    let mut f = SeatFixture::new(false);
    let old = f.send("gpg history").envelope;
    let saved = f.account.get("identity/issuer").unwrap().unwrap();
    let local = LocalSeatKeys::new(SigningKey::from_bytes(&[43; 32]));
    let reset = reset_identity(&f.account, &local, f.time()).unwrap();
    assert_eq!(
        reset.previous,
        Some(bbt_mux::store::digest(
            &f.account.get("identity/issuer-history/1").unwrap().unwrap()
        ))
    );
    assert_eq!(reset.cutover_epoch, "2");
    assert_eq!(reset.kind, IssuerKind::LocalDev);
    assert!(reset.previous.is_some());
    f.chain.verify_history(&old).unwrap();
    assert!(
        f.chain
            .verify(&old)
            .unwrap_err()
            .to_string()
            .contains("identity reset")
    );
    assert!(matches!(
        f.root
            .bootstrap
            .run(&f.account, f.custody.as_ref(), f.time()),
        BootstrapOutcome::Certified(_)
    ));
    let state: bbt_mux::identity::IssuerState =
        serde_json::from_slice(&f.account.get("identity/issuer").unwrap().unwrap()).unwrap();
    assert_eq!(state.cutover_epoch, "2");
    assert_eq!(state.kind, IssuerKind::LocalDev);
    let second = reset_identity(&f.account, &local, f.time()).unwrap();
    assert_eq!(second.cutover_epoch, "3");
    // Reset permits a new local generation even after the old device expires
    // and its secret is lost; old GPG attribution needs public proofs only.
    f.custody.0.lock().unwrap().clear();
    let future = f.time() + 40 * 24 * 3600;
    let historical = std::sync::Arc::new(
        ChainSeatKeys::with_clock(
            f.account.clone(),
            f.custody.clone(),
            &f.root.bootstrap.fingerprint,
            SigningKey::from_bytes(&[43; 32]),
            std::sync::Arc::new(move || future),
        )
        .unwrap(),
    );
    historical.verify_history(&old).unwrap();
    f.registry
        .update(f.seat.id, |r| {
            r.generation = 2;
            Ok(())
        })
        .unwrap();
    let mut local_messaging =
        bbt_mux::messaging::Messaging::open(f._directory.path(), historical.clone()).unwrap();
    let new_local = local_messaging
        .send(
            f.seat.id,
            2,
            bbt_mux::messaging::wire::SendRequest::text(
                format!("seats/{}/inbox", f.peer.id),
                "new reset epoch",
            ),
            future * 1000,
        )
        .unwrap();
    assert_eq!(new_local.envelope.issuer_kind, IssuerKind::LocalDev);
    historical.verify(&new_local.envelope).unwrap();
    f.account.put("identity/issuer", &saved).unwrap();
    let reopened = ChainSeatKeys::open(
        f.account.clone(),
        f.custody.clone(),
        &f.root.bootstrap.fingerprint,
        SigningKey::from_bytes(&[43; 32]),
    );
    assert!(reopened.err().unwrap().to_string().contains("rollback"));
    println!(
        "ID1b reset: explicit local-dev epochs 2/3; old root history retained; bootstrap retry does not undo reset; rollback rejected"
    );
}
#[test]
fn identity_device_without_root_certification_is_refused_before_seat_issuance() {
    use base64::Engine;
    use pgp::ser::Serialize;
    let f = SeatFixture::new(false);
    let head: Signed<AccountHead> =
        serde_json::from_slice(&f.account.get("identity/head").unwrap().unwrap()).unwrap();
    let id = head.payload.active_devices[0].device_id;
    let path = format!("identity/device/{id}");
    let mut identity: bbt_mux::identity::DeviceIdentity =
        serde_json::from_slice(&f.account.get(&path).unwrap().unwrap()).unwrap();
    let cert = identity.public_device_certificate().unwrap();
    let mut cert = SignedPublicKey::from_bytes(cert.as_slice()).unwrap();
    cert.details.users[0].signatures.truncate(1);
    identity.device_certificate =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(cert.to_bytes().unwrap());
    f.account
        .put(&path, &serde_json::to_vec(&identity).unwrap())
        .unwrap();
    assert!(
        bbt_mux::identity::ChainSeatKeys::open(
            f.account.clone(),
            f.custody.clone(),
            &f.root.bootstrap.fingerprint,
            SigningKey::from_bytes(&[42; 32])
        )
        .is_err()
    );
}

#[test]
fn identity_chain_provider_preserves_messaging_store_retry_thread_receipt_and_ack_contracts() {
    use bbt_mux::{
        adapters::{Evidence, WakeOutcome},
        messaging::wire::SendRequest,
    };
    let mut f = SeatFixture::new(false);
    let first = f.send("parent");
    let mut request = SendRequest::text(format!("seats/{}/inbox", f.peer.id), "reply");
    request.reply_to = Some(first.envelope.id);
    let reply = f
        .messaging
        .send(f.seat.id, 1, request.clone(), f.time() * 1000)
        .unwrap();
    assert_eq!(reply.envelope.thread_id, first.envelope.thread_id);
    assert_eq!(
        f.messaging
            .send(f.seat.id, 1, request.clone(), f.time() * 1000)
            .unwrap(),
        reply
    );
    request.text.push_str(" changed");
    assert!(
        f.messaging
            .send(f.seat.id, 1, request, f.time() * 1000)
            .is_err()
    );
    assert_eq!(
        f.messaging.receipts(f.seat.id, first.envelope.id).unwrap()[0].evidence,
        Evidence::RecipientStored
    );
    f.messaging.ready(f.time() * 1000).unwrap();
    let work = f
        .messaging
        .prepare(f.peer.swarm, f.peer.id, f.time() * 1000)
        .unwrap()
        .unwrap();
    assert_eq!(work.messages[0].envelope.id, first.envelope.id);
    f.messaging
        .acknowledge(f.peer.id, first.envelope.id, None, f.time() * 1000)
        .unwrap();
    f.messaging
        .finish(
            f.peer.swarm,
            &work,
            &WakeOutcome {
                evidence: Evidence::NativeWritten,
                note: "fixture".into(),
                native_id: None,
            },
            f.time() * 1000,
            false,
        )
        .unwrap();
    assert_eq!(
        f.messaging.receipts(f.seat.id, first.envelope.id).unwrap()[0].evidence,
        Evidence::ApplicationAcknowledged
    );
    println!(
        "ID1b messaging seam: real chain provider preserved persist-before-dispatch, retry, thread, receipt and ack contracts"
    );
}

#[test]
fn identity_status_reports_no_primary_and_persists_safe_bootstrap_pending_reasons() {
    use bbt_mux::identity::{IdentityRuntime, IdentityState};
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let view = IdentityRuntime::open(
        directory.path(),
        SigningKey::from_bytes(&[42; 32]),
        runtime.handle().clone(),
    )
    .unwrap();
    let local = view.status().unwrap();
    assert_eq!(local.state, IdentityState::LocalDev);
    assert_eq!(local.label, "unsigned, local only");
    assert!(!local.bootstrapped);
    assert!(local.root_fingerprint.is_none());
    assert!(local.custody.is_none());
    assert_eq!(
        view.keys().issuer().kind,
        bbt_mux::messaging::wire::IssuerKind::LocalDev
    );
    let account = SqliteStore::open(&directory.path().join("account")).unwrap();
    let registry = bbt_mux::registry::Registry::new(account.clone());
    let swarm = Ulid::new();
    let sender = bbt_mux::registry::SeatRecord::new(Ulid::new(), swarm, "local sender", "zsh");
    let peer = bbt_mux::registry::SeatRecord::new(Ulid::new(), swarm, "local peer", "zsh");
    registry.insert(&sender).unwrap();
    registry.insert(&peer).unwrap();
    let mut messaging = bbt_mux::messaging::Messaging::open(directory.path(), view.keys()).unwrap();
    let envelope = messaging
        .send(
            sender.id,
            1,
            bbt_mux::messaging::wire::SendRequest::text(
                format!("seats/{}/inbox", peer.id),
                "local only",
            ),
            now() * 1000,
        )
        .unwrap()
        .envelope;
    assert_eq!(
        envelope.issuer_kind,
        bbt_mux::messaging::wire::IssuerKind::LocalDev
    );
    let receiver = SeatFixture::new(false);
    use bbt_mux::messaging::keys::SeatKeys;
    assert!(
        receiver
            .chain
            .verify(&envelope)
            .unwrap_err()
            .to_string()
            .contains("not admitted before cutover")
    );
    let mut fixture = Fixture::new();
    let agent_socket = fixture.bootstrap.agent_socket.clone();
    let custody = MemoryKeys::default();
    fixture.bootstrap.agent_socket = directory.path().join("absent-agent.sock");
    assert!(matches!(
        fixture.bootstrap.run(&account, &custody, now()),
        BootstrapOutcome::Pending(PendingReason::AgentUnavailable)
    ));
    let pending = view.status().unwrap();
    assert_eq!(pending.state, IdentityState::Pending);
    assert_eq!(
        pending.pending_reason,
        Some(PendingReason::AgentUnavailable)
    );
    assert_eq!(
        pending.root_fingerprint.as_deref(),
        Some(fixture.bootstrap.fingerprint.as_str())
    );
    assert!(!pending.authenticated_signing_ready);
    assert!(account.get("identity/head").unwrap().is_none());
    let reopened = IdentityRuntime::open(
        directory.path(),
        SigningKey::from_bytes(&[42; 32]),
        runtime.handle().clone(),
    )
    .unwrap();
    assert_eq!(
        reopened.status().unwrap().pending_reason,
        pending.pending_reason
    );
    let json = serde_json::to_string(&pending).unwrap();
    assert!(!json.contains("absent-agent.sock"));
    assert!(!json.contains("certificate"));
    assert!(!json.contains("keygrip"));
    fixture.bootstrap.agent_socket = agent_socket;
    assert!(matches!(
        fixture.bootstrap.run(&account, &custody, now()),
        BootstrapOutcome::Certified(_)
    ));
    assert!(account.get("identity/attempt").unwrap().is_none());
    let installed_after_open = view.status().unwrap();
    assert!(installed_after_open.bootstrapped);
    assert_eq!(
        installed_after_open.pending_reason,
        Some(PendingReason::ActivationRequired)
    );
    assert!(!installed_after_open.authenticated_signing_ready);
    println!(
        "ID1c: no-primary is unsigned/local-only; pending safe reason survives restart, with no authority installed"
    );
}

#[test]
fn identity_status_is_available_as_data_through_the_bounded_logic_client() {
    use bbt_mux::identity::IdentityState;
    use bbt_mux::logic::{Logic, LogicConfig};
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join(".bboxy");
    fs::create_dir(&repository).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = bbt_mux::Supervisor::start(common::capture_config(directory.path())).unwrap();
    let logic = Logic::start(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: directory.path().join("run"),
            repository,
            ring: Default::default(),
        },
        SigningKey::from_bytes(&[42; 32]),
        runtime.handle(),
    )
    .unwrap();
    let status = runtime.block_on(logic.client.identity_status()).unwrap();
    assert_eq!(status.state, IdentityState::LocalDev);
    assert_eq!(status.label, "unsigned, local only");
    assert!(status.root_fingerprint.is_none());
    let json = serde_json::to_string(&status).unwrap();
    assert!(!json.contains("capability"));
    assert!(!json.contains("socket"));
    logic.shutdown().unwrap();
    println!(
        "ID1c logic: bounded asynchronous client exposes unsigned/local-only status data; no root action or UI code"
    );
}

#[test]
fn identity_status_surfaces_renewal_expiry_custody_hold_and_live_legacy_count() {
    use bbt_mux::identity::{ChainSeatKeys, IdentityState};
    use std::sync::Arc;
    let f = SeatFixture::new(true);
    assert!(
        Command::new("gpgconf")
            .arg("--homedir")
            .arg(f.root.home.path())
            .args(["--kill", "gpg-agent"])
            .env("GNUPGHOME", f.root.home.path())
            .status()
            .unwrap()
            .success()
    );
    let status = f.chain.status_at(f.time()).unwrap();
    assert_eq!(status.state, IdentityState::Certified);
    assert!(status.bootstrapped && status.authenticated_signing_ready);
    assert_eq!(
        status.root_fingerprint.as_deref(),
        Some(f.root.bootstrap.fingerprint.as_str())
    );
    assert_eq!(status.custody_label.as_deref(), Some("test custody"));
    assert_eq!(status.seats_still_local_dev, 2);
    let due = status.device_renewal_due_at.unwrap();
    let expires = status.device_grant_expires_at.unwrap();
    assert_eq!(expires - due, 7 * 24 * 3600);
    assert!(!f.chain.status_at(due - 1).unwrap().device_renewal_required);
    assert!(f.chain.status_at(due).unwrap().device_renewal_required);
    assert_eq!(
        f.chain.status_at(expires).unwrap().pending_reason,
        Some(PendingReason::DeviceGrantExpired)
    );
    f.registry
        .update(f.seat.id, |r| {
            r.generation += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        f.chain.status_at(f.time()).unwrap().seats_still_local_dev,
        1
    );
    f.registry
        .update(f.peer.id, |r| {
            r.lifecycle = bbt_mux::registry::Lifecycle::ClosedRetained;
            Ok(())
        })
        .unwrap();
    assert_eq!(f.chain.still_local().unwrap(), 0);
    let head: Signed<AccountHead> =
        serde_json::from_slice(&f.account.get("identity/head").unwrap().unwrap()).unwrap();
    let id = head.payload.active_devices[0].device_id;
    let key = format!("identity/device/{id}");
    let mut device: bbt_mux::identity::DeviceIdentity =
        serde_json::from_slice(&f.account.get(&key).unwrap().unwrap()).unwrap();
    device.custody = Custody::LoginKeychain; // Public dev-record label; custody remains the memory test trait.
    f.account
        .put(&key, &serde_json::to_vec(&device).unwrap())
        .unwrap();
    assert_eq!(
        f.chain
            .status_at(f.time())
            .unwrap()
            .custody_label
            .as_deref(),
        Some("dev custody")
    );
    f.custody.0.lock().unwrap().clear();
    let held = f.chain.status_at(f.time()).unwrap();
    assert_eq!(held.state, IdentityState::Held);
    assert_eq!(
        held.pending_reason,
        Some(PendingReason::KeychainUnavailable(-25300))
    );
    assert!(!held.authenticated_signing_ready);
    let clock = f.time();
    let reopened = ChainSeatKeys::with_clock(
        f.account.clone(),
        f.custody.clone(),
        &f.root.bootstrap.fingerprint,
        SigningKey::from_bytes(&[42; 32]),
        Arc::new(move || clock),
    )
    .unwrap();
    assert_eq!(
        reopened.status_at(clock).unwrap().state,
        IdentityState::Held
    );
    let expired = ChainSeatKeys::with_clock(
        f.account.clone(),
        f.custody.clone(),
        &f.root.bootstrap.fingerprint,
        SigningKey::from_bytes(&[42; 32]),
        Arc::new(move || expires),
    )
    .unwrap();
    assert_eq!(
        expired.status_at(expires).unwrap().pending_reason,
        Some(PendingReason::DeviceGrantExpired)
    );
    struct Locked;
    impl DeviceKeyStore for Locked {
        fn custody(&self) -> Custody {
            Custody::Memory
        }
        fn load(&self, _: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
            Err(PendingReason::KeychainUnavailable(-25308))
        }
        fn create(&self, _: Ulid, _: &SigningKey) -> bbt_mux::identity::Result<()> {
            panic!("status must not generate a key")
        }
    }
    let locked = ChainSeatKeys::with_clock(
        f.account.clone(),
        Arc::new(Locked),
        &f.root.bootstrap.fingerprint,
        SigningKey::from_bytes(&[42; 32]),
        Arc::new(move || clock),
    )
    .unwrap();
    assert_eq!(
        locked.status_at(clock).unwrap().pending_reason,
        Some(PendingReason::KeychainUnavailable(-25308))
    );
    println!(
        "ID1c: week-before-expiry renewal, dev custody label, expired/missing-key holds and exact live local-generation count surfaced without root signing"
    );
}

#[test]
fn identity_hold_range_stays_ephemeral_until_throwaway_device_enrollment_then_closes() {
    use bbt_mux::{
        Config, Supervisor,
        identity::{
            ChainSeatKeys, HoldClosure, HoldWindow, LocalIssuerOutcome, local_dev_issuer_with_store,
        },
        logic::{Logic, LogicConfig},
        store::events::Draft,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    struct Custody {
        keys: SharedKeys,
        locked: AtomicBool,
    }
    impl DeviceKeyStore for Custody {
        fn custody(&self) -> bbt_mux::identity::Custody {
            bbt_mux::identity::Custody::Memory
        }
        fn load(&self, id: Ulid) -> bbt_mux::identity::Result<Option<SigningKey>> {
            if self.locked.load(Ordering::Relaxed) {
                return Err(PendingReason::KeychainUnavailable(-25308));
            }
            self.keys.load(id)
        }
        fn create(&self, id: Ulid, key: &SigningKey) -> bbt_mux::identity::Result<()> {
            self.keys.create(id, key)
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join(".bboxy");
    fs::create_dir(&repository).unwrap();
    let custody = Arc::new(Custody {
        keys: SharedKeys::default(),
        locked: AtomicBool::new(true),
    });
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let logic = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: directory.path().into(),
            repository,
            ring: Default::default(),
        },
        custody.clone(),
        runtime.handle(),
    )
    .unwrap();
    let held = runtime.block_on(logic.client.identity_status()).unwrap();
    assert_eq!(held.open_ephemeral_holds, 1);
    assert!(held.custody_held_since.is_some());
    let event = runtime
        .block_on(logic.client.bus.publish(
            "account",
            Draft::new(
                "account/hold-fixture/state",
                "fixture",
                serde_json::json!({"alive":true}),
            ),
        ))
        .unwrap();
    event.verify().unwrap();
    assert!(event.event.source.starts_with("ephemeral-hold/"));
    logic.shutdown().unwrap();
    let account = SqliteStore::open(&directory.path().join("account")).unwrap();
    let windows = account.list("identity/bus-signers/").unwrap();
    let (path, bytes) = &windows[0];
    let window: HoldWindow = serde_json::from_slice(bytes).unwrap();
    assert!(window.ranges.as_ref().is_some_and(|r| !r.is_empty()));
    assert!(account.list("identity/hold-closures/").unwrap().is_empty());
    custody.locked.store(false, Ordering::Relaxed);
    let local = match local_dev_issuer_with_store(directory.path(), custody.as_ref()) {
        LocalIssuerOutcome::Ready(key) => *key,
        other => panic!("{other:?}"),
    };
    // No real root: this fixture owns a disposable GNUPGHOME and its agent.
    let root = Fixture::new();
    let before = snapshot(root.home.path());
    let BootstrapOutcome::Certified(device) = root.bootstrap.run(&account, custody.as_ref(), now())
    else {
        panic!("throwaway enrollment")
    };
    let chain = ChainSeatKeys::open(
        account.clone(),
        custody.clone(),
        &root.bootstrap.fingerprint,
        local,
    )
    .unwrap();
    let mut corrupt = window.clone();
    corrupt.ranges.as_mut().unwrap()[0].last_hash = "00".repeat(32);
    account
        .put(path, &serde_json::to_vec(&corrupt).unwrap())
        .unwrap();
    assert!(
        chain
            .close_open_holds(directory.path())
            .unwrap_err()
            .to_string()
            .contains("hash mismatch")
    );
    assert!(account.list("identity/hold-closures/").unwrap().is_empty());
    // Simulate an abandoned process that did not seal its range. Recovery must
    // reconstruct it from signed stream bytes rather than trusting endpoints.
    let mut abandoned = window.clone();
    abandoned.ended_at = None;
    abandoned.ranges = None;
    account
        .put(path, &serde_json::to_vec(&abandoned).unwrap())
        .unwrap();
    assert_eq!(chain.close_open_holds(directory.path()).unwrap(), 1);
    let bytes = account
        .get(&format!("identity/hold-closures/{}", window.run))
        .unwrap()
        .unwrap();
    let closure: bbt_mux::identity::Signed<HoldClosure> = serde_json::from_slice(&bytes).unwrap();
    closure.verify_device(&device).unwrap();
    assert_eq!(closure.payload.window.public_key, window.public_key);
    assert_eq!(closure.payload.window.ranges, window.ranges);
    let mut forged = closure.clone();
    forged.payload.window.public_key = "00".repeat(32);
    assert!(forged.verify_device(&device).is_err());
    assert_eq!(
        chain.close_open_holds(directory.path()).unwrap(),
        0,
        "closure is idempotent"
    );
    custody.locked.store(true, Ordering::Relaxed);
    let mut recovered = bbt_mux::identity::IdentityRuntime::open_custodied_with_store(
        directory.path(),
        custody.clone(),
        runtime.handle().clone(),
    )
    .unwrap();
    assert_eq!(
        recovered.status().unwrap().state,
        bbt_mux::identity::IdentityState::Held
    );
    custody.locked.store(false, Ordering::Relaxed);
    recovered.retry_custody().unwrap();
    assert_eq!(
        recovered.status().unwrap().state,
        bbt_mux::identity::IdentityState::Certified
    );
    assert_eq!(recovered.status().unwrap().open_ephemeral_holds, 0);
    assert_eq!(
        recovered.keys().issuer().kind,
        bbt_mux::messaging::wire::IssuerKind::GpgRoot
    );
    assert_eq!(snapshot(root.home.path()), before);
    println!(
        "ID1d: no-device hold remains ephemeral; throwaway enrollment closes older abandoned range; exact hash range reconstructed/verified; device closure signature and tamper negative; fixture keyring unchanged"
    );
}
