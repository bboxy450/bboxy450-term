---
id: 01M5JQE3ZY2WM2FSH80ZCQ59QD
number: 141
type: invariant
title: the UI thread never waits on IO
status: active
check: sh scripts/ui-lint.sh
cost: one blocking call stalls a frame; at 1000 seats the app freezes
links:
  - { to: "[[runtime]]", rel: constrains }
  - { to: "[[c1-live-seats]]", rel: constrains }
created: 2026-10-22T21:14:24Z
updated: 2026-10-22T21:14:24Z
---
# The UI thread never waits on IO

**Property.** No file, socket, process, sleep, blocking channel receive, lock wait
or `block_on` in the `src/` trees of bbt-ui-model, bbt-ui, bbt-render and the
app ([[runtime]] §1; exemptions listed there).

**Check.** `sh scripts/ui-lint.sh` — clippy `disallowed_methods` from
`clippy.toml` — exit 0 held, exit 1 a disallowed call with its call site.
Part of `sh scripts/ci-local.sh`.

**Cost of a violation.** One blocking call stalls a frame; the hitch meter shows
it; at 1000 seats the app freezes while IO completes. Everything the UI shows
must already be in memory it owns.

**Evidence.** `sh scripts/ui-lint.sh — exit 0 at c0f13f3` (Architect, 2026-10-22).
Release builds do not run `assert_not_ui_thread` ([[architecture]] §7); the lint
is the gate, the assert is debug help.

Owner: Corvin Duskmantle <ARCHITECT>.
