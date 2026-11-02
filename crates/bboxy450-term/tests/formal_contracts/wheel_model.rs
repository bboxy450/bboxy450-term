// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Atomic wheel ownership, independent of physical device timing and PTY delivery.
use super::*;

pub(super) fn wheel_ownership(broken: bool) -> Model {
    let mut m = model(
        "WheelOwnership",
        broken,
        &[
            ("mouse", 1, 0),
            ("alternate", 1, 0),
            ("shift", 1, 0),
            ("historical", 1, 0),
            ("overlay", 1, 0),
            ("current_screen", 1, 1),
            ("owner", 4, 0),
            ("bad", 1, 0),
        ],
    );
    // Owners: 0 no event, 1 local history, 2 program, 3 covering surface,
    // 4 explicit stale-screen refusal. Toggle actions include A-B-A changes.
    for (i, name) in [
        "Mouse",
        "Alternate",
        "Shift",
        "Historical",
        "Overlay",
        "ScreenIdentity",
    ]
    .iter()
    .enumerate()
    {
        action(
            &mut m,
            &format!("Toggle{name}"),
            Bool(true),
            &[(i, Int(1).minus(Var(i)))],
        );
    }
    for mouse in 0..=1 {
        for alternate in 0..=1 {
            for shift in 0..=1 {
                for historical in 0..=1 {
                    for overlay in 0..=1 {
                        for current in 0..=1 {
                            let program =
                                (mouse == 1 || alternate == 1) && shift == 0 && historical == 0;
                            let expected = if overlay == 1 {
                                3
                            } else if current == 0 {
                                4
                            } else if program {
                                2
                            } else {
                                1
                            };
                            let observed = if broken && expected == 2 { 1 } else { expected };
                            action(
                                &mut m,
                                &format!(
                                    "Wheel{mouse}{alternate}{shift}{historical}{overlay}{current}"
                                ),
                                all([
                                    eq(0, mouse),
                                    eq(1, alternate),
                                    eq(2, shift),
                                    eq(3, historical),
                                    eq(4, overlay),
                                    eq(5, current),
                                ]),
                                &[
                                    (6, Int(observed)),
                                    (7, Int(i64::from(observed != expected))),
                                ],
                            );
                        }
                    }
                }
            }
        }
    }
    no_bad(&mut m, 7);
    m
}
