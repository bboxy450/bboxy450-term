---
id: 01M5SXQ96PJH8STZ1687160J90
type: invariant
number: 322
title: "main is red at most until the next revert: a red postsubmit is followed by the culprit's revert before any other code lands, and nothing but a revert lands while main is red"
status: active
slow: true
gate: watch
check: sh scripts/main-red-bounded.sh
cost: a red main that keeps taking landings turns one culprit into a pile nobody can bisect, every Executor rebases onto a broken base, and the night runs against it; the swarm is back in the hell this flow left
links:
  - { to: "[[d321-out-of-integration-hell]]", rel: decision }
  - { to: "[[no-chokepoints]]", rel: builds-on }
  - { to: "[[never-block-swarm]]", rel: builds-on }
created: "2026-10-25T16:18:56Z"
updated: "2026-10-26T03:02:13Z"
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
<started> <ended> <predicate-or-dash> <culprit-or-dash>`, the predicate typed `test:`, `invariant:` or `parity:` per [[landing]] §6 so a parity or invariant red is bisected and reverted like a test red) and `git log
--first-parent origin/main`, and asserts, for every red line: the first later
commit touching the code paths is a revert (subject begins `Revert` and names
the culprit sha), and every commit between that red verdict and the next green
verdict touching the code paths is a revert; it also asserts every code commit
on `main` older than the newest green verdict is covered. **While red starts at publication, not at the snapshot** (Nyx's binding question, 2026-10-25): a red verdict is computed on a snapshot sha while later landings may already be arriving during its long run, and `land.sh` can refuse only once `main-status` says red. So the interval in which only reverts may land runs from the **publication** of the red verdict (the time `main-status` was written red, the postsubmit line's `ended`) to the next green; a code commit whose landing time in `landings.log` precedes that publication landed legitimately and belongs to the next batch, and is never a violation. "The first later code commit is a revert" reads the same way: the first code commit **landed after publication**. **Recording and consequence (Nyx's writer questions, same day).** Publication is the **first** time a red is written to `main-status`, before any bisect or revert, preserved even when the postsubmit line's completion is recorded later; `landings.log` records each landing's base as well as its sha and time, so a multi-commit cut is attributed whole, and older lines without a base are named `unobserved`, never guessed. A violation of this invariant is a fact about the **order things already landed in**, which no revert can undo: it is recorded as a watch failure with a nonzero exit and raised to the Oracle as a finding, and it **never** sets `main-status` red or enters the culprit and revert loop, since blocking every landing over history would be a chokepoint that fixes nothing. **What it evaluates
and what pending means (Nyx's question, 2026-10-25):** the bound is structural,
the next code commit, never a wall clock, so the check invents no expiry. When
`origin/main` is exactly the red sha and no later code commit exists, the state
is **pending**, reported as such and held so far, exit 0: nothing has landed
that the property forbids. When the check runs inside a presubmit, it evaluates
the **candidate `HEAD`** if that candidate is a descendant of `origin/main`
consisting only of reverts, so the revert that clears the red is not refused by
the red it clears; otherwise it evaluates `origin/main`. A non-revert candidate
on a red tip is the violation, and `land.sh` refuses it first with
`main_red`. The length of each red interval is a metric in the postsubmit log,
never an assertion of this check. **A silent watcher is visible, not green** (2026-10-25: the check held over a manual verdict while the watcher's next full run had written nothing for hours): the check reports `uncovered`, the code commits on `main` after `covered_through`, and their count; the hard gate is unchanged, because a missing verdict is not a red, but an `uncovered` count above the batch the watcher was last seen running is a **finding** shown on the board and raised to the Oracle, since a dead or wedged watcher otherwise looks exactly like a green main. The uncovered count only grows when code lands, so the check also reports the age of the watcher's lease (`main-watch.lock` in the landing state directory); a lease older than the watcher's longest recorded run is the same finding when `main` is quiet (the Oracle, same day). Both are advice; the gate stays structural. **Bootstrap:** before the watcher has
recorded any green there is no culprit to search; a red that survives the
flake check then ends by the Oracle's logged hand call (`manual-green` or
`manual-revert`), which the check accepts as the verdict that closes the red
interval ([[landing]] §6, 2026-10-25). **Pending** until the
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

**Active (2026-10-25).** The first full watcher verdict is 5c6d0edc, green, every gate including parity; the check run on live state exits 0, held, no violations, covered through 8b259fb3 (a later infrastructure red closed by a logged `manual-green`). It stays in the **slow tier with `gate: watch`**: the check reads the live landing state and describes `main`, not the cut under test, so it belongs to the watcher and the night, where that state lives, and never to a presubmit. With no postsubmit history at all it reports `pending`, `covered_through: null`, no violations, exit 0, as the pending rule above says (Nyx, from the landed source and its bootstrap fixture, verified on a new empty state directory; an earlier sentence here said exit 2, from a run of an older build, and was wrong).

Owner: Corvin Duskmantle <ARCHITECT>.
