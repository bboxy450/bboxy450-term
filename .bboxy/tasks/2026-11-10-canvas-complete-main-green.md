---
allowed_paths:
- Makefile
- crates/
- verification/
- .bboxy/
assignee: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
authorship:
  kind: seat
  process_generation: '1'
  public_key: SH4gyl9T/JLUCgaQ+HiQWysI1jVaxJrpVWImHC0iz9k=
  signer: seat:01M878M9YVHKXHPDTT3VSC4VQ8
created: 2026-11-10T20:13:20.444Z
created_by: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
delegated_by: seat:01M878M9YVHKXHPDTT3VSC4VQ8
id: 01M73HFYSCK7H4DP5R8GGGD8QT
number: 1794292861627478
reconciliation:
- by: seat:01M878M9YVHKXHPDTT3VSC4VQ8
  disposition: implemented
  evidence:
  - commit 320d40e3f — canvas goldens use renderer tolerance, main green
  - tags v0.2.0 / crates-v0.2.0
  fresh_accepted: false
  kind: historical
  op_id: 01M878MD0TJT8AFEEED52BBVBF
  previous_status: claimed
  reason: Main green with canvas complete; released.
role: solo
status: done
swarm: 01M6ABW0CX6MY0D5WGCSQ3ADTG
title: 'Canvas C0-C1b complete: main green under the full signed gate'
type: task
updated: 2026-11-24T17:11:08.951Z
updated_by: seat:01M878M9YVHKXHPDTT3VSC4VQ8
verify: make land-full
provenance:
  digest: sha256:0a058732650969f925ee70342f23651bddf5d3ec1fa4d688cd75782e3e22658c
  frontmatter_digest: sha256:b9c4b12bcfcfcdaebd1a586320e1ee7b356410a905f2a09293d26db85f5c4aab
  signer: seat:01M878M9YVHKXHPDTT3VSC4VQ8
  sig: K1JrSjeGTSNXtTSGyTQDInWcagfqaWP1ZYG5CwoSIe0Au0P1kLiceggRWiIuS5UnRrBa1iES8WNrlr8bUjCnBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1903
  event:
    id: 01M878ME0Y3ZVJF4DDT20S6J6M
    sequence: 1903
    hash: 7e7a635f57d53f8fa0ffb7f17919f383a74944ab50fd83051b9d066a9df68160
---
ROLE: SOLO (operator, 2026-11-10: "make sure its all on main and tests and full land passes then you can close your own seat with canvas being completed ... do work yourself, no executors").

## Goal

Canvas C0, C1a and C1b ([[canvas]]) are on `origin/main` (`733358c87..32d66d325`), but they landed past a red lean gate. Make `main` pass the full signed gate, so the canvas work is verified in the whole suite and not only in its own crates.

## Scope

Fix what makes `make land-full` red on current `origin/main`, smallest change first. A stale test follows the product source that moved; real product breaks get product fixes. Do not weaken a check to get green. Record each failure with its cause.

## Acceptance

- [ ] `make land-full` exits 0 on the pushed commit; its formal receipt is current.
- [ ] Each fixed failure has `file:line — cause`, in the report.
- [ ] The report states what was removed.
- [ ] The canvas tasks C0, C1a, C1b are done; their seats are closed and worktrees released.

## Verification

`make land-full`
