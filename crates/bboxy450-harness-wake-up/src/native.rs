//! Installed native protocol contracts; no thread creation/resume, user config
//! changes, model calls or automatic resubmission on an ambiguous mutation.
use super::{
    Binding, Endpoint, Evidence, Grade, Observation, Priority, Session, State, WaitingKind,
    WakeOutcome,
};
use crate::{Result, Ulid, invalid};
use futures_util::{SinkExt, StreamExt};
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
    tungstenite::{Message, protocol::WebSocketConfig},
};

const LIMIT: usize = 64 * 1024;
pub const DEADLINE: Duration = Duration::from_secs(2);
pub async fn claude_preflight(path: &Path) -> Result<()> {
    own_socket(path)?;
    let _ = tokio::time::timeout(DEADLINE, UnixStream::connect(path)).await??;
    Ok(())
}
pub fn process_start(pid: u32) -> Result<String> {
    crate::assert_not_ui_thread();
    let output = std::process::Command::new("/bin/ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()?;
    let stamp = std::str::from_utf8(&output.stdout)?.trim();
    if !output.status.success() || stamp.is_empty() {
        return Err(invalid("harness process unavailable"));
    }
    Ok(stamp.into())
}
pub fn verify_process(binding: &Binding) -> Result<()> {
    binding.validate()?;
    if process_start(binding.pid)? != binding.process_start {
        return Err(invalid("harness process was replaced"));
    }
    Ok(())
}
/// Match the transport to the launch-selected runtime, not a user-global locator.
pub fn verify_codex_runtime(binding: &Binding, path: &Path) -> Result<()> {
    if binding.session.starts_with("process-") {
        return Err(invalid("native thread is not yet bound"));
    }
    verify_runtime_process(binding, path)
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
    pub binding: Binding,
    pub path: PathBuf,
}
impl ClaudeRegistration {
    pub fn matches(&self, binding: &Binding) -> bool {
        // A logic restart creates fresh in-memory attachment handles, while
        // the supervisor can keep the same native process and session alive.
        self.binding.seat == binding.seat
            && self.binding.pid == binding.pid
            && self.binding.process_start == binding.process_start
            && self.binding.generation == binding.generation
            && self.binding.session == binding.session
    }
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
pub struct ClaudeSessionFile {
    pub pid: u32,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "procStart")]
    pub process_start: String,
    pub status: String,
    #[serde(rename = "waitingFor", default)]
    pub waiting_for: Option<String>,
    #[serde(rename = "messagingSocketPath", default)]
    pub messaging_socket_path: Option<std::path::PathBuf>,
}
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
async fn open_codex(path: &Path, owner: Option<u32>) -> Result<Socket> {
    crate::assert_not_ui_thread();
    own_socket(path)?;
    let stream = UnixStream::connect(path).await?;
    if let Some(owner) = owner
        && stream
            .peer_cred()?
            .pid()
            .and_then(|pid| u32::try_from(pid).ok())
            != Some(owner)
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
    Ok(socket)
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

pub async fn codex_connect(path: &Path, thread: &str) -> Result<Socket> {
    Ok(codex_handshake(path, thread, None).await?.0)
}
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
    let unavailable = |note: &str| WakeOutcome {
        evidence: Evidence::RecipientStored,
        native_id: None,
        note: super::observe::short(note, 512),
    };
    if priority == Priority::Now && !allow_interrupt {
        return unavailable("caller lacks interrupt authority");
    }
    if text.len() > 48 * 1024 {
        return unavailable("native message exceeds 48 KiB; no mutation attempted");
    }
    if let Err(error) = verify_process(binding) {
        return unavailable(&format!(
            "native identity unavailable: {error}; no mutation attempted"
        ));
    }
    let mut attempted = false;
    let result=tokio::time::timeout(DEADLINE,async {
        match endpoint {
            Endpoint::None=>Err(invalid("native delivery unavailable")),
            Endpoint::Claude{path}=> {
                own_socket(path)?;let mut socket=UnixStream::connect(path).await?;
                let priority=match priority{Priority::Next=>"next",Priority::Later=>"later",Priority::Now=>"now"};
                let mut bytes=serde_json::to_vec(&json!({"type":"user","uuid":id.to_string(),"msg_id":id.to_string(),"priority":priority,"message":{"role":"user","content":text}}))?;
                bytes.push(b'\n');if bytes.len()>LIMIT{return Err(invalid("native frame bound"));}
                attempted=true;socket.write_all(&bytes).await?;socket.shutdown().await?;
                Ok(WakeOutcome{evidence:Evidence::NativeWritten,native_id:None,note:"frame written; receiver/model acceptance unverified".into()})
            }
            Endpoint::Codex{path,thread,turn}=> {
                if thread!=&binding.session {return Err(invalid("native thread binding mismatch"));}
                verify_codex_runtime(binding,path)?;
                let owner=binding.runtime.as_ref().ok_or_else(||invalid("native runtime is not bound"))?.pid;
                let (mut socket,_)=codex_handshake(path,thread,Some(owner)).await?;
                let input=json!([{"type":"text","text":text}]);
                let request=if priority==Priority::Now {
                    let expected=turn.as_ref().ok_or_else(||invalid("interrupt requires a current turn ID"))?;
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
                Ok(WakeOutcome{evidence:Evidence::NativeQueued,native_id:Some(native_id),note:"native admission only; model consumption unverified".into()})
            }
        }
    }).await;
    match result {
        Ok(Ok(value)) => value,
        Ok(Err(error)) if error.downcast_ref::<UnknownRuntime>().is_some() => {
            WakeOutcome::unknown(error.to_string())
        }
        Ok(Err(error)) if error.downcast_ref::<Rejected>().is_some() => {
            unavailable("native rejected submission")
        }
        _ if attempted => {
            WakeOutcome::unknown("native write/reply ambiguous; no PTY fallback or automatic retry")
        }
        Ok(Err(error)) => unavailable(&format!(
            "native preflight failed: {error}; no mutation attempted"
        )),
        Err(_) => unavailable("native preflight timed out; no mutation attempted"),
    }
}

#[cfg(test)]
mod registration_tests {
    use super::*;

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
        assert!(
            missing.note.contains("native preflight failed"),
            "{missing:?}"
        );
        assert!(
            missing
                .note
                .contains(&std::io::Error::from_raw_os_error(libc::ENOENT).to_string()),
            "{missing:?}"
        );
        assert!(missing.note.contains("no mutation attempted"));
        let mut replaced = binding;
        replaced.process_start.push_str(" replaced");
        let replaced = runtime.block_on(wake(
            &replaced,
            &Endpoint::None,
            Ulid::new(),
            "fixture",
            Priority::Next,
            false,
        ));
        assert_eq!(replaced.evidence, Evidence::RecipientStored);
        assert!(
            replaced.note.contains("harness process was replaced"),
            "{replaced:?}"
        );
    }
}
