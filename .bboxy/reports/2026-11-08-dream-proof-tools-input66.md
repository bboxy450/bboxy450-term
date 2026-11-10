---
type: report
title: 'Dream Proof Tools INPUT66: positive and corrupt proof pass; pause RI-age control fails'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T19:15:22.639Z
commits:
- 41c3982df58218ea4c4754febb653183ca512d63
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M6Y9CAAGQ0C77FCPJTRJSVJ3
updated: 2026-11-08T19:15:22.639Z
number: 93377961366395
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:ad290f14772b579dfdd0c9bd5242440630b04ecf8d8f310514c495d6f5513b56
  frontmatter_digest: sha256:7e911b203bcf2d0e33e6de8c180a83aa5a3ec18121507545cb782b55bbf97c17
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: JdsRbwr9RwRUm5KW4rU4kSJNFpV3JI9fbb42pkn9qOv+iPYACjkRhKjm35GPmpP7mhm1GGYFGH79E/Vym+/BAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150793
  event:
    id: 01M6Y9CD07XVVH864Y7GFP6R7Y
    sequence: 150793
    hash: d9df6d4d7130d8896def29be64411cd4bd90387888b7523030c4cc314d7a0fd8
---
# INPUT66: positive and corrupt-proof pass; pause-resume RI-age gate fails

BLOCKED after two native passes. Positive and corrupt_proof both exited0 and checked0, settled16/16, with all28 final-case terms true and first_failing_term="none". The same original invocation proceeded to mandatory pause_resume, which exited81 on resource_limit: observed live or paused RI start gap exceeds50ms. The retained historical maximum is103.627ms. Whole canonical verification exited2. No retry or teardown signal occurred; all three controllers were actually reaped. runtime_approved=false.

Exact decision needed: Oracle disposition of pause_resume's103.627ms RI-age failure and a causally changed authorized input or scoped repair before another attempt. The two passing cases remain passed. A later shorter RI interval does not erase the failed maximum, and no cap was waived. No precise operation causing the long interval is identified by the ledger alone.

## Scoped pin implementation and conditional freeze

Authority: [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], final ORACLE FREEZE, INPUT66. Signed code commit41c3982df58218ea4c4754febb653183ca512d63 changes only scripts/dream-proof-tools.py's selected tuple and tests/fixtures/dream-proof-tools/sources.lock.json's canonical_q2. All recipe code after the pin block is byte-identical to c5230f53, including checkers, budgets, original wait ownership and publication order. Attribution/policy headers remain attached. Independent read-only review confirmed the pin-only diff.

Boxy <bboxy450@gmail.com>, key9B07E849480C68328CA20684844ECC0587DE7AB0. git verify-commit HEAD exited0/Good, git diff --check0 and git status --porcelain=v1 0/empty. No code edit followed the signed cut.

The builder retains raw ledger bytes without validating row kinds/fields. The freeze's conditional final_case_conjunction whitelist expansion therefore requires no production schema change. Offline report analysis compared the new records against the exact C35 contract; it did not alter the parser or raw wire. Unknown/missing/duplicate result and terminal fields retain their existing strict checks.

Frozen source7449a048ad08e62e491be971e65c53695e3be940 verifies Good Boxy;28c91dff,f848c03f and2ce4359 ancestry checks exited0. Local origin/main was28c91dff;7449 containment returned1, explicitly NOT a gate. The freeze remains conditional on Q2 landing exactly7449a048; no landing claim is made.

Frozen directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input66/dream-read-c-preparation/compile-ifwmr6v1/.

- proof-helper-probe.c:366855B, SHA256fb34071ec30549c49423c35ab3f1e1068d49f5015ae593fe7215a09245af97e6.
- probe:268616B, SHA2564ba0533e6806678fcf12529192ef0f06a2381d9b2d74e4a1d0a0e78ab2d060b9.
- preparation-7c9fa3868bebdb25b55b6e59c65ea7d5327ee6e0577bc4258c41f83eda0e2b5a.json:530062B; hash matches name.
- INPUT66-source-freeze-tuple-v1.json:25185B, SHA256f943a8056e279e8279c09b3d6b7fdbd3e0773f2c3ded0316a86a9e33ef50b10a.
- input66-independent-audit-detail-v1.json:465626B, SHA2563113034dc5b1ed45523091db92e35bbc35a75320a829741570fb5b0faa429a17.
- C35-ledger-fields.json:3479B, SHA256c45e2253d22b99c13d08b8d66520c0980961fafe44717cd69626ba7a67286334.

All ten inputs match signed git-show and immutable snapshots. Only C changed; nine inputs match INPUT65. Codec blobs remain identical to b26d7cd3 by blob identity. Result/tail/terminal and prior ledger fields are unchanged. Full offline immutable ingestion exited0, including source/executable/receipt/compiler/linker/SDK/header/build-closure/Mach-O/platform/custody checks. Proposal source-domain SHA remainsd13d2e02152f4ec951487cb064a8ff8ee6266e207cb2b67f8cc9fdc0a17af65e. Conditional source/compiler/platform assumptions remain explicit; no whole-native-lifetime theorem or runtime grant is added.

## Commands and original run

E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input66-20261108.

`python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target` — exit0,66tests. E/unit.log:11357B, SHA256b959426e07d358b951e612ca703f916f93dc0dfb0e2e6c6dfcd0e1e810368ed7. Current selected ingestion and source-lock equality also checked0. Recipe SHA2569dd30b9df83fc0bf799da4a1861e36fec6599931d349b01bb7f573fc89125909.

/bin/df -k workspace exited0 before build with26,449,856KiB available, above the strict8,388,608KiB gate. E/disk-gate.json retains the observation. No cleanup by this seat. All seven fresh source-authentication records have auth_exit0 and all60 numbered configure/build/link commands000–059 exited0.

`make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`

ONE original invocation, actual exit2, started_ns1794164763162535000, ended_ns1794164988049064000. E/verify.log is9798B, SHA2564dae5d52c12c30d070e1f976119cd8b9f50ca18a3a02dd9c7936ddee9dfdf65a. It records positive actual_exit0/check_exit0 and corrupt_proof actual_exit0/check_exit0 before the pause-resume refusal and Make build Error1. No timing retry or re-execution of a passed case followed.

Build/run ID01M6Y8VDN0ZJ8XMYC5D04V0FGJ. R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6Y8VDN0ZJ8XMYC5D04V0FGJ. P=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6Y8VDN0ZJ8XMYC5D04V0FGJ. Original prefix dev16777234/inode187302846/incarnation1633753129615083 still matches intent and descriptors. Shared basis SHA2563d11ff486105299196ba66e8b2b801432d7b8ed6c5357b7fc280b2de0520e902.

## Requested outcomes and first-failing terms

| Case | Actual controller exit | Consumer check | Proof/outcome | first_failing_term |
| --- | ---: | ---: | --- | --- |
| positive |0|0|valid/completed, settled16/16|none|
| corrupt_proof |0|0|invalid/expected_rejection, settled16/16|none|
| pause_resume |81|refused|not_reached/failed, settled1/1|not captured: complete-case hook not reached|

Each successful ledger has final_case_conjunction at line92. All28 term values are actual JSON booleans true, coverage_passed/resources_passed true, runtime_approved false, and first_failing_term exactly"none". Key sets and ordering-derived terms match the signed contract. Capture cutoff is exactly before_this_diagnostic_format_write_fsync_close. Separate subsequent formatter/write/fsync/close payment and actual controller/check exits also succeeded; the pre-write snapshot is not treated as a future-success claim.

E/conjunction-contract-check.json stores the exact28 keys, contract hash and per-case comparison. Production ledger handling remains raw retention. Pause-resume has no conjunction record after its early command failure; no enum value was fabricated for that absence.

## Requested debt splits, margins and ordinary maxima

All costs below are final whole-controller cumulative values, and9×(ordinary+helper)=charged=paid for each case. Raw ordinary plus floor surcharge equals ordinary cost; unique helper costs sum to helper total, with no repeated owner/event snapshots or global watermarks summed.

| Case | Helper debt s | Ordinary raw debt s | Ordinary floor debt s | Final total s |
| --- | ---: | ---: | ---: | ---: |
| positive |1.165986|1.228146003|9.031853997|11.425986|
| corrupt_proof |1.090962|1.219613994|8.905386006|11.215962|
| pause_resume |0.110367|0.429409494|6.014590506|6.554367|

Underlying positive costs: helper129554000ns, ordinary_raw136460667ns, floor1003539333ns, ordinary_total1140000000ns. Corrupt costs: helper121218000ns, raw135512666ns, floor989487334ns, ordinary_total1125000000ns. Pause costs: helper12263000ns, raw47712166ns, floor668287834ns, ordinary_total716000000ns. All final ordinary_raw_complete/debt_components_match flags are true. Original raw-result pre-emission debts11.371986/11.161962/6.491367s remain unchanged at their earlier cutoffs.

| Case | Admitted entry→deadline s | Entry→reporting s | Margin at reporting start s | Raw ordinary_max_ns | Terminal reporting-tail ordinary_max_ns |
| --- | ---: | ---: | ---: | ---: | ---: |
| positive |29.827851|13.233093|16.594758|519875|426542|
| corrupt_proof |29.991914|13.006366|16.985548|558833|433708|
| pause_resume |29.992036|7.620140|22.371896|429916|429916|

Margins use serialized original_deadline_ns−reporting_started_ns. They are per-attempt reporting-start observations, not general timing guarantees or runtime approval. The first two cases actually passed their original deadline gates. Pause's spare deadline margin does not excuse its independent50ms RI-age failure. All three final process_create excess values are0ns/count0/known1; no deadline error was recorded.

Source/basis-derived G43=26candidate+17fixture guards, NOT MEASURED. E/guard-derivation.json enumerates unique(root,path) admissions, collapses equal duplicate bindings, excludes absolute compiler/SDK references and introduces no runtime counter.

## Pause-resume failure: historical age versus later RI sample

Pause controllerPID68179 was actually reaped with exit81; native helperPID68760 was own-wait4-reaped and group gone. Its ledger line2 records sample_gap_ns=max_sample_gap_ns103627000, beyond50000000. The native group kill succeeded and no external teardown was needed. C:3076–3089 checks age for live or paused members and refuses the recorded excess.

Ledger3 has ack_mask1/baseline_known1, unchanged raw user85305→85305, user_excess_ns0, live_checked1/terminal_checked1/pause_user_pass1, sticky_violation0 and resume_admitted0. Thus positive STOP/ACK and no user-CPU increase were observed; a successful admitted resume was not. Foreground/ACK/resume ordering is at C:3356–3396,3838–3846.

Ledger4 retains a later RI-start interval1633972032743000→1633972045340000=12.597ms with sticky query-failure mask0. It does not erase the historical103.627ms maximum. The records do not locate the exact operation that caused the long interval or distinguish scheduler delay/rest/other work. Independent read-only audit confirmed these evidence limits. No threshold change or lucky rerun was attempted.

Pause result is settled1/1/retired-coveragetrue but proofnotreached, outcomefailed/resource_limit. Its terminal-cost/persistence passed while control failed and resource_limit count1 remains recorded. This is a failed control, despite actual81 matching one part of the consumer's pause-case exit expectation.

## Raw artifacts and acceptance evidence

Each stdout is byte-identical to its persisted output/result.json. Original artifacts remain unchanged:

| Case/artifact | Bytes | SHA256 |
| --- | ---: | --- |
| positive stdout/result |1935each|92c06bcd00127b44c4c77da6a81babe499a9f3ff5eb4f533a3b2b535b4470b3d|
| positive output/ledger.ndjson |51448|0652954d5264dfe216400263484db1feb55d38478e0bb9e31e20c1cd8e9e0e42|
| positive wait4.json |136|f345025f6ac6d8c4a4729fdb935fffdffb8c826a6295a25e1aea0ada0c03116d|
| corrupt_proof stdout/result |1951each|5ece4649129df874689c2c51c0302db0d6a2fcaf5d5dd8671161d9160c61aed4|
| corrupt_proof output/ledger.ndjson |51439|1f2ab3ee6de185e495e97f54e241e07a055b7af8fb88151c073d5f5f88f0def6|
| corrupt_proof wait4.json |136|d3d82cac8bc26fbd716d0e0c2e69f1c7ffe553ee598e58027175b11442ff2794|
| pause_resume stdout/result |2013each|6783f5ca8cb2c9aaf7e6ece5dc3eb6cf310327cba65bd7f0a6c5c44a8f13c677|
| pause_resume output/ledger.ndjson |3706|37f5572314e00c5655a97128947cc52c4cc2eff32a217c1a8a1b66e878c8890a|
| pause_resume wait4.json |140|838cda9a8d66c60d75ff327f48ef1312028cfc6f67265b9d0175cc059dd34fc3|

- Exact frozen ingestion/signature/ancestry/format compatibility: passed offline; E/preflight.json, selection.json, source-byte-checks.json, ingestion.json, terminal-binding.json and tuple-binding.json retain commands/hashes. Main containment was not a gate.
- Scoped pin implementation and current66units: exit0, unchanged validators/budgets, no schema/dependency/runtime expansion. New diagnostic records remain raw retained; offline contract comparisons passed.
- Real auth/build:7fresh authenticators and60numbered commands0. Positive and corrupt controls both pass actual0/check0 with full16/16 settlement and all28terms true.
- Original required case ordering continues to pause_resume, whose actual resource-limit failure stops the invocation. cancel,stale_incarnation,clock_failure,debt_overlap,reap_delay, final formal/license gates, command-tail tests, qualification, receipt and landing did not run. No task-green or runtime-admission claim.
- All17historical ledgers retain original dev/inode/length/SHA256 in E/preserved-ledgers.json. No reset, stash, source/prefix deletion, peer cleanup or failed-effect replacement occurred.
- E/final-state.json records signed clean source, original prefix identity, both native passes, actual pause81/canonical2, missing qualification/receipt and no retry/teardown signals. All controllers settled; no live quarantine is retained.

E/telemetry.json preserves all three result/terminal cutoffs, actual wait4/check records, conjunction records, helper histories, RI/pause evidence and requested arithmetic. E/handoff.json:211607B, SHA2561adbee2f670a538b2de829ebdde882f9b8b85d75316b90062249550eb2ebe122. This is offline provenance, not a runtime input-budget assertion. The two requested proof cases are PASSED; full task qualification remains blocked on the failed pause-resume control.