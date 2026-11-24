---
allowed_paths:
- crates/bbt-mux
assignee: seat:01M86V7X5NXNHAAPJWWVPMW83A
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
closed:
  by: seat:01M85BW3PMN3QBFPPMWBS866RA
  op_id: 01M871E8BMEPGV174YKZ2KHHD2
created: 2026-11-24T13:16:46.604Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M86V78PQCV2WJ7P0Z8THDD2X
number: 1794292861627894
report:
  commit: e8bb48e2733701d0f29c0273fb9e58e7ff537d97
  op_id: 01M87143EQYCV85TF03VSEPS62
  report: '[[reports/2026-11-24-reassignment-test-flaky.den]]'
  reported_by: seat:01M86V7X5NXNHAAPJWWVPMW83A
  summary: Fixed and landed the reassignment test’s stale index digest race. The baseline failed at the first `task_list` call. Removing the post-start fixture rewrite lets the test use the pre-indexed task, and a new assertion checks that the retained list contains it. The focused test passed 20 consecutive runs with retries disabled, `make land` passed 133 MCP tests, and the Oracle’s concurrent five-test selection passed on landed HEAD with `--retries 0 --test-threads 5`. Formal parity passed for 1,003 models.
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: mcp_task_reassignment_reauthorizes_history_and_refuses_foreign_or_ambiguous_scope is flaky on main (stale_reference)
type: task
updated: 2026-11-24T15:05:27.823Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bboxy450-term-mux --test it -E 'test(mcp_task_reassignment_reauthorizes_history_and_refuses_foreign_or_ambiguous_scope)' --retries 0
verify_result:
  command: cargo nextest run -p bboxy450-term-mux --test it -E 'test(mcp_task_reassignment_reauthorizes_history_and_refuses_foreign_or_ambiguous_scope)' --retries 0
  evidence:
  - criterion: Reproduce and locate the stale-reference failure
    reference: /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-reassignment.W6pO7BZS/baseline-backtrace-1.log:40 — first task_list call on 8e61c3bae, exit 100
  - criterion: Fix the cause and pass 20 consecutive runs with retries disabled
    reference: crates/bbt-mux/tests/mcp.rs:4038 — seeds before indexing with no subsequent rewrite before first list. /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-reassignment.W6pO7BZS/fixed-summary.log — 20 exits of 0
  - criterion: Land the verified signed commit
    reference: /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-reassignment.W6pO7BZS/land.log:121 — make land exited 0 and landed e8bb48e2733701d0f29c0273fb9e58e7ff537d97
  - criterion: Recheck the concurrent reproduction selection on landed HEAD
    reference: /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-reassignment.W6pO7BZS/parallel-landed.log — cargo nextest run --locked -p bboxy450-term-mux --test it -E 'test(/mcp_knowledge_write/) | test(/protected/) | test(mcp_task_reassignment_reauthorizes_history_and_refuses_foreign_or_ambiguous_scope)' --retries 0 --test-threads 5, exit 0, 5/5 passed
  exit_code: 0
provenance:
  digest: sha256:d94b8ddf77fc76fc41b4dc89e9f2c061b12dba63e179298229d9b5a3c9e20164
  frontmatter_digest: sha256:909120625c6013ea6181b7ddb0b6ef088c5cd306aa6327d558e0df27c89ab51f
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: yrRv54Qqw0A5RSLZLxbQDVI1xTWwijMDdDAa2F1sHll2COXoMxzkxPgMDpuaz+LiCwlQZB1YvhhT1gK5LNtRDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1801
  event:
    id: 01M871E9THE69V5DPRHM34HRGF
    sequence: 1801
    hash: 7fcd620d5b23b67765c30b955b7f7a39bf853c7180557c0dd34accf02fb6f5c0
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

## Report from seat:01M86V7X5NXNHAAPJWWVPMW83A

Fixed and landed the reassignment test’s stale index digest race. The baseline failed at the first `task_list` call. Removing the post-start fixture rewrite lets the test use the pre-indexed task, and a new assertion checks that the retained list contains it. The focused test passed 20 consecutive runs with retries disabled, `make land` passed 133 MCP tests, and the Oracle’s concurrent five-test selection passed on landed HEAD with `--retries 0 --test-threads 5`. Formal parity passed for 1,003 models.

```json
{
  "op_id": "01M87143EQYCV85TF03VSEPS62",
  "commit": "e8bb48e2733701d0f29c0273fb9e58e7ff537d97",
  "report": "[[reports/2026-11-24-reassignment-test-flaky.den]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux --test it -E 'test(mcp_task_reassignment_reauthorizes_history_and_refuses_foreign_or_ambiguous_scope)' --retries 0",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Reproduce and locate the stale-reference failure",
        "reference": "/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-reassignment.W6pO7BZS/baseline-backtrace-1.log:40 — first task_list call on 8e61c3bae, exit 100"
      },
      {
        "criterion": "Fix the cause and pass 20 consecutive runs with retries disabled",
        "reference": "crates/bbt-mux/tests/mcp.rs:4038 — seeds before indexing with no subsequent rewrite before first list. /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-reassignment.W6pO7BZS/fixed-summary.log — 20 exits of 0"
      },
      {
        "criterion": "Land the verified signed commit",
        "reference": "/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-reassignment.W6pO7BZS/land.log:121 — make land exited 0 and landed e8bb48e2733701d0f29c0273fb9e58e7ff537d97"
      },
      {
        "criterion": "Recheck the concurrent reproduction selection on landed HEAD",
        "reference": "/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-reassignment.W6pO7BZS/parallel-landed.log — cargo nextest run --locked -p bboxy450-term-mux --test it -E 'test(/mcp_knowledge_write/) | test(/protected/) | test(mcp_task_reassignment_reauthorizes_history_and_refuses_foreign_or_ambiguous_scope)' --retries 0 --test-threads 5, exit 0, 5/5 passed"
      }
    ]
  }
}
```
