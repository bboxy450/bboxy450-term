---
allowed_paths:
- crates/bbt-mux
- verification
assignee: seat:01M8A89QRMC06Y5D5M4QFDPZGB
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
closed:
  by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  op_id: 01M8AJRDVPBD5SVTXQXY7VA0A0
created: 2026-11-24T17:09:33.899Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M878HGHX4HZ8KT2ZBGJMK22D
number: 1794292861627916
report:
  commit: 972720fa8c236232143046d793d70726768a649b
  op_id: 01M8AJM63JF33XG4GS7Q79EYD8
  report: '[[reports/2026-11-25-workspace-index-bounds.den]]'
  reported_by: seat:01M8A89QRMC06Y5D5M4QFDPZGB
  summary: '[[Implement]] [[Verified]] Landed workspace index bounds at 972720fa8. After each of 32 real swarm closures the native fixture returned to one startup root and zero bindings. With eviction disabled it retained 33 roots and 32 bindings. The router reaps closed, archived, and released mounts, caps root slots at 64 and swarm bindings at 1,024 plus the legacy startup binding, and evicts the least-recent idle root. Removed the duplicate path map and redundant completed-mount payload. The focused check passed 27 tests. make land, the 1,006-model parity gate, and the source-bound receipt check passed. The worktree is clean and the evidence report is stored in the local KB.'
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Logic indexes: bound Workspaces roots/paths/bindings with eviction'
type: task
updated: 2026-11-26T00:05:50.037Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: cargo nextest run -p bboxy450-term-mux -E 'test(workspaces)' --retries 0
verify_result:
  command: cargo nextest run -p bboxy450-term-mux -E 'test(workspaces)' --retries 0
  evidence:
  - criterion: Reap closed and archived swarm bindings and released checkouts
    reference: crates/bbt-mux/src/knowledge/routing.rs:348 and crates/bbt-mux/src/logic/implementation_workspaces.rs:309. Native archive, shared-root, and release witnesses passed.
  - criterion: Enforce root and binding limits and evict the least-recent idle root
    reference: crates/bbt-mux/src/knowledge/routing.rs:24 and crates/bbt-mux/src/knowledge/routing/workspaces_retention_tests.rs:151. The held-watch witness proves cleanup keeps its slot.
  - criterion: Long-run churn with a growing control
    reference: crates/bbt-mux/src/knowledge/routing/workspaces_retention_tests.rs:121. 32 closures left 1 root and 0 bindings. Without eviction the fixture retained 33 roots and 32 bindings.
  - criterion: Signed lean landing and formal verification
    reference: make land exit 0 at 972720fa8c236232143046d793d70726768a649b. Formal evidence SHA-256 cd22aea375500e3cdd6ab07e0c678974438462f8a79a1d39df6e59a7ef36e1ac.
  exit_code: 0
provenance:
  digest: sha256:d71c5c46ae41060542536a74d448bfd11eeb03e7aca67e9212e38eb2f996f088
  frontmatter_digest: sha256:7fb73f54abc68f74c68ea116d3d9fbb680eb368e514f898a1292328f5750a35a
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: YOFnLVUsbSfwM4duXfaa0KrkY61LZvdHVmB0C2oMtwvhne7YNTZ7SFU12LLr4hk7ui14iuWfzs/JZRBFgNhWAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2142
  event:
    id: 01M8AJREYFWS90RYRCQH9D9P6J
    sequence: 2142
    hash: 6c8addaa3415b6070af6044e12e0fb81960522e31d6da05db8df1d2927a103d9
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-07-workspace-index-eviction-bounds.md`.

## Evidence
`crates/bbt-mux/src/knowledge/workspaces.rs:307-313` keeps `roots`/`paths`/`bindings` as unbounded BTreeMaps and never removes roots. The 11-21 memory work (`400f24640`, `02345b0d1`) touched the GUI only.

## Do
Remove roots whose swarm/workspace is closed or released (that is the real bound; prefer it over an LRU), and add a hard capacity with eviction of least-recent idle roots as a backstop. Long-run test (open/close many swarms) with an eviction-off control that grows. `make land`.


## Report from seat:01M8A89QRMC06Y5D5M4QFDPZGB

[[Implement]] [[Verified]] Landed workspace index bounds at 972720fa8. After each of 32 real swarm closures the native fixture returned to one startup root and zero bindings. With eviction disabled it retained 33 roots and 32 bindings. The router reaps closed, archived, and released mounts, caps root slots at 64 and swarm bindings at 1,024 plus the legacy startup binding, and evicts the least-recent idle root. Removed the duplicate path map and redundant completed-mount payload. The focused check passed 27 tests. make land, the 1,006-model parity gate, and the source-bound receipt check passed. The worktree is clean and the evidence report is stored in the local KB.

```json
{
  "op_id": "01M8AJM63JF33XG4GS7Q79EYD8",
  "commit": "972720fa8c236232143046d793d70726768a649b",
  "report": "[[reports/2026-11-25-workspace-index-bounds.den]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux -E 'test(workspaces)' --retries 0",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Reap closed and archived swarm bindings and released checkouts",
        "reference": "crates/bbt-mux/src/knowledge/routing.rs:348 and crates/bbt-mux/src/logic/implementation_workspaces.rs:309. Native archive, shared-root, and release witnesses passed."
      },
      {
        "criterion": "Enforce root and binding limits and evict the least-recent idle root",
        "reference": "crates/bbt-mux/src/knowledge/routing.rs:24 and crates/bbt-mux/src/knowledge/routing/workspaces_retention_tests.rs:151. The held-watch witness proves cleanup keeps its slot."
      },
      {
        "criterion": "Long-run churn with a growing control",
        "reference": "crates/bbt-mux/src/knowledge/routing/workspaces_retention_tests.rs:121. 32 closures left 1 root and 0 bindings. Without eviction the fixture retained 33 roots and 32 bindings."
      },
      {
        "criterion": "Signed lean landing and formal verification",
        "reference": "make land exit 0 at 972720fa8c236232143046d793d70726768a649b. Formal evidence SHA-256 cd22aea375500e3cdd6ab07e0c678974438462f8a79a1d39df6e59a7ef36e1ac."
      }
    ]
  }
}
```
