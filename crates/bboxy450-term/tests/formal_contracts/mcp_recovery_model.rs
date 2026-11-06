// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Two admitted Oracles race for one reservation; one orphan close checks ownership.
use super::*;

fn reservation(broken: bool) -> Model {
    // Final source CAS/publication is atomic. Captured revisions and root
    // generations survive delayed completions; changing either invalidates them.
    let mut m = model(
        "TaskReservation",
        broken,
        &[
            ("owner", 2, 0),
            ("revision", 2, 0),
            ("blank", 1, 1),
            ("pending_a", 1, 0),
            ("pending_b", 1, 0),
            ("captured_a", 2, 0),
            ("captured_b", 2, 0),
            ("root", 2, 0),
            ("root_a", 2, 0),
            ("root_b", 2, 0),
            ("writes", 2, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "ChangeRoot",
        Var(7).lt(Int(2)),
        &[(7, Var(7).plus(Int(1)))],
    );
    action(
        &mut m,
        "PopulateElsewhere",
        all([eq(0, 0), Var(1).lt(Int(2))]),
        &[(1, Var(1).plus(Int(1))), (2, Int(0))],
    );
    for (name, actor, pending, captured, root) in [("A", 1, 3, 5, 8), ("B", 2, 4, 6, 9)] {
        action(
            &mut m,
            &format!("Prepare{name}"),
            all([eq(0, 0), eq(2, 1), eq(pending, 0)]),
            &[(pending, Int(1)), (captured, Var(1)), (root, Var(7))],
        );
        let fresh = all([eq(0, 0), eq(2, 1), Var(1).eq(Var(captured))]);
        action(
            &mut m,
            &format!("Initialize{name}"),
            all([
                eq(pending, 1),
                Var(7).eq(Var(root)),
                Var(1).lt(Int(2)),
                Var(10).lt(Int(2)),
                if broken { Bool(true) } else { fresh.clone() },
            ]),
            &[
                (0, Int(actor)),
                (1, Var(1).plus(Int(1))),
                (2, Int(0)),
                (pending, Int(0)),
                (10, Var(10).plus(Int(1))),
                (11, Int(1).minus(Var(2))),
            ],
        );
    }
    no_bad(&mut m, 11);
    safe(&mut m, "OneInitializer", Var(10).lt(Int(2)));
    m
}

fn orphan_close(broken: bool) -> Model {
    // The logic owner does not yield between the authoritative absence reply
    // and registry CAS. OS/process effects and persistence use API witnesses.
    let mut m = model(
        "OrphanClose",
        broken,
        &[
            ("owned", 1, 1),
            ("view", 1, 0),
            ("caller_current", 1, 1),
            ("target_current", 1, 1),
            ("closed", 1, 0),
            ("bad", 1, 0),
        ],
    );
    for (i, label) in [
        (0, "Ownership"),
        (1, "Viewport"),
        (2, "Caller"),
        (3, "Generation"),
    ] {
        action(
            &mut m,
            &format!("Change{label}"),
            eq(4, 0),
            &[(i, Int(1).minus(Var(i)))],
        );
    }
    action(
        &mut m,
        "CloseOrphan",
        all([
            eq(4, 0),
            eq(1, 0),
            eq(2, 1),
            eq(3, 1),
            if broken { Bool(true) } else { eq(0, 0) },
        ]),
        &[(4, Int(1)), (5, Var(0))],
    );
    no_bad(&mut m, 5);
    m
}

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (reservation(false), "pass"),
        (reservation(true), "safety"),
        (orphan_close(false), "pass"),
        (orphan_close(true), "safety"),
    ]
}
