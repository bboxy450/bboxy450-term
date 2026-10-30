// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Account-owned root selection. Slow provider IO is side work; reset is local.
use super::{BboxySource, DataSource, data_source as d};
use crate::{
    index::Index,
    mcp::{self, Actor, Call, journal::Journal},
    plugins::{DataSourceProvider, PluginHost, ProviderConfig, Registry},
    store::{Result, invalid},
};
use bbt_plugin::{
    Refusal,
    registry::{Activation, Conformance, State},
};
use bbt_proto::gui::{KnowledgeProvider, KnowledgeRequest, KnowledgeRoot, KnowledgeStatus};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::{runtime::Handle, sync::Semaphore};

pub(crate) const BUNDLED: &str = "bboxy450-term-data_source-bboxy";
const PREFIX: &str = "knowledge/roots/";

pub(crate) fn root_identity(root: &std::path::Path) -> Result<String> {
    let path = root
        .to_str()
        .ok_or_else(|| invalid("knowledge root must be UTF-8"))?;
    Ok(format!("{:x}", Sha256::digest(path.as_bytes())))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Binding {
    root: String,
    provider: String,
    sha256: Option<String>,
    generation: Option<u64>,
    revision: u64,
}
struct Selected {
    binding: Binding,
    external: std::result::Result<Option<Arc<DataSourceProvider>>, Refusal>,
}
struct Inner {
    root: PathBuf,
    id: String,
    plugins: Registry,
    handle: Handle,
    config: ProviderConfig,
    selected: Mutex<Selected>,
    retired: Mutex<Vec<tokio::task::JoinHandle<()>>>,
    controls: Arc<Semaphore>,
    /// Every enabled plugin's live connection for this root, selected or not.
    plugin_host: PluginHost,
}
#[derive(Clone)]
pub(crate) struct Host(Arc<Inner>);

/// A multi-call operation retains one provider epoch; a swap cannot redirect its effect.
pub(crate) struct BoundSource<'a> {
    host: &'a Host,
    index: &'a Index,
    registry: &'a crate::registry::Registry,
    revision: d::Result<u64>,
}
impl BoundSource<'_> {
    pub(crate) fn revision(&self) -> d::Result<u64> {
        self.revision.clone()
    }
    fn call<T>(&self, f: impl FnOnce(&dyn DataSource) -> d::Result<T>) -> d::Result<T> {
        let revision = self.revision.clone()?;
        self.host.current(revision)?;
        let result = self
            .host
            .with_source_at(self.index, self.registry, Some(revision), f);
        self.host.current(revision)?;
        result
    }
}
impl DataSource for BoundSource<'_> {
    fn import_root(&self) -> d::Result<PathBuf> {
        self.call(|source| source.import_root())
    }
    fn resolve(&self, r: d::ResolveRequest) -> d::Result<d::RecordRef> {
        self.call(|s| s.resolve(r))
    }
    fn fetch(&self, r: d::FetchRequest) -> d::Result<d::Record> {
        self.call(|s| s.fetch(r))
    }
    fn search(&self, r: d::SearchRequest) -> d::Result<d::Page<d::SearchHit>> {
        self.call(|s| s.search(r))
    }
    fn list(&self, r: d::ListRequest) -> d::Result<d::Page<d::RecordRef>> {
        self.call(|s| s.list(r))
    }
    fn links(&self, r: d::FetchRequest) -> d::Result<d::Links> {
        self.call(|s| s.links(r))
    }
    fn graph(&self, r: d::GraphRequest) -> d::Result<d::GraphPage> {
        self.call(|s| s.graph(r))
    }
    fn graph_until(&self, r: d::GraphRequest, at: std::time::Instant) -> d::Result<d::GraphPage> {
        self.call(|s| s.graph_until(r, at))
    }
    fn list_until(
        &self,
        r: d::ListRequest,
        at: std::time::Instant,
    ) -> d::Result<d::Page<d::RecordRef>> {
        self.call(|s| s.list_until(r, at))
    }
    fn fetch_until(&self, r: d::FetchRequest, at: std::time::Instant) -> d::Result<d::Record> {
        self.call(|s| s.fetch_until(r, at))
    }
    fn changes_since(&self, r: d::ChangesRequest) -> d::Result<d::Changes> {
        self.call(|s| s.changes_since(r))
    }
    fn write(&self, r: d::WriteRequest, c: &super::WriteContext<'_>) -> d::Result<d::WriteResult> {
        self.host.current(self.revision.clone()?)?;
        self.call(|s| s.write(r, c)).map_err(|error| {
            if error.code == "root_changed" {
                d::CommandError::new(
                    "write_not_confirmed",
                    "Provider changed during write; inspect the original operation before retry.",
                )
            } else {
                error
            }
        })
    }
    fn commit(
        &self,
        r: d::CommitRequest,
        c: &super::WriteContext<'_>,
    ) -> d::Result<d::CommitResult> {
        self.host.current(self.revision.clone()?)?;
        self.call(|s| s.commit(r, c)).map_err(|error| {
            if error.code == "root_changed" {
                d::CommandError::new(
                    "commit_not_confirmed",
                    "Provider changed during commit; inspect the original operation before retry.",
                )
            } else {
                error
            }
        })
    }
}

fn load(db: &Connection, id: &str) -> Result<Option<Binding>> {
    let value: Option<Vec<u8>> = db
        .query_row(
            "SELECT value FROM kv WHERE key=?1",
            [format!("{PREFIX}{id}")],
            |row| row.get(0),
        )
        .optional()?;
    value
        .map(|v| serde_json::from_slice(&v).map_err(Into::into))
        .transpose()
}
fn portable(error: impl std::fmt::Display) -> d::CommandError {
    d::CommandError::new("provider_unavailable", error.to_string())
}
fn bundled() -> KnowledgeProvider {
    KnowledgeProvider {
        id: BUNDLED.into(),
        name: BUNDLED.into(),
        sha256: None,
    }
}
pub(crate) fn refuse_mounted_disable(
    db: &Connection,
    candidate: &str,
) -> std::result::Result<(), Refusal> {
    let run = || -> Result<bool> {
        let mut statement = db.prepare("SELECT value FROM kv WHERE key >= ?1 AND key < ?2")?;
        let rows =
            statement.query_map([PREFIX, "knowledge/roots0"], |row| row.get::<_, Vec<u8>>(0))?;
        for row in rows {
            let binding: Binding = serde_json::from_slice(&row?)?;
            if binding.provider == candidate {
                return Ok(true);
            }
        }
        Ok(false)
    };
    match run() {
        Ok(false) => Ok(()),
        Ok(true) => Err(Refusal::new(
            "provider_of_root",
            "Select the bundled provider in Settings > Knowledge before disabling this plugin.",
        )),
        Err(error) => Err(Refusal::new(
            "knowledge_store_unavailable",
            error.to_string(),
        )),
    }
}

impl Host {
    pub(crate) fn source_at<'a>(
        &'a self,
        index: &'a Index,
        registry: &'a crate::registry::Registry,
        revision: u64,
    ) -> BoundSource<'a> {
        BoundSource {
            host: self,
            index,
            registry,
            revision: Ok(revision),
        }
    }
    pub(crate) fn source<'a>(
        &'a self,
        index: &'a Index,
        registry: &'a crate::registry::Registry,
    ) -> BoundSource<'a> {
        BoundSource {
            host: self,
            index,
            registry,
            revision: self.revision().map_err(portable),
        }
    }
    pub(crate) fn plugin_control(&self) -> crate::plugins::control::Control {
        crate::plugins::control::Control::new(self.0.plugins.clone())
    }
    pub(crate) fn with_config(
        root: &std::path::Path,
        plugins: Registry,
        handle: Handle,
        config: ProviderConfig,
    ) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let root = root.canonicalize()?;
        let path = root
            .to_str()
            .ok_or_else(|| invalid("knowledge root must be UTF-8"))?
            .to_owned();
        let id = root_identity(&root)?;
        let default = Binding {
            root: path,
            provider: BUNDLED.into(),
            sha256: None,
            generation: None,
            revision: 0,
        };
        let stored = {
            let db = plugins.account().connection()?;
            load(&db, &id)
        };
        let (binding, external) = match stored {
            Ok(value)
                if value
                    .as_ref()
                    .is_none_or(|b| b.root == root.to_string_lossy()) =>
            {
                let binding = value.unwrap_or(default);
                let external = Self::instance(&plugins, &root, &binding, &handle, config);
                (binding, external)
            }
            other => {
                let mut binding = default;
                binding.provider = "unavailable".into();
                let reason = other.err().map_or_else(
                    || "Stored root binding differs from the current root".into(),
                    |e| e.to_string(),
                );
                (
                    binding,
                    Err(Refusal::new(
                        "invalid_root_binding",
                        format!("{reason}; reset this root to the bundled provider."),
                    )),
                )
            }
        };
        let plugin_host = PluginHost::new(plugins.clone(), &root, handle.clone(), config)
            .map_err(|e| invalid(e.to_string()))?;
        Ok(Self(Arc::new(Inner {
            root,
            id,
            plugins,
            handle,
            config,
            selected: Mutex::new(Selected { binding, external }),
            retired: Mutex::new(Vec::new()),
            controls: Arc::new(Semaphore::new(8)),
            plugin_host,
        })))
    }
    pub(crate) fn plugin_host(&self) -> &PluginHost {
        &self.0.plugin_host
    }
    fn instance(
        plugins: &Registry,
        root: &std::path::Path,
        binding: &Binding,
        handle: &Handle,
        config: ProviderConfig,
    ) -> std::result::Result<Option<Arc<DataSourceProvider>>, Refusal> {
        if binding.provider == BUNDLED {
            return Ok(None);
        }
        let entry = plugins
            .refresh()?
            .into_iter()
            .find(|entry| entry.candidate.id == binding.provider)
            .ok_or_else(|| {
                Refusal::new(
                    "provider_missing",
                    "Selected provider is no longer discovered; reset to the bundled provider.",
                )
            })?;
        if entry.candidate.sha256 != binding.sha256
            || entry.activation.as_ref().map(|a| a.generation) != binding.generation
        {
            return Err(Refusal::new(
                "provider_changed",
                "Selected provider authority changed; explicitly select the current checked pin or reset to bundled.",
            ));
        }
        plugins
            .data_source_with_config(&binding.provider, root, handle.clone(), config)
            .map(|source| Some(Arc::new(source)))
    }
    pub(crate) fn external(&self) -> bool {
        self.0
            .selected
            .lock()
            .map_or(true, |s| s.binding.provider != BUNDLED)
    }
    pub(crate) fn root_id(&self) -> &str {
        &self.0.id
    }
    pub(crate) fn close(&self) {
        self.0.plugin_host.close();
        let external = self
            .0
            .selected
            .lock()
            .ok()
            .and_then(|s| s.external.clone().ok().flatten());
        let retired = self
            .0
            .retired
            .lock()
            .map(|mut tasks| std::mem::take(&mut *tasks))
            .unwrap_or_default();
        if external.is_none() && retired.is_empty() {
            return;
        }
        bbt_rt::assert_not_ui_thread();
        self.0.handle.block_on(async {
            if let Some(source) = external {
                source.close().await;
            }
            for task in retired {
                let _ = task.await;
            }
        });
    }
    pub(crate) fn revision(&self) -> Result<u64> {
        Ok(self
            .0
            .selected
            .lock()
            .map_err(|_| invalid("knowledge state unavailable"))?
            .binding
            .revision)
    }
    pub(crate) fn current(&self, revision: u64) -> d::Result<()> {
        if self.revision().map_err(portable)? != revision {
            return Err(d::CommandError::new(
                "root_changed",
                "Knowledge provider changed while this read ran; retry the read.",
            ));
        }
        Ok(())
    }
    /// Pinned descriptor skills of every enabled, checked plugin this root's
    /// workspace discovers, selected or not. Never reads sidecar files; no
    /// provider process or document scan runs.
    pub(crate) fn plugin_skills(
        &self,
    ) -> std::result::Result<Vec<crate::plugins::EnabledSkills>, Refusal> {
        bbt_rt::assert_not_ui_thread();
        self.0.plugins.enabled_skills()
    }
    pub(crate) fn with_source<T>(
        &self,
        index: &Index,
        registry: &crate::registry::Registry,
        f: impl FnOnce(&dyn DataSource) -> d::Result<T>,
    ) -> d::Result<T> {
        self.with_source_at(index, registry, None, f)
    }
    fn with_source_at<T>(
        &self,
        index: &Index,
        registry: &crate::registry::Registry,
        expected: Option<u64>,
        f: impl FnOnce(&dyn DataSource) -> d::Result<T>,
    ) -> d::Result<T> {
        bbt_rt::assert_not_ui_thread();
        let (revision, external) = {
            let selected = self.0.selected.lock().map_err(portable)?;
            if expected.is_some_and(|revision| revision != selected.binding.revision) {
                return Err(d::CommandError::new(
                    "root_changed",
                    "Provider changed before this operation; refresh its source.",
                ));
            }
            (selected.binding.revision, selected.external.clone())
        };
        let external = external.map_err(|r| d::CommandError::new(r.code, r.message))?;
        let result = match external {
            Some(source) => f(source.as_ref()),
            None => f(&BboxySource::new(index).with_registry(registry)),
        };
        self.current(revision)?;
        result
    }
    pub(crate) fn seat_status(&self, index: &Index) -> d::Result<serde_json::Value> {
        let revision = self.revision().map_err(portable)?;
        let status = self.status(index).map_err(portable)?;
        // Compact observations only: never publish descriptor bodies or stderr.
        let plugins: Vec<_> =
            self.0
                .plugins
                .refresh()
                .map_err(portable)?
                .into_iter()
                .map(|entry| {
                    let checked = entry.conformance.as_ref().is_some_and(|c| {
                        c.ok && Some(&c.sha256) == entry.candidate.sha256.as_ref()
                    });
                    let refusal = entry
                        .candidate
                        .refusal
                        .as_ref()
                        .or_else(|| {
                            entry
                                .activation
                                .as_ref()?
                                .inspection
                                .as_ref()?
                                .refusal
                                .as_ref()
                        })
                        .or_else(|| entry.conformance.as_ref()?.refusal.as_ref());
                    serde_json::json!({
                        "id":entry.candidate.id,"sha256":entry.candidate.sha256,
                        "state":entry.state(),"refusal":refusal,"checked":checked,
                    })
                })
                .collect();
        self.current(revision)?;
        Ok(serde_json::json!({"status":status,"plugins":plugins}))
    }
    pub(crate) fn status(&self, index: &Index) -> Result<KnowledgeStatus> {
        let mut offered = vec![bundled()];
        for entry in self.0.plugins.refresh().map_err(portable)? {
            if entry.state() == State::Enabled
                && entry
                    .conformance
                    .as_ref()
                    .is_some_and(|c| c.ok && Some(&c.sha256) == entry.candidate.sha256.as_ref())
                && entry
                    .trust
                    .as_ref()
                    .and_then(|t| t.shape.as_ref())
                    .is_some_and(|shape| shape.systems.iter().any(|s| s.contract == "data_source"))
            {
                offered.push(KnowledgeProvider {
                    id: entry.candidate.id,
                    name: entry
                        .candidate
                        .name
                        .map(|n| format!("bboxy450-term-{}-{}", n.system.name(), n.provider))
                        .unwrap_or_default(),
                    sha256: entry.candidate.sha256,
                });
            }
        }
        let (binding, external) = {
            let selected = self
                .0
                .selected
                .lock()
                .map_err(|_| invalid("knowledge state unavailable"))?;
            (selected.binding.clone(), selected.external.clone())
        };
        let (records, process, refusal) = match external {
            Ok(None) => (Some(index.documents.len() as u64), None, None),
            Err(reason) => (
                None,
                None,
                Some(format!("{}: {}", reason.code, reason.message)),
            ),
            Ok(Some(source)) => {
                let observed = source.observation();
                let refusal = source
                    .check_authority()
                    .err()
                    .map(|e| e.to_string())
                    .or_else(|| observed.as_ref().and_then(|o| o.error.clone()));
                let process = if refusal.is_none() {
                    observed.and_then(|o| o.pid)
                } else {
                    None
                };
                (None, process, refusal)
            }
        };
        let provider = offered
            .iter()
            .find(|p| p.id == binding.provider)
            .cloned()
            .unwrap_or(KnowledgeProvider {
                id: binding.provider,
                name: "selected provider unavailable".into(),
                sha256: binding.sha256,
            });
        self.current(binding.revision)?;
        Ok(KnowledgeStatus {
            offered,
            roots: vec![KnowledgeRoot {
                id: self.0.id.clone(),
                path: self.0.root.to_string_lossy().into_owned(),
                provider,
                revision: binding.revision,
                records,
                process,
                refusal,
            }],
        })
    }
    fn select(&self, actor: &Actor, request: &KnowledgeRequest, _index: &Index) -> Result<()> {
        let (root, provider, sha256, revision, force) = match request {
            KnowledgeRequest::Select {
                root,
                provider,
                sha256,
                revision,
            } => (
                root.as_str(),
                provider.as_str(),
                sha256.clone(),
                *revision,
                false,
            ),
            KnowledgeRequest::Reset { root } => {
                (root.as_str(), BUNDLED, None, self.revision()?, true)
            }
            KnowledgeRequest::Status => return Ok(()),
        };
        if !matches!(actor, Actor::Operator{device_id,..} if !device_id.is_nil()) {
            return Err(invalid(
                "operator_required: root selection is operator-only",
            ));
        }
        if root != self.0.id {
            return Err(invalid(
                "unknown_knowledge_root: refresh the host's root list",
            ));
        }
        let entry = if provider == BUNDLED {
            None
        } else {
            Some(
                self.0
                    .plugins
                    .refresh()
                    .map_err(portable)?
                    .into_iter()
                    .find(|e| e.candidate.id == provider)
                    .ok_or_else(|| invalid("provider_missing"))?,
            )
        };
        let mut binding = Binding {
            root: self.0.root.to_string_lossy().into_owned(),
            provider: provider.into(),
            sha256,
            generation: entry
                .as_ref()
                .and_then(|e| e.activation.as_ref())
                .map(|a| a.generation),
            revision: revision
                .checked_add(1)
                .ok_or_else(|| invalid("knowledge revision exhausted"))?,
        };
        if provider == BUNDLED && binding.sha256.is_some() {
            return Err(invalid("bundled provider has no external pin"));
        }
        let external = Self::instance(
            &self.0.plugins,
            &self.0.root,
            &binding,
            &self.0.handle,
            self.0.config,
        )
        .map_err(portable)?;
        {
            let selected = self
                .0
                .selected
                .lock()
                .map_err(|_| invalid("knowledge state unavailable"))?;
            if !force
                && selected.binding.revision == revision
                && selected.binding.provider == binding.provider
                && selected.binding.sha256 == binding.sha256
                && selected.binding.generation == binding.generation
                && selected.external.is_ok()
            {
                return Ok(());
            }
        }
        if let Some(source) = &external {
            // An actual read must answer before persisting the new choice. No write or file move.
            let page = source.list(d::ListRequest {
                filter: d::Filter::default(),
                cursor: None,
                limit: Some(1),
            })?;
            if page.items.len() > 1 {
                return Err(invalid(
                    "invalid_provider_result: list ignored its one-record probe bound",
                ));
            }
            // No synthetic record in an empty root. Otherwise resolve an actual
            // known path; digests remain provider data, never a swap gate.
            if let Some(known) = page.items.first().map(|r| r.path.as_str()) {
                let reference = source.resolve(d::ResolveRequest {
                    locator: known.into(),
                })?;
                if reference.path != known {
                    return Err(invalid(
                        "invalid_provider_result: resolve returned a different path for the known record",
                    ));
                }
            }
        }
        let mut selected = self
            .0
            .selected
            .lock()
            .map_err(|_| invalid("knowledge state unavailable"))?;
        if !force && selected.binding.revision != revision {
            return Err(invalid("root_changed: refresh before selecting"));
        }
        let mut db = self.0.plugins.account().connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stored = load(&tx, &self.0.id);
        let stored_revision = match stored {
            Ok(binding) => binding.as_ref().map_or(0, |b| b.revision),
            Err(_) if force => selected.binding.revision,
            Err(error) => return Err(error),
        };
        if force {
            binding.revision = stored_revision
                .max(selected.binding.revision)
                .checked_add(1)
                .ok_or_else(|| invalid("knowledge revision exhausted"))?;
        } else if stored_revision != revision {
            return Err(invalid("root_changed: stored selection changed"));
        }
        if let Some(entry) = entry {
            let current: Vec<u8> = tx.query_row(
                "SELECT value FROM kv WHERE key=?1",
                [format!("plugins/activation/{provider}")],
                |row| row.get(0),
            )?;
            let current: Activation = serde_json::from_slice(&current)?;
            if entry.activation.as_ref() != Some(&current)
                || !current.desired_enabled
                || current.needs_retrust
            {
                return Err(invalid(
                    "provider_changed: activation changed during selection",
                ));
            }
            let checked: Vec<u8> = tx.query_row(
                "SELECT value FROM kv WHERE key=?1",
                [format!("plugins/conformance/{provider}")],
                |row| row.get(0),
            )?;
            let checked: Conformance = serde_json::from_slice(&checked)?;
            if !checked.ok || Some(&checked.sha256) != binding.sha256.as_ref() {
                return Err(invalid("provider_unchecked"));
            }
        }
        tx.execute(
            "INSERT INTO kv VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![
                format!("{PREFIX}{}", self.0.id),
                serde_json::to_vec(&binding)?
            ],
        )
        .map_err(|e| {
            invalid(format!(
                "unknown: knowledge selection store failed; retain operation ID: {e}"
            ))
        })?;
        tx.commit().map_err(|e| {
            invalid(format!(
                "unknown: knowledge selection commit failed; retain operation ID: {e}"
            ))
        })?;
        let old = std::mem::replace(
            &mut *selected,
            Selected {
                binding,
                external: Ok(external),
            },
        );
        drop(selected);
        // A root reset or provider swap starts every plugin connection afresh.
        self.0.plugin_host.retire_all();
        if let Ok(Some(old)) = old.external {
            let cleanup = self.0.handle.spawn(async move {
                old.close().await;
            });
            if let Ok(mut retired) = self.0.retired.lock() {
                retired.retain(|task| !task.is_finished());
                retired.push(cleanup);
            }
        }
        Ok(())
    }
    pub(crate) async fn request(
        &self,
        index: Arc<Index>,
        actor: Actor,
        journal: Journal,
        call: Call,
        request: KnowledgeRequest,
    ) -> Result<KnowledgeStatus> {
        let permit = self
            .0
            .controls
            .clone()
            .try_acquire_owned()
            .map_err(|_| invalid("knowledge_busy: retry with the same operation ID"))?;
        let host = self.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            if !matches!(&actor,Actor::Operator {device_id,..} if !device_id.is_nil()) {return Err(invalid("operator_required: knowledge controls are operator-only"));}
            let mutation = request.mutation();
            if mutation {
                let (retained,fresh) = journal.admit(&actor,&call).map_err(|e|invalid(format!("unknown: knowledge admission unavailable; retain operation ID: {e}")))?;
                if !fresh { return decode(retained); }
            }
            let result = (|| {
                if request.mutation() {host.select(&actor,&request,&index)?;}
                host.status(&index).map_err(|error|if mutation {invalid(format!("unknown: selection receipt unavailable; retain operation ID: {error}"))} else {error})
            })();
            let value = match result {
                Ok(status) => {
                    let value=mcp::success(call.op_id, serde_json::json!({"status":status}));
                    if serde_json::to_vec(&value)?.len()>d::MAX_PAGE_BYTES {mcp::error(call.op_id,if mutation {"unknown"} else {"knowledge_inventory_too_large"},"Root response exceeds the bound; retain the operation ID. Document reads remain available.")}
                    else {value}
                },
                Err(error) => {
                    let message=error.to_string();
                    mcp::error(call.op_id,if message.starts_with("unknown:") {"unknown"} else {"knowledge_refused"},message.strip_prefix("unknown: ").unwrap_or(&message))
                },
            };
            let recorded=if mutation {journal.complete(&actor,&call,value).map_err(|e|invalid(format!("unknown: knowledge receipt unavailable; retain operation ID: {e}")))} else {journal.checked(&actor,&call,value)}?;
            decode(recorded)
        }).await.map_err(|_| invalid("unknown: knowledge worker interrupted; retain operation ID"))?
    }
}
fn decode(value: serde_json::Value) -> Result<KnowledgeStatus> {
    if value["isError"] == true {
        let error = &value["structuredContent"]["error"];
        return Err(invalid(format!(
            "{}: {}",
            error["code"].as_str().unwrap_or("unknown"),
            error["message"]
                .as_str()
                .unwrap_or("knowledge result unavailable")
        )));
    }
    serde_json::from_value(value["structuredContent"]["status"].clone()).map_err(Into::into)
}
#[cfg(test)]
#[path = "roots_tests.rs"]
mod tests;
