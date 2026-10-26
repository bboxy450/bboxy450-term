# RESUME — bboxy450-term

Updated 2026-10-26 16:55 PDT by Vesper Greywind (Oracle). Read this first, then GitHub issue #62 and `.bboxy/graph.md`.

## Where we are
0.1.0 is cut, signed and mostly published. The swarm is wound down; Rook (#56) was the last Executor working.

- Tag `v0.1.0` (signed) = `d02f6135`. All 24 crates at 0.1.0; packaging is byte-reproducible (two independent builds matched 24/24).
- crates.io: publishing in dependency order under the new-crate rate limit (one per ~10 min). 14/24 live at 16:53; the loop
  (`publish-rest.sh`, log `out/publish-rest.log` in the Oracle scratchpad) finishes about 18:35. If it died: rerun it; it skips
  published crates and waits out 429s.
- `bboxy-firehose` 0.1.0 was published from `c2218b5b` (= main `c431e24c`), which adds `exclude = ["reference/"]`: the tag's
  package was 29 MB, over crates.io's 10 MiB cap. Every other crate comes from the tag.
- Signed app: `bboxy450-term-0.1.0-macos-arm64.zip` (sha256 `f241b55e…`), Apple Development cert, not notarized; Aster's
  production smoke passed on that exact zip. Info.plist version was stamped by hand (#65).
- GitHub release: **draft** v0.1.0 on `bboxy450/bboxy450-term-releases` with 52 assets (zip, .asc, .sha256, 24 crates + .asc, SHA256SUMS).
- Website: https://bboxy450.lol (Cloudflare Worker `bboxy450-site`, static assets, free tier, account gonzih@gmail.com).

## Finish the release (after the last crate, `bboxy450-term`, is live)
1. `gh repo edit bboxy450/bboxy450-term --visibility public --accept-visibility-change-consequences`
   (operator approved; history scanned clean: no password, passphrase, cargo token or private key in 27,684 objects).
2. `gh release edit v0.1.0 -R bboxy450/bboxy450-term-releases --draft=false --latest`
3. Homebrew: write `Casks/bboxy450-term.rb` in `bboxy450/homebrew-tap` (version 0.1.0, sha256 from the zip's `.sha256`,
   url `.../bboxy450-term-releases/releases/download/v#{version}/bboxy450-term-#{version}-macos-arm64.zip`, `app` +
   `binary` stanzas, postflight strips quarantine), signed commit, push.
4. `icq send '#bboxy450-crates' "bboxy450-term 0.1.0 — first release, 24 crates — https://crates.io/crates/bboxy450-term — https://bboxy450.lol — sig 844ECC0587DE7AB0"`
5. Close #62.

## Website (`site/`)
The lab showcase (`lab/web/showcase.html`) with a Download panel, fed real data by `site/real.js`:
Oracle transcript, one Codex rollout per Executor, GitHub issues as epics, today's ICQ DMs. `site/extract.py` refuses
to write if any local secret appears. Refresh and deploy: `sh site/refresh.sh && cd site && npx wrangler deploy`.

## Open after 0.1.0 (GitHub issues)
- #56 error journal: cuts 1+2 landed (4d0e66e4, 6b4d574f) for 0.1.1; the Codex hook's original exit-1 cause is unproven.
- #65 bundle-macos.sh must stamp the version from Cargo.
- #51 MA-10 mentions: branch `velvet-ma10-mentions`, signed, not landed.
- R2/S4: `aster/r2-s4-ready-20261026` (fd0a8b52), not landed. #40 flaky zsh adapter test: cause unproven.
- Scale: 967 seats miss frame/input budgets (#46 #47 #48 #49 #50). Acceptance coverage gaps: #1–#37.
- #44 ICQ redelivers old briefs ~25 h later: check a DM's timestamp before acting.
- Notarization needs a Developer ID Application certificate (operator decision).

## Landed today (all signed, on main)
#54 Finder cwd (072f213b), #55 keyboard layouts (f47ef1aa), #42 ctl BrokenPipe (77b190be), #360 lock, env isolation,
Knowledge swap, MA-1..8, the 0.1.0 cut (d02f6135), firehose exclude (c431e24c), the site (3284c774 + this cut).

## Machine rules learned today
- One gate at a time when disk < 25 GiB; parallel release builds froze the Mac (swap 4.9 GB, 14 GiB free).
- Gate slots `/tmp/bboxy450-gate-slot-N`; land with `scripts/land.sh` (exit 75 slots, 76 main red, 77 presubmit, 78 conflict, 79 moving).
- Parked signed branches: sable-release-resume-20261026, aster/release-resume-20261026, aster/release-smoke-0.1.0-20261026,
  aster/night-env-tested-20261026, nyx-release-0-1-0-wrapup, rook-error-journal-wip-20261026.

---

## Rook continuation — error journal #56

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
