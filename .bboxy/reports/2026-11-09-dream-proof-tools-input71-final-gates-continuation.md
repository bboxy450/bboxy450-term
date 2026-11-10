---
type: report
title: 'INPUT71 final-gate continuation: Rust tests pass; upstream formal SOURCE pins block'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T20:25:54.531Z
commits:
- c50efb4e155b919f1d5a34620465ce14be725bd6
verify:
  command: /usr/bin/make test-formal-parity test-licenses TARGET_DIR=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target
  exit_code: 2
id: 01M70ZT7GNCPY1W6BJM84EMV1J
updated: 2026-11-09T20:25:54.531Z
number: 93377961366498
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:c236622574a376a330ed4f98459433cd887d7241adf18b74251d2b8e6c591772
  frontmatter_digest: sha256:5cb3dac2361492affaac294a71b9952e4d73dda5a7ecfc4b8f9ade9bf95b85d4
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: wCcxm3R/5ZO7vbhtMH0eBqObzAa5Wuq2FHWArdHPKMFa18WkgEfLR9yIlRs+DZ9ATGbNolnhKAjepOOhSRswAg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151159
  event:
    id: 01M70ZT8PJJNN6N42SC98BSDWZ
    sequence: 151159
    hash: 5bf066aad7a67423dbfa462a27944e162eea9781882c4bfb1d9ee9cee2f0ce94
---
The authorized final-gates-only continuation revalidated all8 original native records and ran exactly one new gate command061 with RUSTC_WRAPPER="" as its sole environment delta. The sccache failure is resolved: Rust compiled and all82 Rust tests passed. The gate then refused at the formal tool's upstream SOURCE-pin check. Continuation exit2; no native rerun, source/recipe change, qualification or receipt.

This is an additional result for [[reports/2026-11-09-dream-proof-tools-input71-clock-correction]] (#66496). Original canonical command060/exit2 and R/terminal.json remain byte/identity unchanged. Production HEAD is still signed `c50efb4e155b919f1d5a34620465ce14be725bd6`, recipe SHA256 `a8187a0ca0b093bdf4e82c2f3e172b664f60a4e5fadc2cb0d50256b8c5d62edd`.

Authority: [[tasks/2026-11-05-dream-proof-tools-provisioner]], “ORACLE RULING (2026-11-09): INPUT71 all 8 native checks PASSED; final-gates-only continuation”, etag `3a186fe85c4360ae6480556f25d1067549f7e0b8948e0d476085bbfdf2eb77a7`. Its constraints remain binding: no native replay, no source/recipe change, only the wrapper environment delta, append-only completion and no receipt unless final gates pass.

Evidence roots:
- E = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input71-clock-correction-20261109`.
- R = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M70XWHWKFCD1W6ZZMQB50WG1`.
- P = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M70XWHWKFCD1W6ZZMQB50WG1`.

## Retained native records and currentness

E/continuation.py is a private continuation instrument, not a production recipe change. It loads the verified production module at its original path (so ROOT/FIX remain correct) and calls existing validators. Before command061 it checked:

- HEAD=c50efb4e, tracked diff0, current script=retained R/recipe.py=a8187a0c; exact original fixture/source-lock tuple.
- Original prefix device/inode/incarnation; Basis and selected INPUT71 source/executable/preparation; live platform/shared-cache binding, compiler/SDK, git/gpg/shim identities and source-review findings.
- Exact eight cases/order and equality to the independently retained E/result-evidence.json whose SHA256 is `17225997801ea0923fa9c6ee4243db75d254ad5e1f86bc2104760e4abb702e19`.
- Every descriptor/report/stderr/wait4/ledger FileRef through custody/hash checks, equality to its original component file, persisted result equality, original argv/input hash/incarnation/prefix binding, actual_wait4 PID/raw-status/exit binding and unchanged checked.json.
- Re-execution of the existing component_result and component_terminal predicates, including the count3 clock causal ledger and reap_delay's preserved partial wire.

All8 rechecks exited0. No inputs/deadlines were regenerated; no native function was invoked. The original native outcomes, debts, margins, shim/pause/escape diagnostics remain exactly those in report66496. The eight record hashes and their FileRefs are in the append-only continuation record. The recorded50 successful shim receipts remain historical evidence, not new receipts.

Two pre-gate instrument preflights exited1 before any command was created: the driver initially applied private-data0600 custody to the tracked source file0644, then found outer evidence JSON0644. The source read was corrected to the existing non-private source reader; only owned outer E files were hardened to0600 (directories0700). No R/source/prefix/native artifact was altered. Production recipe bytes stayed unchanged, and only one actual continuation gate061 was launched.

## Actual gate061 result

Actual argv:
`/usr/bin/make test-formal-parity test-licenses TARGET_DIR=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target`.

The original recipe command logger ran it, retaining its real child PID and exit. Exact061 environment equals060 environment plus only `RUSTC_WRAPPER=""`; the driver checked this equality after completion. HOME/TMPDIR/CARGO_HOME/RUSTUP_HOME/PATH/job count and all other values stayed unchanged. No global Cargo configuration was changed.

Rust compilation completed in1m15s. The three test binaries passed:
- formal_models:26 passed,0 failed;
- formal_nonvacuity:55 passed,0 failed;
- reactor_projection:1 passed,0 failed.

Then Make invoked:
`python3 -I scripts/formal.py --native --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/formal`.

It failed:
```
formal: FAILED: upstream main SOURCE hash changed; re-copy scripts/formal.py
make: *** [test-formal-native] Error 1
```

Make command061 and the continuation driver both actually exited2. Rust unit/model tests are green, but formal-native exploration, native/TLC parity, licenses and remaining post-qualification checks are unverified/unrun. No qualification assembly/publication function was called. The prepared publication-continuation.py instrument was not executed.

Gate artifacts:
-061.command.json996B SHA256 `61384a40e5e608171873a2bb2e994f86ec2e17935de55b5caef564a3b786ca4d`;
-061.exit.json181B SHA256 `22685235c92cdaeabe88fec56c6a805ac425d084e545abafb7c0d6127afbbe76`;
-061.stdout7297B SHA256 `b3bf08546236a50240bdc4906eac3109fcc2f8ba97bc3853dd268427962ffcb4`;
-061.stderr7718B SHA256 `f42eb15760340ca8df8563c3b50558023f20d4069f85ace90a5bada1a06f0a2a`.

## Source-pin diagnosis and decision needed

scripts/formal.py:284–295 reads verification/upstream-sources.json. It first verifies each local vendored snapshot payload against its declared SHA, then, when the sibling bboxy450-term repository exists, compares that pin to `git show main:<source>` there. The error is from that second, moving-upstream comparison, not corrupt local snapshots.

Read-only diagnosis captured bboxy450-term main at `96dbc71ba3631ccb7907b3449ca76dd0f6e83749`. The gate itself did not serialize its observed upstream commit/hash, so this later diagnostic capture is not claimed as a continuous observation of main.

| Source | Declared pin / matching vendored payload | Diagnosed upstream main |
|---|---|---|
| scripts/formal.py |09ca7496419d59f0a41245cff8297b7fdb79dba2a9b81abf5df9e4a6597aa937 |5097490db6569929a1e0a10061fb86a88c06a16170fb6534c134c9fcfbf0c27a |
| scripts/test-formal.py |7627077e427cecf992e00bba1c0ed55f13843f897502f54545cd2e5dd9a4aa85 |11c3d433fab19fd3f27565ed85c98cb84b9294756447d2113a1886f6acf677c4 |
| crates/bboxy450-tlc/examples/verify.rs |f45b999db8be8da4a18afd36ef24558c9654f9d58d642d5c30bb1ff7abbdc278 |Unchanged, same hash |

All three local vendored hashes match their declared pins. Both upstream scripts have drifted. E/formal-source-pin-diagnosis.json binds the actual git-show exits0, paths, lengths, hashes and diagnostic timestamp; SHA256 `bd9acafa1143c9454a727ccfdb33a96e94e8618ef6840c64b50c6ef5171b0357`.

This cannot be cleared within the current only-wrapper/no-source-change ruling. scripts/formal.py, scripts/test-formal.py and verification/upstream* are also outside this task's allowed source paths. I did not re-copy them, change BBOXY_TOOLKIT_ROOT, remove the upstream check, alter a SOURCE hash or bypass a formal gate.

Decision needed: provide an authorized formal-tooling synchronization or frozen-upstream disposition, including how its source/currentness binds to the retained c50efb4e candidate, and authorize the next final-gates-only continuation. The8 native records can remain retained/revalidated; no new native invocation is requested. The blocker is the formal tooling's upstream-source premise, not the candidate or sccache now.

## Append-only completion and preservation

The required new completion record was created at:
`R/final-gates-continuation-01M70ZC391WFNE6Z2D8XS5EG5W.json`.

It is12477B, SHA256 `4bc524861b9ff36b8f24e551858ddbecf2f49566ba18bf4d0e5f2a6c0cf58f83`, outcome=final_gate_failed. It binds the run ID, all8 original record hashes/FileRefs, original component-checks hash, failed060 command/exit/output hashes, original unsupported-terminal hash, production/driver hashes, exact environment delta and actual061 exit/output hashes. It explicitly records no qualification or receipt. No existing record was overwritten.

After061, original060 and unsupported R/terminal.json still match their original byte hashes/device/inode. All previously recorded native result/stdout/stderr/ledger/wait4 artifacts and all33 historical ledgers remain unchanged. Prefix qualification.json and receipt.candidate.json remain absent. No publication, additional gate, cleanup, signal or native rerun occurred after the SOURCE refusal.

Final git signature/diff/status checks exited0: Good Boxy c50efb4e, empty tracked worktree. No new code commit was made because production source/recipe changes were prohibited. Original canonical exit2 is preserved; this separate authorized continuation also exits2. Runtime approval and whole-command lifetime proof remain false.

E/final-gates-continuation-handoff.json SHA256 `cbabd980b5d18b84096ea4c2712b644e2560f6d38426f960e4d700aa643c8334` records the outcome and completion locator. E/final-gates-continuation-state.json retains actual Git checks. The previous native suite remains green; full task verification and the private qualification/receipt remain blocked at formal SOURCE provenance.
