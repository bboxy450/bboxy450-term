// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Three fresh generation births, private file publication and retirement.
use super::{action, all, eq, implies, model, safe};
use bboxy450_tlc::{
    Expr::{Int, Var},
    Model,
};
fn capability(mode: u8) -> Model {
    let name = match mode {
        0 => "SeatCapabilityEnvironment",
        1 => "BrokenSeatCapabilityEnvironment",
        2 => "SeatCapabilityCleanup",
        _ => "BrokenSeatCapabilityCleanup",
    };
    let mut m = model(
        name,
        false,
        &[
            ("birth", 3, 0),
            ("phase", 2, 0),
            ("file_birth", 3, 0),
            ("old_file", 1, 0),
            ("value_in_env", 1, 0),
        ],
    );
    for birth in 1..=3 {
        action(
            &mut m,
            &format!("PublishFreshOverlay{birth}"),
            all([eq(0, birth - 1), eq(1, 0)]),
            &[
                (0, Int(birth)),
                (1, Int(1)),
                (2, Int(birth)),
                (4, Int(i64::from(mode == 1))),
            ],
        );
    }
    action(&mut m, "RequestStop", eq(1, 1), &[(1, Int(2))]);
    // The owned live phase includes a prepared overlay before plan() returns.
    action(
        &mut m,
        "AbandonFailedPlan",
        eq(1, 1),
        &[
            (1, Int(0)),
            (2, Int(0)),
            (3, Int(i64::from(mode == 3))),
            (4, Int(0)),
        ],
    );
    // Files remain usable by repeated hook/MCP starts until the process exits.
    action(
        &mut m,
        "ReadAgain",
        all([eq(1, 0).negate(), Var(0).eq(Var(2))]),
        &[],
    );
    action(
        &mut m,
        "ReapAndRemoveOverlay",
        eq(1, 2),
        &[
            (1, Int(0)),
            (2, Int(0)),
            (3, Int(i64::from(mode == 3))),
            (4, Int(0)),
        ],
    );
    if mode < 2 {
        safe(&mut m, "NoAdmissionValueInEnvironment", eq(4, 0));
    } else {
        safe(
            &mut m,
            "OverlayOwnsCapabilityFile",
            all([
                eq(3, 0),
                implies(eq(1, 0), eq(2, 0)),
                implies(eq(1, 0).negate(), Var(0).eq(Var(2))),
            ]),
        );
    }
    m
}
pub fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (capability(0), "pass"),
        (capability(1), "safety"),
        (capability(2), "pass"),
        (capability(3), "safety"),
    ]
}
