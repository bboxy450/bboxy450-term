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
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{runtime::Handle, task::JoinSet};

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
}
enum Completion {
    Prepared(Ulid, Desired, Result<PreparedRoot>),
    Mounted(String, PreparedRoot, Box<Result<MountedRoot>>),
    Closed,
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
    desired: Desired,
    failed: Option<Instant>,
}
/// Canonical roots are deduplicated after independent IO preparation; an
/// unavailable directory cannot delay another project's mount or any chat.
pub(crate) struct Router {
    pub cache: Workspaces,
    jobs: JoinSet<Completion>,
    attempts: BTreeMap<Ulid, Attempt>,
    mounting: BTreeMap<String, Vec<(Ulid, Desired)>>,
    runtime: Handle,
    scans: BTreeMap<String, ScanState>,
    scan_jobs: JoinSet<ScanCompletion>,
    scan_queue: VecDeque<String>,
    ready_scans: std::collections::BTreeSet<String>,
    ready_cursor: Option<String>,
    #[cfg(test)]
    pub scan_hook: Option<ScanHook>,
    #[cfg(test)]
    pub scan_complete_hook: Option<ScanHook>,
    #[cfg(test)]
    pub fail_next_prepare: bool,
}
fn io_result<T>(repository: &std::path::Path, job: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)).unwrap_or_else(|_| {
        Err(Error::Unavailable {
            repository: repository.to_owned(),
            reason: "workspace IO worker panicked; retry the root".into(),
        })
    })
}
impl Router {
    pub fn new(cache: Workspaces, runtime: Handle) -> Self {
        Self {
            cache,
            jobs: JoinSet::new(),
            attempts: BTreeMap::new(),
            mounting: BTreeMap::new(),
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
            },
            None => Desired {
                revision: 0,
                path: None,
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
        if desired.path.is_none() {
            let binding = self.cache.bind_startup(swarm, desired.revision)?;
            self.attempts.insert(
                swarm,
                Attempt {
                    desired,
                    failed: None,
                },
            );
            return self.port(binding);
        }
        let current_attempt = self
            .attempts
            .get(&swarm)
            .filter(|old| old.desired == desired);
        if let Some(attempt) = current_attempt {
            match self.cache.current(swarm) {
                Ok(binding) if binding.revision == desired.revision => {
                    return self.port(binding.clone());
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
        // Fence old snapshots immediately, before canonicalization or mount.
        self.cache
            .fail_binding(swarm, desired.revision, Error::Pending(swarm))?;
        self.attempts.insert(
            swarm,
            Attempt {
                desired: desired.clone(),
                failed: None,
            },
        );
        let prepared = desired.clone();
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
                    .is_none_or(|old| old.desired != desired)
            {
                return Err(Box::new(Error::Changed(binding.swarm)));
            }
        }
        self.port(binding.clone())?;
        Ok(())
    }
    fn wanted(&self, registry: &Registry, swarm: Ulid, desired: &Desired) -> bool {
        Self::desired(registry, swarm).is_ok_and(|current| current == *desired)
            && self
                .attempts
                .get(&swarm)
                .is_some_and(|attempt| attempt.desired == *desired)
    }
    fn failed(&mut self, swarm: Ulid, desired: &Desired, error: Error) {
        let _ = self.cache.fail_binding(swarm, desired.revision, error);
        if let Some(attempt) = self.attempts.get_mut(&swarm) {
            attempt.failed = Some(Instant::now());
        }
    }
    fn retire(&mut self, mounted: MountedRoot) {
        self.jobs.spawn_blocking_on(
            move || {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| mounted.close()));
                Completion::Closed
            },
            &self.runtime,
        );
    }
    pub fn poll(&mut self, registry: &Registry) {
        self.poll_scans();
        self.start_scans();
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
                        waiters.push((swarm, desired));
                        continue;
                    }
                    self.mounting.insert(id.clone(), vec![(swarm, desired)]);
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
                                self.retire(mounted);
                                continue;
                            };
                            let installed = self.cache.insert(swarm, desired.revision, mounted);
                            if let Some(retired) = installed.retired {
                                self.retire(retired);
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
    }
    /// Reconcile one root on IO. The owner excludes effects/provenance while
    /// its mirror/outbox can change, rather than discarding a raced DB commit.
    pub fn reconcile(&mut self, root: &str) -> store::Result<()> {
        self.queue_scan(root, true)
    }
    pub fn reconcile_pending(&mut self, root: &str) -> store::Result<()> {
        self.queue_scan(root, false)
    }
    /// Before a read: scan the paths the watcher has seen change, undebounced;
    /// with none, release the read at once. A failed watch still scans in full.
    pub fn reconcile_read(&mut self, root: &str) -> store::Result<()> {
        if self.cache.root_mut(root).is_some() {
            self.scans.entry(root.to_owned()).or_default().read = true;
        }
        self.queue_scan(root, false)
    }
    fn queue_scan(&mut self, root: &str, force: bool) -> store::Result<()> {
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
        self.start_scans();
        Ok(())
    }
    fn start_scans(&mut self) {
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
            let batch = if state.force {
                let _ = workspace.watch.take(true);
                ScanBatch::Full
            } else if read && !workspace.failed() {
                let Some(batch) = workspace.watch.take(true) else {
                    self.ready_scans.insert(root);
                    continue;
                };
                batch
            } else {
                let Some(batch) = workspace.watch.take(false) else {
                    continue;
                };
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
                            let replacement = FileWatch::start(&repository).map_err(|error| {
                                Error::Unavailable {
                                    repository: repository.clone(),
                                    reason: error.to_string(),
                                }
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
                        match batch {
                            ScanBatch::Full => index.reconcile_all(),
                            ScanBatch::Paths(paths) => index.scan_paths(&paths),
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
        self.ready_scans.insert(root);
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
        // Legacy/default metadata is itself a trusted startup selection. Event
        // routing does not require an unrelated document read to have run first.
        if desired.path.is_none() {
            return Ok(Some(root == self.cache.startup().root_id()));
        }
        if !self
            .attempts
            .get(&swarm)
            .is_some_and(|attempt| attempt.desired == desired)
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
