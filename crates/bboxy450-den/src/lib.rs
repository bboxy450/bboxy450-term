// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Parser and bounded soundness checker for Den.
#![forbid(unsafe_code)]
mod ast;
mod checker;
mod eval;
mod lean;
mod lower;
mod parser;
pub mod render;
mod types;

/// Result of checking one source file. Any diagnostic makes it unsound.
#[derive(Debug)]
pub struct Report {
    /// Parsed record summaries.
    pub records: Vec<Summary>,
    /// Ordered, positioned errors with explicit repair instructions.
    pub diagnostics: Vec<Diagnostic>,
}
/// One stable diagnostic.
#[derive(Debug)]
pub struct Diagnostic {
    /// Stable Den error code.
    pub code: &'static str,
    /// One-based source line.
    pub line: u32,
    /// One-based Unicode character column.
    pub column: u32,
    /// Containing record, or empty before it is parsed.
    pub record: String,
    /// Reason this source is unsound.
    pub message: String,
    /// How to repair the source.
    pub repair: String,
}
/// Bounded verification results for a parsed record.
#[derive(Debug)]
pub struct Summary {
    /// Record kind (`spec` in this version).
    pub kind: &'static str,
    /// Record name.
    pub name: String,
    /// Required statement of intent.
    pub intent: String,
    /// Enum names and cardinalities, in declaration order.
    pub domains: Vec<(String, usize)>,
    /// Reachable states; zero if exploration did not complete.
    pub states: usize,
    /// Explicit transitions, including rejected/ignored self-loops.
    pub transitions: usize,
    /// Invariant IDs and prose.
    pub invariants: Vec<(String, String)>,
    /// Environmental assumption IDs and prose.
    pub assumptions: Vec<(String, String)>,
    /// Progress property IDs and prose, checked under the declared fairness.
    pub properties: Vec<(String, String)>,
    /// Invariant IDs declared `theorem`. `check` proves them only within the
    /// bounds; the unbounded proof is the Lean gate's evidence, not this report's.
    pub theorems: Vec<String>,
    /// Number of declared scenarios.
    pub scenarios: usize,
}
/// Checks syntax, names, types, totality, steps, bounded safety, vacuity and examples.
pub fn check(source: &str) -> Report {
    checker::check(source)
}
/// One named declaration of a Den record, for navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// spec, enum, field, event, effect, handler, invariant, theorem, property,
    /// assumption or scenario.
    pub kind: &'static str,
    pub name: String,
    /// One-based source line of the name.
    pub line: u32,
}
/// The named declarations of a Den source in source order. This is the syntax
/// only: a file that parses but is unsound still has an outline.
pub fn declarations(source: &str) -> Result<Vec<Declaration>, Vec<Diagnostic>> {
    let spec = parser::parse(source)?;
    let mut out = vec![Declaration {
        kind: "spec",
        name: spec.name.text.clone(),
        line: spec.name.pos.line,
    }];
    let mut add = |kind, name: &ast::Name| {
        out.push(Declaration {
            kind,
            name: name.text.clone(),
            line: name.pos.line,
        })
    };
    spec.enums.iter().for_each(|e| add("enum", &e.name));
    spec.fields.iter().for_each(|f| add("field", &f.name));
    spec.events.iter().for_each(|e| add("event", &e.name));
    spec.effects.iter().for_each(|e| add("effect", &e.name));
    spec.handlers.iter().for_each(|h| add("handler", &h.event));
    spec.invariants
        .iter()
        .for_each(|c| add(if c.theorem { "theorem" } else { "invariant" }, &c.name));
    spec.properties
        .iter()
        .for_each(|p| add("property", &p.name));
    spec.assumptions
        .iter()
        .for_each(|a| add("assumption", &a.name));
    for scenario in &spec.scenarios {
        out.push(Declaration {
            kind: "scenario",
            name: scenario.name.clone(),
            line: scenario.pos.line,
        });
    }
    out.sort_by_key(|declaration| declaration.line);
    Ok(out)
}
/// Checks that a recorded trace is a behavior the spec allows. The trace is the
/// body of a scenario, one step per line: `Event(args) ~> accepted [emits ...]`,
/// `~> rejected Reason`, `~> ignored Reason`, and optionally `expect expr`. An
/// implementation that logs its events and outcomes in this form conforms when
/// the replay is sound. A diagnostic's position is in the trace; an unsound
/// spec returns its own diagnostics and replays nothing.
pub fn replay(spec: &str, trace: &str) -> Report {
    checker::replay(spec, trace)
}
/// The finite model `check` explores, for a spec that is well formed (names,
/// types, totality, steps) but not necessarily sound: a broken control still
/// has a model, and exporting it lets Java TLC refute it independently.
pub fn model(source: &str) -> Result<bboxy450_tlc::Model, Report> {
    let unsound = |diagnostics| Report {
        records: vec![],
        diagnostics,
    };
    let spec = checker::load(source).map_err(unsound)?;
    lower::model(&spec).map_err(|e| {
        unsound(vec![spec.name.pos.diagnostic(
            "X602",
            &spec.name.text,
            e,
            "keep updates within the declared variable ranges",
        )])
    })
}
/// A sound spec's machine for Lean, where its `theorem`s are stated for
/// domains of every size: `Machine::statements` is the text, `Machine::export`
/// writes it beside a proof for `make test-lean`. `None` when the spec declares
/// no theorem. Unsound specs are refused: a bounded counterexample is cheaper
/// than a failed proof.
pub fn lean(source: &str) -> Result<Option<bboxy450_tlc::lean::Machine>, Report> {
    let report = check(source);
    if !report.is_sound() {
        return Err(report);
    }
    let spec = checker::load(source).expect("sound");
    let machine = lean::machine(&spec);
    if machine.theorems().is_empty() {
        return Ok(None);
    }
    match machine.statements() {
        Ok(_) => Ok(Some(machine)),
        Err(e) => Err(Report {
            records: vec![],
            diagnostics: vec![spec.name.pos.diagnostic(
                "X190",
                &spec.name.text,
                e,
                "rename the declaration: State, Step, Reachable, initial and TYPE_OK are generated",
            )],
        }),
    }
}
impl Report {
    /// True exactly when no diagnostics were produced.
    pub fn is_sound(&self) -> bool {
        self.diagnostics.is_empty()
    }
    /// One line per diagnostic: `L:C CODE message. Repair: ...`.
    pub fn to_text(&self) -> String {
        self.diagnostics
            .iter()
            .map(|d| {
                format!(
                    "{}:{} {} {}. Repair: {}",
                    d.line,
                    d.column,
                    d.code,
                    d.message.trim_end_matches('.').replace(['\r', '\n'], " "),
                    d.repair.replace(['\r', '\n'], " ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    /// JSON with stable record, diagnostic and field ordering; no serialization dependency.
    pub fn to_json(&self) -> String {
        let records=self.records.iter().map(|s|format!("{{\"kind\":{},\"name\":{},\"intent\":{},\"domains\":[{}],\"states\":{},\"transitions\":{},\"invariants\":[{}],\"assumptions\":[{}],\"properties\":[{}],\"theorems\":[{}],\"scenarios\":{}}}",quote(s.kind),quote(&s.name),quote(&s.intent),s.domains.iter().map(|(n,c)|format!("[{},{}]",quote(n),c)).collect::<Vec<_>>().join(","),s.states,s.transitions,pairs(&s.invariants),pairs(&s.assumptions),pairs(&s.properties),s.theorems.iter().map(|t|quote(t)).collect::<Vec<_>>().join(","),s.scenarios)).collect::<Vec<_>>().join(",");
        let diagnostics=self.diagnostics.iter().map(|d|format!("{{\"code\":{},\"line\":{},\"column\":{},\"record\":{},\"message\":{},\"repair\":{}}}",quote(d.code),d.line,d.column,quote(&d.record),quote(&d.message),quote(&d.repair))).collect::<Vec<_>>().join(",");
        format!("{{\"records\":[{records}],\"diagnostics\":[{diagnostics}]}}")
    }
}
fn pairs(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(a, b)| format!("[{},{}]", quote(a), quote(b)))
        .collect::<Vec<_>>()
        .join(",")
}
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
#[cfg(test)]
mod tests;
