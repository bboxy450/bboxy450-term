//! Harness-specific launch, observation and guarded delivery. IO belongs to the
//! mux worker/R1 handle; observations never authorize local tool execution.
pub mod guard;
pub mod hook;
pub mod launch;
pub mod native;
pub mod observe;

use crate::{
    Ulid,
    registry::Role,
    store::{Result, invalid},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub use guard::{ArbiterView, Readiness, ReadyToken};
pub use launch::{LaunchContext, LaunchPlan};
pub use observe::{Grade, Observation, OscTap, Session, State, Status, WaitingKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeMethod {
    ClaudeSocket,
    CodexRpc,
    Pty,
    Operator,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateSource {
    Reported,
    Hooks,
    Cadence,
}

/// `kind` is open: custom harnesses use the generic adapter, never an enum cast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessRecord {
    pub kind: String,
    pub name: String,
    pub command: Vec<String>,
    pub tile: String,
    pub icon: String,
    pub default_role: Role,
    pub wake_method: WakeMethod,
    pub state_source: StateSource,
    pub hooks: bool,
}
impl HarnessRecord {
    pub fn builtin(kind: &str) -> Result<Self> {
        let (name, command, wake_method) = match kind {
            "claude" => ("Claude", "claude", WakeMethod::ClaudeSocket),
            "codex" => ("Codex", "codex", WakeMethod::CodexRpc),
            "zsh" => ("zsh", "/bin/zsh", WakeMethod::Pty),
            _ => return Err(invalid("not a built-in harness")),
        };
        Ok(Self {
            kind: kind.into(),
            name: name.into(),
            command: vec![command.into()],
            tile: "#7397d4".into(),
            icon: kind.into(),
            default_role: Role {
                name: "executor".into(),
                class: crate::registry::Class::Worker,
            },
            wake_method,
            state_source: StateSource::Reported,
            hooks: kind != "zsh",
        })
    }
    pub fn validate(&self) -> Result<()> {
        if self.kind.is_empty()
            || self.kind.len() > 128
            || self.name.len() > 192
            || self.command.is_empty()
            || self.command.len() > 128
            || self.command.iter().any(|s| s.contains('\0'))
            || self.command[0].is_empty()
        {
            return Err(invalid("invalid harness record"));
        }
        Ok(())
    }
}

/// Native identity is revalidated on attachment; the process start stamp makes
/// a reused PID insufficient. Session strings are native IDs, not our ULIDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub seat: Ulid,
    pub session: String,
    pub pid: u32,
    pub process_start: String,
    pub generation: u64,
    pub attachment_generation: u64,
}
impl Binding {
    pub fn validate(&self) -> Result<()> {
        if self.seat == Ulid::nil()
            || self.session.is_empty()
            || self.session.len() > 256
            || self.pid == 0
            || self.process_start.is_empty()
            || self.generation == 0
            || self.attachment_generation == 0
        {
            return Err(invalid("invalid harness attachment identity"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Endpoint {
    Claude {
        path: PathBuf,
    },
    Codex {
        path: PathBuf,
        thread: String,
        turn: Option<String>,
    },
    None,
}
#[derive(Debug, Clone)]
pub struct NativeAttachment {
    binding: Binding,
    endpoint: Endpoint,
}
impl NativeAttachment {
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
}
pub type AdapterFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Next,
    Later,
    Now,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Evidence {
    SenderStored,
    RecipientStored,
    NativeQueued,
    NativeWritten,
    NativeUnknown,
    ApplicationAcknowledged,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakeOutcome {
    pub evidence: Evidence,
    pub native_id: Option<String>,
    pub note: String,
}
impl WakeOutcome {
    pub fn unknown(note: impl Into<String>) -> Self {
        Self {
            evidence: Evidence::NativeUnknown,
            native_id: None,
            note: note.into(),
        }
    }
}

pub trait Adapter: Send + Sync {
    fn record(&self) -> &HarnessRecord;
    fn launch(&self, context: &LaunchContext<'_>) -> Result<LaunchPlan>;
    fn observe_channels(&self) -> &'static [&'static str];
    fn on_osc(&mut self, params: &[Vec<u8>], sequence: u64, now_ms: u64) -> Vec<Observation>;
    fn on_native(
        &mut self,
        event: &serde_json::Value,
        sequence: u64,
        now_ms: u64,
    ) -> Result<Vec<Observation>>;
    fn input_guard(&self, arbiter: &ArbiterView, now_ms: u64) -> Readiness;
    fn attach(&self, endpoint: Endpoint) -> AdapterFuture<'_, Result<NativeAttachment>>;
    fn wake<'a>(
        &'a mut self,
        attachment: &'a NativeAttachment,
        id: Ulid,
        text: &'a str,
        priority: Priority,
        allow_interrupt: bool,
    ) -> AdapterFuture<'a, WakeOutcome>;
    fn resume_command(&self, cwd: &Path) -> Option<Vec<String>>;
}
impl Adapter for Session {
    fn record(&self) -> &HarnessRecord {
        &self.record
    }
    fn launch(&self, context: &LaunchContext<'_>) -> Result<LaunchPlan> {
        launch::plan(&self.record, context)
    }
    fn observe_channels(&self) -> &'static [&'static str] {
        match self.record.kind.as_str() {
            "claude" => &["7501", "7502", "sessions_file", "cadence", "exit"],
            "codex" => &["7501", "app_server", "7502", "21337", "cadence", "exit"],
            "zsh" => &["133", "7", "cadence", "exit"],
            _ => &["7501", "7502", "cadence", "exit"],
        }
    }
    fn on_osc(&mut self, params: &[Vec<u8>], sequence: u64, now_ms: u64) -> Vec<Observation> {
        self.osc(params, sequence, now_ms)
    }
    fn on_native(
        &mut self,
        event: &serde_json::Value,
        sequence: u64,
        now_ms: u64,
    ) -> Result<Vec<Observation>> {
        native::observe(self, event, sequence, now_ms)
    }
    fn input_guard(&self, arbiter: &ArbiterView, now_ms: u64) -> Readiness {
        guard::check(self, arbiter, now_ms)
    }
    fn attach(&self, endpoint: Endpoint) -> AdapterFuture<'_, Result<NativeAttachment>> {
        Box::pin(async move {
            native::verify_process(&self.binding)?;
            match &endpoint {
                Endpoint::Claude { path } if self.record.kind == "claude" => {
                    native::claude_preflight(path).await?
                }
                Endpoint::Codex { path, thread, .. }
                    if self.record.kind == "codex" && thread == &self.binding.session =>
                {
                    native::codex_status(path, thread).await?;
                }
                _ => {
                    return Err(invalid(
                        "native endpoint does not match this adapter/session",
                    ));
                }
            }
            Ok(NativeAttachment {
                binding: self.binding.clone(),
                endpoint,
            })
        })
    }
    fn wake<'a>(
        &'a mut self,
        attachment: &'a NativeAttachment,
        id: Ulid,
        text: &'a str,
        priority: Priority,
        allow_interrupt: bool,
    ) -> AdapterFuture<'a, WakeOutcome> {
        Box::pin(async move {
            if attachment.binding != self.binding {
                return WakeOutcome {
                    evidence: Evidence::RecipientStored,
                    native_id: None,
                    note: "native attachment was replaced".into(),
                };
            }
            if !self.can_attempt_native(id) {
                return WakeOutcome::unknown(
                    "original native outcome is unknown; query it, never resubmit",
                );
            }
            let outcome = native::wake(
                &self.binding,
                &attachment.endpoint,
                id,
                text,
                priority,
                allow_interrupt,
            )
            .await;
            self.record_outcome(id, &outcome);
            outcome
        })
    }
    fn resume_command(&self, _cwd: &Path) -> Option<Vec<String>> {
        match self.record.kind.as_str() {
            "claude" => Some(vec![
                self.record.command[0].clone(),
                "--resume".into(),
                self.binding.session.clone(),
            ]),
            "codex" => Some(vec![
                self.record.command[0].clone(),
                "resume".into(),
                self.binding.session.clone(),
            ]),
            "zsh" => Some(self.record.command.clone()),
            _ => None,
        }
    }
}
