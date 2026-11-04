// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Generation-bound hook overruns. One bounded account record per candidate;
//! restart preserves a skip, while re-enable or re-trust advances its generation.
use super::{Registry, activation_key, failure, load, put};
use bbt_plugin::{
    Refusal,
    registry::{Activation, HookWarning},
};
use rusqlite::TransactionBehavior;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
struct BudgetRecord {
    generation: u64,
    warnings: Vec<HookWarning>,
}

fn key(candidate: &str) -> String {
    format!("plugins/hook-budget/{candidate}")
}

impl Registry {
    pub(super) fn hook_warnings(
        &self,
        candidate: &str,
        generation: u64,
    ) -> Result<Vec<HookWarning>, Refusal> {
        let db = self
            .account
            .connection()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let record: Option<BudgetRecord> = load(&db, &key(candidate))?;
        Ok(record
            .filter(|r| r.generation == generation)
            .map(|r| {
                r.warnings
                    .into_iter()
                    .filter(|w| w.consecutive_overruns > 0)
                    .collect()
            })
            .unwrap_or_default())
    }

    /// A late completion can neither warn nor skip a replacement activation.
    /// Returns true exactly once, on the transition into the latched skip.
    pub(super) fn record_hook_budget(
        &self,
        candidate: &str,
        generation: u64,
        event: &str,
        budget_ms: u64,
        elapsed_ms: f64,
    ) -> Result<bool, Refusal> {
        let mut db = self
            .account
            .connection()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let activation: Option<Activation> = load(&tx, &activation_key(candidate))?;
        if !activation
            .is_some_and(|a| a.generation == generation && a.desired_enabled && !a.needs_retrust)
        {
            return Ok(false);
        }
        let mut record: BudgetRecord = load(&tx, &key(candidate))?.unwrap_or_default();
        if record.generation != generation {
            record = BudgetRecord {
                generation,
                ..Default::default()
            };
        }
        let index = match record.warnings.iter().position(|w| w.event == event) {
            Some(index) => index,
            None => {
                // Shape validation bounds the subscriptions; guard persisted data too.
                if record.warnings.len() >= 64 {
                    return Err(failure(
                        "hook_budget_capacity",
                        "hook budget record is full",
                    ));
                }
                record.warnings.push(HookWarning {
                    event: event.into(),
                    budget_ms,
                    elapsed_ms: 0.,
                    consecutive_overruns: 0,
                    skipped: false,
                });
                record.warnings.len() - 1
            }
        };
        let warning = &mut record.warnings[index];
        if warning.skipped {
            return Ok(false); // an already-running healthy completion cannot clear a skip
        }
        warning.budget_ms = budget_ms;
        warning.elapsed_ms = elapsed_ms;
        warning.consecutive_overruns = if elapsed_ms > budget_ms as f64 {
            warning.consecutive_overruns.saturating_add(1).min(3)
        } else {
            0
        };
        warning.skipped = warning.consecutive_overruns == 3;
        let newly_skipped = warning.skipped;
        put(&tx, &key(candidate), &record)?;
        tx.commit()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        Ok(newly_skipped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        plugins::{DiscoveryRoots, RegistryConfig},
        store::sqlite::SqliteStore,
    };
    use serde_json::json;

    fn activate(registry: &Registry, generation: u64) {
        let activation: Activation = serde_json::from_value(json!({
            "candidate":"fixture", "source":"/private/fixture", "scope":"repository",
            "name":{"system":"repo_graph","provider":"fixture"}, "sha256":"fixture",
            "generation":generation,"desired_enabled":true,"needs_retrust":false,
            "inspection":null
        }))
        .unwrap();
        let db = registry.account.connection().unwrap();
        put(&db, &activation_key("fixture"), &activation).unwrap();
    }

    #[test]
    fn overrun_warning_persists_latches_once_and_late_completions_cannot_reset_it() {
        let temp = tempfile::tempdir().unwrap();
        let open = || {
            Registry::with_config(
                SqliteStore::open(&temp.path().join("account")).unwrap(),
                DiscoveryRoots::new(&temp.path().join("app"), &temp.path().join("repo")),
                RegistryConfig::default(),
            )
            .unwrap()
        };
        let registry = open();
        activate(&registry, 1);
        let record = |generation, ms| {
            registry
                .record_hook_budget("fixture", generation, "SessionStart (After)", 100, ms)
                .unwrap()
        };
        assert!(!record(1, 101.));
        assert_eq!(
            registry.hook_warnings("fixture", 1).unwrap()[0].consecutive_overruns,
            1
        );
        assert!(!record(1, 90.));
        assert!(registry.hook_warnings("fixture", 1).unwrap().is_empty());
        assert!(!record(1, 101.));
        assert!(!record(1, 101.));
        assert!(record(1, 101.));
        assert!(!record(1, 101.), "skip audit must be emitted only once");
        assert!(
            !record(1, 1.),
            "a healthy in-flight completion cannot clear a skip"
        );
        assert!(
            open().hook_warnings("fixture", 1).unwrap()[0].skipped,
            "restart retains skip"
        );
        activate(&registry, 2);
        assert!(registry.hook_warnings("fixture", 2).unwrap().is_empty());
        assert!(!record(1, 101.), "late old-generation overrun is discarded");
        assert!(registry.hook_warnings("fixture", 2).unwrap().is_empty());
        assert!(!record(2, 101.));
        assert!(!record(1, 101.));
        assert_eq!(
            registry.hook_warnings("fixture", 2).unwrap()[0].consecutive_overruns,
            1,
            "a late completion erased the replacement counter"
        );
        activate(&registry, 3); // A -> B -> A still has a new activation generation.
        assert!(!record(3, 101.));
        assert!(!record(1, 101.));
        assert_eq!(
            registry.hook_warnings("fixture", 3).unwrap()[0].consecutive_overruns,
            1
        );
    }
}
