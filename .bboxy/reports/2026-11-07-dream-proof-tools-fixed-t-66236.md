---
type: report
title: 'Fixed-T Q2 freeze: measured positive quantum refusal, conservation passes'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6TPMTXM8MQCQXMCVBW60YGV
created: 2026-11-07T13:08:39.940Z
commits:
- ff2e650b694f33908dd34c18415cf0aee9041e41
- 59a301e4c3a555e22e2ba6a9f7ca754b7ade3a5a
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
supersedes: '[[reports/2026-11-07-dream-proof-tools-measured-66236]]'
id: 01M6V205M0NETM19XQXTH9ZR48
updated: 2026-11-07T13:08:39.940Z
number: 93377961366270
created_by: seat:01M6TPMTXM8MQCQXMCVBW60YGV
updated_by: seat:01M6TPMTXM8MQCQXMCVBW60YGV
authorship:
  signer: seat:01M6TPMTXM8MQCQXMCVBW60YGV
  kind: seat
  public_key: bBPoi7SjWcPpxvEzz3+5Ck7p1MHLoPov5NPy/GmicZM=
  process_generation: '1'
provenance:
  digest: sha256:9956689ab7551c4eed56bffa9dabf748ae4d92e5d400e1992555034a4a56d88c
  frontmatter_digest: sha256:207b28bc0da56a1e11ec1965e4d2df61d4e81d8c839d5d98c17beb380fb17329
  signer: seat:01M6TPMTXM8MQCQXMCVBW60YGV
  sig: NsnvCHTr9Pvh+BT8/u/IJ6J7Y55Iv4xKwCzKf+0gjHHcRVa4e6zb/8eoJiT8C1zL1ZV8qZx3n+PknIwdL8YDDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150339
  event:
    id: 01M6V2070Q119SD9CH979VPQCT
    sequence: 150339
    hash: 64553240e42890bd6cb48913de393be91b10126a9d1b9f971f4e2c29fe4421b0
---
The Oracle's exact fixed-T freeze was ingested and committed, but the sole positive PRIVATE component failed at measured grant 51: A932917 + C245583 = Q1178500 ns exceeds 1 ms. The canonical verification sequence exits 2. No retry, later component, final test, qualification, receipt or landing followed the refusal. runtime_approved remains false.

Oracle disposition and a causally changed, explicitly frozen Q2 input or task scope are needed before another native attempt. The Q2 controller is outside this Executor's writing scope. Blocker DM01M6V1SGCYDTM412N98YF17GKC was stored; that alone does not prove model receipt.

This append-only report supersedes [[reports/2026-11-07-dream-proof-tools-measured-66236]] as the current handoff. Every earlier failed run remains failed and retained.

## Source and exact authority

Worktree /Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools. Signed source ff2e650b694f33908dd34c18415cf0aee9041e41 has Good Boxy signature, fingerprint9B07E849480C68328CA20684844ECC0587DE7AB0. It descends from retained a28283160d146f4c215181e466daae00e95bdb4c and d02cdd9fe3406d213699ec82473ef6d9309528b2. Final worktree clean; no reset, stash, cleanup, push, merge, publication or installation.

A separate signed empty metadata checkpoint 59a301e4c3a555e22e2ba6a9f7ca754b7ade3a5a is the final HEAD. It corrects the Report trailer to the lowercase slug required by the native report index after its explicit invalid_report refusal of the uppercase-T path (operation01M6V1YKWXBEQSKFYEPFH9MK1Z). It changes no file, compiled input, original failed effect or verdict and launches no candidate. The actual build remains bound to ff2e650b, not silently relabelled to that metadata checkpoint.

Only two tracked files changed:

- scripts/dream-proof-tools.py:52 — replaces the explicit selected tuple with the fixed-T source/probe/receipt identities, signed commits, 17 receipt bindings, compiler closure and Mach-O facts. The exact root-level C path named by the task is selected.
- scripts/dream-proof-tools.py, _ingest_q2_input — accepts that exact root-level compiler input only when its COMPLETE identity equals the pinned receipt.build.input. Existing signed-snapshot validation, byte equality, actual recorded compiler-input checks, no-follow custody and final rechecks remain. An unrelated same-byte sibling or replaced inode still refuses. The two new inert offline regressions cover these cases; this changes no public DTO, native controller, runtime threshold or lifecycle.
- tests/fixtures/dream-proof-tools/sources.lock.json:1 — changes only canonical_q2 source Ref/hash/length; all seven upstream/source-key/signature/patch pins remain unchanged.

| Frozen input | Exact binding |
|---|---|
| Signed source/preparation commit | bec674fba7a823c36802d1ff3e4380a2ae3e25cb |
| Task-named C, 408276 bytes | 29d55d64120d6ce498316577af5c4c4650dae521d9a66dfb6e50284f6b342646 |
| Supplied probe, 287016 bytes | d2deef94ba6f28236d5c2ef7ade4e757bbe5e615917b81f929f23632a5329ba3 |
| Preparation, 514287 bytes | d3d3cdd123adeb98179c3d9ce76297bef5297aa6a8a0d0618f3f0bd39828b022 |
| Proposal SourceSnapshot byte digest | 7f7edaa9f598709d897d3a4fc2a43ac688acfd6fe183b39fb8dc1b0456f9d84c |

Preparation root F=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/fixed-T-66236-signed-preparation/dream-read-c-preparation/compile-vxqar2c6. Selected C is F/proof-helper-probe.c, exact inode184388642. Its separate signed snapshot copy inode184388693 is also fully checked. Both have the exact bytes above. Probe and receipt paths are those in the task's final freeze, unchanged; no rebuild or substitution.

Full offline ingestion checked 0700 root, 0400 C/receipt and 0500 probe, one-link file identities, signed source/preparation snapshots, all323 lexical header entries/aliases, all352 retained outputs, compiler/linker/SDK/OS facts, actual compiler/link inputs, 17 projection hashes and exact Mach-O facts. Good Boxy source signature and 2ce4359d ancestry both exit0. Four-Rust digest is a source-byte snapshot, not compiled product-currentness or runtime approval.

## Commands and real exits

Evidence root E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/fixed-T-66236-20261107.

| Command | Result |
|---|---|
| python3 -I scripts/dream-proof-tools.py unit | exit0, 58 tests; E/unit.log SHA0cbba1422713ac0ffe69c1268d18087a4e5399be4c58c105f804b18790564bdf |
| selected_q2_input full offline ingestion | exit0, exact task-named root C; E/ingestion.json and selection.json |
| git verify-commit bec674fba7a823c36802d1ff3e4380a2ae3e25cb | exit0, Good Boxy; E/q2-signature.log |
| make fetch-dream-proof-tools | exit0, all7 source signatures freshly authenticated in public-only home |
| make build-dream-proof-tools | exit2, recipe exit1 after actual controller exit81 |
| make test-dream-proof-tools | NOT RUN after stop |
| make test-formal-parity | NOT RUN after stop |
| make test-licenses | NOT RUN after stop |
| git diff --check | exit0 |
| Current source signature, predecessor ancestry and Q2 required ancestry | each exit0, retained E/*signature.log and E/*required-base.log |

The exact task sequence `make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses` was evaluated left-to-right by the unattended stage driver, stopping on the first nonzero exit. Overall2. E/verify-exit.json retains actual exits/timestamps/load; E/verify.log SHA6fd0511f184184222043999f92a8b800dec7c51e961ae8b3376c31f63ac567fd. Free space before build15327186944 bytes, above8GiB. Load8.8706/10.9634/10.8096 →11.1377/11.2744/10.9844; no causal claim that load explains the measured CPU refusal.

All60 builder commands000–059 exited0. Their exact argv/environment/stdout/stderr/exit records are under R/logs, where R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6V1HTRXVYXXW9CK3CWM2CT2. Authentication/extraction, exact guarded postimages, source-supported configure/generated outcomes, two-job builds, actual archives/link maps/compiler/SDK/OS evidence remain retained.

Before the narrow root-compiler-path correction, an initial synthetic56-test run and snapshot-byte ingestion also passed; they are retained as E/preliminary-* and are not represented as the final58-test/current exact-path check. Neither launched any native candidate.

## Sole failed positive and settlement

Original build_id01M6V1HTRXVYXXW9CK3CWM2CT2, incarnation1525435977011500. Prefix P=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6V1HTRXVYXXW9CK3CWM2CT2. Intent SHA c2fdfae224f014ca0c6b8e1104e44d0406de797f4cb6065847eb54fd672bb9c2 remains retained with its original device/inode/effect.

Case directory C=R/components/positive. Descriptor run_id01M6V1QPCBD87VAZKBMGZCTRHF, nonce01M6V1QPCBSZNXJDYMT9EKX69P, input SHAa7c50e6e18a7c812d6e315f843fbe4551ea51956b3f254c2541815f303f7b57e.

- C/output/ledger.ndjson: 56 unaltered lines,116350 bytes, SHAc7a63a73efef4bf8a5cc4bb94013b8ddb75657f232bb3bca49e32f37f32edd5d. This is retained offline raw evidence, not an extra prepared result DTO.
- Ledger53: grant51 measured basis, A932917 + C245583 = Q1178500ns,178500ns over1ms. W1114000ns is below25ms. Closed1, unknown_mask0, ack_mask1, counters identity/conversion/post-ACK masks1. The actual measured charge failed; no wall-proxy reclassification or lucky retry.
- Ledger54: failed unit engine_finish_grant, frozen C:3494. Actual/inclusive controller unit246500ns; supplied_actual245583, supplied_cost1178500, included tail917, final cost1179417. The second diagnostic “ordinary engine unit exceeds one millisecond” is the supplied total-cost gate, NOT a separate controller CPU span above1ms. Stopped units663, stopped_max_cpu221667ns.
- Ledger55: fixed-T conservation PASSED. Actual wait4 upper6265000 ≤ independent_helper_credit6144599 + rounding53834 + N_reaped1×T150000 =6348433ns; excess0. Direct_actual_reaps1, qualified_child_reaps0; the GPG implicit child was not reached.
- Ledger55–56: raw residual118401ns, required/retained cost120401ns, exactly one debit, residual_cost_paid1. T is explicitly not credit. Original helper PID18027 has retained birth/start/pidversion and source original_own_wait4.
- C/stderr SHA9ebe16ed96a7f923ffc7ba4486d96ffec2867c20924c4f81f586307a4f14dbfa: exactly resource_limit count2, first canonical member-window closure refused, last ordinary engine unit exceeds one millisecond. These are the full reported code counts, with no invented middle diagnostic.
- C/stdout:2015 unaltered bytes, SHAc3d88573e491eecb02669ff4d0e612c4b64a3aa15b709d36af42d1e564fcfcd1, byte-equal to C/output/result.json. failed/settled, proof not_reached, attempts1/members1, helper created1/reaped1/group_gone/retired_coverage true. Private runtime_approved=false/conditional/no whole-command theorem/availability not_applicable_private_test are preserved.
- Controller PID16309 was actually reaped by the retained parent's wait4: C/wait4.json SHA15ace01de7bf418f8f49f5773a189ea6076b8ae049a43da1903c5eade10a8cfc, actual_wait4=true, exit81. Helper18027 actual reap is retained in ledger53/56.
- Final terminal receipt says control_verdict=failed, persistence_verdict=persisted and terminal_cost_verdict=passed; final debt25453614753ns charged=paid, retained_units0/overflow0. Passing persistence/terminal cost and conservation does not rescue the failed CPU quantum.

Only the positive directory exists. R/terminal.json records unsupported/effect_unknown=false. No qualification.json or receipt.candidate.json exists. No later component or native attempt followed the failure.

## Acceptance coverage and gaps

| Requirement | Evidence / disposition |
|---|---|
| Exact Root-selected tuple and actual compiler-source path | Signed selection, strict exact compiler identity + signed-snapshot equivalence and full offline ingestion above |
| Seven authenticated upstream sources and exact patches | R/authenticated.json SHA53bab1bee1654a8cac999afcc6d2451ce765b43d0ca2855bb638a892c84e03cc; auth_exit0 for7; source-lock SHA7cd49d8226c1cb5cb5a16128ea554d24a6d5f2c0d2318e932137041f18ef4d9a; unchanged upstream roots/patches |
| Typed complete bounded Basis, config/commands/source review/closure | P/provenance/pretest-basis.json47112B SHAbd325bb3a32424921cad6aaef41c456d81d297434e6cc1e419d5cbf9f4bee03f; auth/generated7, review8, forms16; actual private configs/source/link/compiler artifacts bound |
| Exact source/compiler/platform premise | Recipe SHA0d0872d35600ee96be551446ec43a21a893f9e2653d2eebc2b1cb97123091995; clang21 SHA1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9; SDK27 settings SHA7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb; Darwin27.0.0/build26A434/aarch64/cache UUIDc284a291b68b3d0da1a03f866f87efdc; conditional source/compiler/platform assumptions preserved |
| Real positive/corrupt proof plus remaining causal cases | FAILED at first helper grant51 before proof. Remaining cases and implicit child not reached. Git SHA32f54b0f350be6aeec8324142d06fd2aad65d3d165975efba98efea5f91e44e6; GPG SHA2f71be5e9075cfc0df877ecdff963ca5864d47d3c5c61041954c2e79e4dfb1ce |
| Custody, receipt-last publication and reuse | Existing publication/reuse mechanism unchanged; correctly not reached after refusal. Prefix0700/Basis0600; original incarnation/intent/PIDs/raw bytes retained |
| Current-source formal/Rust integration/license gates | DEFERRED by the explicit stop; no current ff2e650b parity/integration/license success or source-bound formal receipt is claimed. Old d02cdd9f successes remain historical |
| Preserve prior failed effects | All3 original ledger hashes independently rechecked unchanged: 413574d03deaf982b431a7b972b90cf9fb0745a133d55d3160a1d2abb110e01a; 6b6ec65a0e628578f22be29f0d8dcb6ce57624b3c82a1b151cc13d163b90219c; 2132c4e23198d85b7d448f81c15334832b20712b43b4ca02215640b4fda52409 |
| No runtime authority or product-availability claim | No Q2 product edit/approved literal, runtime receipt, lesson publication, release or merge. Q2 separately owns full exporter/four-job/control/corrupt-proof/runtime gates |

The guard/source-reachability review and independent verifier/compiler/SDK/platform fidelity remain conditional trusted premises. Neither source/metadata inventories nor this failed observed sample prove arbitrary native correctness or absence of unseen work.

E/handoff.json is the complete local evidence index:32737 bytes, SHAb1a4917fd25c7cb62ff443ac1bbd986d51dbd16e7978d6c02a2dfec35fe98564, owner0600 inside0700. It retains exact selected/compiled source identities, all build/source/auth/config/compiler/platform pins, failure/conservation/teardown records, raw artifact paths, actual exits and historical evidence checks. The report is the completed handoff; task acceptance remains blocked on the failed positive.
