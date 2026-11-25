// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A pinned startup mount and one additional slot contested by two roots.
use super::*;

pub(super) fn workspace_retention(broken: bool) -> Model {
    const VACANT: i64 = 0;
    const MOUNTING: i64 = 1;
    const BOUND: i64 = 2;
    const IDLE: i64 = 3;
    const RETIRING: i64 = 4;
    const ABANDONED_MOUNT: i64 = 5;

    let mut m = model(
        "WorkspaceRetention",
        broken,
        &[
            ("first", ABANDONED_MOUNT, VACANT),
            ("second", ABANDONED_MOUNT, VACANT),
        ],
    );
    for (slot, other) in [(0, 1), (1, 0)] {
        action(
            &mut m,
            &format!("Reserve{slot}"),
            all([
                eq(slot, VACANT),
                if broken {
                    Expr::Bool(true)
                } else {
                    eq(other, VACANT)
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
            ("EvictIdleMount", IDLE, RETIRING),
            ("FinishRetirement", RETIRING, VACANT),
        ] {
            action(
                &mut m,
                &format!("{name}{slot}"),
                eq(slot, before),
                &[(slot, Int(after))],
            );
        }
    }
    safe(
        &mut m,
        "SlotsBounded",
        implies(ne(0, VACANT), eq(1, VACANT)),
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
            let checked = model.check(100).unwrap();
            assert_eq!(checked.violations.is_empty(), expected == "pass");
            if expected != "pass" {
                assert!(
                    checked
                        .violations
                        .iter()
                        .any(|v| v.property == "SlotsBounded")
                );
            }
            for (name, text) in bboxy450_tlc::build::artifacts(&model).unwrap() {
                let path = dir.join(name);
                if std::env::var_os("BBOXY_UPDATE_MODELS").is_some() {
                    std::fs::write(&path, &text).unwrap();
                }
                assert_eq!(std::fs::read_to_string(path).unwrap(), text);
            }
        }
    }
}
