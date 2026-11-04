// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One seat, three generations; age buckets fresh / 5s / 60s. Reports,
//! heartbeat, cadence, clock, exit and rebind are atomic logic-owner actions.
//! No fairness is needed for safety; Tick is weakly fair for expiry progress.
use super::*;

fn status(broken: u8) -> Model {
    let name = match broken {
        1 => "BrokenHarnessReportedStatus",
        2 => "BrokenHarnessStatusExpiry",
        _ => "HarnessReportedStatus",
    };
    let mut m = model(
        name,
        false,
        &[
            ("generation", 2, 0),
            ("reported", 1, 0),
            ("source", 3, 0),
            ("age", 2, 0),
            ("state", 3, 0),
            ("last_state", 3, 0),
            ("captured_generation", 2, 0),
            ("pending", 1, 0),
            ("fresh_hold", 1, 0),
        ],
    );
    // source: 0=cadence/unobserved, 1=reported, 2=stale, 3=exit.
    for state in 0..=3 {
        action(
            &mut m,
            &format!("Report{state}"),
            ne(2, 3),
            &[
                (1, Int(1)),
                (2, Int(1)),
                (3, Int(0)),
                (4, Int(state)),
                (5, Int(state)),
                (8, Int(1)),
            ],
        );
    }
    action(
        &mut m,
        "Heartbeat",
        all([eq(2, 1), eq(8, 1)]),
        &[(3, Int(0))],
    );
    action(
        &mut m,
        "TickFresh",
        all([eq(2, 1), eq(3, 0), eq(4, 1).or(eq(4, 2))]),
        &[(3, Int(1))],
    );
    action(
        &mut m,
        "TickExpire",
        all([eq(2, 1), eq(3, 1), eq(4, 1).or(eq(4, 2))]),
        &[
            (3, Int(2)),
            (2, Int(if broken == 2 { 1 } else { 2 })),
            (8, Int(0)),
        ],
    );
    action(
        &mut m,
        "Cadence",
        all([
            ne(2, 3),
            if broken == 1 {
                eq(1, 0).or(Int(1).le(Var(3)))
            } else {
                eq(1, 0)
            },
        ]),
        &[(2, Int(0)), (4, Int(0))],
    );
    action(
        &mut m,
        "Exit",
        Bool(true),
        &[
            (1, Int(1)),
            (2, Int(3)),
            (3, Int(0)),
            (4, Int(0)),
            (5, Int(0)),
            (8, Int(0)),
        ],
    );
    action(
        &mut m,
        "Rebind",
        Var(0).lt(Int(2)),
        &[
            (0, Var(0).plus(Int(1))),
            (1, Int(0)),
            (2, Int(0)),
            (3, Int(0)),
            (4, Int(0)),
            (5, Int(0)),
            (8, Int(0)),
        ],
    );
    action(&mut m, "CaptureRead", eq(7, 0), &[(6, Var(0)), (7, Int(1))]);
    action(
        &mut m,
        "PublishCurrentRead",
        all([eq(7, 1), Var(6).eq(Var(0)), ne(2, 3)]),
        &[
            (7, Int(0)),
            (1, Int(1)),
            (2, Int(1)),
            (3, Int(0)),
            (4, Int(1)),
            (5, Int(1)),
            (8, Int(1)),
        ],
    );
    action(
        &mut m,
        "DiscardOldRead",
        all([eq(7, 1), Var(6).eq(Var(0)).negate()]),
        &[(7, Int(0))],
    );
    safe(
        &mut m,
        "ReportedGenerationNeverReturnsToCadence",
        implies(eq(1, 1), ne(2, 0)),
    );
    safe(
        &mut m,
        "OnlyReportsAndExitChangeReportedState",
        implies(eq(1, 1), Var(4).eq(Var(5))),
    );
    safe(
        &mut m,
        "ExpiredReportsAreLabeledStale",
        implies(all([eq(1, 1), eq(3, 2)]), eq(2, 2)),
    );
    safe(
        &mut m,
        "StaleEndsFreshnessHoldWithoutChangingActivity",
        implies(eq(2, 2), all([eq(8, 0), eq(4, 1).or(eq(4, 2))])),
    );
    safe(
        &mut m,
        "ClosedTurnsNeverBecomeStale",
        implies(eq(4, 0).or(eq(4, 3)), ne(2, 2)),
    );
    m.weak_fair.push(
        m.actions
            .iter()
            .enumerate()
            .filter_map(|(index, action)| {
                matches!(action.name.as_str(), "TickFresh" | "TickExpire").then_some(index)
            })
            .collect(),
    );
    if broken == 0 {
        m.responses.push(Response {
            name: "QuietReportsEventuallyStaleOrReplaced".into(),
            request: all([eq(2, 1), eq(3, 1), eq(4, 1).or(eq(4, 2))]),
            response: eq(3, 0).or(eq(2, 2)).or(eq(2, 3)),
        });
    }
    m
}
pub(super) fn cases() -> Vec<(Model, &'static str)> {
    let mut expiry = status(0);
    expiry.name = "HarnessStatusExpiry".into();
    vec![
        (status(0), "pass"),
        (status(1), "safety"),
        (expiry, "pass"),
        (status(2), "safety"),
    ]
}
