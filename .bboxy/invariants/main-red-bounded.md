---
id: 01M5SXQ96PJH8STZ1687160J90
type: invariant
number: 322
title: "main is red at most until the next revert: a red postsubmit is followed by the culprit's revert before any other code lands, and nothing but a revert lands while main is red"
status: pending
slow: true
gate: pending-postsubmit
check: sh scripts/main-red-bounded.sh
cost: a red main that keeps taking landings turns one culprit into a pile nobody can bisect, every Executor rebases onto a broken base, and the night runs against it; the swarm is back in the hell this flow left
links:
  - { to: "[[d321-out-of-integration-hell]]", rel: decision }
  - { to: "[[no-chokepoints]]", rel: builds-on }
  - { to: "[[never-block-swarm]]", rel: builds-on }
created: "2026-10-25T16:18:56Z"
updated: "2026-10-25T16:18:56Z"
---
# Main is red at most until the next revert

**Property.** Every commit on `main` that changes `crates/`, `scripts/` or
`Cargo.lock` is covered by a postsubmit verdict (a green verdict on a later
commit covers the landings before it). When a postsubmit verdict is red, the
**first following code commit on `main` is a revert** of the culprit the
postsubmit named (or of the whole batch when the culprit finder could not
separate it), and **no code commit other than a revert lands between a red
verdict and the next green one**. The time `main` is red is therefore bounded
by one postsubmit run, one culprit search and one revert push, and every red
interval is visible in the log with its length ([[d321-out-of-integration-hell]]).

**Check.** `sh scripts/main-red-bounded.sh` — exit 0 held. The script reads
`.bboxy/runs/postsubmit.log` (one line per postsubmit: `<sha> <green|red>
<started> <ended> <failing-test-or-dash> <culprit-or-dash>`) and `git log
--first-parent origin/main`, and asserts, for every red line: the first later
commit touching the code paths is a revert (subject begins `Revert` and names
the culprit sha), and every commit between that red verdict and the next green
verdict touching the code paths is a revert; it also asserts every code commit
on `main` older than the newest green verdict is covered. **Pending** until the
main watch writes the log; `slow: true` and `gate: pending-postsubmit` keep it
out of the default run until then, and the commit that lands the watch removes
both and the Oracle flips `status` on acceptance.

**Cost of a violation.** A red main that keeps taking landings turns one
culprit into a pile nobody can bisect, every Executor rebases onto a broken
base, and the night runs against it: the swarm is back in the hell this flow
left.

**Evidence.** The two days before this decision: about twenty landings a day,
each preceded by the Oracle's serial merged-tree run and followed by two or
three peer rebases; no postsubmit, so a red main had no definition and no
bound.

Owner: Corvin Duskmantle <ARCHITECT>.
