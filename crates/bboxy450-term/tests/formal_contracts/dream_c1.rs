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

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    let mut cases = Vec::new();
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
    cases
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
