---
type: report
title: 'INPUT71 clock corrected: all eight native checks pass; final gates blocked by sccache'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T20:08:36.256Z
commits:
- c50efb4e155b919f1d5a34620465ce14be725bd6
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M70YTH66WZ9T73W12TCYQG62
updated: 2026-11-09T20:08:36.256Z
number: 93377961366496
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:300ebb08baa11143851d4f72ce925153a98cb4b6a37ea616d1a48884bafc3235
  frontmatter_digest: sha256:258142d60b9e2df2cd9446e55ea2cbb192f013b7db66ecca1836c975ddf02b48
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: 3Wkq6CBvVQYl0wFw6mSQ/Ew937Gtfc2ojnvDkbngMe2MtZzBE66v3lKw49J8ErTL/DV9t9yZ/IOD79YXbRG4Dg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151152
  event:
    id: 01M70YTJRD6SRR9Z8GHJNXY3PE
    sequence: 151152
    hash: 4c7f7e5f026be36f5815bd2cb1d4af75e8e921cb095c6d7cdbac9646f66eb75f
---
All eight native cases PASSED their host checks in the one newly authorized INPUT71 invocation. Positive, corrupt_proof and pause_resume each have all28 conjunction terms true. The clock correction worked, and debt_overlap and reap_delay were reached and accepted. The final formal/license command then failed before Rust compilation because the configured sccache wrapper could not create its socket path. The canonical command therefore remains actual exit2; qualification and candidate receipt were not emitted.

Signed commit `c50efb4e155b919f1d5a34620465ce14be725bd6` changes only the host primary-clock restriction and its inert tests/fixtures. Current worktree is clean. Authority: [[tasks/2026-11-05-dream-proof-tools-provisioner]], “ORACLE RULING (2026-11-09): INPUT71 run result; clock_failure primary-count correction; ONE new native attempt”, etag `4d79eea6b0b55f6e48edeb7e4ccf0cc90147cf670b865019a79a0c43e7fb6925`. The prior run in [[reports/2026-11-09-dream-proof-tools-input71]] stays canonical exit2; nothing was retrospectively reclassified.

Evidence roots:
- E = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input71-clock-correction-20261109`.
- R = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M70XWHWKFCD1W6ZZMQB50WG1`.
- P = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M70XWHWKFCD1W6ZZMQB50WG1`.

## Exact scoped correction and verification

scripts/dream-proof-tools.py:3359–3366 now requires primary clock_unknown count3, first_detail `fixture unknown creation ending clock retains actual effect`, last_detail `closed engine unit has unknown CPU`, for every clock_failure terminal. The check is outside the optional auxiliary branch, so omitting coverage_unknown cannot bypass it. No other error count, ordering or endpoint shape is accepted.

The existing exact resource diagnostic, exactly one missing-exec-receipt auxiliary when present, original result/actual-exit binding, pre-CONT/no-exec-receipt ledger predicates, settlement, debt and persistence checks are unchanged. Tests reject counts1/2/4, reordered3, wrong endpoints and the retired equal-endpoint single shape both with and without the auxiliary. Existing no-CONT/no-exec and unknown-extra-field controls remain. Only this Python file changed (45 insertions/8 deletions); no producer, pin, shim, schema, budget, lifecycle or model contract changed.

Final pure command `python3 -B -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target`:79 tests, exit0. E/unit-final-v2.log13595B SHA256 `6287f5f32c0fa4e5ac3332b41633af867bf5443ccb0d774266f0b2675421c3e9`. Earlier78-test iterations passed; independent review caught and corrected the omitted-auxiliary bypass before the authorized native invocation. These are offline test iterations, not native retries.

The retained old clock result and ledger now pass the corrected predicates offline, but old checked.json remains absent and the old canonical exit file remains2. E/retained-clock-recheck.json explicitly limits that result to predicate verification. Old exit-file SHA256 `8b2fd604abbab82e5e3d43858c568b4eeb91284615e6c37828f114b81abd9f64` is unchanged.

Frozen INPUT71 ingestion with execution seams forbidden exited0 again. Producer source remains signed `ad6c8b72fe0bb766164c879acd27c1ece2e565a7`; C449970B SHA256 `077d63a18be3e36ef0c1289f263ffb87beb85954e059c3e929d970248d4c9a63`; probe303256B/0500 SHA256 `9a9ee37905cdbe6d04c933c7ffc902d06fa29345418dbcd4a59161c4ebf078dc`; receipt SHA256 `ab2227a84f7e1e522bf890910492d7fe1841773a16c98de658de62d41613d521`. No new tuple or producer was substituted. The supplied preparation directory in harness-landing was preserved.

The unchanged shim is35520B/0500, SHA256 `4e95b2a83a34a6328d3c794133c3569dff7e5f1b2ee2c9bfc1d58cd08447527d`. Fresh P/bin/git3766096B/0700 SHA256 `82a1e6f23c21011e59563c2d4e26323bb7a2cd7df55f12363a77453fc4437c5c` embeds exactly P/libexec/dream-exec-shim as DREAM_PROOF_GPG. Actual command039 exited0; GIT-CFLAGS SHA256 `702e4fabc840f6b9f7f3cdfa48105f584320963dc83d6482d932aee7c741dc4b`. E/phase-b-build-bindings.json retains exact path/hash/mode and linked/flag literal checks.

Basis47490B/non-shim47147B/shim delta343B SHA256 `595d30106fbab7d947c296c161ffea5959a1b5c95dc84943d79c1e431ebbefb0`. Four shim hashes, source/compiler/platform assumptions, two closure roots and all ceilings are unchanged;56KiB headroom is unused. Recipe/script SHA256 `a8187a0ca0b093bdf4e82c2f3e172b664f60a4e5fadc2cb0d50256b8c5d62edd` equals the retained R/recipe.py.

Seven source authentications and build commands000–059 exited0 before the final gate. Invocation free disk25385963520B exceeded8589934592B. There was exactly one invocation under this new ruling.

## Native results, including later controls

| Case | Actual wait4 / host check | Outcome / proof | first_failing_term | Terminal physical settlement |
|---|---|---|---|---|
| positive |0 /0 |completed /valid |none,28/28 true |16/16 |
| corrupt_proof |0 /0 |expected_rejection /invalid |none,28/28 true |16/16 |
| pause_resume |81 /0 |completed /valid |none,28/28 true |16/16 |
| cancel |81 /0 |expected cancelled control |Unavailable; no conjunction row |1/1 |
| stale_incarnation |81 /0 |expected coverage_unknown control |Unavailable |1/1 |
| clock_failure |81 /0 |expected clock_unknown control |Unavailable |1/1 |
| debt_overlap |81 /0 |expected coverage_unknown control |Unavailable |1/1 |
| reap_delay |81 /0 |expected settlement_unknown control |Unavailable |1/1 |

Actual81 is retained for pause_resume, as specified by the frozen producer. No exit was rewritten. All8 checked.json records have check_exit0, all stdout bytes equal persisted result.json, and R/component-checks.json preserves the complete ordered eight-case record.

clock_failure produced exactly3 primary clock diagnostics in the authorized endpoint order, exactly1 missing-receipt coverage auxiliary and1 exact process-create resource diagnostic. Its causal ledger proves the suspended first owner had activation/birth/start0, no CONT/resume/exec receipt, and actual final settlement/debt payment.

debt_overlap's coverage histogram count2 has first `fixture pending debt forbids initial helper continuation`, last `helper lacks its one in-process exec receipt`. No helper continuation was admitted; the existing same-class control adapter accepted it.

reap_delay retains its original partial result snapshot: outcome/phase quarantined, created1/reaped0, group_gone=false/retired_coverage=false, error `fixture retains original unreaped handle before prompt refusal`. This wire is NOT claimed as terminal settlement. The original controller subsequently settled1/1 in the final terminal/ledger and the parent actually wait4-reaped it with exit81; the existing narrow adapter accepted that unchanged initial snapshot. It did not manufacture a new result or treat the refusal wire as an exit. No live quarantine remains.

Final cumulative debt in seconds, each charged once from terminal totals (9×cost):

| Case | Helper | Ordinary known raw | Ordinary floors | Final charged=paid | Cutoff margin | ordinary_max_ns |
|---|---:|---:|---:|---:|---:|---:|
| positive |2.104704000 |1.802439756 |9.942560244 |13.849704000 |14.121944000 |594084 |
| corrupt_proof |2.367693000 |1.509965244 |9.659034756 |13.536693000 |14.547151000 |472625 |
| pause_resume |2.209518000 |1.974248244 |10.185062625 |14.368828869 |13.506766000 |904667 |
| cancel |0.104607000 |0.474156378 |5.924843622 |6.503607000 |22.462447000 |456333 |
| stale_incarnation |0.009000000 |0.471971250 |5.909028750 |6.390000000 |22.617067000 |450459 |
| clock_failure |0.009000000 |0.661209372 |5.818790628 |6.516000000 |22.461756000 |651209 |
| debt_overlap |0.009000000 |0.526485762 |5.926514238 |6.462000000 |22.601083000 |719416 |
| reap_delay |0.194823000 |0.592205256 |5.887794744 |6.674823000 |22.579679000 |443541 |

Clock raw_complete=false; its additional0.027000000s ordinary unknown-cost remainder is derived from booked ordinary cost minus recorded raw/floors, multiplied by9. It is not measured raw CPU. Include that remainder to reach6.516s. The other seven splits are complete. Margins are deadline minus reporting_started, not a promise about the final receipt's future write. No deadline occurred; process_create excess known1/count0/ns0 in all8 cases.

## Shim, escape and pause evidence

There are50 successful exec_shim receipts:16 each for positive/corrupt/pause, one each for cancel/reap_delay, none for stale/clock/debt before their injected refusal. All observed checks are true/self_priority4. No binary shim refusal is reported. Intentional missing-receipt diagnostics in clock/debt are controls before CONT.

Every case has zero escape_check_unavailable rows and no positive escape refusal. UNIX ESRCH is deliberately silent in INPUT71; zero rows prove neither read success nor unconditional absence. group_gone remains the conditional best-effort settlement meaning from66459 Amendment2. Coalesced-fork and setsid-orphan assumptions remain explicit.

pause_resume raw fields (all running_gap_max_ns=0, stopped_system_ns=0; diagnostics are not restored sampling gates):

| Sequence | PID | stop_errno | stop_order | terminal_resolved | ACK/resume | stopped_interval_ns |
|---|---:|---:|---|---:|---|---:|
|1 |47585 |0 |terminal_not_observed |0 |1/1 |169926000 |
|2 |47641 |0 |terminal_not_observed |0 |1/1 |169876000 |
|3 |47711 |0 |terminal_not_observed |0 |0/0 |0 |
|4 |47744 |0 |terminal_not_observed |0 |0/0 |0 |
|5 |47779 |0 |terminal_observed_without_stop_attempt |1 |0/0 |0 |
|6 |47829 |0 |terminal_not_observed |0 |0/0 |0 |
|7 |47863 |0 |terminal_not_observed |0 |0/0 |0 |
|8 |47897 |0 |terminal_not_observed |0 |0/0 |0 |
|9 |47933 |0 |terminal_observed_without_stop_attempt |1 |0/0 |0 |
|10 |47979 |0 |terminal_not_observed |0 |0/0 |0 |
|11 |48011 |0 |terminal_observed_without_stop_attempt |1 |0/0 |0 |
|12 |48050 |0 |terminal_observed_without_stop_attempt |1 |0/0 |0 |
|13 |48086 |0 |terminal_observed_without_stop_attempt |1 |0/0 |0 |
|14 |48142 |0 |terminal_observed_without_stop_attempt |1 |0/0 |0 |
|16 |48178 |0 |terminal_not_observed |0 |3/1 |150838000 |

No repeated rows were summed. The implicit verifier consumes the final second attempt; no sequence15 owner is fabricated.

All8 original controllers were actually reaped. Final ps for43841,45370,46775,48255,48839,49412,49967,50554 returned1/no rows. Executor sent no signals or teardown. All33 historical ledgers retain exact length/hash/dev/inode; old canonical exit2 remains unchanged.

## Final-gate blocker and exact continuation decision

After all8 checks passed, command060 ran:
`/usr/bin/make test-formal-parity test-licenses TARGET_DIR=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target`.

Actual exit2. stdout began the test-formal-native prerequisite:
`cargo test --offline --locked --test formal_models --test formal_nonvacuity --test reactor_projection`.

stderr:
```
error: process didn't exit successfully: /opt/homebrew/bin/sccache .../rustc -vV (exit status: 2)
sccache: error: path must be shorter than SUN_LEN
make: *** [test-formal-native] Error 101
```
The ellipsis above abbreviates only the Rust toolchain path;060.stderr retains exact original bytes. The failure occurs before Rust compilation/formal exploration, so there is no current-source formal verdict. TLC/parity and license checks did not run. The outer canonical chain also exited2 during build-dream-proof-tools; its later test-dream-proof-tools/formal/license commands were never reached.

060.command.json selects CARGO_HOME=/Users/feral/.cargo, whose config.toml:2 sets build.rustc-wrapper=/opt/homebrew/bin/sccache. RUSTC_WRAPPER is absent from the command environment. HOME and TMPDIR each name the122-byte R/public-home path. The offending socket pathname was NOT serialized; blaming one exact path is unproven.

The narrow runner fix is explicit RUSTC_WRAPPER="" for final gates; Cargo documents that an empty value disables a configured wrapper. [Cargo environment variables](https://doc.rust-lang.org/cargo/reference/environment-variables.html). No global Cargo configuration or cache was changed.

Evidence:
-060.command.json977B SHA256 `164ca15ae2d5977e8fe5fca1a993a616876da3f04aa0cd366535d61b68f5346d`;
-060.exit.json181B SHA256 `10f6886d068575b8eaa246924631d3f11e0785bf555ca2cd6f75b64d29227ba4`;
-060.stdout190B SHA256 `8a20f07f3d7cb07259cf7281802ec216242c64ec54530f3630c4096a93e587a0`;
-060.stderr263B SHA256 `0e0803ec81065cd71453252163922ddb97cdb4e48f4a8b84af6aadb00b640339`;
-E/final-gate-diagnosis.json SHA256 `cbf7e21a59454a7621f63df8fea1788c8d1934ad7d36becd80275940413c936a`.

Decision needed: authorize a final-gates-only continuation of THIS retained run with only that wrapper override, preserving060/exit2, revalidating/reusing the eight original native records and continuing qualification/receipt without another native invocation. The current recipe ordinarily re-launches native cases in qualify_candidate:3456 and rejects retained terminal failure in previous_build:724–725. Moreover publish_candidate:3050 uses exclusive R/terminal.json AFTER publishing the prefix receipt, while that truthful failed terminal marker already exists. A continuation must explicitly preserve that marker and define a collision-safe append-only completion record; blindly invoking the normal path could fail after receipt publication.

No unapproved continuation, cache-env patch, source-binding relaxation or native invocation occurred. The ruling limits this change to the clock restriction and authorizes ONE new invocation, now consumed. The real script and retained recipe still equal SHA a8187a0c…; changing the production recipe would change the currentness tuple. Source/currentness and original effect identity must remain truthful in any authorized continuation.

Final `git verify-commit --raw HEAD`, `git diff --check` and `git status --short` exited0: Good Boxy c50efb4e and empty tree. R/terminal.json truthfully records unsupported, effect_unknown=false, reason060 failure. P has neither qualification.json nor receipt.candidate.json. Candidate/runtime approval and whole-command lifetime proof remain false.

E/verify.log10469B SHA256 `20c49fe0236f1fa50707886041e248f1bc4978d5a76dddaa2e38aa8a8ae36015`. E/result-evidence.json SHA256 `17225997801ea0923fa9c6ee4243db75d254ad5e1f86bc2104760e4abb702e19` binds all8 raw results/ledgers/wait4/terminals/pause fields and history. E/handoff.json691B SHA256 `4cededbe4b20e588550dec565a0d5c19c7d0fbb248f12239c4ae18d71c3af240`. The native suite is green; the complete canonical task remains unverified at its final gates.
