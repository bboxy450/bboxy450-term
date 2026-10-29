// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Yarn scheduling projection onto the native checker and TLA+ exporter.
//! Opaque Rust handler bodies remain outside this model.

pub use bboxy450_tlc::*;
use std::format;
use std::vec;

/// Scheduling facts projected from a validated `reactor!` declaration.
#[derive(Clone, Copy, Debug)]
pub struct ReactorArm {
    /// Source identity.
    pub name: &'static str,
    /// Successful service terminates the reactor.
    pub terminal: bool,
    /// An opaque Rust guard is abstracted as either Boolean value per round.
    pub guarded: bool,
    /// Maximum items per service window.
    pub drain: usize,
    /// Buffered-yield target by lexical index.
    pub yields_to: Option<usize>,
}
/// Projects biased selection and bounded draining into a finite model.
///
/// One pending item per source, nondeterministic external arrivals, guard
/// snapshots once per round, and yields rechecked between drained items.
/// Handlers terminate and return Continue (or success for a terminal source);
/// failures, rearming effects, source closure and executor wakes are outside this
/// projection. Quiescence adds no stronger environmental fairness assumption.
/// Model properties cover selected source priority and bounded service windows,
/// not general application liveness or elapsed time.
pub fn reactor_model(arms: &[ReactorArm]) -> Model {
    // pc: 0 arbitration, 1 handling; winner: 0 none, i+1 source; stopped: 1 exit.
    let mut vars = vec![
        Variable::new("pc", 0, 1, 0),
        Variable::new("winner", 0, arms.len() as i64, 0),
        Variable::new(
            "handled",
            0,
            arms.iter().map(|a| a.drain).max().unwrap_or(1) as i64,
            0,
        ),
        Variable::new("stopped", 0, 1, 0),
    ];
    for a in arms {
        vars.push(Variable::new(a.name, 0, 1, 0));
    }
    let mut m = Model::new("YarnReactor", vars);
    // Guards are variables: choose next snapshot only at a new arbitration.
    let guard_start = m.variables.len();
    for a in arms {
        m.variables.push(Variable::new(a.name, 0, 1, 1));
    }
    let running = Expr::Var(3).eq(Expr::Int(0));
    for (i, a) in arms.iter().enumerate() {
        m.actions.push(Action::new(
            &format!("arrive_{}", a.name),
            running.clone().and(Expr::Var(4 + i).eq(Expr::Int(0))),
            vec![(4 + i, Expr::Int(1))],
        ));
        if a.guarded {
            for bit in [0, 1] {
                m.actions.push(Action::new(
                    "guard_snapshot",
                    running.clone().and(Expr::Var(0).eq(Expr::Int(0))),
                    vec![(guard_start + i, Expr::Int(bit))],
                ));
            }
        }
    }
    let eligible = |i: usize| {
        let a = &arms[i];
        let mut e = Expr::Var(4 + i)
            .eq(Expr::Int(1))
            .and(Expr::Var(guard_start + i).eq(Expr::Int(1)));
        if let Some(j) = a.yields_to {
            e = e.and(Expr::Var(4 + j).eq(Expr::Int(0)));
        }
        e
    };
    for (i, a) in arms.iter().enumerate() {
        let mut guard = running
            .clone()
            .and(Expr::Var(0).eq(Expr::Int(0)))
            .and(eligible(i));
        for j in 0..i {
            guard = guard.and(eligible(j).negate());
        }
        let updates = vec![
            (0, Expr::Int(1)),
            (1, Expr::Int((i + 1) as i64)),
            (2, Expr::Int(1)),
            (4 + i, Expr::Int(0)),
        ];
        m.actions
            .push(Action::new(&format!("select_{}", a.name), guard, updates));
        let handling = running
            .clone()
            .and(Expr::Var(0).eq(Expr::Int(1)))
            .and(Expr::Var(1).eq(Expr::Int((i + 1) as i64)));
        let mut can_drain = Expr::Var(2)
            .lt(Expr::Int(a.drain as i64))
            .and(Expr::Var(4 + i).eq(Expr::Int(1)));
        if let Some(j) = a.yields_to {
            can_drain = can_drain.and(Expr::Var(4 + j).eq(Expr::Int(0)));
        }
        if !a.terminal && a.drain > 1 {
            m.actions.push(Action::new(
                "drain",
                handling.clone().and(can_drain.clone()),
                vec![(2, Expr::Var(2).plus(Expr::Int(1))), (4 + i, Expr::Int(0))],
            ));
        }
        let end = if a.terminal {
            handling
        } else if a.drain > 1 {
            handling.and(can_drain.negate())
        } else {
            handling
        };
        let mut updates = vec![(0, Expr::Int(0)), (1, Expr::Int(0)), (2, Expr::Int(0))];
        if a.terminal {
            updates.push((3, Expr::Int(1)));
        }
        m.actions.push(Action::new("finish", end, updates));
    }
    m.check_deadlock = false; // An empty/dormant reactor is intentional; no wake claim.
    let mut bound = Expr::Bool(true);
    for (i, a) in arms.iter().enumerate() {
        bound = bound.and(
            Expr::Var(1)
                .eq(Expr::Int((i + 1) as i64))
                .negate()
                .or(Expr::Var(2).le(Expr::Int(a.drain as i64))),
        );
    }
    m.invariants.push(Invariant {
        name: "bounded_service_window".into(),
        condition: bound,
    });
    m
}
