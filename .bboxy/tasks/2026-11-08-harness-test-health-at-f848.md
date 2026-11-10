---
type: task
title: 'Harness test health at f848c03f: triage ~39 integration tests that fail outside their canonical Make env'
role: executor
allowed_paths:
- tests/
- scripts/land.py
- Makefile
verify: make test
id: 01M6YK6GPVXKPP8Q8VZEDDSG7N
status: open
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T22:06:58.090Z
created: 2026-11-08T22:06:58.090Z
number: 93377961366408
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:0d5662758347a4462d43db7693142278f49869b741733ef9bea40ea833088f91
  frontmatter_digest: sha256:01f085f30c62b53cf3604630e2903899b480be1a02115c3f3aca17d4228bb962
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: gdzYN0p9uhpdjI9k9RYpvT0CFmsB79m5V57BD5y3t6/sKV/qYxr/f1QfH3n4spPirwzzntdGi3vc2L/PJGQ7Dg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150845
  event:
    id: 01M6YK6K51CM3S32EPQ718QE25
    sequence: 150845
    hash: ddca831dcd9f4964f1d015241090c2a9418632c5dc573ab9ed2f011221723a83
---
ROLE: EXECUTOR

## Problem
When term's wide land ran the harness package's own integration tests at harness f848c03f (from term, without the harness Makefile environment), about 39 failed (list in /tmp/dream-q3-land.log; copy it before /tmp is cleared). Their real status on harness main under the harness repo's OWN canonical targets is unknown. Some are known: 3 gui_render known defects and 4 gui_rich KNOWN-DEFECT ignores. The dream_c1 tests need DREAM_C1_RUNNER.

## Goal
In the bboxy450-harness repository, run each failing family under its canonical Make target (with its required environment and features) on harness main. For each one:
- passes under its canonical target: record that it is environment-only;
- fails: fix it, or add a KNOWN-DEFECT ignore pointing to a specific fix task.
Then make sure the harness land gate runs every integration-test family with its required environment, so main cannot regress silently.

## Scope
Worktree, allowed paths and verify are set at dispatch.
