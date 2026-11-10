---
type: report
title: Q2 child knote exit witness landed with qualified leader rollup
task: '[[tasks/2026-11-08-dream-q2-child-exit-kqueue-witness]]'
author: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
created: 2026-11-08T13:49:15.339Z
commits:
- b1df2c4cb2aec0af7ace7584920e71f1017a9440
- 390b5836fb3693e3c4861a98def1ef8b53d54d5b
verify:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
id: 01M6XPQ7C6Z40Y9XSVQKKGB1XC
updated: 2026-11-08T13:49:15.339Z
number: 93377961366369
created_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
updated_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  kind: seat
  public_key: 3lIiKAyaYQbqAYQ3xsrS+NTGLp9s//BjqCA4HkK5nF0=
  process_generation: '1'
provenance:
  digest: sha256:8fec28a68442d3bd1c625fed5657b18f4eb16205ce9bc67f4e01bb41cfcafca1
  frontmatter_digest: sha256:974690f370c1d59ced98e0325a267d84753fe11895e56ec0af31523f9e94ca45
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  sig: soY9pUUkLEpjur6M/paSd/3msuhAyXOThRTqkZ7cC6PoVOCWY7oJ5VJdoRmRJgO4/JyOVfiWF2vRZFUqV7A8DQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150689
  event:
    id: 01M6XPQ89ANSWM3ESHKZXBX4AH
    sequence: 150689
    hash: 9127a23e97524e211afec168c8224759b3dab76ca7fc387b31bb9f2bda428c9b
---
Implemented and landed Task 66364 at `390b5836fb3693e3c4861a98def1ef8b53d54d5b`; `origin/main` contains it. Both code commits are signed by Boxy with fingerprint `9B07E849480C68328CA20684844ECC0587DE7AB0`: implementation `b1df2c4cb2aec0af7ace7584920e71f1017a9440` and the Oracle-authorized single QoS attribute `390b5836fb3693e3c4861a98def1ef8b53d54d5b`. Worktree is clean. Prior accepted tasks, inputs, reports, green archives and failed runs remain intact.

Evidence root E: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3`.

Changed files: the C probe, private controls and canonical control driver under `tests/fixtures/dream-read/`; `tests/formal/dream_read.rs`, `tests/formal/models.rs` and the three verification TOMLs; plus only the authorized ignore attribute in `src/dream/resolve.rs`. The code commits retain the Report trailer.

## Mechanism and acceptance evidence

`proof-helper-probe.c:2502` preserves the existing rollup-cover predicate. Its child terminal path accepts a process-bound retained child NOTE_EXIT only after RI ESRCH, exact current watch/token/approved-exec custody, a known first monotonic event receipt and tokenGone. `exit_abstime` remains zero for this source. Missing, foreign, EXEC-only, nonESRCH, unknown, stale and undercovered evidence still refuses; prior sticky child-cost unknown never clears.

The delayed-receipt correction forces an actual fresh original-leader RI read under the original unreaped/current-token pin and positive own-WNOWAIT, bypassing only the terminal cache. The first terminal timestamp remains unchanged; a cached success cannot satisfy bit16. No child bill is added to the authoritative original leader's own wait4 subtree bill.

The sole new ledger field is `retained_child_accounting.child_exit_witness`, a JSON string enum `ri | kqueue_note_exit | none` (`proof-helper-probe.c:3111`). Terminal fields are unchanged. Watch/errno/counter/timestamp proof and budgets remain source-bound to [C27-source-freeze.json](/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/C27-source-freeze.json) (SHA `e126d28585f7ed2e765900ccb5a7ea307758bafdec03140ad79693cbde73735b`).

| Acceptance | Actual evidence |
|---|---|
| Leader reaps watched child; qualified exit despite RI ESRCH | Canonical real positive leader 71029 / child 71032: RI errno 3, exit_abstime 0, witness kqueue_note_exit, mask 31; original leader performs child wait4, controller does not. |
| Delayed event after cached leader RI requires fresh observation | Actual T1=1610744117799000 < child receipt T2=1610744129012000 < genuine fresh RI T3=1610744129266000; first leader terminal timestamp stays T1. |
| Full bill, no duplicate child charge, paid physical settlement | Authoritative subtree CPU 9,158,000 ns; raw child CPU 3,586,000 ns is not added again; helper debt 82,422,000 ns=9×subtreeCPU; original ownwait4/groupgone and paid queue close checked. Global prior 90 ms / target 172.422 ms / paid 190.422 ms distinguish helper debt from ending ordinary costs. |
| Ignore/missing/unbound event and inadequate rollup refuse | Production-API supplied-state negatives: EXEC-only, failed registration, foreign birth, forged udata, nonESRCH, undercoverage, prior sticky unknown and token-present; intended 81. Ignore-note/unbound-note/cached-refresh causal mutants each reach their named 81 assertion. These API negatives make no native-process/physical-settlement claim. Existing physical registration-failure control remains. |
| Finite model and broken controls | New disjoint 16-profile child-exit companion plus IgnoreChildExit/UnboundChildExit controls; previous discovery graphs and other 434 model implementations unchanged. Exact focused child-exit and discovery tests each 1 passed. Current full native/pinned-Java graphs, labelled edges and named verdicts agree for all 442 models. |

Canonical C receipt /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical/private-engine-controls-pos5dj1m/run.json SHA `cd5bdd89f53658cfbbc38efc170a38aeee8124fefa07b0754ac66dd60fb33115`: actual 0, 98 baselines/57 mutants/59 strict compiles/143 commands; C3 bytes are identical on both signed commits. Source-known real process/knote/rollup evidence uses an explicit fixture image-admission premise, not installed Git/GPG qualification. Details: [current-C-suite-summary-v1.json](/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/current-C-suite-summary-v1.json) SHA `dbe8c9a9efbd87db8c08f9680af96f2ee244063ac286f1133d8ed738bb4734d9`. C26's earlier narrow 0 is retained as historical; its cached-RI defect was corrected before final C27 gates.

## Commands, source cuts and one authorized landing rerun

The literal task command was run once, with no knobs or gate waivers:

`make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build`

Actual exit 0 on implementation `b1df2c4cb2aec0af7ace7584920e71f1017a9440`, source unchanged, 2026-11-08T12:31:31Z–13:06:11Z. Log [childexit-exact-four-chain-v1.log](/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/childexit-exact-four-chain-v1.log) SHA `20cc0b626552249dd64b48c70121b51f0dda5719c9bf25ed088d7c4798f52dae`. Strict all-target/fixture checks and the two exact focused model tests also exited 0; their source-bound rows are retained in E. Environment removed FORMAL_JOBS, used jobs 2 and `/usr/bin/nice` rustc wrapper, empty workspace wrapper and disabled incremental builds. Canonical workflows and model/tool limits were unchanged.

The first canonical `make land` at `b1df2c4cb2aec0af7ace7584920e71f1017a9440` exited 2; log SHA `24d10996323009fd098f8265c5087f16f45ab9ab7707c15895bb5c231c6c0c21`. The stale-source arm of `prepared_qualification_absent_and_forged_claims_refuse_before_native_birth` hit one complete ordinary epoch 1,032,458 ns > 1 ms before any helper/reservation/effect. Source inspection found no demonstrated duplicate accounting; separate stage maxima were not summed. Failure remains RED in [land-ordinary-budget-blocker-v1.json](/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/land-ordinary-budget-blocker-v1.json) and [failed-land-b1df-canonical-receipt.json](/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/failed-land-b1df-canonical-receipt.json).

Oracle task amendment etag `aea2366e65aed9979eefc8a892b8df3f6a2deadfb60622073abebafc15782b6a` grants exactly one ignore attribute on that test and one canonical make-land rerun. The exact reverse transformation recovers the whole old resolve.rs; test body/assertions and all product caps are unchanged. Proof [qos-one-attribute-source-freeze-v1.json](/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/qos-one-attribute-source-freeze-v1.json) SHA `cf356b1890d6d10360fc4e15a08d2ec098134c7767a4206355a68d1bf2992761`; old resolve SHA a362c6e7f8f77a998592e8d66a3c7b7f00fc52dbc401c3db592bd002ca344a36, new da9fc20f02608b7a0cf808d24ac08c599e236875ceab7ef153ec12fa7a7ae357.

That ONE rerun at final `390b5836fb3693e3c4861a98def1ef8b53d54d5b` actually exited 0: [childexit-canonical-land-rerun-v2.log](/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/childexit-canonical-land-rerun-v2.log) SHA `03bf6612e5d6fdcfb9d0cfcc41954fe60ce9a55476ac0adcd8d55843e6774dbf`; canonical receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical/harness-land-receipt.json` SHA `a7c41c3bfca21eeed13e372b399b2546c02efd9326cbc8b9b20b84b6bcbb6f68`. Strict/lint gates passed; nextest 193 passed / 10 skipped. The canonical validator detected the old source receipt, then genuinely reran full formal parity and validated fresh current evidence before signed fetch/push. The intermediate stale-source check was repaired by the existing canonical workflow, never relabelled. No second whole-four-chain shell exit 0 is claimed for the one-attribute head. This report preserves the actual implementation-head four-chain exit 0 and the amended final-head canonical-land exit 0 separately.


Narrow verification rows (all actual exit 0, unchanged relevant source): `childexit-model-exit-focused-v1` exact `models::dream_read::verifier_child_exit_tests::retained_child_knote_exit_covers_esrch_without_invented_cost_or_reap`, log SHA a260c3d42df95c8cc4c2bf5518ee64584808273e6768dc98301a64cdacd7bf76; `childexit-model-discovery-focused-v1` exact `models::dream_read::verifier_event_discovery_tests::queued_fork_discovery_preserves_custody_diagnostics_and_all_causal_controls`, log SHA 0e67b01df3fc647e6f9573527e0c39a5ef2484fe4a4c197c176fb01c4ed46afa. Strict `childexit-strict-all-targets-v1` log SHA e70c24f8b9ec813ce95cd254a99a88e3d487a9853ce16b578d0e191085804fe0 and `childexit-strict-fixtures-v1` log SHA aba3386a0a85650032ed6e21f344dee6cb5160899eb3271b0dfa753bc661be52 remain source-bound pre-attribute checks; final canonical land ran both strict checks again on the final head.

## Current formal archive and actual profiles

Final `390b5836fb3693e3c4861a98def1ef8b53d54d5b` formal archive `/Users/feral/mydev/bboxy450-harness-dream-read-q2/verification/evidence/915b39a044e1f131b959fbfff07b4b60d08f3cb32a18e0162720c9b64d108e24`, evidence SHA `915b39a044e1f131b959fbfff07b4b60d08f3cb32a18e0162720c9b64d108e24`: 442 models / 94 contracts / 1 crate, 3,378,027 states/11,366,941 labelled edges, all native/reference graphs and named verdicts equal. Pinned TLC/Java, cap 100000, heap/workers/timeouts and fairness unchanged; finite corpus parity is not complete TLC replacement. Full raw-file SHA/inode/mode journal /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/current-land-formal-all-raw-files-v2.json SHA `851a4b5fd757541df8e88e5990d027b3d3d4d45ed27922ec5f3ebbfc0c0b8d17`. Archive sealed 0400 files / 0500 directories after byte audit; working graphs are separate writable inodes. Prior b1df archive a6f67f7761715482e04c91b8f681039eaa0595131e75b99e68d4170a89037adb remains immutable and separately source-bound.

Actual C1/READ profile metadata and independently copied readonly worker from the successful implementation-head command were preserved before later gates: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/retained-b1df2c4c-actual-profiles-exact-four-chain/preservation.json SHA `e85a8a4ff2ff04c83018191bfc40ca2ea157b0915ca3bfa7db510eacd6a97fa4`, worker SHA 389c7cdbcb4a12d79bfb9d4716fc34ad48067360652482d3f344fb8e5f5767b9. Those are explicitly `b1df2c4cb2aec0af7ace7584920e71f1017a9440` evidence, not relabelled as `390b5836fb3693e3c4861a98def1ef8b53d54d5b` runtime profiles. Five observer JSONs were preserved using the canonical helper after genuine MCP exit 0; [observer-preservation-detail-v1.json](/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/observer-preservation-detail-v1.json). No new product runtime approval follows from profile or model evidence.

Only owned regenerable cold working graph files were transparently compressed after lsof/SHA/stat inventory, preserving paths/bytes/SHA/modes/mtime and journaling inode/ctime/storageflags. Current and prior immutable archives/inputs/profiles were not deleted or compressed. Pre-rerun compression 361 files log SHA 120e6ec739c4329f27edd94b35ff2e8257ab8cf55b517da70f9521b49d7b3edc; other own-cold journals remain in E. No failed formal exploration occurred in this task.

## INPUT63 and limits

ONE final tuple [INPUT63-final-tuple-v2.json](/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/INPUT63-final-tuple-v2.json) SHA `0200af25c72d19765232582e6d84a32f4e52f47534a00d75ac624d0dda176de2` binds final `390b5836fb3693e3c4861a98def1ef8b53d54d5b`, all 10 signed source/preparation inputs, current resolve/trust/c1/read domain `d13d2e02152f4ec951487cb064a8ff8ee6266e207cb2b67f8cc9fdc0a17af65e`, exact immutable executable SHA `c483e7ee0ce682b131df910c31f45036bb0bdbf6cb7d442140304b403ccf9780`, compiler/linker/SDK/OS and complete lexical/resolved header closure. Preparation receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input63-final/dream-read-c-preparation/compile-9b5gmpk0/preparation-888182901a213dfb915f1d9e14ba044da319c8aabf542e800c2830e4357e03c0.json` SHA `888182901a213dfb915f1d9e14ba044da319c8aabf542e800c2830e4357e03c0`, actual compile 0; independent audit `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/input63-final-independent-audit-detail-v2.json` SHA `3282144dea91e8fc73daea427bcbb993cfbbfbcd1cad56807ec4522c956768c2`, exit 0 with 325 actual headers / 997 checks. Retained roots 0700, files 0400, executable 0500, nlink1 and signed schema authority were checked. Earlier b1df preparation exit 0 was never sent as a frozen INPUT63 and is preserved separately, not promoted to the changed readset. Native tuple handoff result is preserved in E/input63-native-handoff-result-v2.json; this is storage/delivery evidence, not builder qualification or acceptance.

No genuine builder/candidate invocation occurred. `runtime_approved=false`, compiled-product currentness/installed-tool qualification and whole-private-candidate 30 s fit remain unproven; no event proves GS1, membership, CPU or a reap by itself. Prior task's source-derived floor estimate and accepted-input limitations remain intact.

Amendment total-count shorthand is 26→27 ignored tests, all nonpasses. Exact source inventory is 21 QoS (20 prior + this 1), 5 separate proof-tool qualification deferrals and 1 pre-existing manual release benchmark; the manual benchmark is not reclassified as QoS. Inventory [final-report-deferral-inventory-v2.json](/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/final-report-deferral-inventory-v2.json) SHA `e07491b822a58343572f790804d775cd3d92a2fb89ba00b33a6988aa2de2d040` lists every exact name/reason/source SHA. No ignore body/assertion was weakened. Historical failures and retired-law physical-negative unproven records remain historical, not current passes.
