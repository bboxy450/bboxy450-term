---
artifacts:
- '[[01M6DDYJJNX3CWH4ME3R7Y84QC]]'
assignee: seat:01M6CQVV188SDF1BZDJYBZ65N8
authorship:
  kind: seat
  process_generation: '1'
  public_key: J+Eete7RzpCHzcNSDpuEpx+ye6zu+UTGX+qDDQkPrw4=
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
created: 2026-11-01T23:42:07.503Z
created_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
delegated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
id: 01M6CQVSJCA37S02QAMN49XPNT
number: 2191
provenance:
  digest: sha256:4e6c1ad9c2063e245dfcaf6c0da51d4ceeeb2f3ffe7a8a7c35b9efe0dce1bbef
  frontmatter_digest: sha256:5aefd5f1a5c1befdfd4cad21e7a491c3a6e38051794a67a4d6bb5f354f656ffc
  log: .bboxy/log/2026-11-02.jsonl#15005
  sig: H9JAtIsJHGTL0s1W6PDk2QrcilXehOyHUeTmbzrizeC3pkWtOK+TZAqgZuaa/GsNqiYxrUvsBnWQ+lIhD9t9Ag==
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
rejection:
  by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
  reason: 'Bookkeeping only; the work is verified. The Oracle read /tmp/bboxy-wheel-ci-final.log `ci-local: result aeff1566… normal 0` and the mode-based routing in wheel.rs:35-49. Acceptance compares the command string exactly: re-report with verify_result command `make test`, exit_code 0, commit aeff1566, and the same evidence, naming TARGET_DIR/CI_LOG and the log path inside the evidence text. Run no new tests. The task-record copy into the landing tree is the Oracle''s integration step.'
report:
  commit: aeff156662db4ae6ee1ebb06f513fc10e2da08f6
  op_id: 01M6DE059TCEW9PCY7J6ZS8EPM
  reported_by: seat:01M6CQVV188SDF1BZDJYBZ65N8
  summary: 'Report: [[reports/2026-11-01-wheel-scroll-dead]]. Signed code aeff156662db4ae6ee1ebb06f513fc10e2da08f6; signed report 5a03687e7d0271093af8b981892382f54507d691 with Report trailer; clean wheel-scroll, no merge/push. Mode-blind history routing reproduced and fixed; native wheel diagnostics bounded/rate-limited. Full make test/strict Clippy 0 on code commit; boundary and focused keyboard 0 (K1 full 1346); real broken-control strict 1 versus fixed process 0; fresh report-tip formal/check 0 (207 models). Scope fallback remains: report-tip corpus 2 solely because task record is outside allowed paths; task and report must be composed in the same landing tree, not a later task-only commit. Actual operator logic-stall causation remains unproven; #2187 untouched. Artifact [[artifacts/01M6DDYJJNX3CWH4ME3R7Y84QC]].'
role: executor
status: rejected
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Mouse-wheel scroll stopped working in every pane; persists across UI restart
type: task
updated: 2026-11-02T06:11:09.424Z
updated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
verify: make test
verify_result:
  command: make test TARGET_DIR=target/wheel-proof CI_LOG=/tmp/bboxy-wheel-ci-final.log
  evidence:
  - criterion: 1. Owned headless reproduction and persistent cause
    reference: 'crates/bboxy450-term/tests/wheel.py:56 — 500-line PTY then mouse/alternate-scroll modes. /tmp/bboxy-wheel-before-process.log observes 100→100→100 across frontend-only restart. /tmp/bboxy-wheel-after-committed-process.log exit0: plain top444; both programs 100→97→94, same backend PIDs/generation1. Runtime/source/binary hash binding: /tmp/bboxy-wheel-evidence/runtime-source.json.'
  - criterion: 2. Root cause and scoped fix
    reference: base1b59b69a crates/bboxy450-term/src/main.rs:8980 — unconditional host-history dispatch ignores live modes; alternate buffer has no scrollback. Current src/main.rs:8871 → src/wheel.rs:119; wheel.rs:35 selects program reporting/history with generation fence and view/owner fractional state; src/keyboard.rs:297 reuses key encoder. Pre-fix wheels never emitted Input, so did not reach dedup cap; no supervisor change.
  - criterion: 3. Real wheel regression fails before and passes after
    reference: /tmp/bboxy-wheel-evidence/control/control.patch (SHA256 be6c754dc3cf9385752434db842e23e05c8ed8ad8bc7f5371c81b77dff60e874) restores unconditional history ownership; canonical build exit0; source restored byte-for-byte. control/observed-dead.log exit0; control/strict-regression.log exit1 specifically offset97 never arrives. Fixed retained binary/script exit0. Native callback and private seam share wheel.rs:119; no direct scroll_pane driver. tests/headless.rs:2451 registers process witness; six new wheel tests passed in full CI.
  - criterion: 4. Silent-consumer diagnostic
    reference: crates/bboxy450-term/src/wheel.rs:372 — fixed-field wheel route=arm line at most1/sec; every consuming arm named. wheel.rs:595 tests open overlay with no solved visible layout, <512-byte line and exact999ms/1s limiter boundary; full/focused test exit0.
  - criterion: 5. Formal model and deliberately broken control
    reference: tests/formal_contracts/wheel_model.rs:10; verification/contracts.toml:597, corpus.toml:700, coverage.toml:204. WheelOwnership/BrokenWheelOwnership native/TLC 320states/2240edges match, positive pass/broken safety. Full code receipt cb0031ae48bc0ed9529db6cc5b49f4af8552c1810f06c9500aa6cc0f22f3281d; current report-tip receipt 1ccd25f778a3c2e6baeaa5cc7501f6e8d1903a2197e5789c9661398fc0f4e152; 207models/32crates/85contracts, clean. Current make test-formal and check-formal-evidence exit0.
  - criterion: Verification and remaining integration gap
    reference: '/tmp/bboxy-wheel-ci-final.log SHA256 78ec620f460d464116a786473e4fff8d9a34117927e976c0a047ee3d732080dc ends exact aeff1566 normal0, 27invariants0failures11slow-skips; full strict workspace Clippy0, private smoke0. make test-boundaries0; test-keyboard-input0, K1full1346, K2existingpartial. Geometry unchanged, parity not run. Post-report make test-kb-reports2: exactly3 assertions for the missing scoped task link. See committed report and /tmp/bboxy-wheel-report-tip-corpus.log; Oracle fallback decisions 01M6CRM26NHM7GS9QW7Q920YBH / 01M6CT15WWEKWVQ0M2B514AT3S.'
  exit_code: 0
---
# Wheel scroll is dead in every pane

ROLE: EXECUTOR. Work only in a new worktree `~/mydev/bboxy450-term-wheel` on branch `wheel-scroll`, created from `origin/main`. Work on your own: find the cause, fix it, and prove it. Message the Oracle only for a blocker that stops all work. Never touch the operator's live app, desktop or run directory, and never inject OS input. Reproduce with your own headless app, test account and `ctl`.

## Symptom (operator, 2026-11-01 ~15:40 local)

Wheel/trackpad scrolling stopped, first in one Claude pane, then in every pane. Restarting the UI (backend kept alive: supervisor 45598, logic 45599) did not fix it. The installed app binary is from 2026-11-01 04:13 (0.1.1) and scrolling worked earlier with it, so suspect state that builds up, not a fresh regression.

## Evidence so far (Oracle, static and log reading)

- Wheel dispatch is a guard chain in `crates/bboxy450-term/src/main.rs:8862-9010`. Arms that consume the event before the pane arm (`:8976`): `app_overlays.open()` (`:8862`, swallows silently), `launcher.is_some()` (`:8863`), `document.open`, `sidebar_viewport` hit (`:8902`), `local_page_layout.contains`/`.focuses` (`:8919`, `:8934`, swallows silently), and the conversation body/rows arms.
- The pane arm needs the pointer inside `focus_bounds`, a seat in `live_state.ui.seats` (`scroll_pane`, `:5192`), and admission through `queue_pending` (`:9215`; a Scroll merge or supersede by `Input`, `:9238`).
- Logic side, `live.rs:1466`: Scroll needs `grids.baseline(seat)` with a matching generation, otherwise `scroll_refused(... "history_view_unavailable")`. `history.request` failures log `history <seat>: <reason>`.
- In the live `~/Library/Application Support/bboxy450-term/run/app.log` (read-only): no `history ` refusal lines and no `history_view_unavailable`. Frequent `GUI operation … rejected: response_bound: result exceeds the bounded response size`. Every frame is a `hitch phase=encode ~34ms` (continuous redraw), which may be related. Seat `01M6AC3D2ECK4W3F8495PAQTS3` also has its input-dedup cap exhausted (task #2187, a separate Executor). Check whether pane scroll in a mouse-reporting program turns into PTY input and so hits that cap. Do not fix #2187 here.

## Required outcome

1. A headless reproduction: your own test account, scripted wheel input (`ctl ui input` or the existing native input seam, #15) into a pane with scrollback, showing the scroll offset not changing. Find what state causes it. Long-lived seats, many seats, large history, response_bound history reads or a stuck overlay/local page are candidates, not conclusions.
2. Root cause as `file:line — mechanism`, and a fix.
3. A regression test that fails before the fix and passes after, driven through the real wheel path, not by calling `scroll_pane` directly.
4. If a wheel-consuming arm can swallow events with nothing visible, add a diagnostic: a bounded log line saying which arm consumed it, rate-limited, so the next such report is one grep.
5. If the cause involves lifecycle, ordering or event-loop arbitration, update the relevant formal model with a broken control (d365).

## Allowed paths

`crates/bboxy450-term/**` (GUI and live worker), `crates/bbt-mux/**` only if the cause is there, `verification/**`, `.bboxy/reports/**`. Not `crates/bbt-supervisor/**`: #2187 owns the input dedup there.

## Verification

`make test` on the exact committed source, exit 0. Also the parity check if you touch UI geometry (`make test-parity`). Strict clippy on touched crates.

## Done

Signed commits on `wheel-scroll`, a report in `.bboxy/reports/` with a `Report:` trailer, and `task_report` with file:line mechanisms and actual exits. Do not merge or push.


## Report from seat:01M6CQVV188SDF1BZDJYBZ65N8

Report: [[reports/2026-11-01-wheel-scroll-dead]]. Signed code aeff156662db4ae6ee1ebb06f513fc10e2da08f6; signed report 5a03687e7d0271093af8b981892382f54507d691 with Report trailer; clean wheel-scroll, no merge/push. Mode-blind history routing reproduced and fixed; native wheel diagnostics bounded/rate-limited. Full make test/strict Clippy 0 on code commit; boundary and focused keyboard 0 (K1 full 1346); real broken-control strict 1 versus fixed process 0; fresh report-tip formal/check 0 (207 models). Scope fallback remains: report-tip corpus 2 solely because task record is outside allowed paths; task and report must be composed in the same landing tree, not a later task-only commit. Actual operator logic-stall causation remains unproven; #2187 untouched. Artifact [[artifacts/01M6DDYJJNX3CWH4ME3R7Y84QC]].

```json
{
  "commit": "aeff156662db4ae6ee1ebb06f513fc10e2da08f6",
  "op_id": "01M6DE059TCEW9PCY7J6ZS8EPM",
  "report": null,
  "verify_result": {
    "command": "make test TARGET_DIR=target/wheel-proof CI_LOG=/tmp/bboxy-wheel-ci-final.log",
    "evidence": [
      {
        "criterion": "1. Owned headless reproduction and persistent cause",
        "reference": "crates/bboxy450-term/tests/wheel.py:56 — 500-line PTY then mouse/alternate-scroll modes. /tmp/bboxy-wheel-before-process.log observes 100→100→100 across frontend-only restart. /tmp/bboxy-wheel-after-committed-process.log exit0: plain top444; both programs 100→97→94, same backend PIDs/generation1. Runtime/source/binary hash binding: /tmp/bboxy-wheel-evidence/runtime-source.json."
      },
      {
        "criterion": "2. Root cause and scoped fix",
        "reference": "base1b59b69a crates/bboxy450-term/src/main.rs:8980 — unconditional host-history dispatch ignores live modes; alternate buffer has no scrollback. Current src/main.rs:8871 → src/wheel.rs:119; wheel.rs:35 selects program reporting/history with generation fence and view/owner fractional state; src/keyboard.rs:297 reuses key encoder. Pre-fix wheels never emitted Input, so did not reach dedup cap; no supervisor change."
      },
      {
        "criterion": "3. Real wheel regression fails before and passes after",
        "reference": "/tmp/bboxy-wheel-evidence/control/control.patch (SHA256 be6c754dc3cf9385752434db842e23e05c8ed8ad8bc7f5371c81b77dff60e874) restores unconditional history ownership; canonical build exit0; source restored byte-for-byte. control/observed-dead.log exit0; control/strict-regression.log exit1 specifically offset97 never arrives. Fixed retained binary/script exit0. Native callback and private seam share wheel.rs:119; no direct scroll_pane driver. tests/headless.rs:2451 registers process witness; six new wheel tests passed in full CI."
      },
      {
        "criterion": "4. Silent-consumer diagnostic",
        "reference": "crates/bboxy450-term/src/wheel.rs:372 — fixed-field wheel route=arm line at most1/sec; every consuming arm named. wheel.rs:595 tests open overlay with no solved visible layout, <512-byte line and exact999ms/1s limiter boundary; full/focused test exit0."
      },
      {
        "criterion": "5. Formal model and deliberately broken control",
        "reference": "tests/formal_contracts/wheel_model.rs:10; verification/contracts.toml:597, corpus.toml:700, coverage.toml:204. WheelOwnership/BrokenWheelOwnership native/TLC 320states/2240edges match, positive pass/broken safety. Full code receipt cb0031ae48bc0ed9529db6cc5b49f4af8552c1810f06c9500aa6cc0f22f3281d; current report-tip receipt 1ccd25f778a3c2e6baeaa5cc7501f6e8d1903a2197e5789c9661398fc0f4e152; 207models/32crates/85contracts, clean. Current make test-formal and check-formal-evidence exit0."
      },
      {
        "criterion": "Verification and remaining integration gap",
        "reference": "/tmp/bboxy-wheel-ci-final.log SHA256 78ec620f460d464116a786473e4fff8d9a34117927e976c0a047ee3d732080dc ends exact aeff1566 normal0, 27invariants0failures11slow-skips; full strict workspace Clippy0, private smoke0. make test-boundaries0; test-keyboard-input0, K1full1346, K2existingpartial. Geometry unchanged, parity not run. Post-report make test-kb-reports2: exactly3 assertions for the missing scoped task link. See committed report and /tmp/bboxy-wheel-report-tip-corpus.log; Oracle fallback decisions 01M6CRM26NHM7GS9QW7Q920YBH / 01M6CT15WWEKWVQ0M2B514AT3S."
      }
    ],
    "exit_code": 0
  }
}
```
