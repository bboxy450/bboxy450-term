---
type: task
title: Fullscreen Mermaid viewer with zoom and pan
verify: make check-mermaid-fullscreen test-mermaid-fullscreen test-licenses
status: done
assignee: none
role: executor
id: 01M6PAZ2Q3D6TN5MGDVMZM5921
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
updated: 2026-11-05T17:26:32.784Z
created: 2026-11-05T17:09:08.216Z
number: 93377961364420
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
  reason: Direct operator work landed and installed; this native task was created unassigned for the external Executor, so record completion without assigning or waking another seat.
  evidence:
  - main commit b6e84fbc7cf5ae484aaf701f320b6d51cab32a09, GPG signature verified
  - 'make check-mermaid-fullscreen test-mermaid-fullscreen test-licenses: exit 0, six focused viewer tests passed'
  - 'make bundle and reopen-prepared: exit 0, signed UI source b6e84fbc ready; supervisor80176 and logic17459 birth identities unchanged'
  - /Users/feral/.local/state/bboxy450-term/mermaid-fullscreen-20261105/report.md
  by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  op_id: 01M6PBYYMAASWQZK7626FBP59K
provenance:
  digest: sha256:e2b8f1dd783a106b252201c5d196fcf8ba74ecd6b792826b2c8a02fb0bb01435
  frontmatter_digest: sha256:05d0e371cb13ffa4c1ec119dd6456492861d912f9caa4e353015d7f1b956878c
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: nzKr0fPROMccr/CovUx/JDDT6zu6fI1F2uUdHRF4wkiDD8Grj4tvoJfqpAa0BQjkqnoZwo+UXYzcMrfkgTAmDg==
  log: bboxy://activity/index/384097
  event:
    id: 01M6PBYZ4NRP6GHZ1EZG36CW3N
    sequence: 384097
    hash: 60e42984c709550c5e6cddeaca4fe93a7b9c4de89c6b56203a82f9ce67a433b2
---
# Fullscreen Mermaid viewer with zoom and pan

Direct operator request: make the Mermaid expand button open a window-filling modal in Markdown documents and embedded harness output. Include zoom, Fit, 1:1, two-axis pan and Escape/Close; preserve the underlying scroll and running supervisor. Implement in fix/mermaid-fullscreen, verify focused checks, signed main landing, install and reopen UI only.
