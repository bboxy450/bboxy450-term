---
allowed_paths:
- crates/bbt-mux
- crates/bboxy450-term/tests/formal_contracts/watcher_model.rs
- verification/contracts.toml
- verification/corpus.toml
assignee: seat:01M8A0C1ZYSTMQQ6WWZRYQ31BM
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
closed:
  by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  op_id: 01M8ABF835MDHGKHX8QKJ16RJ6
created: 2026-11-24T17:09:33.121Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M878HFP4EPKJQPF2PRQ2M99G
number: 1794292861627915
report:
  commit: bf7fd1d4e18a74828cbbdee0bba7a2899e523d36
  op_id: 01M8ABEBB24PDDMV7C7HQ4GEMP
  report: '[[reports/2026-11-25-kb-watch-rescan-fallback.den]]'
  reported_by: seat:01M8A0C1ZYSTMQQ6WWZRYQ31BM
  summary: '[[Implement]] Landed the five-second KB metadata rescan and `knowledge_status` freshness timestamps in signed commit bf7fd1d4e. All 18 watcher tests passed, including injected event loss and the disabled control. Removed the provenance worker''s directory-stamp polling and full-hint forwarding. The initial formal run timed out on an unrelated model. Hash-validated reuse recovered the gate, all three watcher models matched native/TLC verdicts, and `make land` exited 0 on the clean rebased source.'
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'KB index: bounded rescan fallback and freshness status when FSEvents lags'
type: task
updated: 2026-11-25T21:58:29.201Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: cargo nextest run -p bboxy450-term-mux -E 'test(watch)' --retries 0
verify_result:
  command: cargo nextest run -p bboxy450-term-mux -E 'test(watch)' --retries 0
  evidence:
  - criterion: Dropped hints recover incrementally and the disabled control stays stale
    reference: crates/bbt-mux/src/knowledge/routing_tests.rs:19 and target/kb-watch-verify.junit.xml (18/18 nextest tests).
  - criterion: Knowledge status exposes watcher and rescan timestamps
    reference: crates/bbt-mux/tests/mcp/den_records.rs:291 and target/kb-watch-land.log (134/134 MCP integration tests).
  - criterion: Formal watcher family and independent controls
    reference: target/kb-watch-formal-reuse.log and landed formal receipt 30c44afff7600ba24c04a710f532d45eac8761eb7e3417beb24317a59977f384 (1,004 models, exit 0).
  - criterion: Signed and landed
    reference: target/kb-watch-land.log (exit 0, LANDED bf7fd1d4e18a74828cbbdee0bba7a2899e523d36). git verify-commit and git merge-base --is-ancestor HEAD origin/main both exited 0.
  exit_code: 0
provenance:
  digest: sha256:ed1e2773beb855a13f53791f770589dc60bc3a405507f7d6c6d8acd651ab0926
  frontmatter_digest: sha256:67e1effc6fbb6f5a115c26dc484cd48bc89bc1c8ae1a0cd736a344339df2fb1a
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: KXncWdLNNi00f7sKuWDNI6VJeoNGykd+60s3bZhbvUXmBQJLQKSeFweuZ+ze/+71JJZNAGdgyZTgc3f8989ZCA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2114
  event:
    id: 01M8ABF96GR3NGRRT388Z8PGM8
    sequence: 2114
    hash: c6b21f41f8a4f97f130fc5b6d8613e1cfaa31ce0acd6293faf8d1dd45257cb18
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-06-filewatch-fsevents-backlog.md`.

## Evidence
`index/watch.rs` has only a debounce (QUIET 200 ms, MAX_WAIT 1 s); no periodic/backlog rescan and no freshness status. The two flaky tests were deleted in `f2d8e4076`; `53a9e215f` added a test-side `rescan` helper only. `2026-11-22-kb-index-external-adds` (f0365a5b1) fixed git-added records, not this.

## Do
Add one bounded incremental rescan (about every 5 s, mtime/size diff, no full re-read) that catches events FSEvents dropped, and expose index freshness (last event, last rescan) in `knowledge_status`. Test with a watcher that drops events (inject at the watcher boundary) plus a broken control with the rescan disabled. Update the relevant model in `verification/` if the watch lifecycle is modelled. `make land`.

Amendment (Term Oracle, 2026-11-25): allowed_paths now include the watcher model (`crates/bboxy450-term/tests/formal_contracts/watcher_model.rs`), `verification/contracts.toml` and `verification/corpus.toml`, so the dropped-hint model and disabled-rescan control land with the code. Run `make test-formal` before `make land`.


## Report from seat:01M8A0C1ZYSTMQQ6WWZRYQ31BM

[[Implement]] Landed the five-second KB metadata rescan and `knowledge_status` freshness timestamps in signed commit bf7fd1d4e. All 18 watcher tests passed, including injected event loss and the disabled control. Removed the provenance worker's directory-stamp polling and full-hint forwarding. The initial formal run timed out on an unrelated model. Hash-validated reuse recovered the gate, all three watcher models matched native/TLC verdicts, and `make land` exited 0 on the clean rebased source.

```json
{
  "op_id": "01M8ABEBB24PDDMV7C7HQ4GEMP",
  "commit": "bf7fd1d4e18a74828cbbdee0bba7a2899e523d36",
  "report": "[[reports/2026-11-25-kb-watch-rescan-fallback.den]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux -E 'test(watch)' --retries 0",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Dropped hints recover incrementally and the disabled control stays stale",
        "reference": "crates/bbt-mux/src/knowledge/routing_tests.rs:19 and target/kb-watch-verify.junit.xml (18/18 nextest tests)."
      },
      {
        "criterion": "Knowledge status exposes watcher and rescan timestamps",
        "reference": "crates/bbt-mux/tests/mcp/den_records.rs:291 and target/kb-watch-land.log (134/134 MCP integration tests)."
      },
      {
        "criterion": "Formal watcher family and independent controls",
        "reference": "target/kb-watch-formal-reuse.log and landed formal receipt 30c44afff7600ba24c04a710f532d45eac8761eb7e3417beb24317a59977f384 (1,004 models, exit 0)."
      },
      {
        "criterion": "Signed and landed",
        "reference": "target/kb-watch-land.log (exit 0, LANDED bf7fd1d4e18a74828cbbdee0bba7a2899e523d36). git verify-commit and git merge-base --is-ancestor HEAD origin/main both exited 0."
      }
    ]
  }
}
```
