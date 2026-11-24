---
allowed_paths:
- crates/bboxy450-harness-wake-up/src/dream_c1.rs
- crates/bbt-mux/src/logic/dream_c1.rs
- crates/bbt-mux/src/mcp/journal.rs
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-08T20:14:17.392Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
depends_on:
- '[[tasks/2026-11-05-dream-mcp-verified-feedback]]'
id: 01M6YCR7NNG7CP7CPHD6600ZDJ
number: 93377961366403
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - 'successor: tasks/2026-11-24-dream-q3b-read-prepare.md'
  fresh_accepted: false
  kind: historical
  op_id: 01M878K9YQHD3FQTWXJATBR35K
  previous_status: open
  reason: Carried into the live term swarm.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Q3b: ReadPrepare transport/provisioning, producer pin, offer provenance and Dream operation_status for real lesson serving'
type: task
updated: 2026-11-24T17:10:33.172Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: make test-dream-mcp && make test-formal-contracts
provenance:
  digest: sha256:8662c17c12c3beb6fda0902783e91f4c9b410ee8d93d2fa6a247e85d45adcc39
  frontmatter_digest: sha256:ce613e6139e382e4b1f1322db6dc90d9c409d8d4d47302a2916760f09c5c4b09
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: wuuVrnh28ecoAzIwEluGwrdB6Mu9r8cFIOLwdxxVuqeNWk8+nVAEpddb5/fLwCDHpEtjpY+afEVH2ASX0kMaBA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1871
  event:
    id: 01M878KB3GEN3V9TZMPQ7Y9JAW
    sequence: 1871
    hash: c9f64543368f8c3a56e558915703112738dc6d1a5c402be1fa7909b6a91af00f
---
ROLE: EXECUTOR

## Goal
Make Q3's lesson tools serve REAL verified Q2 snapshots end to end. This follows the narrowed Q3 landing (tasks/2026-11-05-dream-mcp-verified-feedback).

## Known gaps (Q3 executor's evidence, 2026-11-08)
- crates/bboxy450-harness-wake-up/src/dream_c1.rs:2153-2160 rejects ReadPrepare and fixes the inspect-c1 argv. Add the canonical read-prepare/1 transport/provisioning path through the EXISTING spawner and provisioning, with no new spawner.
- crates/bbt-mux/src/logic/dream_c1.rs:~9406 pins the older producer. Move the pin to the accepted harness Q2 producer.
- No provider published-proof exporter exists. Positive lessons stay unavailable until one exists; scope a minimal private exporter only if the spec allows it, otherwise record it as a blocker.
- mcp/journal.rs carries only an argument digest. Add bounded offered-content provenance (offer id, snapshot pin, actor) so dream_feedback can validate same-actor offers.
- operation_status excludes Dream. Include Dream original-op recovery.

## Scope
Worktree, allowed paths and verify are set by the Oracle at dispatch. Run after the narrowed Q3 lands.
