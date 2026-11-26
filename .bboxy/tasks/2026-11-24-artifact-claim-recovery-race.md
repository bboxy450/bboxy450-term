---
allowed_paths:
- crates/bbt-mux
assignee: seat:01M89ZVRHFV6XQ0CKRXW7E99DQ
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
closed:
  by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  op_id: 01M8A86Y0BEZTG0AHAK8385BWF
created: 2026-11-25T00:59:29.364Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M883DZ4GDMGJVDFJ42NN19QD
number: 1794292861627939
report:
  commit: 5007ceafc9e2023d5967a31dcd5e6cf64e690d78
  op_id: 01M8A5FFHPDFPMP9SEZHPVBE3E
  report: '[[reports/2026-11-25-artifact-claim-recovery-race.den]]'
  reported_by: seat:01M89ZVRHFV6XQ0CKRXW7E99DQ
  summary: '[[Implement]] Stored artifact IDs now return `artifact_custody_conflict` during catalog recovery. The restart witness forced recovery to stay pending, reproduced the old `held` result, and passed 20/20 nextest runs across four concurrent workers with retries disabled. Removed the blanket recovery-first refusal, repeated workspace lookups, and the timing-dependent placement of the assertion. The signed source commit has a clean worktree and a verified 1,003-model formal receipt. The fresh Java TLC run timed out on a harness control, and the parity retry validated hash-matched archive evidence for the current source. The full report is stored in the local KB, where its checkpoint was refused because `reports/` is Git-ignored.'
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: artifact claim after restart races catalog recovery (held vs artifact_custody_conflict)
type: task
updated: 2026-11-25T21:01:30.612Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: cargo nextest run -p bboxy450-term-mux --test it --retries 0 -E 'test(artifact_mcp_exact_bytes_recovery_revisions_and_lineage_survive_restart)'
verify_result:
  command: cargo nextest run -p bboxy450-term-mux --test it --retries 0 -E 'test(artifact_mcp_exact_bytes_recovery_revisions_and_lineage_survive_restart)'
  evidence:
  - criterion: Stored-ID custody refusal is independent of catalog recovery
    reference: crates/bbt-mux/src/logic.rs:4868 and crates/bbt-mux/tests/mcp/artifacts.rs:172
  - criterion: Ordinary references remain held during recovery
    reference: crates/bbt-mux/tests/mcp/artifacts.rs:155 and :165
  - criterion: 20 parallel runs with retries disabled
    reference: target/claim-race-evidence/parallel-summary.log — 20/20, four workers, every exit 0
  - criterion: Current-source formal receipt
    reference: make check-formal-evidence — exit 0, SHA-256 24674797f55cb907b4a2cecc572b3c7f150f75874560bca5a8fe9a9cbf1d9e18
  exit_code: 0
provenance:
  digest: sha256:59fed3e4c0815faca142d1e822fb58b7de08e585628b7d47928a18127181113f
  frontmatter_digest: sha256:5882394e3cda9ef7b138d465aaba34371197a6dce5cbb19e288d46b74054d4dd
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: Fb2tEWcJDv8WACzieheECVj9ecP1dn7dGg9SbzLZOoy6E6IuNwyp/n64QJLqynXEYSerGzhYkwXiNyQLLF7yCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2092
  event:
    id: 01M8A86YQTCMS8EG5GSGMRNBY0
    sequence: 2092
    hash: 6bb3d8d68c9588689c6bbdeb8cd9da2e70b109c7d8dec9ed715ff6f4da45aa13
---
Our stand: subtract before you add; report what you removed.

## Evidence (2026-11-24, Oracle Vesper Greywind)
In a 30-test parallel run on `093854f86` (`cargo nextest -p bboxy450-term-mux --test it -E binary(it)&(test(claim)|test(artifact))` plus 11 lib tests), `mcp::artifacts::artifact_mcp_exact_bytes_recovery_revisions_and_lineage_survive_restart` failed once at `crates/bbt-mux/tests/mcp/artifacts.rs:228`. It got `held: artifact catalog recovery pending` (`crates/bbt-mux/src/logic.rs:4857`, `provenance.recovering()`) where it expected `artifact_custody_conflict`. The log shows `ProvenanceGap: projection-failed seq=13 ... activity router stopped`. Run alone, it passed 3/3 on `093854f86` and 3/3 on `a9e6a40e7`.

## Do
Decide which is right. Either the test must wait for catalog recovery after its restart (as `committed_claim_fixture` now does with `await_claim_projection`), or the custody check should answer before the recovery hold, since a stored-ID custody conflict does not depend on the catalog. Prefer the second if it is correct: the custody refusal needs no catalog. Prove it with 20 parallel runs at retries 0.


## Report from seat:01M89ZVRHFV6XQ0CKRXW7E99DQ

[[Implement]] Stored artifact IDs now return `artifact_custody_conflict` during catalog recovery. The restart witness forced recovery to stay pending, reproduced the old `held` result, and passed 20/20 nextest runs across four concurrent workers with retries disabled. Removed the blanket recovery-first refusal, repeated workspace lookups, and the timing-dependent placement of the assertion. The signed source commit has a clean worktree and a verified 1,003-model formal receipt. The fresh Java TLC run timed out on a harness control, and the parity retry validated hash-matched archive evidence for the current source. The full report is stored in the local KB, where its checkpoint was refused because `reports/` is Git-ignored.

```json
{
  "op_id": "01M8A5FFHPDFPMP9SEZHPVBE3E",
  "commit": "5007ceafc9e2023d5967a31dcd5e6cf64e690d78",
  "report": "[[reports/2026-11-25-artifact-claim-recovery-race.den]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux --test it --retries 0 -E 'test(artifact_mcp_exact_bytes_recovery_revisions_and_lineage_survive_restart)'",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Stored-ID custody refusal is independent of catalog recovery",
        "reference": "crates/bbt-mux/src/logic.rs:4868 and crates/bbt-mux/tests/mcp/artifacts.rs:172"
      },
      {
        "criterion": "Ordinary references remain held during recovery",
        "reference": "crates/bbt-mux/tests/mcp/artifacts.rs:155 and :165"
      },
      {
        "criterion": "20 parallel runs with retries disabled",
        "reference": "target/claim-race-evidence/parallel-summary.log — 20/20, four workers, every exit 0"
      },
      {
        "criterion": "Current-source formal receipt",
        "reference": "make check-formal-evidence — exit 0, SHA-256 24674797f55cb907b4a2cecc572b3c7f150f75874560bca5a8fe9a9cbf1d9e18"
      }
    ]
  }
}
```
