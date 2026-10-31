// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Small builders for contract models that crates ship with their packages
//! (each crate's `model` feature). A model is a family: the property-holding
//! model and its `Broken` control, which removes one guard and must fail.
//!
//! Adapted from bboxy450-term, crates/bboxy450-term/tests/formal_contracts/models.rs
//! (same project), so shipped and in-tree project models are built alike.
use crate::{Action, Expr, Invariant, Model, Variable};

/// `Var(i) == n`.
pub fn eq(i: usize, n: i64) -> Expr {
    Expr::Var(i).eq(Expr::Int(n))
}

/// `Var(i) != n`.
pub fn ne(i: usize, n: i64) -> Expr {
    eq(i, n).negate()
}

/// The conjunction of `xs` (`TRUE` when empty).
pub fn all(xs: impl IntoIterator<Item = Expr>) -> Expr {
    xs.into_iter().fold(Expr::Bool(true), Expr::and)
}

/// `a => b`.
pub fn implies(a: Expr, b: Expr) -> Expr {
    a.negate().or(b)
}

/// A family member: `<name>` or, when `broken`, `Broken<name>`, over
/// variables `(name, max, initial)` with domain `0..=max`. Deadlock is not
/// checked: completed operations may quiesce.
pub fn model(name: &str, broken: bool, vars: &[(&str, i64, i64)]) -> Model {
    Model {
        name: format!("{}{name}", if broken { "Broken" } else { "" }),
        variables: vars
            .iter()
            .map(|(n, max, initial)| Variable::new(n, 0, *max, *initial))
            .collect(),
        actions: Vec::new(),
        invariants: Vec::new(),
        responses: Vec::new(),
        weak_fair: Vec::new(),
        check_deadlock: false,
    }
}

/// Add one guarded atomic action.
pub fn action(m: &mut Model, name: &str, guard: Expr, updates: &[(usize, Expr)]) {
    m.actions.push(Action::new(name, guard, updates.to_vec()));
}

/// Add one safety invariant.
pub fn safe(m: &mut Model, name: &str, condition: Expr) {
    m.invariants.push(Invariant {
        name: name.into(),
        condition,
    });
}

/// The conventional `NoInvalidEffect` invariant: variable `index` stays 0.
pub fn no_bad(m: &mut Model, index: usize) {
    safe(m, "NoInvalidEffect", eq(index, 0));
}

/// Both members of a family with their expected verdicts: the model passes;
/// the broken control fails liveness when the model has response properties,
/// else safety.
pub fn family(build: fn(bool) -> Model) -> [(Model, &'static str); 2] {
    let good = build(false);
    let temporal = !good.responses.is_empty();
    [
        (good, "pass"),
        (build(true), if temporal { "liveness" } else { "safety" }),
    ]
}

/// The shipped artifacts of one model: `<Name>.tla` and `<Name>.cfg`.
pub fn artifacts(m: &Model) -> Result<[(String, String); 2], String> {
    Ok([
        (format!("{}.tla", m.name), m.tla()?),
        (format!("{}.cfg", m.name), m.config(false)),
    ])
}
