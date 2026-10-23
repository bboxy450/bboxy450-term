//! Readiness is a short-lived, single-use indication bound to the live process,
//! native session, attachment and operator-input epoch. State alone is not one.
use super::{Binding, Evidence, Session, State, WakeOutcome};
use crate::{
    Ulid,
    store::{Result, invalid},
};

#[derive(Debug, Clone)]
pub struct ArbiterView {
    pub binding: Binding,
    pub operator_epoch: u64,
    pub foreground_pid: Option<u32>,
    pub operator_draft: bool,
    pub active_tool: bool,
    pub sync_pending: bool,
    pub screen_switch_pending: bool,
    pub canonical: bool,
    pub echo: bool,
    pub bracketed_paste: bool,
}
#[derive(Debug)]
pub struct ReadyToken {
    binding: Binding,
    proof_revision: u64,
    operator_epoch: u64,
    sequence: u64,
    expires_ms: u64,
}
#[derive(Debug)]
pub enum Readiness {
    Ready(ReadyToken),
    NotReady(&'static str),
    OperatorHandoff,
}
/// A literal paste consumes readiness; Enter is an explicit caller choice.
#[derive(Clone, Copy)]
pub struct LiteralInput<'a> {
    pub id: Ulid,
    pub text: &'a str,
    pub submit: bool,
}
pub fn check(session: &Session, view: &ArbiterView, now_ms: u64) -> Readiness {
    if !matches!(session.record.kind.as_str(), "claude" | "codex" | "zsh") {
        return Readiness::OperatorHandoff;
    }
    if view.binding != session.binding {
        return Readiness::NotReady("process/session/attachment changed");
    }
    if session.dialog || session.status.state == State::Waiting {
        return Readiness::NotReady("pending dialog");
    }
    if session.status.turn_open
        || session.status.state != State::Idle
        || !session.status.subagents.is_empty()
    {
        return Readiness::NotReady("turn or subagent is active");
    }
    let Some(proof) = &session.proof else {
        return Readiness::NotReady("no positive composer-ready indication");
    };
    if proof.binding != view.binding || now_ms.saturating_sub(proof.observed_ms) > 2000 {
        return Readiness::NotReady("readiness indication expired or replaced");
    }
    if view.operator_draft || view.active_tool || view.sync_pending || view.screen_switch_pending {
        return Readiness::NotReady("draft/tool/render transition owns input");
    }
    if view.foreground_pid != Some(session.binding.pid) {
        return Readiness::NotReady("foreground process changed");
    }
    if session.record.kind == "zsh" {
        if !view.canonical || !view.echo {
            return Readiness::NotReady("shell not in canonical echo mode");
        }
    } else if now_ms.saturating_sub(session.last_output_ms) < 500 {
        return Readiness::NotReady("recent output");
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
    pub fn submit_pty(
        &mut self,
        view: &ArbiterView,
        now_ms: u64,
        id: Ulid,
        text: &str,
        journal: &dyn crate::store::Store,
        write: impl FnOnce(Vec<u8>) -> Result<crate::InputReceipt>,
    ) -> Result<WakeOutcome> {
        self.submit_literal(
            view,
            now_ms,
            LiteralInput {
                id,
                text,
                submit: true,
            },
            journal,
            write,
        )
    }
    /// Journal before a guarded literal paste, optionally submitting Enter.
    pub fn submit_literal(
        &mut self,
        view: &ArbiterView,
        now_ms: u64,
        input: LiteralInput<'_>,
        journal: &dyn crate::store::Store,
        write: impl FnOnce(Vec<u8>) -> Result<crate::InputReceipt>,
    ) -> Result<WakeOutcome> {
        let id = input.id;
        let key = format!("adapter_input/{id}");
        if let Some(bytes) = journal.get(&key)? {
            let outcome: WakeOutcome = serde_json::from_slice(&bytes)?;
            return Ok(outcome);
        }
        let token = match check(self, view, now_ms) {
            Readiness::Ready(token) => token,
            Readiness::NotReady(reason) => {
                return Ok(WakeOutcome {
                    evidence: Evidence::RecipientStored,
                    native_id: None,
                    note: reason.into(),
                });
            }
            Readiness::OperatorHandoff => {
                return Ok(WakeOutcome {
                    evidence: Evidence::RecipientStored,
                    native_id: None,
                    note: "operator handoff required".into(),
                });
            }
        };
        let bytes = self.prepare_literal(token, view, now_ms, input)?;
        let pending = WakeOutcome::unknown(
            "PTY submission journaled; query the original supervisor input ID",
        );
        // Persist before any write. A crash here is unknown, never a second Enter.
        journal.put(&key, &serde_json::to_vec(&pending)?)?;
        let result = match write(bytes) {
            Ok(receipt)
                if receipt.input_id == id && receipt.state == crate::InputState::PtyWritten =>
            {
                WakeOutcome {
                    evidence: Evidence::NativeWritten,
                    native_id: Some(id.to_string()),
                    note: "fully PTY-written; consumption unverified".into(),
                }
            }
            Err(error)
                if error.downcast_ref::<crate::Failure>().is_some_and(|f| {
                    matches!(
                        f.code,
                        crate::FailureCode::Busy
                            | crate::FailureCode::StaleLease
                            | crate::FailureCode::StaleWriter
                            | crate::FailureCode::StaleGeneration
                            | crate::FailureCode::Invalid
                            | crate::FailureCode::Limit
                            | crate::FailureCode::Exited
                            | crate::FailureCode::Unauthorized
                            | crate::FailureCode::NotFound
                    )
                }) =>
            {
                WakeOutcome {
                    evidence: Evidence::RecipientStored,
                    native_id: None,
                    note: format!("supervisor refused before mutation: {error}"),
                }
            }
            _ => pending,
        };
        self.record_outcome(id, &result);
        if journal.put(&key, &serde_json::to_vec(&result)?).is_err() {
            return Ok(WakeOutcome::unknown(
                "PTY result could not be journaled; query original input ID",
            ));
        }
        Ok(result)
    }
    /// The caller journals the stable ID before sending these bytes through the
    /// sole supervisor writer lease. A failed/partial write is never replayed.
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
        if bytes.len() > crate::protocol::MAX_INPUT_BYTES {
            return Err(invalid("input exceeds supervisor bound"));
        }
        self.invalidate();
        Ok(bytes)
    }
    pub fn record_outcome(&mut self, id: Ulid, outcome: &WakeOutcome) {
        if outcome.evidence == Evidence::NativeUnknown {
            self.uncertain.insert(id);
            self.invalidate();
        }
    }
    pub fn can_attempt_native(&self, id: Ulid) -> bool {
        !self.uncertain.contains(&id)
    }
}
