---
type: task
title: Fix remaining native folds/diff clipping and run two visual passes
status: claimed
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-codex-visual
allowed_paths:
- src/frontends/gui/face.rs
- src/frontends/gui/face_rows.rs
- tests/gui_visual.rs
- tests/gui_render.rs
- tests/gui_chrome.rs
- scripts/native-visual-pass.py
- scripts/parity-web.sh
- Makefile
- ATTRIBUTION.md
- .bboxy/reports/**
- .bboxy/runs/codex-native-visual.json
verify: RUSTC_WRAPPER= make test
parent: 01M6HCH1SJHDSB6SDAC2ZF1BXX
id: 01M6HHJVJ7M277HKCAB22PC90D
delegated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-03T20:28:53.042Z
created: 2026-11-03T20:28:35.963Z
number: 17271
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
assignee: seat:01M6HHK04M27ANJ2YPTBSC4B0Y
provenance:
  digest: sha256:35723c60c9ec3775d36090f1c6ea89284830fd74bcda509d0862ff6ca6b1f917
  frontmatter_digest: sha256:1ce5960d0a047c4b2a6a11b7be85340620020dae48d2576e2f800fb627b3c2d1
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: mjDLz6YbPLez6nlfixm4bBzDlj/A03rVZ0N1X5zRcEaL4havRXrl8oDCeJYSUecxAPGw9K2Q6hRBcsB1i6rfBA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#99221
---
ROLE: EXECUTOR. Recover the outstanding native visual-pass work from ORACLE-HANDOFF.md under the operator's continue-building instruction. Work ONLY in /Users/feral/mydev/bboxy450-harness-codex-visual, branch codex-native-visual-20261103, initially published main af65c6cf82d9dde6314b1771b9314061b0e24212. Read AGENTS.md, ATTRIBUTION.md, vision, the GUI spec and the handoff visual notes. One writing worktree, exact staging, signed commits; no reset/stash, recruitment, push, merge, install, baseline adoption, threshold weakening, credential access or OS input.

Outcome: fix the two carried rendering defects in src/frontends/gui/face.rs: (1) failed terminal output with exactly one hidden line should show that line as the reference does, and folded counts must use correct singular/plural; (2) long diff/code lines must end with a visible ellipsis inside the card instead of being clipped mid-token, preserving highlights, Unicode and narrow-width containment. Keep Native/Usage completely absent and the shared standalone/embedded HarnessView path intact. These are presentation changes; do not change socket/protocol/lifecycle semantics or touch Bridge, seat, gateway, IR or the reference lab. Give focused geometry/render regressions and a deliberate broken control that fails causally then restores green. Add a missing canonical Make workflow if needed; do not invent another build/install recipe.

Run native visual pass 1 with fixes, inspect every side.png, then run pass 2 on the corrected source. Read /Users/feral/mydev/bboxy450-harness-parity/creativity-pass/{README.md,pass-notes.md,pass.sh} as historical guidance; DO NOT execute its hardcoded shared checkout/Cargo mutations or recreate the removed nyx worktree. Adapt the existing parity instruments in your own worktree/output. Use the SAME lab /Users/feral/mydev/bboxy450-term-harness-ref/lab/web and fixture set /Users/feral/mydev/bboxy450-harness-parity/fixtures for candidate and published main. Capture every fixture collapsed and --expand all, slash/profile menus, full chrome, zoom 0.9/1.0/1.2. Retain lab/source hashes and all command exits. Existing Mermaid/opencode/reference discrepancies must be reported honestly; do not alter the lab/targets to make numbers green or call two audits a clean global sign-off if gaps remain. No baseline adoption. Keep raw images/logs outside Git, commit only concise hash/exit/metric summaries.

Canonical verify is RUSTC_WRAPPER= make test on final committed source, including current-source native/Java formal evidence. One full machine gate at a time: inspect real running canonical processes, self-serve when free and continue useful focused/visual work meanwhile, without operator approval. Pure layout needs specialized tests; do not claim new state-machine coverage from an unchanged model. Two Cargo jobs, worktree-local target, private test HOME/TMPDIR.

Coordinate with rich-socket (01M6HCVV32KS845WHVD5FCX7QE) and rich-host (01M6HCYVGE7CJ8KK6PB20SPF3D): they own transport/hosting. Give a short signed ready-commit handoff when visual changes can be consumed. Integrate/rebase any published main before the final gate, retaining all quiet/status/IR1.2/socket contracts. Full indexed report .bboxy/reports/2026-11-03-codex-native-visual.md with Report trailer, actual source/commands/exits, both pass matrices, inspected images, controls and gaps; register through knowledge tools then task_report. Quiet by default: one completed report, or an actual all-work blocker.