---
type: report
title: INPUT67 proof cases pass; pause STOP refusal blocks qualification
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T02:51:20.835Z
commits:
- db0b6000003dac6402a6da6a4fb2477eab1dd7d2
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M6Z3F551E3GA03ECT40VABRX
updated: 2026-11-09T02:51:20.835Z
number: 93377961366425
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:0f9637419d39d4fb33d46827ba9b3f36cd5b6ba2ed24492e09dfff7e1472cac6
  frontmatter_digest: sha256:ef383f552553f18b4e4d56282018e4db3cebad48f0e57e8555a3873bdb0f751f
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: YlStU6JYQ8vswtY3rfv23T8CJ6XgRdBTS8EABNZvYOEUNVP2ZzCz8cbT5hM3N/D+cjNd07zlxqF5v/B02Uz2Dw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150901
  event:
    id: 01M6Z3F9ZJHHA7RY40F7BJ7YJY
    sequence: 150901
    hash: 75a420bdaa47c67df25703f3a8fbf2873759f0e33ca63481e9e6529dc019df8f
---
INPUT67 is FAILED overall: the sole canonical invocation exited 2 after pause_resume's actual controller wait4 exit 81. Positive and corrupt_proof are PASSED independently (actual exit 0/check 0, valid and invalid proof respectively, full 16/16 settlement, all 28 final conjunction terms true, first_failing_term=none). No retry, teardown, final qualification, receipt or landing occurred. runtime_approved=false.

The required decision is Oracle disposition of the frozen engine's failed foreground STOP syscall and a corrected authorized input or scoped repair before another native attempt. The errno is unproven: the frozen code stores it internally but does not serialize it.

## Immutable inputs and code

Task: [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], current section ORACLE FREEZE, INPUT67. Executor: seat:01M6W6K5SWZB5AEBAAAA73FF4M. Writing tree W=/Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools. Signed code commit db0b6000003dac6402a6da6a4fb2477eab1dd7d2; git verify-commit HEAD exited 0, Good Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0. Final git diff --check and git status --porcelain=v1 exited 0; status empty. Code commit carries Report: [[reports/2026-11-08-dream-proof-tools-input67.md]].

Only scripts/dream-proof-tools.py and tests/fixtures/dream-proof-tools/sources.lock.json changed. Exact INPUT67 selection:
- Signed source 1f1b30ddec05e5d8e092d91b92c161bd94d1cc38, verified Good Boxy. Ancestry checks for 4a62faf1 and 2ce4359, and containment on origin/main, all exited 0. Observed origin/main was exactly this SHA.
- C=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input67/dream-read-c-preparation/compile-lrlj5_sr/proof-helper-probe.c, 387443 bytes, SHA256 98a0ff5f850e38112a614a52152523e4dc842523c7aba67a7fadd8683ae1d07d.
- Sibling probe: 285448 bytes, SHA256 65da9c9e97a130ddf7962e3df03d41f58c226a48458066610e793b159c65195d.
- Sibling preparation-d6a854c8fd0e4f1784ddd56d72da9bf4f645bce126b66e417aa1ede920efac14.json: 530062 bytes; hash matches name.
- All ten inputs match signed-source/snapshot bytes. read.rs SHA c2df3b34314142841a5ccaff3850fc26b796ff62a3852987500343568c98bd94 and C changed; eight inputs unchanged from INPUT66. Proposal source SHA 297fb6bde7c5c275b8f58a6f7923ae59204bcfad46d9950c967f4d58d592ee88.
- Schema blob 96e2839a9d2b6031fb0138934eef96850e9b7a60 and vector blob 54cfab3b0e6cc8996bbc942cc2b0128278fccfaf have identical bytes at b26d7cd3 and INPUT67; no b26 ancestry claim.
- Tuple v2 SHA 48c72b0865936fac5ec5fc43f890db3c4e97e3946b24e63e351cade4249030b4; independent audit SHA 03224241f45e209717e782ae8e92c66756c162d4644cfac9fb8ed369fd78f956; complete ledger delta SHA 2eed29e10b8dcf9971e4167e2cbcaa8d0bc790de58aa4c5cc4abd81834ba0087. Result emission, terminal and paid-tail format strings are byte-identical to INPUT66.

The production adapter retains the raw ledger without interpreting its kinds/fields, so the authorized C36/C40/C45 delta needs no production schema extension. Terminal whitelist/type/bounds/unknown/missing/duplicate refusal remains exact.

Two scoped offline compatibility repairs preceded this single native launch:
1. scripts/dream-proof-tools.py:3121–3122 binds clock_failure's sole permitted auxiliary resource_limit detail to the frozen exact literal "process creation native2ms wall250ms application1ms inclusive3ms gate" (decision 66331, C:3766–3779). Tests at :4618–4641 accept that literal and reject the retired wording, changed wall/CPU/inclusive limits and unrelated detail; cost/histogram/persistence gates still apply.
2. :3137–3149 binds reap_delay's earlier quarantined result to later settled terminal evidence only for actual parent exit 81, final settled 1/1, raw phase/outcome quarantined, proof not_reached, exact integer attempts 1 and exact typed raw settlement {created:1,reaped:0,group_gone:false,retired_coverage:false}. This reflects C:3979,4022–4031,4923–4924,4946 and existing actual-reap law 66074. Tests :4643–4668 reject wrong case/exit, partial or changed snapshots, bool-as-integer substitutions and bad cost; raw bytes remain unchanged.

These repairs alter classification and diagnostic binding, not ownership, lifecycle actions, budgets, ordering, raw results, authority or runtime approval. Existing tests/formal/dream_proof_tools.rs:1518–1682 component_host already models independent wire arrival, actual wait4, helper settlement, debt/persistence and verdicts. Its SHA remains d0be23400297885bfa2d1fe6ffa39414f0c38fc9223dd7ae010accfa32670d41. No new lifecycle/model action was introduced. The current-source formal targets were not reached. clock_failure/reap_delay have unit evidence here but no native witness in this attempt.

## Commands and retained run

E=W/target/dream-proof-tools/input67-20261108.
R=W/target/dream-proof-tools/runs/01M6Z2NJ9Q3GETXWTJ43M9FZGC.
P=W/target/dream-proof-tools/candidates/01M6Z2NJ9Q3GETXWTJ43M9FZGC.

Offline input ingestion exited 0. Unit command "python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target" exited 0, 68 tests; E/unit.log 11737 bytes SHA256 72ba9c314045360fbfb6b5479329617903744f4e93362cd5da24f42f213ba767. Recipe SHA256 7748a297be77488744cfc0222154e24a091590f71fd5dc759b51e8a9ddd97b4d.

Disk gate passed immediately before launch: 14248544 KiB available, strictly greater than 8388608 KiB; df exit 0. No cleanup by this seat. E/launch-disk-gate.json records the sample and command.

Sole canonical invocation:
    make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
Actual final exit 2. E/verify-exit.json records start/end 1794191834302811000/1794192133969890000 ns. E/verify.log 9798 bytes SHA256 03cee31022351715b20dafdf8d8b95190860f53e38a1e6589b809668df76e56c. Seven authenticated source rows each auth_exit=0; build commands 000–059 each exit 0. Build then ran positive, corrupt_proof, pause_resume and stopped at the required refusal. Later canonical targets did not execute.

Original build incarnation 1660824179771583. P retained, no reset/stash/replacement or reuse of failed prefixes. Source lock SHA6040b09c4019933211f4b2b3e62f8c233f1f737da0f1a3c0a8a1bee54fb3cfaa; basis 47112 bytes SHA9730e21c7b232d8cc4d052dd08f761476e9de32ad1a00c2c6126f4c5bee9e9be; git SHA7df63e14ca29388c0a7f461012673727f645b234ca6e8b7bbf4de25eb4e10d8d; gpg SHA7d6f577f60eae7b7b1b283175f1f417327b1e57a94048abc92f18b17b9ef6af2.

G=43 is DERIVED from consistent deduplicated guarded (root,path) refs in descriptor/basis: 26 candidate and 17 fixture refs. It is not a measured runtime counter. Compiler/SDK/platform/review observations are not counted as guarded refs. Source/compiler/native platform assumptions remain conditional; no whole-native-lifetime theorem is asserted.

## Per-case outcomes and costs

Every controller has a retained actual wait4 record; no signal was sent. Ordinary maxima below refer to the raw-result cutoff; final terminal maxima are separately reported.

| Case | Controller / actual exit / check | Outcome and proof | Settlement | first_failing_term | Raw / terminal ordinary max ns |
| --- | --- | --- | --- | --- | --- |
| positive | 89113 / 0 / 0 | PASSED, completed, valid | 16/16, group gone, retired coverage | none, all 28 true | 476959 / 449833 |
| corrupt_proof | 90226 / 0 / 0 | PASSED, expected_rejection, invalid | 16/16, group gone, retired coverage | none, all 28 true | 462709 / 455084 |
| pause_resume | 91279 / 81 / no checked.json | FAILED, coverage_unknown, not_reached | 8/8, group gone, retired coverage | not emitted: failed before conjunction | 482584 / 466667 |

Final-terminal costs and the resulting debt are different units. Costs, seconds:

| Case | Helper cost | Ordinary raw CPU | Ordinary floor surcharge | Ordinary cost |
| --- | ---: | ---: | ---: | ---: |
| positive | 0.107951000 | 0.222708791 | 0.996291209 | 1.219000000 |
| corrupt_proof | 0.113474000 | 0.226838917 | 1.003161083 | 1.230000000 |
| pause_resume | 0.054067000 | 0.154376083 | 0.820623917 | 0.975000000 |

Each component below is nine times its corresponding final cost; charged=paid exactly, no repeated cumulative rows summed. All three process_create excess_ns/count are 0/0, known=1.

| Case | Helper debt s | Ordinary raw debt s | Floor debt s | Final charged=paid s |
| --- | ---: | ---: | ---: | ---: |
| positive | 0.971559000 | 2.004379119 | 8.966620881 | 11.942559000 |
| corrupt_proof | 1.021266000 | 2.041550253 | 9.028449747 | 12.091266000 |
| pause_resume | 0.486603000 | 1.389384747 | 7.385615253 | 9.261603000 |

| Case | Admitted interval s | Entry to reporting s | Remaining margin s |
| --- | ---: | ---: | ---: |
| positive | 29.845569 | 14.158266 | 15.687303 |
| corrupt_proof | 29.989596 | 14.579762 | 15.409834 |
| pause_resume | 29.988610 | 10.759528 | 19.229082 |

Margin is original_deadline_ns minus reporting_started_ns, before the final receipt emission. It is not a post-exit wall measurement. Raw-result paid debt was 11.888559/12.037266/9.198603 s; the later final terminal includes its additional paid tail/diagnostic work. No deadline or ordinary_failed_charge was recorded.

## Pause evidence and refusal

R/components/pause_resume/output/result.json:1 and stderr:2 report coverage_unknown, "foreground STOP pending independent terminal settlement". C:3585 calls kill(-o->ticket.pgid,SIGSTOP), stores pause_stop_errno; C:3586 emits this exact detail on nonzero return. The run therefore establishes a failed STOP syscall. That errno is not serialized; group_kill_errno refers to a different operation. ESRCH or an exit race is not proven. C:4084 checks helper exit before C:4086 sampling/C:4087 pause, but a specific intervening exit cannot be inferred from these logs.

Owner identity includes sequence/PID/birth/start in E/telemetry.json. These are per-owner terminal snapshots, not an additive stopped-time total:

| Seq / PID | Ledger helper / pause line | stopped_interval_ns | running_gap_max_ns | stopped_system_ns | ACK / resume |
| --- | --- | ---: | ---: | ---: | --- |
| 1 / 91778 | 2 / 3 | 178573000 | 13108000 | 0 | 1 / 1 |
| 2 / 91837 | 8 / 9 | 159244000 | 12460000 | 0 | 1 / 1 |
| 3 / 91880 | 14 / 15 | 0 | 9441000 | 0 | 0 / 0 |
| 4 / 91904 | 20 / 21 | 0 | 12572000 | 0 | 0 / 0 |
| 5 / 91935 | 26 / 27 | 0 | 11467000 | 0 | 0 / 0 |
| 6 / 91972 | 32 / 33 | 0 | 11889000 | 0 | 0 / 0 |
| 7 / 92003 | 38 / 39 | 0 | 9313000 | 0 | 0 / 0 |
| 8 / 92030 | 44 / 45 | 0 | 10158000 | 0 | 0 / 0 |

Only owners 1–2 positively stopped/resumed and passed the pause-user check. All eight physically settled. Per-owner maxima across this case are stopped interval 178573000 ns, running gap 13108000 ns, stopped SYSTEM 0 ns; SYSTEM/sticky violation bits are 0 throughout. Owner 8's no-ACK state must not erase earlier positive stopped intervals.

C:2853–2878 requires aggregate stopped SYSTEM at most 50000 ns with known conversion; C:3800–3825 still charges authoritative wait4 USER+SYSTEM and its nine-times debt. Zero growth observed here is not a free SYSTEM allowance. Pause never reached the Git verifier child.

Positive/corrupt signature leader rows at ledger:89/91 show child_image_admitted=1, child_exit_witness=kqueue_note_exit, discovery=fork_event, one fork, no fork-seen/missed child. GS1/GS2 stderr:1–2 retain the same inode/signature bytes with nlink 1→0. Positive's bounded stopped-ACK evidence has 2 waits, maximum actual wait 1132000 ns, summed wall 1145000 ns, STOP→ACK 1280000 ns; corrupt has 0 waits, STOP→ACK 44000 ns. Requested 1 ms wait is not a claim that actual wait never exceeds 1 ms.

## Evidence integrity and remaining acceptance gaps

R/components/<case>/stdout equals output/result.json byte-for-byte for all three. SHA256 pins:

| Case | Unaltered stdout/result SHA256 | Raw ledger SHA256 |
| --- | --- | --- |
| positive | e30110793fc3c30830486ee2139889f6ab6a707f7ccdd80e590c1049e6546950 | 76a12b5f03ed5489873c28e3783d22be596be72a9a2b689aa3573e67ecc9fba6 |
| corrupt_proof | 5e5eea75530c4bf4d74dd0acb836c0cc189635af067e28ba3fe7f5a9b547e912 | 37468c728ee30a77f05a1c20c328362538b78702a5cbaaea000d2c096b51f0d2 |
| pause_resume | 2f17a24524044e96651e65c62ee03a24bdcdc87a0520cd5b6bbdc3a831f470b0 | 6fd8e6af15f596dd8f253da2b1affc88d634112acea6abfb7ba766fcec8eb67e |

E/telemetry.json: 38876 bytes SHA4660b058f4d45a9a2acf96a3e6921f3e65c272843126dcb8879bf36443faaf86. E/handoff.json: 31184 bytes SHAfc1117a41ca3786462709ae1a1f8ba9dd6569444396c75fa22f71a7bc9f49e1e; includes all raw component file/wait4/checked pins, current code checks, source failure lines, auth/build exits, original prefix identity and all 20 prior ledger length/hash/dev/inode preservation checks. New evidence data is mode0600. Read-only evidence assembly exited 0 after correcting two analysis-script assumptions (untyped ledger header and nonchronological directory enumeration); no native process was repeated.

Acceptance state: exact source authentication/private build/config/closure/review/basis retained; unit and the two real proof cases pass. Required pause control fails. cancel, stale_incarnation, clock_failure, debt_overlap and reap_delay did not run. make test-dream-proof-tools, final formal/parity/license gates, qualification, receipt-last publication, make land and full product/exporter/four-job approval did not run. No candidate approval or available lesson exists. No cleanup, threshold relaxation, terminal/ledger fabrication or peer source edits occurred.

Earlier INPUT66 pause and other failures remain historical with their original dispositions; earlier independently passed proof cases remain passed. This report supersedes no evidence. The task remains unqualified pending the requested Oracle/Q2 disposition.
