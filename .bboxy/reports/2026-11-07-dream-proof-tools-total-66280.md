---
type: report
title: 'Dream proof tools: exact 66280 tuple, sole positive deadline refusal'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W0XSKRWPBS72T4MJZ0WGT7
created: 2026-11-07T22:19:48.526Z
commits:
- 12d72305bf74abbadd46ae27abb91b508f40cd89
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M6W1HBB9JMQBFGX7KYG8ASVN
updated: 2026-11-07T22:19:48.526Z
number: 93377961366298
created_by: seat:01M6W0XSKRWPBS72T4MJZ0WGT7
updated_by: seat:01M6W0XSKRWPBS72T4MJZ0WGT7
authorship:
  signer: seat:01M6W0XSKRWPBS72T4MJZ0WGT7
  kind: seat
  public_key: Up2oF6lfNSKywkpPjuuRnZ86KNFZBUeD/ZDX6L6EakU=
  process_generation: '1'
provenance:
  digest: sha256:9bef15e425e1d7159c14e3d723eb2aaec0f02b133a8f54c274976117e9a1e0fb
  frontmatter_digest: sha256:ce5ab37fa6e53712130a8833269f8585e5e0b25c42e69cd9d55e8bc43e081356
  signer: seat:01M6W0XSKRWPBS72T4MJZ0WGT7
  sig: EV0RDLDjfvPLeLgped3fdoL5rIqXfHftM2pKAtWkHvRSZ2bkf9Pdb1JYi18n/cgcuMcERSfdOArqkXGKmjJXDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150444
  event:
    id: 01M6W1HCJVVH4572SX2FSB61DY
    sequence: 150444
    hash: 5f47b49f83efd0f935a708f0ba95afa2fb6ed3029ae4308c2ba91c33f1d9c1d6
---
## Outcome and stop

BLOCKED on the sole authorized positive, not qualified or landed. Decision66280's exact frozen source557c8b47 / Ceca43c1b / executableac4df370 / preparationec4c2eee was ingested without rebuilding or modifying Q2. The private controller actually exited81 and was reaped by its original parent. It emitted ZERO stdout bytes and no output/result.json. The canonical builder correctly refused the missing result, and the exact task verification exited2.

No timing retry, remaining component case, final formal/license gate, qualification, receipt, admission, push, merge, installation or publication followed this refusal. runtime_approved=false. The needed Oracle decision is disposition of the deadline/incomplete-result evidence and a causally changed authorized Q2 input/scope; an unchanged-input retry is not authorized.

## Changes and signed state

Signed code commit: 12d72305bf74abbadd46ae27abb91b508f40cd89, Good Ed25519 signature by Boxy (bboxy450), fingerprint9B07E849480C68328CA20684844ECC0587DE7AB0. Branch feat/dream-proof-tools; git status --short is empty. The commit has Report: [[reports/2026-11-07-dream-proof-tools-total-66280.md]]. All predecessor commits and private artifacts remain.

- scripts/dream-proof-tools.py:52 — pins the task-selected retained root, source, executable, receipt, all identities, preparation projections, linker inputs and Mach-O facts. Existing strict no-follow/custody/header/compiler/snapshot/currentness checks remain.
- tests/fixtures/dream-proof-tools/sources.lock.json:1 — changes only canonical_q2 to the exact frozen C length309104/hash; the seven source/auth/patch pins remain.
- scripts/dream-proof-tools.py:2953,2983 — checks the existing unapproved conditional result and total authoritative wait4 A_ns<=3s, retaining ordinary/controller/cleanup1ms, native2msCPU/25mswall, inclusive3ms, RSS, settlement, attempts and debt gates. Retired Q/W may be null. The exact Q2 producer remains responsible for each250ms command cap, observed QoS/live sampling, roll-up and terminal wait4 verdict.
- scripts/dream-proof-tools.py:3115 — removes retired late_stop from dispatched components. Other canonical cases remain sequential and stop on the first refusal.
- scripts/dream-proof-tools.py:3159 — orders canonical formal/license gates after all components and before qualification/receipt. This new success path was NOT reached; its captured command logs would remain under the original run. Qualification checks currently enumerate component results; final-gate records should also be attached to a future sealed candidate if that path is reached.
- scripts/dream-proof-tools.py:4253 — inert boundary regression accepts total CPU exactly3s with null retired metrics and rejects unknown/bool/negative/over-cap totals.
- tests/formal/dream_proof_tools.rs:15 — updates the source mapping: CanonicalWitness abstracts components plus final formal/license gates. This cut changes no Q2 controller/model; Q2HelperTotalBudget remains the Q2 owner's contract.

## Commands and actual exits

| Command | Actual result |
|---|---|
| python3 -I scripts/dream-proof-tools.py unit | exit0,59 tests; current final source, unit-current.log |
| offline ingest_q2_input of exactly the selected tuple | exit0; ingestion.json |
| git -C Q2 verify-commit557c8b47a474373acd42e759a7f7ac3efc72fc39 | exit0, Good Boxy signature; q2-signature.log |
| git merge-base --is-ancestor2ce4359d HEAD, and the same check against selected Q2 commit | both exit0; source-check-exits.json |
| make fetch-dream-proof-tools | exit0; all seven cached archives reauthenticated with fresh owned public-only home |
| make build-dream-proof-tools | make exit2 / Python exit1; all60 recorded build commands000–059 exited0, then sole positive actual controller exit81 and empty-result refusal |
| make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses | exit2; later three targets NOT RUN because build failed |
| git diff --check | exit0; worktree clean |
| historical ledger hash comparison | exit0; all6 prior ledgers unchanged |

No current formal parity, integration/smoke completion, license success or source-bound final formal receipt is claimed for this cut. Earlier recorded successes remain historical. Initial df showed85,356,028KiB available; build used the existing two niced jobs, offline authenticated caches and no operator step.

## Exact artifacts and trust boundary

Evidence directory: /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/total-66280-20261107.

Run: target/dream-proof-tools/runs/01M6W168YT0EFZRE5EHTE2MYTW.
Unapproved candidate prefix: target/dream-proof-tools/candidates/01M6W168YT0EFZRE5EHTE2MYTW. Its original intent/device/inode, build ID/incarnation, retained source tree/config/link maps/source review,60 command records and all failures are preserved. terminal.json says unsupported, reason canonical Q2 result/LF byte bound, effect_unknown=false. No qualification.json or receipt.candidate.json exists.

Frozen source/preparation signed commit557c8b47a474373acd42e759a7f7ac3efc72fc39:
- C SHA256eca43c1bbf4a165119ead5c7da2cc1c24ff25914c00ef25c172096f8b4595b0b.
- Probe SHA256ac4df370e9e108b9d7c65cf83b1a8a5ecfde6f15650fc172a31eb6909b3c7cdc.
- Receipt SHA256ec4c2eee90bc74d0cae67991b145e4fc99c729898cc56e1cddb1964490a48326.
- Proposal snapshot source-domain5837e5abf3e59a5a975e25bbce5a109c43ba8997bb92da53e4ae1048f1dbcc15 is a byte snapshot, not compiled-product currentness.

Actual candidate Git SHA256b09811cd3fe077136ed6bb617c0601da51fe6011fb5d8cbd02b3c7f7d6f68c77; GPG SHA256d782f6199f5f368fa08eda18764ff4153b617ac6ba3228824652cb2dc719d662.
Actual clang image SHA2561590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9, canonical targetaarch64-apple-darwin, rawarm64-apple-darwin27.0.0; SDK27.0. Platform Darwin27.0.0/build26A434/aarch64/rawarm64/cachec284a291b68b3d0da1a03f866f87efdc, premise trusted_platform_boundary/1.

Fresh Build.auth records contain all seven actual exit0 authentications. Git primary96E07AF25771955980DAD10020D04E5A713660A7; remaining six primary6DAA6E64A76D2840571B4902528897B826403ADA. Independent pinned developer GPG bootstrap, compiler/linker/SDK fidelity, selected source reachability and native platform remain conditional assumptions; nothing here proves arbitrary-native lifetime. All patches/notices/licenses and eight review findings remain bound by the generated basis. Private pretest basis47112B, descriptor3439B, lock7176B; raw result0B is missing required proof, never a successful fit/qualification.

## Sole positive: raw refusal and settlement evidence

All paths below are relative to the run's components/positive directory.

- wait4.json: actual controller PID45375, wait_status20736, exit81, user47,172,000ns, system164,751,999ns, ru_maxrss4,259,840B, actual_wait4=true. SHA256d8d643db2ba65362eb5a7377afcb634b90968a00340afd2fe2c4f0bc0ed9dd25.
- stdout:0B, SHA256e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855; output/result.json absent. Empty raw stdout is retained unchanged in retained.json and the candidate blob store.
- output/ledger.ndjson:11 lines,5140B, SHA256c1a78d9d4c52b176e0d385b793f5625e0a9bfad61c55789f38b84cb74f42202c.
- Ledger2: helper PID46959/sequence1, observed priority4, authoritative rounded wait4 CPU21,965,000ns, terminal RSS3,850,240B, maximum RI gap25,949,000ns, actual_own_wait4=1/group_gone=1/helper_cost_paid=1.
- Ledger7: helper PID47457/sequence2, observed priority4, rounded wait4 CPU20,967,000ns, cumulative job42,932,000ns, terminal RSS4,259,840B, maximum RI gap23,364,000ns, actual_own_wait4=1/group_gone=1/helper_cost_paid=1.
- Both command totals are under250ms and the observed aggregate under3s. Neither row proves completion of the remaining command/proof/child-roll-up obligations. The first line is git_sig_prepare only; GS1/GS2 and a real Git→GPG child were not reached.
- stderr SHA256898575447c242bfd48e9fcdca86ef9332e5e80470107ad79b35bd874ede59e33 retains deadline18, settlement_unknown1 (private result stdout emission incomplete; no retry), source_changed1 (original descriptor pathname changed), helper_created2/helper_reaped2, result_complete=false, control_verdict=failed, persistence_verdict=unknown, terminal_cost_verdict=failed, stdout_errno60. Final declared debt25,838,388,000ns was paid; ordinary_max417,042ns.
- The earlier deadline diagnostic row says count15, while the final histogram says18. Both bytes are preserved; diagnostics_complete=true is a producer claim, not proof that the missing three detail counts were emitted.
- source_changed is the producer's recorded label. The host performed no descriptor rewrite; this evidence does not establish a physical pathname mutation or its cause. Frozen C:820–822 emits that label whenever private_open_walk fails.
- owned-pending.json records the parent's deadline-time pending observation; later wait4.json resolves the controller to actual exit81. No timeout signal, replacement dispatch or lost parent handle occurred.

## Evidence inventory and remaining criteria

handoff.json SHA25661e1c214e342bf7222e827c3972bd3994d650b1f9132b3db479afea05a637f97 contains the complete selected artifact hashes, original operation/candidate identities, compiler/platform/tool/auth/patch projections, all60 real command exits, two unaltered total-budget rows, raw output/log references and six historical hash comparisons.
verify.log SHA25685ec52189a2227677bf72a34e55030d3dd5a71f0c914e007e7a3632e64444407; verify-exit.json SHA256e63e95c3b2c0d6370945ce92ea6709bb0dde4d77a9a1f16487b92960f68d8e53.
unit-current.log SHA2567d65cf5412d9fb25053696d811cccbf422dc62c5148c71c74e284e82fb41040f; ingestion.json SHA256e2022c3cbf3dcd1cba3955d3dd977f9692907d8e1302cddf52f889999f7ed1a7.

Exact authority consumption, source authentication, offline build/config/closure preparation and bounded-result rejection have evidence above. Required full positive/corrupt proof, pause/cancel/ABA/clock/debt/reap controls, complete terminal persistence, final formal/license gates and qualification/receipt remain unmet. B startup barrier and real-tool qualified roll-up remain unproven. Q2 full lesson/exporter/four-job/UI/control gates belong to Q2 and are not claimed. No legacy failure was reclassified or erased.

Blocker DM01M6W1DJPE26VXCWQR73FXZAV6 carries the ledger/hash and exact decision needed; durable storage alone does not prove model receipt.