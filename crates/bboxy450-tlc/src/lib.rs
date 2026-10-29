// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A native explicit-state checker, with a separate TLA+ code generator.
//!
//! This is the **Yarn finite integer/Boolean expression subset**, not a parser
//! or complete implementation of TLA+/TLC. States are exact vectors, not hashes.
//! BFS checks safety/deadlock; SCC analysis checks response properties under
//! explicitly named weak-fair action groups, including TLA stuttering.
//! Unsupported expressions, arithmetic overflow and resource exhaustion fail.
//! The generated TLA+ is checked independently by Java TLC in `make test-formal`.
//! Models describe contracts, not the effects of opaque Rust handlers.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::format;
use std::string::{String, ToString};
use std::vec;
use std::vec::Vec;

/// An exact state, in model variable order.
pub type State = Vec<i64>;

/// The expression subset supported by both backends.
#[derive(Clone, Debug)]
pub enum Expr {
    /// An integer literal.
    Int(i64),
    /// A Boolean literal.
    Bool(bool),
    /// An integer state variable by index.
    Var(usize),
    /// Equality of like-typed values.
    Eq(BoxExpr, BoxExpr),
    /// Integer less-than.
    Lt(BoxExpr, BoxExpr),
    /// Integer less-than-or-equal.
    Le(BoxExpr, BoxExpr),
    /// Checked integer addition.
    Add(BoxExpr, BoxExpr),
    /// Checked integer subtraction.
    Sub(BoxExpr, BoxExpr),
    /// Boolean conjunction, evaluated left to right.
    And(BoxExpr, BoxExpr),
    /// Boolean disjunction, evaluated left to right.
    Or(BoxExpr, BoxExpr),
    /// Boolean negation.
    Not(BoxExpr),
}
/// A boxed subexpression.
pub type BoxExpr = std::boxed::Box<Expr>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Value {
    Int(i64),
    Bool(bool),
}
impl Expr {
    /// Compares two expressions for equality.
    pub fn eq(self, other: Self) -> Self {
        Self::Eq(self.into(), other.into())
    }
    /// Compares two integers.
    pub fn lt(self, other: Self) -> Self {
        Self::Lt(self.into(), other.into())
    }
    /// Compares two integers inclusively.
    pub fn le(self, other: Self) -> Self {
        Self::Le(self.into(), other.into())
    }
    /// Adds two integers, failing on overflow.
    pub fn plus(self, other: Self) -> Self {
        Self::Add(self.into(), other.into())
    }
    /// Subtracts two integers, failing on overflow.
    pub fn minus(self, other: Self) -> Self {
        Self::Sub(self.into(), other.into())
    }
    /// Conjoins two Boolean expressions.
    pub fn and(self, other: Self) -> Self {
        Self::And(self.into(), other.into())
    }
    /// Disjoins two Boolean expressions.
    pub fn or(self, other: Self) -> Self {
        Self::Or(self.into(), other.into())
    }
    /// Negates a Boolean expression.
    pub fn negate(self) -> Self {
        Self::Not(self.into())
    }

    // Static type/reference validation includes unreachable actions and the
    // right-hand side of short-circuit expressions; neither backend may silently
    // accept an unsupported expression simply because this corpus did not take it.
    fn integer_kind(&self, variables: usize) -> Result<bool, String> {
        match self {
            Self::Int(_) => Ok(true),
            Self::Bool(_) => Ok(false),
            Self::Var(i) if *i < variables => Ok(true),
            Self::Var(_) => Err("unknown variable".into()),
            Self::Eq(a, b) => {
                if a.integer_kind(variables)? != b.integer_kind(variables)? {
                    return Err("equality operands have different types".into());
                }
                Ok(false)
            }
            Self::Lt(a, b) | Self::Le(a, b) | Self::Add(a, b) | Self::Sub(a, b) => {
                if !a.integer_kind(variables)? || !b.integer_kind(variables)? {
                    return Err("expected an integer".into());
                }
                Ok(matches!(self, Self::Add(..) | Self::Sub(..)))
            }
            Self::And(a, b) | Self::Or(a, b) => {
                if a.integer_kind(variables)? || b.integer_kind(variables)? {
                    return Err("expected a Boolean".into());
                }
                Ok(false)
            }
            Self::Not(a) => {
                if a.integer_kind(variables)? {
                    return Err("expected a Boolean".into());
                }
                Ok(false)
            }
        }
    }

    fn value(&self, state: &State) -> Result<Value, String> {
        let integer = |e: &Expr| match e.value(state)? {
            Value::Int(v) => Ok(v),
            _ => Err("expected an integer".to_string()),
        };
        Ok(match self {
            Self::Int(n) => Value::Int(*n),
            Self::Bool(b) => Value::Bool(*b),
            Self::Var(i) => Value::Int(*state.get(*i).ok_or("unknown variable")?),
            Self::Eq(a, b) => {
                let (a, b) = (a.value(state)?, b.value(state)?);
                if core::mem::discriminant(&a) != core::mem::discriminant(&b) {
                    return Err("equality operands have different types".into());
                }
                Value::Bool(a == b)
            }
            Self::Lt(a, b) => Value::Bool(integer(a)? < integer(b)?),
            Self::Le(a, b) => Value::Bool(integer(a)? <= integer(b)?),
            Self::Add(a, b) => Value::Int(
                integer(a)?
                    .checked_add(integer(b)?)
                    .ok_or("integer overflow")?,
            ),
            Self::Sub(a, b) => Value::Int(
                integer(a)?
                    .checked_sub(integer(b)?)
                    .ok_or("integer overflow")?,
            ),
            Self::And(a, b) => Value::Bool(a.boolean(state)? && b.boolean(state)?),
            Self::Or(a, b) => Value::Bool(a.boolean(state)? || b.boolean(state)?),
            Self::Not(a) => Value::Bool(!a.boolean(state)?),
        })
    }
    fn boolean(&self, state: &State) -> Result<bool, String> {
        match self.value(state)? {
            Value::Bool(b) => Ok(b),
            _ => Err("expected a Boolean".into()),
        }
    }
    fn tla(&self) -> String {
        let binary = |a: &Expr, op: &str, b: &Expr| format!("({} {op} {})", a.tla(), b.tla());
        match self {
            Self::Int(n) => format!("({n})"),
            Self::Bool(true) => "TRUE".into(),
            Self::Bool(false) => "FALSE".into(),
            Self::Var(i) => format!("v{i}"),
            Self::Eq(a, b) => binary(a, "=", b),
            Self::Lt(a, b) => binary(a, "<", b),
            Self::Le(a, b) => binary(a, "<=", b),
            Self::Add(a, b) => binary(a, "+", b),
            Self::Sub(a, b) => binary(a, "-", b),
            Self::And(a, b) => binary(a, "/\\", b),
            Self::Or(a, b) => binary(a, "\\/", b),
            Self::Not(a) => format!("~({})", a.tla()),
        }
    }
}

/// A finite integer variable. Bounds are checked as TypeOK, never as pruning.
#[derive(Clone, Debug)]
pub struct Variable {
    /// Human-readable meaning; TLA names use stable indices.
    pub name: String,
    /// Inclusive minimum.
    pub min: i64,
    /// Inclusive maximum.
    pub max: i64,
    /// Initial value.
    pub initial: i64,
}
impl Variable {
    /// Creates a finite variable.
    pub fn new(name: &str, min: i64, max: i64, initial: i64) -> Self {
        Self {
            name: name.into(),
            min,
            max,
            initial,
        }
    }
}
/// One guarded atomic transition; updates read the same pre-state.
#[derive(Clone, Debug)]
pub struct Action {
    /// Human-readable transition name.
    pub name: String,
    /// Boolean enabling condition.
    pub guard: Expr,
    /// Simultaneous assignments; omitted variables are unchanged.
    pub updates: Vec<(usize, Expr)>,
}
impl Action {
    /// Creates an atomic transition.
    pub fn new(name: &str, guard: Expr, updates: Vec<(usize, Expr)>) -> Self {
        Self {
            name: name.into(),
            guard,
            updates,
        }
    }
}
/// A named safety condition.
#[derive(Clone, Debug)]
pub struct Invariant {
    /// Human-readable property name.
    pub name: String,
    /// Boolean condition required in every reachable state.
    pub condition: Expr,
}
/// A response property `request ~> response` over infinite behaviors.
#[derive(Clone, Debug)]
pub struct Response {
    /// Human-readable property name.
    pub name: String,
    /// An outstanding obligation when true without the response.
    pub request: Expr,
    /// Discharges an obligation when true.
    pub response: Expr,
}
/// A finite specification shared by the native backend and TLA+ exporter.
#[derive(Clone, Debug)]
pub struct Model {
    /// ASCII TLA module name.
    pub name: String,
    /// Ordered finite variables.
    pub variables: Vec<Variable>,
    /// All possible atomic transitions.
    pub actions: Vec<Action>,
    /// Safety properties.
    pub invariants: Vec<Invariant>,
    /// Temporal response properties.
    pub responses: Vec<Response>,
    /// Weak fairness on each disjunction of action indices. No fairness is implicit.
    pub weak_fair: Vec<Vec<usize>>,
    /// Whether a state with no enabled explicit action is a violation.
    pub check_deadlock: bool,
}
/// A labeled edge in the reachable graph.
#[derive(Clone, Debug)]
pub struct Edge {
    /// Action index. Implicit TLA stuttering is analyzed separately.
    pub action: usize,
    /// Successor index in `Report::states`.
    pub to: usize,
}
/// A concrete safety, deadlock or temporal counterexample.
#[derive(Clone, Debug)]
pub struct Violation {
    /// Stable property name (`TypeOK`, `Deadlock` or a model property).
    pub property: String,
    /// Prefix from the initial state to the bad state or accepting cycle.
    pub prefix: Vec<usize>,
    /// A fair closed walk for liveness; empty for safety/deadlock.
    pub cycle: Vec<usize>,
}
/// Completed reachable graph and all violated properties.
#[derive(Clone, Debug)]
pub struct Report {
    /// Exact state vectors, without fingerprint collisions.
    pub states: Vec<State>,
    /// Explicit transitions, including explicit self-loops.
    pub edges: Vec<Vec<Edge>>,
    /// Counterexamples, with graph indices.
    pub violations: Vec<Violation>,
}

impl Model {
    /// Creates an empty model with deadlock checking enabled.
    pub fn new(name: &str, variables: Vec<Variable>) -> Self {
        Self {
            name: name.into(),
            variables,
            actions: vec![],
            invariants: vec![],
            responses: vec![],
            weak_fair: vec![],
            check_deadlock: true,
        }
    }
    /// Exhaustively explores the finite reachable graph. A limit is an error,
    /// never a successful partial verification.
    pub fn check(&self, max_states: usize) -> Result<Report, String> {
        self.validate()?;
        if max_states == 0 {
            return Err("state limit must be positive".into());
        }
        let initial: State = self.variables.iter().map(|v| v.initial).collect();
        let mut index = BTreeMap::from([(initial.clone(), 0)]);
        let mut report = Report {
            states: vec![initial],
            edges: vec![],
            violations: vec![],
        };
        let mut parents = vec![None];
        let mut cursor = 0;
        while cursor < report.states.len() {
            let state = report.states[cursor].clone();
            for (i, var) in self.variables.iter().enumerate() {
                if !(var.min..=var.max).contains(&state[i]) {
                    return Err(format!(
                        "TypeOK: {} outside {}..={} at {:?}",
                        var.name, var.min, var.max, state
                    ));
                }
            }
            let mut edges = vec![];
            for (action, rule) in self.actions.iter().enumerate() {
                if !rule.guard.boolean(&state)? {
                    continue;
                }
                let mut next = state.clone();
                for (i, expression) in &rule.updates {
                    next[*i] = match expression.value(&state)? {
                        Value::Int(n) => n,
                        _ => return Err("assignment requires an integer".into()),
                    };
                }
                let to = if let Some(to) = index.get(&next) {
                    *to
                } else {
                    if report.states.len() >= max_states {
                        return Err(format!("state limit {max_states} exceeded"));
                    }
                    let to = report.states.len();
                    index.insert(next.clone(), to);
                    report.states.push(next);
                    parents.push(Some(cursor));
                    to
                };
                edges.push(Edge { action, to });
            }
            if edges.is_empty() && self.check_deadlock {
                report.violations.push(Violation {
                    property: "Deadlock".into(),
                    prefix: prefix(cursor, &parents),
                    cycle: vec![],
                });
            }
            report.edges.push(edges);
            cursor += 1;
        }
        for inv in &self.invariants {
            for (i, state) in report.states.iter().enumerate() {
                if !inv.condition.boolean(state)? {
                    report.violations.push(Violation {
                        property: inv.name.clone(),
                        prefix: prefix(i, &parents),
                        cycle: vec![],
                    });
                    break;
                }
            }
        }
        for response in &self.responses {
            if let Some(violation) = self.check_response(&report, response, &parents)? {
                report.violations.push(violation);
            }
        }
        Ok(report)
    }

    fn validate(&self) -> Result<(), String> {
        if self.variables.is_empty()
            || self.name.is_empty()
            || !self
                .name
                .bytes()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
            || !self
                .name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            return Err("model requires variables and an ASCII module identifier".into());
        }
        for v in &self.variables {
            if v.min > v.max {
                return Err("invalid variable domain".into());
            }
        }
        let variables = self.variables.len();
        for expression in self
            .actions
            .iter()
            .map(|a| &a.guard)
            .chain(self.invariants.iter().map(|i| &i.condition))
            .chain(
                self.responses
                    .iter()
                    .flat_map(|r| [&r.request, &r.response]),
            )
        {
            if expression.integer_kind(variables)? {
                return Err("expected a Boolean".into());
            }
        }
        for a in &self.actions {
            let mut seen = BTreeSet::new();
            for (i, expression) in &a.updates {
                if !expression.integer_kind(variables)? {
                    return Err("assignment requires an integer".into());
                }
                if *i >= self.variables.len() || !seen.insert(*i) {
                    return Err("invalid or repeated assignment".into());
                }
            }
        }
        if self
            .weak_fair
            .iter()
            .any(|g| g.is_empty() || g.iter().any(|a| *a >= self.actions.len()))
        {
            return Err("invalid fairness action group".into());
        }
        Ok(())
    }

    fn check_response(
        &self,
        r: &Report,
        p: &Response,
        parents: &[Option<usize>],
    ) -> Result<Option<Violation>, String> {
        // Restrict to paths with an outstanding obligation and no response yet.
        let eligible = r
            .states
            .iter()
            .map(|s| p.response.boolean(s).map(|b| !b))
            .collect::<Result<Vec<_>, _>>()?;
        let mut from_obligation = vec![None; r.states.len()];
        let mut queue = VecDeque::new();
        for (i, s) in r.states.iter().enumerate() {
            if eligible[i] && p.request.boolean(s)? {
                from_obligation[i] = Some(i);
                queue.push_back(i);
            }
        }
        while let Some(i) = queue.pop_front() {
            for e in &r.edges[i] {
                if eligible[e.to] && from_obligation[e.to].is_none() {
                    from_obligation[e.to] = Some(i);
                    queue.push_back(e.to);
                }
            }
        }
        let allowed: Vec<bool> = from_obligation.iter().map(Option::is_some).collect();
        for component in components(&r.edges, &allowed) {
            let members: BTreeSet<usize> = component.iter().copied().collect();
            // Every state permits implicit stuttering. WF only counts nonstuttering steps.
            let fair = self.weak_fair.iter().all(|group| {
                component.iter().any(|&i| {
                    !r.edges[i]
                        .iter()
                        .any(|e| e.to != i && group.contains(&e.action))
                }) || component.iter().any(|&i| {
                    r.edges[i]
                        .iter()
                        .any(|e| e.to != i && members.contains(&e.to) && group.contains(&e.action))
                })
            });
            if !fair {
                continue;
            }
            let root = component[0];
            let mut lead = vec![root];
            let mut i = root;
            while from_obligation[i] != Some(i) {
                i = from_obligation[i].expect("reachable obligation");
                lead.push(i);
            }
            lead.reverse();
            let mut path = prefix(i, parents);
            path.extend(lead.into_iter().skip(1));
            // Visit every member and fair internal edge, so the reported cycle
            // itself witnesses all fairness obligations; a random SCC cycle may not.
            let mut cycle = vec![root];
            for &node in &component {
                append_path(&mut cycle, node, &r.edges, &members);
            }
            for group in &self.weak_fair {
                if let Some((from, e)) = component.iter().find_map(|&from| {
                    r.edges[from]
                        .iter()
                        .find(|e| {
                            e.to != from && members.contains(&e.to) && group.contains(&e.action)
                        })
                        .map(|e| (from, e))
                }) {
                    append_path(&mut cycle, from, &r.edges, &members);
                    cycle.push(e.to);
                }
            }
            append_path(&mut cycle, root, &r.edges, &members);
            if cycle.len() == 1 {
                cycle.push(root);
            }
            return Ok(Some(Violation {
                property: p.name.clone(),
                prefix: path,
                cycle,
            }));
        }
        Ok(None)
    }

    /// Emits readable TLA+ from expressions/actions, independently of the
    /// native reachable graph. Does not serialize native results as a spec.
    pub fn tla(&self) -> Result<String, String> {
        self.validate()?;
        let names: Vec<String> = (0..self.variables.len()).map(|i| format!("v{i}")).collect();
        let mut out = format!(
            "---- MODULE {} ----\n\\* Generated by bboxy450-tlc — https://github.com/bboxy450/bboxy450-tlc\n\\* Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md\nEXTENDS Integers\nVARIABLES {}\nvars == <<{}>>\n",
            self.name,
            names.join(", "),
            names.join(", ")
        );
        out.push_str("TypeOK ==\n");
        for (i, v) in self.variables.iter().enumerate() {
            out.push_str(&format!("  /\\ v{i} \\in ({})..({})\n", v.min, v.max));
        }
        out.push_str("Init ==\n");
        for (i, v) in self.variables.iter().enumerate() {
            out.push_str(&format!("  /\\ v{i} = ({})\n", v.initial));
        }
        for (i, a) in self.actions.iter().enumerate() {
            out.push_str(&format!("A{i} ==\n  /\\ {}\n", a.guard.tla()));
            for (j, _) in self.variables.iter().enumerate() {
                let value = a
                    .updates
                    .iter()
                    .find(|(k, _)| *k == j)
                    .map_or_else(|| format!("v{j}"), |(_, e)| e.tla());
                out.push_str(&format!("  /\\ v{j}' = {value}\n"));
            }
        }
        let actions: Vec<_> = (0..self.actions.len()).map(|i| format!("A{i}")).collect();
        out.push_str(&format!(
            "Next == {}\nSpec == Init /\\ [][Next]_vars\n",
            if actions.is_empty() {
                "FALSE".into()
            } else {
                actions.join(" \\/ ")
            }
        ));
        for group in &self.weak_fair {
            let actions: Vec<_> = group.iter().map(|i| format!("A{i}")).collect();
            out.push_str(&format!("  /\\ WF_vars({})\n", actions.join(" \\/ ")));
        }
        for (i, p) in self.invariants.iter().enumerate() {
            out.push_str(&format!("Inv{i} == {}\n", p.condition.tla()));
        }
        for (i, p) in self.responses.iter().enumerate() {
            out.push_str(&format!(
                "Live{i} == ({}) ~> ({})\n",
                p.request.tla(),
                p.response.tla()
            ));
        }
        out.push_str("====\n");
        Ok(out)
    }
    /// TLC configuration. Graph mode omits checked properties/deadlock to allow
    /// exact comparison of negative-control transition graphs as well.
    pub fn config(&self, graph_only: bool) -> String {
        let mut s = "SPECIFICATION Spec\nINVARIANT TypeOK\n".to_string();
        if graph_only || !self.check_deadlock {
            s.push_str("CHECK_DEADLOCK FALSE\n");
        }
        if !graph_only {
            for i in 0..self.invariants.len() {
                s.push_str(&format!("INVARIANT Inv{i}\n"));
            }
            for i in 0..self.responses.len() {
                s.push_str(&format!("PROPERTY Live{i}\n"));
            }
        }
        s
    }
}

fn prefix(mut i: usize, parents: &[Option<usize>]) -> Vec<usize> {
    let mut p = vec![i];
    while let Some(j) = parents[i] {
        i = j;
        p.push(i);
    }
    p.reverse();
    p
}
// Iterative Kosaraju avoids stack overflow on a long model path.
fn components(edges: &[Vec<Edge>], allowed: &[bool]) -> Vec<Vec<usize>> {
    let mut seen = vec![false; edges.len()];
    let mut order = vec![];
    for root in 0..edges.len() {
        if !allowed[root] || seen[root] {
            continue;
        }
        seen[root] = true;
        let mut stack = vec![(root, 0)];
        while let Some((i, n)) = stack.last_mut() {
            if *n < edges[*i].len() {
                let to = edges[*i][*n].to;
                *n += 1;
                if allowed[to] && !seen[to] {
                    seen[to] = true;
                    stack.push((to, 0));
                }
            } else {
                order.push(*i);
                stack.pop();
            }
        }
    }
    let mut reverse = vec![vec![]; edges.len()];
    for (i, es) in edges.iter().enumerate() {
        for e in es {
            reverse[e.to].push(i);
        }
    }
    seen.fill(false);
    let mut result = vec![];
    for root in order.into_iter().rev() {
        if seen[root] {
            continue;
        }
        let mut component = vec![];
        let mut stack = vec![root];
        seen[root] = true;
        while let Some(i) = stack.pop() {
            component.push(i);
            for &j in &reverse[i] {
                if allowed[j] && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        result.push(component);
    }
    result
}
fn append_path(
    path: &mut Vec<usize>,
    target: usize,
    edges: &[Vec<Edge>],
    members: &BTreeSet<usize>,
) {
    let root = *path.last().expect("nonempty path");
    let mut parents = vec![None; edges.len()];
    parents[root] = Some(root);
    let mut queue = VecDeque::from([root]);
    while parents[target].is_none() {
        let i = queue.pop_front().expect("strongly connected component");
        for e in &edges[i] {
            if members.contains(&e.to) && parents[e.to].is_none() {
                parents[e.to] = Some(i);
                queue.push_back(e.to);
            }
        }
    }
    let mut tail = vec![];
    let mut i = target;
    while i != root {
        tail.push(i);
        i = parents[i].expect("connected path");
    }
    path.extend(tail.into_iter().rev());
}
