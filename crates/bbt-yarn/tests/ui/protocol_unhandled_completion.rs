// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(missing_docs)]

bbt_yarn::protocol! {
    mod Unhandled {
        state { value: bool = false; }
        events { Begin; }
        effects { Fetch completes { Succeeded, UnknownOutcome }; }
        on Begin { case start when true => { preserve value; emit Fetch; } }
        on FetchAdmitted { case admit when true => { preserve value; } }
        on FetchSucceeded { case success when true => { set value = true; } }
    }
}

fn main() {}
