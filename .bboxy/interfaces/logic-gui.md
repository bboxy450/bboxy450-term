---
id: 01M5WG2WRQVKGWJ3AY8SFGBJH5
type: interface
number: 346
title: "ICD: logic ↔ GUI"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-29T04:52:48Z"
---
# ICD: logic ↔ GUI

The operator `SeatControl` action captures `{id, seat, generation, command}` at
menu opening. Logic validates the exact local open generation before an effect;
`AssignTask` additionally carries the captured `SwarmScope`, requires that swarm
to own the target, and uses the existing task delegation authorization and root
revision fences. `SeatControlled {op}` is correlated by operation ID; an unknown
kill outcome retains that ID and never becomes a new automatic kill. Kill uses
the supervisor's exact owned child attachment, keeps the seat/history, and is
distinct from reversible close. `UI_CONTROL_VERSION = 4` requires an old logic
owner to refresh before these controls are sent.

Live role change is the implementation default under the standing session
preservation order and the operator's continuation, not an answer to the pending
live-versus-restart question. It preserves the process/session, updates registry
authority and the next-launch role prompt, and sends one targeted operator notice
with the new instructions and a request to call `bboxy.orient` before the next
action. Existing direct-operator delivery applies even to a muted seat. Static
startup prompts apply on a later explicit Restart; they are not hot-replaced.
The role journal and notice use the same stable operation ID in separate stores;
replay never sends a second notice. A partial write or notice failure is reported
as unknown with that ID. Later explicit operator steering replaces this default.
The complete notice must satisfy the existing 16KiB message bound; an oversized
or invalid prompt refuses before any authority/context write, without truncation.

| | |
|---|---|
| Side A | logic (`bbt-mux`) |
| Side B | the window and headless client (`bboxy450-term`, `bbt-ui`, `bbt-render`) |
| The agreement | [[mux-daemon]] §Logic ↔ GUI state-stream protocol; [[l9-messaging-tasks]] §3 one events lane; the `bbt-proto` `gui/` nouns of #320 |
| Amended by | #328 a page read may wait; #340 the starting pane; #344 relaunch attaches; frame receipts and two sequence domains in [[ctl-cli]] §1a |
| What crosses | snapshots, deltas against an exact base, applied acknowledgements, history pages by cursor, seat state, one durable event lane per device |
| Timing | the next produced frame shows a launched seat's starting pane (T1) and an ingested echo (T2); watched seats get at least half the replay pages; the UI thread never waits on IO |
| States and lifecycle | exactly one GUI attachment per device; disconnect detaches and never deletes the consumer; a relaunched window attaches under the same consumer and restores layout, focus and scroll from the journal before its first frame |
| Failure | an unavailable baseline means a fresh snapshot; stale generations are ignored; a starting pane with no reply by the spawn deadline becomes a named refusal |
| Versioning | the wire golden proves byte identity; additive fields with defaults; one module per noun |
| Verified by | invariants ui-never-waits, watched-replay-priority, restart-keeps-state, seats-967-frame; `tests/gui_wire_golden.rs`; promises T1 to T3, S1 to S4, the K group, F1 and up |
| Open | the 967-seat frame is over budget (CPU p99 292 ms, input to GPU 1 076 ms against 8.33 ms) |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.
