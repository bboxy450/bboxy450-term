---
type: report
title: INPUT68 positive audit-token refusal; original owner settled
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T07:57:33.014Z
commits:
- 6e9031570f9f1c990a8a76284497b41924f3f27d
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M6ZMZYQBBJ9PXE3VQ5MX01FC
updated: 2026-11-09T07:57:33.014Z
number: 93377961366443
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:138e60e65f0cb220c33da115353c93de541aff34f2d291f04051ecb7d6d9545f
  frontmatter_digest: sha256:857b22306f94549ba5bc4414eef698e4938825910535c69b09878ee8507f332c
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: ICvpZdOykk1Yn6pjy9zDy776+OZBdya/XzIA2X99zfQPrK8j1Rvgi0LFXgl3D+ieYfh0Gg8tInDDZ1DRfzXaAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150961
  event:
    id: 01M6ZMZZHP0GCN2PDTG7BT2BG7
    sequence: 150961
    hash: f2f1ed54fb7d34665d67c36c289317672906a7ccb29df2481678aacccf4254d8
---
INPUT68 is FAILED: the sole canonical invocation exited 2 after the positive controller's actual wait4 exit 81. Positive failed before proof verification; all seven later cases, including pause_resume, were not reached. The original raw quarantine became physically settled 10/10 before the final terminal and actual parent reap. No retry, signal, teardown, qualification, receipt or landing occurred. runtime_approved=false.

Decision needed: Oracle/Q2 disposition of the audit-token-refresh refusal and subsequent final-group-identity refusal, followed by corrected authorized input or a scoped repair before another native attempt. This is not a request to waive the positive gate. INPUT66/67 proof passes and all historical failures retain their recorded dispositions.

## Scope, pins and signed change

Task [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], ORACLE FREEZE, INPUT68. Executor seat:01M6W6K5SWZB5AEBAAAA73FF4M. W=/Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools.

Signed code commit 6e9031570f9f1c990a8a76284497b41924f3f27d changes only scripts/dream-proof-tools.py (immutable ROOT_Q2_INPUT selection) and tests/fixtures/dream-proof-tools/sources.lock.json (canonical source pin). git verify-commit HEAD exited 0 with Good Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0; git diff --check exited 0 and git status --porcelain=v1 exited 0 with empty output. Commit carries Report: [[reports/2026-11-08-dream-proof-tools-input68.md]].

No parser, budget, source guard, ownership/lifecycle, model, raw result, test dispatch or runtime-approval change was made. The prior exact clock_failure auxiliary-detail and narrowly typed reap_delay earlier-wire/later-settlement repairs remain unchanged; the Oracle accepted that repair after INPUT67.

Exact consumed tuple:
- Source a68d200268ca816a6741e00c6cd334b28628f805. Independent git verify-commit Good Boxy; 645fec78 and 2ce4359 ancestry and containment on origin/main each exited 0. Observed origin/main exactly a68d2002.
- D=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-pause-stop-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input68/dream-read-c-preparation/compile-p9dauq95/.
- D/proof-helper-probe.c: 397100 bytes, SHA256 3c18a373246830e2efce125df292d153b5cb1c2df9458c027dbd02da9e51cec2.
- D/probe: 285784 bytes, SHA256 87ce8d48283f9a3f7eab41cbf8e073694d3b76ba2066a6223f94c18393d47131.
- D/preparation-7f736d1abc0b493aa0ec12adc443594e5bd9e1ecb65df306101b3dfa1321eb05.json: 532156 bytes; hash matches name.
- All ten inputs match actual signed-source blobs and retained snapshots. C and resolve.rs changed from INPUT67; eight inputs unchanged. Proposal source SHA256 2e4f932481a74490e4fd7b83c4ee79e57f200332e87ae4ce49cb335824241e74.
- Schema blob 96e2839a9d2b6031fb0138934eef96850e9b7a60 and vectors blob 54cfab3b0e6cc8996bbc942cc2b0128278fccfaf are byte-identical at b26d7cd3 and INPUT68; this is blob identity, not b26 ancestry.
- Final INPUT68-source-freeze-tuple-v1.json: 35542 bytes, SHA256 e0f3ba9d8120af99f8e7dfdd8470ccff4414732f855a5ddfb7ef21353bdd3008. Independent retention audit SHA256 dfc4a023f03f926b28957f6906c0773d5cc8b9988acacc9129c1c7ca9c6459d9.
- Terminal, paid-tail and result emission lines are byte-identical to INPUT67. The final tuple and actual frozen C control identity; an earlier referenced pause-stop-exit-probe-freeze-v1.json describes historical bytes/line numbers.

Production raw ledger retention does not decode kinds/fields, so the 15 authorized pause-row additions need no production whitelist expansion. Forensic extraction checked the seven u32 flags, five u64 timestamps, two i32 results/errno and exact six-value order enum against the frozen tuple/source. Terminal unknown/missing/duplicate/type/bounds gates remain unchanged. New C source :3659–3683 and :3573–3591,3939–3955,4111–4117 constrain terminal-before-STOP handling to original custody and independent terminal/wait4/group/cost evidence, without fabricated ACK/resume. This path was not exercised by the unreached pause_resume case.

## Commands, private candidate and evidence

E=W/target/dream-proof-tools/input68-20261108.
R=W/target/dream-proof-tools/runs/01M6ZMJP97QRM0YS85A806NJZW.
P=W/target/dream-proof-tools/candidates/01M6ZMJP97QRM0YS85A806NJZW.
C=D/proof-helper-probe.c.

Offline full input ingestion exited 0. Unit command "python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target" exited 0, 68 tests; E/unit.log SHA256 b9a886f068a1eadb5f3e1cf3c152a9693bd3d0f125bfd85aa5a6b534d332509e. This includes strict terminal and both previously approved compatibility repairs' negative controls. No additional native qualification was launched by unit tests.

Disk gate immediately before launch: df exit 0, 34039372 KiB available >8388608 KiB. No prior run/source/prefix/formal-cache cleanup occurred. The only removed files were this seat's newly generated Python bytecode and its otherwise-empty scripts/__pycache__ directory; E/own-bytecode-cleanup.json records exact identity/hash. The commit wrapper exited 1 after its single successful commit because its clean-status assertion saw that self-created cache. Commit was not repeated; subsequent actual signature/diff/status checks all exited 0. Evidence-only inspection scripts also corrected harmless metadata-shape assumptions; they launched no candidate.

Sole canonical command:
    make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses

Actual exit 2, start/end 1794210614877579000/1794210781571402000 ns. E/verify.log: 9665 bytes SHA256 1df8726cebb5b6238e11f204cd34377623b124917205ef628772fb513f2bc422. Seven source authentication rows each auth_exit=0; numbered build commands 000–059 each exit 0. Build stopped on the first component refusal. The shell's && prevented all later final targets.

Original incarnation 1679604257206333. Prefix device/inode 16777234/189155930, mode0700, retained without replacement. Basis: 47112 bytes SHA256 6aec6de44fabf6629f68d2bb5de8cbafde05331d3fe159627977f9042104f0ec. Source lock SHA256 9651a7a29ed43c87aefe157f93170620f07cfc08c76168f45d1211d5333825ef; git SHA256 08eda8c4a8844644e8826181cc04dcbe42d7e2a4414fadb9155512ddc54edbdc; gpg SHA256 fc756626906313e3ec18cd59741b28c3820053a074656a13b0769eb0f5d43e6e.

Derived G=43 (26 candidate +17 fixture) from consistent deduplicated guarded descriptor/basis references by (root,path). This is derived, not a runtime measurement. Conditional source/compiler/SDK/platform/reachability assumptions remain explicit; no native-lifetime theorem, approved candidate or available lesson exists.

## Outcomes and requested metrics

| Case | Outcome / actual controller exit | first_failing_term | Final debt s | Reporting-start margin s |
| --- | --- | --- | ---: | ---: |
| positive | FAILED / PID5606 actual wait4 81 | not emitted; early coverage refusal | 10.133055 | 18.377769 |
| corrupt_proof | not run | not emitted | unavailable | unavailable |
| pause_resume | not run | not emitted | unavailable | unavailable |
| cancel | not run | not emitted | unavailable | unavailable |
| stale_incarnation | not run | not emitted | unavailable | unavailable |
| clock_failure | not run | not emitted | unavailable | unavailable |
| debt_overlap | not run | not emitted | unavailable | unavailable |
| reap_delay | not run | not emitted | unavailable | unavailable |

Positive raw result: phase/outcome quarantined, proof not_reached, attempts10, created10/reaped9, group_gone=false, retired_coverage=false; first error coverage_unknown "current helper audit identity unavailable". No checked.json or final_case_conjunction row exists. Final terminal instead records stage=settled, helper_created=helper_reaped=10, control_verdict=failed, persistence_verdict=persisted, terminal_cost_verdict=passed. Physical settlement does not turn the positive into a pass.

Positive final-terminal accounting:
- Helper cost 105895000 ns.
- Ordinary raw CPU 154044458 ns + floor surcharge 865955542 ns = ordinary cost 1020000000 ns.
- Helper debt 953055000 ns + ordinary raw debt 1386400122 ns + floor debt 7793599878 ns = charged=paid 10133055000 ns, exactly nine times the total costs.
- Raw result debt 9878067000 ns precedes the remaining helper settlement and diagnostic tail. Final totals supersede that cutoff for final accounting; repeated cumulative owner rows are never summed.
- Raw-result ordinary_max_ns=483875; final-terminal ordinary_max_ns=458166. Neither exceeded 1 ms. Process-create excess_ns/count=0/0, known=1. No ordinary_failed_charge or deadline was recorded.
- Admitted interval 29813229000 ns. Entry→reporting start 11435460000 ns, leaving 18377769000 ns.
- Reporting started before the late original ownwait4. Ledger:63 timestamps that wait4 at entry+11488718000 ns, with 18324511000 ns remaining. Neither cutoff is final receipt/parent-exit wall time.

Requested pause_resume errno/order/resolved and stopped_interval_ns/running_gap_max_ns/stopped_system_ns are unavailable because that case never ran. In this positive case's eleven pause diagnostics, STOP was not attempted, errno/result=0, order=terminal_not_observed and terminal_resolved=0; those defaults are not evidence of a successful pause or of the new terminal-before-STOP behavior.

## Failure chain and actual settlement

R/components/positive/stderr:1–2 record taskall failures for PID6305 and PID6458, result0/errno3 (ESRCH). They had different outcomes:
- PID6305 is sequence5. Ledger:26 records reaped/paid, actual_own_wait4=group_gone=1, and execution continued through sequence9. C:3046–3054,3068–3071,3106–3109 support terminal recovery after an opening/closing taskall ESRCH. The diagnostic does not distinguish which read.
- PID6458 is sequence10. The retained coverage detail uniquely selects C:3076, after known_control_token returned false. That helper performs additional control_observe reads at C:161,166, alongside task-name/audit-token checks; this caller has no terminal-after-error recovery. The exact failing token stage is not serialized.
- Retirement then hit C:3180's live BSD identity predicate, followed by C:4099/C:4078 quarantine. The error histogram retains coverage_unknown count1 and settlement_unknown count2: "live final group kill lacks original current group identity" and "final kill raw error unresolved".
- That BSD predicate's failed field and errno are not serialized. C:3185 sets group_kill_attempted only after the predicate; ledger:56 and :62 both have group_kill_attempted=0. The text "final kill raw error unresolved" does not establish a failed SIGKILL syscall.

Ledger:56 records the original sequence10/PID6458/birth/start quarantined, unreaped/unpaid with queue held. Ledger:62 records the same owner settled_refusal, actual_own_wait4=1, group_gone=1, helper cost10332000 ns retained and paid; :67 records queue closed. C:4130–4149 retains and settles that same original owner. Ten distinct final owner records all have actual ownwait4/group gone; the eleven helper rows include two snapshots of owner10. Sticky quarantined=1 preserves the earlier refusal, not a remaining live process.

R/components/positive/wait4.json:1 confirms the controller's actual wait4 exit81/status20736, USER68337000 ns, SYSTEM195126000 ns, max RSS4177920 bytes. No live owner remains, and no teardown or signal is needed. No verifier child/GS1/C45 witness was reached.

## Integrity, acceptance state and handoff

Unaltered stdout equals output/result.json byte-for-byte. Exact SHA256:
- Raw result/stdout: feb1680e240acf1307df3dc8aebdb5333b448c867db14fcccae108d36387d726.
- Ledger: e99a6878654f62f236f1d150a4089f13792bdbeeb55a81e32c76588cdbc78ead.
- Actual wait4: cd1fe3708fc5624116d86f72fc491f1498cbc4e7c7dc30905336227894a579e9.
- Stderr: 99a16ff001d79a76ab1c9a78b7c8506631f292905b6bef1e28a9e321a6390d9d.
- E/telemetry.json: 72513 bytes SHA256 96e51b93e8b1e2ebb0e27eea7b488fd67037cb07c7794434182c72d5bc0acb3e.
- E/handoff.json: 24196 bytes SHA256 76f0be1df5a91de8af79128a99197f6eb776ba6d662dde208392e1edd2bfd421.
- E/failure-analysis.json: 6417 bytes SHA256 fd4919a6e2c7d2367280cda4137df9c7d91546d55cfa2d3600d4d56a39b41641.

E/handoff.json includes real auth/build/unit/canonical exits, code checks, raw artifact pins, original prefix identity and preservation checks for all 23 historical ledgers (length/SHA256/device/inode unchanged). E/failure-analysis.json binds source excerpts and distinguishes recorded errno, unknown stages and the absence of any group-kill attempt. New evidence data is mode0600. No previous ledger/source/prefix or report was modified.

Acceptance state: exact signed input, source authentication, private build and existing unit controls passed; actual positive qualification failed. Current-source test-dream-proof-tools, formal/TLC/Yarn parity and license final gates were not reached. Qualification/receipt-last publication and make land were not reached. Tests/models/source review from earlier checkpoints remain historical, not current green evidence. Original ownership, strict errors and all budgets remain enforced. No full-product/four-job/exporter/runtime approval is claimed.

This report records the failed INPUT68 attempt and requests the source-owner disposition above; it neither retries nor supersedes the independently passed INPUT66/67 proof cases.
