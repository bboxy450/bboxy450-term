---
id: 01M5T01PFCYWGS7PC1TAS4YGHS
type: decision
number: 328
title: "a page read may wait: EventsRead gains a bounded wait for the next append, so ctl waits on seat and subagent state event-driven without a second consumer, a new lane or a predicate in the bus"
status: decided
decided: "2026-10-25"
operator: []
links:
  - { to: "[[l9-messaging-tasks]]", rel: decides }
  - { to: "[[ctl-cli]]", rel: decides }
  - { to: "[[d270-ctl-screen-input-wait]]", rel: amends }
  - { to: "[[d320-dispatch-by-noun]]", rel: builds-on }
  - { to: "[[presubmit-tests-fast]]", rel: builds-on }
  - { to: "[[no-chokepoints]]", rel: builds-on }
created: "2026-10-25T16:59:35Z"
updated: "2026-10-25T16:59:35Z"
---
# Decision 328: a page read may wait

Ruled at the Oracle's ask from Sable's T8 finding. `ctl` must wait, event-driven, for a seat or subagent state (idle, waiting, active, a reported grade). Today `seat wait` polls with a 200 ms sleep (`ctl.rs:1752`); `AttachEvents` is refused while the device's one durable drain is held by the headless owner (`bus.rs:1014`); `Watch` pushes viewport updates only. The single-consumer rule of [[l9-messaging-tasks]] §3 is not on the table.

**Alternatives.** (a) A read-only, non-durable seat-state observer streaming `GuiReply::Seat` on a dedicated `ctl` path: a second push lane, and a carve-out in the single-consumer rule for it. (b) A narrow blocking wait action, predicate plus deadline, answered by the bus: puts the meaning of "idle" and "reported" into the bus, which knows streams and not seats. (c) **The browse rule, extended by one word: a page read may wait.** `EventsRead { stream, after, limit }` gains an optional `wait_ms`; with it the read blocks until at least one event exists after the cursor or the deadline passes, then returns the page as today with `next_since`. `ctl seat wait` becomes a loop of waiting reads on the seat's state stream with the predicate evaluated in `ctl`; the sleep goes.

**Choice.** (c).

**Reason.** A waiting page read is still a page read: no ack, no durable position, no lane, so the single consumer stays single and the rule needs no carve-out. The predicate stays where the vocabulary is, in `ctl`, and the bus keeps knowing only streams. The wire change is one optional field with a default, the smallest possible under the [[d320-dispatch-by-noun]] golden: every existing case stays byte-identical and one case is added. (a) builds a lane to avoid a field; (b) moves seat semantics into the wrong process. The shape is the one the ICQ inbox already uses: `inbox --after SEQ --wait N`.

**What holds.** (1) One durable consumer per device, the GUI lane, unchanged; a waiting reader holds no cursor row, no lock and no bus slot, only a wakeup registration. (2) The appender's cost is one broadcast notify per stream append, never per-reader work, so a slow or absent reader costs the bus nothing ([[no-chokepoints]]). (3) `wait_ms` is bounded, at most 60 000, and `ctl` loops; a wait that ends by deadline returns the empty page and `next_since` unchanged, exit 0 from the read and the `timeout` kind from `seat wait` when its own deadline passes. (4) Order and cursor semantics are the page's: events after `after`, in order, `next_since` on the last line. (5) Waiting produces no event and writes nothing. (6) The field is additive with a default of no wait; the golden gains a case. (7) `seat wait --state` and `--text` drop their sleeps, which also clears their [[presubmit-tests-fast]] B1 finding; `--text` still reads the screen by `frame_seq`, so it waits on the seat's state stream for a frame advance when the stream carries one and polls the screen only when it does not, at the current cadence, named as such.

Owner: Corvin Duskmantle <ARCHITECT>.
