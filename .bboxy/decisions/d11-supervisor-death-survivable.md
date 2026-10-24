---
id: "01M5JFXASG4SCZEJ9A9ZB8YWFF"
type: "decision"
number: 11
title: "Supervisor death is survivable, not prevented"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#11]]", rel: "supersedes-as-index" }
created: "2026-10-22T19:02:54Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 11: Supervisor death is survivable, not prevented

**Supervisor death is survivable, not prevented.** The supervisor stays its
own process. If it dies: the app restarts it, re-attaches, and every seat
resumes by harness session id (`claude --resume`, `codex resume`) with the
ring replayed; the UI shows the restart. Landed 2026-10-23 (C1c-1):
the launch planner is the single resume path; the trait's
`resume_command` is dead and goes in C1c-2. Unflushed capture bytes are lost
and that is accepted; no third process, no shared-memory mirror. Within
reason: velocity.
