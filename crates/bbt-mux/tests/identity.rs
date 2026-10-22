//! Disposable interoperability/custody checks. Every GnuPG command has an
//! explicit throwaway homedir; these tests never query the operator keyring.
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
    assert_eq!(keys.0.borrow().len(), 1);

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
    let id = *keys.0.borrow().keys().next().unwrap();
    fixture.bootstrap.agent_socket = agent_socket;
    match fixture.bootstrap.run(&account, &keys, now()) {
        BootstrapOutcome::Certified(identity) => assert_eq!(identity.grant.payload.device_id, id),
        pending => panic!("{pending:?}"),
    }
    assert_eq!(keys.0.borrow().len(), 1);
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
