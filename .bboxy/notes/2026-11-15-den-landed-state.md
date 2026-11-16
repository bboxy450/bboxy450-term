---
type: note
title: 'Den and the improvement list: what is on main at 970570e24 (2026-11-15)'
id: 01M7HHBJWEK4SP15CK46RW3HTG
updated: 2026-11-16T06:40:20.770Z
created: 2026-11-16T06:40:20.770Z
number: 1794292861627510
created_by: seat:01M79HGAW321GYNK8468FYN4K9
updated_by: seat:01M79HGAW321GYNK8468FYN4K9
authorship:
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  kind: seat
  public_key: O/GLv3wEDMQc2X2CJwZN8QH5avRWsmx3x4SdZZC6lKU=
  process_generation: '1'
provenance:
  digest: sha256:433f0514bb75fbb155ca0e8e7c27053ab5abe5842eb19793d74fd33e07aa08c2
  frontmatter_digest: sha256:b98dcda3eb607c9e4412161fc8dd564d39a6502eabc2b09e6c4641d8b41cf5b3
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  sig: vcyWYB+DLRInzjby8b/RCgDMhBefh6NmWtVBG2AbVUR8psDJX+GKK45R8n0r4q3s5eMrcotlP6bAsh0jKu2fAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151685
  event:
    id: 01M7HHBNN5TEC53WEPBX4X8G4J
    sequence: 151685
    hash: c266729ae586c2a8ce03090a68500611bba96a7baeeda3c6cba3239e6f342e33
---
# Den and the improvement list: what is on main at 970570e24

Follows [[2026-11-14-den-feedback-from-first-outside-use]], [[d1794292861627507-rust-lean-tla-first-tools]] and [[gaps-and-improvements]]. Written in Markdown because the running app predates the `.den` write path; it converts once a build from this commit is installed.

## Adoption list, by row

| # | Change | State | Commit |
|---|---|---|---|
| 1 | Seats are told Den exists | done: `knowledge_orient` text and the type dictionary name Den, `den_check` and `specs/spec-language`; spec, epic and task default to `.den` | `da5bc314e` |
| 2 | Checker in reach | done: `den_check` in `bboxy_code` | `c1abe109d` |
| 3 | Knowledge tools accept `.den` | done: write (from frontmatter or from Den source, unsound refused with code and line), fetch, search, commit, task lifecycle, signatures | `da5bc314e` |
| 4 | Imports resolve for enums | done, one level deep | `2ae2a0ed2` |
| 5 | Names in traces and state | done | `96c051cbe` |
| 6 | Success summary line | done | on main |
| 7 | Diagnostics per file | done | on main |
| 8 | Reserved words listed, clear message | done: 15 reserved, X103 | on main |
| 9 | `open ID "text"` | done | `fd0be0e88` |
| 10 | `source` links | done | `fd0be0e88` |
| 11 | `define` | done | `0abc7f1d8` |
| 12 | `table` | done | `0abc7f1d8` |
| 13 | Mark a bound as a model bound | owed, needs a decision | |
| 14 | Multiplication and division | owed: `bboxy450-tlc` has neither | |
| 15 | Size bound on reachable states | owed, needs a decision | |
| 16 | One spec's effect bound to another's event | owed, needs a decision | |

## The rest of the list

| Item | State | Commit |
|---|---|---|
| Record kinds task, epic, report, decision, e2e, prose spec | done | `26bc2c6d1` (as landed) |
| `contracts` tool | done | `c1abe109d` |
| Formal loop as `python_execute` functions, `store`, lean result | done, harness `fcc9f2ef` | |
| Gate speed-ups | done; first full run: workspace suite 8.9 min (was 23.9), timed stage 17 s (was 18.9 min) | five commits on main |
| Chat: signal by default, needs-you block, task ledger | done | `615399f17`, `5f87eac85` |
| Move a seat between swarms | done, with model `SeatMove` and its broken control | `a5bd3f36f` |

## Known gaps

- A state-machine spec has no `managed` block, so the host cannot stamp it: existing `.den` specs index with a missing-number error and are read-only records.
- `ctl task create` still writes `.md`.
- Changing an author field through frontmatter lands as a managed override beside the stale author line.
- Live chat rows are unthreaded until a history page covers them; `MessageRow` needs `task_id` and `event`.
- A seat move is several steps, not one transaction.
- `mcp_task_reassignment_reauthorizes_history_and_refuses_foreign_or_ambiguous_scope` fails with `stale_reference` on main, with and without the `.den` change. Unexplained.
- Nothing here has been seen in a real window: the app has not been reinstalled.

## Found by full CI and fixed

- `bb8d2f7fb`: a harness seat fixture launched the test binary as the `bboxy_code` server.
- `970570e24`: the automation bolt moved the view switch 21 px, so a fixed-offset click hit the bolt.
