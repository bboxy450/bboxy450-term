---
type: report
title: Landed Dream live-sample ESRCH race correction and INPUT61
task: '[[tasks/2026-11-08-dream-q2-live-sample-esrch-race]]'
author: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
created: 2026-11-08T09:42:28.187Z
commits:
- f6dffea0ff8038240de796e237362577e58fe843
verify:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
id: 01M6X8KBBMXS35G877F7V7XEEH
updated: 2026-11-08T09:42:28.187Z
number: 93377961366349
created_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
updated_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  kind: seat
  public_key: 3lIiKAyaYQbqAYQ3xsrS+NTGLp9s//BjqCA4HkK5nF0=
  process_generation: '1'
provenance:
  digest: sha256:736d005b5df5a5d1caaa40ad53d694cac9173152b89ca38efa647fef3f4dc678
  frontmatter_digest: sha256:eb5cbc375d8be84aa224caf4a0d6ae2a177b9a36661a548c4d7e4b557151e19a
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  sig: PkI9UKL1SpNg9Uj1f7WgHfw20E9C5gwLslWXdb4DF7rQbyULnS3F7sauALhOCFiPZTR/Q45oTr7NvrvHmk4cBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150613
  event:
    id: 01M6X8KC69JTVC77ERVMR9HP43
    sequence: 150613
    hash: 362241fc5dca9fd9d33f7f5e70f9b4b57638529f82db3edc651ec9477a409b6a
---
Landed the scoped live-sample ESRCH correction at signed commit `f6dffea0ff8038240de796e237362577e58fe843` in `bboxy450/bboxy450-harness` main. Fresh fetch and ancestor check returned0; origin/main and the clean worktree HEAD both equal that full SHA. Signature is G with Boxy key `9B07E849480C68328CA20684844ECC0587DE7AB0`, author/committer Boxy <bboxy450@gmail.com>.

All artifact names below are relative to **E = `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-live-sample-esrch-01M6RZZXSHZT3KHBK1QC7K3XB3`** unless an absolute path is given. These are retained local evidence, not independently transferable runtime qualification. This is the report for NEW Task66347 only. Previously accepted Q2 and process-create tasks/reports/INPUT60 remain frozen and preserved.

## Acceptance evidence

- `tests/fixtures/dream-read/proof-helper-probe.c:107` and `:114` preserve failed TASKALLINFO errno across diagnostics. Lines121 and128–134 assign EIO/EOVERFLOW to later clock/conversion/identity failures following a successful read, so stale ESRCH cannot become exit evidence.
- `proof-helper-probe.c:2623` requires retained original own-unreaped custody and exact PID/birth/start/FULL audit token matching exactly one CURRENT retained row. The shared rescue at2642–2650 accepts only ESRCH and then calls the UNCHANGED existing same-incarnation terminal proof. Both first (`:2662`) and post-RI closing (`:2692`) reads use it. A positive qualified SZOMB or existing RI-exit observation returns terminal1; live, foreign, mismatched, unowned, reaped, unknown or non-ESRCH observations keep coverage refusal. RI errors at2683 and successful closing lineage/QoS at2695–2696 remain strict. No RI/audit/other API retry or absence-as-exit inference was added.
- `private-engine-controls.c:1091` supplies real owned first-read and closing-read exit races (modes0/6→0), birth/token mismatches, EIO, successful native read plus stale-ESRCH clock failure, and ESRCH while still live (modes1..5→81). Actual native results remain separate from deterministic error/identity/clock stimuli. All seven settle by actual original ownwait4, group-gone proof and full real cost/debt. Five causal mutants remove recheck, ownership, errno discrimination, actual-exit proof or closing rescue and fail the intended assertion. They are process/causal witnesses under source-known fixture premises, not a real candidate or general scheduler theorem.
- The fresh canonical C suite inside `make test-dream-mcp` passed: **79 baselines,48 source mutants,50 strict compiles,110 commands**. Receipt `canonical/private-engine-controls-igmml8a2/run.json`, SHA256 `bf043efc833c01a5dd38b050da537a74fbf056d65ddd348ca005c64dd13c3d2a`, records actual commands/exits/raw ledgers and immutable C3 snapshots; current_sources_unchanged=true. Registry, strict tree peel/B bans, prepared-spawn250ms/other-native25ms, CPU2ms/inclusive3ms/application1ms, ordinary1ms/fresh9x, helper250msCPU/job3sCPU/30s/16MiB and custody/settlement law remain unchanged.
- Finite source adds the disjoint9-case same-owner live-sample projection under the existing TotalBudget family. The owned-state abstraction covers exact original/member/incarnation/current-row custody at either read site. Qualified SZOMB and RIexit positives, live/foreign/token/unowned/reaped/unknown/non-ESRCH negatives are explicit. New broken control `BrokenHarnessQ2HelperTotalBudgetMissingTerminalRecheck` violates **LiveSampleDispositionMatchesSameOriginalTerminalProof**, not TypeOK. Source cut `formal/freeze-v2.json`, SHA256 `63e715d1d6c0fd343a90183f602af1f9785b3f2ba32f7efe8dc63d5501c141f0`; original18 controls/83 assertions remain (now19 controls/88 assertions). No fairness, cap, tools, heap, worker or timeout change.
- Current full native/pinned-Java parity is **434 models /93 contracts /1 crate**, **3,374,874 states /11,363,095 labelled edges**, with every full graph and named safety/deadlock/liveness verdict equal. Cap100000 and pinned TLC/JRE stay unchanged. Exact required four-command verification was run ONCE on this signed final head, actual0, from `2026-11-08T08:53:29.917637+00:00` to `2026-11-08T09:27:49.162260+00:00`. No cached reference run is relabelled as fresh execution.

## Source and retained bindings

Only eight allowed code paths changed: C probe/controls/driver, `tests/formal/dream_read.rs`, `tests/formal/models.rs`, and `verification/{contracts,corpus,coverage}.toml`. Rust product sources, top-level integration/ignore sources, `tests/formal/dream_c1.rs`, Makefile and scripts/land.py are unchanged. Exact source lengths/hashes and signed cut are retained in `signed-source-cut-result-v1.json`, SHA256 `43686f67136b7462aa014d12280a4ad51d09874b3d1386648c5e65f45ca6632b`.

| Current C input | Length | SHA256 |
|---|---:|---|
| proof-helper-probe.c | 328721 | `9bacbb0483ab328798af2bc98225bdf8f50f930e6d2ecda6779cec3afec75042` |
| private-engine-controls.c | 132148 | `728cdfa76f49c07a4d984079e8c378d6c4263116ef1eded81455c812856bb104` |
| check-private-engine-controls.py | 43362 | `440c20aa33270cb195697410eafdcacddd2920e40be5acbee2948834bd9fa9bb` |

`C20-source-freeze.json` SHA256 `595bfd0dc895cc2e0efcf852a68a9ac0fc5062dfbdc07c7be3fab98c7fac0b19` binds product/controls; `C21-driver-mutant-fix-freeze.json` SHA256 `3797641a069f69849b8304c17d062429b03bc922678d7d7b331f28dfbca93434` records the exact driver-only correction. C19 first-only source was preserved and never consumed as final.

Actual C1/READ profile metadata was copied byte-for-byte immediately after the exact chain, with independent readonly worker inodes. `retained-f6dffea0-actual-profiles-exact-four-chain/preservation.json` SHA256 `d0e7087fe6e97e869eee8e4edbec7823c45d439b3f749bee1950bbe4f05c5c53` binds current worker SHA256 `a5750d7714f25f5c596fbcbf3a0802bea1c73543e1c72f3804edb1a90b27c970` and READ39 inputs. No old profile metadata was reconstructed or promoted. The canonical scripts/land.py preservation helper retained all five observer JSONs on their same inodes after actual MCP0; detail `observer-preservation-detail-v1.json` SHA256 `ede516cd4d913c7b5fd190e8ed88be1d0be104f4d3e2578e90041f17d73833e0`.

Formal archive: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/verification/evidence/6e455ce97ad6292c1310d16ef3fa115c8f8a5735817ac00cb30a8e66d9e542e9`, evidence SHA256 `6e455ce97ad6292c1310d16ef3fa115c8f8a5735817ac00cb30a8e66d9e542e9`. Every one of4342 raw files (7,429,799,817 bytes) was hashed; original publisher modes/inodes were journaled before sealing files0400/directories0500. `current-formal-all-raw-files-v1.json` SHA256 `ba3cab1de50492ec349b37ae8a968ee6abb7797f993638d46fd7356d66cf6619` records all bytes and the distinct-inode/nlink1 `cp -cR` working clone at `canonical/harness-landing/formal`, then independently validated by `make check-formal-evidence`0. The immutable1602 archive/INPUT60/profiles were not modified. Before new export, only owned cold1602 regenerable working-cache graphs were transparently compressed after lsof/stat/SHA journal;358 files retained bytes/paths/modes/mtime while allocated7,106,416,640→576,761,856 bytes (`compression-cold-1602-regenerable-formal-v1-{plan.json,journal.ndjson,result.json}`). No peer cleanup or evidence deletion.

## Actual commands and exits

Runner JSONs preserve exact argv/head/source-before/source-after, actual exit and log hash. All current rows below record source_unchanged=true. Cargo/Make used unset FORMAL_JOBS, RUSTC_WRAPPER=/usr/bin/nice, empty RUSTC_WORKSPACE_WRAPPER and CARGO_INCREMENTAL=0; no resource/assert/ignore override.

| Command or gate | Actual exit | Retained log |
|---|---:|---|
| Rust strict Clippy, library | 0 | `esrch-strict-lib-v1.log` SHA256 `c56fe1ce0417f7b1fc5a2315a90eeee18743471adb2d69b3e9cb09f5ec0267c8` |
| Rust strict Clippy, all targets | 0 | `esrch-strict-all-targets-v1.log` SHA256 `fe9ee863673871389a0bdef2c54691fb531c1ae1f00a5b0f601cf1ee26a81722` |
| Rust strict Clippy, fixtures feature | 0 | `esrch-strict-fixtures-v1.log` SHA256 `56c0fe35d624cb7ce152a43bfae666f5866f49f473b2870c33df29838d14ca8d` |
| Affected total-budget strong/complete-observer tests (3 passed) | 0 | `esrch-total-budget-three-tests-v1.log` SHA256 `a44a8cba7e891775b0c548272f75afa6d63e2dc11f2b0101b13f45b2d251ac5c` |
| Exact required four-command string, ONCE on final signed head | 0 | `esrch-exact-four-chain-v1.log` SHA256 `2e07a0bbb9f6166b96be871f807596bed66948357edff62d1621e2a2f54be104` |
| Canonical check-formal-evidence on independently cloned current archive | 0 | `esrch-formal-clone-validator-v1.log` SHA256 `bcf2babeada4726b79a16aeaca53ece21271b4f6a0ce16a73e513c0c471b1a92` |
| Canonical prepare-dream-read-c-probe for INPUT61 | 0 | `esrch-preparation-input61-v1.log` SHA256 `a06a7b4f8a88fe1f913cab33eb0ca53a6db147a4d58c0e45c8a7dcbcc0d237a8` |
| Independent immutable preparation audit,997 checks/325 headers | 0 | `esrch-independent-audit-input61-v1.log` SHA256 `e56c99603414e664ad06d01740c07239bf142780055ccb715486be74f1feb5a3` |
| Literal canonical make land | 0 | `esrch-canonical-land-v1.log` SHA256 `1a151ba0c9914ff242f0913b8ddea90e0655c3dde4c48413ea5233ce2e53707b` |
| Fresh fetch, origin/main contains HEAD, approved signature and clean tree | 0 | `esrch-remote-verification-v1.log` SHA256 `9ded7fdea31df06a301462aa44b572e79e7c29b2e5b731a48883902578f7f5f0` |

The verification row's command is exactly `make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build`. Literal canonical landing was `make land TARGET_DIR=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-live-sample-esrch-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical` under the same environment, actual0. It performed strict compilation, selected library nextest (194 passed/9 skipped), current-source formal validation and push. Because no top-level integration source changed, canonical land did not repeat the already successful C1/MCP prefix; the required literal four-chain was separately executed in full above. Land receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-live-sample-esrch-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical/harness-land-receipt.json`, SHA256 `109115b8690625f0cfa74e34b755e4a3c57327567760cfbce2b9910e824b51b2`. Fresh remote verification detail SHA256 `a7b9339329197143008a25d1a3b5869d9e24e1f5d71d2576da256bf45a58a398`.

## INPUT61 handoff

ONE required tuple was stored in the delegation thread: native message sequence1434/id `01M6X8FZM6B3JKH8QVA0JK2N16`, replying to original Delegate `35H8Q21J0K7SYG9D2PJTH2QE3P`. Storage is not model receipt; no ACK or builder run is claimed.

Full tuple `INPUT61-final-tuple-v1.json`, SHA256 `ff1c31cf724ecae33c874cd65ad25e869f3f6843a0d8eb4ca5814900d8689106`, contains all ten exact source input paths/lengths/SHA256/input commits, compiler/version/flags/SDK/platform/linker/header closure, signed source4 snapshot domain, receipt and audit. Source and preparation commit are `f6dffea0ff8038240de796e237362577e58fe843`. Receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-live-sample-esrch-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input61/dream-read-c-preparation/compile-wcb1m3yz/preparation-eb5f1696cba83e117d354c040b50f0950d5f0fd7847f398255d0a699f73dd987.json`, SHA256 `eb5f1696cba83e117d354c040b50f0950d5f0fd7847f398255d0a699f73dd987`; immutable251064-byte probe SHA256 `c50c2ae7d6f8aad7f3c0c99214e0683ea7b78cdf686f6acba8ca31fe340a2009`. Independent audit `input61-independent-audit-detail-v1.json`, SHA256 `533b16290b9296355c7d2017631eb4fe4a2046263b77dfdd54a20d67e2243790`: actual0,997 checks/325 headers,0700 roots/0400 files/0500 executable/nlink1, lexical/resolved pthread/spawn and signatures checked. Compiler SHA256 `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`. Source-domain SHA256 `8e6d9207723751cfb936587ecac7f06673321acbca05be739613e0736c2e4ee8` is the exact read-set snapshot; that snapshot alone is not a compiled-product-currentness theorem. Actual current product/profile evidence is separately retained above.

**Terminal fields changed=false; ledger fields changed=false.** Existing exact fields `process_create_wall_excess_ns:u64` and `process_create_wall_excess_count:u64`, and their known/scope annotations, remain unchanged. Terminal values are checked invocation sums; ledger values are per-original-prepared-spawn snapshots (do not sum repeated snapshots). No new field or wire shape.

## Preserved failures and limits

The C20 narrow run `esrch-c20-causal-v1` remains FAILED1, logSHA256 `d15d0fee336ffcbfcb4a865c85ef359e71dc691abbf1801fe00ab08b37912a9a`. Its actual11-command prefix passed all seven real cases and no-recheck causal mutant, then strict compilation correctly rejected the intentionally ownership-erased mutant's unused pid parameter. The sole C21 driver fix added `(void)pid` to that deliberately broken replacement, with exact reverse-byte proof. The missing-tail run actual0 (9 commands), logSHA256 `3d12eaddc07408bcbb3273865979b710697ff705c4fa6b3953169318d0d90714`; no old failure was reclassified. The fresh full canonical current110-command receipt above independently completed0. An initial cache-seed directory conflict was recorded without replacing the formal owner's evidence directory; the corrected canonical working lane is separately journaled.

Original builder run `01M6X25J5JYW6CS88P06H0QYNF` remains FAILED81 as recorded in Task66347 (TASKALLINFO errno3; only partial ledger digest931699ad...4c2 was supplied). No peer raw path was read or full digest invented. **runtime_approved=false; real builder/tool qualification and whole-private16-form30s fit remain UNPROVEN.** Existing26 Rust deferrals are unchanged/nonpasses:21 QoS and5 qualification, exact names retained at `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-process-create-wall-ceiling-01M6RZZXSHZT3KHBK1QC7K3XB3/current-historical-deferred-tests-v1.json`, SHA256 `8d21bc964e7f15c33d58f4547a81528e9e0f375db980621c6b903258e3e06607`. No new ignore/waiver was added. Historical retired-law physical oversize negative remains separately deferred/unproven, never a pass.

No performance expansion was made. The prior source-derived conditional currentness floor remains33*(G+7)*9ms before other work (G16:6.831s;G32:11.583s;G64:21.087s;G256:78.111s); actual G/dynamic/native/helper/wake costs are not a full30s certification. All actual commands were unattended, without operator/GUI/credentials or genuine candidate provisioning. Previous accepted reports, immutable tuples, raw failures and limited implementation-only proofs remain intact.
