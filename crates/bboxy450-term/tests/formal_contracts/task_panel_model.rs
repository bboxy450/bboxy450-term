// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Read retry deadlines are fenced by selection/refresh generation.
use super::*;
pub(super) fn cases() -> Vec<(Model, &'static str)> {
    let mut cases: Vec<_> = [false, true]
        .into_iter()
        .map(|broken| {
            let mut m = model(
                "TaskPanelRetry",
                broken,
                &[
                    ("epoch", 2, 0),
                    ("swarm", 1, 0),
                    ("timer", 3, 3),
                    ("countdown", 2, 0),
                    ("admitted", 3, 3),
                ],
            );
            action(
                &mut m,
                "ScheduleReadRetry",
                eq(2, 3),
                &[(2, Var(0)), (3, Int(2))],
            );
            for epoch in 0..2 {
                action(
                    &mut m,
                    &format!("SelectOrRefresh{epoch}"),
                    eq(0, epoch),
                    &[(0, Int(epoch + 1)), (1, Int(1 - epoch)), (4, Int(3))],
                );
            }
            action(
                &mut m,
                "Tick",
                all([Var(2).lt(Int(3)), Int(0).lt(Var(3))]),
                &[(3, Var(3).minus(Int(1)))],
            );
            action(
                &mut m,
                "RetryRead",
                all([
                    Var(2).lt(Int(3)),
                    eq(3, 0),
                    if broken {
                        Bool(true)
                    } else {
                        Var(2).eq(Var(0))
                    },
                ]),
                &[(4, Var(2)), (2, Int(3))],
            );
            action(
                &mut m,
                "DiscardOldTimer",
                all([Var(2).lt(Int(3)), Var(2).eq(Var(0)).negate()]),
                &[(2, Int(3))],
            );
            safe(
                &mut m,
                "OnlyCurrentGenerationAdmitsRead",
                eq(4, 3).or(Var(4).eq(Var(0))),
            );
            (m, if broken { "safety" } else { "pass" })
        })
        .collect();
    for broken in [false, true] {
        // One timer owner, one generation retirement. Owner observation is fair;
        // cancellation cannot retract an event already queued before retirement.
        let mut m = model(
            "TaskRetryTimerLifetime",
            broken,
            &[("retired", 1, 0), ("owned", 1, 1)],
        );
        action(&mut m, "RetireGeneration", eq(0, 0), &[(0, Int(1))]);
        action(
            &mut m,
            "ObserveOwner",
            all([eq(0, 1), eq(1, 1)]),
            &[(1, Int(i64::from(broken)))],
        );
        m.weak_fair.push(vec![1]);
        m.responses.push(Response {
            name: "RetiredTimerEventuallyDisposed".into(),
            request: eq(0, 1),
            response: eq(1, 0),
        });
        cases.push((m, if broken { "liveness" } else { "pass" }));
    }
    cases
}
