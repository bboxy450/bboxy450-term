---
assignee: none
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8ZJ1pn6ymQlhdn20cJBq2Rono7S6d/fLcgc3FK45CmU=
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
created: 2026-11-06T19:08:55.917Z
created_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
id: 01M6S474YJW2MJKA4WCHNVWQFQ
number: 93377961364445
reconciliation:
- by: seat:01M8764DX3P3XKP4DF8R2DANYR
  disposition: implemented
  evidence:
  - commit 47b7e5268 — sudo role
  - crates/bbt-mux/src/mcp/sudo.rs:43-48 — sudo_swarms, sudo_as
  - commit b7ed5e5ee — sudo_as workspace follow-up
  fresh_accepted: false
  kind: historical
  op_id: 01M878M3KD419NHKMB008ZSGJE
  previous_status: open
  reason: Sudo role and cross-swarm tools landed.
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Sudo role: one harness with cross-swarm operator authority over MCP'
type: task
updated: 2026-11-24T17:10:59.448Z
updated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
verify: make land
provenance:
  digest: sha256:7f43f0bccfc752380c2d81cefba99818567c50d0c26ca4bfa2a07ee52d28df1e
  frontmatter_digest: sha256:91d1b01682e52506df61a9abee5c57db3917e17ccc570099379b5cc7660244f6
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
  sig: +kSg1GI+PvQT5G54CnLRq9iWV7p1j7gKgD+qJ4FvVV2iwDeZb/yvGTqWa6Rgz6BAbfVpJwSjj43+VN+MLIcvAA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1897
  event:
    id: 01M878M4QSZY7497A8AFXHAYTY
    sequence: 1897
    hash: 18381872018aa285a40be70ac2fb6d63f3da5596bf5c9fea6ee472ff474adef3
---
# Sudo role: one harness with cross-swarm operator authority over MCP

Add a builtin sudo role. Sudo seats get Oracle tools in their own swarm plus sudo_* MCP tools: list every swarm, spawn any role (including oracles) in any or a new swarm, message and read the operator inbox in any swarm, and run any MCP tool as any live seat. Only the operator or another sudo seat may assign the sudo role.
