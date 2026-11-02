---
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
  digest: sha256:3b96e9a8e079cb3bf3d4545fc62b90a2d2d757450c0865bbfc97a4e2d55e7950
  frontmatter_digest: sha256:15fe24cf8c7915ecca4f26f2a7c4cdef44584009ce642b56fa3a012f4ab81f43
  log: .bboxy/log/2026-11-01.jsonl#9707
  sig: mh4wdyqVDwWrzKagzDvZGWhHzAvRJOQJqcf4HJ/FEMmPEY6f5E3O8JdPp3kTkPzzsO75bhd11ML7ziKSwtrCBA==
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
role: executor
status: claimed
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Supervisor refuses all seat input after 4096 inputs (dedup map never evicts)
type: task
updated: 2026-11-01T23:34:13.300Z
updated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
verify: make test
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
