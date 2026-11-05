// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Control ownership and explicit publication boundaries; no wall-clock proof.
use super::*;

fn background(broken: bool) -> Model {
    let mut m = model(
        "ProvenanceBackground",
        broken,
        &[
            ("head", 3, 0),
            ("cursor", 3, 0),
            ("running", 1, 0),
            ("closed", 1, 0),
            ("generation", 2, 0),
            ("job_generation", 2, 0),
            ("owner_waiting", 1, 0),
        ],
    );
    action(
        &mut m,
        "CommitCheckpoint",
        all([eq(3, 0), Var(0).lt(Int(3))]),
        &[(0, Var(0).plus(Int(1)))],
    );
    action(
        &mut m,
        "DispatchOneJob",
        all([eq(2, 0), eq(3, 0), Var(1).lt(Var(0))]),
        &[(2, Int(1)), (5, Var(4)), (6, Int(i64::from(broken)))],
    );
    action(&mut m, "CloseRoot", eq(3, 0), &[(3, Int(1))]);
    action(
        &mut m,
        "ReplaceRoot",
        all([eq(3, 0), Var(4).lt(Int(2))]),
        &[(4, Var(4).plus(Int(1))), (0, Int(0)), (1, Int(0))],
    );
    action(
        &mut m,
        "FinishJob",
        eq(2, 1),
        &[
            (2, Int(0)),
            (
                1,
                all([eq(3, 0), Var(4).eq(Var(5))]).ite(Var(1).plus(Int(1)), Var(1)),
            ),
        ],
    );
    safe(&mut m, "ControlNeverWaitsForProvenance", eq(6, 0));
    safe(
        &mut m,
        "ProjectionNeverOutrunsCommittedHead",
        Var(1).le(Var(0)),
    );
    m
}
fn publication(broken: bool) -> Model {
    let mut m = model(
        "KnowledgePublication",
        broken,
        &[
            ("draft_edits", 3, 0),
            ("commits", 3, 0),
            ("artifacts", 3, 0),
            ("published", 1, 0),
        ],
    );
    action(
        &mut m,
        "EditDraft",
        all([Var(0).lt(Int(3)), Var(2).lt(Int(3))]),
        &[
            (0, Var(0).plus(Int(1))),
            (2, if broken { Var(2).plus(Int(1)) } else { Var(2) }),
            (3, Int(0)),
        ],
    );
    action(
        &mut m,
        "ExplicitCommit",
        all([Var(1).lt(Int(3)), Var(2).lt(Int(3))]),
        &[
            (1, Var(1).plus(Int(1))),
            (2, Var(2).plus(Int(1))),
            (3, Int(1)),
        ],
    );
    safe(
        &mut m,
        "OnlyExplicitCommitsCreateArtifacts",
        Var(1).eq(Var(2)),
    );
    m
}
pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (background(false), "pass"),
        (background(true), "safety"),
        (publication(false), "pass"),
        (publication(true), "safety"),
    ]
}
