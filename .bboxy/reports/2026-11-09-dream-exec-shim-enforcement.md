---
type: report
title: Dream exec shim enforcement and INPUT70 source handoff
task: '[[tasks/2026-11-09-dream-exec-shim-enforcement]]'
author: seat:01M70FJGXV7W40W80200Z0TDN5
created: 2026-11-09T18:04:52.055Z
commits:
- b665598781ff6edfdf9108360b1561376ab95881
- 81b1225bb0b618abe5dd8b834ef0fe90199d033b
- 6d0abc64119b2b1aad549680757850314bf11bc1
- 76045587db4a96612e2406b9bb08d4191c626253
verify:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
derived_from:
- '[[decisions/d93377961366459-dream-external-helper-shim-enforcement]]'
id: 01M70QQZS16X600PJCN6XSFGWP
updated: 2026-11-09T18:04:52.055Z
number: 93377961366477
created_by: seat:01M70FJGXV7W40W80200Z0TDN5
updated_by: seat:01M70FJGXV7W40W80200Z0TDN5
authorship:
  signer: seat:01M70FJGXV7W40W80200Z0TDN5
  kind: seat
  public_key: ZqTue5OR8Q7BNcheRMErqTQtqa+AL9dxIChIyiTZDGM=
  process_generation: '1'
provenance:
  digest: sha256:4d42c81032dd9357b8b436520cc0482ec096b8d11beb52760710d096404115b9
  frontmatter_digest: sha256:f30df923d700ebc402a610d2f6e6b2d03e184e89916ffa9febf3f17554f6ab12
  signer: seat:01M70FJGXV7W40W80200Z0TDN5
  sig: FW0ktQLQ5BfQq0XHVE9xlVvOYuNRdyJDHd8mEhWPwGNF7C8nT0lIYZ4ElIlgaZiRX/40P2+bkkE7dZ+MbC2eDw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151082
  event:
    id: 01M70QR0HTVKY0XSGFN1KXJFXT
    sequence: 151082
    hash: e553fe696e8ad324ce04e202ea6347c802ba43b54e99f46a7bbf46c948b4d7e9
---
Implemented and landed decision [[decisions/d93377961366459-dream-external-helper-shim-enforcement]], including its Darwin CPU correction and Amendment 2. Final signed head and observed remote main are `76045587db4a96612e2406b9bb08d4191c626253`; worktree is clean. `runtime_approved=false`, `whole_command_lifetime_proof=false`. No proof-tools qualification/candidate builder run was initiated. The specifically authorized private witness Git builds supplied by the builder are separate from qualification.

Worktree: `/Users/feral/mydev/bboxy450-harness-dream-read-q2`.
Evidence root E: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness`.

ONE handoff tuple: `E/INPUT70-source-freeze-tuple.json`, 40,743 bytes, SHA-256 `3d42b6ecbdd2a3b5da9ac27a31a6cc7ed1c971300937616d567a8dd7dda0b7d4`. Its binding audit executed 405 assertions, exit 0; tuple readback also exited 0. The replayable binder is `E/bind-input70.py`; `E/INPUT70-binding-audit.json` binds its hash and the checked inputs.

| Changed files | Mechanism and source evidence |
| --- | --- |
| `tests/fixtures/dream-read/dream-exec-shim.c`, `dream-exec-shim.h` | C source:36,69,115,122 — exact fixed forms and sealed variable operands/environment; rejection 81; owned read-only unlinked configuration FD3 and preopened datagram log FD4; self priority 4; in-process nofollow/regular/0600/euid/nlink1/dev/ino/size/hash/exclusive-scratch GS1; exact checked inode transferred by SCM_RIGHTS before absolute execve; descriptor closure and core limit zero. Verify uses the exact public no-autostart configuration. No RLIMIT_CPU enforcement claim. |
| `tests/fixtures/dream-read/proof-helper-probe.c` | :3006 — observed unexpected fork notification refuses and kills; :3504 — ordinary successful RI counters are a CPU backstop, failed/racy reads ignored and unavailable start remains unavailable; :4156 — owned shim receipts and transferred GS1 inode; :4367 — hash-bound shim, SETPGROUP/background/suspended birth; :4549,4582,4717 — original wait4, CPU/job/RSS verdict, complete 9x bill, retained cleanup/final census and GS2. :4213 keeps the foreground STOP/positive ACK or observe_or_terminal/USER0/SYSTEM<=50us/one CONT path. |
| `src/dream/c1.rs`, `src/dream/resolve.rs` | c1.rs:1804 — ordinary live samples become non-gating on failure; no sample-gap/monotonicity or sampled child-rollup terminal gate. Positive initial custody and pause remain separate. The resolve overrun test loses its obsolete <=50ms assertion. |
| `tests/fixtures/dream-read/prepare-c-probe.py`, `check-c-preparation.py` | :218,255 — qualified source set includes shim C and shared header; strict optimized compile and union header audit; compiler/source/executable/form-table hashes embedded in controller and emitted in preparation receipt. Synthetic preparation controls include the new inputs. |
| `tests/fixtures/dream-read/check-exec-shim.py`, `exec-shim-controls.c`, `exec-shim-tool.c` | Ten unattended native controls with source-known tools; actual shim execution, wait4, custody and production event/live-budget branches. No tool qualification claim. |
| `tests/fixtures/dream-read/check-private-engine-controls.py`, `tests/dream_read.rs` | Shared-header include/copy for existing source mutants; preserve the canonical 114-baseline/108-mutant suite and signature-boundary negative control. |
| `tests/formal/dream_read.rs`, `tests/formal/models.rs`, `verification/{coverage,contracts,corpus}.toml` | dream_read.rs:3827,4065 and contracts.toml:1068 — replace Q2HelperTotalBudget with HelperShimBudget and 19 broken gate controls, including failed-live-read-refuses. Model includes full refused-shim billing and no CONT after kill. Existing foreground terminal companion renamed consistently. Whole corpus remains 446 models, 96 contracts, one crate. |
| `Makefile`, `scripts/land.py`, `ATTRIBUTION.md` | Makefile:340 — canonical native shim controls included in Dream MCP and new sources in its source binding. land.py:295 — formal/runtime gates reuse the explicitly selected build tree. Attribution and compact origin headers retained. |

The production shim path is `<candidate>/libexec/dream-exec-shim`. Both Git gpg program options point there. The builder must compile Git with `DREAM_PROOF_GPG=<candidate>/libexec/dream-exec-shim`, reusing its existing projections. The builder recipe's compiler define selects this path; no Git patch-logic change is needed.

The preparation ran AFTER successful landing, exit 0: `E/final-preparation.{json,log}`. Its receipt is `E/prepared-input70/dream-read-c-preparation/compile-u3u9hnyr/preparation-c24749c7481562f853f776dc0004d8197bd25efc3b6508890dfdcf94f228a56b.json`; receipt SHA matches that filename. It retains all 326 audited header inputs and actual strict compiler argv/stdout/stderr. Probe SHA-256: `aa6e1b28bdc60cf2b4154df616768dfaad743fb51c3028e74a6067a94c8db00b`. Shim executable SHA-256: `4e95b2a83a34a6328d3c794133c3569dff7e5f1b2ee2c9bfc1d58cd08447527d`; shim source: `34a9a6ee959843fc3192d64fac05b5439175ffb8dd15cc927b65fd4a81b102cc`; allowlist/form-table digest: `5c9b450c3b1f9558cee49f154d8c771e525ccb8b9303cface9414f957a6cefc9`; compiler: `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`. The final shim executable is byte-identical to the real cryptographic witnesses' shim.

The canonical added private-basis member is exactly **343 bytes**: `shim.allowlist_sha256`, `shim.compiler_sha256`, `shim.exe_sha256`, `shim.source_sha256`, all strings of 64 lowercase hex digits. The compiler hash reuses the existing compiler binding; projections are not restated. From the Oracle's 47,112-byte INPUT69 basis this projects to 47,455 bytes if existing projections otherwise retain their sizes. No new builder basis was generated. The basis ceiling is 57,344 bytes, its non-shim portion remains capped at 49,152 bytes, and the combined ceiling remains 131,072 bytes.

| Acceptance evidence | Actual result |
| --- | --- |
| Required final-head command | `make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build` — exit 0, 2026-11-09 17:04:33–17:35:01 UTC. Environment: `TARGET_DIR=target/harness-landing FORMAL_REUSE=1`. `E/required-four-v2.{json,log}` binds final head. Reuse checks matching model bytes/reference graph outputs; no gate or bound was relaxed. |
| Native/Java parity | 446 complete reachable state/labelled-transition and verdict comparisons, 3,018,457 states and 10,271,690 labelled edges. Current archive `verification/evidence/352f40b4c82aa1b50bca2dd39ba0c7871c1f3d8908040b4f44f31f7da3fc9687`. `E/current-formal-all-raw-files.json` independently hashes all 4,462 retained files, 6,176,630,734 logical bytes. Includes the good HelperShimBudget and all 19 expected-safety controls. Finite parity is not full TLC compatibility or arbitrary OS/Rust correctness. |
| Canonical C preservation | `target/harness-landing/private-engine-controls-_vxxz7tm/run.json` — passed, sources unchanged, 114 baselines, 108 mutants, 288 recorded commands. Landing rerun independently passed `private-engine-controls-901blrdc/run.json`. Includes original pause/terminal/refusal/debt witnesses; supplied API outcomes remain labelled supplied. |
| Shim controls | `target/harness-landing/exec-shim-controls/run-g30zove4/result.json` — three compiler commands and ten native control commands, all wrapper exits 0. Child argv/daemon, env, QoS and 0644 custody mismatch statuses are 81 with zero receipts. A supplied failed RI outcome on an actual owned process is ignored and terminal status remains 0. The terminal-only CPU loop bills 360,415,000ns and refuses. The successful live sample reads 250,525,291ns, kills the group, and actual wait4 bills 250,553,000ns and refuses. Two deliberately uncoalesced fork notes invoke the production refusal/kill path. Landing independently passed `run-r86jxpyp/result.json`. |
| Actual pinned tools | `E/git-shim-crypto-pwa5v219/result.json` — private Git 2.54.0 -> exact final shim -> pinned GPG 2.5.24. Both valid and corrupt commands yield two shim receipts, in-process GS1 and retained-FD GS2. Valid output is U with exact signer/primary fingerprints; corrupt output is B plus empty fingerprints. Git log exits 0 in both cases; B is the corrupt cryptographic refusal. Actual CPU/RSS: valid 38,051,000ns / 4,128,768B; corrupt 25,591,000ns / 4,112,384B. These are real-tool/custody witnesses, not a full candidate qualification. |
| Git argv/env assumption | `E/actual-git-child-env.json` records the exact Git-generated verifier environment. Shim validation recognizes only the fixed verify argv and exact derived config/exec-path/pager/prefix/path variables, then restores the sealed original GPG environment. Git witness B SHA `5981ccf53ca713c4c80030f2a872f31696e00a03729e6fc463e31a8f17cd0265`; its authorized build exit and --exec-path exit are 0. Builder's original witness A remains untouched. |
| Amendment 2 limitations | `E/witness.c`, `E/result.jsonl`, and [[research/2026-11-09-dream-shim-fork-census-counterexamples]] — two forks coalesce into ONE NOTE_FORK with data0; a live setsid orphan can survive original wait4 while both censuses are empty. Exact witness child cleanup was recorded. These are negative evidence, not passed escape-detection criteria. Observed unexpected notes/census hits/forms refuse; silence proves nothing. Acceptance stays conditional_on_bound_source_review for the pinned fixed-form tools. No Seatbelt hardening was added. |
| Landing | `make land` — exit 0 on final signed head, 17:40:01–17:57:40 UTC; `E/canonical-land-v2.{json,log}`, `E/landed-receipt.json`, `E/landed-state.json`. Includes signed-commit verification, licenses, embedded/all-target clippy, 101 concurrent debug tests, every serialized Dream family with nonzero test counts, full Dream MCP and current-source formal receipt. Actual push HEAD:main succeeded. Readonly ls-remote independently matched final head. |

Every terminal/ledger change is recorded in `E/field-changes.json` and included in INPUT70:

- New internal `exec_shim` record: `kind:string` literal, `form:u32` (0..14 parents, 15 verify), `pid:i32`, `self_priority:i32`, `in_process_checks:bool`, `live_read_failures:u64`, `fork_notes_are_diagnostic:bool`, `runtime_approved:bool=false`.
- Existing `verifier_event_discovery` adds `exec_shim_enforced:bool`, `live_cpu_sample_known:bool`, `live_read_failures:u64`. Existing `leader_fork_events:u64` counts notifications, never births.
- No new terminal/public DTO fields or error codes. Existing `private_candidate_result.metrics.members_max:u8` remains diagnostic and may be zero. Existing ledger `final_case_conjunction.members_max_complete:bool` now uses internal shim enforcement OR the legacy sampled-two predicate; production requires the shim.
- Existing `git_sig_temp` types remain: `pid:i32` now comes from the owned in-process receipt; `pidversion:u32` is zero/unavailable in this path, never a fabricated token version; `t_ns:u64` is controller record-publication time, not the in-shim check time. `dev/ino/nlink/size:u64`, `mode:u32`, `name/sha256/phase:string` retain their formats, with GS1 stat/hash sourced from the pre-exec shim and GS2 checked after reap. Sample-gap and sampled child-rollup ledger fields retain their types as diagnostics.
- Private-basis four SHA fields and unchanged ceilings are listed above. Internal C `shim_enforced:unsigned` is not serialized as a new terminal field.

Failures and limits are retained honestly. The earlier four-command run exited 2 because a signature-mutant temporary source copy lacked the new shared header; the copy was fixed, its focused test passed, and the final whole command passed. Earlier exploratory Java parity was invalidated by a moving signed HEAD; only the final current-source receipt above is promoted. The first landing attempt exited 2 before push when a legacy same-wake diagnostic positive reached its unchanged 400,000ns headroom guard with measured 553,917ns. Its exact original process was killed/reaped with group gone. No source, gate or bound changed; the complete next landing passed. See `E/canonical-land.json` and `E/canonical-land-failed-receipt.json`. Link steps also emitted a nonfatal rust-objcopy/libLLVM lookup warning in the private test home; actual test and landing exits above are preserved.

The 29 existing ignored tests in the three retained Q2 source paths remain NONPASSES: 22 QoS, five qualification, one manual and one known defect. No ignore was added. `E/current-source-deferrals.json` rechecks every current name/reason; this is not a repository-wide ignore inventory. In particular, the old ignored overrun test is not promoted by the new source-known CPU-loop witnesses. RLIMIT_CPU behavior is the decision's measured Darwin 27 negative premise; no CPU rlimit is relied upon.

Cleanup: Cargo's package clean refused with exit 101 because the mixed target root lacks CACHEDIR.TAG, before deleting anything. The root was not relabelled as a disposable cache. A selective cleanup script then exited 0, removed 240 task-era package release artifacts identified by dep-info plus the debug tree created by this landing, and retained every frozen input/profile/formal/native evidence directory and other run. `E/cleanup-selective.json` records exact paths/identities and scope; shared release dependency caches remain. Final free disk was 15,080,468,480 bytes (>8GiB). All builds used `target/harness-landing`; the old nested formal evidence was left untouched. `E/source-cut.json` retains all 20 changed files and all four good signed-commit receipts. Report checkpoint and task_report follow this completed handoff.