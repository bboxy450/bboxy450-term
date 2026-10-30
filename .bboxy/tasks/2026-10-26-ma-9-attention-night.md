---
authorship:
  device_id: 01M677XWZPW1XGTQYYGA50W74M
  issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
  kind: device
  principal: operator:01M677XWZPW1XGTQYYGA50W74M
  public_key: cHjhL+5pI9VNQE+Uhb4HJBnYoPqSIH7aCYU+b9MElWs=
  signer: device:01M677XWZPW1XGTQYYGA50W74M
created: 2026-10-26T11:24:58Z
created_by: device:01M677XWZPW1XGTQYYGA50W74M
id: 01M5VZ9QAFFKTBGQ7779KC225Y
issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
links:
- rel: spec
  to: '[[messaging-attention]]'
- rel: invariant
  to: '[[notices-never-wake]]'
- rel: acceptance
  to: '[[e2e-acceptance]]'
number: 343
owner: Kestrel Voidwing
provenance:
  digest: sha256:9fd7dcc85f3fda2da7ddef223bd82d36ddc4be063f5a07ea2b047348a954b817
  frontmatter_digest: sha256:7892659912543f2afec3f16af16704eb7b26fdc9ed31d3c9e965177d36d43bd6
  log: .bboxy/log/2026-10-30.jsonl#761
  sig: //nbtNKtH+nT6rmlDz80kxC4XaXoH3F9UWiGenP4pmAw3gbjxUEmNYviCnwgvfAH2ibFfsKVIOWD/4PpHlRrBQ==
  signer: device:01M677XWZPW1XGTQYYGA50W74M
reconciliation:
- by: operator:01M677XWZPW1XGTQYYGA50W74M
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/1316f3ddb01e7a5cea3c3c44e4540fb703c2e1b0
  fresh_accepted: false
  kind: historical
  op_id: 01M677YGA82G438Q58TZN40QY9
  previous_status: reported
  reason: Operator-authorized historical msgrec reconciliation, 2026-10-30; implementation already landed on main; no new acceptance claimed.
role: executor
status: done
title: 'MA-9: unattended event, DM and fifty-message digest-cap proof'
type: task
updated: 2026-10-30T20:27:47.028Z
updated_by: device:01M677XWZPW1XGTQYYGA50W74M
verify: bboxy450-term e2e night --group 'messaging and tasks'
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

## Report

Implementation landed through land.sh at `a345fdc0`. Canonical messaging night passed all seven scripts; M7 is full, the group amber only for prior partial M1/M3/M5/M6. Author checks: selected Rust969/0, strict Clippy0, invariants24/0, geometry87/87 with0mismatches; one unchanged icon advisory. Exact commands, exits, hashes, original fixture failures and private evidence paths: `.bboxy/runs/ma9-attention-night.log`. Postsubmit acceptance remains the Oracle's review.
