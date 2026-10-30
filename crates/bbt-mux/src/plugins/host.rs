// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Live connections to every enabled, checked plugin of one knowledge root.
//!
//! A connection belongs to one (candidate, sha256, activation generation).
//! Every call rereads that authority before dispatch and again before its
//! result is returned; a changed authority closes the connection, so no
//! result is delivered from a connection whose generation is no longer
//! current. Disable, re-trust and re-check all advance the generation.
//! Modelled by `PluginConnection` in verification/contracts.toml.
//!
//! Hooks (`hooks.rs`) use a second connection per plugin, so a slow seat
//! call never delays a hook and a hook never takes a call's slot.
use super::*;
#[path = "hooks.rs"]
mod hooks;
#[path = "host_api.rs"]
mod host_api;
use crate::knowledge::data_source as d;
use crate::plugins::client::Shared;
use bbt_plugin::policy::RoleClass;
pub use hooks::{HOOK_QUEUE, HookEvent, HookOutcome, Verdict};
pub use host_api::RecordReader;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;
use tokio::{runtime::Handle, sync::Semaphore};

/// Parallel requests on one connection whose descriptor declares
/// `concurrent: true`. Other connections admit one request at a time.
pub const MAX_CONCURRENT_CALLS: usize = 4;

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

/// Request fields only the host supplies. A seat call that sets one is
/// refused, and a contract method whose request requires one stays with the
/// host (data_source write, the signed task_system transitions, repo_graph
/// describe).
pub const HOST_OWNED: &[&str] = &[
    "root",
    "signed_record",
    "author",
    "signer",
    "device",
    "account",
];
/// Contract methods that only read; every other seat tool is an effect.
const READS: &[&str] = &[
    "resolve",
    "fetch",
    "search",
    "list",
    "links",
    "graph",
    "status",
    "describe",
    "changes_since",
];

/// The role class a seat needs for a contract method, or `None` when the
/// method is never a seat tool: host-only contracts (event_sink),
/// data_source write/commit (the host owns signing) and any method whose
/// request requires a [`HOST_OWNED`] field.
pub fn seat_role(contract: Contract, method: &str) -> Option<RoleClass> {
    let role = contract.role(method).ok()?;
    if role == RoleClass::HostOnly
        || contract == Contract::DataSource && matches!(method, "write" | "commit")
    {
        return None;
    }
    let request = contract.method_schema(method, false)?;
    let body = request["$ref"]
        .as_str()
        .and_then(|reference| reference.strip_prefix("#/$defs/"))
        .map_or(&request, |name| &request["$defs"][name]);
    let host_field = body["required"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|field| field.as_str().is_some_and(|f| HOST_OWNED.contains(&f)));
    (!host_field).then_some(role)
}

#[derive(Debug, Clone)]
enum Target {
    Method(Contract, String),
    Declared(String),
}

/// One plugin-provided seat tool (rules in `mcp/plugin_tools.rs`).
#[derive(Debug, Clone)]
pub struct SeatTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    /// Oracle-class seats only.
    pub oracle: bool,
    /// A journaled read; otherwise an effect admitted once per op_id.
    pub read: bool,
    candidate: String,
    target: Target,
}

fn seat_tools(entry: &Entry, record: &TrustRecord) -> Vec<SeatTool> {
    let (Some(name), Some(shape)) = (&entry.candidate.name, &record.shape) else {
        return Vec::new();
    };
    let candidate = &entry.candidate.id;
    let mut tools = Vec::new();
    for system in &shape.systems {
        let Ok(contract) = Contract::named(&system.contract) else {
            continue;
        };
        for command in &system.commands {
            let (Some(role), Ok(seat)) = (
                seat_role(contract, &command.name),
                name.tool(contract, &command.name),
            ) else {
                continue;
            };
            tools.push(SeatTool {
                name: seat,
                description: format!(
                    "{} {} from the {} plugin.",
                    contract.name(),
                    command.name,
                    name.provider
                ),
                input_schema: command.args.clone(),
                output_schema: command.result.clone(),
                oracle: role == RoleClass::Oracle,
                read: READS.contains(&command.name.as_str()),
                candidate: candidate.clone(),
                target: Target::Method(contract, command.name.clone()),
            });
        }
    }
    for tool in &shape.tools {
        let Ok(seat) = tool.seat_name(name) else {
            continue;
        };
        tools.push(SeatTool {
            name: seat,
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
            output_schema: tool.output_schema.clone(),
            oracle: tool.role == RoleClass::Oracle,
            read: false,
            candidate: candidate.clone(),
            target: Target::Declared(tool.name.clone()),
        });
    }
    tools
}

/// The authority a connection was opened under.
#[derive(Clone, PartialEq)]
struct Authority {
    activation: Activation,
    record: TrustRecord,
}

/// Each plugin has up to one connection per lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Lane {
    /// Seat tools and contract calls; a busy connection refuses.
    Calls,
    /// Hook deliveries; one at a time, each waiting within its own budget.
    Hooks,
}

struct Connection {
    candidate: String,
    lane: Lane,
    authority: Authority,
    copy: PathBuf,
    directory: PathBuf,
    slots: Arc<Semaphore>,
    shared: tokio::sync::Mutex<Option<Arc<Shared>>>,
}

impl Connection {
    fn shape(&self) -> &bbt_plugin::shape::Shape {
        self.authority
            .record
            .shape
            .as_ref()
            .expect("an enabled plugin has a cached shape")
    }
    /// Spawn and initialize on first use; later callers reuse the live process.
    async fn open(&self, host: &Arc<Inner>, deadline: Instant) -> d::Result<Arc<Shared>> {
        let mut slot = tokio::time::timeout_at(deadline.into(), self.shared.lock())
            .await
            .map_err(|_| error("deadline_exceeded", "plugin connection is still starting"))?;
        if let Some(shared) = slot.as_ref().filter(|shared| shared.alive()) {
            return Ok(shared.clone());
        }
        if let Some(stale) = slot.take() {
            stale.shutdown().await;
        }
        let limit = host
            .config
            .timeout
            .min(deadline.saturating_duration_since(Instant::now()));
        if limit.is_zero() {
            return Err(error(
                "deadline_exceeded",
                "Operation deadline exhausted before initialization",
            ));
        }
        let mut client = Client::spawn_with_home(
            self.copy.clone(),
            &self.directory,
            false,
            limit,
            host.registry.private_home.as_deref(),
        )
        .map_err(refused)?;
        client.configure_deadlines(limit);
        let shape = self.shape();
        if let Err(reason) = client
            .initialize_shared(shape, &host.root, "bboxy450-term", shape.concurrent)
            .await
        {
            client.shutdown().await;
            return Err(refused(reason));
        }
        client.configure_deadlines(host.config.timeout);
        let handler = host_api::handler(
            Arc::downgrade(host),
            self.candidate.clone(),
            self.authority.clone(),
        );
        let shared = Arc::new(client.into_shared(Some(handler)));
        *slot = Some(shared.clone());
        Ok(shared)
    }
    async fn close(&self) {
        if let Some(shared) = self.shared.lock().await.take() {
            shared.shutdown().await;
        }
    }
}

#[derive(Default)]
struct Held {
    connections: BTreeMap<(String, Lane), Arc<Connection>>,
    closed: bool,
}

struct Inner {
    registry: Registry,
    root: PathBuf,
    handle: Handle,
    config: ProviderConfig,
    state: StdMutex<Held>,
    /// Seat tools of the enabled, checked plugins, as of the last refresh.
    tools: StdMutex<Arc<Vec<SeatTool>>>,
    /// Hook subscriptions, delivery queue and outcomes.
    hooks: hooks::Hooks,
    /// The root's records, read for the host API.
    records: StdMutex<Option<RecordReader>>,
}

/// One per knowledge root, owned by its workspace. Cloning shares it.
#[derive(Clone)]
pub struct PluginHost(Arc<Inner>);

impl PluginHost {
    /// `handle` must be a multi-threaded runtime; calls block on it from IO threads.
    pub fn new(
        registry: Registry,
        root: &Path,
        handle: Handle,
        config: ProviderConfig,
    ) -> Result<Self, Refusal> {
        let root = root
            .canonicalize()
            .map_err(|e| failure("knowledge_root_unavailable", e))?;
        if !root.is_dir() || root.to_str().is_none() {
            return Err(failure(
                "invalid_knowledge_root",
                "root must be a UTF-8 directory",
            ));
        }
        if config.timeout.is_zero() {
            return Err(failure(
                "invalid_provider_config",
                "host deadline must be positive",
            ));
        }
        let (hooks, queue) = hooks::Hooks::new();
        let host = Self(Arc::new(Inner {
            registry,
            root,
            handle,
            config,
            state: StdMutex::default(),
            tools: StdMutex::default(),
            hooks,
            records: StdMutex::default(),
        }));
        host.start_hook_worker(queue);
        host.refresh_tools();
        Ok(host)
    }

    /// Recompute the seat tool and hook snapshots from account records and
    /// pinned copies; no plugin process runs. Blocking IO.
    fn refresh_tools(&self) {
        let registry = &self.0.registry;
        let mut tools = Vec::new();
        let mut subscriptions = Vec::new();
        for entry in registry.refresh().unwrap_or_default() {
            if !enabled_and_checked(&entry) {
                continue;
            }
            let Ok((activation, record)) = registry.authority(&entry) else {
                continue;
            };
            if registry.verify_copy(record).is_err() {
                continue;
            }
            tools.extend(seat_tools(&entry, record));
            subscriptions.extend(hooks::subscriptions(
                &entry.candidate.id,
                activation.generation,
                record,
            ));
        }
        if let Ok(mut held) = self.0.tools.lock() {
            *held = Arc::new(tools);
        }
        self.set_subscriptions(subscriptions);
    }

    /// The seat tools as of the workspace mount or the last plugin control
    /// mutation. A call rechecks the plugin's current authority.
    pub fn seat_tools(&self) -> Arc<Vec<SeatTool>> {
        self.0
            .tools
            .lock()
            .map(|tools| tools.clone())
            .unwrap_or_default()
    }

    /// Call one seat tool from [`PluginHost::seat_tools`].
    pub fn call_seat_tool(
        &self,
        tool: &SeatTool,
        args: Value,
        deadline: Instant,
    ) -> d::Result<Value> {
        if let Some(field) = HOST_OWNED.iter().find(|field| args.get(**field).is_some()) {
            return Err(error(
                "host_field_in_arguments",
                format!("{field} belongs to the host, never to a seat"),
            ));
        }
        let name = match &tool.target {
            Target::Method(contract, method) => {
                return self.call(&tool.candidate, *contract, method, args, deadline);
            }
            Target::Declared(name) => name,
        };
        bbt_rt::assert_not_ui_thread();
        let connection = self.connection(&tool.candidate, Lane::Calls)?;
        let declared = connection
            .shape()
            .tools
            .iter()
            .find(|declared| {
                &declared.name == name && (declared.role == RoleClass::Oracle) == tool.oracle
            })
            .cloned()
            .ok_or_else(|| {
                error(
                    "unknown_tool",
                    "the trusted descriptor no longer declares this tool",
                )
            })?;
        self.dispatch(
            &tool.candidate,
            &connection,
            &declared.wire_name(),
            args,
            &[(&declared.input_schema, "invalid_request")],
            &[(&declared.output_schema, "invalid_provider_result")],
            declared.timeout_ms,
            deadline,
        )
    }

    /// Current authority of an enabled plugin whose check matches its pin.
    fn authorize(&self, candidate: &str) -> d::Result<(Authority, PathBuf)> {
        let registry = &self.0.registry;
        let entry = registry.entry(candidate).map_err(refused)?;
        let (activation, record) = registry.authority(&entry).map_err(refused)?;
        if entry.state() != State::Enabled {
            return Err(error("plugin_disabled", "explicit enable is required"));
        }
        if !entry
            .conformance
            .as_ref()
            .is_some_and(|result| result.ok && result.sha256 == activation.sha256)
        {
            return Err(error(
                "provider_unchecked",
                "run ctl plugin check before calling this plugin",
            ));
        }
        let copy = registry.verify_copy(record).map_err(refused)?;
        Ok((
            Authority {
                activation: activation.clone(),
                record: record.clone(),
            },
            copy,
        ))
    }

    /// The live connection on `lane` for the current authority, replacing a stale one.
    fn connection(&self, candidate: &str, lane: Lane) -> d::Result<Arc<Connection>> {
        let (authority, copy) = self.authorize(candidate)?;
        let mut state = self
            .0
            .state
            .lock()
            .map_err(|_| error("host_unavailable", "plugin host state"))?;
        if state.closed {
            return Err(error(
                "provider_closed",
                "this root's plugin host is closed",
            ));
        }
        let key = (candidate.to_owned(), lane);
        if let Some(current) = state.connections.get(&key)
            && current.authority == authority
        {
            return Ok(current.clone());
        }
        if let Some(old) = state.connections.remove(&key) {
            self.retire(old);
        }
        let parent = Path::new(&authority.record.data_dir).join("roots");
        private_dir(&parent).map_err(refused)?;
        let directory = parent.join(format!(
            "{:x}",
            Sha256::digest(self.0.root.as_os_str().as_encoded_bytes())
        ));
        private_dir(&directory).map_err(refused)?;
        let shape = authority
            .record
            .shape
            .as_ref()
            .ok_or_else(|| error("shape_not_cached", "inspect the plugin first"))?;
        let slots = if shape.concurrent && lane == Lane::Calls {
            MAX_CONCURRENT_CALLS
        } else {
            1
        };
        let connection = Arc::new(Connection {
            candidate: candidate.to_owned(),
            lane,
            authority,
            copy,
            directory,
            slots: Arc::new(Semaphore::new(slots)),
            shared: tokio::sync::Mutex::new(None),
        });
        state.connections.insert(key, connection.clone());
        Ok(connection)
    }

    fn retire(&self, connection: Arc<Connection>) {
        drop(self.0.handle.spawn(async move { connection.close().await }));
    }

    /// Remove `connection` if it is still the one held for `candidate`.
    fn drop_connection(&self, candidate: &str, connection: &Arc<Connection>) {
        let key = (candidate.to_owned(), connection.lane);
        let removed = self.0.state.lock().ok().and_then(|mut state| {
            state
                .connections
                .get(&key)
                .is_some_and(|held| Arc::ptr_eq(held, connection))
                .then(|| state.connections.remove(&key))
                .flatten()
        });
        if let Some(removed) = removed {
            self.retire(removed);
        }
    }

    /// Call one contract method. Arguments and results are checked against the
    /// contract schema and the plugin's declared command schema; the budget is
    /// the declared `timeout_ms`, cut to `deadline`.
    pub fn call(
        &self,
        candidate: &str,
        contract: Contract,
        method: &str,
        args: Value,
        deadline: Instant,
    ) -> d::Result<Value> {
        bbt_rt::assert_not_ui_thread();
        let wire = contract.wire_name(method).map_err(refused)?;
        let connection = self.connection(candidate, Lane::Calls)?;
        let command = connection
            .shape()
            .systems
            .iter()
            .find(|system| system.contract == contract.name())
            .and_then(|system| system.commands.iter().find(|c| c.name == method))
            .cloned()
            .ok_or_else(|| {
                error(
                    "unknown_method",
                    "method is absent from the trusted contract",
                )
            })?;
        let request = contract
            .method_schema(method, false)
            .ok_or_else(|| error("unknown_method", method))?;
        let result = contract
            .method_schema(method, true)
            .ok_or_else(|| error("unknown_method", method))?;
        self.dispatch(
            candidate,
            &connection,
            &wire,
            args,
            &[
                (&request, "invalid_request"),
                (&command.args, "argument_schema_mismatch"),
            ],
            &[
                (&result, "invalid_provider_result"),
                (&command.result, "result_schema_mismatch"),
            ],
            command.timeout_ms,
            deadline,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch(
        &self,
        candidate: &str,
        connection: &Arc<Connection>,
        wire: &str,
        args: Value,
        requests: &[(&Value, &str)],
        results: &[(&Value, &str)],
        timeout_ms: u64,
        deadline: Instant,
    ) -> d::Result<Value> {
        if serde_json::to_vec(&args)
            .map_err(|e| error("invalid_request", e.to_string()))?
            .len()
            > d::MAX_PAGE_BYTES
        {
            return Err(error(
                "request_too_large",
                "plugin request exceeds the portable page limit",
            ));
        }
        for (schema, code) in requests {
            validate(schema, &args, code)?;
        }
        let slots = connection.slots.clone();
        let _permit = match connection.lane {
            Lane::Calls => slots.try_acquire_owned().map_err(|_| {
                let mut reason = error(
                    "provider_busy",
                    "this plugin connection is serving its request limit",
                );
                reason.retry_after_ms = Some(100);
                reason
            })?,
            // A hook waits for the previous hook, never beyond its own budget.
            Lane::Hooks => self
                .0
                .handle
                .block_on(async {
                    tokio::time::timeout_at(deadline.into(), slots.acquire_owned()).await
                })
                .map_err(|_| error("deadline_exceeded", "an earlier hook held the hook lane"))?
                .map_err(|_| error("provider_closed", "the hook lane is closed"))?,
        };
        // A transport failure closes the connection; a plugin refusal does not.
        let outcome: Result<Value, (d::CommandError, bool)> = self.0.handle.block_on(async {
            let shared = connection
                .open(&self.0, deadline)
                .await
                .map_err(|e| (e, true))?;
            let budget = Duration::from_millis(timeout_ms)
                .min(deadline.saturating_duration_since(Instant::now()));
            if budget.is_zero() {
                return Err((
                    error(
                        "deadline_exceeded",
                        "Operation deadline exhausted before plugin dispatch",
                    ),
                    false,
                ));
            }
            let outer = shared
                .request("tools/call", json!({"name":wire,"arguments":args}), budget)
                .await
                .map_err(|reason| (refused(reason), true))?;
            let value = outer.get("structuredContent").cloned().ok_or_else(|| {
                (
                    error(
                        "invalid_provider_result",
                        "tool result has no structuredContent",
                    ),
                    true,
                )
            })?;
            if outer["isError"] == true {
                return Err((
                    serde_json::from_value::<d::CommandError>(value)
                        .map_err(|e| error("invalid_provider_result", e.to_string()))
                        .unwrap_or_else(|e| e),
                    false,
                ));
            }
            if serde_json::to_vec(&value).map_or(true, |bytes| bytes.len() > d::MAX_PAGE_BYTES) {
                return Err((
                    error(
                        "response_too_large",
                        "plugin result exceeds the portable page limit",
                    ),
                    true,
                ));
            }
            for (schema, code) in results {
                validate(schema, &value, code).map_err(|e| (e, true))?;
            }
            Ok(value)
        });
        // Recheck after the call: a result from a superseded generation is refused.
        let current = self
            .authorize(candidate)
            .map(|(authority, _)| authority == connection.authority);
        match (outcome, current) {
            (Ok(value), Ok(true)) => Ok(value),
            (Err((reason, false)), Ok(true)) => Err(reason),
            (Err((reason, true)), Ok(true)) => {
                self.drop_connection(candidate, connection);
                Err(reason)
            }
            (_, Ok(false)) => {
                self.drop_connection(candidate, connection);
                Err(error(
                    "provider_changed",
                    "plugin authority changed during the call; retry against the current plugin",
                ))
            }
            (_, Err(reason)) => {
                self.drop_connection(candidate, connection);
                Err(reason)
            }
        }
    }

    /// Close every connection whose authority is no longer current and
    /// refresh the seat tools. Run after any plugin control mutation;
    /// blocking registry IO, never the UI thread.
    pub fn reconcile(&self) {
        bbt_rt::assert_not_ui_thread();
        let held: Vec<_> = self
            .0
            .state
            .lock()
            .map(|state| {
                state
                    .connections
                    .iter()
                    .map(|((candidate, _), connection)| (candidate.clone(), connection.clone()))
                    .collect()
            })
            .unwrap_or_default();
        for (candidate, connection) in held {
            let current = self
                .authorize(&candidate)
                .is_ok_and(|(authority, _)| authority == connection.authority);
            if !current {
                self.drop_connection(&candidate, &connection);
            }
        }
        self.refresh_tools();
    }

    /// Held call connections as (candidate, activation generation), for tests and status.
    pub fn connected(&self) -> Vec<(String, u64)> {
        self.held(Lane::Calls)
    }

    /// Held hook connections as (candidate, activation generation).
    pub fn hook_connections(&self) -> Vec<(String, u64)> {
        self.held(Lane::Hooks)
    }

    fn held(&self, lane: Lane) -> Vec<(String, u64)> {
        self.0
            .state
            .lock()
            .map(|state| {
                state
                    .connections
                    .iter()
                    .filter(|((_, held_lane), _)| *held_lane == lane)
                    .map(|((candidate, _), held)| {
                        (candidate.clone(), held.authority.activation.generation)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Root reset: close every connection in background; later calls reopen.
    pub(crate) fn retire_all(&self) {
        let connections = self
            .0
            .state
            .lock()
            .map(|mut state| std::mem::take(&mut state.connections))
            .unwrap_or_default();
        for (_, connection) in connections {
            self.retire(connection);
        }
    }

    /// Workspace drop: refuse new calls and reap every process. Blocking IO;
    /// never call it from an async task or the UI thread.
    pub fn close(&self) {
        self.stop_hooks();
        if let Ok(mut records) = self.0.records.lock() {
            records.take();
        }
        let connections = self
            .0
            .state
            .lock()
            .map(|mut state| {
                state.closed = true;
                std::mem::take(&mut state.connections)
            })
            .unwrap_or_default();
        if connections.is_empty() {
            return;
        }
        bbt_rt::assert_not_ui_thread();
        self.0.handle.block_on(async {
            for (_, connection) in connections {
                connection.close().await;
            }
        });
    }
}
