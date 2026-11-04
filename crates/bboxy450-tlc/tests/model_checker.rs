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

// keeps: 9c native constructs — IF-THEN-ELSE, bounded quantifiers, arrays
// by dynamic index and several initial states are typed and fail closed.
#[test]
fn new_constructs_are_typed_bounded_and_fail_closed() {
    use Expr::{Bool, Bound, Int, Var};
    let mut m = Model::new(
        "Constructs",
        vec![
            Variable::choice("a", 0, 1, &[0, 1]),
            Variable::new("b", 0, 1, 0),
            Variable::new("i", 0, 2, 0),
        ],
    );
    m.check_deadlock = false;
    assert_eq!(m.check(10).unwrap().states.len(), 2, "two initial states");
    // An IF with branches of different types, an unbound quantifier
    // variable, an empty range and an array past the variables are refused.
    for guard in [
        Bool(true).ite(Int(1), Bool(false)),
        Bound(0).eq(Int(0)),
        Expr::forall(2, 1, Bool(true)),
        Expr::pick(1, 3, Int(0)).eq(Int(0)),
    ] {
        m.actions = vec![Action::new("bad", guard, vec![])];
        assert!(m.check(10).is_err());
        assert!(m.tla().is_err());
    }
    // A dynamic index outside its array fails rather than reading a neighbour.
    m.actions = vec![Action::new(
        "read",
        Expr::pick(0, 2, Var(2)).eq(Int(0)),
        vec![(2, Var(2).plus(Int(1)))],
    )];
    assert!(m.check(10).unwrap_err().contains("array index"));
    // Nested quantifiers name their variables by depth.
    m.actions = vec![Action::new(
        "nested",
        Expr::forall(0, 1, Expr::exists(0, 1, Bound(0).eq(Bound(1)))),
        vec![(1, Int(1))],
    )];
    let tla = m.tla().unwrap();
    assert!(
        tla.contains("(\\A q0 \\in (0)..(1) : (\\E q1 \\in (0)..(1) : (q1 = q0)))"),
        "{tla}"
    );
    assert!(tla.contains("/\\ v0 \\in {(0), (1)}"), "{tla}");
    assert!(m.check(10).unwrap().violations.is_empty());
}

// keeps: 9c strong fairness — an action enabled infinitely often (not
// continuously) is forced by SF, not by WF; the refined component check
// still finds a cycle that avoids the action's enabled states.
#[test]
fn strong_fairness_forces_an_intermittently_enabled_action() {
    let mut m = Model::new(
        "Strong",
        vec![
            Variable::new("done", 0, 1, 0),
            Variable::new("tick", 0, 1, 0),
        ],
    );
    let v = Expr::Var;
    m.actions.push(Action::new(
        "serve",
        v(0).eq(Expr::Int(0)).and(v(1).eq(Expr::Int(0))),
        vec![(0, Expr::Int(1))],
    ));
    m.actions.push(Action::new(
        "busy",
        Expr::Bool(true),
        vec![(1, Expr::Int(1).minus(v(1)))],
    ));
    m.check_deadlock = false;
    m.responses.push(Response {
        name: "served".into(),
        request: Expr::Bool(true),
        response: v(0).eq(Expr::Int(1)),
    });
    m.weak_fair = vec![vec![0], vec![1]];
    assert_eq!(
        m.check(10).unwrap().violations.len(),
        1,
        "WF cannot force it"
    );
    m.weak_fair = vec![vec![1]];
    m.strong_fair = vec![vec![0]];
    assert!(m.check(10).unwrap().violations.is_empty(), "SF does");
    assert!(m.tla().unwrap().contains("SF_vars(A0)"));
    // Without fairness on `busy` the behaviour may stop at tick = 1, where
    // `serve` is never enabled: SF alone is satisfied, the response is not.
    m.weak_fair.clear();
    assert_eq!(m.check(10).unwrap().violations.len(), 1);
}
