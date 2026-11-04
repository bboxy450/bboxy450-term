// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One recipient, limit two, two reset epochs and one uncertain native call.
use super::*;

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    ["Self", "Budget", "Latch", "Refund"]
        .into_iter()
        .flat_map(|control| {
            let name = format!("WakeCircuit{control}");
            let mut good = circuit(None);
            good.name = name.clone();
            let mut bad = circuit(Some(control));
            bad.name = format!("Broken{name}");
            [(good, "pass"), (bad, "safety")]
        })
        .collect()
}

fn circuit(control: Option<&str>) -> Model {
    let mut m = model(
        "WakeCircuit",
        false,
        &[
            ("count", 3, 0),
            ("open", 1, 0),
            ("inflight", 1, 0),
            ("epoch", 1, 0),
            ("reservation_epoch", 1, 0),
            ("bad", 1, 0),
            ("possible_native_effects", 3, 0),
            ("latched_since_operator", 1, 0),
        ],
    );
    action(
        &mut m,
        "SelfNotify",
        Bool(true),
        &[(5, Int(i64::from(control == Some("Self"))))],
    );
    action(
        &mut m,
        "ReservePeer",
        all([
            eq(1, 0),
            eq(2, 0),
            Var(0).lt(Int(if control == Some("Budget") { 3 } else { 2 })),
        ]),
        &[(0, Var(0).plus(Int(1))), (2, Int(1)), (4, Var(3))],
    );
    action(
        &mut m,
        "TripBeforeNinth",
        all([eq(0, 2), eq(1, 0), eq(2, 0)]),
        &[(1, Int(1)), (7, Int(1))],
    );
    let same = Var(3).eq(Var(4));
    for name in ["NativeQueuedOrUnknown", "RestartWithUnknownEffect"] {
        action(
            &mut m,
            name,
            eq(2, 1),
            &[
                (2, Int(0)),
                (
                    6,
                    same.clone()
                        .ite(Var(6).lt(Int(3)).ite(Var(6).plus(Int(1)), Var(6)), Var(6)),
                ),
                (
                    0,
                    if control == Some("Refund") {
                        all([same.clone(), Int(0).lt(Var(0))]).ite(Var(0).minus(Int(1)), Var(0))
                    } else {
                        Var(0)
                    },
                ),
            ],
        );
    }
    action(
        &mut m,
        "ProvenPreEffectFailure",
        eq(2, 1),
        &[
            (
                0,
                all([same.clone(), Int(0).lt(Var(0))]).ite(Var(0).minus(Int(1)), Var(0)),
            ),
            (2, Int(0)),
        ],
    );
    action(
        &mut m,
        "FreshDirectOperatorMessage",
        eq(3, 0),
        &[
            (0, Int(0)),
            (1, Int(0)),
            (3, Int(1)),
            (6, Int(0)),
            (7, Int(0)),
        ],
    );
    action(
        &mut m,
        "ElapsedUntrippedWindow",
        all([eq(1, 0), eq(2, 0)]),
        &[(0, Int(0)), (6, Int(0))],
    );
    action(
        &mut m,
        "ElapsedTrippedWindow",
        eq(1, 1),
        &[(
            1,
            if control == Some("Latch") {
                Int(0)
            } else {
                Var(1)
            },
        )],
    );
    action(&mut m, "AgentAckOrPolicyEditOrReplay", Bool(true), &[]);
    safe(&mut m, "AtMostLimitReservations", Var(0).le(Int(2)));
    safe(&mut m, "UnknownEffectsKeepCredit", Var(6).le(Var(0)));
    safe(
        &mut m,
        "OnlyOperatorRearmsLatch",
        implies(eq(7, 1), eq(1, 1)),
    );
    no_bad(&mut m, 5);
    m
}
