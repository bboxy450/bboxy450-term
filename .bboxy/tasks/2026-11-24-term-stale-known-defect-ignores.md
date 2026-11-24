---
allowed_paths:
- crates/bbt-mux
- Makefile
- scripts/test-dream-c1.py
- .bboxy/interfaces
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8ZJ1pn6ymQlhdn20cJBq2Rono7S6d/fLcgc3FK45CmU=
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
created: 2026-11-24T17:09:28.832Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
id: 01M878HATACDJVCWN38AESJKGB
number: 1794292861627911
role: executor
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Term: remove #[ignore]s that point at closed dream-c1 budget task; drop deleted-test Makefile filter; settle dream receipt layout'
type: task
updated: 2026-11-24T17:09:28.832Z
updated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
verify: make test-dream-c1 && cargo nextest run -p bboxy450-term-mux --run-ignored all -E 'test(dream_c1)'
provenance:
  digest: sha256:57df924e81e72d1ac0e8d0e321806d0a8ad5e176236c528bbd20d5f54a2e1500
  frontmatter_digest: sha256:f379cc7babf54221b756e57ea564299487e08452b43c7f257af88c9f3bdab52e
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
  sig: uhbRA/BMTaXzHIBYTnu58gxMmYJabG2QRqCFddK5XTg6vHNhTGRx/RnJhQIxj3Cl5GnT6uXDEnFiTeK7Y5fOAg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1841
  event:
    id: 01M878HCD1V0E58EJ9JKE0A36B
    sequence: 1841
    hash: fb2413db9b7cbe3246f01032ab9e49f41ce7a4d105729e6200b66aa1cf7d36a8
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-07-dream-c1-native-unit-budget-overruns.md`, `tasks/2026-11-12-dream-c1-measure-not-refuse.md`, `tasks/2026-11-06-dream-c1-receipt-layout-bridge.md`, `tasks/2026-11-12-remove-flaky-tests.md`.

## Evidence (triage on origin/main 911587f2f)
- `ff8109e68` replaced the 1 ms refusal with a 50 ms OVERRUN (`crates/bbt-mux/src/logic/dream_c1.rs:54`, decision d1794292861627496), but 11 tests still carry `KNOWN DEFECT ... fix: tasks/2026-11-07-dream-c1-native-unit-budget-overruns` ignores: `logic.rs:15535,15543,15570,15793,15944`, `logic/dream_c1.rs:10635,10645`, `registry/dream_c1.rs:2764,3154,3232,3772` (line numbers approximate; grep the task slug).
- `Makefile:2887` still filters the deleted test `task_migration_250_records_authenticated_collision_restart_and_replay`.
- The harness test reads `bootstrap-support.txt`/`hashes.txt` beside `DREAM_C1_RUNNER` (harness `tests/dream_c1.rs:1473-1476`); term writes `.dream-c1.json` (`scripts/test-dream-c1.py:1030`). Possibly moot.

## Do
1. Remove each of the 11 ignores; tests that pass under the 50 ms threshold stay, tests that only proved the 1 ms refusal are deleted. No new ignores.
2. Delete the stale Makefile filter.
3. Run `make test-dream-c1` once. If the receipt layout mismatch fails it, project the receipt into the producer layout and write the layout into `.bboxy/interfaces/dream-term-c1.den`; if it passes, say so and change nothing.
4. `make land`.
