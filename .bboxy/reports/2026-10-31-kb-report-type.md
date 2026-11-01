---
id: 01M6AJ7VNY5G9N7E1RD4B4023N
number: 632
type: report
title: "Built-in KB report type with commit provenance"
task: "[[tasks/2026-10-31-kb-report-type]]"
commits: []
verify:
  command: make test-kb-reports
  exit_code: 0
author: seat:01M6AGAA0G0436ZEJAFB708CF1
created: "2026-11-01T03:25:22Z"
---
# Built-in KB report type and commit provenance

Author: Sable Embertail, seat:01M6AGAA0G0436ZEJAFB708CF1. Assignment: tasks/2026-10-31-kb-report-type.md. Worktree: /Users/feral/mydev/bboxy450-term-reports, branch kb-report-type.

## Files and mechanisms

- `crates/bbt-doc/src/reports.rs:11` validates the report date/path, task/supersedes link syntax, SHA list, verification command/exit, seat author and creation field. Existing Markdown parsing, Index allocation, provider search and link resolution remain the store and addressing paths.
- `crates/bbt-mux/src/mcp/knowledge.rs:295` grants worker report creation and refuses updates. `crates/bbt-mux/src/knowledge/effects.rs:221` checks the authenticated author, assigned task and correction target, captures the task reference, and rechecks its digest at publication. `crates/bbt-mux/src/index.rs:883` enforces append-only publication under its existing root lock and exclusive hard-link birth path.
- `crates/bbt-mux/src/mcp/work_items.rs:868` accepts an optional report locator, normalizes it to a wikilink and stores it as `report.report`. Admission and publication both resolve the actual linked report and its task. Existing unlinked task reports keep their acceptance behavior. The protocol, CLI and portable task contract DTO/schema expose the same optional field. The CLI grammar/help and its typed parser fixture also retain and constrain `--report`.
- `crates/bbt-doc/tests/corpus/reports.rs:147` checks trailer targets from each commit's own Git blobs and refuses report edits/deletions. The existing frontmatter gate calls it. Landing checks candidate commit messages after its existing gates and runs the committed-tree verifier for cuts with a Report trailer, including script-only cuts. `scripts/test-landing.py` checks that dispatch and failure propagation; cuts without a trailer retain their existing Cargo work. A later working-tree file cannot repair a dangling committed trailer. Standalone records checks validate `HEAD^!`; landing supplies the complete `SIGNED_RANGE`. The private Git fixture covers both scopes, including a legacy plain-path Report field before the candidate range.
- `crates/bboxy450-term/src/knowledge.rs:621` validates reports in the existing document loader. The new Make targets run the focused records/native/publication/CLI tests and strict touched-crate lint. The task brief is copied unchanged into this worktree so this report names its actual task in the same committed tree. The shipped bboxy450 skill and repository AGENTS Scope and evidence specify the report/commit/trailer/summary handoff.
- `verification/contracts.toml:466`, its document-crate coverage mapping, corpus and isolated `formal_contracts/report_models.rs` projection for ReportProvenance cover ownership, immutable publication, optional linked acceptance, stale preparation and A-B-A. Its broken model removes those guards. Atomic final validation/publication is an abstraction; external filesystem races after the last read, crash durability, cryptography and arbitrary Rust behavior are not proven by this finite model.

## Acceptance evidence

1. Type/create/fetch/search/numbered wikilink: `crates/bbt-mux/tests/mcp.rs:7476` drives the authenticated native tools. It also proves cross-seat and author-spoof refusal, append-only behavior and correction by supersedes.
2. Task link/acceptance: `crates/bbt-mux/tests/mcp.rs:7531` stores the report link, refuses missing and mismatched reports, and accepts the corrected matching link. `crates/bbt-mux/src/knowledge/effects.rs:432` checks task changes and report removal between preparation and publication.
3. Commit link: `crates/bbt-doc/tests/corpus/reports.rs:204` checks valid/no-trailer/dangling/non-report/malformed trailers. Its private Git fixture at line 235 checks the committed tree and append-only history. The deliberately removed-check experiment and restored positive run are recorded below.
4. Prompting: the repository instructions and embedded skill carry the short required workflow and immutable correction rule.
5. Formal/native controls and actual command results appear below; model bounds and unmodeled behavior are explicit in the contract.
6. This report is committed with the work and linked by the Report trailer. `commits: []` avoids an impossible self-referential commit SHA; the signed commit trailer supplies the containing commit's provenance.

## Commands and actual exits

- `make test-kb-reports` — exit 0; `target/kb-legacy-fix-restored.log`.
- `make test-task-contract` — exit 0; `target/kb-test-task-contract.log`.
- `make test-formal-contracts` — exit 0; `target/kb-legacy-fix-formal-contracts.log`.
- `make check-kb-reports` — exit 0; `target/kb-legacy-fix-clippy.log`.
- `make test-licenses` — exit 0; `target/kb-licenses.log`.
- Trailer-check removal control: `make test-kb-reports` — expected make exit 2 / Rust test exit 101, with exactly two dangling-trailer tests failing. Exact original source bytes were restored (SHA256 `715bc2d5517777e09f1ed7413abe88ef856c419783930d4922165e7e8c47a66b`); the complete restored focused suite then passed. `target/kb-trailer-mutant-final.log` and `.bboxy/runs/2026-10-31-kb-report-type.json` preserve the command/exit/hash summary.
- Native formal-contract tests: the final 8 passed, including all project models and broken controls. After rebasing onto the newer checker and moving the report projection into its own module, `make test-formal-contracts` again exited 0 (`target/kb-rebased-formal-contracts.log`). Java TLC graph/verdict parity is part of the pending final `make test` gate.
- Initial strict lint found two redundant unit expressions in the records test helper (make exit 2); they were removed and the required strict all-target check passed.

The initial focused run failed (make exit 2, Rust test exit 101): the fixture assumed number 2 despite its own numbered provenance records. The corrected test uses the Index's returned number; the subsequent focused run passed. No operator input was used.

The first exact-HEAD full CI attempt exited 2: twelve landing fixture assertions detected an unconditional extra Cargo gate. Limiting the verifier to Report-bearing cuts restored those cases. A subsequent landing fixture run caught additional pre-build overhead at its 0.5-second deadline; moving the optional check after the existing gates kept the deadline checks unchanged. `make test-landing` then exited 0 (134 fixtures, `target/kb-test-landing.log`). Initial failures remain in `target/kb-ci-initial.log` and `target/kb-test-landing-first-fix.log`.

The containing unpublished commit was rebased onto current main. Its new LAND guidance is followed: report models use an isolated module and the document crate's coverage row, without changing the shared family array or mux coverage entry. Final gates use a worktree-local private Cargo build directory. This report and its command summary are finalized with that first-publication source tree.

On the rebased signed candidate, exact-HEAD workspace build, strict Clippy, unit/integration tests and doctests passed, including the report MCP and committed-tree tests. The formal gate then rejected the new contract's check list: its inventory requires exactly `make test-formal` and `make test`, and I had also listed the focused target. Removing that extra inventory entry restored the required format without changing the tests or model. The failed full attempt (make exit 2) remains in `target/kb-ci-before-contract-fix.log`; full CI must run again on the corrected committed manifest.

Signed candidate `b3277331` completed `make test-task-contract && make test` with exit 0 (`target/kb-ci-verified-b327.log`), including 110 native/TLC models and private headless smoke. An earlier run of that same source failed only the 10 GiB free-disk check; byte/metadata-verified compression of this worktree's inactive cache restored capacity without changing a threshold or touching peers' files.

Main then advanced. The first author landing exited 78 on two independent formal-module registrations; both were preserved and the rebase was signed. The scoped landing on `53f25733` verified 112 models and the report corpus, but exited 77 when its worker reached the 900-second RUN bound (worker exit 124); it had no assertion failure and did not push. Its output is `target/kb-land-runtime-limit.log`. The final landing will use the canonical exact-commit full-normal-CI evidence option to avoid that scoped deadline.

After rebasing onto newer main, candidate `c85682bf` completed workspace build, strict Clippy, other unit/integration tests and doctests, but full CI exited 2: two corpus tests rejected three inherited main commits whose older Report fields name log paths. The default `HEAD` had unintentionally checked every ancestor. Standalone checks now use the current-commit range `HEAD^!`; presubmit retains its explicit complete `SIGNED_RANGE`. The private Git fixture verifies that inherited legacy fields do not invalidate a new valid tip, while an earlier dangling trailer inside the supplied candidate range still fails after a later report is added. `target/kb-ci-legacy-history.log` retains the failed output. After the fix, the focused suite (13 checks), native formal contracts (8 checks), strict touched-crate lint, and the repeated removed-check control all behaved as required.

## Final committed-source gates and limitations

This immutable report records verification completed before its containing commit. After committing, the required exact-HEAD `make test` and `sh scripts/land.sh` still have to run. Their actual exit statuses and resulting SHA belong in the final task handoff; `target/ci-local.log` and `target/kb-land.log` retain the raw local output. This paragraph does not claim those future gates passed.

The live seat initially advertised the older task_report schema without a report parameter. If it remains unchanged at handoff, the one-paragraph task_report summary carries this report's qualified wikilink, and the missing live optional parameter is named explicitly. No live application install/restart or unrelated task edits are part of this assignment.

No geometry changed; visual parity and OS process-boundary suites were not separately run. Normal final CI retains the workspace tests, formal gate, invariant checks and private headless smoke. Raw logs and fixture identities remain outside Git.
