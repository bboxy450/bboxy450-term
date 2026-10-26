---
id: "01M5VZ9QAFFKTBGQ7779KC225Y"
type: "task"
number: 343
title: "MA-9: unattended event, DM and fifty-message digest-cap proof"
status: "claimed"
role: "executor"
owner: "Kestrel Voidwing"
verify: "bboxy450-term e2e night --group 'messaging and tasks'"
links:
  - { to: "[[messaging-attention]]", rel: spec }
  - { to: "[[notices-never-wake]]", rel: invariant }
  - { to: "[[e2e-acceptance]]", rel: acceptance }
created: "2026-10-26T11:24:58Z"
updated: "2026-10-26T11:24:58Z"
---
# MA-9 attention night

Assigned by Vesper in `99edc18d-9d85-47d3-992d-ad0a6b97e213`; product priority reconfirmed by `b3c9a2bb-d057-4890-a487-efaf8f7a88d3`. MA-3's admission policy (`ffd8c568`) and MA-4's persisted bounded digests (`6014578f`, `866859d0`) are on the starting main `2d5c2dbd`.

## Scope

- Add the attention promise and its full-coverage script together: `.bboxy/specs/e2e-acceptance.md` section 1, `.bboxy/specs/e2e-script-map.md`, generated `lab/web/e2e.js`, and `scripts/e2e/M7.sh` with its private native-client/scripted-model fixtures and assertions.
- Mechanical promise-count/list ports only in `crates/bbt-ui/tests/e2e_checklist.rs`, `crates/bboxy450-term/tests/e2e_night.rs`, and existing unattended-night coverage checks if needed.
- Use existing ctl/MCP, headless app, private per-run file account and the installed Codex client against a loopback scripted model. No production messaging, policy, backend time, renderer, landing or CI behavior change.

## Acceptance

1. A typed event admitted under the idle receiver's role defaults is stored, receipted with wake none and never reaches the native model input.
2. A direct conversation reaches that receiver once through the real native delivery path and wakes a turn.
3. The first 49 channel messages are durably receipted as deferred; message 50 flushes exactly one native digest containing all 50 IDs in order. The injected-clock window boundary remains the existing MA-4 unit contract, never a ten-minute wall wait.
4. Script assertions and exits decide the verdict, coverage full. The canonical runner writes the promise's e2e record with the derived test-account label; no window, operator account, login Keychain, remote provider or credentials.
5. Land through `scripts/land.sh`; retain command exits, raw-log hashes, exact native request/receipt evidence and any uncovered clause. One author validation, no separate precheck or full gate.

Overlap: `scripts/overlap.sh` is absent on the starting main; read-only tracked-diff checks of exec3/4/6, Nyx and Rook found no in-flight overlap in these paths. Rebase any later shared count/map edits.
