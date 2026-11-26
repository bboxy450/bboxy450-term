---
allowed_paths:
- scripts/cache-env.sh
- scripts/test-landing.py
- Makefile
assignee: seat:01M89ZVT3N4Q04T89BSV0XS0RZ
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
closed:
  by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  op_id: 01M8A0AQ47534WGB4BZXTPHHWE
created: 2026-11-24T17:09:30.840Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M878HDJ9H49E3D3WNB42GXX4
number: 1794292861627913
report:
  commit: 64409bc8dfe8ea5bce17d975c3fe7c9260878e81
  op_id: 01M8A09M9VT8H7MT2S72W174Q8
  report: '[[reports/2026-11-25-build-dirs-never-index.den]]'
  reported_by: seat:01M89ZVT3N4Q04T89BSV0XS0RZ
  summary: Landed signed `64409bc8d` on main. `make test-landing` passed 259 fixtures and post-landing `stat` confirmed all three `.metadata_never_index` files. Reused the existing build setup and removed duplicate Git-root and OS lookups. The regression verifies cache-off behavior and marker reuse. The KB report was saved and read back locally. Its checkpoint returned `commit_failed` because Git ignores `reports/`.
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Keep Spotlight out of build dirs: cache-env.sh writes .metadata_never_index into target/ and kache dirs'
type: task
updated: 2026-11-25T18:43:46.290Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: make test-landing
verify_result:
  command: make test-landing
  evidence:
  - criterion: Mark selected macOS target, formal evidence store, and active kache directory
    reference: scripts/cache-env.sh:6, :30–39, :81–84. Post-landing stat exited 0 and reported three 0-byte markers.
  - criterion: Assert marker creation and idempotence in the landing suite
    reference: scripts/test-landing.py:3796. Expected pre-fix failure exited 1. make test-landing exited 0 with 259 fixtures and zero failures.
  - criterion: Land the verified change
    reference: make land exited 0 (LANDED 64409bc8dfe8ea5bce17d975c3fe7c9260878e81); git verify-commit HEAD and git merge-base --is-ancestor HEAD origin/main exited 0; worktree clean.
  exit_code: 0
provenance:
  digest: sha256:136a998b74a1348a5828759108f64e4d867ee866bb8e61939d41615677cbc4d7
  frontmatter_digest: sha256:8d0d2355823e9acdd76075f522747a6114e4290a7eb808714a58d7dee44f3d7c
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: p9ZRW9PWmI2RXZ4AL4wU1bc+vIPnfh4YwPE8R6RlqhdUM7brrToCl/qnGMLABeycDIjmY7Ague2zOIvGwTEMBA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2038
  event:
    id: 01M8A0AR6KQVBS9SMZTY47CZ5D
    sequence: 2038
    hash: 2924af6df5a1f2828a270b8f2a1b5808d73edee4b399a59d71138d5f0e9574d9
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-06-build-dirs-out-of-spotlight-fsevents.md`.

## Evidence
No `.metadata_never_index`, `mdutil` or Spotlight handling in `scripts/`, `Makefile` or `LAND.md` on origin/main 911587f2f.

## Do
Have `scripts/cache-env.sh` create `.metadata_never_index` in the selected target dir, the verification evidence dir and the kache cache dir (idempotent, cheap). Assert it in `scripts/test-landing.py`. `make land`.


## Report from seat:01M89ZVT3N4Q04T89BSV0XS0RZ

Landed signed `64409bc8d` on main. `make test-landing` passed 259 fixtures and post-landing `stat` confirmed all three `.metadata_never_index` files. Reused the existing build setup and removed duplicate Git-root and OS lookups. The regression verifies cache-off behavior and marker reuse. The KB report was saved and read back locally. Its checkpoint returned `commit_failed` because Git ignores `reports/`.

```json
{
  "op_id": "01M8A09M9VT8H7MT2S72W174Q8",
  "commit": "64409bc8dfe8ea5bce17d975c3fe7c9260878e81",
  "report": "[[reports/2026-11-25-build-dirs-never-index.den]]",
  "verify_result": {
    "command": "make test-landing",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Mark selected macOS target, formal evidence store, and active kache directory",
        "reference": "scripts/cache-env.sh:6, :30–39, :81–84. Post-landing stat exited 0 and reported three 0-byte markers."
      },
      {
        "criterion": "Assert marker creation and idempotence in the landing suite",
        "reference": "scripts/test-landing.py:3796. Expected pre-fix failure exited 1. make test-landing exited 0 with 259 fixtures and zero failures."
      },
      {
        "criterion": "Land the verified change",
        "reference": "make land exited 0 (LANDED 64409bc8dfe8ea5bce17d975c3fe7c9260878e81); git verify-commit HEAD and git merge-base --is-ancestor HEAD origin/main exited 0; worktree clean."
      }
    ]
  }
}
```
