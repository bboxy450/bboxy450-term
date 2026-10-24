---
id: 01M5PEDGYZRKMY02EDJVHNAGEN
type: decision
number: 264
title: "a reply with no destination goes to the original sender; the operator is operator/inbox; a dangling destination error names the valid one; a refused send is never stored"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[mcp-control-plane]]", rel: decides }
  - { to: "[[l9-messaging-tasks]]", rel: builds-on }
  - { to: "[[w2-5-mcp-and-seat-wiring]]", rel: decides }
  - { to: "[[e2e-acceptance]]", rel: evidence }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-24T07:53:44Z"
updated: "2026-10-24T07:53:44Z"
---
# Decision 264: a reply goes to the sender

The first intuitiveness finding of the e2e plan ([[e2e-acceptance]]), 2026-10-24, from Sable's probe: a real Claude seat, woken by an operator DM, read and acked on its own; when it replied it guessed `seats/00000000000000000000000000/inbox` from the operator envelope's nil seat id, was refused as dangling, and then stored its reply in its own inbox. Three things were wrong at once, none of them the seat's.

**Alternatives.** (a) Document the operator's inbox better and leave the tool as it is. (b) `message_send` with `reply_to` and no `destination` routes to the original sender's inbox, which for an operator message is `operator/inbox` ([[l9-messaging-tasks]] §2a already says replies to the operator address that sink); `orient` and the `bboxy450` skill name the operator's inbox plainly; a dangling-destination refusal names the valid destination; a refused send is a tool error to the caller and is stored nowhere. (c) Accept the nil seat id as an alias of the operator.

**Choice.** (b).

**Reason.** The seat did the natural thing twice and was punished twice: it derived a destination from the envelope it had, and when refused it kept its reply somewhere rather than lose it. A tool that needs the caller to know a sink name it was never shown is not intuitive, and "show the user useful information, never dumb information" ([[mantras]] 20) applies to agents: a refusal that says only "dangling" is dumb information; one that says "the operator is `operator/inbox`; reply with `reply_to`" is useful. (c) builds a lie into the address space; (a) keeps the trap and adds a sign.

**Rules.** `message_send { reply_to }` with no `destination` routes to the sender of the referenced message: a seat's inbox, or `operator/inbox` for a message authored by the operator principal (#31). `orient`'s answer and the skill's first lines say: "to reach the operator, reply with `reply_to`, or send to `operator/inbox`". A refusal for a dangling or unknown destination names the valid destination when one can be inferred from the message being replied to, and otherwise lists the sinks the caller may address. A refused send returns a tool error with the reason and is never stored in any inbox, the caller's included. The e2e promise M1 is read against this.

Owner: Corvin Duskmantle <ARCHITECT>.
