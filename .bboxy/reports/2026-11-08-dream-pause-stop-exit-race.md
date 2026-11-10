---
type: report
title: Foreground STOP terminal settlement landed with INPUT68
task: '[[tasks/2026-11-08-dream-q2-pause-stop-exit-race]]'
author: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
created: 2026-11-09T08:01:37.512Z
commits:
- d49f014588e5bbb34dd291e2a3b4ec69794fe9e9
- 830a6eb39e827225bd87df9a1e3611dd450dbd81
- dcb4f02f63f932c1a23de16967829b85b8326c0e
- ab501af00da1e8d64cda9bcbbbd01f8c30d64ff4
- a68d200268ca816a6741e00c6cd334b28628f805
verify:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
id: 01M6ZN7DPDKJAN0EDSWP8B93H5
updated: 2026-11-09T08:01:37.512Z
number: 93377961366446
created_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
updated_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  kind: seat
  public_key: 3lIiKAyaYQbqAYQ3xsrS+NTGLp9s//BjqCA4HkK5nF0=
  process_generation: '1'
provenance:
  digest: sha256:1084308da43681b2e428394fb02da2178e7ac46d7efb39b8e309d3bdaa5089f0
  frontmatter_digest: sha256:783899c61bf29db7d5cf7c2ea96b1fea8a125c453891c0c815f73911abc88b51
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  sig: Abkz2kM75r4p0uPmL/WPfQJAi28kVtK31ZhXzvF5apk9y8HMGJhd3YmC2wPAHQfMKylhRkXq2EdlusSjwmFHCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150972
  event:
    id: 01M6ZN7EAR1CVXQPNQ7FSFHW3M
    sequence: 150972
    hash: e9936c12b2a0e7422d3cbc865fe93f5fa874c629e1904ec9181dcccc26aa398c
---
Implemented and landed Task66427 at signed `a68d200268ca816a6741e00c6cd334b28628f805`; the worktree is clean and origin/main contains that head. The prescribed four-command verification and canonical `make land` both exited **0**. ONE immutable INPUT68 was sent after landing. No real candidate or builder ran; runtime approval and tool qualification remain false/unproven.

Evidence root `E=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-pause-stop-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3`. This report covers this task; the prior accepted INPUT67 history remains in [[reports/2026-11-08-dream-unit-split-and-margin]].

The feature changes eight files: the probe, its C controls/checker, `tests/formal/{dream_read.rs,models.rs}` and `verification/{contracts,corpus,coverage}.toml`. The separately landed workflow changes only `scripts/formal.py`. Rust product source, public/terminal codec, all caps, fresh 9x debt, 50 ms cadence, stopped-interval rules, decision66331 and the 30 s limit remain unchanged.

## Mechanism and acceptance evidence

- `proof-helper-probe.c:3545` records the actual STOP attempt/result/errno and observation order. ESRCH enters pending terminal settlement; EPERM additionally needs the existing independent zombie-only group proof. Neither supplies ACK, terminal proof or CONT authority.
- `proof-helper-probe.c:3573` requires original own-wait4, independent group absence, complete known authoritative CPU and fully paid unchanged debt. Unknown clocks/costs, missing own reap/group absence, other errno and unpaid debt refuse. Over-cap known CPU stays fully billed before refusal. `:3608`, called by central quarantine at `:4077`, records coverage_unknown without own reap and preserves an earlier first error.
- Current canonical C evidence is `E/canonical-C-input68-v5/run.json`, SHA256 `4f0ab0f848a2298ef34ed09015ef7d73cce8b350ffdaceeca00509e6739aa0b0`: **114 cases, 74 causal mutants, 76 strict compiles, 179 commands**, no timeout/current-source drift. Sixteen supplied failed-STOP API cases plus two central-category cases cover valid ESRCH/EPERM, missing reap/group/payment, other errno, unknown CPU and clock failures. The two new mutants exit81 at their intended assertions. API syscall, identity and usage facts are explicitly supplied/nonphysical.
- The actual deterministic process witness PID87982 positively exits before the pause request: STOP attempt/errno/ACK/CONT are0; original own-wait4/groupgone/resolved are1. Actual rounded CPU2,756,000ns incurs full24,804,000ns helper debt; final123,804,000ns is paid. This proves the preobserved-exit route, **not an actual failed kernel SIGSTOP/ESRCH race**. The original INPUT67 fault's kernel cause remains unproven.
- `dream_read.rs:10036` contains the standalone18-field/13-case foreground-STOP companion and its five safety predicates; `:10262` checks all outcomes and the missing-path control. The legacy twenty TotalBudget graphs and assertions are byte-identical to landed d49. Exact native projection comparison preserves states and action-labelled edges: new Good105/104, broken105/108, including failing observer loops (`E/standalone-factor-exact-graph-proof-v3.json`, SHA256 `e43a7ab4ca7c076afd8268298b902f0dab298b7f306befc1b6b2cd980fa5093b`). This is finite protocol/cost-class evidence, not a numeric timestamp/OS-cause or arbitrary implementation theorem.
- `scripts/formal.py:340` uses explicit clonefile return/errno, separately logged plain-copy fallback, ordinary-copy destination modes, and every staged file's source SHA before publication. Clone/fallback/corruption/mode controls and twelve existing Python checks pass. Workflow-only signed d49 was canonically landed0 before the final feature chain; `scripts/land.py` is unchanged.

Fifteen additions appear only in existing `pause_user_and_terminal_rss` rows:

| Type | Keys |
|---|---|
| u32 flags | pause_original_owned, pause_pending_terminal, pause_stop_attempted, pause_terminal_candidate, pause_terminal_preobserved, pause_terminal_resolved, pause_zombie_only_proven |
| u64 | pause_stop_call_ns, pause_stop_return_ns, pause_terminal_observed_ns, pause_wait4_observed_ns, pause_zombie_only_observed_ns |
| i32 | pause_stop_errno, pause_stop_result |
| string enum | pause_stop_order |

The exact enum values are terminal_not_observed, clock_unknown, terminal_observed_without_stop_attempt, terminal_observed_before_stop_attempt, stop_attempt_before_terminal_observation and terminal_observation_inside_stop_call. Original-owned is latched custody proof at request; observation ordering does not establish the actual exit instant. No public or terminal wire field changed.

## Final verification, retention and handoff

Literal verification: `make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build` — **0**, same signed head/source before and after. Record `E/pause-race-exact-four-chain-final-v5.json`; log SHA256 `7b0cd627a51060fdbb110787ca311c2865ffd18773a44ea032e159a7124afa28`.

Full native/pinned-Java parity covers **444 models /95 contracts /1 crate**, **3,378,237 states /11,367,153 labelled edges**, with every expected named verdict and complete graph matching. Archive `verification/evidence/80a07c0dcac05acad1cfbea929fb6be9c76512413282f5c5b94b31e79fcee1aa` retains4442 raw files/7,436,496,058 logical bytes; all raw SHA/modes were checked and sealed400/500. Full manifest `E/current-formal-all-raw-files-input68-v5.json` SHA256 `d041994830344280e12c27959da91ec5c56cefe7e85657871b63121d0bb73817`. All five observer outputs retain original inodes. The53 actual profile files retain generated metadata-file bytes and worker bytes in independent readonly copies with new filesystem inodes/modes. Byte-preserving native-data compression occurred only after native completion while our Make coordinator was paused, before Java; no test/product process, timeout, cap or semantic setting was paused/changed.

Canonical `make land` — **0**; actual push d49→a68. Record `E/pause-race-canonical-make-land-input68-v5.json`; log SHA256 `93fa3c0f61e1b1cfbc1acfb21812b68129dc90ad674d1b6fe4b7c64cf43c9ee2`. The unchanged canonical validator accepted the current proof cache; the cache import force-cloned each file with explicit results, full SHA checks and the same pinned-tool inodes.

Post-land canonical preparation0 and independent audit0 bind all10 signed inputs to the same source/preparation head, actual compiler/linker/flags/SDK/platform,325 headers including lexical pthread alias/target,997 checks and353 retained outputs plus the receipt (354 entries). Private roots are0700, files0400, executable0500, nlink1. Tuple `E/INPUT68-source-freeze-tuple-v1.json` SHA256 `e0f3ba9d8120af99f8e7dfdd8470ccff4414732f855a5ddfb7ef21353bdd3008`; preparation receipt SHA256 `7f736d1abc0b493aa0ec12adc443594e5bd9e1ecb65df306101b3dfa1321eb05`; actual C397100B SHA256 `3c18a373246830e2efce125df292d153b5cb1c2df9458c027dbd02da9e51cec2`; executable285784B SHA256 `87ce8d48283f9a3f7eab41cbf8e073694d3b76ba2066a6223f94c18393d47131`. Audit `E/INPUT68-independent-retention-audit-v1.json`, SHA256 `dfc4a023f03f926b28957f6906c0773d5cc8b9988acacc9129c1c7ca9c6459d9`. The read-set is a byte snapshot, not deployed-product qualification.

ONE handoff was durably stored as seq1572/id/op `01M6ZMA612BNCJEHZE714TXE1A` in the original task thread; storage does not imply application acknowledgement. Runtime/candidate/whole-lifetime-proof flags remain false. Twenty-nine existing ignored attributes remain nonpasses:22 QoS,5 qualification,1 manual benchmark,1 inherited upstream KNOWN-DEFECT; none was added here.

Owned build cleanup0 followed a fresh unchanged claimed-task fetch, full SHA/stat/flag/inode plan and two empty lsof observations. It removed19882 regenerable entries/19,193,900,299 **logical** bytes, preserving immutable inputs, E, all archives, current full proof cache/tools and5106 kept stamps. Result SHA256 `fbf6a53c40c9cb4b4c46a43c3191285f272c1cc633b28700b159e6a4e70f7bf9`; observed free-after38,283,436,032B, not a claim of that much physical space reclaimed. The8GiB reserve remained enforced.

## Preserved failures and limits

Old v1/v2 exact attempts remain interrupted143, not passes. The early workflow wrong-mode cut was stopped before push and remains unlanded/nonpassed. Final-v3 remains2 at the unchanged TLC60s limit; factoring removed needless111-field padding without weakening any old graph/property. Final-v4 remains2 at the observer's aggregate1,293,708ns preparation unit. Three pipes, two flushes, fork and custody bookkeeping were combined; no per-operation cost is inferred. The three-line fixture split pays preparation before fork and retains the original post-fork1ms guard. Its real hold plus three existing lifetime mutants pass (eight commands); current canonical hold also passes, with helper87939 CPU7,304,000ns/full65,736,000ns debt paid. In failed v4, controller10967/helper10969 already existed; helper ownreap/groupgone/full7.37ms CPU bill paid, but owner_lost0 and the overall fixture remained failed. Evidence-helper stale-freeze/argument preflights refused before effects and were corrected without rerunning tests or relabelling receipts. All records are retained. Installed-tool qualification and the historical kernel-error cause remain unproven; no builder run was used for acceptance.
