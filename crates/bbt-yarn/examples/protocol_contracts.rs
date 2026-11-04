// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Export the actual verification declaration for the baseline guard.
#![allow(clippy::disallowed_methods)]

#[path = "../tests/protocol.rs"]
#[allow(dead_code)] // This exporter consumes one contract; the file also carries broken controls.
mod protocols;

fn main() {
    let result = protocols::latest_result_contract::check();
    std::print!("{}", result.to_json());
    assert!(
        result.is_established(),
        "{}: {}",
        result.outcome.as_str(),
        result.message
    );
}
