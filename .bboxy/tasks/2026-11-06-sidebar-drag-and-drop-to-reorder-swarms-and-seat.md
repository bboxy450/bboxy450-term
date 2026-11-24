---
assignee: none
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8ZJ1pn6ymQlhdn20cJBq2Rono7S6d/fLcgc3FK45CmU=
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
created: 2026-11-06T19:11:16.305Z
created_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
id: 01M6S4BE45334YJ59K8VW4SR7Y
number: 93377961364446
reconciliation:
- by: seat:01M8764DX3P3XKP4DF8R2DANYR
  disposition: implemented
  evidence:
  - commit b504d1adc — drag/drop, order persisted in window state
  - commit de2c09cb7 — animation
  fresh_accepted: false
  kind: historical
  op_id: 01M878M4VR2Z53NWDQ0FHBK4KR
  previous_status: open
  reason: Sidebar drag and drop reorders swarms and seats, persisted.
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Sidebar: drag and drop to reorder swarms and seats within a swarm'
type: task
updated: 2026-11-24T17:11:00.677Z
updated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
verify: make land
provenance:
  digest: sha256:c982356253e510447ce72c27a4092a655561c0bb339b70c52a81a328f4a99935
  frontmatter_digest: sha256:f80eca5677e83a890251680da30a489173287c39b14926661ca206b8ff22c54d
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
  sig: CK+3KWntZPzMyxhIzMyYkH/IA+BqzoawhAP6YXsOvotdePpt5K73ElrS38KoTNHHUdQkwlNYfMsuF/Xt+cLhDQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1899
  event:
    id: 01M878M5YMXZPCT95AS8HTKFS2
    sequence: 1899
    hash: 2d830bc68ba2ded503f84b8c932c6cdc59141b33a396d690d63fd0cae33d6640
---
# Sidebar: drag and drop to reorder swarms and seats within a swarm

In the left panel, drag a swarm row to reorder swarms and drag a seat row to reorder seats inside its swarm. Order persists.
