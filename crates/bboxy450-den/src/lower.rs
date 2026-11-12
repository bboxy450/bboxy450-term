// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-term crates/bbt-yarn-macros/src/protocol.rs, Checked::lower
// and generate_model: https://github.com/bboxy450/bboxy450-term
// Sets retain one Boolean model variable per member; enum values retain ordinal encoding.
// Changes: lower the hand-written Den AST directly into the existing TLC expression API.
use crate::{ast::*, eval::*, types::expr_type};
use bboxy450_tlc::{Action, Expr as E, Invariant, Model, Response, Variable};

enum Lowered {
    Scalar(E),
    Set(Vec<E>),
}
impl Lowered {
    fn scalar(self) -> E {
        match self {
            Self::Scalar(e) => e,
            _ => unreachable!("checked scalar"),
        }
    }
    fn set(self) -> Vec<E> {
        match self {
            Self::Set(e) => e,
            _ => unreachable!("checked set"),
        }
    }
}
fn and(terms: impl IntoIterator<Item = E>) -> E {
    terms.into_iter().fold(E::Bool(true), E::and)
}
fn or(terms: impl IntoIterator<Item = E>) -> E {
    terms.into_iter().fold(E::Bool(false), E::or)
}
fn width(s: &Spec, ty: &Ty) -> usize {
    match ty {
        Ty::Set(d) => domain(s, d).members.len(),
        _ => 1,
    }
}
fn offset(s: &Spec, i: usize) -> usize {
    s.fields[..i].iter().map(|f| width(s, &f.ty.kind)).sum()
}
fn constant(ty: &Ty, v: &Value) -> Lowered {
    match v {
        Value::Set(bits) => Lowered::Set(bits.iter().map(|b| E::Bool(*b)).collect()),
        Value::Scalar(n) => Lowered::Scalar(if *ty == Ty::Bool {
            E::Bool(*n != 0)
        } else {
            E::Int(*n)
        }),
    }
}
fn expression(s: &Spec, e: &Expr, params: &Bindings, hint: Option<&Ty>) -> Result<Lowered, String> {
    let recurse = |e: &Expr, h: Option<&Ty>| expression(s, e, params, h);
    let ty = expr_type(s, e, &binding_types(params), hint).map_err(|d| d.message)?;
    Ok(match &e.kind {
        ExprKind::Bool(b) => Lowered::Scalar(E::Bool(*b)),
        ExprKind::Int(n) => Lowered::Scalar(E::Int(*n)),
        ExprKind::Name(n) => {
            if let Some((_, t, v)) = params.iter().rev().find(|(name, _, _)| name == n) {
                constant(t, v)
            } else {
                let i = s
                    .fields
                    .iter()
                    .position(|f| f.name.text == *n)
                    .expect("checked field");
                let start = offset(s, i);
                match &ty {
                    Ty::Set(_) => Lowered::Set(
                        (start..start + width(s, &ty))
                            .map(|i| E::Var(i).eq(E::Int(1)))
                            .collect(),
                    ),
                    Ty::Bool => Lowered::Scalar(E::Var(start).eq(E::Int(1))),
                    _ => Lowered::Scalar(E::Var(start)),
                }
            }
        }
        ExprKind::Member(d, m) => Lowered::Scalar(E::Int(
            domain(s, d)
                .members
                .iter()
                .position(|n| n.text == *m)
                .expect("checked member") as i64,
        )),
        ExprKind::Call(n, args) if n == "Set" => {
            let Ty::Set(d) = &ty else {
                unreachable!("checked set")
            };
            let elements = args
                .iter()
                .map(|a| recurse(a, Some(&Ty::Named(d.clone()))).map(Lowered::scalar))
                .collect::<Result<Vec<_>, _>>()?;
            Lowered::Set(
                (0..domain(s, d).members.len())
                    .map(|i| or(elements.iter().map(|a| a.clone().eq(E::Int(i as i64)))))
                    .collect(),
            )
        }
        ExprKind::Call(n, args) => {
            let field = Expr {
                kind: ExprKind::Name(n.clone()),
                pos: e.pos,
            };
            let bits = recurse(&field, None)?.set();
            let member = recurse(&args[0], None)?.scalar();
            Lowered::Scalar(or(bits
                .into_iter()
                .enumerate()
                .map(|(i, b)| member.clone().eq(E::Int(i as i64)).and(b))))
        }
        ExprKind::Not(a) => Lowered::Scalar(recurse(a, Some(&Ty::Bool))?.scalar().negate()),
        ExprKind::Binary(op, a, b) => {
            let ty = expr_type(s, a, &binding_types(params), None)
                .or_else(|_| expr_type(s, b, &binding_types(params), None))
                .map_err(|d| d.message)?;
            let (a, b) = (recurse(a, Some(&ty))?, recurse(b, Some(&ty))?);
            let result = match (a, b) {
                (Lowered::Set(a), Lowered::Set(b)) => {
                    let eq = and(a.into_iter().zip(b).map(|(a, b)| a.eq(b)));
                    if op == "!=" { eq.negate() } else { eq }
                }
                (Lowered::Scalar(a), Lowered::Scalar(b)) => match op.as_str() {
                    "==" => a.eq(b),
                    "!=" => a.eq(b).negate(),
                    "<" => a.lt(b),
                    "<=" => a.le(b),
                    ">" => b.lt(a),
                    ">=" => b.le(a),
                    "+" => a.plus(b),
                    "-" => a.minus(b),
                    "&&" => a.and(b),
                    "||" => a.or(b),
                    "==>" => a.negate().or(b),
                    _ => unreachable!("checked op"),
                },
                _ => unreachable!("checked binary operands"),
            };
            Lowered::Scalar(result)
        }
        ExprKind::If(c, a, b) => {
            let c = recurse(c, Some(&Ty::Bool))?.scalar();
            match (recurse(a, Some(&ty))?, recurse(b, Some(&ty))?) {
                (Lowered::Scalar(a), Lowered::Scalar(b)) => Lowered::Scalar(c.ite(a, b)),
                (Lowered::Set(a), Lowered::Set(b)) => Lowered::Set(
                    a.into_iter()
                        .zip(b)
                        .map(|(a, b)| c.clone().ite(a, b))
                        .collect(),
                ),
                _ => unreachable!("checked branches"),
            }
        }
        ExprKind::Quant {
            all,
            binding,
            domain: d,
            body,
        } => {
            let mut terms = vec![];
            let mut p = params.clone();
            for n in 0..domain(s, &d.text).members.len() {
                p.push((
                    binding.text.clone(),
                    Ty::Named(d.text.clone()),
                    Value::Scalar(n as i64),
                ));
                terms.push(expression(s, body, &p, Some(&Ty::Bool))?.scalar());
                p.pop();
            }
            Lowered::Scalar(if *all { and(terms) } else { or(terms) })
        }
    })
}
pub(crate) fn model(s: &Spec) -> Result<Model, String> {
    let init = initial(s)?;
    let mut vars = vec![];
    for (i, f) in s.fields.iter().enumerate() {
        match (&f.ty.kind, &init[i]) {
            (Ty::Set(d), Value::Set(bits)) => {
                for (m, b) in domain(s, d).members.iter().zip(bits) {
                    vars.push(Variable::new(
                        &format!("{}.{}", f.name.text, m.text),
                        0,
                        1,
                        i64::from(*b),
                    ))
                }
            }
            (Ty::Int(lo, hi), Value::Scalar(n)) => {
                vars.push(Variable::new(&f.name.text, *lo, *hi, *n))
            }
            (ty, Value::Scalar(n)) => vars.push(Variable::new(
                &f.name.text,
                0,
                cardinality(s, ty) as i64 - 1,
                *n,
            )),
            _ => unreachable!("checked initial"),
        }
    }
    // TLC requires a variable even for an effect-only machine. This constant has no Den state.
    if vars.is_empty() {
        vars.push(Variable::new("unit", 0, 0, 0));
    }
    let mut m = Model::new(&format!("Den{}", s.name.text), vars);
    m.check_deadlock = false;
    // One case is one fairness group over all its argument values, as in Yarn.
    let mut groups: std::collections::BTreeMap<(usize, usize), Vec<usize>> = Default::default();
    for (hi, args) in instances(s).ok_or("event instance limit exceeded")? {
        let h = &s.handlers[hi];
        let p = bindings(s, h, &args);
        let guards = h
            .cases
            .iter()
            .map(|c| {
                c.guard
                    .as_ref()
                    .map(|g| expression(s, g, &p, Some(&Ty::Bool)).map(Lowered::scalar))
                    .transpose()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let otherwise = or(guards.iter().flatten().cloned()).negate();
        for (ci, c) in h.cases.iter().enumerate() {
            let mut updates = vec![];
            if matches!(c.outcome, Outcome::Accept) {
                for stmt in &c.stmts {
                    if let StmtKind::Assign(n, op, e) = &stmt.kind {
                        let i = s
                            .fields
                            .iter()
                            .position(|f| f.name.text == n.text)
                            .expect("checked field");
                        let f = &s.fields[i];
                        let start = offset(s, i);
                        let value = if op == "=" {
                            expression(s, e, &p, Some(&f.ty.kind))?
                        } else {
                            let member = expression(s, e, &p, None)?.scalar();
                            Lowered::Set(
                                (start..start + width(s, &f.ty.kind))
                                    .enumerate()
                                    .map(|(j, k)| {
                                        let bit = E::Var(k).eq(E::Int(1));
                                        let matches = member.clone().eq(E::Int(j as i64));
                                        if op == "+=" {
                                            bit.or(matches)
                                        } else {
                                            bit.and(matches.negate())
                                        }
                                    })
                                    .collect(),
                            )
                        };
                        match value {
                            Lowered::Set(bits) => {
                                for (j, b) in bits.into_iter().enumerate() {
                                    updates.push((start + j, b.ite(E::Int(1), E::Int(0))))
                                }
                            }
                            Lowered::Scalar(e) => updates.push((
                                start,
                                if f.ty.kind == Ty::Bool {
                                    e.ite(E::Int(1), E::Int(0))
                                } else {
                                    e
                                },
                            )),
                        }
                    }
                }
            }
            let name = format!("{}({args:?})/{}#{hi}:{ci}", h.event.text, c.name.text);
            if c.fair.is_some() {
                groups.entry((hi, ci)).or_default().push(m.actions.len());
            }
            m.actions.push(Action::new(
                &name,
                guards[ci].clone().unwrap_or_else(|| otherwise.clone()),
                updates,
            ));
        }
    }
    for inv in &s.invariants {
        m.invariants.push(Invariant {
            name: inv.name.text.clone(),
            condition: expression(s, &inv.expr, &vec![], Some(&Ty::Bool))?.scalar(),
        });
    }
    for ((hi, ci), group) in groups {
        match s.handlers[hi].cases[ci].fair {
            Some((Fairness::Strong, _)) => m.strong_fair.push(group),
            _ => m.weak_fair.push(group),
        }
    }
    for p in &s.properties {
        for (params, label) in property_instances(s, p) {
            m.responses.push(Response {
                name: format!("{}{label}", p.name.text),
                request: expression(s, &p.request, &params, Some(&Ty::Bool))?.scalar(),
                response: expression(s, &p.response, &params, Some(&Ty::Bool))?.scalar(),
            });
        }
    }
    Ok(m)
}
/// Every valuation of a property's binders, with its `(Member, ...)` label.
pub(crate) fn property_instances(s: &Spec, p: &Property) -> Vec<(Bindings, String)> {
    let mut all: Vec<(Bindings, Vec<String>)> = vec![(vec![], vec![])];
    for (binding, d) in &p.binders {
        let members = &domain(s, &d.text).members;
        all = all
            .into_iter()
            .flat_map(|(params, labels)| {
                members.iter().enumerate().map(move |(n, member)| {
                    let (mut params, mut labels) = (params.clone(), labels.clone());
                    params.push((
                        binding.text.clone(),
                        Ty::Named(d.text.clone()),
                        Value::Scalar(n as i64),
                    ));
                    labels.push(member.text.clone());
                    (params, labels)
                })
            })
            .collect();
    }
    all.into_iter()
        .map(|(params, labels)| {
            let label = if labels.is_empty() {
                String::new()
            } else {
                format!("({})", labels.join(", "))
            };
            (params, label)
        })
        .collect()
}
/// The Den state a model state vector encodes.
pub(crate) fn unflatten(s: &Spec, flat: &[i64]) -> State {
    let mut at = 0;
    s.fields
        .iter()
        .map(|f| match &f.ty.kind {
            Ty::Set(d) => {
                let n = domain(s, d).members.len();
                at += n;
                Value::Set(flat[at - n..at].iter().map(|b| *b != 0).collect())
            }
            _ => {
                at += 1;
                Value::Scalar(flat[at - 1])
            }
        })
        .collect()
}
#[cfg(test)]
pub(crate) fn flatten(s: &Spec, state: &State) -> Vec<i64> {
    let mut result = vec![];
    for value in state {
        match value {
            Value::Scalar(n) => result.push(*n),
            Value::Set(bits) => result.extend(bits.iter().map(|b| i64::from(*b))),
        }
    }
    if s.fields.is_empty() {
        result.push(0)
    }
    result
}
