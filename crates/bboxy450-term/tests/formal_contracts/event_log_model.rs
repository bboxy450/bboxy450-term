// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Three rows: historical prefix 1, checkpoint 2, suffix 3. Hash/signature
//! rejection is abstracted as a bad-row identity, not a cryptographic proof.
use super::{action, all, eq, implies, model, safe};
use bboxy450_tlc::{
    Expr::{Bool, Int, Var},
    Model, Response, Variable,
};

fn checkpoint(broken: bool) -> Model {
    let mut m = model(
        "EventLogCheckpoint",
        broken,
        &[
            ("has_checkpoint", 1, 0),
            ("tampered", 3, 0),
            ("phase", 2, 0), // 0 opening, 1 ready, 2 refused
            ("audit_seq", 3, 0),
            ("failure", 1, 0),
            ("reported", 1, 0),
            ("checkpoint_seq", 4, 2),
            ("tip", 4, 3),
        ],
    );
    m.variables[0] = Variable::choice("has_checkpoint", 0, 1, &[0, 1]);
    m.variables[1] = Variable::choice("tampered", 0, 3, &[0, 1, 2, 3]);
    // An upgrade verifies bounded rows 2..3; an existing checkpoint verifies
    // its hash at 2 and then fully verifies row 3. Only the checkpoint-hash
    // check is omitted in the negative control, never suffix verification.
    let reject = eq(1, 3).or(eq(1, 2).and(if broken { eq(0, 0) } else { Bool(true) }));
    action(
        &mut m,
        "RejectOpen",
        eq(2, 0).and(reject.clone()),
        &[(2, Int(2))],
    );
    action(
        &mut m,
        "VerifyTailAndReady",
        eq(2, 0).and(reject.negate()),
        &[(2, Int(1)), (6, Int(3))],
    );
    action(
        &mut m,
        "AppendLocallySigned",
        all([eq(2, 1), eq(7, 3)]),
        &[(6, Int(4)), (7, Int(4))],
    );
    let mut audit = Vec::new();
    for seq in 1..=3 {
        audit.push(m.actions.len());
        action(
            &mut m,
            &format!("AuditRow{seq}"),
            all([eq(2, 1), eq(3, seq - 1), eq(4, 0)]),
            &[(3, Int(seq)), (4, eq(1, seq).ite(Int(1), Int(0)))],
        );
    }
    audit.push(m.actions.len());
    action(
        &mut m,
        "ReportHistoryFailure",
        all([eq(4, 1), eq(5, 0)]),
        &[(5, Int(1))],
    );
    // A living process eventually schedules its audit/report, independently
    // of appends. Process exit/restart and diagnostic socket delivery are not
    // fair here; every subsequent process schedules a new genesis audit.
    m.weak_fair.push(audit);
    safe(
        &mut m,
        "TamperedCheckpointOrSuffixNeverReady",
        implies(eq(2, 1), Var(1).lt(Int(2))),
    );
    safe(&mut m, "CheckpointNeverAheadOfTip", Var(6).le(Var(7)));
    safe(
        &mut m,
        "ReportsHaveAnObservedFailure",
        implies(eq(5, 1), eq(4, 1)),
    );
    m.responses.push(Response {
        name: "HistoricalTamperingEventuallyReported".into(),
        request: eq(2, 1).and(eq(1, 1)),
        response: eq(5, 1),
    });
    m
}

// Two startup openers observe versions independently. Only the writer recheck
// permits a step; SQLite commits its effect and marker together. A single
// interruption discards in-flight observations, never committed state.
fn migration(broken: bool) -> Model {
    let mut m = model(
        "DataMigration",
        broken,
        &[
            ("version", 2, 0),
            ("effects", 2, 0),
            ("first", 3, 3),
            ("second", 3, 3),
            ("interrupted", 1, 0),
        ],
    );
    for (slot, name) in [(2, "First"), (3, "Second")] {
        action(
            &mut m,
            &format!("Observe{name}"),
            eq(slot, 3).and(Var(0).lt(Int(2))),
            &[(slot, Var(0))],
        );
        action(
            &mut m,
            &format!("Commit{name}"),
            Var(slot).eq(Var(0)).and(Var(0).lt(Int(2))),
            &[
                (0, Var(0).plus(Int(1))),
                (1, if broken { Var(1) } else { Var(1).plus(Int(1)) }),
                (slot, Int(3)),
            ],
        );
        action(
            &mut m,
            &format!("Recheck{name}"),
            Var(slot).lt(Var(0)),
            &[(slot, Int(3))],
        );
    }
    action(
        &mut m,
        "InterruptOrSqlFailure",
        eq(4, 0),
        &[(2, Int(3)), (3, Int(3)), (4, Int(1))],
    );
    safe(&mut m, "MarkerAndEffectsCommitTogether", Var(0).eq(Var(1)));
    m.weak_fair.push(vec![0, 1, 2, 3, 4, 5]);
    m.responses.push(Response {
        name: "SupportedSchemaEventuallyCurrent".into(),
        request: Var(0).lt(Int(2)),
        response: eq(0, 2),
    });
    m
}

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (checkpoint(false), "pass"),
        (checkpoint(true), "safety"),
        (migration(false), "pass"),
        (migration(true), "safety"),
    ]
}
