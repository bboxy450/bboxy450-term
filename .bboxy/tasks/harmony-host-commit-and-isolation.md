---
acceptance:
- Knowledge commit uses only selected provider and admitted document paths
- Unrelated Git staging survives built-in commit
- Plugin controls documents tasks and messages switch together per swarm
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
created: 2026-10-29T04:58:49.024Z
created_by: seat:01M62ZBZ8MX6Y5HPE662JVDABC
delegated_by: seat:01M62ZBZ8MX6Y5HPE662JVDABC
id: 01M630CSSMCHDZKG8S8ZKHGER2
number: 374
role: executor
status: done
title: Host scoped KB commits and per-swarm reality
type: task
updated: "2026-11-02T18:30:00Z"
updated_by: "seat:01M6EQFA2G00BFT3TVPY6AMXE6"
verify: make test
reconciliation: [{"by": "seat:01M6EQFA2G00BFT3TVPY6AMXE6", "disposition": "implemented", "evidence": ["https://github.com/bboxy450/bboxy450-term/commit/50a849eb"], "fresh_accepted": false, "kind": "historical", "previous_status": "claimed", "reason": "scoped knowledge provider with native document commits landed (knowledge_commit). Hand-edited: task_reconcile refused (see tasks/2026-11-02-bug-task-lifecycle-unowned-records)."}]
---
ROLE: EXECUTOR

Formal operator addendum to tasks/harmony-plugin-host.md. Same worktree /Users/feral/mydev/bboxy450-term-harmony-host, same allowed host crates/tests, same no merge/push/recruit boundaries. Correct base origin/main 618df56f. Sable owns plugins/scripts/Make and tests actual GitKB backend.

Add explicit knowledge_commit tool/API and ctl equivalent. Optional backwards-compatible data_source_commit capability resolves bounded explicit document locators plus supplied commit message in captured swarm/root. Built-in provider uses local git commit --only -- explicit confined document repo paths, preserving unrelated staged work; no auto commit/push, no git add broad, reject symlink/outside paths. GitKB provider uses native document commit via Sable. Existing 1.0 providers remain conformant with named unsupported refusal. Meaningful private Git fixture inspects actual commit paths and surviving unrelated staged changes.

Operator requires each swarm its own reality: selecting swarm switches cwd/root/local plugins/provider/documents/tasks/messages together, no stale async reply crosses context, separate root identical slug does not collide, selections persist after restart. Full slugs are first-class document/task identity and delegation addresses; native IDs preserve rename continuity. Reuse existing per-swarm source routing and fix missing Plugin/KnowledgeControl scope, no gratuitous messaging rewrite.

Signed bridge finalized: top-level signed_record optional string contains host-signed portable JSON intent; actual adapter YAML keeps native UUID/status while bboxy_signed_record stores exact opaque intent and bboxy_status host lifecycle. Outward FM.id portable ref.id, native UUID external_id. No next_number; external numbers nullable. Verify exact echo and portable stored readback id/body/task fields, strip old proof/derived metadata before signing. Private checker optional fixture bootstrap only in owned disposable account, never Harmony.

Proceed focused Make checks with RUSTUP_TOOLCHAIN=1.99.1. Oracle runs one required combined make test/test-boundaries/test-licenses at final integrated SHA. Report checkpoint evidence provisionally with actual exits/source/commit/gaps; don't claim full acceptance before final gate. Deliver PR authorized: Oracle integrates and opens, never merges.