---
id: 01M5PGDJES80H20WTSXGY9YZAP
type: decision
number: 266
title: "orient says what this seat can do now; empty views say why; ids are minted for the caller; destinations are listed with their rights; knowledge tools read and write records; a count is never null"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[mcp-control-plane]]", rel: decides }
  - { to: "[[knowledge-base]]", rel: builds-on }
  - { to: "[[w2-5-mcp-and-seat-wiring]]", rel: decides }
  - { to: "[[d264-reply-routes-to-sender]]", rel: builds-on }
  - { to: "[[e2e-acceptance]]", rel: evidence }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-24T08:28:43Z"
updated: "2026-10-24T09:12:06Z"
---
# Decision 266: orient says what you can do

The first in-app agent feedback under [[e2e-acceptance]], 2026-10-24: a real Claude seat Vesper drove through `ctl`, verbatim in `.bboxy/runs/e2e-feedback-claude-1.log` (main ed5cfdc7). Its six findings, in its words, and the ruling on each. The lens is [[mantras]] 20 and 21: useful information, never dumb information; always be useful. The seat is the user here.

**1. "I have to make up op_id ULIDs myself, with no helper and no guidance on whether reuse means idempotency."** Ruled: the tool server **mints `op_id`** when the caller omits it and returns it in the result; a caller-supplied `op_id` is honoured for **idempotent retry**: the same `op_id` with the same arguments returns the first result again, the same `op_id` with different arguments is refused as `op_id_reused`. The catalogue states this on every `op_id` field; the skill says "omit op_id unless you are retrying". Rejected: a separate `mint_id` tool (one more call to learn about).

**2. "task, markdown and execution graphs and task_list all came back empty, so I couldn't tell "nothing assigned" from "not visible to a worker"."** Ruled: **no bare empty result**. Every list and graph result carries `scope: { seat: <ULID>, role: <role name>, class: <serialized class> }` and, when empty, `empty_reason`, one of `nothing_assigned | not_visible_to_role | no_grant | no_swarm | nothing_yet`, and `empty_message`, one line of text (field names fixed at Aster's ask, same day). A worker that sees nothing is told it is a worker seeing nothing, not left to guess.

**3. "ch_ls operator was rejected (no read right), but message_send to operator/inbox is apparently allowed. Write-only destinations aren't listed anywhere."** Ruled, with [[d264-reply-routes-to-sender]]: `orient` carries **`destinations`**: every sink and channel the seat may address, each marked `read`, `post` or `write-only`; `operator/inbox` appears for every seat as `write-only`; `ch_ls` on a write-only destination refuses with "write-only; it is in your destinations", never a bare "no read right".

**4. "No document tool: artifact_claim records a claim and cannot read or write."** Ruled: the knowledge base contract becomes seat tools, **`knowledge_fetch`, `knowledge_search`, `knowledge_write`** ([[knowledge-base]] §2, K-0 and A-0 already building `fetch` and the e2e write). `artifact_claim` stays claim-only and says so in its description. Write authority by record type and class: a worker-class seat may **create and update** `e2e`, `comment`, `research`, `note` and `artifact` records, and update the `result` fields of its own `task`; `spec`, `decision`, `invariant`, `proposal` status changes and `task` creation stay with the Oracle class and the operator; the Index signs every write with the calling seat's key and refuses outside the table by name.

**5. "unread_count was null ("no read cursor") and pending_application_ack was 1, which made me guess I'm meant to ack."** Ruled: **a count is never `null`**. `unread` is `{ count, basis }` where `basis` is `cursor` or `no_cursor_yet` (meaning: you have read nothing; everything is unread); `pending_ack` is `{ count, action: "call message_ack for each id listed" }` with the ids. The seat is told what the number means and what to do.

**6. "Change first: orient should say what this seat can do now, listing the send destinations and the reason each graph is empty, and should mint op_ids for me."** Ruled, as the shape of `orient` from now: who I am (seat, name, role, class, swarm, generation); what I can do now (the tools open to my class, one line each); my destinations with their rights (3); my tasks and the empty reason if none (2); pending acks and unread with their basis (5); how to reach the operator (#264); and "ids are minted for you unless you retry" (1). One call, one answer, enough to act on cold, which is what `orient` promised in [[mcp-control-plane]].

**Rejected across the six.** Better documentation without tool changes (the seat read the documentation we had); a verbose `orient` that lists every tool regardless of class (dumb information); defaulting an empty count to zero (a lie: it was unknown).

Owner: Corvin Duskmantle <ARCHITECT>.

**Addendum to 4, field policy (Kestrel's question, same day; Vesper copied).** Tasks: a worker's write is routed through the Report transition and may change only `report.summary`, `report.commit` and `verify_result` on its own assigned task; everything else and the body must match the current record (`task_field_protected`); Oracle class and the operator create tasks only `open` and edit brief, `verify` and `allowed_paths` only while `open`; lifecycle and assignment stay in `task_*`. Artifacts: `claimed`, `produced_by`, `grade`, `history`, `for`, `derived_from`, and `ref` and `digest` once observed or claimed, move only through `artifact_claim`, `artifact_release` and the detector; a generic create gets `claimed: false`, `produced_by` the writer, `grade: inferred`, empty history (`artifact_field_protected`). `observed` is detector-owned and immutable to generic writes, and a generic update never changes any record's `type` (`type_immutable`), so neither the once-observed rule nor a lineage can be erased by relabelling. Rejected: a second report shape; a generic write that can change status or assignment; a generic write that can mint or steal a claim. Kestrel's proposal, taken as stated, with the field names fixed and these two closures added at Kestrel's ask.
