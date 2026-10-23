---
id: "01M5N61RZNPP603PKMV36YQ90P"
type: note
title: "L9-M-a backend verification and handoff"
status: reported
created: "2026-10-23T20:19:37Z"
---
# L9-M-a

Source **59e5614f6c95ea18d1c64e917f442954f7fe55ce**, parent **16b4c98798ee2bfecff6373e7370c47d51146a7a**. Branch `nyx-l9-m-a`, worktree `/Users/feral/mydev/bboxy450-term-nyx`. Source signature verified with machine key `9B07E849480C68328CA20684844ECC0587DE7AB0`. This report and the logs are a records-only successor, not the source that was gated.

GUI messaging and task requests now use the existing W2.4/MCP rules under the real device Operator principal and existing account operation journal. Decision #38 and the `2fcd54b` local-issuer correction are implemented. TaskRequest follows the tools as built; spawn_spec remains off that wire.

## Mechanisms

- `crates/bbt-proto/src/messaging.rs:1`, `gui.rs:83` - shared W2.4 records plus explicit-swarm Messaging/Task wrappers and correlated replies; mux retains signing/validation through extension traits. Existing seat JSON shapes round-trip unchanged.
- `crates/bbt-mux/src/mcp/mod.rs:53`, `logic.rs:3299` - keep SeatActor fields/encoding, add Actor::Operator, resolve device from logic identity and reuse account `control/op/{id}` / `account/control` for replay. Shared MCP messaging and work-item parsers/planners perform validation.
- `crates/bbt-mux/src/identity/status.rs:123`, `identity/chain.rs:217`, `messaging/keys.rs:21` - actual device signer; local-dev grant digest/epoch/public_key comes from persisted `identity/local-issuer`, certified authority from current DeviceGrant. No IssuerState or synthetic seat is created.
- `crates/bbt-mux/src/messaging/ledger.rs:685`, `messaging/service.rs:1664` - persist canonical bus position separately from ledger cursor; named operator recipients and retained `operator/inbox/message` / `operator/inbox/receipt`; native seat scheduling stays separate.
- `crates/bbt-mux/src/logic.rs:1210`, `mcp/work_items.rs:195`, `provenance/author.rs:1` - same task verification gate for seat MCP and GUI; device DSSE author, honest local-dev/certified kind, operator delegator reference, recipient-scoped task notices.
- `crates/bbt-mux/src/logic.rs:284`, `messaging/service.rs:477` - read-only MessageRow projection and exact sorted/deduplicated retained subjects for Kestrel's lane; no consumer/lane implementation here.
- `crates/bboxy450-term/src/cockpit.rs:49`, `live.rs:455` - DTO queue/reply plumbing only. main.rs change is test match exhaustiveness. Complete source file list is in [l9-m-a-evidence.json](l9-m-a-evidence.json).

## Results

| Command/check | Exit | Evidence |
| --- | --- | --- |
| `cargo test -p bbt-proto --locked --test messaging` | 0 | [6 wire tests](l9-m-a-proto-tests-02.log) |
| Focused GUI operator DM/replay/task-verification test | 0 | [GUI test](l9-m-a-gui-tests-03.log) |
| `cargo test -p bbt-mux --locked --test messaging` | 0 | [17 tests](l9-m-a-messaging-tests-01.log) |
| Focused certified-device/operator rejection test | 0 | [Identity test](l9-m-a-operator-identity-tests-01.log) |
| `cargo test -p bbt-mux -p bbt-proto -p bboxy450-term --locked` | 0 | [288 passed, 0 failed, 16 ignored](l9-m-a-regression-tests-01.log); dev run before rebase, code later frozen as 516bb92 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | [Strict clippy on 59e5614](l9-m-a-clippy-01.log) |
| `sh scripts/ci-local.sh` in clean exact-source worktree | 0 | [Full CI](l9-m-a-ci-59e5614.log), source/environment/status/cleanup in [metadata](l9-m-a-ci-59e5614.json) |
| rustfmt check, staged diff check, source signature verification | 0 | Formatting/diff and signature checked before CI |

Full CI log SHA-256: `e23d1be34d2b48e0230d628d5e883ac785068cec261125bf324f04b33d164d18`. At log:1918 the GUI integration proves DM storage and retained receipt, reconnect/restart replay with one ledger row, explicit scope refusals, and shared task acceptance. At :1832/:1942 the signature tests refuse forged operator authority; :1945 proves persisted bus position; :781 shows the known duplex test passed. :2916 native smoke has idle frames=0 and callbacks=0. :2979 has 10 invariants, 0 failures, four skips; :2980 reports all gates passed.

The invariant dispatcher skips clippy-clean (strict clippy already ran explicitly), parity-geometry, seats-1000-bus and seats-400-memory-idle. This was the normal backend CI run granted as slot 2 in `70f913d9`; no visual/scale pass is claimed.

## Development failures, retained

The first two cargo checks exited 101 for mechanical imports/Route field access and the existing app test match; both were corrected. GUI attempt 1 exited 101 for the wrong local issuer record (the real first-launch record was absent at identity/issuer); attempt 2 exited 101 for the fixture's usize SQLite count. The final focused tests and exact-source CI pass. The four logs and causes are indexed in [the evidence manifest](l9-m-a-evidence.json).

## Scope and state

The source contains no UI constructor, rendering, lab or event-lane changes. Event/ACK/read-mark/model binding belongs to Kestrel, compose/task rows to L9-M-c. App reply logging explicitly leaves projection to that later UI step. Rook wire review was requested in `1cab0875`; no review response had been observed when this record was written.

Main advanced to `a817a9dfac285327115dfa29effa3ff9196ec773` during the gate, with .bboxy-only records including #38(g) lane migration. No source rebase or re-gate followed, per AGENTS.md. The clean candidate status remained empty. CI private target was removed immediately on exit (2,786,639,872 allocated bytes); completed dev cache was removed (2,971,344,896 allocated bytes). No peer directory or process was touched. The private CI worktree is removed after this evidence is saved.

Committed development logs remove empty lines at EOF only; the manifest retains original and stored hashes for those copies. The CI log is byte-exact. The records diff initially flagged those blank EOF lines (exit 2); normalization addresses that check without rerunning code.
