---
id: "01M5M727BG0HWSSRDS7ZQVFNE3"
type: "decision"
number: 28
title: "verify_result is structured and task_accept checks it field by field"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#28]]", rel: "supersedes-as-index" }
created: "2026-10-23T11:06:46Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 28: verify_result is structured and task_accept checks it field by field

**`verify_result` is structured and `task_accept` checks it field by
field (2026-10-23, Oracle, W2.5d).** `verify_result = {command,
exit_code, evidence: [{criterion, reference}]}`. Acceptance requires
`command` equal to the task's `verify`, `exit_code` 0, and, when the
task lists acceptance criteria, one evidence entry per criterion. Task
swarm scope (exact wording from W2.5d, Kestrel via the Oracle): every
present seat reference among `assignee` and `delegated_by` must resolve
to the caller's registry swarm; one alone anchors, two must match each
other, none is ambiguous and refuses; never from a path. A fresh
`assignee: none` task with `delegated_by: seat:<Oracle>` is delegable. Rejected:
a free-text evidence blob (nothing to check against); trusting the
reported command string without comparing it to `verify` (a passing
`true` would accept anything); swarm from the task's path (paths move,
registry rows do not).
