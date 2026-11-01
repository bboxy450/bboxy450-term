---
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
  digest: sha256:8b16d2ff6db071e6acfb264c7a5fdb02a6b56d4c80746ffe7237df2bcee3d856
  frontmatter_digest: sha256:bb22c7a58d360aaec3d567818ffdc399a5cabce8bc9db3f8f91703addc193df9
  log: .bboxy/log/2026-11-01.jsonl#677
  sig: c2lnMcbJ8eotuO56JnKQRgV2aFGcgjXjWYJolFrnbLbAo5uLaDpbKHfMcdiTG0Smdcmma+XNVLDjUceimQdWDQ==
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
role: executor
status: claimed
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Built-in KB: report document type, linked from commits by wikilink'
type: task
updated: 2026-11-01T02:51:51.867Z
updated_by: seat:01M6ABH71TVWGZ621NTD701PSH
verify: make test-task-contract && make test
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
