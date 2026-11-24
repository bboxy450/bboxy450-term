---
allowed_paths:
- crates/bboxy450-harness-wake-up
- crates/bbt-mux
- verification
- .bboxy/interfaces
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8ZJ1pn6ymQlhdn20cJBq2Rono7S6d/fLcgc3FK45CmU=
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
created: 2026-11-24T17:09:34.732Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
id: 01M878HHAHFJVH8E6H7SDHEZBM
number: 1794292861627917
role: executor
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Dream Q3b: read-prepare/1 transport through the existing spawner'
type: task
updated: 2026-11-24T17:09:34.732Z
updated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
verify: make test-dream-c1
provenance:
  digest: sha256:fd1f073c1b6f9d200cae9e0a8da6b954647e695fc258241d370c6014076a184a
  frontmatter_digest: sha256:972d67aa29c78d44b887f4db26b1c71a284898f0d43278bcbf90aa67d5246006
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
  sig: nO7DhmCXvkpx/dlWxKX/kfySAVmbs2DWuz8TwRa0Pdpx/Yq5V9DU99ty+63qRTNjdmE6C01nUepE9w9CY1ruCA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1853
  event:
    id: 01M878HJ0X2DXHAQWVVY820VMJ
    sequence: 1853
    hash: 3938466df15399ad75e1e8c60c81c762fe418064b802df7d24c396e2f112ceb9
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-08-dream-q3b-readprepare-transport.md` (full brief there).

## Evidence
`crates/bboxy450-harness-wake-up/src/dream_c1.rs:2159-2166` still refuses any purpose but `InspectC1` and hardcodes the `inspect-c1` argv. No Q3b commits since 11-08.

## Do
Follow the legacy brief: add the read-prepare/1 purpose through the existing spawner (no second spawner), move the producer pin, record offer provenance in `mcp/journal.rs`, include Dream in `operation_status`. Update the ICD and its model/broken control. `make land`.
