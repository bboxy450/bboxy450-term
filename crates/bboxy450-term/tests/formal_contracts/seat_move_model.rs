// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One seat, two swarms (A=0, B=1), policy epochs 0..3, one message addressed
//! to the seat and one signed by it. The operator's move (`SeatControl::Move`)
//! is one atomic action on the logic thread; so is each messaging step.
use super::*;

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![(seat_move(false), "pass"), (seat_move(true), "safety")]
}

const SWARM: usize = 0;
const EPOCH: usize = 1;
const FLOOR: usize = 2;
const GRANT: [usize; 2] = [3, 4];
const HAS_B: usize = 5;
const INBOX: [usize; 2] = [6, 7];
const MSG: usize = 8;
const MSG_SWARM: usize = 9;
const DELIVERED: usize = 10;
const SENT: usize = 11;
const SENT_SWARM: usize = 12;
const SENT_EPOCH: usize = 13;
const BAD: usize = 14;

// Message to the seat: 0 none, 1 waiting for its process, 2 delivered, 3 held
// (muted or paused). Message from the seat: 0 none, 1 signed and held out,
// 2 released to its recipients. No fairness: a message may wait forever.
// The broken control keeps a grant valid after the seat left its swarm.
fn seat_move(b: bool) -> Model {
    let mut m = model(
        "SeatMove",
        b,
        &[
            ("swarm", 1, 0),
            ("epoch", 3, 0),
            ("moved_epoch", 3, 0),
            ("grant_a", 3, 0),
            ("grant_b", 3, 0),
            ("has_grant_b", 1, 0),
            ("inbox_a", 1, 1),
            ("inbox_b", 1, 0),
            ("message", 3, 0),
            ("message_swarm", 1, 0),
            ("delivered", 2, 0),
            ("sent", 2, 0),
            ("sent_swarm", 1, 0),
            ("sent_epoch", 3, 0),
            ("bad", 1, 0),
        ],
    );
    let next = Var(EPOCH).plus(Int(1));
    for (name, from) in [("A", 0), ("B", 1)] {
        let to = 1 - from;
        // Refused while a message still waits for the process; the old tree
        // loses the inbox and every earlier grant falls below the move epoch.
        let mut updates = vec![
            (SWARM, Int(to as i64)),
            (EPOCH, next.clone()),
            (FLOOR, next.clone()),
            (GRANT[to], next.clone()),
            (INBOX[from], Int(0)),
            (INBOX[to], Int(1)),
        ];
        if to == 1 {
            updates.push((HAS_B, Int(1)));
        }
        action(
            &mut m,
            &format!("MoveOutOf{name}"),
            all([eq(SWARM, from as i64), Var(EPOCH).lt(Int(3)), ne(MSG, 1)]),
            &updates,
        );
        // Role and mute changes issue a newer grant in place.
        action(
            &mut m,
            &format!("PolicyChangeIn{name}"),
            all([eq(SWARM, from as i64), Var(EPOCH).lt(Int(3))]),
            &[(EPOCH, next.clone()), (GRANT[from], next.clone())],
        );
    }
    let routed = eq(MSG_SWARM, 0)
        .and(eq(INBOX[0], 1))
        .or(eq(MSG_SWARM, 1).and(eq(INBOX[1], 1)));
    for (name, state) in [("Address", 1), ("AddressHeld", 3)] {
        action(
            &mut m,
            name,
            eq(MSG, 0),
            &[(MSG, Int(state)), (MSG_SWARM, Var(SWARM))],
        );
    }
    action(&mut m, "Hold", eq(MSG, 1), &[(MSG, Int(3))]);
    // A held message is released only by the tree that still routes to the seat.
    action(
        &mut m,
        "Unhold",
        all([eq(MSG, 3), routed.clone()]),
        &[(MSG, Int(1))],
    );
    action(
        &mut m,
        "Deliver",
        all([eq(MSG, 1), routed]),
        &[(MSG, Int(2)), (DELIVERED, Var(DELIVERED).plus(Int(1)))],
    );
    action(
        &mut m,
        "Sign",
        eq(SENT, 0),
        &[
            (SENT, Int(1)),
            (SENT_SWARM, Var(SWARM)),
            (SENT_EPOCH, Var(EPOCH)),
        ],
    );
    // Validation: the grant names the seat's swarm and is not older than its last move.
    let current = Var(SENT_SWARM)
        .eq(Var(SWARM))
        .and(Var(FLOOR).le(Var(SENT_EPOCH)));
    action(
        &mut m,
        "Release",
        all([eq(SENT, 1), if b { Bool(true) } else { current.clone() }]),
        &[(SENT, Int(2)), (BAD, current.ite(Int(0), Int(1)))],
    );
    let valid = |swarm: usize, issued: Expr| {
        if b {
            issued
        } else {
            all([
                issued,
                eq(SWARM, swarm as i64),
                Var(FLOOR).le(Var(GRANT[swarm])),
            ])
        }
    };
    let count = |holds: Expr| holds.ite(Int(1), Int(0));
    safe(
        &mut m,
        "ExactlyOneSwarmHoldsTheCurrentGrant",
        count(valid(0, Bool(true)))
            .plus(count(valid(1, eq(HAS_B, 1))))
            .eq(Int(1)),
    );
    // Every issued grant carries an epoch the policy has reached: it never went back.
    safe(
        &mut m,
        "GrantEpochNeverDecreases",
        all([
            Var(FLOOR).le(Var(EPOCH)),
            Var(GRANT[0]).le(Var(EPOCH)),
            Var(GRANT[1]).le(Var(EPOCH)),
            Var(SENT_EPOCH).le(Var(EPOCH)),
        ]),
    );
    safe(
        &mut m,
        "OneInboxWhereTheSeatIs",
        all([
            Var(INBOX[0]).plus(Var(INBOX[1])).eq(Int(1)),
            implies(eq(SWARM, 0), eq(INBOX[0], 1)),
            implies(eq(SWARM, 1), eq(INBOX[1], 1)),
        ]),
    );
    safe(&mut m, "DeliveredAtMostOnce", Var(DELIVERED).le(Int(1)));
    no_bad(&mut m, BAD);
    m
}
