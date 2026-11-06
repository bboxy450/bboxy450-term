// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Native protocol implementation belongs to the reusable kit.
use super::{Binding, Endpoint, Priority, Ulid, WakeOutcome};
pub use bboxy450_harness_wake_up::native::*;
/// Translate native transport evidence at the app boundary.
pub async fn wake(
    binding: &Binding,
    endpoint: &Endpoint,
    id: Ulid,
    text: &str,
    priority: Priority,
    allow_interrupt: bool,
) -> WakeOutcome {
    let mut outcome = super::mapping::outcome(
        bboxy450_harness_wake_up::native::wake(
            binding,
            endpoint,
            id,
            text,
            priority,
            allow_interrupt,
        )
        .await,
    );
    // Codex turn/steer admits input without cancelling the running turn.
    if priority == Priority::Now
        && matches!(endpoint, Endpoint::Codex { .. })
        && outcome.evidence == super::Evidence::NativeQueued
    {
        outcome.evidence = super::Evidence::NativeSteered;
        outcome.note = "native turn steering admitted; model consumption unverified".into();
    }
    outcome
}
