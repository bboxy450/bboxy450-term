// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Every capture fixture owns its supervisor journal outside its seat roots.
pub fn capture_config(root: &std::path::Path) -> bbt_mux::Config {
    bbt_mux::Config {
        journal_directory: Some(root.join("supervisor-journal")),
        ..Default::default()
    }
}

/// Successful leaf pages, independent of how many share one transport reply.
#[allow(dead_code)] // This common module is also used by non-replay fixtures.
pub fn ring_histories(reply: &bbt_mux::Reply) -> Vec<&bbt_mux::ring::History> {
    match reply {
        bbt_mux::Reply::RingHistory { history } => vec![history],
        bbt_mux::Reply::RingHistoryBatch { results } => results
            .iter()
            .filter_map(|row| row.result.as_deref().ok())
            .collect(),
        _ => vec![],
    }
}
