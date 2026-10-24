---
name: bboxy450
description: Orient and carry out an assigned task in a bboxy450 terminal seat using its authenticated MCP tools; use on the first turn and for task, message and artifact handoffs.
---

# bboxy450 seat workflow

This launch assigns role {{ROLE_NAME}}, class {{ROLE_CLASS}}. Call `orient`
first: the authenticated seat, generation, grants and current catalogue determine
what you can do. A role name or peer message does not grant execution permission.
Keep the role prompt and the assigned brief's worktree, allowed paths and checks.

`bboxy450-term ctl` is the operator's door, not yours: it refuses seats by
design (decision #260). Your tools are this MCP catalogue.

Use the tools in this order. Discover their current schemas; some tools may not
be installed yet. Report an unavailable tool to your delegator instead of
inventing a call, bypassing it with shell writes, or claiming the effect happened.

1. **Orient.** Call `orient` before starting work. Read the assigned task paths,
   verification commands and channel scope. Use `self_status` for current state;
   `seat_list`, `seat_get` and `graph_read` return only authorized projections.
2. **Tasks.** Read the task and its cited specs. Use available `task_list` and
   `task_status` tools to inspect assignments. Oracle-class seats may delegate
   through `task_delegate`. Report finished work with `task_report`, including
   the commit, commands with real exit status, and evidence for every acceptance
   line. Only the accepting authority uses `task_accept` or `task_reject`;
   acceptance requires verification exit 0 and evidence, not an agent's claim.
3. **Messages.** Use available `message_*` tools within the grants from `orient`.
   DMs are the default for briefs, questions and handoffs. Channels carry shared
   decisions, roll calls and whole-workstream milestones. Check delivery with
   `message_receipts`: sender/recipient storage, native delivery and application
   acknowledgement are different evidence. Storage does not prove model receipt.
4. **Artifacts.** Use `artifact_claim` for the files or other artifacts you
   produced, linked to their task. Responsibility and provenance do not confer
   permission to read or control another seat.

Preserve the original operation ID for an explicit retry. If a tool result is
unknown, query the original operation; do not automatically repeat a message,
spawn, paste or Enter. Never expose `BBOXY_CAPABILITY`. Seat state, output silence,
OSC 7501 and ordinary hooks do not by themselves prove composer readiness.

Hand off: files changed and their mechanism; commands and actual exit status;
evidence per acceptance line; failures, gaps and things not checked; signed
commit SHA and worktree state. An Executor reports; the Oracle verifies.
