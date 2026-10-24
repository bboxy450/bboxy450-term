---
id: "01M5JQX1D8KVP28ASZF10Q9FBD"
type: "decision"
number: 18
title: "Claude Code may be an Executor when the operator launches it so"
status: "decided"
decided: "2026-10-22"
operator:
  - "i gave you one more executor that is claude code"
links:
  - { to: "[[2026-10-22-architecture-round-1#18]]", rel: "supersedes-as-index" }
  - { to: "[[mcp-control-plane]]", rel: "decides" }
created: "2026-10-22T21:22:33Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 18: Claude Code may be an Executor when the operator launches it so

**Claude Code may be an Executor when the operator launches it so
(operator, 2026-10-22: "i gave you one more executor that is claude
code").** The `~/.agents/AGENTS.md` default stands for sessions nobody
assigned; an explicit operator launch outranks it, as that file says.
Seraph Quillfire is the first. The terminal's harness-eligibility rule
([[mcp-control-plane]] Caller, role and grants) gains the same clause:
the operator can assign any role to any seat; the signed class in the
grant is what receivers trust. Rejected: editing AGENTS.md (the
operator's file; its override clause already covers this).
