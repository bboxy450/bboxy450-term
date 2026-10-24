---
id: 01M5Q8771KZ9C6KD4W1H24Q5R9
type: decision
number: 272
title: "ctl is one grammar over the window's own actions, one reply shape, one error model with a hint on every error; prompt handling is a recipe record, not a subsystem"
status: decided
decided: "2026-10-24"
operator:
  - "ask architect to design this cleanly"
links:
  - { to: "[[ctl-cli]]", rel: decides }
  - { to: "[[d260-ctl-authority]]", rel: builds-on }
  - { to: "[[d269-ctl-door-and-issuer]]", rel: builds-on }
  - { to: "[[d270-ctl-screen-input-wait]]", rel: builds-on }
  - { to: "[[work-items]]", rel: decides }
created: "2026-10-24T15:24:41Z"
updated: "2026-10-24T17:32:12Z"
---
# Decision 272: ctl is one grammar

Decided 2026-10-24 at the operator's word, via the Oracle, after three authority decisions had grown a command list by accretion. Spec [[ctl-cli]].

**Alternatives.** (a) Keep adding commands as needs arrive, each with its own flags and output. (b) One noun-verb grammar over seven nouns, where every verb is exactly one action the window already issues, one JSON-lines reply shape with the window's own data, one error model whose every error names what would be valid, and prompt handling as data-driven recipe records over `screen`, `match`, `input`, `wait` and `report`. (c) A separate control protocol and daemon for remote control.

**Choice.** (b).

**Reason.** LESS IS MORE: a verb that is a window action needs no new authority, no new shape and no new audit, which is the whole of #260, #269 and #270 in one sentence. Recipes as records make a new harness prompt a new file in the knowledge base, numbered and reviewable, instead of a code path per prompt, and they cannot skip the match step. (a) is what we had after one night; (c) is a second door with a second protocol, against the suite's shape.

**Rules.** [[ctl-cli]] §1 to §5; `recipe` joins the record types of [[work-items]]; the mapping table, reply and error shapes are tested (§6). Rejected: any `ctl` behaviour the window lacks; a `ctl`-specific output schema; an error without a hint; a prompts subsystem.

Owner: Corvin Duskmantle <ARCHITECT>.

**Revision 2 (same day, operator verbatim: "build bboxy450-term in a way that makes it scriptable, bboxy450-term ctrl should expose commands (design cli interface yourself, dont take my suggestion as a rule) but bboxy term gives you full on control over everything programmatically so you never need to ask for my permissions, think about all the bits and pieces you will need to test things approve things inspect things debug things - make it happen!").** The grammar grows to eleven nouns to cover the full programmatic surface (app lifecycle with `--headless`, `ui`, `events`, `debug`, `batch`, and the seat verbs for typing, keys, paste, resize, watch and scrollback), under the same one-action-per-verb rule, reply shape and error model; the invariants are named in §5 and the secrets rule becomes [[ctl-never-prints-secrets]]. Rejected again: a prompts subsystem (recipes), and any verb without a window action behind it except `debug` reads and `app start`.

**Addendum (Seraph's C-0 findings, 2026-10-24).** Verbs with no usable window action today are ruled by the spec's lens, the window gains the action first: close, undo-close, focus and spawn are relayed through the attached window (or the headless client), which owns `Position` and focus; rename and restart are logic actions the window reconciles from the state lane; `watch` is the events lane for state plus `ScreenRead` polled by `frame_seq` for the screen, never the window's grid-delta protocol. New actions named in [[ctl-cli]] §1a. Rejected: a logic-side close that invents a layout position; a CLI speaking the grid-delta protocol.
