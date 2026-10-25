//! One bounded MCP stdio connection; deadlines never follow progress messages.

use std::process::Stdio;
use std::time::Duration;

use bbt_plugin::{Refusal, policy::allowed_env_name, shape::SHAPE_VERSION};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
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
    output: BufReader<ChildStdout>,
    stderr: Option<JoinHandle<Stderr>>,
    next: u64,
    pub pid: u32,
    shape_timeout: Duration,
    #[cfg(test)]
    spawned_environment_names: Vec<String>,
}

impl Client {
    pub fn spawn(fixture: &FixtureAccount, shape: bool) -> Result<Self, Refusal> {
        Self::spawn_verified(
            fixture.executable()?,
            std::path::Path::new(&fixture.record.data_dir),
            shape,
            HANDSHAKE,
        )
    }

    // Both callers first reread their account authority and verify its pinned copy.
    pub(super) fn spawn_verified(
        executable: std::path::PathBuf,
        data_dir: &std::path::Path,
        shape: bool,
        shape_timeout: Duration,
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
            output,
            stderr: Some(capture),
            next: 0,
            pid,
            shape_timeout,
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
            loop {
                let count = self
                    .output
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
        let mut bytes = Vec::new();
        loop {
            let part = self
                .output
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
            self.output.consume(take + usize::from(newline.is_some()));
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
        let contracts: serde_json::Map<String, Value> = shape
            .systems
            .iter()
            .map(|s| (s.contract.clone(), json!(s.contract_version)))
            .collect();
        let result=self.request("initialize",json!({"protocolVersion":VERSIONS[0],"capabilities":{"experimental":{"bboxy450":{"shape_version":SHAPE_VERSION,"contracts":contracts,"concurrent":false,"trace":false}}},"clientInfo":{"name":"bboxy450-plugin-check","version":env!("CARGO_PKG_VERSION")}}),HANDSHAKE).await?;
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
            HANDSHAKE,
            self.send(&json!({"jsonrpc":"2.0","method":"notifications/initialized"})),
        )
        .await
        .map_err(|_| failure("deadline_exceeded", "initialized notification blocked"))??;
        Ok(())
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
            return Err(failure(
                "command_error",
                format!(
                    "{name} returned isError ({})",
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
