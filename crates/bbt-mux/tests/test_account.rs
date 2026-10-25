#![allow(
    clippy::disallowed_methods,
    reason = "private generated identity fixtures; no native custody"
)]
use std::{os::unix::fs::PermissionsExt, path::Path};

use bbt_mux::{
    identity::{AccountHead, Custody, DeviceIdentity, IdentityRuntime, Signed, TestAccount},
    store::{Store, object_dir, sqlite::SqliteStore},
};

fn public(run: &Path) -> (Signed<AccountHead>, DeviceIdentity) {
    let account = SqliteStore::open(&object_dir(run, "account").unwrap()).unwrap();
    let head: Signed<AccountHead> =
        serde_json::from_slice(&account.get("identity/head").unwrap().unwrap()).unwrap();
    let id = head.payload.active_devices[0].device_id;
    let device = serde_json::from_slice(
        &account
            .get(&format!("identity/device/{id}"))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    (head, device)
}

#[test]
fn generated_root_device_and_verified_grants_survive_reopening() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("keys");
    let run = temp.path().join("run");
    let first = TestAccount::open(&root, &run).unwrap();
    let label = first.issued_by().to_string();
    assert!(label.starts_with("test-account:"));
    let (head, identity) = public(&run);
    identity
        .verify(&identity.root_fingerprint, &head, head.payload.issued_at)
        .unwrap();
    assert_ne!(identity.root_fingerprint, bbt_mux::identity::OPERATOR_ROOT);
    assert_eq!(identity.custody, Custody::File);
    let first_key = first
        .key_store()
        .load(identity.grant.payload.device_id)
        .unwrap()
        .unwrap();
    drop(first);
    let reopened = TestAccount::open(&root, &run).unwrap();
    assert_eq!(reopened.issued_by(), label);
    assert_eq!(
        serde_json::to_value(public(&run)).unwrap(),
        serde_json::to_value((&head, &identity)).unwrap()
    );
    let key = reopened
        .key_store()
        .load(identity.grant.payload.device_id)
        .unwrap()
        .unwrap();
    assert_eq!(key.verifying_key(), first_key.verifying_key());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let runtime =
        IdentityRuntime::open_custodied_with_store(&run, reopened.key_store(), rt.handle().clone())
            .unwrap();
    let status = runtime.status().unwrap();
    assert!(
        status.bootstrapped && status.authenticated_signing_ready,
        "{status:?}"
    );
    assert_eq!(status.device_id, Some(identity.grant.payload.device_id));
    assert_eq!(status.custody, Some(Custody::File));
    assert!(status.label.starts_with(&label));
    for entry in std::fs::read_dir(root).unwrap() {
        assert_eq!(
            entry.unwrap().metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn fresh_runs_are_distinct_and_cannot_rebind_an_existing_account() {
    let temp = tempfile::tempdir().unwrap();
    let one = TestAccount::open(&temp.path().join("one"), &temp.path().join("run1")).unwrap();
    let two = TestAccount::open(&temp.path().join("two"), &temp.path().join("run2")).unwrap();
    assert_ne!(one.issued_by(), two.issued_by());
    let (_, a) = public(&temp.path().join("run1"));
    let (_, b) = public(&temp.path().join("run2"));
    assert_ne!(a.root_fingerprint, b.root_fingerprint);
    assert_ne!(a.grant.payload.device_id, b.grant.payload.device_id);
    assert!(TestAccount::open(&temp.path().join("two"), &temp.path().join("run1")).is_err());
    assert!(TestAccount::open(&temp.path().join("one"), &temp.path().join("run2")).is_err());
}

#[test]
fn a_missing_or_corrupt_root_refuses_without_generating_a_new_identity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("keys");
    let run = temp.path().join("run");
    TestAccount::open(&root, &run).unwrap();
    let before = serde_json::to_value(public(&run)).unwrap();
    std::fs::remove_file(root.join("test_root")).unwrap();
    let error = TestAccount::open(&root, &run).err().unwrap();
    assert!(
        error.to_string().contains("test_account_root_missing"),
        "{error}"
    );
    assert!(!root.join("test_root").exists());
    assert_eq!(serde_json::to_value(public(&run)).unwrap(), before);
    std::fs::write(root.join("test_root"), b"corrupt").unwrap();
    std::fs::set_permissions(
        root.join("test_root"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let error = TestAccount::open(&root, &run).err().unwrap();
    assert!(
        error.to_string().contains("test_account_root_corrupt"),
        "{error}"
    );
    assert_eq!(serde_json::to_value(public(&run)).unwrap(), before);
}

#[test]
fn seat_messages_require_the_label_derived_by_the_receiving_account() {
    use bbt_mux::{
        messaging::{
            Messaging,
            wire::{EnvelopeExt, SendRequest},
        },
        registry::{Registry, SeatRecord},
    };
    let temp = tempfile::tempdir().unwrap();
    let run = temp.path().join("run");
    let generated = TestAccount::open(&temp.path().join("keys"), &run).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let runtime = IdentityRuntime::open_custodied_with_store(
        &run,
        generated.key_store(),
        rt.handle().clone(),
    )
    .unwrap();
    let keys = runtime.keys();
    let account = SqliteStore::open(&object_dir(&run, "account").unwrap()).unwrap();
    let registry = Registry::new(account);
    let swarm = bbt_mux::Ulid::new();
    let seat = SeatRecord::new(bbt_mux::Ulid::new(), swarm, "sender", "zsh");
    let peer = SeatRecord::new(bbt_mux::Ulid::new(), swarm, "receiver", "zsh");
    registry.insert(&seat).unwrap();
    registry.insert(&peer).unwrap();
    let mut messaging = Messaging::open(&run, keys.clone()).unwrap();
    let sent = messaging
        .send(
            seat.id,
            1,
            SendRequest::text(format!("seats/{}/inbox", peer.id), "labelled fixture"),
            bbt_mux::store::now_ms(),
        )
        .unwrap();
    assert_eq!(
        sent.envelope().expect("signed fixture envelope").issued_by.as_deref(),
        Some(generated.issued_by())
    );
    keys.verify(&sent.envelope().expect("signed fixture envelope")).unwrap();
    assert!(sent.envelope().expect("signed fixture envelope").verify_signature().is_err()); // ordinary wire rules remain strict
    for bad_label in [None, Some("test-account:01ARZ3NDEKTSV4RRFFQ69G5FAV")] {
        let mut forged = sent.envelope().expect("signed fixture envelope").clone();
        forged.issued_by = bad_label.map(str::to_owned);
        // The fixture owns this leaf. A valid signature cannot invent account context.
        let leaf = keys.leaf(seat.id, 1).unwrap();
        forged.sign_in_account(&leaf, bad_label).unwrap();
        let error = keys.verify(&forged).unwrap_err().to_string();
        assert!(error.contains("test_account_label_mismatch"), "{error}");
    }
}

#[test]
fn a_foreign_nonempty_directory_never_becomes_a_test_identity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("foreign");
    let store = bboxy450_keychain::open(bboxy450_keychain::Backend::File {
        directory: root.clone(),
    })
    .unwrap();
    store
        .put(
            "existing_material",
            b"owned by a different fixture",
            bboxy450_keychain::Acl::OwnerOnly,
        )
        .unwrap();
    let error = TestAccount::open(&root, &temp.path().join("run"))
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains("test_account_directory_not_empty"),
        "{error}"
    );
    assert!(!root.join("test_root").exists());
    assert_eq!(
        store.get("existing_material").unwrap().unwrap().as_bytes(),
        b"owned by a different fixture"
    );
}

#[test]
fn startup_allows_the_selected_private_run_inside_the_account_directory() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("test-account");
    let run = root.join("run");
    // Session prepares its private run before starting logic.
    bboxy450_keychain::open(bboxy450_keychain::Backend::File {
        directory: root.clone(),
    })
    .unwrap();
    bboxy450_keychain::open(bboxy450_keychain::Backend::File {
        directory: run.clone(),
    })
    .unwrap();
    let first = TestAccount::open(&root, &run).unwrap();
    let before = serde_json::to_value(public(&run)).unwrap();
    let reopened = TestAccount::open(&root, &run).unwrap();
    assert_eq!(first.issued_by(), reopened.issued_by());
    assert_eq!(serde_json::to_value(public(&run)).unwrap(), before);
    assert!(root.join("test_root").is_file());
}

#[test]
fn startup_refuses_a_foreign_issuer_before_creating_test_keys_or_context() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("keys");
    let run = temp.path().join("run");
    for directory in [run.clone(), run.join("account")] {
        bboxy450_keychain::open(bboxy450_keychain::Backend::File { directory }).unwrap();
    }
    let account = SqliteStore::open(&object_dir(&run, "account").unwrap()).unwrap();
    account
        .put("identity/issuer", b"existing account issuer")
        .unwrap();
    let error = TestAccount::open(&root, &run)
        .err()
        .expect("foreign issuer refused");
    assert!(error.to_string().contains("TestAccountMismatch"), "{error}");
    assert!(!root.join("test_root").exists());
    assert!(account.get("identity/test-account").unwrap().is_none());
    assert_eq!(
        account.get("identity/issuer").unwrap().unwrap(),
        b"existing account issuer"
    );
}
