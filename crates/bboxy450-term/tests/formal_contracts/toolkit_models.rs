// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Finite toolkit projections; byte parsing and pixels use the mapped witnesses.
//! Uses the same generation fence as AsyncRenderPreparation, stable identity as
//! RenderLinePlan and re-mark discipline as DirtyWake; no second runtime exists.
use super::*;
pub fn stream_to_view(b: bool) -> Model {
    let mut m = model(
        "StreamToView",
        b,
        &[
            ("generation", 3, 1),
            ("active", 1, 1),
            ("next", 3, 0),
            ("captured_generation", 3, 0),
            ("sequence", 2, 0),
            ("pending", 1, 0),
            ("bad", 1, 0),
        ],
    );
    for seq in 0..3 {
        action(
            &mut m,
            &format!("Capture{seq}"),
            all([eq(1, 1), eq(5, 0)]),
            &[(3, Var(0)), (4, Int(seq)), (5, Int(1))],
        );
    }
    for active in 0..=1 {
        action(
            &mut m,
            &format!("ReplaceOrDrop{active}"),
            Var(0).lt(Int(3)),
            &[(0, Var(0).plus(Int(1))), (1, Int(active)), (2, Int(0))],
        );
    }
    let identity = all([eq(1, 1), Var(0).eq(Var(3))]);
    let order = Var(2).eq(Var(4));
    let accepted = all([order.clone(), identity.clone()]);
    action(
        &mut m,
        "ApplyCurrentNext",
        all([eq(5, 1), accepted.clone()]),
        &[(2, Var(2).plus(Int(1))), (5, Int(0))],
    );
    if b {
        action(
            &mut m,
            "ApplyObsolete",
            all([eq(5, 1), order, identity.negate()]),
            &[(2, Var(2).plus(Int(1))), (5, Int(0)), (6, Int(1))],
        );
    }
    action(
        &mut m,
        "RetireLateDuplicateOrGap",
        all([eq(5, 1), accepted.negate()]),
        &[(5, Int(0))],
    );
    no_bad(&mut m, 6);
    m
}
pub fn incremental_markdown(b: bool) -> Model {
    let mut m = model(
        "IncrementalMarkdown",
        b,
        &[
            ("prefix_identity", 2, 0),
            ("cached_identity", 2, 0),
            ("cached", 1, 0),
            ("tail", 2, 0),
        ],
    );
    action(
        &mut m,
        "CacheStablePrefix",
        eq(2, 0),
        &[(1, Var(0)), (2, Int(1))],
    );
    for tail in 0..=2 {
        action(
            &mut m,
            &format!("AppendOpenTail{tail}"),
            eq(3, tail),
            &[(3, Int((tail + 1) % 3))],
        );
    }
    // A late reference or an incorrectly frozen EOF block changes the semantic
    // prefix. Correct append invalidates before any retained render can use it.
    action(
        &mut m,
        "AppendReopensPrefix",
        Var(0).lt(Int(2)),
        &[
            (0, Var(0).plus(Int(1))),
            (2, if b { Var(2) } else { Int(0) }),
        ],
    );
    safe(
        &mut m,
        "CachedPrefixNeverStale",
        implies(eq(2, 1), Var(0).eq(Var(1))),
    );
    m
}
pub fn scroll_stick(b: bool) -> Model {
    let mut m = model(
        "ScrollStick",
        b,
        &[
            ("following", 1, 1),
            ("requested", 3, 0),
            ("maximum", 3, 0),
            ("actual", 3, 0),
            ("manual_intent", 3, 0),
        ],
    );
    for target in 0_i64..=3 {
        for maximum in 0..=3 {
            action(
                &mut m,
                &format!("ManualScroll{target}_Max{maximum}"),
                eq(2, maximum),
                &[
                    (0, Int(0)),
                    (1, Int(target)),
                    (3, Int(target.min(maximum))),
                    (4, Int(target)),
                ],
            );
        }
        action(
            &mut m,
            &format!("FollowingLayout{target}"),
            eq(0, 1),
            &[(2, Int(target)), (3, Int(target))],
        );
        for requested in 0..=3 {
            action(
                &mut m,
                &format!("ManualLayout{target}_Requested{requested}"),
                all([eq(0, 0), eq(1, requested)]),
                &[
                    (2, Int(target)),
                    (1, Int(if b { target } else { requested })),
                    (3, Int(if b { target } else { target.min(requested) })),
                ],
            );
        }
    }
    action(
        &mut m,
        "JumpToLatest",
        Bool(true),
        &[(0, Int(1)), (3, Var(2))],
    );
    safe(
        &mut m,
        "ManualIntentSurvivesLayout",
        implies(eq(0, 0), Var(1).eq(Var(4))),
    );
    safe(
        &mut m,
        "FollowingShowsTail",
        implies(eq(0, 1), Var(3).eq(Var(2))),
    );
    m
}
pub fn dirty_wake_frame(b: bool) -> Model {
    let mut m = model(
        "DirtyWakeFrame",
        b,
        &[("arrivals", 3, 0), ("frames", 3, 0), ("wake", 1, 0)],
    );
    action(
        &mut m,
        "ArrivalMarksDirty",
        Var(0).lt(Int(3)),
        &[(0, Var(0).plus(Int(1))), (2, Int(1))],
    );
    let mut drains = vec![];
    for arrivals in 1..=3 {
        for frames in 0..arrivals {
            drains.push(m.actions.len());
            action(
                &mut m,
                &format!("BudgetedFrame{arrivals}_{frames}"),
                all([eq(2, 1), eq(0, arrivals), eq(1, frames)]),
                &[
                    (1, Int(frames + 1)),
                    (2, Int(i64::from(!b && frames + 1 < arrivals))),
                ],
            );
        }
    }
    // One quantum per frame is an abstraction of the host budget. Weakly fair
    // event-loop servicing, finite arrivals; no frame-time or OS latency proof.
    m.responses.push(Response {
        name: "EveryArrivalGetsAFrame".into(),
        request: Var(1).lt(Var(0)),
        response: Var(1).eq(Var(0)),
    });
    m.weak_fair.push(drains);
    m
}

/// Keep this workstream's registry out of the shared families array (LAND.md).
pub fn cases() -> Vec<(Model, &'static str)> {
    let families: [fn(bool) -> Model; 4] = [
        stream_to_view,
        incremental_markdown,
        scroll_stick,
        dirty_wake_frame,
    ];
    families
        .into_iter()
        .flat_map(|family| {
            let good = family(false);
            let expected = if good.responses.is_empty() {
                "safety"
            } else {
                "liveness"
            };
            [(good, "pass"), (family(true), expected)]
        })
        .collect()
}
