---
type: task
title: 'RemoteDelivery: exhaustive runtime conformance walk and a generated level-1 card'
role: executor
allowed_paths:
- crates/bboxy450-term-remote/
- crates/bbt-yarn/
- crates/bbt-mermaid/tests/
- verification/contracts.toml
- verification/coverage.toml
- Cargo.lock
- .bboxy/reports/
verify: cargo nextest run -p bboxy450-term-remote -p bboxy450-yarn -p bboxy450-mermaid && make test-formal-contracts
status: claimed
assignee: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
id: 01M71GYNQAVZP9XCTR3G4T99PX
number: 205483610615695
created: 2026-11-10T01:25:25.181Z
updated: 2026-11-10T01:33:24.003Z
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
provenance:
  digest: sha256:fa44c14e02398f75147a90a3c6991e101ef3ea69ff31e5bc88b489db62025932
  frontmatter_digest: sha256:39ec99c4e04962f2d05677073c95254e98a7a5baa43d530be44e2274f7675ade
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: vSpW7Aprm81Xik2OnmlJscCPiPFSYvx7StF/73rAeU8sl+ZMb/jbbD09JNXGWG8Q+W6YhAMfyjLDmjF/dRhyAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151270
  event:
    id: 01M71HD9RV7EEX8XQDB5QSRHDS
    sequence: 151270
    hash: 40ba492b24e733a1ec9eba1c90e7edfe7f11136fc312095267c79085f293a6c1
---
ROLE: EXECUTOR

# Goal

First milestone of [[spec-first-os]] (read it, and [[philosophy]], first).
On one component, `RemoteDelivery`, prove two things:

1. **W1 binding.** The real runtime (`link.rs`: `Outbox`, `Inbox`, `FileJournal`) **refines**
   the checked model: every step the runtime can take is a step, or a short sequence of
   steps, the model allows, with the same observable result.
2. **Level-1 view.** A deterministic generator turns the checked model into a Markdown card
   with a collapsed Mermaid state diagram and a generated "Does not promise" section.

Worktree: the managed workspace this seat was launched in. Base `origin/main`.

# Amendment 1, 2026-11-09 (Oracle ruling on your two counterexamples)

Both counterexamples are real and both are the brief's error, not the runtime's. The first
brief walked every *model* edge and demanded the runtime take it. That is the wrong
direction: it asks the model to refine the runtime. Part 1 below is rewritten to walk the
*runtime's* operations and check each against the model. No change to `link.rs` or
`delivery.rs` is needed.

- **Batch send.** `Outbox::sendable()` puts every unacked, not-in-flight message on the
  link at once. It maps to the *sequence* `Send(m)` for each returned id, in returned
  order. The runtime can never send A alone while B is queued; that model edge is simply
  unused by this runtime. Unused model edges are counted and reported, never a failure.
- **Causal ack.** `Outbox` is one side of an open system. It trusts an `Applied` frame; the
  model's `AckArrive` guard (`applied.contains(m)`) is a fact about the *remote*. The walk
  therefore owns the link: an `Applied` frame for `m` exists only because the real
  `Inbox::arrive(m)` returned `Fresh` or `Duplicate` on the current connection. The walk
  never hands `Outbox::on_applied` a frame the `Inbox` did not produce. Record this as the
  environment assumption "the link does not forge `Applied` frames" in the report, in the
  `RemoteDelivery` assumptions text in `verification/contracts.toml`, and so on the card.

Keep your probe's two findings in the report under "What the first walk found".

# Latitude

You decide, without asking, and note it in the report:

- corrections to the mapping table below when the code says otherwise;
- test structure, helper names, file layout inside the allowed paths;
- the card's view predicates and wording;
- any edit inside `crates/bbt-yarn/` and the tests of the three crates that the work needs.

Stop and message only if, under a mapping you believe is correct, the real runtime
violates REMOTE-DELIVERY-001, -002 or -003, or the verify command cannot pass without
weakening a spec, a bound or an existing assumption. A mapping mismatch is not a stop:
fix the mapping, record the reason, continue.

# Do it in this order

1. Write the harness struct and the operation-to-model mapping (table below).
2. Write the walk with the real `FileJournal`. Get it green. Record the counts.
3. Add the memory-journal broken control. Confirm it fails for the stated reason.
4. Delete the scripted trace test and your probe.
5. Write `bbt_yarn::card`, with unit tests in `crates/bbt-yarn` on a tiny inline protocol.
6. Generate and store the `RemoteDelivery` card; add the stale-card test.
7. Add the Mermaid fixture. Update `contracts.toml` and `coverage.toml`.
8. Run clippy and the verify command. Commit signed. `make land`. Report.

# What exists (verified at 3da65fd56)

- `crates/bboxy450-term-remote/src/delivery.rs` — `protocol!` spec `RemoteDelivery`, domain
  `Msg { A, B }`, invariants REMOTE-DELIVERY-001..003, model control `BrokenRemoteDelivery`.
- `crates/bboxy450-term-remote/src/link.rs` — hand-written runtime of that spec.
- `crates/bboxy450-term-remote/tests/delivery_witness.rs` — one scripted trace driving both.
- `M::model()` returns the `bboxy450_tlc::Model` (`check`, edges, report);
  `M::verification_metadata()` returns `bbt_yarn::verify::ProtocolMetadata` (name, source,
  domains, claims, cases). `bbt_mermaid::parse` accepts state diagrams.
- `verification/contracts.toml` entry `RemoteDelivery` holds the recorded `assumptions`.

# Part 1: refinement walk

In `crates/bboxy450-term-remote/tests/`, replace the scripted
`runtime_and_model_agree_through_duplicate_drop_and_crash_replay` with an exhaustive walk.

**The system under test** is a test-local harness holding the real `Outbox`, the real
`Inbox` over a real `FileJournal`, and an explicit link: the set of message frames and the
set of `Applied` frames currently on the connection. The link may duplicate (a delivered
frame stays deliverable) and drop (both sets are cleared).

**Harness operations** (the walk's alphabet), each mapped to model events in one explicit,
commented function:

| Harness operation | Runtime calls | Model events |
|---|---|---|
| submit(m) | `Outbox::submit` | `Submit(m)` |
| flush | `Outbox::sendable`; returned ids become message frames | `Send(m)` per returned id, in order; none when empty |
| deliver(m), only if a message frame for m is on the link | `Inbox::arrive(m)`; an `Applied(m)` frame joins the link | `Arrive(m)` |
| deliver_applied(m), only if an `Applied(m)` frame is on the link | `Outbox::on_applied(m)` | `AckArrive(m)` |
| drop | `Outbox::drop_link`; clear both frame sets | `Drop` |
| reconnect | `Outbox::reconnect` | `Reconnect` |
| crash | drop the `Inbox`, reopen `FileJournal` on the same path, `Outbox::drop_link`, clear frames | `RemoteCrash` |

**The check.** Breadth-first over harness states from the initial state, for m in {a, b},
keeping the shortest operation path to each state (replay the path on a fresh harness and
a fresh temp journal to reach it). Two harness states are the same when their observation
is the same: per message `is_unacked`, `is_acked`, `is_applied`, frame-set membership for
both frame kinds, and link up/down. For every reachable harness state and every operation
enabled there, assert:

- each mapped model event is **accepted** by `M::step` when the runtime acted, and is
  rejected/ignored by the model exactly when the runtime refused or did nothing
  (`submit` → `AlreadySubmitted`; `on_applied` → `false`; `flush` → empty);
- `Arrival::Fresh` exactly when the model emits `Apply`;
- after the step, the runtime observation equals the model state under the mapping:
  `is_unacked`/`unacked`, `is_acked`/`acked`, `is_applied`/`applied`, message frames /
  `inflight`, link up / `link_up`;
- REMOTE-DELIVERY-001..003 hold in the mapped model state.

**Coverage, reported not gated.** Print: harness states, harness transitions, model states
reached out of the model's reachable total, model edges exercised out of total, and list
the unexercised model edges by kind (expected: solo `Send` while another is queued, and
any `AckArrive`/`Arrive` the link cannot produce). Pin the counts as fixed expected numbers
in the test, so a change on either side shows up as a diff.

**Broken control on the runtime side.** The same walk against an `Inbox` whose journal
forgets on crash (a test-local memory journal recreated empty) must fail, and the test
asserts it fails on a `Fresh`-versus-`Apply` or `applied` disagreement, not on any error.
A walker generic over the journal constructor is enough; add no production API.

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

- [ ] The walk covers every reachable harness state and every enabled operation from each;
      the coverage counts are printed and pinned.
- [ ] Unexercised model edges are listed by kind in the report.
- [ ] The runtime-side broken control fails for the stated reason only.
- [ ] The scripted single-trace test and the probe are removed; say so in the report.
- [ ] `cards/RemoteDelivery.md` is generated, at most twelve nodes, with "Does not promise"
      including the no-forged-`Applied` assumption.
- [ ] Editing a guard in a scratch copy of the spec makes the card test fail (show the run).
- [ ] The Mermaid block has a `bbt-mermaid` fixture and passes that crate's tests.
- [ ] `verification/contracts.toml` / `coverage.toml` text for `RemoteDelivery` describes
      the walk and the new assumption. No existing assumption is weakened or removed.
- [ ] `cargo clippy -p bboxy450-term-remote -p bboxy450-yarn --all-targets -- -D warnings` exits 0.
- [ ] The verify command exits 0.

# Out of scope

Changing `protocol!` syntax; `link.rs`/`delivery.rs` behaviour; GUI or document-viewer
changes; level 0 and level 2 views; any other component.

# Report

Follow `AGENTS.md`: stay silent while working. When done: signed commit(s), land with
`make land`, write a new report that supersedes your first one, then `task_report`.
Include: files and mechanisms, **what you removed**, every command with its real exit
status, the coverage counts, every mapping correction you made and why, abstraction gaps
between model and runtime, and a short section "What generalizes": which parts the next
component can reuse unchanged.
