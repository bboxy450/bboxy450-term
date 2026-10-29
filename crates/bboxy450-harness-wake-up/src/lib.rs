// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![doc = include_str!("../README.md")]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]
// The extracted module was covered by mux's same IO-owner allowance. Callers
// run it on IO threads; the optional runtime assertion retains the app's guard.
#![allow(
    clippy::disallowed_methods,
    reason = "harness protocol and launch IO, never UI"
)]
#[cfg(not(unix))]
compile_error!("bboxy450-harness-wake-up currently requires Unix sockets and terminals");

/// Single-use positive readiness and guarded literal terminal input.
pub mod guard;
/// Bounded hook events, OSC projection and caller-authenticated fallback delivery.
pub mod hook;
/// Isolated launch overlays, native runtime plans and owned cleanup.
pub mod launch;
/// Bound process/session native transport contracts and evidence.
pub mod native;
/// Graded activity observations, state projection and bounded terminal taps.
pub mod observe;

pub use ulid::Ulid;
/// Errors from protocol validation or IO, without an app/host dependency.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
/// The kit's fallible result.
pub type Result<T> = std::result::Result<T, Error>;
fn invalid(message: impl Into<String>) -> Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into()).into()
}
fn assert_not_ui_thread() {
    #[cfg(feature = "runtime-assertions")]
    bbt_rt::assert_not_ui_thread();
}

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use guard::{ArbiterView, LiteralInput, Readiness, ReadinessRefusal, ReadyToken};
pub use launch::{LaunchContext, LaunchPlan};
pub use observe::{Grade, Observation, OscTap, Session, State, Status, WaitingKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Preferred delivery mechanism recorded for a harness kind.
pub enum WakeMethod {
    /// Bound cross-session socket transport.
    ClaudeSocket,
    /// Bound per-process app-server transport.
    CodexRpc,
    /// Guarded terminal input under the caller's writer lease.
    Pty,
    /// Caller handoff when no supported automatic transport exists.
    Operator,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Selected source policy for harness activity observations.
pub enum StateSource {
    /// Use explicit harness/native state reports.
    Reported,
    /// Use bounded hook-derived state reports.
    Hooks,
    /// Infer activity from observed output cadence.
    Cadence,
}

/// `kind` is open: custom harnesses use the generic adapter, never an enum cast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessRecord {
    /// Open harness-kind identifier; custom values select the generic adapter.
    pub kind: String,
    /// Caller-facing name associated with this launch or harness record.
    pub name: String,
    /// Program and ordered arguments supplied to the process host.
    pub command: Vec<String>,
    /// Preferred transport category, not proof that an attachment is currently available.
    pub wake_method: WakeMethod,
    /// Selected activity observation policy.
    pub state_source: StateSource,
    /// Whether the harness record requests launch-time hook wiring.
    pub hooks: bool,
}

impl HarnessRecord {
    /// Construct one supported built-in record.
    ///
    /// # Errors
    /// Refuses kinds other than the supported built-in harnesses.
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
            wake_method,
            state_source: StateSource::Reported,
            hooks: kind != "zsh",
        })
    }
    /// Check the open kind, display name and nonempty command against the record bounds.
    ///
    /// # Errors
    /// Refuses an empty/oversized kind, oversized name, empty/oversized argument list,
    /// empty executable or an argument containing a NUL byte.
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
    /// Caller-owned opaque process identity.
    pub seat: Ulid,
    /// Native harness session identity, not the host ULID.
    pub session: String,
    /// Bound operating-system process identifier.
    pub pid: u32,
    /// Observed process start stamp used to reject PID reuse.
    pub process_start: String,
    /// Nonzero host process-generation fence.
    pub generation: u64,
    /// Nonzero attachment-generation fence.
    pub attachment_generation: u64,
    /// The explicitly selected runtime hosting this native thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RuntimeBinding>,
}

/// Identity of the selected runtime, owned by the seat or borrowed from Codex.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeBinding {
    /// Explicit endpoint of the selected native runtime; conveys no ownership.
    pub endpoint: std::path::PathBuf,
    /// Bound operating-system process identifier.
    pub pid: u32,
    /// Observed process start stamp used to reject PID reuse.
    pub process_start: String,
}
impl Binding {
    /// Check process, session, generation and optional runtime identity shape.
    ///
    /// This checks the stored fields; [`native::verify_process`] checks the live process.
    ///
    /// # Errors
    /// Refuses missing identities, zero fences, an oversized session identifier
    /// or an invalid optional runtime endpoint/process identity.
    pub fn validate(&self) -> Result<()> {
        if self.seat == Ulid::nil()
            || self.session.is_empty()
            || self.session.len() > 256
            || self.pid == 0
            || self.process_start.is_empty()
            || self.generation == 0
            || self.attachment_generation == 0
            || self.runtime.as_ref().is_some_and(|r| {
                !r.endpoint.is_absolute() || r.pid == 0 || r.process_start.is_empty()
            })
        {
            return Err(invalid("invalid harness attachment identity"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
/// Explicit endpoint for one bound native attachment.
pub enum Endpoint {
    /// Bound cross-session socket endpoint.
    Claude {
        /// Explicit caller-selected native Unix socket path.
        path: PathBuf,
    },
    /// Bound app-server endpoint and native thread/turn identity.
    Codex {
        /// Explicit caller-selected native Unix socket path.
        path: PathBuf,
        /// Exact native thread identifier bound to the attachment.
        thread: String,
        /// Expected active turn identifier required for interrupt steering.
        turn: Option<String>,
    },
    /// No native endpoint is attached.
    None,
}
#[derive(Debug, Clone)]
/// Preflighted native endpoint bound to a specific process/session generation.
pub struct NativeAttachment {
    binding: Binding,
    endpoint: Endpoint,
}
impl NativeAttachment {
    /// Borrow the exact identity captured by this native attachment.
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
}
/// Sendable asynchronous adapter operation borrowing its caller-owned session state.
pub type AdapterFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Caller delivery intent; queue scheduling and steering remain distinct.
pub enum Priority {
    /// Queue for the next opportunity; higher-level scheduling remains with the caller.
    Next,
    /// Queue with later intent; this does not create a second native scheduler.
    Later,
    /// Request steering only under explicit caller interrupt authority.
    Now,
}
/// Delivery evidence ends at native transport; model consumption is not implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Evidence {
    /// Stored by the sender-side owner; no recipient receipt is established.
    SenderStored,
    /// Retained by the recipient-side owner; native acceptance is not established.
    RecipientStored,
    /// Verified native queue or steer admission; model consumption remains unverified.
    NativeQueued,
    /// Frame written to the native transport; receiver acceptance remains unverified.
    NativeWritten,
    /// Mutation may have occurred; query it without automatic retry or PTY fallback.
    NativeUnknown,
}
/// Bounded explanation of an outcome, never input, retry or receipt authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WakeCause {
    /// The retained attachment belongs to another binding.
    NativeAttachmentReplaced,
    /// This delivery identity already has an unresolved native outcome.
    PreviousOutcomeUnknown,
    /// The caller has no interrupt authority.
    InterruptUnauthorized,
    /// The native message exceeds its existing transport bound.
    MessageTooLarge,
    /// The live process identity could not be established.
    ProcessIdentityUnavailable,
    /// The process start stamp differs from the captured identity.
    ProcessReplaced,
    /// No native endpoint is attached.
    NativeUnavailable,
    /// The selected native thread differs from the bound session.
    NativeThreadMismatch,
    /// Startup has not bound an authenticated native thread yet.
    NativeThreadUnbound,
    /// Steering lacks a current native turn identifier.
    CurrentTurnMissing,
    /// Native thread identity or status is unknown.
    NativeThreadUnknown,
    /// Native preflight failed before any submission was attempted.
    NativePreflightFailed,
    /// Native preflight exceeded its existing deadline.
    NativePreflightTimedOut,
    /// The native peer explicitly rejected the submission.
    NativeRejected,
    /// A native write or reply is ambiguous; never automatically resubmit.
    NativeWriteAmbiguous,
    /// A native frame was written; receiver/model acceptance is unverified.
    NativeFrameWritten,
    /// The native queue admitted the message; model consumption is unverified.
    NativeQueueAdmitted,
    /// Input facts refer to another process/session/attachment.
    InputBindingChanged,
    /// A dialog owns input.
    DialogPending,
    /// A turn or subagent is active.
    TurnActive,
    /// No positive composer readiness has been observed.
    ComposerReadinessMissing,
    /// The observed readiness is expired or belongs to an older binding.
    ComposerReadinessExpired,
    /// A draft, tool or render transition owns input.
    InputOwned,
    /// The foreground process differs from the input owner.
    ForegroundChanged,
    /// The shell is not in canonical echo mode.
    ShellModeInvalid,
    /// Output is too recent for the existing readiness guard.
    RecentOutput,
    /// This harness has no supported autonomous input path.
    OperatorHandoff,
    /// PTY intent was journaled before the writer; its result is unresolved.
    PtySubmissionPending,
    /// The supervisor confirmed the full PTY write, not model consumption.
    PtyWritten,
    /// The supervisor refused the input before mutation.
    SupervisorRefused,
    /// The PTY result could not be retained after the attempted write.
    PtyResultUnjournaled,
    /// The target subagent ended or is no longer reported.
    TargetSubagentUnavailable,
    /// Native attachment discovery failed before submission.
    NativeAttachmentUnavailable,
    /// Live PTY arbiter facts are unavailable.
    PtyArbiterUnavailable,
    /// A caught-up PTY and fresh readiness after prior input are still required.
    PtyReadinessPending,
    /// Logic restarted while an attempt had no retained result.
    RestartedInFlight,
    /// A future wire cause is unknown; it grants no additional authority.
    #[serde(other)]
    Unrecognized,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Native transport evidence and a named diagnostic for one stable delivery identity.
pub struct WakeOutcome {
    /// Transport-level evidence, without a claim of model consumption.
    pub evidence: Evidence,
    /// Native queue or turn identifier returned by the transport, if one was verified.
    pub native_id: Option<String>,
    /// Bounded diagnostic detail explaining the delivery evidence.
    pub note: String,
    /// Optional machine-readable explanation; omitted legacy fields keep their bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<WakeCause>,
}
impl WakeOutcome {
    /// Construct ambiguous native evidence that forbids automatic resubmission.
    pub fn unknown(note: impl Into<String>) -> Self {
        Self {
            evidence: Evidence::NativeUnknown,
            native_id: None,
            note: note.into(),
            cause: None,
        }
    }
    /// Construct a named ambiguous outcome without granting automatic resubmission.
    pub fn unknown_cause(cause: WakeCause, note: impl Into<String>) -> Self {
        Self {
            cause: Some(cause),
            ..Self::unknown(observe::short(&note.into(), 640))
        }
    }
}

/// Harness observation, launch and guarded native-delivery port; application policy stays with the caller.
pub trait Adapter: Send + Sync {
    /// Borrow the selected harness record.
    fn record(&self) -> &HarnessRecord;
    /// Prepare isolated launch wiring on IO.
    ///
    /// # Errors
    /// Returns record/path validation, overlay or serialization/IO failures.
    fn launch(&self, context: &LaunchContext<'_>) -> Result<LaunchPlan>;
    /// List recognized observation-channel labels; this is not a live transport-availability claim.
    fn observe_channels(&self) -> &'static [&'static str];
    /// Decode bounded terminal observations without granting off-terminal authority.
    fn on_osc(&mut self, params: &[Vec<u8>], sequence: u64, now_ms: u64) -> Vec<Observation>;
    /// Project a native event after validating its session identity.
    ///
    /// # Errors
    /// Returns invalid or mismatched native-event identity/shape failures.
    fn on_native(
        &mut self,
        event: &serde_json::Value,
        sequence: u64,
        now_ms: u64,
    ) -> Result<Vec<Observation>>;
    /// Check current caller facts against positive, short-lived composer readiness.
    fn input_guard(&self, arbiter: &ArbiterView, now_ms: u64) -> Readiness;
    /// Validate and attach an exact native process/session endpoint.
    ///
    /// # Errors
    /// Returns process, ownership, endpoint, native identity or preflight failures.
    fn attach(&self, endpoint: Endpoint) -> AdapterFuture<'_, Result<NativeAttachment>>;
    /// Attempt delivery under caller authority and return transport evidence; ambiguous outcomes are retained.
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
    /// Project a native event after validating its session identity.
    ///
    /// # Errors
    /// Returns invalid or mismatched native-event identity/shape failures.
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
                    native::verify_codex_runtime(&self.binding, path)?;
                    native::codex_owned_status(&self.binding).await?;
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
    /// Attempt delivery under caller authority and return transport evidence; ambiguous outcomes are retained.
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
                    cause: Some(WakeCause::NativeAttachmentReplaced),
                };
            }
            if !self.can_attempt_native(id) {
                return WakeOutcome::unknown_cause(
                    WakeCause::PreviousOutcomeUnknown,
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
}

/// Whether an established attachment belongs to this live process/session and
/// generation. This is not proof of receiver/model consumption; wake revalidates
/// the native process and reports pre-write failures or ambiguous writes.
pub fn native_available(attachment: Option<&NativeAttachment>, binding: &Binding) -> bool {
    attachment.is_some_and(|a| a.binding == *binding && !matches!(a.endpoint, Endpoint::None))
}

#[cfg(test)]
mod availability_tests {
    use super::*;
    // keeps: spec harness-kits §5
    #[test]
    fn native_availability_is_bound_to_the_exact_live_attachment() {
        let binding = Binding {
            seat: Ulid::new(),
            session: "session".into(),
            pid: 42,
            process_start: "start".into(),
            generation: 1,
            attachment_generation: 2,
            runtime: None,
        };
        let attachment = NativeAttachment {
            binding: binding.clone(),
            endpoint: Endpoint::Claude {
                path: "/tmp/fixture.sock".into(),
            },
        };
        assert!(!native_available(None, &binding));
        assert!(native_available(Some(&attachment), &binding));
        for field in 0..6 {
            let mut changed = binding.clone();
            match field {
                0 => changed.seat = Ulid::new(),
                1 => changed.session.push_str(" changed"),
                2 => changed.pid += 1,
                3 => changed.process_start.push_str(" changed"),
                4 => changed.generation += 1,
                _ => changed.attachment_generation += 1,
            }
            assert!(
                !native_available(Some(&attachment), &changed),
                "field {field}"
            );
        }
        let absent = NativeAttachment {
            binding: binding.clone(),
            endpoint: Endpoint::None,
        };
        assert!(!native_available(Some(&absent), &binding));
    }
}

#[cfg(test)]
mod wake_cause_tests {
    use super::*;

    // keeps: interface app-harness — additive cause preserves legacy bytes.
    #[test]
    fn wake_causes_preserve_legacy_bytes_and_decode_future_causes_safely() {
        for bytes in [
            r#"{"evidence":"native-unknown","native_id":null,"note":"legacy diagnostic"}"#,
            r#"{"evidence":"native-queued","native_id":"q1","note":"admission only"}"#,
            r#"{"evidence":"recipient-stored","native_id":null,"note":"specific refusal"}"#,
        ] {
            let outcome: WakeOutcome = serde_json::from_str(bytes).unwrap();
            assert_eq!(outcome.cause, None);
            assert_eq!(serde_json::to_string(&outcome).unwrap(), bytes);
        }
        let future: WakeOutcome = serde_json::from_str(
            r#"{"evidence":"native-unknown","native_id":null,"note":"opaque peer","cause":"future-peer-state"}"#,
        ).unwrap();
        assert_eq!(future.cause, Some(WakeCause::Unrecognized));
        assert_eq!(future.evidence, Evidence::NativeUnknown);
        assert_eq!(future.note, "opaque peer");
        let typed =
            WakeOutcome::unknown_cause(WakeCause::NativeWriteAmbiguous, "concrete diagnostic");
        assert_eq!(
            serde_json::to_value(&typed).unwrap()["cause"],
            "native-write-ambiguous"
        );
        assert_eq!(
            serde_json::from_slice::<WakeOutcome>(&serde_json::to_vec(&typed).unwrap()).unwrap(),
            typed
        );
    }
}
