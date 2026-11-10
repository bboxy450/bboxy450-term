---
type: report
title: 'INPUT71: three proof cases pass; host primary-clock assumption stops the run'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T19:46:07.995Z
commits:
- a7bb5535010affe9ddebc3b4407b794ef060ca26
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M70XHD2JTYXNR7Y63A1VXADP
updated: 2026-11-09T19:46:07.995Z
number: 93377961366492
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:4c2c1b3117cffe2bb37f0b7b01aafc2eaef64b1b15b69248e01a4221af2f9519
  frontmatter_digest: sha256:b319ce8956ce0c219062385827730465c859b4f987025ed247e9952a49147649
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: svtgdNsMdTo6z4x0KlQ4RUzC286L8cPdnRaUr2seT/vun2QdZ3xG4Pjf5oIQtn88zy0ZUTFYzapHMk8YGWkbAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151137
  event:
    id: 01M70XHE3844TF81ZB4HEXN8D7
    sequence: 151137
    hash: e38098dc1ccf30ad48c10a63581ffd92337f252e7da5e43df57dd9e6d552c968
---
INPUT71's positive, corrupt_proof and pause_resume PASSED their independent checks, each with all28 conjunction terms true and physical16/16 settlement. cancel and stale_incarnation also reached and passed their expected causal checks. The single invocation then stopped at clock_failure because of an overly strict HOST check I added in99bda9ca: it demands one clock_unknown diagnostic, while the frozen producer deterministically emits three diagnostics from the same injected unknown cost. Actual canonical exit2 is retained; this is not a full qualification pass. debt_overlap, reap_delay and final gates did not run. No retry, retrospective checked.json, signals, cleanup, qualification or candidate receipt was performed.

Signed pin-only commit: `a7bb5535010affe9ddebc3b4407b794ef060ca26`, on99bda9ca; clean worktree. This report adds INPUT71 evidence and preserves [[reports/2026-11-09-dream-proof-tools-input70]] as the failed historical INPUT70 run. Authority is [[tasks/2026-11-05-dream-proof-tools-provisioner]], section “ORACLE FREEZE, INPUT71”, etag `3c90f883ad7806a69125321a77aa4e5d4ff821bec8421792a1c106c156c6d252`, plus its existing clock auxiliary ruling and corrected decision66459 Amendment2.

Evidence:
- E = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input71-20261109`.
- R = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M70WYNQV6MRHPMDTEMR8Q1CJ`.
- P = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M70WYNQV6MRHPMDTEMR8Q1CJ`.
- C = `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/harness-landing/dream-read-c-preparation/compile-_ybqbhvt/proof-helper-probe.c`. The entire supplied preparation directory is retained; no cache cleanup occurred.

## Scope and frozen bindings

Only README.md, the immutable ROOT_Q2_INPUT selection in scripts/dream-proof-tools.py:52, and sources.lock.json changed. The shim install, compiler-define mechanism, source authentication, closure, budgets, clock checker, publication and ownership machinery remain99bda9ca. README now explains INPUT71's conditional post-reap settlement semantics.

Actual source signature/ancestry/main checks exited0 for `ad6c8b72fe0bb766164c879acd27c1ece2e565a7`, Good Boxy; INPUT70 is an ancestor. All12 inputs matched both signed git bytes and retained snapshots; only C changed from INPUT70. E/frozen-input-checks.json preserves commands and hashes.

- Tuple37964B SHA256 `f8d82d84b92557d346bc4942d398a21a841addd5769420d914d55c26519cccf1`.
- C449970B SHA256 `077d63a18be3e36ef0c1289f263ffb87beb85954e059c3e929d970248d4c9a63`.
- Supplied probe303256B/0500 SHA256 `9a9ee37905cdbe6d04c933c7ffc902d06fa29345418dbcd4a59161c4ebf078dc`.
- Receipt501669B SHA256 `ab2227a84f7e1e522bf890910492d7fe1841773a16c98de658de62d41613d521`.
- Shim35520B/0500 SHA256 `4e95b2a83a34a6328d3c794133c3569dff7e5f1b2ee2c9bfc1d58cd08447527d`; source/header/allowlist/compiler bindings unchanged from INPUT70.
- Refreshed path-dependent bindings include shim projection `6bbe51c25e8bccc6c1d496d362ca26a996408dc160b6750d96ac97bf0411ad52`. Proposal source read-set digest remains `45d1503c276f3e00a89694a07449f2b8c7e91422f06b1e60a3332be3c90f7960`.

Frozen ingestion with execution seams forbidden exited0. Receipt, compiler/SDK/header closure, source/executable custody, Mach-O, shim compilation and retained identities passed the existing offline adapter; E/offline-ingestion.json3487B SHA256 `b7db0c4b6d6ef2715050cb887fda209113493ed9cd70f1d7b0825fdd8c486b9b`.

Field manifest SHA256 `8aeb4c1279a79fbb822962db623025e601edec22d04e61f3d392618da2b5f941` adds only raw kind escape_check_unavailable with exact fields kind/site/errno/domain, sites escaped_cleanup_bsd|group_census, errno i32 and domain enum0..4. No terminal/public-wire/Basis fields changed. The host retains the producer-owned ledger unaltered; it has no generic ledger-kind whitelist to expand. The reporting extraction checks exactly the new diagnostic shape when present. The existing causal clock reader continues to require its bound pre-CONT evidence.

C:2853–2856 makes unavailable escaped-cleanup BSD reads diagnostic under shim custody or after reap; C:2923–2926 similarly handles post-reap shim group census. C:4693–4714 performs best-effort post-ownwait4 checks: unavailable reads, unknown/different incarnation and zombies do not refuse; a positive same-incarnation live hit does. UNIX ESRCH deliberately emits no unavailable row (C:2849). group_gone now means completed conditional shim settlement without a positively observed original-incarnation live hit. A zero diagnostic count does not prove read success or unconditional group absence. Coalesced-fork and setsid-orphan source assumptions remain explicit.

P/bin/git is3766096B/0700, SHA256 `eca6d5d02b79224089afffff1ffbc9c368e53e1101d8bb5dc3a1818227a484ac`; its exact DREAM_PROOF_GPG target is P/libexec/dream-exec-shim in both GIT-CFLAGS and linked image. Actual Git build039 exited0; GIT-CFLAGS SHA256 `04cdc4e676d0e60512cc7ec90cb1cddf0c89c7129f707beea55ee5c3f3cf51f9`. The shim hash and0500 mode match the freeze. E/phase-b-build-bindings.json SHA256 `ed1bb22530e15aba93a14296440a2fb90003eb5c4b7719781e0541d8dbde4be7`.

Basis47490B/non-shim47147B/shim delta343B, SHA256 `ad97f692c7e7000770ab780d504d23989f8960b3384f393853240c77c97bcc73`. It fits48KiB; the56KiB headroom is unused. No prepared budget, graph, array, depth or other ceiling changed. No final qualification envelope was emitted.

## Real commands and outcomes

Final pure command `python3 -B -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target`:78 tests, actual exit0. E/unit.log13429B SHA256 `97cf123a29ac4f21329007367fcdcaa18a207b4e1f8e6fc64107c093efc89411`. These tests did not catch the primary clock-count assumption; the real fixture did.

Seven fresh source authentications and all60 build commands000–059 exited0. Exact canonical command `make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`:actual exit2. E/verify-exit.json records the original shell status; E/verify.log10008B SHA256 `3f51ee48c904f71e22986348abe764d6a0e2e6f7443ecd3c8221972aaf1f5cbb`. Invocation free disk26755977216B exceeded8589934592B. No second invocation occurred.

| Case | Actual exit / host check | Raw outcome / proof | first_failing_term |
|---|---|---|---|
| positive |0 /0, PASSED |completed /valid |none, all28 true |
| corrupt_proof |0 /0, PASSED |expected_rejection /invalid |none, all28 true |
| pause_resume |81 /0, PASSED |completed /valid |none, all28 true |
| cancel |81 /0, accepted causal control |failed /not_reached, cancelled |Unavailable; no conjunction row |
| stale_incarnation |81 /0, accepted causal control |failed /not_reached, coverage_unknown |Unavailable; no conjunction row |
| clock_failure |81 /host REFUSED |failed /not_reached, clock_unknown |Unavailable; no conjunction row |
| debt_overlap |Not run |Unavailable |Unavailable |
| reap_delay |Not run |Unavailable |Unavailable |

Exit81 is the frozen producer's specified exit for pause_resume even when its independent control and conjunction pass. It is not converted to0. All three full proof cases settled16/16; all three causal cases settled1/1 with actual controller wait4. All six persisted result.json files equal unaltered stdout. No live quarantine, no pending original child and no teardown is required. Final ps of controllers32495,33731,35153,36462,37190,37861 returned1/no rows; Executor sent no signals. Producer-owned control cleanup signals remain raw evidence.

Final cumulative charged debt in seconds (9×cost), taken once from each terminal row:

| Case | Helper | Ordinary known raw | Ordinary floors | Final total | Reporting-cutoff margin | ordinary_max_ns |
|---|---:|---:|---:|---:|---:|---:|
| positive |2.315115000 |1.927744875 |10.150255125 |14.393115000 |13.293223000 |831958 |
| corrupt_proof |2.260584000 |1.610321625 |9.729678375 |13.600584000 |14.392810000 |470416 |
| pause_resume |2.192598000 |2.095041753 |10.261958247 |14.549598000 |13.251674000 |952416 |
| cancel |0.067941000 |0.509431869 |5.889568131 |6.466941000 |22.269653000 |418166 |
| stale_incarnation |0.009000000 |0.496074375 |5.902925625 |6.408000000 |22.586772000 |436834 |
| clock_failure |0.009000000 |0.513848250 |5.894151750 |6.444000000 |22.590355000 |439792 |

For clock_failure, ordinary_raw_complete=false. There is an additional0.027000000s charged ordinary unknown-cost remainder, derived as9×(booked ordinary cost minus recorded raw minus floors); it is not measured raw CPU. Including this remainder gives the final total. The other five raw splits are complete. Every final charged debt equals paid debt. No deadline was observed; process_create excess was known1/count0/ns0 in every reached case. Margins are deadline minus reporting_started, not claims about the final receipt's future write.

## Shim and escape diagnostics

There are49 successful exec_shim receipts:16 each for positive/corrupt/pause, one for cancel, zero for stale/clock before their injected failure. All recorded receipts have in_process_checks=true/self_priority4. No shim refusal was reported. The clock's missing-receipt coverage auxiliary is a deliberately uncontinued shim, not a binary denial.

All reached cases have zero escape_check_unavailable rows and no recorded positive escape refusal. ESRCH is intentionally silent in the frozen implementation; this is diagnostic evidence only. The full raw ledgers remain retained:
- positive SHA256 `c44a018ee3be936629905c5c05685a59d1aca9da38c273af16cf6822c77c3e7b`;
- corrupt_proof `b2cbdd6d77ab52a8b7207d098d463bcb18cc9e669fee747557a69b840c36c991`;
- pause_resume `58e791bf27126fbffce0b6a08474d76529363fd6dcea4d6f7688a3ced6b282fc`;
- cancel `a4b0bdc5524f0a9cc41bab6e430a85e5015e619c10ad4ad1f7a149896f4053be`;
- stale_incarnation `8458bb96c433dc9a4188e6ecc8dd0be4b9431478b8bab18944d0c527f7db464b`;
- clock_failure `8d1f9d6b215fccd2006c0a8a8c719fe17485f06b4a93541e2b9bcd068053cbcd`.

pause_resume original per-owner fields (all running_gap_max_ns=0 and stopped_system_ns=0; these are producer diagnostics, not a restored sampling gate):

| Sequence | PID | stop_errno | stop_order | terminal_resolved | ACK/resume | stopped_interval_ns |
|---|---:|---:|---|---:|---|---:|
|1 |35698 |0 |terminal_not_observed |0 |1/1 |139944000 |
|2 |35772 |0 |terminal_not_observed |0 |1/1 |181420000 |
|3 |35836 |0 |terminal_not_observed |0 |0/0 |0 |
|4 |35877 |0 |terminal_observed_without_stop_attempt |1 |0/0 |0 |
|5 |35913 |0 |terminal_observed_without_stop_attempt |1 |0/0 |0 |
|6 |35952 |0 |terminal_not_observed |0 |0/0 |0 |
|7 |35983 |0 |terminal_not_observed |0 |0/0 |0 |
|8 |36027 |0 |terminal_not_observed |0 |0/0 |0 |
|9 |36063 |0 |terminal_observed_without_stop_attempt |1 |0/0 |0 |
|10 |36099 |0 |terminal_not_observed |0 |1/1 |133671000 |
|11 |36160 |1 |stop_attempt_before_terminal_observation |1 |0/0 |0 |
|12 |36204 |0 |terminal_not_observed |0 |1/1 |154559000 |
|13 |36253 |0 |terminal_observed_without_stop_attempt |1 |0/0 |0 |
|14 |36295 |0 |terminal_not_observed |0 |0/0 |0 |
|16 |36381 |0 |terminal_not_observed |0 |1/1 |144149000 |

Sequence11's STOP race resolved by the original own-wait4 path; raw errno1 is retained. No repeated rows were summed and no hypothetical sequence15 owner was fabricated (the final implicit verifier consumes the second attempt).

## Clock host defect and precise decision needed

clock_failure actual wait4 exit81/raw20736, PID37861; raw result error is the injected `clock_unknown: fixture unknown creation ending clock retains actual effect`. stderr records:
- clock_unknown count3, first_detail=injected detail, last_detail=`closed engine unit has unknown CPU`;
- exactly one coverage_unknown, both endpoints `helper lacks its one in-process exec receipt`;
- exactly one resource_limit, both endpoints `process creation native2ms wall250ms application1ms inclusive3ms gate`.

The eight-row ledger proves one reaped original owner, activation/birth/start0, no CONT/resume or exec receipt, shim enforcement true, own-wait4/group/paid flags and no quarantine. Row3 ordinary_failed_charge records one engine_birth charge at C:4498 with actual/supplied/cost UINT64_MAX. Direct offline rechecks of the retained unaltered result and causal pre-CONT ledger both exit0. The existing terminal adapter still refuses `clock auxiliary lacks its one injected clock error`; E/clock-checker-diagnosis.json SHA256 `0b36b75cad53521327b393b40d5768d7ae2f6b4a978275215271ec457ff04850`. That diagnosis launches no process and does not create checked.json or change the original verdict.

Source causality: C:4497 injects the first error; C:4498→3138→790–791→729–732 retains its unknown cost and emits `unknown or overflowing closed-unit cost`; C:3146–3148 emits the final `closed engine unit has unknown CPU`. The middle detail is derived from source: the wire retains only first/last endpoints. These are three observations of the same injected unknown cost, not three separately measured native failures.

My additional requirement at scripts/dream-proof-tools.py:3371–3374 imposed clock_unknown count1 and equal first/last endpoints. The recorded ruling requires the first error to be the injected clock_unknown and EXACTLY ONE auxiliary coverage_unknown; it does not impose that primary-count assumption. The restriction I added was a host adapter defect. The existing count2 negative unit encodes that assumption rather than the real producer cascade. No producer defect is claimed here, and the positive71 fix demonstrably passed.

Decision needed: confirm the deterministic three-diagnostic primary cascade under the existing bound no-CONT/no-exec ruling, so the checker can remove/replace only the incorrect primary-count/equal-endpoints restriction while retaining the exact single auxiliary coverage and resource detail, original result/ledger binding, settlement, debt and persistence checks. Any further native invocation needs an explicit new ruling/freeze: the task authorized ONE and it has been consumed. I have stopped at the actual refusal, preserved the failed run and made no retrospective pass or additional invocation.

All27 historical ledgers retain exact length/hash/dev/inode. No supplied preparation, prior prefix, source or witness was deleted or altered. a7bb5535 is signed Good Boxy; final diff-check0/status0 empty. Final formal/parity/license/check/qualification/receipt gates were not reached, so no current-source formal receipt or whole-task green is claimed. Runtime and whole-command native lifetime approval remain false.

E/result-evidence.json SHA256 `65b6d8ad81d6b8d010c66edc9382405cd71cfd964cfd6e21985ba78c275ae916` contains all six raw results, wait4 statuses, terminal splits, per-owner pause and escape rows, hashes and history preservation. E/handoff.json895B SHA256 `cb3d3ddefbfd885f9ca6a832af42200487eac39cc2b4d224d7242aaa278e1bdc`.
