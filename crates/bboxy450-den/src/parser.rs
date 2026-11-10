// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Hand-written lexer and recursive-descent parser; no imported parser runtime.
use crate::Diagnostic;
use crate::ast::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokenKind {
    Word(String),
    Number(String),
    Text(String),
    Quoted(String),
    Symbol(String),
    Newline,
    End,
}
#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    pos: Pos,
}

type ParseResult<T> = Result<T, Diagnostic>;

// Each recursive expression level traverses the full precedence ladder. Keep
// the guard below a native test thread's stack budget, including debug builds.
const MAX_NESTING: usize = 16;
const MAX_CHAIN: usize = 32;
const MAX_TREE_DEPTH: usize = 64;

fn syntax(pos: Pos, message: impl Into<String>) -> Diagnostic {
    pos.diagnostic("X101", "", message, "use the Den grammar at this position")
}
fn unsupported(pos: Pos, construct: &str) -> Diagnostic {
    pos.diagnostic(
        "X190",
        "",
        format!("{construct} is not supported in this version"),
        "use a standalone spec with Bool, bounded Int, enum and Set[Enum] types",
    )
}
fn is_unsupported(word: &str) -> bool {
    matches!(
        word,
        "include"
            | "do"
            | "expose"
            | "discharge"
            | "epic"
            | "loop"
            | "task"
            | "report"
            | "interface"
            | "e2e"
            | "decision"
            | "proposal"
            | "bug"
            | "recipe"
            | "record"
            | "managed"
            | "leads"
            | "fair"
            | "staff"
            | "channel"
            | "work"
            | "waits"
            | "gate"
            | "budget"
            | "done"
            | "by"
            | "between"
            | "roles"
            | "produces"
            | "realizes"
            | "serves"
            | "brief"
    )
}
fn continues(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Symbol(s) if matches!(s.as_str(), "=" | "+=" | "-=" | "=>" | "==>" | "||" | "&&" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "+" | "-"))
}

struct Lexer {
    chars: Vec<char>,
    index: usize,
    pos: Pos,
}
impl Lexer {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            index: 0,
            pos: Pos { line: 1, column: 1 },
        }
    }
    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek(0)?;
        self.index += 1;
        if c == '\n' {
            self.pos.line += 1;
            self.pos.column = 1;
        } else {
            self.pos.column += 1;
        }
        Some(c)
    }
    fn string(&mut self, quoted: bool) -> ParseResult<String> {
        let start = self.pos;
        let triple = !quoted && self.peek(1) == Some('"') && self.peek(2) == Some('"');
        self.bump();
        if triple {
            self.bump();
            self.bump();
        }
        let mut value = String::new();
        loop {
            let Some(c) = self.peek(0) else {
                return Err(syntax(start, "unterminated string"));
            };
            if quoted && c == '`' {
                self.bump();
                return Ok(value);
            }
            if !quoted && c == '"' {
                if !triple {
                    self.bump();
                    return Ok(value);
                }
                if self.peek(1) == Some('"') && self.peek(2) == Some('"') {
                    self.bump();
                    self.bump();
                    self.bump();
                    return Ok(value);
                }
            }
            if !triple && c == '\n' {
                return Err(syntax(start, "a multiline string requires triple quotes"));
            }
            if !quoted && !triple && c == '\\' {
                self.bump();
                let escape = self
                    .bump()
                    .ok_or_else(|| syntax(start, "unterminated escape"))?;
                value.push(match escape {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    _ => return Err(syntax(self.pos, "unknown string escape")),
                });
            } else {
                value.push(c);
                self.bump();
            }
        }
    }
    fn tokens(mut self) -> ParseResult<Vec<Token>> {
        let mut result: Vec<Token> = Vec::new();
        let mut nesting = Vec::new();
        while let Some(c) = self.peek(0) {
            let pos = self.pos;
            if c == '/' && self.peek(1) == Some('/') {
                while self.peek(0).is_some_and(|v| v != '\n') {
                    self.bump();
                }
                continue;
            }
            if c == '\n' {
                self.bump();
                if nesting.is_empty() && !result.last().is_some_and(|t| continues(&t.kind)) {
                    result.push(Token {
                        kind: TokenKind::Newline,
                        pos,
                    });
                }
                continue;
            }
            if c.is_ascii_whitespace() {
                self.bump();
                continue;
            }
            let kind = if c.is_ascii_alphabetic() || c == '_' {
                let mut word = String::new();
                while let Some(v) = self
                    .peek(0)
                    .filter(|v| v.is_ascii_alphanumeric() || *v == '_')
                {
                    word.push(v);
                    self.bump();
                }
                TokenKind::Word(word)
            } else if c.is_ascii_digit() {
                let mut number = String::new();
                while let Some(v) = self.peek(0).filter(char::is_ascii_digit) {
                    number.push(v);
                    self.bump();
                }
                TokenKind::Number(number)
            } else if c == '"' {
                TokenKind::Text(self.string(false)?)
            } else if c == '`' {
                TokenKind::Quoted(self.string(true)?)
            } else {
                let mut symbol = None;
                for candidate in [
                    "==>", "=>", "==", "!=", "<=", ">=", "+=", "-=", "~>", "&&", "||", "..",
                ] {
                    if candidate
                        .chars()
                        .enumerate()
                        .all(|(i, v)| self.peek(i) == Some(v))
                    {
                        symbol = Some(candidate.to_string());
                        break;
                    }
                }
                let symbol = symbol.unwrap_or_else(|| c.to_string());
                if !matches!(
                    symbol.as_str(),
                    "==>"
                        | "=>"
                        | "=="
                        | "!="
                        | "<="
                        | ">="
                        | "+="
                        | "-="
                        | "~>"
                        | "&&"
                        | "||"
                        | ".."
                        | "{"
                        | "}"
                        | "("
                        | ")"
                        | "["
                        | "]"
                        | ","
                        | ":"
                        | ";"
                        | "."
                        | "="
                        | "!"
                        | "+"
                        | "-"
                        | "<"
                        | ">"
                ) {
                    return Err(syntax(pos, format!("unexpected character {c:?}")));
                }
                for _ in symbol.chars() {
                    self.bump();
                }
                match symbol.as_str() {
                    "(" => nesting.push(')'),
                    "[" => nesting.push(']'),
                    ")" | "]" if nesting.pop() != symbol.chars().next() => {
                        return Err(syntax(pos, "mismatched delimiter"));
                    }
                    _ => {}
                }
                TokenKind::Symbol(symbol)
            };
            result.push(Token { kind, pos });
        }
        if !nesting.is_empty() {
            return Err(syntax(self.pos, "unclosed delimiter"));
        }
        result.push(Token {
            kind: TokenKind::End,
            pos: self.pos,
        });
        Ok(result)
    }
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    record: String,
    depth: usize,
}
impl Parser {
    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }
    fn at(&self, text: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Word(s) | TokenKind::Symbol(s) if s == text)
    }
    fn take(&mut self, text: &str) -> bool {
        if self.at(text) {
            self.index += 1;
            true
        } else {
            false
        }
    }
    fn error(&self, message: impl Into<String>) -> Diagnostic {
        let mut d = match &self.current().kind {
            TokenKind::Word(word) if is_unsupported(word) => unsupported(self.current().pos, word),
            _ => syntax(self.current().pos, message),
        };
        d.record.clone_from(&self.record);
        d
    }
    fn need(&mut self, text: &str) -> ParseResult<()> {
        if self.take(text) {
            Ok(())
        } else {
            Err(self.error(format!("expected {text}")))
        }
    }
    fn separators(&mut self) {
        while self.take(";") || self.current().kind == TokenKind::Newline {
            if self.current().kind == TokenKind::Newline {
                self.index += 1;
            }
        }
    }
    fn end_statement(&mut self) -> ParseResult<()> {
        if self.at("}") || self.current().kind == TokenKind::End {
            return Ok(());
        }
        if self.take(";") || self.current().kind == TokenKind::Newline {
            if self.current().kind == TokenKind::Newline {
                self.index += 1;
            }
            self.separators();
            Ok(())
        } else {
            Err(self.error("expected a newline or semicolon"))
        }
    }
    fn name(&mut self) -> ParseResult<Name> {
        let token = self.current().clone();
        match token.kind {
            TokenKind::Word(text) => {
                self.index += 1;
                Ok(Name {
                    text,
                    pos: token.pos,
                })
            }
            _ => Err(self.error("expected a name")),
        }
    }
    fn text(&mut self) -> ParseResult<String> {
        if let TokenKind::Text(value) = self.current().kind.clone() {
            self.index += 1;
            Ok(value)
        } else {
            Err(self.error("expected quoted prose"))
        }
    }
    fn path(&mut self) -> ParseResult<Vec<Name>> {
        let mut result = Vec::new();
        loop {
            if let TokenKind::Quoted(text) = self.current().kind.clone() {
                result.push(Name {
                    text,
                    pos: self.current().pos,
                });
                self.index += 1;
            } else {
                result.push(self.name()?);
            }
            if !self.take(".") {
                break;
            }
        }
        Ok(result)
    }
    fn integer(&mut self) -> ParseResult<i64> {
        let negative = self.take("-");
        let pos = self.current().pos;
        let TokenKind::Number(digits) = &self.current().kind else {
            return Err(self.error("expected an integer"));
        };
        let text = if negative {
            format!("-{digits}")
        } else {
            digits.clone()
        };
        let value = text
            .parse()
            .map_err(|_| syntax(pos, "integer exceeds the supported i64 range"))?;
        self.index += 1;
        Ok(value)
    }
    fn ty(&mut self) -> ParseResult<Type> {
        if self.at("(") {
            return Err(unsupported(self.current().pos, "tuple types"));
        }
        let name = self.name()?;
        let kind = match name.text.as_str() {
            "Bool" => Ty::Bool,
            "Int" => {
                self.need("[")?;
                let lo = self.integer()?;
                self.need("..")?;
                let hi = self.integer()?;
                self.need("]")?;
                Ty::Int(lo, hi)
            }
            "Set" => {
                self.need("[")?;
                let element = self.name()?;
                if matches!(
                    element.text.as_str(),
                    "Bool" | "Int" | "Set" | "List" | "Option" | "Ref" | "Text"
                ) {
                    return Err(unsupported(element.pos, "this set element type"));
                }
                if self.at("[") {
                    return Err(unsupported(element.pos, "generic set element types"));
                }
                self.need("]")?;
                Ty::Set(element.text)
            }
            "Text" | "Markdown" | "Path" | "Sha" | "Time" | "Command" | "List" | "Option"
            | "Ref" => return Err(unsupported(name.pos, &name.text)),
            _ if self.at("[") => return Err(unsupported(name.pos, "generic types")),
            _ => Ty::Named(name.text),
        };
        Ok(Type {
            kind,
            pos: name.pos,
        })
    }
    fn signature(&mut self) -> ParseResult<Signature> {
        let name = self.name()?;
        let mut params = Vec::new();
        if self.take("(") {
            if !self.at(")") {
                loop {
                    let name = self.name()?;
                    self.need(":")?;
                    params.push(Param {
                        name,
                        ty: self.ty()?,
                    });
                    if !self.take(",") {
                        break;
                    }
                }
            }
            self.need(")")?;
        }
        Ok(Signature { name, params })
    }
    fn call(&mut self) -> ParseResult<Call> {
        let name = self.name()?;
        let args = self.arguments()?;
        Ok(Call { name, args })
    }
    fn arguments(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();
        if self.take("(") {
            if !self.at(")") {
                loop {
                    args.push(self.expr()?);
                    if !self.take(",") {
                        break;
                    }
                }
            }
            self.need(")")?;
        }
        Ok(args)
    }
    fn outcome(&mut self, scenario: bool) -> ParseResult<Outcome> {
        if self.take(if scenario { "accepted" } else { "accept" }) {
            return Ok(Outcome::Accept);
        }
        if self.take(if scenario { "rejected" } else { "reject" }) {
            return Ok(Outcome::Reject(self.name()?));
        }
        if self.take(if scenario { "ignored" } else { "ignore" }) {
            return Ok(Outcome::Ignore(self.name()?));
        }
        Err(self.error("expected an outcome"))
    }
    fn statements(&mut self) -> ParseResult<(Outcome, Vec<Stmt>)> {
        self.need("{")?;
        self.separators();
        let mut outcome = Outcome::Accept;
        let mut stmts = Vec::new();
        while !self.take("}") {
            let pos = self.current().pos;
            if self.at("reject") || self.at("ignore") {
                if !matches!(outcome, Outcome::Accept) {
                    return Err(self.error("a case has one outcome"));
                }
                outcome = self.outcome(false)?;
            } else if self.take("emit") {
                stmts.push(Stmt {
                    kind: StmtKind::Emit(self.call()?),
                    pos,
                });
            } else {
                if let TokenKind::Word(word) = &self.current().kind
                    && is_unsupported(word)
                {
                    return Err(unsupported(pos, word));
                }
                let name = self.name()?;
                let op = if self.take("=") {
                    "="
                } else if self.take("+=") {
                    "+="
                } else if self.take("-=") {
                    "-="
                } else {
                    return Err(self.error("expected =, += or -="));
                };
                stmts.push(Stmt {
                    kind: StmtKind::Assign(name, op.into(), self.expr()?),
                    pos,
                });
            }
            self.end_statement()?;
        }
        Ok((outcome, stmts))
    }
    fn handler(&mut self) -> ParseResult<Handler> {
        let event = self.name()?;
        let mut bindings = Vec::new();
        if self.take("(") {
            if !self.at(")") {
                loop {
                    bindings.push(self.name()?);
                    if !self.take(",") {
                        break;
                    }
                }
            }
            self.need(")")?;
        }
        self.need("{")?;
        self.separators();
        let mut cases = Vec::new();
        while !self.take("}") {
            self.need("case")?;
            let name = self.name()?;
            let guard = if self.take("if") {
                Some(self.expr()?)
            } else {
                self.need("otherwise")?;
                None
            };
            self.need("=>")?;
            let (outcome, stmts) = if self.at("{") {
                self.statements()?
            } else {
                let outcome = self.outcome(false)?;
                // Keep malformed reject/ignore mutations in the AST so X502 owns
                // their semantic diagnosis instead of hiding it as a parse error.
                if self.at("{") {
                    let (_, stmts) = self.statements()?;
                    (outcome, stmts)
                } else {
                    (outcome, Vec::new())
                }
            };
            cases.push(Case {
                name,
                guard,
                outcome,
                stmts,
            });
            self.end_statement()?;
        }
        Ok(Handler {
            event,
            bindings,
            cases,
        })
    }
    fn scenario(&mut self) -> ParseResult<Scenario> {
        let pos = self.current().pos;
        let name = self.text()?;
        self.need("{")?;
        self.separators();
        let mut steps = Vec::new();
        while !self.take("}") {
            if self.take("expect") {
                steps.push(ScenarioStep::Expect(self.expr()?));
            } else {
                let call = self.call()?;
                self.need("~>")?;
                let outcome = self.outcome(true)?;
                let mut effects = Vec::new();
                if self.take("emits") {
                    loop {
                        effects.push(self.call()?);
                        if !self.take(",") {
                            break;
                        }
                    }
                }
                steps.push(ScenarioStep::Call {
                    call,
                    outcome,
                    effects,
                });
            }
            self.end_statement()?;
        }
        Ok(Scenario { name, pos, steps })
    }
    fn spec(mut self) -> ParseResult<Spec> {
        self.separators();
        self.need("module")?;
        let module = self.path()?;
        self.end_statement()?;
        let mut imports = Vec::new();
        while self.take("import") {
            imports.push(self.path()?);
            self.end_statement()?;
        }
        if self.at("invariant") {
            return Err(unsupported(self.current().pos, "invariant records"));
        }
        self.need("spec")?;
        let name = self.name()?;
        self.record.clone_from(&name.text);
        self.need("{")?;
        self.separators();
        let mut result = Spec {
            name,
            module,
            imports,
            intent: String::new(),
            enums: Vec::new(),
            fields: Vec::new(),
            events: Vec::new(),
            effects: Vec::new(),
            handlers: Vec::new(),
            invariants: Vec::new(),
            assumptions: Vec::new(),
            scenarios: Vec::new(),
        };
        let mut has_intent = false;
        let mut has_state = false;
        while !self.take("}") {
            let declaration_pos = self.current().pos;
            if self.take("intent") {
                if has_intent {
                    return Err(declaration_pos.diagnostic(
                        "X202",
                        &self.record,
                        "intent is declared twice",
                        "keep one intent declaration",
                    ));
                }
                has_intent = true;
                result.intent = self.text()?;
            } else if self.take("enum") {
                let name = self.name()?;
                if self.at("[") {
                    return Err(unsupported(name.pos, "generic enum declarations"));
                }
                self.need("{")?;
                self.separators();
                let mut members = Vec::new();
                if !self.at("}") {
                    loop {
                        if self.at("case") {
                            return Err(unsupported(self.current().pos, "algebraic enum cases"));
                        }
                        let member = self.name()?;
                        if self.at("(") {
                            return Err(unsupported(member.pos, "algebraic enum cases"));
                        }
                        members.push(member);
                        self.separators();
                        if !self.take(",") {
                            break;
                        }
                        self.separators();
                    }
                }
                self.need("}")?;
                result.enums.push(Enum { name, members });
            } else if self.take("state") {
                if has_state {
                    return Err(declaration_pos.diagnostic(
                        "X202",
                        &self.record,
                        "state is declared twice",
                        "combine fields in one state block",
                    ));
                }
                has_state = true;
                self.need("{")?;
                self.separators();
                while !self.take("}") {
                    let name = self.name()?;
                    self.need(":")?;
                    let ty = self.ty()?;
                    self.need("=")?;
                    let initial = self.expr()?;
                    result.fields.push(Field { name, ty, initial });
                    self.end_statement()?;
                }
            } else if self.take("event") {
                result.events.push(self.signature()?);
            } else if self.take("effect") {
                result.effects.push(self.signature()?);
            } else if self.take("on") {
                result.handlers.push(self.handler()?);
            } else if self.take("invariant") {
                let name = self.name()?;
                let text = self.text()?;
                self.need("=")?;
                let expr = self.expr()?;
                result.invariants.push(Claim { name, text, expr });
            } else if self.take("assume") {
                let name = self.name()?;
                let text = self.text()?;
                result.assumptions.push(Assumption { name, text });
            } else if self.take("scenario") {
                result.scenarios.push(self.scenario()?);
            } else {
                return Err(self.error("expected a spec declaration"));
            }
            self.end_statement()?;
        }
        self.separators();
        if self.current().kind != TokenKind::End {
            if self.at("spec") || self.at("invariant") {
                return Err(unsupported(self.current().pos, "another record"));
            }
            return Err(self.error("expected end of file after the spec record"));
        }
        Ok(result)
    }
    fn expr(&mut self) -> ParseResult<Expr> {
        self.depth += 1;
        if self.depth > MAX_NESTING {
            return Err(self.error(format!("expression nesting exceeds {MAX_NESTING} levels")));
        }
        let result = self.implies();
        self.depth -= 1;
        if let Ok(expression) = &result {
            self.bound_expression(expression)?;
        }
        result
    }
    fn bound_expression(&self, expression: &Expr) -> ParseResult<()> {
        // A shallow syntactic nesting level can still build a deep left-associated
        // AST. Bound the final tree iteratively before downstream recursive walks.
        let mut pending = vec![(expression, 1)];
        while let Some((expression, depth)) = pending.pop() {
            if depth > MAX_TREE_DEPTH {
                return Err(syntax(
                    expression.pos,
                    format!("expression tree exceeds {MAX_TREE_DEPTH} levels"),
                ));
            }
            let mut push = |expression| pending.push((expression, depth + 1));
            match &expression.kind {
                ExprKind::Not(inner) => push(inner.as_ref()),
                ExprKind::Binary(_, left, right) => {
                    push(left.as_ref());
                    push(right.as_ref());
                }
                ExprKind::If(condition, then, otherwise) => {
                    push(condition.as_ref());
                    push(then.as_ref());
                    push(otherwise.as_ref());
                }
                ExprKind::Quant { body, .. } => push(body.as_ref()),
                ExprKind::Call(_, args) => {
                    for argument in args {
                        push(argument);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn binary(
        &mut self,
        next: fn(&mut Self) -> ParseResult<Expr>,
        operators: &[&str],
    ) -> ParseResult<Expr> {
        let mut left = next(self)?;
        let mut count = 0;
        while let Some(op) = operators.iter().find(|op| self.at(op)) {
            count += 1;
            if count > MAX_CHAIN {
                return Err(self.error(format!("expression chain exceeds {MAX_CHAIN} operators")));
            }
            let op = (*op).to_string();
            self.index += 1;
            let right = next(self)?;
            let pos = left.pos;
            left = Expr {
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
                pos,
            };
        }
        Ok(left)
    }
    fn implies(&mut self) -> ParseResult<Expr> {
        let left = self.or()?;
        if self.take("==>") {
            let right = self.expr()?;
            let pos = left.pos;
            Ok(Expr {
                kind: ExprKind::Binary("==>".into(), Box::new(left), Box::new(right)),
                pos,
            })
        } else {
            Ok(left)
        }
    }
    fn or(&mut self) -> ParseResult<Expr> {
        self.binary(Self::and, &["||"])
    }
    fn and(&mut self) -> ParseResult<Expr> {
        self.binary(Self::not, &["&&"])
    }
    fn not(&mut self) -> ParseResult<Expr> {
        if self.at("!") {
            let pos = self.current().pos;
            self.index += 1;
            // Bound repeated negation as well as parenthesized recursion.
            self.depth += 1;
            if self.depth > MAX_NESTING {
                return Err(self.error(format!("expression nesting exceeds {MAX_NESTING} levels")));
            }
            let inner = self.not()?;
            self.depth -= 1;
            Ok(Expr {
                kind: ExprKind::Not(Box::new(inner)),
                pos,
            })
        } else {
            self.compare()
        }
    }
    fn compare(&mut self) -> ParseResult<Expr> {
        self.binary(Self::add, &["==", "!=", "<", "<=", ">", ">="])
    }
    fn add(&mut self) -> ParseResult<Expr> {
        self.binary(Self::primary, &["+", "-"])
    }
    fn primary(&mut self) -> ParseResult<Expr> {
        let pos = self.current().pos;
        let kind = if self.take("true") {
            ExprKind::Bool(true)
        } else if self.take("false") {
            ExprKind::Bool(false)
        } else if self.at("-") || matches!(self.current().kind, TokenKind::Number(_)) {
            ExprKind::Int(self.integer()?)
        } else if self.take("(") {
            let expression = self.expr()?;
            self.need(")")?;
            return Ok(expression);
        } else if self.take("if") {
            let condition = self.expr()?;
            self.need("then")?;
            let then = self.expr()?;
            self.need("else")?;
            let otherwise = self.expr()?;
            ExprKind::If(Box::new(condition), Box::new(then), Box::new(otherwise))
        } else if self.at("forall") || self.at("exists") {
            let all = self.take("forall");
            if !all {
                self.need("exists")?;
            }
            let binding = self.name()?;
            self.need(":")?;
            let domain = self.name()?;
            self.need(".")?;
            let body = Box::new(self.expr()?);
            ExprKind::Quant {
                all,
                binding,
                domain,
                body,
            }
        } else {
            if let TokenKind::Word(word) = &self.current().kind
                && is_unsupported(word)
            {
                return Err(unsupported(pos, word));
            }
            let name = self.name()?;
            if self.take(".") {
                ExprKind::Member(name.text, self.name()?.text)
            } else if self.at("(") {
                ExprKind::Call(name.text, self.arguments()?)
            } else {
                ExprKind::Name(name.text)
            }
        };
        Ok(Expr { kind, pos })
    }
}

pub(crate) fn parse(source: &str) -> Result<Spec, Vec<Diagnostic>> {
    let tokens = Lexer::new(source).tokens().map_err(|e| vec![e])?;
    Parser {
        tokens,
        index: 0,
        record: String::new(),
        depth: 0,
    }
    .spec()
    .map_err(|e| vec![e])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuation_strings_positions_and_simultaneous_body() {
        let s = parse("module specs.`a-b`\nimport notes.Known\nspec Test {\nintent \"\"\"first\nsecond\"\"\"\nenum E { A, B }\nstate { x: Int[-1..2] =\n-1; s: Set[E] = Set() }\nevent Go(e: E)\neffect Tell(e: E)\non Go(e) {\ncase yes if x < 2 &&\n!s(e) =>\n{ x = x + 1; emit Tell(e) }\ncase no otherwise => reject Full\n}\ninvariant TEST_1 \"bounded\" = forall e: E. s(e) ==> x >= -1\nassume TEST_A1 \"environment\"\nscenario \"one\" { Go(E.A) ~> accepted emits Tell(E.A); expect x == 0 }\n}\n").unwrap();
        assert_eq!(s.name.text, "Test");
        assert_eq!(s.module[1].text, "a-b");
        assert_eq!(s.intent, "first\nsecond");
        assert_eq!(s.fields[0].ty.kind, Ty::Int(-1, 2));
        assert_eq!(s.handlers[0].cases.len(), 2);
        assert_eq!(s.handlers[0].cases[0].stmts.len(), 2);
        assert_eq!(s.scenarios[0].steps.len(), 2);
        assert_eq!(s.fields[0].name.pos.line, 7);
    }
    #[test]
    fn implication_is_right_associative_and_not_binds_after_comparison() {
        let s = parse("module T\nspec T { invariant I \"i\" = !true == false ==> true ==> false }")
            .unwrap();
        let ExprKind::Binary(op, left, right) = &s.invariants[0].expr.kind else {
            panic!("implication");
        };
        assert_eq!(op, "==>");
        assert!(matches!(left.kind, ExprKind::Not(_)));
        assert!(matches!(right.kind, ExprKind::Binary(_, _, _)));
    }
    #[test]
    fn unsupported_and_malformed_have_distinct_codes() {
        for input in [
            "module T\nepic T {}",
            "module T\nspec T { include Other as o }",
            "module T\nspec T { state { x: Text = true } }",
            "module T\nspec T { enum D[T] { A } }",
            "module T\nspec T { enum D { case Foo(x: Bool) } }",
            "module T\nspec T { enum D { Foo(x: Bool) } }",
            "module T\nspec T { state { x: D[Bool] = true } }",
            "module T\nspec T { state { x: Set[D[Bool]] = Set() } }",
            "module T\nspec T { on Go { case c if true => { do Other } } }",
        ] {
            let d = parse(input).unwrap_err();
            assert_eq!(d.len(), 1);
            assert_eq!(d[0].code, "X190");
        }
        for input in [
            "",
            "module T\nspec T {",
            "module T\nspec T { intent \"oops }",
            "module T\nspec T { invariant I \"i\" = (true] }",
        ] {
            assert_eq!(parse(input).unwrap_err()[0].code, "X101");
        }
    }
    #[test]
    fn reject_updates_reach_the_step_checker() {
        let s = parse("module T\nspec T { on Go { case c if true => { reject Bad; x = true } } }")
            .unwrap();
        assert!(matches!(s.handlers[0].cases[0].outcome, Outcome::Reject(_)));
        assert_eq!(s.handlers[0].cases[0].stmts.len(), 1);
    }
    #[test]
    fn duplicate_singleton_declarations_are_name_errors() {
        for body in ["intent \"first\"; intent \"second\"", "state {}; state {}"] {
            let source = format!("module T\nspec T {{ {body} }}");
            let d = parse(&source).unwrap_err();
            assert_eq!(d.len(), 1);
            assert_eq!(d[0].code, "X202");
        }
    }
    #[test]
    fn nesting_and_integer_overflow_fail_without_panicking() {
        let deeply_nested = format!(
            "module T\nspec T {{ invariant I \"i\" = {}true{} }}",
            "(".repeat(200),
            ")".repeat(200)
        );
        assert_eq!(parse(&deeply_nested).unwrap_err()[0].code, "X101");
        for expression in [
            format!("{}true", "!".repeat(200)),
            std::iter::repeat_n("true", 200)
                .collect::<Vec<_>>()
                .join(" && "),
        ] {
            let source = format!("module T\nspec T {{ invariant I \"i\" = {expression} }}");
            assert_eq!(parse(&source).unwrap_err()[0].code, "X101");
        }
        let mut expression = "true".to_string();
        for _ in 0..10 {
            expression = format!(
                "({expression}) && {}",
                std::iter::repeat_n("true", 20)
                    .collect::<Vec<_>>()
                    .join(" && ")
            );
        }
        let source = format!("module T\nspec T {{ invariant I \"i\" = {expression} }}");
        assert_eq!(parse(&source).unwrap_err()[0].code, "X101");
        assert_eq!(
            parse("module T\nspec T { state { x: Int[0..1] = 9999999999999999999999 } }")
                .unwrap_err()[0]
                .code,
            "X101"
        );
    }
}
