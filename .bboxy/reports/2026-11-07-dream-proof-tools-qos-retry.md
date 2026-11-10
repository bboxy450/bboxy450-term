---
type: report
title: 'Dream proof tools: sole QoS-conditioned positive retry failed'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6S9GNN1HB7PW20DSH1SY8BG
created: 2026-11-07T07:21:58.509Z
commits:
- d02cdd9fe3406d213699ec82473ef6d9309528b2
verify:
  command: python3 -B -I target/dream-proof-tools/qos-retry-20261107/retry-positive.py
  exit_code: 1
supersedes: '[[reports/2026-11-06-dream-proof-tools-landed]]'
id: 01M6TE5B62E3ZPHN17C24HQFP6
updated: 2026-11-07T07:21:58.509Z
number: 93377961366234
created_by: seat:01M6S9GNN1HB7PW20DSH1SY8BG
updated_by: seat:01M6S9GNN1HB7PW20DSH1SY8BG
authorship:
  signer: seat:01M6S9GNN1HB7PW20DSH1SY8BG
  kind: seat
  public_key: j9rmEML4pGBlMmdgzCzyAhWJXtoIFbtcaz3XVJoJunY=
  process_generation: '1'
provenance:
  digest: sha256:77e9b80da462388c2dd57bdf9e30f64e7099865867bf2d6a82bc61fd36087938
  frontmatter_digest: sha256:d0a0e0c09ce3bfd34b533cfd85a23d8bd8d5be19dce0497febbc554e1ea08844
  signer: seat:01M6S9GNN1HB7PW20DSH1SY8BG
  sig: aRajuBFoYKEpM1w+SIzqgc7iE2JiADmGzDv7S0qDW44pwAb33QncYkHiiAQ1yysZ9iXvN+9cxXkd641Ocp5+Dw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150208
  event:
    id: 01M6TE5D4MA89YDV8JCCNM2WNN
    sequence: 150208
    hash: 99b7e0bf9af7c1f11a5e904110e51a90929c4b3b61aa9c131609b692005d509d
---
The one QoS-conditioned positive retry authorized by the latest task ruling and GO message01M6TDGB3N7FGH93ZPB0M9N9AS failed. The canonical controller actually exited81; its unchanged predicate rejected Q1440792ns >1000000ns. The retry allowance is exhausted. No later component case, further native retry, build, qualification, receipt or landing followed. Blocker DM01M6TE0QCEYF7ADWAB9CJA4QGN delivered the full ledger locator/hash and requested Oracle disposition.

This append-only report supersedes [[reports/2026-11-06-dream-proof-tools-landed]] as the current evidence addendum; that committed report, all earlier deviations and every original failed effect remain intact. This was not a low-load run, so it does not establish that the budget fails at low load.

## Exact execution and unchanged source

Worktree /Users/feral/mydev/bboxy450-harness-dream-proof-tools remains clean at signed d02cdd9fe3406d213699ec82473ef6d9309528b2. No tracked source changed or new code commit was made. The same real pair, original prefix inode, incarnation, 47112-byte Basis (SHA0b266a18d2c3c52ea2a89d89ea323c4a6de2f771c4275538e39c89e3fe01f1f8), source lock, compiler/SDK/platform and fixture inputs were revalidated. All original handoff file/tool hashes were checked unchanged after the retry. Seven canonical read-only currentness command records exited0; no rebuild or candidate --version invocation occurred.

The frozen input was ingested again without recompilation/substitution:
- source/preparation commit35d3e60288d394ff0f0d9e2e6abb521d3d60b007;
- C SHA7a7536546a8cae7d088a3a372d1baeb26f46a3990e2524a792624f59ae12030e;
- supplied executable SHA86c5c9b5705f117cefda90f0d0bcd892813c631cf7aa9de573c19607be5a8858;
- preparation SHAd66feb400b89b47aac0e55fc2d2ada994362bde770a398108fe02f1fb1d318c6.

The term origin/main ancestry check for signed ee60f8184aa1b0181d58a2f55bffb99501eca7d0 exited0. The shared term working checkout was observed behind origin/main and was not modified. Actual process accounting below distinguishes landed code from running process QoS.

Private generated instrument target/dream-proof-tools/qos-retry-20261107/retry-positive.py, SHA7b97ef26b7a549ab36c0278959b103663319f8f43725944cf7f48466d869d6e8, reuses the signed recipe's existing selected_q2_input, build_inputs, private_descriptor, wait_component, raw retention and unchanged component_result/component_terminal predicates. It contains no replacement native decoder/controller or threshold. It uses one exclusive directory and never automatically redispatches. Source/credit remains attached. It is retained private test-parent instrumentation, not a new production recipe or runtime authority.

Actual command: python3 -B -I target/dream-proof-tools/qos-retry-20261107/retry-positive.py, exit1 after actual Q2 exit81. Original build B = target/dream-proof-tools/runs/01M6T0EZ07D77BYZK7YNHP2A32; retry R = B/components/positive-qos-retry. R/authorization.json records the task etag, GO message, exact parent instrument/recipe hashes and zero remaining retry allowance. The original B/components/positive and B/terminal.json were not overwritten. R/spawn-intent.json and pid.json preserve actual birth; R/wait4.json records actual reap. The original pending-effect fence covers R.

## Before/after load and observed QoS

Before/after observations use /bin/ps executable names/numeric identity plus public SDK RUSAGE_INFO_V3 per-QoS CPU counters. Each read is sampled twice and bound to PID/start/image UUID; deltas require the same before/after start identity. No private priority-bit decoder, foreign-process scheduling mutation, process kill or peer cleanup was used. The bounded 128-process projection did not truncate.

| Observation | Before | After |
|---|---|---|
| Load averages, 1/5/15 minutes | 9.4990234375 /11.830078125 /10.4599609375 | 9.5576171875 /11.69384765625 /10.4423828125 |
| cargo53665 | Background CPU already observed; priority4, nice0 | Same identity; background counter delta21626, other QoS deltas0 |
| rustc78045 | Background CPU observed; priority4, nice0 | Same identity,137% ps CPU; background delta838637137, other QoS deltas0 |
| Q2 formal Python54067 | Legacy QoS CPU observed; priority31, nice0 | Same identity,84.4% ps CPU; legacy delta192117604, other QoS deltas0 |
| Q2 formal Java children | Before PIDs retained in raw snapshot | After PIDs81176/81228/82177 at99.8/100.0/105.2% ps CPU, priority31/nice0, CPU charged predominantly to legacy; new/exited identities have no fabricated interval delta |

QoS counters are cumulative class accounting, not an instantaneous foreign-thread class or proof that overrides/other contention were absent. Raw counter units were retained without inventing a conversion. Nice/priority are separately reported and are not used as QoS labels. Parent/test scheduling was not altered. The unchanged canonical C itself sets nice>=10 and utility QoS (frozen source:2350–2358).

R/load-qos-before.json,12504bytes,SHAeacd264d856dc04938007a71c5a5b53e94383e9e2f70e59b76e7d0499cf3a513.
R/load-qos-after.json,16307bytes,SHA511a6e5f80ce7394a321a5175864665e364e44a9405146139b6028ac392f8137.
These snapshots show a changed compiler QoS condition and material remaining legacy formal activity. They do not justify a low-load or fully migrated-workload claim.

## Actual retry result and full ledger

Controller PID80868 actually reaped exit81; helper82281 created/reaped1, group_gone=true, retired_coverage=true. Only first form13/show_keys was reached. The result remains outcome=failed, phase=settled, observed_proof=not_reached, runtime_approved=false. Raw stdout equals persisted output/result.json. Parent wait4 records user39376999ns/system148930999ns/maxRSS2293760bytes.

Full R/output/ledger.ndjson is48750bytes, SHA6b6ec65a0e628578f22be29f0d8dcb6ce57624b3c82a1b151cc13d163b90219c. Line32, grant30:
Q=max(W893000,A100167)+C547792=1440792ns >1000000ns.
STOP call1504911562052000; member endpoint1504911562842000, a789000ns interval. Roster stage wall1548000ns/CPU398417ns. These are retained measurements; no source defect or absence-of-contention conclusion is inferred.

Native birth max395708nsCPU/1328000nswall, inclusive572583ns. Reported helper RSS671744bytes/probe RSS2293760bytes. Final terminal diagnostics say control_verdict=failed, persistence_verdict=persisted, terminal_cost_verdict=passed, complete errors/diagnostics, charged/paid debt18651974625ns, retained units/overflow0. Successful terminal persistence cannot erase the failed engine/resource verdict.

| Raw file | SHA256 |
|---|---|
| R/stdout,2046bytes | b54d6aad66c3662eada9c05244be0b9caeb6668304eaba669d47a033b8338b0a |
| R/stderr,1675bytes | 4c9c033058e4367f11e30f6e614ccb9e513e812a266704fc0d974fb189bed1ed |
| R/wait4.json,141bytes | a7332d94f220d8d7085100ae48f21b2559f2854bf4e0d4b6c895b04fca6172bd |
| R/spawn-time-bounds.json | d7ed49af0ca0f4c6577d877abe050dcdb6a735f7658d9fa4dffef4a6ec47a8b4 |

## Itemized additional errors

The producer's histogram counts every failure call by code and keeps bounded first/last detail samples. It does not retain an individual detail string for every middle call. No unrecorded per-detail count is inferred.

| Run | Code | Total calls | Retained first / last detail |
|---|---|---:|---|
| Original positive | resource_limit |3 | current member-window and controller lower bound exceeds one millisecond / ordinary engine unit exceeds one millisecond |
| Original positive | settlement_unknown |1 | final unreaped group kill unresolved / same |
| QoS retry | resource_limit |3 | current member-window and controller lower bound exceeds one millisecond / ordinary engine unit exceeds one millisecond |
| QoS retry | settlement_unknown |0 | absent |

For each run the first stdout error is resource_limit. Thus the original has two additional resource_limit calls and one additional settlement_unknown call; the retry has two additional resource_limit calls. At least the last resource detail is an ordinary-engine-unit overrun. The other middle detail cannot be reconstructed from the bounded samples. Original stderr SHA269355e1371d9dfc9c8ca48c64d68c6cc5063329945b6a45cb2f58e8935112ea remains unchanged. Actual eventual settlement does not remove the earlier settlement error.

## Verification, retained evidence and stop

Complete handoff: target/dream-proof-tools/qos-retry-20261107/handoff.json,32789bytes,SHA655918ee855e9122bd17440eb9f480271373b7397eb033cbf3e227af2267635d. It includes all19 retained file hashes, original-file equality checks, full process observations, result, peak ledger, wait4 and both error histograms. Disk before this work was about22.6GiB free, above the8GiB floor.

The exact full task command remains the previously recorded exit2; it was not rerun to launch another positive. The current retry is a distinct authorized experiment and also failed. Previous 56 offline tests, Rust process witness, license gate and source-bound354model/87contract native/TLC parity at d02cdd9f remain as documented in the preceding report; they were not rerun after this failed retry and do not qualify it. No tracked source changed.

Remaining component cases and all success-dependent qualification/receipt/landing gates are unmet. No further native attempt is authorized. Oracle disposition of the exhausted retry and remaining contention is needed. artifact_claim was not retried; the Oracle identified its refusal as the known bboxy bug and instructed it be ignored.
