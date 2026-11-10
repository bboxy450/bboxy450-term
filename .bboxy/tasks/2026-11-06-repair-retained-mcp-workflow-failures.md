---
type: task
title: Repair retained MCP workflow failures
verify: make test-mcp-recovery
status: done
assignee: none
role: executor
id: 01M6S0RWM9QY1WC84HMBD8R35H
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
updated: 2026-11-06T19:13:40.356Z
created: 2026-11-06T18:08:42.903Z
number: 93377961364442
created_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
updated_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
authorship:
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  kind: local-dev
  principal: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  device_id: 01M62F1BBJT8MW0F3YMZJ8BENM
  public_key: a8bbsPrJyl7hZraHSoUBkc76MQiRVYXx/kgj6IWVFt0=
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: open
  disposition: implemented
  reason: Verified fixes landed and activated at 3771188e82b78886cea9bac1789ad7dd7dfcaaa4; historical audit closeout, without peer task acceptance.
  evidence:
  - git:3771188e82b78886cea9bac1789ad7dd7dfcaaa4
  - /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/land.log
  - '[[e2e/2026-11-06-mcp-workflow-recovery]]'
  by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  op_id: 01M6S4FRCYTRNRZ6XCE0ME0RCY
provenance:
  digest: sha256:8bb99d712edb0254cb607c921ce41e923a80c5a796f34998ea480ea619d443bc
  frontmatter_digest: sha256:ab181d6b087f5191173b90e89b231ff1f9d49dbc7dea8ca00ab67106cd100db0
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: xZX4T99l/H1doKA12lpBj4una931j+CmMKXHalTlnEak30luxHZto4QuTVNMYARVhFTiw53eNt8yLgLzbzcCBA==
  log: bboxy://activity/index/384253
  event:
    id: 01M6S4FV27VQ8C2RJGWEY9RZ7V
    sequence: 384253
    hash: 339d2e7a5fd9400f20be735a0cb2f38529c315fd68c04e1546b7bc9ae2bf6c59
---
# Repair retained MCP workflow failures

Direct operator request: consume ctl debug errors and fix current defects, landing signed changes on main promptly. Scope: normalize linked report locators before strict provider resolution; support a fenced one-time initialization of blank unowned task reservations by an Oracle; close same-swarm orphaned seats without requiring a viewport. Preserve generation, role, root, etag, task lifecycle and no-replay guards. Worktree: /Users/feral/mydev/bboxy450-term-debug-errors. Keep all evidence private and do not replay failed/unknown production operations. Verify focused native regressions, required formal models and the lean landing gate.
