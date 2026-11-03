// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Descriptor ownership and async lifetime; not a translation of Rust or flock.
use super::*;

const WIDTH: usize = 9;
const BINDING: usize = 18;
const PENDING: usize = 19;
const REPLIED: usize = 20;
const OWNER: usize = 0;
const IO1: usize = 1;
const WAITER1: usize = 3;
const ISSUED: usize = 5;
const CALLBACK: usize = 6;
const BG: usize = 7;
const EPOCH: usize = 8;

fn writer_model(kind: usize, broken: bool) -> Model {
    let name = [
        "ProjectionWriterExclusion",
        "ProjectionWriterLifetime",
        "ProjectionRootAvailability",
    ][kind];
    // Two canonical roots. A has two distinct descriptor/operation identities;
    // B needs one. Binding 0 -> 1 -> 2 is logical A -> B -> A, never token reuse.
    let mut m = model(
        name,
        broken,
        &[
            ("owner_a", 2, 0),
            ("io_a1", 1, 0),
            ("io_a2", 1, 0),
            ("awaiter_a1", 1, 0),
            ("awaiter_a2", 1, 0),
            ("issued_a", 2, 0),
            ("old_callback_a", 2, 0),
            ("background_a", 1, 0),
            ("captured_epoch_a", 2, 0),
            ("owner_b", 2, 0),
            ("io_b1", 1, 0),
            ("io_b2", 1, 0),
            ("awaiter_b1", 1, 0),
            ("awaiter_b2", 1, 0),
            ("issued_b", 1, 0),
            ("old_callback_b", 2, 0),
            ("background_b", 1, 0),
            ("captured_epoch_b", 2, 0),
            ("binding", 2, 0),
            ("pending_read_b", 1, 0),
            ("replied_read_b", 1, 0),
        ],
    );
    let mut fair = Vec::new();
    for (root, bound) in [(0, 2), (1, 1)] {
        let base = root * WIDTH;
        let label = if root == 0 { "A" } else { "B" };
        for token in 1..=bound {
            let io = base + IO1 + token - 1;
            let waiter = base + WAITER1 + token - 1;
            action(
                &mut m,
                &format!("Acquire{label}{token}"),
                all([
                    eq(base + OWNER, 0),
                    Bool(broken && kind == 0).or(eq(base + BG, 0)),
                    eq(base + ISSUED, (token - 1) as i64),
                    if root == 0 {
                        ne(BINDING, 1)
                    } else {
                        Bool(true)
                    },
                ]),
                &[
                    (base + OWNER, Int(token as i64)),
                    (io, Int(1)),
                    (waiter, Int(1)),
                    (base + ISSUED, Int(token as i64)),
                    (base + EPOCH, Var(BINDING)),
                ],
            );
            // An awaiter is not the blocking job. The lifetime-broken control
            // drops its claim on cancellation even though the job still runs.
            action(
                &mut m,
                &format!("Cancel{label}{token}"),
                eq(waiter, 1),
                &[
                    (waiter, Int(0)),
                    (
                        base + OWNER,
                        if broken && kind == 1 {
                            eq(base + OWNER, token as i64).ite(Int(0), Var(base + OWNER))
                        } else {
                            Var(base + OWNER)
                        },
                    ),
                ],
            );
            action(
                &mut m,
                &format!("FinishIO{label}{token}"),
                eq(io, 1),
                &[
                    (io, Int(0)),
                    (
                        base + CALLBACK,
                        eq(base + CALLBACK, 0).ite(Int(token as i64), Var(base + CALLBACK)),
                    ),
                ],
            );
            // Actual IO/prepare refusal/panic completion enables last-reference
            // drop. Fair worker polling/drop is required only after IO ends.
            fair.push(m.actions.len());
            action(
                &mut m,
                &format!("DropCompleted{label}{token}"),
                all([
                    eq(base + OWNER, token as i64),
                    eq(base + IO1, 0),
                    eq(base + IO1 + 1, 0),
                ]),
                &[(base + OWNER, Int(0)), (waiter, Int(0))],
            );
            // Old/out-of-order notifications retain their original identity.
            // Broken release-by-root ignores identity and live-IO lifetime.
            let may_release = all([eq(base + OWNER, token as i64), eq(io, 0)]);
            action(
                &mut m,
                &format!("Deliver{label}{token}"),
                eq(base + CALLBACK, token as i64),
                &[(
                    base + OWNER,
                    if broken && kind == 1 {
                        Int(0)
                    } else {
                        may_release.ite(Int(0), Var(base + OWNER))
                    },
                )],
            );
        }
        action(
            &mut m,
            &format!("BeginProvenance{label}"),
            all([
                eq(base + BG, 0),
                eq(PENDING, 0),
                Bool(broken && kind == 0).or(eq(base + OWNER, 0)),
            ]),
            &[(base + BG, Int(1))],
        );
        fair.push(m.actions.len());
        action(
            &mut m,
            &format!("EndProvenance{label}"),
            eq(base + BG, 1),
            &[(base + BG, Int(0))],
        );
        if kind == 0 {
            safe(
                &mut m,
                &format!("ExclusiveWriter{label}"),
                implies(
                    eq(base + BG, 1),
                    all([
                        eq(base + OWNER, 0),
                        eq(base + IO1, 0),
                        eq(base + IO1 + 1, 0),
                    ]),
                ),
            );
        }
        if kind == 1 {
            for token in 1..=bound {
                safe(
                    &mut m,
                    &format!("LiveIOKeepsClaim{label}{token}"),
                    implies(
                        eq(base + IO1 + token - 1, 1),
                        eq(base + OWNER, token as i64),
                    ),
                );
            }
            m.responses.push(Response {
                name: format!("CompletedIOReleases{label}"),
                request: all([
                    ne(base + OWNER, 0),
                    eq(base + IO1, 0),
                    eq(base + IO1 + 1, 0),
                ]),
                response: eq(base + OWNER, 0),
            });
        }
    }
    action(&mut m, "MapToB", eq(BINDING, 0), &[(BINDING, Int(1))]);
    action(&mut m, "MapBackA", eq(BINDING, 1), &[(BINDING, Int(2))]);
    action(
        &mut m,
        "RequestReadB",
        all([eq(PENDING, 0), eq(REPLIED, 0)]),
        &[(PENDING, Int(1))],
    );
    fair.push(m.actions.len());
    action(
        &mut m,
        "ServeReadB",
        all([
            eq(PENDING, 1),
            eq(BG, 0),
            eq(WIDTH + BG, 0),
            if broken && kind == 2 {
                eq(OWNER, 0)
            } else {
                Bool(true)
            },
        ]),
        &[(PENDING, Int(0)), (REPLIED, Int(1))],
    );
    if kind == 2 {
        m.responses.push(Response {
            name: "IndependentRootReadProgress".into(),
            request: eq(PENDING, 1),
            response: eq(REPLIED, 1),
        });
    }
    // Each ready completion/page return/read service is weakly fair. IO itself,
    // mapping, admission, cancellations and callback arrival are NOT fair.
    m.weak_fair
        .extend(fair.into_iter().map(|action| vec![action]));
    m
}

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    (0..3)
        .flat_map(|kind| {
            [
                (writer_model(kind, false), "pass"),
                (
                    writer_model(kind, true),
                    if kind == 2 { "liveness" } else { "safety" },
                ),
            ]
        })
        .collect()
}

fn replay(m: &Model, result: &bboxy450_tlc::Report, trace: &[&str]) -> usize {
    let mut state = 0;
    for name in trace {
        let action = m
            .actions
            .iter()
            .position(|action| action.name == *name)
            .unwrap();
        state = result.edges[state]
            .iter()
            .find(|edge| edge.action == action)
            .unwrap_or_else(|| panic!("disabled {name} in {trace:?}"))
            .to;
    }
    state
}

#[test]
fn projection_writer_exclusion_lifetime_aba_and_independent_progress() -> Result<(), String> {
    for kind in 0..3 {
        let good = writer_model(kind, false);
        let good_report = good.check(100_000)?;
        let checked = &good_report;
        assert!(
            checked.violations.is_empty(),
            "{} {:?}",
            good.name,
            checked.violations
        );
        let state = replay(&good, checked, &["AcquireA1"]);
        let b = good
            .actions
            .iter()
            .position(|action| action.name == "AcquireB1")
            .unwrap();
        assert!(
            checked.edges[state].iter().any(|edge| edge.action == b),
            "root B admission depends on A"
        );
        let bad = writer_model(kind, true);
        let checked = bad.check(100_000)?;
        assert!(!checked.violations.is_empty());
        assert!(checked.violations.iter().any(
            |failure| !failure.prefix.is_empty() && (!failure.cycle.is_empty()) == (kind == 2)
        ));
        if kind == 0 {
            let state = replay(&bad, &checked, &["AcquireA1", "BeginProvenanceA"]);
            assert_eq!(
                (checked.states[state][OWNER], checked.states[state][BG]),
                (1, 1)
            );
        } else if kind == 1 {
            let state = replay(&bad, &checked, &["AcquireA1", "CancelA1"]);
            assert_eq!(
                (checked.states[state][OWNER], checked.states[state][IO1]),
                (0, 1)
            );
            let state = replay(
                &bad,
                &checked,
                &[
                    "AcquireA1",
                    "FinishIOA1",
                    "DropCompletedA1",
                    "MapToB",
                    "AcquireB1",
                    "FinishIOB1",
                    "DropCompletedB1",
                    "MapBackA",
                    "AcquireA2",
                    "DeliverA1",
                ],
            );
            assert_eq!(
                (
                    checked.states[state][BINDING],
                    checked.states[state][EPOCH],
                    checked.states[state][IO1 + 1],
                    checked.states[state][OWNER]
                ),
                (2, 2, 1, 0)
            );
        }
        println!(
            "{} {}/{}; {} {}/{}",
            good.name,
            good_report.states.len(),
            good_report.edges.iter().map(Vec::len).sum::<usize>(),
            bad.name,
            checked.states.len(),
            checked.edges.iter().map(Vec::len).sum::<usize>()
        );
    }
    Ok(())
}
