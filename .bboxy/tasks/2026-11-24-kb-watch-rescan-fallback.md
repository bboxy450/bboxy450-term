---
allowed_paths:
- crates/bbt-mux
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8ZJ1pn6ymQlhdn20cJBq2Rono7S6d/fLcgc3FK45CmU=
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
created: 2026-11-24T17:09:33.121Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
id: 01M878HFP4EPKJQPF2PRQ2M99G
number: 1794292861627915
role: executor
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'KB index: bounded rescan fallback and freshness status when FSEvents lags'
type: task
updated: 2026-11-24T17:09:33.121Z
updated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
verify: cargo nextest run -p bboxy450-term-mux -E 'test(watch)' --retries 0
provenance:
  digest: sha256:ce47aeec105eb62c6adcd1bae427405ff74506341bab7dd9e3a58aef78d15b3d
  frontmatter_digest: sha256:c87e8ccf691f3de4c3d37d04423b787b557b6e41ad635959baf13e8597740298
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
  sig: H3m/rXW9umDUTLuQ0WT7t8StTs5NOBvqFW8NJDDZAmbOrwvBPnkco5/nUnmHVeqMi+3paUm/btdI1giio2YpDw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1849
  event:
    id: 01M878HGEEDZB187CATZ1EP70M
    sequence: 1849
    hash: 92d393a599a1f2bf14d1e26fbc5484785bfa06557549e18027663ff217bb0927
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-06-filewatch-fsevents-backlog.md`.

## Evidence
`index/watch.rs` has only a debounce (QUIET 200 ms, MAX_WAIT 1 s); no periodic/backlog rescan and no freshness status. The two flaky tests were deleted in `f2d8e4076`; `53a9e215f` added a test-side `rescan` helper only. `2026-11-22-kb-index-external-adds` (f0365a5b1) fixed git-added records, not this.

## Do
Add one bounded incremental rescan (about every 5 s, mtime/size diff, no full re-read) that catches events FSEvents dropped, and expose index freshness (last event, last rescan) in `knowledge_status`. Test with a watcher that drops events (inject at the watcher boundary) plus a broken control with the rescan disabled. Update the relevant model in `verification/` if the watch lifecycle is modelled. `make land`.
