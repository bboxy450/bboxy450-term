---
allowed_paths:
- crates/bbt-mux
- verification
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8ZJ1pn6ymQlhdn20cJBq2Rono7S6d/fLcgc3FK45CmU=
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
created: 2026-11-24T17:09:33.899Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
id: 01M878HGHX4HZ8KT2ZBGJMK22D
number: 1794292861627916
role: executor
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Logic indexes: bound Workspaces roots/paths/bindings with eviction'
type: task
updated: 2026-11-24T17:09:33.899Z
updated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
verify: cargo nextest run -p bboxy450-term-mux -E 'test(workspaces)' --retries 0
provenance:
  digest: sha256:793a77024b5593b48ff7ef9b65454c5d1705a2e7e0b586c4b8b7c0662e0affec
  frontmatter_digest: sha256:2b0f27661528cb9bb722f07685e7856a6b3e130bb52efea2b7415e306fbcfa60
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
  sig: HKvLEkmUreRX+7mLb+LsUqIlXtMAcZ4zl+Ie2tI6I+/Uowv0tCuH7s5IKUamDmjDL/Tp7TPKdppx5MtyBDjkDQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1851
  event:
    id: 01M878HH6Z7CF8TQ5F3YQWNPN0
    sequence: 1851
    hash: 72c25a61acac24c3fcb70be4f229ab7900540c25d67b9bcfa1d9d31ce1077ad4
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-07-workspace-index-eviction-bounds.md`.

## Evidence
`crates/bbt-mux/src/knowledge/workspaces.rs:307-313` keeps `roots`/`paths`/`bindings` as unbounded BTreeMaps and never removes roots. The 11-21 memory work (`400f24640`, `02345b0d1`) touched the GUI only.

## Do
Remove roots whose swarm/workspace is closed or released (that is the real bound; prefer it over an LRU), and add a hard capacity with eviction of least-recent idle roots as a backstop. Long-run test (open/close many swarms) with an eviction-off control that grows. `make land`.
