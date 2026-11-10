---
type: report
title: 'One integration-test binary per crate: implementation and blocked landing'
task: '[[tasks/2026-11-06-one-test-binary-per-crate]]'
author: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
created: 2026-11-07T17:34:20.366Z
commits:
- 5a6197a82b5d838fb81dd6e265df4b334ae92dc3
- 5bd9e57b31624672b1e0adfeb31d504a20d24115
- edd1c60e3e636be0b9aada9991b6e005c8b959b4
- 5b7bd86de89fb414087dcb674d2c523889e88355
- cb7a85e1c8f77fcd01b95d3ea72afe70f15e5836
verify:
  command: cargo nextest run -p bboxy450-term-mux -p bboxy450-term && python3 -I scripts/test-landing.py && make test-formal-contracts
  exit_code: 100
id: 01M6VH6MWDHVTS63JC9VDJP4SW
updated: 2026-11-07T17:34:20.366Z
number: 93377961366286
created_by: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
updated_by: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
authorship:
  signer: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
  kind: seat
  public_key: 7JeS6TZhOLmSrCs9qgPxsHiFBVIgPbJOs0AVMPBDueA=
  process_generation: '1'
provenance:
  digest: sha256:c39b6dc1f22032b2391e5371e1a6d5fd8e1b21e78ba3cb14d20aafcdb6e8b455
  frontmatter_digest: sha256:f371fc2f042065d558f33b1789d77f64fd5db79f4f113ce4d6baa47f677e86cf
  signer: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
  sig: 94Mxqf5hs2+DBtatiGPqzXkDrZR5/kkdEBNDpMNZj8y1cdrMAboar1D1RZO3ygPvspT2epW1VSGrcHzLLlixCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150401
  event:
    id: 01M6VH6NSD8KDQQYZVVW40M9C6
    sequence: 150401
    hash: 5eba55f0f8bcaee720aee4061001c26cda5a81b47ce01f72dfea403236a65d01
---
Implementation is complete within the selection-only brief; landing is blocked. The current Make landing gate stops on two app assertions also reproduced in the original, unconsolidated checkout. No source push, installation, release, gate relaxation, numerical budget change or product-test assertion change occurred. Selection fixtures were extended; the historical selector comparison normalizes the migration while retaining completeness and uniqueness.

## Source and scope

Current source: `cb7a85e1c8f77fcd01b95d3ea72afe70f15e5836`, rebased on `8ced58cb2ed3e710d0351ce114469c3b8d6296ea`. The harness pin remains `38d99096f4f9c475a5547cf2192982af6ff7a5c0`. All five task commits have approved Boxy signatures; `current-signatures.json` records five `git verify-commit` exits 0. The worktree is clean. Every task commit carries this report's trailer.

The rebase brought upstream GUI work, a new `tests/idle_cpu.rs` and an associated Make recipe. The final task commit includes that file in `it` and migrates its recipe, preserving both witnesses and their existing ignored mode. The first landing attempt was cancelled while linting when this new file was discovered; its exact owned process tree and termination are recorded in `obsolete-main-land-processes.json`. It did not push.

- `crates/bbt-mux/Cargo.toml:2,61` and `crates/bboxy450-term/Cargo.toml:2,70`: disable automatic integration targets and declare `it`.
- Both `tests/it.rs` entrypoints include the existing source files with `#[path]`. Mux `tests/common/` and app `tests/support/events.rs` compile once in their respective shared target. Existing source paths remain present.
- `crates/bboxy450-term/Cargo.toml:74` retains the sole binary exception, `scrollback`. Its unchanged `tests/scrollback.rs:50` installs `#[global_allocator]`; linking it into `it` would replace every other app test's allocator. Its six tests remain isolated in their original target.
- Existing self-reexec helpers now use their exact qualified module names, including nested cockpit helpers and Python/shell fixtures. The separate document-corpus executable keeps its independent test names.
- The native-signing test's private Make fixture also copies the existing `scripts/qos.sh` dispatcher. Its tools remain fake; it does not access a Keychain.
- `Makefile` migrates module execution, compile-only artifact selection and direct copied-binary invocations. Ignored flags, thread modes, causal controls, numerical budgets and assertions are retained. `LAND.md` documents module selection and the allocator exception.
- Only diagnostic command strings changed in two formal-contract helper files and one ordinary ignore reason. All 11 `KNOWN DEFECT` ignore attributes remain byte-for-byte unchanged. No original test's ignored flag changed.
- The task changes no production `src/` files, model definitions, contract paths, coverage mappings or harness pin relative to the rebased main. The upstream GUI commit's new models and contract are included in fresh formal evidence.

## Test identity and selection evidence

At the requested QoS base `11b57036170c94a42f7faf5d4162718f517fa940`, actual discovery was 52 mux and 39 app integration binaries, rather than the brief's estimates of 51 and 38.

| Discovery at measured checkpoints | Before | After | Ignored before/after |
|---|---:|---:|---:|
| Mux integration targets | 52 | 1 (`it`) | — |
| Mux all listed test binaries | 53 | 2 | — |
| Mux test cases | 960 | 960 | 72 / 72 |
| Mux cases excluding the acknowledged four | 956 | 956 | — |
| App integration targets | 39 | 2 (`it`, `scrollback`) | — |
| App all listed test binaries | 41 | 4 | — |
| App test cases | 776 | 776 | 122 / 122 |

`final-test-discovery-equivalence.json` compares every normalized test identity and ignored flag in the before/after per-crate nextest lists: no additions, removals or changed flags at the measured checkpoint. Normalization strips exactly one owning module from `it`; nested names and standalone targets are preserved.

The final rebase adds nine upstream app cases, including one ignored native idle witness. Current grouped discovery is mux 963 and app 785: the mux's three additional library-model cases arise from selecting the app's unchanged model dev-dependency in the joint command. `rebased-discovery-equivalence.json` compares the original lists with the current full grouped list: all original identities and flags survive, with exactly those twelve explained additions. Current integration targets remain mux `it` and app `it` plus `scrollback`; all 40 current app source files are accounted for.

`python3 -I scripts/test-landing.py` on current source exits 0: 219 fixtures, zero failures (`rebased-landing-fixtures.log`). The real private `make land` fixture changes only `alpha`; unselected `beta` panics if execution is widened. Nested files, shared helpers, source/lib/bin combinations, standalone allocator selection, safely quoted filters, ignored-only empty/ordinary/malformed failures, constrained quarantine retries and effective native QoS are covered.

`python3 -I scripts/test-night-boundaries.py` exits 0 (four tests). `final-night-selection-equivalence.json` preserves all 112 effective historical selectors with identical normalized hashes. The historical selector fixture itself is unchanged.

## Cold build measurements

Each row starts from a newly empty, task-owned build tree and runs `cargo nextest run --locked -p <package> --no-run`; discovery follows. All four builds exit 0. These samples use the unoptimized test profile with debug information, two Cargo jobs, the normal Darwin policy and optional shared sccache. CPU is child user+system time; a separate sccache server's CPU is outside it. Host load and cache hit rates are uncontrolled, so these are operational samples, not isolated causal speedups.

| Package | Before wall s | After wall s | Before CPU s | After CPU s | Before `debug/deps` KiB | After KiB |
|---|---:|---:|---:|---:|---:|---:|
| bboxy450-term-mux | 1062.990 | 433.886 | 740.300 | 417.565 | 8,068,968 | 2,165,264 |
| bboxy450-term | 935.260 | 1019.129 | 547.720 | 304.502 | 6,452,228 | 5,620,112 |

Whole build-tree KiB: mux 8,158,416 → 2,253,896; app 6,645,536 → 6,147,004. The app wall sample increased.

Before source is `11b57036…`; mux after source is `0874578799ea8fea2fac5ecea0a770f310e82d69`, and app after source is `af14191151546aa87a4dd7351da8bccf4a84604f` with the allocator exception retained. These are explicit measurement checkpoints, rather than the final rebased source. Before/mux-after kept Cargo's artifact root separate from the empty intermediate tree; final app-after co-located them and re-exported `CARGO_TARGET_DIR`. The disk paths and target layouts therefore differ, and cache behavior can differ. The table reports actual measured `debug/deps` allocation without claiming a controlled disk/time ratio.

Before clocks are `/usr/bin/time -lp` values in `before-*-time.log`, corroborated by Cargo elapsed output. Corrected after clocks use `landing.py cost` monotonic time plus an independent monotonic/rusage check (`*-cost.json`). Earlier after attempts with implausible 0.02/0.04-second POSIX wall values and the allocator-inclusive app target are superseded and excluded from this table.

## Controlled lean landing measurements

Real, unmodified `make land` ran on private local clones and bare origins, with throwaway signed identities, the same harness pin, cold co-located targets, cache disabled, Make's existing O1/debug0 profiles, two Cargo jobs and two test threads. Each candidate changes the same two source files by a comment: mux `attention_policy.rs` and app `ui_status_probe.rs`. Both gates execute exactly ten tests (9 + 1), and both private origins advance to their signed candidate. No public remote is used.

| Same two-file cold lean cut | Wall s (independent monotonic clock) | CPU s | `debug/deps` bytes | Whole target bytes | Make exit |
|---|---:|---:|---:|---:|---:|
| Before, source `11b57036…` | 2231.041 | 3813.670 | 2,413,408,256 | 2,661,154,816 | 0 |
| After, source `693b9625…` | 2410.500 | 4049.910 | 2,581,626,880 | 2,827,984,896 | 0 |

This cold changed-file cut is 179.459 wall seconds and 236.240 child CPU seconds higher after consolidation. Module filtering narrows execution; it still compiles the complete shared integration target. These data do not establish a faster lean landing.

Before candidate: `12de57b82be38db1d1d3252f2cadbcf4c61a0d19`; after candidate: `cdce0e2464c78426a060558575e7e1be7e2aa610`. `land-20261107T152000Z-d3369f27/summary.json` records exact sources, commands, local-origin heads, clean worktrees and valid measurements. Each public test key is retained; only its recorded private agent and temporary home were stopped/removed. Build trees were pruned only after measurement. Measurement source `693b9625…` is retained by local ref `refs/bboxy/measurements/one-test-binary-01M6TSKPNQQ8CQC7Z4S3RRVQB1`.

The final real source landing is separate: `make land` exits 2; its gate script exits 77 and its test command exits 100. `current-main-land.log` records 151.017 seconds / 132.698 child CPU seconds / +4,112,384 bytes for the landing script, and the outer command records 151.429 seconds / 133.087 CPU seconds / the same disk delta. It stops before push.

## Current verification and blockers

Current checks use the existing Make dev/test O1 and debug0 settings; no timing limit or assertion was changed.

| Command / evidence | Actual result |
|---|---|
| Assigned full verify string, `rebased-task-verify.log` | exit 100; Rust failure prevents its chained Python/formal steps |
| Full two-package nextest run | 1,553 executed: 1,547 passed, 6 failed; 195 skipped |
| `python3 -I scripts/test-landing.py` independently | exit 0; 219 fixtures |
| `make test-formal-contracts` independently | exit 0; `rebased-formal-contracts.log` |
| `make test-formal FORMAL_REUSE=1` | exit 0; 681 models, 34 crates, 255 contracts; native/TLC parity held |
| `make check-formal-evidence` | exit 0; clean current source bound to receipt below |
| `make land` | exit 2 / script gate 77; lint succeeds; app tests fail before mux execution/push |

Fresh formal evidence is bound to current source `cb7a85e1…`, SHA256 `5a76b46fbfd9a7acb9ff7204d4876f21e61e5de85b168809e3caff704b85c327`. Its persistent archive is under `/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/5a76b46fbfd9a7acb9ff7204d4876f21e61e5de85b168809e3caff704b85c327/`. Reuse is explicit and model/tool/source hashes are checked. This is finite model evidence, not arbitrary Rust refinement.

The six current full-run failures are:

1. `harness_theme::harness_theme_worker_preserves_identity_draft_and_latest_aba` — expected no agent/model operations, observed one.
2. `rich_text_safety::rich_text_safety_private_mcp_expansion_and_inline_paint` — literal-tail renderer assertion.
3. `rich_client::delayed_cancel_cannot_deny_a_new_approval` — epoch positive-calibration wait times out after stale session cancellation is refused. Parent and child each select exactly one correctly qualified test. The session case succeeds; retained epoch fixture operations contain no `ResolveApproval`. Cause remains unproven. This case passes in the final landing run.
4. `harmony_host::local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks`.
5. `harmony_host::held_private_git_hook_keeps_other_swarm_input_resize_and_status_live`.
6. `storage_bus_index::mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes`.

The last three match the acknowledged pre-existing mux failure task. The fourth acknowledged priority-burst case passes. The first two are also reproduced, with original unqualified names and original separate binaries, in the before checkout's post-measurement probe (`before/probe-0.log`: 4 run, 2 passed, 2 failed, exit 100). That probe occurs after freezing the landing metrics. They are the only two failures in the final real landing run: 656 app tests execute, 654 pass, 2 fail. The unchanged scrollback file is outside this lean changed-file selection; its six cases execute successfully in the full run.

Earlier `693b9625…` full/isolated probes also observed C1 CPU-budget refusals in unchanged library code (1,999,875 ns transaction unit, then 1,074,500 ns SQLite-open unit against 1 ms). The original-source private probe and latest rebased full run pass that case. Earlier provenance Git temporary-file failure also passes isolated and in the latest run. No causal attribution to the migration, hidden deferral, new quarantine or budget change is made.

Additional affected-consumer checks at `693b9625…`:

- `make test-rich-cancel-control`: exit 0, expected causal assertion exit 101, exact source restored.
- `make test-capability-control`: exit 0, expected red 101 and restored green 0, restoration hash checked, no operator input.
- `make verify-harness-attachments`: exit 0, producer/runtime/model parity, native worker, private clipboard and strict consumer lint. Its tracked generated JSON was copied into evidence then restored byte-for-byte.
- `make test-rich-controls`: initial cold compile hit its unchanged 120-second command timeout; after warming the unchanged selection, exit 0 with both required causal assertion exits 101.
- `make test-harness-status-control`: initial cold compile hit its unchanged 300-second command timeout. Warm execution still exits 2: the mutant reaches the intended cadence assertion (101), but restored execution fails (101) because its isolated process misses `session_started`. Selector activation is proven; the restored native boundary remains unproven. No third retry and no deadline/assertion change.
- `make test-licenses`: exit 0 at the implementation checkpoint; notices/policy remain intact. Night selectors: four tests, exit 0.

No full workspace `make test`, UI parity gate, release, install or operator-driven probe was run. The scope is test organization/selection. All commands above ran unattended with private fixture identities and resources. Current broader GUI changes belong to the rebased main and have their own evidence.

**Oracle disposition needed:** assign or explicitly authorize investigation/fixes for the failing app fixtures/epoch calibration and the restored native status boundary before landing. The current brief restricts scripts to selection adaptation and explicitly says “No gate, budget or assertion change.” The Executor has not widened that scope or bypassed a red gate.

## Every changed script

| Script | Mechanism |
|---|---|
| `scripts/e2e/P2.sh:11` | Qualified direct gate test names; unchanged budgets. |
| `scripts/gates.sh:48` | Qualified gate in `it`; correct nextest target-dir placement. |
| `scripts/invariants.sh:175` | Historical exact selectors resolve to qualified modules; explicit standalone targets remain unchanged. |
| `scripts/land-fast.sh:61` | Safely passes quoted selection to `lean-test`. |
| `scripts/landing.py:554,616` | Module/helper/standalone plans; argv serialization and execution; mixed batch QoS. |
| `scripts/night-boundaries.d/bboxy450-term.sh:5` | Qualified exact app selectors; modes retained. |
| `scripts/night-boundaries.d/bbt-mux.sh:7` | Qualified exact mux selectors; modes retained. |
| `scripts/qos-policy.py:110` | Former target timing policy follows the module inside `it`; library policy is unchanged. |
| `scripts/quarantine.py:145,246,381` | Exact retry argv, actual ignored-only selection, uniquely observed qualified-name resolution; expiry/retry policy retained. |
| `scripts/rich-cancel-control.py:27` | Same exact Cancel witness in `it`. |
| `scripts/rich-host-controls.py:22` | Same exact PTY fallback witness in `it`. |
| `scripts/test-capability-control.py:20` | Same exact environment-leak witness in `it`. |
| `scripts/test-dream-c1.py:3422` | Module filters, retained ignored/thread modes. |
| `scripts/test-harness-status-control.py:27` | Same exact cadence witness in `it`. |
| `scripts/test-landing.py:444,528` | Inventory, selection, real landing, QoS and fail-closed retry/ignored-only controls. |
| `scripts/test-ledger-fixture-controls.py:33` | Same qualified hook broken control. |
| `scripts/test-night-boundaries.py:30` | Semantic selector equivalence and dispatch checks. |
| `scripts/verify-harness-attachments.py:46` | Attachment module filter and strict `it` compilation. |

## Committed files and evidence

Task commits, all signature checks exit 0:

- `5a6197a82b5d838fb81dd6e265df4b334ae92dc3`
- `5bd9e57b31624672b1e0adfeb31d504a20d24115`
- `edd1c60e3e636be0b9aada9991b6e005c8b959b4`
- `5b7bd86de89fb414087dcb674d2c523889e88355`
- `cb7a85e1c8f77fcd01b95d3ea72afe70f15e5836`

Changed paths relative to rebased main:

```text
LAND.md
Makefile
crates/bboxy450-term/Cargo.toml
crates/bboxy450-term/tests/capability_file.rs
crates/bboxy450-term/tests/cockpit.rs
crates/bboxy450-term/tests/cockpit/launch_environment.rs
crates/bboxy450-term/tests/formal_contracts/gitkb_models.rs
crates/bboxy450-term/tests/formal_contracts/plugin_models.rs
crates/bboxy450-term/tests/harness_theme.rs
crates/bboxy450-term/tests/it.rs
crates/bboxy450-term/tests/native_signing.rs
crates/bboxy450-term/tests/recovery.rs
crates/bboxy450-term/tests/rich_client.rs
crates/bboxy450-term/tests/rich_text_safety.rs
crates/bboxy450-term/tests/swarm_workdir.rs
crates/bboxy450-term/tests/swarm_workdir/agent_context.py
crates/bboxy450-term/tests/test_account.rs
crates/bbt-mux/Cargo.toml
crates/bbt-mux/tests/adapters.rs
crates/bbt-mux/tests/capture_busy.rs
crates/bbt-mux/tests/capture_fairness.rs
crates/bbt-mux/tests/checkpoint_scheduling.rs
crates/bbt-mux/tests/dream_c1.rs
crates/bbt-mux/tests/gates_seats.rs
crates/bbt-mux/tests/gui_events.rs
crates/bbt-mux/tests/harmony_host.rs
crates/bbt-mux/tests/hook_gate.rs
crates/bbt-mux/tests/identity.rs
crates/bbt-mux/tests/identity_io.rs
crates/bbt-mux/tests/identity_local.rs
crates/bbt-mux/tests/input_latency.rs
crates/bbt-mux/tests/it.rs
crates/bbt-mux/tests/knowledge_provider.rs
crates/bbt-mux/tests/logic.rs
crates/bbt-mux/tests/mcp.rs
crates/bbt-mux/tests/measurements.rs
crates/bbt-mux/tests/messaging.rs
crates/bbt-mux/tests/native_unlock.rs
crates/bbt-mux/tests/plugin_attribution.rs
crates/bbt-mux/tests/plugin_control.rs
crates/bbt-mux/tests/plugin_messages.rs
crates/bbt-mux/tests/plugin_setup.rs
crates/bbt-mux/tests/plugin_transport.rs
crates/bbt-mux/tests/provenance.rs
crates/bbt-mux/tests/provider_gui.rs
crates/bbt-mux/tests/recovery.rs
crates/bbt-mux/tests/seat_wiring.rs
crates/bbt-mux/tests/storage_bus_index.rs
crates/bbt-mux/tests/swarm_metadata.rs
scripts/e2e/P2.sh
scripts/gates.sh
scripts/invariants.sh
scripts/land-fast.sh
scripts/landing.py
scripts/night-boundaries.d/bboxy450-term.sh
scripts/night-boundaries.d/bbt-mux.sh
scripts/qos-policy.py
scripts/quarantine.py
scripts/rich-cancel-control.py
scripts/rich-host-controls.py
scripts/test-capability-control.py
scripts/test-dream-c1.py
scripts/test-harness-status-control.py
scripts/test-landing.py
scripts/test-ledger-fixture-controls.py
scripts/test-night-boundaries.py
scripts/verify-harness-attachments.py
```

Raw logs, lists, fixture identities and measurement drivers remain outside Git under `/Users/feral/Library/Application Support/bboxy450-term/run/implementation-workspaces/checkouts/01M6TSKCHEVR1G5F08151055BM/target/one-test-binary-evidence`. The persistent formal archive and this committed report retain current-source evidence beyond temporary build cleanup. Machine/tool versions are in `toolchain.json` (arm64 macOS, Rust/Cargo 1.99.1, nextest 0.9.150).

Selected evidence SHA256:

```text
d27a5e90b260be4a863d377040d100202ae4d23cd7aea12c1d59200fe5dd7ad8  rebased-checks.json
1bbde9e65376534cfb49fb9ec897ecec5767e0d07646fa6afbd53adba4231557  rebased-discovery-equivalence.json
3e52b4a18b445270bb4bfdf96a06bc75bd7f0365b8b77378a69d93716988bf28  current-signatures.json
8844a7d11a040eea5d5d06e7c196f3988f245be044a16eb726619fc6c904c12e  rebased-task-verify.log
2dd932077943e7018927bdcfda943715ac77e3e2965ccdc32cda5e71d718a784  rebased-formal.log
28690d35a4c6c91a4c56c98e4756431e226a12ef28d2f43eb7b315d011547d22  current-main-land.log
296ea1a387fd82fd1dce483c95044f5c579a903a5eef8a8fe966003ab72e7937  land-20261107T152000Z-d3369f27/summary.json
```
