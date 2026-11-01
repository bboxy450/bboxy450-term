// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Persistent hook-budget contract, paired with its stale-completion control.
use super::*;

// Hook timing is abstracted to healthy/overrun completion. The registry owns
// count and skip atomically; delivery captures the activation generation.
pub(super) fn plugin_hook_budget(b: bool) -> Model {
    let mut m = model(
        "PluginHookBudget",
        b,
        &[
            ("generation", 3, 1),
            ("count", 3, 0),
            ("skipped", 1, 0),
            ("pending", 1, 0),
            ("captured", 3, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "Start",
        all([eq(3, 0), eq(2, 0)]),
        &[(3, Int(1)), (4, Var(0))],
    );
    action(&mut m, "Restart", eq(3, 0), &[]); // persistent budget state survives
    action(
        &mut m,
        "ReenableOrRepin",
        Var(0).lt(Int(3)),
        &[(0, Var(0).plus(Int(1))), (1, Int(0)), (2, Int(0))],
    );
    let current = all([eq(3, 1), Var(4).eq(Var(0)), eq(2, 0)]);
    action(
        &mut m,
        "Healthy",
        current.clone(),
        &[(1, Int(0)), (3, Int(0))],
    );
    action(
        &mut m,
        "Overrun",
        all([current.clone(), Var(1).lt(Int(2))]),
        &[(1, Var(1).plus(Int(1))), (3, Int(0))],
    );
    action(
        &mut m,
        "ThirdOverrun",
        all([current, eq(1, 2)]),
        &[(1, Int(3)), (2, Int(1)), (3, Int(0))],
    );
    action(
        &mut m,
        "DiscardLateCompletion",
        all([eq(3, 1), Var(4).eq(Var(0)).negate()]),
        &[(3, Int(0))],
    );
    if b {
        action(
            &mut m,
            "UnfencedCompletion",
            all([eq(3, 1), Var(4).eq(Var(0)).negate()]),
            &[(3, Int(0)), (5, Int(1))],
        );
        action(&mut m, "RunSkippedHook", eq(2, 1), &[(5, Int(1))]);
    }
    safe(&mut m, "NoStaleOrSkippedDelivery", eq(5, 0));
    safe(&mut m, "SkipAfterExactlyThree", eq(2, 1).eq(eq(1, 3)));
    m
}
