// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! IO-owned workspace mounts. A directory selects records, never authority.
//! Mounting and scanning are blocking IO operations; callers schedule them off
//! the UI thread. Reads retain the existing immutable Index snapshots.
use super::{DataSource, data_source as d, roots};
use crate::{
    Ulid,
    bus::BusHandle,
    identity::IdentityRuntime,
    index::{FileWatch, Index},
    plugins::{ProviderConfig, Registry as PluginRegistry},
    provenance::engine::Engine,
    registry::Registry,
    store::{events::Draft, object_dir, sqlite::SqliteStore},
};
use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::runtime::Handle;

/// Existing account/runtime ports, supplied by their owner rather than opened
/// through the operator's HOME or inferred from an untrusted request.
#[derive(Clone)]
pub struct Config {
    pub startup_repository: PathBuf,
    pub run_dir: PathBuf,
    pub plugins: PluginRegistry,
    pub runtime: Handle,
    pub provider: ProviderConfig,
    pub activity: Option<BusHandle>,
    pub issued_by: Option<String>,
}

/// Captured at admission. Reusing a root ID alone does not survive a cwd change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootBinding {
    pub swarm: Ulid,
    pub revision: u64,
    pub root_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Unavailable { repository: PathBuf, reason: String },
    UnknownSwarm(Ulid),
    Pending(Ulid),
    Changed(Ulid),
}
impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "workspace_unavailable",
            Self::UnknownSwarm(_) => "unknown_swarm_workspace",
            Self::Pending(_) => "workspace_pending",
            Self::Changed(_) => "swarm_root_changed",
        }
    }
    fn unavailable(repository: &Path, error: impl fmt::Display) -> Self {
        Self::Unavailable {
            repository: repository.into(),
            reason: error.to_string(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { repository, reason } => write!(
                f,
                "workspace_unavailable: workspace {} unavailable: {reason}",
                repository.display()
            ),
            Self::UnknownSwarm(swarm) => write!(f, "swarm {swarm} has no captured workspace"),
            Self::Pending(swarm) => write!(
                f,
                "workspace_pending: swarm {swarm} workspace is opening; retry shortly"
            ),
            Self::Changed(swarm) => write!(
                f,
                "swarm_root_changed: swarm {swarm} workspace changed; capture its current binding"
            ),
        }
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

/// One provider, file watch and provenance engine per canonical knowledge root.
pub struct Workspace {
    pub index: Arc<Index>,
    pub(crate) host: roots::Host,
    pub(crate) watch: FileWatch,
    pub(crate) provenance: Engine,
    seen_revision: u64,
}
impl Workspace {
    fn mount(config: &Config, repository: &Path, root: &Path, store: &str) -> Result<Self> {
        let open = || -> crate::store::Result<Self> {
            if !std::fs::metadata(repository)?.is_dir() {
                return Err(crate::store::invalid(
                    "working directory is not a directory",
                ));
            }
            let mut index = Index::open(
                root,
                SqliteStore::open(&object_dir(&config.run_dir, store)?)?,
            )?;
            if index.knowledge_root() != root {
                return Err(crate::store::invalid(
                    "workspace root changed during preparation",
                ));
            }
            index.set_issued_by(config.issued_by.clone());
            if let Some(bus) = &config.activity {
                if store == "index" {
                    index.route_activity(bus.clone(), config.runtime.clone());
                } else {
                    index.route_activity_to(bus.clone(), config.runtime.clone(), store.into());
                }
            }
            // Keep the same durable scan outbox; the caller publishes/acks it.
            let _ = index.scan()?;
            let host = roots::Host::with_config(
                index.knowledge_root(),
                config.plugins.for_repository(repository),
                config.runtime.clone(),
                config.provider,
            )?;
            if let Some(bus) = &config.activity {
                host.plugin_host().attach_bus(bus.clone());
            }
            let seen_revision = host.revision()?;
            let provenance = Engine::open(&index, repository, config.runtime.clone())?;
            let watch = FileWatch::start(index.knowledge_root())?;
            Ok(Self {
                index: Arc::new(index),
                host,
                watch,
                provenance,
                seen_revision,
            })
        };
        open().map_err(|error| Error::unavailable(repository, error))
    }
    pub fn root_id(&self) -> &str {
        self.host.root_id()
    }
    pub fn root(&self) -> &Path {
        self.index.knowledge_root()
    }
    pub fn changed(&self) -> bool {
        self.watch.changed()
    }
    pub fn failed(&self) -> bool {
        self.watch.failed()
    }
    pub fn provider_revision(&self) -> crate::store::Result<u64> {
        self.host.revision()
    }
    /// Keep an invalidation pending until its owner publishes it successfully.
    pub fn provider_changed(&self) -> crate::store::Result<Option<u64>> {
        let revision = self.host.revision()?;
        Ok((revision != self.seen_revision).then_some(revision))
    }
    pub fn acknowledge_provider_change(&mut self, revision: u64) {
        self.seen_revision = revision;
    }
    /// Selection is not authorization; host callers still apply the admitted
    /// actor's existing task/document policy before exposing a result.
    pub fn with_source<T>(
        &self,
        registry: &Registry,
        read: impl FnOnce(&dyn DataSource) -> d::Result<T>,
    ) -> d::Result<T> {
        self.host.with_source(&self.index, registry, read)
    }
    pub fn status(&self) -> crate::store::Result<bbt_proto::gui::KnowledgeStatus> {
        self.host.status(&self.index)
    }
    /// Preserve the existing bounded provenance step for this root only.
    pub fn step_provenance(
        &mut self,
        bus: &BusHandle,
        runtime: &Handle,
        identity: &IdentityRuntime,
        active: &[(Ulid, u64, u64)],
    ) -> crate::store::Result<bool> {
        self.provenance.step(
            Arc::make_mut(&mut self.index),
            bus,
            runtime,
            identity,
            active,
        )
    }
    pub fn close(&self) {
        self.host.close();
    }
}

enum Selection {
    Ready(RootBinding),
    Unavailable { revision: u64, error: Error },
}

/// Canonical filesystem preparation, produced on blocking IO before the owner
/// decides whether a mount already exists or is in flight for this root ID.
#[derive(Debug, Clone)]
pub struct PreparedRoot {
    repository: PathBuf,
    root: PathBuf,
    root_id: String,
}
impl PreparedRoot {
    pub fn root_id(&self) -> &str {
        &self.root_id
    }
    pub fn repository(&self) -> &Path {
        &self.repository
    }
}

/// Owned work sent to blocking IO without lending or locking the cache.
pub struct MountJob {
    config: Config,
    prepared: PreparedRoot,
    store: String,
}
impl MountJob {
    pub fn run(self) -> Result<MountedRoot> {
        bbt_rt::assert_not_ui_thread();
        let workspace = Workspace::mount(
            &self.config,
            &self.prepared.repository,
            &self.prepared.root,
            &self.store,
        )?;
        Ok(MountedRoot {
            prepared: self.prepared,
            workspace: Some(workspace),
            cleanup: self.config.runtime,
        })
    }
}
pub struct MountedRoot {
    prepared: PreparedRoot,
    workspace: Option<Workspace>,
    cleanup: Handle,
}
impl Drop for MountedRoot {
    fn drop(&mut self) {
        if let Some(workspace) = self.workspace.take() {
            // Aborted owner jobs can discard a mount result. Provider cleanup
            // remains on IO, including this cancellation/shutdown path.
            self.cleanup.spawn_blocking(move || workspace.close());
        }
    }
}
impl MountedRoot {
    /// Cleanup is an IO operation; never close a provider in the owner's insertion path.
    pub fn close(mut self) {
        if let Some(workspace) = self.workspace.take() {
            workspace.close();
        }
    }
}
#[must_use = "schedule retired mount cleanup on IO"]
pub struct Installation {
    pub binding: Result<RootBinding>,
    pub retired: Option<MountedRoot>,
}

pub struct Workspaces {
    config: Config,
    startup: String,
    roots: BTreeMap<String, Workspace>,
    paths: BTreeMap<PathBuf, String>,
    bindings: BTreeMap<Ulid, Selection>,
}
impl Workspaces {
    pub fn open(config: Config) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        // Preserve initial bootstrap behavior; only the trusted startup default
        // is created. An explicit swarm cwd must already exist.
        std::fs::create_dir_all(&config.startup_repository)
            .map_err(|error| Error::unavailable(&config.startup_repository, error))?;
        let (repository, root) = Self::directory(&config.startup_repository)?;
        // Preserve the existing startup mirror/outbox location across upgrades.
        let workspace = Workspace::mount(&config, &repository, &root, "index")?;
        let startup = workspace.root_id().to_owned();
        Ok(Self {
            config,
            startup: startup.clone(),
            roots: BTreeMap::from([(startup.clone(), workspace)]),
            paths: BTreeMap::from([(root, startup)]),
            bindings: BTreeMap::new(),
        })
    }
    fn directory(repository: &Path) -> Result<(PathBuf, PathBuf)> {
        let resolve = || -> std::io::Result<_> {
            let repository = repository.canonicalize()?;
            if !repository.is_dir() {
                return Err(std::io::Error::other(
                    "working directory is not a directory",
                ));
            }
            let root = repository.join(".bboxy");
            std::fs::create_dir_all(&root)?;
            Ok((repository, root.canonicalize()?))
        };
        resolve().map_err(|error| Error::unavailable(repository, error))
    }
    pub fn prepare(repository: &Path) -> Result<PreparedRoot> {
        bbt_rt::assert_not_ui_thread();
        let (repository, root) = Self::directory(repository)?;
        let root_id =
            roots::root_identity(&root).map_err(|error| Error::unavailable(&repository, error))?;
        Ok(PreparedRoot {
            repository,
            root,
            root_id,
        })
    }
    /// A quick owner-side cache check before scheduling an expensive mount.
    pub fn bind_cached(
        &mut self,
        swarm: Ulid,
        revision: u64,
        prepared: &PreparedRoot,
    ) -> Result<Option<RootBinding>> {
        self.check_revision(swarm, revision)?;
        let Some(id) = self.paths.get(&prepared.root).cloned() else {
            return Ok(None);
        };
        Ok(Some(self.select(swarm, revision, id)))
    }
    /// Build an owned job only. The router deduplicates jobs by prepared root ID.
    pub fn mount(&self, prepared: PreparedRoot) -> MountJob {
        let store = if prepared.root_id == self.startup {
            "index".into()
        } else {
            format!("index-{}", prepared.root_id)
        };
        MountJob {
            config: self.config.clone(),
            prepared,
            store,
        }
    }
    fn install_root(&mut self, mut mounted: MountedRoot) -> (String, Option<MountedRoot>) {
        if let Some(id) = self.paths.get(&mounted.prepared.root) {
            return (id.clone(), Some(mounted));
        }
        let prepared = mounted.prepared.clone();
        let workspace = mounted.workspace.take().expect("owned mount");
        let id = prepared.root_id;
        debug_assert_eq!(workspace.root_id(), id);
        self.paths.insert(prepared.root, id.clone());
        self.roots.insert(id.clone(), workspace);
        (id, None)
    }
    /// Pure owner-side insertion. Stale or duplicate mount cleanup is returned
    /// to the caller to schedule on IO, rather than closing a provider here.
    pub fn insert(&mut self, swarm: Ulid, revision: u64, mounted: MountedRoot) -> Installation {
        if let Err(error) = self.check_revision(swarm, revision) {
            return Installation {
                binding: Err(error),
                retired: Some(mounted),
            };
        }
        let (id, retired) = self.install_root(mounted);
        Installation {
            binding: Ok(self.select(swarm, revision, id)),
            retired,
        }
    }
    fn mount_sync(&mut self, repository: &Path) -> Result<String> {
        let prepared = Self::prepare(repository)?;
        if let Some(id) = self.paths.get(&prepared.root) {
            return Ok(id.clone());
        }
        let mounted = self.mount(prepared).run()?;
        let (id, retired) = self.install_root(mounted);
        if let Some(retired) = retired {
            retired.close();
        }
        Ok(id)
    }
    fn check_revision(&self, swarm: Ulid, revision: u64) -> Result<()> {
        let previous = self.bindings.get(&swarm).map(|selection| match selection {
            Selection::Ready(binding) => binding.revision,
            Selection::Unavailable { revision, .. } => *revision,
        });
        if previous.is_some_and(|current| revision < current) {
            Err(Error::Changed(swarm))
        } else {
            Ok(())
        }
    }
    fn select(&mut self, swarm: Ulid, revision: u64, root_id: String) -> RootBinding {
        let binding = RootBinding {
            swarm,
            revision,
            root_id,
        };
        self.bindings
            .insert(swarm, Selection::Ready(binding.clone()));
        binding
    }
    /// Apply a failed IO completion without probing the filesystem in the owner.
    pub fn fail_binding(&mut self, swarm: Ulid, revision: u64, error: Error) -> Result<()> {
        self.check_revision(swarm, revision)?;
        self.bindings
            .insert(swarm, Selection::Unavailable { revision, error });
        Ok(())
    }
    /// Call only with the host's authoritative swarm metadata. A failed explicit
    /// mount replaces its old selection, so it cannot read another project.
    pub fn bind(
        &mut self,
        swarm: Ulid,
        revision: u64,
        repository: Option<&Path>,
    ) -> Result<RootBinding> {
        bbt_rt::assert_not_ui_thread();
        self.check_revision(swarm, revision)?;
        let root = match repository {
            Some(path) => self.mount_sync(path),
            None => Ok(self.startup.clone()),
        };
        match root {
            Ok(root_id) => Ok(self.select(swarm, revision, root_id)),
            Err(error) => {
                self.fail_binding(swarm, revision, error.clone())?;
                Err(error)
            }
        }
    }
    /// Owner-only selection of the already-mounted legacy startup root.
    pub fn bind_startup(&mut self, swarm: Ulid, revision: u64) -> Result<RootBinding> {
        self.check_revision(swarm, revision)?;
        Ok(self.select(swarm, revision, self.startup.clone()))
    }
    pub(crate) fn root_mut(&mut self, root: &str) -> Option<&mut Workspace> {
        self.roots.get_mut(root)
    }
    pub fn current(&self, swarm: Ulid) -> Result<&RootBinding> {
        match self.bindings.get(&swarm) {
            Some(Selection::Ready(binding)) => Ok(binding),
            Some(Selection::Unavailable { error, .. }) => Err(error.clone()),
            None => Err(Error::UnknownSwarm(swarm)),
        }
    }
    fn validate(&self, binding: &RootBinding) -> Result<()> {
        match self.bindings.get(&binding.swarm) {
            Some(Selection::Ready(current)) if current == binding => Ok(()),
            Some(Selection::Unavailable { revision, error }) if *revision == binding.revision => {
                Err(error.clone())
            }
            _ => Err(Error::Changed(binding.swarm)),
        }
    }
    pub fn get(&self, binding: &RootBinding) -> Result<&Workspace> {
        self.validate(binding)?;
        Ok(self.roots.get(&binding.root_id).expect("mounted binding"))
    }
    pub fn get_mut(&mut self, binding: &RootBinding) -> Result<&mut Workspace> {
        self.validate(binding)?;
        Ok(self
            .roots
            .get_mut(&binding.root_id)
            .expect("mounted binding"))
    }
    pub fn scan(&mut self, binding: &RootBinding) -> Result<Vec<Draft>> {
        let workspace = self.get_mut(binding)?;
        Arc::make_mut(&mut workspace.index)
            .scan()
            .map_err(|error| Error::unavailable(workspace.root(), error))
    }
    pub fn startup(&self) -> &Workspace {
        self.roots.get(&self.startup).expect("startup workspace")
    }
    pub fn startup_mut(&mut self) -> &mut Workspace {
        self.roots
            .get_mut(&self.startup)
            .expect("startup workspace")
    }
    /// Explicit status/provider-control enumeration, never needed for a read.
    pub fn roots(&self) -> impl Iterator<Item = &Workspace> {
        self.roots.values()
    }
    pub fn close(&self) {
        for workspace in self.roots.values() {
            workspace.close();
        }
    }
}
