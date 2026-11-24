---
allowed_paths:
- crates/bbt-mux/src/mcp/
- crates/bbt-mux/src/knowledge/
- crates/bbt-mux/src/provenance/
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
- crates/bbt-mux/tests/
- verification/
- .bboxy/reports/
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-06T22:05:46.090Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6SEAXBXW6QMYTFVPM2ZKZ6E
number: 93377961366137
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - 'successor: tasks/2026-11-24-artifact-claim-after-commit.md'
  fresh_accepted: false
  kind: historical
  op_id: 01M878K4CQCGF281EDK4R1X11A
  previous_status: open
  reason: Carried into the live term swarm.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: artifact_claim refuses committed reports with artifact_not_committed (seen 3 times)
type: task
updated: 2026-11-24T17:10:27.599Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: cargo nextest run -p bboxy450-term-mux -E 'test(/artifact/)' && make test-formal-contracts
provenance:
  digest: sha256:e91c141bced52231cc204f8d825cde13e3905075d27c9604c14fc4d9d1b1c16e
  frontmatter_digest: sha256:234d97d73d77f3a3778f00a16571fdc4c1adeee642894f939a7a9e05e7cbc5c2
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: 6m6wqZXjBVXNeC4wYFWXo9brDcEHeaUilmPM44K1bWK2histOm+Fx+AN9P2UUQgKx2xZ476sN+eS6GC4796PCA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1861
  event:
    id: 01M878K5NSKHP66YG73W4M4AP7
    sequence: 1861
    hash: ca184b1a19af0a13cbfc518e4981c91d8e4b4f0797096c4b3d985876a9583094
---
# artifact_claim says "not committed" for committed reports

Seen three times on 2026-11-06. In each case the executor first committed its report with `knowledge_commit`, and `knowledge_fetch` then showed `publication_state: committed` with a checkpoint artifact ID. `artifact_claim` was still refused with `artifact_not_committed`:

- seat 01M6S0BP… (report checkpoint 87c54975)
- seat 01M6RZZXSH… (knowledge commit 15cd2f78, artifact 01M6SDHC291ZMHACRSPN4P2848; ops 01M6SDHFZWE6D89781ER48YS3S and 01M6SDHSK0NTWEKTNAZJAP7FPC)
- seat 01M6S7423S… (checkpoint d5c3fc67, with the provenance projection fully caught up)

Find which notion of "committed" `artifact_claim` checks (code-repo Git? a different checkpoint table? a projection lag?) and align it with `knowledge_commit`'s signed artifact receipt. Add a test: commit a report, then claim it, and the claim succeeds. Broken control: an uncommitted draft is still refused. Use the formal paths pre-granted above if the provenance model changes. Lean landing; write the report `reports/2026-11-06-artifact-claim.md` and call `task_report` once.
