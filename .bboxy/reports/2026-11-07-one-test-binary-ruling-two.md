---
type: report
title: 'One integration-test binary per crate: landed'
task: '[[tasks/2026-11-06-one-test-binary-per-crate]]'
author: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
created: 2026-11-07T19:23:41.267Z
commits:
- 5a6197a82b5d838fb81dd6e265df4b334ae92dc3
- 5bd9e57b31624672b1e0adfeb31d504a20d24115
- edd1c60e3e636be0b9aada9991b6e005c8b959b4
- 5b7bd86de89fb414087dcb674d2c523889e88355
- cb7a85e1c8f77fcd01b95d3ea72afe70f15e5836
- 6057d3eedbabb4ad837765f749fdbe3c82f283f3
- a27f56f6e9c7d58252cbb8fc6dab4aa2bbeff496
- 261086dbbc123a5ddb5e1870dd5f496923fce05a
verify:
  command: make land
  exit_code: 0
supersedes: '[[reports/2026-11-07-one-test-binary-landing]]'
id: 01M6VQEV3HJV70352G1E204NST
updated: 2026-11-07T19:23:41.267Z
number: 93377961366293
created_by: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
updated_by: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
authorship:
  signer: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
  kind: seat
  public_key: 7JeS6TZhOLmSrCs9qgPxsHiFBVIgPbJOs0AVMPBDueA=
  process_generation: '1'
provenance:
  digest: sha256:472ec03c17d879deb3259d3eaadfd879423f8feccbcf77813f6cc70eba8d9f84
  frontmatter_digest: sha256:6566a3f0ed8cbe362ae5c5e1cb8db026622cbdde09144d03f85e462a00bc9c2d
  signer: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
  sig: 6R+PnMGtPH1JrrclDwmNb3SJF2iWI6DNoRIOIS+utZ0yu0897+1sAfyPlBwsd0zeIMMx25W9HUMF3q7xGYvWBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150428
  event:
    id: 01M6VQEWY0W2CYN57298PB061J
    sequence: 150428
    hash: b17bf7a154c71b514a39520003b5805ad188d5933fc65aac81083b8273f675e4
---
# One integration-test binary per crate: landed

Landed on `https://github.com/bboxy450/bboxy450-term.git` at signed commit **261086dbbc123a5ddb5e1870dd5f496923fce05a**. `make land` exits **0**. `git ls-remote origin refs/heads/main` returns that exact SHA; the worktree is clean. All eight task commits have GPG status G under the approved Boxy identity/key.

This supersedes the blocked landing disposition in [[reports/2026-11-07-one-test-binary-landing]]. The complete migration, eighteen changed script descriptions, discovery/selection proofs and raw benchmark provenance remain in [[reports/2026-11-06-one-test-binary]].

## Final authorized changes

Authenticated **Oracle ruling 2**, task etag `f63b541805e6704636e71cefc645af6c7afd979ba35021158962f39e75ba2ada`, authorizes exactly seven additional mux ignores, with assertions unchanged. Commit `261086dbbc123a5ddb5e1870dd5f496923fce05a` adds exactly seven attribute lines across three files. Removing those seven lines reproduces parent `a27f56f6e9c7d58252cbb8fc6dab4aa2bbeff496` byte-for-byte in each file (`ruling2-source-audit.json`). No budgets, assertions or runtime implementation changed.

These three use the exact reason `KNOWN DEFECT: pre-existing on main; fix: tasks/2026-11-06-mux-preexisting-test-failures`:

- `harmony_host::held_private_git_hook_keeps_other_swarm_input_resize_and_status_live` — crates/bbt-mux/tests/harmony_host.rs:367.
- `harmony_host::local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks` — harmony_host.rs:959.
- `storage_bus_index::mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes` — crates/bbt-mux/tests/storage_bus_index.rs:338.

These four, in `logic::dream_public_api_tests`, use the exact reason `KNOWN DEFECT: C1 per-unit CPU overrun; fix: tasks/2026-11-07-dream-c1-native-unit-budget-overruns`:

- `dream_c1_actual_mute_writer_and_unmute_aba_do_not_revive_old_grant_page` — crates/bbt-mux/src/logic.rs:14678.
- `dream_c1_actual_root_workdir_aba_and_provider_revision_reject_old_pages` — logic.rs:14705.
- `dream_c1_held_provider_refuses_ordinary_mcp_and_gui_read_admission` — logic.rs:14928.
- `dream_c1_held_provider_suppresses_actual_successful_ordinary_gui_completion` — logic.rs:15079.

None of these seven reexecutes itself, so no child selector change is needed. The earlier two authorized app deferrals, their child `--ignored` arguments and their named diagnostic opt-ins remain intact.

The final source audit preserves all **11 original KNOWN DEFECT attributes** unchanged and counts **20 now**: the original eleven plus the two app and seven mux deferrals authorized in the task. These are deferred defects, not repaired assertions.

## Verification on the landed source

The four commands below ran sequentially on clean source `261086dbbc123a5ddb5e1870dd5f496923fce05a`, with task-local `BBOXY_CACHE_MODE=off`. Profile, test budgets and default test isolation remain canonical.

| Command | Actual exit | Wall s | Child CPU s |
| --- | ---: | ---: | ---: |
| make test-formal FORMAL_REUSE=1 | 0 | 761.486 | 1085.031 |
| make test-formal-contracts | 0 | 775.683 | 1284.127 |
| make check-formal-evidence | 0 | 6.171 | 5.130 |
| make land | 0 | 994.724 | 1086.317 |

The final lean gate passes strict all-target clippy, binary build and **1,535 tests**:

- App: **654 passed, 125 skipped**, three selected binaries.
- Mux: **881 passed, 79 skipped**, two selected binaries.
- Push succeeds and prints `LANDED 261086dbbc123a5ddb5e1870dd5f496923fce05a`.

The landing cost tree separately records **991.975 s wall, 1085.154 s CPU, +334,139,392 bytes** target growth. The table uses the independent outer clock. This was an incremental gate after formal preparation, not another controlled cold benchmark.

Formal evidence was refreshed and checked **before push**: **681 models, 34 inventoried crates, 255 contracts**, native/TLC parity held. Unchanged model results were reused through the hash-checked canonical reuse path. Receipt SHA256:

`800371ae2f34232749b1e3a99615a330c056a8385b135114b4f02f8104b680d0`

Persistent receipt:
`/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/800371ae2f34232749b1e3a99615a330c056a8385b135114b4f02f8104b680d0/evidence.json`.

The current formal-contract target also passes strict lint, 19 QoS fixtures and the live QoS probe. Under ten background build workers its MCP p95 is 1.886 ms (limit 250 ms), with the expected background/default priority observations.

## Acceptance evidence retained

- **One integration binary, documented exception:** mux declares only `it`; app declares `it` plus standalone `scrollback`, whose global allocator must remain isolated. Final registry audit covers **all 52 mux and 40 current app root files**, with none missing. Shared helpers have one root module. No test source files moved.
- **Test identities/counts:** the original same-source nextest comparison is mux **960 → 960**, app **776 → 776**, with no removed identities or changed ignored flags during consolidation; excluding the four tracked mux identities gives 956 on each side. Subsequent upstream additions and grouped feature unification explain the later 963 mux/785 app discovery counts. The nine newly authorized deferrals deliberately change active/ignored counts; they remove no registered tests. Standalone scrollback's six cases and the three extra feature-enabled mux model cases are outside this final lean selection and were covered by the earlier full/grouped verification. No current full-suite pass is claimed for that earlier failing compound command.
- **Changed-file selection:** `python3 -I scripts/test-landing.py` passed **219 fixtures**, including a real private `make land` that runs only the changed module. The source-selection scripts, Makefile, configuration, manifests and integration registries are unchanged between that verification at a27f56f6 and the landed SHA (`git diff --quiet` exits 0 for those inputs). Earlier selection equivalence covers 112 night-boundary selections, safe filter quoting, QoS, quarantine and invariant consumers.
- **Contracts:** `git diff 8ced58cb..HEAD -- verification` is empty; implementation-test and coverage paths are unchanged. Current `make test-formal-contracts` and formal receipt checks exit 0.
- **Measurements:** retained below and in the original committed report, with exact raw provenance and the acknowledged layout/cache confounds. The Oracle explicitly accepted the disk/time trade-off.

## Before/after measurements

Cold `cargo nextest run --no-run`, two Cargo jobs; all four builds exit 0. Disk units are **KiB**. These measured source snapshots predate the upstream idle_cpu addition and the final deferrals.

| Crate/layout | Integration binaries | Wall s | Child CPU s | debug/deps KiB | Whole target KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mux before | 52 | 1062.990 | 740.300 | 8,068,968 | 8,158,416 |
| Mux after | 1 | 433.886 | 417.565 | 2,165,264 | 2,253,896 |
| App before | 39 | 935.260 | 547.720 | 6,452,228 | 6,645,536 |
| App after | 2 | 1019.129 | 304.502 | 5,620,112 | 6,147,004 |

Whole build tree reduction is about **72% mux**, **7.5% app**. The original report documents intermediate-directory and cache-warmth differences; these numbers do not establish an isolated causal timing ratio. CPU with a separate persistent cache daemon excludes that daemon.

Controlled real cold two-file `make land` runs, with private local origins, identical O1/debug0 profiles, cache disabled, two jobs/two test threads and the same nine mux plus one app selected tests:

| Layout | Exit | Wall s | CPU s | debug/deps bytes | Whole target bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| Before | 0 | 2231.041 | 3813.670 | 2,413,408,256 | 2,661,154,816 |
| After | 0 | 2410.500 | 4049.910 | 2,581,626,880 | 2,827,984,896 |

The controlled cold land costs **+8% wall, +6% CPU**. Module selection narrows execution while the shared integration binary still compiles the suite. The Oracle accepted this trade-off for the full-build disk saving. No cold benchmark was repeated in this follow-up.

## Evidence, operational notes and remaining coverage

Raw logs remain ignored under `target/one-test-binary-evidence/`:
`ruling2-checks.json`, `ruling2-land.log`, `ruling2-land-result.json`,
`ruling2-formal.log`, `ruling2-formal-contracts.log`,
`ruling2-formal-evidence.log`, `ruling2-source-audit.json`,
`ruling2-final-source-audit.json`. Earlier discovery, module-selection and cold benchmark logs remain referenced by the original report.

An early gate was stopped before push to refresh formal evidence first. The next formal attempt encountered an optional-cache stall: its own sccache client waited over six minutes with zero CPU, no compiler/server children and a live stats endpoint. Only verified owned process trees were stopped; records are `ruling2-preformal-cancelled-pids.json` and `ruling2-cache-stall-cancelled-pids.json`. The successful sequence used the existing task-local cache-off mode; no shared daemon was stopped or reconfigured, and no global configuration or tools were changed. Aborted runs are not completed measurements.

Deferred assertions remain unchanged and their dedicated fix tasks retain the outstanding runtime coverage. Explicit C1 diagnostic receipt validation still fails closed if mandatory executed witnesses are absent; it was not relaxed to certify ignored cases. The original exact compound verifier was not rerun after deferrals: the current-source lean landing and formal checks above are the actual new evidence. No full workspace CI, reinstall or release was needed for this test-harness change. The earlier artifact-claim host refusal remains recorded in the preceding report; no new successful responsibility claim is asserted.

All code is on origin/main, the worktree is clean, and no assignment work remains before Oracle review.
