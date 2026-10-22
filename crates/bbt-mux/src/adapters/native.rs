//! Installed native protocol contracts; no thread creation/resume, user config
//! changes, model calls or automatic resubmission on an ambiguous mutation.
use super::{
    Binding, Endpoint, Evidence, Grade, Observation, Priority, Session, State, WaitingKind,
    WakeOutcome,
};
use crate::{
    Ulid,
    store::{Result, invalid},
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;
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
    bbt_rt::assert_not_ui_thread();
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
fn own_socket(path: &Path) -> Result<()> {
    // Installed Codex publishes an owned symlink locator for its Unix socket.
    let meta = std::fs::metadata(path)?;
    // SAFETY: geteuid has no arguments or side effects.
    if !meta.file_type().is_socket() || meta.uid() != unsafe { libc::geteuid() } {
        return Err(invalid("native endpoint must be an owned Unix socket"));
    }
    Ok(())
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
    bbt_rt::assert_not_ui_thread();
    verify_process(binding)?;
    read_claude_bound(path, binding)
}
/// The supervisor has already pinned process start and owns/reaps this child.
/// Avoid forking ps on every native-file poll; still match all stored fields.
pub(crate) fn read_claude_bound(path: &Path, binding: &Binding) -> Result<ClaudeSessionFile> {
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
async fn codex_handshake(path: &Path, thread: &str) -> Result<(Socket, Value)> {
    bbt_rt::assert_not_ui_thread();
    own_socket(path)?;
    let stream = UnixStream::connect(path).await?;
    let config = WebSocketConfig::default()
        .max_message_size(Some(LIMIT))
        .max_frame_size(Some(LIMIT));
    let (mut socket, _) =
        tokio_tungstenite::client_async_with_config("ws://localhost/", stream, Some(config))
            .await?;
    send(&mut socket,json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"bboxy450-term","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}}})).await?;
    reply(&mut socket, 1).await?;
    send(&mut socket, json!({"method":"initialized"})).await?;
    send(
        &mut socket,
        json!({"id":2,"method":"thread/read","params":{"threadId":thread,"includeTurns":false}}),
    )
    .await?;
    let identity = reply(&mut socket, 2).await?;
    if identity["thread"]["id"] != thread
        || !matches!(
            identity["thread"]["status"]["type"].as_str(),
            Some("idle" | "active")
        )
    {
        return Err(invalid("native thread identity/status mismatch"));
    }
    Ok((socket, identity))
}
pub async fn codex_connect(path: &Path, thread: &str) -> Result<Socket> {
    Ok(codex_handshake(path, thread).await?.0)
}
pub async fn codex_status(path: &Path, thread: &str) -> Result<Value> {
    let (_, identity) = tokio::time::timeout(DEADLINE, codex_handshake(path, thread)).await??;
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
    bbt_rt::assert_not_ui_thread();
    let unavailable = |note: &str| WakeOutcome {
        evidence: Evidence::RecipientStored,
        native_id: None,
        note: note.into(),
    };
    if priority == Priority::Now && !allow_interrupt {
        return unavailable("caller lacks interrupt authority");
    }
    if text.len() > 48 * 1024 || verify_process(binding).is_err() {
        return unavailable("native identity unavailable or message too large");
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
                let mut socket=codex_connect(path,thread).await?;
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
        Ok(Err(error)) if error.downcast_ref::<Rejected>().is_some() => {
            unavailable("native rejected submission")
        }
        _ if attempted => {
            WakeOutcome::unknown("native write/reply ambiguous; no PTY fallback or automatic retry")
        }
        _ => unavailable("native attachment unavailable; no mutation attempted"),
    }
}
