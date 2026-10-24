---
id: "01M5N0BAYRG8W98057TXVZJ31A"
type: "decision"
number: 37
title: "Messaging and tasks reach the app through two wrapper variants on the GUI wire and one events lane"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#37]]", rel: "supersedes-as-index" }
  - { to: "[[l9-messaging-tasks]]", rel: "decides" }
created: "2026-10-23T18:28:39Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 37: Messaging and tasks reach the app through two wrapper variants on the GUI wire and one events lane

**Messaging and tasks reach the app through two wrapper variants on
the GUI wire and one events lane (2026-10-23, Oracle's ask for L9-M).**
`GuiAction::Messaging {{ id, request }}` wraps the whole W2.4 API and
`GuiAction::Task {{ id, request }}` mirrors the W2.5d tool arguments;
the W2.4 request and wire types move to `bbt-proto::messaging` (as [[d6-shared-protocol|#6]]
did for streams); logic dispatches through the same functions the MCP
tools use, under `Operator {{ device_id }}` ([[d31-device-bound-operator-journal|#31]]), journaled and replayed
by id; every push to the app, messages, receipts, task status, seat
state, artifacts and audit, rides Kestrel's W2.6b lane from one durable
consumer per device, and unread is derived from that cursor. Spec
[[l9-messaging-tasks]]. Rejected: one GUI variant per tool (twenty
variants that drift from the tools); a second validation path for GUI
callers (two authority models); a separate message stream beside the
events lane (two cursors, two reconnect stories); storing unread counts
(a second source of truth).
