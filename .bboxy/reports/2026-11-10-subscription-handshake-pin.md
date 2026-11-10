---
id: 01M732K87D0T13HYWMHR95EHAZ
number: 1794325979374
type: report
title: Pin bounded harness subscription setup
task: '[[tasks/2026-11-04-codex-trusted-handoff]]'
commits: []
author: seat:01M732K87DXDW1NJPK0D2CMM42
created: '2026-11-10T15:52:59.373791+00:00'
verify:
  command: CARGO_NET_OFFLINE=true make bump-harness REV=60807ac595de8bc0b5e69664cd6a36bd51c4f237
  exit_code: 0
---

Direct operator request: investigate the rich-harness subscription loader failure and push a quick fix to main, skipping most tests. Standalone Codex execution in an owned worktree based on 3d3a708b5d6b7ed9ce10df3ad1ed047502526715; the task locator is historical context, and the author labels this execution rather than a native seat signature.

crates/bboxy450-harness advances from 20e52d0f937c7ed19abbf506564a3dd8a0c8cdd4 to signed producer main 60807ac595de8bc0b5e69664cd6a36bd51c4f237. Connect, subscription write and Hello read now share one five-second deadline, with socket cancellation before an IO owner starts. The GUI preserves the timeout/protocol reason and explicitly says reconnecting. Command acknowledgement, same-op-id and session-identity behavior remain unchanged. Cargo.lock did not change.

The source report is https://github.com/bboxy450/bboxy450-harness/blob/60807ac595de8bc0b5e69664cd6a36bd51c4f237/.bboxy/reports/2026-11-10-subscription-handshake-recovery.md. Four focused real Unix bridge checks passed: missing endpoint/detach, dispatch/detach, held Hello followed by automatic recovery, and A/B/A session replacement without retargeting intent. Formatting, source/notice checks and TOML parsing passed. Fresh passive subscriptions to the live worker were fast; the screenshot's original transient cause is unconfirmed, so this is bounded handshake recovery and clearer diagnostics, not a claim that a timeout was reproduced.

CARGO_NET_OFFLINE=true make bump-harness REV=60807ac595de8bc0b5e69664cd6a36bd51c4f237 exited 0 and staged only the gitlink. git diff --check exited 0. Exact pin command and log hash: .bboxy/runs/2026-11-10-subscription-handshake-pin.json. Producer commit signature verified with the approved Boxy key. Full CI, formal/native/TLC, broad smoke, visual and bundle checks were skipped under the explicit quick-fix exception; no make land was used. Both commits request [skip ci].

No app rebuild, installation, UI/Logic/Supervisor restart, working-seat interruption or runtime/history change was performed. GUI 35862, Logic 36049 and Supervisor 84759 retained their recorded start times. Shared checkouts and their existing submodule state were preserved. The containing signed commit is this pin checkpoint; only the pin, report and command/hash summary are staged.
