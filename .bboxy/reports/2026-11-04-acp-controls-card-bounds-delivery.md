---
type: report
title: Focused harness card bounds delivery
task: '[[01M6KF628K0H7DHN4R8BCN9WVZ]]'
supersedes: '[[01M6KJKX4CVY6TPFHV939A12T7]]'
commits:
- 6aac038f62e92556e7c3fc2fa9ba90de499a6cb6
- a832bd6ff4b31a36064508632c47840a58cfe56c
- 9501b79845cd7486738e6bf948a09813e70d87cf
verify:
  command: make verify-harness-card-bounds
  exit_code: 0
author: seat:01M6KF1JD34QA47CQP34FRJN5Z
created: 2026-11-04T15:34:24.560Z
id: 01M6KK4W74AF821D2G5PAT8TJK
updated: 2026-11-04T15:34:24.560Z
number: 93377961360887
created_by: seat:01M6KF1JD34QA47CQP34FRJN5Z
updated_by: seat:01M6KF1JD34QA47CQP34FRJN5Z
authorship:
  signer: seat:01M6KF1JD34QA47CQP34FRJN5Z
  kind: seat
  public_key: El6VqSdMLzX3+r6uiKxbH22bBB8VwVCtourrWmlf8Yk=
  process_generation: '1'
provenance:
  digest: sha256:d8bba5c0628b7f30d1702ac6d3d2ed63d562a352c5f8d2ef6e0c3ec932151eac
  frontmatter_digest: sha256:4e03415e03fc66dd797aeee7c133a9c1a6b27e2c331cd8ac5fda867f57f72ffa
  signer: seat:01M6KF1JD34QA47CQP34FRJN5Z
  sig: R045XN51k3T0WC+w+BsoIeQyLIVscCisaZOLwCUnWE0upygk3WeEexKd6CEBswclYszKFz9mAEsrulbN198XDA==
  log: .bboxy/log/2026-11-04.jsonl#120074
---
Completed the narrowed card-bounds handoff for task [[01M6KF628K0H7DHN4R8BCN9WVZ]]. This append-only report supersedes [[01M6KJKX4CVY6TPFHV939A12T7]] for the changed verification command; the original report [[01M6KGWYYSF1W37CJ3T7ETB898]] and all previous signed source remain preserved.

Signed gate/witness commit: `6aac038f62e92556e7c3fc2fa9ba90de499a6cb6`, parent `133d0cc53ca610346e72a2a9cf293a0f1df25607`. Signature verification returned G for Boxy (bboxy450). The inherited renderer is unchanged from `a832bd6ff4b31a36064508632c47840a58cfe56c`; no source/model/process implementation was redone.

- `Makefile` adds `verify-harness-card-bounds`. `scripts/verify-harness-card-bounds.py` runs exactly three named commands, hashes the fresh inputs/logs/captures and writes `.bboxy/runs/codex-acp-controls-card-bounds.json`. It never invokes the old ACP/formal gate.
- The script retrieves the original core receipt from signed commit `9501b79845cd7486738e6bf948a09813e70d87cf`, verifies its exact SHA256 `f46be4ea94b79fbe57bf8a1a8917b0482ef2996a59318d008e49097c5c1310b2`, and verifies 184 semantic source/fixture/formal inputs byte-for-byte against that commit. This includes non-GUI backend/protocol/engine sources, GUI model/bridge, ACP test inputs, fixtures and verification models/tools. Original 15-command evidence is retained as earlier evidence; its logs were independently verified by Oracle per the amended brief and are not reconstructed or relabeled as fresh.
- The inherited renderer receipt from a832bd6 is separately verified as SHA256 `25a795b1a2e5d88e9008a4c935ea23039c8911f46f92fc3ecf7f5c13ec39ab5c`. Its source inputs remain exact except the newly proved Make/attribution changes. This retains the earlier centered welcome-icon/Loading…/motion, queued-history GPU pixels, terminal/MCP and bounded-formal results without rerunning them.
- `tests/gui_card_bounds.rs` renders the real production `face::card` with a deliberately oversized command/heredoc child at narrow and wide sizes. The normal guard produces zero escaped pixels at both sizes. Changing only `panel.clip` to false produces 4,520 escaped pixels at width 180 and 1,972 at width 420. The control keeps identical card geometry, so it demonstrates clipping rather than a changed label or rectangle. Four normal/broken native captures and their hashes are retained under `target/harness-card-bounds/captures/`.
- Existing `tests/gui_rich.rs` long-command/heredoc native checks run unchanged at widths 280 and 1120. They verify bounded headers/panes, compact/expanded command heights, full command/output tail access, independent horizontal arrow/vertical scrolling and 90/100/160% text-zoom extents. The named existing ACP native options test reruns because the inherited shared view code changed.

Actual fresh command results, all exit 0:

1. `cargo test --offline --locked --jobs 2 --target-dir target --features embedded --test gui_card_bounds --test gui_rich acp_controls_tool_cards -- --nocapture` — one real pixel/broken-control test and two retained native geometry/zoom tests pass.
2. `cargo clippy --offline --locked --jobs 2 --target-dir target --no-default-features --features embedded --lib --no-deps -- -D warnings` — strict affected embedded compile passes.
3. `cargo test --offline --locked --jobs 2 --target-dir target --features embedded --test acp_controls acp_controls_native_popover_routes_real_selection_and_unavailable_labels -- --nocapture` — one named native GUI options test passes using its private scripted ACP fixture; no live vendor request.

`make verify-harness-card-bounds` exits 0. The committed compact receipt is SHA256 `e771eb6165b13049343881d0ba06e15e2544c695cd4cd85f8f709743ad8ade2f`; it records exact absolute commands, real exits, output-log hashes, 213 checked input hashes, retained-proof scopes and four capture hashes. All checked source hashes are compared with committed bytes before handoff. `git diff --check`, `git diff --cached --check` and `python3 -I scripts/checks.py headers` also exit 0.

Only Makefile, attribution, the new small verification script, the new pixel witness and its run JSON changed in this checkpoint. No pull/fetch/merge/rebase/cherry-pick/pin changes, additional workers, old ACP/formal gate, whole suites, paid/live providers, live desktop/app/backend operations, installation or publication. Previous renderer fixes remain intact; status-title and readonly handoff work remains with the separate term worker. The source checkpoint worktree was clean; this indexed report is the subsequent intended change. Oracle owns independent acceptance and activation.
