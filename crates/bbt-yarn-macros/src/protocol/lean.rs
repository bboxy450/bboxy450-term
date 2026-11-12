// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Protocol expressions as Lean terms, for `bboxy450_tlc::lean::Machine`.
//!
//! The same typed AST that generates the Rust reducer and the TLC model prints
//! the Lean machine: a bool is a proposition, a set is a predicate on its
//! domain, an option is an `Option`. A domain is an abstract type unless the
//! transitions or a theorem name one of its members. Effects, fairness and
//! response properties are not exported.
use super::*;
use bboxy450_tlc::lean::{STATE, ident};

/// Domains whose members an expression names.
fn named(e: &Expression, out: &mut BTreeSet<usize>) {
    match &e.kind {
        Kind::Value(v) => value_names(&e.ty, v, out),
        Kind::Field(_) | Kind::Parameter(_) => {}
        Kind::Some(a) | Kind::IsNone(a) | Kind::Not(a) => named(a, out),
        Kind::Binary(_, a, b) | Kind::SetOp(_, a, b) => {
            named(a, out);
            named(b, out);
        }
    }
}
fn value_names(ty: &Ty, v: &Value, out: &mut BTreeSet<usize>) {
    match (ty, v) {
        (Ty::Domain(d), _) => {
            out.insert(*d);
        }
        (Ty::Option(d), Value::Scalar(n)) if *n >= 0 => {
            out.insert(*d);
        }
        (Ty::Set(d), Value::Set(bits)) if bits.contains(&true) => {
            out.insert(*d);
        }
        _ => {}
    }
}

struct Printer<'a> {
    c: &'a Checked,
    parameters: &'a [(Ident, Ty)],
    fresh: usize,
}
impl Printer<'_> {
    fn domain(&self, d: usize) -> String {
        ident(&self.c.declaration.domains[d].name.to_string())
    }
    fn member(&self, d: usize, n: i64) -> String {
        let variant = &self.c.declaration.domains[d].variants[n as usize];
        format!("{}.{}", self.domain(d), ident(&variant.to_string()))
    }
    fn element(&mut self) -> String {
        self.fresh += 1;
        format!("ξ{}", self.fresh - 1)
    }
    fn place(&self, e: &Expression) -> Option<String> {
        match &e.kind {
            Kind::Field(i) => Some(format!(
                "{STATE}.{}",
                ident(&self.c.declaration.fields[*i].name.to_string())
            )),
            Kind::Parameter(i) => Some(ident(&self.parameters[*i].0.to_string())),
            _ => None,
        }
    }
    /// A bool expression as a proposition.
    fn prop(&mut self, e: &Expression) -> String {
        if let Some(place) = self.place(e) {
            return place;
        }
        match &e.kind {
            Kind::Value(Value::Scalar(0)) => "False".into(),
            Kind::Value(_) => "True".into(),
            Kind::IsNone(a) => format!("({} = none)", self.value(a)),
            Kind::Not(a) => format!("¬ {}", self.prop(a)),
            Kind::Binary(Op::And, a, b) => format!("({} ∧ {})", self.prop(a), self.prop(b)),
            Kind::Binary(Op::Or, a, b) => format!("({} ∨ {})", self.prop(a), self.prop(b)),
            Kind::Binary(op, a, b) => {
                let equal = match a.ty {
                    Ty::Bool => format!("({} ↔ {})", self.prop(a), self.prop(b)),
                    Ty::Set(d) => {
                        let x = self.element();
                        format!(
                            "(∀ {x} : {}, {} ↔ {})",
                            self.domain(d),
                            self.contains(a, &x),
                            self.contains(b, &x)
                        )
                    }
                    _ => format!("({} = {})", self.value(a), self.value(b)),
                };
                if matches!(op, Op::Eq) {
                    equal
                } else {
                    format!("¬ {equal}")
                }
            }
            Kind::SetOp(_, set, element) => {
                let element = self.value(element);
                self.contains(set, &element)
            }
            _ => unreachable!("typed bool"),
        }
    }
    /// A domain or option expression as a term of that type.
    fn value(&mut self, e: &Expression) -> String {
        if let Some(place) = self.place(e) {
            return place;
        }
        match (&e.kind, &e.ty) {
            (Kind::Value(Value::Scalar(n)), Ty::Domain(d)) => self.member(*d, *n),
            (Kind::Value(Value::Scalar(n)), Ty::Option(d)) if *n < 0 => {
                format!("(none : Option {})", self.domain(*d))
            }
            (Kind::Value(Value::Scalar(n)), Ty::Option(d)) => {
                format!("(some {})", self.member(*d, *n))
            }
            (Kind::Some(a), _) => format!("(some {})", self.value(a)),
            _ => unreachable!("typed value"),
        }
    }
    /// `element ∈ e` for a set expression.
    fn contains(&mut self, e: &Expression, element: &str) -> String {
        if let Some(place) = self.place(e) {
            return format!("{place} {element}");
        }
        match (&e.kind, &e.ty) {
            (Kind::Value(Value::Set(bits)), Ty::Set(d)) => {
                let members: Vec<_> = bits
                    .iter()
                    .enumerate()
                    .filter(|(_, held)| **held)
                    .map(|(n, _)| format!("{element} = {}", self.member(*d, n as i64)))
                    .collect();
                if members.is_empty() {
                    "False".into()
                } else {
                    format!("({})", members.join(" ∨ "))
                }
            }
            (Kind::SetOp(SetOp::With, set, added), _) => {
                format!(
                    "({element} = {} ∨ {})",
                    self.value(added),
                    self.contains(set, element)
                )
            }
            (Kind::SetOp(SetOp::Without, set, removed), _) => {
                format!(
                    "({element} ≠ {} ∧ {})",
                    self.value(removed),
                    self.contains(set, element)
                )
            }
            _ => unreachable!("typed set"),
        }
    }
    fn typed(&mut self, e: &Expression) -> String {
        match e.ty {
            Ty::Bool => self.prop(e),
            Ty::Set(_) => {
                if matches!(&e.kind, Kind::Value(Value::Set(bits)) if !bits.contains(&true)) {
                    return "fun _ => False".into();
                }
                let x = self.element();
                format!("fun {x} => {}", self.contains(e, &x))
            }
            _ => self.value(e),
        }
    }
    fn lean_type(&self, ty: &Ty) -> String {
        match ty {
            Ty::Bool => "Prop".into(),
            Ty::Domain(d) => self.domain(*d),
            Ty::Option(d) => format!("Option {}", self.domain(*d)),
            Ty::Set(d) => format!("{} → Prop", self.domain(*d)),
        }
    }
}

impl Checked {
    /// `pub fn lean()`: the machine, with every expression already printed.
    pub(super) fn generate_lean(&self) -> TokenStream {
        let path = crate::kittens_path();
        let lean = quote!(#path::model::lean);
        let list = quote!(#path::protocol::Effects::from);
        let d = &self.declaration;
        // Members named by the transitions or by a theorem fix their domain.
        let mut concrete = BTreeSet::new();
        for (ty, v) in self.fields.iter().zip(&self.initial) {
            value_names(ty, v, &mut concrete);
        }
        for case in self.arms.iter().flat_map(|a| &a.cases) {
            named(&case.guard, &mut concrete);
            for e in case.assignments.iter().flatten() {
                named(e, &mut concrete);
            }
        }
        for (i, e) in d.invariants.iter().zip(&self.invariants) {
            if i.theorem {
                named(e, &mut concrete);
            }
        }
        let domains = d.domains.iter().enumerate().map(|(i, domain)| {
            let name = domain.name.to_string();
            let members = if concrete.contains(&i) {
                let variants = domain.variants.iter().map(|v| v.to_string());
                quote!(::core::option::Option::Some(#list([#(#variants.into()),*])))
            } else {
                quote!(::core::option::Option::None)
            };
            quote!(#lean::Domain { name: #name.into(), members: #members })
        });
        let mut p = Printer {
            c: self,
            parameters: &[],
            fresh: 0,
        };
        let fields: Vec<_> = (0..self.fields.len())
            .map(|i| {
                let name = d.fields[i].name.to_string();
                let ty = p.lean_type(&self.fields[i]);
                let initial = p.typed(&Expression {
                    ty: self.fields[i].clone(),
                    kind: Kind::Value(self.initial[i].clone()),
                });
                quote!(#lean::Field { name: #name.into(), ty: #ty.into(), initial: #initial.into() })
            })
            .collect();
        let mut steps = Vec::new();
        for arm in &self.arms {
            p.parameters = &arm.parameters;
            for case in arm.cases.iter().filter(|c| c.end.is_none()) {
                let name = format!("{}_{}", d.events[arm.event].name, case.name);
                let params: Vec<_> = arm
                    .parameters
                    .iter()
                    .map(|(n, ty)| {
                        let (n, ty) = (n.to_string(), p.lean_type(ty));
                        quote!((#n.into(), #ty.into()))
                    })
                    .collect();
                let guard = p.prop(&case.guard);
                let mut updates = Vec::new();
                for (i, e) in case.assignments.iter().enumerate() {
                    if let Some(e) = e {
                        let (field, value) = (d.fields[i].name.to_string(), p.typed(e));
                        updates.push(quote!((#field.into(), #value.into())));
                    }
                }
                steps.push(quote!(#lean::Step {
                    name: #name.into(),
                    params: #list([#(#params),*]),
                    guard: #guard.into(),
                    updates: #list([#(#updates),*]),
                }));
            }
        }
        p.parameters = &[];
        let mut claims = Vec::new();
        for (i, e) in d.invariants.iter().zip(&self.invariants) {
            let mut names = BTreeSet::new();
            named(e, &mut names);
            // A claim that names a member of an abstract domain cannot be stated
            // for every size; it stays a bounded invariant only.
            if !names.is_subset(&concrete) {
                continue;
            }
            let (id, text, prop, theorem) =
                (i.name.to_string(), i.id.value(), p.prop(e), i.theorem);
            claims.push(quote!(#lean::Claim {
                id: #id.into(), text: #text.into(), prop: #prop.into(), theorem: #theorem,
            }));
        }
        let module = format!("Yarn{}", d.name);
        let origin = format!("the Rust protocol! declaration {}", d.name);
        quote! {
            /// The machine as Lean 4 declarations: each `#[invariant(.., theorem)]`
            /// is a statement about every reachable state, proved outside Rust
            /// and checked by `make test-lean`.
            pub fn lean() -> #lean::Machine {
                #lean::Machine {
                    module: #module.into(),
                    origin: #origin.into(),
                    domains: #list([#(#domains),*]),
                    fields: #list([#(#fields),*]),
                    steps: #list([#(#steps),*]),
                    claims: #list([#(#claims),*]),
                }
            }
        }
    }
}
