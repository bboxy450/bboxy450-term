// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded report admission and acceptance; real API and Git fixtures remain witnesses.
use super::*;

// One already reported task, two owners (A-B-A), and absent/own/foreign
// reports. Prepare captures a task revision; publication rechecks it. The
// final validation/publication action is atomic in this projection; filesystem
// durability and external edits racing that final read are not modeled.
// No scheduling fairness or eventual acceptance promise: commands may refuse.
fn report_provenance(b: bool) -> Model {
    let mut m = model(
        "ReportProvenance",
        b,
        &[
            ("owner", 1, 0),
            ("report", 2, 0),
            ("linked", 1, 0),
            ("accepted", 1, 0),
            ("bad", 1, 0),
            ("revision", 2, 0),
            ("prepared", 1, 0),
            ("job_revision", 2, 0),
        ],
    );
    action(
        &mut m,
        "Reassign",
        all([eq(3, 0), Var(5).lt(Int(2))]),
        &[(0, Int(1).minus(Var(0))), (5, Var(5).plus(Int(1)))],
    );
    action(
        &mut m,
        "PrepareOwnReport",
        all([eq(0, 0), eq(1, 0), eq(6, 0)]),
        &[(6, Int(1)), (7, Var(5))],
    );
    action(
        &mut m,
        "PublishFreshReport",
        all([eq(6, 1), eq(1, 0), Var(5).eq(Var(7))]),
        &[(1, Int(1)), (6, Int(0))],
    );
    action(
        &mut m,
        "RefuseStaleTask",
        all([eq(6, 1), Var(5).eq(Var(7)).negate()]),
        &[(6, Int(0))],
    );
    action(&mut m, "LinkReport", eq(3, 0), &[(2, Int(1))]);
    action(&mut m, "RemoveOnDisk", eq(3, 0), &[(1, Int(0))]);
    action(&mut m, "CorruptTaskOnDisk", eq(3, 0), &[(1, Int(2))]);
    action(&mut m, "RefuseUpdate", ne(1, 0), &[]);
    action(&mut m, "RefuseForeignOwner", eq(0, 1), &[]);
    action(&mut m, "RefuseInvalidLink", all([eq(2, 1), ne(1, 1)]), &[]);
    action(
        &mut m,
        "Accept",
        all([eq(3, 0), eq(2, 0).or(eq(1, 1))]),
        &[(3, Int(1))],
    );
    if b {
        action(
            &mut m,
            "AcceptDanglingOrMismatchedReport",
            all([eq(3, 0), eq(2, 1), ne(1, 1)]),
            &[(3, Int(1)), (4, Int(1))],
        );
        action(&mut m, "RewriteReport", ne(1, 0), &[(4, Int(1))]);
        action(
            &mut m,
            "CreateForForeignOwner",
            all([eq(0, 1), eq(1, 0)]),
            &[(4, Int(1))],
        );
        action(
            &mut m,
            "PublishStaleTask",
            all([eq(6, 1), Var(5).eq(Var(7)).negate()]),
            &[(4, Int(1))],
        );
    }
    no_bad(&mut m, 4);
    m
}

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (report_provenance(false), "pass"),
        (report_provenance(true), "safety"),
    ]
}
