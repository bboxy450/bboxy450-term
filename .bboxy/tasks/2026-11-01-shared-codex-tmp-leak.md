---
allowed_paths:
- scripts/e2e/shared_codex.py
- scripts/
- Makefile
- .bboxy/runs/
- .bboxy/reports/
assignee: seat:01M6ABQRB8SJNZHP6NRHQD3BC8
authorship:
  kind: seat
  process_generation: '1'
  public_key: dNWbVeHRHyt/TnV6QHFyDXtWIj+UElZjvaWtEgWhcFg=
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
created: 2026-11-01T11:11:21.464Z
created_by: seat:01M6ABH71TVWGZ621NTD701PSH
delegated_by: seat:01M6ABH71TVWGZ621NTD701PSH
id: 01M6BCX3AZFSSZS5GQVATE2Y2J
number: 1796
provenance:
  digest: sha256:c8b93b237771a636bdd7d50c5c9c399c5521b788af3103ab40a5e6bc678c1a8c
  frontmatter_digest: sha256:1dd31d889dce109809a521f4dfd1c5d16c78dad9f47cf0c73f2c6ae7ae7f2cfa
  log: .bboxy/log/2026-11-01.jsonl#7348
  sig: nIfopsUC5crt//RZgH3uKSI9JRfs5z+3KaFnXg9gImJnoMbL6KEj7aJknOx8+i2MDu5SY1WIEd2WNXHR/WTlDw==
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
role: executor
status: claimed
swarm: 01M649102FBS60QWNPZS5VAGF9
title: shared_codex e2e must not leak its /tmp fixture root
type: task
updated: 2026-11-01T11:11:26.056Z
updated_by: seat:01M6ABH71TVWGZ621NTD701PSH
verify: make test
---
# shared_codex e2e must not leak its /tmp fixture root

ROLE: EXECUTOR. Create worktree `~/mydev/bboxy450-term-tmpleak`, branch `shared-codex-tmp-leak` from origin/main. Only this worktree.

## Problem
`scripts/e2e/shared_codex.py:78` creates `/tmp/bbt-shared-codex-*` (≈250 MB–1.2 GB each: private HOME, Codex packages, run dir). On 2026-11-01 the Oracle found 54 leftovers, 13 GB in total. They kept recovery tests under their 10 GiB free-disk floor and blocked landings (#572). The `finally` at :491 writes `cleanup-owner.json` and removes only `home/.codex/packages` (:523), so the root stays behind.

## Required outcome
1. A passing run removes its whole fixture root, after stopping only the PIDs it started and recorded.
2. A failing run keeps at most the diagnostics needed (logs, `cleanup-owner.json`) and drops the bulky package and home trees. Say what is kept.
3. A bound on accumulation: a new run reclaims earlier `bbt-shared-codex-*` roots whose recorded owner PIDs are all dead. Never touch a root whose owner is alive, nor anything not created by this script.
4. Regression: a fixture-level test showing that success leaves no root, that failure leaves only the small retained set, and that a stale root with dead owners is reclaimed while one with a live owner is kept. Include a broken control: with the cleanup removed, the test fails.

Allowed paths: `scripts/e2e/shared_codex.py`, new tests under `scripts/`, `Makefile` (a focused target only), `.bboxy/runs/`, `.bboxy/reports/`.

## Verify
The focused target and `make test` on the exact committed source. Land with `sh scripts/land.sh --gate-log target/ci-local.log`. Write the full report in `.bboxy/reports/` and add a `Report:` trailer.

## Communication
Quiet by default: one task_report at the end. Message the Oracle mid-task only for a blocker that stops all work, in three lines or fewer.
