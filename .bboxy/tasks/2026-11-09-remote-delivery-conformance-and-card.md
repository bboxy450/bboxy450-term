---
type: task
title: 'RemoteDelivery: exhaustive runtime conformance walk and a generated level-1 card'
role: executor
allowed_paths:
- crates/bboxy450-term-remote/
- crates/bbt-yarn/src/
- crates/bbt-yarn/tests/
- crates/bbt-yarn/Cargo.toml
- crates/bbt-yarn/README.md
- crates/bbt-mermaid/tests/
- verification/contracts.toml
- verification/coverage.toml
- Cargo.lock
- .bboxy/reports/
verify: cargo nextest run -p bboxy450-term-remote -p bboxy450-yarn -p bboxy450-mermaid && make test-formal-contracts
id: 01M71GYNQAVZP9XCTR3G4T99PX
status: open
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-10T01:25:25.181Z
created: 2026-11-10T01:25:25.181Z
number: 205483610615695
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
provenance:
  digest: sha256:a38fd33650adaff4928025ff4fb5bd3ed46700729912af57cacab15dfd120d95
  frontmatter_digest: sha256:ee5bf9212e4e53ac971fb2cd3f30a4dd704382b65a74cff3e9415626995376a6
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: X5H3OpJBzb2cCc3IhavOrbaO5VOlJtsYCKdRtsfgWaxha1iiqeLlOiA1ucy/CBcdVTUsx/ido518PPWMJFpeCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151247
  event:
    id: 01M71GYP5TWKG9P5WAD0V8KV9H
    sequence: 151247
    hash: c70c88b19c5ec08c1c469852bafe9f7eda5f26cfc1fb284c10e21f54d39fb8db
---
ROLE: EXECUTOR

# Goal

First milestone of [[specs/spec-first-os]] (read it, and [[specs/philosophy]], first).
On one component, `RemoteDelivery`, prove two things:

1. **W1 binding.** The real runtime (`link.rs`: `Outbox`, `Inbox`, `FileJournal`) agrees
   with the checked model on *every* edge of the model's reachable graph, not on one script.
2. **Level-1 view.** A deterministic generator turns the checked model into a Markdown card
   with a collapsed Mermaid state diagram and a generated "Does not promise" section.

Worktree: the managed workspace this seat was launched in. Base `origin/main`.

# What exists (verified at 3da65fd56)

- `crates/bboxy450-term-remote/src/delivery.rs` — `protocol!` spec `RemoteDelivery`, domain
  `Msg { A, B }`, invariants REMOTE-DELIVERY-001..003, model control `BrokenRemoteDelivery`.
- `crates/bboxy450-term-remote/src/link.rs` — hand-written runtime of that spec.
- `crates/bboxy450-term-remote/tests/delivery_witness.rs` — one scripted trace driving both.
- `M::model()` returns the `bboxy450_tlc::Model` (`check`, edges, report);
  `M::verification_metadata()` returns `bbt_yarn::verify::ProtocolMetadata` (name, source,
  domains, claims, cases). `bbt_mermaid::parse` accepts state diagrams.
- `verification/contracts.toml` entry `RemoteDelivery` holds the recorded `assumptions`.

# Part 1: conformance walk

In `crates/bboxy450-term-remote/tests/`, replace the scripted
`runtime_and_model_agree_through_duplicate_drop_and_crash_replay` with an exhaustive walk:

- Enumerate the reachable states of `RemoteDelivery` by breadth-first search using the
  generated `M::step` over every event (all 11: four kinds x two messages, plus Drop,
  Reconnect, RemoteCrash). Keep the shortest event path to each state.
- For each reachable state and each event: build a fresh runtime, replay the path, apply
  the event, and assert the runtime agrees with the model on (a) the decision
  (accepted / rejected / ignored, as far as the runtime API can observe it), (b) whether
  `Apply` was emitted, (c) the projected state: per message `is_unacked`, `is_acked`,
  `is_applied`, and sendable membership.
- Write the mapping from model events to runtime calls as one explicit, commented function.
  Mapping notes: ids `"a"`, `"b"` map to `Msg::A`, `Msg::B`; `RemoteCrash` drops the `Inbox`
  and reopens `FileJournal` on the same path, and drops the link. Where the runtime cannot
  express a model event or observation, do not paper over it: list it in the report as an
  abstraction gap. If the runtime *disagrees* with the model, stop and report the
  counterexample path; do not edit `delivery.rs` or `link.rs` to get green.
- **Broken control on the runtime side:** the same walker, run against an `Inbox` whose
  journal forgets on crash (a memory journal recreated empty), must fail, and the test must
  assert it fails on an `Apply`/applied disagreement, not on any error. A walker generic
  over a small test-local runtime trait or closure is fine; add no production API for it.
- Assert the walk visited every reachable state the model checker reports and print the
  counts (states, edges) so the number is evidence.

Keep `exported_model_holds_and_volatile_dedup_control_fails` and the resume test.

# Part 2: card generator

Add one module to `crates/bbt-yarn` (no new crate), e.g. `bbt_yarn::card`:

- Input: `ProtocolMetadata`, the `Model` (or its checked report), a *view*: an ordered list
  of `(name, predicate over model state)` where the first matching name is the state's
  node, and free-text `intent` and `assumptions` strings supplied by the caller.
- Output: one deterministic Markdown `String`:
  - title and the intent paragraph;
  - **Invariants**: each claim id and predicate name;
  - **States**: a Mermaid `stateDiagram-v2` of the reachable graph quotiented by the view;
    edges labelled by event name, duplicates merged, self-loops omitted. More than twelve
    nodes, or a reachable state no predicate names, is an error, not a truncated diagram;
  - **Does not promise**, always present: each domain with its size ("checked for 2
    messages"), reachable state count, that no liveness or fairness is claimed when none is
    declared, every `reject`/`ignore` case label, any waivers, and the `assumptions` text;
  - a footer with a hash of `metadata.source` so a changed spec changes the card.
- No model writes the diagram by hand: nodes and edges come only from the explored graph.

In `bboxy450-term-remote`:

- A test builds the `RemoteDelivery` card with a per-message view (suggested names for
  message A: new, queued, in flight, applied, acknowledged; refine as the graph requires)
  and takes `assumptions` from the `RemoteDelivery` entry in `verification/contracts.toml`.
- The card is stored at `crates/bboxy450-term-remote/cards/RemoteDelivery.md`. The test
  fails when the stored card differs from the generated one, and rewrites it only when
  `BBOXY_UPDATE_CARDS=1` is set.
- The card's Mermaid block parses and lays out with `bbt_mermaid`; add it as a fixture in
  `crates/bbt-mermaid/tests/fixtures` the way that crate's tests require.

# Acceptance

- [ ] The walk covers every reachable state and every event from each; counts are printed.
- [ ] The runtime-side broken control fails for the stated reason only.
- [ ] The scripted single-trace test is removed (the walk subsumes it); say so in the report.
- [ ] `cards/RemoteDelivery.md` is generated, at most twelve nodes, with "Does not promise".
- [ ] Editing a guard in a scratch copy of the spec makes the card test fail (show the run).
- [ ] The Mermaid block has a `bbt-mermaid` fixture and passes that crate's tests.
- [ ] `verification/contracts.toml` / `coverage.toml` text for `RemoteDelivery` describes
      the walk instead of the single trace. No assumption is weakened.
- [ ] `cargo clippy -p bboxy450-term-remote -p bboxy450-yarn --all-targets -- -D warnings` exits 0.
- [ ] The verify command exits 0.

# Out of scope

Changing `protocol!` syntax or the macro crate; `link.rs`/`delivery.rs` behaviour; GUI or
document-viewer changes; level 0 and level 2 views; any other component.

# Report

Follow `AGENTS.md`: stay silent while working; one message if blocked, with the evidence
and the exact decision needed. When done: signed commit(s), land with `make land`, write
`reports/2026-11-09-remote-delivery-conformance-and-card.md`, then `task_report`. Include:
files and mechanisms, **what you removed**, every command with its real exit status, state
and edge counts, abstraction gaps between model and runtime, and a short section
"What generalizes": which parts the next component (`bbt-supervisor`) can reuse unchanged.
