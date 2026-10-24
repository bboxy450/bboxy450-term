---
id: "01M5N1M9G8YZNAHG7RX5RRNGD0"
type: "decision"
number: 38
title: "L9-M implementation rulings"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#38]]", rel: "supersedes-as-index" }
created: "2026-10-23T18:51:01Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 38: L9-M implementation rulings

**L9-M implementation rulings (2026-10-23; Kestrel's five, Nyx's four).**
(a) Operator-originated messages and tasks are signed by the device key
with `seat = nil`, `role = operator (Oracle class)`, `role_grant_id` the
DeviceGrant; `mcp::Actor` gains the operator variant; `delegated_by:
operator:<device_id>` is a reference kind; no synthetic seat. (b) The
GUI task wire equals the W2.5d tool arguments as built; `spawn_spec` is
off the wire (spawn then delegate). (c) Events: one consumer per device
with per-stream cursors, `Event{stream, seq, …}`, `EventAck{stream,
seq}`, named `Gap`; ack after publish, never on enqueue; unread is
operator-read from a durable read-mark in the account object, not the
consumer cursor; `operator/inbox/*` is a new retained namespace while
`inbox/>` stays fanout; one active attachment per device, filter change
= versioned name; `Start::New`, read-through history, bounded quanta.
(d) Projections `MessageRow`, `TaskCard`, `TaskReport` in bbt-proto with
the work-items status set canonical and the lab adopting its names;
`p1` is worker-class priority ≥ 7 on the 0–9 scale (restated from a
wrong 0–255 threshold the same day). (e) Residuals from Nyx: under
local-dev the operator's `role_grant_id` is the digest of the persisted
first-launch record `identity/local-issuer` and `role_epoch` its epoch
(corrected the same day from `identity/issuer`, which exists only after
a cutover or reset; Nyx), never a minted grant, no IssuerState created; the operator destination is the sink `operator/inbox` in every
swarm tree with recipient reference `operator:<device_id>`, delivering
to the retained `operator/inbox/message`; replies address the same sink. (f) Every GUI messaging request names
one `swarm_id`; `Send` and `Receipts` must agree with the swarm inferred
from destination or message id; no account-wide union on the wire, the
GUI composes per-swarm pages and per-swarm read-marks (rejected: an
invented cross-swarm cursor; picking the first swarm). (g) A filter
change while the GUI is attached migrates, never restarts (Kestrel,
2026-10-23): the new versioned consumer is created first, inherits the
old one's per-stream anchors and pending rows, then the old is removed;
the next bounded poll drops pending rows for revoked subjects without
projecting them, newly granted subjects start at head with read-through
history, and the migration is one audited line on `account/control`
(old and new version, subjects added and removed, rows dropped), no new
record type (rejected: new = head plus unsubscribe, which silently loses
still-authorized pending rows; `Start::All` for the new version).
(h) The composer's role chip fans out at send time (Oracle, L9-M-c,
2026-10-23): the W2.4 tree has no role sink, so a role resolves to the
exact seats holding it in the current registry membership, one
`Messaging` request per seat with its own id, receipt and ledger row,
shown as one role send with N receipts; refused with a named reason
above 32 seats (rejected: an invented role path or a new tree sink;
groups remain their own destination from `Ls`). (i) The operator
principal alone lists open unassigned tasks, read-only and account-wide
(Oracle, L9-M-c, 2026-10-23): `TaskRequest::List { unassigned: true }`,
path, title, status, no swarm, paged 128; seats keep the existing
refusal; `Delegate` scopes by the chosen assignee. Reason: an unassigned
task has no swarm scope under [[d28-structured-task-verification|#28]] and the operator is the account
principal (rejected: a seat-visible unassigned list; a per-swarm
unassigned pool).
Rejected: an app-owned cursor tail instead of a durable consumer (loses
restart-resume); ack on write (loses rows on a crash between poll and
socket); unread from the consumer cursor (a restart would mark everything
read, and human reading would throttle redelivery to `max_deliver`);
making `inbox/>` retained (changes every seat's delivery semantics for the
operator's convenience); a native label-mapping table for the lab's
`assigned`/`done` (two vocabularies for one state).
