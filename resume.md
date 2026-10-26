# Rook continuation — error journal #56

Updated 2026-10-26. The operator's direct `continue` resumed #56 after the shutdown handoff; no unrelated tasks, desktop automation, publication or scale run.

Worktree: `/Users/feral/mydev/bboxy450-term-rook`, branch `rook-error-journal`. Vesper is the sole Oracle/spec contact; Corvin and Wren sessions are shut down. Read current `AGENTS.md`, GitHub issue #62 and actual Git state before acting on old briefs.

## Landed

`4d0e66e4b0e7b62338ef0651255755910e305b9c` and its two implementation ancestors are on main. #56 cut1 adds bounded best-effort run diagnostics, offline `ctl debug errors [--since UNIX_MS] [--seat S]`, unread-errors HUD and shim/MCP/ctl/plugin/provider/spawn/worker capture. It preserves stdout BrokenPipe handling and user harness configuration.

Author `land.sh` EXIT0: full normal workspace build/tests/strict Clippy/UI lint, headless idle 0 frames/0 callbacks, 27 invariants/0 failures. Focused core6/0, actual CLI1/0. Evidence `.bboxy/runs/error-journal-cut1.log`; raw `/Users/feral/.local/state/bboxy450-term/landing/presubmits/4d0e66e4b0e7b62338ef0651255755910e305b9c-37a99d7a/commands.log`.

## Landed: cut2

Oracle ruling `cf405d78-f563-426a-8ec4-19c22737c945`: observe only a verified loaded private Codex thread and owner PID; `thread/resume` with `excludeTurns:true`, no config/permission/model fields; cancel on binding/generation changes. Never attach to the operator's running seat. No hook wrapping.

Signed landing `6b4d574f32dae18e3d6da28c46e33e2edf17a8ad` is on origin/main, author `land.sh` EXIT0 in431.994s. Affected kit/mux/app build/tests/strict Clippy/invariants and wire goldens all green. Files: kit `src/{native,launch}.rs`, kit `tests/{adapters,ready_exec}.rs`, mux `src/logic.rs`, one line in `.bboxy/interfaces/app-harness.md`, evidence and this resume. Native error entries only, 4KiB tail, explicit unknown exit/duration, bounded duplicate tracking and nonblocking worker/journal handoff. Codex TUI and service exec remove seven inherited parent session identifiers, preserving HOME/PATH/auth and BBOXY admission. The service wrapper execs in the same PID, including on an older surviving supervisor.

Focused kit lib/adapters/ready-exec:24/0 (two private child entry points ignored in the parent harness); mux compilation0. Mux exact-attachment/cancellation2/0, launch/resume wiring2/0 and kit+mux all-target strict Clippy0. Author gate EXIT0; raw log `.bboxy/runs/error-journal-cut2.log` names the exact command output/hash.

## Evidence and gaps

Raw evidence pointer `/tmp/rook-error-journal-root.txt` currently names `/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/rook-error-journal-9sb463or`. Preserve outside Git; durable copy `/Users/feral/.local/state/bboxy450-term/resume/2026-10-26-rook/`.

A controlled installed-ICQ0.21.0 check with a private fake session and inherited Claude ID exits1 in8ms with `ambiguous inherited hook session`, before any home/daemon/registration is created. Actual stderr/hash in `icq-env-guard.{stderr,json}`. This proves the inherited-identity bug class, not the original operator seat's historical cause.

Original seat `01M5WZKA3MZC8BRJ601TFF7FH1` was inspected read-only; its earlier stderr was not retained, and the operator endpoint later disappeared. No app restart/input or global harness-config write was attempted. Hook/completed covers synchronous notifications after subscription; historical/async failures remain unavailable. Keep #56 open for these explicit limits.

## Reboot safety

GitHub #56 is this work; #62 is the global handoff; #64 records the operator's typing/paste-dialog freeze with unknown cause. Scale issues #46–50 already exist. #360 lock repair and frame telemetry already landed; do not redo. Scene-template candidate9739ea66 is parked/unlanded/unverified.

Use public Cargo package names after the 0.1.0 release rename (`bboxy450-rt`, `bboxy450-mux`, `bboxy450-harness-host`); Rust library aliases remain bbt_*. Source cache-env before Cargo, jobs2, own --target-dir. Every commit signed Boxy/bboxy450, machine key9B07E849480C68328CA20684844ECC0587DE7AB0; code only via scripts/land.sh. Local gh is bboxy450; the GitHub connector's retired identity is unsuitable for writes. Old untracked w1-* evidence is unrelated and left untouched.

Final code landing: 6b4d574f32dae18e3d6da28c46e33e2edf17a8ad; parent journal landing4d0e66e4. Presubmit raw SHA256 5c17f455b779e93129d8a3cf2eca48aad8b35a2a8a584a83d34b1044da1586c5. Both are verified ancestors of origin/main. Own focused target retired inactive (977,178,624 allocated bytes); shared slot target left to its owner. No new tasks started.
