// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Two queued modification paths and a four-position wrapping deque index.
use super::{action, all, eq, implies, model, safe};
use bboxy450_tlc::{
    Expr::{Int, Var},
    Model, Response,
};

fn inbox(broken: bool) -> Model {
    let mut m = model(
        "WatcherInbox",
        broken,
        &[
            ("head", 3, 0),
            ("len", 2, 0),
            ("front", 2, 0),
            ("back", 2, 0),
            ("index_a", 4, 0),
            ("index_b", 4, 0),
        ],
    );
    // Stored positions use 1..4; zero means absent. Deque positions wrap at four.
    let sum = Var(0).plus(Var(1));
    let tail = sum
        .clone()
        .lt(Int(4))
        .ite(sum.clone(), sum.minus(Int(4)))
        .plus(Int(1));
    for (path, index) in [(1, 4), (2, 5)] {
        action(
            &mut m,
            &format!("Push{path}"),
            all([eq(index, 0), Var(1).lt(Int(2))]),
            &[
                (1, Var(1).plus(Int(1))),
                (2, eq(1, 0).ite(Int(path), Var(2))),
                (3, eq(1, 1).ite(Int(path), Var(3))),
                (index, tail.clone()),
            ],
        );
    }
    action(
        &mut m,
        "Pop",
        Int(0).lt(Var(1)),
        &[
            (0, eq(0, 3).ite(Int(0), Var(0).plus(Int(1)))),
            (1, Var(1).minus(Int(1))),
            (2, Var(3)),
            (3, Int(0)),
            (
                4,
                if broken {
                    Var(4)
                } else {
                    eq(2, 1).ite(Int(0), Var(4))
                },
            ),
            (
                5,
                if broken {
                    Var(5)
                } else {
                    eq(2, 2).ite(Int(0), Var(5))
                },
            ),
        ],
    );
    for (path, index) in [(1, 4), (2, 5)] {
        action(
            &mut m,
            &format!("Coalesce{path}"),
            Int(0).lt(Var(index)),
            &[],
        );
        let present = eq(2, path).or(eq(3, path));
        let back_position = eq(0, 3).ite(Int(1), Var(0).plus(Int(2)));
        safe(
            &mut m,
            &format!("Index{path}NamesItsQueuedPath"),
            all([
                implies(eq(index, 0), present.clone().negate()),
                implies(
                    Int(0).lt(Var(index)),
                    present.and(
                        eq(2, path)
                            .and(Var(index).eq(Var(0).plus(Int(1))))
                            .or(eq(3, path).and(Var(index).eq(back_position))),
                    ),
                ),
            ]),
        );
        if !broken {
            m.responses.push(Response {
                name: format!("Path{path}EventuallyServed"),
                request: Int(0).lt(Var(index)),
                response: eq(index, 0),
            });
        }
    }
    safe(
        &mut m,
        "NoDuplicateModification",
        implies(eq(1, 2), Var(2).eq(Var(3)).negate()),
    );
    m.weak_fair.push(vec![2]);
    m
}

pub fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (inbox(false), "pass"),
        (inbox(true), "safety"),
        (settle(false), "pass"),
        (settle(true), "liveness"),
        (changes(false), "pass"),
        (changes(true), "safety"),
        (pool(false), "pass"),
        (pool(true), "safety"),
        (publication(false), "pass"),
        (publication(true), "safety"),
        (admission(false), "pass"),
        (admission(true), "safety"),
    ]
}

#[test]
fn watcher_indices_survive_pop_reuse_and_counter_wrap() {
    let healthy = inbox(false).check(1000).unwrap();
    assert!(healthy.violations.is_empty());
    assert!(
        healthy
            .states
            .iter()
            .any(|state| state[0] == 3 && state[1] == 2)
    );
    let broken = inbox(true).check(1000).unwrap();
    assert!(
        broken
            .violations
            .iter()
            .any(|violation| violation.property.starts_with("Index"))
    );
}

// Two age ticks represent the bounded maximum; one idle tick is quiet settling.
// Hints may continue forever. Fair ticks/admission, rather than quiet input,
// must suffice to admit the first dirty batch.
fn settle(broken: bool) -> Model {
    let mut m = model(
        "WatcherSettle",
        broken,
        &[
            ("dirty", 1, 0),
            ("first_age", 2, 0),
            ("last_age", 1, 0),
            ("served", 1, 0),
        ],
    );
    action(
        &mut m,
        "Hint",
        eq(3, 0),
        &[
            (0, Int(1)),
            (
                1,
                if broken {
                    Int(0)
                } else {
                    eq(0, 0).ite(Int(0), Var(1))
                },
            ),
            (2, Int(0)),
        ],
    );
    action(
        &mut m,
        "Tick",
        all([eq(0, 1), eq(3, 0)]),
        &[
            (1, Var(1).lt(Int(2)).ite(Var(1).plus(Int(1)), Var(1))),
            (2, Int(1)),
        ],
    );
    action(
        &mut m,
        "Admit",
        all([eq(0, 1), eq(1, 2).or(eq(2, 1))]),
        &[(0, Int(0)), (3, Int(1))],
    );
    m.responses.push(Response {
        name: "ContinuousHintsEventuallyAdmit".into(),
        request: eq(0, 1),
        response: eq(3, 1),
    });
    m.weak_fair.extend([vec![1], vec![2]]);
    m
}

// The second filesystem revision may arrive after capture but before completion.
// Completion publishes its captured snapshot without clearing newer dirtiness.
fn changes(broken: bool) -> Model {
    let mut m = model(
        "WatcherChanges",
        broken,
        &[
            ("filesystem", 2, 0),
            ("dirty", 1, 0),
            ("running", 1, 0),
            ("captured", 2, 0),
            ("published", 2, 0),
        ],
    );
    action(
        &mut m,
        "Change",
        Var(0).lt(Int(2)),
        &[(0, Var(0).plus(Int(1))), (1, Int(1))],
    );
    action(
        &mut m,
        "Capture",
        all([eq(1, 1), eq(2, 0)]),
        &[(1, Int(0)), (2, Int(1)), (3, Var(0))],
    );
    action(
        &mut m,
        "Complete",
        eq(2, 1),
        &[
            (2, Int(0)),
            (4, Var(3)),
            (1, if broken { Int(0) } else { Var(1) }),
        ],
    );
    safe(
        &mut m,
        "NoUnscheduledChange",
        implies(all([eq(1, 0), eq(2, 0)]), Var(4).eq(Var(0))),
    );
    if !broken {
        for revision in 1..=2 {
            m.responses.push(Response {
                name: format!("Revision{revision}EventuallyVisibleOrSuperseded"),
                request: eq(0, revision),
                response: eq(4, revision).or(Int(revision).lt(Var(0))),
            });
        }
    }
    m.weak_fair.extend([vec![1], vec![2]]);
    m
}

// Three roots compete for two lanes. IO must eventually complete for liveness;
// two permanently stalled IO operations can exhaust this deliberately finite pool.
fn pool(broken: bool) -> Model {
    let mut m = model(
        "WatcherPool",
        broken,
        &[("root_a", 2, 0), ("root_b", 2, 0), ("root_c", 2, 0)],
    );
    let busy = eq(0, 1)
        .ite(Int(1), Int(0))
        .plus(eq(1, 1).ite(Int(1), Int(0)))
        .plus(eq(2, 1).ite(Int(1), Int(0)));
    for root in 0..3 {
        action(
            &mut m,
            &format!("Start{root}"),
            eq(root, 0).and(if broken {
                Int(1).eq(Int(1))
            } else {
                busy.clone().lt(Int(2))
            }),
            &[(root, Int(1))],
        );
        action(
            &mut m,
            &format!("Finish{root}"),
            eq(root, 1),
            &[(root, Int(2))],
        );
        m.weak_fair.extend([vec![root * 2], vec![root * 2 + 1]]);
        if !broken {
            m.responses.push(Response {
                name: format!("Root{root}EventuallyFinishes"),
                request: eq(root, 0),
                response: eq(root, 2),
            });
        }
    }
    safe(&mut m, "AtMostTwoJobs", busy.lt(Int(3)));
    m
}

// One stable durable event ID. Failed/cancelled reads keep it; bus append is
// idempotent for that ID. A single transient failure is allowed before recovery.
fn publication(broken: bool) -> Model {
    let mut m = model(
        "WatcherPublication",
        broken,
        &[
            ("durable", 1, 1),
            ("active", 1, 0),
            ("appended", 1, 0),
            ("failed", 1, 0),
            ("acknowledged", 1, 0),
        ],
    );
    action(&mut m, "Read", all([eq(0, 1), eq(1, 0)]), &[(1, Int(1))]);
    action(&mut m, "Append", eq(1, 1), &[(2, Int(1))]);
    action(
        &mut m,
        "TransientFailure",
        all([eq(1, 1), eq(3, 0)]),
        &[(1, Int(0)), (3, Int(1))],
    );
    action(
        &mut m,
        "Acknowledge",
        eq(1, 1).and(if broken { Int(1).eq(Int(1)) } else { eq(2, 1) }),
        &[(0, Int(0)), (1, Int(0)), (4, Int(1))],
    );
    safe(
        &mut m,
        "AcknowledgeOnlyAfterAppend",
        implies(eq(4, 1), eq(2, 1)),
    );
    if !broken {
        m.responses.push(Response {
            name: "DurableEventEventuallyAppendedAndAcknowledged".into(),
            request: eq(0, 1),
            response: eq(4, 1),
        });
    }
    m.weak_fair.extend([vec![0], vec![1], vec![3]]);
    m
}

#[test]
fn deferred_watcher_models_reject_their_broken_controls() {
    for (model, expected) in cases().into_iter().skip(2) {
        let checked = model.check(10_000).unwrap();
        assert_eq!(
            checked.violations.is_empty(),
            expected == "pass",
            "{}: {:?}",
            model.name,
            checked.violations
        );
    }
}

// Settling postpones reconciliation, but cannot admit mutation from an older
// snapshot. The broken guard confuses "not settled" with "no pending change".
fn admission(broken: bool) -> Model {
    let mut m = model(
        "WatcherAdmission",
        broken,
        &[("phase", 4, 0), ("stale_write", 1, 0)],
    );
    action(&mut m, "Hint", eq(0, 0), &[(0, Int(1))]);
    action(&mut m, "Settle", eq(0, 1), &[(0, Int(2))]);
    action(&mut m, "Capture", eq(0, 2), &[(0, Int(3))]);
    action(&mut m, "Complete", eq(0, 3), &[(0, Int(4))]);
    action(
        &mut m,
        "Write",
        eq(0, 0)
            .or(eq(0, 4))
            .or(if broken { eq(0, 1) } else { Int(0).eq(Int(1)) }),
        &[(1, eq(0, 1).ite(Int(1), Var(1)))],
    );
    safe(&mut m, "NoMutationFromUnsettledSnapshot", eq(1, 0));
    m
}
