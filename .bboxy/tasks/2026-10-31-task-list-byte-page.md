---
artifacts:
- '[[01M6AJC6JBCSJ3C77G1EZR88NC]]'
- '[[01M6AR7KJAQ1AWSHTNXD8QH9K6]]'
assignee: seat:01M6ABGTWW9QF4DCT486NJ4ACM
authorship:
  kind: seat
  process_generation: '1'
  public_key: dNWbVeHRHyt/TnV6QHFyDXtWIj+UElZjvaWtEgWhcFg=
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
created: 2026-11-01T01:31:28.258Z
created_by: seat:01M6ABH71TVWGZ621NTD701PSH
delegated_by: seat:01M6ABH71TVWGZ621NTD701PSH
id: 01M6ABQ9H5X0NVDTC04349AVNG
number: 507
provenance:
  digest: sha256:85c0b9699565f94c4fd9d31049c653f25ad9adf7db5d862b3459178c0f9d60ae
  frontmatter_digest: sha256:fe2b27ce65e0a0fa4cd7e03bf30e4e8ebff74bcf969380da20348020a7e06b5f
  log: .bboxy/log/2026-11-01.jsonl#3668
  sig: tIM0yWUS/NXkVJn/moUr3b6xvgnI6g2Laxh9/9sVRhILyXKC8iij6k351JwS0DhcxmRG3TnzKR/u4sTHMZOnDw==
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
reconciliation:
- by: seat:01M6ABH71TVWGZ621NTD701PSH
  disposition: implemented
  evidence:
  - git ls-remote origin refs/heads/main — e7c6dd6ef5e5c4149368e03778b76baa0ad61004
  - crates/bbt-mux/src/mcp/work_items.rs:34 — TASK_PAGE_BYTES = MAX_RESULT - 16 KiB; list_projection drops bodies (body_truncated); list_page counts structured plus escaped text copy
  - ~/mydev/bboxy450-term-taskpage/target/ci-local.log:1,4959 — source/result e7c6dd6e normal 0; :2360-2361 byte-page and oversized-item tests ok; :2559 capture fairness ok
  - ~/.local/state/bboxy450-term/landing/landings.log — branch task-list-byte-page, tested_sha e7c6dd6e, base ceaf24e0, formal evidence 4b87a7f9…
  fresh_accepted: false
  kind: historical
  op_id: 01M6ARDYGXTYYEQKSGEPAF61F0
  previous_status: reported
  reason: 'Landed on origin/main e7c6dd6e (b2348007 plus the cherry-picked fairness fix), both commits signed G by Boxy. task_accept refused because the record''s verify command names the retired package bbt-mux. The Oracle verified the gate instead: ci-local.log sha256 21bec412… shows source e7c6dd6e and result normal 0, and landings.log records tier wide with formal 110/45/31.'
rejection:
  by: seat:01M6ABH71TVWGZ621NTD701PSH
  reason: 'Not landed: full make test red only at capture_fairness.rs:352 (stale pre-#47 assertion). Cherry-pick Lumen''s signed 2d17186d (same fix, branch plugins-continue), rerun make test on the exact commit, land with sh scripts/land.sh, then re-report with the landed main SHA. Work in fb7db213 itself verified: byte cut work_items.rs TASK_PAGE_BYTES = MAX_RESULT - 16 KiB, lean items, next = last emitted id. Widened scope is task #769, delegated separately.'
report:
  commit: e7c6dd6ef5e5c4149368e03778b76baa0ad61004
  op_id: 01M6ARD7F874KFF8CYWWSDBQAW
  reported_by: seat:01M6ABGTWW9QF4DCT486NJ4ACM
  summary: |-
    Landed #507 on main at e7c6dd6ef5e5c4149368e03778b76baa0ad61004. Implementation: b2348007702f868881d6e2176195a3bae649fcf6. The second commit is the requested signed cherry-pick of Lumen's 2d17186d capture-fairness fix, preserving its -x provenance. Artifact: [[01M6AR7KJAQ1AWSHTNXD8QH9K6]].

    Files and mechanism:
    - crates/bbt-mux/src/mcp/work_items.rs:34,1099,1135: task_list emits lean summaries, counts structured JSON plus its escaped MCP text copy, and stops under MAX_RESULT minus 16 KiB. next names the last emitted ID, so a byte cut never skips its first unreturned item. List bodies are omitted with body_truncated=true; metadata retains TaskCard/ctl identity, title, status, ownership, updated, number and verification exit code. Pathological scalar/path sizes are capped so one summary fits. The existing task_status/get projection at :1092 keeps its full metadata and existing 16 KiB body cap. Retained-operation guards are unchanged.
    - crates/bbt-mux/src/mcp/work_items_inventory_tests.rs:338,423: 160 tasks with exactly 16 KiB bodies, escaped long titles and heavy history are returned completely across byte pages. The same projection with the byte cut removed produces response_bound. A single oversized document is safely projected; full status data and the typed TaskCard adapter are checked.
    - crates/bboxy450-term/tests/messaging_ui.rs:465 proves 90 rows across three short pages follow the actual next IDs. tests/messaging_flow.rs:295 exercises the private real GUI socket against a live-root snapshot: 72 rows (71 valid source records plus one owned fixture), no issue/omission, all entries retained. Bodies/history are preserved while ownership/status/number are normalized to a throwaway identity. The helper at :242 now services the UI's existing bounded transient-read timer. The former helper stopped at workspace_pending/provider_busy and falsely reported zero rows; the fixed run exercised a 100 ms retry and rendered every row.
    - crates/bbt-mux/tests/mcp.rs:4143 waits for the exact indexed digest after direct fixture writes; task mutations still execute once. This fixes the previously exposed stale-reference fixture race.
    - tests/formal_contracts/task_page_model.rs:10 and models.rs register TaskBytePage in its own module. verification/{contracts,corpus,coverage}.toml bind source/witnesses and record finite bounds, assumptions and implementation-only coverage. Registration avoids the shared families array and mux coverage entry per LAND.md.
    - Makefile adds test-task-pages, test-task-pages-gui and check-task-pages. Root ATTRIBUTION.md and its 31 crate mirrors retain the project origin/credit policy.
    - crates/bbt-mux/tests/capture_fairness.rs:353 uses the existing 128-attempt ingestion quantum, retaining serial Screen equality and other-seat progress assertions. The supplied .bboxy/runs/plugins-continue-gap1.log is preserved.

    Commands and actual exits:
    - cargo check --workspace --all-targets --locked --target-dir target: 0 separately on b2348007 and e7c6dd6e after rebase. Logs: /tmp/vesper-taskpage-rebrief-{parent,head}-check.log.
    - BBOXY_TASK_BOARD_SOURCE=/Users/feral/mydev/bboxy450-term/.bboxy make test-task-pages: 0 on final committed source. 13 work-item tests, 1 acceptance integration, 24 sidebar tests, 8 native formal tests, and the 72-row real GUI test. Log: /tmp/vesper-taskpage-rebrief-focused-final.log; SHA256 0c0b3af2be7195cf6d485ca55947ce641fc553b0c86e462dcfc94d15e84ec082.
    - make test: 0 on exact signed e7c6dd6ef5e5c4149368e03778b76baa0ad61004. Full Rust suite, strict workspace/all-target Clippy -D warnings, formal verification, private headless smoke/10-second idle/shutdown, and 27 invariants passed. The formerly red capture-fairness test passed at target/ci-local.log:2559; acceptance at :2825; byte-page and oversized-item regressions at :2360-2361. Exact source/result markers at :1 and :4959. Log SHA256 21bec4123d75271c5e0edd35735c98ca2335b4ad090e600eb1cac1c5e004cadb.
    - sh scripts/land.sh --gate-log /Users/feral/mydev/bboxy450-term-taskpage/target/ci-local.log: 0. Reused that exact green gate; no incremental recheck/retry. Log /tmp/vesper-taskpage-rebrief-land.log, SHA256 dc157039e6b37e2543090de7198f5857374da5d13df8defd1674a93f632b188d.
    - git ls-remote origin refs/heads/main: 0, returned e7c6dd6ef5e5c4149368e03778b76baa0ad61004. git log verifies both commits G with approved key 9B07E849480C68328CA20684844ECC0587DE7AB0, Boxy <bboxy450@gmail.com>. git status is clean and branch equals origin/main.
    - make check-formal-evidence: 0 after landing. Clean-source receipt: 110 models, 45 contracts, 31 crates; native/TLC complete graphs and verdicts match. TaskBytePage: pass, 9 states/8 edges; BrokenTaskBytePage: safety violation, 15/15. Evidence SHA256 4b87a7f924589d30df72495aafcbbf8ea21c81a32e30eccda49b6057836eceff, archived at /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/4b87a7f924589d30df72495aafcbbf8ea21c81a32e30eccda49b6057836eceff/evidence.json. Landing receipt: /Users/feral/.local/state/bboxy450-term/landing/landings.log, source/tested SHA e7c6dd6e, base ceaf24e0.

    Gaps and scope: broader KB read work remains #769 per the rejection/rebrief; those paths were not changed. The installed operator app was not replaced; GUI evidence is the private socket/snapshot witness. Other ignored night suites, 11 canonical slow invariant checks and pixel parity were not run. The task's verify metadata still names retired package bbt-mux (its earlier literal command exited 101); the real bboxy450-term-mux package was checked through Make, and this rebrief's make test passed. The current knowledge_write type catalogue has no report type, so the committed report/trailer workflow remains unavailable through admitted tools; this native task report carries the full handoff. Only obsolete dependencies in this worktree's target/debug/deps were pruned; logs and formal evidence remain.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: task_list pages by bytes, not item count (tasks bar response_bound)
type: task
updated: 2026-11-01T05:13:33.987Z
updated_by: seat:01M6ABH71TVWGZ621NTD701PSH
verify: cargo test -p bbt-mux --locked work_items
verify_result:
  command: make test
  evidence:
  - criterion: 1. Byte-bounded pages; an oversized item is projected with body_truncated and continuation
    reference: crates/bbt-mux/src/mcp/work_items.rs:34,1099,1135; work_items_inventory_tests.rs:423; target/ci-local.log:2361 — both MCP JSON copies counted, 16 KiB margin, bounded single summary, next is last emitted ID.
  - criterion: 2. Lean lists retain GUI/ctl fields; task_status/task_get preserve the full read
    reference: crates/bbt-mux/src/mcp/work_items.rs:1092,1099; work_items_inventory_tests.rs:338 — list allowlist/body omission; full metadata and existing 16 KiB status-body projection unchanged and asserted.
  - criterion: 3. GUI follows next and renders the 66+ task live-repository volume without error
    reference: crates/bboxy450-term/tests/messaging_ui.rs:465; tests/messaging_flow.rs:295; /tmp/vesper-taskpage-rebrief-focused-final.log — three short pages retain 90 rows; real private GUI renders 72 normalized live-snapshot/fixture rows, no issue or omission.
  - criterion: 4. At least 128 large-body tasks list completely; removing the byte cut fails
    reference: crates/bbt-mux/src/mcp/work_items_inventory_tests.rs:338; target/ci-local.log:2360 — all 160 ordered IDs with 16 KiB bodies returned across bounded pages; identical uncut projection returns response_bound.
  - criterion: 5. Paging model and broken control updated and verified
    reference: crates/bboxy450-term/tests/formal_contracts/task_page_model.rs:10; verification/contracts.toml:396; target/ci-local.log:4709-4711 — native/TLC exact parity, valid 9/8, broken 15/15 safety failure. Clean landed-source evidence 4b87a7f924589d30df72495aafcbbf8ea21c81a32e30eccda49b6057836eceff.
  exit_code: 0
---
# task_list pages by bytes, not item count

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-taskpage`, branch `task-list-byte-page` (from origin/main 42320d93). Only this worktree.

## Defect (reproduced 2026-10-31 by the Oracle)
The sidebar tasks bar shows `response_bound: result exceeds the bounded response size`. MCP `task_list {limit:128}` returns `response_bound` ("knowledge result exceeds the MCP response bound"); `limit:10` returns 183,809 bytes. Each item carries full `body` (6–16 KB) and `metadata` (5–9 KB). Pages are cut by item count (`crates/bbt-mux/src/mcp/work_items.rs` list path ~L106–230, L824–843) while the whole result is checked against `MAX_RESULT` = 512 KiB (`crates/bbt-mux/src/mcp/mod.rs:32`, `journal.rs:464`, `knowledge.rs:496`). A full board can never fit, so the bar errors.

## Required outcome
1. A list page is cut on bytes too: stop adding items before the serialized result would exceed a margin under `MAX_RESULT`, and return `next` so the caller pages on. One item that alone exceeds the bound is projected with `body_truncated: true`, never a whole-page error.
2. List items are lean: list projection does not carry full bodies/heavy metadata history the tasks bar does not render; `task_status`/`task_get` stays the full read. Check what the GUI tasks bar and `ctl task list` actually consume and keep those fields.
3. The GUI tasks bar follows `next` (or requests a page that fits) and renders the board with no error for the live repo (66+ tasks).
4. Regression test: a fixture of ≥128 tasks with ~16 KB bodies lists every task across pages with no `response_bound`; broken control (byte cut removed) fails.
5. If retained-op replay or paging cursors are touched, update the matching model in `verification/` per AGENTS.md.

## Verify
`cargo test -p bbt-mux --locked work_items` plus the new regression; strict clippy on touched crates; `make test` on the committed source before handoff. Land with `scripts/land.sh` if green.

## Communication (operator order: no token burn)
No progress DMs, no channel posts, no acknowledgements. One `task_report` at the end: files+mechanism, commands+exit codes, evidence per item above, gaps, commit. Message the Oracle only for a blocker that stops all work, in ≤3 lines.


## Report from seat:01M6ABGTWW9QF4DCT486NJ4ACM

Implemented locally in signed commit fb7db213d5664e31889b62ba76f75fc247361ef2, branch task-list-byte-page, worktree /Users/feral/mydev/bboxy450-term-taskpage. Git status is clean; author Boxy <bboxy450@gmail.com>, signature G with 9B07E849480C68328CA20684844ECC0587DE7AB0. Source artifact [[01M6AJC6JBCSJ3C77G1EZR88NC]]. Not landed: the required full gate is red.

Files/mechanism: crates/bbt-mux/src/mcp/work_items.rs:34,1099,1135 adds lean list-only metadata, omits bodies, marks partial reads body_truncated=true, bounds pathological scalar fields/path, and counts both structured JSON and the escaped MCP text copy. Pages reserve 16 KiB below MAX_RESULT and next is the last emitted ID. task_status/ctl task get retain the existing full-metadata/16 KiB body projection. No retained-operation guard was changed.
crates/bbt-mux/src/mcp/work_items_inventory_tests.rs:338,423 checks 160 tasks with exactly 16 KiB bodies and heavy history, all IDs across byte-cut pages, full status preservation, and a huge single item. Removing the byte cut from the same projection produces response_bound. GUI/CLI TaskCard fields are retained.
crates/bboxy450-term/tests/messaging_ui.rs:465 proves three short pages retain all 90 rows and follow their actual next IDs. tests/messaging_flow.rs:288 exercises the real private GUI socket: 90 synthetic rows passed; the final live-repository snapshot passed 71 rows (70 source records plus one owned fixture). Snapshot ownership/status/number are normalized to a throwaway account; original bodies/history are preserved. The installed/live account was not changed.
tests/formal_contracts/task_page_model.rs:10, models.rs, verification/{contracts,corpus,coverage}.toml add TaskBytePage/BrokenTaskBytePage and both mux/app mappings. Native/TLC exact graphs match: valid 9 states/8 edges; uncut control safety violation, 15 states/15 edges. Scope, assumptions and implementation-only coverage are recorded.
crates/bbt-mux/tests/mcp.rs:4139 adds an exact-digest read barrier after that acceptance fixture's direct filesystem writes; task mutations still execute once. This fixes the stale-reference race exposed by the previous gate. Makefile adds test-task-pages, test-task-pages-gui and check-task-pages. ATTRIBUTION.md and its 31 crate mirrors were synchronized.

Commands and actual results:
- make test-task-pages: 0 after the fixture fix (13 work-item tests, acceptance integration, 24 UI tests, 7 native formal tests, 90-row real GUI test). Log /tmp/vesper-taskpage-fixture-fixed.log.
- BBOXY_TASK_BOARD_SOURCE=/Users/feral/mydev/bboxy450-term/.bboxy make test-task-pages-gui: 0 on final committed source; 71 rows, no issue/omission. Log /tmp/vesper-taskpage-gui-current-live.log.
- make test-licenses: 0. Log /tmp/vesper-taskpage-licenses.log.
- make test: 2 on final committed source; underlying workspace test exit 101. Strict cargo clippy --workspace --all-targets --locked -- -D warnings passed. Byte-page, oversized-item and acceptance tests passed. The sole failing target was capture_fairness (below). Log target/ci-local.log, SHA256 8519bc0d4210cb3d5ec4b233264cd07130010e2ac097ca2b9e38b004528630d8.
- Exact current capture_fairness test executable, isolated failed test: 0; this does not make the full gate green or authorize quarantine. Log /tmp/vesper-taskpage-capture-isolated.log.
- make test-formal: 0 on final source; 100 models, 43 contracts, 31 crates; complete native/Java graph and verdict parity. Log /tmp/vesper-taskpage-formal-current.log.
- make check-formal-evidence: 0. Clean-source receipt fb7db213d5664e31889b62ba76f75fc247361ef2, evidence SHA256 d3bc2de7236089f020584649a00ec4368b4e09129d6a36c418b7af9384876519; archive /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/d3bc2de7236089f020584649a00ec4368b4e09129d6a36c418b7af9384876519/evidence.json.
- make smoke: 0; private ad-hoc bundle, throwaway identity, headless render/idle/shutdown. Log /tmp/vesper-taskpage-smoke.log.

Blocking full-gate finding: four_watched_seats_advance_during_a_large_replay_and_match_serial_screens failed at crates/bbt-mux/tests/capture_fairness.rs:352: instrumentation observed more than one page per turn. All four serial screens matched; the same binary passed alone. The test is unlisted in .bboxy/quarantine.md, so the canonical runner refused it. Capture/ingestion source and this assertion were not changed; no quarantine or gate weakening was added. A capture-fairness owner disposition/fix is needed before landing. scripts/land.sh was not run because the brief authorizes landing if green.

Other gaps: the brief's cargo -p bbt-mux selector is retired (actual exit 101, /tmp/vesper-taskpage-work-items.log); the real package bboxy450-term-mux was checked through Make. Its task verify metadata needs correction. Initial missing formal coverage was fixed; earlier stale-reference acceptance failure was fixed by the fixture barrier. Current live knowledge_write.frontmatter.type has no report variant, and this worktree has no .bboxy/reports directory; the new committed-report/trailer workflow is unavailable through admitted tools. No shell bypass or invented report link was used. Full evidence is recorded here instead. Ignored night suites, live installation/rollout and pixel parity were not claimed.

```json
{
  "commit": "fb7db213d5664e31889b62ba76f75fc247361ef2",
  "op_id": "01M6AJDVA1FQNZK3RXRXKZTZJ7",
  "verify_result": {
    "command": "make test",
    "evidence": [
      {
        "criterion": "1. Byte-bounded pages and oversized-item projection",
        "reference": "crates/bbt-mux/src/mcp/work_items.rs:34,1099,1135 — lean bounded summaries; duplicated escaped MCP JSON is counted, with 16 KiB scope margin; next names the last emitted ID. Current CI byte/oversize tests passed at target/ci-local.log:2263."
      },
      {
        "criterion": "2. Lean list data and preserved full reads",
        "reference": "crates/bbt-mux/src/mcp/work_items.rs:1092,1099 — task_status projection unchanged; list keeps TaskCard title/status/assignee/delegator/updated/exit_code plus scalar identity fields. crates/bbt-mux/src/mcp/work_items_inventory_tests.rs:338 asserts full status metadata/body preservation."
      },
      {
        "criterion": "3. GUI follows next and handles live-board volume",
        "reference": "crates/bboxy450-term/tests/messaging_ui.rs:465 — 90 rows across three short pages. tests/messaging_flow.rs:288 — real private GUI socket. /tmp/vesper-taskpage-gui-current-live.log — exit 0, 71 rows from normalized live snapshot plus one fixture; installed account not changed."
      },
      {
        "criterion": "4. At least 128 large-body tasks and broken control",
        "reference": "crates/bbt-mux/src/mcp/work_items_inventory_tests.rs:338 — 160 tasks with 16 KiB bodies, escaped titles and heavy history, complete ordered IDs across byte pages; same projection without byte cut fails response_bound. Final current-source CI test passed."
      },
      {
        "criterion": "5. Paging model and negative control",
        "reference": "crates/bboxy450-term/tests/formal_contracts/task_page_model.rs:10; verification/contracts.toml:376; verification/coverage.toml:146,204 — TaskBytePage valid 9 states/8 edges; BrokenTaskBytePage safety 15/15, exact native/TLC parity. Clean final-source evidence d3bc2de7236089f020584649a00ec4368b4e09129d6a36c418b7af9384876519."
      }
    ],
    "exit_code": 2
  }
}
```


## Report from seat:01M6ABGTWW9QF4DCT486NJ4ACM

Landed #507 on main at e7c6dd6ef5e5c4149368e03778b76baa0ad61004. Implementation: b2348007702f868881d6e2176195a3bae649fcf6. The second commit is the requested signed cherry-pick of Lumen's 2d17186d capture-fairness fix, preserving its -x provenance. Artifact: [[01M6AR7KJAQ1AWSHTNXD8QH9K6]].

Files and mechanism:
- crates/bbt-mux/src/mcp/work_items.rs:34,1099,1135: task_list emits lean summaries, counts structured JSON plus its escaped MCP text copy, and stops under MAX_RESULT minus 16 KiB. next names the last emitted ID, so a byte cut never skips its first unreturned item. List bodies are omitted with body_truncated=true; metadata retains TaskCard/ctl identity, title, status, ownership, updated, number and verification exit code. Pathological scalar/path sizes are capped so one summary fits. The existing task_status/get projection at :1092 keeps its full metadata and existing 16 KiB body cap. Retained-operation guards are unchanged.
- crates/bbt-mux/src/mcp/work_items_inventory_tests.rs:338,423: 160 tasks with exactly 16 KiB bodies, escaped long titles and heavy history are returned completely across byte pages. The same projection with the byte cut removed produces response_bound. A single oversized document is safely projected; full status data and the typed TaskCard adapter are checked.
- crates/bboxy450-term/tests/messaging_ui.rs:465 proves 90 rows across three short pages follow the actual next IDs. tests/messaging_flow.rs:295 exercises the private real GUI socket against a live-root snapshot: 72 rows (71 valid source records plus one owned fixture), no issue/omission, all entries retained. Bodies/history are preserved while ownership/status/number are normalized to a throwaway identity. The helper at :242 now services the UI's existing bounded transient-read timer. The former helper stopped at workspace_pending/provider_busy and falsely reported zero rows; the fixed run exercised a 100 ms retry and rendered every row.
- crates/bbt-mux/tests/mcp.rs:4143 waits for the exact indexed digest after direct fixture writes; task mutations still execute once. This fixes the previously exposed stale-reference fixture race.
- tests/formal_contracts/task_page_model.rs:10 and models.rs register TaskBytePage in its own module. verification/{contracts,corpus,coverage}.toml bind source/witnesses and record finite bounds, assumptions and implementation-only coverage. Registration avoids the shared families array and mux coverage entry per LAND.md.
- Makefile adds test-task-pages, test-task-pages-gui and check-task-pages. Root ATTRIBUTION.md and its 31 crate mirrors retain the project origin/credit policy.
- crates/bbt-mux/tests/capture_fairness.rs:353 uses the existing 128-attempt ingestion quantum, retaining serial Screen equality and other-seat progress assertions. The supplied .bboxy/runs/plugins-continue-gap1.log is preserved.

Commands and actual exits:
- cargo check --workspace --all-targets --locked --target-dir target: 0 separately on b2348007 and e7c6dd6e after rebase. Logs: /tmp/vesper-taskpage-rebrief-{parent,head}-check.log.
- BBOXY_TASK_BOARD_SOURCE=/Users/feral/mydev/bboxy450-term/.bboxy make test-task-pages: 0 on final committed source. 13 work-item tests, 1 acceptance integration, 24 sidebar tests, 8 native formal tests, and the 72-row real GUI test. Log: /tmp/vesper-taskpage-rebrief-focused-final.log; SHA256 0c0b3af2be7195cf6d485ca55947ce641fc553b0c86e462dcfc94d15e84ec082.
- make test: 0 on exact signed e7c6dd6ef5e5c4149368e03778b76baa0ad61004. Full Rust suite, strict workspace/all-target Clippy -D warnings, formal verification, private headless smoke/10-second idle/shutdown, and 27 invariants passed. The formerly red capture-fairness test passed at target/ci-local.log:2559; acceptance at :2825; byte-page and oversized-item regressions at :2360-2361. Exact source/result markers at :1 and :4959. Log SHA256 21bec4123d75271c5e0edd35735c98ca2335b4ad090e600eb1cac1c5e004cadb.
- sh scripts/land.sh --gate-log /Users/feral/mydev/bboxy450-term-taskpage/target/ci-local.log: 0. Reused that exact green gate; no incremental recheck/retry. Log /tmp/vesper-taskpage-rebrief-land.log, SHA256 dc157039e6b37e2543090de7198f5857374da5d13df8defd1674a93f632b188d.
- git ls-remote origin refs/heads/main: 0, returned e7c6dd6ef5e5c4149368e03778b76baa0ad61004. git log verifies both commits G with approved key 9B07E849480C68328CA20684844ECC0587DE7AB0, Boxy <bboxy450@gmail.com>. git status is clean and branch equals origin/main.
- make check-formal-evidence: 0 after landing. Clean-source receipt: 110 models, 45 contracts, 31 crates; native/TLC complete graphs and verdicts match. TaskBytePage: pass, 9 states/8 edges; BrokenTaskBytePage: safety violation, 15/15. Evidence SHA256 4b87a7f924589d30df72495aafcbbf8ea21c81a32e30eccda49b6057836eceff, archived at /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/4b87a7f924589d30df72495aafcbbf8ea21c81a32e30eccda49b6057836eceff/evidence.json. Landing receipt: /Users/feral/.local/state/bboxy450-term/landing/landings.log, source/tested SHA e7c6dd6e, base ceaf24e0.

Gaps and scope: broader KB read work remains #769 per the rejection/rebrief; those paths were not changed. The installed operator app was not replaced; GUI evidence is the private socket/snapshot witness. Other ignored night suites, 11 canonical slow invariant checks and pixel parity were not run. The task's verify metadata still names retired package bbt-mux (its earlier literal command exited 101); the real bboxy450-term-mux package was checked through Make, and this rebrief's make test passed. The current knowledge_write type catalogue has no report type, so the committed report/trailer workflow remains unavailable through admitted tools; this native task report carries the full handoff. Only obsolete dependencies in this worktree's target/debug/deps were pruned; logs and formal evidence remain.

```json
{
  "commit": "e7c6dd6ef5e5c4149368e03778b76baa0ad61004",
  "op_id": "01M6ARD7F874KFF8CYWWSDBQAW",
  "verify_result": {
    "command": "make test",
    "evidence": [
      {
        "criterion": "1. Byte-bounded pages; an oversized item is projected with body_truncated and continuation",
        "reference": "crates/bbt-mux/src/mcp/work_items.rs:34,1099,1135; work_items_inventory_tests.rs:423; target/ci-local.log:2361 — both MCP JSON copies counted, 16 KiB margin, bounded single summary, next is last emitted ID."
      },
      {
        "criterion": "2. Lean lists retain GUI/ctl fields; task_status/task_get preserve the full read",
        "reference": "crates/bbt-mux/src/mcp/work_items.rs:1092,1099; work_items_inventory_tests.rs:338 — list allowlist/body omission; full metadata and existing 16 KiB status-body projection unchanged and asserted."
      },
      {
        "criterion": "3. GUI follows next and renders the 66+ task live-repository volume without error",
        "reference": "crates/bboxy450-term/tests/messaging_ui.rs:465; tests/messaging_flow.rs:295; /tmp/vesper-taskpage-rebrief-focused-final.log — three short pages retain 90 rows; real private GUI renders 72 normalized live-snapshot/fixture rows, no issue or omission."
      },
      {
        "criterion": "4. At least 128 large-body tasks list completely; removing the byte cut fails",
        "reference": "crates/bbt-mux/src/mcp/work_items_inventory_tests.rs:338; target/ci-local.log:2360 — all 160 ordered IDs with 16 KiB bodies returned across bounded pages; identical uncut projection returns response_bound."
      },
      {
        "criterion": "5. Paging model and broken control updated and verified",
        "reference": "crates/bboxy450-term/tests/formal_contracts/task_page_model.rs:10; verification/contracts.toml:396; target/ci-local.log:4709-4711 — native/TLC exact parity, valid 9/8, broken 15/15 safety failure. Clean landed-source evidence 4b87a7f924589d30df72495aafcbbf8ea21c81a32e30eccda49b6057836eceff."
      }
    ],
    "exit_code": 0
  }
}
```
