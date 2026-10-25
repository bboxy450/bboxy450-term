---
id: 01M5SYRWGQTSXKEFG9QZ09BA55
type: decision
number: 325
title: "CI capacity: the dependency cache done right at zero cost, one cheap ARM Linux box for the portable tier, and a second Mac mini as the postsubmit and night machine; hosted macOS minutes and rented Macs rejected on price"
status: decided
decided: "2026-10-25"
operator:
  - "we build in rust and we only have this machine as a build machine, that's a bottleneck — what is the cheapest way to set up CI for us on this repository? CI that is actually gonna run and build."
links:
  - { to: "[[d15-local-ci]]", rel: amends }
  - { to: "[[d321-out-of-integration-hell]]", rel: builds-on }
  - { to: "[[landing]]", rel: builds-on }
  - { to: "[[d46-cross-os-facades]]", rel: builds-on }
  - { to: "[[d8-open-source]]", rel: builds-on }
  - { to: "[[no-chokepoints]]", rel: builds-on }
created: "2026-10-25T16:37:17Z"
updated: "2026-10-25T22:22:44Z"
---
# Decision 325: the cheapest CI that actually runs

The operator's question, through the Oracle, is quoted above. Facts of the machine: one Apple M4 with 10 cores and 24 GB, shared by nine agents; presubmits of 18 to 63 minutes, parity about an hour; parity needs macOS and Metal, custody and notifications have macOS paths, the Linux and Windows backends exist behind the cross-OS facades ([[d46-cross-os-facades]]); the repository is private on GitHub, and its `ci.yml` (`macos-15`, manual dispatch only) failed in four seconds on billing on 2026-10-22, which is why [[d15-local-ci]] made CI local. Prices below are as found on 2026-10-25; sources at the end.

## What a clean gate actually spends

A gate runs in a clean detached worktree with its own empty target, so every gate recompiles the roughly 480 dependency crates before it compiles ours. The dependencies are the bulk of a cold build; our own crates and the tests are the smaller part. The shared compile cache tried on 2026-10-24 failed because **one** machine-wide server stalled under nine agents, not because caching failed ([[no-chokepoints]], sixth instance); the record then banned the wrapper entirely. That ban cost every gate the whole dependency build.

## The options, priced

| Option | Price found | Our monthly cost | What it offloads |
|---|---|---|---|
| GitHub-hosted macOS runner (3-core M1, slower than the M4) | $0.062 per minute, private-repo included minutes cover about four gates a month | about twenty gates a day at an hour each: **about $2,200** | Everything, at ten times the Linux rate and on a slower core |
| GitHub-hosted Linux | $0.006 per minute | small, but only the portable tier runs there | The portable tier only |
| GitHub self-hosted runner | no per-minute fee (the $0.002 fee announced for March 2026 was postponed and has not taken effect) | the box's price | Whatever the box runs; GitHub's billing stays out of it |
| Hetzner cloud ARM, CAX31 (8 vCPU, 16 GB) or CAX41 (16 vCPU, 32 GB) | about €21 to €26, about €41 to €49 after the 2026 increases; the cost-optimised line is sometimes out of stock | **€25 to €50** | The portable tier: corpus, mermaid, doc, proto, plugin, yarn, core, model, the Linux backends nobody runs today, clippy, every grep invariant |
| Hetzner dedicated AX42 (x86) | €97 for new orders | €97 | The same portable tier, faster, no macOS |
| Scaleway Mac mini M4 | €0.22 per hour, 24-hour minimum | **about €160** continuous | A full macOS tier |
| MacStadium Mac mini M4 | $119 to $149 | **$119 to $149** | A full macOS tier |
| Smaller Mac hosts (M4 mini) | $50 to $130 | $50 to $130, unverified hosts | A full macOS tier |
| AWS EC2 Mac M4 dedicated host | about $1.23 per hour, 24-hour minimum | **about $900** | A full macOS tier |
| **A second Mac mini M4**, 16 GB, bought | $599 list, $549 seen; 24 GB at $799 | **$0 recurring**; about $50 a month over the first year | A full macOS tier, ours, on the LAN, no minimums |
| Cloudflare R2 for a durable cache | $0.015 per GB-month after 10 GB free, no egress | **under $1** for a 30 GB cache | Nothing by itself; makes the cache survive machines |

## Alternatives and choice

(a) Pay GitHub for macOS minutes: the most expensive line in the table, on a slower core, and the billing that already failed once. (b) Rent a Mac: €160 a month buys the same box as a $599 purchase in four months. (c) Buy no macOS capacity and only fix the cache: the dependency build goes away, but postsubmit, the e2e night and the scale run still share the nine agents' machine. **(d) Three steps, in cost order:**

1. **Zero cost, now: the dependency cache done right.** `sccache` with **one server per agent** (`SCCACHE_SERVER_PORT` per Executor, set by `ci-local.sh`, `presubmit.sh` and `invariants.sh`) over **one shared local disk cache** (`SCCACHE_DIR` under the user's cache directory, capped at 50 GB), so the 480 dependencies compile once per toolchain for all nine agents and no single daemon stands in anyone's path. The stall was the single server; the cache itself was never the problem. Add the R2 backend only when a second machine exists and should share it. Expected: a clean gate's dependency build drops from the better part of ten minutes to about one. **Measured, and corrected (Velvet, be2a72a6; the Oracle's ruling, 2026-10-25):** sccache 0.18.0 hashes the absolute working directory and the `CARGO_*` environment into a Rust artifact's key (`src/compiler/rust.rs:1565-1595`; `SCCACHE_BASEDIRS` is ignored on that path), so artifacts are **not** shared across checkout paths: a real gate in a fresh detached tree got 432 misses and a 7 m 17 s build, while the same path got 434 of 434 hits and 127 s. "Compiles once for all nine agents" was false as written. Ruled instead: gates run in **fixed per-slot paths**, `/tmp/bboxy450-gate-slot-<n>/{tree,target}`, still a clean tree each time, so every gate on a slot hits that slot's cache; pre-checks and presubmits hit per worktree, whose paths are stable. The cache is shared per path, not per machine, and the saving is the same where it matters: the dependency build of every gate after the first on each slot.
2. **€25 to €50 a month, this week: one Hetzner ARM box** (CAX31, or CAX41 when in stock) running our own `main-watch.sh` for the **portable tier** on every `main` commit, driven by `git` over SSH, no GitHub Actions involved and no billing: the corpus and records gate, `bbt-core`, `bbt-doc`, `bbt-mermaid`, `bbt-proto`, `bbt-plugin`, `bbt-rt`, `bbt-yarn`, `bbt-ui-model`, the firehose, the wake-up kit, the file and Linux backends, strict clippy on all of them, and every grep invariant. It proves the cross-OS story the facades promise and takes the fast tier off the Mac. The remaining macOS-bound crates today are the app, `bbt-render` (CoreText), `bbt-mux` (custody until the facade port of #281) and `bboxybook` (Metal).
3. **One purchase, about $600 to $800: a second Mac mini M4** as the **postsubmit and night machine**: it runs the full `ci-local.sh --parity` postsubmit of [[landing]] §6, the e2e night and the scheduled scale run, with the e2e test accounts and file custody that never touch the operator's identity ([[d282-app-tests-itself-unattended]]). The nine agents keep this machine for presubmits. It pays for itself against Scaleway in four months and against MacStadium in five, has no 24-hour minimum, and sits on the LAN. The 24 GB model is the safer buy: a parity run and a night together want the room.

**Reason.** The question asked for the cheapest CI that actually runs. Step 1 is free and removes the largest repeated cost in every gate. Step 2 is the price of a coffee a week and is the only way the Linux backends get built at all. Step 3 is the only macOS capacity that is cheap: every rented or hosted Mac costs more in four to five months than the box, and the hosted minutes cost more in a week. Rejected: hosted macOS minutes (a), rented Macs (b) unless the operator wants no purchase, in which case a small host at about $50 to $130 is the fallback and Scaleway and AWS are not.

**What it changes in the record.** [[d15-local-ci]] stands for the agents' presubmits and gains the portable tier on the Linux box and the postsubmit on the second Mac; the sixth instance of [[no-chokepoints]] is amended from "no wrapper" to "no shared server": the cache is back with one server per agent. GitHub Actions stays manual-dispatch on the private repository and becomes the free CI of the **public** crates when [[d8-open-source]] publishes them, where hosted minutes cost nothing.

**Measured after the fact (the Oracle's 175-log parse, same day):** compile 52 %, test run 36 %, six binaries with real-clock waits over eight of thirty-seven hours. The order of the steps stands; the data adds that the second lever after the cache is the tests themselves ([[presubmit-tests-fast]]), which no purchase buys back, so the tests get fixed before the box is bought.

**Deferred by the operator, not rejected (2026-10-25, verbatim: "ok we continue as is for now, i will get more hardware for us soon").** Step 1 is live (the per-worktree cache landed at 4e2aadd5, the fixed slot paths follow). Steps 2 and 3, the Linux box and the second Mac, wait for the operator's hardware and stay the plan when it arrives; until then the swarm stays on this machine and the levers are the cache, the test policy and the presubmit's per-invariant selection.

**Operator's call.** Step 3 is a purchase; the Architect recommends it and does not make it. Steps 1 and 2 need no one's approval beyond this record.

Sources, read 2026-10-25: GitHub's December 2025 changelog on Actions pricing and the postponement; third-party price pages for hosted runners ($0.062 macOS, $0.006 Linux); Hetzner's June 2026 price-adjustment document and third-party CAX trackers; Scaleway's Apple silicon pricing page and a competitor's comparison for the 24-hour minimum; MacStadium and small-host figures from 2026 comparison posts; AWS Mac host figures from the same; Apple Mac mini M4 list and sale prices from price trackers; Cloudflare R2 pricing pages.

Owner: Corvin Duskmantle <ARCHITECT>.
