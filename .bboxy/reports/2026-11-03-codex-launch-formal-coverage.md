---
id: 58DG0V9A1Q9HESZWGSWDQC0X4W
number: 5002231
type: report
title: Codex launch formal coverage and remaining landing blockers
author: "seat:01M6HCFJ8QFC193R49JW3G0VQX"
task: "[[tasks/2026-10-21-w2-3-adapters-claude-codex-zsh]]"
supersedes: "[[reports/2026-11-03-codex-launch-blockers]]"
commits: ["b74e62dfc85cb158be2a39a9ba7f89a74b4884a7"]
created: "2026-11-03T19:47:51Z"
verify:
  command: "formal.coverage + formal.contracts: no project family missing from crate coverage"
  exit_code: 0
---
# Codex launch formal coverage and remaining landing blockers

[[reports/2026-11-03-codex-launch-blockers]] retains the exact Rust, corpus,
import-timeout and inventory results. Same external-conversation author
identifier; no native seat grant is claimed.

The clean b74e62df make test-formal run passed its native model/project and
embedded-harness phases, including exporting the new CodexMcpReadiness pair
and 95 harness models. It then stopped before Java parity: the existing pinned
harness's HarnessTextZoom family was absent from the parent crate inventory.
A read of all contracts and coverage identified that family alone; the new
Codex readiness model was already mapped correctly.

`verification/coverage.toml:346` now registers HarnessTextZoom and its existing
broken control under the embedded harness, matching the pinned child corpus.
The direct inventory check found no remaining unmapped project families,
exit 0. No model property or test assertion was weakened. Full formal
verification is rerun on the signed commit containing this two-line repair;
its command output is target/codex-launch-formal-complete.log.

No main push occurred. The unrelated 250-record import test still failed its
isolated retry, and main's duplicate report number 5002227 plus invalid task
reference still fail the report corpus. The worktree remains assigned and
unlanded, so cleanup after landing has not happened. No gate bypass, report
history rewrite, full make test, installation or deployment was performed.
