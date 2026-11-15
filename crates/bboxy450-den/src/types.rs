// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use std::collections::{BTreeMap, BTreeSet};

use crate::Diagnostic;
use crate::ast::*;

fn diagnostic(
    spec: &Spec,
    pos: Pos,
    code: &'static str,
    message: impl Into<String>,
    repair: &str,
) -> Diagnostic {
    pos.diagnostic(code, &spec.name.text, message, repair)
}

fn duplicates<'a>(
    spec: &Spec,
    names: impl IntoIterator<Item = &'a Name>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeSet::new();
    for name in names {
        if !seen.insert(&name.text) {
            diagnostics.push(diagnostic(
                spec,
                name.pos,
                "X202",
                format!("{} is declared twice in this scope", name.text),
                "give each declaration a unique name",
            ));
        }
    }
}

fn reserved<'a>(
    spec: &Spec,
    names: impl IntoIterator<Item = &'a Name>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for name in names {
        if matches!(name.text.as_str(), "Bool" | "Int" | "Set") {
            diagnostics.push(diagnostic(
                spec,
                name.pos,
                "X202",
                format!("{} is a built-in declaration", name.text),
                "choose a name other than Bool, Int or Set",
            ));
        }
    }
}

struct Names<'a> {
    spec: &'a Spec,
    declared: BTreeMap<String, Pos>,
    used: BTreeSet<String>,
    diagnostics: Vec<Diagnostic>,
    unused: Vec<Diagnostic>,
}
impl Names<'_> {
    fn use_name(&mut self, text: &str, pos: Pos, locals: &[String]) {
        if locals.iter().any(|n| n == text) {
            return;
        }
        if self.declared.contains_key(text) {
            self.used.insert(text.into());
        } else {
            self.diagnostics.push(diagnostic(
                self.spec,
                pos,
                "X201",
                format!("{text} is not declared"),
                "declare the name or correct its spelling",
            ));
        }
    }
    fn ty(&mut self, ty: &Type) {
        if let Ty::Named(name) | Ty::Set(name) = &ty.kind {
            self.use_name(name, ty.pos, &[]);
        }
    }
    fn expr(&mut self, expr: &Expr, locals: &[String]) {
        match &expr.kind {
            ExprKind::Bool(_) | ExprKind::Int(_) => {}
            ExprKind::Name(name) => self.use_name(name, expr.pos, locals),
            ExprKind::Member(domain, member) => {
                self.use_name(domain, expr.pos, &[]);
                if let Some(enumeration) = self.spec.enums.iter().find(|e| e.name.text == *domain)
                    && !enumeration.members.iter().any(|m| m.text == *member)
                {
                    self.diagnostics.push(diagnostic(
                        self.spec,
                        expr.pos,
                        "X201",
                        format!("{domain}.{member} is not declared"),
                        "use a member of the declared enum",
                    ));
                }
            }
            ExprKind::Call(name, args) => {
                if self.spec.tables.iter().any(|t| t.name.text == *name) {
                    // A table call with one argument was expanded by the parser.
                    self.diagnostics.push(diagnostic(
                        self.spec,
                        expr.pos,
                        "X303",
                        format!("table {name} takes one argument, its key"),
                        "pass one value of the table's key enum",
                    ));
                } else if name != "Set" {
                    self.use_name(name, expr.pos, locals);
                }
                for arg in args {
                    self.expr(arg, locals);
                }
            }
            ExprKind::Not(expr) => self.expr(expr, locals),
            ExprKind::Binary(_, left, right) => {
                self.expr(left, locals);
                self.expr(right, locals);
            }
            ExprKind::If(condition, yes, no) => {
                self.expr(condition, locals);
                self.expr(yes, locals);
                self.expr(no, locals);
            }
            ExprKind::Quant {
                binding,
                domain,
                body,
                ..
            } => {
                self.use_name(&domain.text, domain.pos, &[]);
                reserved(self.spec, [binding], &mut self.diagnostics);
                let mut scope = locals.to_vec();
                scope.push(binding.text.clone());
                self.expr(body, &scope);
                let mut used = BTreeSet::new();
                referenced_names(body, &mut used);
                if !used.contains(&binding.text) {
                    self.unused.push(diagnostic(
                        self.spec,
                        binding.pos,
                        "X203",
                        format!("{} is quantified but never used", binding.text),
                        "use the quantified binding or remove the quantifier",
                    ));
                }
            }
        }
    }
    fn call(&mut self, call: &Call, locals: &[String]) {
        self.use_name(&call.name.text, call.name.pos, locals);
        for arg in &call.args {
            self.expr(arg, locals);
        }
    }
}

/// True for a spec with enums and no machine: no state, event or handler.
pub(crate) fn is_vocabulary(spec: &Spec) -> bool {
    !spec.enums.is_empty()
        && spec.fields.is_empty()
        && spec.events.is_empty()
        && spec.handlers.is_empty()
}
/// Checks declarations and references before any typing or execution.
pub(crate) fn names(spec: &Spec) -> Vec<Diagnostic> {
    let mut errors = vec![];
    let module = |path: &Vec<Name>| {
        path.iter()
            .map(|n| n.text.as_str())
            .collect::<Vec<_>>()
            .join(".")
    };
    // The enums that came through an import, by the import's module.
    let brought = |path: &Vec<Name>| {
        let module = module(path);
        spec.enums
            .iter()
            .filter(move |e| e.from.as_deref() == Some(module.as_str()))
    };
    // An import whose spec is named like its enum (`specs.Element`) is one name.
    let imported = spec
        .imports
        .iter()
        .filter(|path| !brought(path).any(|e| Some(&e.name.text) == path.last().map(|n| &n.text)))
        .filter_map(|path| path.last());
    let globals: Vec<_> = spec
        .enums
        .iter()
        .map(|e| &e.name)
        .chain(spec.fields.iter().map(|f| &f.name))
        .chain(spec.events.iter().map(|s| &s.name))
        .chain(spec.effects.iter().map(|s| &s.name))
        .chain(spec.invariants.iter().map(|c| &c.name))
        .chain(spec.properties.iter().map(|p| &p.name))
        .chain(spec.assumptions.iter().map(|a| &a.name))
        .chain(spec.opens.iter().map(|a| &a.name))
        .chain(spec.defines.iter().map(|d| &d.name))
        .chain(spec.tables.iter().map(|t| &t.name))
        .chain(imported)
        .collect();
    // A table is a total function on its key enum.
    for table in &spec.tables {
        let enumeration = |name: &str| spec.enums.iter().find(|e| e.name.text == name);
        if let Ty::Named(domain) | Ty::Set(domain) = &table.value.kind
            && enumeration(domain).is_none()
        {
            errors.push(diagnostic(
                spec,
                table.value.pos,
                "X201",
                format!("{domain} is not declared"),
                "declare the enum or correct its spelling",
            ));
        }
        let Some(key) = enumeration(&table.key.text) else {
            errors.push(diagnostic(
                spec,
                table.key.pos,
                "X201",
                format!("{} is not declared", table.key.text),
                "a table's key is a declared enum",
            ));
            continue;
        };
        duplicates(
            spec,
            table.entries.iter().map(|(member, _)| member),
            &mut errors,
        );
        for (member, _) in &table.entries {
            if !key.members.iter().any(|m| m.text == member.text) {
                errors.push(diagnostic(
                    spec,
                    member.pos,
                    "X201",
                    format!("{}.{} is not declared", key.name.text, member.text),
                    "use a member of the table's key enum",
                ));
            }
        }
        for member in &key.members {
            if !table.entries.iter().any(|(m, _)| m.text == member.text) {
                errors.push(diagnostic(
                    spec,
                    table.name.pos,
                    "X402",
                    format!(
                        "table {} has no entry for {}.{}",
                        table.name.text, key.name.text, member.text
                    ),
                    "add an entry for every member of the key enum",
                ));
            }
        }
    }
    // A source names a declaration, or a case as `Event.case`.
    for source in &spec.sources {
        let first = &source.target[0];
        let known = match source.target.get(1) {
            None => globals.iter().any(|name| name.text == first.text),
            Some(case) => spec.handlers.iter().any(|handler| {
                handler.event.text == first.text
                    && handler.cases.iter().any(|c| c.name.text == case.text)
            }),
        };
        if !known {
            let target: Vec<_> = source.target.iter().map(|n| n.text.as_str()).collect();
            errors.push(diagnostic(
                spec,
                first.pos,
                "X201",
                format!("source names {}, which is not declared", target.join(".")),
                "name a declaration of this spec, or a case as Event.case",
            ));
        }
    }
    duplicates(spec, globals.iter().copied(), &mut errors);
    reserved(spec, globals, &mut errors);
    for enumeration in &spec.enums {
        duplicates(spec, &enumeration.members, &mut errors);
    }
    for signature in spec.events.iter().chain(&spec.effects) {
        duplicates(spec, signature.params.iter().map(|p| &p.name), &mut errors);
        reserved(spec, signature.params.iter().map(|p| &p.name), &mut errors);
    }
    duplicates(spec, spec.handlers.iter().map(|h| &h.event), &mut errors);
    for handler in &spec.handlers {
        duplicates(spec, &handler.bindings, &mut errors);
        reserved(spec, &handler.bindings, &mut errors);
        duplicates(spec, handler.cases.iter().map(|c| &c.name), &mut errors);
    }
    let mut scenario_names = BTreeSet::new();
    for scenario in &spec.scenarios {
        if !scenario_names.insert(&scenario.name) {
            errors.push(diagnostic(
                spec,
                scenario.pos,
                "X202",
                "a scenario name is declared twice",
                "give every scenario a unique name",
            ));
        }
    }
    if !errors.is_empty() {
        return errors;
    }
    for event in &spec.events {
        if !spec
            .handlers
            .iter()
            .any(|h| h.event.text == event.name.text)
        {
            errors.push(diagnostic(
                spec,
                event.name.pos,
                "X204",
                format!("event {} has no on block", event.name.text),
                "add an on block for the event",
            ));
        }
    }
    for handler in &spec.handlers {
        if !spec
            .events
            .iter()
            .any(|s| s.name.text == handler.event.text)
        {
            errors.push(diagnostic(
                spec,
                handler.event.pos,
                "X204",
                format!("on {} names no event", handler.event.text),
                "declare the event or correct the on block",
            ));
        }
    }
    if !errors.is_empty() {
        return errors;
    }

    let declared: BTreeMap<_, _> = spec
        .enums
        .iter()
        .map(|e| &e.name)
        .chain(spec.fields.iter().map(|f| &f.name))
        .chain(spec.events.iter().map(|s| &s.name))
        .chain(spec.effects.iter().map(|s| &s.name))
        .chain(spec.imports.iter().filter_map(|path| path.last()))
        .map(|n| (n.text.clone(), n.pos))
        .collect();
    let mut check = Names {
        spec,
        declared,
        used: BTreeSet::new(),
        diagnostics: vec![],
        unused: vec![],
    };
    // A define is checked where it is written; its uses are then its copy.
    for define in &spec.defines {
        check.expr(&define.expr, &[]);
        // A later define repeats this one's expression: report it once.
        if !check.diagnostics.is_empty() {
            return check.diagnostics;
        }
    }
    for (_, value) in spec.tables.iter().flat_map(|t| &t.entries) {
        check.expr(value, &[]);
    }
    if !check.diagnostics.is_empty() {
        return check.diagnostics;
    }
    for field in &spec.fields {
        check.ty(&field.ty);
        check.expr(&field.initial, &[]);
    }
    for signature in spec.events.iter().chain(&spec.effects) {
        for param in &signature.params {
            check.ty(&param.ty);
        }
    }
    for handler in &spec.handlers {
        check.use_name(&handler.event.text, handler.event.pos, &[]);
        let locals: Vec<_> = handler.bindings.iter().map(|n| n.text.clone()).collect();
        for case in &handler.cases {
            if let Some(guard) = &case.guard {
                check.expr(guard, &locals);
            }
            for stmt in &case.stmts {
                match &stmt.kind {
                    StmtKind::Assign(name, _, expr) => {
                        check.use_name(&name.text, name.pos, &[]);
                        check.expr(expr, &locals);
                    }
                    StmtKind::Emit(call) => check.call(call, &locals),
                }
            }
        }
    }
    for invariant in &spec.invariants {
        check.expr(&invariant.expr, &[]);
    }
    for property in &spec.properties {
        let mut locals = Vec::new();
        let mut used = BTreeSet::new();
        referenced_names(&property.request, &mut used);
        referenced_names(&property.response, &mut used);
        for (binding, domain) in &property.binders {
            check.use_name(&domain.text, domain.pos, &[]);
            reserved(spec, [binding], &mut check.diagnostics);
            if locals.contains(&binding.text) {
                check.diagnostics.push(diagnostic(
                    spec,
                    binding.pos,
                    "X202",
                    format!("{} is bound twice", binding.text),
                    "give every binder of the property a unique name",
                ));
            }
            if !used.contains(&binding.text) {
                check.unused.push(diagnostic(
                    spec,
                    binding.pos,
                    "X203",
                    format!("{} is quantified but never used", binding.text),
                    "use the quantified binding or remove the quantifier",
                ));
            }
            locals.push(binding.text.clone());
        }
        check.expr(&property.request, &locals);
        check.expr(&property.response, &locals);
    }
    for scenario in &spec.scenarios {
        for step in &scenario.steps {
            match step {
                ScenarioStep::Call { call, effects, .. } => {
                    check.call(call, &[]);
                    for effect in effects {
                        check.call(effect, &[]);
                    }
                }
                ScenarioStep::Expect(expr) => check.expr(expr, &[]),
            }
        }
    }
    if !check.diagnostics.is_empty() {
        return check.diagnostics;
    }
    for name in spec
        .defines
        .iter()
        .map(|d| &d.name)
        .chain(spec.tables.iter().map(|t| &t.name))
    {
        let what = [
            format!("define {}", name.text),
            format!("table {}", name.text),
        ];
        if !spec.expansions.iter().any(|(_, used)| what.contains(used)) {
            check.diagnostics.push(diagnostic(
                spec,
                name.pos,
                "X203",
                format!("{} is declared but never used", name.text),
                "use the declaration or remove it",
            ));
        }
    }
    // An imported enum is used through its import, and the enums of a
    // vocabulary spec are there to be imported.
    let exported = |name: &String| {
        spec.enums
            .iter()
            .any(|e| e.name.text == *name && (e.from.is_some() || is_vocabulary(spec)))
    };
    let import_used = |name: &String| {
        spec.imports
            .iter()
            .filter(|path| path.last().is_some_and(|n| n.text == *name))
            .any(|path| brought(path).any(|e| check.used.contains(&e.name.text)))
    };
    for (name, pos) in &check.declared {
        if !check.used.contains(name) && !exported(name) && !import_used(name) {
            check.diagnostics.push(diagnostic(
                spec,
                *pos,
                "X203",
                format!("{name} is declared but never used"),
                "use the declaration or remove it",
            ));
        }
    }
    // Formal parameter names describe the call signature; on bindings are the
    // actual local declarations whose uses matter.
    for handler in &spec.handlers {
        if spec
            .events
            .iter()
            .find(|event| event.name.text == handler.event.text)
            .is_some_and(|event| event.params.len() != handler.bindings.len())
        {
            // Arity belongs to X303, rather than derivative unused bindings.
            continue;
        }
        let mut used = BTreeSet::new();
        for case in &handler.cases {
            if let Some(guard) = &case.guard {
                referenced_names(guard, &mut used);
            }
            for stmt in &case.stmts {
                match &stmt.kind {
                    StmtKind::Assign(_, _, expr) => referenced_names(expr, &mut used),
                    StmtKind::Emit(call) => {
                        for arg in &call.args {
                            referenced_names(arg, &mut used);
                        }
                    }
                }
            }
        }
        for binding in &handler.bindings {
            if !used.contains(&binding.text) {
                check.diagnostics.push(diagnostic(
                    spec,
                    binding.pos,
                    "X203",
                    format!("{} is bound but never used", binding.text),
                    "use the binding or remove the event parameter",
                ));
            }
        }
    }
    check.diagnostics.extend(check.unused);
    check.diagnostics
}

fn referenced_names(expr: &Expr, names: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Name(name) => {
            names.insert(name.clone());
        }
        ExprKind::Call(name, args) => {
            names.insert(name.clone());
            for arg in args {
                referenced_names(arg, names);
            }
        }
        ExprKind::Not(expr) => referenced_names(expr, names),
        ExprKind::Binary(_, left, right) => {
            referenced_names(left, names);
            referenced_names(right, names);
        }
        ExprKind::If(condition, yes, no) => {
            referenced_names(condition, names);
            referenced_names(yes, names);
            referenced_names(no, names);
        }
        ExprKind::Quant { binding, body, .. } => {
            let mut nested = BTreeSet::new();
            referenced_names(body, &mut nested);
            nested.remove(&binding.text);
            names.extend(nested);
        }
        ExprKind::Bool(_) | ExprKind::Int(_) | ExprKind::Member(_, _) => {}
    }
}

fn compatible(left: &Ty, right: &Ty) -> bool {
    matches!((left, right), (Ty::Int(..), Ty::Int(..))) || left == right
}
fn type_error(spec: &Spec, pos: Pos, message: impl Into<String>) -> Diagnostic {
    diagnostic(
        spec,
        pos,
        "X301",
        message,
        "use a value of the declared type",
    )
}
/// A type as it is written: `Bool`, `Int[0..3]`, `Gate`, `Set[Msg]`.
pub(crate) fn show_ty(ty: &Ty) -> String {
    match ty {
        Ty::Bool => "Bool".into(),
        Ty::Int(i64::MIN, i64::MAX) => "Int".into(),
        Ty::Int(lo, hi) if lo == hi => format!("the integer {lo}"),
        Ty::Int(lo, hi) => format!("Int[{lo}..{hi}]"),
        Ty::Named(name) => name.clone(),
        Ty::Set(name) => format!("Set[{name}]"),
    }
}
fn require(spec: &Spec, pos: Pos, actual: &Ty, expected: &Ty) -> Result<(), Diagnostic> {
    if compatible(actual, expected) {
        Ok(())
    } else {
        Err(type_error(
            spec,
            pos,
            format!("expected {}, found {}", show_ty(expected), show_ty(actual)),
        ))
    }
}
fn local_type(spec: &Spec, name: &str, bindings: &[(String, Ty)]) -> Option<Ty> {
    bindings
        .iter()
        .rev()
        .find(|(n, _)| n == name)
        .map(|(_, ty)| ty.clone())
        .or_else(|| {
            spec.fields
                .iter()
                .find(|f| f.name.text == name)
                .map(|f| f.ty.kind.clone())
        })
}

pub(crate) fn expr_type(
    spec: &Spec,
    expr: &Expr,
    bindings: &[(String, Ty)],
    expected: Option<&Ty>,
) -> Result<Ty, Diagnostic> {
    let infer = |expr: &Expr, expected: Option<&Ty>| expr_type(spec, expr, bindings, expected);
    let result = match &expr.kind {
        ExprKind::Bool(_) => Ty::Bool,
        ExprKind::Int(value) => Ty::Int(*value, *value),
        ExprKind::Name(name) => local_type(spec, name, bindings).ok_or_else(|| {
            type_error(
                spec,
                expr.pos,
                format!("{name} is not a state field or local value"),
            )
        })?,
        ExprKind::Member(domain, _) => {
            if !spec.enums.iter().any(|e| e.name.text == *domain) {
                return Err(type_error(
                    spec,
                    expr.pos,
                    format!("{domain} is not an enum"),
                ));
            }
            Ty::Named(domain.clone())
        }
        ExprKind::Call(name, args) if name == "Set" => {
            let domain = match expected {
                Some(Ty::Set(domain)) => domain.clone(),
                _ if !args.is_empty() => match infer(&args[0], None)? {
                    Ty::Named(domain) => domain,
                    _ => {
                        return Err(type_error(
                            spec,
                            expr.pos,
                            "a Set constructor needs enum members",
                        ));
                    }
                },
                _ => {
                    return Err(type_error(
                        spec,
                        expr.pos,
                        "Set() needs a declared set type",
                    ));
                }
            };
            for arg in args {
                let actual = infer(arg, Some(&Ty::Named(domain.clone())))?;
                require(spec, arg.pos, &actual, &Ty::Named(domain.clone()))?;
            }
            Ty::Set(domain)
        }
        ExprKind::Call(name, args) => {
            let Some(Ty::Set(domain)) = local_type(spec, name, bindings) else {
                return Err(type_error(
                    spec,
                    expr.pos,
                    format!("{name} is not a set membership function"),
                ));
            };
            if args.len() != 1 {
                return Err(diagnostic(
                    spec,
                    expr.pos,
                    "X303",
                    format!("{name} membership requires one argument"),
                    "pass one member of the set's enum",
                ));
            }
            let actual = infer(&args[0], None)?;
            if !compatible(&actual, &Ty::Named(domain)) {
                return Err(diagnostic(
                    spec,
                    args[0].pos,
                    "X303",
                    "a membership argument has the wrong enum type",
                    "pass one member of the set's enum",
                ));
            }
            Ty::Bool
        }
        ExprKind::Not(value) => {
            require(spec, value.pos, &infer(value, Some(&Ty::Bool))?, &Ty::Bool)?;
            Ty::Bool
        }
        ExprKind::Binary(op, left, right) => {
            match op.as_str() {
                "&&" | "||" | "==>" => {
                    require(spec, left.pos, &infer(left, Some(&Ty::Bool))?, &Ty::Bool)?;
                    require(spec, right.pos, &infer(right, Some(&Ty::Bool))?, &Ty::Bool)?;
                    Ty::Bool
                }
                "+" | "-" | "<" | "<=" | ">" | ">=" => {
                    let integer = Ty::Int(i64::MIN, i64::MAX);
                    require(spec, left.pos, &infer(left, Some(&integer))?, &integer)?;
                    require(spec, right.pos, &infer(right, Some(&integer))?, &integer)?;
                    if matches!(op.as_str(), "+" | "-") {
                        integer
                    } else {
                        Ty::Bool
                    }
                }
                "==" | "!=" => {
                    // The declared side supplies the element type of Set().
                    let (left_ty, right_ty) = if matches!(&left.kind, ExprKind::Call(n,args) if n == "Set" && args.is_empty())
                    {
                        let right_ty = infer(right, None)?;
                        (infer(left, Some(&right_ty))?, right_ty)
                    } else {
                        let left_ty = infer(left, None)?;
                        let right_ty = infer(right, Some(&left_ty))?;
                        (left_ty, right_ty)
                    };
                    require(spec, right.pos, &right_ty, &left_ty)?;
                    Ty::Bool
                }
                _ => {
                    return Err(type_error(
                        spec,
                        expr.pos,
                        format!("operator {op} is not supported"),
                    ));
                }
            }
        }
        ExprKind::If(condition, yes, no) => {
            require(
                spec,
                condition.pos,
                &infer(condition, Some(&Ty::Bool))?,
                &Ty::Bool,
            )?;
            // If both branches are empty sets their surrounding context is required.
            let yes_ty = match infer(yes, expected) {
                Ok(ty) => ty,
                Err(_) if expected.is_none() => {
                    let no_ty = infer(no, None)?;
                    infer(yes, Some(&no_ty))?
                }
                Err(error) => return Err(error),
            };
            let no_ty = infer(no, Some(&yes_ty))?;
            require(spec, no.pos, &no_ty, &yes_ty)?;
            yes_ty
        }
        ExprKind::Quant {
            binding,
            domain,
            body,
            ..
        } => {
            if !spec.enums.iter().any(|e| e.name.text == domain.text) {
                return Err(type_error(
                    spec,
                    domain.pos,
                    "a quantifier domain must be an enum",
                ));
            }
            let mut scope = bindings.to_vec();
            scope.push((binding.text.clone(), Ty::Named(domain.text.clone())));
            let body_ty = expr_type(spec, body, &scope, Some(&Ty::Bool))?;
            require(spec, body.pos, &body_ty, &Ty::Bool)?;
            Ty::Bool
        }
    };
    if let Some(expected) = expected {
        require(spec, expr.pos, &result, expected)?;
    }
    Ok(result)
}

fn check_call(
    spec: &Spec,
    call: &Call,
    signatures: &[Signature],
    bindings: &[(String, Ty)],
) -> Result<(), Diagnostic> {
    let Some(signature) = signatures.iter().find(|s| s.name.text == call.name.text) else {
        return Err(type_error(
            spec,
            call.name.pos,
            format!("{} is not a call of the required kind", call.name.text),
        ));
    };
    if signature.params.len() != call.args.len() {
        return Err(diagnostic(
            spec,
            call.name.pos,
            "X303",
            format!(
                "{} expects {} arguments, found {}",
                call.name.text,
                signature.params.len(),
                call.args.len()
            ),
            "pass exactly the declared arguments",
        ));
    }
    for (arg, param) in call.args.iter().zip(&signature.params) {
        let actual = expr_type(spec, arg, bindings, Some(&param.ty.kind));
        let valid_type = actual.is_ok();
        let valid_bound = !valid_type
            || !is_closed(arg, &[])
            || match &param.ty.kind {
                Ty::Int(..) => crate::eval::eval(spec, arg, &vec![], &vec![], Some(&param.ty.kind))
                    .is_ok_and(|value| crate::eval::in_range(spec, &value, &param.ty.kind)),
                _ => true,
            };
        if !valid_type || !valid_bound {
            return Err(diagnostic(
                spec,
                arg.pos,
                "X303",
                format!(
                    "argument {} of {} has the wrong type",
                    param.name.text, call.name.text
                ),
                "pass a value of the parameter's declared type",
            ));
        }
    }
    Ok(())
}

fn is_closed(expr: &Expr, bindings: &[String]) -> bool {
    match &expr.kind {
        ExprKind::Name(name) => bindings.contains(name),
        ExprKind::Call(name, args) => {
            name == "Set" && args.iter().all(|arg| is_closed(arg, bindings))
        }
        ExprKind::Not(value) => is_closed(value, bindings),
        ExprKind::Binary(_, left, right) => is_closed(left, bindings) && is_closed(right, bindings),
        ExprKind::If(condition, yes, no) => {
            is_closed(condition, bindings) && is_closed(yes, bindings) && is_closed(no, bindings)
        }
        ExprKind::Quant { binding, body, .. } => {
            let mut scope = bindings.to_vec();
            scope.push(binding.text.clone());
            is_closed(body, &scope)
        }
        ExprKind::Bool(_) | ExprKind::Int(_) | ExprKind::Member(_, _) => true,
    }
}

/// Checks types after all names resolve. Bound checks are separate from integer typing.
pub(crate) fn types(spec: &Spec) -> Vec<Diagnostic> {
    let mut errors = vec![];
    // Defines and tables are typed where they are written, before their uses.
    for define in &spec.defines {
        match expr_type(spec, &define.expr, &[], None) {
            Ok(Ty::Bool | Ty::Int(..)) => {}
            Ok(other) => errors.push(type_error(
                spec,
                define.expr.pos,
                format!(
                    "define {} is {}; a define is a Bool or an Int",
                    define.name.text,
                    show_ty(&other)
                ),
            )),
            Err(error) => errors.push(error),
        }
        if !errors.is_empty() {
            return errors;
        }
    }
    for table in &spec.tables {
        if matches!(table.value.kind, Ty::Set(_)) {
            errors.push(type_error(
                spec,
                table.value.pos,
                "a table maps to Bool, a bounded Int or an enum",
            ));
            continue;
        }
        for (_, value) in &table.entries {
            match expr_type(spec, value, &[], Some(&table.value.kind)) {
                Err(error) => errors.push(error),
                Ok(actual) => {
                    if let Err(error) = require(spec, value.pos, &actual, &table.value.kind) {
                        errors.push(error);
                    } else if let (Ty::Int(lo, hi), ExprKind::Int(n)) =
                        (&table.value.kind, &value.kind)
                        && !(lo..=hi).contains(&n)
                    {
                        errors.push(diagnostic(
                            spec,
                            value.pos,
                            "X302",
                            format!("{n} is outside {}", show_ty(&table.value.kind)),
                            "use a value within the table's declared range",
                        ));
                    }
                }
            }
        }
    }
    if !errors.is_empty() {
        return errors;
    }
    for enumeration in &spec.enums {
        if enumeration.members.is_empty() {
            errors.push(type_error(
                spec,
                enumeration.name.pos,
                "an enum must have at least one member",
            ));
        }
    }
    for ty in spec.fields.iter().map(|f| &f.ty).chain(
        spec.events
            .iter()
            .chain(&spec.effects)
            .flat_map(|s| s.params.iter().map(|p| &p.ty)),
    ) {
        match &ty.kind {
            Ty::Named(name) | Ty::Set(name) if !spec.enums.iter().any(|e| e.name.text == *name) => {
                errors.push(type_error(
                    spec,
                    ty.pos,
                    format!("{name} is not an enum type"),
                ))
            }
            Ty::Int(lo, hi) if lo > hi => errors.push(type_error(
                spec,
                ty.pos,
                "an integer range has its lower bound above its upper bound",
            )),
            _ => {}
        }
    }
    for field in &spec.fields {
        if !is_closed(&field.initial, &[]) {
            errors.push(type_error(
                spec,
                field.initial.pos,
                "an initial value must be a closed expression",
            ));
        } else if let Err(error) = expr_type(spec, &field.initial, &[], Some(&field.ty.kind)) {
            errors.push(error);
        }
    }
    for handler in &spec.handlers {
        let signature = spec
            .events
            .iter()
            .find(|s| s.name.text == handler.event.text)
            .expect("names paired handlers");
        if signature.params.len() != handler.bindings.len() {
            errors.push(diagnostic(
                spec,
                handler.event.pos,
                "X303",
                format!(
                    "on {} binds {} arguments, expected {}",
                    handler.event.text,
                    handler.bindings.len(),
                    signature.params.len()
                ),
                "bind exactly the event's declared arguments",
            ));
            continue;
        }
        let bindings: Vec<_> = handler
            .bindings
            .iter()
            .zip(&signature.params)
            .map(|(name, param)| (name.text.clone(), param.ty.kind.clone()))
            .collect();
        for case in &handler.cases {
            if let Some(guard) = &case.guard
                && let Err(error) = expr_type(spec, guard, &bindings, Some(&Ty::Bool))
            {
                errors.push(error);
            }
            for stmt in &case.stmts {
                let result = match &stmt.kind {
                    StmtKind::Assign(name, operator, expr) => {
                        if let Some(field) = spec.fields.iter().find(|f| f.name.text == name.text) {
                            let expected = if operator == "=" {
                                Some(field.ty.kind.clone())
                            } else if let Ty::Set(domain) = &field.ty.kind {
                                Some(Ty::Named(domain.clone()))
                            } else {
                                None
                            };
                            match expected {
                                Some(expected) => {
                                    expr_type(spec, expr, &bindings, Some(&expected)).map(|_| ())
                                }
                                None => Err(type_error(
                                    spec,
                                    stmt.pos,
                                    "only a set field supports += and -=",
                                )),
                            }
                        } else {
                            Err(type_error(
                                spec,
                                name.pos,
                                "only state fields may be assigned",
                            ))
                        }
                    }
                    StmtKind::Emit(call) => check_call(spec, call, &spec.effects, &bindings),
                };
                if let Err(error) = result {
                    errors.push(error);
                }
            }
        }
    }
    for invariant in &spec.invariants {
        if let Err(error) = expr_type(spec, &invariant.expr, &[], Some(&Ty::Bool)) {
            errors.push(error);
        }
    }
    for property in &spec.properties {
        let mut bindings = Vec::new();
        for (binding, domain) in &property.binders {
            bindings.push((binding.text.clone(), Ty::Named(domain.text.clone())));
        }
        if let Some((_, domain)) = property
            .binders
            .iter()
            .find(|(_, d)| !spec.enums.iter().any(|e| e.name.text == d.text))
        {
            errors.push(type_error(
                spec,
                domain.pos,
                format!("{} is not an enum type", domain.text),
            ));
            continue;
        }
        for side in [&property.request, &property.response] {
            if let Err(error) = expr_type(spec, side, &bindings, Some(&Ty::Bool)) {
                errors.push(error);
            }
        }
    }
    for scenario in &spec.scenarios {
        for step in &scenario.steps {
            match step {
                ScenarioStep::Call { call, effects, .. } => {
                    if let Err(error) = check_call(spec, call, &spec.events, &[]) {
                        errors.push(error);
                    }
                    for effect in effects {
                        if let Err(error) = check_call(spec, effect, &spec.effects, &[]) {
                            errors.push(error);
                        }
                    }
                }
                ScenarioStep::Expect(expr) => {
                    if let Err(error) = expr_type(spec, expr, &[], Some(&Ty::Bool)) {
                        errors.push(error);
                    }
                }
            }
        }
    }
    if !errors.is_empty() {
        return errors;
    }
    for field in &spec.fields {
        let initial =
            crate::eval::eval(spec, &field.initial, &vec![], &vec![], Some(&field.ty.kind));
        if !matches!(initial, Ok(ref value) if crate::eval::in_range(spec, value, &field.ty.kind)) {
            errors.push(diagnostic(
                spec,
                field.initial.pos,
                "X302",
                format!(
                    "initial value of {} is outside its declared type",
                    field.name.text
                ),
                "put the initial value inside its declared range",
            ));
        }
    }
    errors
}
