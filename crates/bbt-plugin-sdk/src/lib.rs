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
pub use bbt_plugin::policy::RoleClass;
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
}

impl Ctx {
    pub fn new(cancelled: Arc<AtomicBool>, budget: Duration) -> Self {
        Self {
            cancelled,
            deadline: Instant::now() + budget,
        }
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
            "hooks": [],
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
