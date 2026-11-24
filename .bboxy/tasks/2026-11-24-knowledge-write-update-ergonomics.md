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
closed:
  by: seat:01M85BW3PMN3QBFPPMWBS866RA
  op_id: 01M86Z7BDWKPTHREKS0JKQDZR5
created: 2026-11-24T12:06:38.314Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M86Q6TYMYXEXA9JX6FRAP6PT
number: 1794292861627887
report:
  commit: 4ef8d0bee1a8ae2e9a367c9681ecf270fb9fd03b
  op_id: 01M86YXXBAYB61B6JFC8HB22ZM
  report: '[[reports/2026-11-24-knowledge-write-update-ergonomics-landed.den]]'
  reported_by: seat:01M86Q7GPVSPJSMXHFZJKV3Q3E
  summary: '[[Implement]] [[Merged]] `make land` exited 0 and pushed signed commit `4ef8d0bee1a8ae2e9a367c9681ecf270fb9fd03b` to `origin/main`. All 946 selected mux tests passed on the landed commit, including the reassignment witness, and the refreshed formal receipt verifies native/TLC parity for 1,003 models. `knowledge_write` now takes fetched IDs or paths with their etag, names malformed fields and expected shapes, and removes the requirement to echo protected frontmatter and the task stamp allowlist. The worktree is clean.'
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'knowledge_write updates: accept the fetched id/path as ref and keep protected fields without echoing them'
type: task
updated: 2026-11-24T14:26:44.346Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bboxy450-term-mux
verify_result:
  command: cargo nextest run -p bboxy450-term-mux
  evidence:
  - criterion: Short refs, partial frontmatter, protected-field refusal, and schema errors
    reference: crates/bbt-mux/tests/mcp.rs:7665. Earlier focused run 11/11. Landed suite reran them, 946/946 overall, exit 0 in target/knowledge-write-nextest-landed.log.
  - criterion: Land signed commit on main
    reference: target/knowledge-write-land.log, exit 0 and LANDED 4ef8d0bee1a8ae2e9a367c9681ecf270fb9fd03b. Strict Clippy, 434 mux unit and 133 MCP integration tests, app check, and git ancestor/signature checks succeeded.
  - criterion: Refresh formal evidence on landed SHA
    reference: target/knowledge-write-formal-landed.log and verified receipt SHA-256 743cdc2bd76e81afda33807e9d9d1918edec37a687fcd4a16e8077fc837619cd. Both formal commands exited 0.
  exit_code: 0
provenance:
  digest: sha256:c64ac47a2e7a74462c2cb53a447cde91a310155b5f5e0521c8091762192d3294
  frontmatter_digest: sha256:d7f87a56120b73f0908c37176a1ee59ceb397e5a81c879ad3f6d71516d50b663
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: d6aSBnrBW2Yg1ZJFpbtllrBShGd/3n0/+hVsCjKmvUw6sQ4FBgbDaj2e15f8CAdnyXjb476JgGs779TsZg9vCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1787
  event:
    id: 01M86Z7CM4EPTSQ6A7YESANMZ2
    sequence: 1787
    hash: 6ee492aeb034b64958d7c828be493111a7b9a918ef4c6809c0c7131c413d0f82
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


## Report from seat:01M86Q7GPVSPJSMXHFZJKV3Q3E

[[Implement]] [[Merged]] `make land` exited 0 and pushed signed commit `4ef8d0bee1a8ae2e9a367c9681ecf270fb9fd03b` to `origin/main`. All 946 selected mux tests passed on the landed commit, including the reassignment witness, and the refreshed formal receipt verifies native/TLC parity for 1,003 models. `knowledge_write` now takes fetched IDs or paths with their etag, names malformed fields and expected shapes, and removes the requirement to echo protected frontmatter and the task stamp allowlist. The worktree is clean.

```json
{
  "op_id": "01M86YXXBAYB61B6JFC8HB22ZM",
  "commit": "4ef8d0bee1a8ae2e9a367c9681ecf270fb9fd03b",
  "report": "[[reports/2026-11-24-knowledge-write-update-ergonomics-landed.den]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Short refs, partial frontmatter, protected-field refusal, and schema errors",
        "reference": "crates/bbt-mux/tests/mcp.rs:7665. Earlier focused run 11/11. Landed suite reran them, 946/946 overall, exit 0 in target/knowledge-write-nextest-landed.log."
      },
      {
        "criterion": "Land signed commit on main",
        "reference": "target/knowledge-write-land.log, exit 0 and LANDED 4ef8d0bee1a8ae2e9a367c9681ecf270fb9fd03b. Strict Clippy, 434 mux unit and 133 MCP integration tests, app check, and git ancestor/signature checks succeeded."
      },
      {
        "criterion": "Refresh formal evidence on landed SHA",
        "reference": "target/knowledge-write-formal-landed.log and verified receipt SHA-256 743cdc2bd76e81afda33807e9d9d1918edec37a687fcd4a16e8077fc837619cd. Both formal commands exited 0."
      }
    ]
  }
}
```
