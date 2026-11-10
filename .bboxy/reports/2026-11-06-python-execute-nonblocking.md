---
type: report
title: Python execution bounded wait, live MCP and legible cells — landed
task: '[[01M6S038KHK6V17WQFSPA4K2Y9]]'
author: seat:01M6S0B7V7W5XHF0D7R2V618AE
created: 2026-11-06T18:34:59.770Z
commits:
- fe266d6a0d76cb76525da72c2a7c409835ab1901
verify:
  command: make test-python-execute && make test-formal
  exit_code: 0
id: 01M6S290088AE9J8WR3AYBYW8C
updated: 2026-11-06T18:34:59.770Z
number: 93377961366071
created_by: seat:01M6S0B7V7W5XHF0D7R2V618AE
updated_by: seat:01M6S0B7V7W5XHF0D7R2V618AE
authorship:
  signer: seat:01M6S0B7V7W5XHF0D7R2V618AE
  kind: seat
  public_key: jj7LCkVL2gQV4vgLN8X3wLQmWXXWpzBIqQFMMH4WsN8=
  process_generation: '1'
provenance:
  digest: sha256:379f81ceb566babe79665726999545acc35cccd722c11a9ad674984e836a6195
  frontmatter_digest: sha256:8e6c2fa634c7aca1a4d0dd2b0b96eadc882e60c5bcf260530d3916233baa6fa2
  signer: seat:01M6S0B7V7W5XHF0D7R2V618AE
  sig: ATfALXihlnEYNakEJRCWkRzcI7cRaYfRitAnNH7hRqayWYNK8gzgzPllXu7GVyINJg9PGbrj1/VEPh95IeXgDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149699
  event:
    id: 01M6S290XJ97YCGV6RH43GXNG9
    sequence: 149699
    hash: c91c0550828e4b4edbf1fe0c8ebf9d7e69caaf2f444c5fb1f469007ccc314cef
---
Landed harness implementation: [fe266d6a0d76cb76525da72c2a7c409835ab1901](https://github.com/bboxy450/bboxy450-harness/commit/fe266d6a0d76cb76525da72c2a7c409835ab1901) (signed Boxy commit). Task: [[2026-11-06-python-execute-nonblocking-and-legible]].

Changes:
- `src/python_execute/mod.rs:46,586,731` and `server.rs:79`: default 1000 ms result wait, 25000 ms request ceiling, independent durable execution deadline, completion notification and running cursor. Two seconds of the request budget are reserved for the existing publication-context fence.
- `src/python_execute/server.rs:37,130` and `worker.py:26,200`: optional progress-token notifications through a separate writer queue; uncaught ToolError preserves code, message, original operation id, server and tool through private control RPC.
- `src/mcp/python_broker.rs:24,298`, `src/mcp.rs:597`, `src/mcp/relay.rs:81`: live relisting, notification revision and exact-Ready publication fences, one refresh on an unsent tool miss, and bounded nested calls. Timeout effects remain unknown and are never replayed. Connection generation and current seat/grant guards remain.
- `src/python_execute/mod.rs:83,1005`: base64 only for invalid UTF-8 chunks, with retained bytes recoverable through cursors.
- `src/frontends/face_core.rs:276` and `src/frontends/gui/face_rows.rs:139`: linked Python command cards with source, execution id, status/exit, emitted preview and expandable output. Indexed pages preserve prior output; stale running polls cannot regress terminal state. Failed reads and child-byte pages remain visible.
- `tests/formal/python_execution.rs` and `verification/{contracts,corpus,coverage}.toml`: bounded wait and catalogue A-B-A contracts with deliberately broken cap, duplicate, stale revision and unknown-effect replay controls.
- `scripts/formal.py:102` and `scripts/test-formal.py`: resolve the real term witness in managed module worktrees; explicit missing witnesses still fail. Attribution updated.

Acceptance evidence (all checked):
| Criterion | Executable witness |
|---|---|
| One-call short script | `src/python_execute/tests.rs:948` — inline completed status, exit 0 and emitted [1]. |
| 60-second worker / wait cap / get completion | `src/python_execute/tests.rs:980` — fake time exercises the real wait path, cap and terminal wake. |
| Role/grant change and live tools | `src/python_execute/tests.rs:532,552,628` — new callable tools, same native PID/connection, and delayed B response discarded after A-B-A. |
| Nested timeout, unknown effect, no replay | `src/python_execute/tests.rs:693` — structured timeout, unknown receipt, original id and exactly one fixture dispatch. |
| Structured uncaught failure | `src/python_execute/tests.rs:966` — fixture_rejected code/message/id/tool/server in execution error. |
| Lean exact bytes | `src/python_execute/tests.rs:432,948` — UTF-8 has no base64; invalid bytes decode exactly; 201000 retained bytes paged. |
| Rendering and parent association | `src/frontends/face_core.rs:736`, `tests/gui_rich.rs:2323` — source/status/exit/preview, folding, output expansion, failed reads, child bytes and stale polls. |
| Formal cap control and lifecycle | `tests/formal/python_execution.rs:351,383`, `tests/formal_nonvacuity.rs` — good models pass and broken cap/duplicate/A-B-A/replay controls are rejected. |

Verification:
- `make test-python-execute && make test-formal`: exit 0; 23 Python process witnesses, 15 formal tests, 44 nonvacuity tests, one reactor projection test; 281 native models, one inventoried crate.
- `cargo test --offline --locked --jobs 2 --lib python_execution_cell_model`: exit 0, one passed.
- `cargo test --offline --locked --jobs 2 --features embedded --test gui_rich python_execution_cards`: exit 0, one passed.
- `python3 -I scripts/test-formal.py`: exit 0, 12 passed.
- Final `make land`: exit 0; 127 library tests passed (one existing release-only measurement skipped), 26 MCP/ACP integration tests passed, and all 281 native/Java TLC graphs and verdicts agreed. Includes signed commit verification, license controls, strict embedded/all-target/fixture Clippy, library/process integration checks and current-source native/Java TLC graph and verdict parity.
- Formal archive and landing command log: `/Users/feral/Library/Application Support/bboxy450-term/run/implementation-workspaces/checkouts/01M6S0B3D278MTNWKZR4S21JAW/verification/evidence/d12153d2385f04ade994c8927343b530d43ad6c1708a33cd391d36a1b73a2e6d/evidence.json` (SHA-256 d12153d2385f04ade994c8927343b530d43ad6c1708a33cd391d36a1b73a2e6d); command output `/Users/feral/Library/Application Support/bboxy450-term/run/implementation-workspaces/checkouts/01M6S0B3D278MTNWKZR4S21JAW/verification/evidence/d12153d2385f04ade994c8927343b530d43ad6c1708a33cd391d36a1b73a2e6d/land.log`. Landing receipt: `/Users/feral/Library/Application Support/bboxy450-term/run/implementation-workspaces/checkouts/01M6S0B3D278MTNWKZR4S21JAW/verification/evidence/d12153d2385f04ade994c8927343b530d43ad6c1708a33cd391d36a1b73a2e6d/harness-land-receipt.json`.

Limits and disposition:
- Formal models are finite abstractions with explicit scheduling assumptions; physical IO timing, optional progress delivery and arbitrary Python semantics remain implementation witnesses.
- No full `make test`, release, install or term submodule bump was run, per the brief. The canonical library gate skips only its existing release-only measurement.
- A broad rustfmt check finds nine pre-existing formatting differences in untouched regions of `tests/formal/models.rs`; baseline HEAD reproduces them. Changed implementation files and added models are formatted.
- Task-owned Cargo outputs and both temporary formal targets were removed; the 1.8 GiB repository-local formal archive, landing receipt and command output were retained. After cleanup, `python3 -I scripts/formal.py --target verification/evidence/d12153d2385f04ade994c8927343b530d43ad6c1708a33cd391d36a1b73a2e6d --verify-evidence` exited 0, binding 281 models, 78 contracts and one crate to the landed SHA. Final source worktree: clean.

The canonical Make abspath expansion for this existing workspace path with spaces used a redundant temporary formal path. The real commands completed and their source-bound archive verified; that task-owned temporary leaf was recorded and removed. No landing script or Makefile gate was bypassed.
