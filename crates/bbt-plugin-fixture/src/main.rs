// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One fixture executable; its variant is compiled in (`BBOXY_PLUGIN_FIXTURE`).

// A plugin is its own process with no UI thread; blocking stdio is its job.
#![allow(clippy::disallowed_methods)]

use bbt_plugin_fixture::Knobs;

fn main() {
    bbt_plugin_fixture::run_fixture(Knobs::compiled(option_env!("BBOXY_PLUGIN_FIXTURE")));
}

#[cfg(test)]
mod tests {
    #[test]
    fn absent_compiled_knobs_select_the_default_reference_variant() {
        assert_eq!(super::Knobs::compiled(None), super::Knobs::default());
    }
}
