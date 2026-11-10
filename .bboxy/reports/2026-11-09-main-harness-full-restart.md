---
id: 01M71FWXWVFW2YSPMNGAXAW3SP
number: 1499550330375407
type: report
title: Pull main and harness, signed reinstall and full app restart
task: '[[tasks/2026-11-04-codex-trusted-handoff]]'
commits:
- 3da65fd563c86bbd02d7a0f8102656d3b3029509
verify:
  command: make restart-recover
  exit_code: 0
author: seat:01M71FWXWVFCSS0N1ZFZYFMR4C
created: '2026-11-10T01:24:18.153000+00:00'
---
Direct operator request: pull main and harness, update the harness pin, rebuild both, reinstall and restart Supervisor and UI; later explicitly pull again, allow rebase, and restart everything. The latest full restart instruction supersedes earlier UI-only requests. The author identifies this standalone Codex execution, not a registered native seat; the historical task supplies delivery context without changing its lifecycle.

Source and scope:
- Both pulls exited 0. The second Term pull was already up to date at 3da65fd563c86bbd02d7a0f8102656d3b3029509; harness origin/main was fd2917aa8381bd01e7bdc1d147e549d20902f6b7.
- make bump-harness exited 0. Term main already contained that exact latest producer pin, so the canonical action produced no gitlink or Cargo.lock diff. No redundant pin or product-code change was created. Source was built in a clean owned worktree.
- All 272 local modified/untracked KB files were hashed before each primary pull and remained byte-identical across those Git operations. The tracked GUI CPU task remains local and unstaged. No stash, reset, broad staging or peer-worktree cleanup was used. The independently dirty producer checkout was outside this clean submodule update.

Build and installation:
- make -o check-licenses bundle TARGET_DIR=/Users/feral/mydev/bboxy450-term/target exited 0, with the matching BBOXY_CARGO_TARGET_DIR selected. Canonical release compilation built Term and harness; the app phase reported 16m08s. The canonical bundled worker preparation additionally built its release executable in 374.23s and validated its input closure. Its receipt scope is Ordinary release worker preparation only; consumer verification incomplete. No native Dream admission or consumer-conformance claim is made.
- The bundle was signed with approved Apple Development identity 3943AF130A015B36824B7FBC86AA672E7B25F3C4. make install-prepared BUNDLE=/Users/feral/mydev/bboxy450-term/target/release/bboxy450-term.app exited 0. Strict installed-signature verification exited 0, and both installed executable hashes equal the signed bundle. The installer retained prior Contents in /Users/feral/Applications/.bboxy450-term-install.bJd1SX/previous-Contents.
- Installed BBOXYSourceRevision is 3da65fd563c86bbd02d7a0f8102656d3b3029509 and BBOXYHarnessRevision is fd2917aa8381bd01e7bdc1d147e549d20902f6b7. Existing missing-libLLVM debug-stripping warnings and unused Dream helper warnings were nonfatal.

Full restart and verification:
- make restart-recover exited 0. The canonical helper captured /Users/feral/Library/Application Support/bboxy450-term/run/restart-20261109T171855.json, stopped the exact recorded GUI/Logic/Supervisor owners and cold-launched recovery. Its immediate snapshot contained zero running/open seats; previously closed seats were not deliberately resurrected from the older build-time observation.
- Old GUI 88974, Supervisor 24351 and Logic 24354 are absent. Final GUI 84696, Supervisor 84759 and Logic 84761 have new birth identities. Their recorded start seconds match the kernel's ps lstart values. The read-only public GUI probe exits 0 and confirms the new rendered source and retained repository.
- ctl app status exited 0 twice: running/backend_ready/gui_ready/events_ready are all true and frame sequence advanced 2156 to 2229 with the same GUI instance. A subsequent seat listing had one running seat. No OS clicks, keystrokes or desktop captures were used.
- The first post-restart attach-only --preserve-backend-check probe exited 1: the cold-launched Logic PATH fingerprint differs from this terminal caller. All schema fields agree. That flag checks whether an existing backend can be preserved for a UI-only handoff; this request explicitly restarted the backend. It was not weakened or retried blindly. The applicable public GUI probe, owner/kernel-start checks, strict signature/hash checks and live readiness/frame checks then exited 0.

CI bypass follows the operator's standing request for these updates: no make land, full CI, formal, unit, smoke, visual or lint suites ran. check-licenses was omitted, with license resources retained by the canonical bundle. This report and .bboxy/runs/2026-11-09-main-harness-full-restart.json are the only task-local repository changes. They are committed with [skip ci]; no separate harness publication is required because the producer cut is already on main. Raw logs remain private at /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-update-20261109-hu2462uc; the committed JSON contains command/exit/hash summaries and public process identities, without control capabilities or transcripts. The evidence-only report commit does not change the deployed source cut.

A concurrent main push added only repository KB records at e22d346b54cd3d13c7f61cd75116b06b7797e06a. The evidence-only report was rebased over that commit; the deployed product code and harness pin are unchanged, so no second build or restart was introduced.
