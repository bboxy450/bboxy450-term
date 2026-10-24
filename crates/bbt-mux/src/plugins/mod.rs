//! Headless plugin conformance. Production discovery and trust are the P-1 cut.

mod check;
mod client;
mod trust;

pub use check::{CheckReport, Measurement, check, measure, run_cli};

use bbt_plugin::Refusal;

fn failure(code: &str, error: impl std::fmt::Display) -> Refusal {
    Refusal::new(code, error.to_string())
}
