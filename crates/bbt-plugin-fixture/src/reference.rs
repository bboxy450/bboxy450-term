// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The P-0 reference contract server and conformance adversary.
//!
//! Translated from bboxy450-term,
//! crates/bbt-mux/tests/fixtures/plugins/bboxy450-term-data_source-reference
//! (the former Python fixture): https://github.com/bboxy450/bboxy450-term
//! Changes: Rust; knobs compiled in; schemas and conformance cases come from
//! `bboxy450-plugin` instead of embedded copies. Most faults break the
//! protocol on purpose, so this server writes JSON-RPC itself rather than
//! through the SDK runtime, which refuses to emit them.

use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bbt_plugin::contracts::Contract;
use bbt_plugin_fixture::Knobs;
use serde_json::{Map, Value, json};

const CASES: &str = include_str!("../../bbt-plugin/tests/fixtures/data_source/conformance.json");
const RECORD_ID: &str = "00000000000000000000000007";

type Out = Arc<Mutex<std::io::Stdout>>;

fn emit(out: &Out, value: &Value) {
    let mut out = out.lock().unwrap();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}

pub(crate) fn append(path: &str, value: &Value) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("fixture log");
    let _ = writeln!(file, "{value}");
}

struct Server<'a> {
    k: &'a Knobs,
    system: &'a str,
    contract: Contract,
    fault: &'a str,
    out: Out,
    declared: Vec<Value>,
    write_ids: HashSet<String>,
}

fn version(system: &str) -> &'static str {
    if matches!(system, "session" | "provider") {
        "0.1"
    } else {
        "1.0"
    }
}

fn declared(k: &Knobs) -> Vec<Value> {
    let mut tools = Vec::new();
    if k.tools {
        tools.push(json!({"name":"echo","description":"Echo the arguments","role":"worker","timeout_ms":2000,
            "input_schema":{"type":"object","properties":{"text":{"type":"string"}},"additionalProperties":false},
            "output_schema":{"type":"object","required":["tool","args"]}}));
        tools.push(json!({"name":"audit","description":"Oracle-only audit","role":"oracle","timeout_ms":2000,
            "input_schema":{"type":"object"},"output_schema":{"type":"object"}}));
    }
    if !k.host_api.is_empty() {
        tools.push(
            json!({"name":"host","description":"Ask the host","role":"worker","timeout_ms":2000,
            "input_schema":{"type":"object"},"output_schema":{"type":"object"}}),
        );
    }
    tools
}

impl Server<'_> {
    fn schema(&self, method: &str, result: bool) -> Value {
        self.contract
            .method_schema(method, result)
            .unwrap_or(Value::Null)
    }

    fn shape(&self) -> Value {
        let timeout = if self.fault == "timeout" {
            50
        } else {
            self.k.timeout_ms.unwrap_or(2000)
        };
        let commands: Vec<Value> = self
            .contract
            .methods()
            .iter()
            .map(|m| {
                json!({"name":m,"args":self.schema(m,false),"result":self.schema(m,true),
                    "timeout_ms":timeout,"role_class":"worker","tier":"read","future_minor_field":true})
            })
            .collect();
        let mut value = json!({"name":"reference","version":"1.0","shape_version":"1.0","health_command":"health",
            "systems":[{"contract":self.system,"contract_version":version(self.system),"commands":commands}],
            "future_minor_field":{"ignored":true}});
        if self.k.concurrent {
            value["concurrent"] = true.into();
        }
        if !self.declared.is_empty() {
            value["tools"] = self.declared.clone().into();
        }
        if !self.k.hooks.is_empty() {
            value["hooks"] = self.k.hooks.clone().into();
        }
        if !self.k.host_api.is_empty() {
            value["host_api"] = self.k.host_api.clone().into();
        }
        match self.fault {
            "unknown_contract" => value["systems"][0]["contract"] = "not_a_contract".into(),
            "major" => value["shape_version"] = "2.0".into(),
            "budget" => {
                value["hooks"] = json!([{"event":"PreToolUse","phase":"before","budget_ms":2001}]);
            }
            _ => (),
        }
        value
    }

    fn reply(&self, req: &Value, result: Value) {
        let mut id = req["id"].clone();
        if self.fault == "wrong_id" {
            id = (id.as_i64().unwrap_or(0) + 1).into();
        }
        emit(
            &self.out,
            &json!({"jsonrpc":"2.0","id":id,"result":result,"future_minor_field":true}),
        );
    }

    fn command_error(&self, req: &Value, code: &str, message: &str) {
        self.reply(
            req,
            json!({"isError":true,"structuredContent":{"code":code,"message":message,"retry_after_ms":10}}),
        );
    }

    fn observe(&self, value: Value) {
        if let Some(path) = &self.k.observe {
            append(path, &value);
        }
    }

    fn delayed(&self) -> bool {
        self.k
            .delay_file
            .as_ref()
            .is_some_and(|path| std::path::Path::new(path).exists())
    }

    // The control-test instrumentation: observe initialize and calls.
    fn instrument(&self, req: &Value) {
        if self.k.observe.is_none() {
            return;
        }
        let host_fields = ["author", "signer", "device", "account"];
        match req["method"].as_str() {
            Some("initialize") => {
                let root = req["params"]["root"].as_str().unwrap_or("");
                assert!(std::path::Path::new(root).join("p0-fixture.md").is_file());
                let cwd = std::env::current_dir().unwrap();
                assert_ne!(
                    std::fs::canonicalize(root).unwrap(),
                    std::fs::canonicalize(&cwd).unwrap()
                );
                assert!(
                    !host_fields
                        .iter()
                        .any(|key| req["params"].get(*key).is_some())
                );
                self.observe(
                    json!({"kind":"initialize","pid":std::process::id(),"root":root,
                    "cwd":cwd.to_string_lossy()}),
                );
                if let Some(gate) = &self.k.initialize_gate {
                    crate::raw::wait_gate(gate);
                }
            }
            Some("tools/call") => {
                let args = req["params"]
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                assert!(
                    !["root", "author", "signer", "device", "account"]
                        .iter()
                        .any(|key| args.get(*key).is_some())
                );
                self.observe(json!({"kind":"call","name":req["params"]["name"],"arguments":args}));
            }
            _ => (),
        }
    }

    fn initialize(&self, req: &Value) {
        let shape_version = if self.fault == "mcp_major" {
            "2.0"
        } else {
            "1.0"
        };
        let mut contracts = Map::new();
        contracts.insert(self.system.into(), version(self.system).into());
        self.reply(
            req,
            json!({"protocolVersion":req["params"]["protocolVersion"],
                "serverInfo":{"name":"reference","version":"1.0"},
                "capabilities":{"tools":{},"experimental":{"bboxy450":{"shape_version":shape_version,
                    "contracts":contracts,"concurrent":false,"trace":false}}}}),
        );
    }

    fn tools(&self, req: &Value) {
        let mut tools: Vec<Value> = self
            .contract
            .methods()
            .iter()
            .map(|m| {
                json!({"name":format!("{}_{m}", self.system),"inputSchema":self.schema(m,false),
                    "outputSchema":self.schema(m,true)})
            })
            .collect();
        tools.push(
            json!({"name":"health","inputSchema":{"type":"object","additionalProperties":true}}),
        );
        tools.extend(self.declared.iter().map(|t| {
            json!({"name":format!("tool_{}", t["name"].as_str().unwrap()),
                "inputSchema":t["input_schema"],"outputSchema":t["output_schema"]})
        }));
        self.reply(req, json!({"tools":tools,"future_minor_field":true}));
    }

    fn hook(&self, req: &Value, args: &Value) {
        if let Some(log) = &self.k.hook_log {
            append(log, args);
        }
        let decision = self.k.decision.as_deref().unwrap_or("allow");
        if decision == "slow" {
            std::thread::sleep(Duration::from_millis(1500));
        }
        let command = args["payload"]["tool_input"]["command"]
            .as_str()
            .unwrap_or("");
        let veto = decision == "deny" || decision == "deny-rm" && command.contains("rm -rf");
        let decision = if veto {
            json!({"decision":"deny","reason":"fixture veto"})
        } else {
            json!({"decision":"allow"})
        };
        self.reply(req, json!({"structuredContent":decision}));
    }

    // Ask the host while this call is in flight and answer with its reply.
    fn host(&self, req: &Value, args: &Value, input: &mut impl BufRead) {
        // A delayed ask lets a test change the plugin's authority mid-call;
        // the host's answer is also logged, since a superseded result is refused.
        if self.delayed() {
            std::thread::sleep(Duration::from_secs(1));
        }
        let asked = format!("h-{}", req["id"]);
        let params = args.get("params").cloned().unwrap_or_else(|| json!({}));
        emit(
            &self.out,
            &json!({"jsonrpc":"2.0","id":asked,"method":format!("host/{}", args["method"].as_str().unwrap_or("")),"params":params}),
        );
        let answer = loop {
            let mut line = String::new();
            if input.read_line(&mut line).unwrap_or(0) == 0 {
                std::process::exit(0);
            }
            let Ok(answer) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if answer["id"] == asked {
                break answer;
            }
        };
        if let Some(log) = &self.k.hook_log {
            append(log, &json!({"host_answer":answer}));
        }
        self.reply(req, json!({"structuredContent":{"answer":answer}}));
    }

    fn health(&self, req: &Value) {
        let mut names: Vec<String> = std::env::vars_os()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        names.sort();
        // macOS adds this runtime variable after exec; host tests inspect pre-exec names separately.
        let allowed = |n: &str| {
            (cfg!(target_os = "macos") && n == "__CF_USER_TEXT_ENCODING")
                || (["PATH", "HOME", "LANG", "TMPDIR", "TERM"].contains(&n) || n.starts_with("LC_"))
                    && !["TOKEN", "KEY", "SECRET", "BBOXY_", "ICQ_"]
                        .iter()
                        .any(|x| n.to_uppercase().contains(x))
        };
        let refused: Vec<&str> = names
            .iter()
            .map(String::as_str)
            .filter(|n| !allowed(n))
            .collect();
        if !refused.is_empty() {
            self.reply(
                req,
                json!({"isError":true,"structuredContent":{"code":format!("fixture_environment:{}", refused.join(","))}}),
            );
            return;
        }
        let executable = std::env::current_exe()
            .and_then(std::fs::canonicalize)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.reply(
            req,
            json!({"structuredContent":{"ok":true,"environment_names":names,"cwd":cwd,"executable":executable}}),
        );
    }

    fn rejects_unknown(&self, method: &str, args: &Value) -> bool {
        let schema = self.schema(method, false);
        let Some(name) = schema["$ref"].as_str().and_then(|r| r.rsplit('/').next()) else {
            return false;
        };
        let fields = &schema["$defs"][name]["properties"];
        args.as_object()
            .is_some_and(|args| args.keys().any(|key| fields.get(key).is_none()))
    }

    fn task_system(&self, req: &Value, method: &str, args: &Value) {
        let mut record = json!({"id":RECORD_ID,"number":1,"external_id":null,"path":"tasks/p0-fixture.md",
            "metadata":{"type":"task","status":"reported"},"body":"P-0 fixture only, not an accepted task.\n",
            "etag":"fixture-etag","signed_record":args.get("signed_record").cloned().unwrap_or_else(|| "opaque-read-fixture".into())});
        let mut value = match method {
            "list" => json!({"items":[record.clone()],"next":null}),
            "changes_since" => json!({"changed":[],"next":"1"}),
            _ => std::mem::take(&mut record),
        };
        if args.get("signed_record").is_some() {
            if self.fault == "omit_signed_record" {
                value.as_object_mut().unwrap().remove("signed_record");
            }
            if self.fault == "alter_signed_record"
                && let Some(signed) = value["signed_record"].as_str()
            {
                value["signed_record"] = format!("{signed}modified").into();
            }
        }
        value["p0_server_minor_field"] = true.into();
        self.reply(req, json!({"structuredContent":value}));
    }

    fn other(&self, req: &Value, method: &str, args: &Value) {
        let mut value = match self.system {
            "session" => json!({"sessions":[{"id":"fixture","name":"Fixture","harness":"zsh"}]}),
            "provider" => {
                json!({"hosts":[{"id":"fixture","name":"Fixture","kind":"ssh","address":"fixture.invalid"}]})
            }
            "repo_graph" if method == "describe" => json!({"repos":[],"edges":[]}),
            "repo_graph" => json!({"changed":[],"next":"1"}),
            "event_sink" => json!({"id":args["envelope"]["id"],"accepted":true}),
            _ => {
                let names: Vec<String> = args["criteria"]
                    .as_object()
                    .map(|c| c.keys().cloned().collect())
                    .unwrap_or_default();
                // serde_json sorts keys; the first supplied name stays the choice.
                let first = args["criteria"]
                    .as_object()
                    .and_then(|c| c.keys().next().cloned())
                    .unwrap_or_default();
                let probabilities: Map<String, Value> = names
                    .iter()
                    .map(|n| (n.clone(), if *n == first { 1.0 } else { 0.0 }.into()))
                    .collect();
                json!({"choice":first,"confidence":1.0,"probabilities":probabilities,"model":"fixture-1"})
            }
        };
        match self.fault {
            "unknown_choice" => value["choice"] = "not-a-supplied-criterion".into(),
            "unknown_probability" => {
                value["probabilities"]["not-a-supplied-criterion"] = 0.1.into();
            }
            "alter_envelope_id" => value["id"] = "00000000000000000000000008".into(),
            "bad_result" => value = json!({"missing":"required fields"}),
            _ => (),
        }
        value["p0_server_minor_field"] = true.into();
        self.reply(req, json!({"structuredContent":value}));
    }

    fn data_source(&mut self, req: &Value, method: &str, args: &Value) {
        let cases: Vec<Value> = serde_json::from_str(CASES).expect("conformance cases");
        let Some(case) = cases.iter().find(|c| c["method"] == method) else {
            self.command_error(req, "fixture_error", "fixture command refused");
            return;
        };
        let mut value = case["result"].clone();
        let fault = self.fault;
        if method == "write" {
            if fault == "write_identity_guard" {
                let identity = args["frontmatter"]["id"].as_str().unwrap_or("").to_owned();
                if identity.is_empty() || self.write_ids.contains(&identity) {
                    self.command_error(req, "immutable_id", "fixture command refused");
                    return;
                }
                let path = args["path"].as_str().unwrap_or("");
                if args["frontmatter"]["slug"].as_str()
                    != Some(path.strip_suffix(".md").unwrap_or(path))
                {
                    self.command_error(req, "slug_mismatch", "fixture command refused");
                    return;
                }
                self.write_ids.insert(identity);
            }
            if matches!(fault, "write_backend_conflict" | "write_backend_deadline")
                && args.get("p0_future_minor_field").is_some()
            {
                let (code, message) = if fault == "write_backend_conflict" {
                    (
                        "source_generation_mismatch",
                        "private native source changed before additive write",
                    )
                } else {
                    (
                        "deadline_exceeded",
                        "private native fixture exceeded its write deadline",
                    )
                };
                self.command_error(req, code, message);
                return;
            }
            let record = &mut value["record"];
            record["frontmatter"] = args["frontmatter"].clone();
            record["body"] = args["body"].clone();
            if let Some(path) = args.get("path").filter(|p| !p.is_null()) {
                record["path"] = path.clone();
            }
            for field in ["id", "slug"] {
                if let Some(v) = args["frontmatter"].get(field) {
                    record[field] = v.clone();
                }
            }
            if matches!(
                fault,
                "write_enrichment"
                    | "write_drop_title"
                    | "write_drop_title_minor"
                    | "write_change_body"
                    | "write_reported_loss"
            ) {
                let (id, slug) = (record["id"].clone(), record["slug"].clone());
                let frontmatter = record["frontmatter"].as_object_mut().unwrap();
                frontmatter.insert("id".into(), id);
                frontmatter.insert("slug".into(), slug);
                frontmatter.insert("updated".into(), "2026-10-29T00:00:00Z".into());
                frontmatter.insert(
                    "native_metadata".into(),
                    json!({"source":"private conformance fixture"}),
                );
                if matches!(fault, "write_drop_title" | "write_reported_loss")
                    || fault == "write_drop_title_minor"
                        && args.get("p0_future_minor_field").is_some()
                {
                    frontmatter.remove("title");
                }
                if fault == "write_change_body" {
                    let body = format!("{}\n", record["body"].as_str().unwrap_or(""));
                    record["body"] = body.into();
                }
                if fault == "write_reported_loss" {
                    value["lossy"] = true.into();
                }
            }
        }
        if fault == "bad_result" {
            value = json!({"missing":"required result fields"});
        }
        value["p0_server_minor_field"] = json!({"ignored":true});
        self.reply(req, json!({"structuredContent":value}));
    }

    fn call(&mut self, req: &Value, input: &mut impl BufRead) {
        let name = req["params"]["name"].as_str().unwrap_or("").to_owned();
        let args = req["params"]
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if name == "hook" && !self.k.hooks.is_empty() {
            return self.hook(req, &args);
        }
        if name == "tool_host" && !self.k.host_api.is_empty() {
            return self.host(req, &args, input);
        }
        if name != "health" && self.delayed() {
            if self.k.concurrent && name == format!("{}_changes_since", self.system) {
                let (out, id) = (self.out.clone(), req["id"].clone());
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(1));
                    emit(
                        &out,
                        &json!({"jsonrpc":"2.0","id":id,"result":{"structuredContent":{"changed":[],"next":"1"}},"future_minor_field":true}),
                    );
                });
                return;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        if self
            .declared
            .iter()
            .any(|t| format!("tool_{}", t["name"].as_str().unwrap_or("")) == name)
        {
            return self.reply(req, json!({"structuredContent":{"tool":name,"args":args}}));
        }
        if name == "health" {
            return self.health(req);
        }
        match self.fault {
            "stdout" => {
                let mut out = self.out.lock().unwrap();
                let _ = writeln!(out, "fixture banner is not protocol");
                let _ = out.flush();
            }
            "eof" => std::process::exit(3),
            "protocol_error" => {
                return emit(
                    &self.out,
                    &json!({"jsonrpc":"2.0","id":req["id"],"error":{"code":-32602,"message":"fixture protocol refusal"}}),
                );
            }
            "command_error" => {
                return self.command_error(req, "fixture_error", "fixture command refused");
            }
            "timeout" => {
                eprintln!("fixture resolve attempt");
                let out = self.out.clone();
                std::thread::spawn(move || {
                    loop {
                        emit(
                            &out,
                            &json!({"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken":"fixture","progress":1}}),
                        );
                        std::thread::sleep(Duration::from_millis(5));
                    }
                });
                return;
            }
            _ => (),
        }
        let prefix = format!("{}_", self.system);
        let method = name.strip_prefix(&prefix).unwrap_or(&name).to_owned();
        if self.fault == "reject_unknown" && self.rejects_unknown(&method, &args) {
            return self.command_error(req, "fixture_error", "fixture command refused");
        }
        match self.system {
            "task_system" => self.task_system(req, &method, &args),
            "data_source" => self.data_source(req, &method, &args),
            _ => self.other(req, &method, &args),
        }
    }
}

pub fn main(k: &Knobs) {
    let system = k.system.as_deref().unwrap_or("data_source");
    let fault = k.fault.as_deref().unwrap_or("");
    let mut server = Server {
        k,
        system,
        contract: Contract::named(system).expect("fixture system"),
        fault,
        out: Arc::new(Mutex::new(std::io::stdout())),
        declared: declared(k),
        write_ids: HashSet::new(),
    };
    if matches!(fault, "timeout" | "ignore_shutdown") {
        // SAFETY: setting a signal disposition to SIG_IGN has no handler code.
        unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
    }
    if fault == "stderr_flood" {
        let mut err = std::io::stderr();
        let _ = err.write_all("fixture-log\n".repeat(32768).as_bytes());
        let _ = err.flush();
    }
    if std::env::args().skip(1).collect::<Vec<_>>() == ["--shape"] {
        server.observe(json!({"kind":"shape"}));
        emit(&server.out, &server.shape());
        return;
    }
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut line = String::new();
        if input.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        server.instrument(&req);
        let method = req["method"].as_str().unwrap_or("");
        if method == "notifications/cancelled" {
            eprintln!("cancelled request");
            continue;
        }
        if req.get("id").is_none() {
            continue;
        }
        if fault == "huge_stdout" {
            let mut out = server.out.lock().unwrap();
            let _ = writeln!(out, "{}", "x".repeat(1024 * 1024 + 1));
            let _ = out.flush();
            continue;
        }
        match method {
            "initialize" => server.initialize(&req),
            "tools/list" => server.tools(&req),
            "tools/call" => server.call(&req, &mut input),
            _ => emit(
                &server.out,
                &json!({"jsonrpc":"2.0","id":req["id"],"error":{"code":-32601,"message":"unknown method"}}),
            ),
        }
    }
    if matches!(fault, "ignore_shutdown" | "timeout") {
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}
