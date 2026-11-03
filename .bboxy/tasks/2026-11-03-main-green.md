---
type: task
title: 'Make main green: harness layering order, HarnessTextZoom coverage, startup-modal report record'
role: executor
verify: make test
id: 01M6HEGJFCQMX0F3MW72ZJAMJ0
status: open
delegated_by: seat:01M6HEDKGMGNSJYV4KEYAH4C70
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-03T19:34:55.027Z
created: 2026-11-03T19:34:55.027Z
number: 16366
created_by: seat:01M6HEDKGMGNSJYV4KEYAH4C70
updated_by: seat:01M6HEDKGMGNSJYV4KEYAH4C70
authorship:
  signer: seat:01M6HEDKGMGNSJYV4KEYAH4C70
  kind: seat
  public_key: Ayn9XrIjVAYAXEZChx8NJacOr4FXag6KviYA5wiyv0c=
  process_generation: '1'
provenance:
  digest: sha256:98f9a836c16dd2fe7c35dc3961aa07924154d71b23b438c2cb58ecfca18973fc
  frontmatter_digest: sha256:38e49d64487e58e8d26043420514d4c3e8d396b3c47c4cace2bf7d15a9b5f788
  signer: seat:01M6HEDKGMGNSJYV4KEYAH4C70
  sig: zqmg0VZvi3VsGrFeOwrdjvSF6NGv4SxmwviROdqvdN+ioGA8RI+RYp5X8uIoYwxVYIv9eKA4/HhXoXYmVw7pBA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#94804
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
