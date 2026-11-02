// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Two receipt slots and four admissions; receipt states are vacant, pending,
//! PTY-written and partial-unknown. The supervisor owns every atomic action.
use super::*;

pub(super) fn input_dedup_eviction(broken: bool) -> Model {
    let (offered, oldest, newest, old_state, new_state) = (0, 1, 2, 3, 4);
    let mut m = model(
        "InputDedupEviction",
        broken,
        &[
            ("offered", 4, 0),
            ("oldest", 4, 0),
            ("newest", 4, 0),
            ("old_state", 3, 0),
            ("new_state", 3, 0),
        ],
    );
    let pending = eq(old_state, 1).or(eq(new_state, 1));
    let admit = Var(offered).lt(Int(4)).and(pending.clone().negate());
    let next = Var(offered).plus(Int(1));
    action(
        &mut m,
        "AdmitFirst",
        admit.clone().and(eq(old_state, 0)),
        &[
            (offered, next.clone()),
            (oldest, next.clone()),
            (old_state, Int(1)),
        ],
    );
    action(
        &mut m,
        "AdmitSecond",
        admit
            .clone()
            .and(Int(1).lt(Var(old_state)))
            .and(eq(new_state, 0)),
        &[
            (offered, next.clone()),
            (newest, next.clone()),
            (new_state, Int(1)),
        ],
    );
    let full_terminal = Int(1).lt(Var(old_state)).and(Int(1).lt(Var(new_state)));
    // The broken control removes only eviction. Retained retries still work,
    // but no fresh input can ever be admitted after the two slots fill.
    if !broken {
        action(
            &mut m,
            "EvictOldestAndAdmit",
            admit.and(full_terminal.clone()),
            &[
                (offered, next.clone()),
                (oldest, Var(newest)),
                (old_state, Var(new_state)),
                (newest, next),
                (new_state, Int(1)),
            ],
        );
    } else {
        action(&mut m, "RefuseAtCapacity", full_terminal, &[]);
    }
    for (slot, state) in [("Old", old_state), ("New", new_state)] {
        action(
            &mut m,
            &format!("Write{slot}"),
            eq(state, 1),
            &[(state, Int(2))],
        );
        action(
            &mut m,
            &format!("Timeout{slot}"),
            eq(state, 1),
            &[(state, Int(3))],
        );
        // Matching retained retries return the current receipt; mismatched
        // bytes refuse without a write. Byte semantics use real PTY witnesses.
        action(&mut m, &format!("Replay{slot}"), ne(state, 0), &[]);
        action(&mut m, &format!("Conflict{slot}"), ne(state, 0), &[]);
    }
    action(&mut m, "BusyDuringWrite", pending.clone(), &[]);
    safe(
        &mut m,
        "SlotIdentityMatchesReceipt",
        all([
            eq(oldest, 0).eq(eq(old_state, 0)),
            eq(newest, 0).eq(eq(new_state, 0)),
        ]),
    );
    safe(
        &mut m,
        "OnlyNewestCanBePending",
        all([
            implies(eq(old_state, 1), eq(offered, 1)),
            implies(eq(new_state, 1), Var(newest).eq(Var(offered))),
            eq(old_state, 1).and(eq(new_state, 1)).negate(),
        ]),
    );
    safe(
        &mut m,
        "OldestFirstBoundedWindow",
        all([
            implies(eq(offered, 1), eq(oldest, 1).and(eq(newest, 0))),
            implies(
                Int(2).le(Var(offered)),
                all([
                    Var(oldest).eq(Var(offered).minus(Int(1))),
                    Var(newest).eq(Var(offered)),
                ]),
            ),
        ]),
    );
    let mut admission = vec![0, 1];
    if !broken {
        admission.push(2);
    }
    m.weak_fair.push(admission);
    let completion = ["WriteOld", "TimeoutOld", "WriteNew", "TimeoutNew"]
        .iter()
        .map(|name| {
            m.actions
                .iter()
                .position(|action| action.name == *name)
                .unwrap()
        })
        .collect();
    m.weak_fair.push(completion);
    m.responses.push(Response {
        name: "FreshInputEventuallyAdmittedBeyondCapacity".into(),
        request: Var(offered).lt(Int(4)),
        response: eq(offered, 4),
    });
    m.responses.push(Response {
        name: "PendingEventuallyTerminal".into(),
        request: pending.clone(),
        response: pending.negate(),
    });
    m
}
