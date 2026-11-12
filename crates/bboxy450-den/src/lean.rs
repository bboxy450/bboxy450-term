// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Den expressions as Lean terms, for `bboxy450_tlc::lean::Machine`.
//!
//! A Bool is a proposition, a set is a predicate on its domain, an Int is an
//! `Int`. An enum whose members neither the transitions nor a theorem name is
//! an abstract type, so a `theorem` is stated for a domain of any size; one
//! whose members are named keeps exactly its declared members. Effects, scenarios and progress
//! properties are not exported.
use crate::{ast::*, types::expr_type};
use bboxy450_tlc::lean::{
    Claim as LeanClaim, Domain, Field as LeanField, Machine, STATE, Step, ident,
};

struct Printer<'a> {
    s: &'a Spec,
    locals: Vec<(String, Ty)>,
    fresh: usize,
}
impl Printer<'_> {
    fn operand_ty(&self, a: &Expr, b: &Expr) -> Ty {
        expr_type(self.s, a, &self.locals, None)
            .or_else(|_| expr_type(self.s, b, &self.locals, None))
            .expect("checked operands")
    }
    fn element(&mut self) -> String {
        self.fresh += 1;
        format!("ξ{}", self.fresh - 1)
    }
    fn name(&self, n: &str) -> String {
        if self.locals.iter().any(|(local, _)| local == n) {
            ident(n)
        } else {
            format!("{STATE}.{}", ident(n))
        }
    }
    /// A Bool expression as a proposition.
    fn prop(&mut self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Bool(true) => "True".into(),
            ExprKind::Bool(false) => "False".into(),
            ExprKind::Name(n) => self.name(n),
            ExprKind::Call(n, args) => {
                let set = Expr {
                    kind: ExprKind::Name(n.clone()),
                    pos: e.pos,
                };
                let element = self.value(&args[0]);
                self.member(&set, &element)
            }
            ExprKind::Not(a) => format!("¬ {}", self.prop(a)),
            ExprKind::Binary(op, a, b) => match op.as_str() {
                "&&" => format!("({} ∧ {})", self.prop(a), self.prop(b)),
                "||" => format!("({} ∨ {})", self.prop(a), self.prop(b)),
                "==>" => format!("({} → {})", self.prop(a), self.prop(b)),
                "==" | "!=" => {
                    let equal = match self.operand_ty(a, b) {
                        Ty::Bool => format!("({} ↔ {})", self.prop(a), self.prop(b)),
                        Ty::Set(d) => {
                            let x = self.element();
                            format!(
                                "(∀ {x} : {}, {} ↔ {})",
                                ident(&d),
                                self.member(a, &x),
                                self.member(b, &x)
                            )
                        }
                        _ => format!("({} = {})", self.value(a), self.value(b)),
                    };
                    if op == "==" {
                        equal
                    } else {
                        format!("¬ {equal}")
                    }
                }
                _ => {
                    let op = match op.as_str() {
                        "<=" => "≤",
                        ">=" => "≥",
                        other => other,
                    };
                    format!("({} {op} {})", self.value(a), self.value(b))
                }
            },
            ExprKind::If(c, a, b) => {
                let c = self.prop(c);
                format!("(({c} ∧ {}) ∨ (¬ {c} ∧ {}))", self.prop(a), self.prop(b))
            }
            ExprKind::Quant {
                all,
                binding,
                domain,
                body,
            } => {
                self.locals
                    .push((binding.text.clone(), Ty::Named(domain.text.clone())));
                let body = self.prop(body);
                self.locals.pop();
                format!(
                    "({} {} : {}, {body})",
                    if *all { "∀" } else { "∃" },
                    ident(&binding.text),
                    ident(&domain.text)
                )
            }
            ExprKind::Int(_) | ExprKind::Member(..) => unreachable!("checked Bool"),
        }
    }
    /// An Int or enum expression as a term of that type.
    fn value(&mut self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Int(n) => format!("({n} : Int)"),
            ExprKind::Name(n) => self.name(n),
            ExprKind::Member(d, m) => format!("{}.{}", ident(d), ident(m)),
            ExprKind::Binary(op, a, b) => {
                format!("({} {op} {})", self.value(a), self.value(b))
            }
            ExprKind::If(c, a, b) => format!(
                "(@ite _ {} (Classical.propDecidable _) {} {})",
                self.prop(c),
                self.value(a),
                self.value(b)
            ),
            _ => unreachable!("checked value"),
        }
    }
    /// `element ∈ e` for a set expression.
    fn member(&mut self, e: &Expr, element: &str) -> String {
        match &e.kind {
            ExprKind::Name(n) => format!("{} {element}", self.name(n)),
            ExprKind::Call(_, args) if args.is_empty() => "False".into(),
            ExprKind::Call(_, args) => {
                let members: Vec<_> = args
                    .iter()
                    .map(|a| format!("{element} = {}", self.value(a)))
                    .collect();
                format!("({})", members.join(" ∨ "))
            }
            ExprKind::If(c, a, b) => {
                let c = self.prop(c);
                format!(
                    "(({c} ∧ {}) ∨ (¬ {c} ∧ {}))",
                    self.member(a, element),
                    self.member(b, element)
                )
            }
            _ => unreachable!("checked set"),
        }
    }
    /// An expression of a field's or parameter's type.
    fn typed(&mut self, e: &Expr, ty: &Ty) -> String {
        match ty {
            Ty::Bool => self.prop(e),
            Ty::Set(_) => {
                if matches!(&e.kind, ExprKind::Call(_, args) if args.is_empty()) {
                    return "fun _ => False".into();
                }
                let x = self.element();
                format!("fun {x} => {}", self.member(e, &x))
            }
            _ => self.value(e),
        }
    }
}
fn lean_type(ty: &Ty) -> String {
    match ty {
        Ty::Bool => "Prop".into(),
        Ty::Int(..) => "Int".into(),
        Ty::Named(d) => ident(d),
        Ty::Set(d) => format!("{} → Prop", ident(d)),
    }
}
fn range(term: &str, ty: &Ty) -> Option<String> {
    match ty {
        Ty::Int(lo, hi) => Some(format!("(({lo} : Int) ≤ {term} ∧ {term} ≤ ({hi} : Int))")),
        _ => None,
    }
}
fn names_member(e: &Expr, named: &mut Vec<String>) {
    match &e.kind {
        ExprKind::Member(d, _) => named.push(d.clone()),
        ExprKind::Call(_, args) => args.iter().for_each(|a| names_member(a, named)),
        ExprKind::Not(a) => names_member(a, named),
        ExprKind::Binary(_, a, b) => [a, b].into_iter().for_each(|e| names_member(e, named)),
        ExprKind::If(c, a, b) => [c, a, b].into_iter().for_each(|e| names_member(e, named)),
        ExprKind::Quant { body, .. } => names_member(body, named),
        ExprKind::Bool(_) | ExprKind::Int(_) | ExprKind::Name(_) => {}
    }
}
/// The machine of a well-formed spec. Integer ranges become the `TYPE_OK` theorem.
pub(crate) fn machine(s: &Spec) -> Machine {
    let mut named = vec![];
    for f in &s.fields {
        names_member(&f.initial, &mut named);
    }
    for c in s.handlers.iter().flat_map(|h| &h.cases) {
        c.guard.iter().for_each(|g| names_member(g, &mut named));
        for stmt in &c.stmts {
            if let StmtKind::Assign(_, _, e) = &stmt.kind {
                names_member(e, &mut named);
            }
        }
    }
    // An invariant that is not a theorem does not fix a domain: if it names a
    // member of an otherwise abstract one, it stays a bounded claim only.
    for i in s.invariants.iter().filter(|i| i.theorem) {
        names_member(&i.expr, &mut named);
    }
    let mut p = Printer {
        s,
        locals: vec![],
        fresh: 0,
    };
    let fields = s
        .fields
        .iter()
        .map(|f| LeanField {
            name: f.name.text.clone(),
            ty: lean_type(&f.ty.kind),
            initial: p.typed(&f.initial, &f.ty.kind),
        })
        .collect();
    let mut steps = vec![];
    for h in &s.handlers {
        let event = s.events.iter().find(|e| e.name.text == h.event.text);
        let event = event.expect("checked event");
        p.locals = h
            .bindings
            .iter()
            .zip(&event.params)
            .map(|(b, param)| (b.text.clone(), param.ty.kind.clone()))
            .collect();
        let explicit: Vec<_> = h
            .cases
            .iter()
            .map(|c| c.guard.as_ref().map(|g| p.prop(g)))
            .collect();
        let others: Vec<_> = explicit.iter().flatten().cloned().collect();
        for (c, guard) in h.cases.iter().zip(&explicit) {
            if !matches!(c.outcome, Outcome::Accept) {
                continue;
            }
            let mut guard = vec![match guard {
                Some(g) => g.clone(),
                None if others.is_empty() => "True".into(),
                None => format!("¬ ({})", others.join(" ∨ ")),
            }];
            guard.extend(p.locals.iter().filter_map(|(n, ty)| range(&ident(n), ty)));
            let mut updates = vec![];
            for stmt in &c.stmts {
                let StmtKind::Assign(n, op, e) = &stmt.kind else {
                    continue;
                };
                let field = s.fields.iter().find(|f| f.name.text == n.text);
                let ty = &field.expect("checked field").ty.kind;
                let value = match (op.as_str(), ty) {
                    ("=", ty) => p.typed(e, ty),
                    (op, Ty::Set(_)) => {
                        let x = p.element();
                        let member = p.value(e);
                        let held = format!("{STATE}.{} {x}", ident(&n.text));
                        if op == "+=" {
                            format!("fun {x} => {x} = {member} ∨ {held}")
                        } else {
                            format!("fun {x} => {x} ≠ {member} ∧ {held}")
                        }
                    }
                    _ => unreachable!("checked set update"),
                };
                updates.push((n.text.clone(), value));
            }
            steps.push(Step {
                name: format!("{}_{}", h.event.text, c.name.text),
                params: p
                    .locals
                    .iter()
                    .map(|(n, ty)| (n.clone(), lean_type(ty)))
                    .collect(),
                guard: guard.join(" ∧ "),
                updates,
            });
        }
    }
    p.locals.clear();
    let mut claims: Vec<_> = s
        .invariants
        .iter()
        .filter(|i| {
            let mut own = vec![];
            names_member(&i.expr, &mut own);
            own.iter().all(|d| named.contains(d))
        })
        .map(|i| LeanClaim {
            id: i.name.text.clone(),
            text: i.text.clone(),
            prop: p.prop(&i.expr),
            theorem: i.theorem,
        })
        .collect();
    let ranges: Vec<_> = s
        .fields
        .iter()
        .filter_map(|f| range(&format!("{STATE}.{}", ident(&f.name.text)), &f.ty.kind))
        .collect();
    // The Lean fields are unbounded integers: the machine is Den's only while
    // every field stays in its declared range, so that is owed too.
    if !ranges.is_empty() && claims.iter().any(|c| c.theorem) {
        claims.push(LeanClaim {
            id: "TYPE_OK".into(),
            text: "every integer field stays in its declared range".into(),
            prop: ranges.join(" ∧ "),
            theorem: true,
        });
    }
    Machine {
        module: format!("Den{}", s.name.text),
        origin: format!(
            "Den spec {}",
            s.module
                .iter()
                .map(|n| n.text.as_str())
                .collect::<Vec<_>>()
                .join(".")
        ),
        domains: s
            .enums
            .iter()
            .map(|e| Domain {
                name: e.name.text.clone(),
                members: named
                    .contains(&e.name.text)
                    .then(|| e.members.iter().map(|m| m.text.clone()).collect()),
            })
            .collect(),
        fields,
        steps,
        claims,
    }
}
