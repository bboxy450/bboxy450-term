// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Origin: P2 task #5084, ported from bboxy450-log-converter into bboxy450-term.
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Finite lc-bxl abstraction: 3 frames × 2 events, 2 readers, 1 rotation.
//! Complete unsynced bytes survive in this local filesystem abstraction; recovery
//! syncs the validated prefix. Kernel power-loss behavior is implementation-only.
use bboxy450_tlc::{
    Action,
    Expr::{self, Bool as B, Int as I, Var as V},
    Invariant, Model, Response, Variable,
};
fn all(expressions: impl IntoIterator<Item = Expr>) -> Expr {
    expressions.into_iter().fold(B(true), Expr::and)
}
fn prefix(n: usize, min: i64) -> Expr {
    all((0..3).map(|i| {
        if i < n {
            I(min - 1).lt(V(i + 1))
        } else {
            V(i + 1).lt(I(2))
        }
    }))
}
pub fn model(name: &str, broken: u8) -> Model {
    let names = [
        "crashed",
        "frame0",
        "frame1",
        "frame2",
        "durable",
        "index",
        "staged",
        "temp_synced",
        "reader_a",
        "reader_b",
        "rotated",
        "table_left",
        "table_right",
        "table_read_phase",
        "observed_left",
        "observed_right",
        "rewrite_phase",
        "recover_expected",
        "recover_stage",
        "recover_actual",
    ];
    let maxima = [1, 3, 3, 3, 3, 3, 3, 1, 3, 3, 1, 1, 1, 2, 1, 1, 2, 3, 2, 3];
    let mut m = Model::new(
        name,
        names
            .into_iter()
            .zip(maxima)
            .map(|(name, max)| Variable::new(name, 0, max, 0))
            .collect(),
    );
    m.check_deadlock = false; // A full log with caught-up readers intentionally stops.
    for i in 0..3 {
        let previous = if i == 0 { B(true) } else { V(i).eq(I(3)) };
        m.actions.push(Action::new(
            &format!("BeginFrame{i}"),
            all([V(0).eq(I(0)), V(i + 1).eq(I(0)), previous]),
            vec![(i + 1, I(1)), (18, I(0)), (17, I(0)), (19, I(0))],
        ));
        m.actions.push(Action::new(
            &format!("CompleteFrame{i}"),
            V(0).eq(I(0)).and(V(i + 1).eq(I(1))),
            vec![(i + 1, I(2))],
        ));
        m.actions.push(Action::new(
            &format!("SyncFrame{i}"),
            V(0).eq(I(0)).and(V(i + 1).eq(I(2))),
            vec![(i + 1, I(3)), (4, I((i + 1) as i64))],
        ));
    }
    for n in 1..=3 {
        let available = if broken == 2 {
            all((1..=n).map(|i| I(1).lt(V(i))))
        } else {
            V(4).eq(I(n as i64))
        };
        m.actions.push(Action::new(
            &format!("WriteIndexTemp{n}"),
            all([
                V(0).eq(I(0)),
                V(6).eq(I(0)),
                V(5).lt(I(n as i64)),
                available,
            ]),
            vec![(6, I(n as i64)), (7, I(0))],
        ));
    }
    m.actions.push(Action::new(
        "SyncIndexTemp",
        all([V(0).eq(I(0)), I(0).lt(V(6)), V(7).eq(I(0))]),
        vec![(7, I(1))],
    ));
    m.actions.push(Action::new(
        "RenameIndex",
        V(0).eq(I(0)).and(V(7).eq(I(1))),
        vec![(5, V(6)), (6, I(0)), (7, I(0))],
    ));
    m.actions.push(Action::new(
        "Crash",
        V(0).eq(I(0)),
        vec![(0, I(1)), (6, I(0)), (7, I(0))],
    ));
    for n in 0..=3 {
        let mut assignments = vec![
            (0, I(0)),
            (4, I(n as i64)),
            (5, I(n as i64)),
            (6, I(0)),
            (7, I(0)),
            (17, I(n as i64)),
            (18, I(1)),
            (19, I(n as i64)),
        ];
        assignments.extend((0..3).map(|i| (i + 1, I(if i < n { 3 } else { 0 }))));
        m.actions.push(Action::new(
            &format!("RecoverPrefix{n}"),
            V(0).eq(I(1)).and(prefix(n, 2)),
            assignments,
        ));
        m.actions.push(Action::new(
            &format!("RecoverAgain{n}"),
            all([V(0).eq(I(0)), V(18).eq(I(1)), prefix(n, 3)]),
            vec![(18, I(2)), (19, I(n as i64))],
        ));
    }
    for (reader, label) in [(8, "A"), (9, "B")] {
        let mut fair = vec![];
        for i in 0..3 {
            fair.push(m.actions.len());
            let valid = if broken == 1 {
                I(0).lt(V(i + 1))
            } else {
                I(1).lt(V(i + 1))
            };
            m.actions.push(Action::new(
                &format!("Reader{label}Step{i}"),
                V(reader).eq(I(i as i64)).and(valid),
                vec![(reader, I((i + 1) as i64))],
            ));
            m.responses.push(Response {
                name: format!("Tail{label}CatchesFrame{i}"),
                request: I(1).lt(V(i + 1)),
                response: I(i as i64).lt(V(reader)),
            });
        }
        m.weak_fair.push(fair);
    }
    m.actions.push(Action::new(
        "SealRotate",
        all([V(0).eq(I(0)), V(1).eq(I(3)), V(10).eq(I(0))]),
        vec![(10, I(1))],
    ));
    // A parked reader has read the first half of the old complete seek table.
    m.actions.push(Action::new(
        "ReadOldTableFirstHalf",
        V(10).eq(I(1)).and(V(13).eq(I(0))),
        vec![(13, I(1)), (14, V(11))],
    ));
    m.actions.push(Action::new(
        "ReadOldTableSecondHalf",
        V(13).eq(I(1)),
        vec![(13, I(2)), (15, V(12))],
    ));
    let mut first = vec![(16, I(1))];
    let mut second = vec![(16, I(2))];
    if broken == 3 {
        first.push((11, I(1)));
        second.push((12, I(1)));
    }
    // Good writes append a new table elsewhere; the old table's bytes never move.
    // Control (c) writes the old table halves in place, exposing actual mixed bytes.
    m.actions.push(Action::new(
        "WriteNewTableFirstHalf",
        all([V(0).eq(I(0)), V(10).eq(I(1)), V(16).eq(I(0))]),
        first,
    ));
    m.actions.push(Action::new(
        "WriteNewTableSecondHalf",
        V(0).eq(I(0)).and(V(16).eq(I(1))),
        second,
    ));
    let readers_valid = all([8, 9]
        .into_iter()
        .flat_map(|r| (0..3).map(move |i| V(r).le(I(i as i64)).or(I(1).lt(V(i + 1))))));
    m.invariants.push(Invariant {
        name: "NoTornRead".into(),
        condition: readers_valid
            .clone()
            .and(V(13).eq(I(2)).negate().or(V(14).eq(V(15)))),
    });
    m.invariants.push(Invariant {
        name: "NoGapNoDup".into(),
        condition: readers_valid,
    });
    m.invariants.push(Invariant {
        name: "IndexNeverPastDurable".into(),
        condition: V(5).le(V(4)),
    });
    m.invariants.push(Invariant {
        name: "SyncedSurvives".into(),
        condition: all((0..3).map(|i| V(4).le(I(i as i64)).or(V(i + 1).eq(I(3))))),
    });
    m.invariants.push(Invariant {
        name: "IndexSound".into(),
        condition: all((0..3).map(|i| V(5).le(I(i as i64)).or(V(i + 1).eq(I(3))))),
    });
    m.invariants.push(Invariant {
        name: "RecoveryIdempotent".into(),
        condition: V(18).eq(I(2)).negate().or(V(17).eq(V(19))),
    });
    m
}
