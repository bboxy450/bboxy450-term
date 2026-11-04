// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Startup may proceed only after this generation's required MCP connection.
use super::{action, all, eq, implies, model, safe};
use bboxy450_tlc::{
    Expr::{Int, Var},
    Model,
};

fn readiness(broken: bool) -> Model {
    let mut m = model(
        "CodexMcpReadiness",
        broken,
        &[
            ("door", 3, 0),
            ("generation", 2, 1),
            ("phase", 3, 0),
            ("catalog_generation", 2, 0),
        ],
    );
    for door in 1..=3 {
        action(
            &mut m,
            &format!("LaunchDoor{door}"),
            eq(2, 0),
            &[(0, Int(door)), (2, Int(1))],
        );
    }
    for generation in 1..=2 {
        action(
            &mut m,
            &format!("ConnectCatalog{generation}"),
            all([eq(2, 1), eq(1, generation)]),
            &[(3, Int(generation))],
        );
    }
    action(
        &mut m,
        "DiscardOldCompletion",
        all([eq(1, 2), eq(2, 1)]),
        &[],
    );
    action(&mut m, "ConnectionFailure", eq(2, 1), &[(2, Int(3))]);
    action(
        &mut m,
        "StartSession",
        if broken {
            eq(2, 1)
        } else {
            all([eq(2, 1), Var(1).eq(Var(3))])
        },
        &[(2, Int(2))],
    );
    action(
        &mut m,
        "Restart",
        all([eq(1, 1), eq(0, 0).negate()]),
        &[(1, Int(2)), (2, Int(1)), (3, Int(0))],
    );
    action(&mut m, "RetainedOperation", eq(2, 2).or(eq(2, 3)), &[]);
    safe(
        &mut m,
        "SessionHasCurrentCatalog",
        implies(eq(2, 2), Var(1).eq(Var(3))),
    );
    m
}

pub fn cases() -> Vec<(Model, &'static str)> {
    vec![(readiness(false), "pass"), (readiness(true), "safety")]
}
