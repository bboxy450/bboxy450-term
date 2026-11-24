---
allowed_paths:
- crates/bboxy450-harness
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8ZJ1pn6ymQlhdn20cJBq2Rono7S6d/fLcgc3FK45CmU=
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
created: 2026-11-24T17:09:30.072Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
id: 01M878HCFAH78PJHXBS5ZRHABC
number: 1794292861627912
role: executor
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Harness: clear the ignored-test debt (23 QoS-deferred dream tests, bill/group-proof race, 4 gui_rich, profile-switch SessionClosed)'
type: task
updated: 2026-11-24T17:09:30.072Z
updated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
verify: make -C crates/bboxy450-harness test
provenance:
  digest: sha256:461b95b499da1a6fbb1aab56f8c4b3d40d3b357b34ae1283d4f8aa51a722b188
  frontmatter_digest: sha256:6de0800680a64fa43c3739e55ec188260af68a97adbabd40a54f8c52a13267b3
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
  sig: ryf9CkxvvWfr0bMmYkmdQRt+0JRaX9dpIZ88i7L6TZJC1/OMYL8KDvtEESVqGRHVfS+hCUV/6cWwUrEFW5RwCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1843
  event:
    id: 01M878HDEZHYZ62JQKHVTCBFW6
    sequence: 1843
    hash: 8c99176c5f0ca025bd5f4d73b878a42059ef42045be713225fae1477f00f1c13
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-08-reenable-qos-deferred-dream-tests.md`, `tasks/2026-11-08-harness-known-bill-group-proof-race.md`, `tasks/2026-11-08-harness-gui-test-known-defects.md`, `tasks/2026-11-08-gui-profile-switch-session-closed.md`.

Work in the harness submodule (its own repo, bboxy450/bboxy450-harness); land it there, then `make bump-harness` in term.

## Evidence (harness origin/main e9efa3b)
- 23 `load-sensitive ... re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp` ignores: `src/dream/resolve.rs` 9, `tests/dream_c1.rs` 7, `tests/dream_read.rs` 6, `fixtures/dream-bootstrap/process.rs` 1. That task is closed.
- Ignore at `src/dream/resolve.rs:4861`: known bill vs group-proof upgrade ordering race.
- 4 `gui_rich` ignores at `tests/gui_rich.rs:238,295,550,1658` (mermaid figure, reasoning prose, live tool tail, MCP raw cards).
- `gui_socket_start_switch_aba_resume_and_cleanup` (`tests/support/seat_socket.rs:669`, wait :588) intermittently misses the old session's `SessionClosed`; no fix commit.

## Do
1. QoS ignores: tests must not time the machine (`53a9e215f` stance). Rewrite each to assert ordering/outcome instead of wall time, or delete it if that leaves nothing. Remove all 23 ignores.
2. Fix the bill/group-proof race with a deterministic test for both orders; remove the ignore.
3. Fix the 4 gui_rich tests and remove their ignores.
4. Root-cause the missing `SessionClosed` (or prove it gone with a load run of 20 iterations, retries 0).
5. Land in the harness repo, `make bump-harness`, `make land` in term.
