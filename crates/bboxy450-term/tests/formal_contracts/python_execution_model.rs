// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Preflight, real provider capture and publication can straddle A-B-A changes.
use super::*;

fn native(broken: bool) -> Model {
    let mut m = model(
        "PythonNativeContext",
        broken,
        &[
            ("phase", 3, 0),
            ("root", 1, 0),
            ("epoch", 2, 0),
            ("expected_root", 1, 0),
            ("expected_epoch", 2, 0),
            ("cross_scope_effect", 1, 0),
            ("visible", 1, 0),
            ("published_epoch", 2, 0),
        ],
    );
    action(
        &mut m,
        "Preflight",
        eq(0, 0),
        &[(0, Int(1)), (3, Var(1)), (4, Var(2))],
    );
    action(
        &mut m,
        "ChangeAToB",
        eq(2, 0),
        &[(1, Int(1)), (2, Int(1)), (6, Int(0))],
    );
    action(
        &mut m,
        "ChangeBToA",
        eq(2, 1),
        &[(1, Int(0)), (2, Int(2)), (6, Int(0))],
    );
    let current = all([Var(1).eq(Var(3)), Var(2).eq(Var(4))]);
    action(
        &mut m,
        "CaptureActualProviderAndDispatch",
        all([eq(0, 1), current.clone()]),
        &[(0, Int(2))],
    );
    if broken {
        action(
            &mut m,
            "BrokenUsePreflightAfterRetarget",
            all([eq(0, 1), current.clone().negate()]),
            &[(0, Int(2)), (5, Int(1))],
        );
    }
    action(
        &mut m,
        "PublishCurrentResult",
        all([eq(0, 2), current]),
        &[(0, Int(3)), (6, Int(1)), (7, Var(2))],
    );
    action(
        &mut m,
        "RefuseStale",
        all([eq(0, 1).or(eq(0, 2)), Var(2).eq(Var(4)).negate()]),
        &[(0, Int(3))],
    );
    safe(&mut m, "NativeEffectCannotRetarget", eq(5, 0));
    safe(
        &mut m,
        "OldResultIsNotCurrent",
        eq(6, 0).or(Var(7).eq(Var(2))),
    );
    m
}
pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![(native(false), "pass"), (native(true), "safety")]
}
