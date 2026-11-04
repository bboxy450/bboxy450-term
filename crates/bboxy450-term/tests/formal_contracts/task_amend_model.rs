// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One task, two amendments, two verify commands (including A → B → A).
use super::*;

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    let mut cases = Vec::new();
    for control in ["Worker", "Delegator", "Terminal", "Verify", "Etag"] {
        let name = if control == "Worker" {
            "TaskAmendment".to_owned()
        } else {
            format!("TaskAmendment{control}")
        };
        let mut passing = amendment(None);
        passing.name = name.clone();
        let mut broken = amendment(Some(control));
        broken.name = format!("Broken{name}");
        cases.extend([(passing, "pass"), (broken, "safety")]);
    }
    cases
}

fn amendment(control: Option<&str>) -> Model {
    // Actor: worker=0, exact delegator=1, same-swarm Oracle=2, foreign Oracle=3.
    // Status: open/claimed/running/reported/rejected/blocked/accepted/closed=0..7.
    // Body and paths are abstract payloads. Publication with the final digest
    // check is atomic; real source races, signatures and persistence use API tests.
    let mut m = model(
        "TaskAmendment",
        false,
        &[
            ("actor", 3, 1),
            ("status", 7, 0),
            ("version", 2, 0),
            ("verify_revision", 2, 0),
            ("command", 1, 0),
            ("report_revision", 2, 0),
            ("reported", 1, 0),
            ("last_actor", 3, 1),
            ("last_status", 7, 0),
            ("last_etag_current", 1, 1),
            ("pending_wake", 1, 0),
            ("report_command", 1, 0),
        ],
    );
    for actor in 0..=3 {
        action(
            &mut m,
            &format!("Caller{actor}"),
            Bool(true),
            &[(0, Int(actor))],
        );
    }
    for (name, status) in [
        ("Claim", 1),
        ("Run", 2),
        ("Reject", 4),
        ("Block", 5),
        ("Close", 7),
    ] {
        action(&mut m, name, Var(1).lt(Int(6)), &[(1, Int(status))]);
    }
    action(
        &mut m,
        "ReportCurrentVerify",
        Var(1).lt(Int(6)),
        &[(1, Int(3)), (5, Var(3)), (6, Int(1)), (11, Var(4))],
    );
    action(
        &mut m,
        "Accept",
        all([
            eq(1, 3),
            eq(6, 1),
            Var(4).eq(Var(11)),
            if control == Some("Verify") {
                Bool(true)
            } else {
                Var(3).eq(Var(5))
            },
        ]),
        &[(1, Int(6))],
    );
    let owner = match control {
        Some("Worker") => eq(0, 1).or(eq(0, 0)),
        Some("Delegator") => eq(0, 1).or(eq(0, 2)),
        _ => eq(0, 1),
    };
    let live = if control == Some("Terminal") {
        Bool(true)
    } else {
        Var(1).lt(Int(6))
    };
    for verify in [false, true] {
        let mut updates = vec![
            (2, Var(2).plus(Int(1))),
            (7, Var(0)),
            (8, Var(1)),
            (9, Int(1)),
            (10, Int(1)),
        ];
        if verify {
            updates.extend([(3, Var(3).plus(Int(1))), (4, Int(1).minus(Var(4)))]);
        }
        action(
            &mut m,
            if verify {
                "AmendVerify"
            } else {
                "AmendBodyOrPaths"
            },
            all([owner.clone(), live.clone(), Var(2).lt(Int(2))]),
            &updates,
        );
    }
    // A delayed request names the version before a completed amendment.
    if control == Some("Etag") {
        action(
            &mut m,
            "PublishStaleEtag",
            all([owner, live, ne(2, 0)]),
            &[(7, Var(0)), (8, Var(1)), (9, Int(0)), (10, Int(1))],
        );
    }
    let deliver = m.actions.len();
    action(&mut m, "DeliverAmendmentWake", eq(10, 1), &[(10, Int(0))]);
    safe(&mut m, "OnlyExactDelegatorAmends", eq(7, 1));
    safe(&mut m, "TerminalBriefFrozen", Var(8).lt(Int(6)));
    safe(&mut m, "StaleEtagHasNoEffect", eq(9, 1));
    safe(
        &mut m,
        "AcceptanceUsesCurrentVerifyRevision",
        implies(eq(1, 6), Var(3).eq(Var(5))),
    );
    // Healthy admitted receiver/delivery is weakly fair. Identity recovery,
    // native composer readiness and physical deadlines are not assumed here.
    m.weak_fair.push(vec![deliver]);
    m.responses.push(Response {
        name: "AmendmentEventuallyDelivered".into(),
        request: eq(10, 1),
        response: eq(10, 0),
    });
    m
}
