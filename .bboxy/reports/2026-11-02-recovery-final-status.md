---
author: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
authorship:
  kind: seat
  process_generation: '2'
  public_key: 8G8gIL1MJqmoRkKikaIqqCqCFW1VHfzquBV0bXH7xs8=
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
commits:
- ffd3fc762446996aea788b421b87d31fe185fee8
- 96c847dab90c8fab924a49bf204cd54a4b1e3131
created: 2026-11-02T21:13:45.623Z
created_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
id: 01M6F1RV8ZK17ECVM39MXV4W6X
number: 5002223
provenance:
  digest: sha256:5753f1d0860a4eeea5f482dc8609d72424c229c07415cf6388ad30841f84397b
  frontmatter_digest: sha256:09bf320f1e05d4ffc54f12e488c1ac2274f2b9fddeca393dae5041caecfc5f83
  log: .bboxy/log/2026-11-02.jsonl#8403
  sig: /aXaiqD0vVDpN78Aw1VI+8hnsCQUEfrjA1bwY0SgYLU1sHtD0eDVrUK3FdceHHwulLkuRahISYBaBBQtgbBSAw==
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
supersedes: '[[reports/2026-11-02-lean-ignored-suite]]'
task: '[[tasks/2026-11-02-recovery-disk-budget]]'
title: Cleanup complete; landing blocked by upstream formal initialization
type: report
updated: 2026-11-02T21:13:45.623Z
updated_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
verify:
  command: make test-recovery-disk
  exit_code: 0
---
Cleanup is complete. The chat and recovery changes are signed in /Users/feral/mydev/bboxy450-term-chat-mute on fix/chat-mute-notices. Landing remains blocked by the upstream harness's mandatory JSON feature-isolation guard. The earlier reports retain the implementation, red regression, cleanup and native evidence; this append-only correction records the latest source and gate failure.

All 25 reviewed aged worktrees were removed (git worktree remove --force, every exit 0). Three old bboxy fixture owners were stopped; six authorised Node/esbuild servers received SIGKILL. A fresh audit verified 25 absent paths, 25 saved HEAD references and all 12 salvaged local-data directories, zero errors. Saved refs use cleanup-preserved/2026-11-02/...; salvage is /Users/feral/worktree-salvage/2026-11-02. Audit /tmp/nyx-old-worktrees-cleanup.json has SHA256 d965b53dd32296ea6e524dbd1b6d7d49645a92118530e0aa1288b79deb0e7edf.

Source mechanisms: messaging/service.rs:1145 projects the signed presence name and mute/unmute action for live and reopened history without rewriting envelopes. The private native test in swarm_workdir.rs:1876 checks the four named notices and UI restart. recovery.rs:317 replaces the arbitrary 10 GiB free-space floor with 128 MiB plus 1 MiB per seat, real retained-file accounting and a sparse oversized negative control; ring/journal/SQLite/checkpoint persistence and crash/replay/screen/debt/reaping assertions remain. Child stdout already uses Stdio::null(). Size checks are observations, not an OS quota or a proof of every transient peak.

scripts/quarantine.py:197 and land-fast.sh accept nextest no-tests only for an explicit lean opt-in and a real nonempty all-ignored integration target. Filters, empty/malformed listings, failed queries and build/test failures still fail. Makefile adds test-recovery-disk, test-chat-mute-notices and test-lean-ignored. LAND.md documents those distinctions. verification/coverage.toml adds the existing HarnessFoldedCard contract/control pair; no witness or feature-isolation rule was removed.

Latest completed native invocation at c09f2bd8ee0950c91379115cd3cb249a10462703 with harness f8de77ae7dcbb5c064180268e491b9ed52fc72b0: make test-recovery-disk test-chat-mute-notices smoke test-lean-ignored — exit 0. Four-seat recovery passed all four tests with 28,225,846 bytes under 138,412,032; fifty-seat recovery passed with 45,836,472 bytes under 186,646,528. Projection/history and private native restart passed (native 3.321s); the restored private capture was visually inspected and shows all four names/actions/timestamps. Smoke passed a 10.005s idle probe and orderly child reaping. Thirteen runner/controller tests passed; the real 28 ignored cases were reported compiled only; empty-expression and invalid-target controls still failed. /tmp/nyx-pinned-final-checks.log SHA256 ec042148aea3cd83dfd317d26911d6288393d168528c7a473e551dec5f158230. Both then-current commits separately passed cargo check --locked --workspace --all-targets, exit 0 (/tmp/nyx-current-parent-check.log and /tmp/nyx-current-tip-check.log). Earlier license checks passed. No UI geometry changed.

make land then fetched moving main and stopped on a land-fast.sh conflict (make exit 2, script exit 78). /tmp/nyx-pinned-final-land.log SHA256 5824dc366fee4f07e00af24dbfb68642475c41f26d8b916edb7180bbdb23b138. The conflict is resolved, retaining both upstream's executable build before bin tests and this task's ignored-only opt-in. Signed source commits before this report amendment: ffd3fc762446996aea788b421b87d31fe185fee8 and 96c847dab90c8fab924a49bf204cd54a4b1e3131.

Current parent main b9e87a09 pins harness cf56cf89ecc53e101cafa23bd7f2b17d2fc7bde4. make init-submodules updated to that exact gitlink but failed the unchanged parent feature guard. make test-formal FORMAL_REUSE=1 exited 2 during initialization: harness: parent feature isolation rejected: serde_json/preserve_order. The harness now enables acp-agent by default; scripts/harness-submodule.py:71 rejects the resolved preserve_order feature. /tmp/nyx-pinned-formal-failure.log SHA256 1561b1f45bd622a2774dff11bfc584c8734e3820cde86092432153842773bd25. Correcting that upstream pin/integration contract is outside this task's source scope.

The final task_report supplies the containing signed SHA, last canonical attempt and clean worktree state. Current-source formal verification and landing are incomplete. Historical formal receipts and native results identify their actual revisions; no unrun current-source result is claimed. No trusted install or live operator backend restart was performed.