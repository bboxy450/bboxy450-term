// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! IO-only data_source adapter. Root selection/admission belongs to the host;
//! this connection never receives an author, a signing key or a per-call root.
use super::*;
use crate::knowledge::{DataSource, WriteContext, data_source as d};
use serde_json::{Value, json};
use std::sync::{
    Mutex as ObservationLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;
use tokio::{runtime::Handle, sync::Mutex};

#[derive(Debug, Clone, Copy)]
pub struct ProviderConfig {
    /// Ordinary calls and production initialization; optional commit has its own 15 s ceiling.
    pub timeout: Duration,
}
impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
        }
    }
}

/// Last actual response/attempt, not a health claim or a liveness lease.
#[derive(Debug, Clone, Default)]
pub struct ProviderObservation {
    pub pid: Option<u32>,
    pub method: Option<String>,
    pub error: Option<String>,
}

pub struct DataSourceProvider {
    registry: Registry,
    activation: Activation,
    record: TrustRecord,
    root: PathBuf,
    directory: PathBuf,
    handle: Handle,
    config: ProviderConfig,
    client: Mutex<Option<Client>>,
    observed: ObservationLock<ProviderObservation>,
    closed: AtomicBool,
}

fn error(code: &str, message: impl Into<String>) -> d::CommandError {
    d::CommandError::new(code, message)
}
fn refused(reason: Refusal) -> d::CommandError {
    error(&reason.code, reason.message)
}
fn validate(schema: &Value, value: &Value, code: &str) -> d::Result<()> {
    jsonschema::validator_for(schema)
        .map_err(|reason| error("invalid_json_schema", reason.to_string()))?
        .validate(value)
        .map_err(|reason| error(code, reason.to_string()))
}

impl Registry {
    pub fn data_source(
        &self,
        candidate: &str,
        root: &Path,
        handle: Handle,
    ) -> Result<DataSourceProvider, Refusal> {
        self.data_source_with_config(candidate, root, handle, ProviderConfig::default())
    }

    /// Host configuration only; JSON callers cannot widen these deadlines.
    pub fn data_source_with_config(
        &self,
        candidate: &str,
        root: &Path,
        handle: Handle,
        config: ProviderConfig,
    ) -> Result<DataSourceProvider, Refusal> {
        bbt_rt::assert_not_ui_thread();
        let entry = self.entry(candidate)?;
        let (activation, record) = self.authority(&entry)?;
        if entry.state() != State::Enabled {
            return Err(failure("plugin_disabled", "explicit enable is required"));
        }
        if !entry
            .conformance
            .as_ref()
            .is_some_and(|result| result.ok && result.sha256 == activation.sha256)
        {
            return Err(failure(
                "provider_unchecked",
                "run ctl plugin check before selecting a provider",
            ));
        }
        if !record.shape.as_ref().is_some_and(|shape| {
            shape
                .systems
                .iter()
                .any(|system| system.contract == "data_source")
        }) {
            return Err(failure(
                "wrong_plugin_contract",
                "knowledge roots require data_source",
            ));
        }
        self.verify_copy(record)?;
        let root = root
            .canonicalize()
            .map_err(|e| failure("knowledge_root_unavailable", e))?;
        if config.timeout.is_zero() {
            return Err(failure(
                "invalid_provider_config",
                "host deadline must be positive",
            ));
        }
        if !root.is_dir() || root.to_str().is_none() {
            return Err(failure(
                "invalid_knowledge_root",
                "root must be a UTF-8 directory",
            ));
        }
        let parent = Path::new(&record.data_dir).join("roots");
        private_dir(&parent)?;
        let directory = parent.join(format!(
            "{:x}",
            Sha256::digest(root.as_os_str().as_encoded_bytes())
        ));
        private_dir(&directory)?;
        Ok(DataSourceProvider {
            registry: self.clone(),
            activation: activation.clone(),
            record: record.clone(),
            root,
            directory,
            handle,
            config,
            client: Mutex::new(None),
            observed: ObservationLock::new(ProviderObservation::default()),
            closed: AtomicBool::new(false),
        })
    }
}

impl DataSourceProvider {
    pub fn candidate(&self) -> &str {
        &self.activation.candidate
    }
    pub fn digest(&self) -> &str {
        &self.activation.sha256
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn data_dir(&self) -> &Path {
        &self.directory
    }
    pub fn observation(&self) -> Option<ProviderObservation> {
        self.observed.try_lock().ok().map(|value| value.clone())
    }

    pub(crate) fn check_authority(&self) -> d::Result<()> {
        self.current().map(|_| ())
    }
    fn current(&self) -> d::Result<PathBuf> {
        if self.closed.load(Ordering::Acquire) {
            return Err(error("provider_closed", "this root instance is closed"));
        }
        let entry = self
            .registry
            .entry(&self.activation.candidate)
            .map_err(refused)?;
        let (activation, record) = self.registry.authority(&entry).map_err(refused)?;
        if activation != &self.activation
            || record != &self.record
            || entry.state() != State::Enabled
        {
            return Err(error(
                "provider_changed",
                "provider authority changed; select the current checked provider",
            ));
        }
        if !entry
            .conformance
            .as_ref()
            .is_some_and(|result| result.ok && result.sha256 == self.activation.sha256)
        {
            return Err(error(
                "provider_unchecked",
                "provider conformance is no longer current",
            ));
        }
        self.registry.verify_copy(record).map_err(refused)
    }

    fn observe(&self, pid: Option<u32>, method: &str, result: &d::Result<Value>) {
        if let Ok(mut observed) = self.observed.lock() {
            *observed = ProviderObservation {
                pid,
                method: Some(method.into()),
                error: result.as_ref().err().map(|reason| {
                    format!(
                        "{}: {}",
                        reason.code,
                        reason.message.chars().take(2048).collect::<String>()
                    )
                }),
            };
        }
    }

    fn request<T: DeserializeOwned>(&self, method: &str, request: impl Serialize) -> d::Result<T> {
        self.request_before(method, request, None)
    }
    fn fetch_before(
        &self,
        request: d::FetchRequest,
        deadline: Option<Instant>,
    ) -> d::Result<d::Record> {
        let record: d::Record = self.request_before("fetch", request, deadline)?;
        for link in &record.resources {
            crate::knowledge::BboxySource::validate_resource(self.data_dir(), link)?;
        }
        Ok(record)
    }
    /// One provider call, measured host-side like every plugin call.
    fn request_before<T: DeserializeOwned>(
        &self,
        method: &str,
        request: impl Serialize,
        deadline: Option<Instant>,
    ) -> d::Result<T> {
        let mut trace = super::measure::Trace::start(&format!("data_source_{method}"), None);
        let mut refused_by_plugin = false;
        let result = self.request_traced(
            method,
            request,
            deadline,
            &mut trace,
            &mut refused_by_plugin,
        );
        let (outcome, code) = super::measure::outcome(&result, refused_by_plugin);
        trace.finish(
            self.registry.measurements(),
            &self.activation.candidate,
            outcome,
            code,
        );
        result.and_then(|value| {
            serde_json::from_value(value)
                .map_err(|e| error("invalid_provider_result", e.to_string()))
        })
    }

    fn request_traced(
        &self,
        method: &str,
        request: impl Serialize,
        deadline: Option<Instant>,
        trace: &mut super::measure::Trace,
        refused_by_plugin: &mut bool,
    ) -> d::Result<Value> {
        bbt_rt::assert_not_ui_thread();
        let mut args =
            serde_json::to_value(request).map_err(|e| error("invalid_request", e.to_string()))?;
        // Optional Rust fields serialize as null; the portable request schemas
        // describe absence. Required nullable ref.number is deliberately kept.
        if let Some(object) = args.as_object_mut() {
            for key in ["cursor", "limit", "ref", "path", "etag"] {
                if object.get(key).is_some_and(Value::is_null) {
                    object.remove(key);
                }
            }
            if let Some(filter) = object.get_mut("filter").and_then(Value::as_object_mut) {
                for key in ["type", "status", "assignee", "since", "swarm"] {
                    if filter.get(key).is_some_and(Value::is_null) {
                        filter.remove(key);
                    }
                }
            }
        }
        if serde_json::to_vec(&args)
            .map_err(|e| error("invalid_request", e.to_string()))?
            .len()
            > d::MAX_PAGE_BYTES
        {
            return Err(error(
                "request_too_large",
                "provider request exceeds the portable page limit",
            ));
        }
        let contract = Contract::DataSource;
        let shape = self
            .record
            .shape
            .as_ref()
            .expect("constructor checked the descriptor");
        let command = shape
            .systems
            .iter()
            .find(|system| system.contract == contract.name())
            .and_then(|system| {
                system
                    .commands
                    .iter()
                    .find(|command| command.name == method)
            })
            .ok_or_else(|| {
                error(
                    "unknown_method",
                    "method is absent from the trusted contract",
                )
            })?;
        validate(
            &contract
                .method_schema(method, false)
                .ok_or_else(|| error("unknown_method", method))?,
            &args,
            "invalid_request",
        )?;
        validate(&command.args, &args, "argument_schema_mismatch")?;
        trace.mark("validate");
        let mut slot = self.client.try_lock().map_err(|_| {
            let mut reason = error(
                "provider_busy",
                "one read is already using this root connection",
            );
            reason.retry_after_ms = Some(100);
            reason
        })?;
        let copy = match self.current() {
            Ok(copy) => copy,
            Err(reason) => {
                if let Some(client) = slot.take() {
                    self.handle.block_on(client.shutdown());
                }
                self.observe(None, method, &Err(reason.clone()));
                return Err(reason);
            }
        };
        trace.mark("authority");
        let result = self.handle.block_on(async {
            if slot.is_none() {
                let initialize_limit = deadline
                    .map(|d| {
                        self.config
                            .timeout
                            .min(d.saturating_duration_since(Instant::now()))
                    })
                    .unwrap_or(self.config.timeout);
                if initialize_limit.is_zero() {
                    return Err(error(
                        "deadline_exceeded",
                        "Operation deadline exhausted before initialization",
                    ));
                }
                let mut client = Client::spawn_with_home(
                    copy,
                    self.data_dir(),
                    false,
                    initialize_limit,
                    self.registry.private_home.as_deref(),
                )
                .map_err(refused)?;
                client.configure_deadlines(initialize_limit);
                if let Err(reason) = client
                    .initialize_root(shape, Some(&self.root), "bboxy450-term")
                    .await
                {
                    client.shutdown().await;
                    return Err(refused(reason));
                }
                client.configure_deadlines(self.config.timeout);
                *slot = Some(client);
                trace.mark("connect");
            }
            let client = slot.as_mut().expect("connection initialized");
            let ceiling = if method == "commit" {
                Duration::from_secs(15)
            } else {
                self.config.timeout
            };
            let budget = Duration::from_millis(command.timeout_ms).min(ceiling);
            let budget = deadline
                .map(|d| budget.min(d.saturating_duration_since(Instant::now())))
                .unwrap_or(budget);
            if budget.is_zero() {
                return Err(error(
                    "deadline_exceeded",
                    "Operation deadline exhausted before provider dispatch",
                ));
            }
            let name = contract.wire_name(method).map_err(refused)?;
            let outer = client
                .request("tools/call", json!({"name":name,"arguments":args}), budget)
                .await
                .map_err(refused)?;
            trace.mark("plugin");
            // Only a shape that declares trace was asked for spans.
            if shape.trace {
                trace.plugin_spans(&outer);
            }
            let value = outer.get("structuredContent").cloned().ok_or_else(|| {
                error(
                    "invalid_provider_result",
                    "tool result has no structuredContent",
                )
            })?;
            if outer["isError"] == true {
                *refused_by_plugin = true;
                return Err(serde_json::from_value::<d::CommandError>(value)
                    .map_err(|e| error("invalid_provider_result", e.to_string()))?);
            }
            validate(
                &contract.method_schema(method, true).expect("known method"),
                &value,
                "invalid_provider_result",
            )?;
            validate(&command.result, &value, "result_schema_mismatch")?;
            trace.mark("result");
            Ok(value)
        });
        let pid = slot.as_ref().map(|client| client.pid);
        let result = if serde_json::to_vec(&result)
            .map_err(|e| error("invalid_provider_result", e.to_string()))?
            .len()
            > d::MAX_PAGE_BYTES
        {
            Err(error(
                "response_too_large",
                "provider result exceeds the portable page limit",
            ))
        } else {
            result
        };
        let result = result.and_then(|value| self.current().map(|_| value));
        if result.is_err()
            && let Some(client) = slot.take()
        {
            self.handle.block_on(client.shutdown());
        }
        self.observe(pid, method, &result);
        trace.mark("recheck");
        result
    }

    /// The owner may close in background when a root is reset; callers never retry effects.
    pub async fn close(&self) {
        self.closed.store(true, Ordering::Release);
        let client = self.client.lock().await.take();
        if let Some(client) = client {
            client.shutdown().await;
        }
    }
}

impl Drop for DataSourceProvider {
    fn drop(&mut self) {
        if let Some(client) = self.client.get_mut().take() {
            drop(self.handle.spawn(async move {
                client.shutdown().await;
            }));
        }
    }
}

impl DataSource for DataSourceProvider {
    fn commit(
        &self,
        request: d::CommitRequest,
        _context: &WriteContext<'_>,
    ) -> d::Result<d::CommitResult> {
        let deadline = Instant::now() + Duration::from_secs(15);
        crate::knowledge::commit::validate(&request)?;
        if !self.record.shape.as_ref().is_some_and(|shape| {
            shape.systems.iter().any(|s| {
                s.contract == "data_source" && s.commands.iter().any(|c| c.name == "commit")
            })
        }) {
            return Err(error(
                "unsupported_operation",
                "Selected provider does not support explicit document commits.",
            ));
        }
        for reference in &request.refs {
            if self
                .fetch_before(
                    d::FetchRequest {
                        reference: reference.clone(),
                    },
                    Some(deadline),
                )?
                .reference
                != *reference
            {
                return Err(error("stale_reference", "commit source changed"));
            }
        }
        let result: d::CommitResult = self
            .request_before("commit", &request, Some(deadline))
            .map_err(|e| {
                error(
                    "commit_not_confirmed",
                    format!("Commit dispatched; {e}. Inspect original operation before retry."),
                )
            })?;
        if result.committed != request.refs
            || result.commit.trim().is_empty()
            || result.commit.len() > 256
            || result.commit.chars().any(char::is_control)
        {
            return Err(error(
                "commit_not_confirmed",
                "provider committed references differ from the requested records",
            ));
        }
        Ok(result)
    }
    fn resolve(&self, request: d::ResolveRequest) -> d::Result<d::RecordRef> {
        self.request("resolve", request)
    }
    fn fetch(&self, request: d::FetchRequest) -> d::Result<d::Record> {
        self.fetch_before(request, None)
    }
    fn search(&self, request: d::SearchRequest) -> d::Result<d::Page<d::SearchHit>> {
        self.request("search", request)
    }
    fn list(&self, request: d::ListRequest) -> d::Result<d::Page<d::RecordRef>> {
        self.request("list", request)
    }
    fn links(&self, request: d::FetchRequest) -> d::Result<d::Links> {
        self.request("links", request)
    }
    fn graph(&self, request: d::GraphRequest) -> d::Result<d::GraphPage> {
        self.request("graph", request)
    }
    fn graph_until(&self, request: d::GraphRequest, deadline: Instant) -> d::Result<d::GraphPage> {
        self.request_before("graph", request, Some(deadline))
    }
    fn list_until(
        &self,
        request: d::ListRequest,
        deadline: Instant,
    ) -> d::Result<d::Page<d::RecordRef>> {
        self.request_before("list", request, Some(deadline))
    }
    fn fetch_until(&self, request: d::FetchRequest, deadline: Instant) -> d::Result<d::Record> {
        self.fetch_before(request, Some(deadline))
    }
    fn changes_since(&self, request: d::ChangesRequest) -> d::Result<d::Changes> {
        self.request("changes_since", request)
    }
    fn write(
        &self,
        mut request: d::WriteRequest,
        context: &WriteContext<'_>,
    ) -> d::Result<d::WriteResult> {
        if !self
            .record
            .shape
            .as_ref()
            .is_some_and(|shape| shape.signed_writer)
        {
            return Err(error(
                "signed_writer_unavailable",
                "Selected provider does not support host-signed writes. Select bboxy450-term-data_source-bboxy in Settings > Knowledge to write locally.",
            ));
        }
        let path = request
            .reference
            .as_ref()
            .map(|r| r.path.as_str())
            .or(request.path.as_deref())
            .ok_or_else(|| {
                error(
                    "path_required",
                    "create requires a root-relative Markdown path",
                )
            })?;
        if Path::new(path).is_absolute()
            || Path::new(path).extension().is_none_or(|e| e != "md")
            || Path::new(path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(error(
                "invalid_path",
                "write must stay inside the selected root",
            ));
        }
        if request.reference.is_some() && request.path.is_some() {
            return Err(error("invalid_path", "update uses ref without path"));
        }
        if let Some(reference) = request.reference.clone() {
            let current = self.fetch(d::FetchRequest {
                reference: reference.clone(),
            })?;
            if current.reference != reference
                || request
                    .etag
                    .as_ref()
                    .is_some_and(|etag| etag != &current.etag)
            {
                return Err(error("etag_mismatch", "record changed before signing"));
            }
            request.etag = Some(current.etag);
        } else if request.etag.is_some() {
            return Err(error("etag_mismatch", "create cannot supply an etag"));
        }
        let front = request
            .frontmatter
            .as_object_mut()
            .ok_or_else(|| error("invalid_frontmatter", "frontmatter must be an object"))?;
        // Proof and native projection fields are derived. Never recursively sign an older proof.
        for field in [
            "bboxy_signed_record",
            "bboxy_status",
            "gitkb",
            "provenance",
            "authorship",
            "created",
            "updated",
        ] {
            front.remove(field);
        }
        front.insert("authorship".into(), context.author.public.clone());
        front.insert("updated_by".into(), json!(context.author.id));
        if request.reference.is_none() {
            front.insert("created_by".into(), json!(context.author.id));
        }
        let signed_record = super::super::signed::sign(&request, context)?;
        let mut args =
            serde_json::to_value(&request).map_err(|e| error("invalid_request", e.to_string()))?;
        args["signed_record"] = json!(signed_record);
        let value: Value = self.request("write", args).map_err(|reason| {
            error(
                "write_not_confirmed",
                format!(
                    "Signed write dispatched; {reason}. Inspect original operation before retry."
                ),
            )
        })?;
        // An invalid response after dispatch is unknown, never a safe retry.
        let check = || -> d::Result<d::WriteResult> {
            if value["signed_record"].as_str() != Some(&signed_record) {
                return Err(error(
                    "signed_record_mismatch",
                    "provider did not echo the host proof",
                ));
            }
            super::super::signed::verify(&signed_record, &request, context)?;
            let result: d::WriteResult = serde_json::from_value(value.clone())
                .map_err(|e| error("invalid_provider_result", e.to_string()))?;
            super::super::signed::matches(&request, &result.record)?;
            if result.lossy {
                return Err(error(
                    "lossy_write",
                    "provider reported a lossy signed write",
                ));
            }
            let fetched: Value = self.request(
                "fetch",
                d::FetchRequest {
                    reference: result.record.reference.clone(),
                },
            )?;
            if fetched["signed_record"].as_str() != Some(&signed_record)
                && fetched["frontmatter"]["bboxy_signed_record"].as_str() != Some(&signed_record)
            {
                return Err(error(
                    "signed_record_mismatch",
                    "stored proof differs from the admitted write",
                ));
            }
            let stored: d::Record = serde_json::from_value(fetched)
                .map_err(|e| error("invalid_provider_result", e.to_string()))?;
            super::super::signed::matches(&request, &stored)?;
            if stored != result.record {
                return Err(error(
                    "write_changed",
                    "stored readback differs from write result",
                ));
            }
            for link in &stored.resources {
                crate::knowledge::BboxySource::validate_resource(self.data_dir(), link)?;
            }
            Ok(result)
        };
        check().map_err(|reason| {
            error(
                "write_not_confirmed",
                format!(
                    "Signed write dispatched; {}. Inspect original operation before retry.",
                    reason
                ),
            )
        })
    }
}
