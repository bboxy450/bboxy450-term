---
id: 01M715JK1FBF2KP403G5P408HP
number: 181193791776312
type: report
title: Signed delivery of full-access ACP defaults and native OMP support
task: '[[tasks/2026-11-04-codex-trusted-handoff]]'
supersedes: '[[reports/2026-11-09-acp-yolo-pin]]'
commits:
- 3681c3b6a9d93ec98258ff7e603c7cb406ede8c2
verify:
  command: make reopen-prepared BUNDLE=target/release/bboxy450-term.app GUI_CTL=1
  exit_code: 0
author: seat:01M70ZAHH7P4B75NWMQRJ3SGSR
created: '2026-11-09T22:06:34.543121+00:00'
---
Direct operator request: full YOLO for harnesses inside bboxy ACP, prioritizing Claude/Codex and including Pi/OMP; bypass CI and restart only the UI. Delivered from a clean owned worktree. The author is this standalone Codex execution identifier, not a registered native-seat signature; the historical task supplies delivery context without changing its lifecycle.

The signed Term source commit 3681c3b6a9d93ec98258ff7e603c7cb406ede8c2 and signed harness commit d64b817091189f106c8bf245408a915683f754c4 are on their main branches. The source pin and mechanism are recorded in [[reports/2026-11-09-acp-yolo-pin]] and crates/bboxy450-harness/.bboxy/reports/2026-11-09-acp-yolo-defaults.md. Saved ACP profiles without a policy now default to yolo. Claude opts into the SDK's dangerous-permission support and confirms bypassPermissions before prompting; Codex uses agent-full-access from startup. Pi retains its unrestricted RPC tools and automatic permission replies. OMP is discovered through its native ACP server and receives --yolo. Offered full-access config options and allow_always-only grants are supported; explicit ask remains an opt-in and cancellation remains cancellation.

Verified delivery:
- make test-acp-unrestricted exited 0: three permission checks, all 26 ACP process cases and exact private profile discovery passed. Private real Pi/OMP initialize probes also exited 0, with no provider prompts/tools. These are bounded fixture/startup checks, not paid vendor conformance.
- Canonical make -o check-licenses bundle exited 0, compiling both executables and signing with the approved Apple Development identity 3943AF130A015B36824B7FBC86AA672E7B25F3C4. App compilation reported 58m33s; the shared cache server exited unexpectedly and Cargo recovered locally. Worker preparation reported 12m23s. No global cache daemon was stopped. Existing missing-libLLVM debug-stripping and unused Dream helper warnings were nonfatal.
- Ordinary worker preparation passed both before/after input-closure checks. Its scope is Ordinary release worker preparation only; consumer verification incomplete; native Dream admission/consumer conformance is not claimed or relaxed. Worker receipt: /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-58b_cujw/receipt.json.
- make reopen-prepared BUNDLE=target/release/bboxy450-term.app GUI_CTL=1, from the primary checkout, exited 0. The canonical helper installed the signed bundle, guarded replacement of GUI 77536 and retained the repository. New GUI 88974 / 01M715E0GVSSF4DP4M16M7PQ4Y renders source 3681c3b6a9d93ec98258ff7e603c7cb406ede8c2.
- Post-install assertions exited 0: installed source and harness metadata match the source cuts; installed executable hashes match the signed bundle; backend/gui/events are ready; frame sequence advanced from 4 to 673. Supervisor PID 24351 birth [1793985974, 714884] and Logic PID 24354 birth [1793985974, 732777] equal the initial, preinstall and postinstall identities. Neither backend owner was restarted.

The shared Term checkout was fast-forwarded and its harness checked out at the new pin. Existing .bboxy/tasks/2026-11-06-gui-process-cpu.md bytes were preserved through that operation (SHA256 063487a5d34981e157d8a032b3c9e6d64ed4bda268b3747aaf481a4f48c944be). Git's submodule update changed a shared core.worktree setting; per-worktree settings were restored so primary and owned harness checkouts resolve to their own paths, with tracked files clean. No peer worktree or process was modified.

Machine-readable command/exit/hash evidence is .bboxy/runs/2026-11-09-acp-yolo-delivery.json. Canonical GUI receipt: /Users/feral/mydev/bboxy450-term/target/release/gui-reopen-u9fm3ogb; prior installed Contents retained at /Users/feral/Applications/.bboxy450-term-install.NPrSmM/previous-Contents. Raw command logs remain private at /tmp/bboxy-acp-20261109-bicor4fc, outside Git.

CI bypass was explicit: no make land, full CI, formal, lint or visual suite ran. Both source pushes have [skip ci] markers and gh run list returned no workflow runs for their exact SHAs. The broader exploratory onboarding raster remains failed on conversation-scroll restoration (f32::MAX versus 123); it is reported, left untouched and outside the 30-check delivery claim. The two ACP fixture issues and their final passing checks are detailed in the producer report.

Existing agent workers retain their running executable/config until relaunched; a UI-only restart preserves them. The screenshot's text can also mean Claude abandoned an unfinished streamed tool before execution, including during steering, so yolo cannot eliminate every such bubble. No execution/result is fabricated.

This final evidence-only commit does not change the installed source cut. Owned worktrees are released after checking their commits are on main, their trees are clean and no process has them open; live source and daemon processes are preserved.
