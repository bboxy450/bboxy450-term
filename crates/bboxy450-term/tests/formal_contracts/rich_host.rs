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
        1 => "BrokenRichHostDuplicate",
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
pub fn cases() -> Vec<(Model, &'static str)> {
    (0..=4)
        .map(|c| (host(c), if c == 0 { "pass" } else { "safety" }))
        .collect()
}
