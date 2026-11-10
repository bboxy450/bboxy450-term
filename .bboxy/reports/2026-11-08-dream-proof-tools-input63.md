---
type: report
title: 'Dream Proof Tools INPUT63: positive passes, corrupt-proof ordinary-unit quarantine'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T13:59:45.421Z
commits:
- 9ff0f6acbe32c7dfb0b06dcb870783defc8489f7
verify:
  command: python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target
  exit_code: 0
id: 01M6XQAETHPH7E6XH3H4CWW7KA
updated: 2026-11-08T13:59:45.421Z
number: 93377961366372
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:00e71592d2e34a04d8017fa6222ddf9e318cd44580283ab7a7301d7c5b45dd45
  frontmatter_digest: sha256:baf2ec1e7a4102df3b621f03536ecb3fac4581a5013b030f2c6e6d35c6bd1700
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: TxUBauom1vUrkFU4382KkmAevO1Q+b7tRQpqcwtyN3eMYRyaRg1kNrPq3juNX0P7W39z59C5YLj55RnIszSUCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150701
  event:
    id: 01M6XQAFKFHD50X2PA53YP3DB0
    sequence: 150701
    hash: 5996bb3865877dacf099610bca0bc49ff3cf7159c6b85888558678d22e0e8336
---
# INPUT63: positive passes; corrupt-proof control remains live quarantined

BLOCKED after the positive passed with actual controller exit 0 and checker exit 0. The same original qualification invocation proceeded to corrupt_proof, which refused an ordinary engine_checkpoint of 1,020,958 ns against the unchanged 1,000,000 ns ceiling. Its controller PID 53612 and original wait parent PID 5130 remain alive and retained. Corrupt controller and canonical command exits are UNKNOWN, not 81 or a timeout status. No retry, signal or replacement occurred. runtime_approved=false.

The exact decision needed is Oracle disposition of the corrupt-proof ordinary-unit failure and original live 53612/5130 custody, followed by a causally changed authorized tuple or scoped repair before another qualification attempt. The earlier teardown authorization named only INPUT62 controller94751 and does not cover this controller. No additional native attempt or landing is authorized by a passing positive alone.

Report metadata verify records ONLY the completed 66-unit command, exit0. The required canonical command remains RUNNING without a real exit; no new canonical verify_result is supplied to task_report.

## Scoped implementation and exact freeze

Signed code commit 9ff0f6acbe32c7dfb0b06dcb870783defc8489f7, Boxy <bboxy450@gmail.com>, key 9B07E849480C68328CA20684844ECC0587DE7AB0. Only scripts/dream-proof-tools.py and tests/fixtures/dream-proof-tools/sources.lock.json changed, selecting the exact INPUT63 source/executable/preparation and canonical_q2 source reference. All recipe code after the pin block is byte-identical to0068adc9, including terminal/result checkers and budgets. Attribution/source headers remain attached.

The freeze condition concerning a child_exit_witness parser does not apply: scripts/dream-proof-tools.py:3173–3175 retains ledger bytes without parsing ledger fields or kinds. No new decoder, schema mirror, test counter, cap expansion, Q2 edit or runtime grant was introduced. Existing 66 unit tests cover the unchanged strict checkers. Independent read-only review confirmed this pin-only diff.

Authority: [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], final ORACLE FREEZE, INPUT63; decisions66280/66331 and ordinary1ms/nativeCPU2ms/inclusive3ms laws remain unchanged.

- Source/preparation commit390b5836fb3693e3c4861a98def1ef8b53d54d5b and parentb1df2c4c verify Good Boxy, exit0. INPUT62 7ab80fac is an ancestor and390b5836 is on observed origin/main, merge-base checks exit0.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input63-final/dream-read-c-preparation/compile-9b5gmpk0/.
- proof-helper-probe.c:346237B, SHA25654fcf3f3d6acd9bb24cdcfbf4bcb151f4c05a0108bf6f7f5db5db66166d80572.
- probe:268024B, SHA256c483e7ee0ce682b131df910c31f45036bb0bdbf6cb7d442140304b403ccf9780.
- preparation-888182901a213dfb915f1d9e14ba044da319c8aabf542e800c2830e4357e03c0.json:534948B; hash matches name.
- INPUT63-final-tuple-v2.json:18293B, SHA2560200af25c72d19765232582e6d84a32f4e52f47534a00d75ac624d0dda176de2. Independent audit detail:472801B, SHA2563282144dea91e8fc73daea427bcbb993cfbbfbcd1cad56807ec4522c956768c2.

All ten inputs match signed git-show and retained snapshot bytes. Only C and src/dream/resolve.rs changed; eight inputs remain identical to INPUT62. The Rust delta fromb1df2c4c is the one authorized #[ignore] line. Schema/vector blobs match b26d7cd3 by blob identity. Result, terminal and exit-tail format lines remain byte-identical to INPUT62.

Complete immutable offline ingestion exited0, including source/executable/receipt, compiler/linker/SDK/header/build-closure/Mach-O/retained-output/current-platform/custody checks. The source-domain proposal SHA is d13d2e02152f4ec951487cb064a8ff8ee6266e207cb2b67f8cc9fdc0a17af65e. These remain conditional source/compiler/platform premises; no whole-native-lifetime theorem or compiled-product-currentness claim is added.

## Actual checks and original invocation

E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input63-20261108.

`python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target` — exit0,66tests. E/unit.log:11357B, SHA256130eaac67c9b9ed4f385d2e35038930de63129cfb10b826ed259368489310a19. Current selected-input ingestion and canonical source-lock equality also checked0.

Before build, /bin/df -k workspace exited0 with17,904,104KiB available, above the strict8,388,608KiB gate; E/disk-gate.json retains the observation. This seat performed no cleanup. All seven fresh source authenticators have auth_exit0 and all60 numbered configure/build/link commands000–059 exited0.

`make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses` — launched ONCE, still running with actual exit UNKNOWN. Original exec session61032 remains open because wait_component preserves the original corrupt controller handle until actual wait4. E/verify-exit.json is absent. The log records canonical private component positive actual_exit=0 check_exit=0, then OWNED PENDING for corrupt controller53612; that pending marker is not a timeout kill or exit verdict.

Build/run ID01M6XPVSR73HYNHFAB3ECZSZX8. R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6XPVSR73HYNHFAB3ECZSZX8. P=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6XPVSR73HYNHFAB3ECZSZX8. C+=R/components/positive; C−=R/components/corrupt_proof. Build incarnation1614891229937666; original basis is47112B, SHA256bf7a25b6c73b2cea31eab01dc93b247e1f0abae735e574f7e38fd4b2017bca6a. Recipe SHA256258a13eedfb1a0ad04665133513067271a858d5826180e0fce18afa47e879205.

## Positive proof, actual reap and requested signature telemetry

C+/wait4.json records actual_wait4true, controllerPID51787, exit0/status0, user_cpu_ns125183999, system_cpu_ns547519000, max_rss_bytes4276224. C+/checked.json independently records check_exit0/actual_exit0. The raw result is completed/settled, proofvalid, errornull, created16/reaped16/group-gonetrue/retired-coveragetrue, members_max2. Final terminal is settled, control/persistence/terminal-cost passed, all retained units paid, error_count0 and no deadline observation. Runtime approval remains false.

C+ ledger91 is the signature leader sequence16/PID53506 snapshot:

```json
{"kind":"verifier_event_discovery","leader_fork_events":1,"leader_exec_events":0,"child_discovered_via":"fork_event","fork_seen_child_missed":false,"notification_counts_not_birth_counts":true,"registered_mask":3,"registration_errno":[0,0],"wait_errno":0,"queue_fd_held":0,"close_attempted":1,"close_errno":0,"runtime_approved":false}
```

C+ ledger89 records verifier53509, child_exit_witness=kqueue_note_exit, child_created1/image_admitted1, terminal_observed_ns1615064766147000, child_exit_abstime0, rollup_mask31, rollup_proven1 and child_cost_unproven0. The zero RI exit timestamp is preserved; the typed NOTE_EXIT witness supplies the new premise. GS1 and GS2 in stderr bind the same signature inode186871736/hashd716c2d0807ca1339c774486d55d31768fa9f953df90872f2ceef4cb4568c792, with nlink1→0. Event rows are per-owner snapshots and were never summed.

The positive's final debt split is exact and charged=paid:

| Component | Retained cost ns | 9× debt ns |
| --- | ---: | ---: |
| Helper |152899000|1376091000|
| Ordinary raw CPU |514889667|4634007003|
| Ordinary floor surcharge |2094847542|18853627878|
| Total |2762636209|24863725881|

Ordinary total cost2609737209ns; helper+ordinary total×9=24.863725881s. ordinary_raw_complete=true, debt_components_match=true. The earlier raw-result debt24.809725881s has its own pre-emission cutoff and remains unchanged. Final process_create_wall_excess_ns0/count0/known1, scopeinvocation_sum_once_per_prepared_spawn.

Positive reporting began29.144471s after original_entry, within the admitted29.834517s entry-to-deadline interval, leaving690.046ms at reporting start. Actual exit0 and checked0 establish that this one full positive passed the original deadline gate. That is per-attempt evidence, not a general timing guarantee or product-runtime approval.

Source/basis-derived G=43, NOT MEASURED:26candidate+17fixture guards. E/guard-derivation.json enumerates unique(root,path) admissions, collapses equal duplicates and excludes absolute compiler/SDK review references. No runtime counter was introduced.

## Corrupt-proof refusal and retained live custody

C− persisted raw result is quarantined/proofnotreached, resource_limit ordinary engine unit exceeds one millisecond, created16/reaped15/group-gonefalse/retired-coveragefalse. Its controller53612 remains live under parent5130; ps exit0 and C−/owned-pending.json preserve those observations. No C−/wait4.json, checked.json or final terminal exists.

C− ledger87, ordinary_failed_charge, records one failed unit: first/last site engine_checkpoint, C:2687; open482403500→end483424458, actual/cost1020958ns, supplied cost1020166ns. It exceeds the hard1ms ceiling by20958ns. The frozen charge wrapper includes the open→end span (C:2646–2654), then refuses ordinary actual/cost above1ms (:2672–2674). The ledger does not identify the checkpoint caller; a post-GS1 checkpoint is only a source-consistent inference, not an observed call-site claim.

C− signature leader sequence16/PID55312 still records actual own leader wait4/group-gone, but phasequarantined. Ledger90 records verifier55315, child_exit_witnessnone, rollup_mask0/proven0 and child_cost_unproven1. Its signature event snapshot records fork1/exec0/viafork_event/missedfalse, mask3, errno0, queueheld1/closeattempt0. GS1 bound the corrupt228B signature, SHA256cd9777c930d18c7b5b3603ad58a6f63bd64fca702d87bc4e518fca5c1cf0a17b. This is not the required clean expected-rejection verdict.

The kill path marks an unproven live child sticky at C:2914–2915; settlement at:3499 and quarantine readiness at:3563 require covered child cost. Independent read-only audit confirmed that unchanged frozen hold cannot clear this predicate. No live-Q2 source, immutable probe or cap was changed.

C− raw paid debt23.530964994s is only its persisted pre-result snapshot. Its final helper/raw/floor split is UNAVAILABLE because quarantine prevents the final terminal. Retained unique helper rows show process_create excess0ns/count0/all-known1; no repeated global watermark or event snapshot was summed. No corrupt-proof exit or full-control deadline pass is claimed.

## Raw evidence and acceptance accounting

| Artifact | Bytes | SHA256 |
| --- | ---: | --- |
| C+/stdout = C+/output/result.json |1939each|cf1907c0d043e8697c54d453cabd3cffbf249f3e283657c7261aa2a3de8f0694|
| C+/stderr |2741|b85d6b47a200c605d624a290325d90a5b5c093db06b5d7fcc1897d1fb60eb1cb|
| C+/output/ledger.ndjson |50562|4b3e368ba8b576bd7152f56ce12b524324eaf0f68323855fabc624b707ec9342|
| C+/wait4.json |137|a2ffe0207e9505f0a3b97e12c36b802ac2ddadf0be59af7c59b1b0c355fb31b3|
| C+/checked.json |79|6947e772955554ef8a7b886c5283fdeb187a18eeaf9ee526bbf997a179f9f0c4|
| C−/stdout = C−/output/result.json |2037each|710e4d73263c0e3a920e8dfda89428ad2fe57485f1301f13d84c463afad985bb|
| C−/stderr |256|45c9f50451d03efb143255ab96f881bcc7446fe200af0a860fb4a81aa79afd25|
| C−/output/ledger.ndjson |50918|131ccb59fd58e1ae7c4b48b50bc7a9917aa771f89c255d9aea22f51619246feb|

- Exact freeze/source authenticity/custody: passed offline; E/preflight.json, source-byte-checks.json, resolve-ignore-change.json, selection.json, ingestion.json, tuple-binding.json and terminal-binding.json retain commands, hashes and scope.
- Pin-only adapter/current tests:66units exit0; E/scope-audit.json establishes unchanged checkers/budgets and source-lock equality. No conditional ledger schema extension was needed.
- Real source authentication/build:7fresh auth exits0 and60numbered commands0. The real positive now passes both producer and consumer checks with actual reap0,16/16settlement and requested NOTE_EXIT witness.
- Positive→remaining components→final gates→qualification→receipt: blocked in corrupt_proof's original live quarantine. pause_resume,cancel,stale_incarnation,clock_failure,debt_overlap,reap_delay did not run. Final formal/license gates and command-tail tests did not run. Qualification and receipt files are absent. No landing, merge, runtime admission or task-green claim.
- All13historical ledgers retain original dev/inode/length/SHA256 in E/preserved-ledgers.json. No resets, stash, old source/prefix/ledger removal or peer cleanup occurred. E/pending-state.json records signed HEAD, diff/status0/empty, exact two current cases and missing canonical exit/qualification/receipt.

E/joined-telemetry.json preserves both original result cutoffs, all unique-owner helper history, complete final positive terminal, signature event/child rows, failed ordinary record and actual/pending exits. E/pending-handoff.json is138959B, SHA25659d65db741756f5bddadc184fd2b02782c970a33bacdae5c5887826a867c3918. It binds every numbered build exit and authenticated source plus raw artifacts. It is offline provenance, not a runtime input-budget assertion. Positive succeeds; overall qualification remains BLOCKED with canonical exit unknown.