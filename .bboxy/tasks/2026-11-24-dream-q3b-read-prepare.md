---
allowed_paths:
- crates/bboxy450-harness-wake-up
- crates/bbt-mux
- verification
- .bboxy/interfaces
- scripts/test-dream-c1.py
assignee: seat:01M8AJX1D7Q52D249AQZ71GSH5
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
closed:
  by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  op_id: 01M8AYKVSCPQAWGT5K31V4VC7G
created: 2026-11-24T17:09:34.732Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M878HHAHFJVH8E6H7SDHEZBM
number: 1794292861627917
report:
  commit: b151d124106554e67e584125f8c79d9e77384284
  op_id: 01M8AYJRJDMDVF7ZWZPRP0J8CY
  report: '[[reports/2026-11-25-dream-q3b-landing.den]]'
  reported_by: seat:01M8AJX1D7Q52D249AQZ71GSH5
  summary: Dream Q3b landed on origin/main in signed commits 95ec19e7a, e2dd073b4 and b151d1241. `make test-dream-mcp` exited 0 with 37 passes, `make test-boundaries` exited 0 with 41 passes and `make land` exited 0 with native/TLC parity across 1,008 models. Read preparation shares the existing spawner, and same-seat reported-only feedback recovers through the original operation. Removed the InspectC1-only guard, hardcoded argv, blanket feedback refusal and duplicate literal tuple. Moving the read-purpose witness to the Dream read suite resolved the first landing attempt’s empty quarantined selection. The prior `make test-dream-c1` exited 2 and remains with [[tasks/2026-11-25-dream-c1-gate-repair.md]].
  verify_revision: 1
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Dream Q3b: read-prepare/1 transport through the existing spawner'
type: task
updated: 2026-11-26T03:33:03.170Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: make test-dream-mcp && make test-boundaries
verify_result:
  command: make test-dream-mcp && make test-boundaries
  evidence:
  - criterion: Shared read-prepare transport and process refusals
    reference: crates/bboxy450-harness-wake-up/tests/dream_read.rs:45. Exact Start, wrong purpose, probe, and actual child reap. make test-dream-mcp exit 0, 37 passed. target/dream-q3b/rebased-verify.log.
  - criterion: Same-seat offer provenance and original-operation feedback recovery
    reference: crates/bbt-mux/src/mcp/journal.rs:248 and crates/bbt-mux/tests/dream_read.rs:326,407. Amended verification exit 0.
  - criterion: Process boundaries
    reference: make test-boundaries exit 0, 41 passed. target/dream-q3b/rebased-verify.log, SHA-256 72b2db31ad14c069d3fc39422ca4e15cff863500100b79705aae34622182d808.
  - criterion: Formal model and broken control, signed landing
    reference: make land exit 0. Current-source formal receipt a2869093e5d6021c52c786093d576da60f15aa66eeeed9a737d2ebd8c2ed8602, 1008 models. target/dream-q3b/land.log:1269 and :1270. git verify-commit and ancestry checks exited 0.
  exit_code: 0
verify_revision: 1
provenance:
  digest: sha256:e3fc2f5e5ba2d22359e651634ad36bfce27b029ca2a9c9377a8ae83eff94c67c
  frontmatter_digest: sha256:fd19e66a8c7549e5ad86b7bf4f191b0357e7b5e691f1cba4eb56b8e70fe2416e
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: 6Tf9nkqdy5vHqPnSiO4wkRRj031lPVHF/dKJ7OEEeUgcOu2TQ6gPQK/FdErOIfDHf7yadPMVQDJW1jO4Fe1zBA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2193
  event:
    id: 01M8AYKWWGC4TYKW1S6HP5TZ9T
    sequence: 2193
    hash: 29a0e2bfb6e8da2455c37a729de2cd6e50af0b501a8f10dba59e13d3e71e7357
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-08-dream-q3b-readprepare-transport.md` (full brief there).

## Evidence
`crates/bboxy450-harness-wake-up/src/dream_c1.rs:2159-2166` still refuses any purpose but `InspectC1` and hardcodes the `inspect-c1` argv. No Q3b commits since 11-08.

## Do
Follow the legacy brief: add the read-prepare/1 purpose through the existing spawner (no second spawner), move the producer pin, record offer provenance in `mcp/journal.rs`, include Dream in `operation_status`. Update the ICD and its model/broken control. `make land`.

## Oracle ruling 2026-11-25 (amendment): approve the current worker tuple, then land
Verified on origin/main 972720fa8: `scripts/test-dream-c1.py:39-43` approves only `(2ce4359…, e003fd3d…, 2b85c711…)`, but the pinned harness gitlink is `f9f0b4a666ea4a9fdd49b71302b4c1e9be86229c` and the toolchain is rustc 1.100.0. Main's `make test-dream-c1` therefore refuses at `:1019` exactly as yours does. The approval drifted; your change did not cause it. The comment at `:36` makes the Oracle the approver, and this is that approval.

1. In `scripts/test-dream-c1.py`, set `APPROVED_NATIVE_BUILD` to the tuple your gate measured: source `f9f0b4a666ea4a9fdd49b71302b4c1e9be86229c`, compiler `408acb044ad9f92dcbaeb61386ef4dc1c39dd1c92d20b02c82b694a783c4b809`, binary `87794659c83500d720fd46454c4a80ca1e9c30f436a896c55762165a50ed3d75`. Done in 8626973219.
2. Keep one copy of the tuple. Done: the runtime's duplicate literal is removed.
3. Out of scope: the live-lessons selected-publication exporter and the runtime proof-tool approval. They are follow-ups.

## Oracle ruling 2026-11-25 (second amendment): split the C1 gate out, land Q3b now
This is the second refusal from `make test-dream-c1`, and both refusals are rot on main, not Q3b. The stale tuple hid every later layer. The gate could not run on main, so it never proved anything about main. The remaining layers are a zero-test nextest selection of an ignored producer exporter, image replacement by the release test build, and a runtime probe Deadline before birth. They are a separate task: `tasks/2026-11-25-dream-c1-gate-repair.md`. It is not yours to fix here. The gate's own standalone approved-worker probe (`target/dream-q3b/c1-approved-gate.json`) admitted the first launch and settled Hello and reap.

Verify is now `make test-dream-mcp && make test-boundaries`. Those are the witnesses that cover this diff, plus the formal parity you already ran.
1. Rebase onto origin/main and run the new verify once on the final commit. Both must exit 0.
2. `make land` (it runs `make test-formal` because `verification/` changed). Do not touch `make test-dream-c1` or its script beyond the tuple commit.
3. Report once, landed: commits on origin/main, both verify exits, the land exit. State `make test-dream-c1`'s exit honestly as the open follow-up. The report stays a draft; a `knowledge_commit` refusal on `reports/` is expected.


## Report from seat:01M8AJX1D7Q52D249AQZ71GSH5

Dream Q3b landed on origin/main in signed commits 95ec19e7a, e2dd073b4 and b151d1241. `make test-dream-mcp` exited 0 with 37 passes, `make test-boundaries` exited 0 with 41 passes and `make land` exited 0 with native/TLC parity across 1,008 models. Read preparation shares the existing spawner, and same-seat reported-only feedback recovers through the original operation. Removed the InspectC1-only guard, hardcoded argv, blanket feedback refusal and duplicate literal tuple. Moving the read-purpose witness to the Dream read suite resolved the first landing attempt’s empty quarantined selection. The prior `make test-dream-c1` exited 2 and remains with [[tasks/2026-11-25-dream-c1-gate-repair.md]].

```json
{
  "op_id": "01M8AYJRJDMDVF7ZWZPRP0J8CY",
  "commit": "b151d124106554e67e584125f8c79d9e77384284",
  "report": "[[reports/2026-11-25-dream-q3b-landing.den]]",
  "verify_result": {
    "command": "make test-dream-mcp && make test-boundaries",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Shared read-prepare transport and process refusals",
        "reference": "crates/bboxy450-harness-wake-up/tests/dream_read.rs:45. Exact Start, wrong purpose, probe, and actual child reap. make test-dream-mcp exit 0, 37 passed. target/dream-q3b/rebased-verify.log."
      },
      {
        "criterion": "Same-seat offer provenance and original-operation feedback recovery",
        "reference": "crates/bbt-mux/src/mcp/journal.rs:248 and crates/bbt-mux/tests/dream_read.rs:326,407. Amended verification exit 0."
      },
      {
        "criterion": "Process boundaries",
        "reference": "make test-boundaries exit 0, 41 passed. target/dream-q3b/rebased-verify.log, SHA-256 72b2db31ad14c069d3fc39422ca4e15cff863500100b79705aae34622182d808."
      },
      {
        "criterion": "Formal model and broken control, signed landing",
        "reference": "make land exit 0. Current-source formal receipt a2869093e5d6021c52c786093d576da60f15aa66eeeed9a737d2ebd8c2ed8602, 1008 models. target/dream-q3b/land.log:1269 and :1270. git verify-commit and ancestry checks exited 0."
      }
    ]
  }
}
```
