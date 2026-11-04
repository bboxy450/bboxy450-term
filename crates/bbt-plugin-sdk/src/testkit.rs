// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Drive a built plugin executable over stdio, as the host does. Used by
//! plugin conformance tests and read-only probes.
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::encoded;

/// The host's environment allowlist (`bboxy450-plugin` policy).
pub fn allowed_env() -> Vec<(String, String)> {
    std::env::vars()
        .filter(|(k, _)| {
            matches!(k.as_str(), "PATH" | "HOME" | "LANG" | "TMPDIR" | "TERM")
                || k.starts_with("LC_")
        })
        .collect()
}

/// `tools/call` params for `tool` made on behalf of a seat lending `env`
/// (request `_meta` `bboxy450/attribution`, see
/// [`bbt_plugin::policy::ATTRIBUTION_META`]); no `_meta` when `env` is empty.
pub fn attributed(tool: &str, args: Value, env: &BTreeMap<String, String>) -> Value {
    let mut params = json!({"name": tool, "arguments": args});
    if !env.is_empty() {
        params["_meta"] = json!({ bbt_plugin::policy::ATTRIBUTION_META: { "env": env } });
    }
    params
}

/// Print and parse `--shape`.
pub fn shape(plugin: &Path) -> Result<Value, String> {
    let output = Command::new(plugin)
        .arg("--shape")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("--shape exited {}", output.status));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}

/// One plugin process and its reply stream.
pub struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    replies: Receiver<Value>,
    next_id: u64,
    pub data: PathBuf,
}

/// A failed call: the JSON-RPC `error`, or an `isError` tool result's
/// `structuredContent`.
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    Protocol(Value),
    Tool(Value),
    NoReply(String),
}

impl Failure {
    /// The refusal code, wherever it lives.
    pub fn code(&self) -> Option<&str> {
        match self {
            Failure::Protocol(error) => error["data"]["code"].as_str(),
            Failure::Tool(value) => value["code"].as_str(),
            Failure::NoReply(_) => None,
        }
    }
}

impl Client {
    /// Start `plugin` with `cwd` as its private data directory and `env`
    /// as its complete environment.
    pub fn spawn(plugin: &Path, data: &Path, env: &[(String, String)]) -> std::io::Result<Self> {
        std::fs::create_dir_all(data)?;
        let mut child = Command::new(plugin)
            .current_dir(data)
            .env_clear()
            .envs(env.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdout = child.stdout.take().expect("piped stdout");
        let (tx, replies) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str::<Value>(&line)
                    && tx.send(value).is_err()
                {
                    break;
                }
            }
        });
        let stdin = child.stdin.take();
        Ok(Self {
            child,
            stdin,
            replies,
            next_id: 0,
            data: data.to_owned(),
        })
    }

    /// Send one raw message (request or notification).
    pub fn send(&mut self, message: &Value) -> std::io::Result<()> {
        let stdin = self.stdin.as_mut().ok_or(std::io::ErrorKind::BrokenPipe)?;
        writeln!(stdin, "{}", encoded(message))?;
        stdin.flush()
    }

    /// A request and its reply, waiting at most `wait`.
    pub fn request(
        &mut self,
        method: &str,
        params: Value,
        wait: Duration,
    ) -> Result<Value, Failure> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .map_err(|e| Failure::NoReply(e.to_string()))?;
        let deadline = Instant::now() + wait;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let reply = self
                .replies
                .recv_timeout(left)
                .map_err(|_| Failure::NoReply(method.into()))?;
            if reply["id"] != id {
                continue;
            }
            if let Some(error) = reply.get("error") {
                return Err(Failure::Protocol(error.clone()));
            }
            return Ok(reply["result"].clone());
        }
    }

    /// Send every request before reading any reply (pipelined, as a
    /// concurrent host does), then wait at most `wait` for all replies.
    /// Each slot is its request's reply, or `NoReply`.
    pub fn batch(
        &mut self,
        requests: &[(&str, Value)],
        wait: Duration,
    ) -> Vec<Result<Value, Failure>> {
        let first = self.next_id + 1;
        let mut replies: Vec<Option<Result<Value, Failure>>> = vec![None; requests.len()];
        for (method, params) in requests {
            self.next_id += 1;
            let frame = json!({"jsonrpc":"2.0","id":self.next_id,"method":method,"params":params});
            if self.send(&frame).is_err() {
                break;
            }
        }
        let deadline = Instant::now() + wait;
        while replies.iter().any(Option::is_none) {
            let left = deadline.saturating_duration_since(Instant::now());
            let Ok(reply) = self.replies.recv_timeout(left) else {
                break;
            };
            let Some(slot) = reply["id"]
                .as_u64()
                .and_then(|id| id.checked_sub(first))
                .and_then(|i| replies.get_mut(i as usize))
            else {
                continue;
            };
            // A second reply to one id is a protocol fault; keep the first.
            if slot.is_none() {
                *slot = Some(match reply.get("error") {
                    Some(error) => Err(Failure::Protocol(error.clone())),
                    None => Ok(reply["result"].clone()),
                });
            }
        }
        replies
            .into_iter()
            .map(|slot| slot.unwrap_or_else(|| Err(Failure::NoReply("batch".into()))))
            .collect()
    }

    /// The exit code once the process has ended (`-1` for a signal), else `None`.
    pub fn exited(&mut self) -> Option<i32> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(status.code().unwrap_or(-1)),
            _ => None,
        }
    }

    /// `initialize`, then `notifications/initialized`.
    pub fn initialize(&mut self, params: Value, wait: Duration) -> Result<Value, Failure> {
        let mut params = params;
        params["protocolVersion"] = crate::PROTOCOL_VERSION.into();
        let result = self.request("initialize", params, wait)?;
        let _ = self.send(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        Ok(result)
    }

    /// Call a tool and return its structured result, checking that the text
    /// content is the same JSON.
    pub fn call(&mut self, tool: &str, args: Value, wait: Duration) -> Result<Value, Failure> {
        self.call_attributed(tool, args, &Default::default(), wait)
    }

    /// [`Client::call`] on behalf of a seat lending `env`, as the host sends it.
    pub fn call_attributed(
        &mut self,
        tool: &str,
        args: Value,
        env: &BTreeMap<String, String>,
        wait: Duration,
    ) -> Result<Value, Failure> {
        let result = self.request("tools/call", attributed(tool, args, env), wait)?;
        let structured = result["structuredContent"].clone();
        if result["isError"] == true {
            return Err(Failure::Tool(structured));
        }
        let text = result["content"][0]["text"].as_str().unwrap_or("");
        if serde_json::from_str::<Value>(text).ok().as_ref() != Some(&structured) {
            return Err(Failure::NoReply(format!(
                "{tool}: text content differs from structuredContent"
            )));
        }
        Ok(structured)
    }

    /// Close stdin and wait for exit; returns the exit code.
    pub fn close(mut self, wait: Duration) -> Option<i32> {
        drop(self.stdin.take());
        let deadline = Instant::now() + wait;
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                return status.code();
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return None;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Run the plugin doctor ([`crate::doctor::run`]) against `binary` and fail
/// the calling test with the gap list when any gap is an error. Warnings and
/// skipped checks are in the returned report.
pub fn doctor(binary: &Path, options: &crate::doctor::Options) -> crate::doctor::Report {
    let report = crate::doctor::run(binary, options);
    assert!(report.is_clean(), "{}", report.human());
    report
}

/// Build example `name` of the package at `manifest_dir` (small, stripped,
/// release, in a private target beside the running test executable, shared
/// with `bboxy450-term-plugin-fixture`'s builds) and return its executable.
/// For tests of plugins that are examples, which have no `CARGO_BIN_EXE_*`.
pub fn example(manifest_dir: &Path, name: &str) -> PathBuf {
    let exe = std::env::current_exe().expect("test executable path");
    let base = exe
        .ancestors()
        .find(|dir| dir.file_name().is_some_and(|n| n == "deps"))
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map_or_else(|| manifest_dir.join("target"), Path::to_path_buf)
        .join("plugin-fixtures");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "build",
            "--quiet",
            "--release",
            "--example",
            name,
            "--manifest-path",
        ])
        .arg(manifest_dir.join("Cargo.toml"))
        .env("CARGO_PROFILE_RELEASE_OPT_LEVEL", "s")
        .env("CARGO_PROFILE_RELEASE_STRIP", "symbols")
        .env("CARGO_PROFILE_RELEASE_DEBUG", "false")
        .env("CARGO_TARGET_DIR", base.join("target"))
        .env("CARGO_BUILD_BUILD_DIR", base.join("build"))
        .env("RUSTC_WRAPPER", "")
        .env("RUSTC_WORKSPACE_WRAPPER", "")
        .env_remove("CARGO_INCREMENTAL")
        .output()
        .expect("run cargo for the example");
    assert!(
        output.status.success(),
        "example {name} build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    base.join("target/release/examples").join(name)
}
