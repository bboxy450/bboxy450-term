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
            ("applied0", 3, 0),
            ("applied1", 3, 0),
            ("applied2", 3, 0),
            ("origin0", 3, 0),
            ("origin1", 3, 0),
            ("origin2", 3, 0),
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
        let mut updates = vec![(0, Var(0).plus(Int(1))), (1, Int(active)), (2, Int(0))];
        updates.extend((6..12).map(|i| (i, Int(0))));
        action(
            &mut m,
            &format!("ReplaceOrDrop{active}"),
            Var(0).lt(Int(3)),
            &updates,
        );
    }
    let accepted = all([eq(1, 1), Var(0).eq(Var(3)), Var(2).eq(Var(4))]);
    let mut applied = vec![(2, Var(2).plus(Int(1))), (5, Int(0))];
    for seq in 0..3 {
        let chosen = eq(4, seq as i64);
        applied.push((
            6 + seq,
            chosen.clone().ite(Var(6 + seq).plus(Int(1)), Var(6 + seq)),
        ));
        applied.push((9 + seq, chosen.ite(Var(3), Var(9 + seq))));
    }
    action(
        &mut m,
        "ApplyCurrentNext",
        all([eq(5, 1), accepted.clone()]),
        &applied,
    );
    if b {
        action(
            &mut m,
            "ApplyWithoutIdentityOrOrderFence",
            all([eq(5, 1), Var(2).lt(Int(3)), accepted.clone().negate()]),
            &applied,
        );
    }
    action(
        &mut m,
        "RetireLateDuplicateOrGap",
        all([eq(5, 1), accepted.negate()]),
        &[(5, Int(0))],
    );
    safe(
        &mut m,
        "AppliedExactlyOnceInSequenceOrder",
        all((0..3).map(|seq| Var(6 + seq).eq(Int(seq as i64).lt(Var(2)).ite(Int(1), Int(0))))),
    );
    safe(
        &mut m,
        "AppliedOriginsBelongToCurrentGeneration",
        all((0..3).map(|seq| implies(ne(6 + seq, 0), Var(9 + seq).eq(Var(0))))),
    );
    safe(
        &mut m,
        "DroppedViewHasNoEffects",
        implies(eq(1, 0), eq(2, 0)),
    );
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
            ("definitions", 1, 0),
            ("line_kind", 2, 0),
        ],
    );
    action(
        &mut m,
        "CacheStablePrefix",
        all([eq(2, 0), eq(4, 0), eq(5, 0)]),
        &[(1, Var(0)), (2, Int(1))],
    );
    for tail in 0..=2 {
        action(
            &mut m,
            &format!("AppendOrdinaryOrInlineBracketLine{tail}"),
            all([eq(5, 0), eq(3, tail)]),
            &[(3, Int((tail + 1) % 3)), (5, Int(1))],
        );
    }
    action(
        &mut m,
        "FinishOrdinaryLine",
        eq(5, 1),
        &[(2, eq(4, 1).ite(Int(0), Var(2))), (5, Int(0))],
    );
    action(
        &mut m,
        "AppendReferenceOrFootnoteDefinitionLine",
        all([eq(5, 0), Var(0).lt(Int(2))]),
        &[(0, Var(0).plus(Int(1))), (5, Int(2))],
    );
    // Classification and invalidation are internal steps of one append call.
    // Only line_kind=0 is externally observable; grammar is witnessed in Rust.
    action(
        &mut m,
        "InvalidateDefinitionPrefix",
        eq(5, 2),
        &[
            (2, if b { Var(2) } else { Int(0) }),
            (4, Int(1)),
            (5, Int(0)),
        ],
    );
    safe(
        &mut m,
        "ReturnedPrefixCacheNeverStale",
        implies(all([eq(5, 0), eq(2, 1)]), Var(0).eq(Var(1))),
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
                    (0, Int(i64::from(target >= maximum))),
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
        &[(0, Int(1)), (1, Int(0)), (3, Var(2)), (4, Int(0))],
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
    // Adapted from this project's upstream DirtyWake (models.rs): separate
    // bits/pending publication and drain/recheck, extended with the UI queue.
    // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    let mut m = model(
        "DirtyWakeFrame",
        b,
        &[
            ("bits", 1, 0),
            ("pending", 1, 0),
            ("producer_pc", 1, 0),
            ("drain_pc", 2, 0),
            ("queued", 3, 0),
            ("admitted", 3, 0),
            ("consumed", 3, 0),
            ("frame_ready", 1, 0),
            ("rearm_pc", 1, 0),
        ],
    );
    action(
        &mut m,
        "ArrivalMarksBits",
        all([eq(2, 0), Var(5).lt(Int(3)), Var(4).lt(Int(3))]),
        &[
            (0, Int(1)),
            (2, Int(1)),
            (4, Var(4).plus(Int(1))),
            (5, Var(5).plus(Int(1))),
        ],
    );
    let post = m.actions.len();
    action(
        &mut m,
        "PostCoalescedWake",
        eq(2, 1),
        &[(1, Int(1)), (2, Int(0))],
    );
    let take = m.actions.len();
    action(
        &mut m,
        "TakeBits",
        all([eq(3, 0), eq(1, 1)]),
        &[(0, Int(0)), (3, Int(1)), (7, eq(0, 1).ite(Int(1), Var(7)))],
    );
    let clear = m.actions.len();
    action(
        &mut m,
        "ClearPending",
        eq(3, 1),
        &[(1, Int(0)), (3, Int(2))],
    );
    let empty = m.actions.len();
    action(
        &mut m,
        "RecheckEmpty",
        all([eq(3, 2), eq(0, 0)]),
        &[(3, Int(0))],
    );
    let published = m.actions.len();
    action(
        &mut m,
        "RecheckPublished",
        all([eq(3, 2), eq(0, 1)]),
        &[(3, Int(0)), (1, Int(1))],
    );
    let mut frames = vec![];
    for queued in 0..=3_i64 {
        let count = queued.min(2);
        let mut updates = vec![
            (4, Int(queued - count)),
            (6, Var(6).plus(Int(count))),
            (7, Int(0)),
        ];
        if queued > count && !b {
            updates.extend([(0, Int(1)), (8, Int(1))]);
        }
        frames.push(m.actions.len());
        action(
            &mut m,
            &format!("BudgetedFrame{queued}"),
            all([eq(3, 0), eq(7, 1), eq(8, 0), eq(4, queued)]),
            &updates,
        );
    }
    let rearm = m.actions.len();
    action(
        &mut m,
        "PostBudgetRemainderWake",
        eq(8, 1),
        &[(1, Int(1)), (8, Int(0))],
    );
    safe(
        &mut m,
        "NoLostImmediateWake",
        implies(all([eq(2, 0), eq(3, 0), eq(8, 0), eq(0, 1)]), eq(1, 1)),
    );
    safe(
        &mut m,
        "QueueConservesAdmittedDeltas",
        Var(4).plus(Var(6)).eq(Var(5)),
    );
    m.responses.push(Response {
        name: "EveryAdmittedDeltaReachesAFrame".into(),
        request: ne(4, 0),
        response: eq(4, 0),
    });
    m.weak_fair.extend([
        vec![post],
        vec![take],
        vec![clear],
        vec![empty, published],
        frames,
        vec![rearm],
    ]);
    m
}

/// Keep this workstream's registry out of the shared families array (LAND.md).
pub fn cases() -> Vec<(Model, &'static str)> {
    [
        (stream_to_view as fn(bool) -> Model, "safety"),
        (incremental_markdown, "safety"),
        (scroll_stick, "safety"),
        (dirty_wake_frame, "liveness"),
    ]
    .into_iter()
    .flat_map(|(family, expected)| [(family(false), "pass"), (family(true), expected)])
    .collect()
}

#[cfg(test)]
mod rework_witnesses {
    use super::*;
    #[test]
    fn toolkit_models_and_broken_controls_have_expected_verdicts() {
        for (m, expected) in cases() {
            let r = m.check(100_000).unwrap();
            let actual = if r.violations.is_empty() {
                "pass"
            } else if r.violations.iter().any(|v| !v.cycle.is_empty()) {
                "liveness"
            } else {
                "safety"
            };
            assert_eq!(actual, expected, "{}: {:?}", m.name, r.violations);
            eprintln!(
                "{}: {actual}, {} states, {} edges",
                m.name,
                r.states.len(),
                r.edges.iter().map(Vec::len).sum::<usize>()
            );
        }
    }
    #[test]
    fn removing_the_generation_guard_violates_the_applied_ledger() {
        let mut m = stream_to_view(false);
        let apply = m
            .actions
            .iter_mut()
            .find(|a| a.name == "ApplyCurrentNext")
            .unwrap();
        apply.guard = all([eq(5, 1), Var(2).eq(Var(4))]);
        let r = m.check(100_000).unwrap();
        assert!(
            r.violations
                .iter()
                .any(|v| v.property == "AppliedOriginsBelongToCurrentGeneration"),
            "a removed generation guard must corrupt ledger origins: {:?}",
            r.violations
        );
    }
    #[test]
    fn scroll_model_matches_the_real_endpoint_and_jump_policy() {
        let m = scroll_stick(false);
        let r = m.check(100_000).unwrap();
        let endpoint = m
            .actions
            .iter()
            .position(|a| a.name == "ManualScroll3_Max3")
            .unwrap();
        let jump = m
            .actions
            .iter()
            .position(|a| a.name == "JumpToLatest")
            .unwrap();
        let mut actual = bbt_ui::modules::transcript::ScrollStick::default();
        actual.user_scroll(3., 3.);
        for (from, edges) in r.edges.iter().enumerate() {
            for edge in edges {
                if edge.action == endpoint {
                    assert_eq!(r.states[edge.to][0], i64::from(actual.following()));
                }
                if edge.action == jump {
                    let before = &r.states[from];
                    let after = &r.states[edge.to];
                    let mut real = bbt_ui::modules::transcript::ScrollStick::default();
                    real.user_scroll(before[1] as f32, before[2] as f32);
                    real.jump_to_latest();
                    assert_eq!(after[0], i64::from(real.following()));
                    assert_eq!(after[3] as f32, real.layout_offset().min(after[2] as f32));
                    assert_eq!((after[1], after[4]), (0, 0), "jump resets both intents");
                }
            }
        }
    }

    #[test]
    fn dirty_frame_extends_coalesced_wake_and_can_consume_two_arrivals() {
        let m = dirty_wake_frame(false);
        for name in ["bits", "pending", "drain_pc", "queued", "consumed"] {
            assert!(
                m.variables.iter().any(|v| v.name == name),
                "missing upstream wake/queue state {name}"
            );
        }
        let consumed = m
            .variables
            .iter()
            .position(|v| v.name == "consumed")
            .unwrap();
        let r = m.check(100_000).unwrap();
        assert!(
            r.edges.iter().enumerate().any(|(from, edges)| edges
                .iter()
                .any(|e| r.states[e.to][consumed] - r.states[from][consumed] == 2)),
            "two arrivals must coalesce into one frame"
        );
    }
}
