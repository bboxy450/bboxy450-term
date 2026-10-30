// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Build bboxy450-term plugins in Rust. A plugin implements [`Plugin`] and
//! calls [`run`]; the SDK owns the descriptor, the stdio MCP runtime,
//! cancellation, deadlines, limits and refusal encoding. Contract types and
//! schemas come from `bboxy450-plugin`, so the host and plugins share one
//! definition of every contract.

// A plugin is its own process with no UI thread; blocking stdio is its job.
#![allow(clippy::disallowed_methods)]

mod runtime;
pub mod testkit;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub use bbt_plugin::contracts::Contract;
pub use bbt_plugin::policy::{Hook, Phase, RoleClass};
pub use bbt_plugin::shape::{Command, Shape, System, Tool};
pub use bbt_plugin::skills::Skill;
pub use runtime::{run, serve};
use serde_json::{Map, Value, json};

/// Executable-name prefix the host discovers: `bboxy450-term-<contract>-<provider>`.
pub const APP: &str = "bboxy450-term";
/// The descriptor version this SDK emits.
pub const SHAPE_VERSION: &str = "1.0";
/// The MCP protocol revision answered by `initialize`.
pub const PROTOCOL_VERSION: &str = "2025-06-18";
/// Largest accepted request line.
pub const MAX_FRAME: usize = 8 * 1024 * 1024;
/// Largest serialized tool result; the host refuses larger pages.
pub const MAX_RESULT: usize = 256 * 1024;
/// Requests admitted at once, including the running one.
pub const MAX_PENDING: usize = 8;
/// Default per-command budget declared to the host.
pub const DEFAULT_TIMEOUT_MS: u64 = 2_000;
/// Tool-argument fields that belong to the host, never to a caller.
pub const HOST_FIELDS: &[&str] = &["root", "author", "signer", "device", "account"];

/// A named, machine-readable refusal. Extra fields are carried verbatim in
/// the structured result, for example `candidates` on an ambiguous locator.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub code: String,
    pub message: String,
    pub fields: Map<String, Value>,
}

impl Refusal {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            fields: Map::new(),
        }
    }

    /// Attach one additional structured field.
    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// The structured form sent to the host.
    pub fn value(&self) -> Value {
        let mut value = self.fields.clone();
        value.insert("code".into(), self.code.clone().into());
        value.insert("message".into(), self.message.clone().into());
        Value::Object(value)
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Refusal {}

impl From<bbt_plugin::Refusal> for Refusal {
    fn from(refusal: bbt_plugin::Refusal) -> Self {
        Self::new(refusal.code, refusal.message)
    }
}

impl From<serde_json::Error> for Refusal {
    fn from(error: serde_json::Error) -> Self {
        Self::new("invalid_arguments", error.to_string())
    }
}

/// Per-request cancellation and deadline. Long work checks it between steps.
#[derive(Debug, Clone)]
pub struct Ctx {
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
    link: Option<Arc<runtime::Link>>,
}

impl Ctx {
    pub fn new(cancelled: Arc<AtomicBool>, budget: Duration) -> Self {
        Self {
            cancelled,
            deadline: Instant::now() + budget,
            link: None,
        }
    }

    pub(crate) fn with_link(mut self, link: Arc<runtime::Link>) -> Self {
        self.link = Some(link);
        self
    }

    /// The host API, bound to this request's deadline. Outside a served
    /// request (a detached context) every call refuses `host_unavailable`.
    pub fn host(&self) -> Host<'_> {
        Host { ctx: self }
    }

    /// A context that is never cancelled, for background work and tests.
    pub fn detached(budget: Duration) -> Self {
        Self::new(Arc::default(), budget)
    }

    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Refuse with `cancelled` or `deadline_exceeded` once either applies.
    pub fn check(&self) -> Result<(), Refusal> {
        if self.is_cancelled() {
            Err(Refusal::new("cancelled", "request cancelled"))
        } else if Instant::now() >= self.deadline {
            Err(Refusal::new(
                "deadline_exceeded",
                "the call exhausted its declared budget",
            ))
        } else {
            Ok(())
        }
    }
}

/// Requests to the host over the plugin's own stdio (`host/<method>`), each
/// answered while the plugin is still serving its current call. A method
/// must be requested in [`Manifest::host_api`] and granted by the operator at
/// trust; otherwise it refuses `host_api_not_granted`.
#[derive(Debug, Clone, Copy)]
pub struct Host<'a> {
    ctx: &'a Ctx,
}

impl Host<'_> {
    /// One host request within the context's deadline.
    pub fn request(&self, method: &str, params: Value) -> Result<Value, Refusal> {
        self.ctx.check()?;
        let link = self.ctx.link.as_ref().ok_or_else(|| {
            Refusal::new("host_unavailable", "no host connection in this context")
        })?;
        link.request(method, params, self.ctx.deadline)
    }

    /// data_source `search` over this plugin's knowledge root.
    pub fn records_search(&self, request: Value) -> Result<Value, Refusal> {
        self.request("records.search", request)
    }

    /// data_source `fetch` over this plugin's knowledge root.
    pub fn records_fetch(&self, request: Value) -> Result<Value, Refusal> {
        self.request("records.fetch", request)
    }

    /// Publish `payload` (an object, at most 64 KiB) as bus event
    /// `plugin/<candidate>/<subject>`; returns `{id, subject}`.
    pub fn events_publish(&self, subject: &str, payload: Value) -> Result<Value, Refusal> {
        self.request(
            "events.publish",
            json!({"subject": subject, "payload": payload}),
        )
    }
}

/// `initialize` parameters. The root is host-chosen; the plugin validates it.
#[derive(Debug, Clone)]
pub struct Init {
    pub root: Option<String>,
    /// The host's disposable conformance fixture (`fixture_bootstrap`).
    pub fixture: bool,
    /// Every parameter, including fields this SDK does not know yet.
    pub params: Value,
}

/// One contract a plugin fulfils, with the commands it exposes.
#[derive(Debug, Clone)]
pub struct ContractSpec {
    pub contract: Contract,
    /// Contract methods plus declared extensions, e.g. data_source `commit`.
    pub methods: Vec<String>,
    pub timeouts_ms: Vec<(String, u64)>,
}

/// Everything the descriptor says about a plugin. Build it with the
/// chained setters; [`Manifest::shape`] produces the host descriptor.
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Executable name, `bboxy450-term-<contract>-<provider>`.
    pub executable: String,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub version: String,
    pub identity: Map<String, Value>,
    pub contracts: Vec<ContractSpec>,
    pub skills: Vec<Skill>,
    /// Seat tools beyond the contracts, served as MCP tools `tool_<name>`.
    pub tools: Vec<Tool>,
    /// Harness and bus events delivered to [`Plugin::on_hook`].
    pub hooks: Vec<Hook>,
    /// Host API methods requested for [`Ctx::host`].
    pub host_api: Vec<String>,
    /// Prefix of embedded skill resource URIs, e.g. `gitkb`.
    pub resource_scheme: String,
    pub signed_writer: bool,
    pub fixture_bootstrap: bool,
}

impl Manifest {
    pub fn new(executable: &str, name: &str, description: &str) -> Self {
        let scheme = executable.rsplit('-').next().unwrap_or("plugin").to_owned();
        Self {
            executable: executable.into(),
            name: name.into(),
            description: description.into(),
            icon: None,
            version: "1.0.0".into(),
            identity: Map::new(),
            contracts: Vec::new(),
            skills: Vec::new(),
            tools: Vec::new(),
            hooks: Vec::new(),
            host_api: Vec::new(),
            resource_scheme: scheme,
            signed_writer: false,
            fixture_bootstrap: false,
        }
    }

    /// Fulfil every method of `contract`.
    pub fn contract(mut self, contract: Contract) -> Self {
        let methods = contract.methods().iter().map(|m| (*m).to_owned()).collect();
        self.contracts.push(ContractSpec {
            contract,
            methods,
            timeouts_ms: Vec::new(),
        });
        self
    }

    /// Add an extension method (it must still have a contract schema).
    pub fn method(mut self, contract: Contract, method: &str) -> Self {
        if let Some(spec) = self.contracts.iter_mut().find(|s| s.contract == contract) {
            spec.methods.push(method.into());
        }
        self
    }

    /// Declare a non-default budget for one method.
    pub fn timeout(mut self, contract: Contract, method: &str, ms: u64) -> Self {
        if let Some(spec) = self.contracts.iter_mut().find(|s| s.contract == contract) {
            spec.timeouts_ms.push((method.into(), ms));
        }
        self
    }

    pub fn skill(mut self, skill: Skill) -> Self {
        self.skills.push(skill);
        self
    }

    /// Declare a seat tool. Seats call it as `plugin_tool_<provider>_<name>`;
    /// the runtime routes it to [`Plugin::call_tool`]. `role` is
    /// [`RoleClass::Worker`] (every seat) or [`RoleClass::Oracle`].
    pub fn tool(
        mut self,
        name: &str,
        description: &str,
        input_schema: Value,
        output_schema: Value,
        role: RoleClass,
    ) -> Self {
        self.tools.push(Tool {
            name: name.into(),
            description: description.into(),
            input_schema,
            output_schema,
            role,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        });
        self
    }

    /// Declare a non-default budget for one declared tool.
    pub fn tool_timeout(mut self, name: &str, ms: u64) -> Self {
        if let Some(tool) = self.tools.iter_mut().find(|t| t.name == name) {
            tool.timeout_ms = ms;
        }
        self
    }

    /// Subscribe to a harness event (`PreToolUse`, `PostToolUse`,
    /// `UserPromptSubmit`, `PermissionRequest`, `SessionStart`, `SessionEnd`,
    /// `Interrupt`, `SeatState`) at `phase`, within `budget_ms` (at most the
    /// host ceiling: 1 s for ending events, 2 s before a tool, else 5 s).
    /// Events arrive in [`Plugin::on_hook`]. Their payload arrives only when
    /// the operator granted it; a `before` reply can veto only with the
    /// operator's block grant.
    pub fn hook(mut self, event: &str, phase: Phase, budget_ms: u64) -> Self {
        self.hooks.push(Hook {
            event: event.into(),
            phase,
            budget_ms,
            subject: None,
        });
        self
    }

    /// Subscribe to bus events whose subject matches `subject` (`*` one
    /// token, `>` the tail; never `inbox/…` or `plugin/…`). Delivered after
    /// publication as event `bus`; the envelope payload only with the
    /// operator's `bus` payload grant.
    pub fn bus_hook(mut self, subject: &str, budget_ms: u64) -> Self {
        self.hooks.push(Hook {
            event: bbt_plugin::policy::BUS_EVENT.into(),
            phase: Phase::After,
            budget_ms,
            subject: Some(subject.into()),
        });
        self
    }

    /// Request a host API method (`records.search`, `records.fetch`,
    /// `events.publish`) for [`Ctx::host`]. The operator still grants it.
    pub fn host_api(mut self, method: &str) -> Self {
        self.host_api.push(method.into());
        self
    }

    /// The declared tool behind a `tool_<name>` wire name.
    pub fn declared_tool(&self, wire: &str) -> Option<&Tool> {
        let name = wire.strip_prefix("tool_")?;
        self.tools.iter().find(|tool| tool.name == name)
    }

    pub fn identity(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.identity.insert(key.into(), value.into());
        self
    }

    pub fn version(mut self, version: &str) -> Self {
        self.version = version.into();
        self
    }

    pub fn signed_writer(mut self) -> Self {
        self.signed_writer = true;
        self
    }

    pub fn fixture_bootstrap(mut self) -> Self {
        self.fixture_bootstrap = true;
        self
    }

    /// The declared budget of one wire method.
    pub fn timeout_ms(&self, contract: Contract, method: &str) -> u64 {
        self.contracts
            .iter()
            .filter(|s| s.contract == contract)
            .flat_map(|s| s.timeouts_ms.iter())
            .find(|(m, _)| m == method)
            .map_or(DEFAULT_TIMEOUT_MS, |(_, ms)| *ms)
    }

    /// Map a `<contract>_<method>` tool name back to its parts.
    pub fn route(&self, tool: &str) -> Option<(Contract, &str)> {
        self.contracts.iter().find_map(|spec| {
            let method = tool.strip_prefix(spec.contract.name())?.strip_prefix('_')?;
            spec.methods
                .iter()
                .find(|m| *m == method)
                .map(|m| (spec.contract, m.as_str()))
        })
    }

    /// Every exposed tool as `(wire name, contract, method)`.
    pub fn tools(&self) -> Vec<(String, Contract, String)> {
        self.contracts
            .iter()
            .flat_map(|spec| {
                spec.methods.iter().map(|m| {
                    (
                        format!("{}_{m}", spec.contract.name()),
                        spec.contract,
                        m.clone(),
                    )
                })
            })
            .collect()
    }

    /// The URI of an embedded skill resource.
    pub fn skill_uri(&self, skill: &Skill) -> String {
        format!("{}://skills/{}/SKILL.md", self.resource_scheme, skill.name)
    }

    /// The host descriptor, as printed by `--shape`.
    pub fn shape(&self) -> Value {
        let systems: Vec<Value> = self
            .contracts
            .iter()
            .map(|spec| {
                let commands: Vec<Value> = spec
                    .methods
                    .iter()
                    .map(|m| {
                        json!({
                            "name": m,
                            "args": spec.contract.method_schema(m, false).unwrap_or(Value::Null),
                            "result": spec.contract.method_schema(m, true).unwrap_or(Value::Null),
                            "timeout_ms": self.timeout_ms(spec.contract, m),
                        })
                    })
                    .collect();
                json!({
                    "contract": spec.contract.name(),
                    "contract_version": spec.contract.version(),
                    "commands": commands,
                })
            })
            .collect();
        let mut identity = self.identity.clone();
        identity.insert("executable".into(), self.executable.clone().into());
        let mut shape = json!({
            "name": self.name,
            "description": self.description,
            "identity": identity,
            "version": self.version,
            "shape_version": SHAPE_VERSION,
            "systems": systems,
            "hooks": self.hooks,
            "health_command": "health",
            "signed_writer": self.signed_writer,
            "fixture_bootstrap": self.fixture_bootstrap,
            "concurrent": false,
            "trace": false,
            "skills": self.skills,
        });
        if let Some(icon) = &self.icon {
            shape["icon"] = icon.clone().into();
        }
        if !self.tools.is_empty() {
            shape["tools"] = serde_json::to_value(&self.tools).unwrap_or(Value::Null);
        }
        if !self.host_api.is_empty() {
            shape["host_api"] = self.host_api.clone().into();
        }
        shape
    }

    /// Check the descriptor with the host's own validator.
    pub fn validate(&self) -> Result<(), Refusal> {
        let name = bbt_plugin::shape::PluginName::parse(APP, &self.executable)?;
        let shape: Shape = serde_json::from_value(self.shape())?;
        shape.validate(&name)?;
        bbt_plugin::skills::validate(&self.skills)?;
        Ok(())
    }
}

/// The MCP tool name the host calls to deliver one hook event.
pub const HOOK_TOOL: &str = "hook";

/// One delivered hook event: the `hook` tool's arguments.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct HookEvent {
    /// The harness event as the harness spelled it, or `bus`.
    pub event: String,
    /// The phase of the matching declared hook.
    pub phase: Phase,
    /// Bus events: the published subject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// The seat the event belongs to, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat: Option<String>,
    /// The delivery budget, `min(budget_ms, ceiling)`.
    pub budget_ms: u64,
    /// Present only when the operator granted this event's payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

/// A hook's answer. Only a `before` hook holding the operator's block grant
/// can veto; every other reply is recorded and ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookReply {
    Allow,
    Deny { reason: String },
}

impl HookReply {
    /// The `hook` tool result.
    pub fn value(&self) -> Value {
        match self {
            Self::Allow => json!({"decision": "allow"}),
            Self::Deny { reason } => json!({"decision": "deny", "reason": reason}),
        }
    }
}

/// A plugin process bound to one root. The runtime calls these on one worker
/// thread, one request at a time.
pub trait Plugin: Sized + 'static {
    /// The static descriptor.
    fn manifest() -> Manifest;

    /// Bind the root. Called once; later `initialize` requests are refused.
    fn initialize(init: Init, ctx: &Ctx) -> Result<Self, Refusal>;

    /// Serve one contract method. `args` has already passed the host-field
    /// guard; the result must serialize within [`MAX_RESULT`].
    fn call(
        &mut self,
        contract: Contract,
        method: &str,
        args: Value,
        ctx: &Ctx,
    ) -> Result<Value, Refusal>;

    /// Serve one declared seat tool (see [`Manifest::tool`]). `name` is the
    /// declared name without the `tool_` prefix; `args` already passed the
    /// host-field guard. The default refuses every name.
    fn call_tool(&mut self, name: &str, _args: Value, _ctx: &Ctx) -> Result<Value, Refusal> {
        Err(Refusal::new(
            "unknown_tool",
            format!("this plugin does not serve tool {name}"),
        ))
    }

    /// Receive one hook event declared with [`Manifest::hook`] or
    /// [`Manifest::bus_hook`], within `event.budget_ms`. The default allows.
    fn on_hook(&mut self, _event: &HookEvent, _ctx: &Ctx) -> Result<HookReply, Refusal> {
        Ok(HookReply::Allow)
    }

    /// `health_command`; extend with plugin-specific facts.
    fn health(&mut self, _ctx: &Ctx) -> Result<Value, Refusal> {
        Ok(json!({"healthy": true}))
    }

    /// Budget for `initialize`; fixture bootstrap may need longer.
    fn initialize_budget(init: &Init) -> Duration {
        Duration::from_millis(if init.fixture { 14_500 } else { 1_900 })
    }
}

/// Deserialize typed arguments with a uniform refusal.
pub fn args<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, Refusal> {
    serde_json::from_value(value).map_err(|e| Refusal::new("invalid_arguments", e.to_string()))
}

/// Canonical compact JSON with sorted keys (serde_json's default map).
pub fn encoded(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest::new(
            "bboxy450-term-data_source-example",
            "Example",
            "Example provider",
        )
        .contract(Contract::DataSource)
        .method(Contract::DataSource, "commit")
        .timeout(Contract::DataSource, "commit", 15_000)
        .signed_writer()
    }

    #[test]
    fn shape_is_accepted_by_the_host_validator() {
        manifest().validate().unwrap();
        let shape = manifest().shape();
        let commands = shape["systems"][0]["commands"].as_array().unwrap();
        assert_eq!(commands.len(), 9);
        assert!(
            commands
                .iter()
                .all(|c| c["args"].is_object() && c["result"].is_object())
        );
        assert_eq!(
            shape["identity"]["executable"],
            "bboxy450-term-data_source-example"
        );
    }

    #[test]
    fn routes_wire_names_and_budgets() {
        let m = manifest();
        assert_eq!(
            m.route("data_source_commit"),
            Some((Contract::DataSource, "commit"))
        );
        assert_eq!(m.route("data_source_nope"), None);
        assert_eq!(m.route("task_system_list"), None);
        assert_eq!(m.timeout_ms(Contract::DataSource, "commit"), 15_000);
        assert_eq!(
            m.timeout_ms(Contract::DataSource, "list"),
            DEFAULT_TIMEOUT_MS
        );
        assert_eq!(m.tools().len(), 9);
    }

    #[test]
    fn declared_tools_reach_the_shape_and_are_validated_by_the_host() {
        let object = json!({"type": "object"});
        let m = manifest()
            .tool(
                "summarize",
                "Summarize",
                object.clone(),
                object.clone(),
                RoleClass::Oracle,
            )
            .tool_timeout("summarize", 5_000);
        m.validate().unwrap();
        let shape = m.shape();
        assert_eq!(shape["tools"][0]["name"], "summarize");
        assert_eq!(shape["tools"][0]["role"], "oracle");
        assert_eq!(shape["tools"][0]["timeout_ms"], 5_000);
        assert_eq!(m.declared_tool("tool_summarize").unwrap().name, "summarize");
        assert!(m.declared_tool("summarize").is_none());
        assert!(
            manifest().shape().get("tools").is_none(),
            "absent when unused"
        );
        for (bad, code) in [
            (
                manifest().tool(
                    "Bad-Name",
                    "",
                    object.clone(),
                    object.clone(),
                    RoleClass::Worker,
                ),
                "invalid_tool",
            ),
            (
                manifest().tool("x", "", object.clone(), object.clone(), RoleClass::HostOnly),
                "invalid_tool",
            ),
            (
                manifest().tool(
                    &"y".repeat(60),
                    "",
                    object.clone(),
                    object.clone(),
                    RoleClass::Worker,
                ),
                "tool_name_too_long",
            ),
            (
                manifest().tool(
                    "z",
                    "",
                    json!({"type": "string"}),
                    object.clone(),
                    RoleClass::Worker,
                ),
                "invalid_tool",
            ),
            (
                manifest()
                    .tool("w", "", object.clone(), object.clone(), RoleClass::Worker)
                    .tool("w", "", object.clone(), object.clone(), RoleClass::Worker),
                "duplicate_tool",
            ),
        ] {
            assert_eq!(bad.validate().unwrap_err().code, code);
        }
    }

    // keeps: G3 hooks — declared hooks reach the shape and the host validator.
    #[test]
    fn declared_hooks_reach_the_shape_and_are_validated_by_the_host() {
        let m = manifest()
            .hook("PreToolUse", Phase::Before, 2_000)
            .bus_hook("task/*/status", 500);
        m.validate().unwrap();
        let shape = m.shape();
        assert_eq!(
            shape["hooks"],
            json!([{"event":"PreToolUse","phase":"before","budget_ms":2000},
                   {"event":"bus","phase":"after","budget_ms":500,"subject":"task/*/status"}])
        );
        assert_eq!(manifest().shape()["hooks"], json!([]));
        for (bad, code) in [
            (
                manifest().hook("PreToolUse", Phase::Before, 2_001),
                "hook_budget_exceeded",
            ),
            (manifest().hook("Nope", Phase::After, 10), "unknown_hook"),
            (manifest().bus_hook("inbox/>", 10), "invalid_subject"),
        ] {
            assert_eq!(bad.validate().unwrap_err().code, code);
        }
        assert_eq!(
            HookReply::Deny {
                reason: "no".into()
            }
            .value(),
            json!({"decision":"deny","reason":"no"})
        );
    }

    // keeps: G7 host API — requested methods reach the shape; unknown ones are refused.
    #[test]
    fn requested_host_api_reaches_the_shape_and_is_validated_by_the_host() {
        let m = manifest()
            .host_api("records.search")
            .host_api("events.publish");
        m.validate().unwrap();
        assert_eq!(
            m.shape()["host_api"],
            json!(["records.search", "events.publish"])
        );
        assert!(manifest().shape().get("host_api").is_none());
        assert_eq!(
            manifest()
                .host_api("messages.send")
                .validate()
                .unwrap_err()
                .code,
            "unknown_host_api"
        );
    }

    #[test]
    fn call_tool_refuses_by_default() {
        struct Bare;
        impl Plugin for Bare {
            fn manifest() -> Manifest {
                manifest()
            }
            fn initialize(_: Init, _: &Ctx) -> Result<Self, Refusal> {
                Ok(Bare)
            }
            fn call(&mut self, _: Contract, _: &str, _: Value, _: &Ctx) -> Result<Value, Refusal> {
                Ok(Value::Null)
            }
        }
        let ctx = Ctx::detached(Duration::from_secs(1));
        assert_eq!(
            Bare.call_tool("anything", json!({}), &ctx)
                .unwrap_err()
                .code,
            "unknown_tool"
        );
    }

    #[test]
    fn refusal_value_keeps_extra_fields() {
        let value = Refusal::new("ambiguous_locator", "two")
            .with("candidates", json!([1, 2]))
            .value();
        assert_eq!(value["code"], "ambiguous_locator");
        assert_eq!(value["candidates"], json!([1, 2]));
    }

    #[test]
    fn ctx_reports_cancellation_before_deadline() {
        let flag = Arc::new(AtomicBool::new(false));
        let ctx = Ctx::new(flag.clone(), Duration::from_secs(5));
        assert!(ctx.check().is_ok());
        flag.store(true, Ordering::Relaxed);
        assert_eq!(ctx.check().unwrap_err().code, "cancelled");
        assert_eq!(
            Ctx::detached(Duration::ZERO).check().unwrap_err().code,
            "deadline_exceeded"
        );
    }
}
