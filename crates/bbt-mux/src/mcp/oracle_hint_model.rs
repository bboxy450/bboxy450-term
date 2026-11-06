// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Atomic, finite contract for Oracle rulings reaching a running MCP caller.
use bboxy450_tlc::Expr::{Int, Var};
use bboxy450_tlc::build::{action, all, eq, model, safe};
use bboxy450_tlc::{Expr, Model};

pub fn oracle_in_turn(broken: bool) -> Model {
    // Ledger owns per-ID state: absent/unread/read/explicitly acknowledged.
    // Native callbacks own transport: stored/queued/steered. They never read or
    // acknowledge a message. MCP owns surfaced bits and the worker time window.
    // Every action is atomic; native completion, exact-ID reads and MCP calls
    // may interleave. The turn stays busy throughout: no cancel/interrupt action.
    // Progress assumes the seat makes another MCP call; no fairness is needed
    // for the safety promise that that call surfaces every eligible Oracle ID.
    // Two Oracle IDs, two pending worker arrivals and capacity two abstract the
    // concrete five-message batch. Oracle IDs consume slots before workers.
    // Byte truncation, signed sender validation, persistence and transport IO
    // remain implementation witnesses, rather than claims of this finite model.
    const O0: usize = 0;
    const O1: usize = 1;
    const H0: usize = 2;
    const H1: usize = 3;
    const N0: usize = 4;
    const N1: usize = 5;
    const WORKERS: usize = 6;
    const WINDOW_HINTS: usize = 7;
    const BATCH: usize = 8;
    const LOST: usize = 9;
    const BUSY: usize = 10;
    let mut m = model(
        "OracleInTurn",
        broken,
        &[
            ("oracle0", 3, 0),
            ("oracle1", 3, 0),
            ("surfaced0", 1, 0),
            ("surfaced1", 1, 0),
            ("native0", 2, 0),
            ("native1", 2, 0),
            ("workers", 2, 0),
            ("workerWindowHints", 2, 0),
            ("batch", 3, 0),
            ("lost", 1, 0),
            ("busy", 1, 1),
        ],
    );
    for (id, state, native) in [(0, O0, N0), (1, O1, N1)] {
        action(
            &mut m,
            &format!("ReceiveOracle{id}"),
            eq(state, 0),
            &[(state, Int(1))],
        );
        // A filtered page names exactly one ID; it leaves the other ID unread.
        action(
            &mut m,
            &format!("ReadExactId{id}"),
            eq(state, 1),
            &[(state, Int(2))],
        );
        action(
            &mut m,
            &format!("ModelAck{id}"),
            eq(state, 1).or(eq(state, 2)),
            &[(state, Int(3))],
        );
        // Late callbacks cannot erase an explicit model acknowledgement or
        // turn queued/steered transport evidence into a model acknowledgement.
        action(
            &mut m,
            &format!("NativeQueued{id}"),
            all([Int(0).lt(Var(state)), eq(native, 0)]),
            &[(native, Int(1))],
        );
        action(
            &mut m,
            &format!("NativeSteered{id}"),
            all([Int(0).lt(Var(state)), Var(native).lt(Int(2))]),
            &[(native, Int(2))],
        );
    }
    action(
        &mut m,
        "ReceiveWorker",
        Var(WORKERS).lt(Int(2)),
        &[(WORKERS, Var(WORKERS).plus(Int(1)))],
    );
    action(
        &mut m,
        "NewWorkerTimeWindow",
        Expr::Bool(true),
        &[(WINDOW_HINTS, Int(0))],
    );
    let pending0 = all([eq(O0, 1), eq(H0, 0)]);
    let pending1 = all([eq(O1, 1), eq(H1, 0)]);
    let output0 = if broken {
        Expr::Bool(false)
    } else {
        pending0.clone()
    };
    let output1 = if broken {
        Expr::Bool(false)
    } else {
        pending1.clone()
    };
    let oracle_count = output0
        .clone()
        .ite(Int(1), Int(0))
        .plus(output1.clone().ite(Int(1), Int(0)));
    let worker_output = all([
        eq(WINDOW_HINTS, 0),
        Int(0).lt(Var(WORKERS)),
        oracle_count.clone().lt(Int(2)),
    ]);
    let missing = pending0
        .and(output0.clone().negate())
        .or(pending1.and(output1.clone().negate()));
    action(
        &mut m,
        "NextMcpCall",
        eq(BUSY, 1),
        &[
            (H0, output0.ite(Int(1), Var(H0))),
            (H1, output1.ite(Int(1), Var(H1))),
            (
                WORKERS,
                worker_output
                    .clone()
                    .ite(Var(WORKERS).minus(Int(1)), Var(WORKERS)),
            ),
            (
                WINDOW_HINTS,
                worker_output
                    .clone()
                    .ite(Var(WINDOW_HINTS).plus(Int(1)), Var(WINDOW_HINTS)),
            ),
            (BATCH, oracle_count.plus(worker_output.ite(Int(1), Int(0)))),
            (LOST, missing.ite(Int(1), Var(LOST))),
        ],
    );
    safe(&mut m, "NextCallSurfacesOracle", eq(LOST, 0));
    safe(&mut m, "HintBatchBounded", Var(BATCH).lt(Int(3)));
    safe(
        &mut m,
        "WorkerHintsRateLimited",
        Var(WINDOW_HINTS).lt(Int(2)),
    );
    safe(&mut m, "RunningTurnNeverInterrupted", eq(BUSY, 1));
    m
}
