---
id: 01M5WG2WRM4Q254SM76XDKJY39
type: interface
number: 345
title: "ICD: supervisor ↔ logic"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-31T18:21:00Z"
---
# ICD: supervisor ↔ logic

| | |
|---|---|
| Side A | the supervisor process (`bbt-supervisor`) |
| Side B | logic (`bbt-mux`) |
| The agreement | [[mux-daemon]] §Supervisor ↔ logic protocol and §Restart and reattachment |
| Amended by | #253 checkpoint policy; #314 retry rotation; #318 batched ring reads; #319 kill only own PIDs; #344 relaunch attaches |
| What crosses | PTY records in one observed order per seat; input with stable ids and receipts; resize; checkpoints with exact cursors; ring history by cursor, batched at most 8 seats and 768 KiB per request |
| Timing | a seat's output reaches logic within one frame interval of the supervisor writing it, 8.33 ms ([[budgets]]; measured 1 349 ms on the first night run, b61ea30b, the dominant leg of input to GPU); seat read/parse completions progress independently (#47 amendment below); parsed debt at kill at most 16 MiB and replay under 71.7 s; per-seat write exclusion until a receipt is final |
| States and lifecycle | exactly one logic lease mutates supervisor state; the supervisor survives GUI and logic death; a replacement logic takes the lease through the replacement path; a relaunched app attaches, never starts a second supervisor (#344) |
| Failure | per-member Gone, Stale, Busy, Limit, never an oversize frame; a stale lease is fenced; an unknown write outcome is kept as unknown and never retried automatically; every kill is of a recorded own child |
| Versioning | versioned, length-framed, limits negotiated in Hello; additive fields only |
| Verified by | invariants checkpoint-bounded, ring-reads-batched, no-seat-starves, restart-keeps-state, kill-only-own-pids, seats-400-memory-idle; promises T5, T9, S1 to S4 |
| Open | T5 find-and-attach and window-state restore (#344), in work; supervisor-write to logic-read latency 1 349 ms against 8.33 ms |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.

## Independent seat ingestion — operator brief #47, 2026-10-30

Owners: supervisor capture/client (`bbt-supervisor`) and logic ingestion
(`bbt-mux`). The direct operator brief authorizes this amendment on both sides.
Reason: an aggregate history reply and synchronous parsing on the logic owner
couple every seat to unrelated output/history work. This supersedes #318's
aggregate hot-sweep round-trip count; the batched wire API remains compatible.

Each seat has one outstanding ring read and one bounded parse job/result, never
two cursor writers. Read requests carry the exact attachment, generation and
from/to cursor and complete independently; a held seat does not occupy another
seat's completion slot. Capture refusal or expiration leaves its cursor unchanged and is local to that
seat. Typed provider-down admission remains a host park/resume signal, never a permanent per-seat IO refusal. An expired socket capture keeps its charged correlation slot until the
late reply drains; it never closes healthy reads or reuses a late reply ID. Notifications
remain ring-backed and coalesced per seat/generation/kind; no tail is dropped.
The existing 64 KiB history page limit and checkpoint debt limits remain in force.

The native control port admits without waiting for ring IO. A socket keeps eight
independent capture requests plus the ordinary control request; correlated replies
may arrive in either order under the same lease. The socket reserves a largest
legal frame for each of these nine replies, retaining the original event byte
capacity (under 3 MiB): total queue capacity is under 12 MiB plus framing. Full
admission returns Busy before sending. Capture frame writes are nonblocking and
retain at most eight largest legal outgoing frames (under 8 MiB plus framing);
an ordinary control request refuses Busy while a partial frame owns the wire,
never interleaving bytes or waiting for the peer. Live reads retry with fair retry by the existing 3:1
watched/background seat scheduler. The owner services at most 128 fair seat
attempts or a 1 ms service quantum before admitting fresh commands/notifications;
new read/parse admission counts as progress before cursor publication. Busy ring
reads have a seat-local monotonic retry deadline rather than a global sleep. No IO mutex is retained while awaiting a
capture result, including checkpoint publication. The supervisor's existing
128 pending capture slots and per-seat capture/durability rules are unchanged.

A seat's bounded parse task reads ordered records from an immutable checkpoint
and returns the next playback and effects to the logic owner. Publication checks
the exact attachment, view incarnation and starting cursor. Cancellation, owner
restart or generation replacement discards old completions; a reused display
identity cannot admit an earlier incarnation (A → B → A). Live query/OSC effects
occur only after current publication; replay suppresses them. Cross-seat registry,
bus and GUI publication remain on the logic owner; they do not await a seat read
or parser. This is not a wall-clock or arbitrary filesystem progress proof.

GUI history paging has its own bounded per-seat read completion, outside output
parse turns. It cannot hold the live ingestion owner while a ring page is pending.
Shutdown drops uncommitted parse work and retains the last applied checkpoint;
retained ring replay recovers the unapplied suffix without replaying live effects.
Verification: `make test-seat-ingestion`, `make test-formal`, real terminal-reply
and lifecycle witnesses, and quiet `make profile-ingestion`. The finite model
assumes fair runnable seat/task service and eventual healthy-seat read completion;
its deliberately shared-queue control must fail healthy-seat progress while a
peer read is held. Timing remains 8.333333 ms ingestion and 16.666667 ms headless
input-to-GPU; a failed measurement leaves #47 open.

The headless profiling observer forwards independent capture admission on the
same already admitted connection; instrumentation must not substitute a blocking
`request()` fallback. Its held-reply witness rejects that substitution. On a
socket, nonblocking polls drain available eventless reply fragments within a
bounded 128-read/1 ms service quantum; one partial frame must not consume a whole
logic turn. Reply IDs, lease ownership, endpoint matching and byte order remain
unchanged. The first replay record is observed before checkpoint admission;
later records are observed in parser order, without pre-scanning a future probe.

CPU allocation for this boundary: at most four independent parser lanes, capped
at one less than the runtime's worker count (minimum one). A lane owns only
bounded CPU parsing, never a ring/history/checkpoint wait, and yields after
8 KiB or 32 records. On the ten-worker profile this leaves six runtime workers
available to control/bus IO; this is a concurrency allocation, not CPU affinity
or a wall-clock proof. Parse spans include lane admission wait. Each seat still
has only one pending parse job/result; lane queues are bounded by that seat cap
and the existing maximum seat count. Healthy lane/task service is assumed fair.

Scheduling versus completion (#47): the 3:1 watched/background quota remains on
runnable owner turns. A pending read/parser job cannot admit another page; its
wait donates completion capacity without blocking another seat. Independent RPC
completion order is not a scheduling-round count. `test-watched-replay-priority`
checks the exact eight-turn queue floor separately from the real fixed-budget
watched-tail and serial-screen witness; see invariant watched-replay-priority.
