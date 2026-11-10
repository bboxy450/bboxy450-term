---
type: task
title: 'Q3b: ReadPrepare transport/provisioning, producer pin, offer provenance and Dream operation_status for real lesson serving'
role: executor
allowed_paths:
- crates/bboxy450-harness-wake-up/src/dream_c1.rs
- crates/bbt-mux/src/logic/dream_c1.rs
- crates/bbt-mux/src/mcp/journal.rs
verify: make test-dream-mcp && make test-formal-contracts
depends_on:
- '[[tasks/2026-11-05-dream-mcp-verified-feedback]]'
id: 01M6YCR7NNG7CP7CPHD6600ZDJ
status: open
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T20:14:17.392Z
created: 2026-11-08T20:14:17.392Z
number: 93377961366403
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:8662c17c12c3beb6fda0902783e91f4c9b410ee8d93d2fa6a247e85d45adcc39
  frontmatter_digest: sha256:8444fc96c8705de0b0768e7340f527e3c953a3a575ad87e2c760625b89b69e7b
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: CM2+9k28gT0CfmJWtfDF6kPp7s3E8ALYo6DWjywVgQPWUy6pi9AvYOIrGgXoUTRXuJryKVRSooG7fNWW7qF1Cg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150828
  event:
    id: 01M6YCR8WQGR8K1BX3EHRC8G0W
    sequence: 150828
    hash: dee313c5ba995946acabb0212afb1817685ede686a9b03e0203bea4fe713ab7d
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
