# Events — the spine

Owner: Vesper Greywind <ORACLE>. 2026-10-21. Operator: "EVENT DRIVEN SYSTEM. Tasks and markdown docs live side by side with the event system: touch a doc, whoever subscribed gets the update. With external and cloud-based events." Sources: [[kittens-events]] (the operator's kittens bus and journal), [[vision]] Seats are sockets, [[storage]] Events store, [[messaging]] pubsub, [[work-items]], [[provenance]] activity log.

## One bus, many producers, many subscribers

Everything that happens is an event on one bus hosted by the mux logic process. Producers and the topics they emit on:

| Producer | Topics | Example events |
|---|---|---|
| seat | `seat/<id>` | state, task, hook, output, title, cwd, lifecycle, input receipt (the seat socket envelope from [[kittens-events]]) |
| markdown | `doc/<id>`, `doc/path/<glob>` | created, modified (frontmatter diff + body digest), status changed, link added, claimed, derived_from changed |
| artifact | `artifact/<id>` | created, modified, deleted, renamed, committed, published, claimed, released |
| task / epic / bug | `task/<id>`, `swarm/<id>/tasks` | delegated, acknowledged, running, reported, accepted, rejected, reassigned, blocked |
| swarm | `swarm/<id>` | seat joined/left, mute/unmute notices, presence, audit lines |
| messaging | `swarm/<id>/stream`, `channel/<name>`, `dm/<seat>` | message with evidence states |
| mux | `mux` | seat spawned, generation changed, reconnect, resource warnings |
| **external** | `ext/<source>/<kind>` | GitHub webhook (PR opened/merged, review, CI status), CI runs, cloud provider (VM up/down, billing), cron ticks, operator-defined webhooks, iTerm/other terminals via their APIs |
| **cloud** | `cloud/<provider>/<object>` | remote object events relayed over the Transport from a remote supervisor or a cloud-hosted object |

Every event is the same envelope: `{id, seq, ts, topic, kind, source, seat?, generation?, payload, provenance: {signer, sig}}`. Sequence is per topic from the producing object's `events.sqlite` ([[storage]]); the bus never reorders within a topic.

## Subscriptions

Anyone with a grant can subscribe: a seat (so a harness is woken when a doc it watches changes), the GUI, an external sink (webhook out), another object. Subscription = `{topics: [pattern], since?: seq, filter?: kind list, deliver: socket | wake | webhook | gui}`. Patterns: exact topic, `doc/path/.bboxy/specs/*`, `task/*` within a swarm, `ext/github/*`. A subscription is itself a markdown-less record in the swarm object; subscriptions by seats expire with their grant.

**Docs side by side with events:** every markdown in `.bboxy/` has a topic by id and by path. A seat that was delegated a task is auto-subscribed to that task's doc and its `derived_from` docs; the Oracle seat is auto-subscribed to every task it delegated. Editing a spec wakes the seats whose tasks cite it, with a one-line event, not the file.

## Delivery guarantees

The same evidence vocabulary as messages: stored, delivered-to-socket, acknowledged. Replay from `since` with explicit `gap` when evicted. Wake of a harness goes through the adapter's input guard; an event never types into a PTY by itself. Backpressure: bounded per-subscriber queues, coalescing of `modified` events per doc, never drop without a `gap`.

## External events in

An `ext` ingress per source: GitHub (webhook receiver on a local port or a polling adapter using `gh`; signed payloads verified), CI (GitHub Actions run events), cloud providers (via the CloudProvider trait in [[networking]]), cron (the mux's own scheduler), generic HTTP webhook (operator-defined, token-authenticated). Each ingress normalises to the envelope and records provenance `source: ext/github`, signer = the ingress's own leaf key, with the original payload digest.

## Events out

A subscription with `deliver: webhook` POSTs envelopes to an operator-configured URL with a signature header; this is how an enterprise wires bboxy into Slack, PagerDuty, their own bus. The per-object `events.sqlite` is the durable tail for everything else ([[storage]]).

## Cloud

A remote object (seat on a VM, swarm in the cloud) runs the same bus locally and relays its topics over the Transport to subscribers elsewhere; subscriptions cross machines by account grant. Phase 2 per [[vision]]; the envelope and topic names are fixed now so nothing is renamed later.

## Messages are an abstraction on the bus (operator, 2026-10-21)

A message is an event on `swarm/<id>/stream`, `channel/<name>` or `dm/<seat>` whose payload is the signed messaging envelope; delivery evidence states are events; mute is a subscription filter; priority is consumer-side ordering. [[messaging]] keeps its envelope, grants and evidence vocabulary and loses its own transport: it is a client of this bus. Research on NATS-class systems is running (`research/pubsub-nats.md`); the bus is to be designed under LESS IS MORE and KISS, borrowing NATS subject grammar and JetStream's stream/consumer model where it is small, and refusing everything that is not.

## Rules

- One envelope, one bus, one sequence per topic. Messages, task transitions, doc edits and GitHub webhooks are the same kind of thing.
- Subscribing is cheap and explicit; nothing is pushed to a seat it did not subscribe to or was not delegated.
- Every event has a signer. External ingress signs with its own leaf; the original digest is kept.

## Build

Lands in W2.2 (logic process) as the bus plus `doc/*`, `task/*`, `seat/*` producers and socket/GUI delivery; `ext/github` and `webhook out` in W2.6; cloud relay in phase 2.

## Gaps

- **Gap:** subscription storm at 1000 seats each watching a few docs: fan-out cost per doc edit; measure in W2.2.
- **Gap:** external ingress on a laptop behind NAT needs either polling or the relay from [[networking]]; decide per source.

Links: [[vision]] [[kittens-events]] [[storage]] [[messaging]] [[work-items]] [[provenance]] [[networking]] [[mux-daemon]] [[mcp-control-plane]]
