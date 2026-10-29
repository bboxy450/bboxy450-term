// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Installed native protocol contracts; no thread creation, user config changes
//! or model calls. Diagnostics may rejoin only a verified loaded bound thread
//! without overrides; ambiguous control mutations are never automatically retried.
use super::{
    Binding, Endpoint, Evidence, Grade, Observation, Priority, Session, State, WaitingKind,
    WakeCause, WakeOutcome,
};
use crate::{Result, Ulid, invalid};
use futures_util::{Sink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Message,
        protocol::{
            WebSocketConfig,
            frame::{
                Frame,
                coding::{Data, OpCode},
            },
        },
    },
};

const LIMIT: usize = 64 * 1024;
/// Default two-second deadline for bounded native preflight and mutation exchanges.
pub const DEADLINE: Duration = Duration::from_secs(2);
/// Check that a caller-selected Claude endpoint is an owned Unix socket and accepts a connection.
///
/// # Errors
/// Returns endpoint ownership, connection or deadline failures without sending a message.
pub async fn claude_preflight(path: &Path) -> Result<()> {
    own_socket(path)?;
    let _ = tokio::time::timeout(DEADLINE, UnixStream::connect(path)).await??;
    Ok(())
}
/// Read the live process start stamp using the platform process tool, on an IO thread.
///
/// # Errors
/// Returns process lookup, decoding or unavailable-process failures.
pub fn process_start(pid: u32) -> Result<String> {
    crate::assert_not_ui_thread();
    let output = std::process::Command::new("/bin/ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()?;
    process_start_output(pid, output)
}

fn process_start_output(pid: u32, output: std::process::Output) -> Result<String> {
    let stamp = std::str::from_utf8(&output.stdout)?.trim();
    if !output.status.success() || stamp.is_empty() {
        // A failed query alone does not prove that the harness exited. Retain
        // only ps metadata and its bounded diagnostic, never process arguments
        // or environment. The caller that owns the child can join exit/PTY data.
        let stderr = String::from_utf8_lossy(&output.stderr[..output.stderr.len().min(512)]);
        return Err(invalid(format!(
            "harness process lookup unavailable: pid={pid} ps_status={} stdout_bytes={} stderr={stderr:?}",
            output.status,
            output.stdout.len(),
        )));
    }
    Ok(stamp.into())
}
/// Validate a binding and reject a missing process or reused process identifier.
///
/// # Errors
/// Returns binding validation, process lookup or start-stamp mismatch failures.
pub fn verify_process(binding: &Binding) -> Result<()> {
    check_process(binding).map_err(|(_, error)| error)
}
type IdentityCheck = std::result::Result<(), (WakeCause, crate::Error)>;
fn check_process(binding: &Binding) -> IdentityCheck {
    binding
        .validate()
        .map_err(|error| (WakeCause::ProcessIdentityUnavailable, error))?;
    let start = process_start(binding.pid)
        .map_err(|error| (WakeCause::ProcessIdentityUnavailable, error))?;
    if start != binding.process_start {
        return Err((
            WakeCause::ProcessReplaced,
            invalid("harness process was replaced"),
        ));
    }
    Ok(())
}
/// Match the transport to the launch-selected runtime, not a user-global locator.
pub fn verify_codex_runtime(binding: &Binding, path: &Path) -> Result<()> {
    check_codex_runtime(binding, path).map_err(|(_, error)| error)
}
fn check_codex_runtime(binding: &Binding, path: &Path) -> IdentityCheck {
    if binding.session.starts_with("process-") {
        return Err((
            WakeCause::NativeThreadUnbound,
            invalid("native thread is not yet bound"),
        ));
    }
    verify_runtime_process(binding, path).map_err(|error| (WakeCause::NativePreflightFailed, error))
}
fn verify_runtime_process(binding: &Binding, path: &Path) -> Result<()> {
    let runtime = binding
        .runtime
        .as_ref()
        .ok_or_else(|| invalid("native runtime is not bound"))?;
    if runtime.endpoint != path || process_start(runtime.pid)? != runtime.process_start {
        return Err(invalid("native runtime endpoint/process changed"));
    }
    own_socket(path)
}

#[derive(Debug)]
struct UnknownRuntime;
impl std::fmt::Display for UnknownRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("native thread identity/status unknown; no mutation or automatic retry")
    }
}
impl std::error::Error for UnknownRuntime {}
fn own_socket(path: &Path) -> Result<()> {
    // Installed Codex publishes an owned symlink locator for its Unix socket.
    let meta = std::fs::metadata(path)?;
    // SAFETY: geteuid has no arguments or side effects.
    if !meta.file_type().is_socket() || meta.uid() != unsafe { libc::geteuid() } {
        return Err(invalid("native endpoint must be an owned Unix socket"));
    }
    Ok(())
}
/// Non-secret hook discovery, retained only for this native process/session.
#[derive(Serialize, Deserialize)]
pub struct ClaudeRegistration {
    /// Exact process, native-session and attachment identity being observed.
    pub binding: Binding,
    /// Explicit native socket endpoint path.
    pub path: PathBuf,
}
impl ClaudeRegistration {
    /// Match the retained native process and session, permitting a fresh logic attachment handle.
    ///
    /// Compares seat, process identifier, start stamp, generation and native session.
    /// The in-memory attachment generation is deliberately excluded.
    pub fn matches(&self, binding: &Binding) -> bool {
        // A logic restart creates fresh in-memory attachment handles, while
        // the supervisor can keep the same native process and session alive.
        self.binding.seat == binding.seat
            && self.binding.pid == binding.pid
            && self.binding.process_start == binding.process_start
            && self.binding.generation == binding.generation
            && self.binding.session == binding.session
    }
    /// Validate a matching registration and its absolute, caller-owned Unix socket.
    ///
    /// # Errors
    /// Returns identity, path, ownership or endpoint-kind failures.
    pub fn validate(&self, binding: &Binding) -> Result<()> {
        if !self.matches(binding) {
            return Err(invalid("Claude socket registration identity mismatch"));
        }
        if !self.path.is_absolute() {
            return Err(invalid("Claude socket path must be absolute"));
        }
        own_socket(&self.path)
    }
}
/// Called only on the admitted Hook RPC, never on terminal OSC observations.
pub fn claude_registration(
    session: &Session,
    event: &super::hook::ShimEvent,
) -> Result<Option<ClaudeRegistration>> {
    let Some(path) = event.raw.get("bboxy_native_socket") else {
        return Ok(None);
    };
    if session.record.kind != "claude"
        || event.raw["harness"] != "claude"
        || event.raw["session_id"] != session.binding.session
        || !matches!(event.event.as_str(), "SessionStart" | "Stop")
    {
        return Err(invalid("Claude socket hook identity mismatch"));
    }
    let registration = ClaudeRegistration {
        binding: session.binding.clone(),
        path: PathBuf::from(
            path.as_str()
                .ok_or_else(|| invalid("Claude socket hook path must be text"))?,
        ),
    };
    registration.validate(&session.binding)?;
    Ok(Some(registration))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Bounded native session-file metadata, validated against the selected process and session.
pub struct ClaudeSessionFile {
    /// Bound operating-system process identifier.
    pub pid: u32,
    #[serde(rename = "sessionId")]
    /// Native session identifier, serialized under the installed `sessionId` key.
    pub session_id: String,
    #[serde(rename = "procStart")]
    /// Observed process start stamp used to reject PID reuse.
    pub process_start: String,
    /// Native activity label, interpreted only after the session binding matches.
    pub status: String,
    #[serde(rename = "waitingFor", default)]
    /// Optional native waiting reason, such as permission, authentication or a question.
    pub waiting_for: Option<String>,
    #[serde(rename = "messagingSocketPath", default)]
    /// Optional native messaging endpoint reported by the session file; not authority by itself.
    pub messaging_socket_path: Option<std::path::PathBuf>,
}
/// Read an owned bounded Claude session file after revalidating the live process start stamp.
///
/// # Errors
/// Returns process, file ownership, size, decoding or exact session-identity failures.
pub fn read_claude_session(path: &Path, binding: &Binding) -> Result<ClaudeSessionFile> {
    crate::assert_not_ui_thread();
    verify_process(binding)?;
    read_claude_bound(path, binding)
}
/// The supervisor has already pinned process start and owns/reaps this child.
/// Avoid forking ps on every native-file poll; still match all stored fields.
pub fn read_claude_bound(path: &Path, binding: &Binding) -> Result<ClaudeSessionFile> {
    let meta = std::fs::symlink_metadata(path)?;
    // SAFETY: geteuid has no arguments or side effects.
    if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } || meta.len() > LIMIT as u64 {
        return Err(invalid("invalid sessions file"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > LIMIT {
        return Err(invalid("sessions file exceeded bound"));
    }
    let file: ClaudeSessionFile = serde_json::from_slice(&bytes)?;
    if file.pid != binding.pid
        || file.process_start != binding.process_start
        || file.session_id != binding.session
    {
        return Err(invalid("sessions file process/session mismatch"));
    }
    Ok(file)
}
impl Session {
    /// Apply a matching Claude session-file observation at reported grade and refresh readiness.
    ///
    /// # Errors
    /// Returns identity mismatches or unknown native status labels and invalidates readiness for unknown status.
    pub fn observe_claude_session(
        &mut self,
        file: &ClaudeSessionFile,
        sequence: u64,
        now_ms: u64,
    ) -> Result<Vec<Observation>> {
        if self.record.kind != "claude"
            || file.pid != self.binding.pid
            || file.session_id != self.binding.session
            || file.process_start != self.binding.process_start
        {
            return Err(invalid("native session observation identity mismatch"));
        }
        if self.status.source == "exit" {
            return Ok(Vec::new());
        }
        let waiting = file.waiting_for.as_ref().is_some_and(|v| !v.is_empty());
        let state = if waiting {
            State::Waiting
        } else {
            match file.status.as_str() {
                "idle" => State::Idle,
                "busy" | "shell" => State::Active,
                "waiting" => State::Waiting,
                "stopped" | "exited" => {
                    self.invalidate();
                    return Ok(Vec::new());
                }
                _ => {
                    self.invalidate();
                    return Err(invalid("unknown Claude sessions status"));
                }
            }
        };
        let mut obs = self.observation(sequence, now_ms, "sessions_file", Grade::Reported, state);
        if state == State::Waiting {
            obs.waiting_kind = Some(match file.waiting_for.as_deref() {
                Some("permission" | "permission_prompt") => WaitingKind::Permission,
                Some("auth") => WaitingKind::Auth,
                _ => WaitingKind::Question,
            });
            obs.detail = file.waiting_for.clone();
        }
        self.apply(obs.clone());
        if state == State::Idle && !self.dialog {
            self.ready(sequence, now_ms);
        } else {
            self.invalidate();
        }
        Ok(vec![obs])
    }
}
/// Project a Codex event for the currently bound native thread into graded activity observations.
///
/// # Errors
/// Returns a mismatched thread, missing method or unsupported native status shape.
pub fn observe(
    session: &mut Session,
    event: &Value,
    sequence: u64,
    now_ms: u64,
) -> Result<Vec<Observation>> {
    if session.record.kind != "codex" || session.status.source == "exit" {
        return Ok(Vec::new());
    }
    let params = &event["params"];
    if params["threadId"].as_str() != Some(&session.binding.session) {
        return Err(invalid("Codex event is for another native thread"));
    }
    let method = event["method"]
        .as_str()
        .ok_or_else(|| invalid("native event missing method"))?;
    let state = match method {
        "thread/status/changed" => match params["status"]["type"].as_str() {
            Some("idle") => State::Idle,
            Some("active") => {
                if params["status"]["activeFlags"]
                    .as_array()
                    .is_some_and(|flags| {
                        flags
                            .iter()
                            .any(|v| v == "waitingOnApproval" || v == "waitingOnUserInput")
                    })
                {
                    State::Waiting
                } else {
                    State::Active
                }
            }
            Some("systemError") => State::Errored,
            _ => {
                session.invalidate();
                return Ok(Vec::new());
            }
        },
        "turn/started" | "serverRequest/resolved" => State::Active,
        "turn/completed" => match params["turn"]["status"].as_str() {
            Some("completed" | "interrupted") => State::Idle,
            Some("failed") => State::Errored,
            _ => {
                session.invalidate();
                return Ok(Vec::new());
            }
        },
        _ if event.get("id").is_some()
            && (method.ends_with("requestApproval") || method.ends_with("requestUserInput")) =>
        {
            State::Waiting
        }
        _ => return Ok(Vec::new()),
    };
    let source = if event["source"] == "snapshot" {
        "app_server_snapshot"
    } else {
        "app_server"
    };
    let mut obs = session.observation(sequence, now_ms, source, Grade::Reported, state);
    if state == State::Waiting {
        obs.waiting_kind = Some(
            if method.ends_with("requestUserInput")
                || params["status"]["activeFlags"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v == "waitingOnUserInput"))
            {
                WaitingKind::Question
            } else {
                WaitingKind::Permission
            },
        );
    }
    obs.resolves = matches!(
        method,
        "serverRequest/resolved" | "turn/started" | "turn/completed"
    ) || (method == "thread/status/changed" && state != State::Waiting);
    session.apply(obs.clone());
    if state == State::Idle && !session.dialog {
        session.ready(sequence, now_ms);
    } else {
        session.invalidate();
    }
    Ok(vec![obs])
}

type Socket = WebSocketStream<UnixStream>;
#[derive(Debug)]
struct Rejected;
impl std::fmt::Display for Rejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("native RPC rejected")
    }
}
impl std::error::Error for Rejected {}
async fn send(socket: &mut Socket, value: Value) -> Result<()> {
    let text = serde_json::to_string(&value)?;
    if text.len() > LIMIT {
        return Err(invalid("native frame bound"));
    }
    socket.send(Message::Text(text.into())).await?;
    Ok(())
}
async fn reply(socket: &mut Socket, id: u64) -> Result<Value> {
    for _ in 0..128 {
        let frame = socket
            .next()
            .await
            .ok_or_else(|| invalid("native socket closed"))??;
        let text = match frame {
            Message::Text(v) => v,
            Message::Ping(_) => {
                socket.flush().await?;
                continue;
            }
            Message::Pong(_) => continue,
            _ => return Err(invalid("native socket closed/unexpected frame")),
        };
        let value: Value = serde_json::from_str(&text)?;
        if value.get("method").is_some() {
            if value.get("id").is_some() {
                return Err(invalid("native server request requires operator handling"));
            }
            continue;
        }
        if value["id"] != id {
            return Err(invalid("native reply ID mismatch"));
        }
        if let Some(error) = value.get("error") {
            if !error.is_object() || !error["code"].is_i64() || !error["message"].is_string() {
                return Err(invalid("malformed native RPC error; admission unknown"));
            }
            return Err(Rejected.into());
        }
        return value
            .get("result")
            .cloned()
            .ok_or_else(|| invalid("native reply missing result"));
    }
    Err(invalid("native notification bound exceeded"))
}
async fn open_codex_peer(path: &Path, owner: Option<u32>) -> Result<(Socket, Option<u32>)> {
    crate::assert_not_ui_thread();
    own_socket(path)?;
    let stream = UnixStream::connect(path).await?;
    let peer = stream
        .peer_cred()?
        .pid()
        .and_then(|pid| u32::try_from(pid).ok());
    if let Some(owner) = owner
        && peer != Some(owner)
    {
        return Err(invalid(
            "native socket peer does not match the launch-selected runtime",
        ));
    }
    let config = WebSocketConfig::default()
        .max_message_size(Some(LIMIT))
        .max_frame_size(Some(LIMIT));
    let (mut socket, _) =
        tokio_tungstenite::client_async_with_config("ws://localhost/", stream, Some(config))
            .await?;
    send(&mut socket,json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"bboxy450-term","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}}})).await?;
    reply(&mut socket, 1).await?;
    send(&mut socket, json!({"method":"initialized"})).await?;
    Ok((socket, peer))
}
async fn open_codex(path: &Path, owner: Option<u32>) -> Result<Socket> {
    Ok(open_codex_peer(path, owner).await?.0)
}
async fn read_codex_thread(
    socket: &mut Socket,
    path: &Path,
    thread: &str,
    request_id: u64,
) -> Result<Value> {
    send(
        socket,
        json!({"id":request_id,"method":"thread/read","params":{"threadId":thread,"includeTurns":false}}),
    )
    .await?;
    let identity = reply(socket, request_id).await?;
    // Opt-in observation of the read-only preflight, before a submission can run.
    #[cfg(debug_assertions)]
    if std::env::var_os("BBOXY_CODEX_PREFLIGHT_TRACE").is_some() {
        eprintln!(
            "BBOXY_CODEX_PREFLIGHT {}",
            json!({
                "requested_thread": super::observe::short(thread, 256),
                "returned_thread": identity["thread"]["id"].as_str()
                    .map(|id| super::observe::short(id, 256)),
                "status_type": identity["thread"]["status"]["type"].as_str()
                    .map(|status| super::observe::short(status, 256)),
                "socket_target": std::fs::canonicalize(path).ok(),
            })
        );
    }
    if identity["thread"]["id"] != thread
        || !matches!(
            identity["thread"]["status"]["type"].as_str(),
            Some("idle" | "active")
        )
    {
        return Err(Box::new(UnknownRuntime));
    }
    Ok(identity)
}
async fn codex_handshake(path: &Path, thread: &str, owner: Option<u32>) -> Result<(Socket, Value)> {
    let mut socket = open_codex(path, owner).await?;
    let identity = read_codex_thread(&mut socket, path, thread, 2).await?;
    Ok((socket, identity))
}

/// Inspect only the top-level method; unrelated catalog/config fields are skipped.
/// Duplicate keys retain Value's last-key semantics; escaped method strings still decode.
/// Origin: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
fn codex_proxy_method(text: &str) -> Option<std::borrow::Cow<'_, str>> {
    use serde::de::{IgnoredAny, MapAccess, Visitor};
    use std::{borrow::Cow, fmt};

    #[derive(Deserialize)]
    #[serde(field_identifier, rename_all = "lowercase")]
    enum Field {
        Method,
        #[serde(other)]
        Other,
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Method<'a> {
        Text(#[serde(borrow)] Cow<'a, str>),
        Other(IgnoredAny),
    }
    struct Envelope<'a>(Option<Cow<'a, str>>);
    impl<'de> Deserialize<'de> for Envelope<'de> {
        fn deserialize<D: serde::Deserializer<'de>>(
            deserializer: D,
        ) -> std::result::Result<Self, D::Error> {
            struct Fields;
            impl<'de> Visitor<'de> for Fields {
                type Value = Envelope<'de>;
                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a JSON object")
                }
                fn visit_map<A: MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> std::result::Result<Self::Value, A::Error> {
                    let mut method = None;
                    while let Some(field) = map.next_key::<Field>()? {
                        match field {
                            Field::Method => {
                                method = match map.next_value::<Method<'de>>()? {
                                    Method::Text(value) => Some(value),
                                    Method::Other(IgnoredAny) => None,
                                };
                            }
                            Field::Other => {
                                map.next_value::<IgnoredAny>()?;
                            }
                        }
                    }
                    Ok(Envelope(method))
                }
            }
            deserializer.deserialize_map(Fields)
        }
    }
    serde_json::from_str::<Envelope<'_>>(text).ok()?.0
}

const CODEX_RELAY_CHUNK: usize = 64 * 1024;

/// Tungstenite 0.30 front-drains its Vec after each partial socket write. Bound
/// that shifting to one standard continuation frame, sharing the original Bytes.
/// The receiver still observes one unchanged Text/Binary message, including UTF-8
/// characters split across frames. Small messages and control frames stay intact.
/// Origin: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
async fn codex_proxy_send<S: Sink<Message> + Unpin>(
    socket: &mut S,
    message: Message,
) -> std::result::Result<(), S::Error> {
    let kind = match &message {
        Message::Text(text) if text.len() > CODEX_RELAY_CHUNK => Data::Text,
        Message::Binary(bytes) if bytes.len() > CODEX_RELAY_CHUNK => Data::Binary,
        _ => return socket.send(message).await,
    };
    let payload = message.into_data();
    for start in (0..payload.len()).step_by(CODEX_RELAY_CHUNK) {
        let end = (start + CODEX_RELAY_CHUNK).min(payload.len());
        let opcode = OpCode::Data(if start == 0 { kind } else { Data::Continue });
        socket
            .send(Message::Frame(Frame::message(
                payload.slice(start..end),
                opcode,
                end == payload.len(),
            )))
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod codex_proxy_relay_tests {
    use super::*;
    use std::{
        convert::Infallible,
        pin::Pin,
        task::{Context, Poll},
    };

    #[derive(Default)]
    struct Frames(Vec<Message>);
    impl Sink<Message> for Frames {
        type Error = Infallible;
        fn poll_ready(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn start_send(self: Pin<&mut Self>, item: Message) -> std::result::Result<(), Self::Error> {
            self.get_mut().0.push(item);
            Ok(())
        }
        fn poll_flush(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn poll_close(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    #[test]
    fn borrowed_method_probe_matches_value_semantics_and_ignores_unrelated_fields() {
        for text in [
            r#"{"method":"thread/start","params":{"nested":{"method":"other"}}}"#,
            r#"{"params":{"method":"thread/start"}}"#,
            r#"{"method":null,"method":"thread/start"}"#,
            r#"{"method":"thread/start","method":null}"#,
            r#"{"method":{"method":"thread/start"},"method":"thread/resume"}"#,
            r#"{"method":"other","method":"thread/start"}"#,
            r#"{"\u006dethod":"thread\/resume"}"#,
            r#"{"method":"thread/start","truncated":"#,
            r#"[{"method":"thread/start"}]"#,
        ] {
            let expected = serde_json::from_str::<Value>(text).ok().and_then(|value| {
                value
                    .get("method")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });
            assert_eq!(
                codex_proxy_method(text).map(|method| method.into_owned()),
                expected
            );
        }
        let text = format!(
            r#"{{"params":{{"catalog":"{}"}},"method":"thread/start"}}"#,
            "x".repeat(1024 * 1024)
        );
        assert!(matches!(
            codex_proxy_method(&text),
            Some(std::borrow::Cow::Borrowed("thread/start"))
        ));
    }

    #[tokio::test]
    async fn large_data_uses_shared_bounded_continuations_and_controls_remain_whole() {
        let text = format!(
            "{}界{}",
            "x".repeat(CODEX_RELAY_CHUNK - 1),
            "🙂".repeat(32_768)
        );
        assert!(!text.is_char_boundary(CODEX_RELAY_CHUNK));
        let binary = (0..CODEX_RELAY_CHUNK * 3 + 17)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        for (message, kind) in [
            (Message::Text(text.into()), Data::Text),
            (Message::Binary(binary.into()), Data::Binary),
        ] {
            let original = message.clone().into_data();
            let mut frames = Frames::default();
            codex_proxy_send(&mut frames, message).await.unwrap();
            assert_eq!(frames.0.len(), original.len().div_ceil(CODEX_RELAY_CHUNK));
            let mut offset = 0;
            for (index, message) in frames.0.iter().enumerate() {
                let Message::Frame(frame) = message else {
                    panic!("large relay emitted a whole message");
                };
                let end = (offset + CODEX_RELAY_CHUNK).min(original.len());
                assert_eq!(
                    frame.header().opcode,
                    OpCode::Data(if index == 0 { kind } else { Data::Continue })
                );
                assert_eq!(frame.header().is_final, end == original.len());
                assert!(
                    frame.payload() == &original[offset..end],
                    "relay payload differs"
                );
                assert_eq!(
                    frame.payload().as_ptr(),
                    original.as_ptr().wrapping_add(offset),
                    "relay must share original Bytes"
                );
                offset = end;
            }
            assert_eq!(offset, original.len());
        }
        for message in [
            Message::Text("small".into()),
            Message::Binary(vec![1, 2].into()),
            Message::Ping(vec![3].into()),
            Message::Pong(vec![4].into()),
            Message::Close(None),
        ] {
            let expected = message.clone();
            let mut frames = Frames::default();
            codex_proxy_send(&mut frames, message).await.unwrap();
            assert!(
                frames.0.as_slice() == [expected],
                "small/control message changed"
            );
        }
    }
}

/// Relay a seat's TUI to Codex's shared daemon, adding startup configuration to
/// each connection's first thread start/resume only. Native clients can connect
/// alongside the TUI. Never owns the daemon.
///
/// # Errors
/// Returns private listener setup/accept failures. A failed client is closed and
/// reported without disconnecting other clients or stopping the borrowed daemon.
pub async fn codex_proxy(
    frontend: &Path,
    backend: &Path,
    launch: &super::launch::CodexLaunch,
    program: &str,
) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let listener = tokio::net::UnixListener::bind(frontend)?;
    std::fs::set_permissions(frontend, std::fs::Permissions::from_mode(0o600))?;
    // Futures remain owned by this proxy; dropping it closes clients only.
    let thread = std::sync::Mutex::new(launch.resume.clone());
    let mut clients = futures_util::stream::FuturesUnordered::new();
    loop {
        tokio::select! {
            accepted = listener.accept(), if clients.len() < 16 => {
                let (stream, _) = accepted?;
                clients.push(codex_proxy_client(stream, backend, launch, program, &thread));
            }
            completed = clients.next(), if !clients.is_empty() => {
                if let Some(Err(error)) = completed {
                    eprintln!("Codex proxy client failed: {error}");
                }
            }
        }
    }
}

async fn codex_proxy_client(
    stream: UnixStream,
    backend: &Path,
    launch: &super::launch::CodexLaunch,
    program: &str,
    bound_thread: &std::sync::Mutex<Option<String>>,
) -> Result<()> {
    // Match Codex's remote transport, including large plugin/skill catalogs.
    // Tungstenite's default 16 MiB frame cap otherwise causes a reconnect loop.
    let transport_config = || {
        WebSocketConfig::default()
            .max_message_size(Some(128 << 20))
            .max_frame_size(Some(128 << 20))
    };
    fn overlay(into: &mut Value, from: Value) {
        if let (Value::Object(existing), Value::Object(next)) = (&mut *into, &from) {
            for (key, value) in next {
                overlay(
                    existing.entry(key.clone()).or_insert(Value::Null),
                    value.clone(),
                );
            }
        } else {
            *into = from;
        }
    }
    // Another launcher may have added session-scoped hooks before this proxy.
    // Retain exact groups once; ordinary arrays still use the seat overlay.
    fn retain_client_hooks(client: &Value, seat: &mut Value) {
        let (Some(existing), Some(added)) = (
            client.get("hooks").and_then(Value::as_object),
            seat.get_mut("hooks").and_then(Value::as_object_mut),
        ) else {
            return;
        };
        for (event, groups) in added {
            if let (Some(existing), Value::Array(groups)) =
                (existing.get(event).and_then(Value::as_array), groups)
            {
                let mut combined = existing.clone();
                for group in groups.iter() {
                    if !combined.contains(group) {
                        combined.push(group.clone());
                    }
                }
                *groups = combined;
            }
        }
    }
    let accepted = tokio::time::timeout(
        Duration::from_secs(5),
        tokio_tungstenite::accept_async_with_config(stream, Some(transport_config())),
    )
    .await;
    let Ok(Ok(mut client)) = accepted else {
        return Ok(());
    }; // readiness probes are not WebSockets
    super::launch::ensure_codex_daemon(backend, program)?;
    own_socket(backend)?;
    let stream = UnixStream::connect(backend).await?;
    let (mut server, _) = tokio_tungstenite::client_async_with_config(
        "ws://localhost/",
        stream,
        Some(transport_config()),
    )
    .await?;
    let mut pending: Option<(Value, Option<String>)> = None;
    let mut configured = false;
    let mut internal: Option<Value> = None;
    loop {
        tokio::select! {
            frame = client.next() => {
                let Some(Ok(mut frame)) = frame else { break; };
                if !configured && pending.is_none()
                    && let Message::Text(text) = &frame
                    && matches!(codex_proxy_method(text).as_deref(), Some("thread/start" | "thread/resume"))
                    && let Ok(mut call) = serde_json::from_str::<Value>(text)
                    && matches!(call["method"].as_str(), Some("thread/start" | "thread/resume"))
                {
                    let expected = call["params"]["threadId"].as_str().map(str::to_owned);
                    let bound = bound_thread.lock().map_err(|_| invalid("Codex proxy thread binding unavailable"))?.clone();
                    if bound.as_ref().is_some_and(|id| expected.as_ref() != Some(id)) {
                        return Err(invalid("Codex TUI resumed a different thread"));
                    }
                    let mut config = serde_json::to_value(&launch.config)?;
                    retain_client_hooks(&call["params"]["config"], &mut config);
                    overlay(&mut call["params"]["config"], config);
                    if let Some(instructions) = launch.config.get("developer_instructions") {
                        call["params"]["developerInstructions"] = instructions.clone();
                    }
                    for (key, parameter) in [("approval_policy", "approvalPolicy"), ("sandbox_mode", "sandbox")] {
                        if let Some(value) = launch.config.get(key) {
                            call["params"][parameter] = value.clone();
                        }
                    }
                    call["params"]["cwd"] = json!(launch.cwd);
                    pending = Some((call["id"].clone(), expected));
                    frame = Message::Text(serde_json::to_string(&call)?.into());
                }
                if codex_proxy_send(&mut server, frame).await.is_err() { break; }
            }
            frame = server.next() => {
                let Some(Ok(frame)) = frame else { break; };
                // Unrelated catalog responses can be tens of MiB. Relay
                // them directly instead of allocating a second JSON tree.
                if (pending.is_some() || internal.is_some())
                    && let Message::Text(text) = &frame
                    && let Ok(response) = serde_json::from_str::<Value>(text)
                {
                    if internal.as_ref().is_some_and(|id| response.get("id") == Some(id)) {
                        internal = None;
                        continue;
                    }
                    if let Some((id, expected)) = &pending
                        && response.get("id") == Some(id)
                    {
                        if let Some(thread) = response["result"]["thread"]["id"].as_str() {
                            if expected.as_ref().is_some_and(|id| id != thread) {
                                return Err(invalid("Codex returned a different thread"));
                            }
                            {
                                let mut bound = bound_thread.lock().map_err(|_| invalid("Codex proxy thread binding unavailable"))?;
                                if bound.as_ref().is_some_and(|id| id != thread) {
                                    return Err(invalid("Codex returned a different thread"));
                                }
                                *bound = Some(thread.to_owned());
                            }
                            let registration = std::process::Command::new(&launch.host)
                                .args(["hook", "--agent", "codex", &json!({"hook_event_name":"SessionStart","session_id":thread,"bboxy_proxy":true}).to_string()])
                                .env("BBOXY_HOOK_SOCKET_ONLY", "1").status()?;
                            if !registration.success() { return Err(invalid("Codex thread registration failed")); }
                            let name_id = json!(format!("bboxy-name-{}", Ulid::new()));
                            server.send(Message::Text(json!({"id":name_id,"method":"thread/name/set","params":{"threadId":thread,"name":launch.name}}).to_string().into())).await?;
                            internal = Some(name_id);
                            configured = true;
                        }
                        pending = None;
                    }
                }
                if codex_proxy_send(&mut client, frame).await.is_err() { break; }
            }
        }
    }
    Ok(())
}

/// Pin a borrowed daemon to its kernel peer and this already authenticated thread.
/// Never enumerate, load, resume or select another thread during discovery.
///
/// # Errors
/// Refuses an unbound thread, replaced TUI/daemon, unavailable socket, unknown or
/// unloaded thread, identity mismatch or a bounded read-only handshake timeout.
pub async fn shared_codex_runtime(binding: &Binding, path: &Path) -> Result<super::RuntimeBinding> {
    verify_process(binding)?;
    if binding.session.starts_with("process-") {
        return Err(Box::new(UnknownRuntime));
    }
    let runtime = tokio::time::timeout(DEADLINE, async {
        let (mut socket, pid) = open_codex_peer(path, None).await?;
        let pid = pid.ok_or_else(|| invalid("native daemon peer PID unavailable"))?;
        let runtime = super::RuntimeBinding {
            endpoint: path.to_owned(),
            pid,
            process_start: process_start(pid)?,
        };
        read_codex_thread(&mut socket, path, &binding.session, 2).await?;
        Ok::<_, crate::Error>(runtime)
    })
    .await??;
    verify_process(binding)?;
    let mut pinned = binding.clone();
    pinned.runtime = Some(runtime.clone());
    verify_runtime_process(&pinned, path)?;
    Ok(runtime)
}

/// Read only the threads loaded in the launch-selected runtime. A fresh TUI can
/// be idle before any SessionStart hook rebinds the caller's bootstrap identity.
/// Never select from stored/global history, create or load a thread, or guess
/// among multiple loaded threads. An unknown thread is a terminal error.
pub async fn discover_codex_thread(binding: &Binding) -> Result<Option<(String, Value)>> {
    verify_process(binding)?;
    let runtime = binding
        .runtime
        .as_ref()
        .ok_or_else(|| invalid("native runtime is not bound"))?;
    if let Err(error) = verify_runtime_process(binding, &runtime.endpoint) {
        if error
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        {
            return Ok(None);
        }
        return Err(error);
    }
    tokio::time::timeout(DEADLINE, async {
        let mut socket = open_codex(&runtime.endpoint, Some(runtime.pid)).await?;
        send(
            &mut socket,
            json!({"id":2,"method":"thread/loaded/list","params":{}}),
        )
        .await?;
        let loaded = reply(&mut socket, 2).await?;
        let ids = loaded["data"]
            .as_array()
            .ok_or_else(|| invalid("invalid loaded-thread page"))?;
        if ids.is_empty() && loaded["nextCursor"].is_null() {
            return Ok(None);
        }
        if ids.len() != 1 || !loaded["nextCursor"].is_null() {
            return Err(invalid(
                "private runtime has ambiguous loaded threads; no binding",
            ));
        }
        let id = ids[0]
            .as_str()
            .filter(|id| !id.is_empty() && id.len() <= 256)
            .ok_or_else(|| invalid("invalid native thread ID"))?;
        let identity = read_codex_thread(&mut socket, &runtime.endpoint, id, 3).await?;
        Ok(Some((id.to_owned(), identity["thread"]["status"].clone())))
    })
    .await?
}

/// Apply a read-only discovery only while the original launch binding is still
/// current. A late completion never replaces a hook, generation or runtime.
pub fn bind_discovered_codex(
    session: &mut Session,
    original: &Binding,
    thread: &str,
    status: Value,
    sequence: u64,
    now_ms: u64,
) -> Result<bool> {
    if session.binding != *original
        || session.record.kind != "codex"
        || session.status.source == "exit"
        || !original.session.starts_with("process-")
        || original.runtime.is_none()
    {
        return Ok(false);
    }
    if !matches!(status["type"].as_str(), Some("idle" | "active")) {
        return Err(Box::new(UnknownRuntime));
    }
    let mut binding = original.clone();
    binding.session = thread.to_owned();
    binding.attachment_generation = binding
        .attachment_generation
        .checked_add(1)
        .ok_or_else(|| invalid("native attachment generation exhausted"))?;
    session.rebind(binding)?;
    observe(
        session,
        &json!({"source":"snapshot","method":"thread/status/changed","params":{"threadId":thread,"status":status}}),
        sequence,
        now_ms,
    )?;
    Ok(true)
}

/// Connect and perform a read-only handshake for one explicit Codex endpoint and thread.
///
/// Callers bound the lifetime of this connection; unlike status queries, this helper
/// does not impose an outer timeout or validate a runtime process binding.
///
/// # Errors
/// Returns socket ownership, connection, framing or native thread-identity failures.
pub async fn codex_connect(path: &Path, thread: &str) -> Result<Socket> {
    Ok(codex_handshake(path, thread, None).await?.0)
}
/// Read one explicit Codex thread status with the default native deadline.
///
/// # Errors
/// Returns endpoint, handshake, thread-identity or deadline failures.
pub async fn codex_status(path: &Path, thread: &str) -> Result<Value> {
    let (_, identity) =
        tokio::time::timeout(DEADLINE, codex_handshake(path, thread, None)).await??;
    Ok(identity["thread"]["status"].clone())
}
/// Poll the exact launch-selected peer. The caller validated the process start
/// when creating its reader; every connection still checks the kernel peer PID.
pub async fn codex_owned_status(binding: &Binding) -> Result<Value> {
    let runtime = binding
        .runtime
        .as_ref()
        .ok_or_else(|| invalid("native runtime is not bound"))?;
    let (_, identity) = tokio::time::timeout(
        DEADLINE,
        codex_handshake(&runtime.endpoint, &binding.session, Some(runtime.pid)),
    )
    .await??;
    Ok(identity["thread"]["status"].clone())
}
/// Read the next JSON text event, flushing non-text frames, within a caller-supplied deadline.
///
/// # Errors
/// Returns transport closure, invalid JSON, WebSocket or deadline failures.
pub async fn next_codex_event(socket: &mut Socket, deadline: Duration) -> Result<Value> {
    tokio::time::timeout(deadline, async {
        loop {
            let frame = socket
                .next()
                .await
                .ok_or_else(|| invalid("native observer closed"))??;
            if let Message::Text(text) = frame {
                return Ok(serde_json::from_str(&text)?);
            }
            socket.flush().await?;
        }
    })
    .await?
}
/// Submit one stable-ID native message after live process validation.
///
/// Success is transport evidence, never proof of model consumption. An ambiguous
/// write or reply returns [`Evidence::NativeUnknown`], forbidding automatic retry
/// or terminal-input fallback. Preflight refusals preserve recipient-storage evidence.
///
/// `allow_interrupt` is caller authority, never the target seat's role. Queue
/// and steer are distinct; Next/Later scheduling stays above the native queue.
pub async fn wake(
    binding: &Binding,
    endpoint: &Endpoint,
    id: Ulid,
    text: &str,
    priority: Priority,
    allow_interrupt: bool,
) -> WakeOutcome {
    crate::assert_not_ui_thread();
    let unavailable = |cause, note: &str| WakeOutcome {
        evidence: Evidence::RecipientStored,
        native_id: None,
        note: super::observe::short(note, 512),
        cause: Some(cause),
    };
    if priority == Priority::Now && !allow_interrupt {
        return unavailable(
            WakeCause::InterruptUnauthorized,
            "caller lacks interrupt authority",
        );
    }
    if text.len() > 48 * 1024 {
        return unavailable(
            WakeCause::MessageTooLarge,
            "native message exceeds 48 KiB; no mutation attempted",
        );
    }
    if let Err((cause, error)) = check_process(binding) {
        return unavailable(
            cause,
            &format!("native identity unavailable: {error}; no mutation attempted"),
        );
    }
    let mut attempted = false;
    let mut preflight_cause = WakeCause::NativePreflightFailed;
    let result=tokio::time::timeout(DEADLINE,async {
        match endpoint {
            Endpoint::None=>{preflight_cause=WakeCause::NativeUnavailable;Err(invalid("native delivery unavailable"))},
            Endpoint::Claude{path}=> {
                own_socket(path)?;let mut socket=UnixStream::connect(path).await?;
                let priority=match priority{Priority::Next=>"next",Priority::Later=>"later",Priority::Now=>"now"};
                let mut bytes=serde_json::to_vec(&json!({"type":"user","uuid":id.to_string(),"msg_id":id.to_string(),"priority":priority,"message":{"role":"user","content":text}}))?;
                bytes.push(b'\n');if bytes.len()>LIMIT{preflight_cause=WakeCause::MessageTooLarge;return Err(invalid("native frame bound"));}
                attempted=true;socket.write_all(&bytes).await?;socket.shutdown().await?;
                Ok(WakeOutcome{evidence:Evidence::NativeWritten,native_id:None,note:"frame written; receiver/model acceptance unverified".into(),cause:Some(WakeCause::NativeFrameWritten)})
            }
            Endpoint::Codex{path,thread,turn}=> {
                if thread!=&binding.session {preflight_cause=WakeCause::NativeThreadMismatch;return Err(invalid("native thread binding mismatch"));}
                if let Err((cause,error))=check_codex_runtime(binding,path) {preflight_cause=cause;return Err(error);}
                let owner=binding.runtime.as_ref().ok_or_else(||invalid("native runtime is not bound"))?.pid;
                let (mut socket,_)=codex_handshake(path,thread,Some(owner)).await?;
                let input=json!([{"type":"text","text":text}]);
                let request=if priority==Priority::Now {
                    let expected=turn.as_ref().ok_or_else(||{preflight_cause=WakeCause::CurrentTurnMissing;invalid("interrupt requires a current turn ID")})?;
                    json!({"id":3,"method":"turn/steer","params":{"threadId":thread,"expectedTurnId":expected,"clientUserMessageId":id.to_string(),"input":input}})
                }else{json!({"id":3,"method":"thread/queue/add","params":{"threadId":thread,"clientUserMessageId":id.to_string(),"input":input}})};
                attempted=true;send(&mut socket,request).await?;let response=reply(&mut socket,3).await?;
                let native_id=if priority==Priority::Now {
                    let expected=turn.as_deref().unwrap();if response["turnId"]!=expected {return Err(invalid("wrong steered turn"));}expected.into()
                }else {
                    let queued=&response["queuedSubmission"];
                    if queued["clientUserMessageId"]!=id.to_string()
                        || !queued["input"].as_array().is_some_and(|a|a.len()==1&&a[0]["type"]=="text"&&a[0]["text"]==text)
                        || response.get("threadId").is_some_and(|v|v!=thread) {
                        return Err(invalid("queue admission mismatch"));
                    }
                    queued["id"].as_str().filter(|s|!s.is_empty()&&s.len()<=256).ok_or_else(||invalid("missing native queue ID"))?.into()
                };
                Ok(WakeOutcome{evidence:Evidence::NativeQueued,native_id:Some(native_id),note:"native admission only; model consumption unverified".into(),cause:Some(WakeCause::NativeQueueAdmitted)})
            }
        }
    }).await;
    match result {
        Ok(Ok(value)) => value,
        Ok(Err(error)) if error.downcast_ref::<UnknownRuntime>().is_some() => {
            WakeOutcome::unknown_cause(WakeCause::NativeThreadUnknown, error.to_string())
        }
        Ok(Err(error)) if error.downcast_ref::<Rejected>().is_some() => {
            unavailable(WakeCause::NativeRejected, "native rejected submission")
        }
        _ if attempted => WakeOutcome::unknown_cause(
            WakeCause::NativeWriteAmbiguous,
            "native write/reply ambiguous; no PTY fallback or automatic retry",
        ),
        Ok(Err(error)) => unavailable(
            preflight_cause,
            &format!("native preflight failed: {error}; no mutation attempted"),
        ),
        Err(_) => unavailable(
            WakeCause::NativePreflightTimedOut,
            "native preflight timed out; no mutation attempted",
        ),
    }
}

/// A bounded, native hook failure; context/model output is never retained here.
pub struct HookFailure {
    /// Native run identity, used only for bounded duplicate suppression.
    pub id: String,
    /// Hook event name supplied by the native client.
    pub event: String,
    /// Native hook configuration source classification.
    pub source: String,
    /// Configuration path, metadata only: never opened by this projection.
    pub source_path: String,
    /// Native terminal status message; redact at the application journal boundary.
    pub message: String,
    /// Measured native duration, absent if unavailable/invalid.
    pub duration_ms: Option<u64>,
    /// Explicitly reported exit code, otherwise unknown (never guessed from status).
    pub exit_code: Option<i32>,
    /// Only error-kind entries, retaining the last four KiB.
    pub stderr: String,
}
fn hook_exit_code(text: &str) -> Option<i32> {
    let lower = text.to_ascii_lowercase();
    for marker in ["exited with code ", "exit code "] {
        if let Some((_, rest)) = lower.split_once(marker) {
            let number: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '-')
                .collect();
            if let Ok(code) = number.parse() {
                return Some(code);
            }
        }
    }
    None
}
/// Select a failure from the exact observed thread, without copying hook input/context.
pub fn codex_hook_failure(event: &Value, thread: &str) -> Option<HookFailure> {
    if event["method"] != "hook/completed" || event["params"]["threadId"] != thread {
        return None;
    }
    let run = &event["params"]["run"];
    let id = run["id"]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 256)?;
    let message = run["statusMessage"].as_str().unwrap_or("");
    let exit_code = hook_exit_code(message);
    if !matches!(run["status"].as_str(), Some("failed" | "blocked"))
        && !exit_code.is_some_and(|n| n != 0)
    {
        return None;
    }
    let mut stderr = String::new();
    for entry in run["entries"].as_array()? {
        if entry["kind"] != "error" {
            continue;
        }
        if let Some(text) = entry["text"].as_str() {
            if !stderr.is_empty() {
                stderr.push('\n');
            }
            stderr.push_str(text);
            let drop = stderr.ceil_char_boundary(stderr.len().saturating_sub(4096));
            if drop > 0 {
                stderr.drain(..drop);
            }
        }
    }
    Some(HookFailure {
        id: id.into(),
        event: super::observe::short(run["eventName"].as_str()?, 128),
        source: super::observe::short(run["source"].as_str().unwrap_or("unknown"), 64),
        source_path: super::observe::short(run["sourcePath"].as_str().unwrap_or("unknown"), 512),
        message: super::observe::short(message, 1024),
        duration_ms: run["durationMs"].as_u64(),
        exit_code,
        stderr,
    })
}
/// Private-runtime observation channel, with no method that sends native control.
pub struct CodexHookObserver {
    socket: Socket,
    pending: std::collections::VecDeque<HookFailure>,
    thread: String,
}
impl CodexHookObserver {
    /// Await one bounded hook failure; unrelated model/tool events are discarded.
    /// The IO owner cancels the future when the seat/session/generation ends.
    /// # Errors
    /// Returns a closed socket or invalid native frame. No native request is retried.
    pub async fn next_failure(&mut self) -> Result<HookFailure> {
        if let Some(next) = self.pending.pop_front() {
            return Ok(next);
        }
        loop {
            let frame = self
                .socket
                .next()
                .await
                .ok_or_else(|| invalid("native observer closed"))??;
            if let Message::Text(text) = frame {
                let event: Value = serde_json::from_str(&text)?;
                if let Some(failure) = codex_hook_failure(&event, &self.thread) {
                    return Ok(failure);
                }
            } else {
                self.socket.flush().await?;
            }
        }
    }
}
/// Join only an already-loaded thread in its verified launch-selected runtime.
/// Sends no model/config/permission overrides and never creates or starts a turn.
/// # Errors
/// Rejects stale process/runtime identities, absent/ambiguous/unloaded threads,
/// unknown handshake results and bounded-frame/prefetch failures. No retry is attempted.
pub async fn codex_hook_observer(binding: &Binding) -> Result<CodexHookObserver> {
    verify_process(binding)?;
    let runtime = binding
        .runtime
        .as_ref()
        .ok_or_else(|| invalid("native runtime is not bound"))?;
    verify_codex_runtime(binding, &runtime.endpoint)?;
    tokio::time::timeout(DEADLINE, async {
        let mut socket = open_codex(&runtime.endpoint, Some(runtime.pid)).await?;
        send(&mut socket, json!({"id":2,"method":"thread/loaded/list","params":{}})).await?;
        let loaded = reply(&mut socket, 2).await?;
        if loaded["data"] != json!([binding.session]) || !loaded["nextCursor"].is_null() {
            return Err(invalid("observer thread is not the unique loaded private thread"));
        }
        read_codex_thread(&mut socket, &runtime.endpoint, &binding.session, 3).await?;
        send(&mut socket, json!({"id":4,"method":"thread/resume","params":{"threadId":binding.session,"excludeTurns":true}})).await?;
        let mut pending = std::collections::VecDeque::new();
        for _ in 0..128 {
            let frame = socket.next().await
                .ok_or_else(|| invalid("native observer closed during join"))??;
            let value: Value = match frame {
                Message::Text(text) => serde_json::from_str(&text)?,
                Message::Ping(_) | Message::Pong(_) => { socket.flush().await?; continue; }
                _ => return Err(invalid("native observer closed during join")),
            };
            if value.get("method").is_some() {
                if value.get("id").is_some() {
                    return Err(invalid("native observer cannot answer server control requests"));
                }
                if let Some(failure) = codex_hook_failure(&value, &binding.session) {
                    if pending.len() == 8 {
                        return Err(invalid("native hook observer startup failures clipped"));
                    }
                    pending.push_back(failure);
                }
                continue;
            }
            if value["id"] != 4 || value.get("error").is_some()
                || value["result"]["thread"]["id"] != binding.session
                || !matches!(value["result"]["thread"]["status"]["type"].as_str(), Some("idle" | "active")) {
                return Err(invalid("native observer join unconfirmed; no retry"));
            }
            return Ok(CodexHookObserver { socket, pending, thread: binding.session.clone() });
        }
        Err(invalid("native observer join notification bound"))
    }).await?
}

#[cfg(test)]
mod registration_tests {
    use super::*;

    #[test]
    fn process_start_retains_failed_query_metadata_without_claiming_child_exit() {
        use std::os::unix::process::ExitStatusExt;
        for (raw_status, stdout) in [(256, b"".as_slice()), (0, b""), (256, b"a stamp")] {
            let error = process_start_output(
                1234,
                std::process::Output {
                    status: std::process::ExitStatus::from_raw(raw_status),
                    stdout: stdout.to_vec(),
                    stderr: b"fixture query failure".to_vec(),
                },
            )
            .unwrap_err()
            .to_string();
            assert!(
                error.contains("lookup unavailable: pid=1234 ps_status="),
                "{error}"
            );
            assert!(
                error.contains(&format!("stdout_bytes={}", stdout.len())),
                "{error}"
            );
            assert!(error.contains("fixture query failure"), "{error}");
            assert!(!error.contains("child exited"), "{error}");
        }
        let stamp = process_start_output(
            1234,
            std::process::Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: b"  Wed Oct 28 22:00:00 2026\n".to_vec(),
                stderr: Vec::new(),
            },
        )
        .unwrap();
        assert_eq!(stamp, "Wed Oct 28 22:00:00 2026");
    }

    #[test]
    fn process_start_bounds_query_stderr() {
        use std::os::unix::process::ExitStatusExt;
        let error = process_start_output(
            1234,
            std::process::Output {
                status: std::process::ExitStatus::from_raw(256),
                stdout: Vec::new(),
                stderr: [vec![b'x'; 512], b"not retained".to_vec()].concat(),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(&"x".repeat(512)));
        assert!(!error.contains("not retained"));
    }

    fn binding() -> Binding {
        Binding {
            seat: Ulid::new(),
            session: "native-fixture-session".into(),
            pid: std::process::id(),
            process_start: process_start(std::process::id()).unwrap(),
            generation: 1,
            attachment_generation: 1,
            runtime: None,
        }
    }

    // keeps: decision #279
    #[test]
    fn retained_socket_requires_the_same_process_generation_and_session() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("native.sock");
        let _listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let current = binding();
        let registration = ClaudeRegistration {
            binding: current.clone(),
            path,
        };
        let bytes = serde_json::to_vec(&registration).unwrap();
        let restored: ClaudeRegistration = serde_json::from_slice(&bytes).unwrap();
        restored.validate(&current).unwrap();
        let mut fresh_handle = current.clone();
        fresh_handle.attachment_generation += 1;
        restored.validate(&fresh_handle).unwrap();
        for field in 0..5 {
            let mut stale = current.clone();
            match field {
                0 => stale.seat = Ulid::new(),
                1 => stale.pid += 1,
                2 => stale.process_start.push_str(" replaced"),
                3 => stale.generation += 1,
                _ => stale.session.push_str(" replaced"),
            }
            assert!(restored.validate(&stale).is_err(), "identity field {field}");
        }
    }

    // keeps: decision #279
    #[test]
    fn hook_socket_metadata_requires_the_admitted_native_session_and_owned_socket() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("native.sock");
        let _listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let session = Session::new(
            super::super::HarnessRecord::builtin("claude").unwrap(),
            binding(),
        )
        .unwrap();
        let mut event = super::super::hook::ShimEvent {
            event: "SessionStart".into(),
            raw: json!({"hook_event_name":"SessionStart", "harness":"claude", "session_id":session.binding.session, "bboxy_native_socket":path}),
        };
        assert!(claude_registration(&session, &event).unwrap().is_some());
        let prepared = super::super::hook::prepare(event.raw.clone(), "claude").unwrap();
        // stdin cannot supply trusted endpoint metadata; the shim adds the
        // documented environment export only after its input allowlist.
        assert!(prepared.raw.get("bboxy_native_socket").is_none());
        event.raw["session_id"] = "another-session".into();
        assert!(claude_registration(&session, &event).is_err());
        event.raw["session_id"] = session.binding.session.clone().into();
        event.raw["bboxy_native_socket"] = root.path().to_string_lossy().into_owned().into();
        assert!(claude_registration(&session, &event).is_err());
    }

    // keeps: spec harness-kits §2 — honest-specific-failure public-note contract
    #[test]
    fn pre_write_failures_preserve_the_specific_cause() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("missing.sock");
        let binding = binding();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let missing = runtime.block_on(wake(
            &binding,
            &Endpoint::Claude { path },
            Ulid::new(),
            "fixture",
            Priority::Next,
            false,
        ));
        assert_eq!(missing.evidence, Evidence::RecipientStored);
        assert_eq!(missing.cause, Some(WakeCause::NativePreflightFailed));
        assert!(
            missing
                .note
                .contains(&std::io::Error::from_raw_os_error(libc::ENOENT).to_string()),
            "{missing:?}"
        );
        assert!(missing.note.contains("no mutation attempted"));
        let mut replaced = binding.clone();
        replaced.process_start.push_str(" replaced");
        let error = verify_process(&replaced).unwrap_err();
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::InvalidData
        );
        let replaced = runtime.block_on(wake(
            &replaced,
            &Endpoint::None,
            Ulid::new(),
            "fixture",
            Priority::Next,
            false,
        ));
        assert_eq!(replaced.evidence, Evidence::RecipientStored);
        assert_eq!(replaced.cause, Some(WakeCause::ProcessReplaced));
        assert!(!replaced.note.is_empty() && replaced.note.len() <= 512);

        let path = root.path().join("untouched.sock");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        for (priority, text, cause) in [
            (
                Priority::Now,
                "fixture".to_owned(),
                WakeCause::InterruptUnauthorized,
            ),
            (
                Priority::Next,
                "x".repeat(48 * 1024 + 1),
                WakeCause::MessageTooLarge,
            ),
        ] {
            let outcome = runtime.block_on(wake(
                &binding,
                &Endpoint::Claude { path: path.clone() },
                Ulid::new(),
                &text,
                priority,
                false,
            ));
            assert_eq!(outcome.evidence, Evidence::RecipientStored);
            assert_eq!(outcome.cause, Some(cause));
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
        let unavailable = runtime.block_on(wake(
            &binding,
            &Endpoint::None,
            Ulid::new(),
            "fixture",
            Priority::Next,
            false,
        ));
        assert_eq!(unavailable.evidence, Evidence::RecipientStored);
        assert_eq!(unavailable.cause, Some(WakeCause::NativeUnavailable));
    }
}
