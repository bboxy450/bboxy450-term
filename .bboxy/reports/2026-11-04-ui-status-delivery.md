---
type: report
title: Bounded UI status and safe read-only handoff diagnostics
task: '[[tasks/2026-11-04-ui-status-and-handoff-diagnostics]]'
commits:
- 727ee898740e31f38d59461c4f157547b1495cd4
verify:
  command: make verify-ui-status
  exit_code: 0
author: seat:01M6KGM9DJ6TH7222SFTQM2D0Y
created: 2026-11-04T15:12:40.474Z
id: 01M6KHX2K67Y0RGXN6AQVSV87F
updated: 2026-11-04T15:12:40.474Z
number: 93377961360452
created_by: seat:01M6KGM9DJ6TH7222SFTQM2D0Y
updated_by: seat:01M6KGM9DJ6TH7222SFTQM2D0Y
authorship:
  signer: seat:01M6KGM9DJ6TH7222SFTQM2D0Y
  kind: seat
  public_key: sgUIh+kVbgLLY9GhI40lBjigTvWO79blZKnsnoaJEXs=
  process_generation: '1'
provenance:
  digest: sha256:f60e05966a658a73c25069a085c41d0c0007e5d990921fa2c7ddf300f7a47387
  frontmatter_digest: sha256:bf73b0023cc31371b6b13ad28fc519b5dd75f60a1f106d3419843de38c6165f5
  signer: seat:01M6KGM9DJ6TH7222SFTQM2D0Y
  sig: Bs5+o2Qga7YiUPj3nWItRd3RV7L+vcTqSLndROn27giYprJmN2pwd+TaknBiknNZV/pqHz6ubtd/phw0GR1jBQ==
  log: .bboxy/log/2026-11-04.jsonl#119039
---
# Delivery

Task: [[tasks/2026-11-04-ui-status-and-handoff-diagnostics]].
Worktree: `/Users/feral/mydev/bboxy450-term-ui-status`, branch `codex-ui-status-20261104`.
Base: `cadf6700cbdb3d3575b9fc0dad7567cb5c96f0c9`.
Implementation: `727ee898740e31f38d59461c4f157547b1495cd4` (signed Boxy).
This report is indexed by the host; its exact indexed bytes are committed separately with the Report trailer.

## Changes and mechanism

- `crates/bbt-mux/src/logic.rs:9703` chooses a terminal title, otherwise observer detail; `crates/bboxy450-term/src/scene.rs:934` projects that title into the shared SeatRow. The pinned harness producer's `frontends/seat.rs:340` emits Run plus command arguments in its activity caption. These producer, model and ownership paths were inspected and remain untouched.
- `crates/bbt-ui/src/modules/seat_row.rs:14` bounds chrome strings to 192 Unicode scalar values, replaces whitespace with spaces and drops other control characters. `seat_row.rs:31` reduces harness operation captions to their existing verb while retaining the ACP prefix. Thinking remains unchanged; ordinary shell titles keep their command/title semantics.
- `crates/bbt-ui/src/modules/seat_row.rs:135` and `crates/bbt-ui/src/modules/strip.rs:55` enforce one text line with ellipsis. `seat_row.rs:544` and `strip.rs:251` use the same title projection; `seat_row.rs:771` clips row children to their existing geometry. Names, roles, subtitles/cwd, activity, freshness, generation owners and action IDs remain in their existing models and layout paths.
- `crates/bboxy450-harness-wake-up/src/harness_log.rs:265` uses a nonempty operation name without whitespace, otherwise the item kind. It never falls back to raw command source. The raw session log and rich tool command/output content are retained.
- `crates/bboxy450-term/src/cockpit/gui_handoff.rs:129` formats only mismatched public version numbers and PATH fingerprints. Missing and malformed fingerprints use safe reason codes. It never serializes LogicReady, capabilities, identity notes, custody data, account identifiers, paths or environment values.
- `crates/bboxy450-term/src/cockpit.rs:914` calls that formatter on the existing compatibility refusal. The compatibility predicate, cold-start refusal, live-owner checks, non-claiming attach, owner fences and refresh prohibition are unchanged.
- `Makefile:2281` adds the canonical focused target, with two Cargo jobs, worktree-local target outputs, selected nextest checks and affected strict Clippy compilation.

## Acceptance evidence

| Acceptance | Executable/source evidence |
| --- | --- |
| Clean Thinking/status; multiline source cannot escape header/sidebar geometry | `crates/bboxy450-term/src/scene.rs:2613` uses native font/layout measurement for multiline Python heredocs, long words, combining marks, emoji and Unicode separators at 200/320/800 px, for harness and ordinary terminal rows. It checks bounded single-line text, row/strip clips, retained Thinking caption and unchanged header/action rectangles (less than 0.001 px floating-point tolerance). Existing `view_switch_header_geometry_and_hits` also passes. |
| Raw command details retained; concise operation captions | `crates/bboxy450-harness-wake-up/tests/harness_status.rs:196` observes terminal-kind status and asserts the complete original command remains in the log, including heredoc/Unicode and a 4096-character word. Existing quiet-tool, stale/generation, approval and completion witnesses pass. Native layout assertions require exactly `acp:codex-acp · Run` at wide width and retain the original model title. No producer/tool-card changes. |
| Exact nonsecret readonly diagnostic; no weakened guards or cold start | `crates/bboxy450-term/src/cockpit/gui_handoff.rs:317` independently varies all five version fields and missing/malformed/different PATH fingerprints; exact mismatched fields and expected/observed values are asserted. Private secret sentinels must be absent. `gui_handoff.rs:372` exercises the real preflight against private live-owner fixtures and verifies owner bytes/lock remain unchanged, a missing executable is never launched, and cold start creates no files. `crates/bboxy450-term/tests/ui_status_probe.rs:15` exercises the public CLI in a bounded owned child, refuses cold start, preserves missing run directories and excludes a secret environment sentinel. |
| Focused checks and affected strict compile; unattended | `make verify-ui-status` exit 0: 14 harness-status tests plus 5 selected app tests across 3 binaries; 448 unrelated app tests filtered out. Strict Clippy with `-D warnings` passes for the wake-up library, shared UI library and app library/binary. `make test-licenses` exit 0. |

## Production public observation

The final test-built executable was invoked with only `--gui-handoff-probe --preserve-backend-check`, explicit production run directory `/Users/feral/Library/Application Support/bboxy450-term/run` and repository `/Users/feral/Library/Application Support/bboxy450-term/workspace`, under a 15-second subprocess bound. It exited **1**, the expected guard refusal, in **0.552 seconds**. No GUI detach, claim, backend refresh/restart, signal, capability-file inspection or operator input was performed.

The sole mismatch was:

| Field | Expected by this launch | Observed retained Logic |
| --- | --- | --- |
| launch_path_fingerprint | `78c1d194ec384ba9` | `d74860b838acf725` |

All five checked version fields matched: MCP control, GUI/window state, swarm metadata, UI control and swarm workspace. This inference follows from the formatter emitting every mismatched field and the public refusal containing only the PATH field. The expected fingerprint is the existing login-zsh PATH loader's result, not a guessed environment. PATH contents are not disclosed or inferred. The Oracle owns environment selection and GUI activation.

Binary SHA256: `64e44fabaefffd763b6d05d74600211b5b67feb5a6edc906981a4210d524a675`.
Raw public-probe receipt remains outside Git at `/tmp/codex-ui-status-production-probe.json`.

## Verification and limits

- `make verify-ui-status` — exit 0; raw log `/tmp/codex-ui-status-verify-passed.log`, SHA256 `842cfbc72b8bcb2ad2dbe9956ba567b8c99816e52501ac91524ea6d42087d6c2`.
- `make test-licenses` — exit 0; 34 workspace crates and retained notices/source headers checked; raw log `/tmp/codex-ui-status-licenses.log`, SHA256 `f4ef9fd17996b0c0ad023afba4f666846cc75bdad06833b2dcb0865135b8a1d3`.
- `python3 -I scripts/verify-signatures.py cadf6700cbdb3d3575b9fc0dad7567cb5c96f0c9..HEAD` — exit 0, implementation signature verified under the pinned identity.
- `git diff --check` — exit 0. Implementation worktree clean after the source commit.
- Compact command/exit/hash summary: `.bboxy/runs/codex-ui-status.json`. Tested source hash: `9328df5a2daf2f5c5e76fb127cabcca462039d25f1afc05596c49aeac667a3ca`.

Earlier verification attempts exposed a sandbox socket denial, fixture permissions, insignificant flex arithmetic rounding and one strict needless-borrow lint. Those were corrected; the final target passes. Socket fixtures ran with authorized sandbox escalation, without operator input.

Deliberately omitted per the brief: the whole test/parity/formal/runtime corpus, formal model changes (formatting/geometry and readonly diagnostic changes only), desktop input/capture, installation, publication, integration, fetch/pull/rebase/merge and harness pin changes. No new state transition, protocol or persistence behavior was introduced. Existing canonical attribution and source headers remain attached; the new fixture includes the same project/policy notices. No unresolved implementation failure remains.
