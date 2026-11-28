// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Credit copied, adapted and translated reuse accurately.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! An exact exited binding keeps one final source read before retiring.
use bboxy450_tlc::Expr::Int;
use bboxy450_tlc::build::{action, all, eq, implies, model, ne, safe};
use bboxy450_tlc::{Model, Response};

pub(super) fn cases() -> [(Model, &'static str); 2] {
    [
        (closed_exit_follower(false), "pass"),
        (closed_exit_follower(true), "liveness"),
    ]
}

fn closed_exit_follower(broken: bool) -> Model {
    const CLOSED: usize = 0;
    const ABSENT: usize = 1;
    const STOPPED: usize = 2;
    const CURRENT: usize = 3;
    const DISPOSITION: usize = 4;
    let mut m = model(
        "TermDreamC1ClosedExitFollower",
        broken,
        &[
            ("durableClose", 1, 0),
            ("processAbsent", 1, 0),
            ("stopRequested", 1, 0),
            ("bindingCurrent", 1, 1),
            ("disposition", 2, 0), // following, sealed, returned without seal
        ],
    );
    action(
        &mut m,
        "WriteDurableClose",
        all([eq(CLOSED, 0), eq(ABSENT, 0)]),
        &[(CLOSED, Int(1))],
    );
    action(
        &mut m,
        "ExactProcessExit",
        eq(ABSENT, 0),
        &[(ABSENT, Int(1))],
    );
    action(
        &mut m,
        "StopBeforeSeal",
        all([eq(ABSENT, 1), eq(STOPPED, 0)]),
        &[
            (STOPPED, Int(1)),
            (DISPOSITION, Int(if broken { 2 } else { 0 })),
        ],
    );
    action(
        &mut m,
        "ChangeBinding",
        eq(CURRENT, 1),
        &[(CURRENT, Int(0))],
    );
    let valid = all([eq(CLOSED, 1), eq(ABSENT, 1), eq(CURRENT, 1)]);
    let pending = all([eq(STOPPED, 1), eq(DISPOSITION, 0)]);
    let final_pass = m.actions.len();
    action(
        &mut m,
        "DrainAndSeal",
        pending.clone().and(valid.clone()),
        &[(DISPOSITION, Int(1))],
    );
    action(
        &mut m,
        "DrainAndRefuse",
        pending.and(valid.clone().negate()),
        &[(DISPOSITION, Int(2))],
    );
    safe(
        &mut m,
        "SealRequiresDurableCloseAndExit",
        implies(eq(DISPOSITION, 1), all([eq(CLOSED, 1), eq(ABSENT, 1)])),
    );
    m.weak_fair.push(vec![final_pass, final_pass + 1]);
    m.responses.push(Response {
        name: "ClosedExitEventuallySeals".into(),
        request: eq(STOPPED, 1).and(valid),
        response: eq(DISPOSITION, 1).or(ne(CURRENT, 1)),
    });
    m
}

#[cfg(test)]
mod tests {
    #[test]
    fn final_follower_pass_and_immediate_stop_control() {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("models");
        for (model, expected) in super::cases() {
            let report = model.check(100_000).unwrap();
            assert_eq!(report.violations.is_empty(), expected == "pass");
            if expected == "liveness" {
                assert!(
                    report
                        .violations
                        .iter()
                        .all(|v| v.property == "ClosedExitEventuallySeals")
                );
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
