// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Readiness is a short-lived, single-use indication bound to the live process,
//! native session, attachment and operator-input epoch. State alone is not one.
use super::{Binding, Evidence, Session, State, WakeCause, WakeOutcome};
use crate::{Result, Ulid, invalid};

/// Maximum encoded literal paste, matching the existing host input bound.
pub const MAX_INPUT_BYTES: usize = 64 * 1024;

/// Transport-independent arbiter facts sampled by the caller.
#[derive(Debug, Clone)]
pub struct ArbiterView {
    /// Exact process, native-session and attachment identity being observed.
    pub binding: Binding,
    /// Caller input-ownership epoch that must still match at guarded input.
    pub operator_epoch: u64,
    /// Caller-observed foreground process identity.
    pub foreground_pid: Option<u32>,
    /// Whether an operator draft currently owns input.
    pub operator_draft: bool,
    /// Whether an active tool currently owns input.
    pub active_tool: bool,
    /// Whether synchronized terminal output is pending.
    pub sync_pending: bool,
    /// Whether a screen transition is pending.
    pub screen_switch_pending: bool,
    /// Whether canonical terminal input processing was observed.
    pub canonical: bool,
    /// Whether terminal echo was observed.
    pub echo: bool,
    /// Whether bracketed-paste input framing is currently enabled.
    pub bracketed_paste: bool,
}
#[derive(Debug)]
/// Single-use positive readiness bound to identity, proof revision, input epoch and expiry.
pub struct ReadyToken {
    binding: Binding,
    proof_revision: u64,
    operator_epoch: u64,
    sequence: u64,
    expires_ms: u64,
}
#[derive(Debug)]
/// Current readiness result; activity state alone cannot produce a positive token.
pub enum Readiness {
    /// Positive short-lived composer readiness that must be rechecked and consumed.
    Ready(ReadyToken),
    /// Named current reason automatic literal input cannot be admitted.
    NotReady(ReadinessRefusal),
    /// The selected harness has no supported guarded input adapter.
    OperatorHandoff,
}
/// A refusal assigned at its owning guard branch, without classifying note text.
#[derive(Debug, Clone, Copy)]
pub struct ReadinessRefusal {
    /// Bounded machine-readable explanation, not input authority.
    pub cause: WakeCause,
    /// Existing human diagnostic for this guard branch.
    pub note: &'static str,
}
fn not_ready(cause: WakeCause, note: &'static str) -> Readiness {
    Readiness::NotReady(ReadinessRefusal { cause, note })
}
/// A literal paste consumes readiness; Enter is an explicit caller choice.
#[derive(Clone, Copy)]
pub struct LiteralInput<'a> {
    /// Stable caller delivery identity, journaled before the terminal writer is called.
    pub id: Ulid,
    /// Literal caller text, never interpreted as a shell command by the input encoder.
    pub text: &'a str,
    /// Whether to append the explicit Enter transition after literal input.
    pub submit: bool,
}
/// Validate positive composer proof, foreground identity, input ownership and current transition state.
pub fn check(session: &Session, view: &ArbiterView, now_ms: u64) -> Readiness {
    if !matches!(session.record.kind.as_str(), "claude" | "codex" | "zsh") {
        return Readiness::OperatorHandoff;
    }
    if view.binding != session.binding {
        return not_ready(
            WakeCause::InputBindingChanged,
            "process/session/attachment changed",
        );
    }
    if session.dialog || session.status.state == State::Waiting {
        return not_ready(WakeCause::DialogPending, "pending dialog");
    }
    if session.status.turn_open
        || session.status.state != State::Idle
        || !session.status.subagents.is_empty()
    {
        return not_ready(WakeCause::TurnActive, "turn or subagent is active");
    }
    let Some(proof) = &session.proof else {
        return not_ready(
            WakeCause::ComposerReadinessMissing,
            "no positive composer-ready indication",
        );
    };
    if proof.binding != view.binding || now_ms.saturating_sub(proof.observed_ms) > 2000 {
        return not_ready(
            WakeCause::ComposerReadinessExpired,
            "readiness indication expired or replaced",
        );
    }
    if view.operator_draft || view.active_tool || view.sync_pending || view.screen_switch_pending {
        return not_ready(
            WakeCause::InputOwned,
            "draft/tool/render transition owns input",
        );
    }
    if view.foreground_pid != Some(session.binding.pid) {
        return not_ready(WakeCause::ForegroundChanged, "foreground process changed");
    }
    if session.record.kind == "zsh" {
        if !view.canonical || !view.echo {
            return not_ready(
                WakeCause::ShellModeInvalid,
                "shell not in canonical echo mode",
            );
        }
    } else if now_ms.saturating_sub(session.last_output_ms) < 500 {
        return not_ready(WakeCause::RecentOutput, "recent output");
    }
    Readiness::Ready(ReadyToken {
        binding: view.binding.clone(),
        proof_revision: proof.revision,
        operator_epoch: view.operator_epoch,
        sequence: proof.sequence,
        expires_ms: proof.observed_ms.saturating_add(2000),
    })
}
impl Session {
    /// The caller journals the stable ID before sending these bytes through the
    /// sole supervisor writer lease. A failed/partial write is never replayed.
    ///
    /// # Errors
    /// Returns stale/consumed/ambiguous readiness, unsafe control text or input-bound failures.
    pub fn prepare_input(
        &mut self,
        token: ReadyToken,
        view: &ArbiterView,
        now_ms: u64,
        id: Ulid,
        text: &str,
    ) -> Result<Vec<u8>> {
        self.prepare_literal(
            token,
            view,
            now_ms,
            LiteralInput {
                id,
                text,
                submit: true,
            },
        )
    }
    /// Recheck and consume a positive token before encoding literal text.
    ///
    /// # Errors
    /// Refuses stale/consumed readiness, unresolved native writes, disallowed controls,
    /// unbracketed multiline/tab input or an encoded paste above 64 KiB.
    pub fn prepare_literal(
        &mut self,
        token: ReadyToken,
        view: &ArbiterView,
        now_ms: u64,
        input: LiteralInput<'_>,
    ) -> Result<Vec<u8>> {
        let LiteralInput { id, text, submit } = input;
        if token.binding != view.binding
            || token.operator_epoch != view.operator_epoch
            || token.expires_ms < now_ms
            || self.uncertain.contains(&id)
            || self
                .proof
                .as_ref()
                .is_none_or(|p| p.revision != token.proof_revision || p.sequence != token.sequence)
            || !matches!(check(self, view, now_ms), Readiness::Ready(_))
        {
            return Err(invalid("stale, consumed or ambiguous input readiness"));
        }
        if text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
        {
            return Err(invalid("message contains terminal control bytes"));
        }
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        if !view.bracketed_paste && text.contains(['\n', '\t']) {
            return Err(invalid(
                "multiline/tab input without bracketed paste needs operator handoff",
            ));
        }
        let mut bytes = if view.bracketed_paste {
            format!("\x1b[200~{text}\x1b[201~").into_bytes()
        } else {
            text.into_bytes()
        };
        if submit {
            bytes.push(b'\r');
        }
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(invalid("input exceeds supervisor bound"));
        }
        self.invalidate();
        Ok(bytes)
    }
    /// Retain ambiguous native outcomes and invalidate terminal readiness for that delivery identity.
    pub fn record_outcome(&mut self, id: Ulid, outcome: &WakeOutcome) {
        if outcome.evidence == Evidence::NativeUnknown {
            self.uncertain.insert(id);
            self.invalidate();
        }
    }
    /// Return whether this stable delivery identity has no retained ambiguous native mutation.
    pub fn can_attempt_native(&self, id: Ulid) -> bool {
        !self.uncertain.contains(&id)
    }
}
