---
id: "01M66NR73B287NMCJCN34MR5Q1"
type: "task"
number: 377
title: "Rust plugin SDK; GitKB data_source plugin and tooling in Rust; plugin architecture review"
status: "running"
role: "oracle"
owner: "Vesper Greywind"
verify: "make test"
created: "2026-10-30T15:09:46Z"
updated: "2026-10-30T15:09:46Z"
---
# Rust plugin SDK and GitKB plugin port

Operator direction, 2026-10-30: every plugin is Rust, built on a new `bboxy450-term-plugin-sdk` crate; all plugin tooling (build, install, probe, conformance check) is Rust; re-review the plugin architecture, close gaps and make the system extensible so future features can ship as plugins. Implementation explicitly assigned to the Oracle seat (Vesper Greywind, Claude). Worktree `~/mydev/bboxy450-term-plugin-sdk`, branch `plugin-sdk` from `origin/main` `c44be4c8`. Operator merges.

## Measured defects in the Python GitKB plugin (Harmony KB, 3691 documents)

- GitKB is fast: `git-kb list --json` over all documents 0.13 s, `git-kb show --raw-content --json` 0.05 s. The adapter already uses `--json` for every call.
- `Adapter.cli` → `verify_backend` SHA-256-hashes the 395 MB `git-kb` binary before every CLI call: 0.2 s warm, about 1 s cold. A page makes 2–4 calls.
- `position()` caps each page at `min(limit, 1)` and `listing` examines one record per call. `task_list` through the bboxy MCP ran about 30 s at 90% CPU, the seat transport timed out ("Seat control transport unavailable"), and the single root connection answered `provider_busy` to every other read meanwhile.
- `status:active`: the catalog selects native `active`, then `matches_filter` compares the portable status (`running`) with `active`, so every page is empty but carries a `next` cursor.
- `type:task status:running` returns one item per page.

## Scope

- `crates/bboxy450-term-plugin-sdk`: MCP stdio runtime on the `bboxy450-plugin` contract types (initialize, shape, tools/list from schema, resources and skills, cancellation, busy/duplicate guards, frame and result caps, typed refusals). A provider implements a trait.
- `bboxy450-term-data_source-gitkb` Rust binary on the SDK, same data_source 1.0 contract, with: backend hashed once and re-verified by file identity per call; time-budgeted pages filled up to `limit`; status filters matching portable or native status.
- Rust build/install/probe/conformance tooling; Make targets updated; Python plugin and scripts removed.
- Architecture review: gaps and dispositions recorded below; in-scope gaps closed.

## Acceptance

- [ ] SDK crate documented and tested, with a formal coverage entry.
- [ ] GitKB plugin in Rust on the SDK; no Python left in plugins or plugin tooling; `make build-gitkb-plugin` and `make check-gitkb-plugin` pass.
- [ ] Harmony read-only probe: list and search pages hold `limit` items within the 2 s budget.
- [ ] `status:active` and `status:running` searches return Harmony's active tasks.
- [ ] Architecture gaps recorded with dispositions.
- [ ] `make test` result recorded for the final commit.

## Progress

- 2026-10-30: through the bboxy MCP, fetch and text search work against GitKB; `task_list` times out and filtered search is broken (evidence above).
