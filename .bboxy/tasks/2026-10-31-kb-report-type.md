---
artifacts:
- '[[01M6B1RFBMM5NSSPM36VDFF7F6]]'
assignee: seat:01M6AGAA0G0436ZEJAFB708CF1
authorship:
  kind: seat
  process_generation: '1'
  public_key: dNWbVeHRHyt/TnV6QHFyDXtWIj+UElZjvaWtEgWhcFg=
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
created: 2026-11-01T02:51:44.027Z
created_by: seat:01M6ABH71TVWGZ621NTD701PSH
delegated_by: seat:01M6ABH71TVWGZ621NTD701PSH
id: 01M6AGA8G9C5MXP3WTTN5QBY5M
number: 631
provenance:
  digest: sha256:060242542f46bfa7057ab3b17561d269206b3a5d5ee537f8d6cd0f9d2ec00e77
  frontmatter_digest: sha256:080068bf05a3fd5f2116699e3926737bfceb17881ff94af05dfce4c7629501f8
  log: .bboxy/log/2026-11-01.jsonl#5969
  sig: tRszoDT78/KmWQDi99Gcl2T+HivDMZGutRdFTdz9cMZ1vJSHaBSvV8njj9fUlrLn+Dhko2c210BX3Mm9bjGNBw==
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
report:
  commit: ee6b7d5ed5b5c2a412ba8352478a052cd63f47cd
  op_id: 01M6B1TVMNBP5HD221SBC78GBZ
  reported_by: seat:01M6AGAA0G0436ZEJAFB708CF1
  summary: 'Implemented and landed ee6b7d5ed5b5c2a412ba8352478a052cd63f47cd; full report: [[reports/2026-10-31-kb-report-type]]. Exact-commit make test-task-contract && make test exited 0, including strict workspace Clippy, unattended headless smoke and native/TLC parity for 114 models across 31 crates and 47 contracts. Author landing with --gate-log exited 0; remote main matches, the approved Boxy signature verifies, and this worktree is clean. The running MCP schema still lacks the optional report argument, so the report link is supplied in this summary; application installation/restart and separate parity/boundary suites were outside this brief.'
role: executor
status: accepted
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Built-in KB: report document type, linked from commits by wikilink'
type: task
updated: 2026-11-01T07:58:08.517Z
updated_by: seat:01M6ABH71TVWGZ621NTD701PSH
verify: make test-task-contract && make test
verify_result:
  command: make test-task-contract && make test
  evidence:
  - criterion: 1. First-class report type, existing Index/store, own-task creation and immutable corrections
    reference: crates/bbt-doc/src/reports.rs:11; crates/bbt-mux/src/knowledge/effects.rs:221; crates/bbt-mux/src/index.rs:883; crates/bbt-mux/tests/mcp.rs:7476. target/ci-local.log:2743 — authenticated create/fetch/type:report search, foreign-task/author refusal, append-only and supersedes assertions passed.
  - criterion: 2. Optional task report locator and linked acceptance guards
    reference: crates/bbt-mux/src/mcp/work_items.rs:1419; protocol/portable task DTO/schema and CLI --report updated. target/ci-local.log:2744 — missing/mismatched report refusal and matching acceptance passed; line 2190 — publication refetch rejects stale task and removed report.
  - criterion: 3. Commit Report trailer and task resolution in the same committed tree
    reference: crates/bbt-doc/tests/corpus/reports.rs:147 and :176; scripts/landing.py:857. Current-commit and explicit complete candidate-range Git fixtures passed, including legacy history and an earlier dangling link hidden by a later file. target/kb-trailer-mutant-final.log — removed guard caused exactly two failures, make exit 2/Rust 101; source restored byte-for-byte SHA256 715bc2d5517777e09f1ed7413abe88ef856c419783930d4922165e7e8c47a66b; restored focused suite exit 0.
  - criterion: 4. Short executor workflow in shipped skill and repository instructions
    reference: crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md:39; AGENTS.md:87 — worktree report, same commit, Report wikilink trailer, summary plus locator, supersedes correction.
  - criterion: 5. Native, formal and unattended verification
    reference: 'make test-kb-reports, make test-formal-contracts and make check-kb-reports all exited 0 (target/kb-legacy-fix-{restored,formal-contracts,clippy}.log). Exact-HEAD full normal CI exited 0, SHA256 9fec221d879b05f205226851ce08dcc31696a5466141cdf404fc0d4cb22ad24b; 135 landing fixtures, workspace tests/doctests, strict lint, private headless smoke, 27 invariant checks/0 failures/11 normal slow skips. verification/contracts.toml:466 and coverage.toml:190 map ReportProvenance: 64 states/263 edges pass; broken control 144 states/725 edges safety. Formal receipt fcd8c864ae3b30c424bda121516717aa94ee592d93717c30f4c801470a9d8729 binds clean ee6b7d5e, 114 models/31 crates/47 contracts.'
  - criterion: 6. Committed dogfood report and signed landing provenance
    reference: '.bboxy/reports/2026-10-31-kb-report-type.md, SHA256 fb6c461998149e16964d27e5b460ed150370ec73ebd14bcad3a9d1f83b5aed91; Report: [[reports/2026-10-31-kb-report-type]] in signed commit ee6b7d5e. artifact_claim 01M6B1RFBMM5NSSPM36VDFF7F6 links its exact GitHub blob and digest. sh scripts/land.sh --gate-log /Users/feral/mydev/bboxy450-term-reports/target/ci-local.log exited 0; landing receipt tested_sha == landed SHA, retries 0, approved key 9B07E849480C68328CA20684844ECC0587DE7AB0. git ls-remote confirms main; git status --short empty.'
  - criterion: Failures, coverage limits and live-tool gap
    reference: The committed report and .bboxy/runs/2026-10-31-kb-report-type.json record earlier fixes, disk recovery and the timed scoped landing. A later same-HEAD deadline-classification fixture failed once (target/kb-ci-deadline-fixture.log, CI exit 2); controlled same-HEAD rerun passed all 135 fixtures and full CI, with no threshold change. Raw logs stay outside Git. No separate parity/boundary run or live application installation; current task_report schema has no report parameter, so no unsupported argument was sent.
  exit_code: 0
---
# Built-in KB: `report` document type with commit provenance

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-reports`, branch `kb-report-type` from origin/main 302e61ce. Only this worktree.

Operator, 2026-10-31: "our built-in KB should also support report as a doc type, so executor … submits full report, it commits, attaches report as a wikilink in commit, that way everything has provenance."

## Required outcome
1. **Type.** `report` is a first-class built-in KB type at `.bboxy/reports/<YYYY-MM-DD>-<slug>.md`, numbered by the Index like decisions. Frontmatter: `type: report`, `task` (wikilink/#N of the task), `commits` (list of SHAs, may be empty at write time), `verify` {command, exit_code}, `author` (seat), `created`. Body = the full report (files+mechanism, commands+exits, evidence per criterion, gaps). Search `type:report`, fetch, wikilink resolution and the records format gate all know it. Worker-class seats may create reports for their own assigned task; a report is append-only after creation (corrections are a new report that `supersedes:` the old one). Follow the existing decision/task code paths in `crates/bbt-mux/src/knowledge/`, `crates/bbt-mux/src/mcp/knowledge.rs`, `crates/bboxy450-term/src/knowledge.rs`; no parallel store.
2. **Task link.** `task_report` takes an optional `report` locator; when present the task record stores the link and `task_accept` refuses if the linked report does not exist or names a different task. Update tool schemas/descriptions.
3. **Commit link.** Commit message trailer `Report: [[<report-slug>]]`. The records gate (and `scripts/land.sh` presubmit) fails a commit whose `Report:` trailer does not resolve to a report in the same tree; a report's `task` must resolve too. Commits without a trailer stay valid.
4. **Prompting.** Update the shipped skill `crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md` and repo `AGENTS.md` (Scope and evidence) to the workflow: write the report in your worktree under `.bboxy/reports/`, commit it with the work, add the `Report:` trailer, then `task_report` with a one-paragraph summary + `report` link. Keep it short (the shared rule is already in `~/.agents/AGENTS.md`).
5. **Proof.** Tests: create/fetch/search a report; worker can't write a report for another seat's task; append-only refusal; `task_accept` refuses a missing/mismatched report; records gate refuses a dangling `Report:` trailer (broken control: remove the check → test fails). Formal: if task acceptance/lifecycle changes, update its model in `verification/` with a negative control per AGENTS.md; add the coverage row.
6. **Dogfood.** Your own final report is `.bboxy/reports/2026-10-31-kb-report-type.md`, committed with the work and linked by trailer.

## Verify
`make test-task-contract`, `make test-formal-contracts`, strict clippy on touched crates, `make test` on the final committed source. Land with `sh scripts/land.sh`.

## Communication (operator order: no token burn)
No acknowledgements, progress DMs or channel posts. One `task_report` at the end. Message the Oracle mid-task only for a blocker that stops all work, ≤3 lines.


## Report from seat:01M6AGAA0G0436ZEJAFB708CF1

Implemented and landed ee6b7d5ed5b5c2a412ba8352478a052cd63f47cd; full report: [[reports/2026-10-31-kb-report-type]]. Exact-commit make test-task-contract && make test exited 0, including strict workspace Clippy, unattended headless smoke and native/TLC parity for 114 models across 31 crates and 47 contracts. Author landing with --gate-log exited 0; remote main matches, the approved Boxy signature verifies, and this worktree is clean. The running MCP schema still lacks the optional report argument, so the report link is supplied in this summary; application installation/restart and separate parity/boundary suites were outside this brief.

```json
{
  "commit": "ee6b7d5ed5b5c2a412ba8352478a052cd63f47cd",
  "op_id": "01M6B1TVMNBP5HD221SBC78GBZ",
  "verify_result": {
    "command": "make test-task-contract && make test",
    "evidence": [
      {
        "criterion": "1. First-class report type, existing Index/store, own-task creation and immutable corrections",
        "reference": "crates/bbt-doc/src/reports.rs:11; crates/bbt-mux/src/knowledge/effects.rs:221; crates/bbt-mux/src/index.rs:883; crates/bbt-mux/tests/mcp.rs:7476. target/ci-local.log:2743 — authenticated create/fetch/type:report search, foreign-task/author refusal, append-only and supersedes assertions passed."
      },
      {
        "criterion": "2. Optional task report locator and linked acceptance guards",
        "reference": "crates/bbt-mux/src/mcp/work_items.rs:1419; protocol/portable task DTO/schema and CLI --report updated. target/ci-local.log:2744 — missing/mismatched report refusal and matching acceptance passed; line 2190 — publication refetch rejects stale task and removed report."
      },
      {
        "criterion": "3. Commit Report trailer and task resolution in the same committed tree",
        "reference": "crates/bbt-doc/tests/corpus/reports.rs:147 and :176; scripts/landing.py:857. Current-commit and explicit complete candidate-range Git fixtures passed, including legacy history and an earlier dangling link hidden by a later file. target/kb-trailer-mutant-final.log — removed guard caused exactly two failures, make exit 2/Rust 101; source restored byte-for-byte SHA256 715bc2d5517777e09f1ed7413abe88ef856c419783930d4922165e7e8c47a66b; restored focused suite exit 0."
      },
      {
        "criterion": "4. Short executor workflow in shipped skill and repository instructions",
        "reference": "crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md:39; AGENTS.md:87 — worktree report, same commit, Report wikilink trailer, summary plus locator, supersedes correction."
      },
      {
        "criterion": "5. Native, formal and unattended verification",
        "reference": "make test-kb-reports, make test-formal-contracts and make check-kb-reports all exited 0 (target/kb-legacy-fix-{restored,formal-contracts,clippy}.log). Exact-HEAD full normal CI exited 0, SHA256 9fec221d879b05f205226851ce08dcc31696a5466141cdf404fc0d4cb22ad24b; 135 landing fixtures, workspace tests/doctests, strict lint, private headless smoke, 27 invariant checks/0 failures/11 normal slow skips. verification/contracts.toml:466 and coverage.toml:190 map ReportProvenance: 64 states/263 edges pass; broken control 144 states/725 edges safety. Formal receipt fcd8c864ae3b30c424bda121516717aa94ee592d93717c30f4c801470a9d8729 binds clean ee6b7d5e, 114 models/31 crates/47 contracts."
      },
      {
        "criterion": "6. Committed dogfood report and signed landing provenance",
        "reference": ".bboxy/reports/2026-10-31-kb-report-type.md, SHA256 fb6c461998149e16964d27e5b460ed150370ec73ebd14bcad3a9d1f83b5aed91; Report: [[reports/2026-10-31-kb-report-type]] in signed commit ee6b7d5e. artifact_claim 01M6B1RFBMM5NSSPM36VDFF7F6 links its exact GitHub blob and digest. sh scripts/land.sh --gate-log /Users/feral/mydev/bboxy450-term-reports/target/ci-local.log exited 0; landing receipt tested_sha == landed SHA, retries 0, approved key 9B07E849480C68328CA20684844ECC0587DE7AB0. git ls-remote confirms main; git status --short empty."
      },
      {
        "criterion": "Failures, coverage limits and live-tool gap",
        "reference": "The committed report and .bboxy/runs/2026-10-31-kb-report-type.json record earlier fixes, disk recovery and the timed scoped landing. A later same-HEAD deadline-classification fixture failed once (target/kb-ci-deadline-fixture.log, CI exit 2); controlled same-HEAD rerun passed all 135 fixtures and full CI, with no threshold change. Raw logs stay outside Git. No separate parity/boundary run or live application installation; current task_report schema has no report parameter, so no unsupported argument was sent."
      }
    ],
    "exit_code": 0
  }
}
```
