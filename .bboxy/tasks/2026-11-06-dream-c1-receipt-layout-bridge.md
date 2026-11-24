---
allowed_paths:
- crates/bbt-mux/src/registry/dream_c1.rs
- crates/bbt-mux/src/logic/dream_c1.rs
- scripts/test-dream-c1.py
- Makefile
- crates/bboxy450-term/tests/dream_c1.rs
- verification/
- .bboxy/interfaces/dream-term-c1.md
- .bboxy/reports/
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-06T20:28:48.103Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6S8SCR6ZS4Q2GZVGC19AK76
number: 93377961366112
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - 'successor: tasks/2026-11-24-term-stale-known-defect-ignores.md step 3'
  fresh_accepted: false
  kind: historical
  op_id: 01M878K6TJT4QZJ3QCX9VYT4CF
  previous_status: open
  reason: Carried into the live term swarm.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Align producer/term C1 runner receipt layout so the joined run gets past Hello into Store/export
type: task
updated: 2026-11-24T17:10:29.685Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: make test-dream-c1 && make test-formal-contracts
provenance:
  digest: sha256:6c52cfc0679a2ab541ca162ab34dda85b430d0f60939fde36dfb3c80ac51353b
  frontmatter_digest: sha256:3094ae6f71e3ff19da444da4f356bf41f580589a0d4cc7b4df4cb7bd0bc35635
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: hQZIFqfEVSAzyHYM8R740NvwXqOz5JfXmdN/ejuHlbITL5rD2E9g9ZHD+rXCFVYOq0dYSg7olDmudRDO9lBbBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1865
  event:
    id: 01M878K7PTGRF6H23X8TVBP4C4
    sequence: 1865
    hash: 0bfb9bdbf79a2c2526183f38e5096b82a2bedd3d70393d735dff4532eb2b01bc
---
# C1 joined run stops at the receipt layout

Context: the C1 consumer landed under a source-only override. Its ONE run on signed 8d9cb55f passed the first-launch admission (`--help`, exit 0 in 295.5 ms) and **passed Hello on the original 250 ms clock** (entry 4.3–6.9 ms after spawn returned). Then `make test-dream-c1` failed before the producer's Store/export job. The pinned producer's `approved_runner()` (harness `tests/dream_c1.rs:1218`) expects `bootstrap-support.txt` and `hashes.txt` beside `DREAM_C1_RUNNER`, but term provisioning supplies its private `.dream-c1.json`. Evidence is `command-02.log` in `/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-3tubkf94/`.

## Required

1. **Decide the one canonical layout.** Read the ICD [[interfaces/dream-term-c1]]. Either term prepares the producer's expected files from its verified private receipt (a derived, hash-bound projection; never a second source of truth), or the ICD names `.dream-c1.json` and the producer side changes, which is a harness-repo change and therefore a separate task. Prefer the term-side projection, since the producer is pinned and accepted. Update the ICD line so the boundary is written down.
2. **One new joined run** on the corrected layout: admission, then Hello, then Store/export, then the full four-job/frame/heap/200-call gate. Old failed runs stay recorded as failed, with no retry loop.
3. **Model and tests:** the projection must be byte-identical to the receipt (with hash checks), plus a broken control in which a missing or stale projection file fails closed.

Lean landing. Write the report `reports/2026-11-06-dream-c1-receipt-layout.md` and call `task_report` once.
