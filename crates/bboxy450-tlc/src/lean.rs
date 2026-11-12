// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Inspired by Veil's shallow embedding of transition systems in Lean 4
// (https://github.com/verse-lab/veil) and by TLA+ (Leslie Lamport). No source copied;
// the output needs only Lean's core library.
//! A Lean 4 code generator for typed state machines, shared by every frontend.
//!
//! [`Model`](crate::Model) is finite: a set over a two-member domain is two
//! integer variables. A proof for *every* domain size needs the machine before
//! that flattening, so a frontend (Den, Yarn) describes it here with its
//! expressions already printed as Lean terms, and this module prints the
//! declarations around them. The state is a structure of propositions, a step
//! is an inductive relation with one constructor per accepting case, and each
//! theorem is the statement that a claim holds in every reachable state.
//!
//! Nothing here runs Lean or proves anything: the kernel does, in
//! `make test-lean`. Liveness and effects are not exported.
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::format;
use std::path::Path;
use std::string::{String, ToString};
use std::vec::Vec;

/// The state variable in every printed term. Not a Den or Rust identifier.
pub const STATE: &str = "σ";

/// A finite domain. Abstract when the machine never names a member: the
/// theorems then hold for a type of any size.
#[derive(Clone, Debug)]
pub struct Domain {
    /// Type name.
    pub name: String,
    /// Members of a concrete domain, or `None` for an abstract type.
    pub members: Option<Vec<String>>,
}
/// One state field.
#[derive(Clone, Debug)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Lean type: `Prop`, `Int`, a domain, or `D → Prop` for a set.
    pub ty: String,
    /// Closed Lean term for the initial value.
    pub initial: String,
}
/// One accepting case of one event.
#[derive(Clone, Debug)]
pub struct Step {
    /// Constructor name, unique in the machine.
    pub name: String,
    /// Event parameters as `(name, Lean type)`.
    pub params: Vec<(String, String)>,
    /// Lean proposition over [`STATE`] and the parameters.
    pub guard: String,
    /// `(field, Lean term over the pre-state)`; other fields are unchanged.
    pub updates: Vec<(String, String)>,
}
/// A named state predicate.
#[derive(Clone, Debug)]
pub struct Claim {
    /// Identifier, unique in the machine.
    pub id: String,
    /// What it means, in prose.
    pub text: String,
    /// Lean proposition over [`STATE`].
    pub prop: String,
    /// Whether a proof for every reachable state is owed.
    pub theorem: bool,
}
/// A typed state machine with its expressions printed as Lean terms.
#[derive(Clone, Debug)]
pub struct Machine {
    /// Lean module and namespace name.
    pub module: String,
    /// Where the machine came from, for the file header.
    pub origin: String,
    /// Finite domains, abstract or concrete.
    pub domains: Vec<Domain>,
    /// State fields.
    pub fields: Vec<Field>,
    /// Accepting transitions. Rejected and ignored events change nothing.
    pub steps: Vec<Step>,
    /// State predicates; those marked `theorem` become proof obligations.
    pub claims: Vec<Claim>,
}

const KEYWORDS: &[&str] = &[
    "at",
    "by",
    "do",
    "end",
    "from",
    "fun",
    "have",
    "if",
    "in",
    "let",
    "match",
    "show",
    "then",
    "else",
    "with",
    "where",
    "open",
    "def",
    "theorem",
    "example",
    "instance",
    "structure",
    "inductive",
    "namespace",
    "section",
    "variable",
    "universe",
    "import",
    "export",
    "deriving",
    "mutual",
    "private",
    "protected",
    "partial",
    "unsafe",
    "axiom",
    "opaque",
    "abbrev",
    "class",
    "macro",
    "syntax",
    "notation",
    "infix",
    "prefix",
    "postfix",
    "set_option",
    "attribute",
    "local",
    "scoped",
    "using",
    "suffices",
    "calc",
    "for",
    "while",
    "return",
    "try",
    "catch",
    "finally",
    "unless",
    "nomatch",
    "nofun",
    "forall",
    "exists",
    "Type",
    "Sort",
    "Prop",
    "true",
    "false",
    "this",
    "extends",
    "mut",
    "initialize",
    "noncomputable",
    "termination_by",
    "decreasing_by",
    "omit",
    "include",
    "public",
    "meta",
    "module",
];
/// A source identifier as a Lean identifier.
pub fn ident(name: &str) -> String {
    if KEYWORDS.contains(&name) {
        format!("«{name}»")
    } else {
        name.to_string()
    }
}

impl Machine {
    /// IDs of the claims that owe a proof, in declaration order.
    pub fn theorems(&self) -> Vec<&str> {
        let owed = self.claims.iter().filter(|c| c.theorem);
        owed.map(|c| c.id.as_str()).collect()
    }
    fn abstracts(&self) -> Vec<&str> {
        let open = self.domains.iter().filter(|d| d.members.is_none());
        open.map(|d| d.name.as_str()).collect()
    }
    fn validate(&self) -> Result<(), String> {
        let mut seen: BTreeSet<String> = ["State", "Step", "Reachable", "initial"]
            .into_iter()
            .map(String::from)
            .collect();
        let names = self.domains.iter().map(|d| d.name.clone()).chain(
            self.claims
                .iter()
                .flat_map(|c| [c.id.clone(), format!("{}_statement", c.id)]),
        );
        for name in names {
            if !seen.insert(name.clone()) {
                return Err(format!("{name} is declared twice in the Lean module"));
            }
        }
        let mut steps = BTreeSet::new();
        if let Some(step) = self.steps.iter().find(|s| !steps.insert(&s.name)) {
            return Err(format!("step {} is declared twice", step.name));
        }
        Ok(())
    }
    /// `MODULE.lean`: the definitions and one `ID_statement : Prop` per theorem.
    pub fn statements(&self) -> Result<String, String> {
        self.validate()?;
        let abstracts = self.abstracts();
        let args: String = abstracts.iter().map(|a| format!(" {}", ident(a))).collect();
        let state = if abstracts.is_empty() {
            "State".to_string()
        } else {
            format!("(State{args})")
        };
        let mut out = format!(
            "-- Generated by bboxy450-tlc from {}. Do not edit: regenerate.\n-- Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term\n-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md\n--\n-- The state is a structure of propositions and `Step` has one constructor per\n-- accepting case; rejected and ignored events leave the state unchanged and are\n-- omitted. A theorem is `ID_statement`: prove it as `ID_proved` in namespace\n-- `{}`.\n",
            self.origin, self.module
        );
        for d in &self.domains {
            if d.members.is_none() {
                let _ = writeln!(
                    out,
                    "-- `{}` is a type of any size: no member is named by the machine.",
                    d.name
                );
            }
        }
        let _ = writeln!(out, "\nnamespace {}\n", ident(&self.module));
        for d in &self.domains {
            if let Some(members) = &d.members {
                let _ = writeln!(out, "inductive {} where", ident(&d.name));
                for m in members {
                    let _ = writeln!(out, "  | {}", ident(m));
                }
                let _ = writeln!(out, "  deriving DecidableEq\n");
            }
        }
        let binders: String = abstracts
            .iter()
            .map(|a| format!(" ({} : Type)", ident(a)))
            .collect();
        let _ = writeln!(out, "structure State{binders} where");
        for f in &self.fields {
            let _ = writeln!(out, "  {} : {}", ident(&f.name), f.ty);
        }
        if !abstracts.is_empty() {
            let _ = writeln!(out, "\nvariable {{{} : Type}}", args.trim_start());
        }
        let _ = writeln!(out, "\ndef initial : {state} where");
        for f in &self.fields {
            let _ = writeln!(out, "  {} := {}", ident(&f.name), f.initial);
        }
        let _ = writeln!(out, "\ninductive Step : {state} → {state} → Prop where");
        for s in &self.steps {
            let params: String = s
                .params
                .iter()
                .map(|(n, t)| format!(" ({} : {t})", ident(n)))
                .collect();
            let next = if s.updates.is_empty() {
                STATE.to_string()
            } else {
                let updates: Vec<_> = s
                    .updates
                    .iter()
                    .map(|(f, v)| format!("{} := {v}", ident(f)))
                    .collect();
                format!("{{ {STATE} with {} }}", updates.join(", "))
            };
            let _ = writeln!(
                out,
                "  | {} ({STATE} : {state}){params} (guard : {}) :\n      Step {STATE} {next}",
                ident(&s.name),
                s.guard
            );
        }
        let _ = writeln!(
            out,
            "\ninductive Reachable : {state} → Prop where\n  | initial : Reachable initial\n  | step {{{STATE} τ : {state}}} : Reachable {STATE} → Step {STATE} τ → Reachable τ"
        );
        for c in &self.claims {
            let doc = c.text.replace("-/", "- /");
            let _ = writeln!(
                out,
                "\n/-- {doc} -/\ndef {} ({STATE} : {state}) : Prop := {}",
                ident(&c.id),
                c.prop
            );
            if c.theorem {
                let _ = writeln!(
                    out,
                    "/-- {doc}: in every reachable state{}. -/\ndef {}_statement : Prop :=\n  ∀{} ({STATE} : {state}), Reachable {STATE} → {} {STATE}",
                    if abstracts.is_empty() {
                        ""
                    } else {
                        ", for domains of every size"
                    },
                    c.id,
                    binders,
                    ident(&c.id)
                );
            }
        }
        let _ = writeln!(out, "\nend {}", ident(&self.module));
        Ok(out)
    }
    /// Writes `MODULE.lean`, `MODULECheck.lean` and the given proof as
    /// `MODULEProof.lean` into `out`, and appends the `lean.tsv` row that
    /// `scripts/lean.py --check` reads. `expected` is `pass`, or `fail` for a
    /// broken control. A module already exported there is refused.
    // Development-time exporter run by `make test-lean`; no UI thread exists here.
    #[allow(clippy::disallowed_methods)]
    pub fn export(&self, out: &Path, proof: &str, expected: &str) -> Result<(), String> {
        let statements = self.statements()?;
        let io = |path: &Path, e: std::io::Error| format!("{}: {e}", path.display());
        std::fs::create_dir_all(out).map_err(|e| io(out, e))?;
        let manifest = out.join("lean.tsv");
        let mut rows = std::fs::read_to_string(&manifest).unwrap_or_default();
        if rows
            .lines()
            .any(|row| row.split('\t').next() == Some(&self.module))
        {
            return Err(format!(
                "module {} is already exported to {}",
                self.module,
                out.display()
            ));
        }
        let _ = writeln!(
            rows,
            "{}\t{expected}\t{}",
            self.module,
            self.theorems().join(",")
        );
        for (suffix, text) in [
            ("", statements.as_str()),
            ("Proof", proof),
            ("Check", self.check().as_str()),
        ] {
            let path = out.join(format!("{}{suffix}.lean", self.module));
            std::fs::write(&path, text).map_err(|e| io(&path, e))?;
        }
        std::fs::write(&manifest, rows).map_err(|e| io(&manifest, e))
    }
    /// `MODULECheck.lean`: every proof has exactly its generated statement, and
    /// reports the axioms it rests on. The proof module is `MODULEProof`.
    pub fn check(&self) -> String {
        let m = ident(&self.module);
        let mut out = format!(
            "-- Generated by bboxy450-tlc. Do not edit: regenerate.\nimport {}\nimport {}Proof\n",
            self.module, self.module
        );
        for id in self.theorems() {
            let _ = writeln!(
                out,
                "theorem den_check_{id} : {m}.{id}_statement := {m}.{id}_proved\n#print axioms den_check_{id}"
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn machine() -> Machine {
        Machine {
            module: "Toggle".into(),
            origin: "a test".into(),
            domains: vec![Domain {
                name: "Msg".into(),
                members: None,
            }],
            fields: vec![Field {
                name: "open".into(),
                ty: "Msg → Prop".into(),
                initial: "fun _ => False".into(),
            }],
            steps: vec![Step {
                name: "Add_add".into(),
                params: vec![("m".into(), "Msg".into())],
                guard: "¬ σ.«open» m".into(),
                updates: vec![("open".into(), "fun ξ0 => ξ0 = m ∨ σ.«open» ξ0".into())],
            }],
            claims: vec![Claim {
                id: "T1".into(),
                text: "anything".into(),
                prop: "True".into(),
                theorem: true,
            }],
        }
    }
    #[test]
    fn keywords_are_quoted_and_the_statement_quantifies_over_every_domain() {
        let text = machine().statements().unwrap();
        assert!(text.contains("  «open» : Msg → Prop"), "{text}");
        assert!(text.contains("{ σ with «open» := fun ξ0 => ξ0 = m ∨ σ.«open» ξ0 }"));
        assert!(text.contains("∀ (Msg : Type) (σ : (State Msg)), Reachable σ → T1 σ"));
        assert_eq!(machine().theorems(), ["T1"]);
        assert!(machine().check().contains(
            "theorem den_check_T1 : Toggle.T1_statement := Toggle.T1_proved\n#print axioms den_check_T1"
        ));
    }
    #[test]
    fn a_name_that_collides_with_a_generated_declaration_is_refused() {
        let mut m = machine();
        m.domains[0].name = "Step".into();
        assert!(
            m.statements()
                .unwrap_err()
                .contains("Step is declared twice")
        );
    }
}
