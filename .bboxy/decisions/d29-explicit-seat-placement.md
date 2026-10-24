---
id: "01M5MCGGG8KGHEA4Z381B5ACMQ"
type: "decision"
number: 29
title: "Explicit seat placement on the GUI wire"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#29]]", rel: "supersedes-as-index" }
created: "2026-10-23T12:41:57Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 29: Explicit seat placement on the GUI wire

**Explicit seat placement on the GUI wire (2026-10-23, Nyx's read-only
seam proposal, accepted with two simplifications).** `SpawnSpec.placement`
optional, captured at the pick (`NewTab{tab}` or `Split{tab, target,
target_generation, side}`); `Spawn{id, spec}` with a caller-minted id,
`Spawned{op, seat}` and `Error{op}` correlate by it; pending placement
held in the immutable model until the matching `Spawned`; metadata
pushes never place; anchor gone or changed → seat kept, named partial
outcome, no silent split, no relaunch; reconciliation touches only
affected leaves. Simplified: the retry ledger is the existing control
op journal (the spawn id is an op_id), and the protocol boundary is the
GUI socket's existing admission `version`, bumped, no capability
negotiation. Rejected: resolving placement from focus at reply time
(the bug); a separate placement message after spawn (two round trips
and a window where the seat has no home).
