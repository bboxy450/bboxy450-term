// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Finite read-only observation ownership/snapshot/eligibility projections.
//! Opaque allocator overhead, native filesystem/SQL timing, byte hashing and
//! arbitrary Rust behavior are unmodeled; actual API/process witnesses cover them.
use super::*;
pub(super) fn cases() -> Vec<(Model, &'static str)> {
    let mut cases = vec![];
    for (family, controls) in [
        ("DreamReadOwnership", &["Foreign", "ABA"][..]),
        ("DreamReadSnapshot", &["MixedCursor"][..]),
        (
            "DreamReadFeedback",
            &[
                "ForeignOffer",
                "ForeignEvidence",
                "ABA",
                "Digest",
                "Duplicate",
                "Promote",
            ][..],
        ),
        (
            "DreamReadEligibility",
            &[
                "FakeLesson",
                "RowBinding",
                "ClusterOrigin",
                "MemberCompleteness",
                "Reservation",
                "MissingPause",
            ][..],
        ),
    ] {
        let mut good = projection(family, None);
        good.name = family.into();
        cases.push((good, "pass"));
        for control in controls {
            let mut bad = projection(family, Some(control));
            bad.name = format!("Broken{family}{control}");
            cases.push((bad, "safety"));
        }
    }
    cases
}
fn projection(family: &str, broken: Option<&str>) -> Model {
    if family == "DreamReadFeedback" {
        // One original operation, two request digests and callers; revocation
        // A0/B1/A2 cannot revive an offer. Journal commit is atomic; a delayed
        // or lost reply permits retry, never another report or policy effect.
        let mut m = model(
            family,
            false,
            &[
                ("offered", 1, 0),
                ("foreign", 1, 0),
                ("evidence", 1, 0),
                ("epoch", 2, 0),
                ("request", 1, 0),
                ("claimed", 1, 0),
                ("reports", 2, 0),
                ("policy", 1, 0),
                ("bad", 1, 0),
            ],
        );
        action(&mut m, "Offer", eq(5, 0), &[(0, Int(1))]);
        action(&mut m, "ForeignActor", Bool(true), &[(1, Int(1))]);
        action(&mut m, "ForeignEvidence", Bool(true), &[(2, Int(1))]);
        action(
            &mut m,
            "Invalidate",
            Var(3).lt(Int(2)),
            &[(3, Var(3).plus(Int(1)))],
        );
        action(&mut m, "ChangeBody", Bool(true), &[(4, Int(1))]);
        let mut guards = vec![eq(0, 1)];
        for (control, index) in [
            ("ForeignOffer", 1),
            ("ForeignEvidence", 2),
            ("ABA", 3),
            ("Digest", 4),
        ] {
            if broken != Some(control) {
                guards.push(eq(index, 0));
            }
        }
        let invalid = eq(1, 1).or(eq(2, 1)).or(Int(0).lt(Var(3))).or(eq(4, 1));
        let duplicate = if broken == Some("Duplicate") {
            eq(6, 1).ite(Int(2), Int(1))
        } else {
            Int(1)
        };
        action(
            &mut m,
            "ReportOrRetryLostResponse",
            all(guards),
            &[
                (5, Int(1)),
                (6, duplicate),
                (7, Int(i64::from(broken == Some("Promote")))),
                (8, invalid.ite(Int(1), Var(8))),
            ],
        );
        safe(&mut m, "OnlySameOfferEvidenceAndOriginalRequest", eq(8, 0));
        safe(
            &mut m,
            "OriginalOperationAtMostOneReport",
            Var(6).le(Int(1)),
        );
        safe(&mut m, "ReportedOnlyNeverPolicyAuthority", eq(7, 0));
        m
    } else if family == "DreamReadOwnership" {
        // epoch0=A,1=B,2=A: equal label after ABA is not equal incarnation.
        let mut m = model(
            family,
            false,
            &[
                ("foreign", 1, 0),
                ("epoch", 2, 0),
                ("presented", 1, 0),
                ("bad", 1, 0),
            ],
        );
        action(&mut m, "ForeignCaller", eq(2, 0), &[(0, Int(1))]);
        action(
            &mut m,
            "Invalidate",
            all([eq(2, 0), Var(1).lt(Int(2))]),
            &[(1, Var(1).plus(Int(1)))],
        );
        let owner = if broken == Some("Foreign") {
            Bool(true)
        } else {
            eq(0, 0)
        };
        let epoch = if broken == Some("ABA") {
            eq(1, 0).or(eq(1, 2))
        } else {
            eq(1, 0)
        };
        action(
            &mut m,
            "Present",
            all([eq(2, 0), owner, epoch]),
            &[
                (2, Int(1)),
                (3, eq(0, 1).or(Int(0).lt(Var(1))).ite(Int(1), Int(0))),
            ],
        );
        safe(&mut m, "OnlyCurrentOwnIncarnation", eq(3, 0));
        m
    } else if family == "DreamReadSnapshot" {
        let mut m = model(
            family,
            false,
            &[
                ("revision", 2, 0),
                ("query", 1, 0),
                ("presented", 1, 0),
                ("mixed", 1, 0),
            ],
        );
        action(
            &mut m,
            "Rollover",
            all([eq(2, 0), Var(0).lt(Int(2))]),
            &[(0, Var(0).plus(Int(1)))],
        );
        action(&mut m, "ChangeQuery", eq(2, 0), &[(1, Int(1))]);
        let guard = if broken.is_some() {
            Bool(true)
        } else {
            all([eq(0, 0), eq(1, 0)])
        };
        action(
            &mut m,
            "CursorPage",
            all([eq(2, 0), guard]),
            &[
                (2, Int(1)),
                (3, Int(0).lt(Var(0)).or(eq(1, 1)).ite(Int(1), Int(0))),
            ],
        );
        safe(&mut m, "CursorNeverMixesRevisionOrQuery", eq(3, 0));
        m
    } else {
        // exact closed completion; no read-triggered preparation transition.
        let mut m = model(
            family,
            false,
            &[
                ("closed", 1, 0),
                ("settled", 1, 0),
                ("prepared", 1, 0),
                ("lesson", 1, 0),
                ("row_bound", 1, 1),
                ("cluster_origin", 1, 0),
                ("members_honest", 1, 1),
                ("reserved", 1, 1),
                ("presented", 1, 0),
                ("bad", 1, 0),
                ("foreground", 1, 0),
                ("unsafe_preparation", 1, 0),
            ],
        );
        action(&mut m, "AuthoritativeClose", eq(8, 0), &[(0, Int(1))]);
        action(&mut m, "Settle", all([eq(0, 1), eq(8, 0)]), &[(1, Int(1))]);
        action(&mut m, "Foreground", eq(8, 0), &[(10, Int(1))]);
        action(
            &mut m,
            "PrepareCompleted",
            all([
                eq(0, 1),
                eq(1, 1),
                eq(7, 1),
                eq(8, 0),
                if broken == Some("MissingPause") {
                    Bool(true)
                } else {
                    eq(10, 0)
                },
            ]),
            &[(2, Int(1)), (11, eq(10, 1).ite(Int(1), Int(0)))],
        );
        safe(&mut m, "PreparationNeverRunsWhileForeground", eq(11, 0));
        for (name, index, value) in [
            ("FakeLesson", 3, 1),
            ("RowBinding", 4, 0),
            ("ClusterOrigin", 5, 1),
            ("MemberCompleteness", 6, 0),
            ("Reservation", 7, 0),
        ] {
            action(
                &mut m,
                name,
                all([eq(2, 1), eq(8, 0)]),
                &[(index, Int(value))],
            );
        }
        let mut guards = vec![eq(0, 1), eq(1, 1), eq(2, 1), eq(8, 0)];
        for (name, index, value) in [
            ("FakeLesson", 3, 0),
            ("RowBinding", 4, 1),
            ("ClusterOrigin", 5, 0),
            ("MemberCompleteness", 6, 1),
            ("Reservation", 7, 1),
        ] {
            if broken != Some(name) {
                guards.push(eq(index, value));
            }
        }
        let bad = eq(3, 1).or(eq(4, 0)).or(eq(5, 1)).or(eq(6, 0)).or(eq(7, 0));
        action(
            &mut m,
            "PresentObservation",
            all(guards),
            &[(8, Int(1)), (9, bad.ite(Int(1), Int(0)))],
        );
        safe(&mut m, "OnlyPreparedBoundReservedObservations", eq(9, 0));
        m
    }
}
