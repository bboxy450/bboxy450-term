---
type: report
title: 'Dream Proof Tools INPUT65: valid proof and settlement but complete-case coverage refusal'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T17:40:10.982Z
commits:
- c5230f53ddb2be8e620ac40a5690eef529d3c214
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M6Y3Y1YDTMBX9VXR2F9Y7G1B
updated: 2026-11-08T17:40:10.982Z
number: 93377961366389
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:5770c0db041d1d1763e61dd08012f03972a50fc20c3e9347fa0a4fdbc41a9c32
  frontmatter_digest: sha256:194c76d54eebb0057213f1ce7bec4ad19d1f3c2c8870febe334ff390825da114
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: T84TlVZ147WMa5mSboq6S8jpC7Eo1B04KS0fvWL0NkLAK6uOtT8Cr4zAbO2A2wCpt1Q3tfSiYQDMBl7B5KQhCA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150768
  event:
    id: 01M6Y3Y36T3ZQHBFCXFZ26SG52
    sequence: 150768
    hash: a76bbdcac298a1b398b1a4511325f51abb40a781b304d25c1242b7975c505d7b
---
# INPUT65: actual exit81 after valid proof/settlement; visible complete-case predicate fails

BLOCKED. The sole INPUT65 positive exited81, and the canonical command exited2. Raw proof is valid and physical settlement is16/16, but outcomefailed/control_verdictfailed with coverage_unknown: complete fixed case lacks actual semantic, verifier or physical settlement coverage. The visible failed predicate is members_max1 where frozen C requires2. The verifier was discovered, admitted, bound by GS1/GS2 and settled with kqueue_note_exit/mask31. No retry or teardown signal was issued. runtime_approved=false.

Exact decision needed: Oracle disposition of the producer's complete-case members_max predicate/bookkeeping, then a causally changed authorized tuple or scoped repair before another attempt. No local recipe mismatch is established. Other opaque terms of the final conjunction are not serialized, so this report does not claim members_max was its sole failed term. corrupt_proof was not dispatched after the positive refusal.

## Scoped implementation and exact conditional freeze

Task [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], final ORACLE FREEZE, INPUT65, replacing void INPUT64. Signed implementation commitc5230f53ddb2be8e620ac40a5690eef529d3c214 changes only scripts/dream-proof-tools.py's tuple pins and tests/fixtures/dream-proof-tools/sources.lock.json's canonical_q2. All code after the pin block, including strict result/terminal checks, budgets, raw ledger retention and publication order, is byte-identical to4332bd53. Existing origin/policy headers remain attached. Independent read-only review confirmed this pin-only diff.

Git signer/author: Boxy <bboxy450@gmail.com>, key9B07E849480C68328CA20684844ECC0587DE7AB0. git verify-commit HEAD exited0/Good; git diff --check0; git status --porcelain=v1 0/empty. No source edits occurred after the signed cut.

Source28c91dff6411ef18e8fec1eea68328e090cac8e3 verifies Good Boxy. Requiredf848c03f,390b5836 and2ce4359 ancestry checks exited0. Local origin/main wasf848c03ff0462e55dfa0b17fcfd550626a52db2b; source containment returned1 and was explicitly NOT a gate. The task condition is Q2 landing exactly28c91dff; no main-landing claim is made.

Frozen directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input65/dream-read-c-preparation/compile-cr0g4pl5/.

- proof-helper-probe.c:361112B, SHA256ca5bd97ca8197662d706837021b154509ca4d593a87a22ef7710c6437023c02e.
- probe:268392B, SHA256bc9e220cbc8f68cfc64f69f8ad2ee4fefee45ca383744cb76b323602b9e3bf1f.
- preparation-ef15bc9f8757ae3d9c39764e6397f53262809ed7259a59a8f65bdca700a8edf4.json:530062B; hash matches name.
- INPUT65-source-freeze-tuple-v1.json:23428B, SHA2563d961ab861ec4d543a44b3847ad5cd2eb1cc28f3a2e2488e6e38e4d67ab14095.
- input65-independent-audit-detail-v1.json:465626B, SHA25689f672edf5402a457b63f5645c4cd06c8391084e75a8c3796aff6365e33f1f55.

All ten inputs match signed git-show and immutable snapshot bytes; only C changed, nine inputs match INPUT64. Schema/vector blobs remain identical to b26d7cd3 by blob identity. Result/tail/terminal and ledger field contracts remain unchanged. The adapter retains raw ledger bytes without a field/kind parser, so there is no new whitelist or schema mirror.

Full offline immutable ingestion exited0, including source/executable/receipt, compiler/linker/SDK/header/build-closure/Mach-O/current-platform/retained-output/custody checks. Proposal source-domain SHA remainsd13d2e02152f4ec951487cb064a8ff8ee6266e207cb2b67f8cc9fdc0a17af65e. These are conditional source/compiler/platform premises, not a whole-native-lifetime theorem or runtime approval.

## Commands and original artifacts

E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input65-20261108.

`python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target` — exit0,66tests. E/unit.log:11357B, SHA256a0202e077433c41478d33b015c75f4bb378c1311d6e36992db28f0a19879973e. Current selected ingestion and source-lock equality also checked0. Recipe SHA256c8d16937f5f20124f07e1aab4a969f43c51056972e528f3ab70d430af6107211.

/bin/df -k workspace exited0 before build with26,615,356KiB available, above the strict8,388,608KiB gate. E/disk-gate.json retains the actual observation. No cleanup by this seat. Seven fresh source-authentication records have auth_exit0 and all60 numbered configure/build/link commands000–059 exited0.

`make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`

ONE original invocation, actual exit2, started_ns1794159170243204000, ended_ns1794159371098495000. E/verify.log is9665B, SHA2564bf341ba42c06ef8ff0520c01b712495588eabe96269960461329fcbe4db4c06, ending canonical real proof/component verdict refusal; no timing retry, Make build Error1. No later command in the && tail ran.

Build/run ID01M6Y3GQNVZDMH82PY9DNMKAT9. R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6Y3GQNVZDMH82PY9DNMKAT9. P=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6Y3GQNVZDMH82PY9DNMKAT9. C=R/components/positive. Original incarnation1628160084884666; descriptor3439B SHA25618080d0dd05a4e041f766f108b016f242f33c1a84568181aa501e19f8a54124a. Original basis47112B SHA256dc5b628239f459b97bdae6190b487ee1af450a55a9ca7e67cd237ae21a7041d3.

C/wait4.json proves actual own-parent reap of controllerPID62745 with exit81/status20736, user_cpu_ns85857000, system_cpu_ns243005000 and max_rss_bytes4177920. This was a natural failed controller exit, not forced teardown or inferred exit from result wire.

| Original artifact | Bytes | SHA256 |
| --- | ---: | --- |
| C/stdout = C/output/result.json |2052each|609a83cbecec0f656de8b89f17b386b0e437a04f79bae88bef48d14899bb5e02|
| C/stderr |3037|57c17fb75343174afcc6a6417438edd58feb3070143266d65ff10cf424e616e7|
| C/output/ledger.ndjson |50501|db7e2fbbbfbc3aaf567279b519bf5691f38860e12777e0204e8fb838a024ab20|
| C/wait4.json |141|78a5227d440859a243d6897bf5f40b560ae3b3b7f0da885fcafdb70fea48f8bb|
| C/retained.json |1410|fc678e9479af5a403ffb916b8043b22a27ed1578983dae11dbb1f69b04e06b93|

Raw result remains phase settled/outcomefailed, proofvalid, coverage_unknown, created16/reaped16/group-gonetrue/retired-coveragetrue, members_max1. Final terminal is settled but control_verdictfailed, persistencepersisted, terminal-costpassed, error_count1/coverage_unknown1. Persistence and physical settlement do not turn the complete-case refusal into success.

## Requested two-case telemetry

| Value | Positive | corrupt_proof |
| --- | --- | --- |
| Actual controller exit |81, failed qualification|NOT RUN|
| Final paid helper debt |1.414215s|NOT RUN|
| Final paid ordinary raw debt |1.525853250s|NOT RUN|
| Final paid ordinary floor debt |8.950146750s|NOT RUN|
| Final paid total |11.890215s|NOT RUN|
| Admitted entry→deadline interval |29.857377s|NOT RUN|
| Entry→reporting-start elapsed |13.689567s|NOT RUN|
| Margin at reporting-start cutoff |16.167810s|NOT RUN|
| ordinary_max_ns, raw control metric |749833ns|NOT RUN|
| Final reporting-tail ordinary_max_ns |464542ns|NOT RUN|
| Final process_create excess |0ns/count0/known1|NOT RUN|

The margin is original_deadline_ns1628371609357000 minus reporting_started_ns1628355441547000; original_entry_ns1628341751980000. It is a reporting-start observation, not a successful-positive or general timing guarantee. deadline_observed_ns0. The failure was coverage, with no ordinary_failed_charge row.

Final costs: helper157135000ns, ordinary_raw169539250ns, ordinary_floor994460750ns, ordinary_total1164000000ns. Raw+floor equals ordinary total;9×(ordinary+helper)=charged=paid11890215000ns. ordinary_raw_complete/debt_components_match=true. Earlier raw-result debt11827215000ns is preserved at its pre-emission cutoff. All15 unique helper-owner costs sum to the final helper cost total; repeated snapshots/cumulative watermarks were not summed.

Source/basis-derived G43=26candidate+17fixture guards, NOT MEASURED; E/guard-derivation.json enumerates unique(root,path) admissions and equal-duplicate collapse, excluding absolute compiler/SDK review references. No runtime G counter was introduced.

## Signature leader verifier rows and visible failed conjunction

C/output/ledger.ndjson:91 is signature leader sequence16/PID63912:

```json
{"kind":"verifier_event_discovery","leader_fork_events":1,"leader_exec_events":0,"child_discovered_via":"fork_event","fork_seen_child_missed":false,"notification_counts_not_birth_counts":true,"registered_mask":3,"registration_errno":[0,0],"wait_errno":0,"queue_fd_held":0,"close_attempted":1,"close_errno":0,"runtime_approved":false}
```

Ledger89 records verifier63916, child_created1, child_image_admitted1, child_exit_witnesskqueue_note_exit, terminal_observed_ns1628354873008000, child_exit_abstime0, rollup_mask31, rollup_proven1 and child_cost_unproven0. Leader own wait4/group-gone is recorded. GS1/GS2 bind signature inode187091222/hashd716c2d0807ca1339c774486d55d31768fa9f953df90872f2ceef4cb4568c792 with nlink1→0. Thus discovery, image admission, signature custody, typed exit witness and qualified roll-up are retained; no raw field was rewritten.

Frozen C:4116–4121 requires members_max2 in the final conjunction; wire value1 is a directly established failed term. The field's only writes are initial leader admission at:3524 and later live group census at:3237–3240. Same-wake child admission does not update it. A verifier can therefore be retained/admitted/settled before a later census samples two processes together. That is a source-supported bookkeeping explanation; exact missed-census timing remains inference. Other internal conjunction flags are not serialized, so this report does not assert that this was the only failed term. Independent read-only audit confirmed these limits.

## Acceptance evidence and remaining work

- Exact freeze/signature/ancestry/custody: offline passed; E/preflight.json, source-byte-checks.json, selection.json, ingestion.json, terminal-binding.json and tuple-binding.json retain commands and hashes. Main containment was not made a gate.
- Scoped adapter/current controls: pin-only change, unchanged validators/budgets,66units0 and current ingestion0. No wire, dependency, model contract or runtime grant changed.
- Real source auth/build:7fresh authenticators and60numbered commands0. Native positive actually81 and consumer verdict refuses it; valid crypto and physical16/16 settlement do not satisfy all producer/consumer acceptance predicates.
- Positive→corrupt_proof→remaining components→final gates→qualification→receipt: stopped at the positive refusal. corrupt_proof, all later cases, final formal/license gates, command-tail tests, qualification, receipt and landing did not run. Qualification/receipt files are absent. No approved runtime, available lesson or task-green claim.
- All16historical ledgers retain original dev/inode/length/SHA256 in E/preserved-ledgers.json. No reset, stash, historical source/prefix deletion or peer cleanup occurred.
- E/final-state.json records signed clean source, only positive, actual controller81/canonical2, no checked positive or candidate receipt, actual controller reap and no retry/teardown signals.

E/telemetry.json retains unaltered raw result values, final terminal, signature rows, actual wait4, per-owner helper history and requested arithmetic. E/handoff.json:94532B, SHA2564f009bf700da852a594f5af459b13b5a475259770a18faf4d90aa62f3a8e9cc5. It binds original artifacts, every numbered build exit, fresh authenticators and predecessor preservation. It is offline provenance, not a runtime budget grant. Current qualification remains FAILED81; a changed authorized input or scoped repair is required before another attempt.