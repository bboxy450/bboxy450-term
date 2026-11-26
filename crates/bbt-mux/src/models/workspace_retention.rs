// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One pinned startup mount and one or two discretionary slots for two roots.
use super::*;

const VACANT: i64 = 0;
const MOUNTING: i64 = 1;
const BOUND: i64 = 2;
const IDLE: i64 = 3;
const RETIRING: i64 = 4;
const ABANDONED_MOUNT: i64 = 5;
const PRESSURE_RETIRING: i64 = 6;
const SLOTS: usize = 2;
const SAFETY: &str = "SlotsBoundedAndSinglePressureRetirement";

pub(super) fn workspace_retention(broken: bool) -> Model {
    retention_model(broken, false)
}

fn retention_model(break_eviction: bool, break_reservation: bool) -> Model {
    let mut m = model(
        "WorkspaceRetention",
        break_eviction || break_reservation,
        &[
            ("first", PRESSURE_RETIRING, VACANT),
            ("second", PRESSURE_RETIRING, VACANT),
            ("slots", 2, 1),
        ],
    );
    // Also explore the pressure case with two already-mounted idle roots.
    action(
        &mut m,
        "StartTwoIdleRoots",
        all([eq(SLOTS, 1), eq(0, VACANT), eq(1, VACANT)]),
        &[(SLOTS, Int(2)), (0, Int(IDLE)), (1, Int(IDLE))],
    );
    for (slot, other) in [(0, 1), (1, 0)] {
        action(
            &mut m,
            &format!("Reserve{slot}"),
            all([
                eq(slot, VACANT),
                if break_reservation {
                    Expr::Bool(true)
                } else {
                    eq(SLOTS, 2).or(eq(other, VACANT))
                },
            ]),
            &[(slot, Int(MOUNTING))],
        );
        for (name, before, after) in [
            ("PublishMount", MOUNTING, BOUND),
            ("AbandonMount", MOUNTING, ABANDONED_MOUNT),
            ("DiscardMount", ABANDONED_MOUNT, RETIRING),
            ("ForgetBinding", BOUND, IDLE),
            ("CloseLastBinding", BOUND, RETIRING),
            ("ReuseIdleMount", IDLE, BOUND),
        ] {
            action(
                &mut m,
                &format!("{name}{slot}"),
                eq(slot, before),
                &[(slot, Int(after))],
            );
        }
        action(
            &mut m,
            &format!("EvictIdleMount{slot}"),
            all([
                eq(slot, IDLE),
                eq(SLOTS, 1).or(ne(other, VACANT)),
                if break_eviction {
                    Expr::Bool(true)
                } else {
                    all([ne(other, RETIRING), ne(other, PRESSURE_RETIRING)])
                },
            ]),
            &[(slot, Int(PRESSURE_RETIRING))],
        );
        action(
            &mut m,
            &format!("FinishRetirement{slot}"),
            eq(slot, RETIRING).or(eq(slot, PRESSURE_RETIRING)),
            &[(slot, Int(VACANT))],
        );
    }
    safe(
        &mut m,
        SAFETY,
        all([
            implies(all([eq(SLOTS, 1), ne(0, VACANT)]), eq(1, VACANT)),
            implies(eq(0, PRESSURE_RETIRING), ne(1, PRESSURE_RETIRING)),
        ]),
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_retention_model_and_broken_control() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("models");
        for (model, expected) in bboxy450_tlc::build::family(workspace_retention) {
            let checked = model.check(128).unwrap();
            assert_eq!(checked.violations.is_empty(), expected == "pass");
            if expected != "pass" {
                assert!(checked.violations.iter().any(|violation| {
                    let state = &checked.states[*violation.prefix.last().unwrap()];
                    violation.property == SAFETY
                        && state[SLOTS] == 2
                        && state[0] == PRESSURE_RETIRING
                        && state[1] == PRESSURE_RETIRING
                }));
            }
            for (name, text) in bboxy450_tlc::build::artifacts(&model).unwrap() {
                let path = dir.join(name);
                if std::env::var_os("BBOXY_UPDATE_MODELS").is_some() {
                    std::fs::write(&path, &text).unwrap();
                }
                assert_eq!(std::fs::read_to_string(path).unwrap(), text);
            }
        }

        // The original reservation fault still runs independently.
        let checked = retention_model(false, true).check(128).unwrap();
        assert!(checked.violations.iter().any(|violation| {
            let state = &checked.states[*violation.prefix.last().unwrap()];
            violation.property == SAFETY
                && state[SLOTS] == 1
                && state[0] != VACANT
                && state[1] != VACANT
        }));
    }
}
