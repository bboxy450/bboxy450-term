---
type: task
title: 'Fix logic hang: bound provenance recover per step, interruptible shutdown, bounded startup audit, process witness'
role: executor
verify: make test
id: 01M6HEHFAAYYGBKSZ3R5VJV04K
status: done
delegated_by: seat:01M6HEDKGMGNSJYV4KEYAH4C70
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T14:14:52.610Z
created: 2026-11-03T19:35:24.532Z
number: 16371
created_by: seat:01M6HEDKGMGNSJYV4KEYAH4C70
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: open
  disposition: implemented
  reason: 'The logic-worker recovery fix is delivered in published/installed main ca7cde96 through bounded recovery and subsequent JSONL/write-route hardening. The original open record is stale. Historical closure only: its source/process/formal checks are retained; no fresh all-merge verification is claimed.'
  evidence:
  - Signed80566df996974e33981bf35e97942a594aba6811 is an ancestor of approved origin/main ca7cde968dfee7b60937fd1a7086fe9bc5f96c15; later123eeb2f/19b21c5c retain bounded recovery and actual writer IO ownership.
  - 'ca7cde96:crates/bbt-mux/src/provenance/projection/recovery.rs: RECOVERY_PAGE=16, BUDGET=8ms, resumable durable cursor/publication; logic/hol_tests.rs and verification/contracts.toml connect request/shutdown witnesses and four LogicWorker contracts.'
  - '/tmp/hol-final-complete-land.log: ci-local result40f45c5838f521bd70e9b8ff907caca3f8784eb9 normal0,290 native/TLC models,34 crates,119 contracts; LANDED40f45c58. /tmp/hol-report-land.log: LANDED184b770f9f43094c518570f143e5eeaeb7e999ef, retained source-bound gate reuse. Both commits are ancestors of published ca7cde96.'
  by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  op_id: 01M6KEK8N7N03Y7RZC1H65E8HH
provenance:
  digest: sha256:3dea81461165fd435e8b3c65e8f42c383b16b1f844cb40680dfa39d52ca58c2a
  frontmatter_digest: sha256:8853e1a15bce65cc4adfd4784a2e8b4ca64f4fcad4ce47d8ded89cc8ae3c6ae6
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: EysdNlfxal4ttLMvFEFd/0nCeRCMx9/ngK0KoKov1zMQRGbMr23fGF/tH2vmhOlSeK0MIKwUdDjD8QewnR2dAw==
  log: .bboxy/log/2026-11-04.jsonl#114026
---
# Fix the 2026-11-03 logic hang

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-hol-fix`, branch `hol-fix`, from `origin/main`.

## Cause (Oracle reading of origin/main; confirm it yourself)

The single logic worker (thread `bbt-logic`, `Worker::run`) serves GUI and MCP requests and also runs `provenance::engine::Engine::step` under an 8 ms budget (`crates/bbt-mux/src/provenance/engine.rs:37`). `projection::step` (`crates/bbt-mux/src/provenance/projection.rs:246-266`) falls back to `recover()` (`:205-243`). `recover()` verifies the whole chain and then appends **every** event from genesis in one call (`while state.cursor < head`). On a 440,727-event (904 MB) log, that holds the worker for minutes: GUI hello misses its deadline, MCP returns "transport unavailable", and `Logic::shutdown` (`logic.rs:2282`) blocks in `join` until the child is killed. Evidence: the sample at `/private/tmp/claude-501/-Users-feral-mydev-bboxy450-term/47f8a74d-c00f-4cc9-a2c5-b87221bdf01b/scratchpad/logic-sample2.txt` (bbt-logic in `recover` for 3336/3412 samples) and `run/app.log` ("logic shutdown timed out; child was killed").

## Outcome

1. **Bounded recovery.** No single provenance step does history-proportional work on the request-serving worker. Either make recovery resumable (persist the rebuild cursor and temporary-directory state across steps, one bounded page or 8 ms per step, the atomic swap only at the end, crash-safe restart of an interrupted rebuild) or move it to a dedicated thread with cancellation, whose results come back through a message. Pick one, justify it in the report, and keep the existing crash-safety guarantees (`:203-204` comments). The same applies to `verify_chain()` inside recover.
2. **Interruptible shutdown.** `Logic::shutdown` completes within the existing refresh deadline even while recovery is mid-way: cancel or abandon the rebuild at a page boundary; the next start resumes or restarts it.
3. **Bounded startup audit.** `store/events.rs:444` `verify_history_after_ready` (since 49a16d44): bound its concurrency (e.g. one or two audits at a time, off the worker) so it cannot starve the worker or hold store locks against it. Confirm with the code whether it actually contends, and say so.
4. **Process witness** (`crates/bbt-mux/tests/logic.rs` or a new process test, headless, no operator): build a large synthetic provenance history (big enough that the old `recover` takes over 5 s on this machine; measure and report the time) and force recovery (delete or corrupt the projection state). While it is in flight, assert that GUI hello and one MCP call are each answered within budget (use the existing deadline constants), and that `shutdown` returns before its deadline. The test must **fail on origin/main**: run it once against the unfixed code and record the failure in the report.
5. Record the witness in `verification/contracts.toml` as the implementation witness for the logic-worker head-of-line model (a separate Executor is writing that model in `crates/bboxy450-term/tests/formal_contracts/`; leave its files alone and fill the witness slot by name). Update `provenance_models.rs` only if its existing projection model's assumptions change, together with its broken control.

Allowed paths: `crates/bbt-mux/**`, `verification/contracts.toml`, `verification/coverage.toml`, `crates/bboxy450-term/tests/formal_contracts/provenance_models.rs`, `.bboxy/reports/**`.

`make test` exit 0 on the final commit (main may still be red; rebase once main-green lands), then land with `scripts/land.sh`. Report with `verify.command` = `make test` verbatim. Full `make test`/`make test-formal` runs take the shared CPU lock described in your launch prompt.
