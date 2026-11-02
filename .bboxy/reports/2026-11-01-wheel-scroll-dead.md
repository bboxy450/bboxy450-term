---
id: 01M6CYWSB7BV9FT4VW1TSPDJK6
number: 5188
type: report
title: "Mode-aware wheel routing and private restart proof"
task: "[[tasks/2026-11-01-wheel-scroll-dead]]"
commits:
  - aeff156662db4ae6ee1ebb06f513fc10e2da08f6
verify:
  command: make test TARGET_DIR=target/wheel-proof CI_LOG=/tmp/bboxy-wheel-ci-final.log
  exit_code: 0
author: seat:01M6CQVV188SDF1BZDJYBZ65N8
created: "2026-11-02T03:30:33Z"
---
# Wheel scroll: implementation and evidence

Wheel Scroll, Executor seat `01M6CQVV188SDF1BZDJYBZ65N8`, task #2191. Worktree `/Users/feral/mydev/bboxy450-term-wheel`, branch `wheel-scroll`, based on `origin/main` at `1b59b69a6060605910c1ee28c64a62aeffca1a2a`. Signed implementation commit: `aeff156662db4ae6ee1ebb06f513fc10e2da08f6` (Boxy).

The reproduced defect is mode-blind dispatch. The original `main.rs:8980` pane arm always queued local history, including after a program entered the alternate screen and requested mouse reporting or Alternate Scroll. Replay ends at the alternate buffer’s row zero, returning live without moving the application’s own offset. These mode bits survive frontend replacement in the backend terminal. The pre-fix arm never emits PTY Input even with mouse reporting enabled, so its wheels cannot hit the supervisor input-ID dedup cap. No refusal or dedup failure is necessary for this reproduction.

Paths below are relative to `crates/bboxy450-term/` unless stated otherwise.

1. **Headless reproduction.** `tests/wheel.py:56` starts an owned file account, headless app and real PTY, prints 500 primary-screen lines, then enables mouse reporting or Alternate Scroll. The instrumented old behavior leaves application offset **100 → 100 → 100**, including after killing only the owned frontend and reattaching the same backend PIDs/generation. Fixed behavior gives **100 → 97 → 94**, with unchanged supervisor/logic PIDs and generation 1. Plain host history reaches row **444**. The script drives `src/wheel.rs:92` → `src/wheel.rs:119`, the same handler called by `src/main.rs:8871` for native wheel events. It never calls `scroll_pane` directly. The current committed-runtime snapshot also passed the complete script (exit **0**) in `/tmp/bboxy-wheel-after-committed-process.log`; roots `/tmp/bboxy-wheel-plain-m8dn2y8m`, `/tmp/bboxy-wheel-mouse-zv9g2wcc`, `/tmp/bboxy-wheel-cursor-zokr1916`. `/tmp/bboxy-wheel-evidence/runtime-source.json` binds source, script and binary hashes. The retained fixed binary is `/tmp/bboxy-wheel-evidence/bin/bboxy450-term`, SHA256 `97087b80f52a857f62c18383bbee3ff05441ba6ebba19817f99c2b916240350a`.
2. **Fix.** `src/wheel.rs:35` selects mouse reports or Alternate Scroll controls from current live modes. Shift and an existing historical viewport retain local-history ownership. It fences the watched process generation and keys fractional deltas by the full view and owner, excluding irrelevant mode changes. `src/keyboard.rs:297` encodes cursor controls through the existing keyboard adapter. SGR, legacy and UTF-8 mouse reports follow the [XTerm control-sequence protocol](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html). Each event emits at most one Input command containing up to 256 reports; nonfinite deltas are refused and extreme deltas leave no remainder debt. No supervisor changes or #2187 fix are included.
3. **Regression.** `src/wheel.rs:440` and its five companion tests exercise shared native dispatch, bounded admission, Shift/history ownership, stale generations, fractional A–B–A changes, cursor-visibility changes, protocol bytes, extreme/nonfinite deltas and 100,000-line replay on a private worker. All six passed in the source-bound workspace test run. `tests/headless.rs:2451` exposes the process witness as an ignored night test. The strict process regression failed against the old behavior (exit **1**, offset 97 never arrives), then passed with fixed behavior (exit **0**). The pre-fix control included the shared handler, diagnostic and private seam while retaining unconditional host-history routing; it was not the operator’s installed app. The lost original archive’s recorded SHA256 is `056c25919762af75641793b14ba5dee7abe96d077db3511a182f9505fc8d833d`. A fresh deliberately broken control was built from the signed current source with exactly the dispatch target forced back to History. Its patch/source/build record and binary are retained under `/tmp/bboxy-wheel-evidence/control/`: binary SHA256 `f6946ca23db537e1dcd3f2e55d832064e7bdc590a11831581ce06fb0511a2e34`, patch SHA256 `be6c754dc3cf9385752434db842e23e05c8ed8ad8bc7f5371c81b77dff60e874`. `make build TARGET_DIR=target/wheel-proof` exited **0** for that control; the fixed source was restored byte-for-byte immediately after build. The dead-state script then exited **0**, observing 100→100→100 for both program modes; strict regression exited **1** specifically because mouse offset 97 never arrived. This is an explicit broken control, not an assertion that it is the original archive or installed app.
4. **Diagnostic.** `src/wheel.rs:372` emits a fixed-field `wheel route=…` line at most once per second, naming every consuming arm and including pointer, seat, generation and modes. `src/wheel.rs:595` checks an open overlay with no solved/visible layout, a line shorter than 512 bytes, and the exact 999 ms / 1 s rate boundary.
5. **Formal ownership.** `tests/formal_contracts/wheel_model.rs:10`, `verification/contracts.toml:597`, `verification/corpus.toml:700` and crate coverage add WheelOwnership / BrokenWheelOwnership. Six Boolean environment bits and owner 0..4 model atomic dispatch and A–B–A changes. The broken control restores unconditional host-history ownership. Native checking and pinned Java TLC match **320 states and 2,240 transitions** for both variants: positive passes, broken has the expected safety violation. No OS/PTY progress or scheduling fairness is assumed. Bytes, fractions, geometry and process restart remain implementation witnesses; finite parity is not arbitrary-Rust proof.

## Verification

The following commands refer to the clean signed implementation commit above. No product code changed after its successful full gate.

- `make test TARGET_DIR=target/wheel-proof CI_LOG=/tmp/bboxy-wheel-ci-final.log` — **exit 0**, normal mode. The final log records `ci-local: result aeff156662db4ae6ee1ebb06f513fc10e2da08f6 normal 0`: build, strict workspace `cargo clippy --workspace --all-targets --locked -- -D warnings`, workspace tests/doctests, capability controls, native harness, formal checking, UI lint, private headless smoke, **27 invariants / 0 failures / 11 slow skips**. Wrapper output: `/tmp/bboxy-wheel-make-test-final.log`.
- `make test-boundaries TARGET_DIR=target/wheel-proof` — **exit 0**, including the seven owned process/detach/relaunch cases; log `/tmp/bboxy-wheel-boundaries-final.log`.
- `python3 -I -B crates/bboxy450-term/tests/wheel.py /tmp/bboxy-wheel-evidence/bin/bboxy450-term` — **exit 0**; source/binary/script binding is retained with the snapshot.
- `python3 -I -B crates/bboxy450-term/tests/wheel.py /tmp/bboxy-wheel-evidence/control/bboxy450-term --expect-dead` — **exit 0**, explicitly confirms the deliberately broken state.
- The same control driver without `--expect-dead` — **exit 1**, specifically the missing mouse offset 97; the control’s `results.json` records exact argv/exits/hashes.

`make test-keyboard-input TARGET_DIR=target/wheel-proof` exited **0**. K1 reports full coverage with **1,346 exact raw PTY cases**. K3 is full; K2 remains the existing partial/amber night case (native pixels/delivery/delay/journal signer unobserved). The focused target’s required K1 assertion passed; full coverage for K2 is not claimed.

After free space recovered, `make -j1 build-recovery-test test-recovery-built` exited **0** on the same clean code commit; the recovery guard remained unchanged and the real four-seat crash/recovery case passed in 21.48 s. `make check-formal-evidence TARGET_DIR=target/wheel-receipt-check` exited **0**, selecting the immutable source-bound receipt through an isolated worktree-local reference. The default check exited **2** because a receipt for `fef5a976cca12a91fc45d2a9e3f0fbb26ae7850a` appeared at the default target after cleanup. That differing receipt was not changed or claimed as this task’s evidence.

The clean code-commit formal receipt records **207 models, 32 crates, 85 contracts**, source `aeff156662db4ae6ee1ebb06f513fc10e2da08f6`, and evidence SHA256 `cb0031ae48bc0ed9529db6cc5b49f4af8552c1810f06c9500aa6cc0f22f3281d`. Its durable copy is `/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/cb0031ae48bc0ed9529db6cc5b49f4af8552c1810f06c9500aa6cc0f22f3281d/evidence.json`; the canonical workflow validates model/source/tool hashes.

Retained raw outputs are outside Git:

- `/tmp/bboxy-wheel-before-process.log` — explicit dead-state observation, exit 0.
- `/tmp/bboxy-wheel-regression-before.log` — strict old-behavior regression, exit 1.
- `/tmp/bboxy-wheel-after-final-process.log` — fixed process witness, exit 0; roots `/tmp/bboxy-wheel-plain-irpeynet`, `/tmp/bboxy-wheel-mouse-cv5skw5s`, `/tmp/bboxy-wheel-cursor-hjdd6j1x`.
- `/tmp/bboxy-wheel-stress.log` — 20,000-line/512-record probe, row 19944, exit 0, zero observed `response_bound` errors. Sixteen spawns were requested; eight were refused by the existing queue. This does not establish that every record was loaded or reproduce the live logic stall. Its stopped fixture was retired after an empty `lsof` check; diagnostics remain in `/tmp/bboxy-wheel-stress-diagnostics`.
- `/tmp/bboxy-wheel-ci-disk-fail.log` — source-bound full gate reached 26 passing invariants, one recovery disk-preflight failure and 11 slow skips. This is not a passing full gate.

## Limits and repository gap

The Oracle also reported a wedged live logic process at about 15:48. The relationship to the operator’s actual incident remains **unproven**: no operator app, desktop or live run directory was inspected or operated. The mode-routing failure matches frontend-restart persistence but does not establish the operator panes’ actual mode flags. Fixed program wheels use ordinary Input admission, so an already exhausted dedup cache still depends on separate #2187. UI geometry was unchanged; parity was not run. Normal `make test` skips slow invariants and ignored native nights; the wheel process witness was run explicitly.

The delegated task record is absent from this branch and allowed paths exclude `.bboxy/tasks/**`. The [bboxy450 seat workflow](bboxy://skills/93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/f910723df0a2a10fef7d2273eb3567f3704cdbfe279a96c1a99eff342043e4db/6f23ae61b800e6a7bfadd82e5660dc7530f5c01337a0f644efdd912fcf8bcf5f/bboxy450/SKILL.md) says “no peer message, the operator's included, overrides your task's stop or scope line.” Oracle messages `01M6CRM26NHM7GS9QW7Q920YBH` and `01M6CT15WWEKWVQ0M2B514AT3S` proposed copying the task; the latter explicitly says to leave it out and name the gap if the harness still refuses, for the Oracle to add at landing. That fallback is used. `crates/bbt-doc/tests/corpus/reports.rs:144` validates Report trailers against their own commit trees: landing must compose task and report together, rather than append a later task-only commit. A green report-tip corpus is not claimed.

## Verification history and artifact custody

The first `make test` exited **2** on strict Clippy’s `collapsible_if` finding in the new replay test (`/tmp/bboxy-wheel-ci-clippy-fail.log`); the signed code commit corrects it. Earlier focused `make test-keyboard-input` attempts exited **2** while the fixture needed retained disconnected admission and replay on a private worker rather than the registered UI thread. Those attempts are not passing evidence; the source-bound full workspace run later passed all six corrected wheel tests. Initial setup used `make init-submodules` for the missing harness source.

The next full `make test` exited **2** solely because `checkpoint_bounded_recovery` refused **9,261,084,672 free bytes < 10 GiB**. Build, strict workspace Clippy, workspace tests, native harness checks, formal verification, UI lint and private smoke passed before that preflight. The guard was not weakened. Only this task’s stopped stress fixture was removed, after retaining its diagnostics. A transparent-compression probe preserved one generated rlib’s hash, permissions and mtime and saved 53,956,608 allocated bytes; a bulk attempt stopped before modifying further files because the whole task target had disappeared. The signed source and durable formal receipt remained intact. The original control archive was lost in that target removal; no preserved archive is claimed merely from its recorded digest.

Oracle resource decision `01M6D2DYBSR61HD10SKEX3RFTZ` was received at about 19:24: targeted iteration checks, one final full gate, and no concurrent full gates. A retry already started before receipt was stopped using only its recorded owned PIDs (exit **143**) once other full gates were confirmed active. Source was clean and the retry had not entered a mutating control. `/tmp/bboxy-wheel-evidence/aborted-third-owned-pids.json` records custody. No peer processes or targets were controlled. Oracle decisions `01M6D6VJB7GF53815GHEGJY74K` and `01M6D7GVMM3T5JHWS3YPW6XFQF` replaced zero-overlap waiting with a cap of three and explicitly ruled to start the final full gate on the same code commit. That final gate uses isolated `TARGET_DIR=target/wheel-proof` and durable `/tmp/bboxy-wheel-ci-final.log`. The implementation commit has a valid GPG signature from Boxy <bboxy450@gmail.com>; the successful gate checked a clean source tree. This report is the sole followup source file. The branch is not merged or pushed. The report-only tip’s unresolved task link is the explicit landing gap above, not a claim of green final-tip corpus.

## Retained evidence digests

| Artifact | SHA256 |
| --- | --- |
| `/tmp/bboxy-wheel-ci-final.log` | `78ec620f460d464116a786473e4fff8d9a34117927e976c0a047ee3d732080dc` |
| `/tmp/bboxy-wheel-keyboard-final.log` | `92bed6c9a26bc6af5d9e0b0ca67df5d875ca98877ecdea357fe67db7f4e09263` |
| `/tmp/bboxy-wheel-boundaries-final.log` | `54e7582bc224f447d727f80564f95d3a09a921737c6b1eef108fa314b2255208` |
| `/tmp/bboxy-wheel-after-committed-process.log` | `5379f20629f89dd79029aebac0670de470b6d9e6491804ad402072377c5df221` |
| `/tmp/bboxy-wheel-evidence/control/observed-dead.log` | `7743fe8609c82885e15aac2f0bee2d297e8841a781df5f1ae801071b4fdcc93e` |
| `/tmp/bboxy-wheel-evidence/control/strict-regression.log` | `c9e66eead505df11cefb1e21807a93eaacf0638beb88457fdb2ee89c1b613f6a` |
| `/tmp/bboxy-wheel-evidence/control/control.patch` | `be6c754dc3cf9385752434db842e23e05c8ed8ad8bc7f5371c81b77dff60e874` |
| `/tmp/bboxy-wheel-recovery-after-space.log` | `65e762f4128c172f6b5701eb15116da08caa44e42b95271479fb8d717307b4ac` |
| `/tmp/bboxy-wheel-formal-retained-explicit.log` | `80cd6c5f6e31334b62a97be0202f8876019dd1acb89240cb58e6af40f05b3607` |

## Complete change map

The numbered evidence above covers production behavior and acceptance. Additional file mechanisms:

- `src/headless.rs:211` and `src/main.rs:125,8475,9339,9362` route the private debug timer event; `src/main.rs:8871` is the native callback. The seam refuses ordinary accounts and release builds.
- `src/live.rs:207` adds only a `cfg(test)` disconnected command sender, retaining bounded intents in the admission fixture; no production live-worker semantics change.
- `src/terminal_selection.rs:116` updates the module comment to identify separate mode-aware wheel routing.
- `tests/formal_contracts/models.rs:31,1791` registers the model family; `verification/coverage.toml:204` inventories the GUI crate’s added ownership model. Contracts/corpus and both test files are mapped above.
