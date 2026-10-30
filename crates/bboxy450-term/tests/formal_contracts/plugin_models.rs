// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Plugin-system contract projections: live connections, hook delivery and
//! the host API. Kept apart from the shared family list so plugin models are
//! added without touching other contracts' lines.
use super::*;

pub fn cases() -> Vec<(Model, &'static str)> {
    let families: [fn(bool) -> Model; 3] = [plugin_connection, plugin_hook, plugin_host_api];
    families
        .into_iter()
        .flat_map(|family| {
            let good = family(false);
            let temporal = !good.responses.is_empty();
            [
                (good, "pass"),
                (family(true), if temporal { "liveness" } else { "safety" }),
            ]
        })
        .collect()
}

fn plugin_connection(b: bool) -> Model {
    let mut m = model(
        "PluginConnection",
        b,
        &[
            ("generation", 4, 1),
            ("enabled", 1, 1),
            ("connection", 1, 0),
            ("connection_generation", 4, 0),
            ("call", 1, 0),
            ("call_generation", 4, 0),
            ("reconcile_pending", 1, 0),
            ("bad", 1, 0),
        ],
    );
    let current = all([eq(1, 1), Var(3).eq(Var(0))]);
    action(
        &mut m,
        "OpenConnection",
        all([eq(2, 0), eq(1, 1)]),
        &[(2, Int(1)), (3, Var(0))],
    );
    action(
        &mut m,
        "ReplaceStaleConnection",
        all([eq(2, 1), eq(1, 1), Var(3).eq(Var(0)).negate()]),
        &[(3, Var(0))],
    );
    action(
        &mut m,
        "StartCall",
        all([eq(4, 0), eq(2, 1), current.clone()]),
        &[(4, Int(1)), (5, Var(3))],
    );
    let bump = Var(0).plus(Int(1));
    action(
        &mut m,
        "Disable",
        all([eq(1, 1), Var(0).lt(Int(4))]),
        &[(1, Int(0)), (0, bump.clone()), (6, Int(1))],
    );
    action(
        &mut m,
        "Enable",
        all([eq(1, 0), Var(0).lt(Int(4))]),
        &[(1, Int(1)), (0, bump), (6, Int(1))],
    );
    action(
        &mut m,
        "ReconcileKeep",
        all([eq(6, 1), current.clone().or(eq(2, 0))]),
        &[(6, Int(0))],
    );
    action(
        &mut m,
        "ReconcileClose",
        all([eq(6, 1), eq(2, 1), current.clone().negate()]),
        &[(6, Int(0)), (2, Int(0))],
    );
    action(
        &mut m,
        "CallFailsOnClosedConnection",
        all([eq(4, 1), eq(2, 0)]),
        &[(4, Int(0))],
    );
    let call_current = all([eq(1, 1), Var(5).eq(Var(0))]);
    action(
        &mut m,
        "DeliverCurrentResult",
        all([eq(4, 1), eq(2, 1), call_current.clone()]),
        &[(4, Int(0))],
    );
    action(
        &mut m,
        "RefuseSupersededResult",
        all([eq(4, 1), eq(2, 1), call_current.clone().negate()]),
        &[(4, Int(0)), (2, Int(0))],
    );
    if b {
        // Without the completion recheck a superseded generation's result is delivered.
        action(
            &mut m,
            "DeliverWithoutGenerationCheck",
            all([eq(4, 1), eq(2, 1), call_current.negate()]),
            &[(4, Int(0)), (7, Int(1))],
        );
    }
    no_bad(&mut m, 7);
    safe(
        &mut m,
        "NoConnectionSurvivesDisable",
        implies(all([eq(1, 0), eq(6, 0)]), eq(2, 0)),
    );
    m
}

fn plugin_hook(b: bool) -> Model {
    let mut m = model(
        "PluginHook",
        b,
        &[
            ("generation", 3, 1),
            ("enabled", 1, 1),
            ("snapshot", 3, 1),
            ("queued", 2, 0),
            ("slot0", 3, 0),
            ("slot1", 3, 0),
            ("dropped", 1, 0),
            ("decision", 2, 0),
            ("waited", 3, 0),
            ("captured", 3, 0),
            ("reply", 2, 0),
            ("verdict", 2, 0),
            ("block_grant", 1, 1),
            ("fail_closed", 1, 0),
            ("bad", 1, 0),
        ],
    );
    let bump = Var(0).plus(Int(1));
    action(
        &mut m,
        "Disable",
        all([eq(1, 1), Var(0).lt(Int(3))]),
        &[(1, Int(0)), (0, bump.clone())],
    );
    action(
        &mut m,
        "Enable",
        all([eq(1, 0), Var(0).lt(Int(3))]),
        &[(1, Int(1)), (0, bump.clone())],
    );
    // Re-trust without the block grant is a new, disabled activation.
    action(
        &mut m,
        "RetrustWithoutBlock",
        all([eq(12, 1), Var(0).lt(Int(3))]),
        &[(12, Int(0)), (1, Int(0)), (0, bump)],
    );
    action(
        &mut m,
        "ChooseFailClosed",
        all([eq(7, 0), eq(13, 0)]),
        &[(13, Int(1))],
    );
    action(
        &mut m,
        "ReconcileEnabled",
        all([eq(1, 1), Var(2).eq(Var(0)).negate()]),
        &[(2, Var(0))],
    );
    action(
        &mut m,
        "ReconcileDisabled",
        all([eq(1, 0), ne(2, 0)]),
        &[(2, Int(0))],
    );
    // notify: offer to the bounded queue, or drop and count; never wait.
    action(
        &mut m,
        "OfferFirst",
        all([ne(2, 0), eq(3, 0)]),
        &[(4, Var(2)), (3, Int(1))],
    );
    action(
        &mut m,
        "OfferSecond",
        all([ne(2, 0), eq(3, 1)]),
        &[(5, Var(2)), (3, Int(2))],
    );
    action(
        &mut m,
        "OfferWhenFull",
        all([ne(2, 0), eq(3, 2), eq(6, 0)]),
        &[(6, Int(1))],
    );
    let shift = [(4, Var(5)), (5, Int(0)), (3, Var(3).minus(Int(1)))];
    let deliverable = all([eq(1, 1), Var(4).eq(Var(0))]);
    action(
        &mut m,
        "DeliverCurrent",
        all([ne(3, 0), deliverable.clone()]),
        &shift,
    );
    action(
        &mut m,
        "RefuseStale",
        all([ne(3, 0), deliverable.clone().negate()]),
        &shift,
    );
    // decide: only a subscriber holding the block grant is waited for.
    action(
        &mut m,
        "StartDecision",
        all([eq(7, 0), ne(2, 0), eq(12, 1)]),
        &[(7, Int(1)), (8, Int(0)), (9, Var(2)), (10, Int(0))],
    );
    action(
        &mut m,
        "AllowWithoutBlocker",
        all([eq(7, 0), eq(2, 0).or(eq(12, 0))]),
        &[(7, Int(2)), (11, Int(1))],
    );
    action(
        &mut m,
        "Tick",
        all([eq(7, 1), Var(8).lt(Int(2))]),
        &[(8, Var(8).plus(Int(1)))],
    );
    action(
        &mut m,
        "PluginAllows",
        all([eq(7, 1), eq(10, 0)]),
        &[(10, Int(1))],
    );
    action(
        &mut m,
        "PluginDenies",
        all([eq(7, 1), eq(10, 0)]),
        &[(10, Int(2))],
    );
    let current = all([eq(1, 1), eq(12, 1), Var(9).eq(Var(0))]);
    action(
        &mut m,
        "AcceptReply",
        all([eq(7, 1), ne(10, 0), current.clone()]),
        &[(7, Int(2)), (11, Var(10))],
    );
    for (name, guard) in [
        (
            "SupersededFailsOpen",
            all([ne(10, 0), current.negate(), eq(13, 0)]),
        ),
        ("TimeoutFailsOpen", all([eq(10, 0), eq(8, 2), eq(13, 0)])),
    ] {
        action(
            &mut m,
            name,
            all([eq(7, 1), guard]),
            &[(7, Int(2)), (11, Int(1))],
        );
    }
    let current = all([eq(1, 1), eq(12, 1), Var(9).eq(Var(0))]);
    for (name, guard) in [
        (
            "SupersededFailsClosed",
            all([ne(10, 0), current.negate(), eq(13, 1)]),
        ),
        ("TimeoutFailsClosed", all([eq(10, 0), eq(8, 2), eq(13, 1)])),
    ] {
        action(
            &mut m,
            name,
            all([eq(7, 1), guard]),
            &[(7, Int(2)), (11, Int(2))],
        );
    }
    if b {
        // Without the generation fence a queued item reaches a disabled or
        // superseded activation.
        let mut stale = shift.to_vec();
        stale.push((14, Int(1)));
        action(
            &mut m,
            "DeliverWithoutGenerationCheck",
            all([ne(3, 0), deliverable.negate()]),
            &stale,
        );
    }
    no_bad(&mut m, 14);
    safe(&mut m, "WaitWithinCeiling", Var(8).le(Int(2)));
    safe(
        &mut m,
        "UndecidedOnlyWhileWaiting",
        implies(eq(11, 0), eq(7, 0).or(eq(7, 1))),
    );
    m
}

// A plugin-originated host request while the host waits on that plugin's
// call (plugins/client.rs demultiplex). The reader admits the request to a
// handler task and keeps reading; the handler checks the grant and the
// connection's generation; the plugin answers its call only after the host
// answered it. Fair scheduling of the reader, handler and plugin is assumed.
fn plugin_host_api(b: bool) -> Model {
    let mut m = model(
        "PluginHostApi",
        b,
        &[
            ("call", 2, 0),
            ("callback", 4, 0),
            ("generation", 2, 1),
            ("connection_generation", 2, 1),
            ("grant", 1, 1),
            ("effect", 1, 0),
            ("refused", 1, 0),
        ],
    );
    action(&mut m, "StartCall", eq(0, 0), &[(0, Int(1))]);
    action(
        &mut m,
        "PluginRequestsHost",
        all([eq(0, 1), eq(1, 0)]),
        &[(1, Int(1))],
    );
    // The broken reader serves plugin requests only between calls.
    let admit = if b {
        all([eq(1, 1), ne(0, 1)])
    } else {
        eq(1, 1)
    };
    action(&mut m, "ReaderAdmitsRequest", admit, &[(1, Int(2))]);
    let granted = all([eq(4, 1), Var(3).eq(Var(2))]);
    action(
        &mut m,
        "HandleGranted",
        all([eq(1, 2), granted.clone()]),
        &[(5, Int(1)), (1, Int(3))],
    );
    action(
        &mut m,
        "RefuseUngrantedOrStale",
        all([eq(1, 2), granted.negate()]),
        &[(6, Int(1)), (1, Int(3))],
    );
    action(&mut m, "PluginReadsAnswer", eq(1, 3), &[(1, Int(4))]);
    action(
        &mut m,
        "PluginAnswersCall",
        all([eq(0, 1), eq(1, 4)]),
        &[(0, Int(2))],
    );
    action(
        &mut m,
        "Supersede",
        Var(2).lt(Int(2)),
        &[(2, Var(2).plus(Int(1)))],
    );
    action(
        &mut m,
        "RetrustWithoutGrant",
        all([eq(4, 1), Var(2).lt(Int(2))]),
        &[(4, Int(0)), (2, Var(2).plus(Int(1)))],
    );
    safe(
        &mut m,
        "AnsweredCallbackHasOneOutcome",
        implies(Var(1).lt(Int(3)).negate(), Var(5).plus(Var(6)).eq(Int(1))),
    );
    m.responses.push(Response {
        name: "CallAnsweredDespiteCallback".into(),
        request: eq(0, 1),
        response: eq(0, 2),
    });
    // The reader, handler and plugin are scheduled fairly; supersession and
    // re-trust are not.
    for fair in [vec![1], vec![2], vec![3, 4], vec![5], vec![6]] {
        m.weak_fair.push(fair);
    }
    m
}
