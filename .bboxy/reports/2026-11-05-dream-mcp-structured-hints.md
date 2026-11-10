---
type: report
title: Dream MCP draft alignment with shared structured host hints
task: '[[tasks/2026-11-05-dream-mcp-structured-hints-contract]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T15:56:19.546Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6P6SPDECKAFAMXN7EWZ9HB7
updated: 2026-11-05T15:56:19.546Z
number: 93377961364702
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:619ff04bc782dcb006ed6014e17d0099e52493fc6219936af86216410438a7ae
  frontmatter_digest: sha256:08fc65a768a935669fc881d9f878ede2853798242236205828efe612789fa4e5
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: ho7iFkzhsVNTlaZzBO1FVb+7Zx10oJ89bWmx5S9QvUXokujOf4kFvl9MuFokQkx7zmlxbTjhLA5M7kfHrNYEDg==
  log: .bboxy/log/2026-11-05.jsonl#137456
---
# Dream MCP structured-hints draft alignment

Task: [[tasks/2026-11-05-dream-mcp-structured-hints-contract]] (01M6P6MXAGWVCRM5C3H3VTVBEQ). Amended only the brief bodies of the original Q1/Q3 drafts through current knowledge_fetch refs/etags and frontmatter-preserving knowledge_write. Both writes were admitted; no authority bypass or replacement task was needed.

## Changes and acceptance evidence

Both original drafts now carry “Shared structured host hints (required integration)” under [[decisions/d93377961364692-dream-mcp-structured-host-hints]], [[interfaces/dreaming-mcp]] and [[specs/dreaming-mcp]]:

- Reuse the ONE actual shared schema/generator/post-journal off-owner projection after integration. No duplicate helper/renderer, independent control-version bump, nested provider/plugin-result mutation or changes to C1/read-prepare wire, proof bytes/hashes, original operations, signed receipts, pins or authority.
- Essential guidance is in structuredContent.hints with matching serialized text. Closed Dream domain DTOs stay strict; the closed exported MCP schema explicitly includes known host op_id/shared-hints fields, validates the FINAL hinted document and separates only declared host fields before typed-domain parsing. No incompatible allOf or global unknown-field loosening.
- Reserve guidance before selecting whole hits/pages. Every structured/text/_meta hint copy is charged within existing16KiB/context budgets; existing C1 closure/resource/control/foreground/settlement/ABA requirements remain.
- Actual admitted structured-only fixtures discard content/_meta and cover supported positive/page/empty/unavailable/pending/error/stale/foreign/original-op recovery responses. Future gates require final schema validation, text-hint equality, repeated socket+bridge projection, near-limit pages and poisoned historical/provider-data controls. No live Codex account/model experiment or installed-client claim is substituted for joined-source evidence.

Existing checks, dependencies and Make targets were retained. The shared implementation remains owned by the operator's parallel harness; Q1/Q3 consume its actual integrated source and do not implement a competing helper. No source or runtime behavior was changed by this task.

## Exact records, readback and field preservation

| Original draft / native ID | Before digest | Returned/readback digest |
|---|---|---|
| [[tasks/2026-11-05-dream-mcp-observation-reads]] / 01M6P1PS9F0AGDDW9HNJR1N99C | a5d81813030e41553ad3982a6aa4729666c9f82c9f7f8d6432ccbf34adca6050 | c71bfac7754234b0c1db33bb6dec0573f2314e638b1ab59403ba3fa319e78bc9 |
| [[tasks/2026-11-05-dream-mcp-verified-feedback]] / 01M6P1PWPKEEYWPZGQW21EYCW9 | a7132f7df671308142d3369fb83a7638394772a373298e57bc5af23a91842298 | 8dc5f8475fe3a926ed4d9a8ed50dc15ada6491c6ac32e28b98a9da82dad68b4c |

Full returned bodies equal intended amended bodies; no truncated readback remains. All original body text was retained, with one required section inserted before Checks/handoff. Frontmatter comparison found only host-generated updated/provenance changes in each record. Role, allowed_paths, verify, verify_revision=1, ID/number, creation/creator, derived_from, depends_on, status, delegated_by, assignee and swarm were preserved; all other frontmatter was unchanged too. Native task_status independently returned open with no assignee for both. Neither draft was delegated, assigned or started.

Both retained verify:
make test-dream-mcp && make test-formal && make test-formal-contracts && make test-boundaries && make test-rich-launch-context && make test-harness-native.
These are their future implementation gates, not commands run here.

Current governing documents read fully (including ICD continuation):
decision8563ddf2bedcf6a072045d452449cb980c09bdebf009ce20034bb673c08f0e96;
ICDc9497e6ff5315ec076706fc5e55d6eb8542bb474b1745c596ab7f4380326508e;
specb79ec04a5b931cc129fcf27521ed5b53bbc6632394ae112ed898aed8c876821d.

## Actual verification and limits

At2026-11-05 15:55:38 UTC, git diff --check in /Users/feral/mydev/bboxy450-term returned exit0 with empty output. It checks tracked diff only; the full knowledge readbacks above verify these untracked task records.

No source edits, builds, runtime/formal/performance/client tests, new research, provider/service/account/credential access, delegation/recruitment, installation, merge/push or commits. This report proves task/contract alignment and record preservation only. Actual shared-helper integration and final projected-schema/structured-only/budget/runtime witnesses remain the implementation tasks' acceptance work.
