// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded capture under one disk-full episode; filesystem framing has native witnesses.
use super::*;
fn capture(broken: bool) -> Model {
    let mut m = model(
        "CaptureStoragePressure",
        broken,
        &[
            ("space_phase", 2, 0),
            ("accepted", 2, 0),
            ("committed", 2, 0),
            ("paused", 1, 0),
            ("alive", 1, 1),
            ("controls", 1, 0),
            ("committed_at_full", 2, 0),
        ],
    );
    action(
        &mut m,
        "Admit",
        all([eq(4, 1), eq(3, 0), Var(1).lt(Int(2))]),
        &[(1, Var(1).plus(Int(1)))],
    );
    action(
        &mut m,
        "DiskFull",
        eq(0, 0),
        &[(0, Int(1)), (6, Var(2)), (5, Int(0))],
    );
    action(
        &mut m,
        "WriteFull",
        all([eq(0, 1), eq(4, 1), Var(2).lt(Var(1))]),
        &[(3, Int(1)), (4, if broken { Int(0) } else { Var(4) })],
    );
    action(&mut m, "SpaceRestored", eq(0, 1), &[(0, Int(2))]);
    action(
        &mut m,
        "Commit",
        all([eq(4, 1), ne(0, 1), Var(2).lt(Var(1))]),
        &[(2, Var(1)), (3, Int(0))],
    );
    action(&mut m, "Control", all([eq(4, 1), eq(5, 0)]), &[(5, Int(1))]);
    safe(&mut m, "BackendSurvivesDiskFull", eq(4, 1));
    safe(&mut m, "NoUnwrittenCommit", Var(1).lt(Var(2)).negate());
    safe(
        &mut m,
        "NoCommitWhileFull",
        implies(eq(0, 1), Var(2).eq(Var(6))),
    );
    if !broken {
        m.responses.push(Response {
            name: "RecoveryEventuallyCommits".into(),
            request: all([eq(0, 2), Var(2).lt(Var(1))]),
            response: Var(2).eq(Var(1)),
        });
        m.responses.push(Response {
            name: "ControlRemainsResponsive".into(),
            request: eq(5, 0),
            response: eq(5, 1),
        });
    }
    m.weak_fair.extend([vec![4], vec![5]]);
    m
}
pub fn cases() -> Vec<(Model, &'static str)> {
    vec![(capture(false), "pass"), (capture(true), "safety")]
}
#[test]
fn disk_pressure_keeps_the_owner_alive_and_recovers_without_false_durability() {
    for (model, expected) in cases() {
        assert_eq!(
            model.check(10_000).unwrap().violations.is_empty(),
            expected == "pass"
        );
    }
}
