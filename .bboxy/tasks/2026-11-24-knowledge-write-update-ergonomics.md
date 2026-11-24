---
allowed_paths:
- crates/bbt-mux
- crates/bboxy450-harness
assignee: seat:01M86Q7GPVSPJSMXHFZJKV3Q3E
authorship:
  kind: seat
  process_generation: '1'
  public_key: SmmofdjUZm33xDyoZGReapgoNQbvNynTXZw1DNs03Bk=
  signer: seat:01M86Q7GPVSPJSMXHFZJKV3Q3E
created: 2026-11-24T12:06:38.314Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M86Q6TYMYXEXA9JX6FRAP6PT
number: 1794292861627887
report:
  commit: 817513adf9a565c8530054b8975e482e9f692bcc
  op_id: 01M86TVJJGB01FXMKN2G7PXA6G
  report: '[[reports/2026-11-24-knowledge-write-update-ergonomics.den]]'
  reported_by: seat:01M86Q7GPVSPJSMXHFZJKV3Q3E
  summary: '[[Implement]] Signed patch 817513adf passes 11 focused MCP tests and current-source formal parity for 1,003 models. It adds fetched ID and path references with their etag, names malformed fields and expected shapes, and removes the protected-frontmatter echo requirement and task stamp allowlist. [[Blocked]] The required mux suite exited 100 with 945 passes and the known reassignment stale_reference failure. **Unblock** Assign or authorize the reassignment repair before make land. The clean candidate and full machine-local report are ready for review. The KB checkpoint refused the ignored reports directory.'
  verify_revision: 0
role: executor
status: reported
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'knowledge_write updates: accept the fetched id/path as ref and keep protected fields without echoing them'
type: task
updated: 2026-11-24T13:10:23.749Z
updated_by: seat:01M86Q7GPVSPJSMXHFZJKV3Q3E
verify: cargo nextest run -p bboxy450-term-mux
verify_result:
  command: cargo nextest run -p bboxy450-term-mux
  evidence:
  - criterion: Fetched ID and path, omitted protected fields, explicit protected-field refusal, stale etag and schema diagnostics
    reference: crates/bbt-mux/tests/mcp.rs:7665 and target/knowledge-write-focused.log (11 passed, exit 0)
  - criterion: Required mux suite
    reference: target/knowledge-write-nextest.log (945 passed, 1 failed, exit 100). Same stale_reference failure recorded on main in .bboxy/notes/2026-11-15-den-landed-state.md:69.
  - criterion: Formal parity and receipt
    reference: target/knowledge-write-formal.log and formal receipt sha256 49bc0e7c43b724d9e1acf7d6fd26e4891d100ced1fb7e91c451269f1e10eebf9, both formal commands exit 0.
  exit_code: 100
provenance:
  digest: sha256:adcd062ba4f5e7fbac8db74cc5fc4c8be9dd001ec5ea2366110d3e510c3b0aaf
  frontmatter_digest: sha256:94e06dedafb776f30c1d5de8ef22d490f7751503413a00fa93cd55f54c305e26
  signer: seat:01M86Q7GPVSPJSMXHFZJKV3Q3E
  sig: e2oIi/Lfn9A8BaQYSNQVln5X6cx4uS7ykSxQnJq0T5LseBr7Ue82qI4cmrg4YXc7ssrvV1UhPx+2ROzV4SJwCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1750
  event:
    id: 01M86TVKBTFT6EGZE5T3JFGC1P
    sequence: 1750
    hash: 0b5ffad6b917c36e1b05a879ce8ac08b3a62836021c624047f6750cacdc33028
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

## Report from seat:01M86Q7GPVSPJSMXHFZJKV3Q3E

[[Implement]] Signed patch 817513adf passes 11 focused MCP tests and current-source formal parity for 1,003 models. It adds fetched ID and path references with their etag, names malformed fields and expected shapes, and removes the protected-frontmatter echo requirement and task stamp allowlist. [[Blocked]] The required mux suite exited 100 with 945 passes and the known reassignment stale_reference failure. **Unblock** Assign or authorize the reassignment repair before make land. The clean candidate and full machine-local report are ready for review. The KB checkpoint refused the ignored reports directory.

```json
{
  "op_id": "01M86TVJJGB01FXMKN2G7PXA6G",
  "commit": "817513adf9a565c8530054b8975e482e9f692bcc",
  "report": "[[reports/2026-11-24-knowledge-write-update-ergonomics.den]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux",
    "exit_code": 100,
    "evidence": [
      {
        "criterion": "Fetched ID and path, omitted protected fields, explicit protected-field refusal, stale etag and schema diagnostics",
        "reference": "crates/bbt-mux/tests/mcp.rs:7665 and target/knowledge-write-focused.log (11 passed, exit 0)"
      },
      {
        "criterion": "Required mux suite",
        "reference": "target/knowledge-write-nextest.log (945 passed, 1 failed, exit 100). Same stale_reference failure recorded on main in .bboxy/notes/2026-11-15-den-landed-state.md:69."
      },
      {
        "criterion": "Formal parity and receipt",
        "reference": "target/knowledge-write-formal.log and formal receipt sha256 49bc0e7c43b724d9e1acf7d6fd26e4891d100ced1fb7e91c451269f1e10eebf9, both formal commands exit 0."
      }
    ]
  }
}
```
