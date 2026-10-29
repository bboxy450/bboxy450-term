// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Mux owns the durable input journal and host-result classification.
use super::{Evidence, Session, WakeCause, WakeOutcome};
use crate::{Ulid, store::Result};
pub use bboxy450_harness_wake_up::guard::*;
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
            if self.inner.can_attempt_native(id) {
                self.inner
                    .record_outcome(id, &super::mapping::native_outcome(&outcome)?);
            }
            return Ok(outcome);
        }
        let token = match check(self, view, now_ms) {
            Readiness::Ready(token) => token,
            Readiness::NotReady(reason) => {
                return Ok(WakeOutcome {
                    evidence: Evidence::RecipientStored,
                    native_id: None,
                    note: reason.note.into(),
                    cause: Some(reason.cause),
                });
            }
            Readiness::OperatorHandoff => {
                return Ok(WakeOutcome {
                    evidence: Evidence::RecipientStored,
                    native_id: None,
                    note: "operator handoff required".into(),
                    cause: Some(WakeCause::OperatorHandoff),
                });
            }
        };
        let bytes = self.prepare_literal(token, view, now_ms, input)?;
        let pending = WakeOutcome::unknown_cause(
            WakeCause::PtySubmissionPending,
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
                    cause: Some(WakeCause::PtyWritten),
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
                    note: super::observe::short(
                        &format!("supervisor refused before mutation: {error}"),
                        640,
                    ),
                    cause: Some(WakeCause::SupervisorRefused),
                }
            }
            _ => pending,
        };
        self.inner
            .record_outcome(id, &super::mapping::native_outcome(&result)?);
        if journal.put(&key, &serde_json::to_vec(&result)?).is_err() {
            return Ok(WakeOutcome::unknown_cause(
                WakeCause::PtyResultUnjournaled,
                "PTY result could not be journaled; query original input ID",
            ));
        }
        Ok(result)
    }
}
