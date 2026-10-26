//! Account-owned root selection. Slow provider IO is side work; reset is local.
use super::{BboxySource, DataSource, data_source as d};
use crate::{
    index::Index,
    mcp::{self, Actor, Call, journal::Journal},
    plugins::{DataSourceProvider, ProviderConfig, Registry},
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
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::{runtime::Handle, sync::Semaphore};

pub(crate) const BUNDLED: &str = "bboxy450-term-data_source-bboxy";
const PREFIX: &str = "knowledge/roots/";

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
}
#[derive(Clone)]
pub(crate) struct Host(Arc<Inner>);

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
        let id = hex::encode(Sha256::digest(path.as_bytes()));
        let default = Binding {
            root: path,
            provider: BUNDLED.into(),
            sha256: None,
            generation: None,
            revision: 0,
        };
        let stored = load(&plugins.account().connection()?, &id);
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
        Ok(Self(Arc::new(Inner {
            root,
            id,
            plugins,
            handle,
            config,
            selected: Mutex::new(Selected { binding, external }),
            retired: Mutex::new(Vec::new()),
            controls: Arc::new(Semaphore::new(8)),
        })))
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
    pub(crate) fn close(&self) {
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
    pub(crate) fn with_source<T>(
        &self,
        index: &Index,
        registry: &crate::registry::Registry,
        f: impl FnOnce(&dyn DataSource) -> d::Result<T>,
    ) -> d::Result<T> {
        bbt_rt::assert_not_ui_thread();
        let (revision, external) = {
            let selected = self.0.selected.lock().map_err(portable)?;
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
    fn summary(source: &dyn DataSource, timeout: std::time::Duration) -> d::Result<(u64, String)> {
        let started = Instant::now();
        let mut cursor = None;
        let mut seen = BTreeSet::new();
        let mut records = BTreeMap::new();
        loop {
            let page = source.list(d::ListRequest {
                filter: d::Filter::default(),
                cursor,
                limit: Some(d::MAX_PAGE),
            })?;
            if page.items.len() > d::MAX_PAGE {
                return Err(d::CommandError::new(
                    "invalid_provider_result",
                    "Root page exceeds 128 records.",
                ));
            }
            for record in page.items {
                if records.insert(record.path, record.digest).is_some() {
                    return Err(d::CommandError::new(
                        "invalid_provider_result",
                        "Root listing repeated a path.",
                    ));
                }
            }
            cursor = page.next;
            let Some(next) = &cursor else { break };
            if !seen.insert(next.clone()) {
                return Err(d::CommandError::new(
                    "invalid_cursor",
                    "Root listing repeated a cursor.",
                ));
            }
            if started.elapsed() >= timeout || records.len() >= 16384 {
                return Err(d::CommandError::new(
                    "root_summary_unavailable",
                    "Root listing exceeded the bounded summary; document reads remain available.",
                ));
            }
        }
        let mut digest = Sha256::new();
        for (path, hash) in &records {
            digest.update((path.len() as u64).to_be_bytes());
            digest.update(path);
            digest.update(hash);
        }
        Ok((records.len() as u64, hex::encode(digest.finalize())))
    }
    pub(crate) fn status(
        &self,
        index: &Index,
        registry: &crate::registry::Registry,
    ) -> Result<KnowledgeStatus> {
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
        let summary = self.with_source(index, registry, |source| {
            Self::summary(source, self.0.config.timeout)
        });
        let process = summary.as_ref().ok().and_then(|_| {
            external
                .ok()
                .flatten()
                .and_then(|p| p.observation())
                .and_then(|o| o.pid)
        });
        let provider = offered
            .iter()
            .find(|p| p.id == binding.provider)
            .cloned()
            .unwrap_or(KnowledgeProvider {
                id: binding.provider,
                name: "selected provider unavailable".into(),
                sha256: binding.sha256,
            });
        let (records, digest, refusal) = match summary {
            Ok((count, digest)) => (Some(count), Some(digest), None),
            Err(reason) => (None, None, Some(reason.to_string())),
        };
        self.current(binding.revision)?;
        Ok(KnowledgeStatus {
            offered,
            roots: vec![KnowledgeRoot {
                id: self.0.id.clone(),
                path: self.0.root.to_string_lossy().into_owned(),
                provider,
                revision: binding.revision,
                records,
                digest,
                process,
                refusal,
            }],
        })
    }
    fn select(
        &self,
        actor: &Actor,
        root: &str,
        provider: &str,
        sha256: Option<String>,
        revision: u64,
    ) -> Result<()> {
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
        let binding = Binding {
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
            if selected.binding.revision == revision
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
            source.list(d::ListRequest {
                filter: d::Filter::default(),
                cursor: None,
                limit: Some(1),
            })?;
        }
        let mut selected = self
            .0
            .selected
            .lock()
            .map_err(|_| invalid("knowledge state unavailable"))?;
        if selected.binding.revision != revision {
            return Err(invalid("root_changed: refresh before selecting"));
        }
        let mut db = self.0.plugins.account().connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stored = load(&tx, &self.0.id);
        let stored_revision = match stored {
            Ok(_) if provider == BUNDLED && selected.binding.provider == "unavailable" => 0,
            Ok(binding) => binding.as_ref().map_or(0, |b| b.revision),
            Err(_) if provider == BUNDLED && selected.binding.provider == "unavailable" => 0,
            Err(error) => return Err(error),
        };
        if stored_revision != revision {
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
        )?;
        tx.commit()?;
        let old = std::mem::replace(
            &mut *selected,
            Selected {
                binding,
                external: Ok(external),
            },
        );
        drop(selected);
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
        registry: crate::registry::Registry,
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
                let (retained,fresh) = journal.admit(&actor,&call)?;
                if !fresh { return decode(retained); }
            }
            let result = (|| {
                match request {
                    KnowledgeRequest::Select { root, provider, revision, sha256 } => host.select(&actor,&root,&provider,sha256,revision)?,
                    KnowledgeRequest::Reset { root } => host.select(&actor,&root,BUNDLED,None,host.revision()?)?,
                    KnowledgeRequest::Status => {},
                }
                host.status(&index,&registry)
            })();
            let value = match result {
                Ok(status) => {
                    let value=mcp::success(call.op_id, serde_json::json!({"status":status}));
                    if serde_json::to_vec(&value)?.len()>d::MAX_PAGE_BYTES {mcp::error(call.op_id,"knowledge_inventory_too_large","Root controls exceed the bounded response; document reads remain available.")}
                    else {value}
                },
                Err(error) => mcp::error(call.op_id,"knowledge_refused",&error.to_string()),
            };
            decode(if mutation { journal.complete(&actor,&call,value)? } else { journal.checked(&actor,&call,value)? })
        }).await.map_err(|_| invalid("knowledge outcome unknown; retain operation ID"))?
    }
}
fn decode(value: serde_json::Value) -> Result<KnowledgeStatus> {
    if value["isError"] == true {
        return Err(invalid(
            value["structuredContent"]["error"]["message"]
                .as_str()
                .unwrap_or("knowledge refused"),
        ));
    }
    serde_json::from_value(value["structuredContent"]["status"].clone()).map_err(Into::into)
}
#[cfg(test)]
#[path = "roots_tests.rs"]
mod tests;
