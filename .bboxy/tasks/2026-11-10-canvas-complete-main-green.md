---
type: task
title: 'Canvas C0-C1b complete: main green under the full signed gate'
role: solo
allowed_paths:
- Makefile
- crates/
- verification/
- .bboxy/
verify: make land-full
id: 01M73HFYSCK7H4DP5R8GGGD8QT
status: claimed
delegated_by: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
swarm: 01M6ABW0CX6MY0D5WGCSQ3ADTG
updated: 2026-11-10T20:13:26.261Z
created: 2026-11-10T20:13:20.444Z
number: 1794292861627478
created_by: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
updated_by: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
authorship:
  signer: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
  kind: seat
  public_key: JAtTGPtnMPcyMywVfuW1b8WB8e8Yk5NvbvMFpWjq+UE=
  process_generation: '2'
assignee: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
provenance:
  digest: sha256:0a058732650969f925ee70342f23651bddf5d3ec1fa4d688cd75782e3e22658c
  frontmatter_digest: sha256:ac36135cf912b1a2f530615bba17a80f0169b504cb87aa3ac58b49d72f1ed97c
  signer: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
  sig: +ldCs5Dw/7gIY6a4re5Mg4jzGuZsIOlZAQHz0MY0eInuMFa/G8v/voGQqbNARjHIzWEjHhauqt4cl2iyQDykBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151590
  event:
    id: 01M73HG4ZA47VGDQ79BFX9QH8N
    sequence: 151590
    hash: d8a220cfe0122b9d769ae2a10605016af01aad12705892e9ea1829a30650a9b1
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
