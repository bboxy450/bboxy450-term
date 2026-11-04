// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Record-number cache freshness and paced artifact projection progress.
use super::{action, all, eq, implies, model, safe};
use bboxy450_tlc::{
    Expr::{Int, Var},
    Model, Response,
};

// One record file whose number toggles A -> B -> A -> B. Each edit advances
// its stamp (inode/size/mtime/ctime); the broken control's stamp is blind to a
// same-length edit. A lookup stats, then reads on a miss; edits interleave.
fn record_number_cache(b: bool) -> Model {
    let mut m = model(
        "RecordNumberCache",
        b,
        &[
            ("num", 1, 0),
            ("gen", 3, 0),
            ("pc", 1, 0),
            ("seen_gen", 3, 0),
            ("seen_num", 1, 0),
            ("cached", 2, 0),
            ("cached_gen", 3, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "Edit",
        Var(1).lt(Int(3)),
        &[
            (0, Int(1).minus(Var(0))),
            (1, if b { Var(1) } else { Var(1).plus(Int(1)) }),
        ],
    );
    action(
        &mut m,
        "Stat",
        eq(2, 0),
        &[(2, Int(1)), (3, Var(1)), (4, Var(0))],
    );
    let hit = all([eq(2, 1), eq(5, 0).negate(), Var(6).eq(Var(3))]);
    action(
        &mut m,
        "Hit",
        hit.clone(),
        &[
            (2, Int(0)),
            (
                7,
                Var(5).minus(Int(1)).eq(Var(4)).negate().ite(Int(1), Var(7)),
            ),
        ],
    );
    // A miss parses the file now; an edit after the stat leaves a stale label.
    action(
        &mut m,
        "Miss",
        all([eq(2, 1), hit.negate()]),
        &[(2, Int(0)), (5, Var(0).plus(Int(1))), (6, Var(3))],
    );
    safe(&mut m, "HitReturnsTheStattedNumber", eq(7, 0));
    safe(
        &mut m,
        "CurrentStampNamesCurrentNumber",
        implies(
            all([eq(5, 0).negate(), Var(6).eq(Var(1))]),
            Var(5).minus(Int(1)).eq(Var(0)),
        ),
    );
    m
}

// Bounded arrivals; a projection turn rests before the next. The broken
// control renews the rest on every scheduler step and never projects again.
fn projection_pacing(b: bool) -> Model {
    let mut m = model(
        "ProjectionPacing",
        b,
        &[("backlog", 2, 0), ("arrivals", 2, 0), ("rest", 2, 0)],
    );
    action(
        &mut m,
        "Arrive",
        Var(1).lt(Int(2)),
        &[(0, Var(0).plus(Int(1))), (1, Var(1).plus(Int(1)))],
    );
    action(
        &mut m,
        "Project",
        all([eq(2, 0), Int(0).lt(Var(0))]),
        &[(0, Var(0).minus(Int(1))), (2, Int(2))],
    );
    action(
        &mut m,
        "RestElapses",
        Int(0).lt(Var(2)),
        &[(2, Var(2).minus(Int(1)))],
    );
    action(
        &mut m,
        "StepWhileResting",
        all([Int(0).lt(Var(2)), Int(0).lt(Var(0))]),
        &[(2, if b { Int(2) } else { Var(2) })],
    );
    m.responses.push(Response {
        name: "BacklogEventuallyProjected".into(),
        request: Int(0).lt(Var(0)),
        response: eq(0, 0),
    });
    m.weak_fair.extend([vec![1], vec![2], vec![3]]);
    m
}

pub fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (record_number_cache(false), "pass"),
        (record_number_cache(true), "safety"),
        (projection_pacing(false), "pass"),
        (projection_pacing(true), "liveness"),
        (jsonl_repair(false), "pass"),
        (jsonl_repair(true), "safety"),
    ]
}

// Three retained rows, one page per owner turn. Cursor survival across restart
// and eventual publication are separate from OS durability and signature checks.
pub(super) fn jsonl_repair(b: bool) -> Model {
    let mut m = model(
        "JsonlRepair",
        b,
        &[("remaining", 3, 3), ("debt", 2, 0), ("published", 1, 0)],
    );
    action(
        &mut m,
        "RepairPage",
        all([
            Int(0).lt(Var(0)),
            if b { Int(0).eq(Int(0)) } else { eq(1, 0) },
        ]),
        &[
            (0, Var(0).minus(Int(1))),
            (1, Var(1).lt(Int(2)).ite(Var(1).plus(Int(1)), Var(1))),
        ],
    );
    action(&mut m, "OwnerTurn", Int(0).lt(Var(1)), &[(1, Int(0))]);
    action(&mut m, "Publish", all([eq(0, 0), eq(2, 0)]), &[(2, Int(1))]);
    safe(&mut m, "RepairYieldsToOwner", Var(1).lt(Int(2)));
    safe(
        &mut m,
        "PublishedOnlyWhenComplete",
        implies(eq(2, 1), eq(0, 0)),
    );
    m.responses.push(Response {
        name: "RepairEventuallyPublished".into(),
        request: Int(0).eq(Int(0)),
        response: eq(2, 1),
    });
    m.weak_fair.extend([vec![0], vec![1], vec![2]]);
    m
}

#[test]
fn jsonl_repair_yields_and_publishes() {
    assert!(
        jsonl_repair(false)
            .check(1000)
            .unwrap()
            .violations
            .is_empty()
    );
    let broken = jsonl_repair(true).check(1000).unwrap();
    assert!(
        broken
            .violations
            .iter()
            .any(|v| v.property == "RepairYieldsToOwner")
    );
}
