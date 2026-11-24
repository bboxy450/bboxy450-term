// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Finite MCP authority, pointer-wake and absent-process close contracts.
use bboxy450_tlc::Expr::{Int, Var};
use bboxy450_tlc::build::{action, all, eq, implies, model, ne, no_bad, safe};
use bboxy450_tlc::{Expr, Model};

pub fn live_catalogue(broken: bool) -> Model {
    // epoch is monotonic across role A → B → A; every observation uses the
    // current grant, rather than the bridge's launch-time authority.
    let mut m = model(
        "LiveCatalogue",
        broken,
        &[
            ("epoch", 2, 0),
            ("role", 1, 0),
            ("observed", 2, 0),
            ("notified", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "ChangeRole",
        Var(0).lt(Int(2)),
        &[
            (0, Var(0).plus(Int(1))),
            (1, Int(1).minus(Var(1))),
            (3, Int(0)),
        ],
    );
    action(
        &mut m,
        "Observe",
        Expr::Bool(true),
        &[(2, Var(0)), (3, Var(2).eq(Var(0)).ite(Var(3), Int(1)))],
    );
    // The broken control substitutes the last observed grant for the current
    // grant at the shared authority boundary. It can miss an upgrade on list
    // and admit a stale privileged call after downgrade, before observation.
    let oracle = if broken { eq(2, 1) } else { eq(1, 1) };
    action(
        &mut m,
        "List",
        Expr::Bool(true),
        &[(4, oracle.clone().eq(eq(1, 1)).ite(Var(4), Int(1)))],
    );
    action(
        &mut m,
        "CallPrivileged",
        oracle,
        &[(4, eq(1, 0).ite(Int(1), Var(4)))],
    );
    no_bad(&mut m, 4);
    m
}

pub fn delegation_pointer(broken: bool) -> Model {
    // Body sizes: small / 40 KiB / larger. A delegate and a reassign both
    // send a fixed-size pointer while retaining the full source body.
    let mut m = model(
        "DelegationPointer",
        broken,
        &[
            ("body", 2, 0),
            ("wakes", 2, 0),
            ("path", 1, 1),
            ("bounded", 1, 1),
        ],
    );
    action(
        &mut m,
        "GrowBrief",
        all([Var(0).lt(Int(2)), eq(1, 0)]),
        &[(0, Var(0).plus(Int(1)))],
    );
    for (name, count) in [("Delegate", 0), ("Reassign", 1)] {
        action(
            &mut m,
            name,
            eq(1, count),
            &[
                (1, Var(1).plus(Int(1))),
                (2, Int(if broken { 0 } else { 1 })),
            ],
        );
    }
    bboxy450_tlc::build::safe(&mut m, "WakeRetainsPath", eq(2, 1));
    bboxy450_tlc::build::safe(&mut m, "WakeBounded", eq(3, 1));
    m
}

pub fn absent_seat_close(broken: bool) -> Model {
    // process: 0 confirmed absent, 1 alive, 2 owner unavailable. A missing
    // viewport alone is never a death certificate. Atomic owner turns fence
    // registry generation; OS/process queries remain implementation witnesses.
    let mut m = model(
        "AbsentSeatClose",
        broken,
        &[
            ("process", 2, 0),
            ("viewport", 1, 0),
            ("generation", 1, 0),
            ("expected", 1, 0),
            ("closed", 1, 0),
            ("bad", 1, 0),
        ],
    );
    for p in 0..3 {
        action(&mut m, &format!("Owner{p}"), eq(4, 0), &[(0, Int(p))]);
    }
    action(&mut m, "ViewportAppears", eq(4, 0), &[(1, Int(1))]);
    action(&mut m, "Replace", all([eq(4, 0), eq(2, 0)]), &[(2, Int(1))]);
    action(&mut m, "CaptureGeneration", eq(4, 0), &[(3, Var(2))]);
    action(
        &mut m,
        "CloseAbsent",
        all([
            eq(4, 0),
            eq(1, 0),
            Var(2).eq(Var(3)),
            if broken { Expr::Bool(true) } else { eq(0, 0) },
        ]),
        &[(4, Int(1)), (5, eq(0, 0).ite(Int(0), Int(1)))],
    );
    no_bad(&mut m, 5);
    m
}

pub fn shared_codex_close(broken: bool) -> Model {
    // Each ticket represents the seat generation and its unique close operation.
    // Expiry callbacks can be delayed even after an undo and second close.
    let mut m = model(
        "SharedCodexClose",
        broken,
        &[
            ("lifecycle", 3, 0), // Open, grace, final, retained
            ("close", 2, 0),
            ("timer", 2, 0),
            ("result", 2, 0), // Pending, archived, native deadline/error
        ],
    );
    action(
        &mut m,
        "Schedule",
        all([eq(0, 0), Var(1).lt(Int(2))]),
        &[(0, Int(1)), (1, Var(1).plus(Int(1)))],
    );
    action(&mut m, "QueueExpiry", eq(0, 1), &[(2, Var(1))]);
    action(&mut m, "Undo", eq(0, 1), &[(0, Int(0))]);
    action(
        &mut m,
        "ApplyExpiry",
        all([eq(0, 1), Var(2).eq(Var(1))]),
        &[(0, Int(2))],
    );
    action(&mut m, "GraceExit", eq(0, 1), &[(0, Int(2))]);
    action(&mut m, "Archive", all([eq(0, 2), eq(3, 0)]), &[(3, Int(1))]);
    action(
        &mut m,
        "NativeFailure",
        all([eq(0, 2), eq(3, 0)]),
        &[(3, Int(2))],
    );
    action(
        &mut m,
        "Retain",
        all([eq(0, 2), if broken { Expr::Bool(true) } else { ne(3, 0) }]),
        &[(0, Int(3))],
    );
    safe(
        &mut m,
        "ArchiveRequiresFinalClose",
        implies(eq(3, 1), Var(0).le(Int(1)).negate()),
    );
    safe(
        &mut m,
        "RetainedCloseAttemptsArchive",
        implies(eq(0, 3), ne(3, 0)),
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mcp_live_contract_models_reject_broken_controls() {
        for family in [
            live_catalogue,
            delegation_pointer,
            absent_seat_close,
            shared_codex_close,
            crate::mcp::oracle_hint_model::oracle_in_turn,
        ] {
            assert!(family(false).check(10000).unwrap().violations.is_empty());
            assert!(!family(true).check(10000).unwrap().violations.is_empty());
            if std::env::var_os("BBOXY_UPDATE_MCP_MODELS").is_some() {
                let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("models");
                for broken in [false, true] {
                    for (name, content) in bboxy450_tlc::build::artifacts(&family(broken)).unwrap()
                    {
                        std::fs::write(root.join(name), content).unwrap();
                    }
                }
            }
        }
    }
}
