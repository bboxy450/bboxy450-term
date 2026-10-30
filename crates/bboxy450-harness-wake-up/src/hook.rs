// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private hook entry point. No stdout, no user-config edits, no secret tokens in
//! frames. PTY labels are hints; only the capability admits off-PTY fallback.
use super::{Grade, Observation, Session, State, WaitingKind};
use crate::{Result, Ulid, invalid};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::time::{Duration, Instant};

/// Maximum serialized bounded hook payload, in bytes.
pub const MAX_RAW: usize = 64 * 1024;
/// Raw CLI hook ingress can include verbose tool content discarded before framing.
/// The retained frame stays at MAX_RAW and the shared hook deadline stays 50 ms.
pub const MAX_INPUT: usize = 2 * 1024 * 1024;
/// Maximum encoded terminal hook frame, in bytes.
pub const MAX_OSC: usize = 4096;
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Recognized hook event with selected bounded original JSON values.
pub struct ShimEvent {
    /// Recognized hook event name.
    pub event: String,
    /// Selected bounded original hook values, not an assistant-generated summary.
    pub raw: Value,
}
/// Select known hook fields and bound background-task detail before encoding.
///
/// # Errors
/// Refuses unknown/missing event names, non-object payloads, serialization failures or oversized payloads.
pub fn prepare(mut raw: Value, agent: &str) -> Result<ShimEvent> {
    let event = raw
        .get("hook_event_name")
        .or_else(|| raw.get("event"))
        .or_else(|| raw.get("type"))
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("hook event missing"))?
        .to_string();
    if !known(&event) {
        return Err(invalid("unknown hook event"));
    }
    if let Some(items) = raw
        .get_mut("background_tasks")
        .and_then(Value::as_array_mut)
    {
        let total = items.len();
        items.truncate(32);
        raw["background_tasks_total"] = total.into();
    }
    // Keep original status/provenance fields, not command output or file contents.
    // Origin: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    for (field, keep) in [
        (
            "tool_input",
            &["description", "questions", "question", "file_path", "path"][..],
        ),
        ("tool_response", &["agentId"][..]),
    ] {
        if let Some(object) = raw.get_mut(field).and_then(Value::as_object_mut) {
            object.retain(|key, _| keep.contains(&key.as_str()));
        } else if let Some(object) = raw.as_object_mut() {
            object.remove(field);
        }
    }
    let object = raw
        .as_object_mut()
        .ok_or_else(|| invalid("hook payload must be an object"))?;
    // Preserve selected raw values rather than generating an assistant summary.
    object.retain(|k, _| {
        matches!(
            k.as_str(),
            "session_id"
                | "hook_event_name"
                | "event"
                | "type"
                | "agent_id"
                | "agent_type"
                | "tool_name"
                | "tool_use_id"
                | "tool_input"
                | "tool_response"
                | "notification_type"
                | "message"
                | "reason"
                | "source"
                | "error"
                | "last_assistant_message"
                | "last_message"
                | "background_tasks"
                | "background_tasks_total"
        )
    });
    object.insert("harness".into(), agent.into());
    let frame = ShimEvent { event, raw };
    if serde_json::to_vec(&frame)?.len() > MAX_RAW {
        return Err(invalid("hook payload exceeds 64 KiB"));
    }
    Ok(frame)
}
/// Encode a hook OSC frame, or return `None` when the terminal-frame bound would be exceeded.
///
/// # Errors
/// Returns JSON encoding failures.
pub fn osc(seat: Ulid, event: &ShimEvent) -> Result<Option<Vec<u8>>> {
    let payload = STANDARD.encode(serde_json::to_vec(&serde_json::json!({"raw":event.raw}))?);
    let bytes = format!(
        "\x1b]7502;v=1:seat={seat}:ev={}:d={payload}\x1b\\",
        event.event
    )
    .into_bytes();
    Ok((bytes.len() <= MAX_OSC).then_some(bytes))
}
/// Return whether this event name belongs to a supported hook vocabulary.
pub fn known(event: &str) -> bool {
    matches!(
        event,
        "SessionStart"
            | "SessionEnd"
            | "UserPromptSubmit"
            | "PreToolUse"
            | "PostToolUse"
            | "PermissionRequest"
            | "Notification"
            | "Stop"
            | "StopFailure"
            | "SubagentStart"
            | "SubagentStop"
            | "PreCompact"
            | "PostCompact"
            | "session_start"
            | "session_end"
            | "user_prompt_submit"
            | "pre_tool_use"
            | "post_tool_use"
            | "stop"
            | "subagent_start"
            | "subagent_stop"
            | "pre_compact"
            | "post_compact"
            | "agent-turn-complete"
            | "approval-requested"
            | "async-question"
            | "plan-mode-prompt"
    )
}
pub(crate) fn parse(
    session: &mut Session,
    params: &[Vec<u8>],
    sequence: u64,
    now_ms: u64,
) -> Option<Observation> {
    let text = std::str::from_utf8(params.get(1)?).ok()?;
    let pairs: std::collections::BTreeMap<_, _> =
        text.split(':').filter_map(|p| p.split_once('=')).collect();
    if pairs.get("v") != Some(&"1") {
        return None;
    }
    let event = pairs.get("ev")?;
    if !known(event) {
        return None;
    }
    let payload = STANDARD.decode(pairs.get("d")?).ok()?;
    if payload.len() > MAX_RAW {
        return None;
    }
    // S4: `seat=` does not redirect bytes arriving on this session's PTY.
    project(
        session,
        &ShimEvent {
            event: (*event).into(),
            raw: serde_json::from_slice::<Value>(&payload)
                .ok()?
                .get("raw")?
                .clone(),
        },
        sequence,
        now_ms,
    )
}
/// Project a recognized bounded hook into the matching session; invalid observations are ignored.
pub fn project(
    session: &mut Session,
    event: &ShimEvent,
    sequence: u64,
    now_ms: u64,
) -> Option<Observation> {
    if !known(&event.event) || !event.raw.is_object() {
        return None;
    }
    let raw = &event.raw;
    // Claude's question tool opens a dialog even when permission checks are bypassed.
    // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    let asks_user = raw["tool_name"] == "AskUserQuestion"
        && matches!(event.event.as_str(), "PreToolUse" | "PostToolUse");
    if asks_user
        && (session.record.kind != "claude"
            || raw["harness"] != "claude"
            || raw["session_id"].as_str() != Some(session.binding.session.as_str())
            || raw["tool_use_id"]
                .as_str()
                .is_none_or(|id| id.is_empty() || id.len() > 256))
    {
        return None;
    }
    if matches!(event.event.as_str(), "SessionStart" | "session_start")
        && matches!(session.record.kind.as_str(), "claude" | "codex")
        && let Some(id) = raw["session_id"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 256)
        && id != session.binding.session
    {
        let mut binding = session.binding.clone();
        binding.session = id.into();
        binding.attachment_generation += 1;
        if session.rebind(binding).is_err() {
            return None;
        }
    }
    let (state, waiting) = match event.event.as_str() {
        "PreToolUse" if asks_user => (State::Waiting, Some(WaitingKind::Question)),
        "PermissionRequest" | "approval-requested" | "plan-mode-prompt" => {
            (State::Waiting, Some(WaitingKind::Permission))
        }
        "async-question" => (State::Waiting, Some(WaitingKind::Question)),
        "Notification" => match raw["notification_type"].as_str() {
            Some("permission_prompt") => (State::Waiting, Some(WaitingKind::Permission)),
            Some("agent_needs_input" | "elicitation_dialog" | "elicitation_started") => {
                (State::Waiting, Some(WaitingKind::Question))
            }
            Some("idle_prompt") => (State::Idle, None),
            _ => return None,
        },
        "StopFailure" => (
            State::Errored,
            raw["error"]
                .as_str()
                .filter(|s| {
                    matches!(
                        *s,
                        "authentication_failed"
                            | "oauth_org_not_allowed"
                            | "cloud_credential_error"
                    )
                })
                .map(|_| WaitingKind::Auth),
        ),
        "Stop"
        | "stop"
        | "SessionStart"
        | "session_start"
        | "SessionEnd"
        | "session_end"
        | "agent-turn-complete"
        | "SubagentStop"
        | "subagent_stop" => (State::Idle, None),
        _ => (State::Active, None),
    };
    let mut obs = session.observation(sequence, now_ms, "7502", Grade::Shim, state);
    obs.waiting_kind = waiting;
    let tool = raw["tool_use_id"].as_str();
    if state == State::Waiting && raw["agent_id"].as_str().is_none() {
        // A child question must not replace its parent's tool-resolution key.
        session.dialog_key = tool.map(str::to_owned);
    }
    obs.resolves = matches!(
        event.event.as_str(),
        "UserPromptSubmit"
            | "user_prompt_submit"
            | "SessionStart"
            | "session_start"
            | "SessionEnd"
            | "session_end"
    ) || (state != State::Waiting
        && matches!(
            event.event.as_str(),
            "PreToolUse" | "PostToolUse" | "pre_tool_use" | "post_tool_use"
        )
        && tool.is_some()
        && tool == session.dialog_key.as_deref());
    if matches!(event.event.as_str(), "SessionStart" | "session_start") {
        obs.clear = true;
        session.invalidate();
    }
    let subagent = matches!(
        event.event.as_str(),
        "SubagentStart" | "SubagentStop" | "subagent_start" | "subagent_stop"
    );
    let id = raw["agent_id"].as_str();
    // Codex hooks carry the PARENT session_id. Never turn it into a child row.
    if subagent && id.is_none() {
        return None;
    }
    if let Some(id) = id {
        // Hook/native IDs may be full UUIDs; OSC 7501's 32-byte sanitized-ID
        // grammar does not apply to this separate channel.
        if id.is_empty()
            || id.len() > 256
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.+-".contains(&b))
        {
            return None;
        }
        obs.agent_id = Some(id.into());
        obs.title = raw["agent_type"]
            .as_str()
            .map(|s| super::observe::short(s, 192));
        obs.clear = matches!(event.event.as_str(), "SubagentStop" | "subagent_stop");
    }
    if raw["tool_name"] == "Agent" || raw["tool_name"] == "Task" {
        if let (Some(id), Some(description)) = (tool, raw["tool_input"]["description"].as_str())
            && session.pending.len() < 32
        {
            session
                .pending
                .insert(id.into(), super::observe::short(description, 192));
        }
        if let Some(id) = raw["tool_response"]["agentId"].as_str()
            && let Some(title) = tool.and_then(|key| session.pending.remove(key))
            && let Some(row) = session.status.subagents.iter_mut().find(|s| s.id == id)
        {
            row.title = title;
        }
    }
    let question = (asks_user && state == State::Waiting).then(|| {
        raw["tool_input"]["questions"]
            .as_array()
            .and_then(|questions| questions.first())
            .and_then(|question| question["question"].as_str())
            .or_else(|| raw["tool_input"]["question"].as_str())
            .and_then(|question| question.lines().next())
            .map(str::trim)
            .filter(|question| !question.is_empty())
            .unwrap_or("Question needs your input.")
    });
    obs.detail = question
        .or_else(|| {
            raw["message"]
                .as_str()
                .or_else(|| raw["last_assistant_message"].as_str())
                .or_else(|| raw["last_message"].as_str())
                .or_else(|| raw["tool_name"].as_str())
        })
        .map(|s| super::observe::short(s, 192));
    Some(obs)
}

fn writable(fd: std::os::fd::RawFd, until: Instant) -> Result<()> {
    let mut p = libc::pollfd {
        fd,
        events: libc::POLLOUT,
        revents: 0,
    };
    // SAFETY: the pollfd is initialized, owns no pointers and fd stays alive.
    if unsafe {
        libc::poll(
            &mut p,
            1,
            until
                .saturating_duration_since(Instant::now())
                .as_millis()
                .min(50) as i32,
        )
    } <= 0
    {
        return Err(invalid("hook write deadline"));
    }
    Ok(())
}
fn write_deadline(stream: &mut (impl Write + AsRawFd), bytes: &[u8], until: Instant) -> Result<()> {
    let mut rest = bytes;
    while !rest.is_empty() {
        if Instant::now() >= until {
            return Err(invalid("hook write deadline"));
        }
        match stream.write(rest) {
            Ok(0) => return Err(invalid("hook write closed")),
            Ok(n) => rest = &rest[n..],
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                writable(stream.as_raw_fd(), until)?
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
/// Returns quietly even for malformed/unavailable hooks. It is called before
/// UI/runtime setup by the single binary's private `hook` dispatch.
pub fn run_cli(args: &[String], sink: &impl HookSink) {
    // `hook --list codex`: the hooks a Codex seat's launch overlay installs, one JSON line
    // each, for the hook-review recipe (#270). A harness never passes --list.
    if args.first().is_some_and(|a| a == "--list") {
        if args.get(1).is_some_and(|a| a == "codex")
            && let Ok(exe) = std::env::current_exe()
        {
            let command = format!(
                "{} hook --agent codex",
                super::launch::quote(&exe.to_string_lossy())
            );
            for event in super::launch::CODEX_HOOK_EVENTS {
                println!(
                    "{}",
                    serde_json::json!({"event": event, "command": command})
                );
            }
        }
        return;
    }
    let started = Instant::now();
    if let Err(error) = run(args, sink) {
        sink.failed(args, &error.to_string(), started.elapsed());
    }
}
/// The caller authenticates and encodes its own fallback wire. The shared 50 ms
/// deadline includes input, OSC and fallback delivery.
pub trait HookSink {
    /// A host can report a shim failure without delaying or failing the hook.
    fn failed(&self, _args: &[String], _reason: &str, _duration: Duration) {}
    /// Deliver a hook over the caller's authenticated fallback before its shared deadline.
    ///
    /// # Errors
    /// Returns the caller transport's deadline, admission or delivery failure.
    fn send(&self, seat: Ulid, event: ShimEvent, until: Instant) -> Result<()>;
}
fn run(args: &[String], sink: &impl HookSink) -> Result<()> {
    let until = Instant::now() + Duration::from_millis(50);
    let agent = args
        .windows(2)
        .find(|p| p[0] == "--agent")
        .map(|p| p[1].as_str())
        .unwrap_or("claude");
    let raw = if let Some(arg) = args.iter().rev().find(|s| s.starts_with('{')) {
        decode_input(arg.as_bytes())?
    } else {
        let mut bytes = Vec::new();
        let mut stdin = std::io::stdin();
        loop {
            let mut poll = libc::pollfd {
                fd: stdin.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: initialized pollfd, stdin remains alive throughout the call.
            if unsafe {
                libc::poll(
                    &mut poll,
                    1,
                    until.saturating_duration_since(Instant::now()).as_millis() as i32,
                )
            } <= 0
            {
                return Err(invalid("hook input deadline"));
            }
            let mut chunk = [0; 4096];
            let count = stdin.read(&mut chunk)?;
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..count]);
            if bytes.len() > MAX_INPUT {
                return Err(invalid("hook input bound"));
            }
        }
        decode_input(&bytes)?
    };
    let mut event = prepare(raw, agent)?;
    // Claude documents this export before SessionStart, unlike a PID file:
    // https://code.claude.com/docs/en/cross-session-messaging#the-sessions-inbox-socket
    // No messaging token is read or forwarded. Registration uses the admitted
    // socket lane; an OSC label must never select a native delivery endpoint.
    let native_socket =
        if agent == "claude" && matches!(event.event.as_str(), "SessionStart" | "Stop") {
            std::env::var("CLAUDE_CODE_MESSAGING_SOCKET").ok()
        } else {
            None
        };
    if let Some(path) = &native_socket {
        if !std::path::Path::new(path).is_absolute() || path.len() > 1024 {
            return Err(invalid("invalid Claude messaging socket path"));
        }
        event.raw["bboxy_native_socket"] = path.clone().into();
        if serde_json::to_vec(&event)?.len() > MAX_RAW {
            return Err(invalid("hook payload exceeds 64 KiB"));
        }
    }
    let seat = std::env::var("BBOXY_SEAT_ID")?.parse::<Ulid>()?;
    let socket_only = std::env::var_os("BBOXY_HOOK_SOCKET_ONLY").is_some_and(|value| value == "1");
    if !socket_only && native_socket.is_none() && let Some(frame)=osc(seat,&event)?
        && let Ok(mut tty)=std::fs::OpenOptions::new().write(true).custom_flags(libc::O_NONBLOCK).open("/dev/tty")
        // SAFETY: the open file owns this fd. isatty changes no state.
        && unsafe{libc::isatty(tty.as_raw_fd())}==1
        && write_deadline(&mut tty,&frame,until).is_ok()
    {
        return Ok(());
    }
    sink.send(seat, event, until)
}
/// Write an already encoded caller frame to its Unix socket within the shim's
/// deadline. No app protocol, capability or registration policy is interpreted.
///
/// # Errors
/// Refuses unsafe endpoints, expired deadlines or socket IO failures.
pub fn send_socket(path: &std::path::Path, bytes: &[u8], until: Instant) -> Result<()> {
    let mut stream = mio::net::UnixStream::connect(path)?;
    writable(stream.as_raw_fd(), until)?;
    if let Some(error) = stream.take_error()? {
        return Err(error.into());
    }
    write_deadline(&mut stream, bytes, until)
}

fn decode_input(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > MAX_INPUT {
        return Err(invalid("hook input bound"));
    }
    Ok(serde_json::from_slice(bytes)?)
}
#[cfg(test)]
mod input_tests {
    use super::*;
    #[test]
    fn large_hook_fields_codex_notify_discards_conversation_before_framing() {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "type":"agent-turn-complete","thread-id":"private-thread",
            "input-messages":["x".repeat(128*1024)],"last-assistant-message":"Done"
        }))
        .unwrap();
        assert!(bytes.len() > MAX_RAW);
        let event = prepare(decode_input(&bytes).unwrap(), "codex-notify").unwrap();
        assert_eq!(event.event, "agent-turn-complete");
        assert!(event.raw.get("input-messages").is_none());
        assert!(osc(Ulid::from(1_u128), &event).unwrap().is_some());
    }
    #[test]
    fn large_hook_fields_are_bounded_at_ingress_and_projected_before_wire_limits() {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "hook_event_name":"PostToolUse","session_id":"private", "tool_name":"Write",
            "tool_input":{"file_path":"/private/fixture.rs","content":"x".repeat(256*1024)},
            "tool_response":{"agentId":"worker","output":"y".repeat(256*1024)}
        }))
        .unwrap();
        assert!(bytes.len() > MAX_RAW);
        let event = prepare(decode_input(&bytes).unwrap(), "claude").unwrap();
        assert_eq!(event.raw["tool_input"]["file_path"], "/private/fixture.rs");
        assert_eq!(event.raw["tool_response"]["agentId"], "worker");
        assert!(event.raw["tool_input"].get("content").is_none());
        assert!(event.raw["tool_response"].get("output").is_none());
        assert!(serde_json::to_vec(&event).unwrap().len() < MAX_RAW);
        assert!(osc(Ulid::from(1_u128), &event).unwrap().is_some());
        assert!(decode_input(&vec![b' '; MAX_INPUT + 1]).is_err());
        assert!(
            prepare(
                serde_json::json!({"event":"Stop","last_message":"x".repeat(MAX_RAW)}),
                "codex"
            )
            .is_err(),
            "retained hook wire bound must not be loosened"
        );
    }
}
