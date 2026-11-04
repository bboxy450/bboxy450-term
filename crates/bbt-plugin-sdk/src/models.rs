// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The SDK runtime's own contract model (feature `model`), shipped as
//! `models/PluginRuntime.tla` for plugin authors: bounded admission, one
//! worker, cancellation, and host callbacks made while serving a call.
//!
//! The reader thread (runtime.rs `serve`) admits at most two requests in
//! this instance (`MAX_PENDING` abstracted), refusing the rest as busy; it
//! routes cancellations and the host's answers to `host/…` requests while the
//! worker is serving a call; the worker answers every admitted request once.
//! The broken control lets the reader wait for the worker while the worker
//! waits for a host answer only the reader can deliver: a request is then
//! never answered (liveness).
use bboxy450_tlc::Expr::{Int, Var};
use bboxy450_tlc::Model;
use bboxy450_tlc::Response;
use bboxy450_tlc::build::{action, all, eq, model, safe};

/// Every shipped family of this crate.
pub fn families() -> [fn(bool) -> Model; 1] {
    [plugin_runtime]
}

/// `PluginRuntime` (or `BrokenPluginRuntime`).
pub fn plugin_runtime(b: bool) -> Model {
    let mut m = model(
        "PluginRuntime",
        b,
        &[
            ("arrived", 2, 0),
            ("queued", 2, 0),
            ("running", 1, 0),
            ("waiting_host", 1, 0),
            ("answer_unread", 1, 0),
            ("cancelled", 1, 0),
            ("reader_blocked", 1, 0),
            ("done", 2, 0),
        ],
    );
    let admitted = Var(1).plus(Var(2));
    action(
        &mut m,
        "Admit",
        all([Var(0).lt(Int(2)), admitted.clone().lt(Int(2)), eq(6, 0)]),
        &[(0, Var(0).plus(Int(1))), (1, Var(1).plus(Int(1)))],
    );
    action(
        &mut m,
        "Start",
        all([eq(2, 0), Var(1).lt(Int(1)).negate()]),
        &[(1, Var(1).minus(Int(1))), (2, Int(1)), (5, Int(0))],
    );
    // The broken control: the reader hands the next frame to the worker
    // synchronously, so it waits for the worker that waits for the host.
    let ask: &[(usize, bboxy450_tlc::Expr)] = if b {
        &[(3, Int(1)), (6, Int(1))]
    } else {
        &[(3, Int(1))]
    };
    action(&mut m, "AskHost", all([eq(2, 1), eq(3, 0), eq(5, 0)]), ask);
    action(
        &mut m,
        "HostAnswers",
        all([eq(3, 1), eq(4, 0)]),
        &[(4, Int(1))],
    );
    action(
        &mut m,
        "ReaderRoutesAnswer",
        all([eq(4, 1), eq(6, 0)]),
        &[(4, Int(0)), (3, Int(0))],
    );
    action(
        &mut m,
        "ReaderRoutesCancel",
        all([eq(2, 1), eq(5, 0), eq(6, 0)]),
        &[(5, Int(1))],
    );
    action(
        &mut m,
        "AnswerResult",
        all([eq(2, 1), eq(3, 0), eq(5, 0)]),
        &[(2, Int(0)), (7, Var(7).plus(Int(1)))],
    );
    // A cancelled call answers `cancelled` and drops its pending host
    // request; a late answer is read and discarded.
    action(
        &mut m,
        "AnswerCancelled",
        all([eq(2, 1), eq(5, 1), eq(6, 0)]),
        &[
            (2, Int(0)),
            (3, Int(0)),
            (5, Int(0)),
            (7, Var(7).plus(Int(1))),
        ],
    );
    action(
        &mut m,
        "DropLateAnswer",
        all([eq(4, 1), eq(3, 0), eq(6, 0)]),
        &[(4, Int(0))],
    );
    safe(&mut m, "AnsweredAtMostAdmitted", Var(7).le(Var(0)));
    m.responses.push(Response {
        name: "EveryAdmittedRequestIsAnswered".into(),
        request: Var(0).eq(Var(7)).negate(),
        response: Var(0).eq(Var(7)),
    });
    // Weak fairness on every worker, reader and host step.
    m.weak_fair = (1..m.actions.len()).map(|i| vec![i]).collect();
    m
}
