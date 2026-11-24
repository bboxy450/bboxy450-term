---
allowed_paths:
- crates/bbt-mux/src/knowledge/
- crates/bbt-mux/src/index.rs
- crates/bbt-mux/src/index/
- crates/bbt-mux/tests/
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
- verification/
- .bboxy/reports/
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-07T13:09:46.129Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6V226FT3BZWE6NVS0G920WT
number: 93377961366272
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - 'successor: tasks/2026-11-24-workspace-index-bounds.md'
  fresh_accepted: false
  kind: historical
  op_id: 01M878K8Z3N51JTRAFFAV74NC2
  previous_status: open
  reason: Carried into the live term swarm.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Bound logic memory: aggregate workspace indexes have no eviction bound; 1.2 GiB RSS unattributed'
type: task
updated: 2026-11-24T17:10:31.885Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: cargo nextest run -p bboxy450-term-mux -E 'test(/index|memory|evict/)' && make test-formal-contracts
provenance:
  digest: sha256:da768ebfaccc8cee824bdd22d9de7d3123e3656c92bac4479fc5f37cef8a87b2
  frontmatter_digest: sha256:b6f26ff717b7dfe8e19febc0fd47d4eb54bcc7c048bbde7c2d39c4e90be97149
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: 08eb2y1LJpVjPR9J3uYwi3cDBCyqPLhyYsdHOf2jZTaJDV8olEtDWDX4ok7ArtXYPpJgUi096dJ/dvfW2dKxAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1869
  event:
    id: 01M878K9VK24E3S8S6S3JZHHZN
    sequence: 1869
    hash: baaaafef221e7695069f6a2db9221ff4cad0d9fe32cf7b6b624b30afcf806c02
---
# Logic memory is not bounded

The GUI CPU task (landed 8d7c331a / 8ced58cb; report in the KB) produced a per-structure RSS inventory of the logic process. A fresh logic allocation plateau is stable, BUT **aggregate workspace indexes lack eviction bounds**, and the original **1.2 GiB** RSS seen on the live app remains unattributed.

## Required

1. **Attribute.** On a private instance with a realistic seat and workspace count, attribute RSS growth over time to structures (`vmmap`/`sample`, plus allocation counters), never on the operator's live app.
2. **Bound.** Give the workspace and knowledge indexes explicit capacity budgets with LRU or eviction, so memory is O(budget), not O(history). Respect the existing seats-400-memory-idle invariant.
3. **Test.** A long-running synthetic workload (many workspaces and documents) must keep RSS under the budget, plus a broken control: eviction disabled must exceed it.

Lean landing; write the report `reports/2026-11-07-index-eviction.md` and call `task_report` once.
