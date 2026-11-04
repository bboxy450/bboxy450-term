// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Inspired by Quint's finite verification scope: https://quint.sh/docs/checking-properties
// and TLA-Verus statement preservation: https://arxiv.org/abs/2610.14923
// Concepts implemented independently; no Quint/TLA-Verus source copied.
//! Honest, hosted verification evidence. Exhaustion never establishes a claim.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::format;
use std::path::{Path, PathBuf};
use std::string::{String, ToString};
use std::vec::Vec;

use crate::model::{Model, Report, Violation};

/// A stable required id and its named protocol predicate.
#[derive(Clone, Copy, Debug)]
pub struct Requirement {
    /// Stable project requirement id.
    pub id: &'static str,
    /// Protocol module name.
    pub protocol: &'static str,
    /// Declared predicate name.
    pub predicate: &'static str,
}

/// Location of a protocol declaration or claim, supplied by the macro.
#[derive(Clone, Copy, Debug)]
pub struct DeclarationSpan {
    /// Source path as seen by rustc.
    pub file: &'static str,
    /// One-based source line.
    pub line: u32,
    /// One-based source column.
    pub column: u32,
}

/// A finite domain and its exact declared members.
#[derive(Clone, Copy, Debug)]
pub struct DomainMetadata {
    /// Domain name.
    pub name: &'static str,
    /// Members in declaration order.
    pub variants: &'static [&'static str],
}

/// Stable identity and source location of a generated invariant.
#[derive(Clone, Copy, Debug)]
pub struct ClaimMetadata {
    /// Stable claim id.
    pub id: &'static str,
    /// Generated predicate name.
    pub predicate: &'static str,
    /// Predicate declaration location.
    pub span: DeclarationSpan,
}

/// Additive metadata emitted by `protocol!`, without changing reducer semantics.
#[derive(Clone, Copy, Debug)]
pub struct ProtocolMetadata {
    /// Protocol module name.
    pub name: &'static str,
    /// Canonical declaration tokens, including guards and predicates.
    pub source: &'static str,
    /// Declaration location.
    pub span: DeclarationSpan,
    /// Finite domains in declaration order.
    pub domains: &'static [DomainMetadata],
    /// Invariants in declaration order; model response ids are also checked.
    pub claims: &'static [ClaimMetadata],
    /// All Event::case labels, independent of event parameters.
    pub cases: &'static [&'static str],
    /// Version of the generating macro crate.
    pub macro_version: &'static str,
}

/// Verification verdict, independent of the exploration method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The complete declared finite graph satisfies all required checks.
    Established,
    /// A violated condition or unreachable required case was found.
    Refuted,
    /// A resource limit prevented complete exploration.
    Inconclusive,
    /// The declaration, contract or verification configuration is invalid.
    InvalidInput,
    /// No exploration was attempted.
    NotRun,
}

impl Outcome {
    /// Stable JSON spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Established => "established",
            Self::Refuted => "refuted",
            Self::Inconclusive => "inconclusive",
            Self::InvalidInput => "invalid_input",
            Self::NotRun => "not_run",
        }
    }
}

/// An encoded state variable's finite bounds.
#[derive(Clone, Debug)]
pub struct Bound {
    /// Variable name.
    pub name: String,
    /// Inclusive lower bound.
    pub min: i64,
    /// Inclusive upper bound.
    pub max: i64,
}

/// Exact finite verification scope; no environmental fairness is implicit.
#[derive(Clone, Debug)]
pub struct Scope {
    /// Domain names and their complete declared member lists.
    pub domains: Vec<(String, Vec<String>)>,
    /// Bounds for every encoded state variable.
    pub bounds: Vec<Bound>,
    /// Maximum reachable states permitted by this run.
    pub state_limit: usize,
    /// Explicit weak/strong fairness groups, with concrete action names.
    pub fairness_assumed: Vec<String>,
    /// Abstractions assumed by the generated-core model.
    pub assumptions: Vec<String>,
    /// No waivers are supported by this frontend.
    pub waivers: Vec<String>,
}

/// Declaration identities and tool versions, separate from the verdict.
#[derive(Clone, Debug)]
pub struct Provenance {
    /// Deterministic FNV-1a identity of canonical protocol tokens (not a signature).
    pub source_hash: String,
    /// Identity of the verification declaration's canonical tokens.
    pub verification_hash: String,
    /// Original declaration location.
    pub declaration: DeclarationSpan,
    /// Runtime and macro versions.
    pub tool_versions: Vec<(String, String)>,
}

/// A concrete trace and the failed safety, liveness or reachability condition.
#[derive(Clone, Debug)]
pub struct Counterexample {
    /// Event names from the initial state; parameters are retained.
    pub events: Vec<String>,
    /// Lasso cycle events; Stutter denotes an implicit TLA stuttering step.
    pub cycle: Vec<String>,
    /// Stable failed property id or explicit reachability condition.
    pub failed_condition: String,
}

/// Structured result of one run. `to_json` needs no serialization dependency.
#[derive(Clone, Debug)]
pub struct VerificationResult {
    /// Stable verdict independent of all other result axes.
    pub outcome: Outcome,
    /// Currently exhaustive_finite_model; no sampled proof mode is supported.
    pub method: &'static str,
    /// Contract identity.
    pub contract: String,
    /// Required stable ids, in contract declaration order.
    pub required_ids: Vec<String>,
    /// Cases required to fire at least once in the complete graph.
    pub require_reachable: Vec<String>,
    /// Protocol identity.
    pub protocol: String,
    /// Domains, bounds and explicit assumptions.
    pub scope: Scope,
    /// generated_core; independent Rust replay is a separate witness.
    pub implementation: &'static str,
    /// Source identities and tools.
    pub provenance: Provenance,
    /// Number of states, present only after complete exploration.
    pub states: Option<usize>,
    /// Number of edges, including explicit self-loops, after complete exploration.
    pub transitions: Option<usize>,
    /// Case labels actually fired in the exhaustive graph.
    pub reached_cases: Vec<String>,
    /// Counterexample for the first failed condition, when applicable.
    pub counterexample: Option<Counterexample>,
    /// Human-readable explanation, including missing/extra ids and limits.
    pub message: String,
}

/// Check a complete finite model with exact contract and reachability coverage.
///
/// Canonical-token hashes identify inputs, not cryptographic custody. Signed
/// commits and the repository's SHA-256 formal receipt remain separate evidence.
pub fn check(
    contract: &str,
    requirements: &[Requirement],
    metadata: &ProtocolMetadata,
    model: &Model,
    require_reachable: &[&str],
    max_states: usize,
    verification_source: &str,
) -> VerificationResult {
    let mut result = VerificationResult {
        outcome: Outcome::NotRun,
        method: "exhaustive_finite_model",
        contract: contract.into(),
        required_ids: requirements.iter().map(|r| r.id.into()).collect(),
        require_reachable: require_reachable.iter().map(|c| (*c).into()).collect(),
        protocol: metadata.name.into(),
        scope: Scope {
            domains: metadata
                .domains
                .iter()
                .map(|d| {
                    (
                        d.name.into(),
                        d.variants.iter().map(|s| (*s).into()).collect(),
                    )
                })
                .collect(),
            bounds: model
                .variables
                .iter()
                .map(|v| Bound {
                    name: v.name.clone(),
                    min: v.min,
                    max: v.max,
                })
                .collect(),
            state_limit: max_states,
            fairness_assumed: model
                .weak_fair
                .iter()
                .map(|g| ("weak", g))
                .chain(model.strong_fair.iter().map(|g| ("strong", g)))
                .map(|(kind, g)| {
                    format!(
                        "{kind}({})",
                        g.iter()
                            .map(|&i| model
                                .actions
                                .get(i)
                                .map_or("<invalid action>", |a| a.name.as_str()))
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                })
                .collect(),
            assumptions: [
                "atomic pure protocol steps",
                "effects and environment outside the model",
            ]
            .map(String::from)
            .into(),
            waivers: Vec::new(),
        },
        implementation: "generated_core",
        provenance: Provenance {
            source_hash: token_hash(metadata.source),
            verification_hash: token_hash(verification_source),
            declaration: metadata.span,
            tool_versions: [
                ("bboxy450-yarn".into(), env!("CARGO_PKG_VERSION").into()),
                ("bboxy450-yarn-macros".into(), metadata.macro_version.into()),
            ]
            .into(),
        },
        states: None,
        transitions: None,
        reached_cases: Vec::new(),
        counterexample: None,
        message: String::new(),
    };
    if let Err(message) = validate(requirements, metadata, model, require_reachable, max_states) {
        result.outcome = Outcome::InvalidInput;
        result.message = message;
        return result;
    }
    let report = match model.check(max_states) {
        Ok(report) => report,
        Err(message) => {
            result.outcome = if message.starts_with("state limit ") {
                Outcome::Inconclusive
            } else {
                Outcome::InvalidInput
            };
            result.message = message;
            return result;
        }
    };
    result.states = Some(report.states.len());
    result.transitions = Some(report.edges.iter().map(Vec::len).sum());
    let reached: BTreeSet<_> = report
        .edges
        .iter()
        .flatten()
        .map(|edge| {
            let action = &model.actions[edge.action].name;
            let (event, case) = action
                .rsplit_once("::")
                .expect("generated actions carry case labels");
            format!("{}::{case}", event.split('(').next().unwrap())
        })
        .collect();
    result.reached_cases = reached.iter().cloned().collect();
    if let Some(violation) = report.violations.first() {
        result.outcome = Outcome::Refuted;
        result.message = format!("condition {} is false", violation.property);
        result.counterexample = Some(counterexample(model, &report, violation));
    } else if let Some(case) = require_reachable
        .iter()
        .find(|case| !reached.contains(**case))
    {
        result.outcome = Outcome::Refuted;
        result.message = format!("required case {case} never fires in the exhaustive graph");
        result.counterexample = Some(Counterexample {
            events: Vec::new(),
            cycle: Vec::new(),
            failed_condition: format!("reachable({case})"),
        });
    } else {
        result.outcome = Outcome::Established;
        result.message = "all required claims and cases hold over the complete finite graph".into();
    }
    result
}

fn validate(
    requirements: &[Requirement],
    metadata: &ProtocolMetadata,
    model: &Model,
    reachable: &[&str],
    max_states: usize,
) -> Result<(), String> {
    if max_states == 0 || metadata.name != model.name {
        return Err(
            "positive state limit and matching generated protocol metadata required".into(),
        );
    }
    let required: BTreeSet<_> = requirements.iter().map(|r| r.id).collect();
    let claims: Vec<_> = model
        .invariants
        .iter()
        .map(|i| i.name.as_str())
        .chain(model.responses.iter().map(|p| p.name.as_str()))
        .collect();
    let actual: BTreeSet<_> = claims.iter().copied().collect();
    if required.len() != requirements.len()
        || actual.len() != claims.len()
        || required.iter().any(|id| id.is_empty())
    {
        return Err("required and protocol ids must be nonempty and unique".into());
    }
    if required != actual {
        return Err(format!(
            "contract coverage mismatch: missing claims {:?}; unrequired protocol claims {:?}",
            required.difference(&actual).collect::<Vec<_>>(),
            actual.difference(&required).collect::<Vec<_>>()
        ));
    }
    for r in requirements {
        if r.protocol != metadata.name
            || metadata
                .claims
                .iter()
                .any(|c| c.id == r.id && c.predicate != r.predicate)
        {
            return Err(format!(
                "requirement {} names a different protocol or predicate",
                r.id
            ));
        }
    }
    if reachable.iter().any(|c| !metadata.cases.contains(c))
        || reachable.iter().collect::<BTreeSet<_>>().len() != reachable.len()
    {
        return Err("require_reachable contains an unknown or duplicate Event::case".into());
    }
    if model
        .actions
        .iter()
        .any(|a| a.name.rsplit_once("::").is_none())
    {
        return Err("verification requires generated, case-labeled protocol actions".into());
    }
    Ok(())
}

fn counterexample(model: &Model, report: &Report, violation: &Violation) -> Counterexample {
    let events = |path: &[usize]| {
        path.windows(2)
            .map(|pair| {
                report.edges[pair[0]]
                    .iter()
                    .find(|edge| edge.to == pair[1])
                    .map_or_else(
                        || "Stutter".into(),
                        |edge| {
                            model.actions[edge.action]
                                .name
                                .rsplit_once("::")
                                .unwrap()
                                .0
                                .into()
                        },
                    )
            })
            .collect()
    };
    Counterexample {
        events: events(&violation.prefix),
        cycle: events(&violation.cycle),
        failed_condition: violation.property.clone(),
    }
}

impl VerificationResult {
    /// Retain non-test attributes as explicit potential waivers for baseline review.
    pub fn with_waivers(mut self, waivers: &[&str]) -> Self {
        self.scope
            .waivers
            .extend(waivers.iter().map(|s| (*s).into()));
        self
    }
    /// Only a complete established result is a passing test.
    pub fn is_established(&self) -> bool {
        self.outcome == Outcome::Established
    }

    /// Serialize all result axes as deterministic, dependency-free JSON.
    pub fn to_json(&self) -> String {
        object([
            ("outcome", string(self.outcome.as_str())),
            ("method", string(self.method)),
            ("contract", string(&self.contract)),
            (
                "required_ids",
                strings(self.required_ids.iter().map(String::as_str)),
            ),
            (
                "require_reachable",
                strings(self.require_reachable.iter().map(String::as_str)),
            ),
            ("protocol", string(&self.protocol)),
            (
                "scope",
                object([
                    (
                        "domains",
                        array(self.scope.domains.iter().map(|(name, variants)| {
                            object([
                                ("name", string(name)),
                                ("size", variants.len().to_string()),
                                ("variants", strings(variants.iter().map(String::as_str))),
                            ])
                        })),
                    ),
                    (
                        "bounds",
                        array(self.scope.bounds.iter().map(|b| {
                            object([
                                ("name", string(&b.name)),
                                ("min", b.min.to_string()),
                                ("max", b.max.to_string()),
                            ])
                        })),
                    ),
                    ("state_limit", self.scope.state_limit.to_string()),
                    (
                        "fairness_assumed",
                        strings(self.scope.fairness_assumed.iter().map(String::as_str)),
                    ),
                    (
                        "assumptions",
                        strings(self.scope.assumptions.iter().map(String::as_str)),
                    ),
                    (
                        "waivers",
                        strings(self.scope.waivers.iter().map(String::as_str)),
                    ),
                ]),
            ),
            ("implementation", string(self.implementation)),
            (
                "provenance",
                object([
                    ("source_hash", string(&self.provenance.source_hash)),
                    (
                        "verification_hash",
                        string(&self.provenance.verification_hash),
                    ),
                    (
                        "declaration",
                        object([
                            ("file", string(self.provenance.declaration.file)),
                            ("line", self.provenance.declaration.line.to_string()),
                            ("column", self.provenance.declaration.column.to_string()),
                        ]),
                    ),
                    (
                        "tool_versions",
                        object(
                            self.provenance
                                .tool_versions
                                .iter()
                                .map(|(k, v)| (k.as_str(), string(v))),
                        ),
                    ),
                ]),
            ),
            (
                "states",
                self.states.map_or_else(|| "null".into(), |n| n.to_string()),
            ),
            (
                "transitions",
                self.transitions
                    .map_or_else(|| "null".into(), |n| n.to_string()),
            ),
            (
                "reached_cases",
                strings(self.reached_cases.iter().map(String::as_str)),
            ),
            (
                "counterexample",
                self.counterexample.as_ref().map_or_else(
                    || "null".into(),
                    |c| {
                        object([
                            ("events", strings(c.events.iter().map(String::as_str))),
                            ("cycle", strings(c.cycle.iter().map(String::as_str))),
                            ("failed_condition", string(&c.failed_condition)),
                        ])
                    },
                ),
            ),
            ("message", string(&self.message)),
        ]) + "\n"
    }

    /// Write evidence below the selected Cargo target, including failing results.
    // This hosted development tool cannot depend on the application's IO runtime.
    #[allow(clippy::disallowed_methods)]
    pub fn write_to_target(&self, manifest: &str, test_name: &str) -> std::io::Result<PathBuf> {
        let target = std::env::var_os("BBOXY_CARGO_TARGET_DIR")
            .or_else(|| std::env::var_os("CARGO_TARGET_DIR"))
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                // Test executables live at target/<profile>/deps/<binary>, including
                // custom --target-dir selections and cross-compilation target roots.
                std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent()?.parent()?.parent().map(Path::to_path_buf))
                    .unwrap_or_else(|| Path::new(manifest).join("target"))
            });
        let directory = target.join("yarn-verification");
        std::fs::create_dir_all(&directory)?;
        let name: String = test_name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let path = directory.join(format!("{name}.json"));
        let temporary = directory.join(format!("{name}.{}.tmp", std::process::id()));
        std::fs::write(&temporary, self.to_json())?;
        std::fs::rename(temporary, &path)?;
        Ok(path)
    }
}

fn token_hash(source: &str) -> String {
    let hash = source.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    format!("fnv1a64:{hash:016x}")
}

// Adapted from bboxy450-term, crates/bbt-yarn/examples/verify.rs::json_string:
// https://github.com/bboxy450/bboxy450-term — shared deterministic JSON escaping.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
fn string(value: &str) -> String {
    let mut result = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            c if c < ' ' => write!(result, "\\u{:04x}", u32::from(c)).unwrap(),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

fn strings<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    array(values.into_iter().map(string))
}
fn array(values: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", values.into_iter().collect::<Vec<_>>().join(","))
}
fn object<'a>(values: impl IntoIterator<Item = (&'a str, String)>) -> String {
    format!(
        "{{{}}}",
        values
            .into_iter()
            .map(|(k, v)| format!("{}:{v}", string(k)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escaping_and_token_identity_are_stable() {
        assert_eq!(
            string("\"\\\n\t\u{0}λ"),
            "\"\\\"\\\\\\u000a\\u0009\\u0000λ\""
        );
        assert_eq!(token_hash(""), "fnv1a64:cbf29ce484222325");
        assert_eq!(token_hash("hello"), "fnv1a64:a430d84680aabd0b");
    }
}
