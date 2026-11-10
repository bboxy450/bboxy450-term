---
type: report
title: Formal evidence compression, deduplication and verified retention
task: '[[tasks/2026-11-06-formal-evidence-retention]]'
author: seat:01M6SEB3WJBTKYM886SJSSY7AE
created: 2026-11-06T22:55:19.685Z
commits:
- 6df528c6d16b2f8c4b8fd9666f1b26c06a3cd51a
verify:
  command: python3 -I scripts/test-landing.py && make check-formal-evidence
  exit_code: 0
id: 01M6SH5NTCGXMRX12VWCDGT8MT
updated: 2026-11-06T22:55:19.685Z
number: 93377961366151
created_by: seat:01M6SEB3WJBTKYM886SJSSY7AE
updated_by: seat:01M6SEB3WJBTKYM886SJSSY7AE
authorship:
  signer: seat:01M6SEB3WJBTKYM886SJSSY7AE
  kind: seat
  public_key: f7zYL1YcJuDpV7jtxg7OiXTHA+qi/zaI6Pwt1rLvDHQ=
  process_generation: '1'
provenance:
  digest: sha256:056cdc165897589dcf3060887355c8b093794ae2ce57c078b4dbe721de8d738e
  frontmatter_digest: sha256:12df233ae4638474072f8a129096973b40efd717ffa678682361eee095b42687
  signer: seat:01M6SEB3WJBTKYM886SJSSY7AE
  sig: dNhGoMkavitBjEjakhrkeYWaBXkGPJgnPAEqCjkEo3yeBHWuboHlrbqL3yJ1quoEJzXATjCDiHrqV7AvGqCICg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149951
  event:
    id: 01M6SH5PR50CEWXHR350GMQEPN
    sequence: 149951
    hash: 107276af4b7795fc5c0e2b446496fbf4966100dd8262758da0d90bf7f60d54af
---
Landed signed commit `6df528c6d16b2f8c4b8fd9666f1b26c06a3cd51a` on `origin/main`. The writing worktree is clean.

## Changes and acceptance evidence

| Requirement | Mechanism and evidence |
| --- | --- |
| Live receipts remain available | `scripts/formal.py:495` pins receipts for origin/main and all extant worktree HEADs, plus evidence digests/source SHAs cited by unaccepted reports. Unknown or closed-without-acceptance tasks remain pinned. `scripts/test-landing.py:881` exercises worktree/main pins, pending/closed reports, accepted reports and newest-N retention. |
| Verified monthly retention with dry run | `scripts/formal.py:581` streams one tar.zst per UTC receipt month and a SHA-256 member/archive manifest. Frame integrity, exact member count and every member hash must pass; files, manifest and directory entries are fsynced before removal. Monthly appends preserve prior members and deduplicate compressed graphs with tar hardlinks. `scripts/formal.py:660` refreshes pins and rehashes originals before removal; only maintenance takes its nonblocking lock. `Makefile:1700` exposes KEEP=10 and DRY_RUN=1. |
| Corruption preserves originals | `scripts/test-landing.py:938` corrupts an existing archive and requires the new cold originals to remain. `scripts/test-landing.py:949` injects failed new-archive verification and requires no receipt deletion. The monthly append test also restores both graph members and checks their bytes. |
| Dry run changes nothing | `scripts/test-landing.py:957` compares every store file's bytes and mtime and requires no archive directory to appear. |
| Automatic background maintenance | `Makefile:822` schedules after successful lean land; `scripts/landing.py:1818` covers land.sh. `scripts/formal.py:704` uses the 5 GiB physical-byte budget (FORMAL_EVIDENCE_BUDGET_BYTES override), detached taskpolicy -b / nice 19, and a private prune.log. Maintenance never gates publication. The landing fixture checks both budget branches, QoS arguments and detachment. |
| Compressed, content-addressed graphs | `scripts/formal.py:399` verifies decoded SHA-256 round trips and hardlinks graphs to by-model/graphs/<decoded-sha256>.zst. `scripts/formal.py:324` and `:832` bind decoded graph bytes into new receipts. Legacy plain files remain readable. `scripts/test-formal.py:360` proves reuse through decoding and corruption refusal; `scripts/test-landing.py:967` proves compression, byte recovery and shared inodes across receipts; `:985` checks staging/live-link custody during orphan cleanup. |
| Checker passes after pruning | Ran the real prune target, then the exact assigned verification command successfully. Also ran the canonical Make checker with TARGET_DIR=target/formal-archive-check, whose formal directory points at the compressed retained receipt. |

Updated LAND.md and verification/README.md describe the policy, codec and verification limits.

## Verification

All final commands below exited 0, unattended:

- `make land`: strict affected-app Clippy, binary build and nextest: **447 passed, 12 skipped**. Its mandatory `make test-formal FORMAL_REUSE=1` checked **570 native/Java TLC model graphs and verdicts, 34 crates and 212 contracts**, including broken controls. It printed LANDED for the commit above.
- `python3 -I scripts/test-landing.py && make check-formal-evidence`: **181 fixtures, failures=0**, 82.973 seconds, followed by a valid exact-source formal receipt.
- `make prune-formal-evidence`: **keep=3, cold=0** in the real shared store. Destructive cold-receipt removal is exercised in private repositories by the landing fixtures.
- `make check-formal-evidence TARGET_DIR=target/formal-archive-check`: passed against the compressed retained graphs after pruning.
- `python3 -I scripts/test-formal.py`: **23 tests passed** on the final source.
- `make prune-formal-evidence DRY_RUN=1`: exit 0; initial shared-store observation keep=2, cold=0.
- `make test-licenses`: 34 workspace crates; policy/source notices and negative fixtures passed.
- `git verify-commit HEAD`: good Ed25519 signature from Boxy <bboxy450@gmail.com>, key 9B07E849480C68328CA20684844ECC0587DE7AB0. `git diff --check` and final `git status --porcelain` were clean.

Actual retained output: **2,130,747,831 bytes** of regular exported model files became **125,392,112 logical receipt bytes**, a **16.99× reduction** (the receipt also includes evidence.json). Unique retained inodes total **124,574,945 bytes**, or **17.10×**. Measurement is recorded in target/formal-retention-summary.json.

Formal receipt SHA-256: `3c10919b4c774134cab758c68a5479beea82b88690cf7ab29c19b115c56af618`.
Retained path: `/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/3c10919b4c774134cab758c68a5479beea82b88690cf7ab29c19b115c56af618/evidence.json`.
Full landing log: target/formal-retention-land.log, SHA-256 `2a6c78c0b50252e9996e48d3f95f5cf81686511df839daf1c509379a06566e56`.
Additional local logs: target/formal-compressed-check.log and target/formal-storage-tests.log.

## Limits and remaining work

No remaining failed checks. The six extended moving-main/recovery fixtures selected only by --all, full make test and UI parity/smoke were not requested by this lean tooling assignment and were not run. The retained corpus models keep their existing bounds/assumptions; filesystem/archive witnesses do not claim an OS crash or arbitrary Rust refinement proof.

Existing raw live receipts remain compatible with older worktree checkers and age into the archival policy; new receipts are compressed. The Oracle's pre-Nov-4 archive was left untouched. Build/target exports remain plain for native/TLC execution. No real cold receipt was eligible for deletion during this run; the private removal/corruption fixtures provide that evidence.
