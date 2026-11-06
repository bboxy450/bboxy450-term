// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One retained message, three session incarnations (A/B/A), two sends, one UI face.
//! Worker owns connection/acks; logic owns generation; UI owns face/detach. Atomic steps below.
use super::{action, all, eq, implies, model, safe};
use bboxy450_tlc::{
    Expr,
    Expr::{Bool, Int, Var},
    Model, Response,
};
fn host(control: u8) -> Model {
    let mut m = model(
        "RichHost",
        false,
        &[
            ("incarnation", 3, 1),
            ("bound", 3, 0),
            ("connected", 1, 1),
            ("sends", 2, 0),
            ("effects", 2, 0),
            ("pending", 1, 0),
            ("detached", 1, 0),
            ("rich", 1, 1),
            ("pty_bytes", 1, 0),
            ("stale_effect", 1, 0),
            ("generation", 2, 1),
            ("captured", 2, 1),
        ],
    );
    m.name = match control {
        1 => "BrokenRichHost",
        2 => "BrokenRichHostIdentity",
        3 => "BrokenRichHostDetach",
        4 => "BrokenRichHostRouting",
        _ => "RichHost",
    }
    .into();
    action(&mut m, "Admit", all([eq(1, 0), eq(2, 1)]), &[(1, Var(0))]);
    let target = if control == 2 { Var(0) } else { Var(1) };
    let accepted = all([
        target.eq(Var(0)),
        Var(10).eq(Var(11)),
        if control == 1 { Bool(true) } else { eq(4, 0) },
    ]);
    action(
        &mut m,
        "Send",
        all([Int(0).lt(Var(1)), eq(2, 1), Var(3).lt(Int(2))]),
        &[
            (3, Var(3).plus(Int(1))),
            (5, Int(1)),
            (4, accepted.clone().ite(Var(4).plus(Int(1)), Var(4))),
            (
                9,
                all([accepted, Var(1).eq(Var(0)).negate()]).ite(Int(1), Var(9)),
            ),
        ],
    );
    action(&mut m, "AckOrDeadline", eq(5, 1), &[(5, Int(0))]);
    action(
        &mut m,
        "Switch",
        Var(0).lt(Int(3)),
        &[(0, Var(0).plus(Int(1))), (2, Int(0))],
    );
    action(&mut m, "Reconnect", eq(2, 0), &[(2, Int(1))]);
    action(&mut m, "Restart", eq(10, 1), &[(10, Int(2)), (2, Int(0))]);
    action(
        &mut m,
        "Detach",
        eq(6, 0),
        &[(6, Int(1)), (4, if control == 3 { Int(0) } else { Var(4) })],
    );
    action(&mut m, "Attach", eq(6, 1), &[(6, Int(0))]);
    action(
        &mut m,
        "Key",
        eq(7, 1),
        &[(8, Int(if control == 4 { 1 } else { 0 }))],
    );
    safe(&mut m, "OneTurn", Var(4).le(Int(1)));
    safe(&mut m, "NoStaleEffect", eq(9, 0));
    safe(&mut m, "RichKeysNeverPty", eq(8, 0));
    // Detached views cannot erase admitted durable effects.
    safe(
        &mut m,
        "DetachPreservesEffect",
        implies(all([eq(6, 1), eq(3, 1), eq(0, 1), eq(10, 1)]), eq(4, 1)),
    );
    m.responses.push(Response {
        name: "DeliveryRetires".into(),
        request: eq(5, 1),
        response: eq(5, 0),
    });
    m.weak_fair.push(vec![2]);
    m
}
// One Escape is captured on A/e1/g1 before any interleaving. Worker owns
// dequeue/admission; UI owns retirement; session owner changes session/epoch;
// logic changes generation. Approval is pending in every announced incarnation.
fn cancel(broken: bool) -> Model {
    let mut m = model(
        if broken {
            "BrokenRichCancelFence"
        } else {
            "RichCancelFence"
        },
        false,
        &[
            ("session", 3, 1),
            ("epoch", 2, 1),
            ("generation", 2, 1),
            ("captured_session", 3, 1),
            ("captured_epoch", 2, 1),
            ("captured_generation", 2, 1),
            ("queued", 1, 1),
            ("ready", 1, 1),
            ("retired", 1, 0),
            ("stale_admission", 1, 0),
        ],
    );
    action(
        &mut m,
        "Switch",
        Var(0).lt(Int(3)),
        &[(0, Var(0).plus(Int(1))), (7, Int(0))],
    );
    action(&mut m, "Resume", eq(1, 1), &[(1, Int(2)), (7, Int(0))]);
    action(&mut m, "Restart", eq(2, 1), &[(2, Int(2)), (7, Int(0))]);
    action(&mut m, "Ready", eq(7, 0), &[(7, Int(1))]);
    action(&mut m, "Disconnect", eq(7, 1), &[(7, Int(0))]);
    action(&mut m, "Retire", eq(8, 0), &[(8, Int(1))]);
    let valid = all([
        Var(0).eq(Var(3)),
        Var(1).eq(Var(4)),
        Var(2).eq(Var(5)),
        eq(7, 1),
        eq(8, 0),
    ]);
    let admit = if broken { Bool(true) } else { valid.clone() };
    action(
        &mut m,
        "DeliverCancel",
        eq(6, 1),
        &[
            (6, Int(0)),
            (9, all([admit, valid.negate()]).ite(Int(1), Var(9))),
        ],
    );
    safe(&mut m, "NoStaleCancelAdmission", eq(9, 0));
    m.responses.push(Response {
        name: "CancelRetires".into(),
        request: eq(6, 1),
        response: eq(6, 0),
    });
    m.weak_fair.push(vec![6]);
    m
}

// Decision65940. Two panels, two UI reservations per panel, one accepted local
// edit immediately queued/retired at each reservation. Worker start/restore can
// run in either order; only internal queue drain and completion are weakly fair.
// Values are four distinct edit bits, not arbitrary composer/attachment bytes.
// Receiver moves are atomic; full ViewDraft fidelity and real executor behavior
// remain the mapped rich_pane implementation witnesses, not Rust refinement.
const DRAFT_WIDTH: usize = 12;
fn draft_bit(index: usize, bit: i64) -> Expr {
    (0..16)
        .filter(|value| value & bit != 0)
        .map(|value| eq(index, value))
        .fold(Bool(false), Expr::or)
}
fn draft_receivers(base: usize, receiver: i64, off_owner: bool) -> Expr {
    let mut owners = eq(base + 1, receiver).ite(Int(1), Int(0));
    for worker in 0..2 {
        owners = owners.plus(
            all([eq(base + 6 + worker, 1), eq(base + 4 + worker, receiver)]).ite(Int(1), Int(0)),
        );
    }
    if off_owner {
        // Before its deferred reservation, the fresh receiver is still owned
        // by that unscheduled worker envelope, never silently unowned.
        owners = owners.plus(eq(base + 1 + receiver as usize, 1).ite(Int(1), Int(0)));
    }
    owners
}
fn draft_chain(control: u8) -> Model {
    let mut m = model(
        match control {
            1 => "BrokenRichDraftChainReservation",
            2 => "BrokenRichDraftChainEarlyRestore",
            3 => "BrokenRichDraftChainDrop",
            _ => "RichDraftChain",
        },
        false,
        &[
            ("a_reservations", 2, 0),
            ("a_head", 2, 0),
            ("a_old_phase", 5, 0),
            ("a_new_phase", 5, 0),
            ("a_old_predecessor", 2, 0),
            ("a_new_predecessor", 2, 0),
            ("a_old_holds_receiver", 1, 0),
            ("a_new_holds_receiver", 1, 0),
            ("a_old_value", 15, 0),
            ("a_new_value", 15, 0),
            ("a_saved", 15, 0),
            ("a_collected", 1, 0),
            ("b_reservations", 2, 0),
            ("b_head", 2, 0),
            ("b_old_phase", 5, 0),
            ("b_new_phase", 5, 0),
            ("b_old_predecessor", 2, 0),
            ("b_new_predecessor", 2, 0),
            ("b_old_holds_receiver", 1, 0),
            ("b_new_holds_receiver", 1, 0),
            ("b_old_value", 15, 0),
            ("b_new_value", 15, 0),
            ("b_saved", 15, 0),
            ("b_collected", 1, 0),
        ],
    );
    for (panel, label) in ["A", "B"].into_iter().enumerate() {
        let base = panel * DRAFT_WIDTH;
        for (worker, worker_name) in ["Old", "New"].into_iter().enumerate() {
            let phase = base + 2 + worker;
            let predecessor = base + 4 + worker;
            let held = base + 6 + worker;
            let value = base + 8 + worker;
            let bit = 1 << (panel * 2 + worker);
            let mut reserve = vec![(base, Int(worker as i64 + 1)), (phase, Int(1))];
            if control != 1 {
                reserve.extend([
                    (predecessor, Var(base + 1)),
                    (held, Int(0).lt(Var(base + 1)).ite(Int(1), Int(0))),
                    (base + 1, Int(worker as i64 + 1)),
                ]);
            }
            action(
                &mut m,
                &format!("Reserve{label}{worker_name}"),
                eq(base, worker as i64),
                &reserve,
            );
            let mut start = vec![(phase, Int(2))];
            if control == 1 {
                start.extend([
                    (predecessor, Var(base + 1)),
                    (held, Int(0).lt(Var(base + 1)).ite(Int(1), Int(0))),
                    (base + 1, Int(worker as i64 + 1)),
                ]);
            }
            action(
                &mut m,
                &format!("Start{label}{worker_name}"),
                eq(phase, 1),
                &start,
            );
            let predecessor_done = eq(predecessor, 0)
                .or(all([eq(predecessor, 1), eq(base + 2, 5)]))
                .or(all([eq(predecessor, 2), eq(base + 3, 5)]));
            let restored = eq(predecessor, 1)
                .ite(Var(base + 8), eq(predecessor, 2).ite(Var(base + 9), Int(0)));
            let mut restore = vec![(phase, Int(3)), (held, Int(0)), (value, restored)];
            let other = 1 - worker;
            restore.push((
                base + 8 + other,
                eq(predecessor, other as i64 + 1).ite(Int(0), Var(base + 8 + other)),
            ));
            action(
                &mut m,
                &format!("Restore{label}{worker_name}"),
                all([
                    eq(phase, 2),
                    if control == 2 {
                        Bool(true)
                    } else {
                        predecessor_done
                    },
                ]),
                &restore,
            );
            let drain = m.actions.len();
            action(
                &mut m,
                &format!("Drain{label}{worker_name}"),
                eq(phase, 3),
                &[
                    (phase, Int(4)),
                    (
                        value,
                        if control == 3 {
                            Var(value)
                        } else {
                            Var(value).plus(Int(bit))
                        },
                    ),
                ],
            );
            let complete = m.actions.len();
            action(
                &mut m,
                &format!("Complete{label}{worker_name}"),
                eq(phase, 4),
                &[
                    (phase, Int(5)),
                    (
                        value,
                        Int(0)
                            .lt(draft_receivers(base, worker as i64 + 1, control == 1))
                            .ite(Var(value), Int(0)),
                    ),
                ],
            );
            m.weak_fair.push(vec![drain, complete]);
            m.responses.push(Response {
                name: format!("Draft{label}{worker_name}DrainCompletes"),
                request: eq(phase, 3).or(eq(phase, 4)),
                response: eq(phase, 5),
            });
            safe(
                &mut m,
                &format!("DraftRestoreWaitsForPredecessorDrain{label}{worker_name}"),
                implies(
                    Int(2).lt(Var(phase)),
                    eq(predecessor, 0)
                        .or(all([eq(predecessor, 1), eq(base + 2, 5)]))
                        .or(all([eq(predecessor, 2), eq(base + 3, 5)])),
                ),
            );
            safe(
                &mut m,
                &format!("DraftReservationFollowsUiOrder{label}{worker_name}"),
                implies(Int(1).lt(Var(phase)), eq(predecessor, worker as i64)),
            );
            safe(
                &mut m,
                &format!("DraftPredecessorReceiverHasOneOwner{label}{worker_name}"),
                draft_receivers(base, worker as i64 + 1, control == 1).le(Int(1)),
            );
        }
        action(
            &mut m,
            &format!("Collect{label}"),
            all([
                eq(base, 2),
                eq(base + 11, 0),
                all([eq(base + 1, 1), eq(base + 2, 5)]).or(all([eq(base + 1, 2), eq(base + 3, 5)])),
            ]),
            &[
                (base + 10, eq(base + 1, 1).ite(Var(base + 8), Var(base + 9))),
                (base + 8, eq(base + 1, 1).ite(Int(0), Var(base + 8))),
                (base + 9, eq(base + 1, 2).ite(Int(0), Var(base + 9))),
                (base + 1, Int(0)),
                (base + 11, Int(1)),
            ],
        );
        for foreign in 0..2 {
            for value in [base + 8, base + 9, base + 10] {
                safe(
                    &mut m,
                    &format!("DraftNeverAdoptsAnotherPanel{label}Value{value}Bit{foreign}"),
                    draft_bit(value, 1 << ((1 - panel) * 2 + foreign)).negate(),
                );
            }
        }
    }
    for edit in 0..4 {
        let base = (edit / 2) * DRAFT_WIDTH;
        let worker = edit % 2;
        let phase = base + 2 + worker;
        let accepted = Int(worker as i64).lt(Var(base));
        let queued = all([Int(0).lt(Var(phase)), Var(phase).le(Int(3))]);
        let mut owners = queued.ite(Int(1), Int(0));
        for value in [8, 9, 10, 20, 21, 22] {
            owners = owners.plus(draft_bit(value, 1 << edit).ite(Int(1), Int(0)));
        }
        safe(
            &mut m,
            &format!("DraftAcceptedEdit{edit}RemainsOwned"),
            implies(accepted.clone(), Int(0).lt(owners.clone())),
        );
        safe(
            &mut m,
            &format!("DraftAcceptedEdit{edit}HasOneOwner"),
            owners.clone().le(Int(1)),
        );
        safe(
            &mut m,
            &format!("DraftUnacceptedEdit{edit}NeverAppears"),
            implies(accepted.negate(), owners.eq(Int(0))),
        );
    }
    m
}

#[test]
fn rich_draft_chain_ui_order_drain_and_independent_panels_are_nonvacuous() -> Result<(), String> {
    let m = draft_chain(0);
    let r = m.check(100_000)?;
    assert!(r.violations.is_empty());
    let step = |state: usize, name: &str| -> Option<usize> {
        let action = m.actions.iter().position(|a| a.name == name).unwrap();
        r.edges[state]
            .iter()
            .find(|e| e.action == action)
            .map(|e| e.to)
    };
    let mut state = 0;
    for name in ["ReserveAOld", "ReserveANew", "StartANew", "StartAOld"] {
        state = step(state, name).unwrap();
    }
    assert!(step(state, "RestoreANew").is_none());
    for name in [
        "ReserveBOld",
        "StartBOld",
        "RestoreBOld",
        "DrainBOld",
        "CompleteBOld",
        "ReserveBNew",
        "StartBNew",
        "RestoreBNew",
        "DrainBNew",
        "CompleteBNew",
        "CollectB",
    ] {
        state = step(state, name).unwrap();
    }
    assert_eq!(
        (r.states[state][2], r.states[state][3], r.states[state][22]),
        (2, 2, 12)
    );
    for name in ["RestoreAOld", "DrainAOld"] {
        state = step(state, name).unwrap();
    }
    assert!(step(state, "RestoreANew").is_none());
    for name in ["CompleteAOld", "RestoreANew"] {
        state = step(state, name).unwrap();
    }
    assert_eq!(
        (r.states[state][8], r.states[state][9], r.states[state][7]),
        (0, 1, 0)
    );
    for name in ["DrainANew", "CompleteANew", "CollectA"] {
        state = step(state, name).unwrap();
    }
    assert_eq!((r.states[state][10], r.states[state][22]), (3, 12));
    assert!(m.weak_fair.iter().flatten().all(|index| {
        m.actions[*index].name.starts_with("Drain")
            || m.actions[*index].name.starts_with("Complete")
    }));
    eprintln!(
        "RichDraftChain full native states={} edges={}",
        r.states.len(),
        r.edges.iter().map(Vec::len).sum::<usize>()
    );
    Ok(())
}

#[test]
fn rich_draft_chain_removed_guards_fail_safety_at_the_actual_atomic_step() -> Result<(), String> {
    for (control, names, property) in [
        (
            1,
            vec!["ReserveAOld", "ReserveANew", "StartANew"],
            "DraftReservationFollowsUiOrderANew",
        ),
        (
            2,
            vec!["ReserveAOld", "ReserveANew", "StartANew", "RestoreANew"],
            "DraftRestoreWaitsForPredecessorDrainANew",
        ),
        (
            3,
            vec!["ReserveAOld", "StartAOld", "RestoreAOld", "DrainAOld"],
            "DraftAcceptedEdit0RemainsOwned",
        ),
    ] {
        let m = draft_chain(control);
        let r = m.check(100_000)?;
        let mut state = 0;
        for name in names {
            let action = m.actions.iter().position(|a| a.name == name).unwrap();
            state = r.edges[state]
                .iter()
                .find(|edge| edge.action == action)
                .unwrap()
                .to;
        }
        assert!(
            r.violations
                .iter()
                .any(|v| v.property == property && v.prefix.last() == Some(&state)),
            "{} missing causal {property}",
            m.name
        );
        assert!(
            r.violations.iter().all(|v| v.cycle.is_empty()),
            "{} must fail safety, not only liveness",
            m.name
        );
        eprintln!(
            "{} full native states={} edges={}",
            m.name,
            r.states.len(),
            r.edges.iter().map(Vec::len).sum::<usize>()
        );
    }
    Ok(())
}

pub fn cases() -> Vec<(Model, &'static str)> {
    let mut cases = vec![(host(0), "pass"), (host(1), "safety")];
    for control in 2..=4 {
        let broken = host(control);
        let mut good = host(0);
        good.name = broken.name.strip_prefix("Broken").unwrap().into();
        cases.extend([(good, "pass"), (broken, "safety")]);
    }
    cases.extend([(cancel(false), "pass"), (cancel(true), "safety")]);
    cases.push((draft_chain(0), "pass"));
    cases.extend((1..=3).map(|control| (draft_chain(control), "safety")));
    cases
}
