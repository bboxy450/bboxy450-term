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
    super::mapping::outcome(
        bboxy450_harness_wake_up::native::wake(
            binding,
            endpoint,
            id,
            text,
            priority,
            allow_interrupt,
        )
        .await,
    )
}
