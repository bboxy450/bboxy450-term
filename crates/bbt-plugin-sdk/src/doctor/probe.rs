// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Drive the plugin over stdio as the host does (through [`testkit::Client`])
//! and turn every observation into checks.
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use bbt_plugin::contracts::Contract;
use bbt_plugin::shape::{PluginName, Shape};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{Options, Report, env};
use crate::testkit::{Client, Failure};
use crate::{APP, MAX_PENDING, MAX_RESULT, encoded};

/// Slack after a budget before a missing reply counts as no reply at all.
const GRACE: Duration = Duration::from_millis(1_500);
/// `--shape` and `initialize` budgets (`RegistryConfig`, `ProviderConfig`).
const HANDSHAKE: Duration = Duration::from_secs(2);
/// The checker's disposable fixture bootstrap budget (`client.rs`).
const BOOTSTRAP: Duration = Duration::from_secs(15);
/// `commit`'s own ceiling (`source.rs`).
const COMMIT: Duration = Duration::from_secs(15);
/// The seat bridge's overall budget for a declared seat tool (`plugin_tools.rs`).
const SEAT_TOOL: Duration = Duration::from_secs(8);
/// A single unpaged result above this should say it is truncated or page.
const LARGE: usize = 32 * 1024;
/// Pages walked per probe; real roots can be large.
const PAGES: usize = 40;

/// Short hex SHA-256 of a file (the host's trust digest is the full one).
pub(super) fn sha256(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).ok()?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Some(format!("{:x}", digest.finalize()))
}

/// A ULID-shaped id (Crockford base32), unique enough for fixture records.
pub(super) fn ulid() -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let mut seed = Sha256::new();
    seed.update(millis.to_le_bytes());
    seed.update(std::process::id().to_le_bytes());
    seed.update(
        COUNTER
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_le_bytes(),
    );
    let random = seed.finalize();
    let mut value: u128 = u128::from(millis) << 80;
    value |= u128::from_le_bytes(random[..16].try_into().expect("16 bytes")) >> 48;
    (0..26)
        .rev()
        .map(|i| ALPHABET[((value >> (i * 5)) & 31) as usize] as char)
        .collect()
}

/// Run `binary --shape` within `limit`; the stdout bytes or why not.
fn shape_bytes(
    binary: &Path,
    env: &[(String, String)],
    cwd: &Path,
) -> Result<(Vec<u8>, Duration), String> {
    let started = Instant::now();
    let mut child = Command::new(binary)
        .arg("--shape")
        .current_dir(cwd)
        .env_clear()
        .envs(env.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot run {}: {e}", binary.display()))?;
    let mut stdout = child.stdout.take().expect("piped");
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            let bytes = reader.join().unwrap_or_default();
            if !status.success() {
                return Err(format!("--shape exited {status}"));
            }
            return Ok((bytes, started.elapsed()));
        }
        if started.elapsed() > HANDSHAKE + GRACE {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "--shape printed nothing within {} ms",
                (HANDSHAKE + GRACE).as_millis()
            ));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Where the plugin runs: a private data directory, HOME and TMPDIR like the
/// host's checker, and the knowledge root it binds.
struct Workspace {
    _own: Option<TempDir>,
    data: PathBuf,
    env: Vec<(String, String)>,
    root: Option<PathBuf>,
    writable: bool,
    fixture: bool,
}

/// A directory removed on drop.
pub(super) struct TempDir(pub PathBuf);
impl TempDir {
    pub(super) fn new(prefix: &str) -> std::io::Result<Self> {
        let base = std::env::temp_dir().canonicalize()?;
        let path = base.join(format!("{prefix}-{}-{}", std::process::id(), ulid()));
        std::fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        if std::env::var_os("BBOXY450_DOCTOR_KEEP").is_none() {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

impl Workspace {
    fn new(shape: &Shape, options: &Options) -> std::io::Result<Self> {
        let own = TempDir::new("bboxy450-plugin-doctor")?;
        let data = own.0.join("data");
        let home = data.join("home");
        let tmp = home.join("tmp");
        std::fs::create_dir_all(&tmp)?;
        // The host's scrubbed environment, with the checker's private HOME.
        let mut env: Vec<(String, String)> = bbt_plugin::policy::filter_environment(
            std::env::vars()
                .collect::<Vec<_>>()
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str())),
        )
        .into_iter()
        .filter(|(k, _)| k != "HOME" && k != "TMPDIR")
        .collect();
        env.push(("HOME".into(), home.display().to_string()));
        env.push(("TMPDIR".into(), tmp.display().to_string()));
        let data_source = shape.systems.iter().any(|s| s.contract == "data_source");
        let (root, writable, fixture) = if let Some(scratch) = &options.scratch {
            (Some(scratch.clone()), true, false)
        } else if let Some(root) = &options.root {
            (Some(root.clone()), false, false)
        } else if data_source {
            // The checker's disposable root and seed (`trust.rs`).
            let root = tmp.join("checker-knowledge/.bboxy");
            std::fs::create_dir_all(&root)?;
            std::fs::write(
                root.join("p0-fixture.md"),
                "---\nid: 00000000000000000000000007\ntype: note\nnumber: 7\ntitle: Plugin conformance\n---\nPlugin conformance fixture.\n",
            )?;
            (Some(root), true, shape.fixture_bootstrap)
        } else {
            (None, false, false)
        };
        Ok(Self {
            _own: Some(own),
            data,
            env,
            root,
            writable,
            fixture,
        })
    }

    fn init_params(&self, shape: &Shape) -> Value {
        let contracts: serde_json::Map<String, Value> = shape
            .systems
            .iter()
            .map(|s| (s.contract.clone(), json!(s.contract_version)))
            .collect();
        let mut params = json!({
            "capabilities": {"experimental": {"bboxy450": {
                "shape_version": bbt_plugin::shape::SHAPE_VERSION,
                "contracts": contracts, "concurrent": shape.concurrent, "trace": false}}},
            "clientInfo": {"name": "bboxy450-plugin-doctor", "version": env!("CARGO_PKG_VERSION")},
        });
        if let Some(root) = &self.root {
            params["root"] = json!(root.display().to_string());
        }
        if self.fixture {
            params["fixture"] = json!({"version": 1, "kind": "data_source_conformance"});
        }
        params
    }
}

/// A call's outcome, after the transport checks have been recorded.
pub(super) enum Outcome {
    Ok(Value),
    Refused(Value),
    Failed,
}

impl Outcome {
    fn ok(&self) -> Option<&Value> {
        match self {
            Self::Ok(value) => Some(value),
            _ => None,
        }
    }
}

pub(super) struct Probe<'a> {
    pub(super) report: &'a mut Report,
    binary: PathBuf,
    shape: Shape,
    options: Options,
    workspace: Workspace,
    client: Option<Client>,
    catalogue: BTreeMap<String, Value>,
    /// The first call of a process is its cold start.
    cold: bool,
    /// Request `_meta` attached to every call (env checks).
    pub(super) meta: Option<Value>,
    /// Environment of the next spawn.
    pub(super) env: Vec<(String, String)>,
    /// Successful writes go to this root only.
    created: usize,
}

pub(super) fn run(report: &mut Report, binary: &Path, options: &Options) {
    let file = binary.file_name().and_then(|n| n.to_str()).unwrap_or("");
    report.ran(&["shape.name"]);
    let name = match PluginName::parse(APP, file) {
        Ok(name) => name,
        Err(refusal) => {
            report.error(
                "shape.name",
                format!("file name {file:?}: {refusal}"),
                "name the executable bboxy450-term-<contract>-<provider> with a [a-z0-9-] provider",
            );
            report.skip(
                "shape.descriptor",
                "the host refuses this file name before --shape",
            );
            return;
        }
    };
    let digest = sha256(binary);
    let scratch = match TempDir::new("bboxy450-plugin-doctor-shape") {
        Ok(dir) => dir,
        Err(error) => {
            report.skip(
                "shape.descriptor",
                format!("no temporary directory: {error}"),
            );
            return;
        }
    };
    let env: Vec<(String, String)> = crate::testkit::allowed_env();
    report.ran(&["shape.descriptor", "shape.digest"]);
    let first = match shape_bytes(binary, &env, &scratch.0) {
        Ok((bytes, elapsed)) => {
            if elapsed > HANDSHAKE {
                report.error(
                    "shape.descriptor",
                    format!(
                        "--shape took {} ms; the host waits {} ms",
                        elapsed.as_millis(),
                        HANDSHAKE.as_millis()
                    ),
                    "print the static descriptor before any IO or initialization",
                );
            }
            bytes
        }
        Err(error) => {
            report.error(
                "shape.descriptor",
                error,
                "make `<binary> --shape` print the descriptor and exit 0",
            );
            return;
        }
    };
    let value: Value = match serde_json::from_slice(&first) {
        Ok(value) => value,
        Err(error) => {
            report.error(
                "shape.descriptor",
                format!("--shape is not JSON: {error}"),
                "print exactly one JSON descriptor on stdout",
            );
            return;
        }
    };
    let mut extended = value.clone();
    // The host tolerates minor fields; so must the descriptor validator.
    extended["p0_future_minor_field"] = json!(true);
    let shape = match bbt_plugin::shape::parse_descriptor(extended, &name) {
        Ok(shape) => shape,
        Err(refusal) => {
            report.error(
                "shape.descriptor",
                format!("the host's descriptor validator refuses it: {refusal}"),
                "fix the descriptor; Manifest::validate() runs the same validator in a unit test",
            );
            return;
        }
    };
    if let Ok((second, _)) = shape_bytes(binary, &env, &scratch.0)
        && second != first
    {
        report.error(
            "shape.digest",
            "two --shape runs printed different descriptors; the host caches one descriptor per binary digest",
            "make --shape deterministic: no clocks, random ids, paths or environment in it",
        );
    }
    drop(scratch);
    let workspace = match Workspace::new(&shape, options) {
        Ok(workspace) => workspace,
        Err(error) => {
            report.skip("call.cold_start", format!("no private workspace: {error}"));
            return;
        }
    };
    let mut probe = Probe {
        report,
        binary: binary.to_owned(),
        env: workspace.env.clone(),
        shape,
        options: options.clone(),
        workspace,
        client: None,
        catalogue: BTreeMap::new(),
        cold: true,
        meta: None,
        created: 0,
    };
    if probe.start() {
        probe.exercise();
        probe.concurrency();
        probe.children();
    }
    probe.client.take();
    if digest != sha256(binary) {
        probe.report.error(
            "shape.digest",
            "the binary's SHA-256 changed while it ran; the host pins one digest per trust",
            "never rewrite the executable; keep state in the data directory",
        );
    }
}

impl Probe<'_> {
    fn spawn(&mut self) -> Result<Client, String> {
        Client::spawn(&self.binary, &self.workspace.data, &self.env).map_err(|e| e.to_string())
    }

    /// Spawn and initialize; the first spawn also records the cold start and
    /// the environment differential.
    fn start(&mut self) -> bool {
        self.report
            .ran(&["call.cold_start", "env.scrubbed", "call.refusal"]);
        match self.connect(true) {
            Ok(()) => true,
            Err(refusal) => {
                // Does it work with the full inherited environment?
                let scrubbed = std::mem::replace(&mut self.env, std::env::vars().collect());
                let inherited = self.connect(false).is_ok();
                self.client.take();
                self.env = scrubbed;
                if inherited {
                    self.report.error(
                        "env.scrubbed",
                        format!("initialize fails under the host's scrubbed environment ({refusal}) but succeeds with the full inherited one"),
                        "read nothing from the inherited environment: settings come from the shape, initialize and the data directory; seat attribution from _meta bboxy450/attribution",
                    );
                } else {
                    self.report.error(
                        "call.refusal",
                        format!("initialize was refused: {refusal}"),
                        "initialize must bind the host's root and return; refuse only with a typed code the operator can act on",
                    );
                }
                false
            }
        }
    }

    fn connect(&mut self, record: bool) -> Result<(), String> {
        let mut client = self.spawn()?;
        let budget = if self.workspace.fixture {
            BOOTSTRAP
        } else {
            HANDSHAKE
        };
        let started = Instant::now();
        let result = client.initialize(self.workspace.init_params(&self.shape), budget + GRACE);
        let elapsed = started.elapsed();
        match result {
            Ok(_) => {
                if record && elapsed > budget {
                    self.report.error(
                        "call.cold_start",
                        format!("initialize took {} ms; the host waits {} ms", elapsed.as_millis(), budget.as_millis()),
                        "bind the root and return; build indexes on a thread and answer early calls without them",
                    );
                }
                self.client = Some(client);
                self.cold = true;
                Ok(())
            }
            Err(Failure::Protocol(error)) => Err(error["data"]["code"].as_str().map_or_else(
                || error.to_string(),
                |code| {
                    format!(
                        "{code}: {}",
                        error["data"]["message"].as_str().unwrap_or("")
                    )
                },
            )),
            Err(Failure::NoReply(_)) => Err(match client.exited() {
                Some(code) => format!("the process exited {code} during initialize"),
                None => format!(
                    "no initialize reply within {} ms",
                    (budget + GRACE).as_millis()
                ),
            }),
            Err(Failure::Tool(value)) => Err(value.to_string()),
        }
    }

    fn command_budget(&self, contract: Contract, method: &str) -> Duration {
        let declared = self
            .shape
            .systems
            .iter()
            .find(|s| s.contract == contract.name())
            .and_then(|s| s.commands.iter().find(|c| c.name == method))
            .map_or(Duration::from_millis(crate::DEFAULT_TIMEOUT_MS), |c| {
                Duration::from_millis(c.timeout_ms)
            });
        declared.min(if method == "commit" {
            COMMIT
        } else {
            self.options.ceiling
        })
    }

    /// One `tools/call`, with the transport checks every call gets.
    pub(super) fn call(
        &mut self,
        tool: &str,
        args: Value,
        budget: Duration,
        label: &str,
    ) -> Outcome {
        let Some(client) = self.client.as_mut() else {
            return Outcome::Failed;
        };
        let mut params = json!({"name": tool, "arguments": args});
        if let Some(meta) = &self.meta {
            params["_meta"] = meta.clone();
        }
        let cold = std::mem::replace(&mut self.cold, false);
        self.report
            .ran(&["call.deadline", "pending.retry_hint", "result.size"]);
        let started = Instant::now();
        let reply = client.request("tools/call", params, budget + GRACE);
        let elapsed = started.elapsed();
        if elapsed > budget {
            let (check, fix) = if cold {
                (
                    "call.cold_start",
                    "the first call of a process must also fit: answer from what is ready, or refuse with retry_after_ms while warming up",
                )
            } else {
                (
                    "call.deadline",
                    "page the work, check ctx.remaining(), or refuse with a typed code and retry_after_ms before the budget",
                )
            };
            self.report.error(
                check,
                format!(
                    "{label} took {} ms; the host's budget is {} ms",
                    elapsed.as_millis(),
                    budget.as_millis()
                ),
                fix,
            );
        }
        if std::env::var_os("BBOXY450_DOCTOR_TRACE").is_some() {
            let shown = match &reply {
                Ok(result) => encoded(&result["structuredContent"])
                    .chars()
                    .take(200)
                    .collect(),
                Err(failure) => format!("{failure:?}"),
            };
            eprintln!("doctor: {label} ({} ms) -> {shown}", elapsed.as_millis());
        }
        match reply {
            Ok(result) if result["isError"] == true => {
                let refusal = result["structuredContent"].clone();
                self.refusal(&refusal, label);
                Outcome::Refused(refusal)
            }
            Ok(result) => {
                let structured = result["structuredContent"].clone();
                let text = result["content"][0]["text"].as_str().unwrap_or("");
                if serde_json::from_str::<Value>(text).ok().as_ref() != Some(&structured) {
                    self.report.warn(
                        "contract.dto",
                        format!("{label}: text content differs from structuredContent"),
                        "return the same JSON as structuredContent and as the text content (MCP clients without structured output read the text)",
                    );
                }
                let size = encoded(&structured).len();
                if size > MAX_RESULT {
                    self.report.error(
                        "result.size",
                        format!("{label} returned {size} bytes; MAX_RESULT is {MAX_RESULT}"),
                        "page the result (limit and next) or truncate with an explicit flag",
                    );
                }
                Outcome::Ok(structured)
            }
            Err(failure) => {
                let evidence = match &failure {
                    Failure::Protocol(error) => format!(
                        "{label} answered a JSON-RPC error {error}; the host tears the connection down"
                    ),
                    _ => match client.exited() {
                        Some(code) => format!("the process exited {code} during {label}"),
                        None => format!(
                            "{label}: no reply within {} ms",
                            (budget + GRACE).as_millis()
                        ),
                    },
                };
                self.report.error(
                    "call.refusal",
                    evidence,
                    "return every failure as a typed refusal (Err(Refusal::new(code, message))); never panic, exit or answer a protocol error",
                );
                self.recover();
                Outcome::Failed
            }
        }
    }

    /// After a failed call: keep a live process, or start a new one.
    fn recover(&mut self) {
        let alive = self.client.as_mut().is_some_and(|c| c.exited().is_none());
        if !alive {
            self.client.take();
            if self.connect(false).is_err() {
                self.client = None;
            }
        }
    }

    fn refusal(&mut self, refusal: &Value, label: &str) {
        let typed =
            serde_json::from_value::<bbt_plugin::data_source::CommandError>(refusal.clone())
                .ok()
                .filter(|e| !e.code.is_empty());
        let Some(error) = typed else {
            self.report.error(
                "call.refusal",
                format!("{label} refused without a typed code and message: {refusal}"),
                "refuse with Refusal::new(code, message); the host decodes {code, message, retry_after_ms?}",
            );
            return;
        };
        match error.code.as_str() {
            "internal_error" => self.report.error(
                "call.refusal",
                format!("{label} panicked ({}); the host tears the plugin down on it", error.message),
                "turn the failure into a typed refusal instead of panicking (no unwrap/expect on input or IO)",
            ),
            "response_too_large" => self.report.error(
                "result.size",
                format!("{label} with default limits exceeded MAX_RESULT ({})", error.message),
                "honour limit, page with next, or truncate with an explicit flag",
            ),
            code if (code.ends_with("_pending") || code.contains("busy")) && error.retry_after_ms.is_none() => {
                self.report.error(
                    "pending.retry_hint",
                    format!("{label} refused {code} with no retry_after_ms"),
                    "attach .with(\"retry_after_ms\", n) to every pending or busy refusal",
                )
            }
            _ => {}
        }
    }

    /// Validate a contract result against the contract schema, the declared
    /// command schema, the advertised output schema and the typed DTO.
    fn dto(&mut self, contract: Contract, method: &str, value: &Value, label: &str) {
        self.report.ran(&["contract.dto"]);
        let declared = self
            .shape
            .systems
            .iter()
            .find(|s| s.contract == contract.name())
            .and_then(|s| s.commands.iter().find(|c| c.name == method))
            .map(|c| c.result.clone());
        let advertised = contract
            .wire_name(method)
            .ok()
            .and_then(|wire| self.catalogue.get(&wire))
            .and_then(|tool| tool.get("outputSchema").cloned());
        let mut problems = Vec::new();
        for (what, schema) in [
            ("contract schema", contract.method_schema(method, true)),
            ("declared result schema", declared),
            ("advertised outputSchema", advertised),
        ] {
            if let Some(schema) = schema
                && let Some(problem) = schema_problem(&schema, value)
            {
                problems.push(format!("{what}: {problem}"));
            }
        }
        if let Err(refusal) = contract.decode_result(method, value) {
            problems.push(format!("typed DTO: {}", refusal.message));
        }
        if let Some(problem) = problems.first() {
            self.report.error(
                "contract.dto",
                format!("{label} result is not a valid {} {method} DTO: {problem}", contract.name()),
                "return the typed result from bbt_plugin::<contract> (the SDK adapters do this for you)",
            );
        }
    }

    /// A contract method call: budget, transport checks and DTO validation.
    fn method(&mut self, contract: Contract, method: &str, args: Value) -> Outcome {
        let Ok(wire) = contract.wire_name(method) else {
            return Outcome::Failed;
        };
        let label = format!("{wire} {}", encoded(&args));
        let label: String = label.chars().take(160).collect();
        let budget = self.command_budget(contract, method);
        let outcome = self.call(&wire, args, budget, &label);
        if let Outcome::Ok(value) = &outcome {
            let value = value.clone();
            self.dto(contract, method, &value, &label);
        }
        outcome
    }

    fn exercise(&mut self) {
        self.catalogue();
        self.skills();
        self.report.ran(&["contract.dto"]);
        let health = self.shape.health_command.clone();
        match self.call(&health, json!({}), HANDSHAKE, "health") {
            Outcome::Ok(value) if !value.is_object() => self.report.error(
                "contract.dto",
                format!("health returned {value}, not an object"),
                "return an object such as {\"healthy\": true}",
            ),
            _ => {}
        }
        let systems: Vec<Contract> = self
            .shape
            .systems
            .iter()
            .filter_map(|s| Contract::named(&s.contract).ok())
            .collect();
        for contract in systems {
            match contract {
                Contract::DataSource => self.data_source(),
                Contract::TaskSystem => self.task_system(),
                Contract::RepoGraph => {
                    let root = self
                        .workspace
                        .root
                        .clone()
                        .unwrap_or_else(|| self.workspace.data.clone());
                    self.stable(
                        contract,
                        "describe",
                        json!({"root": root.display().to_string()}),
                    );
                    self.walk_changes(contract);
                }
                Contract::Session | Contract::Provider => {
                    self.stable(contract, "describe", json!({}));
                }
                Contract::DecisionModel => {
                    let criteria =
                        json!({"first": "Doctor candidate one", "second": "Doctor candidate two"});
                    let args = json!({"state": {"doctor": true}, "instructions": "Choose one named candidate", "criteria": criteria});
                    if let Outcome::Ok(result) = self.method(contract, "choose", args)
                        && !result["choice"]
                            .as_str()
                            .is_some_and(|c| criteria.get(c).is_some())
                    {
                        self.report.error(
                            "contract.dto",
                            format!(
                                "decision_model_choose chose {}, not a supplied criterion",
                                result["choice"]
                            ),
                            "choose one of the supplied criteria keys",
                        );
                    }
                }
                Contract::Transport => {
                    let fields: BTreeMap<String, String> = self
                        .shape
                        .spawn_fields
                        .iter()
                        .map(|f| {
                            (
                                f.name.clone(),
                                f.default.clone().unwrap_or_else(|| "doctor".into()),
                            )
                        })
                        .collect();
                    self.method(
                        contract,
                        "wrap",
                        json!({"fields": fields, "command": ["/bin/echo", "doctor inner argv"]}),
                    );
                }
                Contract::EventSink => {
                    let envelope = envelope();
                    if let Outcome::Ok(result) = self.method(
                        contract,
                        "deliver",
                        json!({"subject": "fixtures/doctor", "envelope": envelope}),
                    ) && result["id"] != envelope["id"]
                    {
                        self.report.error(
                            "contract.dto",
                            "event_sink_deliver answered a different id than the envelope's",
                            "answer with the envelope id: the host deduplicates by it",
                        );
                    }
                }
            }
            self.malformed(contract);
        }
        self.tools();
        self.hooks();
    }

    fn catalogue(&mut self) {
        self.report.ran(&["shape.catalogue"]);
        let Some(client) = self.client.as_mut() else {
            return;
        };
        let mut cursor: Option<String> = None;
        let mut seen = BTreeSet::new();
        let mut problems = Vec::new();
        loop {
            let params = cursor
                .as_ref()
                .map_or_else(|| json!({}), |c| json!({"cursor": c}));
            let page = match client.request("tools/list", params, HANDSHAKE + GRACE) {
                Ok(page) => page,
                Err(failure) => {
                    problems.push(format!("tools/list failed: {failure:?}"));
                    break;
                }
            };
            for tool in page["tools"].as_array().into_iter().flatten() {
                let name = tool["name"].as_str().unwrap_or("").to_owned();
                if self.catalogue.insert(name.clone(), tool.clone()).is_some() {
                    problems.push(format!("tool {name} is listed twice"));
                }
            }
            cursor = page["nextCursor"].as_str().map(str::to_owned);
            match &cursor {
                None => break,
                Some(next) if !seen.insert(next.clone()) || seen.len() > 128 => {
                    problems.push("tools/list cursor never terminates".into());
                    break;
                }
                _ => {}
            }
        }
        let mut wanted: Vec<String> = Vec::new();
        for system in &self.shape.systems {
            if let Ok(contract) = Contract::named(&system.contract) {
                wanted.extend(
                    system
                        .commands
                        .iter()
                        .filter_map(|c| contract.wire_name(&c.name).ok()),
                );
            }
        }
        wanted.extend(self.shape.tools.iter().map(|t| t.wire_name()));
        for wire in wanted {
            match self.catalogue.get(&wire) {
                None => problems.push(format!("{wire} is declared but not in tools/list")),
                Some(tool) => {
                    for key in ["inputSchema", "outputSchema"] {
                        if let Some(schema) = tool.get(key)
                            && (key == "inputSchema" || !schema.is_null())
                            && jsonschema::validator_for(schema).is_err()
                        {
                            problems.push(format!("{wire} {key} does not compile"));
                        }
                    }
                }
            }
        }
        for problem in problems {
            self.report.error(
                "shape.catalogue",
                problem,
                "serve tools/list from the same Manifest that prints --shape (the SDK runtime does)",
            );
        }
    }

    fn skills(&mut self) {
        if self.shape.skills.is_empty() {
            return;
        }
        self.report.ran(&["shape.skills"]);
        let skills = self.shape.skills.clone();
        let Some(client) = self.client.as_mut() else {
            return;
        };
        let listed = client
            .request("resources/list", json!({}), HANDSHAKE + GRACE)
            .unwrap_or_default();
        for skill in skills {
            let resource = listed["resources"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|r| r["name"] == skill.name.as_str());
            let Some(uri) = resource.and_then(|r| r["uri"].as_str()) else {
                self.report.error(
                    "shape.skills",
                    format!("skill {} is declared but not in resources/list", skill.name),
                    "serve every declared skill as a resource (Manifest::skill does)",
                );
                continue;
            };
            let read = client
                .request("resources/read", json!({"uri": uri}), HANDSHAKE + GRACE)
                .unwrap_or_default();
            if read["contents"][0]["text"].as_str() != Some(skill.content.as_str()) {
                self.report.error(
                    "shape.skills",
                    format!(
                        "skill {} reads back different content from {uri}",
                        skill.name
                    ),
                    "read skills from the embedded descriptor, never from disk",
                );
            }
        }
    }

    /// Call `method` twice with the same arguments: both must agree.
    fn stable(&mut self, contract: Contract, method: &str, args: Value) -> Option<Value> {
        let first = self.method(contract, method, args.clone()).ok()?.clone();
        self.report.ran(&["identity.stable"]);
        if let Outcome::Ok(second) = self.method(contract, method, args.clone()) {
            let differences = differences(&first, &second);
            let what = format!(
                "{}_{method} {} answered differently twice in a row, with no change between",
                contract.name(),
                subject(&args)
            );
            if let Some((path, _, values)) = differences.iter().find(|d| d.1) {
                self.report.error(
                    "identity.stable",
                    format!("{what}: {path} {values}"),
                    "derive ids, digests and etags from content only: no clocks, counters or per-process salts",
                );
            } else if let Some((path, _, values)) = differences.first() {
                self.report.warn(
                    "identity.stable",
                    format!("{what}: {path} {values}"),
                    "answer identical reads identically: stable timestamps and metadata let the host cache and diff",
                );
            }
        }
        Some(first)
    }

    /// Missing required arguments and an unknown tool must be typed refusals.
    fn malformed(&mut self, contract: Contract) {
        for method in contract.methods() {
            if matches!(
                *method,
                "write"
                    | "commit"
                    | "deliver"
                    | "delegate"
                    | "reassign"
                    | "report"
                    | "accept"
                    | "reject"
            ) {
                continue;
            }
            if let Ok(wire) = contract.wire_name(method) {
                let budget = self.command_budget(contract, method);
                self.call(
                    &wire,
                    malformed(),
                    budget,
                    &format!("{wire} with mistyped arguments"),
                );
            }
        }
        let wire = format!("{}_doctor_unknown_method", contract.name());
        self.call(&wire, json!({}), HANDSHAKE, &format!("unknown tool {wire}"));
    }

    fn tools(&mut self) {
        for tool in self.shape.tools.clone() {
            let required = tool.input_schema["required"]
                .as_array()
                .is_some_and(|r| !r.is_empty());
            if required {
                continue;
            }
            let budget = Duration::from_millis(tool.timeout_ms).min(SEAT_TOOL);
            let wire = tool.wire_name();
            if let Outcome::Ok(value) = self.call(&wire, json!({}), budget, &wire)
                && let Some(problem) = schema_problem(&tool.output_schema, &value)
            {
                self.report.error(
                    "contract.dto",
                    format!("{wire} result does not match its declared output_schema: {problem}"),
                    "return what output_schema declares; the host validates it",
                );
            }
        }
    }

    fn hooks(&mut self) {
        for hook in self.shape.hooks.clone() {
            let ceiling = hook.ceiling_ms().unwrap_or(1_000);
            let budget_ms = hook.budget_ms.min(ceiling);
            let mut event =
                json!({"event": hook.event, "phase": hook.phase, "budget_ms": budget_ms});
            if let Some(pattern) = &hook.subject {
                event["event"] = json!("bus");
                event["subject"] = json!(pattern.replace(['*', '>'], "doctor"));
            }
            let label = format!("hook {} {:?}", hook.event, hook.phase);
            if let Outcome::Ok(reply) = self.call(
                crate::HOOK_TOOL,
                event,
                Duration::from_millis(budget_ms),
                &label,
            ) {
                self.report.ran(&["contract.dto"]);
                let valid = match reply["decision"].as_str() {
                    Some("allow") => true,
                    Some("deny") => reply["reason"].is_string(),
                    _ => false,
                };
                if !valid {
                    self.report.error("contract.dto", format!("{label} replied {reply}, not {{decision: allow}} or {{decision: deny, reason}}"), "return HookReply::Allow or HookReply::Deny { reason }");
                }
            }
        }
    }

    /// Follow a paged method's cursor. Returns the item keys in order, the
    /// pages read and whether the walk reached the end.
    fn walk(&mut self, query: &Paged, limit: Option<usize>, pages: usize) -> Walk {
        let Paged {
            contract,
            method,
            base,
            cursor_key,
            items_key,
        } = query;
        let (contract, method, cursor_key, items_key) =
            (*contract, *method, *cursor_key, *items_key);
        self.report.ran(&["page.limit", "page.cursor"]);
        let mut keys = Vec::new();
        let mut seen = BTreeSet::new();
        let mut cursor: Option<String> = None;
        for _ in 0..pages {
            let mut args = base.clone();
            if let Some(limit) = limit {
                args["limit"] = json!(limit);
            }
            if let Some(cursor) = &cursor {
                args[cursor_key] = json!(cursor);
            }
            let value = self.method(contract, method, args).ok()?.clone();
            let items = value[items_key].as_array().cloned().unwrap_or_default();
            let name = format!("{}_{method}", contract.name());
            match limit {
                Some(limit) if items.len() > limit => self.report.error(
                    "page.limit",
                    format!("{name} with limit {limit} returned {} items", items.len()),
                    "never return more than limit items; continue with next",
                ),
                None if items.len() > bbt_plugin::data_source::MAX_PAGE => self.report.error(
                    "page.limit",
                    format!(
                        "{name} with no limit returned {} items; the most is {}",
                        items.len(),
                        bbt_plugin::data_source::MAX_PAGE
                    ),
                    "default to a page of at most 32 items and return next",
                ),
                None if items.len() > bbt_plugin::data_source::DEFAULT_PAGE => self.report.warn(
                    "page.limit",
                    format!(
                        "{name} with no limit returned {} items; the default page is {}",
                        items.len(),
                        bbt_plugin::data_source::DEFAULT_PAGE
                    ),
                    "default to a page of 32 items and return next",
                ),
                _ => {}
            }
            keys.extend(items.iter().map(item_key));
            match value["next"].as_str() {
                None => return Some((keys, true)),
                Some(next) => {
                    if !seen.insert(next.to_owned()) || cursor.as_deref() == Some(next) {
                        self.report.error(
                            "page.cursor",
                            format!("{name} returned cursor {next:?} again; following it never ends"),
                            "derive next from the last item's keyset position so it always advances, and return null after the last page",
                        );
                        return Some((keys, false));
                    }
                    cursor = Some(next.to_owned());
                }
            }
        }
        Some((keys, false))
    }

    /// Paging checks over several page sizes for one query.
    fn paging(&mut self, query: Paged) {
        let Paged {
            contract,
            method,
            items_key,
            ..
        } = query;
        let base = query.base.clone();
        let name = format!("{}_{method}", contract.name());
        let walks: Vec<(String, Walk)> = vec![
            ("limit 1".into(), self.walk(&query, Some(1), PAGES)),
            ("limit 2".into(), self.walk(&query, Some(2), PAGES / 2)),
            ("no limit".into(), self.walk(&query, None, 3)),
        ];
        self.report.ran(&["page.order"]);
        let walks: Vec<(String, Vec<String>, bool)> = walks
            .into_iter()
            .filter_map(|(n, w)| w.map(|(k, c)| (n, k, c)))
            .collect();
        for (label, keys, _) in &walks {
            let mut seen = BTreeSet::new();
            if let Some(dup) = keys.iter().find(|k| !seen.insert(*k)) {
                self.report.error(
                    "page.order",
                    format!("{name} {label}: {dup} appears on two pages"),
                    "page by a stable keyset (e.g. sort key then id) and start after the last returned item",
                );
            }
        }
        if let Some((_, reference, _)) = walks.iter().find(|(l, ..)| l == "no limit") {
            for (label, keys, complete) in &walks {
                let n = keys.len().min(reference.len());
                let comparable = if *complete && walks.iter().all(|w| w.2) {
                    keys.len().max(reference.len())
                } else {
                    n
                };
                if keys.get(..comparable) != reference.get(..comparable) {
                    self.report.error(
                        "page.order",
                        format!(
                            "{name}: {label} pages give {:?}… but one page gives {:?}…",
                            &keys[..n.min(keys.len()).min(4)],
                            &reference[..n.min(4)]
                        ),
                        "keep one total order across page sizes: no skips, no reordering",
                    );
                    break;
                }
            }
        }
        // A limit far above the maximum is clamped or refused, never served.
        let mut huge = base.clone();
        huge["limit"] = json!(100_000);
        if let Outcome::Ok(value) = self.method(contract, method, huge) {
            let count = value[items_key].as_array().map_or(0, Vec::len);
            if count > bbt_plugin::data_source::MAX_PAGE {
                self.report.error(
                    "page.limit",
                    format!("{name} with limit 100000 returned {count} items"),
                    "clamp limit to at most 128 (or refuse it with a typed code)",
                );
            }
        }
    }

    fn walk_changes(&mut self, contract: Contract) {
        self.report.ran(&["page.cursor"]);
        let name = format!("{}_changes_since", contract.name());
        let mut cursor = "0".to_owned();
        let mut seen = BTreeSet::new();
        for _ in 0..PAGES {
            let Outcome::Ok(value) = self.method(
                contract,
                "changes_since",
                json!({"cursor": cursor, "limit": 1}),
            ) else {
                return;
            };
            let changed = value["changed"].as_array().map_or(0, Vec::len);
            if changed > 1 {
                self.report.error(
                    "page.limit",
                    format!("{name} with limit 1 returned {changed} changes"),
                    "never return more than limit changes",
                );
            }
            let next = value["next"].as_str().unwrap_or("").to_owned();
            if changed == 0 {
                return;
            }
            if next == cursor || !seen.insert(next.clone()) {
                self.report.error(
                    "page.cursor",
                    format!(
                        "{name} returned changes with cursor {next:?} again; polling never advances"
                    ),
                    "advance next past every returned change; repeat it only when nothing changed",
                );
                return;
            }
            cursor = next;
        }
    }

    fn data_source(&mut self) {
        let c = Contract::DataSource;
        let first = self.stable(c, "list", json!({"filter": {}}));
        let mut refs: Vec<Value> = first
            .as_ref()
            .and_then(|v| v["items"].as_array().cloned())
            .unwrap_or_default();
        if self.workspace.writable {
            self.writes();
            // Enough records to page through.
            for i in refs.len()..5 {
                self.create(
                    &format!("doctor-page-{i}"),
                    json!({"type": "note", "title": format!("Doctor page {i}")}),
                    "Doctor page\n",
                );
            }
            if let Outcome::Ok(v) = self.method(c, "list", json!({"filter": {}})) {
                refs = v["items"].as_array().cloned().unwrap_or_default();
            }
        } else {
            self.report.skip(
                "write.roundtrip",
                "the root is not writable; pass --scratch <dir> or use the doctor's own root",
            );
            self.report.skip(
                "write.status",
                "the root is not writable; pass --scratch <dir> or use the doctor's own root",
            );
        }
        if refs.len() < 3 {
            self.report.skip(
                "page.order",
                format!("only {} records to page through", refs.len()),
            );
        }
        self.paging(Paged::new(
            c,
            "list",
            json!({"filter": {}}),
            "cursor",
            "items",
        ));
        for reference in refs.iter().take(3).cloned().collect::<Vec<_>>() {
            self.identity(&reference);
        }
        let query = refs
            .first()
            .and_then(|r| r["slug"].as_str())
            .and_then(|s| s.split(['-', '/', '_']).find(|w| w.len() > 2))
            .unwrap_or("doctor")
            .to_owned();
        self.stable(c, "search", json!({"query": query}));
        self.paging(Paged::new(
            c,
            "search",
            json!({"query": query}),
            "cursor",
            "items",
        ));
        let scope = json!({"scope": {"kind": "all", "graph": "markdown"}});
        self.stable(c, "graph", scope.clone());
        self.paging(Paged::new(c, "graph", scope, "cursor", "nodes"));
        self.walk_changes(c);
        // Missing records are refusals, never panics.
        let missing = json!({"id": "00000000000000000000000000", "number": null, "slug": "doctor-missing", "path": "doctor-missing.md", "digest": "0".repeat(64)});
        self.method(c, "fetch", json!({"ref": missing}));
        self.method(c, "links", json!({"ref": missing}));
        self.method(c, "resolve", json!({"locator": "#999999999"}));
    }

    fn identity(&mut self, reference: &Value) {
        let c = Contract::DataSource;
        self.report.ran(&["identity.id", "identity.stable"]);
        let id = reference["id"].as_str().unwrap_or("").to_owned();
        let external = reference["external_id"].as_str().map(str::to_owned);
        let mut locators = vec![("id", id.clone())];
        for key in ["slug", "path"] {
            if let Some(value) = reference[key].as_str().filter(|v| !v.is_empty()) {
                locators.push((key, value.to_owned()));
            }
        }
        if let Some(number) = reference["number"].as_u64() {
            locators.push(("number", format!("#{number}")));
        }
        for (kind, locator) in locators {
            if let Outcome::Ok(resolved) = self.method(c, "resolve", json!({"locator": locator})) {
                let got = resolved["id"].as_str().unwrap_or("");
                if got != id {
                    let swapped = external.as_deref() == Some(got);
                    self.report.error(
                        "identity.id",
                        format!("resolve by {kind} {locator:?} returned id {got:?}, but list gave {id:?}{}", if swapped { " (that is its external_id)" } else { "" }),
                        "id is the portable ULID from frontmatter; a backend's own handle belongs in external_id",
                    );
                }
            }
        }
        if let Some(fetched) = self.stable(c, "fetch", json!({"ref": reference})) {
            if fetched["id"] != reference["id"] {
                self.report.error(
                    "identity.id",
                    format!("fetch of {id} returned id {}", fetched["id"]),
                    "return the record the ref names, with the same id",
                );
            }
            if fetched["digest"] != reference["digest"] {
                self.report.error(
                    "identity.stable",
                    format!(
                        "{id}: list gave digest {} but fetch gave {} with no change between them",
                        reference["digest"], fetched["digest"]
                    ),
                    "compute the digest once, from the stored bytes, the same way in every method",
                );
            }
            let size = encoded(&fetched).len();
            if size > LARGE && fetched.get("truncated").is_none() {
                self.report.warn(
                    "result.size",
                    format!("fetch of {id} returned {size} bytes in one unpaged response"),
                    "keep single results small: truncate large bodies with an explicit flag, or page",
                );
            }
        }
        self.method(c, "links", json!({"ref": reference}));
    }

    /// Create a record at `<slug>.md`; the write result or None.
    fn create(&mut self, slug: &str, extra: Value, body: &str) -> Option<(Value, Value)> {
        self.created += 1;
        let mut frontmatter = json!({"id": ulid(), "slug": slug});
        for (key, value) in extra.as_object().into_iter().flatten() {
            frontmatter[key] = value.clone();
        }
        let request =
            json!({"path": format!("{slug}.md"), "frontmatter": frontmatter, "body": body});
        let result = self
            .method(Contract::DataSource, "write", request.clone())
            .ok()?
            .clone();
        Some((request, result))
    }

    fn writes(&mut self) {
        let c = Contract::DataSource;
        self.report.ran(&["write.roundtrip", "write.status"]);
        let slug = format!("doctor-roundtrip-{}", self.created);
        if let Some((request, result)) = self.create(
            &slug,
            json!({"type": "note", "title": "Doctor round trip"}),
            "Doctor round trip.\n\nSecond paragraph.\n",
        ) {
            let lossy = result["lossy"] == true;
            let stored = result["record"].clone();
            let mut reference = stored.clone();
            for key in ["frontmatter", "body", "etag", "resources", "parse_error"] {
                if let Some(fields) = reference.as_object_mut() {
                    fields.remove(key);
                }
            }
            for (when, record) in [
                ("write result", Some(stored.clone())),
                (
                    "fetch after write",
                    self.method(c, "fetch", json!({"ref": reference}))
                        .ok()
                        .cloned(),
                ),
            ] {
                let Some(record) = record else { continue };
                for problem in roundtrip(&request, &record) {
                    let evidence = format!("{when}: {problem}");
                    let fix = "store exactly the written frontmatter and body; never add shadow fields or copies of host records";
                    if lossy {
                        self.report.warn(
                            "write.roundtrip",
                            format!("{evidence} (reported lossy: true)"),
                            fix,
                        );
                    } else {
                        self.report.error("write.roundtrip", evidence, fix);
                    }
                }
            }
            // An update round-trips too.
            let mut update = request.clone();
            update["ref"] = reference.clone();
            update["etag"] = stored["etag"].clone();
            if let Some(fields) = update.as_object_mut() {
                fields.remove("path");
            }
            update["frontmatter"]["title"] = json!("Doctor round trip, updated");
            update["body"] = json!("Updated body.\n");
            if let Outcome::Ok(updated) = self.method(c, "write", update.clone()) {
                let mut fresh = updated["record"].clone();
                for key in ["frontmatter", "body", "etag", "resources", "parse_error"] {
                    if let Some(fields) = fresh.as_object_mut() {
                        fields.remove(key);
                    }
                }
                if let Outcome::Ok(record) = self.method(c, "fetch", json!({"ref": fresh})) {
                    for problem in roundtrip(&update, &record) {
                        self.report.error(
                            "write.roundtrip",
                            format!("fetch after update: {problem}"),
                            "store exactly the written frontmatter and body on update too",
                        );
                    }
                }
            }
        }
        // A status the provider cannot map is refused, or kept verbatim.
        let status = "doctor-unmappable-status";
        let slug = format!("doctor-status-{}", self.created);
        if let Some((request, result)) = self.create(
            &slug,
            json!({"type": "task", "title": "Doctor status", "status": status}),
            "Doctor status.\n",
        ) {
            let mut reference = result["record"].clone();
            if let Some(fields) = reference.as_object_mut() {
                for key in ["frontmatter", "body", "etag", "resources", "parse_error"] {
                    fields.remove(key);
                }
            }
            if let Outcome::Ok(record) = self.method(c, "fetch", json!({"ref": reference})) {
                let kept = record["frontmatter"]["status"] == status;
                let shadow: Vec<String> = record["frontmatter"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .filter(|(k, v)| {
                        *k != "status"
                            && request["frontmatter"].get(*k).is_none()
                            && v.as_str() == Some(status)
                    })
                    .map(|(k, _)| k.clone())
                    .collect();
                if !kept || !shadow.is_empty() {
                    self.report.error(
                        "write.status",
                        format!("status {status:?} was accepted but stored as status {} {}", record["frontmatter"]["status"], if shadow.is_empty() { String::new() } else { format!("with shadow field(s) {shadow:?}") }),
                        "refuse a status you cannot map with a typed code (e.g. unsupported_status), or store it verbatim",
                    );
                }
            }
        }
    }

    fn task_system(&mut self) {
        let c = Contract::TaskSystem;
        let first = self.stable(c, "list", json!({}));
        let tasks: Vec<Value> = first
            .as_ref()
            .and_then(|v| v["items"].as_array().cloned())
            .unwrap_or_default();
        if tasks.len() < 3 {
            self.report.skip(
                "page.order",
                format!("only {} tasks to page through", tasks.len()),
            );
        }
        self.paging(Paged::new(c, "list", json!({}), "after", "items"));
        self.report.ran(&["identity.id"]);
        for task in tasks.iter().take(3) {
            let id = task["id"].as_str().unwrap_or("").to_owned();
            let mut requests = vec![json!({"task_id": id})];
            if let Some(path) = task["path"].as_str() {
                requests.push(json!({"task_path": path}));
            }
            if let Some(number) = task["number"].as_u64() {
                requests.push(json!({"task_id": format!("#{number}")}));
            }
            for request in requests {
                if let Some(found) = self.stable(c, "status", request.clone())
                    && found["id"] != task["id"]
                {
                    let swapped = found["id"] == task["external_id"];
                    self.report.error(
                        "identity.id",
                        format!(
                            "task_system_status {request} returned id {}, list gave {id}{}",
                            found["id"],
                            if swapped { " (its external_id)" } else { "" }
                        ),
                        "id is the portable ULID; a backend's own handle belongs in external_id",
                    );
                }
            }
        }
        self.walk_changes(c);
        self.method(
            c,
            "status",
            json!({"task_id": "00000000000000000000000000"}),
        );
    }

    /// One read per contract, so a plugin that starts children starts them.
    fn round(&mut self) {
        let health = self.shape.health_command.clone();
        self.call(&health, json!({}), HANDSHAKE, "health");
        for system in self.shape.systems.clone() {
            let Ok(contract) = Contract::named(&system.contract) else {
                continue;
            };
            match contract {
                Contract::DataSource => {
                    let listed = self
                        .method(contract, "list", json!({"filter": {}, "limit": 1}))
                        .ok()
                        .cloned();
                    self.method(contract, "search", json!({"query": "doctor", "limit": 1}));
                    if let Some(first) = listed.and_then(|v| v["items"].get(0).cloned()) {
                        self.method(contract, "fetch", json!({"ref": first}));
                    }
                }
                Contract::TaskSystem => {
                    self.method(contract, "list", json!({"limit": 1}));
                }
                Contract::RepoGraph => {
                    let root = self
                        .workspace
                        .root
                        .clone()
                        .unwrap_or_else(|| self.workspace.data.clone());
                    self.method(
                        contract,
                        "describe",
                        json!({"root": root.display().to_string()}),
                    );
                }
                Contract::Session | Contract::Provider => {
                    self.method(contract, "describe", json!({}));
                }
                _ => {}
            }
        }
    }

    /// Children of an attributed call see only that call's allowlisted
    /// attribution; a later unattributed call's children see none of it.
    fn children(&mut self) {
        if self.options.children.is_empty() {
            self.report.skip("env.children", "no child executable named; pass --child <name> for each program the plugin starts through PATH");
            return;
        }
        self.report.ran(&["env.children"]);
        let base = self.workspace.data.with_file_name("children");
        let observed = base.join("observed");
        let path = self
            .env
            .iter()
            .find(|(k, _)| k == "PATH")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let shimmed = match env::shims(&self.options.children, &path, &base.join("bin"), &observed)
        {
            Ok(shimmed) => shimmed,
            Err(error) => {
                self.report.skip(
                    "env.children",
                    format!("cannot write recording shims: {error}"),
                );
                return;
            }
        };
        let plain = self.env.clone();
        self.env.retain(|(k, _)| k != "PATH");
        self.env.push(("PATH".into(), shimmed));
        self.client.take();
        if let Err(error) = self.connect(false) {
            self.report.skip(
                "env.children",
                format!("the plugin did not start with recording shims on PATH: {error}"),
            );
            self.env = plain;
            return;
        }
        self.meta = Some(env::attributed());
        self.round();
        let attributed = env::recorded(&observed);
        self.meta = None;
        self.round();
        let all = env::recorded(&observed);
        self.client.take();
        self.env = plain;
        if all.is_empty() {
            self.report.skip("env.children", format!("none of {:?} was started through PATH (an absolute or pinned path cannot be observed)", self.options.children));
            return;
        }
        for file in &all {
            let text = std::fs::read_to_string(file).unwrap_or_default();
            let name = env::child(file);
            if text.contains(env::SECRET) {
                self.report.error(
                    "env.children",
                    format!("{name} received ATC_TOKEN from request _meta; only the allowlisted attribution names may reach a child"),
                    "pass ctx.attribution() (validated, allowlisted) to a child, never raw _meta or the whole environment",
                );
            }
            if !attributed.contains(file) && text.contains(env::SESSION) {
                self.report.error(
                    "env.children",
                    format!("{name}, started by a later unattributed call, still carried the earlier call's ATC_SESSION_ID"),
                    "set attribution on that call's own child Command only; never std::env::set_var or a shared backend environment",
                );
            }
        }
    }

    /// Pipelined calls: one reply each, and busy is retryable.
    fn concurrency(&mut self) {
        self.report.ran(&["concurrency.busy"]);
        let read = self
            .shape
            .systems
            .iter()
            .find_map(|s| match s.contract.as_str() {
                "data_source" => Some(("data_source_list", json!({"filter": {}, "limit": 1}))),
                "task_system" => Some(("task_system_list", json!({"limit": 1}))),
                "repo_graph" | "session" | "provider" => None,
                _ => None,
            });
        let mut batch: Vec<(&str, Value)> = Vec::new();
        for i in 0..MAX_PENDING + 4 {
            match (&read, i % 2) {
                (Some((tool, args)), 0) => {
                    batch.push(("tools/call", json!({"name": tool, "arguments": args})))
                }
                _ => batch.push(("tools/call", json!({"name": "health", "arguments": {}}))),
            }
        }
        let budget = self.options.ceiling;
        let Some(client) = self.client.as_mut() else {
            return;
        };
        let wait = budget * (batch.len() as u32) + GRACE;
        let replies = client.batch(&batch, wait);
        let exited = client.exited();
        let mut retry = Vec::new();
        for (i, reply) in replies.iter().enumerate() {
            match reply {
                Ok(result) if result["isError"] == true => {
                    let code = result["structuredContent"]["code"].as_str().unwrap_or("");
                    if code.contains("busy") {
                        if result["structuredContent"]["retry_after_ms"].as_u64().is_none() {
                            self.report.error("pending.retry_hint", format!("pipelined call {} refused {code} with no retry_after_ms", i + 1), "attach retry_after_ms to busy refusals");
                        }
                        retry.push(i);
                    }
                }
                Ok(_) => {}
                Err(Failure::Protocol(error)) if i >= MAX_PENDING && error["message"].as_str().is_some_and(|m| m.contains("busy")) => retry.push(i),
                Err(Failure::Protocol(error)) => self.report.error(
                    "concurrency.busy",
                    format!("pipelined call {} of {} (within the {MAX_PENDING} the host may keep pending) answered a protocol error {error}", i + 1, batch.len()),
                    "admit up to MAX_PENDING requests and answer each one (the SDK runtime does)",
                ),
                Err(_) => self.report.error(
                    "concurrency.busy",
                    match exited {
                        Some(code) => format!("the process exited {code} with {} pipelined calls in flight", batch.len()),
                        None => format!("pipelined call {} of {} got no reply", i + 1, batch.len()),
                    },
                    "answer every admitted request exactly once; never assume one request at a time",
                ),
            }
        }
        if exited.is_some() {
            self.recover();
            return;
        }
        for i in retry {
            let (_, params) = &batch[i];
            let tool = params["name"].as_str().unwrap_or("health").to_owned();
            if let Outcome::Refused(refusal) = self.call(
                &tool,
                params["arguments"].clone(),
                budget,
                &format!("retry of busy call {}", i + 1),
            ) {
                self.report.error("concurrency.busy", format!("retry of a busy call {} after the queue drained was refused again: {refusal}", i + 1), "busy must clear once earlier requests are answered");
            }
        }
    }
}

/// Every common request field with the wrong type.
fn malformed() -> Value {
    let mut args = serde_json::Map::new();
    for key in [
        "limit",
        "cursor",
        "locator",
        "ref",
        "query",
        "filter",
        "scope",
        "task_id",
        "task_path",
        "after",
        "criteria",
        "state",
        "instructions",
        "fields",
        "command",
    ] {
        args.insert(key.into(), json!({"doctor_wrong_type": [7]}));
    }
    Value::Object(args)
}

/// The first JSON Schema error of `value`, if any.
fn schema_problem(schema: &Value, value: &Value) -> Option<String> {
    match jsonschema::validator_for(schema) {
        Ok(validator) => validator.validate(value).err().map(|e| e.to_string()),
        Err(error) => Some(format!("schema does not compile: {error}")),
    }
}

/// One paged query: the method, its base arguments and its page keys.
struct Paged<'a> {
    contract: Contract,
    method: &'a str,
    base: Value,
    cursor_key: &'a str,
    items_key: &'a str,
}

impl<'a> Paged<'a> {
    fn new(
        contract: Contract,
        method: &'a str,
        base: Value,
        cursor_key: &'a str,
        items_key: &'a str,
    ) -> Self {
        Self {
            contract,
            method,
            base,
            cursor_key,
            items_key,
        }
    }
}

/// A walk's item keys in order and whether it reached the last page.
type Walk = Option<(Vec<String>, bool)>;

/// Identity fields: a difference in one of these is an error, any other a warning.
const IDENTITY: &[&str] = &[
    "id",
    "external_id",
    "digest",
    "etag",
    "version",
    "number",
    "slug",
    "path",
    "next",
];

/// Every differing leaf between two results: `(path, is an identity field, a vs b)`.
fn differences(a: &Value, b: &Value) -> Vec<(String, bool, String)> {
    fn walk(
        path: &str,
        key: &str,
        a: Option<&Value>,
        b: Option<&Value>,
        out: &mut Vec<(String, bool, String)>,
    ) {
        if a == b || out.len() >= 32 {
            return;
        }
        match (a, b) {
            (Some(Value::Object(x)), Some(Value::Object(y))) => {
                let keys: BTreeSet<&String> = x.keys().chain(y.keys()).collect();
                for k in keys {
                    walk(&format!("{path}.{k}"), k, x.get(k), y.get(k), out);
                }
            }
            (Some(Value::Array(x)), Some(Value::Array(y))) if x.len() == y.len() => {
                for i in 0..x.len() {
                    walk(&format!("{path}[{i}]"), key, x.get(i), y.get(i), out);
                }
            }
            _ => out.push((
                path.trim_start_matches('.').to_owned(),
                IDENTITY.contains(&key),
                format!("{} vs {}", short(a), short(b)),
            )),
        }
    }
    let mut out = Vec::new();
    walk("", "", Some(a), Some(b), &mut out);
    out
}

/// A short name for a request: the record it addresses, or its arguments.
fn subject(args: &Value) -> String {
    match args["ref"]["id"].as_str() {
        Some(id) => format!("of {id}"),
        None => encoded(args).chars().take(80).collect(),
    }
}

fn short(value: Option<&Value>) -> String {
    value.map_or_else(
        || "absent".into(),
        |v| encoded(v).chars().take(80).collect(),
    )
}

/// A page item's identity key.
fn item_key(item: &Value) -> String {
    for key in ["id", "ref"] {
        if let Some(id) = item[key].as_str() {
            return id.to_owned();
        }
        if let Some(id) = item[key]["id"].as_str() {
            return id.to_owned();
        }
    }
    encoded(item)
}

/// Differences between a write request and the stored record.
fn roundtrip(request: &Value, record: &Value) -> Vec<String> {
    let mut problems = Vec::new();
    let empty = serde_json::Map::new();
    let wrote = request["frontmatter"].as_object().unwrap_or(&empty);
    let got = record["frontmatter"].as_object().unwrap_or(&empty);
    for (key, value) in wrote {
        match got.get(key) {
            None => problems.push(format!("frontmatter {key} was dropped")),
            Some(stored) if stored != value => {
                problems.push(format!("frontmatter {key} {value} came back as {stored}"))
            }
            _ => {}
        }
    }
    let added: Vec<&String> = got.keys().filter(|k| !wrote.contains_key(*k)).collect();
    if !added.is_empty() {
        problems.push(format!(
            "frontmatter gained field(s) {added:?} that were never written"
        ));
    }
    if record["body"] != request["body"] {
        problems.push(format!(
            "body {} came back as {}",
            short(request.get("body")),
            short(record.get("body"))
        ));
    }
    problems
}

/// A schema-valid, unsigned envelope for `event_sink_deliver`.
fn envelope() -> Value {
    use bbt_plugin::event_sink as es;
    let id = ulid();
    serde_json::to_value(es::Envelope {
        version: 1,
        id: id.clone(),
        issuer_kind: es::IssuerKind::LocalDev,
        issuer: "doctor".into(),
        account: "doctor".into(),
        device: "doctor".into(),
        seat: id.clone(),
        author: "doctor".into(),
        swarm_id: id.clone(),
        process_generation: "1".into(),
        context: "fixtures/doctor".into(),
        role: es::Role {
            name: "executor".into(),
            class: es::Class::Worker,
        },
        role_grant_id: "doctor".into(),
        role_epoch: "0".into(),
        priority_class: es::Class::Worker,
        sender_sequence: "1".into(),
        thread_id: id,
        reply_to: None,
        timestamp: 1,
        priority: 0,
        kind: es::Kind::Text,
        body: es::Body::Text {
            text: "plugin doctor fixture".into(),
        },
        attachments: Vec::new(),
        door: None,
        issued_by: None,
        signature: "unsigned-doctor-fixture".into(),
    })
    .unwrap_or(Value::Null)
}
