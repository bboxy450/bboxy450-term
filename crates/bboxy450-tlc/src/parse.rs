// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Read a shipped `<Name>.tla`/`<Name>.cfg` pair back into a [`Model`].
//!
//! This accepts exactly what [`Model::tla`] and [`Model::config`] export, and
//! nothing else: a parsed model must export the same two texts again, so a
//! hand-edited file outside the native subset is refused rather than guessed
//! at. Variable names are not exported; they come back as `v0`, `v1`, …
use crate::{Action, Expr, Invariant, Model, Response, Variable};
use std::format;
use std::string::{String, ToString};
use std::vec::Vec;

/// Parse an exported model and its configuration. The result exports the
/// same `tla` and `cfg` texts (checked), so it is the model those files
/// describe; anything outside the exporter's own form is an error.
pub fn parse(tla: &str, cfg: &str) -> Result<Model, String> {
    let mut lines = tla
        .lines()
        .filter(|line| !line.starts_with("\\*"))
        .peekable();
    let mut next = |what: &str| lines.next().ok_or_else(|| format!("missing {what}"));
    let name = next("MODULE")?
        .strip_prefix("---- MODULE ")
        .and_then(|rest| rest.strip_suffix(" ----"))
        .ok_or("expected `---- MODULE <Name> ----`")?
        .to_string();
    expect(next("EXTENDS")?, "EXTENDS Integers")?;
    let count = next("VARIABLES")?
        .strip_prefix("VARIABLES ")
        .ok_or("expected VARIABLES")?
        .split(", ")
        .count();
    next("vars")?;
    expect(next("TypeOK")?, "TypeOK ==")?;
    let mut variables = Vec::new();
    for i in 0..count {
        let line = next("TypeOK row")?;
        let range = line
            .strip_prefix(&format!("  /\\ v{i} \\in "))
            .ok_or_else(|| format!("expected the domain of v{i}"))?;
        let (lo, hi) = range.split_once("..").ok_or("expected lo..hi")?;
        variables.push(Variable::new(
            &format!("v{i}"),
            literal(lo)?,
            literal(hi)?,
            0,
        ));
    }
    expect(next("Init")?, "Init ==")?;
    for (i, variable) in variables.iter_mut().enumerate() {
        let line = next("Init row")?;
        if let Some(value) = line.strip_prefix(&format!("  /\\ v{i} = ")) {
            variable.initial = literal(value)?;
        } else if let Some(set) = line
            .strip_prefix(&format!("  /\\ v{i} \\in {{"))
            .and_then(|s| s.strip_suffix('}'))
        {
            let values = set
                .split(", ")
                .map(literal)
                .collect::<Result<Vec<_>, _>>()?;
            *variable = Variable::choice(&variable.name, variable.min, variable.max, &values);
        } else {
            return Err(format!("expected the initial value of v{i}"));
        }
    }
    let mut actions = Vec::new();
    loop {
        let line = next("Next")?;
        if let Some(rest) = line.strip_prefix("Next == ") {
            if rest != "FALSE" && rest.split(" \\/ ").count() != actions.len() {
                return Err("Next must name every action".into());
            }
            break;
        }
        if line != format!("A{} ==", actions.len()) {
            return Err(format!("expected A{} ==, got {line}", actions.len()));
        }
        let guard = expression(
            next("guard")?
                .strip_prefix("  /\\ ")
                .ok_or("expected a guard")?,
        )?;
        let mut updates = Vec::new();
        for j in 0..count {
            let line = next("update")?;
            let value = line
                .strip_prefix(&format!("  /\\ v{j}' = "))
                .ok_or_else(|| format!("expected v{j}'"))?;
            if value != format!("v{j}") {
                updates.push((j, expression(value)?));
            }
        }
        actions.push(Action::new(&format!("A{}", actions.len()), guard, updates));
    }
    expect(next("Spec")?, "Spec == Init /\\ [][Next]_vars")?;
    let mut model = Model::new(&name, variables);
    model.actions = actions;
    for line in lines {
        let group = |prefix: &str| -> Result<Option<Vec<usize>>, String> {
            let Some(inner) = line.strip_prefix(prefix).and_then(|s| s.strip_suffix(')')) else {
                return Ok(None);
            };
            inner
                .split(" \\/ ")
                .map(|a| {
                    a.strip_prefix('A')
                        .and_then(|n| n.parse().ok())
                        .ok_or_else(|| format!("bad fairness action {a}"))
                })
                .collect::<Result<_, _>>()
                .map(Some)
        };
        if let Some(group) = group("  /\\ WF_vars(")? {
            model.weak_fair.push(group);
        } else if let Some(group) = group("  /\\ SF_vars(")? {
            model.strong_fair.push(group);
        } else if let Some(rest) = line.strip_prefix(&format!("Inv{} == ", model.invariants.len()))
        {
            model.invariants.push(Invariant {
                name: format!("Inv{}", model.invariants.len()),
                condition: expression(rest)?,
            });
        } else if let Some(rest) = line.strip_prefix(&format!("Live{} == ", model.responses.len()))
        {
            let (request, response) = rest.split_once(" ~> ").ok_or("expected ~>")?;
            let unwrap = |s: &str| {
                s.strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')'))
                    .map(str::to_string)
                    .ok_or_else(|| "expected a parenthesized response operand".to_string())
            };
            model.responses.push(Response {
                name: format!("Live{}", model.responses.len()),
                request: expression(&unwrap(request)?)?,
                response: expression(&unwrap(response)?)?,
            });
        } else if line == "====" {
            break;
        } else {
            return Err(format!("unexpected line: {line}"));
        }
    }
    model.check_deadlock = !cfg.lines().any(|line| line == "CHECK_DEADLOCK FALSE");
    // Exactly the exporter's form: anything else is outside the native subset.
    if model.tla()? != tla {
        return Err("the .tla file is not in the exported native subset".into());
    }
    if model.config(false) != cfg {
        return Err("the .cfg file differs from the exported configuration".into());
    }
    Ok(model)
}

fn expect(line: &str, wanted: &str) -> Result<(), String> {
    if line == wanted {
        Ok(())
    } else {
        Err(format!("expected `{wanted}`, got `{line}`"))
    }
}

fn literal(text: &str) -> Result<i64, String> {
    text.strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("expected an integer literal, got {text}"))
}

/// One expression, as `Expr::tla` writes it (fully parenthesized).
fn expression(text: &str) -> Result<Expr, String> {
    let tokens = tokens(text)?;
    let mut parser = Parser {
        tokens: &tokens,
        at: 0,
        depth: 0,
    };
    let expr = parser.expr()?;
    if parser.at != tokens.len() {
        return Err(format!("trailing input in {text}"));
    }
    Ok(expr)
}

fn tokens(text: &str) -> Result<Vec<String>, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_alphanumeric()
            || (c == b'-' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
                i += 1;
            }
        } else {
            let two = text.get(i..i + 2).unwrap_or("");
            i += if matches!(
                two,
                "<=" | "/\\" | "\\/" | "->" | "[]" | ".." | "\\A" | "\\E"
            ) {
                2
            } else if text.get(i..i + 3) == Some("\\in") {
                3
            } else if b"()=<+-~:".contains(&c) {
                1
            } else {
                return Err(format!("unexpected character {:?}", c as char));
            };
        }
        out.push(text[start..i].to_string());
    }
    Ok(out)
}

struct Parser<'a> {
    tokens: &'a [String],
    at: usize,
    /// Enclosing quantifiers; `q<k>` is bound at absolute depth `k`.
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self, ahead: usize) -> &str {
        self.tokens.get(self.at + ahead).map_or("", String::as_str)
    }
    fn take(&mut self) -> Result<&str, String> {
        let token = self.tokens.get(self.at).ok_or("unexpected end")?;
        self.at += 1;
        Ok(token)
    }
    fn eat(&mut self, wanted: &str) -> Result<(), String> {
        let got = self.take()?;
        if got == wanted {
            Ok(())
        } else {
            Err(format!("expected {wanted}, got {got}"))
        }
    }
    fn number(&mut self) -> Result<i64, String> {
        let token = self.take()?;
        token
            .parse()
            .map_err(|_| format!("expected a number, got {token}"))
    }
    fn index(token: &str, prefix: char) -> Option<usize> {
        token.strip_prefix(prefix)?.parse().ok()
    }
    fn expr(&mut self) -> Result<Expr, String> {
        let token = self.take()?.to_string();
        match token.as_str() {
            "TRUE" => return Ok(Expr::Bool(true)),
            "FALSE" => return Ok(Expr::Bool(false)),
            "~" => {
                self.eat("(")?;
                let inner = self.expr()?;
                self.eat(")")?;
                return Ok(inner.negate());
            }
            "(" => {}
            other => {
                if let Some(i) = Self::index(other, 'v') {
                    return Ok(Expr::Var(i));
                }
                if let Some(k) = Self::index(other, 'q').filter(|k| *k < self.depth) {
                    return Ok(Expr::Bound(self.depth - 1 - k));
                }
                return Err(format!("unexpected token {other}"));
            }
        }
        let expr = match self.peek(0) {
            "IF" => {
                self.take()?;
                let c = self.expr()?;
                self.eat("THEN")?;
                let a = self.expr()?;
                self.eat("ELSE")?;
                Expr::Ite(c.into(), a.into(), self.expr()?.into())
            }
            "\\A" | "\\E" => {
                let forall = self.take()? == "\\A";
                let bound = format!("q{}", self.depth);
                if self.take()? != bound {
                    return Err("quantifier variable out of order".into());
                }
                self.eat("\\in")?;
                let lo = self.parenthesized_number()?;
                self.eat("..")?;
                let hi = self.parenthesized_number()?;
                self.eat(":")?;
                self.depth += 1;
                let body = self.expr()?;
                self.depth -= 1;
                if forall {
                    Expr::forall(lo, hi, body)
                } else {
                    Expr::exists(lo, hi, body)
                }
            }
            "CASE" => {
                self.take()?;
                let mut index = None;
                let mut base = None;
                let mut len = 0;
                loop {
                    let arm = self.expr()?;
                    self.eat("=")?;
                    if self.number()? != len as i64 {
                        return Err("CASE arms out of order".into());
                    }
                    self.eat("->")?;
                    let var = Self::index(self.take()?, 'v').ok_or("CASE arm needs a variable")?;
                    if *base.get_or_insert(var) + len != var {
                        return Err("CASE arms need consecutive variables".into());
                    }
                    index.get_or_insert(arm);
                    len += 1;
                    if self.peek(0) != "[]" {
                        break;
                    }
                    self.take()?;
                }
                Expr::pick(base.unwrap_or(0), len, index.ok_or("empty CASE")?)
            }
            _ if self.peek(1) == ")" && self.peek(0).parse::<i64>().is_ok() => {
                Expr::Int(self.number()?)
            }
            _ => {
                let a = self.expr()?;
                let op = self.take()?.to_string();
                let b = self.expr()?;
                match op.as_str() {
                    "=" => a.eq(b),
                    "<" => a.lt(b),
                    "<=" => a.le(b),
                    "+" => a.plus(b),
                    "-" => a.minus(b),
                    "/\\" => a.and(b),
                    "\\/" => a.or(b),
                    _ => return Err(format!("unknown operator {op}")),
                }
            }
        };
        self.eat(")")?;
        Ok(expr)
    }
    fn parenthesized_number(&mut self) -> Result<i64, String> {
        self.eat("(")?;
        let n = self.number()?;
        self.eat(")")?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::{action, all, eq, model, safe};
    use Expr::{Int, Var};

    fn sample(broken: bool) -> Model {
        let mut m = model("Sample", broken, &[("a", 2, 0), ("b", 3, 1), ("c", 1, 0)]);
        m.variables[2] = Variable::choice("c", 0, 1, &[0, 1]);
        action(
            &mut m,
            "Step",
            all([Var(0).lt(Int(2)), eq(2, 0).or(Var(1).le(Int(-1)).negate())]),
            &[
                (0, Var(0).plus(Int(1))),
                (1, eq(1, 0).ite(Int(0), Var(1).minus(Int(1)))),
            ],
        );
        action(
            &mut m,
            "Scan",
            Expr::forall(
                0,
                1,
                Expr::exists(0, 2, Expr::Bound(0).le(Expr::Bound(1).plus(Int(2)))),
            ),
            &[(1, Expr::pick(0, 2, Var(2)))],
        );
        safe(
            &mut m,
            "Bounded",
            Var(0).le(Int(if broken { 1 } else { 2 })),
        );
        m.responses.push(Response {
            name: "Done".into(),
            request: Expr::Bool(true),
            response: eq(0, 2),
        });
        m.weak_fair.push(vec![0, 1]);
        m.strong_fair.push(vec![1]);
        m
    }

    // Exported text parses back into a model that exports the same text and
    // reaches the same verdict.
    #[test]
    fn exported_models_round_trip_through_the_parser() {
        for broken in [false, true] {
            let m = sample(broken);
            let tla = m.tla().unwrap();
            let parsed = parse(&tla, &m.config(false)).unwrap();
            assert_eq!(parsed.tla().unwrap(), tla);
            assert_eq!(parsed.config(false), m.config(false));
            let ours = m.check(10_000).unwrap().violations.len();
            assert_eq!(parsed.check(10_000).unwrap().violations.len(), ours);
        }
    }

    #[test]
    fn text_outside_the_exported_form_is_refused() {
        let m = sample(false);
        let tla = m.tla().unwrap();
        let cfg = m.config(false);
        assert!(parse(&tla.replace("(v0 < (2))", "v0 < 2"), &cfg).is_err());
        assert!(parse(&tla.replace("EXTENDS Integers", "EXTENDS Naturals"), &cfg).is_err());
        assert!(parse(&tla, &cfg.replace("PROPERTY Live0\n", "")).is_err());
        assert!(parse(&format!("{tla}Extra == TRUE\n"), &cfg).is_err());
    }
}
