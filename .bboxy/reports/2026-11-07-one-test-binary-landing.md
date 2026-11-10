---
type: report
title: 'One test binary: landing follow-up'
task: '[[tasks/2026-11-06-one-test-binary-per-crate]]'
author: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
created: 2026-11-07T18:20:58.374Z
commits:
- 5a6197a82b5d838fb81dd6e265df4b334ae92dc3
- 5bd9e57b31624672b1e0adfeb31d504a20d24115
- edd1c60e3e636be0b9aada9991b6e005c8b959b4
- 5b7bd86de89fb414087dcb674d2c523889e88355
- cb7a85e1c8f77fcd01b95d3ea72afe70f15e5836
- 6057d3eedbabb4ad837765f749fdbe3c82f283f3
- a27f56f6e9c7d58252cbb8fc6dab4aa2bbeff496
verify:
  command: make land
  exit_code: 2
supersedes: '[[reports/2026-11-06-one-test-binary]]'
id: 01M6VKW0RMFJ5VKPY1RTJN9CDY
updated: 2026-11-07T18:20:58.374Z
number: 93377961366290
created_by: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
updated_by: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
authorship:
  signer: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
  kind: seat
  public_key: 7JeS6TZhOLmSrCs9qgPxsHiFBVIgPbJOs0AVMPBDueA=
  process_generation: '1'
provenance:
  digest: sha256:4fbf4a0f34830caaf1989b6e870117727f431201147a187ac03b8f9be905f389
  frontmatter_digest: sha256:6d9d62c426d60ab3530d7f516825481d9d57dc1c4be6a7f3ec7ce379959d74a6
  signer: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
  sig: Jgtsgwljnwu2/eS7V/JqQG3kqnISnVmReXAtLkyRRXwhRwYEwX0RntUcsacpqtunsLk51DypSFjIw1dv22n6Dw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150416
  event:
    id: 01M6VKW27FCKCZ43SYD1W7YSRX
    sequence: 150416
    hash: 3a24452693f55d0a002d99158d76945c307e229e6609f351c7d9f1c8d0152bb7
---
# One test binary: authorized deferrals and landing follow-up

This supersedes the landing disposition in [[reports/2026-11-06-one-test-binary]]. The implementation, source-selection proofs, raw measurement references and historical failures remain recorded there. The original report is committed at knowledge checkpoint `f46f46677d3522735977b4cb1ca27c83c321c975`.

**Blocked: the requested `make land` still exits 2, and nothing was pushed.** The app is now green under the authorized deferrals. Mux has three tracked integration failures and four C1 library resource-limit failures. The current brief permits exactly two additional ignores; none of these seven was deferred or bypassed.

## Authority and follow-up changes

The authenticated task amendment (etag `cd7cc59486b3240edc0d458c07670f631127979dcde1bc3caf91f0865bf58351`) accepts the measured disk/time trade-off, authorizes exactly the two app deferrals below, moves the restored native-status boundary to [[tasks/2026-11-06-mux-preexisting-test-failures]], and directs a signed commit, `make land`, then `task_report`.

- `crates/bboxy450-term/tests/harness_theme.rs:63` and `rich_text_safety.rs:60`: added the exact authorized `KNOWN DEFECT: pre-existing on main; fix: tasks/2026-11-06-mux-preexisting-test-failures` ignore attributes. Assertions and deadlines are unchanged.
- Their exact, qualified self-reexec selectors now pass `--ignored` (`harness_theme.rs:74`, `rich_text_safety.rs:72`). Explicitly running an ignored parent must still execute its child.
- `Makefile:2503` and `:2517`: the named rich-text-safety and harness-theme diagnostics use `--run-ignored all`, preserving their ability to expose the known defects.

New signed commits:
`6057d3eedbabb4ad837765f749fdbe3c82f283f3` and
`a27f56f6e9c7d58252cbb8fc6dab4aa2bbeff496`.
Both carry `Report: [[reports/2026-11-07-one-test-binary-landing]]`.
All seven task commits have GPG status G under the approved Boxy identity/key. The branch is clean at the latter SHA, based on `8ced58cb2ed3e710d0351ce114469c3b8d6296ea`; harness pin remains `38d99096f4f9c475a5547cf2192982af6ff7a5c0`.

## Acceptance evidence

1. **Integration binaries:** mux Cargo.toml:2,62 disables discovery and declares only `it`; app Cargo.toml:2,71,77 declares `it` and the documented standalone `scrollback` allocator fixture. All 52 original mux files and all 40 current app files remain registered, including upstream `idle_cpu.rs`. Shared helpers have one root module. Files and contract paths remain in place.
2. **Discovery equivalence:** the original before/after nextest JSON comparison has no removed test identities or changed ignore flags. Same-source per-crate counts were mux 960 → 960 (72 ignored), app 776 → 776 (122 ignored); excluding the four tracked mux identities gives 956 on both sides. Rebased grouped discovery adds nine upstream app tests and three feature-enabled mux model tests: mux 963, app 785. The two newly authorized app deferrals deliberately increase app ignored count from 123 to 125; they remove no registrations. The 11 original KNOWN DEFECT attributes, including those in library source, remain byte-for-byte unchanged; only these two were added. `authorized-source-audit.json` records 11 → 13, no removed attributes, and no task changes to either crate's product source.
3. **Changed-file landing:** `python3 -I scripts/test-landing.py` exits 0: **219 fixture tests**, 78.211 s. Its actual `make land` fixture runs only the changed module. Earlier selection-equivalence evidence covers 112 night-boundary selections, helper/entrypoint fallback, standalone scrollback, quoted filters, QoS classification, quarantine and invariant consumers. The two private cold land measurements below also execute only the same nine mux attention-policy tests and one app status-probe test.
4. **Contracts and formal:** `make test-formal-contracts` exits 0 on current source (83.681 s), including native contracts, strict lint and QoS tests. `make test-formal FORMAL_REUSE=1` exits 0 (296.710 s); `make check-formal-evidence` exits 0 (5.920 s). Current clean-source receipt: **681 models, 34 crates, 255 contracts**, native/TLC parity, hash `e576d012a71305dc890641af281658a07cc578546230270d00c505fdc3ff95a1`. Persistent receipt: `/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/e576d012a71305dc890641af281658a07cc578546230270d00c505fdc3ff95a1/evidence.json`. Our diff changes neither `verification/contracts.toml` nor coverage maps.
5. **Measurements:** retained below and in the original committed report. No expensive cold measurement was repeated after the Oracle accepted the trade-off.

Explicit ignored-worker verification used nextest with only these two qualified cases, `--run-ignored all --no-capture --no-fail-fast`. It exits **100**, runs **2 tests, both fail**, and both private children execute one test. They reproduce the existing theme operation-count and renderer literal-tail assertions; they do not silently pass by executing zero child tests. These are expected diagnostic failures, not a green product assertion.

## Measurements retained from the original report

Cold `cargo nextest run --no-run`, two Cargo jobs; all four builds exit 0. Exact recorded disk units are KiB.

| Crate | Layout | Integration binaries | Wall s | Child CPU s | debug/deps KiB | Whole target KiB |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| mux | Before | 52 | 1062.990 | 740.300 | 8,068,968 | 8,158,416 |
| mux | After | 1 | 433.886 | 417.565 | 2,165,264 | 2,253,896 |
| app | Before | 39 | 935.260 | 547.720 | 6,452,228 | 6,645,536 |
| app | After | 2 | 1019.129 | 304.502 | 5,620,112 | 6,147,004 |

These source snapshots predate upstream idle_cpu. Whole build tree reduction: mux about 72%, app about 7.5%. Cache warmth and intermediate-directory layout differ between some no-run measurements; the original report documents those confounds and superseded invalid timing captures. Child CPU excludes work in a separate persistent cache daemon.

Controlled cold real `make land` runs used private signed clones and local bare origins, cache disabled, identical O1/debug0 profiles, two jobs/two test threads, co-located cold target trees and the same two comment-only file changes. Both exit 0 and push only to their private local origins.

| Layout | Wall s | CPU s | debug/deps bytes | Whole target bytes |
| --- | ---: | ---: | ---: | ---: |
| Before | 2231.041 | 3813.670 | 2,413,408,256 | 2,661,154,816 |
| After | 2410.500 | 4049.910 | 2,581,626,880 | 2,827,984,896 |

Thus this cold two-file land costs +8% wall and +6% CPU. Module selection narrows execution while the shared integration binary still compiles the suite. The Oracle explicitly accepted this cost for the full-build disk saving. Raw summary: `target/one-test-binary-evidence/land-20261107T152000Z-d3369f27/summary.json`.

## Current actual landing result

`make land` at `a27f56f6e9c7d58252cbb8fc6dab4aa2bbeff496`:

- Rebase/signature checks, pinned submodule initialization, strict all-target lint and app binary build pass.
- App: **654 passed, 125 skipped**, across three selected lib/bin/integration binaries. Standalone scrollback is unaffected and was covered by the original full run.
- Mux: **888 run: 881 passed, 7 failed, 72 skipped**. This per-package invocation lacks the three extra model cases enabled by the grouped verifier's feature unification.
- Quarantine refuses the unlisted C1 failure; nextest exits **100**, land script **77**, outer Make **2**. Dependent cargo checks and push are not reached. No test budgets, assertions or quarantine allowlists were changed.
- Cost tree: **1128.813 s wall, 1397.089 s CPU, +283,168,768 bytes** target delta. Independent outer clock: 1129.546 s wall, 1397.549 s CPU. This is an incremental mixed-cache failed gate, not another controlled cold comparison.

Four C1 library failures in `logic::dream_public_api_tests`:

| Test suffix | Observed C1 unit CPU ns | Overrun source |
| --- | ---: | --- |
| dream_c1_actual_mute_writer_and_unmute_aba_do_not_revive_old_grant_page | 1,262,833 | src/registry/dream_c1.rs:347 |
| dream_c1_held_provider_refuses_ordinary_mcp_and_gui_read_admission | 1,218,334 | src/registry/dream_c1.rs:1068 |
| dream_c1_actual_root_workdir_aba_and_provider_revision_reject_old_pages | 1,153,708 | src/registry/dream_c1.rs:347 |
| dream_c1_held_provider_suppresses_actual_successful_ordinary_gui_completion | 1,086,667 | src/logic/dream_c1.rs:2628 |

They fail fixture-page/admission assertions in unchanged `src/logic.rs:14040,14042,14086`; the fourth receives `resource_limit` instead of `stale_context`. These cases passed in the previous grouped verifier. No causal conclusion about this run's resource overruns is established.

Three already tracked integration failures recur:

- `harmony_host::local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks`, harmony_host.rs:1035: empty staged-path output instead of the expected .bboxy note.
- `harmony_host::held_private_git_hook_keeps_other_swarm_input_resize_and_status_live`, harmony_host.rs:152: operation completes before private hook entry.
- `storage_bus_index::mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes`, storage_bus_index.rs:394: provenance log lacks the expected .bboxy/log prefix.

A preceding gate on the first deferral commit was deliberately terminated before push when review found the two diagnostic recipes also needed ignored execution. Only its verified owned process tree was stopped; the recorded reason/PIDs are in `authorized-land-cancelled-pids.json`. It is not counted as a completed measurement.

## Evidence and remaining decision

Current logs, under ignored `target/one-test-binary-evidence/`:
`authorized-final-land.log`, `authorized-final-land-result.json`,
`authorized-landing-fixtures.log`, `authorized-source-audit.json`,
`authorized-final-checks.json`, `authorized-formal-contracts.log`,
`authorized-formal.log`, `authorized-formal-evidence.log`,
`authorized-ignored-workers.log`.

The original exact compound verifier last ran on cb7a85e, exit 100, as documented in the first report. It was not repeated in this follow-up; the requested current-source landing gate and formal checks above are the actual new evidence. No install, full workspace CI, public push, merge or release occurred.

**Decision needed:** provide a green fix for the remaining seven mux failures to rebase, or amend the brief to authorize resolving them here. The current exactly-two-deferrals ruling and selection-only script scope do not authorize additional ignores, C1 library changes or a red-gate push. The code, measurements and current formal receipt are ready for review; landing remains blocked.
