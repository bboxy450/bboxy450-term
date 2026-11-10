---
type: task
title: Managed worker workspace provisioning MCP replaces Codex dispatch
verify: make test-workspace-provisioning
status: done
assignee: none
role: executor
id: 01M6PXZ5MA0VXHFD2WPK9KZTZ4
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
updated: 2026-11-06T05:10:18.886Z
created: 2026-11-05T22:41:14.142Z
number: 93377961364428
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
  reason: Implemented and landed on signed term/harness main. Canonical focused gates and source-bound formal receipts passed; both apps installed with the exact existing trusted signing leaf. Actual GUI13933a94 and fresh compatible logic are ready; supervisor PID80176 and all seat generations/swarm identities retained. Operator-owned unassigned task is historically reconciled without fabricating a seat report or fresh acceptance.
  evidence:
  - git:13933a9410322b25c72466bdc65284fc631eb69e
  - harness:a75eabfa4c19ab1af2cd192809d75fe510dff1e3
  - formal:51eb2cbf25dc4638c8d1c21f91bad47edc4ef453b87ea6c851f9c086fce4182c
  - formal:1ee5b2a045ffe2e0e6119342a3686e12b1f261be048c82bf10a2a877aca45306
  - file:/Users/feral/.local/state/bboxy450-term/python-execute-20261105/deployment/installed-verification.json
  - file:/Users/feral/.local/state/bboxy450-term/python-execute-20261105/deployment/owner-preservation.json
  - file:/Users/feral/.local/state/bboxy450-term/python-execute-20261105/deployment/startup-recovery-diagnosis.json
  by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  op_id: 01M6QM7K12JHJ56YTGN4TE71A9
provenance:
  digest: sha256:24b0f4fdd95824575ff93f8c7f48a9f98aee9c1d9e0161a8e55e18f33ee50b9e
  frontmatter_digest: sha256:961b50eba4185d8fc1ad95354df35421b8a386a40321b6c8e1d6e2d9c0647d3e
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: duTYMho7WAjpKj2zVmyHtcAmJGRV9oXCBGRgzlloYp6x4SXehFMBz7siZPBMSfn0tZouLpT08orUddqpGZ9bAQ==
  log: bboxy://activity/index/384177
  event:
    id: 01M6QM7KJ7C7DXZ4FR2MWX88RY
    sequence: 384177
    hash: ebfbfdd34cea14c2740619569b5e1a830e20e03e81578b6af7bbac7a20629ea0
---
# Managed worker workspace provisioning MCP replaces Codex dispatch

Implement native workspace_prepare/get/list/release MCP tools and managed seat_spawn/seat_split/task delegation bindings per /Users/feral/mydev/bboxy450-term/.bboxy/specs/workspace-provisioning.md. Local Git provider first; durable journal/recovery, single writer, separate canonical KB binding, safe release; remote providers fail explicitly. Remove scripts/dispatch-codex.sh, obsolete prompt, Make dispatch and all active references. Own writing worktree /Users/feral/mydev/bboxy450-term-workspace-provisioning on codex/workspace-provisioning. Add focused Make test-workspace-provisioning and relevant formal models/broken controls. Land signed source with make land and skip hosted CI; no .bboxy files in Git.
