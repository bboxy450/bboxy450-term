// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(missing_docs)]

bbt_yarn::protocol! {
    mod Hygiene {
        domain Default { A, B }
        domain u32 { A }
        domain i64 { A }
        domain usize { A }
        state { value: Default = Default::A; number: u32 = u32::A; }
        events { Move(__yarn_state: Default); }
        on Move(__yarn_state) {
            case all when true => { set value = __yarn_state; preserve number; }
        }
    }
}
fn main() {
    let state = Hygiene::State::default();
    let Hygiene::Decision::Accepted { next, effects } =
        Hygiene::step(&state, Hygiene::Event::Move(Hygiene::Default::B));
    assert_eq!(next.value(), Hygiene::Default::B);
    assert!(effects.is_empty());
}
