// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Live plugin connections (`PluginHost`). A separate test binary: each
//! fixture execs several freshly copied plugins, and that burst would share
//! the conformance checker's fixed 2 s budgets with the library unit tests.
#![allow(
    clippy::disallowed_methods,
    reason = "owned private account and plugin fixtures on IO, never operator identity"
)]
use bbt_mux::{
    Ulid,
    knowledge::data_source as d,
    mcp::{Actor, Door},
    plugins::{DiscoveryRoots, PluginHost, ProviderConfig, Registry, RegistryConfig},
    store::sqlite::SqliteStore,
};
use bbt_plugin::contracts::Contract;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, Instant},
};

fn operator() -> Actor {
    Actor::Operator {
        device_id: Ulid::from(91_u128),
        door: Door::Ctl,
        issued_by: None,
    }
}

/// One fixture at a time keeps the exec burst, and its timing, small.
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct Fixture {
    _serial: tokio::sync::MutexGuard<'static, ()>,
    _temp: tempfile::TempDir,
    registry: Registry,
    host: PluginHost,
    candidate: String,
    delay: PathBuf,
}

impl Fixture {
    /// A checked, enabled repo_graph reference plugin on a private account.
    async fn new(concurrent: bool) -> Self {
        let serial = SERIAL.lock().await;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo/.bboxy");
        fs::create_dir_all(root.join("plugins")).unwrap();
        let delay = temp.path().join("delay");
        let source = include_str!("fixtures/plugins/bboxy450-term-data_source-reference")
            .replacen("SYSTEM = \"data_source\"", "SYSTEM = \"repo_graph\"", 1)
            .replacen(
                "CONCURRENT = False",
                if concurrent {
                    "CONCURRENT = True"
                } else {
                    "CONCURRENT = False"
                },
                1,
            )
            .replacen(
                "DELAY_FILE = None",
                &format!("DELAY_FILE = {:?}", delay.to_str().unwrap()),
                1,
            );
        let path = root.join("plugins/bboxy450-term-repo_graph-reference");
        fs::write(&path, source).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let account = SqliteStore::open(&temp.path().join("account")).unwrap();
        let registry = Registry::with_config(
            account,
            DiscoveryRoots::new(&temp.path().join("app"), &temp.path().join("repo")),
            RegistryConfig {
                descriptor_timeout: Duration::from_secs(30),
                conformance_timeout: Duration::from_secs(30),
            },
        )
        .unwrap();
        let entry = registry.refresh().unwrap().remove(0);
        let candidate = entry.candidate.id.clone();
        registry
            .trust(
                &operator(),
                &candidate,
                entry.candidate.sha256.as_deref().unwrap(),
                None,
            )
            .unwrap();
        let inspected = registry
            .inspection(&candidate)
            .unwrap()
            .unwrap()
            .run()
            .await;
        registry.finish_inspection(inspected).unwrap();
        let checked = registry
            .conformance(&operator(), &candidate)
            .unwrap()
            .unwrap()
            .run()
            .await;
        let checked = registry.finish_conformance(checked).unwrap();
        assert!(checked.conformance.as_ref().unwrap().ok, "{checked:?}");
        registry.set_enabled(&operator(), &candidate, true).unwrap();
        let host = PluginHost::new(
            registry.clone(),
            &root,
            tokio::runtime::Handle::current(),
            ProviderConfig {
                timeout: Duration::from_secs(30),
            },
        )
        .unwrap();
        Self {
            _serial: serial,
            _temp: temp,
            registry,
            host,
            candidate,
            delay,
        }
    }

    async fn call_at(&self, deadline: Instant) -> d::Result<Value> {
        let host = self.host.clone();
        let candidate = self.candidate.clone();
        tokio::task::spawn_blocking(move || {
            host.call(
                &candidate,
                Contract::RepoGraph,
                "changes_since",
                json!({"cursor": "0"}),
                deadline,
            )
        })
        .await
        .unwrap()
    }

    async fn changes(&self) -> d::Result<Value> {
        self.call_at(Instant::now() + Duration::from_secs(10)).await
    }

    fn enable(&self, enabled: bool) {
        self.registry
            .set_enabled(&operator(), &self.candidate, enabled)
            .unwrap();
    }

    fn generation(&self) -> Option<u64> {
        self.host
            .connected()
            .into_iter()
            .find(|(candidate, _)| *candidate == self.candidate)
            .map(|(_, generation)| generation)
    }

    async fn reconcile(&self) {
        let host = self.host.clone();
        tokio::task::spawn_blocking(move || host.reconcile())
            .await
            .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let host = self.host.clone();
        let _ = std::thread::spawn(move || host.close()).join();
    }
}

// keeps: PluginConnection — one live connection per (candidate, sha, generation);
// disable closes it and the next call is refused; re-enable opens a new generation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enabled_plugins_serve_calls_and_disable_closes_their_connection() {
    let f = Fixture::new(false).await;
    assert!(f.host.connected().is_empty(), "connections open lazily");
    let changes = f.changes().await.unwrap();
    assert_eq!(changes["next"], "1");
    let opened = f.generation().expect("the first call opened a connection");
    f.changes().await.unwrap();
    assert_eq!(f.generation(), Some(opened), "the connection is reused");

    f.enable(false);
    f.reconcile().await;
    assert!(
        f.host.connected().is_empty(),
        "disable closes the connection"
    );
    assert_eq!(f.changes().await.unwrap_err().code, "plugin_disabled");

    // A -> B -> A: enabled again, at a newer generation, without a reconcile.
    f.enable(true);
    f.changes().await.unwrap();
    let reopened = f.generation().unwrap();
    assert!(reopened > opened, "{reopened} <= {opened}");
    f.enable(false);
    f.enable(true);
    f.changes().await.unwrap();
    assert!(
        f.generation().unwrap() > reopened,
        "a call never reuses a superseded generation's connection"
    );
    assert_eq!(
        f.call_at(Instant::now()).await.unwrap_err().code,
        "deadline_exceeded",
        "an exhausted deadline never dispatches"
    );
}

// keeps: PluginConnection — a result that completes after the activation
// generation changed is refused, and the stale connection is closed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_result_from_a_superseded_generation_is_refused() {
    let f = Fixture::new(false).await;
    f.changes().await.unwrap();
    fs::write(&f.delay, b"").unwrap();
    let slow = {
        let host = f.host.clone();
        let candidate = f.candidate.clone();
        tokio::task::spawn_blocking(move || {
            host.call(
                &candidate,
                Contract::RepoGraph,
                "changes_since",
                json!({"cursor": "0"}),
                Instant::now() + Duration::from_secs(10),
            )
        })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;
    // concurrent:false keeps one request in flight and refuses, never queues.
    let busy = f.changes().await.unwrap_err();
    assert_eq!(busy.code, "provider_busy");
    assert_eq!(busy.retry_after_ms, Some(100));
    f.enable(false);
    f.enable(true);
    assert_eq!(slow.await.unwrap().unwrap_err().code, "provider_changed");
    assert!(f.host.connected().is_empty(), "the stale connection closed");
    fs::remove_file(&f.delay).unwrap();
    f.changes().await.unwrap();
}

// keeps: G3 concurrent — a descriptor declaring concurrent admits parallel calls.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_descriptors_admit_parallel_requests() {
    let f = Fixture::new(true).await;
    f.changes().await.unwrap();
    fs::write(&f.delay, b"").unwrap();
    let started = Instant::now();
    let (first, second) = tokio::join!(f.changes(), f.changes());
    first.unwrap();
    second.unwrap();
    // Each held call takes one second; serial service would need two.
    assert!(
        started.elapsed() < Duration::from_millis(1900),
        "{:?}",
        started.elapsed()
    );
    // A workspace drop closes the host; later calls are refused.
    let host = f.host.clone();
    tokio::task::spawn_blocking(move || host.close())
        .await
        .unwrap();
    assert_eq!(f.changes().await.unwrap_err().code, "provider_closed");
}
