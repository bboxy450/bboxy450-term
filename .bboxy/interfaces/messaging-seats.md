---
id: 01M5WG2WS0GPS3RPT60P5VXVX2
type: interface
number: 349
title: "ICD: messaging ↔ seats"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-29T19:48:09Z"
---
# ICD: messaging ↔ seats

| | |
|---|---|
| Side A | the messaging service in logic (`bbt-mux`) |
| Side B | every seat through MCP tools and native delivery |
| The agreement | [[messaging]]; [[l9-messaging-tasks]]; [[messaging-attention]] |
| Amended by | #285 no gated communications and operator-local continuity; #334 attention, events and threads |
| What crosses | signed envelopes; conversation and event classes derived from the kind; receipts with evidence states and the wake decision; `mentions`, the seat ids a channel message names, resolved from `@name` by the sender's tool and signed, never parsed from prose by the receiver. **Mention grammar** (Velvet, MA-10, 2026-10-26): `@token` matches an open seat in the same swarm whose display name, or its first word, equals the token, case-insensitively, and must match exactly one seat; `@{Full Display Name}` names a seat whose name has spaces; an `@` inside a word (an email address) is not a mention; an unknown or ambiguous mention is refused before any effect, with a bounded hint of valid names; ids are deduplicated, at most 32 per message, sorted and signed; the receiver and journal replay read ids only and never resolve a name again |
| Timing | digest windows at most 10 minutes or 50 messages; bus p99 at most 13.3 s at 1 000 seats |
| States and lifecycle | under a custody hold delivery continues as pending intents; the wake decision is pinned at admission; one GUI consumer per device |
| Failure | nothing is held but by the operator's mute; superseded and done messages never wake; an unknown thread is refused with the open threads named |
| Versioning | additive kinds and fields; signatures unchanged when new fields are absent |
| Verified by | `make test-signed-mentions` (sender grammar, signed IDs, replay, pending upgrades, composer and retained-logic activation); invariants messaging-never-held, notices-never-wake, seats-1000-bus; promises M1 to M6 |
| Plugin senders | 2026-10-30, [[plugins]] §3c: a seat may receive a message whose envelope is `issuer_kind: host` with `issued_by: plugin:<candidate>`; it is from a plugin, signed by the host, never a seat or the operator, and seat-facing text says so. Verified by `messaging/plugin_sender_tests.rs` and `tests/plugin_messages.rs`. |
| Open | MA-5 to MA-9, in work |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.


## Existing-seam fresh composer read — 2026-10-29 (#388)

Producer Lumen Foxglove and consumer Vesper Greywind (Oracle), contract owner
Nyx Copperwing, select existing canonical Recent readback after a known commit
under [[l9-messaging-tasks]] and [[logic-gui]]. Messaging operator/swarm/context
read authority and the returned StoredMessage bind the row; no fabricated
composer envelope, broader subject/grant or new wire source field is allowed.
Actual receipt summaries remain snapshots, and storage/native delivery is never
model acknowledgement. Coalesced refresh preserves canonical messages/history,
older pagination, the single Event lane and cursor/pending/ACK/read semantics.

The retained-348000 produced-frame witness and independent direct-subject,
no-effect dedup and refresh mutations must prove the selected existing seam.
Later receipt-only freshness is a separate measured assertion, using existing
Event/explicit read or changed-high triggers; it does not justify protocol
expansion without a counterexample and both owners' recorded agreement.
Implementation/acceptance evidence remains open under supplement #447 / issue73.

An authenticated, matched Recent item and canonical row with equal `bus: None`
may be displayed before event publication, preserving None and conferring no
Event admission, unread coverage, cursor/pending movement, read mark or ACK.
Keep ordinary Event stream fences: later actual authorized publication merges
by canonical message ID/context and its real position without duplicate rows or
loss of confirmed signature/ACK evidence; require an actual canonical-None →
actual-Event production witness and a negative control restoring the old None
rejection before implementation acceptance.


### Seat document recovery, #74 (2026-10-30 direct operator batch)

Mux journal ↔ seat MCP adds `operation_status` as specified in
[[w2-5-mcp-and-seat-wiring]] “Original-operation recovery”. Current admission
and root selection precede lookup; provider scans do not. The durable original
entry owns the outcome across logic restart; lookup has no write/redispatch
transition. Missing/unavailable evidence stays unknown. The same seat in a new
launch can recover its original document effect, within the same swarm, issuer
and root. Cross-context entries are refused. Byte bounds are the existing MCP
bounds. `make test-workspace-boundary` includes the collision and journal/API
witnesses; `make test-formal` checks `OperationRecovery` and its broken controls.


### Seat diagnostics, #75 (2026-10-30 direct operator batch)

Mux diagnostic IO ↔ authenticated seat MCP adds `diagnostic_errors`, specified
in [[mcp-control-plane]] “Own-seat error diagnostics”. Scope is strictly the
calling seat's retained history; no current policy grants swarm diagnostic
reads. Journal v1 has no historical root/swarm provenance, and responses label
that limit explicitly. Admission and completion check current actor/workspace
revision; independent per-seat jobs never queue behind another seat or a
provider scan. Fixed private files, bounded lines/pages and scoped five-minute
cursors fail closed on unsafe stores or changed snapshots. Seat reads never
open or change the operator read mark. Stable error IDs and available original
operation IDs link observations without claiming a cause or effect result.
Evidence: `make test-cli-error-isolation`, `make test-oracle-control` and the
`DiagnosticRead` model/broken controls in `make test-formal`.
