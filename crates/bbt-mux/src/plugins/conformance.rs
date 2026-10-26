//! Explicit operator checks of a trusted pin, executed only in disposable state.
use super::*;
use crate::plugins::{check, trust::FixtureAccount};

pub(super) fn key(candidate: &str) -> String {
    format!("plugins/conformance/{candidate}")
}

pub struct ConformanceJob {
    registry: Registry,
    activation: Activation,
    record: TrustRecord,
    fixture: FixtureAccount,
}

pub struct ConformanceResult {
    activation: Activation,
    record: TrustRecord,
    report: check::CheckReport,
}

impl ConformanceJob {
    pub async fn run(mut self) -> ConformanceResult {
        let report = check::prepared_with_deadline(
            &mut self.fixture,
            self.registry.config.conformance_timeout,
        )
        .await;
        ConformanceResult {
            activation: self.activation,
            record: self.record,
            report,
        }
    }
}

impl ConformanceResult {
    /// These direct children have been reaped, even when a check refused.
    pub fn owned_pids(&self) -> &[u32] {
        &self.report.owned_pids
    }
}

impl Registry {
    /// This grants nothing: trust and a validated descriptor must already exist.
    pub fn conformance(
        &self,
        actor: &Actor,
        candidate: &str,
    ) -> Result<Option<ConformanceJob>, Refusal> {
        bbt_rt::assert_not_ui_thread();
        operator(actor)?;
        let entry = self.entry(candidate)?;
        let (activation, record) = self.authority(&entry)?;
        let copy = self.verify_copy(record)?;
        if record.shape.is_none() {
            return Err(failure(
                "shape_not_cached",
                "trust and inspect the descriptor before checking",
            ));
        }
        if entry.conformance.as_ref().is_some_and(|result| result.ok) {
            return Ok(None);
        }
        let fixture = FixtureAccount::prepare_verified(record, &copy)?;
        Ok(Some(ConformanceJob {
            registry: self.clone(),
            activation: activation.clone(),
            record: record.clone(),
            fixture,
        }))
    }

    /// A result is accepted only against the exact current activation and trust.
    pub fn finish_conformance(&self, result: ConformanceResult) -> Result<Entry, Refusal> {
        bbt_rt::assert_not_ui_thread();
        let entry = self.entry(&result.activation.candidate)?;
        let (activation, record) = self.authority(&entry)?;
        if activation != &result.activation || record != &result.record {
            return Err(failure(
                "stale_plugin_completion",
                "plugin authority changed during conformance",
            ));
        }
        self.verify_copy(record)?;
        let mut db = self
            .account
            .connection()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let current: Activation = load(&tx, &activation_key(&activation.candidate))?
            .ok_or_else(|| failure("untrusted_plugin", "activation disappeared"))?;
        let trust: TrustRecord = load(&tx, &trust_key(&record.sha256))?
            .ok_or_else(|| failure("untrusted_plugin", "trust disappeared"))?;
        if current != result.activation || trust != result.record {
            return Err(failure(
                "stale_plugin_completion",
                "account changed during conformance",
            ));
        }
        let report = result.report;
        let observation = Conformance {
            sha256: activation.sha256.clone(),
            ok: report.ok,
            checked_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| failure("invalid_clock", e))?
                .as_secs(),
            checks: report.checks,
            refusal: report.refusal.map(|mut reason| {
                reason.message = reason.message.chars().take(2048).collect();
                reason
            }),
        };
        put(&tx, &key(&activation.candidate), &observation)?;
        tx.commit()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        self.entry(&activation.candidate)
    }
}
