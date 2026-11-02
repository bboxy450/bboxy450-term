// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Three attempts (accounts A/B/A), a delayed reply and duplicate, ageing and deadline.
use super::{action, all, eq, implies, model, safe};
use bboxy450_tlc::{
    Expr::{Int, Var},
    Model, Response,
};
fn poller(broken: bool) -> Model {
    let mut m = model(
        "UsagePoller",
        broken,
        &[
            ("epoch", 3, 0),
            ("account", 2, 0),
            ("pending", 1, 0),
            ("reply_epoch", 3, 0),
            ("reply_account", 2, 0),
            ("deliveries", 2, 0),
            ("visible_epoch", 3, 0),
            ("visible_account", 2, 0),
            ("fresh", 1, 0),
            ("expired", 1, 0),
            ("publications", 2, 0),
        ],
    );
    for epoch in 1..=3 {
        action(
            &mut m,
            &format!("RefreshOrSwitch{epoch}"),
            eq(0, epoch - 1),
            &[
                (0, Int(epoch)),
                (1, Int(if epoch == 2 { 2 } else { 1 })),
                (2, Int(1)),
                (6, Int(0)),
                (7, Int(0)),
                (8, Int(0)),
                (9, Int(0)),
                (10, Int(0)),
            ],
        );
    }
    action(
        &mut m,
        "Capture",
        all([eq(2, 1), eq(5, 0)]),
        &[(3, Var(0)), (4, Var(1)), (5, Int(2))],
    );
    let guard = all([eq(2, 1), Var(3).eq(Var(0)), Var(4).eq(Var(1))]);
    let accept = if broken {
        bboxy450_tlc::Expr::Bool(true)
    } else {
        guard
    };
    action(
        &mut m,
        "Complete",
        Int(0).lt(Var(5)),
        &[
            (5, Var(5).minus(Int(1))),
            (6, accept.clone().ite(Var(3), Var(6))),
            (7, accept.clone().ite(Var(4), Var(7))),
            (8, accept.clone().ite(Int(1), Var(8))),
            (10, accept.clone().ite(Var(10).plus(Int(1)), Var(10))),
            (2, accept.ite(Int(0), Var(2))),
        ],
    );
    action(
        &mut m,
        "Deadline",
        eq(2, 1),
        &[
            (2, Int(0)),
            (6, Int(0)),
            (7, Int(0)),
            (8, Int(0)),
            (9, Int(1)),
        ],
    );
    action(&mut m, "Age", eq(8, 1), &[(8, Int(0))]);
    safe(
        &mut m,
        "NoAccountOrAttemptLeak",
        implies(
            Int(0).lt(Var(6)),
            all([Var(6).eq(Var(0)), Var(7).eq(Var(1))]),
        ),
    );
    safe(
        &mut m,
        "FreshHasValue",
        implies(eq(8, 1), Int(0).lt(Var(6))),
    );
    safe(&mut m, "NoTimedOutPublication", implies(eq(9, 1), eq(6, 0)));
    safe(&mut m, "AtMostOnePublication", Var(10).le(Int(1)));
    m.responses.push(Response {
        name: "RefreshTerminates".into(),
        request: eq(2, 1),
        response: eq(2, 0),
    });
    let deadline = m
        .actions
        .iter()
        .position(|action| action.name == "Deadline")
        .unwrap();
    m.weak_fair.push(vec![deadline]);
    m
}
pub fn cases() -> Vec<(Model, &'static str)> {
    vec![(poller(false), "pass"), (poller(true), "safety")]
}
