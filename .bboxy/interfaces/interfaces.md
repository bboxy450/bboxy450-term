---
id: 01M5WG2WSDBY7AH6A8Q037H9ZH
type: spec
number: 354
title: "interfaces: the nine boundaries of bboxy450-term, each with an ICD, and the verification matrix from interface to evidence"
status: baselined
links:
  - { to: "[[budgets]]", rel: builds-on }
  - { to: "[[e2e-script-map]]", rel: builds-on }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T16:18:20Z"
---
# Interfaces

Operator, 2026-10-26, after reading how large engineering programmes handle integration: "update architect promit in global agent settings, do all the improvement for this process". The Architect is now the lead systems engineer (`~/.agents/AGENTS.md`): it owns the interactions between parts, the shared [[budgets]], and the evidence the whole works. These are the boundaries and what proves each.

**Why now.** Most of the Architect's rulings on 2026-10-25 were interface questions Executors hit mid-implementation and got answered one at a time: two frame counters with one name, what #279 isolates, how a relaunched app finds the survivors, what "within one frame" counts from, a dozen messaging contract points. Each was one line in an agreement nobody had written.

## The verification matrix

| Interface | Verified by | Open |
|---|---|---|
| [[supervisor-logic]] #345 | invariants checkpoint-bounded, ring-reads-batched, no-seat-starves, restart-keeps-state, kill-only-own-pids, seats-400-memory-idle; promises T5, T9, S1 to S4 | T5 find-and-attach and window-state restore (#344), in work |
| [[logic-gui]] #346 | invariants ui-never-waits, watched-replay-priority, restart-keeps-state, seats-967-frame; `tests/gui_wire_golden.rs`; promises T1 to T3, S1 to S4, K1 to K4, F1 and up | the 967-seat frame is over budget (CPU p99 292 ms, input to GPU 1 076 ms against 8.33 ms) |
| [[ctl-app]] #347 | invariants ctl-opt-in, ctl-never-prints-secrets; the ctl library tests; every night script drives the app through it | the operator's attention setter arrives with MA-8 |
| [[app-harness]] #348 | promises L3, L4, T6, T9; the native integration proof on installed clients | a real model's choice to orient is advisory only, never a verdict |
| [[messaging-seats]] #349 | invariants messaging-never-held, notices-never-wake, seats-1000-bus; promises M1 to M6 | MA-5 to MA-9, in work |
| [[plugin-host]] #350 | `plugin check` on its fixture store; contract conformance fixtures; promises G3, G4, D1 and up, N1 and up | the signed-writer bridge for external providers |
| [[identity-app]] #351 | invariants messaging-never-held, ctl-never-prints-secrets, signed-main; promises I1 to I4 | the native path's port onto the keychain facade, deferred |
| [[lab-native]] #352 | invariants parity-geometry, modules-through-components; `scripts/generated-sources.sh`; promises R1 to R6 | none |
| [[night-app]] #353 | invariant e2e-runs-unattended; every promise | 50 of 55 promises partial on 2026-10-25 |

## How it is used

- **Before a cut touches a boundary**, its brief names the ICD; the Executor builds against it.
- **A question about a boundary** is answered by adding the line to the ICD.
- **A change to a boundary** names both sides, the reason and the verification, and lands only when both owners agree.
- **A cross-boundary bug** is the Architect's to follow until it has an owner and a disposition, recorded in the ICD's Open row.
- **Status** is the matrix: which interfaces have every promise full, which are open.

Owner: Corvin Duskmantle <ARCHITECT>.
