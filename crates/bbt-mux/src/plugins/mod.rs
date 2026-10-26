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
    ConformanceJob, ConformanceResult, InspectionJob, InspectionResult, Registry, RegistryConfig,
};

use bbt_plugin::Refusal;

fn failure(code: &str, error: impl std::fmt::Display) -> Refusal {
    Refusal::new(code, error.to_string())
}
