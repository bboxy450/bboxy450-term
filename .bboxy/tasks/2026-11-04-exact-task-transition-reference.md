---
type: task
title: Close the two stale launch aliases through exact task-reference resolution
role: executor
parent: 01M6KEVVYNZW5H21RPTFW1RVDR
allowed_paths:
- crates/bbt-mux/src/mcp/work_items.rs
- crates/bbt-mux/src/knowledge/**
- crates/bbt-mux/src/index.rs
- crates/bbt-mux/tests/mcp/**
- .bboxy/reports/**
- .bboxy/runs/exact-task-transition.json
verify: make verify-task-exact-reference
acceptance:
- Task transition resolves the original exact id/path/digest even when a report or nested task shares its basename
- True stale writes still refuse; wrong task/report never changes
- 'After authorized activation, #19226 and #19209 are reconciled superseded by accepted #19244 with original IDs/history retained'
id: 01M6KF9SQBXGNHXFNNFPG95XFG
status: open
delegated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T14:27:10.738Z
created: 2026-11-04T14:27:10.738Z
number: 21254
created_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
provenance:
  digest: sha256:bc7f4a5c209a1853195ea294a81a0534feec3cd15c3997a692c6e0837a78cf25
  frontmatter_digest: sha256:3af40c07aba8ae833dfbec64a59be484d702387705cdbdd9b5d8d960ad4dae46
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: M4Bahvl+655LX7BHltY+JaFXBb/G2NQMqGmGm16usKoPLbBDTnCnSm5RMf0lc2hudx2O327Pj5ZAFzwn7uGHDQ==
  log: .bboxy/log/2026-11-04.jsonl#114158
---
ROLE: EXECUTOR. TRIAGED FOLLOW-UP, NOT YET DELEGATED. No implementation authorization until a scoped worktree and signed task_delegate assignment are provided.

During post-crash triage in swarm01M649102FBS60QWNPZS5VAGF9, task_reconcile on duplicate launch-context task #19226 (id01M6HREVGM5DB1KQ1P4CZ20VWH, tasks/2026-11-03-codex-launch-parity.md) and nested duplicate #19209 (id01M6HRD5XR91MD7MH557R77N5Y, .bboxy/tasks/2026-11-03-codex-launch-parity.md relative to KB root) was explicitly rejected conflict/document-changed-before-write. Terminal operations01M6KE85X08NCDVYPF4P33XFRM and01M6KECXY7A3XZZFDEQVY3A12M; first confirmed terminal/retry_allowed=false through operation_status01M6KEA4SVMBZFMBJ31YR0HBDG. Do not replay them or claim either alias is closed.

Both aliases are superseded by accepted canonical task #19244 (01M6HRGGBFA6Z97D4TPDSZWPK8), report #19624 and signeda231295. No source work remains for the aliases. Six other historical task transitions succeeded through the same current Oracle, so this is not a blanket task-authority failure.

Observed bytes: knowledge_fetch #19226 returns exact path tasks/2026-11-03-codex-launch-parity.md and digest/etag de1f7b7ba542a34ea4eefe0187fe10c8502d9705e281ece8449dcad0e3141f94; actual canonical file SHA matches. Nested #19209 file SHA1882d5f4fc2de570c43124f415e83b837a1042f89ea0dd3aa7724127603d5018. Same-stem report reports/2026-11-03-codex-launch-parity.md exists. Suspected boundary: knowledge/effects Prepared::Task re-resolves update.path rather than carrying the originally resolved exact reference. Confirm with a private fixture; this is an inference, not a proven root cause.

Use existing authenticated MCP controls, never shell ctl, direct task-status edits, report renaming, arbitrary importer cloning/new duplicate IDs or relaxed compare-and-swap. task_import creates new identities and is inappropriate for closing these existing records. Preserve accepted #19244 and all reports/refs. New private regression should include task/report same basename plus nested task aliases, exact changed-record assertions and a genuinely stale-digest negative control.

Focus only necessary tests/compile. No shared backend restart while another swarm works; if activation needs a backend refresh, keep tested source/evidence ready and report that constraint honestly. Oracle will assign this administrative repair after the current profile/ACP controls priority, or when a safe activation path exists.
