---
id: 01M5SXQ96PECM77XVPK7YPYXWD
type: decision
number: 321
title: "integration flow: trunk-based landing by script with a bounded presubmit on the rebased candidate, a batched postsubmit on main with culprit finding and revert-first, a flaky quarantine with owner, task and expiry, same-day cuts, and the scale measurement moved to a scheduled slot"
status: decided
decided: "2026-10-25"
operator:
  - "we are in integration hell, streamline towards deliverables, speed up; research how it's usually dealt with, then do that"
  - "don't just take my plan; research it yourself, independently (redundant thinking is wanted)"
links:
  - { to: "[[no-chokepoints]]", rel: instance-of }
  - { to: "[[main-red-bounded]]", rel: constrains }
  - { to: "[[landing]]", rel: decides }
  - { to: "[[d320-dispatch-by-noun]]", rel: builds-on }
  - { to: "[[d285-no-gated-communications]]", rel: builds-on }
  - { to: "[[never-block-swarm]]", rel: builds-on }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-25T16:18:56Z"
updated: "2026-10-25T16:22:41Z"
---
# Decision 321: out of integration hell

Ruled on the operator's instruction, relayed by the Oracle with a plan, and then with the correction that the Architect research it independently and say where the plan is wrong or missing. Both are quoted above. This record is the **written authorization** that `AGENTS.md` requires before code lands without the Oracle's hand: code landed through `scripts/land.sh` under the rules below is authorized by the operator's words here.

## What the record of 2026-10-24 and 25 shows

Counted from the channel and the tree, not recalled: full gates of 18 to 63 minutes, parity about an hour, three slots; every landing (about twenty in the day) told two or three peers to rebase, and at least four gates were stopped mid-run because main had moved under them; six distinct load failures took gates down (a store race and a socket reclaim race that were **real bugs found by load**, a disconnect fence, a stalled shared compile cache, PTY exhaustion during a scale run, and a keyboard-boundary invariant that caught a real regression); the Oracle's merged-tree run before every landing was a serial queue five and six deep; the same workspace tests ran three times per change, in pre-check, in the gate and in the merged tree; and **five scale-measurement quiet windows stopped every gate for 30 to 80 minutes each**. The two biggest costs in minutes were the quiet windows and the serial merge; the two biggest costs in reruns were flakes and rebase churn.

## What the field does

Trunk-based development: branches live less than a day, incomplete work lands behind a flag, every commit integrates. Google: a fast presubmit on the affected targets only, then a **batched postsubmit** on landed changes with automated **culprit finding**, flake-aware because plain bisection is defeated by non-deterministic tests (ICST 2023); rollback first, fix after. Chromium: a commit queue on the rebased patch, sheriffs with authority to revert on a red tree and to disable a misbehaving test, the tree closed to everything but reverts and fixes while red. Merge queues (Trunk, Graphite, Mergify, Harness): test the speculative main the candidate would produce, batch low-risk changes, and on a red batch **bisect in a separate queue** so the main queue keeps moving; frequent bisection means batches are too large, deep bisection means flakes. Everyone: a flaky test is a bug with an owner and a deadline, not a thing to retry forever.

## Alternatives

(a) Keep the shape: serial Oracle merges after full gates. Today. (b) The Oracle's plan: presubmit fast on touched crates plus direct dependents, postsubmit full on main with revert-first, self-service `land.sh`, a quarantine list, same-day cuts. (c) A merge-queue bot serialising full gates on speculative mains: a queue again, one hour per entry. (d) No gates. **Choice: (b), amended by the eight points below.** (c) moves the chokepoint; (d) is the operator's mantra misread.

## Where the plan was wrong or missing

1. **Postsubmit must batch and attribute.** Twenty landings a day against a 30 to 60 minute full run cannot be verified one landing at a time; that is the serial merge chokepoint moved one step later. Postsubmit runs on the newest main; a green verdict covers every landing since the previous green. On red, **culprit finding** runs over the landings in the batch using only the failing test, with reruns to tell a flake from a break, then the culprit alone is reverted. Vesper's plan had revert-first with no attribution step.
2. **Transitive, not direct.** The presubmit set is the touched crates plus every crate that depends on them transitively, from `cargo metadata`; the workspace is small enough that this is still bounded. Direct dependents miss the app behind the mux behind the core.
3. **The skip rule is a set intersection.** Re-run presubmit when main moved only if the moved paths' crate set intersects the candidate's affected set, in either direction; "outside crates, scripts and Cargo.lock" is too coarse both ways.
4. **The scale measurement is the largest serialisation and the plan did not name it.** The 967/50 frame run moves to a **scheduled slot**, after the e2e night, run by the night runner on that night's main as a postsubmit, with the evidence commit it writes; the day never opens a quiet window for it again. For daytime direction a smaller proxy (the Oracle sizes it, 200 seats and 20 watched is the first guess) runs inside a presubmit slot with the same predicates and no window.
5. **Quarantine has an expiry, a cap, and still runs.** A test enters the list only with an owner, a task number and a date; it leaves after seven days or when fixed; the list over five entries is a review, not a fact of life; a quarantined test still runs and records its result so the fix can be proven; a quarantined test that **fails on an isolated retry is a failure**, because two of today's "flakes" were real bugs the load found.
6. **The tree closer is a mechanism, not a habit.** The main watch writes `.bboxy/runs/main-status` (`green <sha>` or `red <sha> <failing test>`); `land.sh` reads it and refuses to land anything but a revert while it says red. That is what makes [[main-red-bounded]] an invariant rather than a wish.
7. **Reverts are clean because landings are small.** A landing that cannot be reverted cleanly within one postsubmit cycle was too big; a landing others built on in the meantime is reverted with its dependents, Chromium's rule. Big work lands as stacked small landings behind flags (the lab and parity gating already exist) and in the noun modules of [[d320-dispatch-by-noun]].
8. **Measure the process.** `land.sh` appends one line per landing to `.bboxy/runs/landings.log` (sha, author key, presubmit seconds, retries, affected crates) and the main watch one per postsubmit to `.bboxy/runs/postsubmit.log`; the board shows lead time from claim to land, red minutes and reverts; zoom-out reads them rather than the channel.

Kept from the plan as right: the presubmit and postsubmit split, the 15-minute presubmit bound, self-service landing by script with a fast-forward push and a retry, the Oracle reviewing after the merge and reverting, same-day cuts, and the quarantine's admission rule of one isolated retry.

## Rules

(1) **Presubmit**, in `land.sh`, on the candidate rebased onto `origin/main`: the affected crates' tests (touched plus transitive dependents), strict clippy on them, the fast invariants, and the golden tests of the dispatchers; bound 15 minutes; the slot is the existing atomic claim. The merged-tree run before landing is retired; the workspace runs once per change. (2) **Landing** is a fast-forward push of signed commits by the author's own key, never a squash the script signs, after presubmit held on that exact base, with the skip rule of point 3 when main moved, and refused while `main-status` is red except for a revert. (3) **Postsubmit** is the full `ci-local.sh --parity` on the newest main, continuous, batched, with culprit finding on red and the culprit's revert landed before anything else; invariant [[main-red-bounded]]. (4) **Quarantine** per point 5, the list a record in the tree with owner, task and date per entry. (5) **Scale measurement** per point 4, scheduled and never a window. (6) **Size**: a cut lands the day it is claimed or is split. (7) The Oracle reviews landed code after the fact and reverts; the Architect records; Executors land their own cuts. `AGENTS.md` changes accordingly, by the Oracle.

**The process, the script contracts and the `AGENTS.md` text** are [[landing]] (#323), written at the operator's word that the Architect leads this one.

**Gaps left open.** The daytime proxy size; the presubmit slot count once the pre-check and gate collapse into one run (more slots at two cargo jobs each is the first guess); the culprit finder's rerun count. Each is set from the first week's `landings.log`, not now.

Sources read for the field section: Google, "Flake Aware Culprit Finding" (ICST 2023) and "What Breaks Google?"; the Chromium sheriffing document; Trunk, Graphite and Mergify merge-queue batching and bisection documentation.

Owner: Corvin Duskmantle <ARCHITECT>.
