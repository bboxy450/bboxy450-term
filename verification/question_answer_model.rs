// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Three fixed operations compete for one question. Delivery can be lost or repeated.
use super::{action, all, eq, implies, model, safe};
use bboxy450_tlc::{
    Expr::{Bool, Int, Var},
    Model, Response,
};

fn question_answer(broken: bool) -> Model {
    // Reply 0 is unresolved, 1 applied, 2 already_answered. The actor retains
    // each outcome while the wire can lose a response and replay the operation.
    let mut m = model(
        "QuestionAnswer",
        broken,
        &[
            ("winner", 3, 0),
            ("applied", 3, 0),
            ("link", 2, 0), // connected, disconnected, reconnected
            ("DOutcome", 2, 0),
            ("POutcome", 2, 0),
            ("MOutcome", 2, 0),
            ("DWire", 2, 0),
            ("PWire", 2, 0),
            ("MWire", 2, 0),
            ("DReceived", 2, 0),
            ("PReceived", 2, 0),
            ("MReceived", 2, 0),
            ("DLost", 1, 0),
            ("PLost", 1, 0),
            ("MLost", 1, 0),
        ],
    );
    action(&mut m, "Disconnect", eq(2, 0), &[(2, Int(1))]);
    let reconnect = m.actions.len();
    action(&mut m, "Reconnect", eq(2, 1), &[(2, Int(2))]);
    m.weak_fair.push(vec![reconnect]);
    let connected = eq(2, 0).or(eq(2, 2));

    for (i, who) in ["D", "P", "M"].into_iter().enumerate() {
        let outcome = 3 + i;
        let wire = 6 + i;
        let received = 9 + i;
        let lost = 12 + i;
        let apply = all([eq(outcome, 0), if broken { Bool(true) } else { eq(0, 0) }]);
        let answer = m.actions.len();
        action(
            &mut m,
            &format!("Answer{who}"),
            all([connected.clone(), eq(received, 0), eq(wire, 0)]),
            &[
                (0, apply.clone().ite(Int(i as i64 + 1), Var(0))),
                (1, apply.clone().ite(Var(1).plus(Int(1)), Var(1))),
                (
                    outcome,
                    eq(outcome, 0).ite(apply.clone().ite(Int(1), Int(2)), Var(outcome)),
                ),
                (
                    wire,
                    eq(outcome, 0).ite(apply.ite(Int(1), Int(2)), Var(outcome)),
                ),
            ],
        );
        action(
            &mut m,
            &format!("Lose{who}"),
            all([Int(0).lt(Var(wire)), eq(received, 0), eq(lost, 0)]),
            &[(wire, Int(0)), (lost, Int(1))],
        );
        let deliver = m.actions.len();
        action(
            &mut m,
            &format!("Deliver{who}"),
            all([connected.clone(), Int(0).lt(Var(wire)), eq(received, 0)]),
            &[(received, Var(wire))],
        );
        action(
            &mut m,
            &format!("Duplicate{who}"),
            Int(0).lt(Var(received)),
            &[(received, Var(wire))],
        );
        m.weak_fair.extend([vec![answer], vec![deliver]]);
        m.responses.push(Response {
            name: format!("EventuallyReceives{who}"),
            request: Bool(true),
            response: Int(0).lt(Var(received)),
        });
        safe(
            &mut m,
            &format!("OnlyWinnerReceivesApplied{who}"),
            implies(eq(received, 1), eq(0, i as i64 + 1)),
        );
    }
    safe(&mut m, "AtMostOneApplied", Var(1).le(Int(1)));
    m.responses.push(Response {
        name: "EventuallyApplies".into(),
        request: Bool(true),
        response: Int(0).lt(Var(1)),
    });
    m
}

pub fn cases() -> Vec<(Model, &'static str)> {
    vec![
        (question_answer(false), "pass"),
        (question_answer(true), "safety"),
    ]
}
