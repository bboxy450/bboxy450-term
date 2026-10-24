//! Mux owns the durable input journal and host-result classification.
use super::{Evidence, Session, WakeOutcome};
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
        self.inner
            .record_outcome(id, &super::mapping::native_outcome(&result)?);
        if journal.put(&key, &serde_json::to_vec(&result)?).is_err() {
            return Ok(WakeOutcome::unknown(
                "PTY result could not be journaled; query original input ID",
            ));
        }
        Ok(result)
    }
}
