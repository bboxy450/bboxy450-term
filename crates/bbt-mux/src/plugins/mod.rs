//! Logic-owned plugin discovery, account trust and isolated conformance checks.

mod check;
mod client;
pub(crate) mod control;
mod discovery;
mod registry;
mod trust;

pub use check::{CheckReport, Measurement, check, measure, run_cli};
pub use discovery::DiscoveryRoots;
pub use registry::{
    ConformanceJob, ConformanceResult, DataSourceProvider, InspectionJob, InspectionResult,
    ProviderConfig, ProviderObservation, Registry, RegistryConfig,
};

use bbt_plugin::Refusal;

/// Host-only budgets. Wire callers and plugin shapes cannot widen them.
#[derive(Debug, Clone, Copy, Default)]
pub struct HostConfig {
    pub registry: RegistryConfig,
    pub provider: ProviderConfig,
}

fn failure(code: &str, error: impl std::fmt::Display) -> Refusal {
    let message = error.to_string();
    bbt_rt::error_journal::report(bbt_rt::error_journal::Failure::new(
        "plugin", code, &message,
    ));
    Refusal::new(code, message)
}
