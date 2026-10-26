---
id: 01M5WG2WRM4Q254SM76XDKJY39
type: interface
number: 345
title: "ICD: supervisor ↔ logic"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T18:31:26Z"
---
# ICD: supervisor ↔ logic

| | |
|---|---|
| Side A | the supervisor process (`bbt-supervisor`) |
| Side B | logic (`bbt-mux`) |
| The agreement | [[mux-daemon]] §Supervisor ↔ logic protocol and §Restart and reattachment |
| Amended by | #253 checkpoint policy; #314 retry rotation; #318 batched ring reads; #319 kill only own PIDs; #344 relaunch attaches |
| What crosses | PTY records in one observed order per seat; input with stable ids and receipts; resize; checkpoints with exact cursors; ring history by cursor, batched at most 8 seats and 768 KiB per request |
| Timing | a seat's output reaches logic within one frame interval of the supervisor writing it, 8.33 ms ([[budgets]]; measured 1 349 ms on the first night run, b61ea30b, the dominant leg of input to GPU); ring reads cost at most ceil(N/8) round trips per pass; parsed debt at kill at most 16 MiB and replay under 71.7 s; per-seat write exclusion until a receipt is final |
| States and lifecycle | exactly one logic lease mutates supervisor state; the supervisor survives GUI and logic death; a replacement logic takes the lease through the replacement path; a relaunched app attaches, never starts a second supervisor (#344) |
| Failure | per-member Gone, Stale, Busy, Limit, never an oversize frame; a stale lease is fenced; an unknown write outcome is kept as unknown and never retried automatically; every kill is of a recorded own child |
| Versioning | versioned, length-framed, limits negotiated in Hello; additive fields only |
| Verified by | invariants checkpoint-bounded, ring-reads-batched, no-seat-starves, restart-keeps-state, kill-only-own-pids, seats-400-memory-idle; promises T5, T9, S1 to S4 |
| Open | T5 find-and-attach and window-state restore (#344), in work; supervisor-write to logic-read latency 1 349 ms against 8.33 ms |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.
