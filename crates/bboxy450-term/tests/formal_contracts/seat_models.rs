// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded seat recovery contracts; SQL, signing and arbitrary Rust use API witnesses.
use super::*;

// One durable operation, two roots and two seat identities. Ledger 0=missing,
// 1=admitted/unknown, 2=completed. Atomic journal writes survive restart.
// No fairness assumption: a lost completion may remain unknown forever.
pub(super) fn recovery(b: bool) -> Model {
    let mut m = model(
        "OperationRecovery",
        b,
        &[
            ("ledger", 2, 0),
            ("effects", 2, 0),
            ("root", 1, 0),
            ("actor", 1, 0),
            ("bad", 1, 0),
            ("launch", 1, 0),
        ],
    );
    action(&mut m, "Admit", eq(0, 0), &[(0, Int(1))]);
    action(
        &mut m,
        "EffectOnce",
        all([eq(0, 1), eq(1, 0)]),
        &[(1, Int(1))],
    );
    action(
        &mut m,
        "Complete",
        all([eq(0, 1), eq(1, 1)]),
        &[(0, Int(2))],
    );
    for i in [2, 3, 5] {
        action(
            &mut m,
            &format!("ChangeContext{i}"),
            Bool(true),
            &[(i, Int(1).minus(Var(i)))],
        );
    }
    action(&mut m, "LookupUnknown", Var(0).lt(Int(2)), &[]);
    action(
        &mut m,
        "LookupCompletedOwnRoot",
        all([eq(0, 2), eq(2, 0), eq(3, 0)]),
        &[],
    );
    action(
        &mut m,
        "RefuseOtherContext",
        all([eq(0, 2), ne(2, 0).or(ne(3, 0))]),
        &[],
    );
    if b {
        action(
            &mut m,
            "LookupRedispatchesUnknown",
            all([eq(0, 1), eq(1, 1)]),
            &[(1, Int(2))],
        );
        action(
            &mut m,
            "LookupLeaksOtherContext",
            all([eq(0, 2), ne(2, 0).or(ne(3, 0))]),
            &[(4, Int(1))],
        );
        action(&mut m, "MissingMeansTerminal", eq(0, 0), &[(4, Int(1))]);
    }
    safe(&mut m, "AtMostOneOriginalEffect", Var(1).le(Int(1)));
    no_bad(&mut m, 4);
    m
}

// One asynchronous read, a two-step workspace A-B-A change, and one row from
// either seat. No fairness assumption on filesystem IO; missing completion may
// remain unavailable. Admission and publication run atomically on logic.
pub(super) fn diagnostics(b: bool) -> Model {
    let mut m = model(
        "DiagnosticRead",
        b,
        &[
            ("revision", 2, 0),
            ("root", 1, 0),
            ("job_revision", 2, 0),
            ("job_root", 1, 0),
            ("job", 2, 0),
            ("row_owner", 1, 0),
            ("operator_mark", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "StartRead",
        eq(4, 0),
        &[(2, Var(0)), (3, Var(1)), (4, Int(1))],
    );
    action(
        &mut m,
        "WorkspaceChanges",
        Var(0).lt(Int(2)),
        &[(0, Var(0).plus(Int(1))), (1, Int(1).minus(Var(1)))],
    );
    action(&mut m, "ObserveForeignRow", eq(4, 1), &[(5, Int(1))]);
    let current = Var(0).eq(Var(2));
    action(
        &mut m,
        "PublishOwnRedactedRow",
        all([eq(4, 1), eq(5, 0), current.clone()]),
        &[(4, Int(2))],
    );
    action(
        &mut m,
        "FilterForeignRow",
        all([eq(4, 1), eq(5, 1)]),
        &[(4, Int(2))],
    );
    action(
        &mut m,
        "RefuseStaleRead",
        all([eq(4, 1), current.clone().negate()]),
        &[(4, Int(2))],
    );
    if b {
        action(
            &mut m,
            "PublishForeignRow",
            all([eq(4, 1), eq(5, 1)]),
            &[(4, Int(2)), (7, Int(1))],
        );
        action(
            &mut m,
            "PublishByRootIdOnly",
            all([eq(4, 1), Var(1).eq(Var(3)), current.negate()]),
            &[(4, Int(2)), (7, Int(1))],
        );
        action(
            &mut m,
            "AcknowledgeOperatorFromSeat",
            eq(4, 1),
            &[(6, Int(1))],
        );
    }
    safe(&mut m, "OperatorReadMarkUnchanged", eq(6, 0));
    no_bad(&mut m, 7);
    m
}
