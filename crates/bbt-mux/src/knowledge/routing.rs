// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Owner-side root routing. No filesystem access or waiting in admission.
use super::{roots::Host, workspaces::*};
use crate::{
    Ulid,
    index::{FileWatch, Index, ScanBatch},
    registry::Registry,
    store,
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{runtime::Handle, task::JoinSet};

// Includes the pinned startup root and each mount or retirement that owns a slot.
const ROOT_LIMIT: usize = 64;
// A swarm sharing a root still owns its own binding and preparation.
const SWARM_LIMIT: usize = 1024;

#[derive(Clone)]
pub(crate) struct Port {
    pub binding: RootBinding,
    pub index: Arc<Index>,
    pub host: Host,
}
#[derive(Clone, PartialEq, Eq)]
struct Desired {
    revision: u64,
    path: Option<PathBuf>,
    archived: bool,
}
enum Completion {
    Prepared(Ulid, Arc<Desired>, Result<PreparedRoot>),
    Mounted(String, PreparedRoot, Box<Result<MountedRoot>>),
    Closed,
    Retired(String),
}
struct ScanCompletion {
    root: String,
    generation: u64,
    watch: Option<FileWatch>,
    base: Arc<Index>,
    index: Arc<Index>,
    result: Result<()>,
}
#[cfg(test)]
pub(crate) type ScanHook = Arc<dyn Fn(&str) + Send + Sync>;
#[derive(Default)]
struct ScanState {
    running: bool,
    queued: bool,
    force: bool,
    /// A read waits only for the watcher's undebounced changes, never a full scan.
    read: bool,
    generation: u64,
    failure: Option<(Instant, Error)>,
}
struct Attempt {
    desired: Arc<Desired>,
    open: bool,
    last_selected: Instant,
    failed: Option<Instant>,
}
/// Canonical roots are deduplicated after independent IO preparation; an
/// unavailable directory cannot delay another project's mount or any chat.
pub(crate) struct Router {
    pub cache: Workspaces,
    jobs: JoinSet<Completion>,
    attempts: BTreeMap<Ulid, Attempt>,
    mounting: BTreeMap<String, BTreeMap<Ulid, Arc<Desired>>>,
    retiring: BTreeSet<String>,
    registry_epoch: Option<u64>,
    root_limit: usize,
    swarm_limit: usize,
    runtime: Handle,
    scans: BTreeMap<String, ScanState>,
    scan_jobs: JoinSet<ScanCompletion>,
    scan_queue: VecDeque<String>,
    ready_scans: BTreeSet<String>,
    ready_cursor: Option<String>,
    #[cfg(test)]
    pub scan_hook: Option<ScanHook>,
    #[cfg(test)]
    pub scan_complete_hook: Option<ScanHook>,
    #[cfg(test)]
    pub fail_next_prepare: bool,
}
fn io_result<T>(repository: &std::path::Path, job: impl FnOnce() -> Result<T>) -> Result<T> {
    bbt_rt::qos::with_utility_qos(|| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)).unwrap_or_else(|_| {
            Err(Error::Unavailable {
                repository: repository.to_owned(),
                reason: "workspace IO worker panicked; retry the root".into(),
            })
        })
    })
}

#[test]
#[cfg(target_os = "macos")]
fn workspace_io_qos_restores_its_owner_after_success_and_caught_panic() {
    std::thread::spawn(|| {
        bbt_rt::qos::initiated_thread();
        let repository = std::path::Path::new("private-qos-witness");
        io_result(repository, || {
            assert_eq!(bbt_rt::qos::requested_class(), 0x11);
            Ok(())
        })
        .unwrap();
        assert_eq!(bbt_rt::qos::requested_class(), 0x19);
        let result: Result<()> = io_result(repository, || {
            assert_eq!(bbt_rt::qos::requested_class(), 0x11);
            panic!("private workspace IO panic");
        });
        assert!(result.is_err());
        assert_eq!(bbt_rt::qos::requested_class(), 0x19);
    })
    .join()
    .unwrap();
}
impl Router {
    pub fn new(cache: Workspaces, runtime: Handle) -> Self {
        Self {
            cache,
            jobs: JoinSet::new(),
            attempts: BTreeMap::new(),
            mounting: BTreeMap::new(),
            retiring: BTreeSet::new(),
            registry_epoch: None,
            root_limit: ROOT_LIMIT,
            swarm_limit: SWARM_LIMIT,
            runtime,
            scans: BTreeMap::new(),
            scan_jobs: JoinSet::new(),
            scan_queue: VecDeque::new(),
            ready_scans: Default::default(),
            ready_cursor: None,
            #[cfg(test)]
            scan_hook: None,
            #[cfg(test)]
            scan_complete_hook: None,
            #[cfg(test)]
            fail_next_prepare: false,
        }
    }
    fn desired(registry: &Registry, swarm: Ulid) -> store::Result<Desired> {
        Ok(match registry.swarm(swarm)? {
            Some(record) => Desired {
                revision: record.revision,
                path: record.workdir.map(PathBuf::from),
                archived: record.archived,
            },
            None => Desired {
                revision: 0,
                path: None,
                archived: false,
            },
        })
    }
    pub fn select(
        &mut self,
        registry: &Registry,
        swarm: Ulid,
        expected: Option<u64>,
    ) -> store::Result<Port> {
        let desired = Self::desired(registry, swarm)?;
        if expected.is_some_and(|revision| revision != desired.revision) {
            return Err(Box::new(Error::Changed(swarm)));
        }
        if desired.archived {
            return Err(Box::new(Error::UnknownSwarm(swarm)));
        }
        if let Some(attempt) = self.attempts.get_mut(&swarm)
            && *attempt.desired == desired
        {
            attempt.last_selected = Instant::now();
            match self.cache.current(swarm) {
                Ok(binding) if binding.revision == desired.revision => {
                    let binding = binding.clone();
                    self.cache.touch(&binding.root_id);
                    return self.port(binding);
                }
                Err(error)
                    if attempt
                        .failed
                        .is_none_or(|at| at.elapsed() < Duration::from_secs(1)) =>
                {
                    return Err(Box::new(error));
                }
                _ => (),
            }
        }
        if desired.path.is_some() && self.jobs.len() >= self.swarm_limit + self.root_limit {
            self.forget(swarm);
            return Err(Box::new(Error::Pending(swarm)));
        }
        let open = match self.attempts.get(&swarm) {
            Some(attempt) => attempt.open,
            None => {
                let open = registry.has_open_swarm_seats(swarm)?;
                if self.attempts.len() == self.swarm_limit {
                    let Some(idle) = self
                        .attempts
                        .iter()
                        .filter(|(id, attempt)| !attempt.open && self.binding_idle(**id))
                        .min_by_key(|(_, attempt)| attempt.last_selected)
                        .map(|(id, _)| *id)
                    else {
                        return Err(Box::new(Error::Pending(swarm)));
                    };
                    self.forget(idle);
                }
                open
            }
        };
        self.remove_waiter(swarm);
        let desired = Arc::new(desired);
        let attempt = Attempt {
            desired: desired.clone(),
            open,
            last_selected: Instant::now(),
            failed: None,
        };
        if desired.path.is_none() {
            let binding = self.cache.bind_startup(swarm, desired.revision)?;
            self.attempts.insert(swarm, attempt);
            return self.port(binding);
        }
        // Fence old snapshots immediately, before canonicalization or mount.
        self.cache
            .fail_binding(swarm, desired.revision, Error::Pending(swarm))?;
        self.attempts.insert(swarm, attempt);
        let prepared = desired;
        #[cfg(test)]
        let fail = std::mem::take(&mut self.fail_next_prepare);
        self.jobs.spawn_blocking_on(
            move || {
                let repository = prepared.path.as_deref().expect("explicit cwd");
                let result = io_result(repository, || {
                    #[cfg(test)]
                    assert!(!fail, "injected workspace preparation panic");
                    Workspaces::prepare(repository)
                });
                Completion::Prepared(swarm, prepared, result)
            },
            &self.runtime,
        );
        Err(Box::new(Error::Pending(swarm)))
    }
    pub fn legacy(&mut self) -> store::Result<Port> {
        let binding = self.cache.bind_startup(Ulid::nil(), 0)?;
        self.port(binding)
    }
    pub fn port(&self, binding: RootBinding) -> store::Result<Port> {
        let root = self.cache.get(&binding)?;
        if let Some((_, error)) = self
            .scans
            .get(&binding.root_id)
            .and_then(|state| state.failure.as_ref())
        {
            return Err(Box::new(error.clone()));
        }
        if root.failed() {
            return Err(Box::new(Error::Unavailable {
                repository: root.root().to_owned(),
                reason: "filesystem watch failed; root reconciliation required".into(),
            }));
        }
        Ok(Port {
            binding,
            index: root.index.clone(),
            host: root.host.clone(),
        })
    }
    pub fn current(&self, registry: &Registry, binding: &RootBinding) -> store::Result<()> {
        if binding.swarm != Ulid::nil() {
            let desired = Self::desired(registry, binding.swarm)?;
            if desired.revision != binding.revision
                || self
                    .attempts
                    .get(&binding.swarm)
                    .is_none_or(|old| *old.desired != desired)
            {
                return Err(Box::new(Error::Changed(binding.swarm)));
            }
        }
        self.port(binding.clone())?;
        Ok(())
    }
    fn wanted(&self, registry: &Registry, swarm: Ulid, desired: &Arc<Desired>) -> bool {
        Self::desired(registry, swarm).is_ok_and(|current| current == **desired)
            && self
                .attempts
                .get(&swarm)
                .is_some_and(|attempt| Arc::ptr_eq(&attempt.desired, desired))
    }
    fn failed(&mut self, swarm: Ulid, desired: &Desired, error: Error) {
        let _ = self.cache.fail_binding(swarm, desired.revision, error);
        if let Some(attempt) = self.attempts.get_mut(&swarm) {
            attempt.failed = Some(Instant::now());
        }
    }
    fn retire(&mut self, mounted: MountedRoot, root: Option<String>) {
        if let Some(root) = &root {
            self.retiring.insert(root.clone());
        }
        self.jobs.spawn_blocking_on(
            move || {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| mounted.close()));
                root.map_or(Completion::Closed, Completion::Retired)
            },
            &self.runtime,
        );
    }
    fn remove_waiter(&mut self, swarm: Ulid) {
        for waiters in self.mounting.values_mut() {
            waiters.remove(&swarm);
        }
    }
    fn forget(&mut self, swarm: Ulid) -> Option<String> {
        self.remove_waiter(swarm);
        self.attempts.remove(&swarm);
        self.cache.forget(swarm)
    }
    fn binding_idle(&self, swarm: Ulid) -> bool {
        self.cache.current(swarm).map_or_else(
            |_| self.attempts[&swarm].failed.is_some(),
            |binding| self.root_idle(&binding.root_id),
        )
    }
    fn root_idle(&self, root: &str) -> bool {
        !self.scans.get(root).is_some_and(|scan| scan.running)
            && self
                .cache
                .root(root)
                .is_some_and(|workspace| !workspace.host.borrowed())
    }
    // Archived swarms and open-to-empty membership transitions both release
    // bindings. Unarchived seatless swarms can open a root on demand.
    fn reap_closed(&mut self, registry: &Registry) {
        let epoch = registry.routing_epoch();
        if self.registry_epoch == Some(epoch) || self.attempts.is_empty() {
            return;
        }
        let Ok(memberships) = registry.open_swarm_memberships(self.attempts.keys().copied()) else {
            // Retry a failed authoritative lookup on the next owner turn.
            return;
        };
        for (swarm, open, archived) in memberships {
            let attempt = self
                .attempts
                .get_mut(&swarm)
                .expect("cached swarm membership");
            if archived || (attempt.open && !open) {
                if let Some(root) = self.forget(swarm) {
                    self.cache.retire_if_unbound(&root);
                }
            } else {
                attempt.open = open;
            }
        }
        self.registry_epoch = Some(epoch);
    }
    pub(crate) fn release_workspace(&mut self, checkout: &std::path::Path) {
        let released: Vec<_> = self
            .attempts
            .iter()
            .filter(|(_, attempt)| {
                attempt
                    .desired
                    .path
                    .as_deref()
                    .is_some_and(|path| path.starts_with(checkout))
            })
            .map(|(swarm, _)| *swarm)
            .collect();
        for swarm in released {
            if let Some(root) = self.forget(swarm) {
                self.cache.retire_if_unbound(&root);
            }
        }
        let roots: Vec<_> = self
            .cache
            .roots()
            .filter(|workspace| workspace.root().starts_with(checkout))
            .map(|workspace| workspace.root_id().to_owned())
            .collect();
        // A sibling alias into the released checkout loses its root as well.
        for root in roots {
            self.forget_root_bindings(&root);
            self.cache.retire_if_unbound(&root);
        }
    }
    fn forget_root_bindings(&mut self, root: &str) {
        let swarms: Vec<_> = self
            .attempts
            .keys()
            .copied()
            .filter(|swarm| {
                self.cache
                    .current(*swarm)
                    .is_ok_and(|binding| binding.root_id == root)
            })
            .collect();
        for swarm in swarms {
            self.forget(swarm);
        }
    }
    fn retire_root(&mut self, root: String) {
        let workspace = self
            .cache
            .remove_root(&root)
            .expect("retiring mounted root");
        self.scans.remove(&root);
        self.scan_queue.retain(|queued| queued != &root);
        self.ready_scans.remove(&root);
        self.retiring.insert(root.clone());
        self.jobs.spawn_blocking_on(
            move || {
                let _ =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| workspace.retire()));
                Completion::Retired(root)
            },
            &self.runtime,
        );
    }
    fn evict_idle_root(&mut self) -> Option<String> {
        let protected: BTreeSet<_> = self
            .attempts
            .iter()
            .filter(|(_, attempt)| attempt.open)
            .filter_map(|(swarm, _)| self.cache.current(*swarm).ok())
            .map(|binding| binding.root_id.as_str())
            .collect();
        let idle = self
            .cache
            .roots()
            .filter(|workspace| {
                let root = workspace.root_id();
                root != self.cache.startup().root_id()
                    && !protected.contains(root)
                    && self.root_idle(root)
            })
            .min_by_key(|workspace| workspace.last_selected)
            .map(|workspace| workspace.root_id().to_owned());
        let root = idle?;
        self.forget_root_bindings(&root);
        self.retire_root(root.clone());
        Some(root)
    }
    pub fn poll(&mut self, registry: &Registry) -> Vec<String> {
        self.reap_closed(registry);
        self.poll_scans();
        let mut retired: Vec<_> = self
            .cache
            .roots()
            .filter(|workspace| workspace.retire_when_idle && self.root_idle(workspace.root_id()))
            .map(|workspace| workspace.root_id().to_owned())
            .collect();
        for root in &retired {
            self.retire_root(root.clone());
        }
        self.start_scans(Instant::now());
        while let Some(joined) = self.jobs.try_join_next() {
            let done = match joined {
                Ok(done) => done,
                Err(error) if error.is_cancelled() => continue, // owner shutdown only
                Err(error) => {
                    eprintln!("workspace completion interrupted: {error}");
                    continue;
                }
            };
            match done {
                Completion::Closed => (),
                Completion::Retired(root) => {
                    self.retiring.remove(&root);
                }
                Completion::Prepared(swarm, desired, result) => {
                    if !self.wanted(registry, swarm, &desired) {
                        continue;
                    }
                    let prepared = match result {
                        Ok(root) => root,
                        Err(error) => {
                            self.failed(swarm, &desired, error);
                            continue;
                        }
                    };
                    match self.cache.bind_cached(swarm, desired.revision, &prepared) {
                        Ok(Some(_)) => continue,
                        Err(error) => {
                            self.failed(swarm, &desired, error);
                            continue;
                        }
                        Ok(None) => (),
                    }
                    let id = prepared.root_id().to_owned();
                    if let Some(waiters) = self.mounting.get_mut(&id) {
                        waiters.insert(swarm, desired);
                        continue;
                    }
                    if self.retiring.contains(&id) {
                        self.failed(swarm, &desired, Error::Pending(swarm));
                        continue;
                    }
                    if self.cache.roots().count() + self.mounting.len() + self.retiring.len()
                        >= self.root_limit
                    {
                        // An in-flight retirement still owns its slot and will make room.
                        if self.retiring.is_empty() {
                            retired.extend(self.evict_idle_root());
                        }
                        self.failed(swarm, &desired, Error::Pending(swarm));
                        continue;
                    }
                    self.mounting
                        .insert(id.clone(), BTreeMap::from([(swarm, desired)]));
                    let job = self.cache.mount(prepared.clone());
                    self.jobs.spawn_blocking_on(
                        move || {
                            let result = io_result(prepared.repository(), || job.run());
                            Completion::Mounted(id, prepared, Box::new(result))
                        },
                        &self.runtime,
                    );
                }
                Completion::Mounted(id, prepared, result) => {
                    let waiters = self.mounting.remove(&id).unwrap_or_default();
                    let waiters: Vec<_> = waiters
                        .into_iter()
                        .filter(|(swarm, desired)| self.wanted(registry, *swarm, desired))
                        .collect();
                    match *result {
                        Err(error) => {
                            for (swarm, desired) in waiters {
                                self.failed(swarm, &desired, error.clone());
                            }
                        }
                        Ok(mounted) => {
                            let mut waiters = waiters.into_iter();
                            let Some((swarm, desired)) = waiters.next() else {
                                self.retire(mounted, Some(id));
                                continue;
                            };
                            let installed = self.cache.insert(swarm, desired.revision, mounted);
                            if let Some(retired) = installed.retired {
                                self.retire(retired, None);
                            }
                            if let Err(error) = installed.binding {
                                self.failed(swarm, &desired, error);
                            }
                            for (swarm, desired) in waiters {
                                if let Err(error) =
                                    self.cache.bind_cached(swarm, desired.revision, &prepared)
                                {
                                    self.failed(swarm, &desired, error);
                                }
                            }
                        }
                    }
                }
            }
        }
        retired
    }
    /// Reconcile one root on IO. The owner excludes effects/provenance while
    /// its mirror/outbox can change, rather than discarding a raced DB commit.
    pub fn reconcile(&mut self, root: &str) -> store::Result<()> {
        self.queue_scan(root, true, Instant::now())
    }
    pub fn reconcile_pending(&mut self, root: &str, now: Instant) -> store::Result<()> {
        self.queue_scan(root, false, now)
    }
    /// Before a read: scan the paths the watcher has seen change, undebounced;
    /// with none and no rescan due, release the read at once.
    pub fn reconcile_read(&mut self, root: &str) -> store::Result<()> {
        if self.cache.root_mut(root).is_some() {
            self.scans.entry(root.to_owned()).or_default().read = true;
        }
        self.queue_scan(root, false, Instant::now())
    }
    fn queue_scan(&mut self, root: &str, force: bool, now: Instant) -> store::Result<()> {
        if self.cache.root_mut(root).is_none() {
            return Err(store::invalid("unknown workspace root"));
        }
        let state = self.scans.entry(root.to_owned()).or_default();
        state.force |= force;
        if !state.queued {
            state.queued = true;
            if !state.running {
                self.scan_queue.push_back(root.to_owned());
            }
        }
        self.start_scans(now);
        Ok(())
    }
    fn start_scans(&mut self, now: Instant) {
        // One job per root, at most two scans process-wide for this owner. A
        // continuously dirty root rejoins the FIFO behind other workspaces.
        let mut budget = self.scan_queue.len();
        while self.scan_jobs.len() < 2 && budget > 0 {
            budget -= 1;
            let Some(root) = self.scan_queue.pop_front() else {
                break;
            };
            let state = self.scans.get_mut(&root).expect("queued scan");
            if state
                .failure
                .as_ref()
                .is_some_and(|(at, _)| at.elapsed() < Duration::from_secs(1))
            {
                self.scan_queue.push_back(root);
                continue;
            }
            state.queued = false;
            let Some(workspace) = self.cache.root_mut(&root) else {
                continue;
            };
            let read = std::mem::take(&mut state.read);
            // Read-triggered scans also check the deadline so a stream of
            // early admissions cannot starve reconciliation of lost hints.
            let rescan = workspace.index.rescan_due(now);
            let batch = if state.force {
                let _ = workspace.watch.take(true);
                Some(ScanBatch::Full)
            } else {
                let read = read && !workspace.failed();
                let batch = workspace.watch.take(read || rescan);
                if batch.is_none() && !rescan {
                    if read {
                        self.ready_scans.insert(root);
                    }
                    continue;
                }
                batch
            };
            state.force = false;
            state.running = true;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            let replace_watch = workspace.failed();
            let base = workspace.index.clone();
            let repository = workspace.root().to_owned();
            #[cfg(test)]
            let hook = self.scan_hook.clone();
            #[cfg(test)]
            let complete_hook = self.scan_complete_hook.clone();
            self.scan_jobs.spawn_blocking_on(
                move || {
                    let mut index = base.clone();
                    let mut watch = None;
                    let result = io_result(&repository, || {
                        if replace_watch {
                            let replacement =
                                base.start_watch().map_err(|error| Error::Unavailable {
                                    repository: repository.clone(),
                                    reason: error.to_string(),
                                })?;
                            if replacement.root() != repository {
                                return Err(Error::Unavailable {
                                    repository: repository.clone(),
                                    reason: "watch root changed during recovery".into(),
                                });
                            }
                            let _ = replacement.take(true);
                            watch = Some(replacement);
                        }
                        #[cfg(test)]
                        if let Some(hook) = hook {
                            hook(&root);
                        }
                        let index = Arc::make_mut(&mut index);
                        match (rescan, batch) {
                            (true, hints) => index.rescan(hints),
                            (false, Some(ScanBatch::Paths(paths))) => index.scan_paths(&paths),
                            _ => index.scan(),
                        }
                        .map(|_| ())
                        .map_err(|error| Error::Unavailable {
                            repository: repository.clone(),
                            reason: error.to_string(),
                        })
                    });
                    if result.is_err() {
                        Arc::make_mut(&mut index).rebuild_graphs();
                    }
                    #[cfg(test)]
                    if let Some(hook) = complete_hook {
                        hook(&root);
                    }
                    ScanCompletion {
                        root,
                        generation,
                        watch,
                        base,
                        index,
                        result,
                    }
                },
                &self.runtime,
            );
        }
    }
    pub fn scanning(&self, root: &str) -> bool {
        self.scans
            .get(root)
            .is_some_and(|state| state.running || state.queued)
    }
    #[cfg(test)]
    pub fn scan_work(&self) -> (usize, usize) {
        (self.scan_jobs.len(), self.scan_queue.len())
    }
    pub fn reconciliation_failed(&self, root: &str) -> bool {
        self.scans
            .get(root)
            .is_some_and(|state| state.failure.is_some())
    }
    pub fn write_ready(&self, registry: &Registry, binding: &RootBinding) -> store::Result<()> {
        self.current(registry, binding)?;
        if self.scanning(&binding.root_id) || self.cache.get(binding)?.pending() {
            return Err(Box::new(Error::Pending(binding.swarm)));
        }
        Ok(())
    }
    pub fn retry_publication(&mut self, root: String) {
        if self.cache.root(&root).is_some() {
            self.ready_scans.insert(root);
        }
    }
    pub fn take_ready_scans(&mut self) -> Vec<String> {
        use std::ops::Bound::{Excluded, Unbounded};
        let mut selected: Vec<_> = match &self.ready_cursor {
            Some(cursor) => self
                .ready_scans
                .range((Excluded(cursor.clone()), Unbounded))
                .take(2)
                .cloned()
                .collect(),
            None => self.ready_scans.iter().take(2).cloned().collect(),
        };
        if selected.len() < 2 {
            let remaining: Vec<_> = self
                .ready_scans
                .iter()
                .filter(|root| !selected.contains(root))
                .take(2 - selected.len())
                .cloned()
                .collect();
            selected.extend(remaining);
        }
        for root in &selected {
            self.ready_scans.remove(root);
        }
        if let Some(last) = selected.last() {
            self.ready_cursor = Some(last.clone());
        }
        selected
    }
    fn poll_scans(&mut self) {
        while let Some(joined) = self.scan_jobs.try_join_next() {
            let Ok(done) = joined else {
                continue;
            };
            let ScanCompletion {
                root,
                generation,
                base,
                index,
                watch,
                result,
            } = done;
            let mut prepared = Some(index);
            let mut watch = watch;
            let mut retired = None;
            let mut retired_watch = None;
            if let Some(state) = self.scans.get_mut(&root)
                && state.running
                && state.generation == generation
            {
                state.running = false;
                if let Some(workspace) = self.cache.root_mut(&root) {
                    if !Arc::ptr_eq(&workspace.index, &base) {
                        state.failure = Some((
                            Instant::now(),
                            Error::Unavailable {
                                repository: workspace.root().to_owned(),
                                reason: "workspace snapshot changed during reconciliation; retry"
                                    .into(),
                            },
                        ));
                    } else {
                        retired = Some(std::mem::replace(
                            &mut workspace.index,
                            prepared.take().expect("prepared scan"),
                        ));
                        if let Some(replacement) = watch.take() {
                            retired_watch =
                                Some(std::mem::replace(&mut workspace.watch, replacement));
                        }
                        state.failure = result.err().map(|error| (Instant::now(), error));
                    }
                    self.ready_scans.insert(root.clone());
                }
                if state.queued {
                    self.scan_queue.push_back(root);
                }
            }
            // Transfer, rather than clone, the published Arc; retaining a
            // second new Arc here would force the next owner edit to clone it.
            self.jobs.spawn_blocking_on(
                move || {
                    drop((base, retired, prepared, watch, retired_watch));
                    Completion::Closed
                },
                &self.runtime,
            );
        }
    }
    fn cached_root_match(
        &self,
        registry: &Registry,
        swarm: Ulid,
        root: &str,
    ) -> store::Result<Option<bool>> {
        let desired = Self::desired(registry, swarm)?;
        if desired.archived {
            return Ok(Some(false));
        }
        // Legacy/default metadata is itself a trusted startup selection. Event
        // routing does not require an unrelated document read to have run first.
        if desired.path.is_none() {
            return Ok(Some(root == self.cache.startup().root_id()));
        }
        if !self
            .attempts
            .get(&swarm)
            .is_some_and(|attempt| *attempt.desired == desired)
        {
            return Ok(None);
        }
        match self.cache.current(swarm) {
            Ok(binding) if binding.revision == desired.revision => {
                Ok(Some(binding.root_id == root))
            }
            Err(Error::Unavailable { .. }) => Ok(Some(false)),
            _ => Ok(None),
        }
    }
    /// One event's explicit candidate can start its own canonical IO admission.
    /// Pending/unavailable explicit roots remain account-scoped with source-root
    /// evidence; neither case is permission to guess or use the startup root.
    pub fn matches_root(
        &mut self,
        registry: &Registry,
        swarm: Ulid,
        root: &str,
    ) -> store::Result<bool> {
        if let Some(matches) = self.cached_root_match(registry, swarm, root)? {
            return Ok(matches);
        }
        match self.select(registry, swarm, None) {
            Ok(port) => Ok(port.binding.root_id == root),
            Err(error) if error.downcast_ref::<Error>().is_some() => Ok(false),
            Err(error) => Err(error),
        }
    }
    pub fn bound_swarms(&self, registry: &Registry, root: &str) -> store::Result<Vec<Ulid>> {
        let mut swarms: std::collections::BTreeSet<_> = registry
            .swarms()?
            .into_iter()
            .map(|swarm| swarm.id)
            .collect();
        swarms.extend(registry.list()?.into_iter().map(|seat| seat.swarm));
        let mut selected = Vec::new();
        for swarm in swarms {
            if self.cached_root_match(registry, swarm, root)? == Some(true) {
                selected.push(swarm);
            }
        }
        Ok(selected)
    }
}

#[cfg(test)]
mod workspaces_retention_tests;
