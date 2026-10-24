#![cfg(unix)]
#![allow(clippy::disallowed_methods, reason = "isolated file custody fixtures")]
use std::{os::unix::fs::PermissionsExt, sync::Arc};

use bboxy450_keychain_file::{Acl, Error, MAX_SECRET_BYTES, Store};

#[test]
fn private_slots_reopen_and_only_explicit_removal_changes_them() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("custody");
    let first = Store::open(&root).unwrap();
    first
        .put("root", b"a generated fixture key", Acl::OwnerOnly)
        .unwrap();
    let reopened = Store::open(&root).unwrap();
    assert_eq!(
        reopened.get("root").unwrap().unwrap().as_bytes(),
        b"a generated fixture key"
    );
    assert_eq!(reopened.acl("root").unwrap(), Some(Acl::OwnerOnly));
    assert_eq!(
        reopened.put("root", b"replacement", Acl::OwnerOnly),
        Err(Error::Conflict)
    );
    assert_eq!(
        std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(root.join("root"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        format!("{:?}", reopened.get("root").unwrap().unwrap()),
        "Secret(<redacted>)"
    );
    reopened.remove("root").unwrap();
    assert!(first.get("root").unwrap().is_none());
    first.remove("root").unwrap();
}

#[test]
fn relative_paths_symlinks_non_regular_files_and_broad_modes_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("custody");
    let store = Store::open(&root).unwrap();
    assert!(matches!(
        Store::open(std::path::Path::new("relative")),
        Err(Error::UnsafePath)
    ));
    for label in ["", "../other", "a/b", ".", "..", "has.dot", "root\0bad"] {
        assert!(matches!(store.get(label), Err(Error::InvalidLabel)));
    }
    store.put("secret", b"fixture", Acl::OwnerOnly).unwrap();
    std::fs::set_permissions(root.join("secret"), std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(store.get("secret"), Err(Error::Permissions)));
    std::os::unix::fs::symlink(root.join("secret"), root.join("link")).unwrap();
    assert!(matches!(store.get("link"), Err(Error::UnsafePath)));
    std::fs::create_dir(root.join("directory")).unwrap();
    assert!(matches!(store.get("directory"), Err(Error::UnsafePath)));
    std::os::unix::fs::symlink(&root, temp.path().join("alias")).unwrap();
    assert!(matches!(
        Store::open(&temp.path().join("alias")),
        Err(Error::UnsafePath)
    ));
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(Store::open(&root), Err(Error::Permissions)));
    assert!(matches!(store.get("missing"), Err(Error::Permissions)));
}

#[test]
fn writes_and_reads_have_the_same_fixed_size_bound() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("custody");
    let store = Store::open(&root).unwrap();
    store
        .put("largest", &vec![9; MAX_SECRET_BYTES], Acl::OwnerOnly)
        .unwrap();
    assert_eq!(
        store.get("largest").unwrap().unwrap().as_bytes().len(),
        MAX_SECRET_BYTES
    );
    assert_eq!(
        store.put("overflow", &vec![9; MAX_SECRET_BYTES + 1], Acl::OwnerOnly),
        Err(Error::TooLarge)
    );
    assert_eq!(
        store.put("empty", &[], Acl::OwnerOnly),
        Err(Error::EmptySecret)
    );
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(root.join("largest"))
        .unwrap();
    file.set_len(MAX_SECRET_BYTES as u64 + 1).unwrap();
    assert!(matches!(store.get("largest"), Err(Error::TooLarge)));
}

#[test]
fn independent_store_handles_never_replace_a_concurrent_winner() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("custody");
    let stores: Vec<_> = (0..8).map(|_| Store::open(&root).unwrap()).collect();
    let barrier = Arc::new(std::sync::Barrier::new(stores.len()));
    let jobs: Vec<_> = stores
        .into_iter()
        .enumerate()
        .map(|(index, store)| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                (
                    index as u8,
                    store.put("root", &[index as u8], Acl::OwnerOnly),
                )
            })
        })
        .collect();
    let results: Vec<_> = jobs.into_iter().map(|job| job.join().unwrap()).collect();
    let winners: Vec<_> = results
        .iter()
        .filter(|(_, result)| result.is_ok())
        .collect();
    assert_eq!(winners.len(), 1);
    for (_, result) in &results {
        assert!(result.is_ok() || *result == Err(Error::Conflict));
    }
    let store = Store::open(&root).unwrap();
    assert_eq!(
        store.get("root").unwrap().unwrap().as_bytes(),
        &[winners[0].0]
    );
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 1);
}
