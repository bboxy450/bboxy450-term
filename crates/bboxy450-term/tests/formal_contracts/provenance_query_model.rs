// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Cursor scope and evidence grade abstractions, not cryptographic proofs.
use super::*;
fn cursor(broken: bool) -> Model {
    let mut m = model(
        "ProvenanceQueryCursor",
        broken,
        &[
            ("view_epoch", 2, 0),
            ("query", 1, 0),
            ("snapshot", 2, 0),
            ("saved_epoch", 2, 0),
            ("saved_query", 1, 0),
            ("saved_snapshot", 2, 0),
            ("issued", 1, 0),
            ("returned", 1, 0),
        ],
    );
    action(
        &mut m,
        "IssueCursor",
        eq(6, 0),
        &[
            (3, Var(0)),
            (4, Var(1)),
            (5, Var(2)),
            (6, Int(1)),
            (7, Int(0)),
        ],
    );
    action(
        &mut m,
        "ChangeAuthorityOrWorkspace",
        Var(0).lt(Int(2)),
        &[(0, Var(0).plus(Int(1))), (7, Int(0))],
    );
    action(&mut m, "ChangeQuery", eq(1, 0), &[(1, Int(1)), (7, Int(0))]);
    action(
        &mut m,
        "EditSnapshot",
        Var(2).lt(Int(2)),
        &[(2, Var(2).plus(Int(1))), (7, Int(0))],
    );
    action(
        &mut m,
        "ReadPage",
        all([
            eq(6, 1),
            if broken {
                Bool(true)
            } else {
                Var(0).eq(Var(3))
            },
            Var(1).eq(Var(4)),
            Var(2).eq(Var(5)),
        ]),
        &[(7, Int(1))],
    );
    safe(
        &mut m,
        "CursorCannotCrossContext",
        implies(
            eq(7, 1),
            all([Var(0).eq(Var(3)), Var(1).eq(Var(4)), Var(2).eq(Var(5))]),
        ),
    );
    m
}
fn evidence(broken: bool) -> Model {
    let mut m = model(
        "ProvenanceEvidenceGrade",
        broken,
        &[
            ("signature", 1, 0),
            ("columns", 1, 0),
            ("local_link", 1, 0),
            ("integrity_valid", 1, 0),
            ("trust_checked", 1, 0),
            ("trust_claimed", 1, 0),
        ],
    );
    action(&mut m, "VerifySignature", eq(0, 0), &[(0, Int(1))]);
    action(&mut m, "CheckColumns", eq(1, 0), &[(1, Int(1))]);
    action(&mut m, "CheckLocalLink", eq(2, 0), &[(2, Int(1))]);
    action(
        &mut m,
        "PublishIntegrity",
        all([eq(0, 1), eq(1, 1), eq(2, 1)]),
        &[(3, Int(1)), (5, Int(i64::from(broken)))],
    );
    safe(
        &mut m,
        "IntegrityRequiresAllLocalChecks",
        implies(eq(3, 1), all([eq(0, 1), eq(1, 1), eq(2, 1)])),
    );
    safe(
        &mut m,
        "SignatureDoesNotCertifyAuthority",
        Var(5).le(Var(4)),
    );
    m
}
pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (cursor(false), "pass"),
        (cursor(true), "safety"),
        (evidence(false), "pass"),
        (evidence(true), "safety"),
    ]
}
