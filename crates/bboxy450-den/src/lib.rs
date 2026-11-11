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
    /// Number of declared scenarios.
    pub scenarios: usize,
}
/// Checks syntax, names, types, totality, steps, bounded safety, vacuity and examples.
pub fn check(source: &str) -> Report {
    checker::check(source)
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
        let records=self.records.iter().map(|s|format!("{{\"kind\":{},\"name\":{},\"intent\":{},\"domains\":[{}],\"states\":{},\"transitions\":{},\"invariants\":[{}],\"assumptions\":[{}],\"scenarios\":{}}}",quote(s.kind),quote(&s.name),quote(&s.intent),s.domains.iter().map(|(n,c)|format!("[{},{}]",quote(n),c)).collect::<Vec<_>>().join(","),s.states,s.transitions,pairs(&s.invariants),pairs(&s.assumptions),s.scenarios)).collect::<Vec<_>>().join(",");
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
