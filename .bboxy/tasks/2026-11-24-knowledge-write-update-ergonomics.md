---
allowed_paths:
- crates/bbt-mux
- crates/bboxy450-harness
assignee: seat:01M86Q7GPVSPJSMXHFZJKV3Q3E
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
created: 2026-11-24T12:06:38.314Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M86Q6TYMYXEXA9JX6FRAP6PT
number: 1794292861627887
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'knowledge_write updates: accept the fetched id/path as ref and keep protected fields without echoing them'
type: task
updated: 2026-11-24T12:07:01.919Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bboxy450-term-mux
provenance:
  digest: sha256:b0968ad33614cba63fbce4c536800b897e2b3e3e0e7a7615f1de52033ef4adf8
  frontmatter_digest: sha256:f31d2dcf429ac81d3ad90015d3dd98e6695416bed5a901dd4f62b505e7968800
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: 1OLus5MkxW4MOTzz9znd7P+YjZXGK2XCrVdpXw2+p+9RSP5YL4VAAScciK0cFRaQp70lxNYvOdxtlf9m6KA9AA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1730
  event:
    id: 01M86Q7JP2G9S5897B0V0GR4W5
    sequence: 1730
    hash: f1bfb2d7de2af16edb8f25d38fa7866356a3ca7c84cf5e929e861a1712c49e6d
---
Our stand: subtract before you add; report what you removed.

## Evidence (2026-11-24, Oracle, observed, ~8 failed attempts in one session)
Amending a task's `verify` from Python:
- `ref=f['id']` and `ref={"id": f['id']}` → "rejected: arguments do not match the tool schema" with no field named. The schema's RecordRef requires all of id, number, slug, path, digest.
- `path=…` with `etag` → "etag_mismatch: create cannot supply an existing etag" (path means create).
- Full RecordRef with only `{title, verify, allowed_paths, type}` frontmatter → "task_field_protected: protected task fields differ: assignee, delegated_by, role, status, swarm".
- Only `frontmatter=dict(fetched['frontmatter'])` with the one field changed works.
The fetch result already has every needed field; the caller should not have to rebuild them.

## Do
1. Accept `ref` as the fetched id string or path string (resolve it), or the full RecordRef. One rule, documented in the tool description.
2. On update, treat omitted protected/managed frontmatter keys as "unchanged" instead of "differ"; still refuse a caller who sets a protected key to a different value.
3. Schema errors name the field and the expected shape.
4. Tests: id-string ref update; partial frontmatter update keeps protected fields; changing a protected field is still refused (broken control).
5. `make land`.