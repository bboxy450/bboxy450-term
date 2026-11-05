---
type: report
title: 'Standalone harness recovery: native startup and integrated face/status'
task: '[[tasks/2026-11-03-codex-harness-recovery]]'
commits:
- b6c61ba0049328b4efdacc0c1c2fe407ffe772a9
- ea4a3f5b68e97a031c5d6c62bf835520d8ad3ad0
- 2bef2af280d11db0fab6c723bf6bc76549dfae5b
- 2d787cf29393adbdb38a3097a1236df36705c637
- 580ae4b5c1925bd94cb295ed346a083c8cd79395
- ea753072c7ec2a0a278e5086ef21458efa0a08f0
verify:
  command: RUSTC_WRAPPER= make test
  exit_code: 0
author: seat:01M6HCYT7FYN90XD2C4TXGZQZC
created: 2026-11-03T20:04:33.059Z
id: 01M6HG6TRW5DRVW79KAZF3A942
updated: 2026-11-03T20:04:33.059Z
number: 16395
created_by: seat:01M6HCYT7FYN90XD2C4TXGZQZC
updated_by: seat:01M6HCYT7FYN90XD2C4TXGZQZC
authorship:
  signer: seat:01M6HCYT7FYN90XD2C4TXGZQZC
  kind: seat
  public_key: M3Z+ENRGLEvs/7AVU009ijG3F/12VdJt9g4HNxXJvB4=
  process_generation: '1'
provenance:
  digest: sha256:cb352342d2c9714ba222c4c4a233809c75876fd8bf62b47189568c99b6807472
  frontmatter_digest: sha256:6339d937a502d848c276b50995fbe9037ba7538907abbdd4931658855385c5b6
  signer: seat:01M6HCYT7FYN90XD2C4TXGZQZC
  sig: 4B4XX1iCLDCmLgV1l2GpQcezKulG2vVqAde/Y0knTSCiGqt6NCr8VJjZuBR9QKvL8JFhERfxgxWenEMY+n5NBA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#95073
---
ROLE: EXECUTOR. Task [[tasks/2026-11-03-codex-harness-recovery]] (01M6HCYD1S81VTDS3V9M4Y8H4K). Worktree `/Users/feral/mydev/bboxy450-harness-codex-recovery`, branch `oracle/codex-recovery`, base `0c403165e815c2f7bf77ccac293784b021f42f96`. Signed implementation candidate: `ea753072c7ec2a0a278e5086ef21458efa0a08f0`. No push, merge, publication, trusted installation, credential access or desktop input was performed.

The installed standalone app's first chrome frame fails because its shared TitleBar requests the `bboxy` bitmap, while 0c40316 registers only `bboxy-mark`. The recovered capture-chrome alias fixes this root cause. A real native startup process now reaches window creation, Metal surface configuration and live-frame submission. Removing the alias reproduces native exit 1 (`unregistered bitmap`) while the headless replay still passes; restoring it gives native exit 0.

## Startup evidence

- Installed `/Users/feral/Applications/bboxy450-harness.app` was inspected read-only. Plist revision is 0c40316; strict/deep codesign verification exited 0. Launcher, plist and binary hashes are unchanged after all checks. Audit: `target/recovery/inspection.json` and `provenance-current.json`.
- Private copies were assembled/signed/installed through canonical Make targets with ad-hoc signing and private HOME/TMPDIR. LaunchServices `gui --help` and an argv-only fixture both exited 0, so neither was treated as proof of native GUI startup. The bounded unified-log read timed out at 15 seconds; no causal claim was based on it. No harness-named crash report was found; an older unrelated launcher crash was excluded.
- `src/frontends/gui/assets.rs:29` registers the same existing identity portrait as both names. `src/frontends/gui/window.rs:132` keeps the smoke NSWindow hidden; the actual native host creates/configures its Metal surface and uses the live renderer. Hidden windows are occluded on macOS, so the smoke encodes into a private texture with the surface's view formats. It does not claim visible presentation. `scripts/smoke-app.py:47` waits for the real native process, records PID/exit, requires the frame marker and validates the private session log.
- `scripts/test-startup-control.py:46` restores main's missing registration. Current control: Make exit 2 / native exit 1, then restored Make/native exit 0. Logs and process records: `target/startup-control/missing-titlebar-asset{.log,-launcher.log,-native-process.json}`, `restored*`, `evidence.json`. A failed build cannot reuse an earlier process receipt.
- Exploratory instrumentation failures were separated from the product defect: waiting for a hidden native drawable timed out (own process terminated with -15); the first private target omitted the alternate sRGB view format and failed validation (Make exit 2). The witness now mirrors `config.view_formats`. Neither exploratory failure was counted as the startup root cause.
- `Makefile:14` includes native `smoke-app` in the canonical test stack; `make test-app-startup-control` exercises the regression and restoration without an operator.

## Recovered branch lineage and mechanism

| Original branch / signed tip | Integration |
|---|---|
| capture-chrome / df12668c743c18dbfc3af283bdced380f9bb9dcb | Cherry-picked as b6c61ba. Shared standalone/capture tree, bitmap alias, capture chrome and zoom, and their tests retained. |
| fix/native-never-drawn / 3058e12cede120a2e6986bb7cbf238a2038ca397 | Aggregate changes recovered in ea4a3f5; capture API conflicts resolved while keeping both test families. Native/Usage never mount, affect groups or carry a receipt. |
| fix/state-heartbeat / 95fbb72f675ffe163443fd5fb89fcf02068c8037 | Recovered in 2bef2af. Open-turn 2-second phase reports, explicit-phase ownership, PTY witnesses and all three new formal controls retained. |
| feat/agent-initiated-turns / 5ccbe48b023b8e3091f9a8f570c646223418bc53 | Recovered in 2d787cf. IR 1.2, policy-resolved idle work/permissions, measured Quiet, queued-user shortening, headings and process/model witnesses retained. |

580ae4b restores the authorized auxiliary files: unchanged historical quiet/status run summaries, `.gitignore`'s durable formal archive rule and the IR 1.2 schema. The Oracle's signed task amendment explicitly authorizes these paths. ea75307 adds the native startup witness/control, canonical smoke gate, attribution and source mapping.

All four original branch refs remain unchanged and reachable (`target/recovery/source-branches.json`); all six integration commits verify with Boxy's signing key 9B07E849480C68328CA20684844ECC0587DE7AB0. Original reports/diffs were reviewed and retained as history, not current verification. Exact changed paths and fresh command exits are in `.bboxy/runs/codex-harness-recovery.json`.

The obsolete StreamDock fallback was not revived by the agent-heading hunk: the heading remains on the shared rich row path. Typed constructor/capture API migrations preserve the heartbeat assertions and both capture/quiet witnesses. Formal inventories contain the union: 99 model/control variants, 34 contracts, one crate. No contract or control was dropped.

## Acceptance and current verification

| Criterion | Source / actual witness |
|---|---|
| Native/Usage have zero feed geometry, padding, receipt and grouping effect | `src/frontends/gui/face_rows.rs:47`, `:168`, `:297`, `:758`; `tests/gui_render.rs:246` and `tests/seat_golden.rs:15` compare N=1/16, ordinary/native-only sessions, chrome at 90/100/160%, visible geometry and paint/terminal bytes. GUI fallback, seat fallback and rail mutations each fail 101 and restore to 0. |
| Shared chrome, text zoom, tight transcript and tool rails preserved | `HarnessView::standalone` is shared by window/capture. `gui_chrome`, expanded capture and pixel tests pass. Seven fresh-main and candidate native PNGs are byte-identical against the same lab; full-window 120% capture was inspected. |
| Open-turn phase heartbeats | `src/frontends/seat.rs:25`, `:57`; immediate phase changes and every open origin retain working/blocked state. Real PTY silent-tool test passes. Missing heartbeat, stale phase and child-overrides-explicit-phase controls each fail 101 and restore to 0. |
| Additive IR 1.2 and agent turns | `src/protocol/mod.rs:190`, `:265`; `src/backend/acp_agent.rs:620`, `:660`, `:677`. No fabricated user prompt; policy handles idle permissions; no Quiet ending with open work; queued user follows the agent turn. `tests/acp_agent.rs:868`, `tests/ir_v1_corpus.rs:262`, `:312` and ACP export witnesses pass. Frozen prior logs and compatibility policy are unchanged. |
| Formal lifecycle evidence | `tests/formal/models.rs:2196`, `:2389`; 99 native/TLC complete reachable-state and labeled-transition graphs and expected safety/deadlock/liveness verdicts agree. Current source/model/tool hashes, fairness and bounds are checked. Receipt source ea753072c7ec2a0a278e5086ef21458efa0a08f0, dirty=false; digest 2bbcb32495cdfa7ef64185eae4e83e38fa2318d6bde4904488df611be3833a73. Durable ignored archive: `verification/evidence/<digest>/evidence.json`. This is bounded contract evidence, not arbitrary Rust refinement or a complete TLC replacement claim. |

`RUSTC_WRAPPER= make test` exited **0** in 1235.875s on the signed implementation candidate: all 20 canonical core commands, all four GUI commands, packaging and native app smoke passed. Full log: `target/recovery/make-test.log`; top-level exit: `full-result.json`; preserved command/input records: `full-generated/test-current.json` and `gui-gate.json`.

Additional current checks all exited 0 (`target/recovery/post-results.json`):

- `make -f Makefile -f target/recovery/verify.mk recovery-contracts` (standalone/test-fixtures gui_chrome, IR corpus/rich agent fixture, seat goldens, ACP-agent suites).
- `make test-app-startup-control` (actual native failure 1, restored 0).
- `make -f Makefile -f target/recovery/verify.mk recovery-quiet-controls` (three 101 → 0 mutations).
- `make -f Makefile -f target/recovery/verify.mk recovery-status-controls` (three 101 → 0 mutations).
- `make check-formal-evidence` (99 variants / 34 contracts / 1 crate, current clean source).

`make test-ir` also exited 0 directly and in the full gate: 47 locked definitions, 10 forbidden controls refused and four legal controls accepted in both schema forms. Direct `check-transcript` of the new agent-turn fixture exited 0. After all mutations, every canonical source input hash still matches the successful full gate. Tests use private HOME/TMPDIR, explicit cargo/rustup homes, empty RUSTC_WRAPPER, two Cargo jobs and the worktree-local target; operator_input=false.

## Visual evidence and limits

Requested lab: `/Users/feral/mydev/bboxy450-term-harness-ref/lab/web`; five source hashes remained unchanged, combined SHA256 7953a5e7e12a4c618b52358eed1d831efe286b15c3022309439e7811ac56d4f1. Fresh main uses the private copy of the installed 0c40316 binary and main's own face-only capture script; candidate uses `--no-chrome`. Seven native captures are byte-identical. Browser images have only tiny header-glyph variation in two fixtures (mean pixel delta ≤ 2.1e-7); rounded scores are unchanged. `visual-comparison.json` records actual comparisons.

| Fixture | Mean main / candidate | Row mismatch main / candidate |
|---|---:|---:|
| acp-claude-acp | .0070 / .0070 | .0300 / .0300 |
| acp-codex-acp | .0062 / .0062 | .0218 / .0218 |
| acp-omp | .0078 / .0078 | .0227 / .0227 |
| acp-opencode | .0360 / .0360 | .3827 / .3827 |
| acp-pi | .0073 / .0073 | .0355 / .0355 |
| glyph-atlas | .0300 / .0300 | .2309 / .2309 |
| mermaid | .0236 / .0236 | .3327 / .3327 |

Actual native images were inspected in `target/recovery/visual-main/<fixture>/native.png`, candidate examples, `chrome-120/native.png`, `agent-turn.png`, and `native-{before,after}/native.png`. The 16-Native-record replay improves mean difference .0521 → .0070 and row mismatch .4145 → .0300; the screenful of SYSTEM Native cards disappears without changing the normal transcript. The full chrome/120% capture shows the shared title bar; the agent fixture shows agent continued, tool work and Quiet.

No baseline was adopted, no target/tolerance/frozen fixture was weakened, and no global two-pass native visual sign-off is claimed. Existing opencode/Mermaid target misses are not reported as target passes. Native equivalence checks are bounded to N=1/16. Visible surface presentation, OS menu pixels and physical IME/input are not claimed by the hidden process or offscreen images.

Live external adapters: **SKIP**; scripted adapters/providers were used, with no credential lookup or login prompt. Term-hosted rich-pane e2e remains with the rich-socket/rich-host assignments. Their worktrees and ICDs were not changed; `Bridge`/Unix transport are unchanged here. Integration overlaps in seat/headless are the recovered heartbeat and IR constructors and must be retained when those workers' changes are combined.

## Handoff state

Generated tracked run outputs were saved under `target/recovery/generated` and restored; only the requested compact integration summary is intended for commit. Source inputs remain identical to the successful full gate. The report/summary handoff commit uses `Report: [[reports/2026-11-03-codex-harness-recovery]]`; its final task_report will carry that signed commit and refreshed HEAD-bound smoke/formal receipts. Oracle acceptance remains separate.

Three control-plane reads (task_status, knowledge_fetch and orient) timed out with -32001 / WouldBlock after 10 seconds while preparing the handoff. Their operation IDs are retained in the local recovery state; a blocker message was stored for the delegator. No verification command required operator input, and no registry/file-write bypass was used.
