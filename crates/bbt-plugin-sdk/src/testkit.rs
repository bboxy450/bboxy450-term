// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Drive a built plugin executable over stdio, as the host does. Used by
//! plugin conformance tests and read-only probes.
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
        let result = self.request("tools/call", json!({"name": tool, "arguments": args}), wait)?;
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
