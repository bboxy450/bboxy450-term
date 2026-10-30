// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Newline-delimited JSON-RPC MCP over stdio. The reader thread admits
//! requests and cancellations; one worker owns the plugin and answers them in
//! order. Adapted from bboxy450-term `plugins/gitkb/adapter.py` `serve()`.
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use crate::{
    Ctx, HOST_FIELDS, Init, MAX_FRAME, MAX_PENDING, MAX_RESULT, PROTOCOL_VERSION, Plugin, Refusal,
    encoded,
};

type Pending = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

/// Entry point for a plugin binary: `--shape`, or serve stdio. The host runs
/// inspection with no arguments and a closed stdin, and serving with no
/// arguments and an open pipe.
pub fn run<P: Plugin>() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["--shape"] => print_shape::<P>(),
        [] if stdin_is_terminal_or_closed() => print_shape::<P>(),
        [] | ["--serve"] | ["serve"] | ["--stdio"] => {
            serve::<P>(std::io::stdin().lock(), std::io::stdout());
        }
        _ => {
            eprintln!("use --shape or --serve");
            std::process::exit(2);
        }
    }
}

fn print_shape<P: Plugin>() {
    println!("{}", encoded(&P::manifest().shape()));
}

fn stdin_is_terminal_or_closed() -> bool {
    // SAFETY: isatty, a zero-timeout poll and FIONREAD only inspect descriptor 0.
    unsafe {
        if libc::isatty(0) == 1 {
            return true;
        }
        let mut fd = libc::pollfd {
            fd: 0,
            events: libc::POLLIN,
            revents: 0,
        };
        if libc::poll(&mut fd, 1, 0) <= 0 {
            return false;
        }
        // macOS cannot poll devices such as /dev/null: nothing will arrive.
        if fd.revents & libc::POLLNVAL != 0 {
            return true;
        }
        if fd.revents & (libc::POLLIN | libc::POLLHUP) == 0 {
            return false;
        }
        // Readable with zero pending bytes is EOF: a closed pipe. Pending
        // bytes mean a client spoke first; nothing is consumed here. (recv
        // with MSG_PEEK fails with ENOTSOCK on the pipes hosts use.)
        let mut pending: libc::c_int = 0;
        if libc::ioctl(0, libc::FIONREAD, &raw mut pending) == 0 {
            return pending == 0;
        }
        fd.revents & libc::POLLHUP != 0 && fd.revents & libc::POLLIN == 0
    }
}

/// Serve requests from `input` until EOF; replies go to `output`.
pub fn serve<P: Plugin>(input: impl Read, output: impl Write + Send + 'static) {
    let output = Arc::new(Mutex::new(output));
    let pending: Pending = Arc::default();
    let (tx, rx) = mpsc::channel::<(Value, Arc<AtomicBool>)>();
    let worker = {
        let output = output.clone();
        let pending = pending.clone();
        std::thread::spawn(move || {
            let mut state = Worker::<P> {
                plugin: None,
                manifest: P::manifest(),
            };
            for (request, cancelled) in rx {
                let id = request.get("id").cloned().unwrap_or(Value::Null);
                let reply = catch_unwind(AssertUnwindSafe(|| state.process(&request, cancelled)))
                    .unwrap_or_else(|_| {
                        failure(
                            &request,
                            Refusal::new("internal_error", "the plugin panicked"),
                        )
                    });
                send(
                    &output,
                    json!({"jsonrpc": "2.0", "id": id, &reply.0: reply.1}),
                );
                pending.lock().unwrap().remove(&key(&id));
            }
        })
    };
    let mut reader = BufReader::new(input);
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = reader
            .by_ref()
            .take(MAX_FRAME as u64 + 1)
            .read_until(b'\n', &mut line);
        match read {
            Ok(0) | Err(_) => break,
            Ok(_) if line.len() > MAX_FRAME => {
                send(
                    &output,
                    json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"frame_too_large"}}),
                );
                break;
            }
            Ok(_) => (),
        }
        let request: Value = match serde_json::from_slice(&line) {
            Ok(value @ Value::Object(_)) if value["jsonrpc"] == "2.0" => value,
            _ => {
                send(
                    &output,
                    json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"parse error"}}),
                );
                continue;
            }
        };
        if request["method"] == "notifications/cancelled" {
            if let Some(flag) = pending
                .lock()
                .unwrap()
                .get(&key(&request["params"]["requestId"]))
            {
                flag.store(true, Ordering::Relaxed);
            }
            continue;
        }
        let Some(id) = request.get("id").cloned() else {
            continue;
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        {
            let mut admitted = pending.lock().unwrap();
            if admitted.len() >= MAX_PENDING || admitted.contains_key(&key(&id)) {
                drop(admitted);
                send(
                    &output,
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":"busy_or_duplicate_id"}}),
                );
                continue;
            }
            admitted.insert(key(&id), cancelled.clone());
        }
        if tx.send((request, cancelled)).is_err() {
            break;
        }
    }
    drop(tx);
    let _ = worker.join();
}

fn key(id: &Value) -> String {
    encoded(id)
}

fn send(output: &Mutex<impl Write>, value: Value) {
    let mut output = output.lock().unwrap();
    let _ = writeln!(output, "{}", encoded(&value));
    let _ = output.flush();
}

/// A reply field (`result` or `error`) and its value.
type Reply = (String, Value);

fn failure(request: &Value, refusal: Refusal) -> Reply {
    if request["method"] == "tools/call" {
        (
            "result".into(),
            json!({"content":[{"type":"text","text":refusal.to_string()}],
                   "structuredContent":refusal.value(),"isError":true}),
        )
    } else {
        let code = if refusal.code == "unknown_resource" {
            -32002
        } else {
            -32602
        };
        (
            "error".into(),
            json!({"code":code,"message":refusal.to_string(),"data":refusal.value()}),
        )
    }
}

struct Worker<P> {
    plugin: Option<P>,
    manifest: crate::Manifest,
}

impl<P: Plugin> Worker<P> {
    fn process(&mut self, request: &Value, cancelled: Arc<AtomicBool>) -> Reply {
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let result = match request["method"].as_str().unwrap_or("") {
            "initialize" => self.initialize(params, cancelled),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(self.tools()),
            "tools/call" => self.call(&params, cancelled),
            "resources/list" => self.bound().map(|_| self.resources()),
            "resources/read" => self.bound().and_then(|_| self.read_resource(&params)),
            _ => {
                return (
                    "error".into(),
                    json!({"code":-32601,"message":"method not found"}),
                );
            }
        };
        match result {
            Ok(value) => ("result".into(), value),
            Err(refusal) => failure(request, refusal),
        }
    }

    fn bound(&self) -> Result<(), Refusal> {
        if self.plugin.is_some() {
            Ok(())
        } else {
            Err(Refusal::new(
                "not_initialized",
                "initialize.root must bind this process first",
            ))
        }
    }

    fn initialize(&mut self, params: Value, cancelled: Arc<AtomicBool>) -> Result<Value, Refusal> {
        if self.plugin.is_some() {
            return Err(Refusal::new(
                "already_initialized",
                "one process binds one root",
            ));
        }
        let init = Init {
            root: params["root"].as_str().map(str::to_owned),
            fixture: params["fixture"] == json!({"version": 1, "kind": "data_source_conformance"}),
            params,
        };
        let ctx = Ctx::new(cancelled, P::initialize_budget(&init));
        self.plugin = Some(P::initialize(init, &ctx)?);
        let contracts: serde_json::Map<String, Value> = self
            .manifest
            .contracts
            .iter()
            .map(|s| (s.contract.name().to_owned(), s.contract.version().into()))
            .collect();
        let mut bboxy = json!({"shape_version": crate::SHAPE_VERSION, "contracts": contracts,
                               "concurrent": false, "trace": false});
        if self.manifest.signed_writer {
            bboxy["signed_writer"] = "1.0".into();
        }
        Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "serverInfo": {"name": self.manifest.name, "version": self.manifest.version},
            "capabilities": {"tools": {}, "resources": {}, "experimental": {"bboxy450": bboxy}},
        }))
    }

    fn tools(&self) -> Value {
        let mut tools: Vec<Value> = self
            .manifest
            .tools()
            .into_iter()
            .map(|(name, contract, method)| {
                json!({
                    "name": name,
                    "description": format!("{} {method}", self.manifest.name),
                    "inputSchema": contract.method_schema(&method, false).unwrap_or(Value::Null),
                    "outputSchema": contract.method_schema(&method, true).unwrap_or(Value::Null),
                })
            })
            .collect();
        tools.extend(self.manifest.tools.iter().map(|tool| {
            json!({
                "name": tool.wire_name(),
                "description": tool.description,
                "inputSchema": tool.input_schema,
                "outputSchema": tool.output_schema,
            })
        }));
        json!({ "tools": tools })
    }

    fn call(&mut self, params: &Value, cancelled: Arc<AtomicBool>) -> Result<Value, Refusal> {
        self.bound()?;
        let name = params["name"].as_str().unwrap_or("");
        let args = match params.get("arguments") {
            None | Some(Value::Null) => json!({}),
            Some(value @ Value::Object(_)) => value.clone(),
            Some(_) => {
                return Err(Refusal::new(
                    "invalid_arguments",
                    "arguments must be an object",
                ));
            }
        };
        if HOST_FIELDS.iter().any(|field| args.get(*field).is_some()) {
            return Err(Refusal::new(
                "host_field_in_arguments",
                "root and authority belong to initialize and the host",
            ));
        }
        let plugin = self.plugin.as_mut().expect("bound");
        let result = if name == "health" {
            plugin.health(&Ctx::new(cancelled, budget(crate::DEFAULT_TIMEOUT_MS)))?
        } else if let Some(tool) = self.manifest.declared_tool(name) {
            let ctx = Ctx::new(cancelled, budget(tool.timeout_ms));
            let declared = tool.name.clone();
            plugin.call_tool(&declared, args, &ctx)?
        } else {
            let (contract, method) = self
                .manifest
                .route(name)
                .ok_or_else(|| Refusal::new("unknown_method", "unknown tool"))?;
            let ctx = Ctx::new(
                cancelled,
                budget(self.manifest.timeout_ms(contract, method)),
            );
            let method = method.to_owned();
            plugin.call(contract, &method, args, &ctx)?
        };
        let text = encoded(&result);
        if text.len() > MAX_RESULT {
            return Err(Refusal::new(
                "response_too_large",
                "serialized result exceeds 256 KiB; use a smaller page",
            ));
        }
        Ok(
            json!({"content":[{"type":"text","text":text}],"structuredContent":result,"isError":false}),
        )
    }

    fn resources(&self) -> Value {
        let resources: Vec<Value> = self
            .manifest
            .skills
            .iter()
            .map(|skill| {
                json!({"uri": self.manifest.skill_uri(skill), "name": skill.name,
                       "description": skill.description, "mimeType": "text/markdown"})
            })
            .collect();
        json!({ "resources": resources })
    }

    fn read_resource(&self, params: &Value) -> Result<Value, Refusal> {
        // Exact embedded URIs only: no decoding, cwd lookup or filesystem fallback.
        let uri = params["uri"].as_str().unwrap_or("");
        let skill = self
            .manifest
            .skills
            .iter()
            .find(|skill| self.manifest.skill_uri(skill) == uri)
            .ok_or_else(|| {
                Refusal::new("unknown_resource", "only embedded skill URIs are available")
            })?;
        Ok(json!({"contents":[{"uri":uri,"mimeType":"text/markdown","text":skill.content}]}))
    }
}

/// Answer just inside the declared budget so the host sees a refusal, not a
/// transport timeout: 95%, which leaves 100 ms of a 2 s command.
fn budget(timeout_ms: u64) -> Duration {
    Duration::from_millis(timeout_ms * 95 / 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Contract, Manifest, Skill};
    use std::io::Cursor;

    struct Echo {
        root: String,
    }

    impl Plugin for Echo {
        fn manifest() -> Manifest {
            Manifest::new("bboxy450-term-repo_graph-echo", "Echo", "Echo plugin")
                .contract(Contract::RepoGraph)
                .skill(Skill {
                    name: "echo".into(),
                    description: "Echo skill".into(),
                    content: "# Echo\n".into(),
                })
                .tool(
                    "echo",
                    "Echo the arguments",
                    json!({"type":"object","properties":{"text":{"type":"string"}}}),
                    json!({"type":"object"}),
                    crate::RoleClass::Worker,
                )
        }
        fn call_tool(&mut self, name: &str, args: Value, ctx: &Ctx) -> Result<Value, Refusal> {
            assert_eq!(name, "echo");
            Ok(
                json!({"echo": args, "root": self.root, "remaining": ctx.remaining() > Duration::ZERO}),
            )
        }
        fn initialize(init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
            let root = init
                .root
                .ok_or_else(|| Refusal::new("invalid_knowledge_root", "root required"))?;
            Ok(Echo { root })
        }
        fn call(
            &mut self,
            _c: Contract,
            method: &str,
            args: Value,
            ctx: &Ctx,
        ) -> Result<Value, Refusal> {
            match method {
                "describe" if args["panic"] == true => panic!("boom"),
                "describe" if args["sleep_ms"].is_u64() => {
                    std::thread::sleep(Duration::from_millis(
                        args["sleep_ms"].as_u64().unwrap_or(0),
                    ));
                    Ok(json!({}))
                }
                "describe" => Ok(
                    json!({"root": self.root, "args": args, "remaining": ctx.remaining() > Duration::ZERO}),
                ),
                _ => Err(Refusal::new("ambiguous", "two").with("candidates", json!(["a", "b"]))),
            }
        }
    }

    fn session(lines: &[Value]) -> Vec<Value> {
        let input: String = lines.iter().map(|l| encoded(l) + "\n").collect();
        let output = Arc::new(Mutex::new(Vec::<u8>::new()));
        struct Sink(Arc<Mutex<Vec<u8>>>);
        impl Write for Sink {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        serve::<Echo>(Cursor::new(input), Sink(output.clone()));
        let bytes = output.lock().unwrap().clone();
        String::from_utf8(bytes)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn call(id: u64, name: &str, args: Value) -> Value {
        json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}})
    }

    #[test]
    fn serves_initialize_tools_resources_and_refusals() {
        let replies = session(&[
            call(1, "repo_graph_describe", json!({})),
            json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"root":"/r"}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}),
            call(4, "repo_graph_describe", json!({"x": 1})),
            call(5, "repo_graph_changes_since", json!({})),
            call(6, "repo_graph_describe", json!({"root": "/elsewhere"})),
            call(7, "repo_graph_describe", json!({"panic": true})),
        ]);
        let declared = session(&[
            json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"root":"/r"}}),
            call(13, "tool_echo", json!({"text": "hi"})),
            call(14, "tool_unknown", json!({})),
            call(15, "tool_echo", json!({"root": "/elsewhere"})),
        ]);
        // At most MAX_PENDING requests are admitted at once; a second session
        // keeps every request below that bound.
        let more = session(&[
            json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"root":"/r"}}),
            json!({"jsonrpc":"2.0","id":8,"method":"resources/read","params":{"uri":"echo://skills/echo/SKILL.md"}}),
            json!({"jsonrpc":"2.0","id":9,"method":"resources/read","params":{"uri":"echo://skills/echo/../SKILL.md"}}),
            json!({"jsonrpc":"2.0","id":10,"method":"initialize","params":{"root":"/again"}}),
            json!({"jsonrpc":"2.0","id":11,"method":"nope"}),
            call(12, "health", json!({})),
        ]);
        let replies: Vec<Value> = replies
            .into_iter()
            .chain(more.into_iter().filter(|r| r["id"] != 2))
            .chain(declared.into_iter().filter(|r| r["id"] != 2))
            .collect();
        let by_id = |id: u64| replies.iter().find(|r| r["id"] == id).unwrap().clone();
        assert_eq!(
            by_id(1)["result"]["structuredContent"]["code"],
            "not_initialized"
        );
        assert_eq!(
            by_id(2)["result"]["capabilities"]["experimental"]["bboxy450"]["contracts"]["repo_graph"],
            "1.0"
        );
        let listed = by_id(3)["result"]["tools"].as_array().unwrap().clone();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[2]["name"], "tool_echo");
        assert_eq!(
            listed[2]["inputSchema"]["properties"]["text"]["type"],
            "string"
        );
        let echoed = by_id(13)["result"]["structuredContent"].clone();
        assert_eq!(echoed["echo"], json!({"text": "hi"}));
        assert_eq!(echoed["root"], "/r");
        assert_eq!(echoed["remaining"], true);
        assert_eq!(
            by_id(14)["result"]["structuredContent"]["code"],
            "unknown_method"
        );
        assert_eq!(
            by_id(15)["result"]["structuredContent"]["code"],
            "host_field_in_arguments"
        );
        let ok = by_id(4)["result"].clone();
        assert_eq!(ok["isError"], false);
        assert_eq!(ok["structuredContent"]["root"], "/r");
        assert_eq!(
            serde_json::from_str::<Value>(ok["content"][0]["text"].as_str().unwrap()).unwrap(),
            ok["structuredContent"]
        );
        assert_eq!(
            by_id(5)["result"]["structuredContent"]["candidates"],
            json!(["a", "b"])
        );
        assert_eq!(
            by_id(6)["result"]["structuredContent"]["code"],
            "host_field_in_arguments"
        );
        assert_eq!(
            by_id(7)["result"]["structuredContent"]["code"],
            "internal_error"
        );
        assert_eq!(by_id(8)["result"]["contents"][0]["text"], "# Echo\n");
        assert_eq!(by_id(9)["error"]["code"], -32002);
        assert_eq!(by_id(10)["error"]["data"]["code"], "already_initialized");
        assert_eq!(by_id(11)["error"]["code"], -32601);
        assert_eq!(by_id(12)["result"]["structuredContent"]["healthy"], true);
    }

    #[test]
    fn rejects_parse_errors_and_requests_beyond_the_pending_bound() {
        let replies = session(&[json!({"jsonrpc":"1.0","id":1}), json!("not an object")]);
        assert!(replies.iter().all(|r| r["error"]["code"] == -32700));
        // A slow call holds the worker, so the flood behind it must exceed
        // MAX_PENDING while the reader admits it.
        let mut flood = vec![
            json!({"jsonrpc":"2.0","id":100,"method":"initialize","params":{"root":"/r"}}),
            call(101, "repo_graph_describe", json!({"sleep_ms": 300})),
        ];
        flood.extend((1..=20).map(|id| json!({"jsonrpc":"2.0","id":id,"method":"ping"})));
        let replies = session(&flood);
        assert_eq!(replies.len(), 22);
        assert!(
            replies
                .iter()
                .any(|r| r["error"]["message"] == "busy_or_duplicate_id")
        );
        assert!(replies.iter().any(|r| r["result"] == json!({})));
    }

    #[test]
    fn budget_answers_inside_the_declared_timeout() {
        assert_eq!(budget(2_000), Duration::from_millis(1_900));
        assert_eq!(budget(15_000), Duration::from_millis(14_250));
    }
}
