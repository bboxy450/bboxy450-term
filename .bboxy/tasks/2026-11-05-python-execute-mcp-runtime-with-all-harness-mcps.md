---
type: task
title: Python execute MCP runtime with all harness MCPs and command bindings
verify: make test-python-execute
status: done
assignee: none
role: executor
id: 01M6PZ46K8544KS1EJEJ9H266X
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
updated: 2026-11-06T05:10:21.340Z
created: 2026-11-05T23:01:27.494Z
number: 93377961364435
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
  op_id: 01M6QM7NGJR9QX2XRQQ9X7GWES
provenance:
  digest: sha256:3e3700b97cbd51e7ddd5d014dcf7e61f4127570f6a020e95ae078949cd0c8739
  frontmatter_digest: sha256:049b694be5d7aaab42c371b01cb75362a976c991690c6861652ee3dff03ee1b8
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: o/8QfJttwY2eb3KEt4laqllvLry94J3dmJyHNrDoSixFan+26gUYxep84kvQozgaaELsihx84KWZCY7T6Q7+BA==
  log: bboxy://activity/index/384181
  event:
    id: 01M6QM7NYZ7CKW2HF9NGKH56N7
    sequence: 384181
    hash: f64733ddc11187c13a4d89be68d2b4013b95839f8be508fa748ddf92d0f1af15
---
# Python execute MCP runtime with all harness MCPs and command bindings

Implement python_execute MCP with full CPython, current-harness configured MCP broker, direct generic executable/rg/git bindings, durable lifecycle/output/cancel and Python orientation. Integrate managed workspaces from concurrent workspace provisioning task. General orient hints knowledge_orient, capability-eligible workspace_orient and python_execute_orient. Full contract /Users/feral/mydev/bboxy450-term/.bboxy/specs/programmatic-tool-access.md. Own /Users/feral/mydev/bboxy450-term-python-execute and its owned harness submodule worktree; land harness then term pin if harness changes. All Markdown in canonical .bboxy; JSON/log/source helpers can be private state.
