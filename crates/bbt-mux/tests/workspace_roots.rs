// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "private temporary repositories and an owned IO runtime"
)]
use bbt_mux::{
    Ulid,
    knowledge::{
        data_source as d,
        workspaces::{Config, Error, RootBinding, Workspaces},
    },
    plugins::{DiscoveryRoots, ProviderConfig, Registry as PluginRegistry},
    registry::Registry,
    store::{object_dir, sqlite::SqliteStore},
};
use ed25519_dalek::SigningKey;
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

struct Fixture {
    _directory: tempfile::TempDir,
    _runtime: tokio::runtime::Runtime,
    root: PathBuf,
    first: PathBuf,
    second: PathBuf,
    registry: Registry,
    config: Config,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let first = root.join("first");
        let second = root.join("second");
        seed(&first, "first project");
        seed(&second, "second project");
        let run = root.join("run");
        let account = SqliteStore::open(&object_dir(&run, "account").unwrap()).unwrap();
        let registry = Registry::new(account.clone());
        let plugins =
            PluginRegistry::new(account, DiscoveryRoots::new(&root.join("support"), &first))
                .unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let config = Config {
            startup_repository: first.clone(),
            run_dir: run,
            plugins,
            runtime: runtime.handle().clone(),
            provider: ProviderConfig::default(),
            activity: None,
            issued_by: None,
            messages: None,
        };
        Self {
            _directory: directory,
            _runtime: runtime,
            root,
            first,
            second,
            registry,
            config,
        }
    }
    fn open(&self) -> Workspaces {
        Workspaces::open(self.config.clone()).unwrap()
    }
    fn read(&self, roots: &Workspaces, binding: &RootBinding, locator: &str) -> d::Record {
        roots
            .get(binding)
            .unwrap()
            .with_source(&self.registry, |source| {
                let reference = source.resolve(d::ResolveRequest {
                    locator: locator.into(),
                })?;
                source.fetch(d::FetchRequest { reference })
            })
            .unwrap()
    }
}
fn seed(repository: &Path, body: &str) {
    fs::create_dir_all(repository.join(".bboxy/notes")).unwrap();
    fs::write(
        repository.join(".bboxy/notes/same.md"),
        format!(
            "---\nid: \"{}\"\nnumber: 1\ntype: note\ntitle: Same slug\n---\n{body}\n",
            Ulid(1)
        ),
    )
    .unwrap();
}

#[test]
fn colliding_handles_and_slugs_read_and_write_only_the_bound_root() {
    let fixture = Fixture::new();
    let mut roots = fixture.open();
    let first = roots.bind(Ulid(10), 1, Some(&fixture.first)).unwrap();
    let second = roots.bind(Ulid(20), 1, Some(&fixture.second)).unwrap();
    assert_ne!(first.root_id, second.root_id);
    for locator in ["#1", "same", "notes/same.md"] {
        assert_eq!(
            fixture.read(&roots, &first, locator).body,
            "first project\n"
        );
        assert_eq!(
            fixture.read(&roots, &second, locator).body,
            "second project\n"
        );
    }
    let key = SigningKey::from_bytes(&[37; 32]);
    Arc::make_mut(&mut roots.get_mut(&first).unwrap().index)
        .write(
            "notes/created.md",
            json!({"id":Ulid(2).to_string(),"type":"note","title":"Created in first"}),
            "new first content",
            Ulid(90),
            &key,
        )
        .unwrap();
    roots.scan(&first).unwrap();
    assert!(fixture.first.join(".bboxy/notes/created.md").is_file());
    assert!(!fixture.second.join(".bboxy/notes/created.md").exists());
    assert_eq!(fixture.read(&roots, &second, "#1").body, "second project\n");

    // Scanning A never replaces B's captured catalogue or searches every root.
    let second_snapshot = roots.get(&second).unwrap().index.clone();
    seed(&fixture.second, "second project updated");
    roots.scan(&first).unwrap();
    assert!(Arc::ptr_eq(
        &second_snapshot,
        &roots.get(&second).unwrap().index
    ));
    roots.scan(&second).unwrap();
    assert!(!Arc::ptr_eq(
        &second_snapshot,
        &roots.get(&second).unwrap().index
    ));
    assert_eq!(
        fixture.read(&roots, &second, "#1").body,
        "second project updated\n"
    );
    assert!(fixture.config.run_dir.join("index/object.sqlite").is_file());
    assert!(
        fixture
            .config
            .run_dir
            .join(format!("index-{}/object.sqlite", second.root_id))
            .is_file()
    );
    roots.close();
}

#[test]
fn canonical_aliases_share_one_mount_and_none_reuses_startup_identity() {
    let fixture = Fixture::new();
    let mut roots = fixture.open();
    let startup = roots.startup().index.clone();
    let first = roots.bind(Ulid(10), 1, None).unwrap();
    let alias = fixture.root.join("alias");
    std::os::unix::fs::symlink(&fixture.first, &alias).unwrap();
    let prepared = Workspaces::prepare(&alias).unwrap();
    let same = roots.bind_cached(Ulid(20), 7, &prepared).unwrap().unwrap();
    assert_eq!(same.root_id, first.root_id);
    assert!(Arc::ptr_eq(&startup, &roots.get(&same).unwrap().index));
    assert_eq!(roots.roots().count(), 1);
    roots.bind(Ulid(10), 2, Some(&fixture.second)).unwrap();
    let reset = roots.bind(Ulid(10), 3, None).unwrap();
    assert_eq!(reset.root_id, first.root_id);
    assert!(Arc::ptr_eq(&startup, &roots.get(&reset).unwrap().index));
    assert!(Arc::ptr_eq(
        &roots.get(&reset).unwrap().index,
        &roots.get(&same).unwrap().index
    ));
    roots.close();
}

#[test]
fn captured_revision_and_root_are_fenced_and_unavailable_cwd_never_falls_back() {
    let fixture = Fixture::new();
    let mut roots = fixture.open();
    let first = roots.bind(Ulid(10), 1, None).unwrap();
    let second = roots.bind(Ulid(10), 2, Some(&fixture.second)).unwrap();
    assert!(matches!(
        roots.bind(Ulid(10), 1, None),
        Err(Error::Changed(_))
    ));
    assert_eq!(roots.current(Ulid(10)).unwrap(), &second);
    assert!(matches!(roots.get(&first), Err(Error::Changed(_))));
    let forged = RootBinding {
        root_id: first.root_id,
        ..second.clone()
    };
    assert!(matches!(roots.get(&forged), Err(Error::Changed(_))));
    assert!(matches!(
        roots.bind(Ulid(10), 3, Some(&fixture.root.join("missing"))),
        Err(Error::Unavailable { .. })
    ));
    assert!(matches!(
        roots.current(Ulid(10)),
        Err(Error::Unavailable { .. })
    ));
    assert!(matches!(roots.get(&second), Err(Error::Changed(_))));
    let reset = roots.bind(Ulid(10), 4, None).unwrap();
    assert_eq!(fixture.read(&roots, &reset, "#1").body, "first project\n");
    roots.close();
}

#[test]
fn io_mount_completion_is_targeted_and_stale_preparation_cannot_replace_newer_binding() {
    let fixture = Fixture::new();
    let mut roots = fixture.open();
    let second = roots.bind(Ulid(20), 1, Some(&fixture.second)).unwrap();
    let second_snapshot = roots.get(&second).unwrap().index.clone();
    let third = fixture.root.join("third");
    seed(&third, "third project");
    let prepared = Workspaces::prepare(&third).unwrap();
    assert!(roots.bind_cached(Ulid(10), 1, &prepared).unwrap().is_none());
    let job = roots.mount(prepared);
    let pending_mount = fixture._runtime.spawn_blocking(move || job.run());
    // The IO job owns its ports, leaving unrelated cache reads available.
    assert_eq!(fixture.read(&roots, &second, "#1").body, "second project\n");
    let mounted = fixture._runtime.block_on(pending_mount).unwrap().unwrap();
    let installed = roots.insert(Ulid(10), 1, mounted);
    let first = installed.binding.unwrap();
    assert!(installed.retired.is_none());
    assert_eq!(fixture.read(&roots, &first, "#1").body, "third project\n");
    assert!(Arc::ptr_eq(
        &second_snapshot,
        &roots.get(&second).unwrap().index
    ));

    let stale_path = fixture.root.join("stale");
    seed(&stale_path, "stale preparation");
    let pending = roots
        .mount(Workspaces::prepare(&stale_path).unwrap())
        .run()
        .unwrap();
    let latest = roots.bind(Ulid(10), 2, None).unwrap();
    let count = roots.roots().count();
    let stale = roots.insert(Ulid(10), 1, pending);
    assert!(matches!(stale.binding, Err(Error::Changed(_))));
    stale.retired.unwrap().close();
    assert_eq!(roots.current(Ulid(10)).unwrap(), &latest);
    assert_eq!(roots.roots().count(), count);
    assert!(Arc::ptr_eq(
        &second_snapshot,
        &roots.get(&second).unwrap().index
    ));
    let failed = Workspaces::prepare(&fixture.root.join("missing"))
        .err()
        .unwrap();
    roots.fail_binding(Ulid(10), 3, failed).unwrap();
    assert!(matches!(
        roots.current(Ulid(10)),
        Err(Error::Unavailable { .. })
    ));
    assert!(matches!(roots.get(&latest), Err(Error::Changed(_))));
    let vanished = fixture.root.join("vanished");
    seed(&vanished, "removed before mount");
    let prepared = Workspaces::prepare(&vanished).unwrap();
    fs::remove_dir_all(&vanished).unwrap();
    assert!(matches!(
        roots.mount(prepared).run(),
        Err(Error::Unavailable { .. })
    ));
    assert!(
        !vanished.exists(),
        "late preparation recreated a removed working directory"
    );
    roots.close();
}
