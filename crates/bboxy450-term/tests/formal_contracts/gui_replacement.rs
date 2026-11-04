// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One replacement, three GUI births, old/new installed bytes and one agent owner.
//! Install/quit/exit/spawn/ready/rollback are atomic IO-owner actions. Fair progress
//! retires an admitted replacement; OS scheduling, PID reuse and bytes are native witnesses.
use super::{action, all, eq, implies, model, safe};
use bboxy450_tlc::{
    Expr::{Bool, Int, Var},
    Model, Response,
};
fn replacement(control: u8) -> Model {
    let mut m = model(
        "GuiReplacement",
        false,
        &[
            ("phase", 8, 0),
            ("gui", 3, 1),
            ("installed", 2, 1),
            ("backup", 1, 0),
            ("agents_alive", 1, 1),
            ("birth", 2, 0),
            ("captured", 2, 0),
            ("actual_source", 2, 1),
        ],
    );
    m.name = match control {
        1 => "BrokenGuiReplacementOwners",
        2 => "BrokenGuiReplacementDuplicate",
        3 => "BrokenGuiReplacementSource",
        4 => "BrokenGuiReplacementRollback",
        _ => "GuiReplacement",
    }
    .into();
    action(
        &mut m,
        "Prepare",
        eq(0, 0),
        &[(0, Int(1)), (2, Int(2)), (3, Int(1)), (6, Var(5))],
    );
    action(
        &mut m,
        "ConcurrentGuiBirth",
        all([eq(0, 1), Var(5).lt(Int(2))]),
        &[(5, Var(5).plus(Int(1)))],
    );
    action(
        &mut m,
        "RequestQuit",
        all([eq(0, 1), Var(5).eq(Var(6))]),
        &[(0, Int(2)), (4, Int(if control == 1 { 0 } else { 1 }))],
    );
    action(
        &mut m,
        "RefuseStaleOrTimeout",
        eq(0, 1),
        &[(0, Int(6)), (2, Int(1))],
    );
    action(&mut m, "OldExit", eq(0, 2), &[(0, Int(3)), (1, Int(0))]);
    action(&mut m, "QuitTimeout", eq(0, 2), &[(0, Int(6)), (2, Int(1))]);
    action(
        &mut m,
        "StartNew",
        if control == 2 {
            eq(0, 2).or(eq(0, 3))
        } else {
            eq(0, 3)
        },
        &[
            (0, Int(4)),
            (
                1,
                if control == 2 {
                    Var(1).plus(Int(2))
                } else {
                    Int(2)
                },
            ),
            (7, Int(if control == 3 { 1 } else { 2 })),
        ],
    );
    action(
        &mut m,
        "NewFrame",
        all([eq(0, 4), if control == 3 { Bool(true) } else { eq(7, 2) }]),
        &[(0, Int(5))],
    );
    action(
        &mut m,
        "StartupFailure",
        eq(0, 4),
        &[(0, Int(7)), (1, Int(0))],
    );
    action(
        &mut m,
        "Rollback",
        eq(0, 7),
        &[
            (0, Int(8)),
            (1, Int(1)),
            (2, Int(if control == 4 { 2 } else { 1 })),
            (7, Int(1)),
        ],
    );
    safe(&mut m, "AgentsNeverStopped", eq(4, 1));
    safe(&mut m, "OneGuiOwner", Var(1).le(Int(2)));
    safe(
        &mut m,
        "ReadyNamesActualSource",
        implies(eq(0, 5), all([eq(1, 2), eq(7, 2), eq(2, 2)])),
    );
    safe(
        &mut m,
        "FailurePreservesOldBuild",
        implies(eq(0, 8), all([eq(1, 1), eq(2, 1), eq(3, 1)])),
    );
    m.responses.push(Response {
        name: "ReplacementRetires".into(),
        request: all([Int(0).lt(Var(0)), Var(0).lt(Int(5))]),
        response: eq(0, 5).or(eq(0, 6)).or(eq(0, 8)),
    });
    m.weak_fair.push(vec![2, 3, 4, 5, 6, 7, 8, 9]);
    m
}
pub fn cases() -> Vec<(Model, &'static str)> {
    (0..=4)
        .map(|c| (replacement(c), if c == 0 { "pass" } else { "safety" }))
        .collect()
}
