// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::{Diagnostic, ast::*, types::expr_type};

pub(crate) const VALUATIONS: usize = 65_536;
pub(crate) const STATES: usize = 100_000;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Value {
    Scalar(i64),
    Set(Vec<bool>),
}
impl Value {
    pub fn scalar(&self) -> i64 {
        match self {
            Self::Scalar(n) => *n,
            _ => unreachable!("checked scalar"),
        }
    }
    pub fn truth(&self) -> bool {
        self.scalar() != 0
    }
}
pub(crate) type State = Vec<Value>;
pub(crate) type Bindings = Vec<(String, Ty, Value)>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Effect {
    pub name: String,
    pub args: Vec<Value>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Accepted(State, Vec<Effect>),
    Rejected(String),
    Ignored(String),
}

pub(crate) fn domain<'a>(s: &'a Spec, name: &str) -> &'a Enum {
    s.enums
        .iter()
        .find(|e| e.name.text == name)
        .expect("checked domain")
}
pub(crate) fn cardinality(s: &Spec, ty: &Ty) -> usize {
    match ty {
        Ty::Bool => 2,
        Ty::Int(lo, hi) => hi
            .checked_sub(*lo)
            .and_then(|n| n.checked_add(1))
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(usize::MAX),
        Ty::Named(n) => domain(s, n).members.len(),
        Ty::Set(n) => 1usize
            .checked_shl(domain(s, n).members.len().try_into().unwrap_or(u32::MAX))
            .unwrap_or(usize::MAX),
    }
}
pub(crate) fn values(s: &Spec, ty: &Ty) -> Vec<Value> {
    let count = cardinality(s, ty);
    assert!(
        count <= VALUATIONS,
        "enumeration is bounded before allocation"
    );
    (0..count)
        .map(|n| match ty {
            Ty::Int(lo, _) => Value::Scalar(*lo + n as i64),
            Ty::Set(name) => Value::Set(
                (0..domain(s, name).members.len())
                    .map(|bit| n & (1usize << bit) != 0)
                    .collect(),
            ),
            _ => Value::Scalar(n as i64),
        })
        .collect()
}
pub(crate) fn product(s: &Spec, types: &[Ty], limit: usize) -> Option<Vec<Vec<Value>>> {
    let count = types
        .iter()
        .try_fold(1usize, |n, t| n.checked_mul(cardinality(s, t)))?;
    if count > limit || types.iter().any(|t| cardinality(s, t) > VALUATIONS) {
        return None;
    }
    let mut result = vec![vec![]];
    for ty in types {
        let vals = values(s, ty);
        result = result
            .into_iter()
            .flat_map(|prefix| {
                vals.iter().map(move |v| {
                    let mut p = prefix.clone();
                    p.push(v.clone());
                    p
                })
            })
            .collect();
    }
    Some(result)
}
pub(crate) fn binding_types(bindings: &Bindings) -> Vec<(String, Ty)> {
    bindings
        .iter()
        .map(|(n, t, _)| (n.clone(), t.clone()))
        .collect()
}
pub(crate) fn bindings(s: &Spec, h: &Handler, args: &[Value]) -> Bindings {
    let event = s
        .events
        .iter()
        .find(|e| e.name.text == h.event.text)
        .expect("checked event");
    h.bindings
        .iter()
        .zip(&event.params)
        .zip(args)
        .map(|((n, p), v)| (n.text.clone(), p.ty.kind.clone(), v.clone()))
        .collect()
}
pub(crate) fn eval(
    s: &Spec,
    e: &Expr,
    state: &State,
    params: &Bindings,
    hint: Option<&Ty>,
) -> Result<Value, String> {
    let recurse = |x: &Expr, h: Option<&Ty>| eval(s, x, state, params, h);
    let boolean = |b: bool| Value::Scalar(i64::from(b));
    Ok(match &e.kind {
        ExprKind::Bool(b) => boolean(*b),
        ExprKind::Int(n) => Value::Scalar(*n),
        ExprKind::Name(n) => params
            .iter()
            .rev()
            .find(|(name, _, _)| name == n)
            .map(|(_, _, v)| v.clone())
            .unwrap_or_else(|| {
                state[s
                    .fields
                    .iter()
                    .position(|f| f.name.text == *n)
                    .expect("checked field")]
                .clone()
            }),
        ExprKind::Member(d, m) => Value::Scalar(
            domain(s, d)
                .members
                .iter()
                .position(|n| n.text == *m)
                .expect("checked member") as i64,
        ),
        ExprKind::Call(n, args) if n == "Set" => {
            let ty = expr_type(s, e, &binding_types(params), hint).map_err(|d| d.message)?;
            let Ty::Set(d) = ty else {
                unreachable!("checked set")
            };
            let mut bits = vec![false; domain(s, &d).members.len()];
            for a in args {
                bits[recurse(a, Some(&Ty::Named(d.clone())))?.scalar() as usize] = true;
            }
            Value::Set(bits)
        }
        ExprKind::Call(n, args) => {
            let field = Expr {
                kind: ExprKind::Name(n.clone()),
                pos: e.pos,
            };
            let Value::Set(bits) = recurse(&field, None)? else {
                unreachable!("checked membership")
            };
            boolean(bits[recurse(&args[0], None)?.scalar() as usize])
        }
        ExprKind::Not(a) => boolean(!recurse(a, Some(&Ty::Bool))?.truth()),
        ExprKind::Binary(op, a, b) => {
            if op == "&&" && !recurse(a, Some(&Ty::Bool))?.truth() {
                return Ok(boolean(false));
            }
            if op == "||" && recurse(a, Some(&Ty::Bool))?.truth() {
                return Ok(boolean(true));
            }
            if op == "==>" && !recurse(a, Some(&Ty::Bool))?.truth() {
                return Ok(boolean(true));
            }
            let ty = expr_type(s, a, &binding_types(params), None)
                .or_else(|_| expr_type(s, b, &binding_types(params), None))
                .map_err(|d| d.message)?;
            let (a, b) = (recurse(a, Some(&ty))?, recurse(b, Some(&ty))?);
            match op.as_str() {
                "==" => boolean(a == b),
                "!=" => boolean(a != b),
                "<" => boolean(a.scalar() < b.scalar()),
                "<=" => boolean(a.scalar() <= b.scalar()),
                ">" => boolean(a.scalar() > b.scalar()),
                ">=" => boolean(a.scalar() >= b.scalar()),
                "+" => Value::Scalar(
                    a.scalar()
                        .checked_add(b.scalar())
                        .ok_or("integer addition overflow")?,
                ),
                "-" => Value::Scalar(
                    a.scalar()
                        .checked_sub(b.scalar())
                        .ok_or("integer subtraction overflow")?,
                ),
                "&&" => boolean(a.truth() && b.truth()),
                "||" => boolean(a.truth() || b.truth()),
                "==>" => boolean(!a.truth() || b.truth()),
                _ => unreachable!("checked operator"),
            }
        }
        ExprKind::If(c, a, b) => recurse(
            if recurse(c, Some(&Ty::Bool))?.truth() {
                a
            } else {
                b
            },
            hint,
        )?,
        ExprKind::Quant {
            all,
            binding,
            domain: d,
            body,
        } => {
            let mut bound = params.clone();
            let mut answer = *all;
            for n in 0..domain(s, &d.text).members.len() {
                bound.push((
                    binding.text.clone(),
                    Ty::Named(d.text.clone()),
                    Value::Scalar(n as i64),
                ));
                let v = eval(s, body, state, &bound, Some(&Ty::Bool))?.truth();
                bound.pop();
                if v != *all {
                    answer = v;
                    break;
                }
            }
            boolean(answer)
        }
    })
}
pub(crate) fn initial(s: &Spec) -> Result<State, String> {
    s.fields
        .iter()
        .map(|f| eval(s, &f.initial, &vec![], &vec![], Some(&f.ty.kind)))
        .collect()
}
pub(crate) fn selected(
    s: &Spec,
    h: &Handler,
    state: &State,
    params: &Bindings,
) -> Result<Vec<usize>, String> {
    let mut indices = Vec::new();
    for (i, c) in h.cases.iter().enumerate() {
        if let Some(g) = &c.guard
            && eval(s, g, state, params, Some(&Ty::Bool))?.truth()
        {
            indices.push(i)
        }
    }
    if indices.is_empty() {
        indices.extend(
            h.cases
                .iter()
                .enumerate()
                .filter(|(_, c)| c.guard.is_none())
                .map(|(i, _)| i),
        );
    }
    Ok(indices)
}
pub(crate) fn call_effect(
    s: &Spec,
    c: &Call,
    state: &State,
    params: &Bindings,
) -> Result<Effect, String> {
    let sig = s
        .effects
        .iter()
        .find(|e| e.name.text == c.name.text)
        .expect("checked effect");
    let args = c
        .args
        .iter()
        .zip(&sig.params)
        .map(|(e, p)| eval(s, e, state, params, Some(&p.ty.kind)))
        .collect::<Result<Vec<_>, _>>()?;
    if args
        .iter()
        .zip(&sig.params)
        .any(|(v, p)| !in_range(s, v, &p.ty.kind))
    {
        return Err("effect argument is outside its declared type".into());
    }
    Ok(Effect {
        name: c.name.text.clone(),
        args,
    })
}
pub(crate) fn apply(
    s: &Spec,
    c: &Case,
    state: &State,
    params: &Bindings,
) -> Result<Decision, String> {
    match &c.outcome {
        Outcome::Reject(r) => return Ok(Decision::Rejected(r.text.clone())),
        Outcome::Ignore(r) => return Ok(Decision::Ignored(r.text.clone())),
        Outcome::Accept => {}
    }
    let mut next = state.clone();
    let mut effects = vec![];
    for stmt in &c.stmts {
        match &stmt.kind {
            StmtKind::Emit(call) => effects.push(call_effect(s, call, state, params)?),
            StmtKind::Assign(n, op, e) => {
                let i = s
                    .fields
                    .iter()
                    .position(|f| f.name.text == n.text)
                    .expect("checked assignment");
                next[i] = if op == "=" {
                    eval(s, e, state, params, Some(&s.fields[i].ty.kind))?
                } else {
                    let Value::Set(mut bits) = state[i].clone() else {
                        unreachable!("checked set update")
                    };
                    let m = eval(s, e, state, params, None)?.scalar() as usize;
                    bits[m] = op == "+=";
                    Value::Set(bits)
                };
            }
        }
    }
    Ok(Decision::Accepted(next, effects))
}
pub(crate) fn step(
    s: &Spec,
    h: &Handler,
    state: &State,
    args: &[Value],
) -> Result<(usize, Decision), String> {
    let p = bindings(s, h, args);
    let selected = selected(s, h, state, &p)?;
    if selected.len() != 1 {
        return Err("event does not select exactly one case".into());
    }
    let i = selected[0];
    Ok((i, apply(s, &h.cases[i], state, &p)?))
}
pub(crate) fn instances(s: &Spec) -> Option<Vec<(usize, Vec<Value>)>> {
    let mut result = vec![];
    for (i, h) in s.handlers.iter().enumerate() {
        let ev = s
            .events
            .iter()
            .find(|e| e.name.text == h.event.text)
            .expect("checked event");
        for args in product(
            s,
            &ev.params
                .iter()
                .map(|p| p.ty.kind.clone())
                .collect::<Vec<_>>(),
            VALUATIONS,
        )? {
            result.push((i, args));
        }
    }
    Some(result)
}
pub(crate) fn in_range(s: &Spec, v: &Value, ty: &Ty) -> bool {
    match (v, ty) {
        (Value::Scalar(n), Ty::Bool) => (0..=1).contains(n),
        (Value::Scalar(n), Ty::Int(lo, hi)) => (*lo..=*hi).contains(n),
        (Value::Scalar(n), Ty::Named(d)) => *n >= 0 && (*n as usize) < domain(s, d).members.len(),
        (Value::Set(bits), Ty::Set(d)) => bits.len() == domain(s, d).members.len(),
        _ => false,
    }
}
pub(crate) fn describe(s: &Spec, state: &State, params: &Bindings) -> String {
    let entries = s
        .fields
        .iter()
        .zip(state)
        .map(|(f, v)| format!("{}={v:?}", f.name.text))
        .chain(params.iter().map(|(n, _, v)| format!("{n}={v:?}")))
        .collect::<Vec<_>>();
    format!("{{{}}}", entries.join(", "))
}
pub(crate) fn diagnostic(
    s: &Spec,
    pos: Pos,
    code: &'static str,
    message: impl Into<String>,
    repair: &str,
) -> Diagnostic {
    pos.diagnostic(code, &s.name.text, message, repair)
}
