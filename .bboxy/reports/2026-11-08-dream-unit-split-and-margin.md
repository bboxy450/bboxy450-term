---
type: report
title: 'Dream Q2 unit split, margin and INPUT67: landed verified source'
task: '[[tasks/2026-11-08-dream-q2-unit-split-and-margin]]'
author: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
created: 2026-11-09T02:38:46.759Z
commits:
- 1f1b30ddec05e5d8e092d91b92c161bd94d1cc38
- 6e323e07c71907efa47b6ccad3c9c197bf048f72
- afa125def907e5d02ff50bad795cfcf590de427b
- c81b2db623c7dab9e2a895864eb6317193f2bbd1
- 14166f8723ec3246302c97385470dfeae7d9a4f0
- c15b287a127386750b7a97f0e3c43a3ef10bdcb4
- bc4f7fadabb3c7c73a67bba90c3abc2037e6b16b
verify:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
id: 01M6Z2R8AY71KCC8HS24AGCMPV
updated: 2026-11-09T02:38:46.759Z
number: 93377961366422
created_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
updated_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  kind: seat
  public_key: 3lIiKAyaYQbqAYQ3xsrS+NTGLp9s//BjqCA4HkK5nF0=
  process_generation: '1'
provenance:
  digest: sha256:a7b2f884d69b133f1594d7cae3f160a5f1ee9ca2dcd7186ce2acc15dd3563947
  frontmatter_digest: sha256:6f647cfbe5cfe192cfee1b0c52d6221260aac5597859f77e2a78ba27c42d50cb
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  sig: gsQG8Xf4sSvFzmgy6BshqJXr8cM6MAIrTxQ5BU7stYmLXu/sztjwSXC83Qb+yTE5BnD6oFSgSSK5SGwtQaTyCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150889
  event:
    id: 01M6Z2R9KTWNWX67P7ZG0QWGQ5
    sequence: 150889
    hash: 3b9be840971c3704a9aab4b68c3435a194c89d10a217c6f04379b40b1deeacf1
---
Landed signed commit **1f1b30ddec05e5d8e092d91b92c161bd94d1cc38** with the exact prescribed four-command verification and canonical `make land`, both actual exit 0. The worktree is clean; actual post-land remote verification found `origin/main` at that HEAD and proved containment. INPUT67 was compiled and independently audited after landing, then handed off once as native message **1557 / 01M6Z1XS14Q37RAC5JZKAYJ0KB**. No builder or real candidate was run.

This reports [[tasks/2026-11-08-dream-q2-unit-split-and-margin]] (#93377961366374), latest unchanged brief digest `a8e11e4bfab0cb2a18777f7847d37a53845352d397e55128e2ab81c2d60b8fbb`, including native rulings 1507, 1545 and 1551. Earlier accepted tasks remain frozen. Evidence paths below are relative to **E = target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3** in `/Users/feral/mydev/bboxy450-harness-dream-read-q2`; immutable formal evidence is under `verification/evidence/`.

## Changes and acceptance evidence

| Requirement | Implemented mechanism and evidence | Result and boundary |
|---|---|---|
| Split the oversized helper observation | `proof-helper-probe.c` separates queue/census/IO and member/watch/GS1 work through epoch-aware paid cuts. Discovery through EXEC/image admission, the existing one-shot custody STOP and positive ACK remains one open span; full rest then precedes GS1. The old 1,020,958 ns row identifies signature/child-GS1 work, not an individually timed syscall. The supplied composition control conserves that complete old cost; its old-combined mutant exceeds the unchanged bound. | Current canonical checker and exact chain 0. Nominal 400 us CPU prechecks, 64 finite iterations/steps where specified, hard 1 ms ending charge and fresh full 9x rest remain. Finite source bounds are not an OS worst-case execution-time theorem. |
| Reduce floors while retaining checks | Currentness batches retain all 33 fences, all three root checks, deduplicated retained file guards, platform/seal checks and ordering. Batches use actual 400 us/64-step guards, including open/stat/close work; each failed/final unit pays its complete ending bill. Real retained-file tamper defeats the dropped-check mutant even when its completion count is forged. | Current C 0. No missing check, free ending unit or debt credit. Source-conditional projection below is the accepted margin evidence. |
| Same-wake image admission | Both first CLONE capture and retained unadmitted capture stay in the same open span through retained-knote event wait and approved image. Fresh actual RI original-start validation precedes the existing STOP; closing TASKALLINFO supplies its genuine positive ACK. Newly failed observations cannot dispatch new STOP/GS1. Rest is paid while positively stopped before GS1, then leader refresh/payment precedes guarded CONT. | Real slow-CLONE/GS1/GS2 process positive; supplied pre-STOP-rest and nonblocking-only production-API mutants fail their intended assertions. Exit-before-admission refuses distinctly. API negatives are explicitly nonphysical; they do not claim physical quarantine settlement. |
| Actual member maximum and diagnostic conjunction | `members_max` derives from actual admitted exact-current live retained rows, including same-wake admission; the existing group-census path remains. All 28 independent final-case booleans and first failing term are serialized through the original ledger FD. Reporting tail work is measured/paid before outcome commit. | Real same-wake peak 2; separate census-only peak 1. Current drop-update mutant uses supplied production API rows and supplied other classifier terms, producing count/peak 1 and `members_max_complete`; no physical GS1/reap/bill claim. Historical real physical negative remains separately retained. Omitted-term serialization mutant fails. |
| Exclude only certified stopped time from cadence | Opening positive stopped observation plus fresh RI and closing corroboration establish ACK; fresh pre-resume RI seals both endpoints. Raw begin/return/max gap is retained; pre-ACK and post-resume running sides are independently limited to 50 ms. Unknown/reversed clocks, absent ACK, return overshoot and stale end sample refuse. First failed endpoints are retained. | Real >80 ms ACK-held pause passes with full known bill. Running-gap/fake-stop/stale-end API controls refuse. Native 1507 classifies the old 103.627 ms case by source ordering only: numeric ACK/failing endpoints and old SYSTEM equality were not reconstructed; its original failure remains failed. |
| Exact USER and bounded SYSTEM | USER remains exact, with only existing terminal wait4 USER quantization of 1 us. Retained Mach timebase converts each member's raw monotonic SYSTEM delta by checked ceiling; checked sum of nonduplicated per-member maxima must be <=50,000 ns total. No allowance is multiplied by member count. Unknown/overflow/excess remains raw and refuses. | APIs cover USER +1, 50,000/50,001 ns, aggregate 30,000+20,000/+20,001 and conversion boundaries. USER-only live/terminal mutants are labelled production API, SYSTEM fixed, supplied whole CPU fully billed once. Real modes 9/10 remain. SYSTEM is already inside authoritative leader wait4 CPU: never subtracted or separately added. |
| Bounded post-STOP event observation | Retained-knote wait remains in the same open ordinary span. Native 1545/1551 caps **requested timeout** at 1 ms and remaining deadline/RI headroom; actual STOP-to-FIRST-positive-ACK must be <=25 ms, with fresh RI/running 50 ms/deadline/CPU checks on return. No usleep, new STOP or deferred closed-unit debt. | Physical delayed-observation control uses the existing hook and same production wait, then genuine stopped status. It demonstrates delayed observation, not a kernel slow-STOP theorem. Supplied never-ACK at 25 ms+1 ns refuses with exact `stop ACK not observed within bound`. Actual per-wait wall may exceed the requested 1 ms and is recorded honestly. |
| Complete bounded lifetime observer | `private-engine-controls.c` scans complete fstat-bounded snapshots from the same once-opened ledger FD, existing 2 MiB cap, 4096-byte chunks, finite positive progress and N-1 overlap across chunks/appends. Every observation/format/close is charged/paid. | Required owner-lost marker beyond 8192 bytes is found without reopen. Original controller exit 81, owned helper wait4/group-gone, sticky refusal, deadlines and full cost assertions remain. This repairs fixture truncation, not producer law. |
| First-priority Rust retirement work | `src/dream/read.rs` now books the same captured pending CPU into closed-first pipe metrics and canonical Budget under the helper mutex; open-first `account_pipe` remains. Unlock precedes payment. Both-order gated controls check metric=pending and billed=pending+actual signal span, first Cancel and fresh post-span 9x rest. | Separately signed/landed `bc4f7fadabb3c7c73a67bba90c3abc2037e6b16b` before feature continuation. Original named rest failure is UNESTABLISHED; the actual adjacent metric omission and old-skip causal failure are documented below. |
| Models and full reference parity | Six formal sources are unchanged: 442 models, 94 contracts, one crate. Existing graphs prove aggregate complete cost/debt and custody/lifecycle; no absent concrete timing/counter coordinate is invented. Source/process/API controls cover the refinements above. | Fresh current-source native and pinned Java full state/labelled-edge/named-verdict parity 0; complete exploration, unchanged limits/tools/fairness. No numeric 50 us/50 ms/STOP-wall, derived peak or pipe-metric theorem is claimed from those models. |

The source-conditional projection `unit-split-floor-projection-v2.json`, SHA256 `fda1c98d16d2ca7b90662aa68bb1548efed8e79f1e2b728791b774285e84910f`, uses `N = 33*(G+7) - sum(B) - K` and `new margin = 690 ms + 9 ms*N - deltaWorkWake`. Actual deduplicated G, batch/poll counts and additional work/wake are unavailable. G8/B2/K4 gives 4.515 s only assuming delta=0; G2/B2/K4 gives 2.733 s and is not certified. The Task explicitly accepts this conditional evidence. Its separately cited INPUT66 builder run reports positive/corrupt completion and 16.59/16.99 s margins. Those are historical external INPUT66 facts, not an INPUT67 loaded-margin or full-private-30 s qualification claim.

## Actual final commands and preserved current evidence

The exact command was executed on the final signed HEAD, with source-before equal to source-after:

```sh
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
```

Actual exit **0**. Record `unitmargin-exact-four-chain-final-v8.json` SHA256 `f74b442026f537d8070ada3cb3c86996d00fd80f0a3dedcb64fa06557b401ef6`; full log SHA256 `315bbf7430a5066e352dd4c7224db87bd3f1d325fa324d77ee11e648f825da78`. Each component's actual 0 follows the retained literal `&&` chain and output. The run used the single `target/harness-landing` tree, `env -u FORMAL_JOBS`, `RUSTC_WRAPPER=/usr/bin/nice`, empty `RUSTC_WORKSPACE_WRAPPER` and `CARGO_INCREMENTAL=0`; no gate knob or threshold waiver. C1 includes all 17 seat tests; MCP client 18 and codec 324 controls pass. Nonfatal installed-rustup `rust-objcopy`/missing-libLLVM debug-strip warnings are retained; commands still exited 0, and no toolchain repair was attempted.

Canonical **`make land` exited 0**: `unitmargin-canonical-make-land-input67-v4.json` SHA256 `bc4dc3e2cd14affc11b78409fc42bc5b37f9c39a2615fe280c68de1fbd379c83`, log `36b3f12f870c279c0309630f741af1ce3e8aa608fc8a969e96a4d32200457410`, canonical receipt `9a017ffd12e0fc18f770464e832b8d40579e3242b4e7778739867da1444b7b5b`. Its genuine strict/clippy/nextest/serialized timing/formal/fetch/push rows passed. Upstream `scripts/land.py` SHA256 `e40c6a10601aae4044b38c852251a6274178b01e5f72714c7b0221c417257d6a` was consumed unchanged. No local workflow relaxation or GUI ignore was added.

Post-land `landed-remote-and-signature-verification-input67-v4.json`, SHA256 `29f2351f9008ff15f752f108ab70460b009791cd85170b751d6a60a71693bdd1`, records clean HEAD equal to observed remote main, containment exit 0 and Good signatures on all six outgoing commits using key `9B07E849480C68328CA20684844ECC0587DE7AB0`. Current upstream includes `4a62faf11e2edb0068cc1fa89dcea05b7d7fa4cd`; the standalone priority fix is an ancestor.

The current C checker receipt `canonical-C-input67-v5/run.json`, SHA256 **`d54f3d64de9e9d00c0773a61b12c76a098a61165deedcd814efee395a83c0818`**, contains **113 baseline cases, 72 intended mutants exit 81, 74 strict compiles exit 0, 174 commands, no timeout, source unchanged**. Its 676 raw nonexecutables are preserved by `canonical-C-input67-preservation-v4.json` SHA256 `dc300cd19f1c3be776f2b990d4ea171f9e3f10933ea66d5746d886f4452f2639` (the preservation filename is v4, receipt directory v5).

Current final-chain physical measurements, distinct from earlier standalone runs:

- Delayed-observation leader 58179/child 58183: requested timeout 1 ms, actual maximum wait **1,127,000 ns**, STOP-to-FIRST-genuine-ACK **1,138,000 ns**, full critical unit 179,375 ns, RI max 36.480 ms. GS1/GS2/mask31 and original ownwait4/group-gone/queue+FIFO close hold. Helper CPU 11.602 ms produces full 104.418 ms helper debt; completed global 329.418 ms is paid.
- Main paused original 57579: ACK-held 128.089 ms, raw gap 107.840 ms, running maximum 10.468 ms; USER172765 and SYSTEM77625 remain exact, stopped SYSTEM 0. Full CPU10.452 ms/helper94.068 ms/global274.068 ms paid.
- Physical mode9 incidentally observed SYSTEM45001->45175, 174 ticks at125/3, **7250 ns**, exact USER and full authoritative CPU12.946 ms/helper116.514 ms/242.514 ms ledger debt paid. This is an incidental bounded physical observation, not a targeted reproduction of the historical106 ticks or a kernel-cause/qualification theorem.
- Normal same-wake 57925/57928: unit241792 ns, actual peak2 with census peak bypass, CPU7.069 ms/helper63.621 ms/completed288.621 ms paid.
- Held controller58124/helper58129: exact settled-owner-lost marker offset8862 in13496 bytes; four4096-byte same-FD reads. Controller81/one result follows actual original helper wait4/group-gone; CPU5.548 ms/helper49.932 ms/global238.932 ms paid, ownerloss/refusal stays sticky.

Current complete formal archive is `verification/evidence/f5acd1e5688f0ff4161827e2bb7727dc3170d07513e5e1ce4586cb4f744e292e`: source1f1b/dirtyfalse, **3,378,027 states and 11,366,941 labelled edges**, all graph_equal, 442/94/1. `current-formal-all-raw-files-input67-v5.json`, SHA256 `f59d825f9e191b53298518750951aefbaba3fc8fabd59e6a43b4f61d1c14b201`, records driver verification of **all4422 raw files/7,436,152,618 logical bytes**, then sealed0400 files/0500 dirs. The working proof copy was cloned into an absent cache with distinct inodes/nlink1, only working graph data writable; original tool paths/inodes retained. Canonical `check-formal-evidence` exited0. Supplemental independent review rehashed small records only; it does not replace the driver's complete raw hash check. `complete_tlc_replacement=false`; no arbitrary Rust/OS or full TLC-completeness claim.

Actual C1/READ generated metadata was preserved byte-for-byte with independent readonly worker inodes, manifest `retained-1f1b30dd-actual-profiles-exact-four-chain/preservation.json` SHA256 `923134df64daa422308904e775e4b0d4c49880b68178ecdfcd189a504f5cfe18`. Both actual worker hashes are `c1427d9fbb154ccc6de81386055dd887bf3d69a04bdb01018d7f3fb0537d25e4`. READ has its actual39-source snapshot; C1's compact profile has no separate READ-style input map. Five observer JSONs were preserved using the canonical helper on their original inodes after real MCP0, manifest SHA256 `11af45a71661f20b094c8d6e9b2c11f2decefb6337134214f8136f7bfd4b3edf`. Metadata was never reconstructed or rebound from an old HEAD.

## INPUT67: immutable preparation after landing

The sole handed-off tuple is `INPUT67-source-freeze-tuple-v2.json`, SHA256 **`48c72b0865936fac5ec5fc43f890db3c4e97e3946b24e63e351cade4249030b4`**. It supersedes INPUT66 as current consumer input; all old inputs remain immutable. An unsent v1 draft confused997 audit checks with353 output entries; v2 corrects that count without a second preparation or handoff. Native message1557 is the sole essential INPUT67 handoff; receipt is `INPUT67-native-handoff-receipt-v1.json`.

Actual Land finished2026-11-09T02:15:21.211082Z; preparation began02:16:57.926270Z and finished02:17:00.562001Z. `INPUT67-after-land-preparation-order-v1.json` binds current HEAD==source_commit==preparation_commit and all10 signed input commits. The generic preparation script alone does not establish that ordering; the retained actual sequence does.

Immutable compile root: `E/prepared-input67/dream-read-c-preparation/compile-lrlj5_sr`. Exact physical C387443 bytes SHA256 `98a0ff5f850e38112a614a52152523e4dc842523c7aba67a7fadd8683ae1d07d`; executable285448 bytes SHA256 **`65da9c9e97a130ddf7962e3df03d41f58c226a48458066610e793b159c65195d`**. Receipt filename/hash `preparation-d6a854c8fd0e4f1784ddd56d72da9bf4f645bce126b66e417aa1ede920efac14.json`. Actual strict optimized compiler exit0; independent audit exit0, detail SHA256 `03224241f45e209717e782ae8e92c66756c162d4644cfac9fb8ed369fd78f956`. Roots0700, retained files0400, executable0500, nlink1; **353 output entries,325 actual lexical/resolved headers,997 checks**, including both pthread.h and pthread/pthread.h. Source, compiler/linker/SDK/OS/dylib identities, exact argv, source/output paths and stdout/stderr are retained.

Compiler is Apple clang21.0.0 (clang-2100.3.34.2), executable SHA256 `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`, actual `-v -O2 -std=c11 -Wall -Wextra -Werror -target arm64-apple-darwin -isysroot .../MacOSX.sdk`, SDK27.0. The four-Rust source snapshot domain digest is `297fb6bde7c5c275b8f58a6f7923ae59204bcfad46d9950c967f4d58d592ee88`; it is a snapshot, **compiled_product_currentness_proven=false** in this C preparation. Current compiled Rust product evidence is the separate actual canonical profiles above. Schema authority remains signed `b26d7cd3432e9cae3e6c68492afd73a82cc8f2e9`; schema/vectors unchanged.

### Exact internal ledger delta since INPUT66

No public codec or terminal field changed. `INPUT67-complete-ledger-delta-C36-C40-C45-v2.json` SHA256 `2eed29e10b8dcf9971e4167e2cbcaa8d0bc790de58aa4c5cc4abd81834ba0087` is the full typed manifest. Existing `final_case_conjunction`28booleans/first_failing_term were already in INPUT66. New fields follow:

**`helper_total_budget`**

| Field | Exact type and meaning |
|---|---|
| `running_gap_max_ns` | u64; max continuous pre-ACK or post-resume running side |
| `stopped_interval_ns` | u64; maximum nonduplicated common ACKed counter-stable interval; admitted resume extends to signal cutoff, otherwise last validated RI return; final kill bounds it |
| `stopped_system_ns` | u64; checked aggregate of per-member maximum converted SYSTEM delta from immutable ACK baselines, total ceiling50000ns; known excess retained, MAX if unavailable; diagnostic only, authoritative wait4 bills once. |

**`pause_user_and_terminal_rss`**

| Field | Exact type and meaning |
|---|---|
| `ack_RI_begin_ns` | u64[2] |
| `ack_RI_return_ns` | u64[2] |
| `ack_ns` | u64; max actual per-member ACK |
| `ack_system_raw` | u64[2] |
| `counter_known_mask` | u32 mask |
| `final_kill_signal_ns` | u64; pre-final-KILL cutoff |
| `member_ack_ns` | u64[2]; opening positive stopped TASK before RI, closing corroborated |
| `observed_system_raw` | u64[2] |
| `resume_RI_begin_ns` | u64[2] |
| `resume_RI_return_ns` | u64[2] |
| `resume_signal_ns` | u64; pre-SIGCONT cutoff |
| `system_growth_limit_ns` | u64 literal50000 |
| `system_growth_ns` | u64[2]; maximum delta per retained member, no child-rollup duplicate |
| `system_timebase_denom` | u32 |
| `system_timebase_numer` | u32 |
| `system_violation` | u32 sticky flag |

**`retained_RI_interval`**

| Field | Exact type and meaning |
|---|---|
| `first_failed_check_ns` | u64; first failure never overwritten |
| `first_failed_previous_begin_ns` | u64[2] |
| `first_failed_raw_gap_ns` | u64[2] |
| `first_failed_running_gap_ns` | u64[2] |

**`retained_child_accounting`**

| Field | Exact type and meaning |
|---|---|
| `custody_stop_ack_ns` | u64 |
| `custody_stop_call_ns` | u64 |
| `custody_stop_errno` | i32 |
| `custody_stop_return_ns` | u64 |
| `custody_stop_to_ack_ns` | u64 |
| `max_actual_wait_ns` | u64; observed actual per-wait wall maximum; requested timeout is capped1ms, actual bounded by STOP-to-ACK25ms/RI50ms/deadline/fullCPU/debt/custody, never claimed<=1ms |
| `post_stop_wait_RI_begin_ns` | u64 |
| `post_stop_wait_RI_return_ns` | u64 |
| `post_stop_wait_calls` | u64 |
| `post_stop_wait_first_ns` | u64 |
| `post_stop_wait_last_ns` | u64 |
| `post_stop_wait_wall_is_actual` | bool true |
| `post_stop_wait_wall_ns` | u64 |
| `stop_ack_bound_ns` | u64 constant25000000 |

`max_actual_wait_ns` replaces the old serialized key `post_stop_wait_max_ns`; it is not an alias and is an observed maximum, never a requested-timeout claim. Retained SYSTEM maxima are converted individually with checked ceiling then summed; conservative rounding can add less than1ns/member and never changes the wait4 bill.

## Priority repair, failed evidence and explicit limits

The exact original named test on main6409 passed alone0, log `cbd208cbebb44e09526c9660eabacbccae47f880012f7ff550e12109a20c076b`. Its alleged always-failure remains **UNESTABLISHED**. A literal no-default embedded diagnostic failed compilation101/zero tests (log `45a6c5ae10abaaec8568cfb356a102bbeeb3c84452e1c3fc56972da0e73b140e`), separate and outside scope. Default+embedded load100 instead exposed a different pipe metric race while the named test passed (log `bb3a855588e47d9d40fab73c8849a7ea4171d34296d31cbbaacebc81da9c5490`). Native1537 authorized that narrow adjacent fix. Four affected tests0 (`81f1bdfcbe925f2472a8b13f92988dc176a1aa701e9a0ca74a66a79bee96f0be`); oldskip mutant101 exactly metric0 vs billed135917 (`9f8e8f989b37f6e47fa02e5402f272fa66bb083e24dcab2c337b889e2252c0fe`); embedded load212pass/10skip0 (`dec187dcc0d2598888e2c84f8f3976c2f8eeb7e66cb8f0f2d0529584df18e9cb`); standalone priority make land0 (`6a2d47f9f9aaac6728912a7ab8893de081d69303a586e66fb5502eaa9881e5f6`). Signedbc4f7fad was landed separately and reported as native1539 before returning to this feature.

All historical red runs remain red; no old receipt is relabelled as current. Retained source/driver-state/preservation manifests include:

- Original corrupt1.020958ms, VOIDINPUT64/1ec exact-chain2 with complete1.027125ms Validate charge, and C30/C31/C32 same-wake failures. C31 showed407917ns before any poll; C32 showed401125ns/51polls, not64 exhaustion. C33 was **UNRUN**; fresh-RI-before-STOP source defect was corrected before execution. C34's pre-rebase real narrow witness and subsequent INPUT65 success remain historical.
- INPUT65 builder's substantive success but derived member maximum1 prompted C35. INPUT66 external positive/corrupt success did not turn its old pause103.627ms failure into a pass. Source-order1507 disposition has no invented numeric ACK timestamp.
- fd28 GUI2s durable SessionClosed absence; the later diagnostic temporary BASE literal contaminated a shared Cargo test image. That was a driver-introduced diagnostic cache defect, not a demonstrated Q2/QoS cause. Original selected0 with invalid runtime SourceRoot was not classification evidence. Exact own outputs were preserved/rebuilt for correct root; no GUI source change or ignore was applied. GUI original cause remains unestablished.
- C36/C37 pre-stimulus cause remains unproven. C38 saved direct CONT->RI-return<=50ms/firstcounter evidence. C39 USER-mutant physical stimulus was confounded by strict SYSTEM rejection before CONT: intended USER assertion was not reached. Old106tick failure remains failed; the conversion replay is supplied API evidence, not a new physical reproduction.
- C40/C42 timing cutoffs remain unproven. C42's first112-case default0 followed by wrapper2 was a stale111 regex; only count/error text changed. C42v2 later hit410833ns/outer45s timeout, controller15391 owner-loss; settlement **UNPROVEN**, later helper absence is not owned reap. Preservation `0a68bcfe12796239a9ca0f57ac63bd4802ae5a243aacabd530b3df6922c3093d` retains it.
- C43 removed per-callback diagnostic clock queries and labels supplied-derived CPU/unknown wall honestly; no measured overhead amount or unique ACK-delay cause is claimed. Its fullC0 and later exactv7RED416208ns are distinct. v7 log `3eeb2b39cd0944468c83d184f510bebb2cb31414e3ab49766b0092a36d4c1ba5`; preservation `b40afa71d68682dc042395a8616b35ef8044d4b5249293abbfadf245bd4be35f`. Fixture wait4/status9/group0 was cleanup, not product GS1/full payment.
- C44 narrow actual wait1.128ms/ACK1.228ms was clarified by1551. C44 full2 was the8192-byte reader missing a present marker at8870 in13507bytes; preservation `e411626958ab36053beb473ebda68ef86b537f5433f38a61ac9af56319f5c255`. C45 fixed that bounded fixture reader. Earlier standaloneC45 full0 (`ccffef5e3c155675dc5af2064dfe8ae5153faf706170f170beee0d93024fdc7e`) measured wait1.130ms/ACK1.174ms and incidental3375ns SYSTEM. Those figures are not the current v8 values1.127/1.138ms and7250ns.

## Deferrals, resource custody and completion

There are **28 ignored attributes:22QoS +5qualification +1pre-existing manual benchmark**, all **nonpasses**. Exactly one Task-authorized QoS attribute was added, to `real_worker_reuses_failure_retry_and_denial_fingerprints`; body/assertions/caps unchanged. No GUI known-defect ignore or local timing lane was added. Source inventory `current-deferral-source-inventory-final-v2.json` SHA256 `88e36dfbd806ae93346c128e035440b3dca7814a7851a157e7e50eeb0749ed68` preserves exact names/reasons (its original source cut remains historical). The retained ignored names are:

**QoS:**

- `src/dream/resolve.rs::prepared_qualification_absent_and_forged_claims_refuse_before_native_birth`
- `src/dream/resolve.rs::foreground_pause_and_cancel_reach_real_group_while_proof_before_waits`
- `src/dream/resolve.rs::current_group_rejects_old_hooks_after_distinct_real_group_settled`
- `src/dream/resolve.rs::descendant_cleanup_panic_retains_its_actual_launch_and_fault_requirements`
- `src/dream/resolve.rs::descendant_before_panic_retains_its_actual_launch_and_fault_requirements`
- `src/dream/resolve.rs::admitted_public_key_uses_exact_stdin_after_physical_file_deleted`
- `src/dream/resolve.rs::owned_stdin_streams_multiple_chunks_and_closes_eof`
- `src/dream/resolve.rs::backpressured_stdin_does_not_block_foreground_or_cancel`
- `src/dream/resolve.rs::spent_preparation_cancel_retains_disposal_and_actual_settlement`
- `tests/dream_c1.rs::source_blob_sequence_version_and_custody_controls`
- `tests/dream_c1.rs::real_worker_reuses_failure_retry_and_denial_fingerprints`
- `tests/dream_c1.rs::actual_store_fork_is_explicitly_unsupported_and_parent_rows_never_pass`
- `tests/dream_c1.rs::paused_owner_does_not_delay_another_process_and_owned_kill_never_completes`
- `tests/dream_c1.rs::actual_over_limit_ir_corpus_is_rejected_before_reading`
- `tests/dream_c1.rs::four_children_keep_independent_controls_and_individual_memory_shares`
- `tests/dream_c1.rs::near_limit_rejected_envelope_drains_before_process_done_and_join`
- `tests/dream_read.rs::read_bootstrap_held_birth_has_zero_admission_then_paid_hello_start_and_actual_reap`
- `tests/dream_read.rs::read_bootstrap_probe_null_context_permanently_refuses_matching_start_before_source`
- `tests/dream_read.rs::no_candidates_protocol_pause_cancel_and_descriptor_custody_are_separate`
- `tests/dream_read.rs::selected_multi_mib_four_jobs_keep_control_and_measured_storage_limits`
- `tests/dream_read.rs::selected_dense_owner_and_total_input_overflow_refuse_before_publication`
- `tests/dream_read.rs::stdout_backpressure_and_activity_flood_leave_cancel_independent`

**Qualification:**

- `tests/dream_read.rs::prepared_tool_capability_probe_has_stopped_start_and_actual_git_gpg_rollup`
- `tests/dream_read.rs::qualifying_four_jobs_complete_real_proofs_with_foreground_debt_memory_and_reap`
- `tests/dream_read.rs::qualifying_helper_foreground_cancel_settles_actual_descendant_before_done`
- `tests/dream_read.rs::helper_provision_capability_and_executable_guards_refuse_without_path_fallback`
- `tests/dream_read.rs::signed_publication_exact_bundle_survives_offline_sources_and_refuses_tamper`

**Manual benchmark:**

- `src/dream/resolve.rs::hashbrown_record_cache_bench`

One canonical build tree was used. Only own cold build outputs were transparently compressed with lsof/SHA/stat/mode/time/inode/flag journals; actual reclaimed compressed bytes were3,121,901,568 (the earlier native capacity message's50,000-byte arithmetic typo is corrected by retained measurements). Native1556 records capacity reclaimed by its actual owner; this seat did not clean peers. Current archive publication completed0 above the8GiB reserve.

After final exact0/Land0/INPUT67, fresh Taska8e remained unchanged. Authorized own completed-build cleanup exited0: `owned-completed-build-output-cleanup-result-input67-v4.json` SHA256 `1ea31e6b976c86ebe48ef75f5ad05c96e9fb7d11973b46558bcf7f3299bc1131`, full plan hash `60129718267e66ad2ba8f2eee4223334ef51d3bcea422cd7ed71798e426d7daa`, command log `f357caebc0a0b36d9d7642e90827cd7c1e691de4e412aafd6117e51544401039`. Two lsof snapshots had empty stdout/stderr (exit1/no open handles). It removed23,833 journaled own regenerable compiled entries, retained pinned tools and the complete current source-bound proof cache with5086 retained entry stamps unchanged, preserved all immutable Inputs/profiles/logs/green graph archives, and left **22,536,441,856 bytes free** (>8GiB). Git remained clean at signed1f1b. No peer cleanup, broad kill, stash/reset, force-push, release, install or credential action occurred.

`runtime_approved=false`, `whole_command_lifetime_proof=false`, real installed Git/GPG qualification and INPUT67 whole-private30s loaded margin remain **UNPROVEN**. Source-known actual positives, supplied API negatives, finite formal evidence and immutable C preparation are distinct domains. No current candidate/second builder was launched. Final signed code, exact command0, canonical landing0, remote containment0, current formal/profile/observer retention and sole INPUT67 handoff are complete; the accepting authority independently verifies this report.
