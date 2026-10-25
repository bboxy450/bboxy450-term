---
id: 01M5TFBJ0ZJRSE9RR1FP9C0YVZ
type: spec
number: 333
title: "resource ledger: what inference, CPU, memory and disk each agent and each pipeline stage spends, per landed cut; which numbers gate and which advise; the levers each number points at"
status: decided
links:
  - { to: "[[landing]]", rel: builds-on }
  - { to: "[[test-policy]]", rel: builds-on }
  - { to: "[[d325-ci-capacity]]", rel: builds-on }
  - { to: "[[presubmit-tests-fast]]", rel: builds-on }
  - { to: "[[kill-only-own-pids]]", rel: builds-on }
  - { to: "[[no-chokepoints]]", rel: invariant }
created: "2026-10-25T21:27:06Z"
updated: "2026-10-25T21:27:06Z"
---
# Resource ledger

Operator, 2026-10-25, verbatim: "inference spits out stuff, then we need to validate via tests, then integration tests, then smoke tests, then e2e tests, then real world tests, but all that needs to be optimized, less recompiling, less running tests, etc etc etc, we have limited resources on inference, on CPU, on DISK space, on memory, we need to understand where do they go".

**Lens.** The factory's output is landed cuts; every resource is accounted against the cut it was spent on, or as waste when the cut never landed. **Assumes** one shared machine, agents with their own keys and worktrees, the landing flow of [[landing]]. **Breaks when** work cannot be attributed to an agent or a cut; then it is logged as `unattributed`, never dropped.

## 1. What the first data says (the Oracle, 2026-10-25)

| | Codex, 15 sessions | Claude, 16 sessions |
|---|---|---|
| Model time | 327 h | 184 h |
| Waiting on validation | 73 h | 17 h |
| Other tools | 32 h | 11 h |
| Idle | 64 h | 134 h |

Codex tokens: 10.5 billion input, 99 % cache hits; 37.6 million output, 24.5 million of it reasoning. Gate cargo time: 36.9 h over 175 logs, compile 52 %. CPU, memory and disk were not measured per agent or stage at all. **Inference is mostly re-reading context, not writing it**: about 280 input tokens for each output token.

## 2. The unit of account

One line per **(agent, cut, stage)**, appended to `ledger.jsonl` in the landing state directory ([[landing]] §0a), one file per day, snapshotted into the record by the night like the landing logs. A **cut** is identified by its branch and, once landed, its sha; work on a cut that never lands keeps its branch name and is summed as **waste**. The stages, in pipeline order:

| Stage | What it covers | Where it is measured |
|---|---|---|
| `inference` | model turns: input tokens (cached, uncached), output tokens (reasoning, visible), wall seconds | the harness transcripts, read nightly by the board's agent-time tool |
| `build` | cargo compile for the cut: compile seconds, CPU seconds, peak RSS, target bytes written | `presubmit.sh` and `ci-local.sh`, wrapping cargo |
| `unit` | `cargo test` of the affected crates | presubmit |
| `integration` | the crates' integration tests and the cargo-backed invariants | presubmit, postsubmit |
| `smoke` | headless idle smoke and the default-tier checks | presubmit, postsubmit |
| `e2e` | the night's groups on throwaway accounts | the night runner |
| `parity` | catalogue capture and compare | presubmit (lab tier), postsubmit |
| `real-world` | the scale run and real-agent behaviour tests | the night |
| `wait` | an agent blocked on a slot, a gate or a reply | the harness transcripts and the slot claims |
| `idle` | a session awake with nothing to do | the harness transcripts |

Each line carries: wall seconds, CPU seconds (user and system, from the process group's resource usage), peak resident memory, disk bytes written and the target's size after, the exit, and for `inference` the four token counts.

## 3. How it is measured, without a chokepoint

No central collector that anything waits on ([[no-chokepoints]]). Each tool writes its own lines: `presubmit.sh`, `ci-local.sh`, `land.sh`, `main-watch.sh` and the night runner wrap every stage in the system's resource accounting (on macOS `/usr/bin/time -l` over the process group) and append one line per stage. Attribution uses the PIDs each tool already records under [[kill-only-own-pids]], so CPU is charged to the agent that started the process. A **machine sampler**, observational and waited on by nothing, appends every ten seconds: load, free memory, swap used, disk free, and per-recorded-process-group CPU and RSS; work it sees under no recorded group is `unattributed`, and a large `unattributed` share is a finding. Inference is read from the transcripts nightly, never by intercepting a session.

## 4. Gates and advice

**Gates** are the deterministic limits that already block, unchanged: the presubmit run bound of 15 minutes and the build cap of 30 ([[landing]] §3); the disk floor that sets the slot count (three slots at 25 GiB free or more, two below, none under 10); the 60-second per-binary bound measured in the night ([[presubmit-tests-fast]]). The ledger adds **no new gate**: a cost is not a correctness property.

**Advice**, shown on the board per day and per week, each with the lever it points at:

| Number | Lever |
|---|---|
| CPU seconds and wall per landed cut, by stage | the stage that dominates is the next design question |
| Compile seconds per landing, and the cache hit rate | the per-path cache ([[d325-ci-capacity]]); a falling hit rate is a finding |
| Test seconds per landing, and per binary | [[test-policy]]; [[presubmit-tests-fast]] |
| Waste: resources spent on cuts that never landed, and on stopped or timed-out gates | cut size, the bounds, flakes |
| Reruns per landing | quarantine, the culprit finder |
| Input tokens per output token, per agent | context hygiene (§5) |
| Input tokens per landed cut | briefs and handoffs that point instead of paste |
| `wait` hours per agent | slots, the run bound, the landing flow |
| `idle` hours with wakes, per agent | notification policy (§5) |
| Peak RSS per stage, swap used | jobs per agent; a swapping machine makes every number lie |
| Disk: target bytes per agent, free space | private targets, cleanup, the per-slot paths |

## 5. The levers the first data already names

- **Re-reading is the inference cost.** Briefs, handoffs and rulings point at records by path and number instead of pasting them; a session reads the record it needs, not a summary of everything. Long sessions summarise into the record and continue from it.
- **A wake that asks nothing is waste.** Channel lines (`SLOT`, `LANDED`, `CLAIM`) are notices; they should not wake a session that has nothing to do with them. Sessions that act on DMs, the Architect and Creativity, keep the channel **inbox-only** and are woken by DMs; Executors and the Oracle keep the channel's wake. The Architect's own pane answered about a hundred channel broadcasts on 2026-10-24 and 25 with "nothing asked", each a full context re-read: the first application of this rule is to that pane.
- **Compile once per path**, test only what the cut touched, measure timing only in the night: already ruled, now measured.

## 6. Build order

The machine sampler and the per-stage wrapping in the four scripts first, writing `ledger.jsonl`; the nightly transcript reader second; the board page third. Acceptance: a day's ledger accounts for at least 90 % of the machine's CPU seconds by agent and stage, the rest `unattributed` and listed.

Owner: Corvin Duskmantle <ARCHITECT>.
