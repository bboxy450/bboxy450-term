---
type: task
title: Oracle messages to a busy Codex executor stay native-queued for its whole turn; surface them inside the turn
role: executor
allowed_paths:
- crates/bbt-mux/src/mcp/
- crates/bbt-mux/src/messaging/
- crates/bbt-mux/src/adapters/
- crates/bbt-mux/tests/
- crates/bboxy450-term/src/
- verification/
- .bboxy/reports/
- crates/bbt-mux/src/logic.rs
- crates/bbt-mux/src/seat_socket.rs
- crates/bbt-proto/src/messaging.rs
- crates/bbt-proto/tests/
- crates/bboxy450-harness-wake-up/src/
- crates/bboxy450-harness-wake-up/tests/
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
- crates/bboxy450-term/tests/native_signing.rs
- crates/bbt-mux/src/index.rs
- crates/bbt-mux/src/knowledge/routing_tests.rs
verify: cargo nextest run -p bboxy450-term-mux -E 'test(/hint|unread|steer|deliver/)' && make test-formal-contracts
status: accepted
id: 01M6S5E4NHPW1S8AJ2X64Z1H8W
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-06T22:05:40.784Z
created: 2026-11-06T19:30:13.854Z
number: 93377961366094
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M6S7423SHR6V4WNJP29GVSK0
verify_result:
  command: cargo nextest run -p bboxy450-term-mux -E 'test(/hint|unread|steer|deliver/)' && make test-formal-contracts
  exit_code: 0
  evidence:
  - criterion: Bounded in-turn surfacing and exact read/ACK suppression
    reference: crates/bbt-mux/src/messaging/ledger/surfacing.rs:25; crates/bbt-mux/src/seat_socket.rs:1870; crates/bbt-mux/tests/mcp/oracle_hints.rs:39 and :112; /tmp/oracle-in-turn-final-verify.log — 41 selected tests passed
  - criterion: Native Oracle steering without cancellation
    reference: crates/bbt-mux/src/logic.rs:9709; crates/bbt-mux/src/adapters/native.rs:30; crates/bbt-mux/tests/mcp/oracle_hints.rs:189 — private Codex RPC witness passed
  - criterion: Honest queued/hinted/steered/acknowledged receipts and additive wire compatibility
    reference: crates/bbt-mux/src/messaging/ledger/surfacing.rs:45; crates/bbt-proto/src/messaging.rs:428; crates/bbt-proto/tests/message_evidence.rs — 29 proto tests passed in landing gate
  - criterion: Busy-turn, worker rate limit and disabled-hints broken control
    reference: crates/bbt-mux/tests/mcp/oracle_hints.rs:39,:112,:161; formal OracleInTurn pass 5898 states/38756 edges, BrokenOracleInTurn expected safety failure 2388 states/15464 edges; source-bound evidence SHA256 194bfb9618113d148e78ea9dede16aa2b02e9640eb588ab7526367b29f2a35c3
  - criterion: Signed landing and unattended verification
    reference: target/oracle-in-turn-land.log:1089 — LANDED d88df43b73162494c40fea87dc910edfb4a73677, make land exit 0; make test-python-execute-native, make test-boundaries, make test-licenses and make check-formal-evidence exit 0; git verify-commit HEAD exit 0; clean worktree
  - criterion: Committed report and provenance handoff limitation
    reference: knowledge_commit operation 01M6SE7MQW0EQFZED9VFNJZEFN — publication_state committed, checkpoint d5c3fc6747210c237b12f76f821a063942c658e0; artifact 01M6SE7NT4XKMJ5R027HB0Y30B; artifact_claim operations 01M6SE7ZVJC3HKRENZDQYZQDKW and 01M6SE96DBHNFF9TMRPK0MXRN0 explicitly rejected artifact_not_committed; no further retries
report:
  summary: 'Landed signed d88df43b73162494c40fea87dc910edfb4a73677 through make land: bounded Oracle unread hints now surface on active MCP calls, Codex Oracle input uses turn/steer without cancellation, and receipts distinguish native admission, hint/read and explicit ACK. Prescribed verification passed (41 selected tests and 24 contracts); lean gate passed 550 tests plus 570-model native/Yarn/Java TLC parity, with current-source receipt validation, private Python proxy, process boundaries and license checks all exit 0. No reinstall. Full report [[reports/2026-11-06-oracle-messages-in-turn]] is committed in native knowledge checkpoint d5c3fc6747210c237b12f76f821a063942c658e0. Handoff gap: artifact_claim was refused as artifact_not_committed despite the committed checkpoint and a fully caught-up provenance projection; no claim success is asserted.'
  commit: d88df43b73162494c40fea87dc910edfb4a73677
  reported_by: seat:01M6S7423SHR6V4WNJP29GVSK0
  op_id: 01M6SEA463R12E1VG5NC6N0NGM
  verify_revision: 0
  report: '[[reports/2026-11-06-oracle-messages-in-turn.md]]'
provenance:
  digest: sha256:cc89454c541590f0a07c43f3ff72f98cb364164b648030b447532d49790b0044
  frontmatter_digest: sha256:6396e7230b908ba3f9794aacb33cb38d789dae7d2ffc537646d0aa7da9434794
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: doTCuS5/1dhNmu/HribM7vFaVkC4aSKt4n8sBKLGGzKKfLgDyMx0znuyvlwjoLSUQDb0/YUIffn/yfj+muQ5CA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149906
  event:
    id: 01M6SEASNXEQPX18C1S7PHYZDG
    sequence: 149906
    hash: 28abf621d61e065a4ea4829c43c2ecb6eb8473f06dc853f1b41c11584cee67c5
---
# Rulings don't reach working executors

Live evidence, 2026-11-06. Oracle ruling DMs `01M6S2EVSGVANNRKQW96D4NMAY` (18:38) and `01M6S3C0ZZ305MNKA2B2T3C9TM` (18:54) to Codex executor 01M6RZZXSHZT3KHBK1QC7K3XB3 still showed `evidence: native-queued, note: native admission only; model consumption unverified` in `message_receipts` at about 19:30. Meanwhile the executor twice reported that decision as still open, and kept working on assumptions the rulings had already settled. The C1 executor likewise re-sent a blocker that my reply had already answered. Codex consumes queued input only at a turn boundary, and these executors run turns lasting tens of minutes.

## Required

1. **In-turn surfacing (main fix).** Executors call bboxy MCP tools constantly. Every MCP tool result for a seat that has unread messages from its delegating Oracle (or any oracle-class sender) carries a structured hint with the count, sender, message IDs and first line, plus a `message_list` call to read them. The hint is bounded (≤5 messages, ≤512 B each) and appears once per message until it is read or acked. No new wire version: reuse the existing structured `hints`.
2. **Native steer where supported.** If the harness adapter supports injecting into a running turn (Codex app-server turn steering, if available), deliver oracle-class messages that way and record the evidence as `native-steered`. Otherwise fall back to (1). Never interrupt or cancel the turn.
3. **Honest receipts.** `message_receipts` distinguishes queued, hinted (the surfacing hint was returned in a tool result), steered and model-acknowledged. The Oracle can then tell whether a ruling was seen.
4. **Tests.** A busy seat with a long-running turn plus an Oracle DM gets the hint on its next MCP call. Duplicate suppression works once the message is read. A non-oracle sender rate limit keeps worker-to-worker DMs from spamming hints. Broken control: hints disabled means the test seat never sees the ruling, and the test must fail.

Lean landing; no reinstall. Write the report `reports/2026-11-06-oracle-messages-in-turn.md` and call `task_report` once.

**Amend (Oracle):** granted logic.rs (dispatch ~3825), seat_socket.rs (~1867), bbt-proto/src/messaging.rs plus its tests, and bboxy450-harness-wake-up src/tests (Codex `turn/steer` at native.rs:1159 already exists, but its evidence collapses to NativeQueued). WIRE RULE: `Evidence` in bbt-proto is a persisted wire enum, so the change is ADDITIVE ONLY. Append new variants (e.g. `NativeSteered`, `Hinted`) at the end. Never rename, remove or reorder existing variants. Unknown-variant handling for old readers must not fail closed on a receipt read; if a golden or wire test covers it, extend that test. Formal coverage for wake-up is in scope under verification/.

**Amend 2 (Oracle):** granted crates/bbt-mux/src/models.rs (family registration :36) and crates/bbt-mux/models/ for the in-turn model and its disabled-hints broken control.

**Amend 3 (Oracle):** granted crates/bboxy450-term/tests/native_signing.rs for a FIXTURE-ONLY repair. Base b28f0efa (Maksim Soltan) added scripts/signing-identity.sh, which finds the identity in the Keychain, and the private bundle fixture omits it (Oracle confirmed). The fixture must provide a STUB signing-identity.sh that prints a fixed fake/ad-hoc identity. NEVER copy or run the real script in a test, because a Keychain lookup can prompt the operator, and unattended tests are a hard rule. Do not change scripts/signing-identity.sh itself.

**Amend 4 (Oracle):** fixture-only race repairs granted: crates/bbt-mux/src/index.rs (test at ~1756) and crates/bbt-mux/src/knowledge/routing_tests.rs (~524). (a) FSEvents wait: make the fixture wait bounded and event-driven with a generous cap (≤30 s) instead of a fixed 5 s, and keep the same assertion. The 5 s timeout coincided with an Oracle cleanup that deleted 55 worktrees and 39 GiB at once, flooding fseventsd. (b) routing test: accept that a one-shot knowledge_fetch may legitimately return workspace_pending after the ready probe; poll it bounded until ready, then assert the same final result. No product code or assertion weakening, and no test deleted or ignored.


## Report from seat:01M6S7423SHR6V4WNJP29GVSK0

Landed signed d88df43b73162494c40fea87dc910edfb4a73677 through make land: bounded Oracle unread hints now surface on active MCP calls, Codex Oracle input uses turn/steer without cancellation, and receipts distinguish native admission, hint/read and explicit ACK. Prescribed verification passed (41 selected tests and 24 contracts); lean gate passed 550 tests plus 570-model native/Yarn/Java TLC parity, with current-source receipt validation, private Python proxy, process boundaries and license checks all exit 0. No reinstall. Full report [[reports/2026-11-06-oracle-messages-in-turn]] is committed in native knowledge checkpoint d5c3fc6747210c237b12f76f821a063942c658e0. Handoff gap: artifact_claim was refused as artifact_not_committed despite the committed checkpoint and a fully caught-up provenance projection; no claim success is asserted.

```json
{
  "op_id": "01M6SEA463R12E1VG5NC6N0NGM",
  "commit": "d88df43b73162494c40fea87dc910edfb4a73677",
  "report": "[[reports/2026-11-06-oracle-messages-in-turn.md]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux -E 'test(/hint|unread|steer|deliver/)' && make test-formal-contracts",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Bounded in-turn surfacing and exact read/ACK suppression",
        "reference": "crates/bbt-mux/src/messaging/ledger/surfacing.rs:25; crates/bbt-mux/src/seat_socket.rs:1870; crates/bbt-mux/tests/mcp/oracle_hints.rs:39 and :112; /tmp/oracle-in-turn-final-verify.log — 41 selected tests passed"
      },
      {
        "criterion": "Native Oracle steering without cancellation",
        "reference": "crates/bbt-mux/src/logic.rs:9709; crates/bbt-mux/src/adapters/native.rs:30; crates/bbt-mux/tests/mcp/oracle_hints.rs:189 — private Codex RPC witness passed"
      },
      {
        "criterion": "Honest queued/hinted/steered/acknowledged receipts and additive wire compatibility",
        "reference": "crates/bbt-mux/src/messaging/ledger/surfacing.rs:45; crates/bbt-proto/src/messaging.rs:428; crates/bbt-proto/tests/message_evidence.rs — 29 proto tests passed in landing gate"
      },
      {
        "criterion": "Busy-turn, worker rate limit and disabled-hints broken control",
        "reference": "crates/bbt-mux/tests/mcp/oracle_hints.rs:39,:112,:161; formal OracleInTurn pass 5898 states/38756 edges, BrokenOracleInTurn expected safety failure 2388 states/15464 edges; source-bound evidence SHA256 194bfb9618113d148e78ea9dede16aa2b02e9640eb588ab7526367b29f2a35c3"
      },
      {
        "criterion": "Signed landing and unattended verification",
        "reference": "target/oracle-in-turn-land.log:1089 — LANDED d88df43b73162494c40fea87dc910edfb4a73677, make land exit 0; make test-python-execute-native, make test-boundaries, make test-licenses and make check-formal-evidence exit 0; git verify-commit HEAD exit 0; clean worktree"
      },
      {
        "criterion": "Committed report and provenance handoff limitation",
        "reference": "knowledge_commit operation 01M6SE7MQW0EQFZED9VFNJZEFN — publication_state committed, checkpoint d5c3fc6747210c237b12f76f821a063942c658e0; artifact 01M6SE7NT4XKMJ5R027HB0Y30B; artifact_claim operations 01M6SE7ZVJC3HKRENZDQYZQDKW and 01M6SE96DBHNFF9TMRPK0MXRN0 explicitly rejected artifact_not_committed; no further retries"
      }
    ]
  }
}
```
