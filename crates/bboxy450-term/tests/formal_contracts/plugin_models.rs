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
    let families: [fn(bool) -> Model; 1] = [plugin_connection];
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
