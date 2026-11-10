---
type: task
title: 'Sudo role: one harness with cross-swarm operator authority over MCP'
verify: make land
status: open
assignee: none
role: executor
id: 01M6S474YJW2MJKA4WCHNVWQFQ
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
updated: 2026-11-06T19:08:55.917Z
created: 2026-11-06T19:08:55.917Z
number: 93377961364445
created_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
updated_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
authorship:
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  kind: local-dev
  principal: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  device_id: 01M62F1BBJT8MW0F3YMZJ8BENM
  public_key: a8bbsPrJyl7hZraHSoUBkc76MQiRVYXx/kgj6IWVFt0=
provenance:
  digest: sha256:7f43f0bccfc752380c2d81cefba99818567c50d0c26ca4bfa2a07ee52d28df1e
  frontmatter_digest: sha256:d9335575ae8d1dafcc24eb7175a9d559713ef854d28d09e3d63dced5ace2a508
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: vi3zpat/uLi3+tnO6sqbYe7xODK7EM2zLqx4ngei1K6QGVkvlXf8f9M/rmYk5PTq7fwP9UfxCaDwQ9OpaSZaBQ==
  log: bboxy://activity/index/384239
  event:
    id: 01M6S4759QBFJQRVFHA8E06GDA
    sequence: 384239
    hash: 989a8d0353fcf492d57323b3b7f496cd49b4e7da713431a4eaf4d868cdf294e7
---
# Sudo role: one harness with cross-swarm operator authority over MCP

Add a builtin sudo role. Sudo seats get Oracle tools in their own swarm plus sudo_* MCP tools: list every swarm, spawn any role (including oracles) in any or a new swarm, message and read the operator inbox in any swarm, and run any MCP tool as any live seat. Only the operator or another sudo seat may assign the sudo role.
