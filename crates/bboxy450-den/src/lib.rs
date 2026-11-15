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
mod record;
pub mod render;
mod types;
pub use record::{Field, Record, Value, body, load, metadata, set_managed};

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
    /// Record kind: `spec`, or a header kind, whose counts below stay empty.
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
    /// `open ID "text"`: the questions the spec leaves undecided. They are
    /// never an error and never change a check.
    pub open: Vec<(String, String)>,
}
/// Checks syntax, names, types, totality, steps, bounded safety, vacuity and examples.
/// A header record (every kind but a `spec` with a machine) gets the checks of [`load`].
pub fn check(source: &str) -> Report {
    if !record::is_record(source) {
        return checker::check(source);
    }
    let (records, diagnostics) = match (record::parse(source), load(source)) {
        (Ok(parsed), result) => (vec![parsed.record], result.err().unwrap_or_default()),
        (Err(diagnostics), _) => (vec![], diagnostics),
    };
    let summary = |r: &Record| Summary {
        kind: r.kind,
        name: r.name.clone(),
        intent: r
            .fields
            .iter()
            .find(|field| field.name == "intent")
            .map(|field| field.value.text())
            .unwrap_or_default(),
        domains: vec![],
        states: 0,
        transitions: 0,
        invariants: vec![],
        assumptions: vec![],
        properties: vec![],
        theorems: vec![],
        scenarios: 0,
        open: vec![],
    };
    Report {
        records: records.iter().map(summary).collect(),
        diagnostics,
    }
}
/// One named declaration of a Den record, for navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// spec, enum, field, event, effect, handler, invariant, theorem, property,
    /// assumption, scenario, open, source, define or table. A header record is its record kind, then one
    /// `field` per top-level field and one for its `managed` block.
    pub kind: &'static str,
    pub name: String,
    /// One-based source line of the name.
    pub line: u32,
}
/// The named declarations of a Den source in source order. This is the syntax
/// only: a file that parses but is unsound still has an outline.
pub fn declarations(source: &str) -> Result<Vec<Declaration>, Vec<Diagnostic>> {
    if record::is_record(source) {
        let parsed = record::parse(source)?;
        let mut out = vec![Declaration {
            kind: parsed.record.kind,
            name: parsed.record.name.clone(),
            line: parsed.record.line,
        }];
        let fields = parsed
            .record
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.line));
        for (name, line) in fields.chain(parsed.managed_line.map(|line| ("managed", line))) {
            out.push(Declaration {
                kind: "field",
                name: name.to_owned(),
                line,
            });
        }
        out.sort_by_key(|declaration| declaration.line);
        return Ok(out);
    }
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
    for (kind, name) in spec
        .defines
        .iter()
        .map(|d| ("define", &d.name))
        .chain(spec.tables.iter().map(|t| ("table", &t.name)))
    {
        out.push(Declaration {
            kind,
            name: name.text.clone(),
            line: name.pos.line,
        });
    }
    spec.opens.iter().for_each(|o| {
        out.push(Declaration {
            kind: "open",
            name: o.name.text.clone(),
            line: o.name.pos.line,
        })
    });
    for source in &spec.sources {
        let target: Vec<_> = source.target.iter().map(|n| n.text.as_str()).collect();
        out.push(Declaration {
            kind: "source",
            name: target.join("."),
            line: source.target[0].pos.line,
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
    if record::is_record(source) {
        return Ok(None);
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
        let records=self.records.iter().map(|s|format!("{{\"kind\":{},\"name\":{},\"intent\":{},\"domains\":[{}],\"states\":{},\"transitions\":{},\"invariants\":[{}],\"assumptions\":[{}],\"properties\":[{}],\"theorems\":[{}],\"scenarios\":{},\"open\":[{}]}}",quote(s.kind),quote(&s.name),quote(&s.intent),s.domains.iter().map(|(n,c)|format!("[{},{}]",quote(n),c)).collect::<Vec<_>>().join(","),s.states,s.transitions,pairs(&s.invariants),pairs(&s.assumptions),pairs(&s.properties),s.theorems.iter().map(|t|quote(t)).collect::<Vec<_>>().join(","),s.scenarios,pairs(&s.open))).collect::<Vec<_>>().join(",");
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
