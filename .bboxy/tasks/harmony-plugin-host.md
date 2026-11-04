---
acceptance:
- Scoped discovery and Shape identity metadata work
- GitKB mounted source handles documents and authorized task lifecycle without bundled fallback
- Required checks pass with source evidence
allowed_paths:
- crates/bbt-plugin
- crates/bbt-mux
- crates/bbt-proto
- crates/bbt-ui
- crates/bboxy450-term
assignee: seat:01M62ZA3HPE6N4CJ3F3SZB2Q6E
authorship:
  kind: seat
  process_generation: '1'
  public_key: PtawKUprAOBmHUgC+Qpt9hciMQWgq9SCP/8ID3uAPx4=
  signer: seat:01M62ZBZ8MX6Y5HPE662JVDABC
created: 2026-10-29T04:45:41.008Z
created_by: seat:01M62ZBZ8MX6Y5HPE662JVDABC
delegated_by: seat:01M62ZBZ8MX6Y5HPE662JVDABC
id: 01M62ZMR89BRGJSAZY50EV5E2E
number: 367
role: executor
status: done
title: Harmony scoped plugin host and writable knowledge integration
type: task
updated: "2026-11-02T18:30:00Z"
updated_by: "seat:01M6EQFA2G00BFT3TVPY6AMXE6"
verify: make test
reconciliation: [{"by": "seat:01M6EQFA2G00BFT3TVPY6AMXE6", "disposition": "implemented", "evidence": ["https://github.com/bboxy450/bboxy450-term/commit/0f8261d1"], "fresh_accepted": false, "kind": "historical", "previous_status": "claimed", "reason": "account-owned plugin discovery, trust and activation landed on main. Hand-edited: task_reconcile refused (see tasks/2026-11-02-bug-task-lifecycle-unowned-records)."}]
---
ROLE: EXECUTOR

Implement the operator's plugin/knowledge host half. Worktree: /Users/feral/mydev/bboxy450-term-harmony-host. Allowed: crates/bbt-plugin/**, crates/bbt-mux/**, crates/bbt-proto/**, crates/bbt-ui/**, crates/bboxy450-term/** and relevant host tests. Do not edit Makefile/scripts/plugins; Sable owns adapter and build integration. No recruiting, merge, push or live account control.

Read shared/project instructions, vision, plugin/knowledge specs and ICD. Review existing binary + --shape + stdio MCP contract. Implement global ~/.bboxy/plugins and repository .bboxy/plugins discovery with isolation preserved; preserve existing app-support discovery compatibility where needed. Shape carries human name, description and optional icon/identity metadata, backward compatible. Make real git-kb document provider usable for a Harmony swarm, selected for that workspace only, including document reads/search/graph AND writes/task delegation lifecycle via host authority (no silently writing bundled .bboxy when gitkb selected). Coordinate explicit wire boundary with Sable through me before incompatibilities. Existing external writes currently signed_writer_unavailable: implement smallest host-owned bridge necessary. Internal semantics should match GitKB numbered/slug/id docs, typed frontmatter, links and explicit task intent/status/verification. Preserve signing, role authorization, confinement, paging, UI rendering and unattended operation. Use existing plugin system rather than replace it.

Acceptance: discovery tests prove local provider doesn't leak; Shape metadata round-trip; real adapter can select, resolve/search/fetch and create/update a doc plus swarm delegate/report/accept task with same backing source, host signature/authority preserved. Regression failures and unsupported operations explicit. Add meaningful integration coverage. Coordinate task_system scope with Oracle if data_source bridge is better. Verify make test and make test-boundaries (actual process changes) in your final committed tree; use Make canonical interface; signed exact-path commit. Return source pointers, raw check outputs and gaps. Stop before merge/publish/install.