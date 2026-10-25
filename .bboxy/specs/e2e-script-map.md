---
id: 01M5R9D3Y7S6NATG9B47HD5YZA
type: spec
number: 317
title: "e2e script map: one row per promise, the night script, its coverage, the missing assertions and the task tracking them; agrees with the scripts' coverage headers"
status: draft
links:
  - { to: "[[e2e-acceptance]]", rel: builds-on }
  - { to: "[[d316-pass-means-the-whole-promise]]", rel: decision }
created: "2026-10-25T01:04:37Z"
updated: "2026-10-25T01:19:45Z"
---
# e2e script map

**Lens.** Each promise of [[e2e-acceptance]] §1 is tested by one night script, `scripts/e2e/<ID>.sh`; this map says, per promise, how much of the promise the script proves. Decision [[d316-pass-means-the-whole-promise]]: the script's `# coverage:` header is the source of truth, this table must agree with it, and only `full` is green.

Aster's read-only map arrives in six parts and fills the rows below; a row is one line, `missing` in the promise's own words, `tracked-by` the open task's number when partial. Blank cells mean not yet mapped, not full. **Whoever lands or changes a script's `# coverage:` header edits that promise's row in the same commit**, so the table and the headers agree by construction; a later corpus check only confirms it. Aster's read-only map seeds rows for scripts nobody is touching; Seraph's runner-headless commit carries the terminal rows (T1 to T9) with its headers.

| ID | script | coverage | missing | tracked-by |
|---|---|---|---|---|
| T1 | `scripts/e2e/T1.sh` | partial | the one-frame bound | |
| T2 | `scripts/e2e/T2.sh` | partial | the full compare (the script truncates it) | |
| T5 | `scripts/e2e/T5.sh` | partial | the app restarted, not only logic killed | |
| T6 | `scripts/e2e/T6.sh` | partial | the harness resumed by session id (only the zsh respawn is proven) | |

Owner: Corvin Duskmantle <ARCHITECT>; rows by Aster Cinderwing <EXECUTOR> under the Oracle's brief.
