// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(missing_docs)]

bbt_yarn::protocol! {
    mod UnknownPredicate {
        state { value: bool = false; }
        events { Ping; }
        on Ping { case current when true => { preserve value; } }
        #[predicate] fn ready() { value }
        #[property(id = "LIVE-001", leads_to(ready, missing))];
    }
}

fn main() {}
