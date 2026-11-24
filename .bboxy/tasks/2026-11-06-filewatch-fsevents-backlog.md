---
allowed_paths:
- crates/bbt-mux/src/index.rs
- crates/bbt-mux/src/index/
- crates/bbt-mux/src/knowledge/
- crates/bbt-mux/tests/
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
- verification/
- .bboxy/quarantine.md
- .bboxy/reports/
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-07T00:56:13.385Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6SR31WT64E5F3TBSHT5YGMT
number: 93377961366183
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - 'successor: tasks/2026-11-24-kb-watch-rescan-fallback.md'
  fresh_accepted: false
  kind: historical
  op_id: 01M878K5RC1TW10Y5HQ5FQ8QTQ
  previous_status: open
  reason: Carried into the live term swarm.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: FileWatch misses real edits for >30s under FSEvents backlog (land-time churn); add a bounded rescan fallback
type: task
updated: 2026-11-24T17:10:28.689Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: cargo nextest run -p bboxy450-term-mux -E 'test(/watch|file_watch|routing/)' && make test-formal-contracts
provenance:
  digest: sha256:5bb4db733571def9810a1d4aa0114d7d322dd53eb4ff23a617b06d308dbfb520
  frontmatter_digest: sha256:970700277b4857d817783dc165696fb018ad122990641e8cb44ca68980d36662
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: JDKa9LnT/sJk9bqpTF+Jmjn7/Ijdtz22bu42VKlHmq5fEsLQlLA6QfxJ18s6yTkvLV/2A57hd022Z5OiyXJpDw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1863
  event:
    id: 01M878K6PG9SDHCK9JQJ3C7Z2W
    sequence: 1863
    hash: 1a623d4c3dee37b19235ce798bd56545edf019f2e94e3f0365522e9cc7055200
---
# FileWatch goes blind under FSEvents backlog

Evidence (QoS executor seat 01M6S48V…, 2026-11-07): during `make land`, `crates/bbt-mux/src/index.rs:1760` and `crates/bbt-mux/src/knowledge/routing_tests.rs:69` saw no real FileWatch edit for 30 s (log `target/land-qos-final.log:274–300` in the QoS workspace). Run alone, they passed at both utility and default QoS (1.74 s and 1.92 s, `target/verify-watch-qos.log`). `fseventsd` was measured at 102% CPU during land-time churn earlier that day. So under heavy filesystem churn, FSEvents delivery lags by more than 30 s, and the knowledge index goes stale for that long, in the product, not only in tests.

## Required

1. **Bounded staleness.** If no FSEvents batch has arrived for a watched root within N seconds while writes are expected (or simply every N s, cheap and bounded), do a bounded incremental rescan (mtime/size/inode diff over the watched tree, chunked into ≤1 ms units with rest, at background or utility QoS). Pick N so the worst-case staleness is about 5 s, not unbounded. FSEvents stays primary; the rescan only catches what it delays.
2. **Honest status.** Expose index freshness (last FSEvents batch and last rescan) so a stale index is labeled, not silent.
3. **Tests.** Add a deterministic delayed-FSEvents fixture (inject delay or drop) in which the rescan still observes the edit within the bound, plus a broken control: with the rescan disabled, the delayed edit is missed and the test fails. Then remove the two quarantine entries added for these tests.

Lean landing; write the report `reports/2026-11-07-filewatch-backlog.md` and call `task_report` once.
