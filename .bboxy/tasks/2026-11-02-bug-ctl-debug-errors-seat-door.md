---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0OVKhorXXMxBqAXmj/BmazAFIsK0n2oFhw5vv5izOp8=
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
created: 2026-11-02T18:19:40.254Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQT2GQTMKWMV9AFZRE5GC1
number: 5002207
provenance:
  digest: sha256:d572a2cf1a3882ec0f78c00ed401acbbdf0ce4e7f1ad3fc581c59a58a4af80b4
  frontmatter_digest: sha256:416fa3d9ea3f6f33c700f0222c9fde81dfd5166c7d121f0ade08b2cdae9dd230
  log: .bboxy/log/2026-11-02.jsonl#2586
  sig: L2BSzJmM8bxDnXrSjehAtgROHBkXzopu1rC1U0pnwbY5UcKndO4Xcjp+rD/YtnLiEmwLwlouGBLDGYW+CgpOAw==
  signer: seat:01M6EW8HFQXY48AT9JNJA2P404
reconciliation:
- by: seat:01M6EW8HFQXY48AT9JNJA2P404
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/30853aea871fd17bb82b607463a0d125cffeafbb
  - 'make land (lean) LANDED b681d962: bboxy450-term --lib --bins --test diagnostics 432 passed; seat_debug_errors_is_refused_and_leaves_the_operator_journal_unread included'
  - 'broken control: guard disabled -> test fails with exit 0 != 1'
  fresh_accepted: false
  kind: historical
  op_id: 01M6EXAPBQMVA090HNAZGS7GF2
  previous_status: open
  reason: 'Landed: ctl debug errors refuses a seat (BBOXY_SEAT_ID) with ancestry_refused before the direct journal read, so a seat cannot acknowledge the operator''s journal.'
role: executor
status: done
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: ctl debug errors lets a seat read and acknowledge the operator''s error journal'
type: task
updated: 2026-11-02T19:56:07.617Z
updated_by: seat:01M6EW8HFQXY48AT9JNJA2P404
verify: cargo nextest run -p bboxy450-term --test ctl
---
**Observed (2026-11-02 10:15, seat 01M6EQFA2G00BFT3TVPY6AMXE6, installed app).** From inside a seat, `bboxy450-term ctl debug errors` returned `ok:true` with `acknowledged:true, unread_before:14, unread:0`. The same process then ran `ctl task list` and got `ancestry_refused` ("ctl is the operator's door; your tools are the MCP catalogue").

**Mechanism.** The ancestry refusal lives in the `gui.sock` door peer policy: `crates/bbt-mux/src/ctl_door.rs:57-72` (`classify_with`) refuses a peer whose ancestry reaches the supervisor. `ctl debug errors` (`crates/bboxy450-term/src/ctl/debug.rs:165`) sends `DebugRequest::Errors` on a route that this policy does not cover. The MCP surface says the opposite: `diagnostic_errors` "Never acknowledges the operator's journal", and `ctl/debug.rs:292` says a seat "uses only its admitted MCP tools".

**Impact.** Any seat can silently clear the operator's unread error count. The operator then sees no unread failures.

**Fix.** Put every `debug` verb that marks state, and preferably every verb, behind the same door classification. Alternatively, refuse the acknowledging (unfiltered) read when the peer descends from the supervisor. Add a `tests/ctl.rs` case next to the existing `ancestry_refused` assertion (`tests/ctl.rs:1154`) for `debug errors`.

Source: `ctl debug errors` triage, report [[reports/2026-11-02-error-journal-and-task-triage]].
