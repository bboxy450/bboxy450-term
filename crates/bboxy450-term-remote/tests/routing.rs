// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxy450_term_remote::routing_model::{
    BrokenRemoteRoutingCleanup, BrokenRemoteRoutingOutput, RemoteRouting,
};

#[test]
fn routing_model_and_independent_broken_controls() {
    let correct = RemoteRouting::model().check(100_000).unwrap();
    assert!(correct.violations.is_empty(), "{:?}", correct.violations);
    assert!(correct.states.len() > 10, "exercise interleavings");
    for model in [
        BrokenRemoteRoutingOutput::model(),
        BrokenRemoteRoutingCleanup::model(),
    ] {
        let broken = model.check(100_000).unwrap();
        assert!(!broken.violations.is_empty(), "{} escaped", model.name);
    }
}
