---
type: task
title: Discover repository task slug families and improve numbered knowledge references
verify: make check-gitkb-plugin
status: done
assignee: none
role: executor
id: 01M6QMSKGDCWRFB0GECGWZ77R5
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
updated: 2026-11-06T05:54:00.271Z
created: 2026-11-06T05:20:09.027Z
number: 93377961364438
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
  reason: 'Implemented on signed main and installed/approved for Harmony. Repository task conventions are read through the pinned GitKB parser; #N references remain valid for native and GitKB stores. Focused checks,386host tests,application compatibility and live provider readiness passed. Supervisor,logic and GUI owners retained; no seat impersonation or fresh acceptance claimed for this operator-owned unassigned task.'
  evidence:
  - git:2847e213386fbd7dcfa2cdf289e40552ccc4755d
  - plugin-sha256:6b5d7055eba9d27b674cc9c7739c00bdbfcb8a904cbfec3858efdebbd8d6620e
  - file:/Users/feral/.local/state/bboxy450-term/task-slugs-20261105/land.log
  - file:/Users/feral/.local/state/bboxy450-term/task-slugs-20261105/check-gitkb-final.log
  - file:/Users/feral/.local/state/bboxy450-term/task-slugs-20261105/approval.json
  - file:/Users/feral/.local/state/bboxy450-term/task-slugs-20261105/knowledge-ready.json
  - e2e/2026-11-06-gitkb-task-conventions.md
  by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  op_id: 01M6QPQJKGFRPS1WV2W8386JA2
provenance:
  digest: sha256:6bfe18078d63537f2d38f58cd58d56f95f98f4b2bcb51418b7c54856c554a4e9
  frontmatter_digest: sha256:d8459eb1124d37ee8c3c6dbefcb0db831ad0a391ca039b6e54870a4395f6bb62
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: x7pB3iX0Iqy07+E1r8wVGBwwBiN/hTDSfUgzBOu/3SiMYSFEMasd28en2leDMCC/PL5UZyZkYxWo7sF2+kl7BA==
  log: bboxy://activity/index/384213
  event:
    id: 01M6QPQKGE2FAYSH57WXMACDVH
    sequence: 384213
    hash: 32c1793e5e4f67fbe425837c90a43f2392acbbee9f0d42be0d7092349faebe1c
---
# Discover repository task slug families and improve numbered knowledge references

Use Harmony .kb/config.toml slug_authority tasks/harmony for GitKB orientation, creation guidance and safe numbered task lookup; retain native KB numbered references. Implement in isolated codex/task-slug-guidance worktree, focused tests only, signed make land, build/install/approve plugin for Harmony, preserve supervisor and live seats.
