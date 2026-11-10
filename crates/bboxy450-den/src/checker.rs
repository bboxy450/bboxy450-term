// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::{Diagnostic, Report, Summary, ast::*, eval::*, lower, parser, types};
use std::collections::BTreeSet;

pub(crate) fn check(source: &str) -> Report {
    let spec = match parser::parse(source) {
        Ok(s) => s,
        Err(diagnostics) => {
            return Report {
                records: vec![],
                diagnostics,
            };
        }
    };
    let s = &spec;
    let mut summary = Summary {
        kind: "spec",
        name: s.name.text.clone(),
        intent: s.intent.clone(),
        domains: s
            .enums
            .iter()
            .map(|d| (d.name.text.clone(), d.members.len()))
            .collect(),
        states: 0,
        transitions: 0,
        invariants: s
            .invariants
            .iter()
            .map(|i| (i.name.text.clone(), i.text.clone()))
            .collect(),
        assumptions: s
            .assumptions
            .iter()
            .map(|a| (a.name.text.clone(), a.text.clone()))
            .collect(),
        scenarios: s.scenarios.len(),
    };
    // Module identity is parsed here; filesystem/import resolution belongs to the next version.
    let _module = &s.module;
    let mut diagnostics = types::names(s);
    if diagnostics.is_empty() {
        diagnostics = types::types(s)
    }
    if diagnostics.is_empty() {
        diagnostics = argument_bounds(s)
    }
    if diagnostics.is_empty() {
        diagnostics = totality(s)
    }
    if diagnostics.is_empty() {
        diagnostics = steps(s)
    }
    if diagnostics.is_empty() {
        diagnostics = model(s, &mut summary)
    }
    if diagnostics.is_empty() {
        diagnostics = scenarios(s)
    }
    if diagnostics.is_empty() {
        diagnostics = completeness(s)
    }
    Report {
        records: vec![summary],
        diagnostics,
    }
}
fn too_large(s: &Spec, pos: Pos) -> Diagnostic {
    diagnostic(
        s,
        pos,
        "X603",
        "the finite state/argument space exceeds the checking limit",
        "reduce the finite domains or state variables",
    )
}
fn universe(s: &Spec) -> Option<Vec<State>> {
    product(
        s,
        &s.fields
            .iter()
            .map(|f| f.ty.kind.clone())
            .collect::<Vec<_>>(),
        VALUATIONS,
    )
}
fn args(s: &Spec, h: &Handler) -> Option<Vec<Vec<Value>>> {
    let ev = s
        .events
        .iter()
        .find(|e| e.name.text == h.event.text)
        .expect("checked event");
    product(
        s,
        &ev.params
            .iter()
            .map(|p| p.ty.kind.clone())
            .collect::<Vec<_>>(),
        VALUATIONS,
    )
}
// Check value-dependent bounded argument types without requiring unbounded enumeration.
// Oversized universes are rejected by the totality limit immediately after this group.
fn argument_bounds(s: &Spec) -> Vec<Diagnostic> {
    let Some(states) = universe(s) else {
        return vec![];
    };
    for inv in &s.invariants {
        for state in &states {
            if let Err(error) = eval(s, &inv.expr, state, &vec![], Some(&Ty::Bool)) {
                return vec![diagnostic(
                    s,
                    inv.expr.pos,
                    "X301",
                    error,
                    "keep invariant arithmetic within the supported integer range",
                )];
            }
        }
    }
    for h in &s.handlers {
        let Some(args) = args(s, h) else { continue };
        if states
            .len()
            .checked_mul(args.len())
            .is_none_or(|n| n > VALUATIONS)
        {
            continue;
        }
        for state in &states {
            for a in &args {
                let p = bindings(s, h, a);
                let Ok(selected) = selected(s, h, state, &p) else {
                    continue;
                };
                for ci in selected {
                    for stmt in &h.cases[ci].stmts {
                        if let StmtKind::Emit(call) = &stmt.kind {
                            let signature = s
                                .effects
                                .iter()
                                .find(|e| e.name.text == call.name.text)
                                .expect("checked effect");
                            for (expr, param) in call.args.iter().zip(&signature.params) {
                                if !eval(s, expr, state, &p, Some(&param.ty.kind))
                                    .is_ok_and(|value| in_range(s, &value, &param.ty.kind))
                                {
                                    return vec![diagnostic(
                                        s,
                                        expr.pos,
                                        "X303",
                                        format!(
                                            "argument {} of {} is outside its declared type at {}",
                                            param.name.text,
                                            call.name.text,
                                            describe(s, state, &p)
                                        ),
                                        "guard the effect or pass an argument within its declared type",
                                    )];
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    vec![]
}
fn totality(s: &Spec) -> Vec<Diagnostic> {
    let Some(states) = universe(s) else {
        return vec![too_large(s, s.name.pos)];
    };
    let mut ds = vec![];
    for h in &s.handlers {
        let Some(args) = args(s, h) else {
            return vec![too_large(s, h.event.pos)];
        };
        if states
            .len()
            .checked_mul(args.len())
            .is_none_or(|n| n > VALUATIONS)
        {
            return vec![too_large(s, h.event.pos)];
        }
        let mut overlap = false;
        let mut gap = false;
        for state in &states {
            for a in &args {
                let p = bindings(s, h, a);
                match selected(s, h, state, &p) {
                    Ok(cases) if cases.len() > 1 && !overlap => {
                        overlap = true;
                        ds.push(diagnostic(
                            s,
                            h.event.pos,
                            "X401",
                            format!(
                                "{} selects multiple cases at {}",
                                h.event.text,
                                describe(s, state, &p)
                            ),
                            "make the case guards disjoint",
                        ))
                    }
                    Ok(cases) if cases.is_empty() && !gap => {
                        gap = true;
                        ds.push(diagnostic(
                            s,
                            h.event.pos,
                            "X402",
                            format!(
                                "{} selects no case at {}",
                                h.event.text,
                                describe(s, state, &p)
                            ),
                            "cover the missing values or add an otherwise case",
                        ))
                    }
                    Err(e) => {
                        return vec![diagnostic(
                            s,
                            h.event.pos,
                            "X301",
                            e,
                            "keep guard arithmetic within the supported integer range",
                        )];
                    }
                    _ => {}
                }
            }
        }
    }
    ds
}
fn steps(s: &Spec) -> Vec<Diagnostic> {
    let mut ds = vec![];
    for h in &s.handlers {
        for c in &h.cases {
            let mut assigned = BTreeSet::new();
            for stmt in &c.stmts {
                if let StmtKind::Assign(n, _, _) = &stmt.kind
                    && !assigned.insert(&n.text)
                {
                    ds.push(diagnostic(
                        s,
                        stmt.pos,
                        "X501",
                        format!("{} assigns {} twice", c.name.text, n.text),
                        "assign each state variable at most once per case",
                    ))
                }
            }
            if !matches!(c.outcome, Outcome::Accept) && !c.stmts.is_empty() {
                ds.push(diagnostic(
                    s,
                    c.name.pos,
                    "X502",
                    "a rejected or ignored case updates state or emits an effect",
                    "remove the updates and effects or accept the event",
                ))
            }
        }
    }
    if !ds.is_empty() {
        return ds;
    }
    let states = universe(s).expect("totality bounded");
    for h in &s.handlers {
        let mut noops = BTreeSet::new();
        for a in args(s, h).expect("totality bounded") {
            let p = bindings(s, h, &a);
            for state in &states {
                let ci = selected(s, h, state, &p).expect("totality checked")[0];
                if !noops.contains(&ci)
                    && let Ok(Decision::Accepted(next, effects)) = apply(s, &h.cases[ci], state, &p)
                    && next == *state
                    && effects.is_empty()
                {
                    noops.insert(ci);
                    ds.push(diagnostic(s,h.cases[ci].name.pos,"X503",format!("accepting case {} changes nothing and emits nothing at {}",h.cases[ci].name.text,describe(s,state,&p)),"ignore this event or make the accepted step change state or emit an effect"));
                }
            }
        }
    }
    ds
}
fn model(s: &Spec, summary: &mut Summary) -> Vec<Diagnostic> {
    let m = match lower::model(s) {
        Ok(m) => m,
        Err(e) => {
            return vec![diagnostic(
                s,
                s.name.pos,
                "X602",
                e,
                "keep updates within the declared variable ranges",
            )];
        }
    };
    let graph = match m.check(STATES) {
        Ok(g) => g,
        Err(e) => {
            return vec![diagnostic(
                s,
                s.name.pos,
                if e.contains("limit") { "X603" } else { "X602" },
                e,
                "reduce the bounds or keep updates within their declared ranges",
            )];
        }
    };
    summary.states = graph.states.len();
    summary.transitions = graph.edges.iter().map(Vec::len).sum();
    let ds: Vec<_> = graph
        .violations
        .iter()
        .filter_map(|v| {
            s.invariants
                .iter()
                .find(|i| i.name.text == v.property)
                .map(|inv| {
                    let trace = v
                        .prefix
                        .windows(2)
                        .map(|w| {
                            let edge = graph.edges[w[0]]
                                .iter()
                                .find(|e| e.to == w[1])
                                .expect("trace edge");
                            m.actions[edge.action]
                                .name
                                .split('/')
                                .next()
                                .unwrap_or_default()
                                .to_string()
                        })
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    diagnostic(
                        s,
                        inv.name.pos,
                        "X601",
                        format!(
                            "invariant {} is false; shortest trace: {}",
                            inv.name.text,
                            if trace.is_empty() {
                                "initial state"
                            } else {
                                &trace
                            }
                        ),
                        "repair the transition or the invariant",
                    )
                })
        })
        .collect();
    if !ds.is_empty() {
        return ds;
    }
    // Explicit rejected and ignored self-loops also count as taking a case.
    let mut ds = vec![];
    let mut taken = BTreeSet::new();
    for edges in &graph.edges {
        for edge in edges {
            let suffix = m.actions[edge.action]
                .name
                .rsplit('#')
                .next()
                .expect("action case");
            taken.insert(suffix.to_string());
        }
    }
    for (hi, h) in s.handlers.iter().enumerate() {
        for (ci, c) in h.cases.iter().enumerate() {
            if !taken.contains(&format!("{hi}:{ci}")) {
                ds.push(diagnostic(
                    s,
                    c.name.pos,
                    "X701",
                    format!("case {} is never taken from a reachable state", c.name.text),
                    "remove the case or make its guard reachable",
                ))
            }
        }
    }
    let states = universe(s).expect("totality bounded");
    for inv in &s.invariants {
        if states
            .iter()
            .all(|st| eval(s, &inv.expr, st, &vec![], Some(&Ty::Bool)).is_ok_and(|v| v.truth()))
        {
            ds.push(diagnostic(
                s,
                inv.name.pos,
                "X702",
                format!(
                    "invariant {} cannot be false in any state of the declared types",
                    inv.name.text
                ),
                "state a property that excludes some typed states",
            ))
        }
    }
    ds
}
fn scenarios(s: &Spec) -> Vec<Diagnostic> {
    let mut ds = vec![];
    for scenario in &s.scenarios {
        let mut state = initial(s).expect("checked initial");
        for step_ in &scenario.steps {
            match step_ {
                ScenarioStep::Expect(e) => {
                    if !eval(s, e, &state, &vec![], Some(&Ty::Bool)).is_ok_and(|v| v.truth()) {
                        ds.push(diagnostic(
                            s,
                            e.pos,
                            "X802",
                            format!("expect is false in scenario {}", scenario.name),
                            "correct the expectation or the preceding steps",
                        ));
                        break;
                    }
                }
                ScenarioStep::Call {
                    call,
                    outcome,
                    effects,
                } => {
                    let h = s
                        .handlers
                        .iter()
                        .find(|h| h.event.text == call.name.text)
                        .expect("checked handler");
                    let ev = s
                        .events
                        .iter()
                        .find(|e| e.name.text == call.name.text)
                        .expect("checked event");
                    let a = call
                        .args
                        .iter()
                        .zip(&ev.params)
                        .map(|(e, p)| eval(s, e, &state, &vec![], Some(&p.ty.kind)))
                        .collect::<Result<Vec<_>, _>>();
                    let a = match a {
                        Ok(a)
                            if a.iter()
                                .zip(&ev.params)
                                .all(|(v, p)| in_range(s, v, &p.ty.kind)) =>
                        {
                            a
                        }
                        _ => {
                            ds.push(diagnostic(
                                s,
                                call.name.pos,
                                "X303",
                                "an event argument is outside its declared type",
                                "pass arguments within the event's declared types",
                            ));
                            break;
                        }
                    };
                    let result = step(s, h, &state, &a).map(|(_, d)| d);
                    let expected_effects = effects
                        .iter()
                        .map(|e| call_effect(s, e, &state, &vec![]))
                        .collect::<Result<Vec<_>, _>>();
                    if expected_effects.is_err() {
                        ds.push(diagnostic(
                            s,
                            call.name.pos,
                            "X303",
                            "an expected effect argument is outside its declared type",
                            "pass arguments within the effect's declared types",
                        ));
                        break;
                    }
                    let matched = match (&result, outcome, &expected_effects) {
                        (Ok(Decision::Accepted(_, actual)), Outcome::Accept, Ok(expected)) => {
                            actual == expected
                        }
                        (Ok(Decision::Rejected(r)), Outcome::Reject(n), _)
                        | (Ok(Decision::Ignored(r)), Outcome::Ignore(n), _) => {
                            *r == n.text && effects.is_empty()
                        }
                        _ => false,
                    };
                    if !matched {
                        ds.push(diagnostic(s,call.name.pos,"X801",format!("{} has a different outcome or effects in scenario {}; observed {result:?}",call.name.text,scenario.name),"write the actual outcome and exact emitted effects or repair the event"));
                        break;
                    }
                    if let Ok(Decision::Accepted(next, _)) = result {
                        state = next
                    }
                }
            }
        }
    }
    ds
}
fn completeness(s: &Spec) -> Vec<Diagnostic> {
    let mut ds = vec![];
    if s.intent.trim().is_empty() {
        ds.push(diagnostic(
            s,
            s.name.pos,
            "X901",
            "intent is missing or empty",
            "add a nonempty intent string",
        ))
    }
    if s.invariants.is_empty() {
        ds.push(diagnostic(
            s,
            s.name.pos,
            "X902",
            "the spec has no invariant",
            "state at least one nonvacuous safety invariant",
        ))
    }
    if s.scenarios.is_empty() {
        ds.push(diagnostic(
            s,
            s.name.pos,
            "X903",
            "the spec has no scenario",
            "add a checked event trace and expectations",
        ))
    }
    ds
}
