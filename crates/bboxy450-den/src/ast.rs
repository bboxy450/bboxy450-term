// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::Diagnostic;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pos {
    pub line: u32,
    pub column: u32,
}
impl Pos {
    pub fn diagnostic(
        self,
        code: &'static str,
        record: &str,
        message: impl Into<String>,
        repair: &str,
    ) -> Diagnostic {
        Diagnostic {
            code,
            line: self.line,
            column: self.column,
            record: record.into(),
            message: message.into(),
            repair: repair.into(),
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Name {
    pub text: String,
    pub pos: Pos,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Ty {
    Bool,
    Int(i64, i64),
    Named(String),
    Set(String),
}
#[derive(Clone, Debug)]
pub(crate) struct Type {
    pub kind: Ty,
    pub pos: Pos,
}
#[derive(Clone, Debug)]
pub(crate) struct Expr {
    pub kind: ExprKind,
    pub pos: Pos,
}
#[derive(Clone, Debug)]
pub(crate) enum ExprKind {
    Bool(bool),
    Int(i64),
    Name(String),
    Member(String, String),
    Call(String, Vec<Expr>),
    Not(Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Quant {
        all: bool,
        binding: Name,
        domain: Name,
        body: Box<Expr>,
    },
}
#[derive(Clone, Debug)]
pub(crate) struct Enum {
    pub name: Name,
    pub members: Vec<Name>,
}
#[derive(Clone, Debug)]
pub(crate) struct Field {
    pub name: Name,
    pub ty: Type,
    pub initial: Expr,
}
#[derive(Clone, Debug)]
pub(crate) struct Param {
    pub name: Name,
    pub ty: Type,
}
#[derive(Clone, Debug)]
pub(crate) struct Signature {
    pub name: Name,
    pub params: Vec<Param>,
}
#[derive(Clone, Debug)]
pub(crate) struct Call {
    pub name: Name,
    pub args: Vec<Expr>,
}
#[derive(Clone, Debug)]
pub(crate) enum StmtKind {
    Assign(Name, String, Expr),
    Emit(Call),
}
#[derive(Clone, Debug)]
pub(crate) struct Stmt {
    pub kind: StmtKind,
    pub pos: Pos,
}
#[derive(Clone, Debug)]
pub(crate) enum Outcome {
    Accept,
    Reject(Name),
    Ignore(Name),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fairness {
    Weak,
    Strong,
}
#[derive(Clone, Debug)]
pub(crate) struct Case {
    pub name: Name,
    /// The environment promises this case is eventually taken when it stays
    /// (weak) or keeps becoming (strong) selectable. One case is one group.
    pub fair: Option<(Fairness, Pos)>,
    pub guard: Option<Expr>,
    pub outcome: Outcome,
    pub stmts: Vec<Stmt>,
}
#[derive(Clone, Debug)]
pub(crate) struct Handler {
    pub event: Name,
    pub bindings: Vec<Name>,
    pub cases: Vec<Case>,
}
#[derive(Clone, Debug)]
pub(crate) struct Claim {
    pub name: Name,
    pub text: String,
    pub expr: Expr,
    /// Declared with `theorem`: also owes a Lean proof for every domain size.
    pub theorem: bool,
}
/// `request leads to response`, once per value of the binders.
#[derive(Clone, Debug)]
pub(crate) struct Property {
    pub name: Name,
    pub text: String,
    pub binders: Vec<(Name, Name)>,
    pub request: Expr,
    pub response: Expr,
}
#[derive(Clone, Debug)]
pub(crate) struct Assumption {
    pub name: Name,
    pub text: String,
}
#[derive(Clone, Debug)]
pub(crate) enum ScenarioStep {
    Call {
        call: Call,
        outcome: Outcome,
        effects: Vec<Call>,
    },
    Expect(Expr),
}
#[derive(Clone, Debug)]
pub(crate) struct Scenario {
    pub name: String,
    pub pos: Pos,
    pub steps: Vec<ScenarioStep>,
}
#[derive(Clone, Debug)]
pub(crate) struct Spec {
    pub name: Name,
    pub module: Vec<Name>,
    pub imports: Vec<Vec<Name>>,
    pub intent: String,
    pub enums: Vec<Enum>,
    pub fields: Vec<Field>,
    pub events: Vec<Signature>,
    pub effects: Vec<Signature>,
    pub handlers: Vec<Handler>,
    pub invariants: Vec<Claim>,
    pub properties: Vec<Property>,
    pub assumptions: Vec<Assumption>,
    pub scenarios: Vec<Scenario>,
}
