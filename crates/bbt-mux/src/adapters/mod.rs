//! Harness-specific launch, observation and guarded delivery. IO belongs to the
//! mux worker/R1 handle; observations never authorize local tool execution.
pub mod guard;
pub mod hook;
pub mod launch;
pub mod native;
pub mod observe;

use crate::{
    Ulid,
    registry::{Class, Role},
    store::{Result, invalid},
};
use serde::{Deserialize, Serialize};

pub mod mapping;
pub use bboxy450_harness_wake_up::{
    ArbiterView, Binding, Endpoint, NativeAttachment, Priority, Readiness, ReadyToken, StateSource,
    WakeMethod, native_available,
};
pub use launch::{LaunchContext, LaunchPlan};
pub use observe::{Grade, Observation, OscTap, Session, State, Status, WaitingKind};

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

/// Enforce harness authority before admitting a launch, including direct PTYs.
pub fn validate_role(kind: &str, role: &Role) -> Result<()> {
    if kind == "codex" && role.class == Class::Oracle {
        return Err(invalid("codex_oracle_role_forbidden"));
    }
    if role.class != role_preset(&role.name)?.class {
        return Err(invalid("role_class_mismatch"));
    }
    Ok(())
}

/// Resolve the five decided role names to their grant authority (#262).
pub fn role_preset(name: &str) -> Result<Role> {
    let class = match name {
        "oracle" | "architect" => Class::Oracle,
        "creativity" | "executor" | "shell" => Class::Worker,
        _ => return Err(invalid("unknown_role_preset")),
    };
    Ok(Role {
        name: name.into(),
        class,
    })
}

impl HarnessRecord {
    /// A spawn preset changes the name and class together; absence uses settings.
    pub fn resolve_role(&self, preset: Option<&str>) -> Result<Role> {
        let role = role_preset(preset.unwrap_or(&self.default_role.name))?;
        validate_role(&self.kind, &role)?;
        Ok(role)
    }

    pub fn builtin(kind: &str) -> Result<Self> {
        let record = bboxy450_harness_wake_up::HarnessRecord::builtin(kind)?;
        Ok(Self {
            kind: record.kind,
            name: record.name,
            command: record.command,
            tile: "#7397d4".into(),
            icon: kind.into(),
            default_role: role_preset(match kind {
                "claude" => "oracle",
                "zsh" => "shell",
                _ => "executor",
            })?,
            wake_method: record.wake_method,
            state_source: record.state_source,
            hooks: record.hooks,
        })
    }
    pub fn validate(&self) -> Result<()> {
        mapping::harness(self).validate()?;
        validate_role(&self.kind, &self.default_role)
    }
}

pub use bbt_proto::messaging::Evidence;
pub type AdapterFuture<'a, T> = bboxy450_harness_wake_up::AdapterFuture<'a, T>;
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
    fn record(&self) -> &bboxy450_harness_wake_up::HarnessRecord;
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
}
impl Adapter for Session {
    fn record(&self) -> &bboxy450_harness_wake_up::HarnessRecord {
        &self.inner.record
    }
    fn launch(&self, context: &LaunchContext<'_>) -> Result<LaunchPlan> {
        launch::plan_neutral(&self.inner.record, context)
    }
    fn observe_channels(&self) -> &'static [&'static str] {
        bboxy450_harness_wake_up::Adapter::observe_channels(&self.inner)
    }
    fn on_osc(&mut self, params: &[Vec<u8>], sequence: u64, now_ms: u64) -> Vec<Observation> {
        bboxy450_harness_wake_up::Adapter::on_osc(&mut self.inner, params, sequence, now_ms)
    }
    fn on_native(
        &mut self,
        event: &serde_json::Value,
        sequence: u64,
        now_ms: u64,
    ) -> Result<Vec<Observation>> {
        bboxy450_harness_wake_up::Adapter::on_native(&mut self.inner, event, sequence, now_ms)
    }
    fn input_guard(&self, arbiter: &ArbiterView, now_ms: u64) -> Readiness {
        bboxy450_harness_wake_up::Adapter::input_guard(&self.inner, arbiter, now_ms)
    }
    fn attach(&self, endpoint: Endpoint) -> AdapterFuture<'_, Result<NativeAttachment>> {
        bboxy450_harness_wake_up::Adapter::attach(&self.inner, endpoint)
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
            let outcome = bboxy450_harness_wake_up::Adapter::wake(
                &mut self.inner,
                attachment,
                id,
                text,
                priority,
                allow_interrupt,
            )
            .await;
            mapping::outcome(outcome)
        })
    }
}
