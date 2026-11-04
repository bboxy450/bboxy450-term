---
id: 01M6H5DNY1CAT8YV2VXMJM0D84
number: 5002227
type: report
title: "Checkpointed event-log startup verification"
task: "[[tasks/2026-10-21-w2-2-logic-bus-index]]"
commits: []
verify: {command: "make test-formal", exit_code: 0}
author: "seat:01M6H3YD84FG1TQB9PTFA7RG22"
created: "2026-11-03T16:56:02.753294Z"
---
# Checkpointed event-log startup verification

Direct operator brief, 2026-11-03: fix startup verification in
`/Users/feral/mydev/bboxy450-term-eventlog-verify`, branch
`fix/eventlog-startup-verify`, base `2d73acbd`. The task link above is the existing
storage/bus component umbrella; this direct request governs the work, not that
historical task's assignment or status. No native task/seat identifier or native
task-report tool is available in this session. The author ULID is this standalone
Codex thread UUID encoded as a ULID; it does not claim native seat registration.
The signed containing commit and its Report trailer identify the implementation
cut without a self-referential commit hash.

## Files and acceptance evidence

- `crates/bbt-mux/src/store/events.rs:125` — adds only the new singleton
  `verification_checkpoint(seq, hash)` table to each event SQLite database. Its
  separate migration handles existing `user_version=1` databases whose ordinary
  schema fast path skips DDL. Existing table/index/trigger definitions are left
  intact. An unchanged current checkpoint takes no writer lock on open.
- `crates/bbt-mux/src/store/events.rs:217` — checks that the checkpoint row still
  exists, its stored hash matches, and its parsed payload hashes to that same
  checkpoint, with matching sequence and SQL/envelope previous hash. A cached SQL
  hash alone cannot conceal payload tampering. The verified hash binds the
  checkpoint signature without repeating its Ed25519 verification.
- `crates/bbt-mux/src/store/events.rs:257` — verifies only the suffix, retaining
  sequence continuity, SQL/envelope prev/hash linkage, maximum event size and
  unchanged `Envelope::verify`. Verification and checkpoint advancement share a
  SQLite snapshot transaction, and a failed open cannot advance its checkpoint.
  With no checkpoint, an indexed reverse seek selects an anchor before at most
  128 tail rows; the skipped prefix is explicitly unaudited, not represented as
  having passed the historical audit.
- `crates/bbt-mux/src/store/events.rs:364` — locally signed appends advance only a
  checkpoint matching the previous tip, in the append's existing immediate
  transaction. They cannot skip an unverified imported suffix. Normal appends
  therefore leave reopen work independent of retained history. An older writer
  that bypasses checkpoint maintenance leaves a suffix that open fully verifies.
- `crates/bboxy450-term/src/cockpit.rs:827` — writes the actual logic readiness
  line before scheduling `verify_history_after_ready`. At
  `crates/bbt-mux/src/store/events.rs:433` each retained stream gets an independent
  detached blocking audit from genesis on every logic-process start. No full
  history scan is joined by the readiness or bus path. Full `verify_chain` stays
  available and unchanged as an explicit API.
- `crates/bbt-mux/src/store/events.rs:419` — discovery, audit and worker-join
  failures go to existing `error_journal` source `event_log`, operation
  `verify_history`, and stderr. The explicit `events.sqlite` label survives path
  redaction; stderr retains the full path if the diagnostic datagram is absent
  or full. Failure in one stream does not stop its peers' audit jobs.
- `crates/bbt-mux/src/store/events.rs:514` — 512 rows, checkpoint 511, exactly one
  completed `Envelope::verify` on open; next open verifies zero. Checkpoint SQL
  hash and payload corruption both refuse open. Suffix payload/prev/gap/signature
  faults and missing checkpoint rows refuse open without advancing checkpoint.
  The invalid-signature case recomputes the row hash so signature verification
  itself must reject it.
- `crates/bbt-mux/src/store/events.rs:617` — an old-format database with the
  checkpoint table removed and marker still 1 verifies exactly 128 rows, retains
  all old schema SQL and append-only triggers, and still passes a full audit.
  At line 662 both checkpointed and old-format prefix tampering are found after
  open by the detached audit and observed over a real private diagnostic socket.
- `crates/bbt-mux/tests/storage_bus_index.rs:84` — updates the pre-checkpoint
  tampering witness to require historical-audit rejection rather than the removed
  full startup scan. Dedupe, conflict, signature, torn-database and bus fixtures
  still run.
- `crates/bboxy450-term/tests/formal_contracts/event_log_model.rs:16`, its
  registration in `formal_contracts/models.rs`, and `verification/contracts.toml`,
  `corpus.toml`, `coverage.toml` — a three-row finite model covers existing and
  absent checkpoints, checkpoint/suffix refusal, one atomic signed append and
  eventual prefix-failure reporting under fair audit/report scheduling. The
  broken control omits only the existing checkpoint-hash check and must fail
  safety. The contract records owners/atomic steps, bounds, trusted checkpoint
  metadata, fairness, and remaining implementation-only behavior.
- Root `ATTRIBUTION.md` and its 33 packaged mirrors — register these project-owned
  contributions and retain existing origin/license notices and absolute policy
  URLs. `make sync-licenses` performs the canonical synchronization.

## Completed commands

| Command | Actual exit and result | Local log |
| --- | --- | --- |
| `make help` | 0 | tool output |
| `make init-submodules` | 0; pinned harness `131a2e0cc2c08ee862b570760d3e5ce44821f8cb` | tool output |
| `cargo test -p bbt-mux store::events` | 101; package ID does not exist at this base | tool output |
| `cargo clippy -p bbt-mux --all-targets -- -D warnings` | 101; same package-name mismatch | tool output |
| `cargo test -p bboxy450-term-mux store::events` | 0; seven focused tests passed | `target/eventlog-unit.log` |
| `cargo test -p bboxy450-term-mux --lib store::sqlite::tests` | 0; three tests, including fifty current-schema opens with writer locks held | `target/eventlog-schema.log` |
| `cargo test -p bboxy450-term-mux --test storage_bus_index` | 0; fourteen passed, one subprocess fixture intentionally ignored | `target/eventlog-storage-bus.log` |
| `cargo clippy -p bboxy450-term-mux --all-targets -- -D warnings` | 0 | `target/eventlog-clippy.log` |
| `make sync-licenses` | 0; 34 workspace crates checked | tool output |

The first corrected-name focused run exited 101: six tests passed, and the new
historical diagnostic test found that the existing scrubber redacted the long
fixture path including its filename. Adding the explicit file label fixed that
actual observability failure. The successful rerun above uses the final source.
The original failure is retained in `target/eventlog-unit-initial-failure.log`.

| `make test-formal` | 0; 266 models, 34 crates, 109 contracts; native/TLC graph and verdict parity | `target/eventlog-formal.log` |
| `make test-licenses` | 0; six policy fixtures, nine source-notice controls and all 34 crates | `target/eventlog-licenses.log` |
| `make test-kb-reports` | 0; repository corpus and native report/task witnesses | `target/eventlog-report-check.log` |

Native and Java TLC agree on `EventLogCheckpoint`: pass, 40 states, 42 edges.
`BrokenEventLogCheckpoint`: safety failure, 47 states, 52 edges; its counterexample
opens with an existing checkpoint, a tampered checkpoint row, and then reaches
ready because the checkpoint hash check is absent. The native trace is
`target/formal/models/BrokenEventLogCheckpoint.native-trace`.
The exploratory formal receipt is explicitly dirty, on base `2d73acbd`, evidence
SHA256 `faf86a7686f4edfddc693972a9888f2ebe65a83e1483670e51283b006011e458`.
The final full gate must refresh this on the committed source.

Final precommit log SHA256 values:

- `target/eventlog-unit.log`: `dcbb120e2a4690e229527d62cfd4de10bc51d81d9c7b34f4c9cfa06ef59d5f54`.
- `target/eventlog-schema.log`: `8de906d5b82f6a8c184ef326d83ec7be78c358b8e0d7125f11c16fd89a5ffce6`.
- `target/eventlog-storage-bus.log`: `9fb9999a0b63bfb3cab465dc5ca80ce79ca9c1ce3590e13e69c2fa858ab39840`.
- `target/eventlog-clippy.log`: `70c2bf9076e99b7e9fd1dd114c61fcd03bff8360b6b7e5f28d0137cfeaa37015`.
- `target/eventlog-formal.log`: `24178cf9c7102fee2cfc6ecab19650e0299c2e2819179dadbcce2c8b7491f813`.
- `target/eventlog-licenses.log`: `437372ec31edbc4a12e4c010a2ecfb6a51141a38259857d7a5ef23c65e5524d7`.
- `target/eventlog-report-check.log`: `57c56c0f726706286d403c1ddf18eddaebde57baa7158260ed7dbff7b6dbb2a3`.

## Final committed-source gate and limits

The requested single final `make test` runs after this report and the source are
signed and committed, with the active `ci-local.sh` count checked first. This
pre-gate report does not claim that future gate passed. The final handoff and a
separate append-only verification report will record its actual source SHA, exit,
log hash, formal receipt and worktree state.

No operator event databases were read or modified and no live production-history
startup timing was measured. The count-based witnesses establish verification
work bounds; they do not claim a measured 440,727-row application startup time.
Historical audits require a continuing process and runtime scheduling; shutdown
can interrupt a pass, and the next start schedules a fresh genesis audit.
Diagnostics use the existing best-effort bounded datagram plus stderr fallback.
The same-database checkpoint is trusted local metadata; coordinated replacement
of checkpoint and history, cryptographic collision proofs, filesystem crashes,
concurrent malicious writes after the opening snapshot and arbitrary Rust
refinement remain outside the finite model. Unit/API tests cover actual bytes,
Ed25519, SQLite state, schema compatibility and diagnostic transport.

All changes stay in the assigned worktree. The preexisting untracked
`.bboxy-brief-eventlog-verify.md` stays untouched and unstaged. No push, merge,
installation, release, shared-checkout edit or operator input was performed.
