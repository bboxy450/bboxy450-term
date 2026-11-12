// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Export the bounded differential corpus. Java TLC remains the reference.
// Standalone development tool, outside the UI; depending on bbt-rt would invert layering.
#![allow(clippy::disallowed_methods)]
use bbt_yarn::model::corpus::export;
use std::path::Path;

#[path = "../tests/protocol_runtime.rs"]
mod protocol_language;
#[path = "../tests/protocol.rs"]
#[allow(dead_code)] // Model export includes runtime-only declarations and contract references.
mod protocols;
#[path = "../../bboxy450-term-remote/src/delivery.rs"]
mod remote_delivery;
#[path = "../../bboxy450-term-remote/src/resume_model.rs"]
mod remote_resume;
#[path = "../../bboxy450-term-remote/src/routing_model.rs"]
mod remote_routing;
#[path = "../tests/protocol_temporal.rs"]
mod temporal_protocols;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .ok_or("usage: verify OUTPUT_DIRECTORY")?;
    let root = Path::new(&output);
    let mut manifest = std::fs::read_to_string(root.join("manifest.tsv"))?;
    let reactor = bbt_yarn::reactor_model! {
        policy {selection: biased;required_phases: [];}
        #[source(stop)] #[readiness(quiescent)] #[shutdown]
        _ = sources.stop => { Ok(()) }
        #[source(input)] #[readiness(may_remain_ready)]
        #[starvation(allowed, reason="test corpus explores source scheduling, not handler termination")]
        _ = sources.input => { Ok(Control::Continue) }
        #[source(output)] #[readiness(may_remain_ready)] #[last] #[drain(max=2)]
        #[starvation(allowed, reason="background output may wait behind operator input")]
        _ = sources.output => { Ok(Control::Continue) }
    };
    let mut yield_reactor = bbt_yarn::reactor_model! {
        policy {selection: biased;required_phases: [];}
        #[source(stop)] #[readiness(quiescent)] #[shutdown]
        _ = sources.stop => { Ok(()) }
        #[source(output)] #[readiness(may_remain_ready)] #[yields_to(input,when=buffered)] #[drain(max=2)]
        _ = sources.output => { Ok(Control::Continue) }
        #[source(input)] #[readiness(may_remain_ready)] #[last] #[when(enabled)]
        _ = sources.input => { Ok(Control::Continue) }
    };
    yield_reactor.name = "YarnBufferedYield".into();
    for (model, expected) in [
        (reactor, "pass"),
        (yield_reactor, "pass"),
        (protocols::LatestResult::model(), "pass"),
        (protocols::BrokenLatestResult::model(), "safety"),
        (remote_delivery::RemoteDelivery::model(), "pass"),
        (remote_delivery::BrokenRemoteDelivery::model(), "safety"),
        (remote_resume::RemoteResume::model(), "pass"),
        (remote_resume::BrokenRemoteResume::model(), "safety"),
        (remote_routing::RemoteRouting::model(), "pass"),
        (remote_routing::BrokenRemoteRoutingOutput::model(), "safety"),
        (
            remote_routing::BrokenRemoteRoutingCleanup::model(),
            "safety",
        ),
        (protocol_language::ProtocolLanguage::model(), "pass"),
        (temporal_protocols::PresentGeneration::model(), "pass"),
        (
            temporal_protocols::BrokenPresentGeneration::model(),
            "safety",
        ),
        (temporal_protocols::PresentProgress::model(), "pass"),
        (
            temporal_protocols::BrokenPresentProgress::model(),
            "liveness",
        ),
        (temporal_protocols::RetryUnknown::model(), "pass"),
        (temporal_protocols::BrokenRetryUnknown::model(), "safety"),
        (temporal_protocols::ProtocolProgress::model(), "pass"),
        (
            temporal_protocols::BrokenProtocolProgress::model(),
            "liveness",
        ),
    ] {
        export(model, expected, root, &mut manifest)?;
    }
    std::fs::write(root.join("manifest.tsv"), manifest)?;
    Ok(())
}
