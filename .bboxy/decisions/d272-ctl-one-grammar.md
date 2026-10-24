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
updated: "2026-10-24T15:24:41Z"
---
# Decision 272: ctl is one grammar

Decided 2026-10-24 at the operator's word, via the Oracle, after three authority decisions had grown a command list by accretion. Spec [[ctl-cli]].

**Alternatives.** (a) Keep adding commands as needs arrive, each with its own flags and output. (b) One noun-verb grammar over seven nouns, where every verb is exactly one action the window already issues, one JSON-lines reply shape with the window's own data, one error model whose every error names what would be valid, and prompt handling as data-driven recipe records over `screen`, `match`, `input`, `wait` and `report`. (c) A separate control protocol and daemon for remote control.

**Choice.** (b).

**Reason.** LESS IS MORE: a verb that is a window action needs no new authority, no new shape and no new audit, which is the whole of #260, #269 and #270 in one sentence. Recipes as records make a new harness prompt a new file in the knowledge base, numbered and reviewable, instead of a code path per prompt, and they cannot skip the match step. (a) is what we had after one night; (c) is a second door with a second protocol, against the suite's shape.

**Rules.** [[ctl-cli]] §1 to §5; `recipe` joins the record types of [[work-items]]; the mapping table, reply and error shapes are tested (§6). Rejected: any `ctl` behaviour the window lacks; a `ctl`-specific output schema; an error without a hint; a prompts subsystem.

Owner: Corvin Duskmantle <ARCHITECT>.
