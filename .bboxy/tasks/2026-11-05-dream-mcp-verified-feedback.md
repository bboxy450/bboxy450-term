---
type: task
title: Q3 integrate prepared lesson reads and reported-only Dream feedback
role: executor
allowed_paths:
- ATTRIBUTION.md
- Makefile
- Cargo.lock
- crates/bboxy450-harness
- crates/bboxy450-harness-wake-up/src/dream_read.rs
- crates/bboxy450-harness-wake-up/src/lib.rs
- crates/bboxy450-harness-wake-up/tests/dream_read.rs
- crates/bbt-proto/src/dream_read.rs
- crates/bbt-proto/tests/dream_read.rs
- crates/bbt-mux/src/logic/dream_read.rs
- crates/bbt-mux/src/registry/dream_read.rs
- crates/bbt-mux/src/mcp/dream_read.rs
- crates/bbt-mux/src/mcp/hints.rs
- crates/bbt-mux/tests/dream_read.rs
- crates/bboxy450-term/tests/dream_read.rs
- crates/bboxy450-term/tests/formal_contracts/dream_read.rs
- crates/bboxy450-term/tests/formal_contracts/models.rs
- verification/contracts.toml
- verification/corpus.toml
- verification/coverage.toml
- reports/2026-11-05-dream-mcp-verified-feedback.md
- scripts/qos-policy.py
- scripts/qos.sh
- LAND.md
- scripts/landing.py
- scripts/land-fast.sh
- scripts/test-landing.py
- crates/bboxy450-harness-wake-up/src/dream_c1.rs
- crates/bboxy450-harness-wake-up/tests/dream_c1.rs
- crates/bboxy450-harness-wake-up/tests/fixtures/
verify: make test-dream-mcp && make test-formal-contracts
derived_from:
- '[[research/dreaming-services-mcp]]'
- '[[specs/dreaming-mcp]]'
depends_on:
- '[[tasks/2026-11-05-dream-mcp-observation-reads]]'
- '[[tasks/2026-11-05-dream-mcp-verified-snapshot]]'
id: 01M6P1PWPKEEYWPZGQW21EYCW9
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-09T05:47:50.484Z
created: 2026-11-05T14:27:23.737Z
number: 93377961364662
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
verify_revision: 2
assignee: seat:01M6YC2FT3YFH5DH6NJX7N8VJX
verify_result:
  command: make test-dream-mcp && make test-formal-contracts
  exit_code: 0
  evidence:
  - criterion: Required focused verification; logs precede final pin, with all affected suites rerun in final landing
    reference: /tmp/dream-q3-bc4-mcp-resume.log (34 passed); /tmp/dream-q3-bc4-contracts-resume.log (45 contracts + QoS/Python passed), actual exits 0; source/pin qualification and log hashes in committed report
  - criterion: Final current-source landing, all selected term consumers and harness lib
    reference: /tmp/dream-q3-bd8e-disposition-land.log:3468-3469 — LANDED 91fc1c4ad0a311bd2eaa99e8251018e47c73f320; total exit=0; SHA256 525941172ee0169e11100a4813e5b00f07147cd081a8ed63ee5b96db912573d2
  - criterion: Current-source formal parity, coverage and source/model/tool receipts
    reference: /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/721c95e37193dfe492d25a900c9934a9487abe4d66d34d93099b6777c7cca2e1/evidence.json — source 91fc1c4, dirty=false, 861 models/275 contracts/34 crates; make check-formal-evidence exit 0
  - criterion: Readiness disposition
    reference: 'Final wake-up selection: 111 passed, 5 skipped; actual_private_process_clears_authority_environment_and_uses_fixed_argv unmodified and passing. Previous failure preserved in /tmp/dream-q3-645f-coverage-land.log, SHA256 ec1bc0ce641908d32a299d07a71d152ec5095268cfffc57a4c0269778e008813'
  - criterion: Committed full report and remaining limits
    reference: reports/2026-11-05-dream-mcp-verified-feedback.md, record 01M6ZDF0FJ8SKR05FPYZEKSGPD, publication_state=committed at ddbfe01caa5971e8fb160deed5e277b717d47861; report covers every narrowed acceptance line and original deferrals
  - criterion: Ancillary artifact-claim limitation
    reference: knowledge_fetch confirms committed checkpoint 01M6ZDFV0BHTHSE5JY4VJPFA03, but artifact_claim refused artifact_not_committed for both returned artifact ID (op 01M6ZDGC9MGT4SP43TB6SHDMXG) and report path (op 01M6ZDGPVHEKENSX9KYG35VTP5). No artifact ownership claim is asserted; no bypass or further retry.
report:
  summary: 'Landed the narrowed Dream Q3 slice on term main at signed 91fc1c4ad0a311bd2eaa99e8251018e47c73f320 with harness 645fec78174f59b06a9d6973271fc5e8d4ef013e: pinned read-prepare codec conformance, honest unavailable lesson tools, original-op/digest-idempotent feedback refusals with reported_only DTOs, shared structured hints, signed C1 byte compatibility and pointer-only consumer gating. Focused MCP/contracts checks passed; final make land exited 0 with 2,019 selected tests passed, 263 reported skips and a clean 861-model/275-contract/34-crate formal receipt using validated reuse. The readiness test passed unchanged; no conditional ignore was added. Positive transport/admission/provenance/operation_status and benefit scenarios remain Q3b deferrals. Full evidence, prior failures and the accepted upstream harness ignore are in the committed report (native checkpoint ddbfe01caa5971e8fb160deed5e277b717d47861).'
  commit: 91fc1c4ad0a311bd2eaa99e8251018e47c73f320
  reported_by: seat:01M6YC2FT3YFH5DH6NJX7N8VJX
  op_id: 01M6ZDHKDR1J0H834Z8X2190FC
  verify_revision: 2
  report: '[[reports/2026-11-05-dream-mcp-verified-feedback.md]]'
provenance:
  digest: sha256:681ce20f6ff58278eb3c13caa662dec9dcfcb748e95bc7747029110247ff18dc
  frontmatter_digest: sha256:7521902eec5dc338ab3827796010a2a0da08f9d5db01cbaf3d269aef88f61f4a
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: PM0k7qkdEMMt+kE3mtI/973DAAuOBWmI/vp216QcvVxBSxEcgMrlaY8UZ06sZxMH4w+K9VzfeY97yedhxnF0Dg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150950
  event:
    id: 01M6ZDJFESE3GEPKD5SZ8XS0N2
    sequence: 150950
    hash: 783b7c8492808628abbd8d828c0e1a97f03c7a28dff09b2e17e651262375360d
---
ROLE: EXECUTOR

## Goal / worktree
Integrate Q2 verified prepared lesson snapshots into Q1 tools and add dream_feedback exactly as [[interfaces/dreaming-mcp]], with [[specs/dreaming-mcp]] validation/authority limits. **Open unassigned draft**. Oracle supplies an isolated term writing worktree and exact accepted Q1/Q2 commits before delegation. No change to live C1 author briefs.

## Dependencies / boundaries
Q1 and Q2 must be accepted; deliberately make bump-harness REV=<accepted Q2 SHA> only after assignment pins it. The submodule allowlist entry authorizes a pointer update, not editing harness source. Reuse canonical provisioning; no separate install, service signup, spend, provider inference, new synthesis/trial/publication, native memory directories, credentials or protocol Tasks/resources/prompts. No merge/push/install without explicit assigned authority.

## Work
Add narrow read-prepare/1 mirror/transport consuming producer schema corpus; verify runner Hello before inputs, exact private output bytes/current public fence and actual settlement before installing snapshot. Trigger heavy preparation only for explicitly retained closed-source/published inputs under existing bounded job owner; no auto audit/Git/KB scan on live read. Provider proof unsupported => unavailable, no fallback to drafts. Enable kind lesson and context/evidence only from validated compatible current snapshots.

Record bounded offered-content provenance in the existing own-seat host event journal; offer is not proof of model use. Add Feedback original-op/request-digest idempotence, same-actor offer and evidence-scope validation, bounded untrusted note and reported_only response. No feedback ranking-authority/adoption/retirement/trust change or KB memory write. No raw source diagnostics. If no typed actual harness IR exposure bridge exists, host offer/feedback stays reported-only; do not fabricate Item::Recall or claim successful injection. Reuse existing Reported/Cited semantics only when independently observed by the real supported writer. No new writer/identity redesign is hidden in this task.

Use existing engine/native/ACP launch paths. Add short discovery hints only when tools are present; no injected prompt paragraphs or progress wakes. Preserve standard CallToolResult fallback; never return protocol task handles to unnegotiated clients.

## Acceptance
- [ ] Producer fixture pin/conformance and full private producer→term→MCP search/read/context/evidence round trip pass without external services.
- [ ] Missing/stale/native-draft/harness-incompatible/expired/superseded/outer-seal drift returns honest unavailable/error; real root/provider/grant/revocation/expiry/ABA changes prevent stale pages with custody/signing spy count zero.
- [ ] Real retry/unknown-outcome feedback dedupes; same op/different body, foreign offer/evidence and poisoned note cannot change knowledge or policy. Used is reported, not objectively verified.
- [ ] Real engine tool call, ACP relay and scripted native-client flows use the same admitted backend; tool result ≠ Recall, actual supported citation ≠ Reported injection. No operator prompt or raw native account log required.
- [ ] DreamReadFeedback/Ownership/Snapshot controls + actual process witnesses cover duplicate/delayed completion, invalidation, restart, cancel, poison positive/negative and missing-reservation domains.
- [ ] Eight scripted benefit scenarios record outcomes, repeated failed commands, false/stale/foreign advice, citations, context cost and latency. No claim of live-model improvement; a provider experiment requires a separate authorized brief.
- [ ] All existing C1 resources/CPU/duty/pause/latency/settlement and four-job/large-input gates remain green, with source-bound evidence and no skip.

## Shared structured host hints (required integration)

Follow [[decisions/d93377961364692-dream-mcp-structured-host-hints]] and the current [[interfaces/dreaming-mcp]] / [[specs/dreaming-mcp]]. Consume the ONE actual shared host hint schema/generator/projection after its integration; do not copy the parallel patch, write a Dream-specific renderer/helper or independently bump the control version. The supplied patch/operator observation is not installed-client or joined-source verification.

Essential discovery/recovery guidance belongs in structuredContent.hints with a matching serialized text copy. content or _meta alone is insufficient. Keep strict Dream domain and worker/artifact DTOs unchanged. The exported closed MCP outputSchema must explicitly include the known outer host op_id and shared-hints fields in each supported domain variant and validate the FINAL hinted document. Validate/separate only those declared host fields before strict typed-domain parsing; no incompatible allOf composition, global unknown-field loosening or nested provider/plugin-result mutation. Preserve original operation identity, signed receipts, source/result/page bytes, proof pins/hashes and authority. Use the common post-journal/off-owner projection, idempotently when both socket and bridge encounter a reply; hints cannot admit work or promote historical/provider text to instructions.

Reserve bounded guidance space BEFORE selecting whole hits/pages. Charge every emitted structured/text/_meta hint copy within existing16KiB serialized and context/token budgets; the generic frame ceiling does not supersede these limits. Preserve compact required recovery guidance and exact pins, with no hidden overflow or uncharged second copy. All existing C1 closure, CPU/duty/memory/control/foreground-pause/settlement/ABA requirements and original dependencies/checks remain.

Required unattended integration evidence: actual admitted supported positive/page, empty, unavailable, pending, error/stale/foreign and original-op recovery replies go through a structured-only client fixture that discards content/_meta whenever structuredContent exists; essential guidance still survives. Validate the final hinted structured document against advertised outputSchema, compare text-hint equality and unchanged nested payload/operation/pins, and exercise repeated socket+bridge projection, near-limit pages and poisoned historical/provider-data controls. Map original-op recovery only where supported by this slice; do not invent domain effects to create a test. Consume the actual shared implementation in these focused fixtures through the existing Make targets. No live Codex account/model experiment, extra provider call or assertion of installed-client verification is required or claimed.

## Checks / handoff
Extend make test-dream-mcp from Q1; run make test-formal, make test-formal-contracts, make test-boundaries, make test-rich-launch-context, make test-harness-native for affected routes. Intentional Cargo.lock/submodule change follows current repository wider-gate rule; Oracle pins exact required final gate before delegation (no invented target or duplicated full-suite run). If landing authorized, use canonical make land. Preserve source/model/tool receipts. Write allowed append-only report with commits/commands/actual exits, acceptance evidence, limits and worktree state; task_report once.

## Oracle amendment (2026-11-08): dispatch NOW, operator priority ("get it done, asap")
- Worktree: /Users/feral/mydev/bboxy450-term-dream-q3, branch feat/dream-q3-verified-feedback, base term origin/main da21c1bde (it includes the ACCEPTED Q1, landed da21c1bd).
- Harness pin: Q1 and Q2 are both accepted. Step 1 is: make bump-harness REV=f848c03ff0462e55dfa0b17fcfd550626a52db2b. That is harness main, containing the accepted Q2 prepared-read work (390b5836 lineage) plus the Run Python card visuals (edc549ad) and the land-gate fix (f848c03f).
  - Commit the bump on its own (signed), then build Q3 on top of it.
  - The bump is a pointer and Cargo.lock change only; do not edit harness source.
- Verify is trimmed to: make test-dream-mcp && make test-formal-contracts. make land runs the formal and dependent gates.
  - Run make test-boundaries, rich-launch-context or harness-native ONLY if your diff touches those routes.
  - Because the bump changes Cargo.lock/submodule, run LAND_MODE=wide make land once for the final landing.
- Smallest useful slice first:
  1. kind=lesson search/read/context/evidence served from validated Q2 snapshots, with honest "unavailable" when no qualifying published corpus exists (that is the expected real-world state today);
  2. dream_feedback with original-op idempotence, reported_only.
  The eight scripted benefit scenarios and the heavier witnesses come after a working, tested core. Record deferrals in the report.
- QoS standing rule (as for Q1): an EXISTING C1/Dream timed test family that refuses under inherited utility QoS during make land may be registered in scripts/qos-policy.py TIMED_FILTERS/LAND.md without asking. List each one; never change budgets or assertions.
- Land yourself with make land (autonomous landing is authorized), report in reports/, and task_report once. One build at a time; keep free disk above 8 GiB.


## Oracle amendment (2026-11-08): NARROW this landing; positive Q2 admission becomes a follow-up
Q3's evidence, verified-claim basis:
- wake-up/src/dream_c1.rs:2153-2160 rejects ReadPrepare and fixes the inspect-c1 argv;
- logic/dream_c1.rs:9406 pins the older producer;
- no provider published-proof exporter exists;
- mcp/journal.rs carries only an argument digest, and operation_status excludes Dream.
No qualifying published lesson can exist today in any case.
This landing is NARROWED to:
1. the harness bump to f848c03f (already authorized);
2. the pinned read-prepare/1 codec mirror consuming the producer schema corpus;
3. kind=lesson search/read/context/evidence returning HONEST unavailable (no_published_input / transport_unavailable), with the ICD shapes and hints;
4. dream_feedback with original-op/request-digest idempotence and reported_only. Feedback referencing an offer that the host never made refuses honestly; no fabricated offer provenance.
DEFERRED to the new task tasks/2026-11-08-dream-q3b-readprepare-transport: ReadPrepare transport/provisioning in wake-up and logic dream_c1, the producer pin, the offer-provenance journal, and operation_status for Dream. Record these deferrals in the report.
Verify stays make test-dream-mcp && make test-formal-contracts, then LAND_MODE=wide make land once (submodule/Cargo.lock).


## Oracle amendment (2026-11-08): harness-bump landing; wake-up C1 compatibility
The wide land on 88a8a262+5bde29bb hit 42 failures (/tmp/dream-q3-land.log). About 39 are the harness PACKAGE's own integration tests (bboxy450-harness::dream_c1/acp_*/gui_*/store_*/context_auto/...). Term's wide gate runs them only because the submodule pointer changed, and without the harness Makefile's environment and features (for example DREAM_C1_RUNNER for dream_c1). The harness repository's own gate owns those tests. Their health on harness main is NOT established; it is tracked in tasks/2026-11-08-harness-test-health-at-f848.
Ruling:
1. Landing gate. For a commit range whose ONLY harness change is the submodule pointer (plus Cargo.lock), the term land gate runs:
   - cargo check of bboxy450-harness (lib, features as term uses them) and its lib unit tests;
   - FULL tests of every term crate that depends on it (bboxy450-harness-wake-up, bboxy450-harness-host, bbt-mux, bboxy450-term, and any others the planner finds).
   It does NOT run the harness package's own integration-test binaries (tests/*.rs inside crates/bboxy450-harness).
   Implement this in scripts/landing.py (and land-fast.sh if needed) with a self-test in scripts/test-landing.py, and document it in LAND.md. Use plain make land (lean). The LAND_MODE=wide instruction is withdrawn.
2. Real compatibility: the 3 bboxy450-harness-wake-up::dream_c1 failures (canonical disagreement: start / signed producer bytes) are genuine term-side breaks from the new pinned producer. Repair them in crates/bboxy450-harness-wake-up/src/dream_c1.rs and its tests/fixtures, ONLY to match the signed producer bytes of harness f848c03f byte for byte. Weaken no assertions, and do NOT add the ReadPrepare transport, which stays Q3b.
3. Any remaining failure in a TERM crate test is reported with its exact name. Do not ignore it.


## Report from seat:01M6YC2FT3YFH5DH6NJX7N8VJX

Landed the narrowed Dream Q3 slice on term main at signed 91fc1c4ad0a311bd2eaa99e8251018e47c73f320 with harness 645fec78174f59b06a9d6973271fc5e8d4ef013e: pinned read-prepare codec conformance, honest unavailable lesson tools, original-op/digest-idempotent feedback refusals with reported_only DTOs, shared structured hints, signed C1 byte compatibility and pointer-only consumer gating. Focused MCP/contracts checks passed; final make land exited 0 with 2,019 selected tests passed, 263 reported skips and a clean 861-model/275-contract/34-crate formal receipt using validated reuse. The readiness test passed unchanged; no conditional ignore was added. Positive transport/admission/provenance/operation_status and benefit scenarios remain Q3b deferrals. Full evidence, prior failures and the accepted upstream harness ignore are in the committed report (native checkpoint ddbfe01caa5971e8fb160deed5e277b717d47861).

```json
{
  "op_id": "01M6ZDHKDR1J0H834Z8X2190FC",
  "commit": "91fc1c4ad0a311bd2eaa99e8251018e47c73f320",
  "report": "[[reports/2026-11-05-dream-mcp-verified-feedback.md]]",
  "verify_result": {
    "command": "make test-dream-mcp && make test-formal-contracts",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Required focused verification; logs precede final pin, with all affected suites rerun in final landing",
        "reference": "/tmp/dream-q3-bc4-mcp-resume.log (34 passed); /tmp/dream-q3-bc4-contracts-resume.log (45 contracts + QoS/Python passed), actual exits 0; source/pin qualification and log hashes in committed report"
      },
      {
        "criterion": "Final current-source landing, all selected term consumers and harness lib",
        "reference": "/tmp/dream-q3-bd8e-disposition-land.log:3468-3469 — LANDED 91fc1c4ad0a311bd2eaa99e8251018e47c73f320; total exit=0; SHA256 525941172ee0169e11100a4813e5b00f07147cd081a8ed63ee5b96db912573d2"
      },
      {
        "criterion": "Current-source formal parity, coverage and source/model/tool receipts",
        "reference": "/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/721c95e37193dfe492d25a900c9934a9487abe4d66d34d93099b6777c7cca2e1/evidence.json — source 91fc1c4, dirty=false, 861 models/275 contracts/34 crates; make check-formal-evidence exit 0"
      },
      {
        "criterion": "Readiness disposition",
        "reference": "Final wake-up selection: 111 passed, 5 skipped; actual_private_process_clears_authority_environment_and_uses_fixed_argv unmodified and passing. Previous failure preserved in /tmp/dream-q3-645f-coverage-land.log, SHA256 ec1bc0ce641908d32a299d07a71d152ec5095268cfffc57a4c0269778e008813"
      },
      {
        "criterion": "Committed full report and remaining limits",
        "reference": "reports/2026-11-05-dream-mcp-verified-feedback.md, record 01M6ZDF0FJ8SKR05FPYZEKSGPD, publication_state=committed at ddbfe01caa5971e8fb160deed5e277b717d47861; report covers every narrowed acceptance line and original deferrals"
      },
      {
        "criterion": "Ancillary artifact-claim limitation",
        "reference": "knowledge_fetch confirms committed checkpoint 01M6ZDFV0BHTHSE5JY4VJPFA03, but artifact_claim refused artifact_not_committed for both returned artifact ID (op 01M6ZDGC9MGT4SP43TB6SHDMXG) and report path (op 01M6ZDGPVHEKENSX9KYG35VTP5). No artifact ownership claim is asserted; no bypass or further retry."
      }
    ]
  }
}
```
