// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Dream C1 bounded contracts, not Rust/OS refinement or measured latency proofs.
//! State, initial values and each atomic owner action are explicit below. Files,
//! hashes, SQLite durability, unit CPU/heap/RSS and process identity need witnesses.
use super::*;

const ADMISSION_NAMES: [&str; 5] = [
    "TermDreamC1Admission",
    "TermDreamC1AdmissionDedup",
    "TermDreamC1AdmissionCursor",
    "TermDreamC1AdmissionLegacy",
    "TermDreamC1AdmissionOriginalOp",
];

fn admission(kind: usize, broken: bool) -> Model {
    // Two exact retained sources. Each source starts open with no cursor, run,
    // original operation, expensive hash or legacy work. One restart is bounded.
    const WIDTH: usize = 6;
    let mut m = model(
        ADMISSION_NAMES[kind],
        broken,
        &[
            ("closed_a", 1, 0),
            ("cursor_a", 1, 0),
            ("runs_a", 2, 0),
            ("op_a", 1, 0),
            ("hashes_a", 2, 0),
            ("legacy_a", 1, 0),
            ("closed_b", 1, 0),
            ("cursor_b", 1, 0),
            ("runs_b", 2, 0),
            ("op_b", 1, 0),
            ("hashes_b", 2, 0),
            ("legacy_b", 1, 0),
            ("restarted", 1, 0),
        ],
    );
    for (source, label) in [(0, "A"), (1, "B")] {
        let i = source * WIDTH;
        // Only the process/socket/log observer can supply Close. Idle, EOF and
        // turn-end are visible external actions and leave eligibility unchanged.
        action(&mut m, &format!("Close{label}"), eq(i, 0), &[(i, Int(1))]);
        for event in ["Idle", "TurnEnded", "NativeEof"] {
            action(&mut m, &format!("{event}{label}"), Bool(true), &[]);
        }
        for owner in ["Auto", "Manual"] {
            action(
                &mut m,
                &format!("{owner}Register{label}"),
                all([
                    Bool(broken && kind == 0).or(eq(i, 1)),
                    if broken && kind == 1 {
                        Var(i + 2).lt(Int(2))
                    } else {
                        eq(i + 2, 0)
                    },
                ]),
                &[
                    // The cursor, SourceKey run and original operation are one
                    // journal transaction before any source IO/CPU admission.
                    (i + 1, if broken && kind == 2 { Int(0) } else { Int(1) }),
                    (i + 2, Var(i + 2).plus(Int(1))),
                    (i + 3, Int(1)),
                ],
            );
        }
        action(
            &mut m,
            &format!("Hash{label}"),
            all([eq(i + 3, 1), Var(i + 4).lt(Var(i + 2))]),
            &[(i + 4, Var(i + 4).plus(Int(1)))],
        );
        action(
            &mut m,
            &format!("RepeatCloseOrSelection{label}"),
            ne(i + 2, 0),
            &[],
        );
        action(
            &mut m,
            &format!("RetryOriginalOperation{label}"),
            all([eq(i + 3, 1), eq(i + 4, 1)]),
            &[(
                i + 4,
                if broken && kind == 4 {
                    Int(2)
                } else {
                    Var(i + 4)
                },
            )],
        );
        action(
            &mut m,
            &format!("LegacyCloseHook{label}"),
            eq(i, 1),
            &[(i + 5, if broken && kind == 3 { Int(1) } else { Int(0) })],
        );
        safe(
            &mut m,
            &format!("ClosedOnly{label}"),
            implies(ne(i + 2, 0), eq(i, 1)),
        );
        safe(
            &mut m,
            &format!("AtomicCloseCursorAndOriginalOperation{label}"),
            implies(ne(i + 2, 0), all([eq(i + 1, 1), eq(i + 3, 1)])),
        );
        safe(
            &mut m,
            &format!("UniqueSourceKey{label}"),
            Var(i + 2).le(Int(1)),
        );
        safe(
            &mut m,
            &format!("NoSecondHashBurst{label}"),
            Var(i + 4).le(Int(1)),
        );
        safe(&mut m, &format!("LegacySuppressed{label}"), eq(i + 5, 0));
    }
    // Loss of an observer/daemon does not erase durable source/original-op keys.
    action(&mut m, "RestartRetainsJournal", eq(12, 0), &[(12, Int(1))]);
    // Safety only: no fair external close, requested inspection or IO is assumed.
    m
}

const SCHEDULING_NAMES: [&str; 5] = [
    "TermDreamC1Scheduling",
    "TermDreamC1SchedulingDutyCycle",
    "TermDreamC1SchedulingCapacity",
    "TermDreamC1SchedulingHotOwner",
    "TermDreamC1SchedulingIndependentRoot",
];

fn scheduling(kind: usize, broken: bool) -> Model {
    // Two roots/jobs and one peer claim on A. Production caps 2/root,4/account
    // are scaled to 1/root,2/account; ownership and no-global-pool law is unchanged.
    // phase: 0 absent,1 deferred,2 running,3 paused,4 settling,5 stopped,6 quarantine.
    // No source begins eligible and foreground begins true. One unit is atomic;
    // rest=1 means the mandatory 9 ms wall wait has not elapsed. CPU is abstract.
    const WIDTH: usize = 8;
    const PEER: usize = 16;
    const BAD: usize = 17;
    let mut m = model(
        SCHEDULING_NAMES[kind],
        broken,
        &[
            ("closed_a", 1, 0),
            ("phase_a", 6, 0),
            ("claim_a", 1, 0),
            ("child_a", 1, 0),
            ("foreground_a", 2, 1),
            ("rest_a", 1, 0),
            ("units_a", 2, 0),
            ("exit_a", 1, 0),
            ("closed_b", 1, 0),
            ("phase_b", 6, 0),
            ("claim_b", 1, 0),
            ("child_b", 1, 0),
            ("foreground_b", 1, 1),
            ("rest_b", 1, 0),
            ("units_b", 2, 0),
            ("exit_b", 1, 0),
            ("peer_claim_a", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "HoldPeerA",
        all([eq(PEER, 0), eq(2, 0)]),
        &[(PEER, Int(1))],
    );
    action(&mut m, "ReleasePeerA", eq(PEER, 1), &[(PEER, Int(0))]);
    let mut fair = Vec::new();
    for (root, label) in [(0, "A"), (1, "B")] {
        let i = root * WIDTH;
        action(&mut m, &format!("Close{label}"), eq(i, 0), &[(i, Int(1))]);
        action(
            &mut m,
            &format!("Propose{label}"),
            all([eq(i, 1), eq(i + 1, 0)]),
            &[(i + 1, Int(1))],
        );
        // B's quiescent revision is stable for the conditional-progress property.
        // A additionally exercises a foreground turn interrupting historical work.
        action(
            &mut m,
            &format!("Quiescent{label}"),
            ne(i + 4, 0),
            &[
                (i + 4, Int(0)),
                (i + 1, eq(i + 1, 3).ite(Int(2), Var(i + 1))),
            ],
        );
        if root == 0 {
            action(
                &mut m,
                "ForegroundA",
                all([eq(i + 4, 0), ne(i + 1, 5), ne(i + 1, 6)]),
                &[
                    (i + 4, Int(1)),
                    (i + 1, eq(i + 1, 2).ite(Int(3), Var(i + 1))),
                ],
            );
        }
        if root == 0 {
            action(
                &mut m,
                "UnknownActivityA",
                ne(i + 4, 2),
                &[
                    (i + 4, Int(2)),
                    (i + 1, eq(i + 1, 2).ite(Int(3), Var(i + 1))),
                ],
            );
        }
        fair.push(m.actions.len());
        action(
            &mut m,
            &format!("Admit{label}"),
            all([
                eq(i + 1, 1),
                eq(i + 4, 0),
                if broken && kind == 2 {
                    Bool(true)
                } else {
                    if root == 0 {
                        eq(PEER, 0)
                    } else {
                        Var(2).plus(Var(PEER)).lt(Int(2))
                    }
                },
                if broken && kind == 4 && root == 1 {
                    eq(2, 0)
                } else {
                    Bool(true)
                },
            ]),
            &[(i + 1, Int(2)), (i + 2, Int(1)), (i + 3, Int(1))],
        );
        action(
            &mut m,
            &format!("WorkUnit{label}"),
            all([
                if broken && kind == 0 {
                    eq(i + 1, 2).or(eq(i + 1, 3))
                } else {
                    all([eq(i + 1, 2), eq(i + 4, 0)])
                },
                if broken && kind == 1 {
                    Bool(true)
                } else {
                    eq(i + 5, 0)
                },
                Var(i + 6).lt(Int(2)),
            ]),
            &[
                (i + 5, Int(1)),
                (i + 6, Var(i + 6).plus(Int(1))),
                (
                    BAD,
                    ne(i + 4, 0)
                        .or(eq(i + 5, 1))
                        .or(Bool(broken && kind == 3))
                        .ite(Int(1), Var(BAD)),
                ),
            ],
        );
        action(
            &mut m,
            &format!("RestElapsed{label}"),
            eq(i + 5, 1),
            &[(i + 5, Int(0))],
        );
        // Deadline/control owner is independent of a stalled compute/output owner.
        fair.push(m.actions.len());
        action(
            &mut m,
            &format!("CancelOrDeadline{label}"),
            eq(i + 1, 2).or(eq(i + 1, 3)),
            &[(i + 1, Int(4))],
        );
        action(
            &mut m,
            &format!("ObserveActualExit{label}"),
            all([eq(i + 1, 4), eq(i + 3, 1)]),
            &[(i + 3, Int(0)), (i + 7, Int(1))],
        );
        fair.push(m.actions.len());
        action(
            &mut m,
            &format!("SettleStopped{label}"),
            all([eq(i + 1, 4), eq(i + 7, 1)]),
            &[(i + 1, Int(5)), (i + 2, Int(0))],
        );
        fair.push(m.actions.len());
        action(
            &mut m,
            &format!("QuarantineUnproven{label}"),
            all([eq(i + 1, 4), eq(i + 3, 1)]),
            &[(i + 1, Int(6))],
        );
        safe(
            &mut m,
            &format!("LiveChildKeepsSlot{label}"),
            implies(eq(i + 3, 1), eq(i + 2, 1)),
        );
        safe(
            &mut m,
            &format!("TerminalStoppedRequiresExit{label}"),
            implies(eq(i + 1, 5), all([eq(i + 7, 1), eq(i + 3, 0)])),
        );
        m.responses.push(Response {
            name: format!("AdmittedEventuallySettledOrQuarantined{label}"),
            request: eq(i + 1, 2).or(eq(i + 1, 3)).or(eq(i + 1, 4)),
            response: eq(i + 1, 5).or(eq(i + 1, 6)),
        });
    }
    safe(&mut m, "PerRootCapacity", Var(2).plus(Var(PEER)).le(Int(1)));
    safe(
        &mut m,
        "AccountCapacity",
        Var(2).plus(Var(10)).plus(Var(PEER)).le(Int(2)),
    );
    no_bad(&mut m, BAD);
    m.responses.push(Response {
        name: "IndependentReadyRootProgress".into(),
        request: all([eq(9, 1), eq(12, 0)]),
        response: ne(9, 1),
    });
    // Only bounded internal Admit/deadline/settle/quarantine transitions are fair.
    // No fairness/termination assumption is made for source IO or actual OS exit.
    m.weak_fair.extend(fair.into_iter().map(|a| vec![a]));
    m
}

const OWNERSHIP_NAMES: [&str; 6] = [
    "TermDreamC1Ownership",
    "TermDreamC1OwnershipValidation",
    "TermDreamC1OwnershipReap",
    "TermDreamC1OwnershipCancel",
    "TermDreamC1OwnershipUnknown",
    "TermDreamC1OwnershipRestart",
];

fn ownership(kind: usize, broken: bool) -> Model {
    // One retained original operation, one child/result, two source-generation
    // boundaries A1 -> B2 -> A3. Epoch is a composite of current account, seat,
    // grants, swarm/workspace/root/provider incarnations, not their display names.
    const CTX: usize = 0;
    const CAPTURED: usize = 1;
    const ADMITTED: usize = 2;
    const OWNER: usize = 3;
    const CHILD: usize = 4;
    const HASHES: usize = 5;
    const DONE: usize = 6;
    const VALID: usize = 7;
    const EXIT: usize = 8;
    const COMMITTED: usize = 9;
    const CANCEL: usize = 10;
    const RESTART: usize = 11;
    const DISPOSITION: usize = 12;
    const BAD: usize = 13;
    let mut m = model(
        OWNERSHIP_NAMES[kind],
        broken,
        &[
            ("context", 2, 0),
            ("captured", 2, 0),
            ("admitted", 1, 0),
            ("owner", 1, 0),
            ("child", 1, 0),
            ("hashes", 2, 0),
            ("done", 1, 0),
            ("validated", 1, 0),
            ("exit", 1, 0),
            ("committed", 1, 0),
            ("cancel", 1, 0),
            ("restart", 1, 0),
            ("disposition", 3, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "AdmitOriginal",
        eq(ADMITTED, 0),
        &[(ADMITTED, Int(1)), (OWNER, Int(1)), (CAPTURED, Var(CTX))],
    );
    action(
        &mut m,
        "HashSnapshot",
        all([eq(ADMITTED, 1), eq(HASHES, 0), eq(CANCEL, 0)]),
        &[(HASHES, Int(1))],
    );
    action(
        &mut m,
        "SpawnOwnedChild",
        all([
            eq(HASHES, 1),
            eq(CHILD, 0),
            eq(EXIT, 0),
            eq(CANCEL, 0),
            eq(DISPOSITION, 0),
        ]),
        &[(CHILD, Int(1))],
    );
    action(
        &mut m,
        "WorkerDone",
        all([eq(CHILD, 1), eq(DONE, 0)]),
        &[(DONE, Int(1))],
    );
    action(
        &mut m,
        "ValidateImmutableResult",
        eq(DONE, 1),
        &[(VALID, Int(1))],
    );
    action(
        &mut m,
        "ObserveActualExit",
        eq(CHILD, 1),
        &[(CHILD, Int(0)), (EXIT, Int(1))],
    );
    action(
        &mut m,
        "PublishCompleted",
        all([
            eq(DONE, 1),
            eq(CANCEL, 0),
            eq(COMMITTED, 0),
            if broken && kind == 1 {
                Bool(true)
            } else {
                eq(VALID, 1)
            },
            if broken && kind == 2 {
                Bool(true)
            } else {
                eq(EXIT, 1)
            },
        ]),
        &[(COMMITTED, Int(1)), (DISPOSITION, Int(1)), (OWNER, Int(0))],
    );
    for (name, from, to) in [("ContextToB", 0, 1), ("ContextBackToA", 1, 2)] {
        action(
            &mut m,
            name,
            eq(CTX, from),
            &[
                (CTX, Int(to)),
                (
                    CANCEL,
                    eq(ADMITTED, 1)
                        .and(eq(COMMITTED, 0))
                        .ite(Int(1), Var(CANCEL)),
                ),
            ],
        );
    }
    action(
        &mut m,
        "PresentCurrentResult",
        all([
            eq(COMMITTED, 1),
            if broken && kind == 0 {
                Var(CTX)
                    .eq(Var(CAPTURED))
                    .or(all([eq(CAPTURED, 0), eq(CTX, 2)]))
            } else {
                Var(CTX).eq(Var(CAPTURED))
            },
        ]),
        &[(BAD, Var(CTX).eq(Var(CAPTURED)).ite(Var(BAD), Int(1)))],
    );
    let deadline = m.actions.len();
    action(
        &mut m,
        "CancelOrDeadline",
        all([eq(ADMITTED, 1), eq(DISPOSITION, 0)]),
        &[
            (CANCEL, Int(1)),
            (
                OWNER,
                if broken && kind == 3 {
                    Int(0)
                } else {
                    Var(OWNER)
                },
            ),
        ],
    );
    action(
        &mut m,
        "DaemonPipeLost",
        all([eq(ADMITTED, 1), eq(RESTART, 0), eq(DISPOSITION, 0)]),
        &[(RESTART, Int(1)), (CANCEL, Int(1))],
    );
    action(
        &mut m,
        "RetryUnknownOriginal",
        all([eq(RESTART, 1), eq(HASHES, 1)]),
        &[(
            HASHES,
            if broken && kind == 4 {
                Int(2)
            } else {
                Var(HASHES)
            },
        )],
    );
    let stopped = m.actions.len();
    action(
        &mut m,
        "SettleProvenStopped",
        all([eq(CANCEL, 1), eq(CHILD, 0), eq(DISPOSITION, 0)]),
        &[
            (OWNER, Int(0)),
            (DISPOSITION, Int(2)),
            (
                COMMITTED,
                if broken && kind == 5 {
                    eq(RESTART, 1).ite(Int(1), Var(COMMITTED))
                } else {
                    Var(COMMITTED)
                },
            ),
        ],
    );
    let quarantine = m.actions.len();
    action(
        &mut m,
        "QuarantineUnproven",
        all([eq(CANCEL, 1), eq(CHILD, 1), eq(DISPOSITION, 0)]),
        &[(DISPOSITION, Int(3))],
    );
    safe(
        &mut m,
        "ChildRetainsExactOwner",
        implies(eq(CHILD, 1), eq(OWNER, 1)),
    );
    safe(
        &mut m,
        "CompletedNeedsDoneValidatedAndReaped",
        implies(
            eq(COMMITTED, 1),
            all([eq(DONE, 1), eq(VALID, 1), eq(EXIT, 1)]),
        ),
    );
    safe(
        &mut m,
        "UnknownOutcomeNeverRedispatched",
        Var(HASHES).le(Int(1)),
    );
    no_bad(&mut m, BAD);
    m.responses.push(Response {
        name: "AdmittedHasExplicitDisposition".into(),
        request: eq(ADMITTED, 1),
        response: ne(DISPOSITION, 0),
    });
    m.weak_fair
        .extend([vec![deadline], vec![stopped], vec![quarantine]]);
    m
}

// Decision d93377961364581: credits cover requested capacities/retained
// application entries, not opaque serde/SQLite/native allocation or sampled heap.
const CREDIT_NAMES: [&str; 5] = [
    "TermDreamC1DeclaredCredit",
    "TermDreamC1DeclaredCreditControl",
    "TermDreamC1DeclaredCreditTransfer",
    "TermDreamC1DeclaredCreditGrowth",
    "TermDreamC1DeclaredCreditIsolation",
];

fn declared_credit(kind: usize, broken: bool) -> Model {
    // One job, parent and child. Units are MiB, with exact production shares:
    // parent 22 work + 8 helper allowance + 2 protected control = 32;
    // child 78 work + 16 helper allowance + 2 protected control = 96.
    // Three fixed requested capacities exercise old+new overlap: two fit each
    // work share, the third must refuse. Credit/transfer/drop are atomic here.
    // Buffer phase 0 absent, 1 snapshot owner, 2 other owner; charge is separate.
    const WIDTH: usize = 8;
    let mut m = model(
        CREDIT_NAMES[kind],
        broken,
        &[
            ("parent_reserved", 33, 0),
            ("parent_control", 2, 2),
            ("parent_charge_old", 1, 0),
            ("parent_buffer_old", 2, 0),
            ("parent_charge_new", 1, 0),
            ("parent_buffer_new", 2, 0),
            ("parent_charge_growth", 1, 0),
            ("parent_buffer_growth", 2, 0),
            ("child_reserved", 117, 0),
            ("child_control", 2, 2),
            ("child_charge_old", 1, 0),
            ("child_buffer_old", 2, 0),
            ("child_charge_new", 1, 0),
            ("child_buffer_new", 2, 0),
            ("child_charge_growth", 1, 0),
            ("child_buffer_growth", 2, 0),
        ],
    );
    let mut live_sides = Vec::new();
    for (side, label, capacity, work, helper, ceiling) in
        [(0, "Parent", 11, 22, 8, 32), (1, "Child", 39, 78, 16, 96)]
    {
        let i = side * WIDTH;
        let mut live = Int(0);
        let mut charged = Int(0);
        for (slot, name) in [(0, "Old"), (1, "New"), (2, "Growth")] {
            let charge = i + 2 + slot * 2;
            let buffer = charge + 1;
            live = live.plus(ne(buffer, 0).ite(Int(capacity), Int(0)));
            charged = charged.plus(eq(charge, 1).ite(Int(capacity), Int(0)));
            action(
                &mut m,
                &format!("Reserve{label}{name}"),
                all([
                    eq(charge, 0),
                    eq(buffer, 0),
                    if broken && kind == 4 && side == 0 {
                        Bool(true)
                    } else {
                        Var(i).plus(Int(capacity)).le(Int(work))
                    },
                ]),
                &[(charge, Int(1)), (i, Var(i).plus(Int(capacity)))],
            );
            action(
                &mut m,
                &format!("Allocate{label}{name}"),
                all([
                    eq(buffer, 0),
                    if broken && kind == 0 {
                        Bool(true)
                    } else {
                        eq(charge, 1)
                    },
                ]),
                &[(buffer, Int(1))],
            );
            action(
                &mut m,
                &format!("Transfer{label}{name}"),
                eq(buffer, 1),
                &[
                    (buffer, Int(2)),
                    (
                        charge,
                        if broken && kind == 2 {
                            Int(0)
                        } else {
                            Var(charge)
                        },
                    ),
                    (
                        i,
                        if broken && kind == 2 {
                            eq(charge, 1).ite(Var(i).minus(Int(capacity)), Var(i))
                        } else {
                            Var(i)
                        },
                    ),
                ],
            );
            action(
                &mut m,
                &format!("Drop{label}{name}"),
                ne(buffer, 0),
                &[
                    (buffer, Int(0)),
                    (charge, Int(0)),
                    (i, eq(charge, 1).ite(Var(i).minus(Int(capacity)), Var(i))),
                ],
            );
            action(
                &mut m,
                &format!("FallibleBuildRejected{label}{name}"),
                all([eq(buffer, 0), eq(charge, 1)]),
                &[(charge, Int(0)), (i, Var(i).minus(Int(capacity)))],
            );
            safe(
                &mut m,
                &format!("{label}{name}RetainsChargeThroughTransfer"),
                implies(ne(buffer, 0), eq(charge, 1)),
            );
        }
        action(
            &mut m,
            &format!("Grow{label}WithOldAndNewLive"),
            all([
                ne(i + 3, 0),
                ne(i + 5, 0),
                eq(i + 7, 0),
                eq(i + 6, 0),
                if broken && kind == 3 {
                    Bool(true)
                } else {
                    Var(i).plus(Int(capacity)).le(Int(work))
                },
            ]),
            &[
                (i + 7, Int(1)),
                (i + 6, if broken && kind == 3 { Int(0) } else { Int(1) }),
                (
                    i,
                    if broken && kind == 3 {
                        Var(i)
                    } else {
                        Var(i).plus(Int(capacity))
                    },
                ),
            ],
        );
        action(
            &mut m,
            &format!("Borrow{label}ProtectedControlForWork"),
            Bool(broken && kind == 1).and(eq(i + 1, 2)),
            &[(i + 1, Int(0))],
        );
        safe(
            &mut m,
            &format!("{label}RequestedLiveCapacityCovered"),
            live.clone().le(Var(i)),
        );
        safe(
            &mut m,
            &format!("{label}CreditCounterEqualsOwnedCharges"),
            Var(i).eq(charged),
        );
        safe(
            &mut m,
            &format!("{label}WorkDoesNotBorrowOtherShares"),
            Var(i).le(Int(work)),
        );
        safe(
            &mut m,
            &format!("{label}ProtectedControlRemainsProvisioned"),
            eq(i + 1, 2),
        );
        safe(
            &mut m,
            &format!("{label}DeclaredShareCeiling"),
            Var(i).plus(Int(helper + 2)).le(Int(ceiling)),
        );
        live_sides.push(live);
    }
    safe(
        &mut m,
        "CombinedDeclaredCreditsAtMost128MiB",
        Var(0).plus(Var(8)).plus(Int(8 + 16 + 2 + 2)).le(Int(128)),
    );
    safe(
        &mut m,
        "CombinedRequestedLiveStorageCoveredByOwnShares",
        live_sides
            .remove(0)
            .plus(live_sides.remove(0))
            .plus(Int(28))
            .le(Int(128)),
    );
    // No allocation/OS fairness assumption. Each admitted job has its own copy
    // of this ledger; a separate scheduling model bounds jobs and owner counts.
    m
}

const PUBLIC_NAMES: [&str; 3] = [
    "TermDreamC1PublicFenceAdmission",
    "TermDreamC1PublicFenceABA",
    "TermDreamC1PublicFenceCustody",
];
fn public_fence(kind: usize, broken: bool) -> Model {
    // Durable authority A1/B2/A3 uses monotonic revisions 0/1/2; A is merely a
    // repeated display label. 3 means no published/captured public proof.
    // A public refresh is successful verification, not a ready/signing flag.
    const HEAD: usize = 0;
    const PUBLISHED: usize = 1;
    const PROOF: usize = 2;
    const CAPTURED: usize = 3;
    const ADMITTED: usize = 4;
    const COMPLETE: usize = 5;
    const CANCEL: usize = 6;
    const BAD: usize = 7;
    const CUSTODY: usize = 8;
    let mut m = model(
        PUBLIC_NAMES[kind],
        broken,
        &[
            ("durable_public_revision", 2, 0),
            ("published_revision", 3, 3),
            ("public_proof_valid", 1, 1),
            ("captured_revision", 3, 3),
            ("admitted", 1, 0),
            ("complete", 1, 0),
            ("cancel", 1, 0),
            ("bad", 1, 0),
            ("private_custody_calls", 1, 0),
        ],
    );
    action(
        &mut m,
        "VerifyCurrentBoundedPublicProof",
        eq(PROOF, 1),
        &[(PUBLISHED, Var(HEAD))],
    );
    let current = all([eq(PROOF, 1), Var(PUBLISHED).eq(Var(HEAD))]);
    action(
        &mut m,
        "AdmitFromHostPublicFence",
        all([
            eq(ADMITTED, 0),
            if broken && kind == 0 {
                Bool(true)
            } else {
                current.clone()
            },
        ]),
        &[
            (ADMITTED, Int(1)),
            (CAPTURED, Var(PUBLISHED)),
            (BAD, current.clone().ite(Var(BAD), Int(1))),
        ],
    );
    for (name, from, to, valid) in [
        ("RevokeOrCutoverToB", 0, 1, 0),
        ("RestoreLabelAWithNewRevision", 1, 2, 1),
    ] {
        action(
            &mut m,
            name,
            eq(HEAD, from),
            &[(HEAD, Int(to)), (PROOF, Int(valid)), (PUBLISHED, Int(3))],
        );
    }
    action(
        &mut m,
        "ExpirePublicGrant",
        eq(PROOF, 1),
        &[(PROOF, Int(0)), (PUBLISHED, Int(3))],
    );
    let exact = current.clone().and(Var(CAPTURED).eq(Var(HEAD)));
    action(
        &mut m,
        "NextExpensiveUnit",
        all([
            eq(ADMITTED, 1),
            eq(CANCEL, 0),
            eq(COMPLETE, 0),
            if broken && kind == 1 {
                current.clone().and(
                    Var(CAPTURED)
                        .eq(Var(HEAD))
                        .or(all([eq(CAPTURED, 0), eq(HEAD, 2)])),
                )
            } else {
                exact.clone()
            },
        ]),
        &[
            (COMPLETE, Int(1)),
            (BAD, exact.clone().ite(Var(BAD), Int(1))),
        ],
    );
    action(
        &mut m,
        "PresentImmutableStatusPage",
        all([
            eq(COMPLETE, 1),
            if broken && kind == 1 {
                current.clone().and(
                    Var(CAPTURED)
                        .eq(Var(HEAD))
                        .or(all([eq(CAPTURED, 0), eq(HEAD, 2)])),
                )
            } else {
                exact.clone()
            },
        ]),
        &[(BAD, exact.clone().ite(Var(BAD), Int(1)))],
    );
    action(
        &mut m,
        "AuthorityMonitorRequestsCancellation",
        all([eq(ADMITTED, 1), exact.negate()]),
        &[(CANCEL, Int(1))],
    );
    action(
        &mut m,
        "CredentialFallbackForMissingFence",
        all([Bool(broken && kind == 2), ne(PUBLISHED, 0)]),
        &[(CUSTODY, Int(1))],
    );
    no_bad(&mut m, BAD);
    safe(
        &mut m,
        "PublicInspectionNeverTouchesPrivateCustody",
        eq(CUSTODY, 0),
    );
    // Monitoring may be delayed, so each effect/page independently checks the
    // exact current public revision. Process ownership after cancel is checked
    // by TermDreamC1Ownership; crypto/SQLite revision durability need witnesses.
    m
}

const PROVIDER_NAMES: [&str; 3] = [
    "TermDreamC1ProviderPolling",
    "TermDreamC1ProviderAcknowledgement",
    "TermDreamC1ProviderABA",
];

fn provider_polling(kind: usize, broken: bool) -> Model {
    // Selection owns its lock, the logic owner owns seen, and isolated C1
    // controls independently validate currentness. Revisions A0/B1/A2 never
    // reuse an incarnation even when the provider name returns to A.
    let mut m = model(
        PROVIDER_NAMES[kind],
        broken,
        &[
            ("revision", 2, 0),
            ("busy", 1, 0),
            ("observed", 2, 0),
            ("seen", 2, 0),
            ("published", 2, 0),
            ("bad", 1, 0),
        ],
    );
    action(&mut m, "SelectionHeld", eq(1, 0), &[(1, Int(1))]);
    action(&mut m, "SelectionReleased", eq(1, 1), &[(1, Int(0))]);
    action(&mut m, "ProviderToB", eq(0, 0), &[(0, Int(1))]);
    action(&mut m, "ProviderBackToA", eq(0, 1), &[(0, Int(2))]);
    action(&mut m, "BusyPollRetainsCursor", eq(1, 1), &[]);
    action(
        &mut m,
        "ObserveCurrentRevision",
        eq(1, 0),
        &[(
            2,
            if broken && kind == 2 {
                eq(0, 2).ite(Int(0), Var(0))
            } else {
                Var(0)
            },
        )],
    );
    action(
        &mut m,
        "PublicationFailedRetainsCursor",
        Var(3).lt(Var(2)),
        if broken && kind == 1 {
            &[(3, Var(2))]
        } else {
            &[]
        },
    );
    action(
        &mut m,
        "PublishThenAcknowledgeExactObservation",
        Var(3).lt(Var(2)),
        &[(4, Var(2)), (3, Var(2))],
    );
    action(
        &mut m,
        "AdmittedControlWhileSelectionHeld",
        eq(1, 1),
        &[(5, if broken && kind == 0 { Int(1) } else { Var(5) })],
    );
    action(
        &mut m,
        "PresentCapturedA0",
        all([
            eq(1, 0),
            eq(2, 0),
            if broken && kind == 2 {
                ne(0, 1)
            } else {
                eq(0, 0)
            },
        ]),
        &[(5, ne(0, 0).ite(Int(1), Var(5)))],
    );
    safe(&mut m, "NoSharedOwnerLockWaitOrStalePresentation", eq(5, 0));
    safe(
        &mut m,
        "AcknowledgedOnlyAfterSuccessfulPublication",
        Var(3).le(Var(4)),
    );
    // No fairness of an external transition/lock release/publication service
    // is assumed. Nonvacuity below checks progress after those inputs succeed.
    action(&mut m, "Idle", Bool(true), &[]);
    m
}

const BOOTSTRAP_NAMES: [&str; 11] = [
    "C1BootstrapAdmission",
    "C1BootstrapParentOverlap",
    "C1BootstrapUnpaidGrant",
    "C1BootstrapUnknownCost",
    "C1BootstrapDebtReset",
    "C1BootstrapAbaPermit",
    "C1BootstrapUnknownOwner",
    "C1BootstrapPrebirthProfile",
    "C1BootstrapDuplicatePermit",
    "C1BootstrapEarlyHello",
    "C1BootstrapUnpaidStart",
];
const BOOTSTRAP_RETAINED_ACTIONS: &[&str] = &[
    "RetainActualReturnedChild",
    "RetainUnknownBirth",
    "ParentCostVerdict",
    "ParentClockFailure",
    "PayParentAfterSpanEnd",
    "PayChildAfterSpanEnd",
    "Revoke",
    "Cancel",
    "ParentEof",
    "RestartCannotReopenPipe",
    "ReadinessExpiresFromBirthOrigin",
    "ObserveActualExit",
    "SettleRefusedAfterActualExit",
    "QuarantineUnknownBirth",
    "QuarantineNoQualifyingHello",
    "IndependentStatusCancelControl",
    "IndependentOtherJobProgress",
];
fn bootstrap_admission(kind: usize, broken: bool) -> Model {
    let mut model = bootstrap_admission_reference(kind, broken);
    if broken && matches!(kind, 0 | 3 | 4) {
        // First-safety-fault projection only. The raw repaired reference is
        // independently constructed below, never recovered by stripping guards.
        let safe = all(model
            .invariants
            .iter()
            .map(|invariant| invariant.condition.clone()));
        for action in &mut model.actions {
            if !BOOTSTRAP_RETAINED_ACTIONS.contains(&action.name.as_str()) {
                action.guard = all([safe.clone(), action.guard.clone()]);
            }
        }
    }
    model
}
fn bootstrap_admission_reference(kind: usize, broken: bool) -> Model {
    // One original child/anonymous pipe/permit, epochs A1/B2/A3. CPU/rest debt
    // are abstract ms and9ms steps, not real elapsed/native/allocator proofs.
    const SOURCE: usize = 0;
    const PROFILE: usize = 1;
    const EPOCH: usize = 2;
    const CAPTURED: usize = 3;
    const FG: usize = 4;
    const CANCEL: usize = 5;
    const PREPARED: usize = 6;
    const BIRTH: usize = 7;
    const CALLS: usize = 8;
    const FLIGHT: usize = 9;
    const RETAINED: usize = 10;
    const SLOT: usize = 11;
    const KNOWN: usize = 12;
    const VERDICT: usize = 13;
    const PD: usize = 14;
    const PO: usize = 15;
    const PP: usize = 16;
    const CD: usize = 17;
    const CO: usize = 18;
    const CP: usize = 19;
    const OWNER: usize = 20; // parent0,childBootstrap1,parentPrepare2,childWork3,settlement4,none5
    const ATTEMPTS: usize = 21;
    const PERMIT: usize = 22; // none0,partial1,valid2,revoked3
    const LEASE: usize = 23;
    const ARMED: usize = 24;
    const READ: usize = 25;
    const WAITING: usize = 26;
    const SPANS: usize = 27;
    const BOOT: usize = 28;
    const HELLO: usize = 29;
    const RETIRED: usize = 30;
    const PPREP: usize = 31;
    const START: usize = 32;
    const WORK: usize = 33;
    const EXIT: usize = 34;
    const VALID: usize = 35;
    const SETTLED: usize = 36;
    const QUARANTINE: usize = 37;
    const EXPIRED: usize = 38;
    const BAD: usize = 39;
    const CONTROL: usize = 40;
    const OTHER: usize = 41;
    let nine = |value: Expr| (0..9).fold(Int(0), |sum, _| sum.plus(value.clone()));
    let mut m = model(
        BOOTSTRAP_NAMES[kind],
        broken,
        &[
            ("source_closed", 1, 0),
            ("prebirth_profile", 1, 0),
            ("epoch_a1_b2_a3", 2, 0),
            ("bound_incarnation", 2, 0),
            ("foreground_unknown", 2, 1),
            ("cancel", 1, 0),
            ("prepared", 1, 0),
            ("birth", 2, 0),
            ("birth_attempts", 2, 0),
            ("native_inflight", 1, 0),
            ("effect_retained", 1, 0),
            ("slot", 1, 0),
            ("cost_known", 1, 0),
            ("parent_verdict", 2, 0),
            ("parent_debt", 27, 0),
            ("parent_owed", 27, 0),
            ("parent_paid", 3, 0),
            ("child_debt", 27, 0),
            ("child_owed", 27, 0),
            ("child_paid", 3, 0),
            ("expensive_owner", 5, 0),
            ("permit_attempts", 2, 0),
            ("permit", 3, 0),
            ("lease_open", 1, 0),
            ("transfer_armed", 1, 0),
            ("grant_read", 1, 0),
            ("child_waiting", 1, 0),
            ("bootstrap_spans", 2, 0),
            ("bootstrapped", 1, 0),
            ("hello", 1, 0),
            ("gate_retired", 1, 0),
            ("parent_prepared", 1, 0),
            ("start", 1, 0),
            ("child_work_span", 1, 0),
            ("actual_exit", 1, 0),
            ("validated", 1, 0),
            ("settled", 1, 0),
            ("quarantined", 1, 0),
            ("readiness_expired", 1, 0),
            ("bad", 1, 0),
            ("control_reply", 1, 0),
            ("independent_job", 2, 0),
        ],
    );
    let current = Var(EPOCH).eq(Var(CAPTURED));
    let healthy = all([current.clone(), eq(FG, 0), eq(CANCEL, 0), eq(EXPIRED, 0)]);
    action(
        &mut m,
        "CloseSource",
        all([eq(SOURCE, 0), eq(EPOCH, 0)]),
        &[(SOURCE, Int(1))],
    );
    action(
        &mut m,
        "VerifyPackagedBootstrapBeforeBirth",
        all([eq(PROFILE, 0), eq(CALLS, 0)]),
        &[(PROFILE, Int(1))],
    );
    action(
        &mut m,
        "InvalidateProvision",
        eq(PROFILE, 1),
        &[
            (PROFILE, Int(0)),
            (LEASE, Int(0)),
            (ARMED, Int(0)),
            (PERMIT, eq(ATTEMPTS, 0).ite(Var(PERMIT), Int(3))),
        ],
    );
    action(&mut m, "Quiescent", ne(FG, 0), &[(FG, Int(0))]);
    action(
        &mut m,
        "Foreground",
        ne(FG, 1),
        &[
            (FG, Int(1)),
            (LEASE, Int(0)),
            (ARMED, Int(0)),
            (PERMIT, eq(ATTEMPTS, 0).ite(Var(PERMIT), Int(3))),
            (OWNER, eq(ATTEMPTS, 0).ite(Var(OWNER), Int(4))),
        ],
    );
    action(
        &mut m,
        "UnknownForeground",
        ne(FG, 2),
        &[
            (FG, Int(2)),
            (LEASE, Int(0)),
            (ARMED, Int(0)),
            (PERMIT, eq(ATTEMPTS, 0).ite(Var(PERMIT), Int(3))),
            (OWNER, eq(ATTEMPTS, 0).ite(Var(OWNER), Int(4))),
        ],
    );
    action(
        &mut m,
        "PrepareFrozenBindingAndPipe",
        all([
            eq(SOURCE, 1),
            eq(PROFILE, 1),
            healthy.clone(),
            eq(PREPARED, 0),
            eq(OWNER, 0),
            eq(PD, 0),
        ]),
        &[
            (PREPARED, Int(1)),
            (CAPTURED, Var(EPOCH)),
            (PD, Int(9)),
            (PO, Int(9)),
            (PP, Int(0)),
        ],
    );
    let birth_law = all([
        eq(SOURCE, 1),
        eq(PROFILE, 1),
        healthy.clone(),
        eq(PREPARED, 1),
        eq(OWNER, 0),
        eq(PD, 0),
        eq(CALLS, 0),
        eq(BIRTH, 0),
    ]);
    action(
        &mut m,
        "Birth",
        all([
            eq(SOURCE, 1),
            healthy.clone(),
            eq(PREPARED, 1),
            eq(OWNER, 0),
            eq(PD, 0),
            eq(CALLS, 0),
            eq(BIRTH, 0),
            if broken && kind == 7 {
                Bool(true)
            } else {
                eq(PROFILE, 1)
            },
        ]),
        &[
            (BIRTH, Int(2)),
            (CALLS, Int(1)),
            (FLIGHT, Int(1)),
            (SLOT, Int(1)),
            (WAITING, Int(1)),
            (BAD, birth_law.ite(Var(BAD), Int(1))),
        ],
    );
    action(
        &mut m,
        "RetainActualReturnedChild",
        all([eq(BIRTH, 2), eq(FLIGHT, 1)]),
        &[(BIRTH, Int(1)), (FLIGHT, Int(0)), (RETAINED, Int(1))],
    );
    action(
        &mut m,
        "RetainUnknownBirth",
        all([eq(BIRTH, 2), eq(FLIGHT, 1)]),
        &[
            (FLIGHT, Int(0)),
            (RETAINED, Int(1)),
            (OWNER, Int(4)),
            (VERDICT, Int(2)),
        ],
    );
    action(
        &mut m,
        "ParentCostVerdict",
        all([
            eq(BIRTH, 1),
            eq(RETAINED, 1),
            eq(VERDICT, 0),
            eq(SETTLED, 0),
        ]),
        &[
            (KNOWN, Int(1)),
            (VERDICT, Int(1)),
            (PD, Int(27)),
            (PO, Int(27)),
            (PP, Int(0)),
        ],
    );
    action(
        &mut m,
        "ParentClockFailure",
        all([
            eq(BIRTH, 1),
            eq(RETAINED, 1),
            eq(VERDICT, 0),
            eq(SETTLED, 0),
        ]),
        &[
            (KNOWN, Int(0)),
            (VERDICT, Int(2)),
            (PD, Int(27)),
            (PO, Int(27)),
            (PP, Int(0)),
            (OWNER, Int(4)),
        ],
    );
    let pay_parent = m.actions.len();
    action(
        &mut m,
        "PayParentAfterSpanEnd",
        all([ne(PD, 0), eq(OWNER, 0).or(eq(OWNER, 2)).or(eq(OWNER, 4))]),
        &[(PD, Var(PD).minus(Int(9))), (PP, Var(PP).plus(Int(1)))],
    );
    let grant_law = all([
        eq(PROFILE, 1),
        healthy.clone(),
        eq(BIRTH, 1),
        eq(RETAINED, 1),
        eq(OWNER, 0),
        eq(KNOWN, 1),
        eq(VERDICT, 1),
        eq(PD, 0),
        eq(ATTEMPTS, 0),
        eq(PERMIT, 0),
    ]);
    let grant_guard = all([
        eq(PROFILE, 1),
        eq(FG, 0),
        eq(CANCEL, 0),
        eq(EXPIRED, 0),
        eq(BIRTH, 1),
        eq(RETAINED, 1),
        eq(ATTEMPTS, 0),
        eq(PERMIT, 0),
        if broken && (kind == 3 || kind == 5) {
            eq(OWNER, 0).or(eq(OWNER, 4))
        } else {
            eq(OWNER, 0)
        },
        if broken && kind == 2 {
            Bool(true)
        } else {
            eq(PD, 0)
        },
        if broken && kind == 3 {
            Bool(true)
        } else {
            all([eq(KNOWN, 1), eq(VERDICT, 1)])
        },
        if broken && kind == 5 {
            Bool(true)
        } else {
            current.clone()
        },
    ]);
    action(
        &mut m,
        "TransferBeforePermitWrite",
        grant_guard,
        &[
            (OWNER, Int(1)),
            (ARMED, Int(1)),
            (BAD, grant_law.ite(Var(BAD), Int(1))),
        ],
    );
    for (permit, label) in [
        (2, "WriteFullPermitOnce"),
        (1, "WritePartialConsumesAttempt"),
    ] {
        let write_law = all([
            eq(OWNER, 1),
            eq(ARMED, 1),
            eq(ATTEMPTS, 0),
            eq(PERMIT, 0),
            healthy.clone(),
        ]);
        action(
            &mut m,
            label,
            all([
                eq(OWNER, 1),
                eq(ARMED, 1),
                healthy.clone(),
                if broken && kind == 8 {
                    Var(ATTEMPTS).lt(Int(2))
                } else {
                    eq(ATTEMPTS, 0)
                },
            ]),
            &[
                (ATTEMPTS, Var(ATTEMPTS).plus(Int(1))),
                (PERMIT, Int(permit)),
                (LEASE, Int(if permit == 2 { 1 } else { 0 })),
                (OWNER, Int(if permit == 2 { 1 } else { 4 })),
                (ARMED, Int(if permit == 2 { 1 } else { 0 })),
                (BAD, write_law.ite(Var(BAD), Int(1))),
            ],
        );
    }
    action(
        &mut m,
        "ReadGrant",
        all([
            eq(BIRTH, 1),
            eq(PERMIT, 2),
            eq(LEASE, 1),
            eq(OWNER, 1),
            healthy.clone(),
            eq(READ, 0),
        ]),
        &[(READ, Int(1))],
    );
    let child_law = all([
        eq(PROFILE, 1),
        healthy.clone(),
        eq(BIRTH, 1),
        eq(PERMIT, 2),
        eq(LEASE, 1),
        eq(READ, 1),
        eq(OWNER, 1),
        eq(PD, 0),
        eq(KNOWN, 1),
        eq(CD, 0),
        eq(HELLO, 0),
    ]);
    for (span, label, cost) in [(1, "ChildArgvSpan", 9), (2, "ChildComputeOwnerSpan", 27)] {
        let order = if span == 1 {
            eq(SPANS, 0)
        } else {
            eq(SPANS, 1)
        };
        action(
            &mut m,
            label,
            all([
                eq(BIRTH, 1),
                eq(EXIT, 0),
                // The native call graph is one argv span followed by one
                // compute-owner span. Removing its authorization gate cannot
                // replay/reverse those program locations after a violation.
                order.clone(),
                if broken && kind == 0 {
                    Bool(true)
                } else {
                    child_law.clone()
                },
            ]),
            &[
                (SPANS, Int(span)),
                (CD, Int(cost)),
                (CO, Int(cost)),
                (CP, Int(0)),
                (BAD, all([child_law.clone(), order]).ite(Var(BAD), Int(1))),
            ],
        );
    }
    let pay_child = m.actions.len();
    action(
        &mut m,
        "PayChildAfterSpanEnd",
        ne(CD, 0),
        &[(CD, Var(CD).minus(Int(9))), (CP, Var(CP).plus(Int(1)))],
    );
    let hello_law = all([child_law.clone(), eq(SPANS, 2), eq(CD, 0)]);
    action(
        &mut m,
        "PaidHelloRetiresReadGate",
        all([
            eq(BIRTH, 1),
            eq(PERMIT, 2),
            eq(LEASE, 1),
            eq(OWNER, 1),
            healthy.clone(),
            eq(SPANS, 2),
            eq(HELLO, 0),
            if broken && kind == 9 {
                Bool(true)
            } else {
                eq(CD, 0)
            },
        ]),
        &[
            (BOOT, Int(1)),
            (HELLO, Int(1)),
            (WAITING, Int(0)),
            (RETIRED, Int(1)),
            (BAD, hello_law.ite(Var(BAD), Int(1))),
        ],
    );
    action(
        &mut m,
        "AcceptHelloAndCloseLease",
        all([
            eq(HELLO, 1),
            eq(BOOT, 1),
            eq(CD, 0),
            eq(OWNER, 1),
            eq(PROFILE, 1),
            healthy.clone(),
        ]),
        &[(LEASE, Int(0)), (OWNER, Int(2)), (ARMED, Int(0))],
    );
    let parent_law = all([eq(OWNER, 2), healthy.clone(), eq(PD, 0), eq(PPREP, 0)]);
    action(
        &mut m,
        "ParentPrepareSpan",
        if broken && kind == 1 {
            all([eq(PPREP, 0), eq(OWNER, 2).or(eq(OWNER, 1)).or(eq(OWNER, 3))])
        } else {
            parent_law.clone()
        },
        &[
            (PPREP, Int(1)),
            (PD, Int(9)),
            (PO, Int(9)),
            (PP, Int(0)),
            (BAD, parent_law.ite(Var(BAD), Int(1))),
        ],
    );
    let start_law = all([
        eq(OWNER, 2),
        eq(HELLO, 1),
        eq(BOOT, 1),
        eq(RETIRED, 1),
        eq(PPREP, 1),
        eq(PD, 0),
        healthy.clone(),
        eq(START, 0),
    ]);
    action(
        &mut m,
        "PaidStart",
        all([
            eq(OWNER, 2),
            eq(HELLO, 1),
            eq(BOOT, 1),
            eq(RETIRED, 1),
            eq(PPREP, 1),
            healthy.clone(),
            eq(START, 0),
            if broken && kind == 10 {
                Bool(true)
            } else {
                eq(PD, 0)
            },
        ]),
        &[
            (START, Int(1)),
            (OWNER, Int(3)),
            (BAD, start_law.ite(Var(BAD), Int(1))),
        ],
    );
    action(
        &mut m,
        "ChildWorkSpan",
        all([
            eq(OWNER, 3),
            eq(START, 1),
            eq(CD, 0),
            eq(WORK, 0),
            healthy.clone(),
        ]),
        &[(WORK, Int(1)), (CD, Int(9)), (CO, Int(9)), (CP, Int(0))],
    );
    for label in ["Revoke", "Cancel", "ParentEof", "RestartCannotReopenPipe"] {
        action(
            &mut m,
            label,
            all([ne(BIRTH, 0), eq(SETTLED, 0)]),
            &[
                (LEASE, Int(0)),
                (ARMED, Int(0)),
                (PERMIT, Int(3)),
                (OWNER, Int(4)),
                (CANCEL, Int(1)),
            ],
        );
    }
    action(
        &mut m,
        "ContextToB2",
        eq(EPOCH, 0),
        &[
            (EPOCH, Int(1)),
            (LEASE, Int(0)),
            (ARMED, Int(0)),
            (PERMIT, eq(ATTEMPTS, 0).ite(Var(PERMIT), Int(3))),
            (OWNER, eq(BIRTH, 0).ite(Var(OWNER), Int(4))),
        ],
    );
    action(&mut m, "ContextBackToA3", eq(EPOCH, 1), &[(EPOCH, Int(2))]);
    action(
        &mut m,
        "ReadinessExpiresFromBirthOrigin",
        all([eq(CALLS, 1), eq(HELLO, 0), eq(EXPIRED, 0)]),
        &[(EXPIRED, Int(1)), (LEASE, Int(0)), (ARMED, Int(0))],
    );
    action(
        &mut m,
        "ObserveActualExit",
        all([eq(BIRTH, 1), eq(EXIT, 0)]),
        &[
            (EXIT, Int(1)),
            (WAITING, Int(0)),
            (LEASE, Int(0)),
            (OWNER, Int(4)),
        ],
    );
    action(
        &mut m,
        "ValidateImmutableOutput",
        all([
            eq(EXIT, 1),
            eq(START, 1),
            eq(WORK, 1),
            eq(PD, 0),
            eq(CD, 0),
            healthy.clone(),
            eq(VALID, 0),
            eq(SETTLED, 0),
        ]),
        &[(VALID, Int(1)), (PD, Int(9)), (PO, Int(9)), (PP, Int(0))],
    );
    action(
        &mut m,
        "Complete",
        all([
            eq(EXIT, 1),
            eq(VALID, 1),
            eq(PD, 0),
            eq(CD, 0),
            healthy.clone(),
            eq(SETTLED, 0),
        ]),
        &[(SETTLED, Int(1)), (SLOT, Int(0)), (OWNER, Int(5))],
    );
    let settle = m.actions.len();
    action(
        &mut m,
        "SettleRefusedAfterActualExit",
        all([
            eq(EXIT, 1),
            eq(PD, 0),
            eq(CD, 0),
            eq(SETTLED, 0),
            eq(VALID, 0),
            ne(VERDICT, 0),
            eq(VERDICT, 2)
                .or(eq(CANCEL, 1))
                .or(eq(EXPIRED, 1))
                .or(eq(START, 0))
                .or(eq(WORK, 0))
                .or(current.clone().negate()),
        ]),
        &[(SETTLED, Int(1)), (SLOT, Int(0)), (OWNER, Int(5))],
    );
    let unknown = m.actions.len();
    action(
        &mut m,
        "QuarantineUnknownBirth",
        all([eq(BIRTH, 2), eq(QUARANTINE, 0)]),
        &[(QUARANTINE, Int(1)), (OWNER, Int(4)), (RETAINED, Int(1))],
    );
    let known = m.actions.len();
    action(
        &mut m,
        "QuarantineNoQualifyingHello",
        all([eq(BIRTH, 1), eq(HELLO, 0), eq(EXIT, 0), eq(QUARANTINE, 0)]),
        &[
            (QUARANTINE, Int(1)),
            (OWNER, Int(4)),
            (LEASE, Int(0)),
            (EXPIRED, Int(1)),
        ],
    );
    action(
        &mut m,
        "BrokenResetDebt",
        all([Bool(broken && kind == 4), ne(PD, 0)]),
        &[(PD, Int(0)), (BAD, Int(1))],
    );
    action(
        &mut m,
        "BrokenReleaseUnknownBirth",
        all([Bool(broken && kind == 6), eq(BIRTH, 2)]),
        &[(SLOT, Int(0)), (RETAINED, Int(0)), (OWNER, Int(5))],
    );
    let controls = m.actions.len();
    action(
        &mut m,
        "IndependentStatusCancelControl",
        Bool(true),
        &[(CONTROL, Int(1).minus(Var(CONTROL)))],
    );
    action(
        &mut m,
        "IndependentOtherJobProgress",
        Var(OTHER).lt(Int(2)),
        &[(OTHER, Var(OTHER).plus(Int(1)))],
    );
    safe(&mut m, "AllBirthPermitOwnerAndHandoffGuards", eq(BAD, 0));
    safe(
        &mut m,
        "OneOriginalBirthAndOneConsumedPermitAttempt",
        all([Var(CALLS).le(Int(1)), Var(ATTEMPTS).le(Int(1))]),
    );
    safe(
        &mut m,
        "UnknownBirthKeepsCustodyAndSlot",
        implies(eq(BIRTH, 2), eq(SLOT, 1)),
    );
    safe(
        &mut m,
        "KnownLiveChildKeepsSlot",
        implies(all([eq(BIRTH, 1), eq(EXIT, 0)]), eq(SLOT, 1)),
    );
    safe(
        &mut m,
        "FullParentDebtCannotReset",
        Var(PO).le(Var(PD).plus(nine(Var(PP)))),
    );
    safe(
        &mut m,
        "FullChildDebtCannotReset",
        Var(CO).le(Var(CD).plus(nine(Var(CP)))),
    );
    safe(
        &mut m,
        "SettledKnownChildRequiresActualExitAndPaidDebt",
        implies(eq(SETTLED, 1), all([eq(EXIT, 1), eq(PD, 0), eq(CD, 0)])),
    );
    m.responses.push(Response {
        name: "ControlIndependentOfBirthDebtPermit".into(),
        request: eq(CONTROL, 0),
        response: eq(CONTROL, 1),
    });
    m.responses.push(Response {
        name: "UnknownBirthGetsHonestQuarantine".into(),
        request: eq(BIRTH, 2),
        response: eq(QUARANTINE, 1).or(eq(BIRTH, 1)),
    });
    m.responses.push(Response {
        name: "MissingQualifyingHelloGetsDisposition".into(),
        request: all([eq(BIRTH, 1), eq(HELLO, 0), eq(EXIT, 0)]),
        response: eq(HELLO, 1).or(eq(EXIT, 1)).or(eq(QUARANTINE, 1)),
    });
    // Deadline/quarantine observations abstract their due instant; actual
    //250ms same-origin and2s+2s limits remain process evidence. No fair OS
    // return, permit delivery, foreground recovery or actual exit is assumed.
    m.weak_fair.extend([
        vec![pay_parent],
        vec![pay_child],
        vec![controls],
        vec![settle],
        vec![unknown],
        vec![known],
    ]);

    m
}

const PROBE_NAMES: [&str; 3] = [
    "C1BootstrapProbeContext",
    "C1BootstrapProbeStart",
    "C1BootstrapProbeOmittedContext",
];
fn bootstrap_probe(kind: usize, broken: bool) -> Model {
    // Canonical context input shape only; no private bytes/permit layout is
    // independently defined here. Full cryptographic vectors are producer-owned.
    let mut m = model(
        PROBE_NAMES[kind],
        broken,
        &[
            ("probe", 1, 0),
            ("context", 2, 0),
            ("profile", 1, 0),
            ("birth", 1, 0),
            ("hello", 1, 0),
            ("start", 1, 0),
            ("source_read", 1, 0),
            ("exited", 1, 0),
            ("owner", 1, 0),
            ("bad", 1, 0),
        ],
    );
    action(
        &mut m,
        "ProbeExplicitNull",
        eq(3, 0),
        &[(0, Int(1)), (1, Int(1))],
    );
    action(
        &mut m,
        "RealFullContext",
        eq(3, 0),
        &[(0, Int(0)), (1, Int(2))],
    );
    action(
        &mut m,
        "RealNullContext",
        eq(3, 0),
        &[(0, Int(0)), (1, Int(1))],
    );
    action(&mut m, "OmittedContext", eq(3, 0), &[(1, Int(0))]);
    action(&mut m, "PrebirthKnownProfile", eq(3, 0), &[(2, Int(1))]);
    let context = eq(0, 1).ite(eq(1, 1), eq(1, 2));
    action(
        &mut m,
        "BirthFromFrozenBinding",
        all([
            eq(2, 1),
            eq(3, 0),
            if broken && kind == 0 {
                Bool(true)
            } else if broken && kind == 2 {
                context.clone().or(eq(1, 0))
            } else {
                context.clone()
            },
        ]),
        &[(3, Int(1)), (8, Int(1)), (9, context.ite(Var(9), Int(1)))],
    );
    action(
        &mut m,
        "PaidBootstrapHello",
        all([eq(3, 1), eq(4, 0), eq(7, 0), eq(8, 1)]),
        &[(4, Int(1))],
    );
    action(
        &mut m,
        "MatchingOrHostileStart",
        all([eq(4, 1), eq(5, 0), eq(7, 0), eq(8, 1)]),
        &[
            (
                5,
                if broken && kind == 1 {
                    Int(1)
                } else {
                    eq(0, 1).ite(Int(0), Int(1))
                },
            ),
            (
                6,
                if broken && kind == 1 {
                    Int(1)
                } else {
                    eq(0, 1).ite(Int(0), Int(1))
                },
            ),
        ],
    );
    action(
        &mut m,
        "ActualExitJoinAndReap",
        eq(3, 1),
        &[(7, Int(1)), (8, Int(0))],
    );
    safe(&mut m, "ExplicitNullOnlyForProbeBeforeBirth", eq(9, 0));
    safe(
        &mut m,
        "ProbePermanentlyRefusesEveryStartSourceFree",
        implies(eq(0, 1), all([eq(5, 0), eq(6, 0)])),
    );
    // No fair Hello/delivery/OS exit. Real owned join/reap is required evidence.
    m
}

const FINAL_NAMES: [&str; 6] = [
    "C1FinalPublicAdmission",
    "C1FinalPublicPayload",
    "C1FinalPublicBounds",
    "C1FinalPublicCache",
    "C1FinalPublicRepeatedCheck",
    "C1FinalPublicRecursiveDebt",
];
fn final_public_admission(kind: usize, broken: bool) -> Model {
    // Two final sites only. Prepared public cache exists before initial27ms
    // spawn debt; no connection construction or payload data belongs to control.
    const SITE: usize = 0;
    const DEBT: usize = 1;
    const OWED: usize = 2;
    const PAID: usize = 3;
    const EPOCH: usize = 4;
    const HELD: usize = 5;
    const CPU: usize = 6;
    const CLOCK: usize = 7;
    const CLASS: usize = 8;
    const ROWS: usize = 9;
    const CELL: usize = 10;
    const TOTAL: usize = 11;
    const PC: usize = 12;
    const SC: usize = 13;
    const COUNT: usize = 14;
    const EFFECT: usize = 15;
    const REFUSED: usize = 16;
    const OWNER: usize = 17;
    const EXIT: usize = 18;
    const RELEASED: usize = 19;
    const BAD: usize = 20;
    const CONTROL: usize = 21;
    let nine = |value: Expr| (0..9).fold(Int(0), |sum, _| sum.plus(value.clone()));
    let mut m = model(
        FINAL_NAMES[kind],
        broken,
        &[
            ("site", 3, 0),
            ("work_debt", 27, 27),
            ("owed", 27, 27),
            ("paid", 3, 0),
            ("public_epoch_a1_b2_a3", 2, 0),
            ("dependency_held", 1, 0),
            ("inclusive_control_cpu", 2, 1),
            ("clock_known", 1, 1),
            ("control_class", 3, 0),
            ("public_rows", 8, 7),
            ("cell_kib", 17, 16),
            ("total_kib", 113, 112),
            ("permit_checks", 2, 0),
            ("start_checks", 2, 0),
            ("total_final_checks", 3, 0),
            ("real_handoff_effect", 2, 0),
            ("refused", 1, 0),
            ("child_owner_retained", 1, 1),
            ("actual_exit", 1, 0),
            ("released", 1, 0),
            ("bad", 1, 0),
            ("control_reply", 1, 0),
        ],
    );
    let pay = m.actions.len();
    action(
        &mut m,
        "PayExistingWorkDebt",
        ne(DEBT, 0),
        &[
            (DEBT, Var(DEBT).minus(Int(9))),
            (PAID, Var(PAID).plus(Int(1))),
        ],
    );
    action(
        &mut m,
        "AfterDebtPublicTamper",
        all([eq(DEBT, 0), eq(EPOCH, 0)]),
        &[(EPOCH, Int(1))],
    );
    action(&mut m, "DisplayReturnsA3", eq(EPOCH, 1), &[(EPOCH, Int(2))]);
    action(
        &mut m,
        "HoldPreparedDependency",
        eq(HELD, 0),
        &[(HELD, Int(1))],
    );
    action(&mut m, "ReleaseDependency", eq(HELD, 1), &[(HELD, Int(0))]);
    action(
        &mut m,
        "InjectTwoMsFinalControl",
        eq(CPU, 1),
        &[(CPU, Int(2))],
    );
    action(
        &mut m,
        "UnknownFinalClock",
        eq(CLOCK, 1),
        &[(CLOCK, Int(0))],
    );
    action(
        &mut m,
        "MisclassifyPayload",
        eq(CLASS, 0),
        &[(CLASS, Int(1))],
    );
    action(
        &mut m,
        "MisclassifyGenericSqliteOrSetup",
        eq(CLASS, 0),
        &[(CLASS, Int(2))],
    );
    action(
        &mut m,
        "MisclassifyPreparedConnectionSetup",
        eq(CLASS, 0),
        &[(CLASS, Int(3))],
    );
    action(
        &mut m,
        "OversizePublicCellBeforeFetch",
        eq(CELL, 16),
        &[(CELL, Int(17))],
    );
    action(
        &mut m,
        "EighthPublicRowBeforeFetch",
        eq(ROWS, 7),
        &[(ROWS, Int(8))],
    );
    action(
        &mut m,
        "ExcessTotalBeforeFetch",
        eq(TOTAL, 112),
        &[(TOTAL, Int(113))],
    );
    let valid = all([
        eq(EXIT, 0),
        eq(EPOCH, 0),
        eq(HELD, 0),
        Var(CPU).le(Int(1)),
        eq(CLOCK, 1),
        eq(CLASS, 0),
        Var(ROWS).le(Int(7)),
        Var(CELL).le(Int(16)),
        Var(TOTAL).le(Int(112)),
    ]);
    let allowed = all([
        eq(EXIT, 0),
        if broken && kind == 3 {
            Bool(true)
        } else {
            eq(EPOCH, 0)
        },
        eq(HELD, 0),
        if broken && kind == 0 {
            Bool(true)
        } else {
            all([Var(CPU).le(Int(1)), eq(CLOCK, 1)])
        },
        if broken && kind == 1 {
            Bool(true)
        } else {
            eq(CLASS, 0)
        },
        if broken && kind == 2 {
            Bool(true)
        } else {
            all([
                Var(ROWS).le(Int(7)),
                Var(CELL).le(Int(16)),
                Var(TOTAL).le(Int(112)),
            ])
        },
    ]);
    for (site, index, label, effect) in [
        (0, PC, "FinalPermitControlAndWrite", 1),
        (2, SC, "FinalStartControlAndWrite", 2),
    ] {
        action(
            &mut m,
            label,
            all([eq(SITE, site), eq(DEBT, 0), eq(index, 0), allowed.clone()]),
            &[
                (index, Int(1)),
                (COUNT, Var(COUNT).plus(Int(1))),
                (EFFECT, Int(effect)),
                (SITE, Int(if site == 0 { 1 } else { 3 })),
                (
                    DEBT,
                    if broken && kind == 5 {
                        Int(9)
                    } else {
                        Var(DEBT)
                    },
                ),
                // A mislabeled control span legally owes zero work debt.
                // Start its own finite ledger instead of carrying the three
                // payments of the prior spawn span into a fourth payment.
                (
                    OWED,
                    if broken && kind == 5 {
                        Int(0)
                    } else {
                        Var(OWED)
                    },
                ),
                (
                    PAID,
                    if broken && kind == 5 {
                        Int(0)
                    } else {
                        Var(PAID)
                    },
                ),
                (
                    BAD,
                    all([valid.clone(), Bool(!(broken && kind == 5))]).ite(Var(BAD), Int(1)),
                ),
            ],
        );
        action(
            &mut m,
            &format!("Refuse{label}"),
            all([
                eq(SITE, site),
                eq(DEBT, 0),
                eq(index, 0),
                valid.clone().negate(),
            ]),
            &[
                (index, Int(1)),
                (COUNT, Var(COUNT).plus(Int(1))),
                (SITE, Int(3)),
                (REFUSED, Int(1)),
            ],
        );
    }
    // A real write can precede its clock/CPU failure. Revoke and retain the
    // effect/Child; an over-budget verdict is never proof of absent effect.
    action(
        &mut m,
        "LateTimingFailureRetainsEffect",
        all([
            eq(SITE, 0),
            eq(DEBT, 0),
            eq(PC, 0),
            eq(EPOCH, 0),
            eq(HELD, 0),
            eq(CLASS, 0),
            eq(CPU, 2).or(eq(CLOCK, 0)),
        ]),
        &[
            (PC, Int(1)),
            (COUNT, Int(1)),
            (EFFECT, Int(1)),
            (SITE, Int(3)),
            (REFUSED, Int(1)),
        ],
    );
    action(
        &mut m,
        "PaidHelloThenOrdinaryParentPreparation",
        all([eq(SITE, 1), eq(DEBT, 0)]),
        &[
            (SITE, Int(2)),
            (DEBT, Int(9)),
            (OWED, Int(9)),
            (PAID, Int(0)),
        ],
    );
    action(
        &mut m,
        "BrokenRepeatPermitFinalCheck",
        all([Bool(broken && kind == 4), eq(PC, 1)]),
        &[(PC, Int(2)), (COUNT, Var(COUNT).plus(Int(1)))],
    );
    action(&mut m, "ActualChildExit", eq(EXIT, 0), &[(EXIT, Int(1))]);
    let settle = m.actions.len();
    action(
        &mut m,
        "ReleaseOnlyAfterActualExit",
        all([eq(EXIT, 1), eq(SITE, 3), eq(RELEASED, 0), eq(DEBT, 0)]),
        &[(OWNER, Int(0)), (RELEASED, Int(1))],
    );
    let control = m.actions.len();
    action(
        &mut m,
        "IndependentStatusCancel",
        Bool(true),
        &[(CONTROL, Int(1).minus(Var(CONTROL)))],
    );
    safe(
        &mut m,
        "OnlyExactCurrentBoundedPublicAdmissionControl",
        eq(BAD, 0),
    );
    safe(
        &mut m,
        "AtMostOneCheckPerHandoffTwoTotal",
        all([
            Var(PC).le(Int(1)),
            Var(SC).le(Int(1)),
            Var(COUNT).le(Int(2)),
        ]),
    );
    safe(
        &mut m,
        "NoRecursiveWorkDebtForFinalControl",
        Var(OWED).eq(Var(DEBT).plus(nine(Var(PAID)))),
    );
    safe(
        &mut m,
        "NoReleaseOfUnknownRealChildEffect",
        implies(eq(EXIT, 0), eq(OWNER, 1)),
    );
    m.responses.push(Response {
        name: "FinalControlNeverBlocksStatusCancel".into(),
        request: eq(CONTROL, 0),
        response: eq(CONTROL, 1),
    });
    m.weak_fair.extend([vec![pay], vec![settle], vec![control]]);
    m
}

const QUANTUM_NAMES: [&str; 7] = [
    "C1QuantumDebt",
    "C1QuantumDebtParserTag",
    "C1QuantumDebtRepeatedEffect",
    "C1QuantumDebtEffectOwner",
    "C1QuantumDebtHiddenNativeCpu",
    "C1QuantumDebtWrapper",
    "C1QuantumDebtHotOwner",
];

fn quantum_debt(kind: usize, broken: bool) -> Model {
    let nine = |value: Expr| (0..9).fold(Int(0), |sum, _| sum.plus(value.clone()));
    // Decision65086: two jobs, one selected original operation each, epochs
    // A1/B2/A3. At most two selected calls; distinct later operations remain
    // outside this bounded instance. A explores
    // all cost/effect verdicts; B uses one fixed allowed1ms native call. CPU is
    // abstract ms, wall_over=1 means >25ms; selected profiles have total<=3ms.
    // Both original jobs capture A1; B2/A3 cannot recapture their old operations.
    // The solo-A projection covers all four selected native profiles/tags and
    // cost/verdict controls. The explicit two-job projection uses the3ms full
    // process profile. B never inspects A's tag/app/native-cost fields: its
    // guards depend only on current authority, the two-call cap and its OWN
    // debt. Keeping every native-class history crossed with B's independent
    // return/Start/validation/exit history adds no new dependency to this
    // projection. The pair still explores every before/after-control boundary
    // of all three9ms A debt payments, plus B's two own debt payments.
    // Real clocks/OS calls, other simultaneous cost/tag pairs, fractional costs
    // and larger overshoots require process witnesses; this is not a refinement
    // proof of unbounded composition or arbitrary Rust.
    const EPOCH: usize = 0;
    const FG: usize = 1; // 0 quiescent,1 foreground,2 unknown
    const CANCEL: usize = 2;
    const DEADLINE: usize = 3;
    const CLOSED: usize = 4;
    const PHASE: usize = 5; // 0 ready,1 external flight,2 returned,3 verdict,4 quarantine,5 complete,6 failed
    const CAPTURED: usize = 6;
    const OWNER: usize = 7;
    const EFFECT: usize = 8; // 0 absent,1 present,2 unresolved
    const CALLS: usize = 9;
    const TAG: usize = 10; // 0 app,1 process,2 compute owner,3 no-replace,4 parser,5 SQLite,6 control
    const APP_CPU: usize = 11;
    const NATIVE_CPU: usize = 12;
    const FULL_CPU: usize = 13;
    const DEBT: usize = 14;
    const PAID: usize = 15; // completed9ms wall waits after this span's end
    const CLOCK: usize = 16;
    const WALL_OVER: usize = 17;
    const VALIDATED: usize = 18;
    const EXITED: usize = 19;
    const ACCEPTED: usize = 20;
    const RECORDED: usize = 21;
    const HOT: usize = 22;
    const B_CLOSED: usize = 23;
    const B_PHASE: usize = 24; // 0 ready,1 flight,2 returned,3 completed
    const B_CAPTURED: usize = 25;
    const B_EFFECT: usize = 26;
    const B_OWNER: usize = 27;
    const B_CALLS: usize = 28;
    const B_DEBT: usize = 29;
    const B_EXITED: usize = 30;
    const B_VALIDATED: usize = 31;
    const CONTROL: usize = 32;
    const STARTED: usize = 33;
    const B_STARTED: usize = 34;
    const BAD_START: usize = 35;
    const PAIR: usize = 36;
    let mut m = model(
        QUANTUM_NAMES[kind],
        broken,
        &[
            ("epoch_a1_b2_a3", 2, 0),
            ("foreground_unknown", 2, 1),
            ("cancel_a", 1, 0),
            ("deadline_a", 1, 0),
            ("closed_a", 1, 0),
            ("phase_a", 6, 0),
            ("captured_a", 2, 0),
            ("owner_a", 1, 0),
            ("effect_a", 2, 0),
            ("calls_a", 2, 0),
            ("tag_a", 7, 0),
            ("app_cpu_a", 2, 0),
            ("native_cpu_a", 3, 0),
            ("full_cpu_a", 3, 0),
            ("debt_a", 27, 0),
            ("paid_after_end_a", 3, 0),
            ("clock_available_a", 1, 1),
            ("native_wall_over_a", 1, 0),
            ("validated_a", 1, 0),
            ("actual_exit_a", 1, 0),
            ("accepted_a", 1, 0),
            ("cost_recorded_a", 1, 0),
            ("hot_owner_a", 1, 0),
            ("closed_b", 1, 0),
            ("phase_b", 3, 0),
            ("captured_b", 2, 0),
            ("effect_b", 2, 0),
            ("owner_b", 1, 0),
            ("calls_b", 1, 0),
            ("debt_b", 9, 0),
            ("actual_exit_b", 1, 0),
            ("validated_b", 1, 0),
            ("control_reply", 1, 0),
            ("start_handed_a", 1, 0),
            ("start_handed_b", 1, 0),
            ("start_debt_violation", 1, 0),
            ("independent_pair_projection", 1, 0),
        ],
    );
    action(
        &mut m,
        "SelectIndependentPairProjection",
        all([
            eq(PAIR, 0),
            eq(CLOSED, 0),
            eq(B_CLOSED, 0),
            eq(PHASE, 0),
            eq(CALLS, 0),
            eq(RECORDED, 0),
            eq(EPOCH, 0),
            eq(CANCEL, 0),
            eq(DEADLINE, 0),
        ]),
        &[(PAIR, Int(1))],
    );
    action(
        &mut m,
        "CloseA",
        all([eq(CLOSED, 0), eq(EPOCH, 0)]),
        &[(CLOSED, Int(1)), (CAPTURED, Var(EPOCH))],
    );
    action(
        &mut m,
        "CloseB",
        all([eq(B_CLOSED, 0), eq(EPOCH, 0), eq(PAIR, 1)]),
        &[(B_CLOSED, Int(1)), (B_CAPTURED, Var(EPOCH))],
    );
    action(&mut m, "Quiescent", ne(FG, 0), &[(FG, Int(0))]);
    action(&mut m, "Foreground", ne(FG, 1), &[(FG, Int(1))]);
    action(&mut m, "UnknownForeground", ne(FG, 2), &[(FG, Int(2))]);
    // Select one interrupt to keep the fault-state product bounded; it may
    // occur before/after the effect and before its independent verdict.
    let uninterrupted = all([
        eq(EPOCH, 0),
        eq(CANCEL, 0),
        eq(DEADLINE, 0),
        eq(CLOCK, 1),
        eq(WALL_OVER, 0),
    ]);
    action(
        &mut m,
        "ContextToB2",
        uninterrupted.clone(),
        &[(EPOCH, Int(1))],
    );
    action(&mut m, "ContextBackToA3", eq(EPOCH, 1), &[(EPOCH, Int(2))]);
    action(
        &mut m,
        "CancelA",
        uninterrupted.clone(),
        &[(CANCEL, Int(1))],
    );
    action(
        &mut m,
        "DeadlineA",
        uninterrupted.clone(),
        &[(DEADLINE, Int(1))],
    );
    action(
        &mut m,
        "SelectHotOwner",
        all([eq(PHASE, 0), eq(CALLS, 0)]),
        &[(HOT, Int(1))],
    );
    let ready = all([
        eq(CLOSED, 1),
        eq(FG, 0),
        eq(CANCEL, 0),
        eq(DEADLINE, 0),
        Var(EPOCH).eq(Var(CAPTURED)),
        eq(PHASE, 0),
    ]);
    for cpu in 0..=2 {
        action(
            &mut m,
            &format!("AdmitAppCpu{cpu}"),
            all([
                ready.clone(),
                eq(DEBT, 0),
                Bool(cpu <= 1),
                eq(PAIR, 0).or(Bool(cpu == 1)),
            ]),
            &[
                (OWNER, Int(1)),
                (TAG, Int(0)),
                (APP_CPU, Int(cpu)),
                (NATIVE_CPU, Int(0)),
                (FULL_CPU, Int(cpu)),
                (DEBT, Int(9 * cpu.max(1))),
                (PAID, Int(0)),
                (RECORDED, Int(1)),
            ],
        );
    }
    for (tag, app, native) in [(1, 1, 2), (2, 0, 1), (3, 0, 0), (3, 0, 3)] {
        let admission = if broken && kind == 2 {
            all([
                eq(CLOSED, 1),
                eq(FG, 0),
                eq(CANCEL, 0),
                eq(DEADLINE, 0),
                Var(EPOCH).eq(Var(CAPTURED)),
                Var(CALLS).lt(Int(2)),
            ])
        } else {
            all([ready.clone(), eq(CALLS, 0)])
        };
        action(
            &mut m,
            &format!("AdmitNative{tag}App{app}Cpu{native}"),
            all([
                admission,
                eq(PAIR, 0).or(Bool(tag == 1 && app == 1 && native == 2)),
                if broken && kind == 0 {
                    Bool(true)
                } else {
                    eq(DEBT, 0)
                },
                if broken && kind == 6 {
                    Bool(true)
                } else {
                    eq(HOT, 0)
                },
                Var(CALLS).plus(Var(B_CALLS)).lt(Int(2)),
            ]),
            &[
                (PHASE, Int(1)),
                (OWNER, Int(1)),
                (EFFECT, Int(2)),
                (CALLS, Var(CALLS).plus(Int(1))),
                (TAG, Int(tag)),
                (APP_CPU, Int(app)),
                (NATIVE_CPU, Int(native)),
                (FULL_CPU, Int(app + native)),
                (RECORDED, Int(0)),
            ],
        );
    }
    for (tag, label, fault) in [
        (4, "Parser", 1),
        (7, "Cleanup", 1),
        (5, "SqliteWrapper", 5),
        (6, "ControlTickWrapper", 5),
    ] {
        action(
            &mut m,
            &format!("Mislabel{label}AsNative"),
            all([
                ready.clone(),
                eq(DEBT, 0),
                eq(CALLS, 0),
                // The explicit pair projection fixes A to the existing
                // app1/native2/full3 process profile, including in mutants.
                // Fake parser/cleanup/wrapper costs belong to solo A; B's
                // independent history cannot alter their tag/CPU verdict.
                eq(PAIR, 0),
                Bool(broken && kind == fault),
            ]),
            &[
                (PHASE, Int(1)),
                (OWNER, Int(1)),
                (EFFECT, Int(2)),
                (CALLS, Int(1)),
                (TAG, Int(tag)),
                (APP_CPU, Int(2)),
                (NATIVE_CPU, Int(0)),
                (FULL_CPU, Int(2)),
                (RECORDED, Int(0)),
            ],
        );
    }
    action(
        &mut m,
        "MissingClock",
        all([uninterrupted.clone(), eq(PHASE, 1).or(eq(PHASE, 2))]),
        &[(CLOCK, Int(0))],
    );
    action(
        &mut m,
        "NativeWallExceeds25ms",
        all([eq(PHASE, 1), uninterrupted]),
        &[(WALL_OVER, Int(1))],
    );
    for (effect, label) in [(0, "Absent"), (1, "Present")] {
        let cost = if broken && kind == 4 {
            Var(APP_CPU)
        } else {
            Var(FULL_CPU)
        };
        action(
            &mut m,
            &format!("ReturnNative{label}"),
            eq(PHASE, 1).or(eq(PHASE, 4)),
            &[
                (PHASE, Int(2)),
                (EFFECT, Int(effect)),
                (OWNER, Int(1)),
                (DEBT, cost.clone().lt(Int(1)).ite(Int(9), nine(cost))),
                (PAID, Int(0)),
                (RECORDED, Int(1)),
            ],
        );
    }
    let valid_verdict = all([
        eq(CLOCK, 1),
        eq(WALL_OVER, 0),
        Var(APP_CPU).le(Int(1)),
        Var(NATIVE_CPU).le(Int(2)),
        Var(FULL_CPU).le(Int(3)),
        eq(CANCEL, 0),
        eq(DEADLINE, 0),
        Var(EPOCH).eq(Var(CAPTURED)),
        eq(EFFECT, 1),
    ]);
    action(
        &mut m,
        "VerdictRetainsTypedEffect",
        eq(PHASE, 2),
        &[
            (PHASE, Int(3)),
            (ACCEPTED, valid_verdict.ite(Int(1), Int(0))),
            (OWNER, if broken && kind == 3 { Int(0) } else { Int(1) }),
        ],
    );
    let rest_a = m.actions.len();
    action(
        &mut m,
        "PayRestAAfterSpanEnd",
        all([eq(RECORDED, 1), ne(DEBT, 0)]),
        &[
            (DEBT, Var(DEBT).minus(Int(9))),
            (PAID, Var(PAID).plus(Int(1))),
        ],
    );
    action(
        &mut m,
        "ObserveActualExitA",
        all([ne(EFFECT, 2), eq(EFFECT, 1), ne(EXITED, 1)]),
        &[(EXITED, Int(1))],
    );
    action(
        &mut m,
        "HandStartToChildA",
        all([
            eq(PHASE, 3),
            eq(ACCEPTED, 1),
            eq(STARTED, 0),
            eq(FG, 0),
            eq(CANCEL, 0),
            eq(DEADLINE, 0),
            Var(EPOCH).eq(Var(CAPTURED)),
            if broken && kind == 0 {
                Bool(true)
            } else {
                eq(DEBT, 0)
            },
        ]),
        &[
            (STARTED, Int(1)),
            (BAD_START, eq(DEBT, 0).ite(Int(0), Int(1))),
        ],
    );
    action(
        &mut m,
        "ValidateImmutableResultA",
        all([
            eq(PHASE, 3),
            eq(ACCEPTED, 1),
            eq(STARTED, 1),
            eq(DEBT, 0),
            eq(VALIDATED, 0),
            eq(FG, 0),
            eq(CANCEL, 0),
            eq(DEADLINE, 0),
            Var(EPOCH).eq(Var(CAPTURED)),
        ]),
        &[
            (VALIDATED, Int(1)),
            (TAG, Int(0)),
            (APP_CPU, Int(1)),
            (NATIVE_CPU, Int(0)),
            (FULL_CPU, Int(1)),
            (DEBT, Int(9)),
            (PAID, Int(0)),
            (RECORDED, Int(1)),
        ],
    );
    let settle_a = m.actions.len();
    action(
        &mut m,
        "PresentA",
        all([
            eq(PHASE, 3),
            eq(ACCEPTED, 1),
            eq(VALIDATED, 1),
            eq(EXITED, 1),
            eq(DEBT, 0),
            eq(FG, 0),
            eq(CANCEL, 0),
            eq(DEADLINE, 0),
            Var(EPOCH).eq(Var(CAPTURED)),
        ]),
        &[(PHASE, Int(5)), (OWNER, Int(0))],
    );
    let fail_a = m.actions.len();
    action(
        &mut m,
        "SettleFailedA",
        all([
            eq(PHASE, 3),
            eq(ACCEPTED, 0),
            eq(DEBT, 0),
            eq(EFFECT, 0).or(eq(EXITED, 1)),
        ]),
        &[(PHASE, Int(6)), (OWNER, Int(0))],
    );
    let quarantine_a = m.actions.len();
    action(
        &mut m,
        "QuarantineUnreturnedA",
        eq(PHASE, 1),
        &[(PHASE, Int(4)), (DEADLINE, Int(1))],
    );
    action(
        &mut m,
        "AdmitIndependentNativeB",
        all([
            eq(B_CLOSED, 1),
            eq(B_PHASE, 0),
            eq(B_DEBT, 0),
            eq(FG, 0),
            Var(EPOCH).eq(Var(B_CAPTURED)),
            Var(CALLS).plus(Var(B_CALLS)).lt(Int(2)),
        ]),
        &[
            (B_PHASE, Int(1)),
            (B_EFFECT, Int(2)),
            (B_OWNER, Int(1)),
            (B_CALLS, Int(1)),
        ],
    );
    action(
        &mut m,
        "ReturnNativeB",
        eq(B_PHASE, 1),
        &[(B_PHASE, Int(2)), (B_EFFECT, Int(1)), (B_DEBT, Int(9))],
    );
    let rest_b = m.actions.len();
    action(
        &mut m,
        "PayRestBAfterSpanEnd",
        eq(B_DEBT, 9),
        &[(B_DEBT, Int(0))],
    );
    action(
        &mut m,
        "ObserveActualExitB",
        all([eq(B_PHASE, 2), eq(B_EXITED, 0)]),
        &[(B_EXITED, Int(1))],
    );
    action(
        &mut m,
        "HandStartToChildB",
        all([
            eq(B_PHASE, 2),
            eq(B_DEBT, 0),
            eq(B_STARTED, 0),
            eq(FG, 0),
            Var(EPOCH).eq(Var(B_CAPTURED)),
        ]),
        &[(B_STARTED, Int(1))],
    );
    action(
        &mut m,
        "ValidateImmutableResultB",
        all([
            eq(B_PHASE, 2),
            eq(B_STARTED, 1),
            eq(B_DEBT, 0),
            eq(B_VALIDATED, 0),
            eq(FG, 0),
            Var(EPOCH).eq(Var(B_CAPTURED)),
        ]),
        &[(B_VALIDATED, Int(1)), (B_DEBT, Int(9))],
    );
    let settle_b = m.actions.len();
    action(
        &mut m,
        "PresentB",
        all([
            eq(B_PHASE, 2),
            eq(B_EXITED, 1),
            eq(B_VALIDATED, 1),
            eq(B_DEBT, 0),
            eq(FG, 0),
            Var(EPOCH).eq(Var(B_CAPTURED)),
        ]),
        &[(B_PHASE, Int(3)), (B_OWNER, Int(0))],
    );
    let control = m.actions.len();
    action(
        &mut m,
        "IndependentControl",
        Bool(true),
        &[(CONTROL, Int(1).minus(Var(CONTROL)))],
    );
    action(
        &mut m,
        "RestartOrOriginalOperationRetryRetainsDebtAndEffect",
        Bool(true),
        &[],
    );
    safe(
        &mut m,
        "AtMostOneEffectPerOriginalOperationAndTwoSelectedCalls",
        all([
            Var(CALLS).le(Int(1)),
            Var(CALLS).plus(Var(B_CALLS)).le(Int(2)),
        ]),
    );
    safe(
        &mut m,
        "NoNativeCallOnHotOwnerOrFakeTag",
        implies(
            eq(PHASE, 1),
            all([eq(HOT, 0), Int(1).le(Var(TAG)), Var(TAG).le(Int(3))]),
        ),
    );
    safe(
        &mut m,
        "StrictApplicationCpuWithoutWrapperException",
        Var(APP_CPU).le(Int(1)),
    );
    safe(
        &mut m,
        "KnownOrUnknownUnsettledEffectRetainsOwner",
        implies(
            eq(PHASE, 1)
                .or(eq(PHASE, 2))
                .or(eq(PHASE, 3))
                .or(eq(PHASE, 4)),
            eq(OWNER, 1),
        ),
    );
    safe(
        &mut m,
        "FullInclusiveCostCreatesAfterEndDebt",
        implies(
            eq(RECORDED, 1),
            Var(FULL_CPU)
                .lt(Int(1))
                .ite(Int(9), nine(Var(FULL_CPU)))
                .le(Var(DEBT).plus(nine(Var(PAID)))),
        ),
    );
    safe(
        &mut m,
        "NoAdmissionDuringPriorDebt",
        implies(eq(PHASE, 1), eq(DEBT, 0)),
    );
    safe(&mut m, "NoStartBeforeParentDebtPaid", eq(BAD_START, 0));
    safe(
        &mut m,
        "AcceptedNativeHasExactCpuWallAndClockEvidence",
        implies(
            eq(ACCEPTED, 1),
            all([Var(NATIVE_CPU).le(Int(2)), eq(WALL_OVER, 0), eq(CLOCK, 1)]),
        ),
    );
    safe(
        &mut m,
        "CompletedNeedsCurrentValidatedActualExitAndPaidDebt",
        implies(
            eq(PHASE, 5),
            all([eq(VALIDATED, 1), eq(EXITED, 1), eq(DEBT, 0)]),
        ),
    );
    safe(
        &mut m,
        "SecondUnresolvedEffectRetainsIndependentOwner",
        implies(eq(B_PHASE, 1).or(eq(B_PHASE, 2)), eq(B_OWNER, 1)),
    );
    m.responses.push(Response {
        name: "RequestedControlEventuallyReplies".into(),
        request: eq(CONTROL, 0),
        response: eq(CONTROL, 1),
    });
    m.responses.push(Response {
        name: "StoppedExternalCallGetsHonestQuarantine".into(),
        request: eq(PHASE, 1),
        response: ne(PHASE, 1),
    });
    // Quarantine is the finite abstraction of the internal deadline/settlement
    // observation becoming due. It never forces OS return or releases custody;
    // the quantitative30s/2s+2s timing still needs real process witnesses.
    // Internal debt payment/control/disposition only. No fairness of native
    // Return/exit/validation, external quiescence or authority restoration.
    m.weak_fair.extend([
        vec![rest_a],
        vec![rest_b],
        vec![control],
        vec![quarantine_a],
        vec![settle_a],
        vec![fail_a],
        vec![settle_b],
    ]);
    m
}

const CLOSED_EXIT_NAMES: [&str; 2] = [
    "TermDreamC1ClosedExitActivity",
    "TermDreamC1ClosedExitLateCallback",
];

fn closed_exit_activity(kind: usize, broken: bool) -> Model {
    // One process binding A may retain a live fresh session B, or exit. A fresh
    // generation owns a different packed word. Two revisions bound races; no
    // external process exit, callback, source closure or poll fairness is assumed.
    const GEN: usize = 0;
    const SESSION: usize = 1;
    const CLOSED: usize = 2;
    const ABSENT: usize = 3;
    const REV: usize = 4;
    const FG: usize = 5;
    const TERMINAL: usize = 6;
    const CAPTURED: usize = 7;
    const CAPTURED_VALID: usize = 8;
    let mut m = model(
        CLOSED_EXIT_NAMES[kind],
        broken,
        &[
            ("generation", 1, 0),
            ("session", 1, 0),
            ("durable_current_close", 1, 0),
            ("exact_process_absent", 1, 0),
            ("packed_revision", 2, 0),
            ("foreground_or_unknown", 1, 1),
            ("terminal_closed_exit", 1, 0),
            ("captured_revision", 2, 0),
            ("capture_valid", 1, 0),
        ],
    );
    action(
        &mut m,
        "ObserveCurrentDurableClose",
        all([eq(GEN, 0), eq(SESSION, 0), eq(CLOSED, 0)]),
        &[(CLOSED, Int(1))],
    );
    // This action deliberately supplies absence without any closure authority.
    action(
        &mut m,
        "NativeEofOrExactExit",
        eq(ABSENT, 0),
        &[(ABSENT, Int(1))],
    );
    action(
        &mut m,
        "RetainedLiveSessionB",
        all([
            eq(GEN, 0),
            eq(SESSION, 0),
            eq(ABSENT, 0),
            Var(REV).lt(Int(2)),
        ]),
        &[
            (SESSION, Int(1)),
            (CLOSED, Int(0)),
            (FG, Int(1)),
            (REV, Var(REV).plus(Int(1))),
        ],
    );
    action(
        &mut m,
        "CapturePackedRevision",
        all([eq(GEN, 0), eq(CAPTURED_VALID, 0)]),
        &[(CAPTURED, Var(REV)), (CAPTURED_VALID, Int(1))],
    );
    action(
        &mut m,
        "RichUnknownOrTurnCallback",
        all([
            eq(GEN, 0),
            Var(REV).lt(Int(2)),
            if broken && kind == 1 {
                Bool(true)
            } else {
                eq(TERMINAL, 0)
            },
        ]),
        &[(REV, Var(REV).plus(Int(1))), (FG, Int(1))],
    );
    action(
        &mut m,
        "SealClosedExitedBinding",
        all([
            eq(GEN, 0),
            eq(SESSION, 0),
            eq(CAPTURED_VALID, 1),
            Var(REV).eq(Var(CAPTURED)),
            eq(ABSENT, 1),
            eq(TERMINAL, 0),
            Var(REV).lt(Int(2)),
            if broken && kind == 0 {
                Bool(true)
            } else {
                eq(CLOSED, 1)
            },
        ]),
        &[
            (TERMINAL, Int(1)),
            (FG, Int(0)),
            (REV, Var(REV).plus(Int(1))),
        ],
    );
    action(
        &mut m,
        "FreshGenerationOwnsDifferentWord",
        eq(GEN, 0),
        &[
            (GEN, Int(1)),
            (SESSION, Int(1)),
            (CLOSED, Int(0)),
            (ABSENT, Int(0)),
            (REV, Int(0)),
            (FG, Int(1)),
            (TERMINAL, Int(0)),
        ],
    );
    action(&mut m, "StatusOrDelayedFollowerTick", Bool(true), &[]);
    safe(
        &mut m,
        "TerminalQuiescenceRequiresCurrentDurableCloseAndExactExit",
        implies(
            eq(TERMINAL, 1),
            all([eq(CLOSED, 1), eq(ABSENT, 1), eq(GEN, 0), eq(SESSION, 0)]),
        ),
    );
    safe(
        &mut m,
        "LateRichCallbackCannotRestoreUnknownAfterTerminalProof",
        implies(eq(TERMINAL, 1), eq(FG, 0)),
    );
    // Source eligibility, actual ESRCH and authority proof, OS process identity,
    // packed overflow and wire masking still require their implementation tests.
    m
}

fn retained_cleanup(broken: bool) -> Model {
    // One owned parse result: a partial collection has two cleanup chunks;
    // returned/normal results have three. A verdict never performs the drain.
    // The parse/snapshot owner and requested-capacity credits survive every
    // rejection/cancel/clock outcome until the last real cleanup step completes.
    const PHASE: usize = 0; // 0 absent,1 owned,2 draining,3 settled
    const KIND: usize = 1; // 0 absent,1 partial,2 returned,3 normal-scope value
    const LEFT: usize = 2;
    const FREED: usize = 3;
    const OWNER: usize = 4;
    const CREDIT: usize = 5;
    const VERDICT: usize = 6; // 0 pending,1 normal,2 rejected,3 cancelled,4 clock error
    let mut m = model(
        "TermDreamC1RetainedCleanup",
        broken,
        &[
            ("phase", 3, 0),
            ("owned_kind", 3, 0),
            ("chunks_left", 3, 0),
            ("chunks_freed", 3, 0),
            ("owner_retained", 1, 0),
            ("credits_retained", 1, 0),
            ("verdict", 4, 0),
        ],
    );
    for (kind, label, chunks) in [(1, "Partial", 2), (2, "Returned", 3), (3, "Normal", 3)] {
        action(
            &mut m,
            &format!("AcquireOwned{label}"),
            eq(PHASE, 0),
            &[
                (PHASE, Int(1)),
                (KIND, Int(kind)),
                (LEFT, Int(chunks)),
                (OWNER, Int(1)),
                (CREDIT, Int(1)),
            ],
        );
    }
    for (verdict, label) in [
        (1, "NormalScopeExit"),
        (2, "Reject"),
        (3, "Cancel"),
        (4, "ClockFailure"),
    ] {
        action(
            &mut m,
            &format!("{label}RetainsCleanup"),
            eq(PHASE, 1),
            &[
                (PHASE, Int(2)),
                (VERDICT, Int(verdict)),
                (OWNER, Int(if broken { 0 } else { 1 })),
                (CREDIT, Int(if broken { 0 } else { 1 })),
            ],
        );
    }
    // A later cancellation or unavailable clock cannot retire the cleanup
    // owner, restore already freed chunks, or release outstanding credits.
    for (verdict, label) in [(3, "CancelDuringDrain"), (4, "ClockFailureDuringDrain")] {
        action(
            &mut m,
            label,
            all([eq(PHASE, 2), ne(VERDICT, verdict)]),
            &[(VERDICT, Int(verdict))],
        );
    }
    let drain = m.actions.len();
    action(
        &mut m,
        "DrainOneOwnedChunk",
        all([eq(PHASE, 2), ne(LEFT, 0)]),
        &[
            (LEFT, Var(LEFT).minus(Int(1))),
            (FREED, Var(FREED).plus(Int(1))),
        ],
    );
    let settle = m.actions.len();
    action(
        &mut m,
        "SettleAfterActualDrain",
        all([eq(PHASE, 2), eq(LEFT, 0)]),
        &[(PHASE, Int(3)), (OWNER, Int(0)), (CREDIT, Int(0))],
    );
    action(&mut m, "StatusDoesNotRetireOwner", Bool(true), &[]);
    safe(
        &mut m,
        "OwnedOrDrainingKeepsOwnerAndCredits",
        implies(
            eq(PHASE, 1).or(eq(PHASE, 2)),
            all([eq(OWNER, 1), eq(CREDIT, 1)]),
        ),
    );
    safe(
        &mut m,
        "SettlementRequiresActualCompleteDrain",
        implies(
            eq(PHASE, 3),
            all([eq(LEFT, 0), eq(OWNER, 0), eq(CREDIT, 0)]),
        ),
    );
    safe(
        &mut m,
        "CleanupCannotLoseOrRepeatChunks",
        all([
            implies(eq(KIND, 1), Var(LEFT).plus(Var(FREED)).eq(Int(2))),
            implies(
                eq(KIND, 2).or(eq(KIND, 3)),
                Var(LEFT).plus(Var(FREED)).eq(Int(3)),
            ),
        ]),
    );
    m.responses.push(Response {
        name: "FairInternalCleanupEventuallySettles".into(),
        request: eq(PHASE, 2),
        response: eq(PHASE, 3),
    });
    // Only finite internal deallocation chunks and final retirement are weakly
    // fair. No external parse success, verdict, IO, OS exit or clock is assumed.
    // This proves bounded ownership order, not arbitrary Rust/CPU refinement.
    m.weak_fair.extend([vec![drain], vec![settle]]);
    m
}

// Decision66026. Private rollback-catalogue mode/transaction semantics and
// cooperative public preparation; wall time, VFS calls and native cost need the
// real cold/migration/crash/API witnesses. Account/provenance WAL is outside it.
const PERSISTENCE_NAMES: [&str; 6] = [
    "TermDreamC1CatalogueMigration",
    "TermDreamC1CatalogueAtomicReady",
    "TermDreamC1CatalogueBusy",
    "TermDreamC1PreparationUnits",
    "TermDreamC1PreparationAccounting",
    "TermDreamC1PreparationClock",
];
fn catalogue_mode(kind: usize, broken: bool) -> Model {
    // phase choose0,mode1,selected2,tx3,DDL4,marker5,ready6,refusal/crash7.
    // mode WAL0/PERSIST1; old rows abbreviate original-op/run/source/cursor bytes.
    let mut m = model(
        PERSISTENCE_NAMES[kind],
        broken,
        &[
            ("phase", 7, 0),
            ("mode", 1, 0),
            ("busy", 1, 0),
            ("existing", 1, 0),
            ("old_rows", 1, 0),
            ("ready", 1, 0),
            ("tables", 1, 0),
            ("staged", 1, 0),
            ("marker", 1, 0),
            ("control", 1, 0),
            ("busy_admitted", 1, 0),
            ("reopened", 1, 0),
        ],
    );
    action(
        &mut m,
        "SeedLegacy",
        eq(0, 0),
        &[
            (0, Int(1)),
            (3, Int(1)),
            (4, Int(1)),
            (5, Int(1)),
            (6, Int(1)),
        ],
    );
    action(&mut m, "ChooseFresh", eq(0, 0), &[(0, Int(1))]);
    action(
        &mut m,
        "HoldReaderOrWriter",
        all([ne(0, 0), ne(0, 7), eq(2, 0)]),
        &[(2, Int(1))],
    );
    action(&mut m, "ReleaseReaderOrWriter", eq(2, 1), &[(2, Int(0))]);
    action(
        &mut m,
        "SelectPersist",
        all([
            eq(0, 1),
            if broken && kind == 2 {
                Bool(true)
            } else {
                eq(2, 0)
            },
        ]),
        &[
            (0, Int(2)),
            (1, Int(1)),
            (4, if broken && kind == 0 { Int(0) } else { Var(4) }),
            (10, Var(2)),
        ],
    );
    // A cache-miss Status/Cancel can refuse a held own header before public
    // preparation. It publishes no run/authority and leaves durable rows alone.
    action(
        &mut m,
        "ProbeHeldHeaderRefusal",
        all([eq(0, 1), eq(2, 1)]),
        &[(0, Int(7))],
    );
    action(
        &mut m,
        "BusyRefusal",
        all([eq(0, 1), eq(2, 1)]),
        &[(0, Int(7))],
    );
    action(
        &mut m,
        "ExistingReady",
        all([eq(0, 2), eq(5, 1)]),
        &[(0, Int(6))],
    );
    action(
        &mut m,
        "BeginSchema",
        all([eq(0, 2), eq(5, 0), eq(2, 0)]),
        &[(0, Int(3))],
    );
    action(&mut m, "WriteDdl", eq(0, 3), &[(0, Int(4)), (7, Int(1))]);
    action(
        &mut m,
        "StageReady",
        eq(0, 4),
        &[
            (0, Int(5)),
            (8, Int(1)),
            (5, if broken && kind == 1 { Int(1) } else { Var(5) }),
        ],
    );
    action(
        &mut m,
        "CommitSchemaAndMarker",
        all([eq(0, 5), eq(2, 0)]),
        &[
            (0, Int(6)),
            (5, Int(1)),
            (6, Int(1)),
            (7, Int(0)),
            (8, Int(0)),
        ],
    );
    action(
        &mut m,
        "CrashOrRollback",
        all([Int(3).le(Var(0)), Var(0).le(Int(5))]),
        &[(0, Int(7)), (7, Int(0)), (8, Int(0))],
    );
    action(
        &mut m,
        "ExplicitReopen",
        all([eq(0, 7), eq(2, 0), eq(11, 0)]),
        &[(0, Int(1)), (11, Int(1))],
    );
    action(&mut m, "DuplicateOriginal", eq(0, 6), &[]);
    action(
        &mut m,
        "IndependentStatusCancel",
        Bool(true),
        &[(9, Int(1).minus(Var(9)))],
    );
    safe(
        &mut m,
        "LegacyRowsAndOriginalOperationPreserved",
        Var(4).eq(Var(3)),
    );
    safe(
        &mut m,
        "ReadyOnlyAfterAtomicCommit",
        implies(eq(5, 1), eq(6, 1)),
    );
    safe(&mut m, "BusyModeCannotAdmit", eq(10, 0));
    m
}
fn preparation_phases(kind: usize, broken: bool) -> Model {
    // Three representative real preparation phases, cost in1ms abstract units.
    // debt1 means a full after-end9x obligation; total never stands for maximum.
    let mut m = model(
        PERSISTENCE_NAMES[kind + 3],
        broken,
        &[
            ("phase", 5, 0),
            ("known", 1, 1),
            ("debt", 1, 0),
            ("units", 3, 0),
            ("cumulative", 3, 0),
            ("maximum", 2, 0),
            ("connection", 1, 0),
            ("cache", 1, 0),
            ("published", 1, 0),
            ("control", 1, 0),
        ],
    );
    action(
        &mut m,
        "PreparationPhase",
        all([
            Var(0).lt(Int(3)),
            eq(2, 0),
            Var(5).le(Int(1)),
            if broken && kind == 2 {
                Bool(true)
            } else {
                eq(1, 1)
            },
        ]),
        &[
            (0, Var(0).plus(Int(1))),
            (2, Int(1)),
            (3, Var(3).plus(Int(1))),
            (
                4,
                if broken && kind == 1 {
                    Int(1)
                } else {
                    Var(4).plus(Int(1))
                },
            ),
            (5, Var(5).lt(Int(1)).ite(Int(1), Var(5))),
            (6, Int(1)),
            (7, Int(1)),
        ],
    );
    if broken && kind == 0 {
        action(
            &mut m,
            "CollapsedTwoPhases",
            all([eq(0, 0), eq(2, 0)]),
            &[
                (0, Int(2)),
                (2, Int(1)),
                (3, Int(2)),
                (4, Int(2)),
                (5, Int(2)),
                (6, Int(1)),
                (7, Int(1)),
            ],
        );
    }
    action(
        &mut m,
        "UnknownOrReversedClock",
        Var(0).lt(Int(3)),
        &[(1, Int(0))],
    );
    let pay = m.actions.len();
    action(&mut m, "PayAfterEndRest", eq(2, 1), &[(2, Int(0))]);
    action(
        &mut m,
        "PublishPreparedGuard",
        all([
            eq(0, 3),
            eq(2, 0),
            if broken && kind == 2 {
                Bool(true)
            } else {
                eq(1, 1)
            },
        ]),
        &[(8, Int(1))],
    );
    action(
        &mut m,
        "RetainForCleanup",
        all([eq(6, 1), Var(0).lt(Int(4))]),
        &[(0, Int(4)), (8, Int(0))],
    );
    action(
        &mut m,
        "FinalizeCache",
        all([eq(0, 4), eq(7, 1), eq(2, 0)]),
        &[(7, Int(0)), (2, Int(1))],
    );
    action(
        &mut m,
        "CloseConnection",
        all([eq(0, 4), eq(7, 0), eq(2, 0)]),
        &[(0, Int(5)), (6, Int(0))],
    );
    action(
        &mut m,
        "IndependentStatusCancel",
        Bool(true),
        &[(9, Int(1).minus(Var(9)))],
    );
    safe(&mut m, "EveryPreparationUnitWithinOne", Var(5).le(Int(1)));
    safe(
        &mut m,
        "CumulativeCostNeverClampedToMaximum",
        Var(4).eq(Var(3)),
    );
    safe(
        &mut m,
        "UnknownClockCannotPublish",
        implies(eq(8, 1), eq(1, 1)),
    );
    safe(
        &mut m,
        "PreparedConnectionRetained",
        implies(eq(8, 1), eq(6, 1)),
    );
    m.weak_fair.push(vec![pay]);
    m
}

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    let mut cases = Vec::new();
    for kind in 0..3 {
        cases.extend([
            (catalogue_mode(kind, false), "pass"),
            (catalogue_mode(kind, true), "safety"),
            (preparation_phases(kind, false), "pass"),
            (preparation_phases(kind, true), "safety"),
        ]);
    }
    cases.push((bootstrap_admission(0, false), "pass"));
    for kind in 0..BOOTSTRAP_NAMES.len() {
        cases.push((bootstrap_admission(kind, true), "safety"));
    }
    cases.push((bootstrap_probe(0, false), "pass"));
    for kind in 0..PROBE_NAMES.len() {
        cases.push((bootstrap_probe(kind, true), "safety"));
    }
    cases.push((final_public_admission(0, false), "pass"));
    for kind in 0..FINAL_NAMES.len() {
        cases.push((final_public_admission(kind, true), "safety"));
    }
    cases.push((quantum_debt(0, false), "pass"));
    for kind in 0..QUANTUM_NAMES.len() {
        cases.push((quantum_debt(kind, true), "safety"));
    }
    for kind in 0..ADMISSION_NAMES.len() {
        cases.extend([
            (admission(kind, false), "pass"),
            (admission(kind, true), "safety"),
        ]);
    }
    for kind in 0..SCHEDULING_NAMES.len() {
        cases.extend([
            (scheduling(kind, false), "pass"),
            (
                scheduling(kind, true),
                if kind == 4 { "liveness" } else { "safety" },
            ),
        ]);
    }
    for kind in 0..OWNERSHIP_NAMES.len() {
        cases.extend([
            (ownership(kind, false), "pass"),
            (ownership(kind, true), "safety"),
        ]);
    }
    for kind in 0..CREDIT_NAMES.len() {
        cases.extend([
            (declared_credit(kind, false), "pass"),
            (declared_credit(kind, true), "safety"),
        ]);
    }
    for kind in 0..PUBLIC_NAMES.len() {
        cases.extend([
            (public_fence(kind, false), "pass"),
            (public_fence(kind, true), "safety"),
        ]);
    }
    for kind in 0..PROVIDER_NAMES.len() {
        cases.extend([
            (provider_polling(kind, false), "pass"),
            (provider_polling(kind, true), "safety"),
        ]);
    }
    for kind in 0..CLOSED_EXIT_NAMES.len() {
        cases.extend([
            (closed_exit_activity(kind, false), "pass"),
            (closed_exit_activity(kind, true), "safety"),
        ]);
    }
    cases.extend([
        (retained_cleanup(false), "pass"),
        (retained_cleanup(true), "safety"),
    ]);
    cases
}

/// Keep the finite cap and expected semantic verdict identical to the complete
/// exporter, while retaining the exact C1 case if exploration itself refuses.
#[test]
fn dream_c1_catalogue_migration_crash_busy_and_preparation_cost_controls_are_nonvacuous()
-> Result<(), String> {
    for kind in 0..3 {
        let m = catalogue_mode(kind, false);
        let r = m.check(100_000)?;
        assert!(r.violations.is_empty());
        let legacy = trace(
            &m,
            &r,
            &[
                "SeedLegacy",
                "SelectPersist",
                "ExistingReady",
                "DuplicateOriginal",
            ],
        );
        assert_eq!(r.states[legacy][4], 1);
        let busy = trace(
            &m,
            &r,
            &[
                "SeedLegacy",
                "HoldReaderOrWriter",
                "BusyRefusal",
                "IndependentStatusCancel",
            ],
        );
        assert_eq!((r.states[busy][1], r.states[busy][4]), (0, 1));
        let crash = trace(
            &m,
            &r,
            &[
                "ChooseFresh",
                "SelectPersist",
                "BeginSchema",
                "WriteDdl",
                "StageReady",
                "CrashOrRollback",
            ],
        );
        assert_eq!((r.states[crash][5], r.states[crash][6]), (0, 0));
        let recovered = trace_from(
            &m,
            &r,
            crash,
            &[
                "ExplicitReopen",
                "SelectPersist",
                "BeginSchema",
                "WriteDdl",
                "StageReady",
                "CommitSchemaAndMarker",
            ],
        );
        assert_eq!((r.states[recovered][5], r.states[recovered][6]), (1, 1));
        let bad = catalogue_mode(kind, true).check(100_000)?;
        assert!(bad.violations.iter().any(|v| v.property
            == [
                "LegacyRowsAndOriginalOperationPreserved",
                "ReadyOnlyAfterAtomicCommit",
                "BusyModeCannotAdmit"
            ][kind]));
        let m = preparation_phases(kind, false);
        let r = m.check(100_000)?;
        assert!(r.violations.is_empty());
        let ready = trace(
            &m,
            &r,
            &[
                "PreparationPhase",
                "PayAfterEndRest",
                "PreparationPhase",
                "PayAfterEndRest",
                "PreparationPhase",
                "PayAfterEndRest",
                "PublishPreparedGuard",
            ],
        );
        assert_eq!((r.states[ready][4], r.states[ready][5]), (3, 1));
        let closed = trace_from(
            &m,
            &r,
            ready,
            &[
                "RetainForCleanup",
                "FinalizeCache",
                "PayAfterEndRest",
                "CloseConnection",
            ],
        );
        assert_eq!(r.states[closed][6], 0);
        let bad = preparation_phases(kind, true).check(100_000)?;
        assert!(bad.violations.iter().any(|v| v.property
            == [
                "EveryPreparationUnitWithinOne",
                "CumulativeCostNeverClampedToMaximum",
                "UnknownClockCannotPublish"
            ][kind]));
    }
    Ok(())
}

#[test]
fn dream_c1_complete_corpus_has_named_native_verdicts() -> Result<(), String> {
    // Only bounded textual refusals survive each iteration. Complete reachable
    // graphs retire here before checking the next unchanged corpus case.
    let mut errors = Vec::new();
    for (model, expected) in cases() {
        eprintln!(
            "C1 native begin model={} state_limit=100000 expected={expected}",
            model.name
        );
        let report = match model.check(100_000) {
            Ok(report) => report,
            Err(error) => {
                let error = format!("{}: {error}", model.name);
                eprintln!("C1 native refused {error}");
                errors.push(error);
                continue;
            }
        };
        let actual = if report.violations.is_empty() {
            "pass"
        } else if report
            .violations
            .iter()
            .any(|violation| !violation.cycle.is_empty())
        {
            "liveness"
        } else if report
            .violations
            .iter()
            .any(|violation| violation.property == "Deadlock")
        {
            "deadlock"
        } else {
            "safety"
        };
        if actual != expected {
            errors.push(format!(
                "{}: native expected {expected}, got {actual}",
                model.name
            ));
        }
        eprintln!(
            "C1 native complete model={} states={} edges={} verdict={actual}",
            model.name,
            report.states.len(),
            report.edges.iter().map(Vec::len).sum::<usize>()
        );
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn step(m: &Model, r: &bboxy450_tlc::Report, state: usize, name: &str) -> Option<usize> {
    let action = m.actions.iter().position(|a| a.name == name).unwrap();
    r.edges[state]
        .iter()
        .find(|e| e.action == action)
        .map(|e| e.to)
}
fn trace(m: &Model, r: &bboxy450_tlc::Report, names: &[&str]) -> usize {
    names.iter().fold(0, |s, n| {
        step(m, r, s, n).unwrap_or_else(|| panic!("disabled {n} in {names:?}"))
    })
}

#[test]
fn dream_c1_closed_only_dedup_independent_root_and_aba_are_nonvacuous() -> Result<(), String> {
    let m = admission(0, false);
    let r = m.check(100_000)?;
    assert!(step(&m, &r, 0, "AutoRegisterA").is_none());
    let state = trace(&m, &r, &["IdleA", "TurnEndedA", "NativeEofA"]);
    assert!(step(&m, &r, state, "ManualRegisterA").is_none());
    let state = trace(
        &m,
        &r,
        &[
            "CloseA",
            "AutoRegisterA",
            "HashA",
            "RestartRetainsJournal",
            "RepeatCloseOrSelectionA",
            "RetryOriginalOperationA",
        ],
    );
    assert_eq!((r.states[state][2], r.states[state][4]), (1, 1));
    assert!(step(&m, &r, state, "ManualRegisterA").is_none());

    let m = scheduling(0, false);
    let r = m.check(100_000)?;
    let state = trace(
        &m,
        &r,
        &[
            "CloseA",
            "ProposeA",
            "QuiescentA",
            "AdmitA",
            "ForegroundA",
            "CloseB",
            "ProposeB",
            "QuiescentB",
        ],
    );
    assert!(step(&m, &r, state, "WorkUnitA").is_none());
    assert!(
        step(&m, &r, state, "AdmitB").is_some(),
        "paused A blocks independent B"
    );

    let m = ownership(0, false);
    let r = m.check(100_000)?;
    let state = trace(
        &m,
        &r,
        &[
            "AdmitOriginal",
            "HashSnapshot",
            "SpawnOwnedChild",
            "WorkerDone",
            "ValidateImmutableResult",
            "ObserveActualExit",
            "PublishCompleted",
            "ContextToB",
            "ContextBackToA",
        ],
    );
    assert!(
        step(&m, &r, state, "PresentCurrentResult").is_none(),
        "old A authority survived ABA"
    );
    Ok(())
}

#[test]
fn dream_c1_declared_credit_and_public_fence_guards_are_nonvacuous() -> Result<(), String> {
    let m = declared_credit(0, false);
    let r = m.check(100_000)?;
    assert!(step(&m, &r, 0, "AllocateParentOld").is_none());
    let state = trace(
        &m,
        &r,
        &[
            "ReserveParentOld",
            "AllocateParentOld",
            "TransferParentOld",
            "ReserveParentNew",
            "AllocateParentNew",
        ],
    );
    assert_eq!(r.states[state][0], 22);
    assert!(step(&m, &r, state, "ReserveParentGrowth").is_none());
    assert!(step(&m, &r, state, "GrowParentWithOldAndNewLive").is_none());
    let state = step(&m, &r, state, "DropParentOld").unwrap();
    assert_eq!(r.states[state][0], 11);
    assert!(step(&m, &r, state, "ReserveParentGrowth").is_some());

    let m = public_fence(0, false);
    let r = m.check(100_000)?;
    assert!(step(&m, &r, 0, "AdmitFromHostPublicFence").is_none());
    let state = trace(
        &m,
        &r,
        &[
            "VerifyCurrentBoundedPublicProof",
            "AdmitFromHostPublicFence",
            "NextExpensiveUnit",
            "RevokeOrCutoverToB",
            "RestoreLabelAWithNewRevision",
            "VerifyCurrentBoundedPublicProof",
        ],
    );
    assert!(step(&m, &r, state, "PresentImmutableStatusPage").is_none());
    Ok(())
}

#[test]
fn dream_c1_provider_busy_publication_failure_and_aba_are_nonvacuous() -> Result<(), String> {
    let m = provider_polling(0, false);
    let r = m.check(100_000)?;
    let held = trace(
        &m,
        &r,
        &[
            "SelectionHeld",
            "ProviderToB",
            "ProviderBackToA",
            "BusyPollRetainsCursor",
            "AdmittedControlWhileSelectionHeld",
        ],
    );
    assert_eq!((r.states[held][3], r.states[held][4]), (0, 0));
    let observed = step(&m, &r, held, "SelectionReleased").unwrap();
    let observed = step(&m, &r, observed, "ObserveCurrentRevision").unwrap();
    assert!(step(&m, &r, observed, "PresentCapturedA0").is_none());
    let failed = step(&m, &r, observed, "PublicationFailedRetainsCursor").unwrap();
    assert_eq!((r.states[failed][2], r.states[failed][3]), (2, 0));
    let published = step(&m, &r, failed, "PublishThenAcknowledgeExactObservation").unwrap();
    assert_eq!((r.states[published][3], r.states[published][4]), (2, 2));
    Ok(())
}

#[test]
fn dream_c1_retained_cleanup_verdicts_keep_owner_and_credits_until_real_drain() -> Result<(), String>
{
    let m = retained_cleanup(false);
    let r = m.check(100_000)?;
    for (acquire, chunks) in [
        ("AcquireOwnedPartial", 2),
        ("AcquireOwnedReturned", 3),
        ("AcquireOwnedNormal", 3),
    ] {
        for verdict in [
            "NormalScopeExitRetainsCleanup",
            "RejectRetainsCleanup",
            "CancelRetainsCleanup",
            "ClockFailureRetainsCleanup",
        ] {
            let mut state = trace(&m, &r, &[acquire, verdict]);
            assert_eq!(
                (r.states[state][2], r.states[state][4], r.states[state][5]),
                (chunks, 1, 1)
            );
            assert!(step(&m, &r, state, "SettleAfterActualDrain").is_none());
            state = step(&m, &r, state, "DrainOneOwnedChunk").unwrap();
            state = step(&m, &r, state, "StatusDoesNotRetireOwner").unwrap();
            // Late cancellation and a clock failure remain observable while
            // both the collection's owner and its capacity charge survive.
            if r.states[state][6] != 3 {
                state = step(&m, &r, state, "CancelDuringDrain").unwrap();
            }
            state = step(&m, &r, state, "ClockFailureDuringDrain").unwrap();
            for left in (0..chunks - 1).rev() {
                assert_eq!((r.states[state][4], r.states[state][5]), (1, 1));
                assert!(step(&m, &r, state, "SettleAfterActualDrain").is_none());
                state = step(&m, &r, state, "DrainOneOwnedChunk").unwrap();
                assert_eq!(r.states[state][2], left);
            }
            assert_eq!(
                (r.states[state][2], r.states[state][4], r.states[state][5]),
                (0, 1, 1)
            );
            state = step(&m, &r, state, "SettleAfterActualDrain").unwrap();
            assert_eq!(
                (r.states[state][0], r.states[state][4], r.states[state][5]),
                (3, 0, 0)
            );
        }
    }
    Ok(())
}

#[test]
fn dream_c1_closed_exit_activity_does_not_turn_eof_or_stale_close_into_quiescence()
-> Result<(), String> {
    let m = closed_exit_activity(0, false);
    let r = m.check(100_000)?;
    let bare_eof = trace(&m, &r, &["CapturePackedRevision", "NativeEofOrExactExit"]);
    assert!(step(&m, &r, bare_eof, "SealClosedExitedBinding").is_none());
    let live_b = trace(
        &m,
        &r,
        &[
            "ObserveCurrentDurableClose",
            "CapturePackedRevision",
            "RetainedLiveSessionB",
        ],
    );
    assert!(step(&m, &r, live_b, "SealClosedExitedBinding").is_none());
    let stale = trace(
        &m,
        &r,
        &[
            "ObserveCurrentDurableClose",
            "NativeEofOrExactExit",
            "CapturePackedRevision",
            "RichUnknownOrTurnCallback",
        ],
    );
    assert!(step(&m, &r, stale, "SealClosedExitedBinding").is_none());
    let terminal = trace(
        &m,
        &r,
        &[
            "ObserveCurrentDurableClose",
            "NativeEofOrExactExit",
            "CapturePackedRevision",
            "SealClosedExitedBinding",
        ],
    );
    assert_eq!((r.states[terminal][5], r.states[terminal][6]), (0, 1));
    assert!(step(&m, &r, terminal, "RichUnknownOrTurnCallback").is_none());
    let fresh = step(&m, &r, terminal, "FreshGenerationOwnsDifferentWord").unwrap();
    assert_eq!((r.states[fresh][5], r.states[fresh][6]), (1, 0));
    assert!(step(&m, &r, fresh, "SealClosedExitedBinding").is_none());
    Ok(())
}

#[test]
fn c1_quantum_debt_effect_failure_and_independent_job_are_nonvacuous() -> Result<(), String> {
    let m = quantum_debt(0, false);
    let r = m.check(100_000)?;
    eprintln!(
        "c1_quantum_debt_complete states={} edges={} state_limit=100000 projections=solo_all_profiles,pair_full_cpu3",
        r.states.len(),
        r.edges.iter().map(Vec::len).sum::<usize>()
    );
    assert!(step(&m, &r, 0, "AdmitNative1App1Cpu2").is_none());
    assert!(step(&m, &r, 0, "CloseB").is_none());
    for native in [
        "AdmitNative1App1Cpu2",
        "AdmitNative2App0Cpu1",
        "AdmitNative3App0Cpu0",
        "AdmitNative3App0Cpu3",
    ] {
        let solo = trace(&m, &r, &["CloseA", "Quiescent", native]);
        assert_eq!((r.states[solo][9], r.states[solo][36]), (1, 0));
        assert!(step(&m, &r, solo, "CloseB").is_none());
    }
    let unpaid = trace(&m, &r, &["CloseA", "Quiescent", "AdmitAppCpu1"]);
    assert_eq!(r.states[unpaid][14], 9);
    assert!(step(&m, &r, unpaid, "AdmitNative1App1Cpu2").is_none());
    assert!(step(&m, &r, unpaid, "IndependentControl").is_some());
    let returned = trace(
        &m,
        &r,
        &[
            "CloseA",
            "Quiescent",
            "AdmitNative1App1Cpu2",
            "ReturnNativePresent",
        ],
    );
    assert_eq!(
        (
            r.states[returned][8],
            r.states[returned][7],
            r.states[returned][14]
        ),
        (1, 1, 27)
    );
    let failed = step(&m, &r, returned, "MissingClock").unwrap();
    let failed = step(&m, &r, failed, "VerdictRetainsTypedEffect").unwrap();
    assert_eq!((r.states[failed][7], r.states[failed][20]), (1, 0));
    let failed = trace_from(
        &m,
        &r,
        failed,
        &[
            "PayRestAAfterSpanEnd",
            "PayRestAAfterSpanEnd",
            "PayRestAAfterSpanEnd",
        ],
    );
    assert!(step(&m, &r, failed, "SettleFailedA").is_none());
    let failed = step(&m, &r, failed, "ObserveActualExitA").unwrap();
    let failed = step(&m, &r, failed, "SettleFailedA").unwrap();
    assert_eq!((r.states[failed][5], r.states[failed][7]), (6, 0));
    for interrupt in [&["CancelA"][..], &["ContextToB2", "ContextBackToA3"][..]] {
        let interrupted = trace_from(&m, &r, returned, interrupt);
        let interrupted = step(&m, &r, interrupted, "VerdictRetainsTypedEffect").unwrap();
        assert_eq!(
            (
                r.states[interrupted][7],
                r.states[interrupted][20],
                r.states[interrupted][9]
            ),
            (1, 0, 1),
        );
    }
    for native in ["AdmitNative1App1Cpu2", "AdmitNative3App0Cpu3"] {
        let mut refused = trace(&m, &r, &["CloseA", "Quiescent", native]);
        if native == "AdmitNative1App1Cpu2" {
            refused = step(&m, &r, refused, "NativeWallExceeds25ms").unwrap();
        }
        let refused = trace_from(
            &m,
            &r,
            refused,
            &["ReturnNativePresent", "VerdictRetainsTypedEffect"],
        );
        assert_eq!(
            (
                r.states[refused][7],
                r.states[refused][20],
                r.states[refused][14]
            ),
            (1, 0, 27)
        );
    }
    let both = trace(
        &m,
        &r,
        &[
            "SelectIndependentPairProjection",
            "CloseA",
            "CloseB",
            "Quiescent",
            "AdmitNative1App1Cpu2",
            "AdmitIndependentNativeB",
        ],
    );
    assert_eq!((r.states[both][8], r.states[both][26]), (2, 2));
    let b_done = trace_from(
        &m,
        &r,
        both,
        &[
            "ReturnNativeB",
            "PayRestBAfterSpanEnd",
            "HandStartToChildB",
            "ObserveActualExitB",
            "ValidateImmutableResultB",
            "PayRestBAfterSpanEnd",
            "PresentB",
        ],
    );
    assert_eq!(
        (
            r.states[b_done][5],
            r.states[b_done][7],
            r.states[b_done][24]
        ),
        (1, 1, 3)
    );
    let quarantined = trace_from(
        &m,
        &r,
        b_done,
        &["CancelA", "QuarantineUnreturnedA", "IndependentControl"],
    );
    assert_eq!(
        (
            r.states[quarantined][5],
            r.states[quarantined][7],
            r.states[quarantined][8]
        ),
        (4, 1, 2)
    );
    let stale = trace(
        &m,
        &r,
        &["CloseA", "Quiescent", "ContextToB2", "ContextBackToA3"],
    );
    assert!(step(&m, &r, stale, "AdmitNative1App1Cpu2").is_none());
    let control = m
        .actions
        .iter()
        .position(|action| action.name == "IndependentControl")
        .unwrap();
    assert!(
        r.edges
            .iter()
            .all(|edges| edges.iter().any(|edge| edge.action == control)),
        "control disabled during debt/native/settlement"
    );
    Ok(())
}

#[test]
fn c1_fake_native_mutants_preserve_solo_cost_and_fixed_pair_projection() -> Result<(), String> {
    for (kind, labels) in [
        (1, ["MislabelParserAsNative", "MislabelCleanupAsNative"]),
        (
            5,
            [
                "MislabelSqliteWrapperAsNative",
                "MislabelControlTickWrapperAsNative",
            ],
        ),
    ] {
        let model = quantum_debt(kind, true);
        let report = model.check(100_000)?;
        assert!(report.violations.iter().any(|v| {
            v.property == "NoNativeCallOnHotOwnerOrFakeTag"
                || v.property == "StrictApplicationCpuWithoutWrapperException"
        }));
        let solo = trace(&model, &report, &["CloseA", "Quiescent"]);
        let pair = trace(
            &model,
            &report,
            &[
                "SelectIndependentPairProjection",
                "CloseA",
                "CloseB",
                "Quiescent",
            ],
        );
        for label in labels {
            let invalid = step(&model, &report, solo, label).unwrap();
            assert_eq!(
                (report.states[invalid][11], report.states[invalid][13]),
                (2, 2)
            );
            assert!(step(&model, &report, pair, label).is_none());
        }
        assert!(step(&model, &report, pair, "AdmitNative1App1Cpu2").is_some());
        assert!(step(&model, &report, pair, "AdmitIndependentNativeB").is_some());
    }
    Ok(())
}

fn trace_from(m: &Model, r: &bboxy450_tlc::Report, start: usize, names: &[&str]) -> usize {
    names.iter().fold(start, |state, name| {
        step(m, r, state, name).unwrap_or_else(|| panic!("disabled {name} from {state}"))
    })
}

// Bounded one-step queries reuse the native engine's own Expr/action semantics.
// Only a synthetic phase prevents following a second reference transition.
type LabelledSteps = std::collections::BTreeSet<(usize, String, Vec<i64>)>;
struct NativeOneStep {
    model: Model,
    width: usize,
    safety_marker: usize,
}
impl NativeOneStep {
    fn new(reference: &Model) -> Result<Self, String> {
        // Validate every original AST against its original variable mapping
        // before the appended phase could mask an out-of-range reference.
        // tla() uses the existing engine validator; no expression is evaluated
        // or reimplemented here and no generated artifact is retained.
        let _ = reference.tla()?;
        let mut model = reference.clone();
        let width = model.variables.len();
        let safety = all(model
            .invariants
            .iter()
            .map(|invariant| invariant.condition.clone()));
        model
            .variables
            .push(Variable::new("one_step_phase", 0, 1, 0));
        for action in &mut model.actions {
            action.guard = all([eq(width, 0), action.guard.clone()]);
            action.updates.push((width, Int(1)));
        }
        let safety_marker = model.actions.len();
        action(
            &mut model,
            "OneStepAllSafetyHold",
            all([eq(width, 0), safety]),
            &[(width, Int(1))],
        );
        action(&mut model, "OneStepTerminal", eq(width, 1), &[]);
        // These queries prove a transition relation, not temporal behavior.
        // Original properties/fairness are compared independently and checked
        // on the entire declared frontier graph by native/Java parity.
        model.invariants.clear();
        model.responses.clear();
        model.weak_fair.clear();
        model.strong_fair.clear();
        model.check_deadlock = false;
        Ok(Self {
            model,
            width,
            safety_marker,
        })
    }
    fn at(&mut self, state: &[i64]) -> Result<(bool, LabelledSteps), String> {
        if state.len() != self.width {
            return Err("one-step variable mapping changed".into());
        }
        for (variable, value) in self.model.variables[..self.width].iter_mut().zip(state) {
            variable.initial = *value;
            variable.also_initial.clear();
        }
        let report = self.model.check(100_000)?;
        if report.states.len() > self.safety_marker + 2 {
            return Err("one-step probe escaped its finite successor bound".into());
        }
        let mut steps = LabelledSteps::new();
        let mut safe = false;
        for edge in &report.edges[0] {
            if edge.action == self.safety_marker {
                safe = true;
            }
            if edge.action < self.safety_marker {
                steps.insert((
                    edge.action,
                    self.model.actions[edge.action].name.clone(),
                    report.states[edge.to][..self.width].to_vec(),
                ));
            }
        }
        Ok((safe, steps))
    }
}
fn frontier_header_and_dispositions(frontier: &Model, reference: &Model) -> Result<(), String> {
    // Debug is a structural comparison of the existing typed declarations,
    // never expression evaluation or a second transition semantics.
    let mut a = frontier.clone();
    let mut b = reference.clone();
    for action in a.actions.iter_mut().chain(&mut b.actions) {
        action.guard = Bool(true);
    }
    if format!("{a:?}") != format!("{b:?}") {
        return Err(format!(
            "{}: initial/bounds/labels/updates/properties/fairness mapping differs",
            frontier.name
        ));
    }
    for (actual, expected) in frontier.actions.iter().zip(&reference.actions) {
        if BOOTSTRAP_RETAINED_ACTIONS.contains(&actual.name.as_str())
            && format!("{actual:?}") != format!("{expected:?}")
        {
            return Err(format!(
                "{}: retained disposition changed {}",
                frontier.name, actual.name
            ));
        }
    }
    for group in frontier.weak_fair.iter().chain(&frontier.strong_fair) {
        for &index in group {
            if !BOOTSTRAP_RETAINED_ACTIONS.contains(&frontier.actions[index].name.as_str()) {
                return Err(format!(
                    "{}: frontier guarded a fair disposition",
                    frontier.name
                ));
            }
        }
    }
    Ok(())
}
fn same_one_step(
    candidate: &mut NativeOneStep,
    reference: &mut NativeOneStep,
    state: &[i64],
) -> Result<(), String> {
    let actual = candidate.at(state)?;
    let expected = reference.at(state)?;
    if actual != expected {
        return Err("pre-frontier labelled one-step successor mismatch".into());
    }
    Ok(())
}
#[test]
fn c1_bootstrap_frontier_preserves_every_first_fault_prefix_and_retained_disposition()
-> Result<(), String> {
    for kind in [0usize, 3, 4] {
        let frontier = bootstrap_admission(kind, true);
        let reference = bootstrap_admission_reference(kind, true);
        frontier_header_and_dispositions(&frontier, &reference)?;
        let report = frontier
            .check(100_000)
            .map_err(|error| format!("{}: {error}", frontier.name))?;
        let mut native = NativeOneStep::new(&reference)?;
        let mut safe = Vec::with_capacity(report.states.len());
        let mut compared = 0usize;
        let mut first_fault_edges = 0usize;
        for (index, state) in report.states.iter().enumerate() {
            let (before_fault, expected) = native
                .at(state)
                .map_err(|error| format!("{}: one-step state {index}: {error}", frontier.name))?;
            safe.push(before_fault);
            let actual: LabelledSteps = report.edges[index]
                .iter()
                .map(|edge| {
                    (
                        edge.action,
                        frontier.actions[edge.action].name.clone(),
                        report.states[edge.to].clone(),
                    )
                })
                .collect();
            if before_fault {
                if actual != expected {
                    return Err(format!(
                        "{}: first-fault successor mismatch at state {index}",
                        frontier.name
                    ));
                }
                compared += 1;
            } else if actual
                .iter()
                .any(|(_, name, _)| !BOOTSTRAP_RETAINED_ACTIONS.contains(&name.as_str()))
            {
                return Err(format!(
                    "{}: policy reopened after a safety fault at state {index}",
                    frontier.name
                ));
            }
            let retained = |steps: &LabelledSteps| -> LabelledSteps {
                steps
                    .iter()
                    .filter(|(_, name, _)| BOOTSTRAP_RETAINED_ACTIONS.contains(&name.as_str()))
                    .cloned()
                    .collect()
            };
            if retained(&actual) != retained(&expected) {
                return Err(format!(
                    "{}: retained disposition successor changed at state {index}",
                    frontier.name
                ));
            }
        }
        for (from, edges) in report.edges.iter().enumerate() {
            for edge in edges {
                let next = &report.states[edge.to];
                if report.states[from][39] == 1 && next[39] != 1 {
                    return Err(format!("{}: failure latch cleared", frontier.name));
                }
                if !safe[from] && safe[edge.to] {
                    return Err(format!(
                        "{}: safety/permission revived after fault",
                        frontier.name
                    ));
                }
                if frontier.actions[edge.action].name == "BrokenResetDebt" {
                    if !safe[from] {
                        return Err("debt reset admitted after the first fault".into());
                    }
                    // Existing recorded cost/payments survive the reset; its
                    // BAD bit records an already real debt deficit, not an
                    // earlier invented failure or erased native cost.
                    if next[15] != report.states[from][15]
                        || next[16] != report.states[from][16]
                        || next[14] != 0
                        || next[15] <= 9 * next[16]
                    {
                        return Err("BAD latch did not record the original debt deficit".into());
                    }
                    for (coordinate, value) in next.iter().enumerate() {
                        if coordinate != 14
                            && coordinate != 39
                            && *value != report.states[from][coordinate]
                        {
                            return Err("debt latch changed unrelated custody/history".into());
                        }
                    }
                }
                if safe[from] && !safe[edge.to] {
                    first_fault_edges += 1;
                    if next[39] != 1 {
                        return Err(format!("{}: first fault was not latched", frontier.name));
                    }
                }
            }
        }
        assert!(
            compared > 0 && first_fault_edges > 0,
            "frontier comparison was vacuous"
        );
        assert!(
            report
                .violations
                .iter()
                .all(|violation| violation.cycle.is_empty() && violation.property != "Deadlock")
        );
        let returned = trace(
            &frontier,
            &report,
            &[
                "CloseSource",
                "VerifyPackagedBootstrapBeforeBirth",
                "Quiescent",
                "PrepareFrozenBindingAndPipe",
                "PayParentAfterSpanEnd",
                "Birth",
                "RetainActualReturnedChild",
            ],
        );
        let (before, after, property) = match kind {
            0 => (
                returned,
                step(&frontier, &report, returned, "ChildArgvSpan").unwrap(),
                "AllBirthPermitOwnerAndHandoffGuards",
            ),
            3 => (
                returned,
                step(&frontier, &report, returned, "TransferBeforePermitWrite").unwrap(),
                "AllBirthPermitOwnerAndHandoffGuards",
            ),
            4 => {
                let cost = step(&frontier, &report, returned, "ParentCostVerdict").unwrap();
                let reset = step(&frontier, &report, cost, "BrokenResetDebt").unwrap();
                assert_eq!(
                    (
                        report.states[reset][14],
                        report.states[reset][15],
                        report.states[reset][16]
                    ),
                    (0, 27, 0),
                    "reset erased recorded cost"
                );
                (cost, reset, "FullParentDebtCannotReset")
            }
            _ => unreachable!(),
        };
        assert!(safe[before] && !safe[after]);
        assert_eq!(
            (
                report.states[after][10],
                report.states[after][11],
                report.states[after][39]
            ),
            (1, 1, 1)
        );
        assert!(
            report
                .violations
                .iter()
                .any(|violation| violation.property == property),
            "{}: missing intended {property}",
            frontier.name
        );
        eprintln!(
            "C1 frontier exact model={} states={} compared_pre_fault={} first_fault_edges={} cap=100000",
            frontier.name,
            report.states.len(),
            compared,
            first_fault_edges
        );
    }
    // Projection is forbidden on correct/other cases, including their guards.
    for kind in 0..BOOTSTRAP_NAMES.len() {
        for broken in [false, true] {
            if !broken || !matches!(kind, 0 | 3 | 4) {
                assert_eq!(
                    format!("{:?}", bootstrap_admission(kind, broken)),
                    format!("{:?}", bootstrap_admission_reference(kind, broken))
                );
            }
        }
    }
    Ok(())
}
#[test]
fn c1_bootstrap_frontier_comparison_rejects_independent_dropped_and_changed_pre_fault_edges()
-> Result<(), String> {
    let frontier = bootstrap_admission(0, true);
    let reference = bootstrap_admission_reference(0, true);
    let report = frontier.check(100_000)?;
    let before = trace(
        &frontier,
        &report,
        &[
            "CloseSource",
            "VerifyPackagedBootstrapBeforeBirth",
            "Quiescent",
            "PrepareFrozenBindingAndPipe",
            "PayParentAfterSpanEnd",
            "Birth",
            "RetainActualReturnedChild",
        ],
    );
    let state = &report.states[before];
    let mut native = NativeOneStep::new(&reference)?;
    same_one_step(&mut NativeOneStep::new(&frontier)?, &mut native, state)?;
    let index = frontier
        .actions
        .iter()
        .position(|action| action.name == "ChildArgvSpan")
        .unwrap();
    let mut dropped = frontier.clone();
    dropped.actions[index].guard = Bool(false);
    frontier_header_and_dispositions(&dropped, &reference)?;
    let refused = same_one_step(&mut NativeOneStep::new(&dropped)?, &mut native, state)
        .expect_err("comparison accepted a removed dangerous pre-fault edge");
    assert_eq!(refused, "pre-frontier labelled one-step successor mismatch");
    let mut changed = frontier.clone();
    let (_, span) = changed.actions[index]
        .updates
        .iter_mut()
        .find(|(slot, _)| *slot == 27)
        .unwrap();
    *span = Int(2);
    let refused = same_one_step(&mut NativeOneStep::new(&changed)?, &mut native, state)
        .expect_err("comparison accepted a changed first-fault target");
    assert_eq!(refused, "pre-frontier labelled one-step successor mismatch");
    assert!(
        frontier_header_and_dispositions(&changed, &reference).is_err(),
        "mapping accepted changed action updates"
    );
    Ok(())
}

#[test]
fn c1_bootstrap_gate_mutant_preserves_fixed_argv_then_builder_program_order() -> Result<(), String>
{
    let model = bootstrap_admission(0, true);
    let report = model
        .check(100_000)
        .map_err(|error| format!("{}: {error}", model.name))?;
    let returned = trace(
        &model,
        &report,
        &[
            "CloseSource",
            "VerifyPackagedBootstrapBeforeBirth",
            "Quiescent",
            "PrepareFrozenBindingAndPipe",
            "PayParentAfterSpanEnd",
            "Birth",
            "RetainActualReturnedChild",
        ],
    );
    assert_eq!(
        (report.states[returned][12], report.states[returned][22]),
        (0, 0),
        "causal mutant witness must precede both cost verdict and permit"
    );
    assert!(
        step(&model, &report, returned, "ChildComputeOwnerSpan").is_none(),
        "removing authorization must not jump over argv's program location"
    );
    let argv = step(&model, &report, returned, "ChildArgvSpan").unwrap();
    assert_eq!(
        (report.states[argv][27], report.states[argv][39]),
        (1, 1),
        "the ungated argv admission must still produce the semantic violation"
    );
    assert!(step(&model, &report, argv, "ChildArgvSpan").is_none());
    assert!(
        step(&model, &report, argv, "ChildComputeOwnerSpan").is_none(),
        "no second expensive span belongs beyond the first safety violation"
    );
    assert!(step(&model, &report, argv, "IndependentStatusCancelControl").is_some());
    assert!(step(&model, &report, argv, "QuarantineNoQualifyingHello").is_some());
    let authorized_argv = trace_from(
        &model,
        &report,
        returned,
        &[
            "ParentCostVerdict",
            "PayParentAfterSpanEnd",
            "PayParentAfterSpanEnd",
            "PayParentAfterSpanEnd",
            "TransferBeforePermitWrite",
            "WriteFullPermitOnce",
            "ReadGrant",
            "ChildArgvSpan",
        ],
    );
    assert_eq!(
        (
            report.states[authorized_argv][27],
            report.states[authorized_argv][39]
        ),
        (1, 0)
    );
    assert!(step(&model, &report, authorized_argv, "ChildArgvSpan").is_none());
    // Builder's own unpaid admission remains a distinct first violation.
    let bad_builder = step(&model, &report, authorized_argv, "ChildComputeOwnerSpan").unwrap();
    assert_eq!(
        (
            report.states[bad_builder][27],
            report.states[bad_builder][39]
        ),
        (2, 1)
    );
    let authorized_builder = trace_from(
        &model,
        &report,
        authorized_argv,
        &["PayChildAfterSpanEnd", "ChildComputeOwnerSpan"],
    );
    assert_eq!(
        (
            report.states[authorized_builder][27],
            report.states[authorized_builder][39]
        ),
        (2, 0)
    );
    for action in ["ChildArgvSpan", "ChildComputeOwnerSpan"] {
        assert!(
            step(&model, &report, authorized_builder, action).is_none(),
            "fixed bootstrap program replayed {action}"
        );
    }
    assert!(
        report
            .violations
            .iter()
            .any(|violation| violation.property == "AllBirthPermitOwnerAndHandoffGuards")
    );
    Ok(())
}

#[test]
fn c1_bootstrap_birth_gate_debt_hello_start_and_unknown_custody_are_nonvacuous()
-> Result<(), String> {
    let m = bootstrap_admission(0, false);
    let r = m.check(100_000)?;
    let prepared = trace(
        &m,
        &r,
        &[
            "CloseSource",
            "VerifyPackagedBootstrapBeforeBirth",
            "Quiescent",
            "PrepareFrozenBindingAndPipe",
        ],
    );
    assert!(
        step(&m, &r, prepared, "Birth").is_none(),
        "preparation debt was bypassed"
    );
    let held = trace_from(
        &m,
        &r,
        prepared,
        &[
            "PayParentAfterSpanEnd",
            "Birth",
            "RetainActualReturnedChild",
        ],
    );
    assert!(step(&m, &r, held, "ChildArgvSpan").is_none());
    assert!(step(&m, &r, held, "ChildComputeOwnerSpan").is_none());
    assert!(step(&m, &r, held, "TransferBeforePermitWrite").is_none());
    let cost = step(&m, &r, held, "ParentCostVerdict").unwrap();
    assert_eq!(r.states[cost][14], 27);
    assert!(step(&m, &r, cost, "TransferBeforePermitWrite").is_none());
    let paid = trace_from(
        &m,
        &r,
        cost,
        &[
            "PayParentAfterSpanEnd",
            "PayParentAfterSpanEnd",
            "PayParentAfterSpanEnd",
        ],
    );
    let grant = trace_from(
        &m,
        &r,
        paid,
        &[
            "TransferBeforePermitWrite",
            "WriteFullPermitOnce",
            "ReadGrant",
            "ChildArgvSpan",
        ],
    );
    assert!(
        step(&m, &r, grant, "ParentPrepareSpan").is_none(),
        "parent work overlapped bootstrap"
    );
    let boot = trace_from(
        &m,
        &r,
        grant,
        &["PayChildAfterSpanEnd", "ChildComputeOwnerSpan"],
    );
    assert!(step(&m, &r, boot, "PaidHelloRetiresReadGate").is_none());
    let hello = trace_from(
        &m,
        &r,
        boot,
        &[
            "PayChildAfterSpanEnd",
            "PayChildAfterSpanEnd",
            "PayChildAfterSpanEnd",
            "PaidHelloRetiresReadGate",
            "AcceptHelloAndCloseLease",
            "ParentPrepareSpan",
        ],
    );
    assert!(step(&m, &r, hello, "PaidStart").is_none());
    let completed = trace_from(
        &m,
        &r,
        hello,
        &[
            "PayParentAfterSpanEnd",
            "PaidStart",
            "ChildWorkSpan",
            "PayChildAfterSpanEnd",
            "ObserveActualExit",
            "ValidateImmutableOutput",
            "PayParentAfterSpanEnd",
            "Complete",
        ],
    );
    assert_eq!(
        (
            r.states[completed][32],
            r.states[completed][34],
            r.states[completed][36],
            r.states[completed][11]
        ),
        (1, 1, 1, 0)
    );
    let unknown = trace_from(
        &m,
        &r,
        prepared,
        &[
            "PayParentAfterSpanEnd",
            "Birth",
            "RetainUnknownBirth",
            "QuarantineUnknownBirth",
            "IndependentOtherJobProgress",
            "IndependentStatusCancelControl",
        ],
    );
    assert_eq!(
        (
            r.states[unknown][7],
            r.states[unknown][11],
            r.states[unknown][37]
        ),
        (2, 1, 1)
    );
    assert!(step(&m, &r, unknown, "Birth").is_none());
    let partial = trace_from(
        &m,
        &r,
        paid,
        &["TransferBeforePermitWrite", "WritePartialConsumesAttempt"],
    );
    assert_eq!(
        (
            r.states[partial][21],
            r.states[partial][22],
            r.states[partial][23]
        ),
        (1, 1, 0)
    );
    assert!(step(&m, &r, partial, "ReadGrant").is_none());
    assert!(step(&m, &r, partial, "WriteFullPermitOnce").is_none());
    let stale = trace_from(&m, &r, paid, &["ContextToB2", "ContextBackToA3"]);
    assert!(step(&m, &r, stale, "TransferBeforePermitWrite").is_none());
    let control = m
        .actions
        .iter()
        .position(|a| a.name == "IndependentStatusCancelControl")
        .unwrap();
    assert!(
        r.edges
            .iter()
            .all(|edges| edges.iter().any(|edge| edge.action == control))
    );
    Ok(())
}

#[test]
fn c1_probe_explicit_null_and_permanent_start_refusal_are_nonvacuous() -> Result<(), String> {
    let m = bootstrap_probe(0, false);
    let r = m.check(100_000)?;
    let probe = trace(
        &m,
        &r,
        &[
            "ProbeExplicitNull",
            "PrebirthKnownProfile",
            "BirthFromFrozenBinding",
            "PaidBootstrapHello",
            "MatchingOrHostileStart",
        ],
    );
    assert_eq!((r.states[probe][5], r.states[probe][6]), (0, 0));
    let real = trace(&m, &r, &["RealNullContext", "PrebirthKnownProfile"]);
    assert!(step(&m, &r, real, "BirthFromFrozenBinding").is_none());
    let omitted = trace(
        &m,
        &r,
        &[
            "ProbeExplicitNull",
            "OmittedContext",
            "PrebirthKnownProfile",
        ],
    );
    assert!(step(&m, &r, omitted, "BirthFromFrozenBinding").is_none());
    Ok(())
}

#[test]
fn c1_recursive_final_control_debt_is_semantic_refusal_with_bounded_span_ledger()
-> Result<(), String> {
    let model = final_public_admission(5, true);
    let report = model
        .check(100_000)
        .map_err(|error| format!("{}: {error}", model.name))?;
    let paid = trace(
        &model,
        &report,
        &[
            "PayExistingWorkDebt",
            "PayExistingWorkDebt",
            "PayExistingWorkDebt",
        ],
    );
    let permit = step(&model, &report, paid, "FinalPermitControlAndWrite").unwrap();
    assert_eq!(
        (
            report.states[permit][1],
            report.states[permit][2],
            report.states[permit][3],
            report.states[permit][20]
        ),
        (9, 0, 0, 1)
    );
    assert!(
        step(
            &model,
            &report,
            permit,
            "PaidHelloThenOrdinaryParentPreparation"
        )
        .is_none(),
        "ordinary preparation overwrote the erroneous control span's unpaid debt"
    );
    let permit_paid = step(&model, &report, permit, "PayExistingWorkDebt").unwrap();
    assert_eq!(
        (
            report.states[permit_paid][1],
            report.states[permit_paid][2],
            report.states[permit_paid][3]
        ),
        (0, 0, 1)
    );
    let start = trace_from(
        &model,
        &report,
        permit_paid,
        &[
            "PaidHelloThenOrdinaryParentPreparation",
            "PayExistingWorkDebt",
            "FinalStartControlAndWrite",
        ],
    );
    assert_eq!(
        (
            report.states[start][1],
            report.states[start][2],
            report.states[start][3],
            report.states[start][14]
        ),
        (9, 0, 0, 2)
    );
    assert!(report.states.iter().all(|state| state[3] <= 3));
    assert!(
        report
            .violations
            .iter()
            .any(|violation| violation.property == "NoRecursiveWorkDebtForFinalControl")
    );
    Ok(())
}

#[test]
fn c1_final_public_admission_is_after_debt_once_bounded_and_control_only() -> Result<(), String> {
    let m = final_public_admission(0, false);
    let r = m.check(100_000)?;
    assert!(step(&m, &r, 0, "FinalPermitControlAndWrite").is_none());
    let paid = trace(
        &m,
        &r,
        &[
            "PayExistingWorkDebt",
            "PayExistingWorkDebt",
            "PayExistingWorkDebt",
        ],
    );
    for fault in [
        "AfterDebtPublicTamper",
        "HoldPreparedDependency",
        "InjectTwoMsFinalControl",
        "UnknownFinalClock",
        "MisclassifyPayload",
        "MisclassifyGenericSqliteOrSetup",
        "MisclassifyPreparedConnectionSetup",
        "OversizePublicCellBeforeFetch",
        "EighthPublicRowBeforeFetch",
        "ExcessTotalBeforeFetch",
    ] {
        let rejected = step(&m, &r, paid, fault).unwrap();
        assert!(
            step(&m, &r, rejected, "FinalPermitControlAndWrite").is_none(),
            "{fault} granted a handoff"
        );
        let rejected = step(&m, &r, rejected, "RefuseFinalPermitControlAndWrite").unwrap();
        assert_eq!(
            (
                r.states[rejected][12],
                r.states[rejected][17],
                r.states[rejected][19]
            ),
            (1, 1, 0)
        );
    }
    let permit = step(&m, &r, paid, "FinalPermitControlAndWrite").unwrap();
    assert!(step(&m, &r, permit, "FinalPermitControlAndWrite").is_none());
    let prepared = step(&m, &r, permit, "PaidHelloThenOrdinaryParentPreparation").unwrap();
    assert!(step(&m, &r, prepared, "FinalStartControlAndWrite").is_none());
    let started = trace_from(
        &m,
        &r,
        prepared,
        &["PayExistingWorkDebt", "FinalStartControlAndWrite"],
    );
    assert_eq!(
        (
            r.states[started][12],
            r.states[started][13],
            r.states[started][14]
        ),
        (1, 1, 2)
    );
    let control = m
        .actions
        .iter()
        .position(|a| a.name == "IndependentStatusCancel")
        .unwrap();
    assert!(
        r.edges
            .iter()
            .all(|edges| edges.iter().any(|edge| edge.action == control))
    );
    Ok(())
}
