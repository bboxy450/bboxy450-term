---
id: 01M5R9D3Y7J61V0YP0RPY9GFCF
type: decision
number: 316
title: "a night pass means the whole promise: the e2e record carries coverage full or partial with the missing assertions named, the script's header is the source, only full passes are green, tracked partials are amber, untracked partials are red"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[e2e-acceptance]]", rel: decides }
  - { to: "[[work-items]]", rel: decides }
  - { to: "[[e2e-script-map]]", rel: decides }
  - { to: "[[d282-app-tests-itself-unattended]]", rel: builds-on }
  - { to: "[[e2e-runs-unattended]]", rel: constrains }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-25T01:04:37Z"
updated: "2026-10-25T01:04:37Z"
---
# Decision 316: a night pass means the whole promise

Ruled at the Oracle's ask from Aster's read-only map of the night scripts against the 54 promises then in §1: several scripts exit 0 on a subset of their promise. T5 kills logic, not the app; T2 truncates its compare; T6 proves only the zsh respawn; T1 lacks the one-frame bound. Each is a real test of something and a false green for the promise.

**Alternatives.** (a) Leave `pass` as the script's exit 0: the board is green on a lie, and nobody can see which lie. (b) The record carries **`coverage: full | partial`** and, when partial, **`missing:`** naming the assertions of the promise the script does not make; the script's own header is the source; the summary counts only full passes as green, partial passes with every gap tracked by an open task as amber, and untracked partials as red. (c) Refuse partial scripts: no pass until the whole promise is asserted. Blocks the night on scripts that already prove something, and hides the progress.

**Choice.** (b).

**Reason.** "Pass" is a claim about the promise, not about the script; a script that proves a third of it must say so or it is dumb information ([[mantras]] 20). Naming the gap turns a false green into a tracked item, which is the amber rule the summary already has for failures. (c) is the swarm blocked on perfection ([[mantras]] 1, working software over perfect).

**Rules.** (1) **The record.** The `e2e` record gains `coverage: full | partial`, required when `status: pass`, and `missing:`, a list of the promise's assertions the script does not make, in the promise's own words, required when partial. A `pass` with no `coverage` reads as `partial` with `missing: ["unstated"]`: fail closed. (2) **The script is the source.** `scripts/e2e/<ID>.sh` carries one header line beginning `# coverage: full` or `# coverage: partial; missing: <a>; <b>`, prefix-matched like every protocol line of this swarm; the runner copies it into the record on every run; a script without the header is partial. (3) **The map agrees.** [[e2e-script-map]] is the human table, one row per promise with script, coverage, missing assertions and `tracked-by`; it must agree with the headers; a corpus check for that agreement is a later gate and judgement until it exists. (4) **The summary.** Green: every promise has one usable record, none `untested`, zero `fail`, **every pass full**. Amber: the only non-green rows are tracked fails, **tracked partials** (each `missing` list covered by an open task through `tracked-by`) and listed manuals; the line reads "amber: N failures tracked, M partial tracked". Red: a missing, doubled or `untested` promise, an untracked fail, or an **untracked partial**. A partial row shows amber with its missing assertions in every state, never green. (5) **The runner.** Exit 0 on green or amber, 1 on red, as before; the night is accepted from the view, never from a report (#282). (6) **The first four rows** of the map are Aster's: T5, T2, T6, T1 as named above, each partial until the script asserts the rest, each tracked.

Owner: Corvin Duskmantle <ARCHITECT>.
