// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Inspired by Verus state_machine!: https://verus-lang.github.io/verus/state_machines/
// (explicit pre-state transitions), and machine-check: https://github.com/onderjan/machine-check
// (a constrained Rust-facing language). Independently implemented; no source copied.
//! Closed finite language: one typed AST feeds Rust and symbolic TLC expressions.

use std::collections::{BTreeSet, HashSet};

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Error, Ident, Result, Token, braced, parenthesized};

mod lean;

const MAX_UNIVERSE: usize = 65_536;

mod kw {
    syn::custom_keyword!(domain);
    syn::custom_keyword!(state);
    syn::custom_keyword!(events);
    syn::custom_keyword!(effects);
    syn::custom_keyword!(completes);
    syn::custom_keyword!(on);
    syn::custom_keyword!(case);
    syn::custom_keyword!(when);
}

fn error(at: impl Spanned, id: &str, message: impl std::fmt::Display, repair: &str) -> Error {
    super::ktr(at.span(), id, &message.to_string(), repair)
}

pub(super) fn expand(input: TokenStream) -> TokenStream {
    let declaration_source = input.to_string();
    match syn::parse2::<Declaration>(input)
        .map_err(|e| {
            if e.to_string().starts_with("PRO") {
                e
            } else {
                super::ktr(
                    e.span(),
                    "PRO001",
                    "unsupported protocol syntax",
                    "use the documented finite protocol grammar",
                )
            }
        })
        .and_then(|d| d.check())
        .map(|d| d.generate(&declaration_source))
    {
        Ok(tokens) => tokens,
        Err(e) => e.into_compile_error(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Ty {
    Bool,
    Domain(usize),
    Option(usize),
    Set(usize),
}

struct Domain {
    name: Ident,
    variants: Vec<Ident>,
}
struct Field {
    name: Ident,
    ty: syn::Type,
    init: syn::Expr,
}
#[derive(Clone)]
struct Parameter {
    name: Ident,
    ty: syn::Type,
}
struct Message {
    name: Ident,
    parameters: Vec<Parameter>,
    completions: Vec<Ident>,
}
struct Arm {
    event: Ident,
    bindings: Vec<Ident>,
    cases: Vec<Case>,
}
struct Case {
    name: Ident,
    fairness: Option<Fairness>,
    guard: syn::Expr,
    statements: Vec<Statement>,
}
enum Statement {
    Set(Ident, syn::Expr),
    Preserve(Vec<Ident>),
    Emit(Ident, Vec<syn::Expr>),
    Reject(Ident),
    Ignore(Ident),
}
struct Invariant {
    id: syn::LitStr,
    name: Ident,
    expression: syn::Expr,
    /// Also owes a Lean proof for every reachable state: `#[invariant(id = "..", theorem)]`.
    theorem: bool,
}
struct Predicate {
    name: Ident,
    expression: syn::Expr,
}
struct Property {
    id: syn::LitStr,
    request: Ident,
    response: Ident,
}
#[derive(Clone, Copy)]
enum Fairness {
    Weak,
    Strong,
}
struct FairCase {
    event: Ident,
    case: Ident,
    fairness: Fairness,
}
struct Declaration {
    visibility: syn::Visibility,
    name: Ident,
    domains: Vec<Domain>,
    fields: Vec<Field>,
    events: Vec<Message>,
    effects: Vec<Message>,
    arms: Vec<Arm>,
    invariants: Vec<Invariant>,
    predicates: Vec<Predicate>,
    properties: Vec<Property>,
    fair_cases: Vec<FairCase>,
}

fn comma_idents(input: ParseStream<'_>) -> Result<Vec<Ident>> {
    Ok(input
        .parse_terminated(Ident::parse, Token![,])?
        .into_iter()
        .collect())
}
fn messages(input: ParseStream<'_>, effects: bool) -> Result<Vec<Message>> {
    let content;
    braced!(content in input);
    let mut messages = Vec::new();
    while !content.is_empty() {
        let name = content.parse()?;
        let mut parameters = Vec::new();
        if content.peek(syn::token::Paren) {
            let args;
            parenthesized!(args in content);
            while !args.is_empty() {
                let name = args.parse()?;
                args.parse::<Token![:]>()?;
                parameters.push(Parameter {
                    name,
                    ty: args.parse()?,
                });
                if !args.is_empty() {
                    args.parse::<Token![,]>()?;
                }
            }
        }
        let completions = if effects && content.peek(kw::completes) {
            content.parse::<kw::completes>()?;
            let outcomes;
            braced!(outcomes in content);
            let names = comma_idents(&outcomes)?;
            if names.is_empty() {
                return Err(error(
                    &name,
                    "PRO013",
                    "empty effect outcomes",
                    "list every completion outcome",
                ));
            }
            names
        } else {
            vec![]
        };
        content.parse::<Token![;]>()?;
        messages.push(Message {
            name,
            parameters,
            completions,
        });
    }
    Ok(messages)
}
impl Parse for Declaration {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let visibility = input.parse()?;
        input.parse::<Token![mod]>()?;
        let name = input.parse()?;
        let body;
        braced!(body in input);
        let mut d = Self {
            visibility,
            name,
            domains: vec![],
            fields: vec![],
            events: vec![],
            effects: vec![],
            arms: vec![],
            invariants: vec![],
            predicates: vec![],
            properties: vec![],
            fair_cases: vec![],
        };
        let mut sections = HashSet::new();
        while !body.is_empty() {
            if body.peek(kw::domain) {
                body.parse::<kw::domain>()?;
                let name = body.parse()?;
                let variants;
                braced!(variants in body);
                d.domains.push(Domain {
                    name,
                    variants: comma_idents(&variants)?,
                });
            } else if body.peek(kw::state) {
                body.parse::<kw::state>()?;
                if !sections.insert("state") {
                    return Err(error(
                        &d.name,
                        "PRO001",
                        "duplicate state section",
                        "declare one state section",
                    ));
                }
                let fields;
                braced!(fields in body);
                while !fields.is_empty() {
                    let name: Ident = fields.parse()?;
                    fields.parse::<Token![:]>()?;
                    let ty = fields.parse()?;
                    if !fields.peek(Token![=]) {
                        return Err(error(
                            &name,
                            "PRO003",
                            format!("field `{name}` has no initial value"),
                            "add `= <finite initial value>` to the field",
                        ));
                    }
                    fields.parse::<Token![=]>()?;
                    let init = fields.parse()?;
                    fields.parse::<Token![;]>()?;
                    d.fields.push(Field { name, ty, init });
                }
            } else if body.peek(kw::events) || body.peek(kw::effects) {
                let section: Ident = body.parse()?;
                if !sections.insert(if section == "events" {
                    "events"
                } else {
                    "effects"
                }) {
                    return Err(error(
                        &section,
                        "PRO001",
                        "duplicate message section",
                        "declare each section once",
                    ));
                }
                let entries = messages(&body, section == "effects")?;
                if section == "events" {
                    d.events = entries;
                } else {
                    d.effects = entries;
                }
            } else if body.peek(kw::on) {
                body.parse::<kw::on>()?;
                let event = body.parse()?;
                let bindings = if body.peek(syn::token::Paren) {
                    let args;
                    parenthesized!(args in body);
                    comma_idents(&args)?
                } else {
                    vec![]
                };
                let cases;
                braced!(cases in body);
                let mut parsed = Vec::new();
                while !cases.is_empty() {
                    let attrs = cases.call(syn::Attribute::parse_outer)?;
                    let fairness = match attrs.as_slice() {
                        [] => None,
                        [attr] if attr.path().is_ident("fair") => Some(parse_fair(attr, false)?.0),
                        _ => {
                            return Err(error(
                                &d.name,
                                "PRO012",
                                "unsupported case attribute",
                                "attach #[fair(weak)] or #[fair(strong)] to a named case",
                            ));
                        }
                    };
                    cases.parse::<kw::case>()?;
                    let name = cases.parse()?;
                    cases.parse::<kw::when>()?;
                    let guard = cases.parse()?;
                    cases.parse::<Token![=>]>()?;
                    let statements;
                    braced!(statements in cases);
                    let mut operations = Vec::new();
                    while !statements.is_empty() {
                        let op: Ident = statements.parse()?;
                        operations.push(match op.to_string().as_str() {
                            "set" => {
                                let field = statements.parse()?;
                                statements.parse::<Token![=]>()?;
                                Statement::Set(field, statements.parse()?)
                            }
                            "preserve" => {
                                let mut fields = vec![statements.parse()?];
                                while statements.peek(Token![,]) {
                                    statements.parse::<Token![,]>()?;
                                    fields.push(statements.parse()?);
                                }
                                Statement::Preserve(fields)
                            }
                            "emit" => {
                                let name = statements.parse()?;
                                let mut expressions = Vec::new();
                                if statements.peek(syn::token::Paren) {
                                    let args;
                                    parenthesized!(args in statements);
                                    expressions = args
                                        .parse_terminated(syn::Expr::parse, Token![,])?
                                        .into_iter()
                                        .collect();
                                }
                                Statement::Emit(name, expressions)
                            }
                            "reject" => Statement::Reject(statements.parse()?),
                            "ignore" => Statement::Ignore(statements.parse()?),
                            _ => {
                                return Err(error(
                                    &op,
                                    "PRO001",
                                    "unsupported protocol statement",
                                    "use set, preserve, emit, reject or ignore",
                                ));
                            }
                        });
                        statements.parse::<Token![;]>()?;
                    }
                    parsed.push(Case {
                        name,
                        fairness,
                        guard,
                        statements: operations,
                    });
                }
                d.arms.push(Arm {
                    event,
                    bindings,
                    cases: parsed,
                });
            } else if body.peek(Token![#]) {
                let attrs = body.call(syn::Attribute::parse_outer)?;
                if attrs.len() != 1 {
                    return Err(error(
                        &d.name,
                        "PRO001",
                        "expected one protocol attribute",
                        "use one predicate, property or fairness attribute",
                    ));
                }
                let attr = &attrs[0];
                if attr.path().is_ident("property") {
                    d.properties.push(attr.parse_args::<Property>()?);
                    body.parse::<Token![;]>()?;
                    continue;
                }
                if attr.path().is_ident("fair") {
                    let (fairness, target) = parse_fair(attr, true)?;
                    let (event, case) = target.expect("required target checked");
                    d.fair_cases.push(FairCase {
                        event,
                        case,
                        fairness,
                    });
                    body.parse::<Token![;]>()?;
                    continue;
                }
                let is_predicate = ["predicate", "terminal", "quiescent"]
                    .iter()
                    .any(|name| attr.path().is_ident(name));
                let mut id = None;
                let mut theorem = false;
                if !is_predicate && !attr.path().is_ident("invariant") {
                    return Err(error(
                        &d.name,
                        "PRO001",
                        "unsupported protocol attribute",
                        "use #[invariant(id = \"stable-id\")] on a state predicate",
                    ));
                }
                if is_predicate {
                    if !matches!(attr.meta, syn::Meta::Path(_)) {
                        return Err(error(
                            attr,
                            "PRO001",
                            "predicate attribute takes no arguments",
                            "use #[predicate], #[terminal] or #[quiescent]",
                        ));
                    }
                } else {
                    attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("theorem") && !theorem {
                            theorem = true;
                            return Ok(());
                        }
                        if !meta.path.is_ident("id") || id.is_some() {
                            return Err(meta.error(
                                "PRO001 expected one invariant id. Repair: use id = \"stable-id\".",
                            ));
                        }
                        id = Some(meta.value()?.parse::<syn::LitStr>()?);
                        Ok(())
                    })?;
                }
                let id = if is_predicate {
                    None
                } else {
                    Some(id.filter(|id| !id.value().is_empty()).ok_or_else(|| {
                        error(
                            &d.name,
                            "PRO001",
                            "missing invariant id",
                            "provide a nonempty id",
                        )
                    })?)
                };
                body.parse::<Token![fn]>()?;
                let name = body.parse()?;
                let args;
                parenthesized!(args in body);
                if !args.is_empty() {
                    return Err(error(
                        &d.name,
                        "PRO001",
                        "invariant parameters are unsupported",
                        "refer directly to state fields",
                    ));
                }
                let predicate;
                braced!(predicate in body);
                let expression = predicate.parse()?;
                if !predicate.is_empty() {
                    return Err(error(
                        &d.name,
                        "PRO001",
                        "invariants require one expression",
                        "use one finite Boolean predicate without statements",
                    ));
                }
                if let Some(id) = id {
                    d.invariants.push(Invariant {
                        id,
                        name,
                        expression,
                        theorem,
                    });
                } else {
                    d.predicates.push(Predicate { name, expression });
                }
            } else {
                return Err(error(
                    &d.name,
                    "PRO001",
                    "unsupported protocol declaration",
                    "use domain, state, events, effects, on, or an invariant",
                ));
            }
        }
        // Outcome events carry the original intent's finite parameters. Admission
        // and completion are separate inputs to step, never recursive execution.
        for effect in &d.effects {
            if !effect.completions.is_empty() {
                unique(effect.completions.clone(), &["Admitted"])?;
                for outcome in std::iter::once(format_ident!("Admitted"))
                    .chain(effect.completions.iter().cloned())
                {
                    d.events.push(Message {
                        name: format_ident!("{}{}", effect.name, outcome),
                        parameters: effect.parameters.clone(),
                        completions: vec![],
                    });
                }
            }
        }
        Ok(d)
    }
}

impl Parse for Property {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let id_key: Ident = input.parse()?;
        if id_key != "id" {
            return Err(error(
                id_key,
                "PRO011",
                "missing property id",
                "use id = \"stable-id\", leads_to(P, Q)",
            ));
        }
        input.parse::<Token![=]>()?;
        let id: syn::LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let leads_to: Ident = input.parse()?;
        if leads_to != "leads_to" || id.value().is_empty() {
            return Err(error(
                leads_to,
                "PRO011",
                "unsupported temporal property",
                "use a nonempty id and leads_to(P, Q)",
            ));
        }
        let predicates;
        parenthesized!(predicates in input);
        let request = predicates.parse()?;
        predicates.parse::<Token![,]>()?;
        let response = predicates.parse()?;
        if !predicates.is_empty() {
            return Err(predicates
                .error("PRO011 leads_to needs two state predicates. Repair: use leads_to(P, Q)."));
        }
        Ok(Self {
            id,
            request,
            response,
        })
    }
}

fn parse_fair(attr: &syn::Attribute, targeted: bool) -> Result<(Fairness, Option<(Ident, Ident)>)> {
    attr.parse_args_with(|input: ParseStream<'_>| {
        let kind: Ident = input.parse()?;
        let fairness = match kind.to_string().as_str() {
            "weak" => Fairness::Weak,
            "strong" => Fairness::Strong,
            _ => {
                return Err(error(
                    kind,
                    "PRO012",
                    "unsupported fairness",
                    "use weak or strong fairness on a named case",
                ));
            }
        };
        let target = if targeted {
            input.parse::<Token![,]>()?;
            input.parse::<kw::case>()?;
            input.parse::<Token![=]>()?;
            let event = input.parse()?;
            input.parse::<Token![::]>()?;
            let case = input.parse()?;
            Some((event, case))
        } else {
            None
        };
        Ok((fairness, target))
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Scalar(i64),
    Set(Vec<bool>),
}
#[derive(Clone)]
struct Expression {
    ty: Ty,
    kind: Kind,
}
#[derive(Clone)]
enum Kind {
    Value(Value),
    Field(usize),
    Parameter(usize),
    Some(Box<Expression>),
    IsNone(Box<Expression>),
    Not(Box<Expression>),
    Binary(Op, Box<Expression>, Box<Expression>),
    SetOp(SetOp, Box<Expression>, Box<Expression>),
}
#[derive(Clone, Copy)]
enum Op {
    Eq,
    Ne,
    And,
    Or,
}
#[derive(Clone, Copy)]
enum SetOp {
    Contains,
    With,
    Without,
}

struct Environment<'a> {
    declaration: &'a Declaration,
    fields: &'a [Ty],
    parameters: &'a [(Ident, Ty)],
}
impl Declaration {
    fn ty(&self, ty: &syn::Type) -> Result<Ty> {
        if let syn::Type::Path(p) = ty
            && p.qself.is_none()
            && p.path.leading_colon.is_none()
            && p.path.segments.len() == 1
        {
            let segment = &p.path.segments[0];
            let name = &segment.ident;
            match &segment.arguments {
                syn::PathArguments::None => {
                    if name == "bool" {
                        return Ok(Ty::Bool);
                    }
                    if let Some(i) = self.domains.iter().position(|d| d.name == *name) {
                        return Ok(Ty::Domain(i));
                    }
                }
                syn::PathArguments::AngleBracketed(args) if args.args.len() == 1 => {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first()
                        && let Ty::Domain(i) = self.ty(inner)?
                    {
                        if name == "Option" {
                            return Ok(Ty::Option(i));
                        }
                        if name == "Set" {
                            return Ok(Ty::Set(i));
                        }
                    }
                }
                _ => {}
            }
        }
        Err(error(
            ty,
            "PRO008",
            "unsupported protocol type",
            "use bool, a declared domain, Option<Domain> or Set<Domain>",
        ))
    }
    fn type_name(&self, ty: &Ty) -> String {
        match ty {
            Ty::Bool => "bool".into(),
            Ty::Domain(i) => self.domains[*i].name.to_string(),
            Ty::Option(i) => format!("Option<{}>", self.domains[*i].name),
            Ty::Set(i) => format!("Set<{}>", self.domains[*i].name),
        }
    }
    fn cardinality(&self, ty: &Ty) -> usize {
        match ty {
            Ty::Bool => 2,
            Ty::Domain(i) => self.domains[*i].variants.len(),
            Ty::Option(i) => self.domains[*i].variants.len() + 1,
            Ty::Set(i) => 1_usize
                .checked_shl(self.domains[*i].variants.len() as u32)
                .unwrap_or(usize::MAX),
        }
    }
    fn values(&self, ty: &Ty) -> Vec<Value> {
        (0..self.cardinality(ty))
            .map(|n| match ty {
                Ty::Option(_) => Value::Scalar(n as i64 - 1),
                Ty::Set(i) => Value::Set(
                    (0..self.domains[*i].variants.len())
                        .map(|bit| n & (1 << bit) != 0)
                        .collect(),
                ),
                _ => Value::Scalar(n as i64),
            })
            .collect()
    }
}

impl Environment<'_> {
    fn expression(&self, expr: &syn::Expr, expected: Option<&Ty>) -> Result<Expression> {
        use syn::Expr as E;
        let unsupported = || {
            error(
                expr,
                "PRO001",
                "unsupported protocol expression or arbitrary call",
                "use finite values, ==, !=, &&, ||, !, Some, is_none, contains, with or without",
            )
        };
        let result = match expr {
            E::Paren(p) => return self.expression(&p.expr, expected),
            E::Group(g) => return self.expression(&g.expr, expected),
            E::Lit(l) => {
                if let syn::Lit::Bool(b) = &l.lit {
                    Expression {
                        ty: Ty::Bool,
                        kind: Kind::Value(Value::Scalar(i64::from(b.value))),
                    }
                } else {
                    return Err(unsupported());
                }
            }
            E::Path(p)
                if p.qself.is_none()
                    && p.path.leading_colon.is_none()
                    && p.path
                        .segments
                        .iter()
                        .all(|s| matches!(s.arguments, syn::PathArguments::None)) =>
            {
                let segments: Vec<_> = p.path.segments.iter().map(|s| &s.ident).collect();
                match segments.as_slice() {
                    [name] if *name == "None" => {
                        if let Some(Ty::Option(d)) = expected {
                            Expression {
                                ty: Ty::Option(*d),
                                kind: Kind::Value(Value::Scalar(-1)),
                            }
                        } else {
                            return Err(error(
                                expr,
                                "PRO008",
                                "None needs an Option<Domain> context",
                                "compare with a typed field or assign to an Option field",
                            ));
                        }
                    }
                    [name] => {
                        if let Some(i) = self.parameters.iter().position(|(n, _)| n == *name) {
                            Expression {
                                ty: self.parameters[i].1.clone(),
                                kind: Kind::Parameter(i),
                            }
                        } else if let Some(i) = self
                            .declaration
                            .fields
                            .iter()
                            .position(|f| f.name == **name)
                        {
                            if let Some(ty) = self.fields.get(i) {
                                Expression {
                                    ty: ty.clone(),
                                    kind: Kind::Field(i),
                                }
                            } else {
                                return Err(error(
                                    expr,
                                    "PRO008",
                                    "initial values cannot read state",
                                    "use a constant finite value",
                                ));
                            }
                        } else {
                            return Err(error(
                                expr,
                                "PRO008",
                                format!("unknown protocol name `{name}`"),
                                "use a state field, event binding or qualified domain variant",
                            ));
                        }
                    }
                    [domain, variant] => {
                        let d = self
                            .declaration
                            .domains
                            .iter()
                            .position(|d| d.name == **domain)
                            .ok_or_else(unsupported)?;
                        let v = self.declaration.domains[d]
                            .variants
                            .iter()
                            .position(|v| v == *variant)
                            .ok_or_else(unsupported)?;
                        Expression {
                            ty: Ty::Domain(d),
                            kind: Kind::Value(Value::Scalar(v as i64)),
                        }
                    }
                    _ => return Err(unsupported()),
                }
            }
            E::Call(call) => {
                if let E::Path(p) = call.func.as_ref() {
                    if p.qself.is_some() {
                        return Err(unsupported());
                    }
                    if p.path.is_ident("Some") && call.args.len() == 1 {
                        let inner_ty = match expected {
                            Some(Ty::Option(d)) => Some(Ty::Domain(*d)),
                            _ => None,
                        };
                        let inner = self.expression(&call.args[0], inner_ty.as_ref())?;
                        if let Ty::Domain(d) = inner.ty {
                            Expression {
                                ty: Ty::Option(d),
                                kind: Kind::Some(Box::new(inner)),
                            }
                        } else {
                            return Err(error(
                                expr,
                                "PRO008",
                                "Some requires a domain value",
                                "use Some(Domain::Variant) or a domain binding",
                            ));
                        }
                    } else if p.path.segments.len() == 2
                        && p.path.leading_colon.is_none()
                        && p.path.segments[0].ident == "Set"
                        && p.path.segments[1].ident == "empty"
                        && p.path
                            .segments
                            .iter()
                            .all(|s| matches!(s.arguments, syn::PathArguments::None))
                        && call.args.is_empty()
                    {
                        if let Some(Ty::Set(d)) = expected {
                            Expression {
                                ty: Ty::Set(*d),
                                kind: Kind::Value(Value::Set(vec![
                                    false;
                                    self.declaration.domains[*d]
                                        .variants
                                        .len()
                                ])),
                            }
                        } else {
                            return Err(error(
                                expr,
                                "PRO008",
                                "Set::empty needs a Set<Domain> context",
                                "assign to a Set field or use a typed set operand",
                            ));
                        }
                    } else {
                        return Err(unsupported());
                    }
                } else {
                    return Err(unsupported());
                }
            }
            E::Unary(u) if matches!(u.op, syn::UnOp::Not(_)) => Expression {
                ty: Ty::Bool,
                kind: Kind::Not(Box::new(self.expression(&u.expr, Some(&Ty::Bool))?)),
            },
            E::Binary(b) => {
                let op = match b.op {
                    syn::BinOp::Eq(_) => Op::Eq,
                    syn::BinOp::Ne(_) => Op::Ne,
                    syn::BinOp::And(_) => Op::And,
                    syn::BinOp::Or(_) => Op::Or,
                    _ => return Err(unsupported()),
                };
                let (left, right) = if matches!(op, Op::And | Op::Or) {
                    (
                        self.expression(&b.left, Some(&Ty::Bool))?,
                        self.expression(&b.right, Some(&Ty::Bool))?,
                    )
                } else if matches!(b.left.as_ref(), E::Path(p) if p.path.is_ident("None")) {
                    let right = self.expression(&b.right, None)?;
                    (self.expression(&b.left, Some(&right.ty))?, right)
                } else {
                    let left = self.expression(&b.left, None)?;
                    let right = self.expression(&b.right, Some(&left.ty))?;
                    (left, right)
                };
                Expression {
                    ty: Ty::Bool,
                    kind: Kind::Binary(op, Box::new(left), Box::new(right)),
                }
            }
            E::MethodCall(m) if m.turbofish.is_none() => {
                let name = m.method.to_string();
                let receiver_expected = if matches!(name.as_str(), "with" | "without") {
                    expected
                } else {
                    None
                };
                let receiver = self.expression(&m.receiver, receiver_expected)?;
                if name == "is_none" && m.args.is_empty() && matches!(receiver.ty, Ty::Option(_)) {
                    Expression {
                        ty: Ty::Bool,
                        kind: Kind::IsNone(Box::new(receiver)),
                    }
                } else if let Ty::Set(d) = receiver.ty {
                    if m.args.len() != 1 {
                        return Err(unsupported());
                    }
                    let op = match name.as_str() {
                        "contains" => SetOp::Contains,
                        "with" => SetOp::With,
                        "without" => SetOp::Without,
                        _ => return Err(unsupported()),
                    };
                    let arg = self.expression(&m.args[0], Some(&Ty::Domain(d)))?;
                    Expression {
                        ty: if matches!(op, SetOp::Contains) {
                            Ty::Bool
                        } else {
                            Ty::Set(d)
                        },
                        kind: Kind::SetOp(op, Box::new(receiver), Box::new(arg)),
                    }
                } else {
                    return Err(unsupported());
                }
            }
            _ => return Err(unsupported()),
        };
        if expected.is_some_and(|ty| *ty != result.ty) {
            return Err(error(
                expr,
                "PRO008",
                format!(
                    "protocol type mismatch: expected {}, found {}",
                    self.declaration.type_name(expected.unwrap()),
                    self.declaration.type_name(&result.ty)
                ),
                "use a value from the declared type and domain",
            ));
        }
        Ok(result)
    }
}

impl Expression {
    fn read_fields(&self, fields: &mut BTreeSet<usize>) {
        match &self.kind {
            Kind::Field(i) => {
                fields.insert(*i);
            }
            Kind::Some(a) | Kind::IsNone(a) | Kind::Not(a) => a.read_fields(fields),
            Kind::Binary(_, a, b) | Kind::SetOp(_, a, b) => {
                a.read_fields(fields);
                b.read_fields(fields);
            }
            _ => {}
        }
    }
    fn eval(&self, fields: &[Value], parameters: &[Value]) -> Value {
        let scalar = |e: &Expression| match e.eval(fields, parameters) {
            Value::Scalar(n) => n,
            _ => unreachable!("typed scalar"),
        };
        let bit = |b| Value::Scalar(i64::from(b));
        match &self.kind {
            Kind::Value(v) => v.clone(),
            Kind::Field(i) => fields[*i].clone(),
            Kind::Parameter(i) => parameters[*i].clone(),
            Kind::Some(a) => a.eval(fields, parameters),
            Kind::IsNone(a) => bit(scalar(a) == -1),
            Kind::Not(a) => bit(scalar(a) == 0),
            Kind::Binary(op, a, b) => bit(match op {
                Op::Eq => a.eval(fields, parameters) == b.eval(fields, parameters),
                Op::Ne => a.eval(fields, parameters) != b.eval(fields, parameters),
                Op::And => scalar(a) != 0 && scalar(b) != 0,
                Op::Or => scalar(a) != 0 || scalar(b) != 0,
            }),
            Kind::SetOp(op, a, b) => {
                let Value::Set(mut bits) = a.eval(fields, parameters) else {
                    unreachable!("typed set")
                };
                let i = scalar(b) as usize;
                match op {
                    SetOp::Contains => bit(bits[i]),
                    SetOp::With => {
                        bits[i] = true;
                        Value::Set(bits)
                    }
                    SetOp::Without => {
                        bits[i] = false;
                        Value::Set(bits)
                    }
                }
            }
        }
    }
}

struct Checked {
    declaration: Declaration,
    fields: Vec<Ty>,
    initial: Vec<Value>,
    effects: Vec<Vec<Ty>>,
    arms: Vec<CheckedArm>,
    invariants: Vec<Expression>,
    predicates: Vec<Expression>,
}
struct CheckedArm {
    event: usize,
    parameters: Vec<(Ident, Ty)>,
    cases: Vec<CheckedCase>,
}
struct CheckedCase {
    name: Ident,
    fairness: Option<Fairness>,
    guard: Expression,
    assignments: Vec<Option<Expression>>,
    emits: Vec<(usize, Vec<Expression>)>,
    end: Option<End>,
}
enum End {
    Reject(Ident),
    Ignore(Ident),
}

fn unique(names: impl IntoIterator<Item = Ident>, reserved: &[&str]) -> Result<()> {
    let mut seen: HashSet<_> = reserved.iter().map(|s| (*s).to_owned()).collect();
    for name in names {
        if !seen.insert(name.to_string()) {
            return Err(error(
                &name,
                "PRO009",
                format!("duplicate or reserved protocol name `{name}`"),
                "choose a unique name outside the generated API",
            ));
        }
    }
    Ok(())
}

// Finite mixed-radix enumeration. Never substitutes reachable states or invariants
// for the declared universe; all guard reads count, including short-circuit RHSs.
fn each_valuation(
    d: &Declaration,
    types: &[Ty],
    mut visit: impl FnMut(&[Value]) -> Result<()>,
) -> Result<()> {
    let choices: Vec<_> = types.iter().map(|ty| d.values(ty)).collect();
    let mut digits = vec![0; types.len()];
    loop {
        let values: Vec<_> = choices
            .iter()
            .zip(&digits)
            .map(|(v, i)| v[*i].clone())
            .collect();
        visit(&values)?;
        let mut i = digits.len();
        loop {
            if i == 0 {
                return Ok(());
            }
            i -= 1;
            digits[i] += 1;
            if digits[i] < choices[i].len() {
                break;
            }
            digits[i] = 0;
        }
    }
}
impl Declaration {
    fn check(mut self) -> Result<Checked> {
        let model_name = self.name.to_string();
        if !model_name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic())
            || !model_name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(error(
                &self.name,
                "PRO008",
                "protocol module name is not a TLA+ identifier",
                "use an ASCII module name starting with a letter",
            ));
        }
        unique(
            self.domains.iter().map(|d| d.name.clone()),
            &[
                "bool",
                "Option",
                "Set",
                "State",
                "Event",
                "Effect",
                "Decision",
                "RejectReason",
                "IgnoreReason",
            ],
        )?;
        for d in &self.domains {
            if d.variants.is_empty() || d.variants.len() > 63 {
                return Err(error(
                    &d.name,
                    "PRO010",
                    "a protocol domain needs 1..=63 variants",
                    "declare a nonempty finite domain within the set representation bound",
                ));
            }
            unique(d.variants.clone(), &[])?;
        }
        if self.fields.is_empty() || self.events.is_empty() {
            return Err(error(
                &self.name,
                "PRO001",
                "protocol needs state fields and events",
                "declare nonempty state and events sections",
            ));
        }
        unique(
            self.fields.iter().map(|f| f.name.clone()),
            &["default", "model_values"],
        )?;
        unique(self.events.iter().map(|e| e.name.clone()), &[])?;
        unique(self.effects.iter().map(|e| e.name.clone()), &[])?;
        unique(
            self.invariants
                .iter()
                .map(|i| i.name.clone())
                .chain(self.predicates.iter().map(|p| p.name.clone())),
            &[
                "model",
                "step",
                "action_event",
                "action_case",
                "verification_metadata",
            ],
        )?;
        let mut ids = HashSet::new();
        for invariant in &self.invariants {
            if !ids.insert(invariant.id.value()) {
                return Err(error(
                    &invariant.id,
                    "PRO006",
                    format!("duplicate invariant id `{}`", invariant.id.value()),
                    "give each invariant a unique stable id",
                ));
            }
        }
        for property in &self.properties {
            if !ids.insert(property.id.value()) {
                return Err(error(
                    &property.id,
                    "PRO006",
                    "duplicate property id",
                    "give every invariant and temporal property a unique stable id",
                ));
            }
            for predicate in [&property.request, &property.response] {
                if !self.predicates.iter().any(|p| p.name == *predicate)
                    && !self.invariants.iter().any(|p| p.name == *predicate)
                {
                    return Err(error(
                        predicate,
                        "PRO011",
                        format!("unknown state predicate `{predicate}`"),
                        "declare the predicate with #[predicate], #[terminal], #[quiescent] or #[invariant]",
                    ));
                }
            }
        }
        for fair in &self.fair_cases {
            let case = self
                .arms
                .iter_mut()
                .find(|a| a.event == fair.event)
                .and_then(|a| a.cases.iter_mut().find(|c| c.name == fair.case))
                .ok_or_else(|| {
                    error(
                        &fair.case,
                        "PRO012",
                        format!("unknown fair case `{}::{}`", fair.event, fair.case),
                        "name a declared Event::case or attach #[fair(weak)] directly to that case",
                    )
                })?;
            if case.fairness.replace(fair.fairness).is_some() {
                return Err(error(
                    &fair.case,
                    "PRO012",
                    "case has multiple fairness annotations",
                    "declare fairness once per named case",
                ));
            }
        }
        let fields = self
            .fields
            .iter()
            .map(|f| self.ty(&f.ty))
            .collect::<Result<Vec<_>>>()?;
        let initial_env = Environment {
            declaration: &self,
            fields: &[],
            parameters: &[],
        };
        let initial = self
            .fields
            .iter()
            .zip(&fields)
            .map(|(f, ty)| {
                initial_env
                    .expression(&f.init, Some(ty))
                    .map(|e| e.eval(&[], &[]))
            })
            .collect::<Result<Vec<_>>>()?;
        let effect_types = self
            .effects
            .iter()
            .map(|e| {
                e.parameters
                    .iter()
                    .map(|p| self.ty(&p.ty))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        for message in self.events.iter().chain(&self.effects) {
            unique(message.parameters.iter().map(|p| p.name.clone()), &[])?;
        }
        unique(self.arms.iter().map(|a| a.event.clone()), &[])?;
        for arm in &self.arms {
            if !self.events.iter().any(|e| e.name == arm.event) {
                return Err(error(
                    &arm.event,
                    "PRO002",
                    "on arm names an undeclared event",
                    "declare the event in events",
                ));
            }
        }
        let mut arms = Vec::new();
        for (event_index, event) in self.events.iter().enumerate() {
            let arm = self
                .arms
                .iter()
                .find(|a| a.event == event.name)
                .ok_or_else(|| {
                    error(
                        &event.name,
                        "PRO002",
                        format!("event `{}` has no on arm", event.name),
                        "add an on arm with exhaustive, disjoint cases",
                    )
                })?;
            if event.parameters.len() != arm.bindings.len() {
                return Err(error(
                    &arm.event,
                    "PRO008",
                    "event binding arity mismatch",
                    "bind every declared event parameter in declaration order",
                ));
            }
            unique(arm.bindings.clone(), &[])?;
            if let Some(binding) = arm
                .bindings
                .iter()
                .find(|b| self.fields.iter().any(|f| f.name == **b))
            {
                return Err(error(
                    binding,
                    "PRO009",
                    "event binding shadows a state field",
                    "rename the binding",
                ));
            }
            let parameters = arm
                .bindings
                .iter()
                .zip(&event.parameters)
                .map(|(b, p)| Ok((b.clone(), self.ty(&p.ty)?)))
                .collect::<Result<Vec<_>>>()?;
            let env = Environment {
                declaration: &self,
                fields: &fields,
                parameters: &parameters,
            };
            unique(arm.cases.iter().map(|c| c.name.clone()), &[])?;
            let mut cases = Vec::new();
            for case in &arm.cases {
                let guard = env.expression(&case.guard, Some(&Ty::Bool))?;
                let mut seen = HashSet::new();
                let mut assignments = vec![None; fields.len()];
                let mut emits = Vec::new();
                let mut end = None;
                let field_index = |name: &Ident| {
                    self.fields
                        .iter()
                        .position(|f| f.name == *name)
                        .ok_or_else(|| {
                            error(
                                name,
                                "PRO008",
                                format!("unknown state field `{name}`"),
                                "name a declared field",
                            )
                        })
                };
                for statement in &case.statements {
                    match statement {
                        Statement::Set(name, expression) => {
                            let i = field_index(name)?;
                            if !seen.insert(i) {
                                return Err(error(
                                    name,
                                    "PRO009",
                                    "field set or preserved twice",
                                    "account for each field exactly once",
                                ));
                            }
                            assignments[i] = Some(env.expression(expression, Some(&fields[i]))?);
                        }
                        Statement::Preserve(names) => {
                            for name in names {
                                let i = field_index(name)?;
                                if !seen.insert(i) {
                                    return Err(error(
                                        name,
                                        "PRO009",
                                        "field set or preserved twice",
                                        "account for each field exactly once",
                                    ));
                                }
                            }
                        }
                        Statement::Emit(name, args) => {
                            let i = self
                                .effects
                                .iter()
                                .position(|e| e.name == *name)
                                .ok_or_else(|| {
                                    error(
                                        name,
                                        "PRO008",
                                        "undeclared effect",
                                        "declare it in effects",
                                    )
                                })?;
                            if args.len() != effect_types[i].len() {
                                return Err(error(
                                    name,
                                    "PRO008",
                                    "effect argument arity mismatch",
                                    "provide each declared effect parameter",
                                ));
                            }
                            let args = args
                                .iter()
                                .zip(&effect_types[i])
                                .map(|(e, ty)| env.expression(e, Some(ty)))
                                .collect::<Result<Vec<_>>>()?;
                            emits.push((i, args));
                        }
                        Statement::Reject(name) | Statement::Ignore(name) => {
                            if end.is_some() {
                                return Err(error(
                                    name,
                                    "PRO009",
                                    "case has multiple terminal decisions",
                                    "use one reject or ignore",
                                ));
                            }
                            end = Some(if matches!(statement, Statement::Reject(_)) {
                                End::Reject(name.clone())
                            } else {
                                End::Ignore(name.clone())
                            });
                        }
                    }
                }
                if let Some((_, field)) = self
                    .fields
                    .iter()
                    .enumerate()
                    .find(|(i, _)| !seen.contains(i))
                {
                    return Err(error(
                        &case.name,
                        "PRO004",
                        format!(
                            "case `{}` neither sets nor preserves field `{}`",
                            case.name, field.name
                        ),
                        "set or explicitly preserve every field, including reject/ignore cases",
                    ));
                }
                if end.is_some() && (assignments.iter().any(Option::is_some) || !emits.is_empty()) {
                    return Err(error(
                        &case.name,
                        "PRO008",
                        "reject/ignore cannot update fields or emit effects",
                        "preserve every field and remove set/emit from this case",
                    ));
                }
                cases.push(CheckedCase {
                    name: case.name.clone(),
                    fairness: case.fairness,
                    guard,
                    assignments,
                    emits,
                    end,
                });
            }
            let mut reads = BTreeSet::new();
            for case in &cases {
                case.guard.read_fields(&mut reads);
            }
            let types: Vec<_> = parameters
                .iter()
                .map(|(_, t)| t.clone())
                .chain(reads.iter().map(|i| fields[*i].clone()))
                .collect();
            let count = types
                .iter()
                .try_fold(1_usize, |n, ty| n.checked_mul(self.cardinality(ty)))
                .unwrap_or(usize::MAX);
            let describe = |name: &Ident, ty: &Ty| {
                format!("{name}: {} ({})", self.type_name(ty), self.cardinality(ty))
            };
            let universe = format!(
                "parameters [{}] × guard state [{}]; {count} valuations, all declared values without reachability/invariant pruning",
                parameters
                    .iter()
                    .map(|(n, t)| describe(n, t))
                    .collect::<Vec<_>>()
                    .join(", "),
                reads
                    .iter()
                    .map(|i| describe(&self.fields[*i].name, &fields[*i]))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if count > MAX_UNIVERSE {
                return Err(error(
                    &arm.event,
                    "PRO010",
                    format!("guard universe exceeds {MAX_UNIVERSE}: {universe}"),
                    "reduce domain sizes, event parameters or guard state reads; partial checking is refused",
                ));
            }
            each_valuation(&self, &types, |values| {
                let (params, state_values) = values.split_at(parameters.len());
                let mut state = initial.clone();
                for (i, value) in reads.iter().zip(state_values) {
                    state[*i] = value.clone();
                }
                let enabled = cases
                    .iter()
                    .filter(|c| c.guard.eval(&state, params) == Value::Scalar(1))
                    .count();
                if enabled != 1 {
                    return Err(error(
                        &arm.event,
                        "PRO005",
                        format!(
                            "event `{}` guards {} ({enabled} matching cases); checked universe: {universe}",
                            arm.event,
                            if enabled == 0 {
                                "do not cover"
                            } else {
                                "overlap"
                            }
                        ),
                        "partition this entire finite universe into exactly one case per valuation",
                    ));
                }
                Ok(())
            })?;
            arms.push(CheckedArm {
                event: event_index,
                parameters,
                cases,
            });
        }
        let invariants = self
            .invariants
            .iter()
            .map(|i| {
                Environment {
                    declaration: &self,
                    fields: &fields,
                    parameters: &[],
                }
                .expression(&i.expression, Some(&Ty::Bool))
            })
            .collect::<Result<Vec<_>>>()?;
        let predicates = self
            .predicates
            .iter()
            .map(|p| {
                Environment {
                    declaration: &self,
                    fields: &fields,
                    parameters: &[],
                }
                .expression(&p.expression, Some(&Ty::Bool))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Checked {
            declaration: self,
            fields,
            initial,
            effects: effect_types,
            arms,
            invariants,
            predicates,
        })
    }
}

impl Checked {
    fn rust_ty(&self, ty: &Ty) -> TokenStream {
        let path = super::kittens_path();
        match ty {
            Ty::Bool => quote!(::core::primitive::bool),
            Ty::Domain(i) => {
                let name = &self.declaration.domains[*i].name;
                quote!(#name)
            }
            Ty::Option(i) => {
                let name = &self.declaration.domains[*i].name;
                quote!(::core::option::Option<#name>)
            }
            Ty::Set(i) => {
                let name = &self.declaration.domains[*i].name;
                quote!(#path::protocol::Set<#name>)
            }
        }
    }
    fn rust_value(&self, ty: &Ty, value: &Value) -> TokenStream {
        match (ty, value) {
            (Ty::Bool, Value::Scalar(n)) => {
                let b = *n != 0;
                quote!(#b)
            }
            (Ty::Domain(d), Value::Scalar(n)) => {
                let domain = &self.declaration.domains[*d].name;
                let variant = &self.declaration.domains[*d].variants[*n as usize];
                quote!(#domain::#variant)
            }
            (Ty::Option(_), Value::Scalar(-1)) => quote!(::core::option::Option::None),
            (Ty::Option(d), value) => {
                let inner = self.rust_value(&Ty::Domain(*d), value);
                quote!(::core::option::Option::Some(#inner))
            }
            (Ty::Set(d), Value::Set(bits)) => {
                let ty = self.rust_ty(ty);
                let mut result = quote!(<#ty>::empty());
                for (i, b) in bits.iter().enumerate() {
                    if *b {
                        let value = self.rust_value(&Ty::Domain(*d), &Value::Scalar(i as i64));
                        result = quote!(#result.with(#value));
                    }
                }
                result
            }
            _ => unreachable!("typed value"),
        }
    }
    fn rust_expr(&self, expression: &Expression, parameters: &[(Ident, Ty)]) -> TokenStream {
        let recurse = |e| self.rust_expr(e, parameters);
        let state = Ident::new("__yarn_state", Span::mixed_site());
        match &expression.kind {
            Kind::Value(v) => self.rust_value(&expression.ty, v),
            Kind::Field(i) => {
                let name = &self.declaration.fields[*i].name;
                quote!(#state.#name)
            }
            Kind::Parameter(i) => {
                let name = &parameters[*i].0;
                quote!(#name)
            }
            Kind::Some(a) => {
                let a = recurse(a);
                quote!(::core::option::Option::Some(#a))
            }
            Kind::IsNone(a) => {
                let a = recurse(a);
                quote!((#a).is_none())
            }
            Kind::Not(a) => {
                let a = recurse(a);
                quote!(!(#a))
            }
            Kind::Binary(op, a, b) => {
                let (a, b) = (recurse(a), recurse(b));
                let op = match op {
                    Op::Eq => quote!(==),
                    Op::Ne => quote!(!=),
                    Op::And => quote!(&&),
                    Op::Or => quote!(||),
                };
                quote!((#a) #op (#b))
            }
            Kind::SetOp(op, a, b) => {
                let (a, b) = (recurse(a), recurse(b));
                let method = format_ident!(
                    "{}",
                    match op {
                        SetOp::Contains => "contains",
                        SetOp::With => "with",
                        SetOp::Without => "without",
                    }
                );
                quote!((#a).#method(#b))
            }
        }
    }
    fn message(&self, enum_name: &Ident, name: &Ident, args: &[TokenStream]) -> TokenStream {
        if args.is_empty() {
            quote!(#enum_name::#name)
        } else {
            quote!(#enum_name::#name(#(#args),*))
        }
    }
    fn generate(&self, declaration_source: &str) -> TokenStream {
        let d = &self.declaration;
        let path = super::kittens_path();
        let domains = d.domains.iter().map(|domain| {
            let name = &domain.name;
            let variants = &domain.variants;
            let ordinals = (0..variants.len()).map(|i| i as u32);
            quote! {
                #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                pub enum #name { #(#variants),* }
                impl #path::protocol::FiniteDomain for #name {
                    fn ordinal(self) -> ::core::primitive::u32 { match self { #(Self::#variants => #ordinals),* } }
                }
            }
        });
        let field_names: Vec<_> = d.fields.iter().map(|f| &f.name).collect();
        let field_types: Vec<_> = self.fields.iter().map(|ty| self.rust_ty(ty)).collect();
        let initial = self
            .fields
            .iter()
            .zip(&self.initial)
            .map(|(ty, value)| self.rust_value(ty, value));
        let events = d.events.iter().zip(&self.arms).map(|(event, arm)| {
            let name = &event.name;
            let types: Vec<_> = arm
                .parameters
                .iter()
                .map(|(_, ty)| self.rust_ty(ty))
                .collect();
            if types.is_empty() {
                quote!(#name)
            } else {
                quote!(#name(#(#types),*))
            }
        });
        let effects = d.effects.iter().zip(&self.effects).map(|(effect, types)| {
            let name = &effect.name;
            let types: Vec<_> = types.iter().map(|ty| self.rust_ty(ty)).collect();
            if types.is_empty() {
                quote!(#name)
            } else {
                quote!(#name(#(#types),*))
            }
        });
        let reasons = |reject: bool| {
            let mut seen = BTreeSet::new();
            self.arms
                .iter()
                .flat_map(|a| &a.cases)
                .filter_map(|c| match &c.end {
                    Some(End::Reject(n)) if reject => Some(n),
                    Some(End::Ignore(n)) if !reject => Some(n),
                    _ => None,
                })
                .filter(|name| seen.insert(name.to_string()))
                .cloned()
                .collect::<Vec<_>>()
        };
        let rejects = reasons(true);
        let ignores = reasons(false);
        let state = Ident::new("__yarn_state", Span::mixed_site());
        let event_var = Ident::new("__yarn_event", Span::mixed_site());
        let event_enum = format_ident!("Event");
        let effect_enum = format_ident!("Effect");
        let arms = self.arms.iter().map(|arm| {
            let event = &d.events[arm.event].name;
            let bindings: Vec<_> = arm.parameters.iter().map(|(n, _)| quote!(#n)).collect();
            let pattern = self.message(&event_enum, event, &bindings);
            let cases = arm.cases.iter().map(|case| {
                let guard = self.rust_expr(&case.guard, &arm.parameters);
                let result = match &case.end {
                    Some(End::Reject(n)) => quote!(Decision::Rejected { reason: RejectReason::#n }),
                    Some(End::Ignore(n)) => quote!(Decision::Ignored { reason: IgnoreReason::#n }),
                    None => {
                        let next = case.assignments.iter().enumerate().map(|(i, e)| e.as_ref().map_or_else(|| { let name = field_names[i]; quote!(#state.#name) }, |e| self.rust_expr(e, &arm.parameters)));
                        let emitted = case.emits.iter().map(|(i, args)| {
                            let args: Vec<_> = args.iter().map(|e| self.rust_expr(e, &arm.parameters)).collect();
                            self.message(&effect_enum, &d.effects[*i].name, &args)
                        });
                        quote!(Decision::Accepted { next: State { #(#field_names: #next),* }, effects: #path::protocol::Effects::from([#(#emitted),*]) })
                    }
                };
                quote!(if #guard { return #result; })
            });
            quote!(#pattern => { #(#cases)* unreachable!("protocol guards exhaustively checked over the declared finite universe") })
        });
        let invariants = d.invariants.iter().zip(&self.invariants).map(|(i, e)| {
            let name = &i.name;
            let predicate = self.rust_expr(e, &[]);
            quote!(pub fn #name(#state: &State) -> ::core::primitive::bool { #predicate })
        });
        let predicates = d.predicates.iter().zip(&self.predicates).map(|(p, e)| {
            let name = &p.name;
            let predicate = self.rust_expr(e, &[]);
            quote!(pub fn #name(#state: &State) -> ::core::primitive::bool { #predicate })
        });
        let model = self.generate_model(declaration_source);
        let visibility = &d.visibility;
        let name = &d.name;
        quote! {
            #[doc = "Generated by bboxy450-term protocol!: https://github.com/bboxy450/bboxy450-term. Credit copied/adapted/translated reuse accurately; independent inspiration credit requested. Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md"]
            #[allow(non_snake_case, non_camel_case_types, missing_docs)]
            #visibility mod #name {
                #(#domains)*
                #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                pub struct State { #(#field_names: #field_types),* }
                impl ::core::default::Default for State { fn default() -> Self { Self { #(#field_names: #initial),* } } }
                impl State { #(pub fn #field_names(&self) -> #field_types { self.#field_names })* }
                #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                pub enum Event { #(#events),* }
                #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                pub enum Effect { #(#effects),* }
                #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                pub enum RejectReason { #(#rejects),* }
                #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                pub enum IgnoreReason { #(#ignores),* }
                pub type Decision<S = State, E = Effect> = #path::protocol::Decision<S, E, RejectReason, IgnoreReason>;
                #[allow(unused_variables)]
                pub fn step(#state: &State, #event_var: Event) -> Decision<State, Effect> {
                    match #event_var { #(#arms),* }
                }
                #(#invariants)*
                #(#predicates)*
                #path::__yarn_protocol_model! { #model }
            }
        }
    }
}

// A set is a vector of Boolean expressions (one variable per declared member).
// No bit arithmetic, graph tabulation, or backend-specific semantics enter TLC.
enum Lowered {
    Scalar(TokenStream),
    Set(Vec<TokenStream>),
}
impl Lowered {
    fn scalar(self) -> TokenStream {
        match self {
            Self::Scalar(e) => e,
            _ => unreachable!("typed scalar"),
        }
    }
    fn set(self) -> Vec<TokenStream> {
        match self {
            Self::Set(e) => e,
            _ => unreachable!("typed set"),
        }
    }
}
impl Checked {
    fn width(&self, ty: &Ty) -> usize {
        match ty {
            Ty::Set(d) => self.declaration.domains[*d].variants.len(),
            _ => 1,
        }
    }
    fn offsets(&self) -> Vec<usize> {
        let mut offset = 0;
        self.fields
            .iter()
            .map(|ty| {
                let start = offset;
                offset += self.width(ty);
                start
            })
            .collect()
    }
    fn lower_value(&self, ty: &Ty, value: &Value) -> Lowered {
        let path = super::kittens_path();
        match value {
            Value::Scalar(n) if *ty == Ty::Bool => {
                let b = *n != 0;
                Lowered::Scalar(quote!(#path::model::Expr::Bool(#b)))
            }
            Value::Scalar(n) => Lowered::Scalar(quote!(#path::model::Expr::Int(#n))),
            Value::Set(bits) => Lowered::Set(
                bits.iter()
                    .map(|b| quote!(#path::model::Expr::Bool(#b)))
                    .collect(),
            ),
        }
    }
    fn lower(&self, expression: &Expression, parameters: &[Value]) -> Lowered {
        let path = super::kittens_path();
        let e = quote!(#path::model::Expr);
        let recurse = |a| self.lower(a, parameters);
        match &expression.kind {
            Kind::Value(value) => self.lower_value(&expression.ty, value),
            Kind::Parameter(i) => self.lower_value(&expression.ty, &parameters[*i]),
            Kind::Field(i) => {
                let start = self.offsets()[*i];
                match expression.ty {
                    Ty::Bool => Lowered::Scalar(quote!(#e::Var(#start).eq(#e::Int(1)))),
                    Ty::Set(_) => Lowered::Set(
                        (start..start + self.width(&expression.ty))
                            .map(|i| quote!(#e::Var(#i).eq(#e::Int(1))))
                            .collect(),
                    ),
                    _ => Lowered::Scalar(quote!(#e::Var(#start))),
                }
            }
            Kind::Some(a) => recurse(a),
            Kind::IsNone(a) => {
                let a = recurse(a).scalar();
                Lowered::Scalar(quote!((#a).eq(#e::Int(-1))))
            }
            Kind::Not(a) => {
                let a = recurse(a).scalar();
                Lowered::Scalar(quote!((#a).negate()))
            }
            Kind::Binary(op, a, b) => {
                let (a, b) = (recurse(a), recurse(b));
                let expression = match (a, b) {
                    (Lowered::Scalar(a), Lowered::Scalar(b)) => match op {
                        Op::Eq => quote!((#a).eq(#b)),
                        Op::Ne => quote!((#a).eq(#b).negate()),
                        Op::And => quote!((#a).and(#b)),
                        Op::Or => quote!((#a).or(#b)),
                    },
                    (Lowered::Set(a), Lowered::Set(b)) => {
                        let terms = a.iter().zip(&b).map(|(a, b)| quote!((#a).eq(#b)));
                        let mut equal = quote!(#e::Bool(true));
                        for term in terms {
                            equal = quote!((#equal).and(#term));
                        }
                        if matches!(op, Op::Ne) {
                            quote!((#equal).negate())
                        } else {
                            equal
                        }
                    }
                    _ => unreachable!("typed equality"),
                };
                Lowered::Scalar(expression)
            }
            Kind::SetOp(op, a, b) => {
                let bits = recurse(a).set();
                let element = recurse(b).scalar();
                let mut terms = Vec::new();
                for (i, bit) in bits.iter().enumerate() {
                    let i = i as i64;
                    let is_member = quote!((#element).eq(#e::Int(#i)));
                    terms.push(match op {
                        SetOp::Contains => quote!((#is_member).and(#bit)),
                        SetOp::With => quote!((#bit).or(#is_member)),
                        SetOp::Without => quote!((#bit).and((#is_member).negate())),
                    });
                }
                if matches!(op, SetOp::Contains) {
                    let mut contains = quote!(#e::Bool(false));
                    for term in terms {
                        contains = quote!((#contains).or(#term));
                    }
                    Lowered::Scalar(contains)
                } else {
                    Lowered::Set(terms)
                }
            }
        }
    }
    fn value_label(&self, ty: &Ty, value: &Value) -> String {
        match (ty, value) {
            (Ty::Bool, Value::Scalar(n)) => (*n != 0).to_string(),
            (Ty::Domain(d), Value::Scalar(n)) => {
                self.declaration.domains[*d].variants[*n as usize].to_string()
            }
            (Ty::Option(d), Value::Scalar(n)) if *n >= 0 => {
                format!(
                    "Some({})",
                    self.declaration.domains[*d].variants[*n as usize]
                )
            }
            (Ty::Option(_), _) => "None".into(),
            (Ty::Set(d), Value::Set(bits)) => format!(
                "{{{}}}",
                bits.iter()
                    .zip(&self.declaration.domains[*d].variants)
                    .filter(|(b, _)| **b)
                    .map(|(_, v)| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            _ => unreachable!("typed label"),
        }
    }
    fn generate_model(&self, declaration_source: &str) -> TokenStream {
        let path = super::kittens_path();
        let d = &self.declaration;
        let name = d.name.to_string();
        let mut variables = Vec::new();
        let mut encoded = Vec::new();
        for (i, ty) in self.fields.iter().enumerate() {
            let field = &d.fields[i].name;
            let field_name = field.to_string();
            match (ty, &self.initial[i]) {
                (Ty::Set(dom), Value::Set(bits)) => {
                    for (variant, initial) in d.domains[*dom].variants.iter().zip(bits) {
                        let label = format!("{field_name}::{variant}");
                        let n = i64::from(*initial);
                        let domain = &d.domains[*dom].name;
                        variables.push(quote!(#path::model::Variable::new(#label, 0, 1, #n)));
                        encoded.push(quote!(::core::primitive::i64::from(self.#field.contains(#domain::#variant))));
                    }
                }
                (_, Value::Scalar(initial)) => {
                    let min = if matches!(ty, Ty::Option(_)) {
                        -1_i64
                    } else {
                        0
                    };
                    let max = match ty {
                        Ty::Bool => 1_i64,
                        Ty::Domain(dom) | Ty::Option(dom) => {
                            d.domains[*dom].variants.len() as i64 - 1
                        }
                        _ => unreachable!(),
                    };
                    variables.push(
                        quote!(#path::model::Variable::new(#field_name, #min, #max, #initial)),
                    );
                    encoded.push(match ty {
                        Ty::Bool => quote!(::core::primitive::i64::from(self.#field)),
                        Ty::Domain(_) => quote!(::core::primitive::i64::from(#path::protocol::FiniteDomain::ordinal(self.#field))),
                        Ty::Option(_) => quote!(self.#field.map_or(-1_i64, |v| ::core::primitive::i64::from(#path::protocol::FiniteDomain::ordinal(v)))),
                        _ => unreachable!(),
                    });
                }
                _ => unreachable!("typed initial"),
            }
        }
        let mut actions = Vec::new();
        let mut action_events = Vec::new();
        let mut action_cases = Vec::new();
        // Inspired by TLA+ WF_vars/SF_vars (Leslie Lamport):
        // https://lamport.azurewebsites.net/tla/high-level-view.html
        // One case is one disjunction, not independent per-parameter fairness.
        let mut fairness = Vec::new();
        let offsets = self.offsets();
        for arm in &self.arms {
            let mut case_actions = vec![Vec::new(); arm.cases.len()];
            let types: Vec<_> = arm.parameters.iter().map(|(_, t)| t.clone()).collect();
            each_valuation(d, &types, |values| {
                let params = types.iter().zip(values).map(|(ty, v)| self.value_label(ty, v)).collect::<Vec<_>>().join(",");
                let event = &d.events[arm.event].name;
                let label = if types.is_empty() { event.to_string() } else { format!("{event}({params})") };
                let args: Vec<_> = types.iter().zip(values).map(|(ty, v)| self.rust_value(ty, v)).collect();
                for (case_index, case) in arm.cases.iter().enumerate() {
                    let guard = self.lower(&case.guard, values).scalar();
                    let label = format!("{label}::{}", case.name);
                    let mut updates = Vec::new();
                    for (i, expression) in case.assignments.iter().enumerate() {
                        if let Some(expression) = expression {
                            match self.lower(expression, values) {
                                Lowered::Scalar(value) => {
                                    let value = if expression.ty == Ty::Bool { quote!((#value).ite(#path::model::Expr::Int(1), #path::model::Expr::Int(0))) } else { value };
                                    let index = offsets[i];
                                    updates.push(quote!((#index, #value)));
                                }
                                Lowered::Set(bits) => for (j, bit) in bits.iter().enumerate() {
                                    let index = offsets[i] + j;
                                    updates.push(quote!((#index, (#bit).ite(#path::model::Expr::Int(1), #path::model::Expr::Int(0)))));
                                },
                            }
                        }
                    }
                    let index = actions.len();
                    case_actions[case_index].push(index);
                    let event_value = self.message(&format_ident!("Event"), event, &args);
                    action_events.push(quote!(#index => ::core::option::Option::Some(#event_value)));
                    let case_label = format!("{event}::{}", case.name);
                    action_cases.push(quote!(#index => ::core::option::Option::Some(#case_label)));
                    actions.push(quote!(model.actions.push(#path::model::Action::new(#label, #guard, #path::protocol::Effects::from([#(#updates),*])));));
                }
                Ok(())
            }).expect("universe was bounded during validation");
            for (case, indices) in arm.cases.iter().zip(case_actions) {
                if let Some(kind) = case.fairness {
                    let field = match kind {
                        Fairness::Weak => format_ident!("weak_fair"),
                        Fairness::Strong => format_ident!("strong_fair"),
                    };
                    fairness.push(
                        quote!(model.#field.push(#path::protocol::Effects::from([#(#indices),*]));),
                    );
                }
            }
        }
        let invariants = d.invariants.iter().zip(&self.invariants).map(|(invariant, expression)| {
            let id = invariant.id.value();
            let condition = self.lower(expression, &[]).scalar();
            quote!(model.invariants.push(#path::model::Invariant { name: #id.into(), condition: #condition });)
        });
        let predicate = |name: &Ident| {
            d.predicates
                .iter()
                .position(|p| p.name == *name)
                .map(|i| &self.predicates[i])
                .unwrap_or_else(|| {
                    &self.invariants[d
                        .invariants
                        .iter()
                        .position(|p| p.name == *name)
                        .expect("validated predicate")]
                })
        };
        let responses = d.properties.iter().map(|property| {
            let id = property.id.value();
            let request = self.lower(predicate(&property.request), &[]).scalar();
            let response = self.lower(predicate(&property.response), &[]).scalar();
            quote!(model.responses.push(#path::model::Response { name: #id.into(), request: #request, response: #response });)
        });
        let verification_metadata = super::verify::metadata(
            &d.name,
            declaration_source,
            d.domains.iter().map(|d| (&d.name, d.variants.as_slice())),
            d.invariants.iter().map(|i| (&i.id, &i.name)),
            self.arms.iter().flat_map(|a| {
                a.cases
                    .iter()
                    .map(|c| format!("{}::{}", d.events[a.event].name, c.name))
            }),
        );
        let lean = self.generate_lean();
        quote! {
            #verification_metadata
            #lean
            impl State {
                /// Exact finite encoding, in `model().variables` order.
                pub fn model_values(&self) -> #path::protocol::Effects<::core::primitive::i64> {
                    #path::protocol::Effects::from([#(#encoded),*])
                }
            }
            /// Event associated with an action index of this declaration's model.
            pub fn action_event(action: ::core::primitive::usize) -> ::core::option::Option<Event> {
                match action { #(#action_events,)* _ => ::core::option::Option::None }
            }
            /// Parameter-independent Event::case associated with a model action.
            pub fn action_case(action: ::core::primitive::usize) -> ::core::option::Option<&'static str> {
                match action { #(#action_cases,)* _ => ::core::option::Option::None }
            }
            /// Symbolic finite model with only the declaration's named fairness assumptions.
            pub fn model() -> #path::model::Model {
                let mut model = #path::model::Model::new(#name, #path::protocol::Effects::from([#(#variables),*]));
                model.check_deadlock = false;
                #(#actions)*
                #(#invariants)*
                #(#fairness)*
                #(#responses)*
                model
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_syntax_and_resource_limits_fail_closed() {
        let e = expand(
            quote!(mod Large { domain D { A,B,C,D,E,F,G,H,I,J,K,L,M,N,O,P,Q } state { x: bool = false; } events { E(s: Set<D>); } on E(s) { case all when true => { preserve x; } } }),
        );
        assert!(e.to_string().contains("PRO010"));
        let e = expand(
            quote!(mod Invalid { state { x: bool = false; } events { E; } on E { case all when true => { set x = arbitrary(); } } }),
        );
        assert!(e.to_string().contains("PRO001"));
        assert!(e.to_string().contains("Repair:"));
    }
}
