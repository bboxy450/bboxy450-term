---
type: report
title: 'RemoteDelivery conformance: stopped on direct runtime/model counterexamples'
task: '[[tasks/2026-11-09-remote-delivery-conformance-and-card]]'
author: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
created: 2026-11-10T01:31:01.546Z
commits: []
verify:
  command: cargo nextest run -p bboxy450-term-remote --test delivery_conformance_probe --no-capture
  exit_code: 100
id: 01M71H8XG9NNDDAQC7REHED0ME
updated: 2026-11-10T01:31:01.546Z
number: 1499550330375412
created_by: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
updated_by: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
authorship:
  signer: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
  kind: seat
  public_key: uACX3zuhcDlbiNfc2nk5cFskWaUe4+OKMqxc6XxLvtM=
  process_generation: '1'
provenance:
  digest: sha256:227b6c89cefd8ef4432bb8ffdec20ee1eb07917ad53afa9aed2d7d63776b19d8
  frontmatter_digest: sha256:8aae050936d29283c2b65701c4dce65a1e0a0d889f5eaba7c5fb813e82048ae7
  signer: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
  sig: hJDVae66L35hYPGNNFQoxYXzO1lsjRqAHdjud/gxQpDT/wOLHH0ovUPeXRyJO+bPPLXlm7MJsn+1V0ifyZo/Dg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151265
  event:
    id: 01M71H8YN078RFB9218K2X202K
    sequence: 151265
    hash: 651d209c14f7afad62465e26403548b4db55749ab1c6be39ec9af00e8b637e0a
---
The task's stop condition fired: “If the runtime disagrees with the model, stop and report the counterexample path; do not edit delivery.rs or link.rs to get green.” No acceptance or completion is claimed.

## Counterexamples

1. `Submit(A), Send(A), AckArrive(A)`: Outbox::on_applied("a") returns true without any Inbox arrival. The generated model returns Ignored { reason: NoAck }. Evidence: `crates/bboxy450-term-remote/src/link.rs:66` checks only link and in-flight membership; `crates/bboxy450-term-remote/src/delivery.rs:69` additionally requires applied membership. The diagnostic assertion reports runtime true versus model false.
2. `Submit(A), Submit(B), Send(A)`: Outbox::sendable() returns and marks both a and b in flight. The generated Send(A) transitions only A, leaving B sendable. Evidence: `link.rs:52` collects every unsent message and `:62` marks the whole batch; `delivery.rs:43` adds only m to inflight. The diagnostic assertion reports [(a, turn), (b, turn)] versus [(a, turn)].

Both probes compiled and ran; neither failure is an IO/build error. These expose a mapping/abstraction mismatch at the public APIs, not proof of an invalid real transport trace: the outbox trusts the remote's Applied assertion, and its send primitive is batched. Gating acknowledgements using model state or discarding B from sendable() would hide the distinction and would not establish the requested every-edge binding.

## Decision needed

Amend the brief with the explicit approved refinement mapping for batched Send and causal AckArrive (and how per-event state/decisions remain observable), or authorize runtime/API changes in a separate scoped assignment. Current scope expressly excludes changing delivery.rs/link.rs behaviour. Implementation stopped before the card work as required.

## Changes and removals

Added only the uncommitted diagnostic `crates/bboxy450-term-remote/tests/delivery_conformance_probe.rs`, containing the two direct calls above. It is retained for Oracle inspection/reproduction. SHA256: a7695d0c3adb78c9071814e02898ceb5dcc3c2fee8591153411ca08b8fa6c8aa.

Removed nothing. The original scripted witness and resume/model checks remain untouched because the stop line fired before replacement. No runtime, spec, metadata, dependencies or generated card was changed. No source commit or landing was attempted with a red check.

## Commands and actual results

- Initial repository/instruction inspection (`pwd`, `git status --short`, `git log -3 --oneline`, reading shared AGENTS/ATTRIBUTION/LAND and `make help`): exit 0; source worktree initially clean.
- Source/contract/coverage reads, identity checks and UTC date: exit 0. Approved git identity is Boxy <bboxy450@gmail.com>, signing key 9B07E849480C68328CA20684844ECC0587DE7AB0.
- Initial syntax search `rg -n 'enum Decision|Ignored\\(' crates/bbt-yarn-macros/src`: exit 1 (no match); subsequent commands in that shell's && chain did not run. Follow-up `rg -n 'Decision|Ignored' ... | head -30`: exit 0 and identified struct variant syntax.
- `. scripts/cache-env.sh && cargo nextest run -p bboxy450-term-remote --test delivery_conformance_probe --no-capture`, first attempt: exit 101, probe compilation error E0164 from tuple-style Ignored pattern. Corrected solely to Ignored { .. }. This run is not counterexample evidence. Log: `/tmp/bboxy-remote-conformance-probe-01M71H0S-build-error.log`.
- Same targeted command after syntax correction: exit 100; 2 tests executed, 0 passed, 2 assertion failures with the values above, 0 skipped. Log: `/tmp/bboxy-remote-conformance-probe-01M71H0S.log`, SHA256 c0d805fa9731b6dde0f4a44860d80b6e119328317d3aa65c748089d40f6fbd41.
- Final `git rev-parse HEAD`, `git status --short`, `shasum -a 256`, numbered source reads and UTC date: exit 0.

## Coverage and remaining work

No BFS counts were measured. Exhaustive walk, runtime broken control, card generation/staleness scratch mutation, Mermaid fixture, contracts/coverage updates, clippy, complete task verify command, license checks, formal checks and make land were not run/completed after the stop condition. Do not interpret the two short probes as exhaustive conformance.

Source HEAD: 7789ca02c1a5413f171df2bcaf26f5dfbd51f4f7. Worktree: one untracked diagnostic test, no tracked source modifications. This report is checkpointed through authenticated knowledge tools, separately from source Git; commits is empty because no source commit was made.

## What generalizes

The next component can reuse the existing protocol Decision, model check/report graph and verification metadata. The mapping must first state which public calls are atomic model events and which are environment actions. No new generic walker or card API was produced before this stop.