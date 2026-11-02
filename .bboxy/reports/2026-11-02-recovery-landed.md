---
author: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
authorship:
  kind: seat
  process_generation: '2'
  public_key: 8G8gIL1MJqmoRkKikaIqqCqCFW1VHfzquBV0bXH7xs8=
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
commits:
- 5256f9e3e9c3156d35a6906057d3a188d06afbaa
created: 2026-11-02T22:12:50.320Z
created_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
id: 01M6F550Y24SK5B6WK28EHAB77
number: 5002225
provenance:
  digest: sha256:b6a0baf563369c533fae16d8b585b1cb484aed7b34d65688060aab9dd145a120
  frontmatter_digest: sha256:c04c27321d1b4134c6a62ff8d1bb4784f014aad6c514046300709f601e5887ec
  log: .bboxy/log/2026-11-02.jsonl#10970
  sig: jstzAgB4d533tAoJz0+5CsNnkCXTFW5KuplVkWfSKu4QHG9KAOKa3WqZ81IvFp0r3YvEzGaHTr6QYqVPcLjNAQ==
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
supersedes: '[[reports/2026-11-02-recovery-final-status]]'
task: '[[tasks/2026-11-02-recovery-disk-budget]]'
title: Recovery disk budget landed by operator instruction
type: report
updated: 2026-11-02T22:12:50.320Z
updated_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
verify:
  command: make test-recovery-disk
  exit_code: 0
---
The recovery/disk and lean-runner changes are now on main in signed commit 5256f9e3e9c3156d35a6906057d3a188d06afbaa. After rebase onto main 1a177bb6, signature-policy verification passed (one commit, zero historical exceptions), git diff --check passed, git push origin HEAD:main exited 0, and git ls-remote independently returned that exact SHA. The mute/unmute fix ffd3fc76 is already an ancestor of main.

The operator instructed: “push everything to main including tasks acceptance and such!” after the previous test waiver and the explicitly reported upstream formal blocker. This landing follows that operator direction using the completed checks below. It is an operator-directed gate exception, not a claim that the canonical/formal gate passed. No new Rust test run was performed for this rebase. Latest make init-submodules updated to the exact current harness pin 3d767101524a76e2797c683a93f37ce33d640441, then still exited 2 on the existing serde_json/preserve_order isolation check (/tmp/nyx-push-all-init.log). The upstream guard was not changed.

Implementation: recovery.rs:317 budgets 128 MiB plus 1 MiB per seat and observes retained filesystem bytes; the sparse oversized negative fixture rejects actual metadata over the bound. All real ring, journal, SQLite/checkpoint, SIGKILL, replay, screen, debt and reaping checks remain. Child stdout already uses Stdio::null(); the old 10 GiB requirement was a free-space floor, not an allocation. The lean runner accepts an opted-in nonempty integration target only when nextest metadata proves every case is ignored; explicit empty filters, malformed metadata and real build/test failures remain errors. Make targets, LAND.md, focused runner controls and HarnessFoldedCard coverage accompany those changes.

Completed evidence retained from the actual tested revisions: make test-recovery-disk test-chat-mute-notices smoke test-lean-ignored exited 0 at c09f2bd8. Four-seat recovery passed four tests at 28,225,846 bytes under 138,412,032; fifty-seat recovery passed at 45,836,472 under 186,646,528. Native chat/restart and private smoke passed; thirteen runner/controller tests and real nextest negative controls passed. /tmp/nyx-pinned-final-checks.log SHA256 ec042148aea3cd83dfd317d26911d6288393d168528c7a473e551dec5f158230. Both subsequent source commits separately compiled. At 8d78d3d2, strict Clippy, 308 Rust tests and nine corpus tests passed before the canonical gate stopped at upstream formal initialization; /tmp/nyx-canonical-final-land.log. These historical results are not relabelled as exact-current-head formal verification.

Cleanup is complete: all 25 reviewed worktrees aged at least 14 hours were removed, every removal exit 0; the authorised stale processes stopped. Independent audit verified 25 absent paths, 25 preserved HEAD references and all 12 salvaged local-data directories. Audit /tmp/nyx-old-worktrees-cleanup.json SHA256 d965b53dd32296ea6e524dbd1b6d7d49645a92118530e0aa1288b79deb0e7edf. Salvage remains under /Users/feral/worktree-salvage/2026-11-02.

Task report/acceptance is recorded through MCP. The resulting task Markdown, this append-only correction and linked artifact records are copied exactly from the host into the writing worktree, signed and pushed as record updates. No raw logs, fixture accounts, unrelated peer changes or trusted installation are included.