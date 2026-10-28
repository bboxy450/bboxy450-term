// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Shared seat display-name data. Names do not grant roles or authority.

/// Checked Rust copy of the canonical `seat_names.json` consumed by the lab.
pub const POOL: [&str; 14] = [
    "Vesper Greywind",
    "Nyx Copperwing",
    "Lumen Foxglove",
    "Rook Ashwhisker",
    "Sable Quill",
    "Pip Hollow",
    "Marrow Finch",
    "Tamsin Voss",
    "Corvin Ember",
    "Ilka Thistle",
    "Dax Riven",
    "Wren Starling",
    "Halcyon Moss",
    "Juniper Vale",
];

#[cfg(test)]
mod tests {
    use super::*;

    // keeps: spec e2e-acceptance §1
    #[test]
    fn rust_names_match_the_shared_json_and_are_unique() {
        let source: serde_json::Value =
            serde_json::from_str(include_str!("seat_names.json")).unwrap();
        assert_eq!(source["pool"], serde_json::json!(POOL));
        assert_eq!(
            POOL.iter().collect::<std::collections::BTreeSet<_>>().len(),
            POOL.len()
        );
        assert!(POOL.iter().all(|name| !name.trim().is_empty()));
    }
}
