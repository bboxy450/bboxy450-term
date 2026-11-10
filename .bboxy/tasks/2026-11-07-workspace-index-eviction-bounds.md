---
type: task
title: 'Bound logic memory: aggregate workspace indexes have no eviction bound; 1.2 GiB RSS unattributed'
role: executor
allowed_paths:
- crates/bbt-mux/src/knowledge/
- crates/bbt-mux/src/index.rs
- crates/bbt-mux/src/index/
- crates/bbt-mux/tests/
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
- verification/
- .bboxy/reports/
verify: cargo nextest run -p bboxy450-term-mux -E 'test(/index|memory|evict/)' && make test-formal-contracts
status: open
id: 01M6V226FT3BZWE6NVS0G920WT
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-07T13:09:46.129Z
created: 2026-11-07T13:09:46.129Z
number: 93377961366272
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:da768ebfaccc8cee824bdd22d9de7d3123e3656c92bac4479fc5f37cef8a87b2
  frontmatter_digest: sha256:673bfcf3d7c232d6270e7c834adad0505b2c41294b54b2296278f6be7fbf2337
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: Y5fhRrLK26dX14Ce/IGUVE2lhDkrYwz2tcy3eOcuyKsdvU3H327tP/M8shX/cx/lhkxOrnPNI5pl12/tCGXMAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150350
  event:
    id: 01M6V227N65A1GJT08CFAFENF1
    sequence: 150350
    hash: 6bea489a3dd15e8bbdf29d063ea4b21555862797480c1022c2595725e101feba
---
# Logic memory is not bounded

The GUI CPU task (landed 8d7c331a / 8ced58cb; report in the KB) produced a per-structure RSS inventory of the logic process. A fresh logic allocation plateau is stable, BUT **aggregate workspace indexes lack eviction bounds**, and the original **1.2 GiB** RSS seen on the live app remains unattributed.

## Required

1. **Attribute.** On a private instance with a realistic seat and workspace count, attribute RSS growth over time to structures (`vmmap`/`sample`, plus allocation counters), never on the operator's live app.
2. **Bound.** Give the workspace and knowledge indexes explicit capacity budgets with LRU or eviction, so memory is O(budget), not O(history). Respect the existing seats-400-memory-idle invariant.
3. **Test.** A long-running synthetic workload (many workspaces and documents) must keep RSS under the budget, plus a broken control: eviction disabled must exceed it.

Lean landing; write the report `reports/2026-11-07-index-eviction.md` and call `task_report` once.
