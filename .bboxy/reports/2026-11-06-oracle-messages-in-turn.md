---
type: report
title: Oracle messages surface inside active MCP turns
task: '[[tasks/2026-11-06-oracle-messages-reach-busy-executors]]'
author: seat:01M6S7423SHR6V4WNJP29GVSK0
created: 2026-11-06T22:03:46.430Z
commits:
- d88df43b73162494c40fea87dc910edfb4a73677
verify:
  command: cargo nextest run -p bboxy450-term-mux -E 'test(/hint|unread|steer|deliver/)' && make test-formal-contracts
  exit_code: 0
id: 01M6SE78ZVE86CCRFJJMP3CGH0
updated: 2026-11-06T22:03:46.430Z
number: 93377961366134
created_by: seat:01M6S7423SHR6V4WNJP29GVSK0
updated_by: seat:01M6S7423SHR6V4WNJP29GVSK0
authorship:
  signer: seat:01M6S7423SHR6V4WNJP29GVSK0
  kind: seat
  public_key: cjG1WJ+Il+idkgQBG9muqnwNE581NcSGIOzBSvJx9wA=
  process_generation: '1'
provenance:
  digest: sha256:6011cb124d864103fa96dc292926e7157d39bfd601fdd0132fac0f1bcb69fc28
  frontmatter_digest: sha256:3ac8547d612d8a75271c7cc7cb8625bc73f2ce790d6ded5203874a0a92789fb7
  signer: seat:01M6S7423SHR6V4WNJP29GVSK0
  sig: mYuP0TmKQrL5IQK1Ep2V+t0bGP9VzY7zagjb69KJ6uAWqf4vlyH4ybj8GnFjvfGsMBvrahC4tblFLlsvM+FmCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149896
  event:
    id: 01M6SE79ZMX1FH43AFGV4B6T67
    sequence: 149896
    hash: dcfd1c283cbbbd06ff7cf575f32c51d1841b95d533042853f926a4c2b5aac03d
---
Landed signed commit `d88df43b73162494c40fea87dc910edfb4a73677` on `origin/main` through `make land` (exit 0). No reinstall or deployment was performed.

## Acceptance evidence

1. **In-turn surfacing.** `crates/bbt-mux/src/seat_socket.rs:1870` projects fresh mailbox hints after the ordinary or guarded MCP result has been journaled. `src/logic.rs:11030` revalidates the caller/generation; `src/messaging/ledger/surfacing.rs:25` owns durable recipient-local unread state. It emits at most five entries, each serialized entry at most 512 bytes, with count, authenticated sender, message ID, first line and a usable `message_list` action. Oracle entries precede worker entries; unseen entries rotate through bounded batches. Exact IDs returned by successful reads and explicit ACK suppress hints. Superseded/withdrawn originals are excluded. `src/mcp/bridge.rs:441` also projects actual Python proxy results; internal context reads do not count as visible hints. Caller-supplied sidecar results cannot forge native read/receipt evidence.
2. **Native steering.** `crates/bbt-mux/src/logic.rs:9709` selects `Priority::Now` only for authenticated Oracle input targeting a known Codex turn, independently of scheduler credit. `src/adapters/native.rs:30` labels acknowledged steering `native-steered`, with model consumption explicitly unverified. The existing native `turn/steer` path is used without cancellation/interrupt. Other harnesses retain queued delivery plus the hint fallback. `tests/mcp/oracle_hints.rs:189` drives a private Codex RPC server; the fixture checks `turn/steer`, `expectedTurnId`, worker `queue/add`, and rejects cancellation/interrupt RPCs.
3. **Honest receipts.** `crates/bbt-mux/src/messaging/ledger/surfacing.rs:45` adds independent hinted/read timestamps and explicit ACK status to receipt projections; first hint publication appends a Hinted event without replacing native/ACK receipt evidence. `ledger.rs:1186` retires unread state on explicit ACK. `crates/bbt-proto/src/messaging.rs:428` appends new evidence variants, preserving prior names/order; unknown future variants degrade to `native-unknown`. `tests/message_evidence.rs` checks old spellings and forward fallback. Reading or steering never claims model acknowledgement.
4. **Executable witnesses and broken controls.** `crates/bbt-mux/tests/mcp/oracle_hints.rs:39` keeps a private harness turn open, sends Oracle DMs, observes the next MCP result, checks exact filtered-read/ACK suppression, honest timestamps and zero PTY writes. Tests at :112 and :161 cover seven-message rotation, UTF-8/escaping bounds, worker cooldown, restart persistence and the same observation assertion failing with projection disabled. Tests at :194/:219/:250/:296 cover addressed shared contexts, supersession/withdrawal, sidecar read forgery and Oracle metadata. `tests/mcp/python_execution.rs:254` exercises the actual private Host/CPython/MCP proxy, including an Oracle hint in `python_execute_orient`.

## Files and mechanisms

All `src/` paths above are within `crates/bbt-mux/` unless fully qualified.

- Messaging: `src/messaging/ledger.rs`, `ledger/surfacing.rs`, `service.rs` and `wake_cost.rs` install/index unread state, expose result projection, and retain the new transport evidence.
- MCP/dispatch: `src/logic.rs`, `seat_socket.rs`, `mcp/hints.rs`, `mcp/bridge.rs`, `mcp/bridge/python.rs`, `mcp/seats.rs`, `adapters/native.rs` and `adapters/mapping.rs` integrate fresh hints, preserve mirrors and wire bounds, restrict sidecar projection and distinguish native steering.
- Formal: `src/mcp/oracle_hint_model.rs`, `mcp/mod.rs`, `mcp/live_models.rs`, `src/models.rs`, four `models/{OracleInTurn,BrokenOracleInTurn}.{tla,cfg}` exports, plus `verification/{contracts,corpus,coverage}.toml` register the contract, broken control and implementation witnesses. Parent coverage also now lists the six Python wait/catalogue models already present in the current pinned harness; the strict inventory gate exposed that inherited omission.
- Tests: `tests/mcp/oracle_hints.rs`, `tests/mcp.rs`, `tests/mcp/python_execution.rs`, `tests/messaging.rs`, and `crates/bbt-proto/tests/message_evidence.rs` supply the busy-turn, native RPC, proxy, compatibility and replay witnesses. The burst fixture remains above both scheduler credit and batch bounds while fitting the separate eight-wake circuit.
- Oracle-authorized fixture repairs: `crates/bboxy450-term/tests/native_signing.rs:216` supplies only a fixed fake signing-identity stub and checks its fake codesign argument, without real Keychain lookup. `src/index.rs:1756` uses the same event-driven watcher assertion with a 30-second cap. `src/knowledge/routing_tests.rs:440` retries only transient `workspace_pending` with the same operation ID. The artifact-protection fixture in `tests/mcp.rs` waits boundedly for its explicitly seeded files to be indexed before retaining its original protection assertions. No tests were deleted or newly ignored.

## Formal result and limits

The current-source archive is `/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/194bfb9618113d148e78ea9dede16aa2b02e9640eb588ab7526367b29f2a35c3/evidence.json`; its SHA256 is `194bfb9618113d148e78ea9dede16aa2b02e9640eb588ab7526367b29f2a35c3`. It records source `d88df43b73162494c40fea87dc910edfb4a73677`, dirty=false, 570 models, 34 crates and 212 contracts, with native/Yarn and pinned Java TLC graph/verdict parity.

`OracleInTurn`: pass, 5,898 states / 38,756 edges. `BrokenOracleInTurn`: expected safety failure, 2,388 states / 15,464 edges. The finite model has two Oracle IDs, two worker arrivals, atomic delivery/read/ACK/MCP actions and capacity two abstracting the concrete five-entry bound. Progress requires another MCP call; it does not promise unsolicited model consumption. SQLite/OS durability, real clocks/process identity, wire byte bounds and arbitrary Rust behavior retain implementation coverage. Older binaries cannot acquire new enum semantics; updated readers tolerate future values.

## Verification

All commands below completed with exit 0 on the landed source unless explicitly noted.

- `make land`: strict all-target Clippy; 2 native-signing tests (1 existing ignored), 519 mux lib/MCP/messaging tests (25 existing ignored), 29 proto tests; full formal target and push. Log `target/oracle-in-turn-land.log`, SHA256 `65bcb7b618780c5375b46d38ea01eb70ab9c95f6e5f48e74a9a4a0a94b9bb5fc`.
- Prescribed `cargo nextest run -p bboxy450-term-mux -E 'test(/hint|unread|steer|deliver/)' && make test-formal-contracts`: 41 selected tests passed; 24 native contract tests passed plus strict contract lint. Log `/tmp/oracle-in-turn-final-verify.log`, SHA256 `df73ac89d8d0238b1b511d79cf03c5a6a91497724d5cc12124233cdbc5a3e0d0`.
- `make check-formal-evidence`: exact source/model/tool hashes and clean 570-model receipt validated. Log `/tmp/oracle-in-turn-formal-receipt.log`, SHA256 `d04725b5737c2a3120c09d1ad04c56e0c1f6319ba02aa90b7bc5a63dd3dcd634`.
- `make test-python-execute-native`: all 3 tests passed, including the existing explicitly selected ignored private Host/CPython proxy witness. Log `/tmp/oracle-in-turn-final-python.log`, SHA256 `0db8d69ae45550c1a8a7b9393ec03e36d91b4decfd9fdd6ff1863107326b9b52`.
- `make test-boundaries`: 39 process/CLI/lifecycle tests passed across its targeted suites (1 existing ignored). Log `/tmp/oracle-in-turn-final-boundaries.log`, SHA256 `7fc6cdc465f1de819003cb4cbf4a0cd3ef60c1699b47b0526a71345864db09c2`.
- `make test-licenses`: three suites, 25 tests passed. Log `/tmp/oracle-in-turn-final-licenses.log`, SHA256 `1d679653e371009bf428ec87734a49e40fd9faa4edcfe0322c430a58a71e0311`.
- Model export/control command during implementation: `BBOXY_UPDATE_MCP_MODELS=1 cargo test --locked -p bboxy450-term-mux --features model --lib mcp::live_models::tests::mcp_live_contract_models_reject_broken_controls -- --exact`, exit 0. Final exported bytes were subsequently checked by the landing formal gate.
- `git verify-commit HEAD`, `git fetch -q origin main` and `git merge-base --is-ancestor HEAD origin/main`: exit 0. HEAD and freshly fetched origin/main both equal the reported full SHA; worktree clean.


Earlier failed gate attempts exposed the fixture races, fake-identity mismatch, strict contract-check metadata and inherited harness coverage omission described above. Those were corrected; the final landing gate passed without bypasses. Full `make test`, GUI parity and an installed-app smoke/reinstall were not run: this brief calls for lean landing and no reinstall. Native RPC, real private bridge/proxy and process-boundary witnesses ran unattended.

## Handoff

`git verify-commit HEAD` exited 0 with approved Boxy Ed25519 key `9B07E849480C68328CA20684844ECC0587DE7AB0`. `origin/main` equals the commit above; `git status --porcelain=v1` is empty. The commit carries `Report: [[reports/2026-11-06-oracle-messages-in-turn]]`. This full report is committed through the native knowledge provider, whose separate private Git checkpoint preserves its signed provenance. Oracle acceptance and seat/worktree cleanup remain with the delegator.
