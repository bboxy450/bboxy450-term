// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Built-in document dictionary. Paths are conventions; write authority remains host policy.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DocumentType {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub purpose: &'static str,
    pub path: &'static str,
    pub writing: &'static str,
}

pub const DOCUMENT_TYPES: &[DocumentType] = &[
    DocumentType {
        kind: "concern",
        purpose: "An ongoing area of care or responsibility that gives work its purpose. It records what is at stake and links the work and evidence that address it.",
        path: "concerns/<slug>.md",
        writing: "Name what matters, to whom, what is at stake, and what would show it is being addressed. Link related decisions, specs, epics, tasks and evidence; the concern may remain active after individual tasks finish. Requires Oracle authority.",
    },
    DocumentType {
        kind: "epic",
        purpose: "An outcome spanning several specifications and tasks. It groups the work needed to achieve that outcome.",
        path: "epics/<slug>.md",
        writing: "Describe the desired outcome and acceptance criteria; link child specs and tasks. Create open and unassigned with Oracle authority; use task_* for assignment and lifecycle.",
    },
    DocumentType {
        kind: "spec",
        purpose: "A design and behavioral contract for one concern. It states what must be built and how acceptance will be checked.",
        path: "specs/<slug>.md",
        writing: "Describe scope, behavior, constraints and acceptance criteria; cite the concern, decisions and research. Requires Oracle authority.",
    },
    DocumentType {
        kind: "task",
        purpose: "One scoped assignment with an owner, boundaries and a verification command. It tracks implementation through reporting and acceptance.",
        path: "tasks/<date>-<slug>.md",
        writing: "Provide the brief, allowed_paths, acceptance criteria and verify command. For a provider-allocated blank, open, unowned task, fetch its ref and etag, then initialize only its brief, verify and allowed_paths with knowledge_write; the host sets the delegator and swarm before task_delegate. Existing ownership or a populated brief cannot be adopted. Oracle creates open and unassigned; use task_delegate, task_report and task_accept for lifecycle. An assignee may update only its permitted report fields through knowledge_write.",
    },
    DocumentType {
        kind: "bug",
        purpose: "A reproducible defect in expected behavior. It connects the failure to affected specifications, invariants and repair work.",
        path: "bugs/<slug>.md",
        writing: "Record reproduction steps, expected and actual behavior, and evidence. Oracle creates open and unassigned; assignment, reports and lifecycle use task_*.",
    },
    DocumentType {
        kind: "note",
        purpose: "General knowledge worth retaining, including runbooks, observations and supporting evidence. Use it when a more specific document type does not fit.",
        path: "notes/<slug>.md",
        writing: "Write the useful context and sources; link related records. Workers and Oracles may create and update notes.",
    },
    DocumentType {
        kind: "invariant",
        purpose: "A property that must hold before and after changes. It includes an executable check and the cost of violating the property.",
        path: "invariants/<slug>.md",
        writing: "State the property, check command, successful exit status and cost of failure; link constrained specs. Requires Oracle authority.",
    },
    DocumentType {
        kind: "artifact",
        purpose: "A record of a produced file, commit, release or other output. It preserves responsibility, provenance and links to the work that produced it.",
        path: "artifacts/<slug>.md",
        writing: "Artifacts describe committed checkpoints, not work in progress. Use knowledge_commit for document checkpoints or commit source changes with Git. Generated artifact records and provenance are derived views; do not recursively checkpoint them. Use artifact_claim and artifact_release for responsibility; claims, observations and protected lineage fields cannot be set through generic knowledge_write.",
    },
    DocumentType {
        kind: "decision",
        purpose: "A durable record of a choice, its alternatives and rationale. It explains why the resulting design or policy exists.",
        path: "decisions/<slug>.md",
        writing: "Record alternatives, choice, reasons and date; link the affected proposal or spec. Create with this unnumbered path; the Index returns decisions/d<number>-<slug>.md. Requires Oracle authority.",
    },
    DocumentType {
        kind: "proposal",
        purpose: "A design offered for review before a decision is made. It records motivation, alternatives and unresolved questions.",
        path: "proposals/<slug>.md",
        writing: "Describe motivation, proposed design, alternatives and open questions; link review comments and the eventual decision. Requires Oracle authority.",
    },
    DocumentType {
        kind: "comment",
        purpose: "One review remark about a particular document version. It preserves a question, objection, suggestion or approval with its context.",
        path: "comments/<date>-<slug>.md",
        writing: "Set on to the reviewed document, on_digest to its digest, anchor to the reviewed section and kind to question, objection, suggestion or approval. Workers and Oracles may write comments.",
    },
    DocumentType {
        kind: "research",
        purpose: "Findings supported by precise source references. It supplies evidence for specifications and decisions.",
        path: "research/<slug>.md",
        writing: "Record file:line and source references with the facts they establish. Use parent/children for a research tree, derived_from for origins, and links with rel used or evidence for sources.",
    },
    DocumentType {
        kind: "recipe",
        purpose: "A reusable procedure for achieving a particular purpose with a tool or workflow. It gives people and harnesses practical steps, examples and a way to check the result.",
        path: "recipes/<slug>.md",
        writing: "State the goal, prerequisites, inputs, steps, examples, expected result and verification. Name the tools and permissions needed, plus common failures and recovery steps. A recipe may describe any tool or workflow; reading it grants no execution authority. Requires Oracle authority to write through MCP.",
    },
    DocumentType {
        kind: "skill",
        purpose: "Repository-crafted guidance a harness loads into its context for a kind of work. Served by skill_list, skill_search and skill_read, with signed revisions and provenance like any document, so skills can be improved from evidence.",
        path: "skills/<name>/SKILL.md",
        writing: "Agent Skills format: frontmatter name (lowercase words joined by hyphens, equal to the directory name) and description (when to use it), then concise Markdown instructions. Link the reports or research that motivated a revision with derived_from. The repository links .agents/skills to .bboxy/skills so every harness sees the same skills. A skill grants no authority. Requires Oracle authority to write through MCP.",
    },
    DocumentType {
        kind: "e2e",
        purpose: "An end-to-end acceptance record for one promised user-facing function. It records the procedure, observed result and coverage gaps.",
        path: "e2e/<slug>.md",
        writing: "Provide function, group, steps, expected, status, coverage, last_run, result, run_by and evidence. Name missing checks for partial coverage and link failing or partial results to repair tasks.",
    },
    DocumentType {
        kind: "report",
        purpose: "An append-only record of completed work and its verification evidence. Corrections preserve the earlier report and identify what they supersede.",
        path: "reports/YYYY-MM-DD-<slug>.md",
        writing: "Set task to a [[task locator]] or #N, author to your authenticated seat:ULID (never its display name), created to a timestamp such as 2026-11-05T15:00:00Z, commits to a list of full commit SHAs (possibly empty), and verify to an object with command and integer exit_code. Record changes, evidence and gaps in the body; use task_report to hand it back. Create a new report with supersedes for corrections.",
    },
    DocumentType {
        kind: "interface",
        purpose: "A contract between components or subsystems. It defines the data, operations and obligations at their boundary.",
        path: "interfaces/<slug>.md",
        writing: "Identify the participants, data and operation shapes, ownership, ordering, errors and compatibility rules. Link the relevant specs and verification evidence. Requires Oracle authority.",
    },
];
