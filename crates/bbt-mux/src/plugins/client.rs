// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One bounded MCP stdio connection; deadlines never follow progress messages.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use bbt_plugin::{Refusal, policy::allowed_env_name, shape::SHAPE_VERSION};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout, timeout_at};

use super::{failure, trust::FixtureAccount};
use crate::mcp::bridge::{MAX_FRAME, VERSIONS};

const STDERR_BYTES: usize = 32 * 1024;
const HANDSHAKE: Duration = Duration::from_secs(2);

#[derive(Default)]
pub(super) struct Stderr {
    pub text: String,
    pub truncated: bool,
}

pub(super) struct Client {
    child: Child,
    input: Option<ChildStdin>,
    /// Taken by [`Client::into_shared`]; the demultiplexer then owns stdout.
    output: Option<BufReader<ChildStdout>>,
    stderr: Option<JoinHandle<Stderr>>,
    next: u64,
    pub pid: u32,
    shape_timeout: Duration,
    call_limit: Duration,
    handshake_timeout: Duration,
    #[cfg(test)]
    spawned_environment_names: Vec<String>,
}

impl Client {
    pub fn spawn(fixture: &FixtureAccount, shape: bool) -> Result<Self, Refusal> {
        let home = std::path::Path::new(&fixture.record.data_dir).join("home");
        for directory in [&home, &home.join("tmp")] {
            std::fs::create_dir_all(directory)
                .map_err(|e| failure("fixture_store_unavailable", e))?;
        }
        Self::spawn_with_home(
            fixture.executable()?,
            std::path::Path::new(&fixture.record.data_dir),
            shape,
            HANDSHAKE,
            Some(&home),
        )
    }

    // Both callers first reread their account authority and verify its pinned copy.
    pub(super) fn spawn_with_home(
        executable: std::path::PathBuf,
        data_dir: &std::path::Path,
        shape: bool,
        shape_timeout: Duration,
        home: Option<&std::path::Path>,
    ) -> Result<Self, Refusal> {
        let mut command = Command::new(executable);
        command
            .current_dir(data_dir)
            .env_clear()
            .envs(std::env::vars_os().filter(|(key, _)| key.to_str().is_some_and(allowed_env_name)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(home) = home {
            command.env("HOME", home).env("TMPDIR", home.join("tmp"));
        }
        if shape {
            command.arg("--shape");
        }
        #[cfg(test)]
        let spawned_environment_names = command
            .as_std()
            .get_envs()
            .filter_map(|(name, value)| value.map(|_| name.to_string_lossy().into_owned()))
            .collect();
        let mut child = command
            .spawn()
            .map_err(|e| failure("plugin_spawn_failed", e))?;
        let pid = child.id().expect("new child has a PID");
        let input = child.stdin.take();
        let output = BufReader::new(child.stdout.take().expect("stdout is piped"));
        let mut stderr = child.stderr.take().expect("stderr is piped");
        let capture = tokio::spawn(async move {
            let mut bytes = Vec::new();
            let mut part = [0; 4096];
            let mut truncated = false;
            while let Ok(count) = stderr.read(&mut part).await {
                if count == 0 {
                    break;
                }
                let take = count.min(STDERR_BYTES.saturating_sub(bytes.len()));
                bytes.extend_from_slice(&part[..take]);
                truncated |= take < count;
            }
            Stderr {
                text: String::from_utf8_lossy(&bytes).into_owned(),
                truncated,
            }
        });
        Ok(Self {
            child,
            input,
            output: Some(output),
            stderr: Some(capture),
            next: 0,
            pid,
            shape_timeout,
            call_limit: HANDSHAKE,
            handshake_timeout: HANDSHAKE,
            #[cfg(test)]
            spawned_environment_names,
        })
    }

    pub async fn shape(&mut self) -> Result<Value, Refusal> {
        // --shape is one JSON document and may be pretty-printed; MCP is lines.
        self.input.take();
        let read = async {
            let mut bytes = Vec::new();
            let mut part = [0; 4096];
            let output = self
                .output
                .as_mut()
                .ok_or_else(|| failure("shape_io", "stdout belongs to the demultiplexer"))?;
            loop {
                let count = output
                    .read(&mut part)
                    .await
                    .map_err(|e| failure("shape_io", e))?;
                if count == 0 {
                    break;
                }
                if bytes.len() + count > MAX_FRAME {
                    return Err(failure(
                        "frame_too_large",
                        "shape exceeds the MCP frame bound",
                    ));
                }
                bytes.extend_from_slice(&part[..count]);
            }
            let status = self
                .child
                .wait()
                .await
                .map_err(|e| failure("shape_io", e))?;
            if !status.success() {
                return Err(failure("shape_process_failed", status));
            }
            serde_json::from_slice(&bytes).map_err(|e| failure("invalid_shape_json", e))
        };
        timeout(self.shape_timeout, read)
            .await
            .map_err(|_| failure("deadline_exceeded", "--shape exceeded its fixed deadline"))?
    }

    async fn send(&mut self, value: &Value) -> Result<(), Refusal> {
        let mut bytes = serde_json::to_vec(value).map_err(|e| failure("protocol_error", e))?;
        if bytes.len() > MAX_FRAME {
            return Err(failure(
                "frame_too_large",
                "outbound MCP frame exceeds the bound",
            ));
        }
        bytes.push(b'\n');
        let input = self
            .input
            .as_mut()
            .ok_or_else(|| failure("stdin_closed", "plugin stdin is closed"))?;
        input
            .write_all(&bytes)
            .await
            .map_err(|e| failure("plugin_io", e))?;
        input.flush().await.map_err(|e| failure("plugin_io", e))
    }

    async fn frame(&mut self) -> Result<Value, Refusal> {
        let output = self
            .output
            .as_mut()
            .ok_or_else(|| failure("plugin_io", "stdout belongs to the demultiplexer"))?;
        read_frame(output).await
    }

    pub async fn request(
        &mut self,
        method: &str,
        params: Value,
        budget: Duration,
    ) -> Result<Value, Refusal> {
        self.next += 1;
        let id = self.next;
        let deadline = Instant::now() + budget;
        let call = async {
            self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
                .await?;
            loop {
                let value = self.frame().await?;
                if value["jsonrpc"] != "2.0" {
                    return Err(failure("protocol_error", "missing JSON-RPC 2.0 marker"));
                }
                if let Some(notification) = value.get("method").and_then(Value::as_str) {
                    if value.get("id").is_some()
                        || !matches!(
                            notification,
                            "notifications/progress" | "notifications/message"
                        )
                    {
                        return Err(failure("unexpected_server_message", notification));
                    }
                    tokio::task::yield_now().await;
                    continue;
                }
                if value.get("id") != Some(&json!(id)) {
                    return Err(failure(
                        "request_id_mismatch",
                        "response does not match the outstanding request",
                    ));
                }
                if value.get("error").is_some() {
                    if value.get("result").is_some() {
                        return Err(failure(
                            "protocol_error",
                            "response has both error and result",
                        ));
                    }
                    return Err(failure("jsonrpc_error", "server returned a JSON-RPC error"));
                }
                return value
                    .get("result")
                    .cloned()
                    .ok_or_else(|| failure("protocol_error", "response has no result"));
            }
        };
        match timeout_at(deadline, call).await {
            Ok(result) => result,
            Err(_) => {
                // MCP initialization must not be cancelled. No timed-out call is replayed.
                if method != "initialize" {
                    let _=timeout(Duration::from_millis(100),self.send(&json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":id,"reason":"fixed deadline exceeded"}}))).await;
                }
                Err(failure(
                    "deadline_exceeded",
                    format!("{method} exceeded its fixed deadline"),
                ))
            }
        }
    }

    pub async fn initialize(&mut self, fixture: &FixtureAccount) -> Result<(), Refusal> {
        let shape = fixture.record.shape.as_ref().ok_or_else(|| {
            failure(
                "shape_not_cached",
                "trust must cache a validated shape first",
            )
        })?;
        let root = fixture.knowledge_root()?;
        self.initialize_parameters(shape, root.as_deref(), "bboxy450-plugin-check", true, false)
            .await
    }

    /// A connection is bound to one host-selected root once, never by method arguments.
    pub(super) async fn initialize_root(
        &mut self,
        shape: &bbt_plugin::shape::Shape,
        root: Option<&std::path::Path>,
        client_name: &str,
    ) -> Result<(), Refusal> {
        self.initialize_parameters(shape, root, client_name, false, false)
            .await
    }
    /// Like [`Client::initialize_root`], for a connection that
    /// [`Client::into_shared`] will demultiplex. `concurrent` tells the plugin
    /// that the host may keep several requests in flight.
    pub(super) async fn initialize_shared(
        &mut self,
        shape: &bbt_plugin::shape::Shape,
        root: &std::path::Path,
        client_name: &str,
        concurrent: bool,
    ) -> Result<(), Refusal> {
        self.initialize_parameters(shape, Some(root), client_name, false, concurrent)
            .await
    }
    async fn initialize_parameters(
        &mut self,
        shape: &bbt_plugin::shape::Shape,
        root: Option<&std::path::Path>,
        client_name: &str,
        fixture: bool,
        concurrent: bool,
    ) -> Result<(), Refusal> {
        let contracts: serde_json::Map<String, Value> = shape
            .systems
            .iter()
            .map(|s| (s.contract.clone(), json!(s.contract_version)))
            .collect();
        let mut params = json!({"protocolVersion":VERSIONS[0],"capabilities":{"experimental":{"bboxy450":{"shape_version":SHAPE_VERSION,"contracts":contracts,"concurrent":concurrent,"trace":false}}},"clientInfo":{"name":client_name,"version":env!("CARGO_PKG_VERSION")}});
        if let Some(root) = root {
            params["root"] = json!(
                root.to_str()
                    .ok_or_else(|| failure("invalid_knowledge_root", "root path must be UTF-8"))?
            );
        }
        let fixture_bootstrap = fixture && root.is_some() && shape.fixture_bootstrap;
        if fixture_bootstrap {
            params["fixture"] = json!({"version":1,"kind":"data_source_conformance"});
        }
        // A real disposable GitKB initialization measured 6.077 s (including
        // its WAL checkpoint). This setup cap never applies to production calls.
        let budget = if fixture_bootstrap {
            Duration::from_secs(15)
        } else {
            self.handshake_timeout
        };
        let result = self.request("initialize", params, budget).await?;
        if !result["protocolVersion"]
            .as_str()
            .is_some_and(|v| VERSIONS.contains(&v))
        {
            return Err(failure(
                "mcp_version_mismatch",
                "server selected an unsupported MCP version",
            ));
        }
        let capabilities = &result["capabilities"]["experimental"]["bboxy450"];
        let version = capabilities["shape_version"].as_str().ok_or_else(|| {
            failure(
                "missing_plugin_capabilities",
                "server omitted bboxy450 shape_version",
            )
        })?;
        bbt_plugin::shape::Version::compatible(version, SHAPE_VERSION)?;
        for system in &shape.systems {
            let version = capabilities["contracts"][&system.contract]
                .as_str()
                .ok_or_else(|| failure("missing_plugin_capabilities", &system.contract))?;
            bbt_plugin::shape::Version::compatible(version, &system.contract_version)?;
        }
        timeout(
            self.handshake_timeout,
            self.send(&json!({"jsonrpc":"2.0","method":"notifications/initialized"})),
        )
        .await
        .map_err(|_| failure("deadline_exceeded", "initialized notification blocked"))??;
        Ok(())
    }

    pub(super) fn call_limit(&self) -> Duration {
        self.call_limit
    }

    /// Host configuration only; tests can use a hang guard without changing production defaults.
    pub(super) fn configure_deadlines(&mut self, deadline: Duration) {
        self.call_limit = deadline;
        self.handshake_timeout = deadline;
    }

    pub async fn call(
        &mut self,
        name: &str,
        args: Value,
        budget: Duration,
    ) -> Result<Value, Refusal> {
        let result = self
            .request("tools/call", json!({"name":name,"arguments":args}), budget)
            .await?;
        if result["isError"] == true {
            let code = result["structuredContent"]["code"]
                .as_str()
                .unwrap_or("unspecified");
            let message = result["structuredContent"]["message"]
                .as_str()
                .filter(|message| !message.is_empty())
                .map(|message| format!(": {}", message.chars().take(1024).collect::<String>()))
                .unwrap_or_default();
            return Err(failure(
                "command_error",
                format!(
                    "{name} returned isError ({}){message}",
                    code.chars().take(256).collect::<String>()
                ),
            ));
        }
        result
            .get("structuredContent")
            .cloned()
            .ok_or_else(|| failure("missing_structured_content", name))
    }

    pub async fn shutdown(mut self) -> Stderr {
        self.input.take();
        if timeout(Duration::from_millis(100), self.child.wait())
            .await
            .is_err()
        {
            if self.child.try_wait().ok().flatten().is_none() {
                // This is the PID of our own unreaped direct child, never a name lookup.
                unsafe {
                    libc::kill(self.pid as i32, libc::SIGTERM);
                }
            }
            if timeout(Duration::from_millis(100), self.child.wait())
                .await
                .is_err()
            {
                let _ = self.child.start_kill();
                let _ = self.child.wait().await;
            }
        }
        let mut capture = self.stderr.take().expect("one stderr capture");
        match timeout(Duration::from_millis(100), &mut capture).await {
            Ok(Ok(captured)) => captured,
            _ => {
                capture.abort();
                Stderr {
                    truncated: true,
                    ..Default::default()
                }
            }
        }
    }
}

type Waiter = oneshot::Sender<Result<Value, Refusal>>;

/// Outstanding request IDs. `issued` separates a late reply to a cancelled
/// call (ignored) from an ID the host never sent (a protocol failure).
#[derive(Default)]
struct Routes {
    waiting: HashMap<u64, Waiter>,
    issued: u64,
    failed: Option<Refusal>,
}

type Writer = Arc<tokio::sync::Mutex<Option<ChildStdin>>>;

/// A demultiplexed connection: one reader task routes each response to its
/// waiting caller by ID, so several requests can be in flight. The owner
/// bounds how many; this type never queues behind a busy plugin.
pub(super) struct Shared {
    client: tokio::sync::Mutex<Option<Client>>,
    writer: Writer,
    routes: Arc<StdMutex<Routes>>,
    reader: JoinHandle<()>,
}

/// Serves one plugin-originated request `(method, params, deadline)` on a
/// blocking thread. A refusal's code reaches the plugin as `error.data.code`.
pub(super) type HostHandler =
    Arc<dyn Fn(&str, Value, std::time::Instant) -> Result<Value, Refusal> + Send + Sync>;
/// Plugin-originated requests served at once per connection; more are
/// refused as `host_api_busy`, never queued.
pub(super) const MAX_HOST_CALLS: usize = 4;
/// Budget of one plugin-originated request.
pub(super) const HOST_CALL_BUDGET: Duration = Duration::from_secs(2);

/// Answer one plugin-originated request on its own task, so the reader keeps
/// reading: the plugin may be waiting for this answer before it answers a
/// host call that is in flight on the same connection.
fn host_request(
    id: Value,
    method: String,
    params: Value,
    writer: Writer,
    handler: Option<HostHandler>,
    calls: Arc<tokio::sync::Semaphore>,
) {
    let permit = calls.try_acquire_owned();
    tokio::spawn(async move {
        let outcome = match (handler, permit) {
            (None, _) => {
                let reply = json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"host API not available"}});
                let _ = write_frame(&writer, &reply).await;
                return;
            }
            (Some(_), Err(_)) => Err(Refusal::new(
                "host_api_busy",
                format!("at most {MAX_HOST_CALLS} host requests at once"),
            )),
            (Some(handler), Ok(permit)) => {
                let deadline = std::time::Instant::now() + HOST_CALL_BUDGET;
                // The permit lives as long as the work, even past a timeout.
                let work = tokio::task::spawn_blocking(move || {
                    let _permit = permit;
                    handler(&method, params, deadline)
                });
                match timeout(HOST_CALL_BUDGET, work).await {
                    Ok(Ok(outcome)) => outcome,
                    Ok(Err(_)) => Err(Refusal::new("internal_error", "host request failed")),
                    Err(_) => Err(Refusal::new(
                        "deadline_exceeded",
                        "host request exceeded its budget",
                    )),
                }
            }
        };
        let error = |reason: &Refusal| {
            let code = if reason.code == "unknown_host_api" {
                -32601
            } else {
                -32000
            };
            json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":reason.message,
                "data":{"code":reason.code,"message":reason.message}}})
        };
        let reply = match &outcome {
            Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
            Err(reason) => error(reason),
        };
        if let Err(reason) = write_frame(&writer, &reply).await
            && reason.code == "frame_too_large"
        {
            let too_large = Refusal::new("response_too_large", "host result exceeds the frame");
            let _ = write_frame(&writer, &error(&too_large)).await;
        }
    });
}

async fn write_frame(writer: &Writer, value: &Value) -> Result<(), Refusal> {
    let mut bytes = serde_json::to_vec(value).map_err(|e| failure("protocol_error", e))?;
    if bytes.len() > MAX_FRAME {
        return Err(failure(
            "frame_too_large",
            "outbound MCP frame exceeds the bound",
        ));
    }
    bytes.push(b'\n');
    let mut input = writer.lock().await;
    let input = input
        .as_mut()
        .ok_or_else(|| failure("stdin_closed", "plugin stdin is closed"))?;
    input
        .write_all(&bytes)
        .await
        .map_err(|e| failure("plugin_io", e))?;
    input.flush().await.map_err(|e| failure("plugin_io", e))
}

fn response(value: &Value) -> Result<Value, Refusal> {
    if value.get("error").is_some() {
        if value.get("result").is_some() {
            return Err(failure(
                "protocol_error",
                "response has both error and result",
            ));
        }
        return Err(failure("jsonrpc_error", "server returned a JSON-RPC error"));
    }
    value
        .get("result")
        .cloned()
        .ok_or_else(|| failure("protocol_error", "response has no result"))
}

async fn demultiplex(
    mut output: BufReader<ChildStdout>,
    writer: Writer,
    routes: Arc<StdMutex<Routes>>,
    handler: Option<HostHandler>,
) {
    let calls = Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CALLS));
    let failed = loop {
        let value = match read_frame(&mut output).await {
            Ok(value) => value,
            Err(reason) => break reason,
        };
        if value["jsonrpc"] != "2.0" {
            break failure("protocol_error", "missing JSON-RPC 2.0 marker");
        }
        if let Some(method) = value.get("method").and_then(Value::as_str) {
            // Notifications (progress, logs) never extend a deadline.
            if let Some(id) = value.get("id") {
                host_request(
                    id.clone(),
                    method.to_owned(),
                    value.get("params").cloned().unwrap_or(Value::Null),
                    writer.clone(),
                    handler.clone(),
                    calls.clone(),
                );
            }
            continue;
        }
        let Some(id) = value.get("id").and_then(Value::as_u64) else {
            break failure("request_id_mismatch", "response has no host request ID");
        };
        let waiter = {
            let mut routes = routes.lock().unwrap_or_else(|e| e.into_inner());
            if id == 0 || id > routes.issued {
                None
            } else {
                Some(routes.waiting.remove(&id))
            }
        };
        match waiter {
            None => {
                break failure(
                    "request_id_mismatch",
                    "response does not match an issued request",
                );
            }
            Some(Some(waiter)) => {
                let _ = waiter.send(response(&value));
            }
            // A late reply to a call that already timed out and was cancelled.
            Some(None) => {}
        }
    };
    let waiting = {
        let mut routes = routes.lock().unwrap_or_else(|e| e.into_inner());
        routes.failed.get_or_insert(failed.clone());
        std::mem::take(&mut routes.waiting)
    };
    for (_, waiter) in waiting {
        let _ = waiter.send(Err(failed.clone()));
    }
}

impl Client {
    /// Hand stdout to a reader task. Call after `initialize`; the request IDs
    /// continue from the handshake's. Plugin-originated requests go to
    /// `handler`, or are refused with -32601 without one.
    pub(super) fn into_shared(mut self, handler: Option<HostHandler>) -> Shared {
        let output = self.output.take().expect("stdout not yet demultiplexed");
        let writer: Writer = Arc::new(tokio::sync::Mutex::new(self.input.take()));
        let routes = Arc::new(StdMutex::new(Routes {
            issued: self.next,
            ..Default::default()
        }));
        let reader = tokio::spawn(demultiplex(output, writer.clone(), routes.clone(), handler));
        Shared {
            client: tokio::sync::Mutex::new(Some(self)),
            writer,
            routes,
            reader,
        }
    }
}

impl Shared {
    /// One JSON-RPC request within `budget`. A timed-out call is cancelled and
    /// never replayed; its late reply is dropped by the reader.
    pub async fn request(
        &self,
        method: &str,
        params: Value,
        budget: Duration,
    ) -> Result<Value, Refusal> {
        let deadline = Instant::now() + budget;
        let (sender, receiver) = oneshot::channel();
        let id = {
            let mut routes = self.routes.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(failed) = &routes.failed {
                return Err(failed.clone());
            }
            routes.issued += 1;
            let id = routes.issued;
            routes.waiting.insert(id, sender);
            id
        };
        let forget = || {
            self.routes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .waiting
                .remove(&id);
        };
        let frame = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        match timeout_at(deadline, write_frame(&self.writer, &frame)).await {
            Ok(Ok(())) => {}
            Ok(Err(reason)) => {
                forget();
                return Err(reason);
            }
            Err(_) => {
                forget();
                return Err(failure(
                    "deadline_exceeded",
                    format!("{method} exceeded its fixed deadline"),
                ));
            }
        }
        match timeout_at(deadline, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(self.failed()),
            Err(_) => {
                forget();
                let cancel = json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":id,"reason":"fixed deadline exceeded"}});
                let _ = timeout(
                    Duration::from_millis(100),
                    write_frame(&self.writer, &cancel),
                )
                .await;
                Err(failure(
                    "deadline_exceeded",
                    format!("{method} exceeded its fixed deadline"),
                ))
            }
        }
    }

    fn failed(&self) -> Refusal {
        self.routes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .failed
            .clone()
            .unwrap_or_else(|| failure("plugin_eof", "connection closed"))
    }

    /// False once the reader stopped; the owner then opens a new connection.
    pub fn alive(&self) -> bool {
        !self.reader.is_finished()
            && self
                .routes
                .lock()
                .is_ok_and(|routes| routes.failed.is_none())
    }

    /// Close stdin, fail every waiting call and reap the process. Idempotent.
    pub async fn shutdown(&self) {
        {
            let mut routes = self.routes.lock().unwrap_or_else(|e| e.into_inner());
            let closed = failure("provider_closed", "this plugin connection is closed");
            routes.failed.get_or_insert(closed.clone());
            for (_, waiter) in std::mem::take(&mut routes.waiting) {
                let _ = waiter.send(Err(closed.clone()));
            }
        }
        self.writer.lock().await.take();
        if let Some(client) = self.client.lock().await.take() {
            client.shutdown().await;
        }
        self.reader.abort();
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        // Client::kill_on_drop reaps an un-shut-down process.
        self.reader.abort();
    }
}

async fn read_frame(output: &mut BufReader<ChildStdout>) -> Result<Value, Refusal> {
    let mut bytes = Vec::new();
    loop {
        let part = output
            .fill_buf()
            .await
            .map_err(|e| failure("plugin_io", e))?;
        if part.is_empty() {
            return Err(failure(
                "plugin_eof",
                "stdout ended before a complete response",
            ));
        }
        let newline = part.iter().position(|b| *b == b'\n');
        let take = newline.unwrap_or(part.len());
        if bytes.len() + take > MAX_FRAME {
            return Err(failure(
                "frame_too_large",
                "plugin stdout exceeds the MCP frame bound",
            ));
        }
        bytes.extend_from_slice(&part[..take]);
        output.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            break;
        }
    }
    serde_json::from_slice(&bytes).map_err(|_| {
        failure(
            "non_protocol_stdout",
            "stdout must contain newline JSON-RPC only",
        )
    })
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(capture) = self.stderr.take() {
            capture.abort();
        }
        // Child::kill_on_drop is the cancellation backstop; normal paths reap above.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    // keeps: spec plugins
    #[tokio::test]
    async fn real_stdio_keeps_opaque_signed_bytes_and_only_allowed_environment_names() {
        let files = tempfile::tempdir().unwrap();
        let source = files.path().join("bboxy450-term-data_source-reference");
        fs::write(
            &source,
            include_str!("../../tests/fixtures/plugins/bboxy450-term-data_source-reference"),
        )
        .unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
        let mut account = FixtureAccount::prepare(&source).unwrap();
        let mut descriptor = Client::spawn(&account, true).unwrap();
        let shape = descriptor.shape().await;
        descriptor.shutdown().await;
        let shape = serde_json::from_value(shape.unwrap()).unwrap();
        account.cache_shape(shape).unwrap();
        let mut client = Client::spawn(&account, false).unwrap();
        let spawned_environment_names = client.spawned_environment_names.clone();
        let signed = " \n{\"payload\":\"opaque \\u2603\", \"signature\":\"fixture\"}\n";
        let result=async {
            client.initialize(&account).await?;
            let health=client.call("health",json!({}),CALL_BUDGET).await?;
            let written=client.call("data_source_write",json!({"path":"opaque.md","frontmatter":{"type":"note","signed_record":signed},"body":"verbatim\n"}),CALL_BUDGET).await?;
            Ok::<_,Refusal>((health,written))
        }.await;
        client.shutdown().await;
        let (health, written) = result.unwrap();
        assert!(
            spawned_environment_names
                .iter()
                .all(|name| allowed_env_name(name))
        );
        assert!(
            !spawned_environment_names
                .iter()
                .any(|name| name == "__CF_USER_TEXT_ENCODING")
        );
        assert_eq!(
            health["cwd"],
            fs::canonicalize(&account.record.data_dir)
                .unwrap()
                .to_str()
                .unwrap()
        );
        assert_eq!(
            health["executable"],
            fs::canonicalize(&account.record.copy)
                .unwrap()
                .to_str()
                .unwrap()
        );
        let names = health["environment_names"].as_array().unwrap();
        assert!(names.iter().all(|n| {
            let name = n.as_str().unwrap();
            allowed_env_name(name) || cfg!(target_os = "macos") && name == "__CF_USER_TEXT_ENCODING"
        }));
        assert_eq!(
            written["record"]["frontmatter"]["signed_record"]
                .as_str()
                .unwrap()
                .as_bytes(),
            signed.as_bytes()
        );
    }

    const CALL_BUDGET: Duration = Duration::from_secs(2);

    /// Answers id 2 before id 1, asks the host one question in between and
    /// emits notifications; then replies with an ID it was never sent.
    const DEMUX_SERVER: &str = r#"#!/usr/bin/env python3
import json, sys
def emit(value): print(json.dumps(value), flush=True)
held = None
for line in sys.stdin:
    req = json.loads(line)
    if "id" not in req: continue
    if req["id"] == "plugin-1":
        sys.stderr.write("host answered " + json.dumps(req["error"]) + "\n"); sys.stderr.flush()
        emit({"jsonrpc":"2.0","method":"notifications/message","params":{"level":"info","data":"x"}})
        emit({"jsonrpc":"2.0","id":2,"result":{"order":"second request first"}})
        emit({"jsonrpc":"2.0","id":held,"result":{"order":"first request last"}})
        continue
    if req["params"].get("name") == "stray":
        emit({"jsonrpc":"2.0","id":req["id"]+40,"result":{}}); continue
    if held is None:
        held = req["id"]
        emit({"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken":"t","progress":1}})
        continue
    emit({"jsonrpc":"2.0","id":"plugin-1","method":"host/records.search","params":{}})
"#;

    #[tokio::test]
    async fn demultiplexed_replies_reach_their_callers_and_plugin_requests_are_answered() {
        let files = tempfile::tempdir().unwrap();
        let source = files.path().join("demux-server");
        fs::write(&source, DEMUX_SERVER).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
        let client =
            Client::spawn_with_home(source, files.path(), false, CALL_BUDGET, None).unwrap();
        let shared = client.into_shared(None);
        let call = |name: &'static str| {
            shared.request(
                "tools/call",
                json!({"name":name,"arguments":{}}),
                CALL_BUDGET,
            )
        };
        let (first, second) = tokio::join!(call("one"), async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            call("two").await
        });
        assert_eq!(first.unwrap()["order"], "first request last");
        assert_eq!(second.unwrap()["order"], "second request first");
        assert!(shared.alive());
        let stray = call("stray").await.unwrap_err();
        assert_eq!(stray.code, "request_id_mismatch");
        assert!(!shared.alive());
        assert_eq!(call("after").await.unwrap_err().code, "request_id_mismatch");
        let mut client = shared.client.lock().await;
        let captured = client.take().unwrap().shutdown().await;
        assert!(
            captured
                .text
                .contains(r#"host answered {"code": -32601, "message": "host API not available"}"#),
            "{}",
            captured.text
        );
    }

    /// While serving a call, asks the host `host/<name>` and answers the call
    /// only with the host's answer.
    const CALLBACK_SERVER: &str = r#"#!/usr/bin/env python3
import json, sys
def emit(value): print(json.dumps(value), flush=True)
for line in sys.stdin:
    req = json.loads(line)
    if "method" not in req or "id" not in req: continue
    emit({"jsonrpc":"2.0","id":"cb-"+str(req["id"]),"method":"host/"+req["params"]["name"],"params":{"n":req["id"]}})
    while True:
        answer = json.loads(sys.stdin.readline())
        if answer.get("id") == "cb-"+str(req["id"]): break
    emit({"jsonrpc":"2.0","id":req["id"],"result":answer})
"#;

    // keeps: PluginHostApi — a host request issued while the host waits on
    // that plugin's call is served; the reader never blocks on it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_host_request_during_a_call_is_answered_and_the_call_completes() {
        let files = tempfile::tempdir().unwrap();
        let source = files.path().join("callback-server");
        fs::write(&source, CALLBACK_SERVER).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
        let client =
            Client::spawn_with_home(source, files.path(), false, CALL_BUDGET, None).unwrap();
        let handler: HostHandler = Arc::new(|method, params, _deadline| match method {
            "host/records.search" => {
                std::thread::sleep(Duration::from_millis(100));
                Ok(json!({"served": params["n"]}))
            }
            _ => Err(Refusal::new("host_api_not_granted", method)),
        });
        let shared = client.into_shared(Some(handler));
        let call = |name: &'static str| {
            shared.request(
                "tools/call",
                json!({"name":name,"arguments":{}}),
                CALL_BUDGET,
            )
        };
        let served = call("records.search").await.unwrap();
        let id = served["result"]["served"].as_u64().unwrap();
        assert_eq!(served["id"], format!("cb-{id}"));
        let refused = call("events.publish").await.unwrap();
        assert_eq!(refused["error"]["code"], -32000);
        assert_eq!(refused["error"]["data"]["code"], "host_api_not_granted");
        assert!(shared.alive());
        shared.shutdown().await;
    }

    #[tokio::test]
    async fn progress_cannot_extend_a_deadline_and_cancellation_does_not_replay() {
        let files = tempfile::tempdir().unwrap();
        let source = files.path().join("bboxy450-term-data_source-timeout");
        let bytes =
            include_str!("../../tests/fixtures/plugins/bboxy450-term-data_source-reference")
                .replacen("FAULT = None", "FAULT = 'timeout'", 1);
        fs::write(&source, bytes).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
        let mut account = FixtureAccount::prepare(&source).unwrap();
        let mut descriptor = Client::spawn(&account, true).unwrap();
        let shape = descriptor.shape().await;
        descriptor.shutdown().await;
        account
            .cache_shape(serde_json::from_value(shape.unwrap()).unwrap())
            .unwrap();
        let mut client = Client::spawn(&account, false).unwrap();
        let result = async {
            client.initialize(&account).await?;
            client
                .call(
                    "data_source_resolve",
                    json!({"locator":"#7"}),
                    Duration::from_millis(50),
                )
                .await
        }
        .await;
        let captured = client.shutdown().await;
        assert_eq!(result.unwrap_err().code, "deadline_exceeded");
        assert_eq!(captured.text.matches("fixture resolve attempt").count(), 1);
        assert_eq!(captured.text.matches("cancelled request").count(), 1);
    }
}
