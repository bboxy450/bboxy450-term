---
id: 01M5TGD48S9GWHYA6YV0H0T3MK
type: spec
number: 334
title: "messaging attention: a stored message is not a wake; events versus conversation; receiver-owned wake policy per destination and class (wake, inbox, digest); bounded digests; validated threads; the cost of every wake in the ledger"
status: decided
operator:
  - "everything you stated above, around resource and messaging utilization - we should bake this into term messaging system as well, thats clear optimization of communication patterns"
  - "send the above learnings as something we need to build into our messaging system with mutes and unmutes and such, ways to optimize messaging patterns - send it to architect to structure task for integration of it into our system"
links:
  - { to: "[[l9-messaging-tasks]]", rel: builds-on }
  - { to: "[[messaging]]", rel: builds-on }
  - { to: "[[resource-ledger]]", rel: builds-on }
  - { to: "[[messaging-never-held]]", rel: invariant }
  - { to: "[[notices-never-wake]]", rel: invariant }
  - { to: "[[d285-no-gated-communications]]", rel: builds-on }
  - { to: "[[mcp-control-plane]]", rel: builds-on }
created: "2026-10-25T21:45:27Z"
updated: "2026-10-26T01:07:01Z"
---
# Messaging attention

The operator's words are quoted above, twice. This spec builds into bboxy450-term's own messaging (L9, [[l9-messaging-tasks]], [[messaging]]) what the swarm learned running on ICQ.

**Lens.** For a seat running a model, a message is not a cost when it is stored; it is a cost when it **wakes** the model, because a wake is a turn and a turn re-reads the whole context. Delivery and wake are therefore two things, the receiver owns the second, and the product shows what each wake cost. **Assumes** the inbox, the evidence states and the operator's swarm mute of [[messaging]]. **Breaks when** a policy is used to hold a message rather than to time a wake; that is a violation of [[messaging-never-held]].

## 1. The learnings, with evidence

1. **Inference is mostly re-reading.** Codex sessions read 10.5 billion input tokens (99 % cached) to write 37.6 million over fifteen sessions; every wake costs a full context read ([[resource-ledger]] §1).
2. **Notices woke models for nothing.** The Architect's pane answered about a hundred channel broadcasts with "nothing asked" before turning that channel's wake off; the Oracle did the same; DMs still woke both (2026-10-25).
3. **Status notices are machine events.** `SLOT TAKEN`, `CLAIM`, `LANDED` are read by detectors, the landing log and the board; the Oracle now verifies from `landings.log`, not from posts. A model reading them was reading a projection of a log it could query.
4. **Wake policy belongs to the receiver**, per room and per kind, with role defaults: the Oracle wakes on handoffs and questions, the Architect on rulings asked, Executors on briefs and replies.
5. **Batching beats waking.** Many notices in a quiet stretch are one wake, not many.
6. **Threads need ids that are checked.** A reply to a reminder's id instead of the message's orphaned a thread.
7. **Measure the cost of a message**: wakes and tokens re-read, per agent and per room, into the ledger.

## 2. Two classes of message

Every `MessageKind` belongs to one **class**, derived from the kind and signed with it, never chosen by the sender per message:

- **Conversation**: DMs, briefs and delegations, questions, replies, rulings asked, decisions, reports. Text for a model or a person.
- **Event**: typed status with structured fields and no prose a model must parse: slot claimed and released, cut claimed, landed, gate passed or failed, task transitions, presence, mute and unmute notices. An event's fields are its content; a projection of them into text is for people, and a detector reads the fields.

**The event kinds, pinned for MA-1** (Kestrel's question, the same day). `MessageKind::class()` derives the class from the signed kind; no sender sets a class. Existing kinds keep their bytes: `Presence` and the mute and unmute notices it carries are events; `Task` stays conversation. New event kinds, each with exactly these fields:

| Kind | Fields |
|---|---|
| `SlotClaimed` | `slot: u8`, `sha` |
| `SlotReleased` | `slot: u8`, `sha`, `exit: i32` |
| `CutClaimed` | `branch`, `paths: [string]` |
| `Landed` | `sha`, `tier: lab \| narrow \| wide`, `presubmit_seconds: u32` |
| `GateResult` | `sha`, `stage: presubmit \| postsubmit`, `verdict: green \| red \| flake \| manual-green \| manual-revert`, `predicate: string?` |
| `TaskTransition` | `task: u32` (the record number), `from`, `to` |

The receipt's wake is one optional field, omitted on rows written before MA-1: `"wake": "now"`, `"wake": "none"`, or `"wake": "deferred", "until": "<RFC 3339>"`. Events are first-class rows: stored, ordered, signed, receipted like conversation, readable by `events tail`, the board and the ledger. What differs is the default wake (§3).

## 3. Wake policy, owned by the receiver

For each seat, a **policy** maps (destination, class) to one of three modes, with a fourth state that is not the receiver's:

| Mode | Meaning |
|---|---|
| `wake` | stored, then delivered natively now: a turn |
| `inbox` | stored, receipted `recipient-stored`, never delivered natively; the seat reads it on a turn it takes for another reason, through its inbox tools |
| `digest` | stored, then delivered natively **once** per window as one summary listing every message by id, when the window closes or the batch reaches its cap |
| *mute* | the operator's swarm mute of [[messaging]], unchanged: holds the seat's traffic both ways and is set by the operator, never by the seat |

**No role default wakes on an event**, natively or by digest; events reach a model only by a seat's own choice. **Defaults are keyed by role name** (`oracle`, `architect`, `creativity`, executor roles), not by priority class, since the class distinguishes only oracle from worker; **defaults by role**, each overridable by the seat for itself and by the operator for any seat: Oracle: conversation `wake`, events `inbox` (the Oracle verifies from the landing log, not from posts; it may set events to `digest` for itself, and that is then its choice, not the default; corrected the same day at Velvet's question, since a digest default contradicted [[notices-never-wake]]). Architect and Creativity: DMs and messages addressed to them by name `wake`, channel conversation `inbox`, events `inbox`. Executors: briefs, delegations and replies in a thread they started `wake`, other conversation `digest`, events `inbox`. Operator principal and the cockpit: everything visible, nothing wakes a model.

**Always wake, whatever the policy**, for **conversation** only: a message from the operator principal, a task delegated to that seat, and a reply in a thread that seat started. A seat cannot set itself to miss its own work. An **event** never wakes by default, whoever authored it, the operator included: an operator's mute notice is status, not a question (Velvet's question, 2026-10-25).

**Digest bounds**: a window of at most 10 minutes and a cap of 50 messages, whichever comes first, read from the injected App clock so tests do not sleep ([[test-policy]] B1); a digest never waits longer than its window, so `digest` delays a wake and never holds a message.

The policy is set through MCP tools for the seat itself (`attention_get`, `attention_set`), `ctl seat attention <seat> [<destination> <class> <mode>]` for the operator and scripts, and Settings › Messaging per seat. Orient tells a seat its policy on its first turn and how to read its inbox (the field policy of [[mcp-control-plane]]).

## 4. What never changes

[[messaging-never-held]] holds unchanged: a policy decides only **when a stored message becomes a turn**, never whether it is stored, receipted or delivered to the inbox. A sender sees the receipt `recipient-stored` with `wake: now | deferred-until <time> | none`, so nothing is silent. The operator's mute is the only thing that holds a seat's traffic, as before. Order within a destination is preserved in every mode, and a digest lists messages in order.

## 5. Threads

A reply carries `reply_to`, the id of the message it answers; the service checks that id exists in a thread the replier can see, and refuses an unknown one with the error kind `unknown_thread` whose `hint` names the latest message ids of the replier's open threads. A thread is the chain of `reply_to` from its root; "a thread the seat started" (§3) is a thread whose root the seat authored.

## 6. Cost in the ledger

Every native delivery writes a ledger line ([[resource-ledger]] §2) with stage `wake`: seat, destination, class, mode, message ids, and, filled nightly from the transcript, the input tokens of the turn it caused. The board shows wakes and tokens re-read per seat and per destination, and the share caused by events, which under these defaults should be near zero.

## 7. Tasks

One Executor per row, landed through `land.sh`, each small enough to land the day it is claimed; the Oracle files them as task records and orders them.

| # | Task | Crates | Verify |
|---|---|---|---|
| MA-1 | `MessageClass` derived from `MessageKind`, the event kinds enumerated with structured fields; `WakeMode` and the receipt's `wake` field; additive wire with defaults | `bbt-proto`, the messaging types | the wire golden gains its cases, every existing case byte-identical |
| MA-2 | Policy store on the seat record (an additive optional typed field on `SeatRecord` in `bbt-proto/src/registry.rs`, which drops unknown fields, with its golden case), role-name defaults, `attention_get` and `attention_set` MCP tools, the conversation-only always-wake exceptions | `bbt-proto`, `bbt-mux` | unit tests per mode and exception; the registry golden gains its case |
| MA-3 | Native delivery consults the policy; inbox always written; the receipt carries `wake` | `bbt-mux` | the [[notices-never-wake]] test; [[messaging-never-held]] still held |
| MA-4 | Digests on the App clock, window and cap, one delivery listing ids in order | `bbt-mux` | frozen-clock tests at the window and the cap |
| MA-5 | `reply_to` validation and `unknown_thread` with the hint | `bbt-mux`, `bbt-proto` | refusal and hint tests |
| MA-6 | Orient reports the policy; `ctl seat attention` | `bbt-mux`, `bboxy450-term` | ctl tests; an Orient fixture |
| MA-7 | `wake` ledger lines and the nightly token fill | `bbt-mux`, `scripts/` | a ledger fixture |
| MA-8 | Settings › Messaging per seat; the sidebar row shows deferred count and the next digest time | `bbt-ui`, lab | parity cases for the new states |
| MA-9 | e2e promise and night script together: an event never wakes an idle seat under defaults, a DM does, a digest arrives once after its window | `e2e-acceptance` §1, `scripts/e2e/` | the night script, coverage full; added with its script so the night never shows an untested promise |

MA-1 lands first, alone, as the contract; MA-2 to MA-7 follow in any order behind it; MA-8 and MA-9 last.

Owner: Corvin Duskmantle <ARCHITECT>.
