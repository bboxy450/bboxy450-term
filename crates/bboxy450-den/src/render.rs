// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Lowering one sound `spec` record to one Markdown record, the projection a
//! Markdown-only store such as GitKB keeps (`specs/spec-language.md`, "Lowering
//! to Markdown"). Den is the source; the Markdown says so in `generated_from:`
//! and carries the whole source in a fenced `den` block, so a reader can
//! recover it and tell an edited projection from a faithful one.
use std::fmt::Write;

use sha2::{Digest, Sha256};

use crate::ast::*;
use crate::{Report, Summary};

/// One lowered record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The Markdown record: YAML header, generated sections, the Den source.
    pub markdown: String,
    /// Dotted module path of the source.
    pub module: String,
    /// SHA-256 of the exact source bytes, hex.
    pub source_sha256: String,
}

/// Hex SHA-256 of the source bytes.
pub fn source_sha256(source: &str) -> String {
    let digest = Sha256::digest(source.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Lower a sound record; an unsound one returns its report and nothing is
/// written. Deterministic: the same source gives the same bytes.
pub fn render(source: &str) -> Result<Rendered, Report> {
    render_with(source, true)
}

/// [`render`], with the Mermaid views left out when `mermaid` is false.
pub fn render_with(source: &str, mermaid: bool) -> Result<Rendered, Report> {
    let report = crate::check(source);
    if !report.is_sound() {
        return Err(report);
    }
    let spec = crate::parser::parse(source).map_err(|diagnostics| Report {
        records: Vec::new(),
        diagnostics,
    })?;
    let summary = report.records.first();
    let module = spec
        .module
        .iter()
        .map(|n| n.text.as_str())
        .collect::<Vec<_>>()
        .join(".");
    let source_sha256 = source_sha256(source);
    let markdown = markdown(&spec, summary, &module, &source_sha256, source, mermaid);
    Ok(Rendered {
        markdown,
        module,
        source_sha256,
    })
}

/// The Den source and declared hash embedded in a lowered record, if it is
/// one: `(source, declared_sha256)`. The caller compares the hash with
/// [`source_sha256`] of the recovered source; a mismatch is an edited projection.
pub fn embedded(markdown: &str) -> Option<(String, String)> {
    let mut lines = markdown.lines();
    if lines.next() != Some("---") {
        return None;
    }
    let mut sha = None;
    for line in lines.by_ref() {
        if line == "---" {
            break;
        }
        if let Some(value) = line.trim().strip_prefix("source_sha256:") {
            sha = Some(value.trim().to_owned());
        }
    }
    let sha = sha?;
    let start = markdown.rfind("\n```den\n")? + "\n```den\n".len();
    let rest = &markdown[start..];
    let end = rest.rfind("\n```")?;
    Some((rest[..end + 1].to_owned(), sha))
}

/// The record's title: the intent's first sentence (a list shows one line).
fn title_of(intent: &str) -> &str {
    let intent = intent.trim();
    intent
        .find(". ")
        .map_or(intent, |end| &intent[..=end])
}

fn markdown(
    spec: &Spec,
    summary: Option<&Summary>,
    module: &str,
    sha: &str,
    source: &str,
    mermaid: bool,
) -> String {
    let mut out = String::new();
    let intent = spec.intent.trim();
    let _ = writeln!(out, "---");
    let _ = writeln!(out, "type: spec");
    let _ = writeln!(out, "title: {}", yaml(title_of(intent)));
    let _ = writeln!(out, "generated_from:");
    let _ = writeln!(out, "  module: {module}");
    let _ = writeln!(out, "  source_sha256: {sha}");
    if let Some(s) = summary {
        let _ = writeln!(out, "den_states: {}", s.states);
        let _ = writeln!(out, "den_transitions: {}", s.transitions);
    }
    let _ = writeln!(out, "---");
    let _ = writeln!(out, "# {}\n", spec.name.text);
    let _ = writeln!(out, "{intent}\n");
    let _ = writeln!(
        out,
        "Lowered from `{module}` ({}); the Den source at the end of this record is the original.\n",
        spec.name.text
    );

    if !spec.enums.is_empty() {
        let _ = writeln!(out, "## Domains\n");
        for e in &spec.enums {
            let members = e
                .members
                .iter()
                .map(|m| format!("`{}`", m.text))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(out, "- `{}`: {members}", e.name.text);
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "## State\n");
    let _ = writeln!(out, "| Field | Type | Initial |");
    let _ = writeln!(out, "|---|---|---|");
    for f in &spec.fields {
        let _ = writeln!(
            out,
            "| `{}` | `{}` | `{}` |",
            f.name.text,
            ty(&f.ty.kind),
            expr(&f.initial)
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Events\n");
    for s in &spec.events {
        let _ = writeln!(out, "- `{}`", signature(s));
    }
    let _ = writeln!(out);
    if !spec.effects.is_empty() {
        let _ = writeln!(out, "## Effects\n");
        for s in &spec.effects {
            let _ = writeln!(out, "- `{}`", signature(s));
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "## Handlers\n");
    for h in &spec.handlers {
        let bindings = if h.bindings.is_empty() {
            String::new()
        } else {
            format!(
                "({})",
                h.bindings
                    .iter()
                    .map(|b| b.text.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let _ = writeln!(out, "### on {}{bindings}\n", h.event.text);
        let _ = writeln!(out, "| Case | When | Outcome |");
        let _ = writeln!(out, "|---|---|---|");
        for c in &h.cases {
            let when = match &c.guard {
                Some(g) => format!("`{}`", expr(g)),
                None => "otherwise".to_owned(),
            };
            let effect = match &c.outcome {
                Outcome::Accept => {
                    let steps = c.stmts.iter().map(stmt).collect::<Vec<_>>();
                    if steps.is_empty() {
                        "accepted".to_owned()
                    } else {
                        format!("accepted: `{}`", steps.join("; "))
                    }
                }
                Outcome::Reject(n) => format!("rejected `{}`", n.text),
                Outcome::Ignore(n) => format!("ignored `{}`", n.text),
            };
            let _ = writeln!(out, "| `{}` | {when} | {effect} |", c.name.text);
        }
        let _ = writeln!(out);
    }

    if !spec.invariants.is_empty() {
        let _ = writeln!(out, "## Invariants\n");
        for i in &spec.invariants {
            let _ = writeln!(
                out,
                "- **{}**: {}. `{}`",
                i.name.text,
                i.text.trim(),
                expr(&i.expr)
            );
        }
        let _ = writeln!(out);
    }
    if !spec.assumptions.is_empty() {
        let _ = writeln!(out, "## Assumptions\n");
        for a in &spec.assumptions {
            let _ = writeln!(out, "- **{}**: {}.", a.name.text, a.text.trim());
        }
        let _ = writeln!(out);
    }

    if !spec.scenarios.is_empty() {
        let _ = writeln!(out, "## Scenarios\n");
        for (n, s) in spec.scenarios.iter().enumerate() {
            let _ = writeln!(out, "### {}. {}\n", n + 1, s.name.trim());
            for (i, step) in s.steps.iter().enumerate() {
                let line = match step {
                    ScenarioStep::Call {
                        call,
                        outcome,
                        effects,
                    } => {
                        let outcome = match outcome {
                            Outcome::Accept => "accepted".to_owned(),
                            Outcome::Reject(n) => format!("rejected {}", n.text),
                            Outcome::Ignore(n) => format!("ignored {}", n.text),
                        };
                        let emits = if effects.is_empty() {
                            String::new()
                        } else {
                            format!(
                                " emits {}",
                                effects.iter().map(call_text).collect::<Vec<_>>().join(", ")
                            )
                        };
                        format!("`{} ~> {outcome}{emits}`", call_text(call))
                    }
                    ScenarioStep::Expect(e) => format!("`expect {}`", expr(e)),
                };
                let _ = writeln!(out, "{}. {line}", i + 1);
            }
            let _ = writeln!(out);
        }
    }

    if mermaid && !spec.scenarios.is_empty() {
        let _ = writeln!(out, "## Views\n");
        for (n, s) in spec.scenarios.iter().enumerate() {
            let _ = writeln!(out, "Scenario {}: {}\n", n + 1, s.name.trim());
            let _ = writeln!(out, "```mermaid");
            let _ = writeln!(out, "flowchart TD");
            let _ = writeln!(out, "    s0([\"start\"])");
            for (i, step) in s.steps.iter().enumerate() {
                let label = match step {
                    ScenarioStep::Call { call, outcome, .. } => {
                        let outcome = match outcome {
                            Outcome::Accept => "accepted".to_owned(),
                            Outcome::Reject(n) => format!("rejected {}", n.text),
                            Outcome::Ignore(n) => format!("ignored {}", n.text),
                        };
                        format!("{} ~> {outcome}", call_text(call))
                    }
                    ScenarioStep::Expect(e) => format!("expect {}", expr(e)),
                };
                let _ = writeln!(
                    out,
                    "    s{} --> s{}[\"{}\"]",
                    i,
                    i + 1,
                    mermaid_label(&label)
                );
            }
            let _ = writeln!(out, "```\n");
        }
    }

    let _ = writeln!(out, "## Source\n");
    let _ = writeln!(out, "```den");
    out.push_str(source);
    if !source.ends_with('\n') {
        out.push('\n');
    }
    let _ = writeln!(out, "```");
    out
}

/// A Mermaid node label: quotes and newlines cannot appear inside one.
fn mermaid_label(text: &str) -> String {
    text.replace('"', "#quot;").replace('\n', " ")
}

/// Single-quoted YAML scalar: safe for any text, no type guessing.
fn yaml(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''").replace('\n', " "))
}

fn ty(kind: &Ty) -> String {
    match kind {
        Ty::Bool => "Bool".into(),
        Ty::Int(lo, hi) => format!("Int[{lo}..{hi}]"),
        Ty::Named(n) => n.clone(),
        Ty::Set(n) => format!("Set[{n}]"),
    }
}

fn signature(s: &Signature) -> String {
    if s.params.is_empty() {
        return s.name.text.clone();
    }
    let params = s
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name.text, ty(&p.ty.kind)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({params})", s.name.text)
}

fn call_text(c: &Call) -> String {
    if c.args.is_empty() {
        return c.name.text.clone();
    }
    format!(
        "{}({})",
        c.name.text,
        c.args.iter().map(expr).collect::<Vec<_>>().join(", ")
    )
}

fn stmt(s: &Stmt) -> String {
    match &s.kind {
        StmtKind::Assign(name, op, e) => format!("{} {op} {}", name.text, expr(e)),
        StmtKind::Emit(c) => format!("emit {}", call_text(c)),
    }
}

/// Den source text for an expression; compound operands are parenthesized,
/// so the text re-parses to the same tree.
pub(crate) fn expr(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Bool(b) => b.to_string(),
        ExprKind::Int(i) => i.to_string(),
        ExprKind::Name(n) => n.clone(),
        ExprKind::Member(a, b) => format!("{a}.{b}"),
        ExprKind::Call(name, args) => format!(
            "{name}({})",
            args.iter().map(expr).collect::<Vec<_>>().join(", ")
        ),
        ExprKind::Not(x) => format!("!{}", atom(x)),
        ExprKind::Binary(op, l, r) => format!("{} {op} {}", atom(l), atom(r)),
        ExprKind::If(c, t, o) => format!("if {} then {} else {}", expr(c), expr(t), expr(o)),
        ExprKind::Quant {
            all,
            binding,
            domain,
            body,
        } => format!(
            "{} {}: {}. {}",
            if *all { "forall" } else { "exists" },
            binding.text,
            domain.text,
            expr(body)
        ),
    }
}

fn atom(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Binary(..) | ExprKind::If(..) | ExprKind::Quant { .. } => {
            format!("({})", expr(e))
        }
        _ => expr(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOGGLE: &str = "module specs.Toggle\n\nspec Toggle {\n  intent \"Two flags toggle together.\"\n  state {\n    x: Bool = false\n    y: Bool = false\n  }\n  event Toggle\n  on Toggle {\n    case toggle if true => { x = !x; y = !y }\n  }\n  invariant TOGGLE_001 \"the flags agree\" = x == y\n  scenario \"toggle both flags\" {\n    Toggle ~> accepted\n    expect x && y\n  }\n}\n";

    #[test]
    fn a_sound_spec_lowers_deterministically_with_its_source_and_hash() {
        let first = render(TOGGLE).unwrap();
        let second = render(TOGGLE).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.module, "specs.Toggle");
        assert_eq!(first.source_sha256.len(), 64);
        let md = &first.markdown;
        assert!(md.starts_with("---\ntype: spec\ntitle: 'Two flags toggle together.'\ngenerated_from:\n  module: specs.Toggle\n  source_sha256: "), "{md}");
        assert!(md.contains("## Views\n\nScenario 1: toggle both flags\n\n```mermaid\nflowchart TD\n    s0([\"start\"])\n    s0 --> s1[\"Toggle ~> accepted\"]\n    s1 --> s2[\"expect x && y\"]\n```"), "{md}");
        assert!(!render_with(TOGGLE, false).unwrap().markdown.contains("## Views"));
        assert_eq!(title_of("One. Two sentences."), "One.");
        assert_eq!(title_of("No period"), "No period");
        for needle in [
            "| `x` | `Bool` | `false` |",
            "### on Toggle",
            "| `toggle` | `true` | accepted: `x = !x; y = !y` |",
            "- **TOGGLE_001**: the flags agree. `x == y`",
            "1. `Toggle ~> accepted`",
            "2. `expect x && y`",
            "```den\nmodule specs.Toggle",
        ] {
            assert!(md.contains(needle), "missing {needle:?} in\n{md}");
        }
        let (source, declared) = embedded(md).unwrap();
        assert_eq!(source, TOGGLE);
        assert_eq!(declared, source_sha256(&source));
    }

    #[test]
    fn an_edited_projection_no_longer_matches_its_declared_hash() {
        let rendered = render(TOGGLE).unwrap();
        let edited = rendered.markdown.replace("x = !x", "x = x");
        let (source, declared) = embedded(&edited).unwrap();
        assert_ne!(declared, source_sha256(&source));
        assert!(embedded("# plain markdown\n").is_none());
    }

    #[test]
    fn an_unsound_spec_renders_nothing_and_returns_its_diagnostics() {
        let report = render("module a.B\nspec B { intent \"x\" state { } event E }\n").unwrap_err();
        assert!(!report.is_sound());
    }

    #[test]
    fn expressions_print_in_den_syntax_with_parentheses_only_where_needed() {
        let spec = crate::parser::parse(
            "module a.B\nspec B {\n  intent \"x\"\n  enum D { P, Q }\n  state { n: Int[0..3] = 0; d: D = D.P }\n  event E(k: Int[0..3])\n  on E(k) {\n    case go if (n + k) <= 3 && !(d == D.Q) => { n += k; d = if n == 3 then D.Q else D.P }\n    case no otherwise => reject Full\n  }\n  invariant I_001 \"bounded\" = forall m: D. (d == m) ==> true\n}\n",
        )
        .unwrap_or_else(|d| panic!("{d:?}"));
        let case = &spec.handlers[0].cases[0];
        assert_eq!(expr(case.guard.as_ref().unwrap()), "((n + k) <= 3) && !(d == D.Q)");
        assert_eq!(stmt(&case.stmts[1]), "d = if n == 3 then D.Q else D.P");
        assert_eq!(expr(&spec.invariants[0].expr), "forall m: D. (d == m) ==> true");
    }
}
