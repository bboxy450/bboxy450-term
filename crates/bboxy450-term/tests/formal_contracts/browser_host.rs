// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One process owner, with reused wire IDs separated by monotonically issued pipes.
use super::*;

const PHASE: usize = 0;
const EPOCH: usize = 1;
const ISSUED: usize = 2;
const SEAT_CLOSED: usize = 3;
const OWNS_HOST: usize = 4;
const MISDELIVERY: usize = 5;

const ABSENT: i64 = 0;
const STARTING: i64 = 1;
const READY: i64 = 2;
const IN_FLIGHT: i64 = 3;
const CRASHED: i64 = 4;
const CLOSING: i64 = 5;

pub(super) fn cases() -> impl Iterator<Item = (Model, &'static str)> {
    [false, true].into_iter().map(|broken| {
        let mut model = model(
            "BrowserHost",
            broken,
            &[
                ("phase", CLOSING, ABSENT),
                ("epoch", 3, 0),
                ("issued", 2, 0),
                ("seat_closed", 1, 0),
                ("owns_host", 1, 0),
                ("misdelivery", 1, 0),
            ],
        );
        action(
            &mut model,
            "Start",
            all([eq(PHASE, ABSENT), eq(SEAT_CLOSED, 0), Var(EPOCH).lt(Int(3))]),
            &[
                (PHASE, Int(STARTING)),
                (EPOCH, Var(EPOCH).plus(Int(1))),
                (ISSUED, Int(0)),
                (OWNS_HOST, Int(1)),
            ],
        );
        action(
            &mut model,
            "Ready",
            eq(PHASE, STARTING),
            &[(PHASE, Int(READY))],
        );
        action(
            &mut model,
            "Send",
            all([eq(PHASE, READY), Var(ISSUED).lt(Int(2))]),
            &[(PHASE, Int(IN_FLIGHT)), (ISSUED, Var(ISSUED).plus(Int(1)))],
        );
        action(
            &mut model,
            "Cancel",
            eq(PHASE, IN_FLIGHT),
            &[(PHASE, Int(READY))],
        );

        for epoch in 1..=3 {
            for request in 1..=2 {
                let admitted = eq(EPOCH, epoch).and(eq(ISSUED, request));
                // Old replies may repeat, including after A/E1 -> B/E2 -> A/E3.
                // Older pipes conservatively may have emitted either wire ID.
                let possible = Int(epoch)
                    .lt(Var(EPOCH))
                    .or(eq(EPOCH, epoch).and(Int(request - 1).lt(Var(ISSUED))));
                action(
                    &mut model,
                    &format!("Reply{epoch}To{request}"),
                    all([
                        eq(PHASE, IN_FLIGHT),
                        possible,
                        if broken { Bool(true) } else { admitted.clone() },
                    ]),
                    &[
                        (PHASE, Int(READY)),
                        (
                            MISDELIVERY,
                            Expr::Ite(Box::new(admitted), Box::new(Int(0)), Box::new(Int(1))),
                        ),
                    ],
                );
            }
        }
        action(
            &mut model,
            "Crash",
            all([Int(ABSENT).lt(Var(PHASE)), Var(PHASE).lt(Int(CRASHED))]),
            &[(PHASE, Int(CRASHED))],
        );
        action(
            &mut model,
            "BrowserClose",
            all([Int(ABSENT).lt(Var(PHASE)), Var(PHASE).lt(Int(CLOSING))]),
            &[(PHASE, Int(CLOSING))],
        );
        action(
            &mut model,
            "SeatClose",
            eq(SEAT_CLOSED, 0),
            &[
                (SEAT_CLOSED, Int(1)),
                (
                    PHASE,
                    Expr::Ite(
                        Box::new(eq(OWNS_HOST, 0)),
                        Box::new(Int(ABSENT)),
                        Box::new(Int(CLOSING)),
                    ),
                ),
            ],
        );
        let reap = model.actions.len();
        let retiring = eq(PHASE, CLOSING).or(eq(PHASE, CRASHED));
        action(
            &mut model,
            "ReapAndRemoveProfile",
            retiring.clone(),
            &[(PHASE, Int(ABSENT)), (OWNS_HOST, Int(0))],
        );
        safe(
            &mut model,
            "OnlyCurrentPipeAndRequestDeliver",
            eq(MISDELIVERY, 0),
        );
        safe(
            &mut model,
            "AbsentHasNoHost",
            implies(eq(PHASE, ABSENT), eq(OWNS_HOST, 0)),
        );
        safe(
            &mut model,
            "ClosedSeatCannotRun",
            implies(eq(SEAT_CLOSED, 1), eq(PHASE, ABSENT).or(eq(PHASE, CLOSING))),
        );
        model.weak_fair.push(vec![reap]);
        model.responses.push(Response {
            name: "RetiringHostEventuallyReaped".into(),
            request: retiring,
            response: eq(PHASE, ABSENT),
        });
        (model, if broken { "safety" } else { "pass" })
    })
}
