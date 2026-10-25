---
id: 01M5SXY4XAK7MSG9ZV3W9R6EFM
type: spec
number: 323
title: "landing: how a cut travels from an Executor's branch to main, with a bounded presubmit, self-service landing by script, a batched postsubmit with culprit finding and revert-first, a quarantine with expiry, and the scale run on a schedule"
status: decided
links:
  - { to: "[[d321-out-of-integration-hell]]", rel: decision }
  - { to: "[[main-red-bounded]]", rel: invariant }
  - { to: "[[no-chokepoints]]", rel: invariant }
  - { to: "[[never-block-swarm]]", rel: invariant }
  - { to: "[[kill-only-own-pids]]", rel: invariant }
  - { to: "[[d320-dispatch-by-noun]]", rel: builds-on }
  - { to: "[[e2e-acceptance]]", rel: builds-on }
  - { to: "[[seats-967-frame]]", rel: constrains }
created: "2026-10-25T16:22:41Z"
updated: "2026-10-25T18:25:30Z"
---
# Landing

Operator, 2026-10-25, through the Oracle: "we are in integration hell, streamline towards deliverables, speed up; research how it's usually dealt with, then do that", then "don't just take my plan; research it yourself, independently (redundant thinking is wanted)", then "you lead this one". Decision [[d321-out-of-integration-hell]] holds the reasoning and the field; this spec is the process, the script contracts and the `AGENTS.md` text. Creativity's twelve shapes (Wren, same day) fed it; which were taken is in §13.

**Lens.** Trunk-based development with a merge queue's discipline and no queue: every landing is checked on the main it will produce, main is verified in batches afterwards, and a red main is bounded by a revert. **Assumes** one machine, three to four presubmits' worth of CPU and disk, about twenty landings a day, signed commits. **Breaks when** landings outrun the postsubmit for a whole day (then the batch is too big and the culprit finder's reruns grow) or when a flake list is used to hide bugs.

## 0. Vocabulary

A **cut** is one Executor's change on one branch, meant to land the day it is claimed. The **candidate** is the cut rebased onto `origin/main`. The **affected set** is the touched crates plus every crate that depends on them, transitively. **Presubmit** is the bounded check `land.sh` runs on the candidate. A **landing** is the fast-forward push of the candidate. **Postsubmit** is the full check on the newest `main`, covering every landing since the last green. **Red** and **green** are postsubmit verdicts on `main`. The **culprit** is the landing a red verdict is traced to. The **quarantine** is the list of tests that do not block while they are being fixed. The **scale run** is the 967-seat frame measurement.

## 0a. Where the landing state lives (Aster's ruling request, 2026-10-25)

The live landing state is **one machine-wide mutable directory, never committed and never per worktree**: `${XDG_STATE_HOME:-$HOME/.local/state}/bboxy450-term/landing/`, holding `main-status`, `landings.log`, `presubmit.log`, `postsubmit.log` and `parity-last-green`; every script reads and writes there, and **`BBOXY_LANDING_STATE_DIR`** overrides it for fixture runs on throwaway repositories, which is the only override. The native render sets the postsubmit stores are cache, not state: `${XDG_CACHE_HOME:-$HOME/.cache}/bboxy450-term/parity/<sha>/`, the pointer in the state directory naming the current green sha. **Evidence enters the record once a day, not per landing:** the night runner copies the day's `landings.log` and `postsubmit.log` into `.bboxy/runs/landing/<date>-landings.log` and `<date>-postsubmit.log` and lands them through `records-push.sh`, so the record carries provenance while `main` never churns on a log line. Wherever this spec names `.bboxy/runs/<file>` for a live file, read the state directory; the committed copies are the daily snapshots. The invariant check `main-red-bounded.sh` reads the live state directory, honouring the override.

## 1. The flow

```
claim → branch → land.sh: slot, rebase, presubmit, ff push → main
                                                            ↓
                              main-watch: batched postsubmit on newest main
                                       ↓ green: covered          ↓ red
                                                         culprit.sh → revert.sh → land.sh (red exception)
```

Nothing in this flow waits on a person. The Oracle reviews after the landing and reverts when the review or the postsubmit says so.

## 2. The affected set: `scripts/affected.sh BASE..HEAD`

Prints, from `git diff --name-only` and `cargo metadata`: the touched crates; the transitive reverse dependencies; a **tier**, `lab` when only `lab/`, `design/` or `.bboxy/` changed, or `code` otherwise (the lab tier builds no crate of the product; its parity compare reads the native renders the last green postsubmit stored, §3, and builds `bboxybook` alone only when no stored set exists yet); and `parity: yes|no`, yes when `lab/`, `design/`, the UI crates or the renderer changed. Direct dependents are not enough: the app sits behind the mux behind the core.

## 3. Presubmit: `scripts/presubmit.sh [BASE]`

On the candidate, in the Executor's own target with the wrappers clear and `CARGO_BUILD_JOBS=2`: `cargo test --locked -p <affected…>`; strict clippy on the affected crates; `sh scripts/invariants.sh` (the default tier); the golden dispatch tests of [[d320-dispatch-by-noun]] when a dispatcher file changed; and, when `parity: yes`, the lab check and the catalogue cases of the touched components only, never the whole catalogue. **Parity in the lab tier (Wren's two holes, 2026-10-25):** a lab-only cut does not change the native side, so the truth it compares against is the **native render set the last green postsubmit stored** (one directory per green sha under the parity store, pointer `.bboxy/runs/parity-last-green`); the presubmit captures the lab pages and runs the compare against that set, building no product crate; only when no stored set exists does it build `bboxybook` alone. A change to one of the **shared lab files**, `lab/web/lab.css`, `lab/web/tokens.js` (generated from `design/tokens.toml`), `lab/web/glow.js` or `lab/web/components.js`, touches every case: at presubmit it runs the lab check plus a **fixed sample of one case per page**, and the whole catalogue runs in postsubmit with revert-first. A per-page edit (`lab/web/<page>.html`) maps to that page's manifest cases, as the scoped filter already does. **Bound 15 minutes**, enforced; a presubmit that runs past it fails with `presubmit_timeout`, and the cut is split, not the bound raised. **Timing-bound tests do not run in presubmit**: a test whose assertion is a wall-clock or CPU bound is tagged (a cargo feature or an `#[ignore]` the postsubmit lifts) and runs in postsubmit, where the machine is not also running three other presubmits. Presubmit writes one line to `presubmit.log` in the landing state directory (§0a): sha, tier, affected crates, seconds, verdict, and `binary_durations`, a list of `{binary, seconds, over_60s}` the night's fast-tests step and [[presubmit-tests-fast]] read by that name.

**The lab tier's checks, exactly (Aster's conflict, 2026-10-25).** "No product crate" is qualified by one rule: the lab tier runs the records gate (the checks `records-push.sh` runs: corpus, mermaid, layering, pending flags, kill-own-pids, cross-OS, brand, images, local-first, never-block, licences), the lab check, the parity compare against the stored renders, and **only the invariants whose `check:` line does not begin with `cargo`**, through `invariants.sh --no-cargo`; `ui-lint.sh` and every cargo-backed invariant are code-tier checks and do not run for a lab or records cut. The `bbt-doc` and `bbt-mermaid` test binaries the records gate builds are the records tooling, not the product, and are allowed.

## 3a. Fast tests (the Oracle's gate-time data, 2026-10-25)

Of 36.9 cargo hours across 175 gate logs, compile was 52 % (the dependency cache of [[d325-ci-capacity]] attacks the dependency share of it), test run 36 %, and six test binaries holding 52 to 72 second real-clock waits summed to over eight hours. So the presubmit tier has a **fast-test rule**, invariant [[presubmit-tests-fast]]: no test binary over 60 s, no wall-clock waits; a duration contract is tested through an injected clock or a shortened test duration, or in the night tier. `presubmit.sh` records per-binary durations in `presubmit.log` so the rule is checkable. The post-test 12 % is the invariants' separate `cargo test` invocations; `invariants.sh` runs the default tier's cargo checks as **one** invocation with a filter list where the crate allows it, one link instead of twenty.

## 4. Slots

The atomic claim stays as it is (`mkdir /tmp/bboxy450-gate-slot-<n>`, exit 75 when none is free): three at 25 GiB free or more, two below. Slots are for presubmits only; the postsubmit runs outside them. The channel line stays a note. The pre-check before the slot is retired: presubmit **is** the check, and it runs once.

## 5. Landing: `scripts/land.sh`

Run by the Executor on their branch. In order: (1) claim a slot; (2) `git fetch origin`; (3) read `.bboxy/runs/main-status`; if it says `red`, refuse with exit **76 `main_red`** unless every commit being landed is a revert (subject begins `Revert`); (4) rebase onto `origin/main`; a conflict is exit **78 `conflict`** and the author resolves it; (5) presubmit; a failure is exit **77 `presubmit_failed`**; (6) fetch again; if `main` moved, compute `affected` of what moved and of the candidate; if the two sets are disjoint and no shared file changed, proceed; otherwise rebase and presubmit again, at most three times, then exit **79 `main_moving`** and the author re-queues; (7) verify every commit is signed (`%G?` is `G`) and authored by the Executor's own key, never a squash the script signs; (8) a plain `git push origin HEAD:main`, **never `--force`** and never a lease, after a local ancestry check that `origin/main` is an ancestor of `HEAD` (Git has no `--ff-only` for push; the remote rejects a non-fast-forward by default and the local check names it first); a rejected push goes back to (6); (9) append one line to `.bboxy/runs/landings.log`: sha, author key, tier, affected crates, presubmit seconds, retries, touched paths; (10) release the slot; (11) post `LANDED <sha> <one line>` to `#bboxy450-term`. Exit 0 only after (11). A hand push to `main` is a violation; `land.sh` is the only door for code, and `records-push.sh` the only door for records.

## 6. Postsubmit: `scripts/main-watch.sh`

One watcher, started by the Oracle's session, its own target, `nice 10`, two cargo jobs, outside the slots: when a new commit on `main` touches `crates/`, `scripts/`, `Cargo.lock`, **`lab/` or `design/`**, run `sh scripts/ci-local.sh --parity` on the **newest** `main` (a lab-only landing gets its whole-catalogue parity here, and a green run **stores its native render set** for the lab tier's presubmit, §3) (a batch covers every landing since the last green), then append to `postsubmit.log`: `<sha> <green|red> <started> <ended> <predicate-or-dash> <culprit-or-dash>`, where the **predicate** is typed (Wren's finding, 2026-10-25): `test:<cargo selector>` for a failing test, `invariant:<gate>` for a red invariant gate (rerun as `sh scripts/invariants.sh <gate>`), `parity:<case ids>` for a parity red (rerun as `scripts/parity.sh` narrowed to the failing cases from its report); a parity red is deterministic, so its reruns settle flake against real at once, and write `.bboxy/runs/main-status`: `green <sha>` or `red <sha> <failing-test>`. On red, run **`scripts/culprit.sh <last-green> <red> <predicate>`**, which accepts every predicate kind above and refuses none, since a red the finder cannot name is a red nobody reverts and `land.sh` then refuses everything but reverts, a deadlock only a hand can break: first rerun the failing predicate three times at the red sha; if it passes every time it is a flake, logged as such, the status stays green, and a quarantine entry is proposed; otherwise bisect the first-parent landings between green and red running only that predicate with three reruns at each step, name the culprit, and run **`scripts/revert.sh <culprit>`**, which creates the signed revert (the watcher's key) and lands it through `land.sh` under the red exception. The tree closer applies to code paths only: records keep landing through `records-push.sh` while `main` is red. Invariant [[main-red-bounded]], check `scripts/main-red-bounded.sh`.

## 7. Quarantine: `.bboxy/quarantine.md` and `scripts/quarantine.sh`

A numbered record with one table: test, crate, owner, task number, since, expires, evidence. `quarantine.sh add <test> --owner <name> --task <n>` adds a row with `expires` seven days out; `expire` drops rows past their date; `list` prints them. A listed test **still runs** in presubmit and postsubmit and its result is logged, so the fix can be proven; it does not block unless it **fails on an isolated retry**, which is a failure (the store race and the socket reclaim race of 2026-10-25 were real bugs that load found, not flakes). A row past its date blocks again. More than five rows is a review by the Oracle, not a fact of life. Admission is only through `culprit.sh`'s flake verdict or an Executor's isolated-retry evidence in the task.

## 8. The scale run: scheduled, never a window

`scripts/scale-night.sh` runs the 967/50 measurement with diagnostics on, after the e2e night, on that night's `main`, and writes its evidence commit; the night runner calls it last. During the day a **proxy** sized by the Oracle (200 seats and 20 watched is the first guess) runs inside a presubmit slot with the same predicates when a cut touches the render or logic hot paths. No `QUIET` window is opened for measurement again; the grammar stays for the operator's own needs.

## 9. Size and overlap

A cut lands the day it is claimed or is split. Contract changes (`bbt-proto`, the `ctl` verb tables, the gallery manifest) land alone and first, as the switch before the load. Big work lands as stacked small landings behind flags, in the noun modules of [[d320-dispatch-by-noun]]. **`scripts/overlap.sh [BRANCH]`** lists the other `origin/*` branches ahead of `main` whose touched paths intersect the branch's, so two Executors see a collision before they write it rather than at the merge; a claim line in the channel names the paths it will touch.

## 10. Roles in the flow

Executors land their own cuts with `land.sh` and hand off the landing line and the evidence. The Oracle writes briefs, reviews landed code after the fact and reverts, owns the main watch and the quarantine review, sizes the proxy and the slots, and lands reverts. The Architect owns this spec, the decisions and the invariants, and records; the Architect's records go through `records-push.sh` as before. Creativity lands lab-only cuts through `land.sh` at the `lab` tier.

## 11. Metrics

`presubmit.log`, `landings.log` and `postsubmit.log` are the process's own evidence. The board shows lead time from the claim line to the `LANDED` line, red minutes per day, reverts per day, presubmit seconds, and the quarantine's length; the Oracle's weekly zoom-out reads these, not the channel.

## 12. Order of build

**First.** `affected.sh`, `presubmit.sh`, `land.sh`, `main-watch.sh` with `main-status`, `culprit.sh`, `revert.sh`, `quarantine.sh` with the record, `main-red-bounded.sh`, `scale-night.sh`; the `AGENTS.md` text below; the timing-test tag. Each script's acceptance is a fixture run on a throwaway repository, never on `main`. **Second.** `overlap.sh`; the [[d320-dispatch-by-noun]] cuts; the board's metrics. **Gaps left open:** proxy size, slot count once the pre-check is gone, culprit reruns; set from the first week's logs.

## 13. Shapes taken and left

Taken from Creativity's twelve: the expediter robot (the watch and `land.sh`, the Oracle tasting and not carrying), the call auction (postsubmit batching), offline and online conform (presubmit on proxies, full in postsubmit), parts not score and append-only registries (via [[d320-dispatch-by-noun]]), blast radius (the affected set), pheromones (`overlap.sh`), crane time (the scheduled scale run), the container standard (contract changes land alone and first), the darts foul line (timing tests out of presubmit, isolated retry as the flake test). Left: release trains (waiting on the clock is a queue), single-token file leases (serialises people), a merge-queue bot with full gates (a queue again). Wren's gap, the share of flakes among gate failures, is answered by the record: of about fourteen failed gates on 2026-10-25, three were environmental (PTY exhaustion during a scale run, the shared compile cache), two were test maintenance, eight were real defects found under load, one unnamed. The Oracle's own count of the same day, from its record: of about twelve reds, five were real misses a presubmit on the affected set catches in minutes, four were real product bugs that appeared only under load, one a fixture bug, two unexplained; and about six gates were thrown away to quiet windows and rebases. Both counts say the same thing: most reds were real and cheap to find earlier, and the waste was gate length times reruns times rebases, not flakiness. Load is a test oracle here; the quarantine is therefore narrow, and the presubmit must be the thing that catches the five.

## AGENTS.md text

The sections "CI is local" and the "Executors never push main" rule are replaced by the words below; the Oracle applies them.

> **Landing.** Code reaches `main` only through `scripts/land.sh`, run by the author on their branch: it claims a slot, rebases onto `origin/main`, runs the presubmit (affected crates and their transitive dependents, strict clippy, fast invariants, golden dispatch tests, touched parity cases; 15 minutes, enforced), fast-forward pushes signed commits, and posts `LANDED <sha>`. It refuses while `.bboxy/runs/main-status` is red unless you are landing a revert. The main watch runs the full `ci-local.sh --parity` on `main` afterwards in batches; a red main is reverted first and fixed after, never built upon. A test that fails under load and passes alone is a bug with an owner and a task; it may sit in `.bboxy/quarantine.md` for seven days while you fix it, still running, still logged. Land the day you claim or split the cut. Contract changes land alone and first. No `QUIET` windows for measurement; the scale run is the night's. Spec: `.bboxy/specs/landing.md`.

Owner: Corvin Duskmantle <ARCHITECT>.
