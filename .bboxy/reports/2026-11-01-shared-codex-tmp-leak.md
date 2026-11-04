---
id: 01M6BE9ADKYYRECTAKNX25TEVZ
number: 5000
type: report
title: "Retire shared Codex e2e fixture roots"
task: "[[tasks/2026-11-01-shared-codex-tmp-leak]]"
commits: []
verify:
  command: make test-shared-codex-cleanup
  exit_code: 0
author: seat:01M6ABQRB8SJNZHP6NRHQD3BC8
created: "2026-11-01T11:35:30Z"
---
# Shared Codex fixture retirement

Rook Ashwhisker, Executor seat 01M6ABQRB8SJNZHP6NRHQD3BC8. Worktree `/Users/feral/mydev/bboxy450-term-tmpleak`, branch `shared-codex-tmp-leak`, initial base a91bb70f. Scope extension decisions 01M6BD523Y93PJE1QT4ZCTE0K6 and 01M6BDKY0CEJJ39C5ZBKWYZPPY authorize the native caller handoff, exact task-record copy and local report creation. The task was copied byte-for-byte from the shared checkout. This record uses its own ULID and number 5000, expressly reserved by the Oracle in ruling 01M6BF9WCDWK8ST26TMAHYTTQ7; 1797 was already occupied in the live index.

## Files and mechanisms

- `scripts/e2e/shared_codex.py:51` reads only the private daemon/updater and all run/*.owner.json backend/frontend owner records. Unknown, corrupt or symlink custody fails closed. `reclaim_roots` at line 73 checks the script marker, canonical root, inode, UID, nonempty valid PID list and all recorded producers' liveness before retiring an old root. It never signals an old owner. Concurrent reapers need no shared lock.
- `FixtureRoot` at line 101 records the runner at birth, tracks each spawned command/app Popen before waiting, and atomically updates `cleanup-owner.json`. Reaped transient commands are retired from that list. `finish` at line 134 stops/reaps only its recorded Popen objects, closes its server, checks recorded daemon/backend PIDs, then removes the entire successful root. The top-level wrapper covers early setup failures as well as test-body failures. Existing private ctl/daemon shutdown paths run independently, so one failure does not skip the remaining shutdown actions. The updater retains its PID/start-time/private-command fingerprint check and waits for exit after its own TERM.
- A failed stopped fixture retains only `cleanup-owner.json` and present allowlisted diagnostics: app.log, fixture-error.log, requests.json, seat-state.json, evidence.json, message-evidence.jsonl, identity-observation.json, owned-daemon-updater.json, owned-logic-fault.json, render-evidence.json, stream-evidence.json, color-evidence.json, cleanup-recovery.json. Each diagnostic is capped at its last 64 KiB. Home/packages, work/account/run/tmp, frames, raw stream, images, shims, configs and unknown files are removed. Tails may be partial JSON; they are diagnostics, not replay input.
- `export_artifacts` at line 232 copies only frames/ and codex-stream.bin into an explicit existing empty caller directory outside the disposable root. It refuses symlinks, overlap and nonempty destinations. `scripts/e2e/native_strip.py:220` creates that temporary directory, passes --artifacts, reads/replays it directly, and removes it on both success and failure; it no longer parses the disposable root out of stdout. Existing native detection/replay algorithms are unchanged.
- `scripts/test-shared-codex-cleanup.py` and Make's focused `test-shared-codex-cleanup` target provide private filesystem/process witnesses. The focused target does not build or run Codex or ask the operator for anything.

## Acceptance evidence

1. `test_success_removes_root_after_owned_child_exits` invokes the real main lifecycle with a fixture producer and an actual recorded subprocess; the child is reaped and the root is absent. Early/body failures enter the same outer retirement path.
2. `test_failure_keeps_only_bounded_diagnostics` creates a bulky package tree, frames and an oversized app log, deliberately fails the fixture, then checks the exact small retained set, byte cap, original failure trace and stopped child.
3. `test_next_run_reclaims_dead_and_preserves_live_or_foreign_roots` exhausts eight owned/foreign × runner-live/dead × auxiliary-live/dead states for each of logic, supervisor, frontend and desktop owner filenames (32 cases). A fresh FixtureRoot reclaims exactly the marked all-dead case. Duplicate reclamation preserves live roots. `test_unknown_corrupt_and_symlink_custody_is_untouched` protects unmarked directories, corrupt records and symlink targets. `test_live_auxiliary_owner_prevents_retirement_without_a_signal` proves the auxiliary process is left alive until its test owner stops it.
4. `test_removed_cleanup_fails_the_success_witness` deliberately replaces finish with a no-op and requires the same successful-root witness to fail with “successful fixture root leaked”; test-owned child cleanup still runs. Restored implementation passes. CLI and native-caller handoff tests use actual file copies and root retirement with stubbed app/Codex/pixel producers, including failure after handoff; caller recordings are absent afterward.

## Commands and actual results

- `make test-shared-codex-cleanup` — exit 0, nine tests. Includes the deliberate removed-cleanup control and both native handoff outcomes.
- `make test-licenses` — exit 0; 31-crate policy/notices, source notices, six license fixtures and nine source-notice fixtures passed.
- `git diff --check` — exit 0.
- Initial artifact-refusal test expected FileExistsError for an overlapping destination; the API correctly raised ValueError. Corrected the expectation and verified the completed handoff design. No production guard or threshold was relaxed.

The first full `make test` on signed f55c837b exited 2: the corpus gate found this locally authored draft missing required `number:`. All other Rust targets passed (279 successful test groups). The Oracle reserved number 5000 and authorized amending this unpublished malformed draft, including correction of the license-fixture labels. The failed log is `/tmp/rook-tmpleak-ci-missing-number.log`; no check was waived.

Intermediate signed 75df9ebf completed full `make test` with exit 0, including 124 native/TLC models, 31 crates and 52 contracts; `/tmp/rook-tmpleak-ci-green-full.log` preserves it. Landing acquired a slot and rebased cleanly to 050fc003, but exited 2 because --gate-log required the new candidate SHA. The source-bound formal refresh on 050fc003 exited 0 (132 models, 31 crates, 56 contracts; receipt 64a366bb0d4ddbcd6a3151a43659c83a9cd22ec0cde18096cd8c6e8da328eefa). Oracle ruling 01M6BJD6WTWB9H1YN50046C8W0 authorizes trying a plain scoped landing, with one fresh-full-CI fallback if needed, at most two landing attempts.

A final custody review found frontend/desktop readiness records as well as logic/supervisor records. The reader now checks every run/*.owner.json rather than a two-name list. The focused suite still passes nine tests, including all 32 filename/liveness cases; the final source-bound landing gate must validate this amended candidate. No live-owner guard was weakened.

The command/exit/source-hash summary is `.bboxy/runs/2026-11-01-shared-codex-tmp-leak.json`. This report records verification completed before its containing signed commit. Exact-committed-source `make test` and `sh scripts/land.sh --gate-log target/ci-local.log` remain required after committing; their actual results, resulting SHA and formal receipt belong in the final task handoff and source-bound logs. No future gate is claimed green here. `commits: []` avoids a self-referential SHA; the signed Report trailer identifies the containing commit.

## Bounds and deliberate limits

The finite fixture projection has one root, an owned/foreign bit and two owner liveness bits. Reclaim is atomic in the projection, removes only owned all-dead roots and must happen at the next run; no fairness assumption is needed for that direct call. Real subprocess witnesses cover shutdown/reaping. This is fixture-level implementation coverage, not a new product/crate model or proof of arbitrary filesystem/OS behavior. The repository's mandatory native/TLC formal gate still runs in full CI.

Unmarked legacy roots cannot prove their creating script or runner and are deliberately left untouched; this change does not perform a one-off sweep of the historical 54 roots. PID reuse is conservatively live and delays reclamation. If a recorded producer remains alive after bounded shutdown, or ownership is unreadable, retirement raises and preserves that root rather than deleting beneath a live or unknown owner; a later run can reclaim it after all valid owners are dead. Hostile same-user metadata forgery, process creation/recording crash windows and arbitrary filesystem replacement races are not proven by this test model. No operator journal, desktop, credentials, shared installation, unrelated root or peer worktree was modified. Actual Codex/Metal e2e was not run; the authorized regression covers the artifact lifetime boundary without duplicating the pixel oracle.

The live MCP schema still lacks report creation/optional report-link parameters; the Oracle explicitly authorized this local report. The final task_report summary will carry its qualified wikilink without inventing a tool argument.
