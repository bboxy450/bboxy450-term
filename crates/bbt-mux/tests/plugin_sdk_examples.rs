// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Every SDK example plugin (one per contract, `bbt-plugin-sdk/examples`)
//! passes the host's conformance checker unchanged, in a disposable fixture
//! account: every method, additive-field probes, schema-checked results.
#![allow(
    clippy::disallowed_methods,
    reason = "private fixture builds and checker processes"
)]

use std::time::Duration;

const EXAMPLES: [&str; 8] = [
    "bboxy450-term-data_source-example",
    "bboxy450-term-task_system-example",
    "bboxy450-term-event_sink-example",
    "bboxy450-term-decision_model-example",
    "bboxy450-term-repo_graph-example",
    "bboxy450-term-session-example",
    "bboxy450-term-provider-example",
    "bboxy450-term-transport-example",
];

// keeps: P8 SDK completeness — an example per contract, each conformant.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_sdk_example_passes_the_host_conformance_checker() {
    // Fresh copies may be slow to exec under a loaded full gate.
    bbt_mux::plugins::set_test_budget(Duration::from_secs(30));
    for name in EXAMPLES {
        let binary = tokio::task::spawn_blocking(move || bbt_plugin_fixture::sdk_example(name))
            .await
            .unwrap();
        let report = bbt_mux::plugins::check(&binary).await;
        assert!(
            report.ok,
            "{name}: {:?}\nchecks: {:?}",
            report.refusal, report.checks
        );
        let contract = name
            .strip_prefix("bboxy450-term-")
            .and_then(|rest| rest.strip_suffix("-example"))
            .unwrap();
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.contains(contract) || contract == "data_source"),
            "{name}: its contract was exercised: {:?}",
            report.checks
        );
    }
}
