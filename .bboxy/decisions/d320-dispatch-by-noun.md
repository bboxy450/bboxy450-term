---
id: 01M5SPS059GFR8T3K5E9BZK1J3
type: decision
number: 320
title: "ctl and GUI dispatch is organised by noun: one module per verb family with its own request, reply and apply, registered from a table, so an additive cut touches its own file and no peer rebases for it"
status: decided
decided: "2026-10-25"
operator: []
links:
  - { to: "[[ctl-cli]]", rel: decides }
  - { to: "[[no-chokepoints]]", rel: instance-of }
  - { to: "[[d272-ctl-one-grammar]]", rel: builds-on }
  - { to: "[[d260-ctl-authority]]", rel: builds-on }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-25T14:17:33Z"
updated: "2026-10-25T14:17:33Z"
---
# Decision 320: dispatch by noun, one module per verb family

Ruled at the Oracle's ask on the second day of `ctl`. Every cut that adds a verb today edits the same few files: the `UiRequest`, `GuiAction` and `GuiReply` enums in `bbt-proto/src/gui.rs` (lines 91, 308, 586), the `VERBS` table and dispatch in `bboxy450-term/src/ctl.rs` (348, 491), `apply_ui` in `main.rs` (4237) and its headless twin, and `gui_capture.rs` in the mux. Each landing therefore forces two or three peers to rebase and re-check: a merge-contention chokepoint, the eighth instance of [[no-chokepoints]], on the swarm side with a product-shaped cause.

**Alternatives.** (a) Leave the shape and accept the rebases: the cost grows with the number of concurrent cuts, which is the thing we are increasing. (b) **One module per noun.** Each verb family (`seat`, `ui`, `app`, `debug`, `events`, `record`, `task`, `identity`, …) owns a module with its own request and reply types and its own `apply`; the top-level enums carry one variant per noun wrapping the noun's type (`UiRequest::Seat(seat::Request)`), the `ctl` verb table is the concatenation of per-noun tables, and the window and the headless client dispatch by noun once and hand the request to the module. Adding a verb then touches one noun module and its table row, and nothing a peer working on another noun is editing. (c) A runtime registry of handlers: the same separation with dynamic dispatch and a second place where a verb can be missing at runtime.

**Choice.** (b).

**Reason.** The wire does not change: §2 of [[ctl-cli]] still has one reply shape and `data` is still the window action's reply unchanged, because the noun's reply type is what travels. What changes is which file a cut edits, and that is exactly the contention. (c) buys nothing over (b) in a program whose verbs are known at compile time and loses the exhaustiveness the compiler gives a closed enum. The mantra is the one about flexibility: the structure should let ten cuts land without ten rebases.

**Rules.** (1) New verbs land in their noun's module; a cut that adds a variant to a top-level enum or a row to a shared table outside its noun's file is asking for the migration, not around it. (2) The migration itself is one cut per dispatcher, mechanical, no behaviour change, its gate the existing ctl and headless suites unchanged. (3) The order is the Oracle's; the first candidate is the file the most concurrent cuts touch today. (4) **Judgement, not a gate, for now:** whether a cut stayed inside its noun is read at review; a check that each verb name appears in exactly one noun module becomes the gate once the modules exist, and is named then, not before.

Owner: Corvin Duskmantle <ARCHITECT>.
