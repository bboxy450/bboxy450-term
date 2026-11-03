// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Logic-owner occupancy, not a translation of projection.rs or wall-clock time.
use super::*;

const PHASE: usize = 0;
const LEFT: usize = 1;
const CURSOR: usize = 2;
const PENDING: usize = 3;
const OLDEST: usize = 4;
const SECOND: usize = 5;
const SHUTDOWN: usize = 6;
const DEADLINE: usize = 7;
const HISTORY: usize = 8;
const AUDITS: usize = 9;
const B: i64 = 2;
const D: i64 = 3;
const H: i64 = 4;
const REQUEST_BOUND: &str = "RequestsStayWithinWorkerTickBound";
const STOP_BOUND: &str = "ShutdownEventuallyStopsBeforeDeadline";

fn increment_to(index: usize, limit: i64) -> Expr {
    Var(index)
        .lt(Int(limit))
        .ite(Var(index).plus(Int(1)), Var(index))
}

fn worker(broken: bool, shutdown_property: bool, audits: bool, history: i64) -> Model {
    let name = match (audits, shutdown_property) {
        (false, false) => "LogicWorkerRequests",
        (false, true) => "LogicWorkerShutdown",
        (true, false) => "LogicWorkerAuditRequests",
        (true, true) => "LogicWorkerAuditShutdown",
    };
    // phase: idle=0, serving=1, inside step=2, stopped=3. Ages saturate at
    // B+1 (retain a violation); elapsed shutdown time saturates at D, forever
    // distinguishing a late join from a successful stop. Two FIFO slots.
    let mut m = model(
        name,
        broken,
        &[
            ("phase", 3, 0),
            ("work_left", H, 0),
            ("cursor", H, 0),
            ("pending", 2, 0),
            ("oldest_age", B + 1, 0),
            ("second_age", B + 1, 0),
            ("shutdown", 1, 0),
            ("deadline", D, 0),
            ("history_pages", H, history),
            ("audit_pages", 2 * H, if audits { 2 * history } else { 0 }),
        ],
    );
    action(
        &mut m,
        "RequestArrives",
        all([ne(PHASE, 3), eq(SHUTDOWN, 0), Var(PENDING).lt(Int(2))]),
        &[(PENDING, Var(PENDING).plus(Int(1)))],
    );
    action(
        &mut m,
        "RequestShutdown",
        all([ne(PHASE, 3), eq(SHUTDOWN, 0)]),
        &[(SHUTDOWN, Int(1))],
    );
    // Environment audit completions are NOT assumed fair. Each of N=2
    // audits has H pages. Independent connections consume no worker ticks.
    action(
        &mut m,
        "AuditPage",
        Int(0).lt(Var(AUDITS)),
        &[(AUDITS, Var(AUDITS).minus(Int(1)))],
    );
    let first_worker_action = m.actions.len();
    action(
        &mut m,
        "ServeRequest",
        all([eq(PHASE, 0), eq(SHUTDOWN, 0), Int(0).lt(Var(PENDING))]),
        &[(PHASE, Int(1))],
    );
    action(
        &mut m,
        "BeginProvenanceStep",
        all([
            eq(PHASE, 0),
            eq(SHUTDOWN, 0),
            eq(PENDING, 0),
            Var(CURSOR).lt(Var(HISTORY)),
        ]),
        &[
            (PHASE, Int(2)),
            (
                LEFT,
                if broken && !audits {
                    Var(HISTORY).minus(Var(CURSOR))
                } else {
                    Int(1)
                },
            ),
        ],
    );
    // Tick is one occupied worker quantum: a page, one request service, or
    // (audit control only) one quantum blocked on the shared store lock.
    // There is no free-running timer that can outrun weakly fair dispatch.
    let serving = eq(PHASE, 1);
    let locked = all([Bool(broken && audits), eq(PHASE, 2), Int(0).lt(Var(AUDITS))]);
    let page = eq(PHASE, 2).and(locked.clone().negate());
    action(
        &mut m,
        "Tick",
        serving
            .clone()
            .or(all([eq(PHASE, 2), Int(0).lt(Var(LEFT))])),
        &[
            (PHASE, serving.clone().ite(Int(0), Var(PHASE))),
            (LEFT, page.clone().ite(Var(LEFT).minus(Int(1)), Var(LEFT))),
            (CURSOR, page.ite(Var(CURSOR).plus(Int(1)), Var(CURSOR))),
            (
                PENDING,
                serving
                    .clone()
                    .ite(Var(PENDING).minus(Int(1)), Var(PENDING)),
            ),
            (
                OLDEST,
                serving.clone().ite(
                    eq(PENDING, 2).ite(increment_to(SECOND, B + 1), Int(0)),
                    Int(0)
                        .lt(Var(PENDING))
                        .ite(increment_to(OLDEST, B + 1), Int(0)),
                ),
            ),
            (
                SECOND,
                serving.ite(
                    Int(0),
                    eq(PENDING, 2).ite(increment_to(SECOND, B + 1), Int(0)),
                ),
            ),
            (
                DEADLINE,
                eq(SHUTDOWN, 1).ite(increment_to(DEADLINE, D), Var(DEADLINE)),
            ),
            (AUDITS, locked.ite(Var(AUDITS).minus(Int(1)), Var(AUDITS))),
        ],
    );
    // Resumable cursor persists across returns; shutdown and FIFO requests
    // have priority before another step. Yield itself consumes no page time.
    action(
        &mut m,
        "YieldStep",
        all([eq(PHASE, 2), eq(LEFT, 0)]),
        &[(PHASE, Int(0))],
    );
    action(
        &mut m,
        "StopWorker",
        all([eq(PHASE, 0), eq(SHUTDOWN, 1)]),
        &[(PHASE, Int(3))],
    );
    // Only the worker action group is weakly fair, never requests/shutdown/
    // audits. This ensures eventual dispatch, not bounded OS scheduling.
    m.weak_fair
        .push((first_worker_action..m.actions.len()).collect());
    if shutdown_property {
        m.responses.push(Response {
            name: STOP_BOUND.into(),
            request: eq(SHUTDOWN, 1),
            response: eq(PHASE, 3).and(Var(DEADLINE).lt(Int(D))),
        });
    } else {
        safe(
            &mut m,
            REQUEST_BOUND,
            implies(
                all([eq(PHASE, 2), Int(0).lt(Var(PENDING))]),
                all([Var(OLDEST).le(Int(B)), Var(SECOND).le(Int(B))]),
            ),
        );
    }
    m
}

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    let mut cases = Vec::new();
    for audits in [false, true] {
        for shutdown in [false, true] {
            cases.push((worker(false, shutdown, audits, H), "pass"));
            cases.push((
                worker(true, shutdown, audits, H),
                if shutdown { "liveness" } else { "safety" },
            ));
        }
    }
    cases
}

#[test]
fn logic_worker_bounds_and_counterexamples() -> Result<(), String> {
    for history in 1..=H {
        for shutdown in [false, true] {
            let good = worker(false, shutdown, false, history).check(100_000)?;
            assert!(
                good.violations.is_empty(),
                "H={history}: {:?}",
                good.violations
            );
            let bad = worker(true, shutdown, false, history).check(100_000)?;
            assert_eq!(
                bad.violations.is_empty(),
                if shutdown { history < D } else { history <= B }
            );
        }
    }
    for audits in [false, true] {
        for shutdown in [false, true] {
            let m = worker(true, shutdown, audits, H);
            let r = m.check(100_000)?;
            let property = if shutdown { STOP_BOUND } else { REQUEST_BOUND };
            let violation = r
                .violations
                .iter()
                .find(|v| v.property == property)
                .expect("named broken control must fail");
            assert!(!violation.prefix.is_empty());
            assert_eq!(!violation.cycle.is_empty(), shutdown);
            // Replay the incident-shaped prefix through actual explored edges:
            // enter step, request and shutdown arrive, three occupied ticks.
            let mut state = 0;
            for name in [
                "BeginProvenanceStep",
                "RequestArrives",
                "RequestShutdown",
                "Tick",
                "Tick",
                "Tick",
            ] {
                let action = m.actions.iter().position(|a| a.name == name).unwrap();
                state = r.edges[state]
                    .iter()
                    .find(|e| e.action == action)
                    .expect("incident prefix enabled")
                    .to;
            }
            let s = &r.states[state];
            assert_eq!(
                (s[PHASE], s[PENDING], s[OLDEST], s[DEADLINE]),
                (2, 1, B + 1, D)
            );
            assert_eq!(s[CURSOR], if audits { 0 } else { 3 });
            if shutdown {
                assert!(violation.cycle.iter().all(|i| r.states[*i][DEADLINE] == D));
            }
        }
    }
    Ok(())
}
