---
type: task
title: 'Make main green: harness layering order, HarnessTextZoom coverage, startup-modal report record'
role: executor
verify: make test
id: 01M6HEGJFCQMX0F3MW72ZJAMJ0
status: done
delegated_by: seat:01M6HEDKGMGNSJYV4KEYAH4C70
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T14:16:14.449Z
created: 2026-11-03T19:34:55.027Z
number: 16366
created_by: seat:01M6HEDKGMGNSJYV4KEYAH4C70
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: open
  disposition: implemented
  reason: 'Post-restart historical triage after workspace readiness returned: the original three main-green blockers were repaired and the complete normal gate passed at40f45c58, before later feature merges. Published/installed ca7cde96 includes those repairs. Close this stale open repair task without claiming a fresh full gate for ca7cde96.'
  evidence:
  - '/tmp/hol-final-complete-land.log SHA2560148a2512afc63d2f9c03ab82af55f84883381f7fb4f5c81f235c453ed95241a: all gates passed, ci-local40f45c5838f521bd70e9b8ff907caca3f8784eb9 normal0,290 models/119 contracts/34 crates and LANDED40f45c58.'
  - '/tmp/hol-report-land.log SHA256849e6bb6835cb57b796113d1b74c911f53c391e682e7309438253d37a975a263: LANDED184b770f9f43094c518570f143e5eeaeb7e999ef with retained checked-source full gate reuse. Both are ancestors of remote mainca7cde96.'
  - Remote mainca7cde96 includes3eafd2be harness text-zoom inventory plus corrected layering/legacy report corpus; accepted rich-host#15252 independently retains the later actual focused corpus repair/delivery proof.
  - The later ca7cde96 merge message explicitly retains new merge-wide test/formal/parity debt; historical repair closure does not erase that limitation.
  by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  op_id: 01M6KENRN0D7QD9RRFS9C9AHGB
provenance:
  digest: sha256:98f9a836c16dd2fe7c35dc3961aa07924154d71b23b438c2cb58ecfca18973fc
  frontmatter_digest: sha256:2b0e01c27fc2d552a7cb4a89ec5ff83c718c55eae6d2d7ab7488ce86ae99f506
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: gjIv7LheaiXbOtGzF4eL+EPDL91qSrrfYjRKLMcrUPsSjLic4kDOVW0NMKbUW8jlnjYuzi7qP+URCAC38sWwAA==
  log: .bboxy/log/2026-11-04.jsonl#114036
---
# Make main green

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-main-green`, branch `main-green`, from `origin/main`.

Main has been red since 3e581dfe/1affb986, and five finished Executor cuts are blocked behind it. Three red gates are known, witnessed by Executors in their worktrees:

1. **Layering**: `sh scripts/layering.sh` exits 1 (`bboxy450-harness -> bboxy450-term-desktop(-core): depends upward`), caused by harness pin 0c403165 (`crates/bboxy450-harness/Cargo.toml:34-35`). The fix is already written and signed as `dfb21967` on branch `layering-harness-desktop` (worktree `~/mydev/bboxy450-term-layering`): it moves desktop-core, desktop-macos and desktop before harness in ORDER, plus doc copies. Cherry-pick it. Oracle check: nothing depends on harness, and the desktop crates depend only on each other.
2. **Formal coverage**: `HarnessTextZoom` is missing from `verification/coverage.toml` (land.sh 77; gate log `~/.local/state/bboxy450-term/landing/presubmits/dfb21967…/commands.log`). Add the correct entry: find which model or source introduced it and map it, in the format of neighbouring entries.
3. **Records**: `.bboxy/reports/2026-11-03-startup-modal.md` (from 1affb986) has an invalid or unresolved task locator at `:9` and duplicate number `5002227`, so the report corpus fails 7/9. Correct the record by the KB's rules for reports (append-only reports are corrected with a superseding report if the rules require it; follow `.bboxy` README/specs) so the corpus passes.

Find and fix any further red gate on main the same way, in the smallest form, and name each one in the report. Do not weaken, skip or quarantine a gate.

Allowed paths: `scripts/layering.sh`, `.bboxy/invariants/layering.md`, files dfb21967 touches, `verification/coverage.toml`, `verification/corpus.toml`, `.bboxy/reports/**`, `.bboxy/tasks/**` (only a missing record a report needs). Anything else needs a report of the exact blocker.

`make test` exit 0 on the final commit, then land with `scripts/land.sh`. Report with `verify.command` = `make test` verbatim.
