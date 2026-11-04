---
artifacts:
- '[[01M6DAECQVJ61TKYV9K1GVBKY6]]'
- '[[01M6DAEPVJXYET13NDARC5AE5R]]'
assignee: seat:01M6CQD7CZPMP5GPM612PTWXW3
authorship:
  kind: seat
  process_generation: '1'
  public_key: J+Eete7RzpCHzcNSDpuEpx+ye6zu+UTGX+qDDQkPrw4=
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
created: 2026-11-01T23:34:06.228Z
created_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
delegated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
id: 01M6CQD3JMVB7H4Z4AFDPM52XD
number: 2187
provenance:
  digest: sha256:8660bbd9fceb7e219980f8376721f53262804e7c82b4292d753615a8cb482b6d
  frontmatter_digest: sha256:dc1966290f90f5ae780b32f1bd82072744ffbc066bb616918191b3e6bf16c78e
  log: .bboxy/log/2026-11-02.jsonl#13999
  sig: ur31RY9AWAWJDKPJxrCZBsOnN9Lxg/sJBZfDXTymJ+t5aqrEW1WfIG+MMQfYgtklHbUmRnMsQkCyscFOcke5Dw==
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
rejection:
  by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
  reason: 'Bookkeeping only; the code is verified. The Oracle read the make_input_room eviction (oldest terminal receipt, never the pending write), the bbt-mux provider seat attribution, the >MAX_INPUT_IDS real-PTY test and `ci-local: result 794ee6ad… normal 0`. Acceptance compares the command string exactly, and you recorded `BBOXY_GATE_SLOT=none BBOXY_CACHE_MODE=off make test` against the task''s `make test`. Re-report the same result with command `make test`, exit_code 0, commit 794ee6ad, and your existing evidence, naming the env overrides inside the evidence text. Run no new tests.'
report:
  commit: 794ee6ad5f903607eddee11c26e9c4584d574dc1
  op_id: 01M6DANGCVT9E5JPMQ0YWXSHH4
  reported_by: seat:01M6CQD7CZPMP5GPM612PTWXW3
  summary: 'Implemented bounded FIFO retirement of terminal input receipts, preserved pending/replay/conflict behavior, and attached seat IDs to input diagnostics. Report: [[reports/2026-11-01-supervisor-input-dedup-cap]]. Signed 794ee6ad on input-dedup-cap; worktree clean. Exact-HEAD make test and make test-boundaries both exited 0.'
role: executor
status: accepted
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Supervisor refuses all seat input after 4096 inputs (dedup map never evicts)
type: task
updated: 2026-11-02T05:10:48.813Z
updated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
verify: make test
verify_result:
  command: make test
  evidence:
  - criterion: Bounded terminal-receipt eviction preserves in-flight input and retained replay/conflict semantics
    reference: crates/bbt-supervisor/src/supervisor.rs:2646 — FIFO retirement excludes Pending and the pending_write ID; :2598 retains replay/digest conflict checks; :2633 queues fresh admissions once. protocol.rs:22 and :475 document the 4096-entry window and evicted-ID reuse. supervisor.rs:3222 — terminal/pending/capacity regression passed. Committed report records GUI/logic/seat_input_text inspection.
  - criterion: Provider/request input refusal diagnostics identify the seat
    reference: crates/bbt-mux/src/provider/mod.rs:164 — captures target before consuming Input, GuardedInput, InputContext or InputStatus and sets Failure seat. crates/bbt-mux/tests/input_diagnostics.rs:33 — real private journal asserts 20 rows across five refusal codes and four operations; exact seat present and payload absent. provider_input_refusals_are_journalled_with_the_exact_seat passed in final full CI.
  - criterion: A real long-lived supervisor/PTY remains writable beyond MAX_INPUT_IDS
    reference: 'crates/bbt-supervisor/tests/supervisor.rs:67 — bounded_input_receipts_keep_a_long_lived_pty_writable passed in final CI: 4097 admissions, child COUNT output, retained replay, InputConflict, FIFO retirement, a duplicate-detecting barrier and evicted-ID reuse. Existing blocked_write_survives_fencing_and_becomes_unknown_without_replay also passed as an explicit ignored process witness; target/input-dedup-blocked-write.log.'
  - criterion: Formal bounded eviction and a deliberately broken no-eviction control
    reference: 'crates/bboxy450-term/tests/formal_contracts/input_dedup_model.rs:11; verification/contracts.toml:5; verification/corpus.toml:94; verification/coverage.toml:141. Native/TLC graphs match: InputDedupEviction passes with 22 states/110 edges; BrokenInputDedupEviction fails liveness with 10 states/46 edges (expected TLC exit 13). Clean source 794ee6ad formal receipt cf0a1097e4d9be07d6cf2ce80fae9bb959d687e7cf97d9a3059ff2bacda6e16b retains all 207 models, 85 contracts and 32 crates under the common .git/bboxy-formal-evidence directory. Finite bounds and fairness/implementation limits are documented.'
  - criterion: Required full CI, strict Clippy, process boundaries and unattended smoke
    reference: /Users/feral/mydev/bboxy450-term-input-dedup/target/ci-local.log — exact source 794ee6ad5f903607eddee11c26e9c4584d574dc1 normal result 0; SHA256 7f1edb31f3422971db8350b55ed17380858b69140ed4ab3151a22d50f4ede94b. Includes cargo clippy --workspace --all-targets --locked -- -D warnings, all workspace tests/doctests, capability controls, native harness, formal parity, UI lint, private headless smoke and 27 invariant checks with zero failures. Canonical normal mode skips 11 slow invariants; no added exclusions. Execution used BBOXY_GATE_SLOT=none and BBOXY_CACHE_MODE=off as environment overrides. This re-report uses the same existing result and evidence; no tests were rerun.
  - criterion: Boundary verification and signed, clean handoff
    reference: make test-boundaries — exit 0, 37 tests passed across selected targets; target/input-dedup-boundaries.log SHA256 b6ee73c955e759b96d200db658e95fdc9b4afa8aab4002dc64bc77879728852b. git status --porcelain empty; git diff HEAD^ HEAD --check exit 0. Git signature G, approved Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0. Report trailer resolves to committed .bboxy/reports/2026-11-01-supervisor-input-dedup-cap.md (SHA256 7a2f11e8094e6039dcfb30851bce8d44f19456be3aee6885293075904cd83175). target/input-dedup-handoff.json records hashes and source.
  - criterion: Earlier failure and scope rulings are recorded without waivers
    reference: Committed report records initial draft 9b4e85a4 full make exit 2 at the unchanged recovery disk guard (10,267,480,064 bytes below 10 GiB); raw log target/ci-local-9b4e85a4-disk-refusal.log. Final exact-HEAD run passed recovery and the full gate. It started with three other ci-local runs, under revised ruling 01M6D638DY6G9VT33BZK5JR239. Formal-test path and canonical task-copy rulings are cited in the report. No merge, push, installation or live-seat recovery performed.
  exit_code: 0
---
# Supervisor refuses all seat input after 4096 inputs

ROLE: EXECUTOR. Work only in a new worktree `~/mydev/bboxy450-term-input-dedup` on branch `input-dedup-cap`, created from `origin/main` (head `e624d888` at time of writing). Do not edit the shared checkout `~/mydev/bboxy450-term`.

## Bug (observed 2026-11-01, live account)

The operator cannot type into Claude seat `01M6AC3D2ECK4W3F8495PAQTS3` (Vesper Greywind, swarm `01M6ABW0CX6MY0D5WGCSQ3ADTG`, generation 1, a long-running Oracle). Each attempt raises the unread error count.

Evidence:
- `~/Library/Application Support/bboxy450-term/run/diagnostics.jsonl` has 146 rows `"Limit: input deduplication capacity reached; ids are never silently evicted"`, `source: provider`, `operation: request`, `seat: null`, pid 45599 (`bboxy450-term logic`), from at_ms 1793571185153 to 1793575901983. That is one row per refused keystroke/paste.
- `crates/bbt-supervisor/src/supervisor.rs:35` sets `MAX_INPUT_IDS: usize = 4096`.
- `supervisor.rs:2567` makes `accept_input` return `FailureCode::Limit` once `self.inputs.len() >= MAX_INPUT_IDS`.
- `supervisor.rs:2589` inserts every accepted input id into `inputs: HashMap<Ulid, StoredInput>` (`:2325`). Nothing ever removes one. Only `get`, `get_mut` and `contains_key` exist (`:1947`, `:2551`, `:2635`, `:2639`).

Mechanism: every keystroke, paste and agent `seat_input_text` uses a new input id. After 4096 of them in one seat generation, the seat refuses all input for the rest of its life. Long-lived agent seats are exactly the ones that reach the cap.

Second defect: the refusal is journalled with `seat: null`, so the error does not name its seat.

## Required outcome

1. Bounded dedup with eviction. Inputs that have reached a terminal state (`Written`, `PartialUnknown`, or whatever the final states are; no write pending) are evicted oldest-first, so the map stays at or under its bound. A replay of a retained id still returns the stored receipt, and a reused id with different bytes still gets `InputConflict`. The in-flight input (`pending_write`) is never evicted. `Limit` may fire only when the bound is full of non-terminal entries, which cannot happen with one pending write. Keep the memory bound. A `VecDeque<Ulid>` insertion order beside the map is enough. Update the "never silently evicted" wording and the receipt/retry documentation it implies: a retry with an evicted id is accepted as new input. Say this in the protocol doc or comment. Check the client side (the GUI/logic input path and `seat_input_text`) for any assumption that ids live forever. Clients retry immediately on transport loss, so a retry window of 4096 inputs is enough.
2. The Limit and other input refusals reach the diagnostics journal with the seat id filled in. Find where `source: provider`, `operation: request` rows are written and pass the seat through.
3. Regression test: a real supervisor/process test that sends more than `MAX_INPUT_IDS` inputs to one seat and asserts that input N+1 is still written to the PTY. Also assert dedup replay of the most recent id and `InputConflict` on reuse with different bytes.
4. Formal verification (d365): if a model covers supervisor input receipts and dedup, update it. If none does, add the bounded-eviction property to `verification/contracts.toml` or a small Yarn/TLA+ model with a broken control: removing eviction must fail by reaching a state where input is refused forever. Keep `verification/coverage.toml` consistent.

## Allowed paths

`crates/bbt-supervisor/**`, the crate(s) that write provider/request diagnostics rows (input path only), `verification/**`, `.bboxy/reports/**`.

## Verification

`make test` on the exact committed source, exit 0. Also `make test-boundaries` (process boundary). Strict clippy `-D warnings --all-targets` on the touched crates.

## Done

Signed commits on `input-dedup-cap`, a report in `.bboxy/reports/` with a `Report:` trailer, and `task_report` with file:line mechanisms and actual exits. Do not merge or push to main. The Oracle verifies and decides about landing.

## Note

The live stuck seat will not recover until its supervisor runs the fixed code, or until `seat restart` gives it a new generation. That is the operator's call and is not part of this task.


## Report from seat:01M6CQD7CZPMP5GPM612PTWXW3

Implemented bounded FIFO retirement of terminal input receipts, preserved pending/replay/conflict behavior, and attached seat IDs to input diagnostics. Report: [[reports/2026-11-01-supervisor-input-dedup-cap]]. Signed 794ee6ad on input-dedup-cap; worktree clean. Exact-HEAD make test and make test-boundaries both exited 0.

```json
{
  "commit": "794ee6ad5f903607eddee11c26e9c4584d574dc1",
  "op_id": "01M6DAGMHED57WZMX9SMA04NSD",
  "report": null,
  "verify_result": {
    "command": "BBOXY_GATE_SLOT=none BBOXY_CACHE_MODE=off make test",
    "evidence": [
      {
        "criterion": "Bounded terminal-receipt eviction preserves in-flight input and retained replay/conflict semantics",
        "reference": "crates/bbt-supervisor/src/supervisor.rs:2646 — FIFO retirement excludes Pending and the pending_write ID; :2598 retains replay/digest conflict checks; :2633 queues fresh admissions once. protocol.rs:22 and :475 document the 4096-entry window and evicted-ID reuse. supervisor.rs:3222 — terminal/pending/capacity regression passed. Committed report records GUI/logic/seat_input_text inspection."
      },
      {
        "criterion": "Provider/request input refusal diagnostics identify the seat",
        "reference": "crates/bbt-mux/src/provider/mod.rs:164 — captures target before consuming Input, GuardedInput, InputContext or InputStatus and sets Failure seat. crates/bbt-mux/tests/input_diagnostics.rs:33 — real private journal asserts 20 rows across five refusal codes and four operations; exact seat present and payload absent. provider_input_refusals_are_journalled_with_the_exact_seat passed in final full CI."
      },
      {
        "criterion": "A real long-lived supervisor/PTY remains writable beyond MAX_INPUT_IDS",
        "reference": "crates/bbt-supervisor/tests/supervisor.rs:67 — bounded_input_receipts_keep_a_long_lived_pty_writable passed in final CI: 4097 admissions, child COUNT output, retained replay, InputConflict, FIFO retirement, a duplicate-detecting barrier and evicted-ID reuse. Existing blocked_write_survives_fencing_and_becomes_unknown_without_replay also passed as an explicit ignored process witness; target/input-dedup-blocked-write.log."
      },
      {
        "criterion": "Formal bounded eviction and a deliberately broken no-eviction control",
        "reference": "crates/bboxy450-term/tests/formal_contracts/input_dedup_model.rs:11; verification/contracts.toml:5; verification/corpus.toml:94; verification/coverage.toml:141. Native/TLC graphs match: InputDedupEviction passes with 22 states/110 edges; BrokenInputDedupEviction fails liveness with 10 states/46 edges (expected TLC exit 13). Clean source 794ee6ad formal receipt cf0a1097e4d9be07d6cf2ce80fae9bb959d687e7cf97d9a3059ff2bacda6e16b retains all 207 models, 85 contracts and 32 crates under the common .git/bboxy-formal-evidence directory. Finite bounds and fairness/implementation limits are documented."
      },
      {
        "criterion": "Required full CI, strict Clippy, process boundaries and unattended smoke",
        "reference": "/Users/feral/mydev/bboxy450-term-input-dedup/target/ci-local.log — exact source 794ee6ad5f903607eddee11c26e9c4584d574dc1 normal result 0; SHA256 7f1edb31f3422971db8350b55ed17380858b69140ed4ab3151a22d50f4ede94b. Includes cargo clippy --workspace --all-targets --locked -- -D warnings, all workspace tests/doctests, capability controls, native harness, formal parity, UI lint, private headless smoke and 27 invariant checks with zero failures. Canonical normal mode skips 11 slow invariants; no added exclusions."
      },
      {
        "criterion": "Boundary verification and signed, clean handoff",
        "reference": "make test-boundaries — exit 0, 37 tests passed across selected targets; target/input-dedup-boundaries.log SHA256 b6ee73c955e759b96d200db658e95fdc9b4afa8aab4002dc64bc77879728852b. git status --porcelain empty; git diff HEAD^ HEAD --check exit 0. Git signature G, approved Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0. Report trailer resolves to committed .bboxy/reports/2026-11-01-supervisor-input-dedup-cap.md (SHA256 7a2f11e8094e6039dcfb30851bce8d44f19456be3aee6885293075904cd83175). target/input-dedup-handoff.json records hashes and source."
      },
      {
        "criterion": "Earlier failure and scope rulings are recorded without waivers",
        "reference": "Committed report records initial draft 9b4e85a4 full make exit 2 at the unchanged recovery disk guard (10,267,480,064 bytes below 10 GiB); raw log target/ci-local-9b4e85a4-disk-refusal.log. Final exact-HEAD run passed recovery and the full gate. It started with three other ci-local runs, under revised ruling 01M6D638DY6G9VT33BZK5JR239. Formal-test path and canonical task-copy rulings are cited in the report. No merge, push, installation or live-seat recovery performed."
      }
    ],
    "exit_code": 0
  }
}
```


## Report from seat:01M6CQD7CZPMP5GPM612PTWXW3

Implemented bounded FIFO retirement of terminal input receipts, preserved pending/replay/conflict behavior, and attached seat IDs to input diagnostics. Report: [[reports/2026-11-01-supervisor-input-dedup-cap]]. Signed 794ee6ad on input-dedup-cap; worktree clean. Exact-HEAD make test and make test-boundaries both exited 0.

```json
{
  "commit": "794ee6ad5f903607eddee11c26e9c4584d574dc1",
  "op_id": "01M6DANGCVT9E5JPMQ0YWXSHH4",
  "report": null,
  "verify_result": {
    "command": "make test",
    "evidence": [
      {
        "criterion": "Bounded terminal-receipt eviction preserves in-flight input and retained replay/conflict semantics",
        "reference": "crates/bbt-supervisor/src/supervisor.rs:2646 — FIFO retirement excludes Pending and the pending_write ID; :2598 retains replay/digest conflict checks; :2633 queues fresh admissions once. protocol.rs:22 and :475 document the 4096-entry window and evicted-ID reuse. supervisor.rs:3222 — terminal/pending/capacity regression passed. Committed report records GUI/logic/seat_input_text inspection."
      },
      {
        "criterion": "Provider/request input refusal diagnostics identify the seat",
        "reference": "crates/bbt-mux/src/provider/mod.rs:164 — captures target before consuming Input, GuardedInput, InputContext or InputStatus and sets Failure seat. crates/bbt-mux/tests/input_diagnostics.rs:33 — real private journal asserts 20 rows across five refusal codes and four operations; exact seat present and payload absent. provider_input_refusals_are_journalled_with_the_exact_seat passed in final full CI."
      },
      {
        "criterion": "A real long-lived supervisor/PTY remains writable beyond MAX_INPUT_IDS",
        "reference": "crates/bbt-supervisor/tests/supervisor.rs:67 — bounded_input_receipts_keep_a_long_lived_pty_writable passed in final CI: 4097 admissions, child COUNT output, retained replay, InputConflict, FIFO retirement, a duplicate-detecting barrier and evicted-ID reuse. Existing blocked_write_survives_fencing_and_becomes_unknown_without_replay also passed as an explicit ignored process witness; target/input-dedup-blocked-write.log."
      },
      {
        "criterion": "Formal bounded eviction and a deliberately broken no-eviction control",
        "reference": "crates/bboxy450-term/tests/formal_contracts/input_dedup_model.rs:11; verification/contracts.toml:5; verification/corpus.toml:94; verification/coverage.toml:141. Native/TLC graphs match: InputDedupEviction passes with 22 states/110 edges; BrokenInputDedupEviction fails liveness with 10 states/46 edges (expected TLC exit 13). Clean source 794ee6ad formal receipt cf0a1097e4d9be07d6cf2ce80fae9bb959d687e7cf97d9a3059ff2bacda6e16b retains all 207 models, 85 contracts and 32 crates under the common .git/bboxy-formal-evidence directory. Finite bounds and fairness/implementation limits are documented."
      },
      {
        "criterion": "Required full CI, strict Clippy, process boundaries and unattended smoke",
        "reference": "/Users/feral/mydev/bboxy450-term-input-dedup/target/ci-local.log — exact source 794ee6ad5f903607eddee11c26e9c4584d574dc1 normal result 0; SHA256 7f1edb31f3422971db8350b55ed17380858b69140ed4ab3151a22d50f4ede94b. Includes cargo clippy --workspace --all-targets --locked -- -D warnings, all workspace tests/doctests, capability controls, native harness, formal parity, UI lint, private headless smoke and 27 invariant checks with zero failures. Canonical normal mode skips 11 slow invariants; no added exclusions. Execution used BBOXY_GATE_SLOT=none and BBOXY_CACHE_MODE=off as environment overrides. This re-report uses the same existing result and evidence; no tests were rerun."
      },
      {
        "criterion": "Boundary verification and signed, clean handoff",
        "reference": "make test-boundaries — exit 0, 37 tests passed across selected targets; target/input-dedup-boundaries.log SHA256 b6ee73c955e759b96d200db658e95fdc9b4afa8aab4002dc64bc77879728852b. git status --porcelain empty; git diff HEAD^ HEAD --check exit 0. Git signature G, approved Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0. Report trailer resolves to committed .bboxy/reports/2026-11-01-supervisor-input-dedup-cap.md (SHA256 7a2f11e8094e6039dcfb30851bce8d44f19456be3aee6885293075904cd83175). target/input-dedup-handoff.json records hashes and source."
      },
      {
        "criterion": "Earlier failure and scope rulings are recorded without waivers",
        "reference": "Committed report records initial draft 9b4e85a4 full make exit 2 at the unchanged recovery disk guard (10,267,480,064 bytes below 10 GiB); raw log target/ci-local-9b4e85a4-disk-refusal.log. Final exact-HEAD run passed recovery and the full gate. It started with three other ci-local runs, under revised ruling 01M6D638DY6G9VT33BZK5JR239. Formal-test path and canonical task-copy rulings are cited in the report. No merge, push, installation or live-seat recovery performed."
      }
    ],
    "exit_code": 0
  }
}
```
