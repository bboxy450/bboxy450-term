---
type: report
title: INPUT69 positive passes; corrupt proof remains owned in quarantine
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T12:14:24.520Z
commits:
- eb42d39aafb86d9de7ded7ec30d9bc00a13297cf
verify:
  command: python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target
  exit_code: 0
id: 01M703P92MH2BVS1R4B1ZFRP4J
updated: 2026-11-09T12:14:24.520Z
number: 93377961366454
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:c366db3d5ea363a326f6e413c742bcf8ef36044063bba626e23336d10037997b
  frontmatter_digest: sha256:4fea796c6f32bed21d06e02ccfc597efa22d5819ceaacfa8fb073b8a4d959d4c
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: VoDij6ziWxhWU8LJiUO2WntQOOcen4LuDByNKH9xF1ourV6yQQsMGMklQprsQVD09OeeK1U4Eii7ip33zaVlAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151002
  event:
    id: 01M703P9VAS7XZD1A6YR5A4Z7X
    sequence: 151002
    hash: cd89bac5e5f42930dd17c2884690358d31e3a4729a48f06c8bb02e33cc3d7815
---
INPUT69 is BLOCKED/FAILED overall. Positive PASSED: actual controller wait4 exit0/check0, valid proof, full16/16 settlement and all28 final conjunction terms true/first_failing_term=none. The same invocation then reached corrupt_proof's retained coverage refusal and remains OWNED PENDING with controller90389 and its original wait parent43277. No actual corrupt or canonical exit has occurred. No retry or signal was sent; runtime_approved=false.

Decision needed: Oracle quarantine disposition for exactly controller90389/original wait parent43277, and Q2 disposition/corrected authorized input for the semantic resource-identity refusal and unrecoverable retained child-coverage hold. This asks for disposition of a failed frozen producer, not permission to waive a gate. The one-attempt scope and actual-reap law prevent a retry, substituted controller, fabricated exit or unilateral teardown.

## Scope, source and signed pin

Task [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], current ORACLE FREEZE, INPUT69. Executor seat:01M6W6K5SWZB5AEBAAAA73FF4M. W=/Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools.

Signed code commit eb42d39aafb86d9de7ded7ec30d9bc00a13297cf changes only the immutable ROOT_Q2_INPUT selection in scripts/dream-proof-tools.py and canonical C source pin in tests/fixtures/dream-proof-tools/sources.lock.json. git verify-commit HEAD exited0, Good Boxy key9B07E849480C68328CA20684844ECC0587DE7AB0; git diff --check exited0, git status --porcelain=v1 exited0 with empty output. Commit carries Report: [[reports/2026-11-09-dream-proof-tools-input69.md]].

No parser, whitelist, budget, source guard, ownership/lifecycle/model action, raw result, dispatch order or runtime approval changed. Existing clock_failure/reap_delay compatibility repairs remain unchanged. Raw ledger retention does not decode kinds, so the new observation_terminal record needs no production decoder expansion. Forensic extraction checks its exact three fields, fixed27 site enum and signed i32 raw code.

Frozen source637cb234d93d08d0c8fcfb820b7e48f9538bc0e1 was independently verified Good Boxy. a68d2002 and2ce4359 ancestry and containment on origin/main all exited0.
D=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-systematic-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input69/dream-read-c-preparation/compile-ewu_xvzb/.
C=D/proof-helper-probe.c.

Exact pins:
- C432619 bytes, SHA256 d1ec07415d4b8dcf953941f8d7f222677baf9a0d766c9fe33fb50869ea30f151.
- D/probe286472 bytes, SHA256 6c63469ff20f4a433c2fafb3a4aed7c91e037e46862da937fc316c9c140b07ba.
- D/preparation-987b8b56107a59230c0e353a7defbe1fef2ff50f7f3903a4c72a9fc290851a3a.json532156 bytes; hash matches name.
- Final tuple SHA2568368278817ac5fc74da15e61d0f8ae069d8b9767849ee8ef4cac0cecd86f8511; independent audit SHA256746fb29bb9e8613d0f6eb1a601fc4a184f9ad0999d57ffefeefd0666c2d28607.
- Ten inputs match signed git blobs and retained snapshots; only C changed from INPUT68. Proposal source SHA2562e4f932481a74490e4fd7b83c4ee79e57f200332e87ae4ce49cb335824241e74 is unchanged.
- Schema blob96e2839a9d2b6031fb0138934eef96850e9b7a60 and vectors blob54cfab3b0e6cc8996bbc942cc2b0128278fccfaf are identical at b26d7cd3 and INPUT69. No b26 ancestry claim.
- Terminal, paid-tail and result emission lines are byte-identical68→69. The result grammar and terminal whitelist are unchanged.

Independent static review found27 enum sites,27 switch cases and27 production routes with matching sets (C:101–130,232–261,3452–3478). Shared terminal takeover at3389–3425 still needs complete original identity, eligible site/domain/code and independently qualified terminal evidence; consuming ownwait4 at4381–4404 still needs fresh positive WNOWAIT. CPU/RSS, retained+paid debt, group/retired coverage and physical settlement remain independent gates. Static fault-route evidence explicitly supplies failures with target_syscall_invoked=false; it does not establish physical kernel-race causality, complete GS1/escaped-child execution or an arbitrary native lifetime theorem.

## Verification and original live custody

E=W/target/dream-proof-tools/input69-20261109.
R=W/target/dream-proof-tools/runs/01M7035811Q2HTZ37K75WQ5K3C.
P=W/target/dream-proof-tools/candidates/01M7035811Q2HTZ37K75WQ5K3C.

Offline complete input ingestion exited0. Unit command:
    python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target
Actual exit0,68 tests. E/unit.log11737 bytes SHA256769f6f024106b97287379e9be4b7cd2ddd8f731bd8dfd478b4a6568e833c72e3. Recipe SHA25635186b0e0b7c69d4cf8fcca665218c87ff7c85185e5558479a93677736b72e07. Report metadata verify refers ONLY to this completed unit command; it is not a claim that canonical qualification passed.

Sole canonical invocation, still pending:
    make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses

E/verify-parent.json records PID43082/start1794225902884912000ns. Original build/wait process43277, original incarnation1694892236421916. Exec session92254 retains the original running command. There is no E/verify-exit.json and no corrupt_proof/wait4.json; actual exit is unknown, not2 or0. No canonical verify_result is submitted with this pending report.

Immediate launch disk gate: df exit0,29448680KiB>8388608KiB. Seven source auth rows each exit0; numbered build commands000–059 each exit0. No source/prefix/run/cache cleanup, reset or stash occurred.

P retained device16777234/inode189320907, mode0700. Basis SHA256288260cd404dc62e5aa78fc3f0cfe0798cf838481a9b1b5630805a9dc9d23139. Source lock SHA256bc3d60f3194914bb38d971c7025a3b19d880e455ac4f06b700f484b470f66baf; git SHAb8a2dac0c19cb4b25df8497d7be0266ace9e53a991cc46346158b516cf3115cf; gpg SHAf4f309270b8c86968d6e359983bf24b65df10504bd493e2d60a2f9b9333a969c.

Derived G43=26candidate+17fixture refs, from consistent deduplication by(root,path) in descriptor/basis. This is derived, not a runtime counter.

Read-only process inventory at1794226238756009000ns: controller90389, parent43277; ps showed parent S and controller SNs, no direct children; lsof exit0,55 numeric-FD lines. Exact ps/pgrep/lsof output is retained in E/pending-process-inventory.json. No other process was signalled. The controller is still an owned effect even though its leader helper has been physically reaped.

## Per-case results, debt and margins

| Case | Outcome / actual wait4 | first_failing_term | Final debt s | Margin s |
| --- | --- | --- | ---: | ---: |
| positive | PASSED / PID89282 exit0,check0 | none;28 terms true |13.286537622|14.438642 at reporting start|
| corrupt_proof | FAILED raw refusal; controller90389 still pending | not emitted | unknown; raw snapshot12.354342750 | unknown; no final receipt |
| pause_resume | not run | not emitted | unavailable | unavailable |
| cancel | not run | not emitted | unavailable | unavailable |
| stale_incarnation | not run | not emitted | unavailable | unavailable |
| clock_failure | not run | not emitted | unavailable | unavailable |
| debt_overlap | not run | not emitted | unavailable | unavailable |
| reap_delay | not run | not emitted | unavailable | unavailable |

Positive final cost split: helper150225000ns; ordinary raw273665291ns + floor1052391667ns = ordinary cost1326056958ns. Final debt split: helper1352025000ns + ordinary raw2462987619ns + floor9471525003ns = charged=paid13286537622ns, exactly9×total costs. Raw-result paid debt13232537622ns precedes the paid tail. Raw/terminal ordinary_max_ns799625/540375; process_create excess0/count0/known1. Admitted interval29858592000ns; entry→reporting15419950000ns; margin14438642000ns. That cutoff precedes final receipt emission and is not post-exit wall time.

Positive wait4 confirms actualexit0/status0, USER102710000ns,SYSTEM324785000ns,maxRSS4276224bytes. Ledger:108 has all28 terms true, coverage/resources true, first_failing_term=none. Final stderr:4 cost/persistence/control all passed. GS1/GS2 retain the same signature inode189339683 with nlink1→0; verifier90322 was image-admitted and exited with RI witness, rollup mask31.

Corrupt raw result is publication-time evidence only: quarantined phase/outcome, proofnot_reached, attempts16, created16/reaped15, group_gone=false, retired_coverage=false, error coverage_unknown "foreign or reversed live helper resource identity". Raw A139511000ns and debt charged=paid12354342750ns are partial; no final ordinary raw/floor split, final total or terminal margin exists. Raw ordinary_max_ns675750 is below1ms but cannot substitute for missing terminal cost evidence. Empty stderr contains no paid-tail, terminal or final-conjunction row. No timeout exit is invented.

Requested pause_resume stop/errno/order/resolved fields are unavailable because the case never ran. Positive/corrupt diagnostic defaults from other cases must not be relabelled as pause evidence.

## Observation takeovers and corrupt hold

Observed observation_terminal records are:
- Positive:16 rows, all obs_terminal_site=escaped_cleanup_bsd, obs_terminal_errno=3, lines2,9,16,23,30,37,44,51,58,65,72,79,86,93,100,101.
- Corrupt:14 rows, the same site/raw code, lines2,9,16,23,30,37,44,51,58,65,72,79,86,93.

These are terminal receipts, not additional helper births, unique owners or evidence that all27 sites were exercised. No other site is evidenced by this run. The raw code is interpreted in its frozen site/domain; errno alone is not an exit witness. Complete per-row records and exact enum validation are retained in E/telemetry-pending.json.

Corrupt's first refusal selects C:3218–3222, engine_usage_identity: RI start absent/mismatched, or decreasing retained USER/SYSTEM/child counters. The error names neither member nor failed comparison. It follows a successful RI read at3510–3513, outside failed-observation terminal recovery. No errno, OS counter regression or specific cause is established.

Corrupt ledger:100 records leader91377 physically own-wait4 reaped, groupgone1 and helper cost8465000ns retained/paid, group_kill_attempted1/errno0. Ledger:103 records child91380 created but image_admitted0, rollup_mask0, child_cost_unproven1, child_exit_witness=none; its missing proof persists. Those flags do not mean an active child is still present.

C:4516 sets owner groupgone before4518's child-cost coverage check; only4519–4521 increments the cumulative child reaped count after coverage. Thus owner groupgone1/leader actualwait4 and component raw16/15 are consistent. child_cost_unproven is sticky (2865–2870), terminal rollup skips it at4457, and hold readiness at4592 still requires engine_settle_proven. The frozen hold cannot recover the missing child proof. The first semantic resource error is distinct from this later accounting failure. No GS1 admission or invalid-proof verdict was reached.

This remains a live quarantine, not a successful corruption control. The original controller handle and parent remain retained under actual-reap law. No teardown is authorized by this report; Oracle's exact disposition is needed.

## Snapshot integrity and remaining acceptance lines

Raw stdout equals output/result.json byte-for-byte for both cases. SHA256:
- Positive stdout/result e5b150648a4b244a591744faf6a370b7165be628f21e76a0f89fe0bb41e443da; ledger574c99c82d1b594aa93f8f5356cdaf65e8d6f0c8a538d57b4ea22715ca7f7cc2; stderrd587d0f684c9b35dedc46188491f2b1831848bb731ea9b5bf5f6c646a8b663f6.
- Corrupt stdout/result3481392a5612ba5d9498caa98155036e1a5aeace21a14c9f3b842a9681ff0ba6; ledger8d5bcbee1de0ec488cbc5864a2379cf98393b0ebd1d98bff6a1e6e8af677184b; stderr empty SHAe3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855.
- E/telemetry-pending.json88594bytes SHAd70ca05d15714f794e779861e0f6bd6f4b8593edb50633903e3de22874298d08.
- E/pending-handoff.json30101bytes SHAf4dc7f47e89493e509c20fa7d7e3a27a810485ab8a4ff4d44861f73a721c054b.
- E/failure-analysis.json4007bytes SHAf0f9521e8f30ec34ac141d3d51d8637116829fb53848270be51e3ec14b5a06ee.

Pending hashes are snapshots as observed before disposition, not promises that still-owned artifacts will never append. E/pending-handoff.json preserves original identities, completed unit/auth/build exits, code checks, raw artifact pins and all24 prior ledger length/hash/device/inode preservation checks. New evidence data is mode0600. All original source/prefix/run evidence remains intact.

Acceptance state: exact signed input/authenticated source/private build/existing unit controls passed; positive real proof passes. Required corrupt proof did not complete and the original controller has not exited. Pause and later controls, final test/formal/TLC/Yarn parity/license gates, qualification, receipt-last publication and make land were not reached. No whole-native-lifetime, Q2 product/four-job/exporter or runtime approval is claimed. Prior independent passes and failures remain historical.

This is the one blocker handoff for INPUT69. No native retry or substitute was attempted.
