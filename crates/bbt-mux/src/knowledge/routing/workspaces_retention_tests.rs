// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use crate::{
    plugins::{DiscoveryRoots, ProviderConfig, Registry as PluginRegistry},
    registry::{Lifecycle, SeatRecord},
    store::{object_dir, sqlite::SqliteStore},
};

struct Fixture {
    router: Router,
    registry: Registry,
    _runtime: tokio::runtime::Runtime,
    dir: tempfile::TempDir,
    disable_close_sweep: bool,
}

impl Fixture {
    fn open(root_limit: usize, swarm_limit: usize) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repository = dir.path().join("startup");
        let run_dir = dir.path().join("run");
        let store = SqliteStore::open(&object_dir(&run_dir, "account").unwrap()).unwrap();
        let registry = Registry::new(store.clone());
        registry.prepare_swarm_index().unwrap();
        let plugins = PluginRegistry::new(
            store,
            DiscoveryRoots::new(&dir.path().join("support"), &repository),
        )
        .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cache = Workspaces::open(Config {
            startup_repository: repository,
            run_dir,
            plugins,
            runtime: runtime.handle().clone(),
            provider: ProviderConfig::default(),
            activity: None,
            issued_by: None,
            messages: None,
        })
        .unwrap();
        let mut router = Router::new(cache, runtime.handle().clone());
        router.root_limit = root_limit;
        router.swarm_limit = swarm_limit;
        Self {
            router,
            registry,
            _runtime: runtime,
            dir,
            disable_close_sweep: false,
        }
    }

    fn workspace(&self, swarm: Ulid) -> PathBuf {
        let path = self.dir.path().join(swarm.to_string());
        std::fs::create_dir(&path).unwrap();
        self.registry
            .update_swarm_workdir(swarm, path.to_str())
            .unwrap();
        path.canonicalize().unwrap()
    }

    fn seat(&self, swarm: Ulid) -> SeatRecord {
        let seat = SeatRecord::new(Ulid::new(), swarm, "fixture", "zsh");
        self.registry.insert(&seat).unwrap();
        seat
    }

    fn close_seat(&self, seat: &SeatRecord) {
        self.registry
            .update(seat.id, |record| {
                record.lifecycle = Lifecycle::ClosedRetained;
                Ok(())
            })
            .unwrap();
    }

    fn poll(&mut self) {
        if self.disable_close_sweep {
            // The growing control suppresses the close sweep on every poll.
            self.router.registry_epoch = Some(self.registry.routing_epoch());
        }
        self.router.poll(&self.registry);
    }

    fn ready(&mut self, swarm: Ulid) -> Port {
        let deadline = Instant::now() + Duration::from_secs(35);
        loop {
            self.poll();
            match self.router.select(&self.registry, swarm, None) {
                Ok(port) => return port,
                Err(e) if e.downcast_ref::<Error>() == Some(&Error::Pending(swarm)) => (),
                Err(e) => panic!("{e}"),
            }
            assert!(Instant::now() < deadline, "workspace {swarm} never mounted");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn assert_slots(&self, roots: usize, swarms: usize) {
        assert_eq!(self.router.cache.roots().count(), roots);
        assert_eq!(self.router.attempts.len(), swarms);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.router.cache.retire();
    }
}

#[test]
fn workspace_churn_reaps_closed_roots_and_bindings_with_growing_control() {
    const SWARMS: u128 = 32;
    const TEST_ROOT_LIMIT: usize = 8;
    let mut bounded = Fixture::open(TEST_ROOT_LIMIT, SWARM_LIMIT);
    let mut unbounded = Fixture::open(usize::MAX / 2, usize::MAX / 2);
    unbounded.disable_close_sweep = true;

    for id in 1..=SWARMS {
        let swarm = Ulid(id);
        for f in [&mut bounded, &mut unbounded] {
            f.workspace(swarm);
            let seat = f.seat(swarm);
            let captured = f.ready(swarm).binding;
            f.close_seat(&seat);
            f.poll();
            if !f.disable_close_sweep {
                assert!(f.router.current(&f.registry, &captured).is_err());
                assert_eq!(
                    f.router.cache.current(swarm),
                    Err(Error::UnknownSwarm(swarm))
                );
            }
        }
        bounded.assert_slots(1, 0);
        unbounded.assert_slots(1 + id as usize, id as usize);
    }
    assert!(unbounded.router.cache.roots().count() > TEST_ROOT_LIMIT);
}

#[test]
fn workspace_pressure_evicts_the_least_recent_idle_root_and_caps_shared_bindings() {
    let mut f = Fixture::open(4, 4);
    let [live, recent, idle, incoming] = [Ulid(1), Ulid(2), Ulid(3), Ulid(4)];
    let idle_seat = f.seat(live);
    for swarm in [live, recent, idle, incoming] {
        f.workspace(swarm);
    }
    let active = f.ready(live).binding;
    let touched = f.ready(recent).binding;
    let evicted = f.ready(idle).binding;
    assert_eq!(f.ready(recent).binding, touched);
    let added = f.ready(incoming).binding;
    f.assert_slots(4, 3);
    assert!(f.router.cache.root(&evicted.root_id).is_none());
    for binding in [&active, &touched, &added] {
        f.router.current(&f.registry, binding).unwrap();
    }

    for id in 5..20 {
        let swarm = Ulid(id);
        let port = f.ready(swarm);
        assert_eq!(port.binding.root_id, f.router.cache.startup().root_id());
        assert_eq!(f.router.attempts.len(), f.router.swarm_limit);
        assert!(f.router.cache.current(live).is_ok());
    }
    f.close_seat(&idle_seat);
    f.router.poll(&f.registry);
    assert!(f.router.cache.root(&active.root_id).is_none());
}

#[test]
fn workspace_release_fences_old_preparation_and_waits_for_borrowed_mount() {
    let mut f = Fixture::open(2, 4);
    let swarm = Ulid(1);
    let incoming = Ulid(2);
    f.workspace(incoming);
    let repository = f.workspace(swarm);
    let port = f.ready(swarm);
    let captured = port.binding.clone();
    let old_attempt = f.router.attempts[&swarm].desired.clone();

    f.router.release_workspace(&repository);
    f.router.poll(&f.registry);
    assert!(f.router.current(&f.registry, &captured).is_err());
    assert!(f.router.cache.root(&captured.root_id).is_some());
    let (enter, entered) = std::sync::mpsc::channel();
    let (release, held) = std::sync::mpsc::channel::<()>();
    f.router
        .cache
        .root_mut(&captured.root_id)
        .unwrap()
        .watch
        .on_drop(move || {
            enter.send(()).unwrap();
            let _ = held.recv();
        });
    drop(port);
    f.router.poll(&f.registry);
    assert!(f.router.cache.root(&captured.root_id).is_none());
    entered.recv_timeout(Duration::from_secs(5)).unwrap();

    assert!(f.router.select(&f.registry, incoming, None).is_err());
    let deadline = Instant::now() + Duration::from_secs(5);
    while f.router.attempts[&incoming].failed.is_none() {
        f.router.poll(&f.registry);
        assert!(
            Instant::now() < deadline,
            "incoming mount did not reach capacity check"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(f.router.mounting.is_empty());
    assert!(f.router.retiring.contains(&captured.root_id));
    drop(release);

    // Reopening the same path and metadata revision is a fresh preparation.
    assert_eq!(f.ready(swarm).binding, captured);
    assert!(!f.router.wanted(&f.registry, swarm, &old_attempt));
}

#[test]
fn workspace_shared_mount_survives_until_the_last_swarm_closes() {
    let mut f = Fixture::open(3, 4);
    let [first, second] = [Ulid(1), Ulid(2)];
    let repository = f.workspace(first);
    f.registry
        .update_swarm_workdir(second, repository.to_str())
        .unwrap();
    let first_seat = f.seat(first);
    let second_seat = f.seat(second);
    let a = f.ready(first).binding;
    let b = f.ready(second).binding;
    assert_eq!(a.root_id, b.root_id);

    f.close_seat(&first_seat);
    f.router.poll(&f.registry);
    assert!(f.router.current(&f.registry, &a).is_err());
    f.router.current(&f.registry, &b).unwrap();
    f.assert_slots(2, 1);

    f.close_seat(&second_seat);
    f.router.poll(&f.registry);
    f.assert_slots(1, 0);
}

#[test]
fn workspace_archive_reaps_a_seatless_swarm_until_it_reopens() {
    let mut f = Fixture::open(3, 4);
    let swarm = Ulid(1);
    f.workspace(swarm);
    let binding = f.ready(swarm).binding;
    let archived = f.registry.archive_empty_swarm(swarm).unwrap();
    f.poll();
    f.assert_slots(1, 0);
    assert!(f.router.current(&f.registry, &binding).is_err());
    assert!(
        !f.router
            .matches_root(&f.registry, swarm, &binding.root_id)
            .unwrap()
    );
    assert_eq!(
        f.router
            .select(&f.registry, swarm, None)
            .err()
            .unwrap()
            .downcast_ref::<Error>(),
        Some(&Error::UnknownSwarm(swarm))
    );

    f.registry.rename_swarm(swarm, "reopened").unwrap();
    let reopened = f.ready(swarm).binding;
    assert_eq!(reopened.root_id, binding.root_id);
    assert!(reopened.revision > archived.revision);
}
