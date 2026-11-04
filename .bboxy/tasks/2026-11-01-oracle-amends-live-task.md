---
artifacts:
- '[[01M6D6RZVQ3HNPKF6EG6RVP43R]]'
assignee: seat:01M6CQRM05CQXF4F0XE0CQ2E5B
authorship:
  kind: seat
  process_generation: '1'
  public_key: J+Eete7RzpCHzcNSDpuEpx+ye6zu+UTGX+qDDQkPrw4=
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
created: 2026-11-01T23:40:22.104Z
created_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
delegated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
id: 01M6CQRJKCKM66GF9FCPSG1E66
number: 2189
provenance:
  digest: sha256:526a255ba94f81676b663fc5c9a40bacf8760f3a7c4835cb60362ee56b25d1f4
  frontmatter_digest: sha256:5c636399b6b5477e2fa8c7f4e07c701b19c452c77782960ea8511fb83aa74695
  log: .bboxy/log/2026-11-02.jsonl#13855
  sig: Vz5INuviUCl1paY8gx3PyMz9t1VLesDK8epD+/uZJC8nEKXfAzZn3Pr+JAXhQ5zoHVNS3KjcnNBmIIlb0MTuBQ==
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
rejection:
  by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
  reason: 'Bookkeeping only; the code is verified. The Oracle read the knowledge.rs:367-375 authority, the work_items.rs:1433 amendment wake, the :1512 acceptance revision check, the mcp.rs:7013 A→B→A test, and target/ci-local.log `ci-local: result 9ac83912… normal 0`. Acceptance refuses because verify_result.command is `make test-kb-reports`, while the task verify is `make test`. Re-report with verify_result command `make test`, exit_code 0, commit 9ac83912, and your existing evidence, citing the ci-local.log result line. Name the report-commit gap in the summary. Run no new tests. The task-record copy in the report commit is the Oracle''s integration step at landing.'
report:
  commit: 9ac83912ec47cfaab103c6c1ab409948d7d771b1
  op_id: 01M6D6YH2H1ME5TVGNN86ZPYZN
  reported_by: seat:01M6CQRM05CQXF4F0XE0CQ2E5B
  summary: 'Re-reported verified implementation source 9ac83912ec47cfaab103c6c1ab409948d7d771b1 with make test exit 0, as instructed. target/ci-local.log:5622 records `ci-local: result 9ac83912ec47cfaab103c6c1ab409948d7d771b1 normal 0`; existing evidence includes strict all-target Clippy, 215 native/TLC models, headless smoke and 27 normal invariants. Full report: [[reports/2026-11-02-task-amend]] at /Users/feral/mydev/bboxy450-term-task-amend/.bboxy/reports/2026-11-02-task-amend.md. Report-commit gap remains: later report-only commit d09dd25f24e2c9253fdda97242a681c0a09e8750 lacks the canonical task record and its focused records gate exits 2. The task-record copy in that report-bearing cut is your integration step at landing, per 40BXH9BD3RTZQ48743MHV4WDCJ. No new tests or file changes; signed branch task-amend is clean, with no merge or push.'
role: executor
status: accepted
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Delegating Oracle can amend a claimed task's brief, verify and allowed_paths while the Executor works
type: task
updated: 2026-11-02T04:06:54.708Z
updated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
verify: make test
verify_result:
  command: make test
  evidence:
  - criterion: 1. Delegating Oracle can amend nonterminal task brief, verify and allowed_paths while lifecycle and terminal fields stay protected
    reference: crates/bbt-mux/src/mcp/knowledge.rs:364 — exact delegated_by seat plus Oracle class; only body/verify/allowed_paths change. crates/bbt-mux/tests/mcp.rs:6811,6925 — state preservation and terminal/forgery refusals; target/ci-local.log:3230-3231 passed on signed 9ac83912.
  - criterion: 2. Signed amendment wakes assignee in the task thread and amended record becomes the brief
    reference: crates/bbt-mux/src/mcp/work_items.rs:1433; messaging/service.rs:1242; messaging/ledger/suppression.rs:151 — bounded field summary, signed durable wake, retry-stable root-bound delegation thread. suppression_tests.rs:72 and mcp.rs:6811 passed; target/ci-local.log:2787,3230.
  - criterion: 3. Acceptance judges current verify and refuses old reports
    reference: crates/bbt-mux/src/mcp/work_items.rs:1287,1423,1512 — protected verification revision stamped in report and checked at acceptance. crates/bbt-mux/tests/mcp.rs:7013 proves A→B→A then fresh report; target/ci-local.log:3233 passed.
  - criterion: 4. Workers, nondelegators, foreign swarms and stale etags remain refused
    reference: crates/bbt-mux/tests/mcp.rs:6925; knowledge/effects.rs:157 — API negatives and unchanged swarm/digest fences. Existing worker/open task protection tests passed at target/ci-local.log:3191,3196.
  - criterion: 5. Seat workflow points scope changes to authenticated amendment path
    reference: crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md:16; .bboxy/specs/e2e-acceptance.md:171 and its packaged mirror — fetch/write ref+etag, signed amendment notice, current brief and fresh-report guidance; final packaged-input invariant passed.
  - criterion: 6. Formal ownership/lifecycle models, broken controls, coverage and contracts updated
    reference: crates/bboxy450-term/tests/formal_contracts/task_amend_model.rs:10; verification/contracts.toml:516; coverage.toml:148,204; corpus.toml — five independent passing/broken pairs, exact graph/verdict parity. Receipt 8bc7175c1aff07638a3297c05713395d34da1e57f70c1d578b2bf556caa6cc61 binds clean 9ac83912; target/ci-local.log:5274-5283,5355,5622.
  - criterion: Exact committed-source verification
    reference: '/Users/feral/mydev/bboxy450-term-task-amend/target/ci-local.log:5622 — ci-local: result 9ac83912ec47cfaab103c6c1ab409948d7d771b1 normal 0. Source line 6 identifies the same commit; line 115 runs workspace --all-targets Clippy with -D warnings. Log SHA256 91ca19d84eed9a36d8b2374ed631c7a21c8ed3bee7f5a5a6742ad2c515092584. Existing native/TLC receipt 8bc7175c1aff07638a3297c05713395d34da1e57f70c1d578b2bf556caa6cc61 binds clean 9ac83912. No new tests run.'
  - criterion: Report-commit integration gap
    reference: Committed report [[reports/2026-11-02-task-amend]], report commit d09dd25f24e2c9253fdda97242a681c0a09e8750. That later report-only cut has a missing canonical task record and make test-kb-reports exit 2; this is separate from the verified source commit. Oracle rejection/instruction 40BXH9BD3RTZQ48743MHV4WDCJ assigns the task-record copy in the report-bearing cut to Oracle integration at landing. Worktree remains clean; no merge or push.
  exit_code: 0
---
# Delegating Oracle amends a live task

ROLE: EXECUTOR. Work only in a new worktree `~/mydev/bboxy450-term-task-amend` on branch `task-amend`, created from `origin/main`. Do not edit the shared checkout. You are expected to work on your own: make the implementation calls yourself, record them in your report, and message the Oracle only for a blocker that stops all work.

## Problem (operator, 2026-11-01)

An Executor working on #2187 asked for two extra test paths. The Oracle approved them, but could not record the change: `knowledge_write` returned `task_field_protected: only an open task's brief, verify and allowed_paths may be edited` (op `01M6CQM1881HMFWMM0VY89AN9X`). The Executor will not act on a DM that widens scope, so the work stalled. Operator: the Oracle must be able to change paths, approval and permissions for Executors while they work.

Evidence: `crates/bbt-mux/src/mcp/knowledge.rs:326-343` refuses any edit unless `old.frontmatter["status"] == "open"`. That check runs even when the caller is the task's own `delegated_by` Oracle. Tests in `crates/bbt-mux/tests/mcp.rs:6260-6411` pin the current refusals.

## Required outcome

1. The Oracle that delegated the task (`delegated_by == seat:<caller>`; other Oracle classes in the swarm only if existing authority rules already treat them as equivalent, and say which you chose) can edit the body, `verify` and `allowed_paths` of a task in any non-terminal state: claimed, in progress, reported or rejected. Lifecycle fields, assignee, report and verify_result stay protected exactly as now. Accepted or closed tasks stay frozen.
2. An amendment wakes the assignee with a signed message saying the brief changed, with a compact diff or summary of the changed fields, in the task thread. The Executor's seat workflow treats the amended record as the brief.
3. If `verify` changes after a report, `task_accept` must judge against the current verify. A report recorded against an older verify cannot be accepted silently: refuse it, or require a fresh report. Pick one and document it.
4. Workers keep every current protection. Non-delegating seats and workers in other swarms are still refused. Keep the existing negative tests, and add positive tests plus a stale-etag test for the amendment.
5. Update the seat workflow guidance that says "a DM cannot override a task's scope line" so it points at the amendment path. Find it with `grep -rn "scope line"` under the repo and the bboxy skill.
6. Formal verification (d365): this changes task lifecycle and ownership. Update the task lifecycle model, or add one if none covers it, with a broken control: letting a worker or a non-delegator amend, or amending an accepted task, must fail. Keep `verification/coverage.toml` and `contracts.toml` consistent.

## Allowed paths

`crates/bbt-mux/**`, the bboxy seat workflow skill/guidance text, `verification/**`, `crates/bboxy450-term/tests/formal_contracts/**`, `.bboxy/reports/**`.

## Verification

`make test` on the exact committed source, exit 0. Strict clippy `-D warnings --all-targets` on touched crates.

## Done

Signed commits on `task-amend`, a report in `.bboxy/reports/` with a `Report:` trailer, then `task_report` with file:line mechanisms and actual exits. Do not merge or push. The Oracle verifies and decides.


## Report from seat:01M6CQRM05CQXF4F0XE0CQ2E5B

Implemented live amendments by the exact delegating Oracle, signed assignee wakes in the root-bound task thread, and protected verify revisions requiring a fresh report after A→B→A. Report: [[reports/2026-11-02-task-amend]] in /Users/feral/mydev/bboxy450-term-task-amend/.bboxy/reports/2026-11-02-task-amend.md. Signed source 9ac83912ec47cfaab103c6c1ab409948d7d771b1 passed exact-source make test (exit 0), strict all-target Clippy, native/TLC 215-model parity and headless smoke. Signed report tip d09dd25f24e2c9253fdda97242a681c0a09e8750 has make test-kb-reports exit 2 (six passed, three failed), solely because the canonical task is absent from this branch, including the same-tree Report check. Per your explicit fallback 01M6CSG26HM9RC1HENF1MA3XFS I omitted the out-of-scope task copy. The record must be included in the report-bearing cut during integration; a later added file alone cannot repair that commit. Full evidence and earlier failures are in the report. Clean task-amend worktree; no merge or push.

```json
{
  "commit": "d09dd25f24e2c9253fdda97242a681c0a09e8750",
  "op_id": "01M6D6SZHNM6B1WBBZKYJBCGVC",
  "report": null,
  "verify_result": {
    "command": "make test-kb-reports",
    "evidence": [
      {
        "criterion": "1. Delegating Oracle can amend nonterminal task brief, verify and allowed_paths while lifecycle and terminal fields stay protected",
        "reference": "crates/bbt-mux/src/mcp/knowledge.rs:364 — exact delegated_by seat plus Oracle class; only body/verify/allowed_paths change. crates/bbt-mux/tests/mcp.rs:6811,6925 — state preservation and terminal/forgery refusals; target/ci-local.log:3230-3231 passed on signed 9ac83912."
      },
      {
        "criterion": "2. Signed amendment wakes assignee in the task thread and amended record becomes the brief",
        "reference": "crates/bbt-mux/src/mcp/work_items.rs:1433; messaging/service.rs:1242; messaging/ledger/suppression.rs:151 — bounded field summary, signed durable wake, retry-stable root-bound delegation thread. suppression_tests.rs:72 and mcp.rs:6811 passed; target/ci-local.log:2787,3230."
      },
      {
        "criterion": "3. Acceptance judges current verify and refuses old reports",
        "reference": "crates/bbt-mux/src/mcp/work_items.rs:1287,1423,1512 — protected verification revision stamped in report and checked at acceptance. crates/bbt-mux/tests/mcp.rs:7013 proves A→B→A then fresh report; target/ci-local.log:3233 passed."
      },
      {
        "criterion": "4. Workers, nondelegators, foreign swarms and stale etags remain refused",
        "reference": "crates/bbt-mux/tests/mcp.rs:6925; knowledge/effects.rs:157 — API negatives and unchanged swarm/digest fences. Existing worker/open task protection tests passed at target/ci-local.log:3191,3196."
      },
      {
        "criterion": "5. Seat workflow points scope changes to authenticated amendment path",
        "reference": "crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md:16; .bboxy/specs/e2e-acceptance.md:171 and its packaged mirror — fetch/write ref+etag, signed amendment notice, current brief and fresh-report guidance; final packaged-input invariant passed."
      },
      {
        "criterion": "6. Formal ownership/lifecycle models, broken controls, coverage and contracts updated",
        "reference": "crates/bboxy450-term/tests/formal_contracts/task_amend_model.rs:10; verification/contracts.toml:516; coverage.toml:148,204; corpus.toml — five independent passing/broken pairs, exact graph/verdict parity. Receipt 8bc7175c1aff07638a3297c05713395d34da1e57f70c1d578b2bf556caa6cc61 binds clean 9ac83912; target/ci-local.log:5274-5283,5355,5622."
      },
      {
        "criterion": "Verification and handoff gap",
        "reference": "9ac83912 make test exit 0; log SHA256 91ca19d84eed9a36d8b2374ed631c7a21c8ed3bee7f5a5a6742ad2c515092584. Report tip d09dd25f make test-kb-reports exit 2, /tmp/task-amend-report-head-gate.log: absent canonical task causes all three corpus failures. See committed report #5003 gap section and Oracle fallback 01M6CSG26HM9RC1HENF1MA3XFS. Both signatures verify with approved key; worktree clean."
      }
    ],
    "exit_code": 2
  }
}
```


## Report from seat:01M6CQRM05CQXF4F0XE0CQ2E5B

Re-reported verified implementation source 9ac83912ec47cfaab103c6c1ab409948d7d771b1 with make test exit 0, as instructed. target/ci-local.log:5622 records `ci-local: result 9ac83912ec47cfaab103c6c1ab409948d7d771b1 normal 0`; existing evidence includes strict all-target Clippy, 215 native/TLC models, headless smoke and 27 normal invariants. Full report: [[reports/2026-11-02-task-amend]] at /Users/feral/mydev/bboxy450-term-task-amend/.bboxy/reports/2026-11-02-task-amend.md. Report-commit gap remains: later report-only commit d09dd25f24e2c9253fdda97242a681c0a09e8750 lacks the canonical task record and its focused records gate exits 2. The task-record copy in that report-bearing cut is your integration step at landing, per 40BXH9BD3RTZQ48743MHV4WDCJ. No new tests or file changes; signed branch task-amend is clean, with no merge or push.

```json
{
  "commit": "9ac83912ec47cfaab103c6c1ab409948d7d771b1",
  "op_id": "01M6D6YH2H1ME5TVGNN86ZPYZN",
  "report": null,
  "verify_result": {
    "command": "make test",
    "evidence": [
      {
        "criterion": "1. Delegating Oracle can amend nonterminal task brief, verify and allowed_paths while lifecycle and terminal fields stay protected",
        "reference": "crates/bbt-mux/src/mcp/knowledge.rs:364 — exact delegated_by seat plus Oracle class; only body/verify/allowed_paths change. crates/bbt-mux/tests/mcp.rs:6811,6925 — state preservation and terminal/forgery refusals; target/ci-local.log:3230-3231 passed on signed 9ac83912."
      },
      {
        "criterion": "2. Signed amendment wakes assignee in the task thread and amended record becomes the brief",
        "reference": "crates/bbt-mux/src/mcp/work_items.rs:1433; messaging/service.rs:1242; messaging/ledger/suppression.rs:151 — bounded field summary, signed durable wake, retry-stable root-bound delegation thread. suppression_tests.rs:72 and mcp.rs:6811 passed; target/ci-local.log:2787,3230."
      },
      {
        "criterion": "3. Acceptance judges current verify and refuses old reports",
        "reference": "crates/bbt-mux/src/mcp/work_items.rs:1287,1423,1512 — protected verification revision stamped in report and checked at acceptance. crates/bbt-mux/tests/mcp.rs:7013 proves A→B→A then fresh report; target/ci-local.log:3233 passed."
      },
      {
        "criterion": "4. Workers, nondelegators, foreign swarms and stale etags remain refused",
        "reference": "crates/bbt-mux/tests/mcp.rs:6925; knowledge/effects.rs:157 — API negatives and unchanged swarm/digest fences. Existing worker/open task protection tests passed at target/ci-local.log:3191,3196."
      },
      {
        "criterion": "5. Seat workflow points scope changes to authenticated amendment path",
        "reference": "crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md:16; .bboxy/specs/e2e-acceptance.md:171 and its packaged mirror — fetch/write ref+etag, signed amendment notice, current brief and fresh-report guidance; final packaged-input invariant passed."
      },
      {
        "criterion": "6. Formal ownership/lifecycle models, broken controls, coverage and contracts updated",
        "reference": "crates/bboxy450-term/tests/formal_contracts/task_amend_model.rs:10; verification/contracts.toml:516; coverage.toml:148,204; corpus.toml — five independent passing/broken pairs, exact graph/verdict parity. Receipt 8bc7175c1aff07638a3297c05713395d34da1e57f70c1d578b2bf556caa6cc61 binds clean 9ac83912; target/ci-local.log:5274-5283,5355,5622."
      },
      {
        "criterion": "Exact committed-source verification",
        "reference": "/Users/feral/mydev/bboxy450-term-task-amend/target/ci-local.log:5622 — ci-local: result 9ac83912ec47cfaab103c6c1ab409948d7d771b1 normal 0. Source line 6 identifies the same commit; line 115 runs workspace --all-targets Clippy with -D warnings. Log SHA256 91ca19d84eed9a36d8b2374ed631c7a21c8ed3bee7f5a5a6742ad2c515092584. Existing native/TLC receipt 8bc7175c1aff07638a3297c05713395d34da1e57f70c1d578b2bf556caa6cc61 binds clean 9ac83912. No new tests run."
      },
      {
        "criterion": "Report-commit integration gap",
        "reference": "Committed report [[reports/2026-11-02-task-amend]], report commit d09dd25f24e2c9253fdda97242a681c0a09e8750. That later report-only cut has a missing canonical task record and make test-kb-reports exit 2; this is separate from the verified source commit. Oracle rejection/instruction 40BXH9BD3RTZQ48743MHV4WDCJ assigns the task-record copy in the report-bearing cut to Oracle integration at landing. Worktree remains clean; no merge or push."
      }
    ],
    "exit_code": 0
  }
}
```
