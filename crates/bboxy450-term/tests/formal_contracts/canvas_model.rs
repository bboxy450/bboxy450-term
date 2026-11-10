// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Three immutable batches, two seats and the operator, one durable canvas log.
//! Conflicting operations follow appended order, never network delivery order.
use super::*;

const LOG: usize = 0;
const DELIVERIES: usize = 3;
const ACK: usize = 6;
const SEQ: usize = 9;
const PICTURE_A: usize = 10;
const PICTURE_B: usize = 11;
const READY: usize = 12;
const RESTARTS: usize = 13;

fn seen(batch: i64) -> Expr {
    eq(LOG, batch).or(eq(LOG + 1, batch)).or(eq(LOG + 2, batch))
}

// Replay is computed independently from the complete durable log. A=0 means
// absent, A=1 original, A=2 changed; B is the second element's presence bit.
fn replay_a() -> Expr {
    (LOG..LOG + 3).fold(Int(0), |picture, slot| {
        eq(slot, 1).ite(
            Int(1),
            eq(slot, 2).ite(
                picture.clone().eq(Int(0)).ite(Int(0), Int(2)),
                eq(slot, 3).ite(Int(0), picture),
            ),
        )
    })
}

fn convergence(broken: bool) -> Model {
    let mut m = model(
        "CanvasConvergence",
        broken,
        &[
            ("log_first", 3, 0),
            ("log_second", 3, 0),
            ("log_third", 3, 0),
            ("seat_a_deliveries", 2, 0),
            ("seat_b_deliveries", 2, 0),
            ("operator_deliveries", 2, 0),
            ("seat_a_acknowledged", 1, 0),
            ("seat_b_acknowledged", 1, 0),
            ("operator_acknowledged", 1, 0),
            ("seq", 6, 0),
            ("picture_a", 2, 0),
            ("picture_b", 1, 0),
            ("ready", 1, 1),
            ("restarts", 2, 0),
        ],
    );
    // A: add A (expect_seq 0). B: add B + change A (expect_seq 0).
    // Operator: remove A (expect_seq 1). B's stale expectation deliberately
    // does not gate append: its add must survive after A advanced the log.
    // Unknown-id changes/removes refuse before append; after removal, the
    // retained identity makes a late Change/Remove a durable no-op.
    for batch in 1..=3 {
        let index = batch as usize - 1;
        let pending = all([eq(READY, 1), Var(DELIVERIES + index).lt(Int(2))]);
        let known = if batch == 1 { Bool(true) } else { seen(1) };
        let absent = seen(batch).negate();
        for slot in 0..3 {
            let free_prefix = all((0..slot).map(|i| eq(LOG + i, 0).negate()));
            let guard = all([
                pending.clone(),
                known.clone(),
                if broken { Bool(true) } else { absent.clone() },
                eq(LOG + slot, 0),
                free_prefix,
            ]);
            let a = match batch {
                1 => Int(1),
                2 => eq(PICTURE_A, 0).ite(Int(0), Int(2)),
                _ => Int(0),
            };
            action(
                &mut m,
                &format!("AppendBatch{batch}At{slot}"),
                guard,
                &[
                    (LOG + slot, Int(batch)),
                    (DELIVERIES + index, Var(DELIVERIES + index).plus(Int(1))),
                    (SEQ, Var(SEQ).plus(Int(if batch == 2 { 2 } else { 1 }))),
                    (PICTURE_A, a),
                    (PICTURE_B, if batch == 2 { Int(1) } else { Var(PICTURE_B) }),
                ],
            );
        }
        if !broken {
            action(
                &mut m,
                &format!("DeduplicateBatch{batch}"),
                pending.clone().and(seen(batch)),
                &[(DELIVERIES + index, Var(DELIVERIES + index).plus(Int(1)))],
            );
        }
        action(
            &mut m,
            &format!("RefuseUnknownBatch{batch}"),
            pending.and(known.negate()),
            &[(DELIVERIES + index, Var(DELIVERIES + index).plus(Int(1)))],
        );
        let ack_action = m.actions.len();
        action(
            &mut m,
            &format!("AcknowledgeBatch{batch}"),
            all([eq(READY, 1), seen(batch), eq(ACK + index, 0)]),
            &[(ACK + index, Int(1))],
        );
        m.weak_fair.push(vec![ack_action]);
        let count = (LOG..LOG + 3).fold(Int(0), |sum, slot| {
            sum.plus(eq(slot, batch).ite(Int(1), Int(0)))
        });
        safe(
            &mut m,
            &format!("Batch{batch}AppliedAtMostOnce"),
            count.le(Int(1)),
        );
        safe(
            &mut m,
            &format!("AcknowledgedBatch{batch}SurvivesRestart"),
            implies(eq(ACK + index, 1), seen(batch)),
        );
        m.responses.push(Response {
            name: format!("DurableBatch{batch}EventuallyAcknowledged"),
            request: seen(batch),
            response: eq(ACK + index, 1),
        });
    }
    action(
        &mut m,
        "Restart",
        all([eq(READY, 1), Var(RESTARTS).lt(Int(2))]),
        &[
            (READY, Int(0)),
            (RESTARTS, Var(RESTARTS).plus(Int(1))),
            (PICTURE_A, Int(0)),
            (PICTURE_B, Int(0)),
        ],
    );
    let recover = m.actions.len();
    action(
        &mut m,
        "ReplayDurableLog",
        eq(READY, 0),
        &[
            (READY, Int(1)),
            (PICTURE_A, replay_a()),
            (PICTURE_B, seen(2).ite(Int(1), Int(0))),
        ],
    );
    m.weak_fair.push(vec![recover]);
    safe(
        &mut m,
        "PictureIsFunctionOfAppendedLog",
        implies(
            eq(READY, 1),
            Var(PICTURE_A)
                .eq(replay_a())
                .and(Var(PICTURE_B).eq(seen(2).ite(Int(1), Int(0)))),
        ),
    );
    safe(
        &mut m,
        "StoreSequenceCountsAppendedOperations",
        Var(SEQ).eq((LOG..LOG + 3).fold(Int(0), |sum, slot| {
            sum.plus(eq(slot, 0).ite(Int(0), eq(slot, 2).ite(Int(2), Int(1))))
        })),
    );
    safe(
        &mut m,
        "LateChangeNeverResurrectsRemovedElement",
        implies(eq(READY, 1).and(seen(3)), eq(PICTURE_A, 0)),
    );
    // No delivery fairness is assumed: a dead seat may leave an unsent batch.
    // Only recovery and replies for already durable batches are weakly fair.
    m
}

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    vec![(convergence(false), "pass"), (convergence(true), "safety")]
}
