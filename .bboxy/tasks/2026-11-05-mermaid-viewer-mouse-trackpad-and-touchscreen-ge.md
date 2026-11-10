---
type: task
title: Mermaid viewer mouse trackpad and touchscreen gestures
verify: make check-mermaid-fullscreen test-mermaid-fullscreen test-licenses
status: done
assignee: none
role: executor
id: 01M6PE9MCCGE95EHFS05TKNE3B
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
updated: 2026-11-05T18:17:43.765Z
created: 2026-11-05T18:07:19.715Z
number: 93377961364422
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
  reason: Direct operator gesture extension landed and installed; external Executor task was intentionally unassigned, so record completion without assigning or waking another seat.
  evidence:
  - main commit 17eabaf2d8e6df2bdc91a67949a489066aafd38e, GPG signature verified
  - 'make check-mermaid-fullscreen test-mermaid-fullscreen test-licenses: exit0, nine focused geometry/native-input tests passed'
  - 'make bundle and reopen-prepared: exit0, signed source17eabaf2 ready; supervisor80176 and logic17459 birth identities unchanged; all14 seat identities preserved'
  - /Users/feral/.local/state/bboxy450-term/mermaid-gestures-20261105/report.md
  by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  op_id: 01M6PEWNMRFJ9RPAV3FBM8Y995
provenance:
  digest: sha256:a6b4591a25d388134ee720232088d75b173caf155e78d8aabdeb5735fb8cb323
  frontmatter_digest: sha256:be7307d9f0cba3880e13a80a6c9a588a0ffd0c398caf1ae07ab74163f9e1e942
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: Ur/FITJ/s1dj8vngbw+MTlbY+VdZNdGw6D7Ihqt2iwUhF5wm87dARb3/SWUebheSWAeU4oE3ZKdmQqizdLwTDA==
  log: bboxy://activity/index/384110
  event:
    id: 01M6PEWP4N1050QSYXBJ3A1AFR
    sequence: 384110
    hash: bbe7844546c55d842806fc13ea60aff6d33d07b278800e1fc37ff2fc77af50ba
---
# Mermaid viewer mouse trackpad and touchscreen gestures

Direct operator extension: add native trackpad pinch/smart zoom, pointer-anchored mouse wheel zoom, touchscreen one-finger pan/two-finger pinch and pan, completed button taps and touch-size controls. Keep all view gestures local to the fullscreen modal; preserve backend and seat sessions. Use fix/mermaid-gestures, focused input/geometry tests, signed main commit and UI-only installation.
