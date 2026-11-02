---
author: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
authorship:
  kind: seat
  process_generation: '2'
  public_key: 8G8gIL1MJqmoRkKikaIqqCqCFW1VHfzquBV0bXH7xs8=
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
commits:
- bd0d33abf934ea1341f2f78186bc41bf82998a4e
created: 2026-11-02T20:59:47.723Z
created_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
id: 01M6F0Z92EZ16J0EH7TRP54QJA
number: 5002222
provenance:
  digest: sha256:9704238d9be8ca31a897a9fe02b388a98813cc872985cce634dc3ee406a221d8
  frontmatter_digest: sha256:32f93535e619b6e1b3b6907c1c183c0df3be56bbb9a1783e4f88c2c74758ae55
  log: .bboxy/log/2026-11-02.jsonl#8312
  sig: hFyy262sW7Y1mURU8g1bGjcVEha6PFtGiKRYEwMJm5sI63UJEVlsqq13jvxop6pNRgqN9azDqCtM3OMyObZ3BA==
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
supersedes: '[[reports/2026-11-02-recovery-disk-budget-integration]]'
task: '[[tasks/2026-11-02-recovery-disk-budget]]'
title: Lean landing handles verified ignored-only integration suites
type: report
updated: 2026-11-02T20:59:47.723Z
updated_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
verify:
  command: make test-recovery-disk
  exit_code: 0
---
ROLE: SOLO. This append-only integration correction retains the recovery budget, 25-worktree cleanup, private 4/50-seat crash/replay, native chat/restart, smoke, license, signature and per-commit compilation evidence in the preceding reports.

The first canonical lean gate stopped before mux tests/formal: nextest selected the changed swarm_workdir integration binary, found its 28 deliberately ignored tests, and returned no-tests status 4. land-fast.sh returned gate failure 77; make returned 2. The changed native chat test had already passed via its explicit Make command. Raw failure: `/tmp/nyx-final-land.log`, SHA256 7cfcac495a6d6956deb1d6c703e06f53e4aa63e4fe82e17438c12713b95f2720  /tmp/nyx-final-land.log.

`scripts/land-fast.sh` now opts in to `--allow-ignored-only` only for lean test selections. `scripts/quarantine.py` consumes that internal flag before Cargo and accepts status 4 only after a successful nextest list confirms a nonempty integration target whose cases are all tests, all ignored, and skipped for the ignored reason. Only the lean planner's package/test/target-dir argument grammar is eligible. Explicit positional/expression/skip filters, empty or malformed listings, failed metadata queries, build failures and real test failures remain nonzero. Normal/full runner behaviour is unchanged. The log says compiled only; ignored tests are not claimed as executed.

`Makefile` adds `make test-lean-ignored`. It passed (exit 0): 13 focused controller/quarantine tests, then live nextest controls using the actual swarm_workdir binary. The target-dir wrapper prefix was exercised; all 28 ignored cases were explicitly reported compiled only. The empty expression filter still returned 4, and an invalid integration target still failed. Raw control log: `/tmp/nyx-lean-ignored-final.log`, SHA256 759835afc173000d493129ead28721eb5b6e41d2cfc3919c11f6ee145ed70111  /tmp/nyx-lean-ignored-final.log. `LAND.md` documents the distinction and the required explicit native Make check.

The real native chat test passed separately on the rebased source, as did four/fifty-seat recovery and private smoke (`/tmp/nyx-current-final-checks.log`). Both rebased commits passed cargo check --locked --workspace --all-targets (`/tmp/nyx-final-parent-check.log`, `/tmp/nyx-final-tip-check.log`). No Rust implementation, contract/control or durable recovery file was removed by this runner correction. The harness FoldedCard pair remains registered in verification/coverage.toml.

This record precedes the final signed containing commit and canonical gate rerun. The final task_report provides the actual gate exit, current-source formal receipt, signature result and landed SHA. No final successful push is claimed in advance.