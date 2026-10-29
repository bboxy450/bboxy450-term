---
id: 01M5WG2WRQVKGWJ3AY8SFGBJH5
type: interface
number: 346
title: "ICD: logic ↔ GUI"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-29T19:48:09Z"
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

The independent `MCP_CONTROL_VERSION = 7` adds the native task import/context/
reconcile catalogue. Normal same-account UI reattach replaces an owner
advertising version 6 or an absent MCP marker, retaining the supervisor, all
seat process IDs/generations/native session IDs and live PTY shell state. A
matching healthy owner is retained. `make test-task-migration` proves this with
private home/account/file custody, retained authenticated Oracle and worker
bridges, selected-root context and actual shell output. The fixture advertises
the older contract in its private owner record; it does not run an old binary
or authorize a live application restart.

UI control version 7 adds `NotificationForeground {frontmost}` for debug,
verified private test accounts (selector plus the admitted signed-run identity
label in a signing-ready state) with an active recording backend and no native
window. Every other context refuses before mutation. It and native focus events
invoke the same App focus transition, including blur effects and notification
sync; it never changes OS focus. The produced-frame `notification_frontmost`
observation is optional and confined to that private recording context. Recording
readback includes `frontmost` and retains the actual submitted/processed revision
fence; a frame reply does not claim the notification worker has settled. G2 waits
for that real completion before checking policy. Owners: Rook (contract), Sable
(GUI control/Settings proof), under live ICD201 and task231.

On the GUI door, a committed SeatControl returns the current authoritative
`GuiReply::Seat` before `SeatControlled {op}`. The GUI applies that metadata before
publishing completion, so its cached seat listing observes its own write without
waiting for event delivery. The prelude consumes no request slot; the pending
SeatControl retains the FIFO boundary for the next ordinary operation. The ctl
door receives only the correlated result, without an unsolicited Seat prelude.
Failure to read committed metadata reports unknown with the original operation ID.

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
| Open | issue73 selected existing-seam read-after-commit agreement below; implementation witness pending; the 967-seat frame is over budget (CPU p99 292 ms, input to GPU 1 076 ms against 8.33 ms) |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.


## Retained-backlog read after commit — 2026-10-29 (#388)

Producer Lumen Foxglove, GUI consumer Vesper Greywind (Oracle), contract owner
Nyx Copperwing. Reason: historical receipt backlog can hide a fresh committed
composer row while its actual canonical message and receipt already exist.
Selected agreement: [[l9-messaging-tasks]] “Retained-backlog read after known
commit” uses existing scoped Recent request/reply and canonical row validation,
triggered/coalesced after known send completion. Older pagination, historical
cursor/pending/explicit ACK/operator read marks, single durable Event lane and
all existing deadlines/fairness remain unchanged. Later receipt refresh uses
existing Event/read/state-change signals; it is not an unbounded polling loop.

No new MessageProjection variant, KB-root revision, source fields, tracking
protocol, wire version or budget is selected. Those in signed 9b3769a3 remain
unselected pending a real counterexample. Identity/snapshot/failure/restart and
retained-348000/16384 produced-frame/mutation witnesses are specified in #156.
Implementation supplement #447 and independently verified results remain open.

An authenticated, matched Recent item and canonical row with equal `bus: None`
may be displayed before event publication, preserving None and conferring no
Event admission, unread coverage, cursor/pending movement, read mark or ACK.
Keep ordinary Event stream fences: later actual authorized publication merges
by canonical message ID/context and its real position without duplicate rows or
loss of confirmed signature/ACK evidence; require an actual canonical-None →
actual-Event production witness and a negative control restoring the old None
rejection before implementation acceptance.
