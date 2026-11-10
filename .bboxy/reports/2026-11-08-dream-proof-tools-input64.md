---
type: report
title: 'Dream Proof Tools INPUT64: positive blocked by missing image admission after retained child exit'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T15:41:11.854Z
commits:
- 4332bd5327906bac33cd94c4e2e17dad904d869e
verify:
  command: python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target
  exit_code: 0
id: 01M6XX46GXT9CMT7D4XZ5FRCMX
updated: 2026-11-08T15:41:11.854Z
number: 93377961366380
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:10ea21253b5a129afb200c37de6de92e6796c1b9db75f29d12bf0459c5926289
  frontmatter_digest: sha256:a3b184f8ef573ea869d228eb7f396859143c3cd84ad9ef0979c8b31102f53903
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: qi0UkMl7xN+msykkSWdi3ppR9QakNVRKbVxbZ93UxtZTlAFQ7+SjtWlrJAkCEiQQkHMsUsbKNE4MclBWZgFkCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150729
  event:
    id: 01M6XX47CP8XC2VSCZM1T9TMDV
    sequence: 150729
    hash: 2471f55660f7bace65a63eacfbea21d62163cbea16e7d71048fde5f3f1a87910
---
# INPUT64 sole attempt: retained child exit, missing live image/GS1 admission

BLOCKED in the positive. The one authorized INPUT64 invocation emitted coverage_unknown: retired child lacks qualified nonduplicated same-leader rollup. Its retained child has a kqueue_note_exit witness but child_image_admitted=0 and rollup_mask30, missing admission bit1. No GS1 record exists. Controller89578 and original parent41501 remain live with the owned handle retained. Actual controller/canonical exits are UNKNOWN. corrupt_proof did not run. No retry, signal, substitute input or cap change occurred; runtime_approved=false.

Exact decision needed: Oracle disposition of the missing live image/GS1 admission and original live89578/41501 custody, followed by a causally changed authorized tuple or scoped repair before another attempt. Earlier signal authorizations named different controllers and were not reused. The INPUT63 positive remains its historical PASSED result; it is not substituted for this INPUT64 refusal.

Report metadata verify records only the completed66-unit command, exit0. The required canonical command is still running without an actual exit; no new canonical verify_result is supplied to task_report.

## Exact authority, scoped pin change and preflight

Task [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], final ORACLE FREEZE, INPUT64. Signed code commit4332bd5327906bac33cd94c4e2e17dad904d869e changes only scripts/dream-proof-tools.py's selected tuple and tests/fixtures/dream-proof-tools/sources.lock.json's canonical_q2. All recipe code after the pin block is byte-identical to9ff0f6ac, including validators, budgets, ledger retention and publication order. Existing source attribution/policy headers remain attached. Independent read-only audit confirmed the pin-only change.

Boxy <bboxy450@gmail.com>, signing key9B07E849480C68328CA20684844ECC0587DE7AB0; git verify-commit HEAD exited0/Good, git diff --check0, git status --porcelain=v1 0/empty.

The source is pinned by SHA under the task's conditional freeze, not by main containment. Source1ec942b280cfe123ecaf70e2dbf5db8ccf2cbe3b verifies Good Boxy, and both390b5836 and2ce4359 ancestry checks exited0. Local origin/main was390b5836; containment returned1, explicitly an observation rather than a failing gate. No differing landed INPUT64 source was observed; Q2's source still matches the frozen C. The freeze remains conditional on Q2 landing exactly1ec942b2; no claim of main landing is made.

Frozen directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input64/dream-read-c-preparation/compile-4ckdm3ob/.

- C:355016B, SHA25615fb63e892d5bc4c5b91a8b582981edc96a7266ce0b632f79d413176bdf3dbf1.
- probe:268232B, SHA256ac88eabe597c2603a0dd21b0c689cb96c22d4d14522d51736fecc89e96301ef5.
- preparation-66bac599863ce564debe34a2482a6fd4d648772e39855479aded0f0ddaa083e5.json:530062B, hash matches name.
- INPUT64-source-freeze-tuple-v1.json:21130B, SHA25660d674e56c196603418112f552c3647bee0283cbce08fdc6fe32afd4f588cc2b.
- input64-independent-audit-detail-v1.json:465626B, SHA2568aa0d6d849b28e21d6aa5e8f09aa2b8c3bcabd4b7c98bb89d270ea7c088d0ebe.

All ten inputs match signed git-show and immutable snapshot bytes; only C changed, the other nine match INPUT63. Schema/vector blobs remain identical to b26d7cd3 by blob identity. Result/tail/terminal and existing ledger format lines are byte-identical. The builder retains raw ledger bytes without a schema parser, so no new fields/checker change is needed.

Full offline immutable ingestion exited0, validating source/executable/receipt, compiler/linker/SDK/headers/retained outputs/build closure/Mach-O/current platform/custody. Source-domain proposal SHA remainsd13d2e02152f4ec951487cb064a8ff8ee6266e207cb2b67f8cc9fdc0a17af65e. These are conditional source/compiler/platform premises, not a whole-native-lifetime theorem or runtime grant.

## Commands and original run

E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input64-20261108.

`python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target` — exit0,66tests. E/unit.log:11357B, SHA256822dd830adfb4b8a939cfd2165689ca2a42016310f34e2ab1c489b498b464ee1. Current selected-input ingestion and canonical source-lock equality also checked0. E/scope-audit.json binds recipe SHA256b4594e8064959876126859886fd0729d2c48cd24b5270e618a554b7c0d4387da.

/bin/df -k workspace exited0 before build, with24,774,184KiB available, above the strict8,388,608KiB gate. E/disk-gate.json retains the observation. No cleanup by this seat.

`make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses` — launched ONCE, still running, actual exit UNKNOWN. All seven fresh source-authentication records have auth_exit0; all60 numbered configure/build/link command exit records000–059 are0. The log records OWNED PENDING for89578. No positive checked result or actual wait4 exists, so the builder has not dispatched corrupt_proof. E/verify-exit.json is absent. The original exec session30398 and parent remain retained; no timeout exit or dropped wait ownership is substituted.

Build/run ID01M6XWN035QSQRFAB367J0AZG0. R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6XWN035QSQRFAB367J0AZG0. P=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6XWN035QSQRFAB367J0AZG0. C=R/components/positive. Original incarnation1620959772800458; descriptor3437B SHA256507bc575ed406d9b2254aba24bacd196b48a7919ac2fca53fcaa3b633a4028ed. Original basis47110B SHA25689d00ca46a455af4ade512391b024933d009dedf4276e12f21c926d14730f2b4.

Raw result remains quarantined/proofnotreached, created16/reaped15/group-gonefalse/retired-coveragefalse, members_max1. Actual ps on exact controller89578 exited0 and observed its original parent41501 and frozen probe command. C/owned-pending.json records the actual retained parent handle; C/wait4.json is absent.

## Requested telemetry and unavailable final observations

| Requested value | Positive | corrupt_proof |
| --- | --- | --- |
| Final helper/raw/floor debt split |UNAVAILABLE: live quarantine prevents final terminal|NOT RUN|
| Persisted pre-result paid debt |12.647472372s|NOT RUN|
| Booked helper debt subtotal, derived from unique owner rows |1.450737s|NOT RUN|
| Snapshot ordinary aggregate debt remainder, derived |11.196735372s; raw/floor separation unavailable|NOT RUN|
| Final margin against admitted interval |UNAVAILABLE: no final entry/reporting terminal fields|NOT RUN|
| Partial margin at last retained leader-RI observation |15.309174s; not final or proof-completion margin|NOT RUN|
| ordinary_max_ns in raw result metrics |660041ns|NOT RUN|
| Retained process_create excess subtotal |0ns/count0/all-known1|NOT RUN|

Partial margin is precisely descriptor deadline1621161153938000 minus retained leader_rollup_read_ns1621145844764000. It is not the projected >=3s final margin and establishes no30s-fit verdict. No final original_entry/reporting_started fields were emitted. Snapshot debt arithmetic uses15 unique helper cost rows totaling161193000ns ×9; no repeated event/owner snapshots or cumulative watermark rows were summed. Qualified child cost remains explicitly unproven, so the booked subtotal is not a complete child-CPU theorem.

ordinary_max_ns660041 is the raw result's control/pre-emission cutoff, not an unavailable final reporting-tail maximum. No ordinary_failed_charge row exists; the recorded failure is coverage, not the1ms gate. The raw controller max is659541ns. Final terminal debt, raw ordinary CPU and floor surcharge values are unavailable, not zero.

Source/basis-derived G43=26candidate+17fixture guards, NOT MEASURED. E/guard-derivation.json enumerates unique(root,path) admissions, collapses equal duplicates, excludes absolute compiler/SDK review references and claims no runtime G counter.

## Signature leader and failed admission predicate

Ledger91 is signature leader sequence16/PID90581, per-owner snapshot:

```json
{"kind":"verifier_event_discovery","leader_fork_events":1,"leader_exec_events":0,"child_discovered_via":"fork_event","fork_seen_child_missed":false,"notification_counts_not_birth_counts":true,"registered_mask":3,"registration_errno":[0,0],"wait_errno":0,"queue_fd_held":1,"close_attempted":0,"close_errno":0,"runtime_approved":false}
```

Ledger89 records child90584, child_created1, child_image_admitted0, child_exit_witnesskqueue_note_exit, terminal_observed_ns1621145823036000, child_exit_abstime0, rollup_mask30, rollup_proven0, child_cost_unproven1 and unproven_child_billed_as_leaderfalse. Signature leader own wait4/group-gone is recorded, but the original controller is still unreaped. Stderr is empty: no GS1 or GS2 record exists. Fork discovery and NOTE_EXIT termination alone do not prove live qualified-image/GS1 admission.

Frozen C:3540 supplies mask bit1 only when child_image_admitted. Mask30 is missing precisely that bit. The new cut at:3093 can pay rest once the child token is known, before sampling/STOP/GS1 at:3095,:2979–2985; a known clone can exit in that interval. This is a source-supported possibility, not an established cause of this particular miss. The ledger proves absent admission, not which scheduling interval caused it.

Failure at:3546 sets sticky child_cost_unproven via:2566; settlement at:2570 requires covered child cost. A later hold cannot reconstruct missing live GS1 admission or clear that audit flag. Independent read-only audit confirmed this predicate chain. No Q2 source, immutable probe or controller was edited.

## Raw artifacts and acceptance accounting

| Original artifact | Bytes | SHA256 |
| --- | ---: | --- |
| C/stdout = C/output/result.json |2048each|26a8531436559432fdc252610b04730e7a6e2beacb7129e8a616941dd1336ad4|
| C/stderr |0|e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855|
| C/output/ledger.ndjson |50507|8fabd6b5e8974ea5f50019f90c6bb7abcd2393da7104c08ca047046340b551d9|
| C/owned-pending.json |86|4b5141cfaa38ae1cfaf892172149d6f26b71517123ff9852436e4fa409de7705|

- Exact freeze/source authenticity/custody: offline passed; E/preflight.json, source-byte-checks.json, selection.json, ingestion.json, terminal-binding.json and tuple-binding.json retain actual commands and hashes. Main containment was not made a gate.
- Scoped adapter and controls: pin-only change, unchanged checkers/budgets,66units0 and complete current-source ingestion0. No schema expansion or dependency was introduced.
- Private real source authentication/build:7fresh authenticators and60numbered commands0. The original positive remains refused/pending actual controller reap; source build success is not native qualification.
- Positive→corrupt_proof→remaining components→final gates→qualification→receipt: stopped in the original positive's live quarantine. corrupt_proof and all later cases, final formal/license gates, command-tail tests, qualification, receipt and landing did not run. P/qualification.json and P/receipt.candidate.json are absent.
- All15historical ledgers match original dev/inode/length/SHA256 in E/preserved-ledgers.json. No reset, stash, historical prefix/source/output deletion or peer cleanup occurred.
- E/pending-state.json records signed clean source, only the positive, missing canonical exit/qualification/receipt and retained original89578/41501 custody. Signal authorizations for older runs were not reused.

E/pending-telemetry.json retains original raw result, all unique-owner helper history, signature event/child rows, snapshot arithmetic and live-process observation. E/pending-handoff.json:101475B, SHA256f2613bc0da3aeacd06e8ccdbbbd557c5e4569f46e764bac4fa84a988d0b48f08. It binds raw artifacts, all numbered exits, authenticators, G derivation and predecessor preservation. It is offline provenance, not a runtime input-budget grant. Current qualification remains BLOCKED; no actual canonical exit or final two-case margin/debt evidence is claimed.