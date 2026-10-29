// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxy450_tlc::{Action, Expr, Model, Response, Variable};

#[test]
fn limits_overflow_and_invalid_updates_cannot_report_success() {
    let mut m = Model::new("Limit", vec![Variable::new("x", 0, 100, 0)]);
    m.actions.push(Action::new(
        "inc",
        Expr::Bool(true),
        vec![(0, Expr::Var(0).plus(Expr::Int(1)))],
    ));
    assert!(m.check(2).unwrap_err().contains("limit"));
    m.variables[0].initial = i64::MAX;
    m.variables[0].max = i64::MAX;
    assert!(m.check(10).unwrap_err().contains("overflow"));
    m.actions[0].updates.push((0, Expr::Int(0)));
    assert!(m.check(10).unwrap_err().contains("assignment"));
}
#[test]
fn domain_escape_and_boolean_mismatch_fail_closed() {
    let mut m = Model::new("Domain", vec![Variable::new("x", 0, 1, 0)]);
    m.actions.push(Action::new(
        "bad",
        Expr::Bool(true),
        vec![(0, Expr::Int(2))],
    ));
    assert!(m.check(10).unwrap_err().contains("TypeOK"));
    m.actions[0].guard = Expr::Int(1);
    assert!(m.check(10).unwrap_err().contains("Boolean"));
}
#[test]
fn stuttering_is_an_infinite_behavior_and_weak_fairness_requires_nonstuttering_steps() {
    let mut m = Model::new("Fair", vec![Variable::new("x", 0, 1, 0)]);
    m.actions.push(Action::new(
        "serve",
        Expr::Var(0).eq(Expr::Int(0)),
        vec![(0, Expr::Int(1))],
    ));
    m.check_deadlock = false;
    m.responses.push(Response {
        name: "progress".into(),
        request: Expr::Bool(true),
        response: Expr::Var(0).eq(Expr::Int(1)),
    });
    let r = m.check(10).unwrap();
    assert_eq!(r.violations.len(), 1);
    assert_eq!(r.violations[0].cycle.len(), 2);
    m.weak_fair.push(vec![0]);
    assert!(m.check(10).unwrap().violations.is_empty());
}

#[test]
fn unreachable_actions_and_short_circuited_expressions_are_still_validated() {
    let mut model = Model::new("Invalid", vec![Variable::new("x", 0, 1, 0)]);
    model.actions.push(Action::new(
        "unreachable",
        Expr::Bool(false),
        vec![(0, Expr::Var(99))],
    ));
    assert!(model.check(10).unwrap_err().contains("unknown variable"));
    assert!(model.tla().unwrap_err().contains("unknown variable"));
    model.actions[0].updates.clear();
    model.actions[0].guard = Expr::Bool(false).and(Expr::Int(1));
    assert!(model.check(10).unwrap_err().contains("Boolean"));
}
