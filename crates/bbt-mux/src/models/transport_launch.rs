// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Captured root/activation authority fences asynchronous transport launch.
use super::*;

pub(super) fn plugin_transport_launch(b: bool) -> Model {
    let mut m = model(
        "PluginTransportLaunch",
        b,
        &[
            ("generation", 3, 1),
            ("root_revision", 3, 1),
            ("enabled", 1, 1),
            ("admitted", 1, 0),
            ("captured_generation", 3, 0),
            ("captured_root", 3, 0),
            ("ready", 1, 0),
            ("launched", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "StartWrap",
        all([eq(3, 0), eq(2, 1)]),
        &[(3, Int(1)), (4, Var(0)), (5, Var(1))],
    );
    action(
        &mut m,
        "CompleteWrap",
        all([eq(3, 1), eq(6, 0), eq(7, 0)]),
        &[(6, Int(1))],
    );
    action(
        &mut m,
        "DisableOrRepin",
        all([eq(2, 1), Var(0).lt(Int(3))]),
        &[(0, Var(0).plus(Int(1))), (2, Int(0))],
    );
    action(
        &mut m,
        "Reenable",
        all([eq(2, 0), Var(0).lt(Int(3))]),
        &[(0, Var(0).plus(Int(1))), (2, Int(1))],
    );
    action(
        &mut m,
        "ChangeWorkspace",
        Var(1).lt(Int(3)),
        &[(1, Var(1).plus(Int(1)))],
    );
    let current = all([eq(2, 1), Var(4).eq(Var(0)), Var(5).eq(Var(1))]);
    action(
        &mut m,
        "Launch",
        all([eq(6, 1), eq(7, 0), current.clone()]),
        &[(6, Int(0)), (7, Int(1))],
    );
    action(
        &mut m,
        "RefuseStalePlan",
        all([eq(6, 1), current.clone().negate()]),
        &[(6, Int(0))],
    );
    // An admitted operation is never submitted twice, including after restart.
    action(&mut m, "QueryOrRestart", eq(8, 0).or(eq(8, 1)), &[]);
    if b {
        action(
            &mut m,
            "LaunchWithoutFence",
            all([eq(6, 1), eq(7, 0), current.negate()]),
            &[(7, Int(1)), (8, Int(1))],
        );
    }
    no_bad(&mut m, 8);
    m
}
