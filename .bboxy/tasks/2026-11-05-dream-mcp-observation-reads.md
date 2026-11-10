---
type: task
title: Q1 implement own-seat prepared C1 observation search/read
role: executor
allowed_paths:
- ATTRIBUTION.md
- Makefile
- crates/bbt-proto/src/dream_read.rs
- crates/bbt-proto/src/lib.rs
- crates/bbt-proto/tests/dream_read.rs
- crates/bbt-mux/src/logic/dream_read.rs
- crates/bbt-mux/src/logic.rs
- crates/bbt-mux/src/registry/dream_read.rs
- crates/bbt-mux/src/registry.rs
- crates/bbt-mux/src/registry/dream_c1.rs
- crates/bbt-mux/src/mcp/dream_read.rs
- crates/bbt-mux/src/mcp/mod.rs
- crates/bbt-mux/src/mcp/tools.rs
- crates/bbt-mux/tests/dream_read.rs
- crates/bboxy450-term/tests/dream_read.rs
- crates/bboxy450-term/tests/formal_contracts/dream_read.rs
- crates/bboxy450-term/tests/formal_contracts/models.rs
- verification/contracts.toml
- verification/corpus.toml
- verification/coverage.toml
- reports/2026-11-05-dream-mcp-observation-reads.md
- scripts/qos.sh
- scripts/qos-policy.py
- LAND.md
verify: make test-dream-mcp && make test-formal-contracts
derived_from:
- '[[research/dreaming-services-mcp]]'
- '[[specs/dreaming-mcp]]'
depends_on:
- '[[tasks/2026-11-05-dream-c1-producer]]'
- '[[tasks/2026-11-05-dream-c1-consumer]]'
id: 01M6P1PS9F0AGDDW9HNJR1N99C
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T20:00:57.072Z
created: 2026-11-05T14:27:20.106Z
number: 93377961364660
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
verify_revision: 2
assignee: seat:01M6XXAAX0KD2M33WBR5DGXA2X
verify_result:
  command: make test-dream-mcp && make test-formal-contracts
  exit_code: 0
  evidence:
  - criterion: Amended native/core route; heavyweight close-to-read witness explicitly deferred
    reference: crates/bboxy450-term/tests/dream_read.rs:151; crates/bbt-mux/tests/dream_read.rs:89; /tmp/dream-q1-committed-mcp.log exit 0
  - criterion: Own-seat/currentness/pin refusal without read credential or signing effects
    reference: crates/bbt-mux/tests/dream_read.rs:38,89,296; crates/bbt-mux/src/registry/dream_read.rs:596,877
  - criterion: Closed final shared-hint schema, two-session row binding and honest zero-link coverage
    reference: crates/bbt-proto/src/dream_read.rs:412; crates/bbt-mux/tests/dream_read.rs:196,272; report contains limits and deferred stress coverage
  - criterion: Bounded completion-only projection, foreground guard and no work on miss; pressure expansion deferred
    reference: crates/bbt-mux/src/registry/dream_c1.rs:2258; crates/bbt-mux/src/registry/dream_read.rs:492,957
  - criterion: Three finite families plus nine causal controls and retained current-source formal proof
    reference: /tmp/dream-q1-land-mapped.log exit 0; receipt f14212217f744ebdf9ff33920e108c7d30346d43eb60cd90e0dfaf1689ca53b2; make check-formal-evidence exit 0
  - criterion: Scoped signed source, notices and actual landing evidence
    reference: HEAD=origin/main=da21c1bde2171af3359d68c5ef61c5617a744507; make test-licenses exit 0; committed report checkpoint 0e2e638c82fcfc42c3eba8be01c3b03dee64c291
report:
  summary: Landed the amended Q1 observation search/read core at da21c1bde2171af3359d68c5ef61c5617a744507 through make land (exit 0). Required test-dream-mcp/formal-contracts passed; final gate passed 196 term, 888 mux and 31 proto tests plus 693 native/TLC models across 34 crates, with clean source and five signed commits. The committed report records exact pins/hints/currentness evidence, deferred optional links and full worker-close/pressure witnesses, and the transient 7.8 GiB disk deviation (restored to 12 GiB). Native report checkpoint 0e2e638c82fcfc42c3eba8be01c3b03dee64c291 is confirmed committed; artifact_claim nevertheless twice refused artifact_not_committed, so no successful manual artifact claim is asserted.
  commit: da21c1bde2171af3359d68c5ef61c5617a744507
  reported_by: seat:01M6XXAAX0KD2M33WBR5DGXA2X
  op_id: 01M6YBZBF5XVGM59EKHZZNNFS3
  verify_revision: 2
  report: '[[reports/2026-11-05-dream-mcp-observation-reads.md]]'
provenance:
  digest: sha256:24f363f4e6c1dc17706ce51eda8fbc57495577d2aeaf1931633f03f8ab709bcc
  frontmatter_digest: sha256:b52317fdd5484d5732f8b32bf4f94934fa536dbca9f79b1666d844672d04632b
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: qaeBULxhtnPUlT5IeevZ3oPQHzHEFtWH55Bx3TDwiqT/iAM/MxWCAOvTUOdskZFwkuOwe+6fELgWOwbBwnZKAA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150817
  event:
    id: 01M6YBZVAVE2FE1MCKFQY211X8
    sequence: 150817
    hash: 7bac978fb301b762603301b0a3a30692a24834d28a91e388d286d3f6fe0d395e
---
ROLE: EXECUTOR

## Goal / worktree
Implement Q1 of [[specs/dreaming-mcp]] and exact [[interfaces/dreaming-mcp]] search/read DTOs for completed own-seat C1 observations. Research [[research/dreaming-services-mcp]]. This is an **open unassigned draft**. Oracle must set an isolated term worktree and exact baseline before delegation; do not start in the shared checkout or running C1 author worktree.

## Dependencies and scope
Start only after both C1 components are accepted and intentionally integrated into term main with current proof/closure/budget gates. Do not modify their live scope. Allowed source paths are in frontmatter; report via the task tools. No inference, KB/provider scan, verified-lesson backend, feedback write, new memory sink, native memory-file modification, MCP tasks/resources/prompts, global config, install/publish/merge/push unless separately authorized.

## Exact implementation
Add public closed DTO/schema fixtures in bbt-proto::dream_read; register dream_search/dream_read in existing role-filtered MCP catalogue and off-owner logic service. Consume only existing validated completed/settled C1 descriptor/pages through one completion hook/read adapter in registry/dream_c1. Never rereplay source or call self-MCP. Store a bounded disposable prepared projection and host-scoped opaque handles/cursors, with explicit incomplete coverage. Reuse public Context/PublicFence, generation/grant/root/provider invalidation and C1 resource ownership. Use deterministic literal identifier/exact-fingerprint/recent metadata investigation. Do not match natural-language symptoms to hashes or imply reusable guidance; Q2/Q3 supply lesson-text retrieval.

Only kind observation is available. Requesting lesson returns unavailable/no_published_input; observation context returns unavailable/not_a_lesson. Available empty, unavailable, pending and errors follow ICD. Every hit retains exact C1 manifest/result/page digest+page index+row index per ResultRowPin, with exact row-kind/field binding. Only Episode has its copied Origin/EventPin; Cluster never fabricates one. Optional ≤8 member links validate same-run episode fingerprint/repo membership and explicitly state omitted/complete coverage; zero links with complete=false is sufficient. Follow the ICD two-session case. trust remains not_evaluated. No adopted/verified/Recall claims; no raw transcript/secret fields. Use existing term launch/client paths; do not add a second server or wake progress agents.

## Acceptance
- [ ] Native admitted MCP route discovers both tools and reads a real scripted C1 result after authoritative close; open/idle/turn-end/EOF creates none.
- [ ] Scope/cursor/query/ABA/revocation/root/provider currentness, source/page tampering and missing preparation refuse correctly; no credential/custody/signing call.
- [ ] Stable literal metadata ranking, exact schema text fallback, byte/token/entry limits, whole-hit truncation and separate index/member coverage pass fixtures. A canonical two-session same-repo/fingerprint result yields one count2 cluster with no origin and two episode origins. Descriptor/page tampering, wrong row index/kind, foreign/nonmatching/duplicate member and one-member-as-complete controls fail; zero/one linked member with honest omitted coverage succeeds. No source rereplay/raw transcript export/C1 wire change.
- [ ] Real parent+child stall/flood/four-job/control/native-frame witness retains all current C1 CPU/duty/heap/latency/pause/settlement limits, no operator input. Cache misses do not start work.
- [ ] Finite DreamReadOwnership/Snapshot/Eligibility models with causal foreign/ABA/mixed-cursor/fake-lesson/row-binding/fabricated-cluster-origin/false-member-completeness/missing-reservation controls and real API/process witnesses are mapped in verification contracts; declare opaque native allocations unmodeled.
- [ ] Source diff/attribution read, tests/gates recorded with actual exits, no current author changes overwritten.

## Shared structured host hints (required integration)

Follow [[decisions/d93377961364692-dream-mcp-structured-host-hints]] and the current [[interfaces/dreaming-mcp]] / [[specs/dreaming-mcp]]. Consume the ONE actual shared host hint schema/generator/projection after its integration; do not copy the parallel patch, write a Dream-specific renderer/helper or independently bump the control version. The supplied patch/operator observation is not installed-client or joined-source verification.

Essential discovery/recovery guidance belongs in structuredContent.hints with a matching serialized text copy. content or _meta alone is insufficient. Keep strict Dream domain and worker/artifact DTOs unchanged. The exported closed MCP outputSchema must explicitly include the known outer host op_id and shared-hints fields in each supported domain variant and validate the FINAL hinted document. Validate/separate only those declared host fields before strict typed-domain parsing; no incompatible allOf composition, global unknown-field loosening or nested provider/plugin-result mutation. Preserve original operation identity, signed receipts, source/result/page bytes, proof pins/hashes and authority. Use the common post-journal/off-owner projection, idempotently when both socket and bridge encounter a reply; hints cannot admit work or promote historical/provider text to instructions.

Reserve bounded guidance space BEFORE selecting whole hits/pages. Charge every emitted structured/text/_meta hint copy within existing16KiB serialized and context/token budgets; the generic frame ceiling does not supersede these limits. Preserve compact required recovery guidance and exact pins, with no hidden overflow or uncharged second copy. All existing C1 closure, CPU/duty/memory/control/foreground-pause/settlement/ABA requirements and original dependencies/checks remain.

Required unattended integration evidence: actual admitted supported positive/page, empty, unavailable, pending, error/stale/foreign and original-op recovery replies go through a structured-only client fixture that discards content/_meta whenever structuredContent exists; essential guidance still survives. Validate the final hinted structured document against advertised outputSchema, compare text-hint equality and unchanged nested payload/operation/pins, and exercise repeated socket+bridge projection, near-limit pages and poisoned historical/provider-data controls. Map original-op recovery only where supported by this slice; do not invent domain effects to create a test. Consume the actual shared implementation in these focused fixtures through the existing Make targets. No live Codex account/model experiment, extra provider call or assertion of installed-client verification is required or claimed.

## Checks / handoff
Add canonical **make test-dream-mcp** (proposed target, not installed in researched main) for bbt-proto/mux DTOs, actual admitted MCP/private term process and source-bound negative controls. Run it plus make test-formal, make test-formal-contracts, make test-boundaries; use make test-rich-launch-context and make test-harness-native for the affected adapter/process witnesses and reuse exact-source receipts. No live account/service/Keychain or desktop interaction. Final landing scope is assigned by Oracle; if authorized use make land, not a repeated full suite. Write the allowed append-only report with task, commits, exact commands/exits, evidence per checkbox, gaps/worktree state; task_report once.

## Oracle amendment (2026-11-08): dispatch NOW, operator priority
The operator said: "continue in parallel, do it fast! we need this system in asap".
- Worktree: /Users/feral/mydev/bboxy450-term-dream-q1, branch feat/dream-q1-observation-reads, base origin/main 96dbc71ba. Work only there.
- Dependency override: the C1 consumer (dream_c1 MCP tool, registry/logic/mcp dream_c1.rs) is already integrated on term main. Build on main as it is; do NOT wait for the consumer task's acceptance and do not modify its scope.
- Shared host hints: crates/bbt-mux/src/mcp/hints.rs is on main. Consume it as the ONE shared projection.
- Verify is trimmed to: make test-dream-mcp && make test-formal-contracts. Add test-dream-mcp to the Makefile (Makefile is allowed) running the bbt-proto/bbt-mux dream_read tests and the term dream_read process test.
- Formal: if you add or change models, make land runs make test-formal itself. Do not run make test-boundaries, rich-launch-context or harness-native separately unless your diff touches those paths.
- Ship the smallest useful slice first: dream_search + dream_read over completed own-seat C1 observations, with the ICD shapes and honest unavailable/not_a_lesson answers. Optional member links and the witness heaviness come after a working, tested core.
- Land yourself with make land once verify is green (autonomous landing is authorized). Then report in reports/ and task_report once.
- CPU/disk: one build at a time, nextest -p for the crates you touch while iterating, and keep free disk above 8 GiB.


## Oracle amendment (2026-11-08): timed-test QoS inventory
Added scripts/qos.sh, scripts/qos-policy.py and LAND.md to allowed_paths, ONLY to register test-dream-mcp and its C1-based timing fixtures (the dream_read targets and test names) in the EXISTING default-QoS timed-test policy (TIMED_TARGETS and the named list), the same way dream_c1/dream_replay are registered, plus the matching LAND.md line.
Witness: targeted nextest exited 100 under inherited utility QoS. capture_context took 1.097750 ms and journal commit 1.236292 ms; log /tmp/dream-q1-mux.log.
Budgets, assertions and the 1 ms quantum stay unchanged. No ignore attributes.


## Oracle amendment (2026-11-08): one more existing C1 timed test
Under the timed-test QoS amendment, also register the existing test dream_c1_structured_only_success_page_and_foreign_refusal_use_advertised_schema in TIMED_TESTS, with its LAND.md inventory line.
Witness: make land hit its unchanged 1 ms journal guard at 1.017042 ms (/tmp/dream-q1-land-fixed.log:225-230).
Budgets and assertions stay unchanged; no ignore.


## Oracle amendment (2026-11-08): register timed C1 test FAMILIES, not single tests
A second identical refusal hit an existing test: dream_c1_cancel_original_operation_is_durable_and_cannot_retarget_a_run, 1.149167 ms against its 1 ms unit (/tmp/dream-q1-land-final.log:222-225).
- Replace the one-test exception with explicit TIMED_FILTERS for registry::dream_c1::registry_tests:: and logic::dream_public_api_tests::, plus the matching LAND.md inventory.
- Budgets, assertions and production QoS stay unchanged; no ignores.
- Standing rule for this task: any further EXISTING C1/Dream timed test family that refuses under inherited utility QoS during make land may be registered the same way, without asking. List each one in the report.


## Report from seat:01M6XXAAX0KD2M33WBR5DGXA2X

Landed the amended Q1 observation search/read core at da21c1bde2171af3359d68c5ef61c5617a744507 through make land (exit 0). Required test-dream-mcp/formal-contracts passed; final gate passed 196 term, 888 mux and 31 proto tests plus 693 native/TLC models across 34 crates, with clean source and five signed commits. The committed report records exact pins/hints/currentness evidence, deferred optional links and full worker-close/pressure witnesses, and the transient 7.8 GiB disk deviation (restored to 12 GiB). Native report checkpoint 0e2e638c82fcfc42c3eba8be01c3b03dee64c291 is confirmed committed; artifact_claim nevertheless twice refused artifact_not_committed, so no successful manual artifact claim is asserted.

```json
{
  "op_id": "01M6YBZBF5XVGM59EKHZZNNFS3",
  "commit": "da21c1bde2171af3359d68c5ef61c5617a744507",
  "report": "[[reports/2026-11-05-dream-mcp-observation-reads.md]]",
  "verify_result": {
    "command": "make test-dream-mcp && make test-formal-contracts",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Amended native/core route; heavyweight close-to-read witness explicitly deferred",
        "reference": "crates/bboxy450-term/tests/dream_read.rs:151; crates/bbt-mux/tests/dream_read.rs:89; /tmp/dream-q1-committed-mcp.log exit 0"
      },
      {
        "criterion": "Own-seat/currentness/pin refusal without read credential or signing effects",
        "reference": "crates/bbt-mux/tests/dream_read.rs:38,89,296; crates/bbt-mux/src/registry/dream_read.rs:596,877"
      },
      {
        "criterion": "Closed final shared-hint schema, two-session row binding and honest zero-link coverage",
        "reference": "crates/bbt-proto/src/dream_read.rs:412; crates/bbt-mux/tests/dream_read.rs:196,272; report contains limits and deferred stress coverage"
      },
      {
        "criterion": "Bounded completion-only projection, foreground guard and no work on miss; pressure expansion deferred",
        "reference": "crates/bbt-mux/src/registry/dream_c1.rs:2258; crates/bbt-mux/src/registry/dream_read.rs:492,957"
      },
      {
        "criterion": "Three finite families plus nine causal controls and retained current-source formal proof",
        "reference": "/tmp/dream-q1-land-mapped.log exit 0; receipt f14212217f744ebdf9ff33920e108c7d30346d43eb60cd90e0dfaf1689ca53b2; make check-formal-evidence exit 0"
      },
      {
        "criterion": "Scoped signed source, notices and actual landing evidence",
        "reference": "HEAD=origin/main=da21c1bde2171af3359d68c5ef61c5617a744507; make test-licenses exit 0; committed report checkpoint 0e2e638c82fcfc42c3eba8be01c3b03dee64c291"
      }
    ]
  }
}
```
