// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One parent owner accumulates measured CPU debt before transferring work.
use bboxy450_tlc::Expr::{Int, Var};
use bboxy450_tlc::build::{action, all, eq, implies, model, ne, safe};
use bboxy450_tlc::{Model, Response};

pub(super) fn cases() -> [(Model, &'static str); 3] {
    [
        (project("C1MeasuredWorkDebt", false, false), "pass"),
        (project("BrokenC1MeasuredWorkDebt", true, false), "safety"),
        (
            project("BrokenC1MeasuredWorkDebtStepFloor", false, true),
            "safety",
        ),
    ]
}

fn project(name: &str, skip_payment: bool, step_floor: bool) -> Model {
    const STEP: usize = 0;
    const OWED: usize = 1;
    const PAID: usize = 2;
    const PHASE: usize = 3;
    const STEPS: i64 = 4;
    // One tick is a quarter millisecond in this illustrative finite window.
    const REST: i64 = 36;
    const DEADLINE: i64 = 100;
    let mut m = model(
        name,
        false,
        &[
            ("steps", STEPS, 0),
            ("owed", REST, 0),
            ("paidRest", STEPS * REST, 0),
            ("phase", 2, 0), // working, transferred, deadline
        ],
    );
    let spent = Var(STEP).plus(Var(PAID));
    let open = eq(PHASE, 0).and(spent.clone().lt(Int(DEADLINE)));
    let batch_due = Int(REST).le(Var(OWED));
    // Successful execution owns the same bounded work in all three cases.
    action(
        &mut m,
        "MeasureOneStep",
        all([open.clone(), ne(STEP, STEPS), batch_due.clone().negate()]),
        &[
            (STEP, Var(STEP).plus(Int(1))),
            (OWED, Var(OWED).plus(Int(if step_floor { REST } else { 9 }))),
        ],
    );
    action(
        &mut m,
        "PayObservedRest",
        all([open.clone(), ne(OWED, 0), batch_due.or(eq(STEP, STEPS))]),
        &[(OWED, Int(0)), (PAID, Var(PAID).plus(Var(OWED)))],
    );
    let finished = open.and(eq(STEP, STEPS));
    action(
        &mut m,
        "TransferToChild",
        if skip_payment {
            finished
        } else {
            finished.and(eq(OWED, 0))
        },
        &[(PHASE, Int(1))],
    );
    action(
        &mut m,
        "ReachDeadline",
        eq(PHASE, 0).and(Int(DEADLINE).le(spent)),
        &[(PHASE, Int(2))],
    );
    safe(
        &mut m,
        "TransferPaysMeasuredCpu",
        implies(eq(PHASE, 1), eq(OWED, 0).and(Int(9 * STEPS).le(Var(PAID)))),
    );
    safe(&mut m, "FiniteWorkFitsDeadline", ne(PHASE, 2));
    m.weak_fair.push(vec![0, 1, 2, 3]);
    m.responses.push(Response {
        name: "WorkEventuallyLeavesOwner".into(),
        request: eq(PHASE, 0),
        response: ne(PHASE, 0),
    });
    m
}

#[cfg(test)]
mod tests {
    #[test]
    fn measured_debt_and_both_broken_controls() {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("models");
        for (model, expected) in super::cases() {
            let report = model.check(100_000).unwrap();
            let violated = match model.name.as_str() {
                "BrokenC1MeasuredWorkDebt" => Some("TransferPaysMeasuredCpu"),
                "BrokenC1MeasuredWorkDebtStepFloor" => Some("FiniteWorkFitsDeadline"),
                _ => None,
            };
            assert_eq!(report.violations.is_empty(), expected == "pass");
            if let Some(property) = violated {
                assert!(report.violations.iter().any(|v| v.property == property));
            }
            for (name, content) in bboxy450_tlc::build::artifacts(&model).unwrap() {
                let path = directory.join(name);
                if std::env::var_os("BBOXY_UPDATE_MODELS").is_some() {
                    std::fs::write(&path, &content).unwrap();
                }
                assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
            }
        }
    }
}
