---
type: task
title: artifact_claim refuses committed reports with artifact_not_committed (seen 3 times)
role: executor
allowed_paths:
- crates/bbt-mux/src/mcp/
- crates/bbt-mux/src/knowledge/
- crates/bbt-mux/src/provenance/
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
- crates/bbt-mux/tests/
- verification/
- .bboxy/reports/
verify: cargo nextest run -p bboxy450-term-mux -E 'test(/artifact/)' && make test-formal-contracts
status: open
id: 01M6SEAXBXW6QMYTFVPM2ZKZ6E
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-06T22:05:46.090Z
created: 2026-11-06T22:05:46.090Z
number: 93377961366137
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:e91c141bced52231cc204f8d825cde13e3905075d27c9604c14fc4d9d1b1c16e
  frontmatter_digest: sha256:0de6e674fc85fba2e315f1a169f1354316aec6c7f03e91cee2c2c5d417969dcf
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: p8f+w49qLXClbA7lqBkAui2rmxx3M5ArinLJO0zipCvzaT5xRUW7CzGLKm7bjjK2QNIhkR/2Vc21aj2coNTwBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149908
  event:
    id: 01M6SEAYX8TD8YS0SAXFK0ZRZA
    sequence: 149908
    hash: e06784ce51fadb80388b995f644b5693f09241bc55153e334e8bdae4aefac79c
---
# artifact_claim says "not committed" for committed reports

Seen three times on 2026-11-06. In each case the executor first committed its report with `knowledge_commit`, and `knowledge_fetch` then showed `publication_state: committed` with a checkpoint artifact ID. `artifact_claim` was still refused with `artifact_not_committed`:

- seat 01M6S0BP… (report checkpoint 87c54975)
- seat 01M6RZZXSH… (knowledge commit 15cd2f78, artifact 01M6SDHC291ZMHACRSPN4P2848; ops 01M6SDHFZWE6D89781ER48YS3S and 01M6SDHSK0NTWEKTNAZJAP7FPC)
- seat 01M6S7423S… (checkpoint d5c3fc67, with the provenance projection fully caught up)

Find which notion of "committed" `artifact_claim` checks (code-repo Git? a different checkpoint table? a projection lag?) and align it with `knowledge_commit`'s signed artifact receipt. Add a test: commit a report, then claim it, and the claim succeeds. Broken control: an uncommitted draft is still refused. Use the formal paths pre-granted above if the provenance model changes. Lean landing; write the report `reports/2026-11-06-artifact-claim.md` and call `task_report` once.
