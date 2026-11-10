---
type: task
title: Register tool-access spec and specify managed worker workspaces
verify: python3 -I /Users/feral/.local/state/bboxy450-term/programmatic-tools-study-20261105/validate_kb_specs.py
status: done
assignee: none
role: executor
id: 01M6PWWF38K2NTJ42XBFNB3G8Z
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
updated: 2026-11-05T22:22:37.591Z
created: 2026-11-05T22:22:16.890Z
number: 93377961364427
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
  reason: 'Documentation deliverable complete: both requested numbered draft specs are in .bboxy and read back by native KB with matching body/digest. This does not claim the proposed runtime features are implemented.'
  evidence:
  - /Users/feral/.local/state/bboxy450-term/programmatic-tools-study-20261105/kb-specs-report.md
  - /Users/feral/.local/state/bboxy450-term/programmatic-tools-study-20261105/kb-specs-validation.json
  - /Users/feral/.local/state/bboxy450-term/programmatic-tools-study-20261105/validation.json
  - /Users/feral/mydev/bboxy450-term/.bboxy/specs/programmatic-tool-access.md sha256:09e2826bc402feca544bee7d5f0d64c834352062522b84a63e7f97f2c9e87214
  - /Users/feral/mydev/bboxy450-term/.bboxy/specs/workspace-provisioning.md sha256:87ae7ba7dab7b99f19054f4e38d82a4f6071167859b7ebeb761be94d9ffe8c21
  by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  op_id: 01M6PWX32RHDR2M8T4DAG22JQY
provenance:
  digest: sha256:1fd09a851a65e0a8a039bc533b8d22a4057d5bf5428b846b3019efff2427e6db
  frontmatter_digest: sha256:f211f76d237879ccab148eae3005cd12caf37b3b052c4f59f9e61558325c2841
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: iu++vOXI80BMJSqWhJkp0fo0xt7Rl8L5p9rPNzpkup0SWSChHl+1jaqOQUthkGeCx5esYBGeeAxndkygrM1cBw==
  log: bboxy://activity/index/384140
  event:
    id: 01M6PWX3JQGJQDQBV7YKA8CK09
    sequence: 384140
    hash: f4f93a461f7927fcc53f1421c5217d126239195ba6aa3e0d52706d1ab481ba71
---
# Register tool-access spec and specify managed worker workspaces

Operator requested both designs as proper numbered specs in .bboxy: register the Python tool-access study and specify one Oracle workspace provisioning interface, Git worktrees first and provider-neutral remote targets later. Specs only; do not implement the proposed runtime.
