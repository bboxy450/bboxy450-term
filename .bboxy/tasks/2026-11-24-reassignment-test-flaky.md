---
allowed_paths:
- crates/bbt-mux
assignee: seat:01M86V7X5NXNHAAPJWWVPMW83A
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
created: 2026-11-24T13:16:46.604Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M86V78PQCV2WJ7P0Z8THDD2X
number: 1794292861627894
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: mcp_task_reassignment_reauthorizes_history_and_refuses_foreign_or_ambiguous_scope is flaky on main (stale_reference)
type: task
updated: 2026-11-24T13:17:10.107Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bboxy450-term-mux --test it -E 'test(mcp_task_reassignment_reauthorizes_history_and_refuses_foreign_or_ambiguous_scope)' --retries 0
provenance:
  digest: sha256:d417a601d1b0dd263341d5eeb74c915571d861ee00fed18a038a31bc9533ef7f
  frontmatter_digest: sha256:b782c24d04a3ddff9c9eec6f2ba0f2190a886599df6bedadb0caba8346279afb
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: HMFPio6IbDRx5a4aV72zelfxEG8syJOnQiZILNrP7+DhPQVdco9mLp2Mzxp80EM2R45VgJ7FSJ7JOWWWybC3BA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1758
  event:
    id: 01M86V808B3VTW7ERXCDD9GR5K
    sequence: 1758
    hash: a78780bc1ca66ce74587c0f3c4fc0e2f908c3dc97ce2de099fff4ff44c93b12d
---
Our stand: subtract before you add; report what you removed.

## Evidence (2026-11-24, Oracle, observed)
- origin/main 739879d4f: `1 passed (1 flaky)` — failed first try, passed on retry.
- Executor run on 817513adf: failed both tries (`TRY 2 FAIL`) with a stale_reference outcome; the same commit then passed 5/5 with `--retries 0` for the Oracle.
So the test races. Find the race (likely an index/reference refresh the test does not wait for), cite file:line, and make the test or the product deterministic.

## Do
1. Reproduce under load (e.g. `--test-threads` high, or 20 runs `--retries 0`); record the failing assertion.
2. Fix the cause, not the retry count. 20 consecutive `--retries 0` passes.
3. `make land`.