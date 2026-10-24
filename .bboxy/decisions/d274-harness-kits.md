---
id: 01M5Q8DB0DPJC3K27AN4B6ECTX
type: decision
number: 274
title: "two harness kits split out and published: bboxy450-harness-wake-up (the hacks, with an honest can-it-be-woken answer and the fallback) and bboxy450-harness-host (PTY hosting, capture, resume); the app maps their vocabulary to its wire"
status: decided
decided: "2026-10-24"
operator:
  - "the logic that we have, about waking up harnesses, like running claude with sockets and messaging, then using that socket to deliver messages, or codex and their control sockets, and other harnesses, we should abstract it into bboxy450-harness-wake-up crate, and reuse it down the line! and that crate should tell its user if the harness can be woken up or no then there is fallback, its not a wrapping crate, just shared reusable hacks on how to tap into harnesses, the crate that wraps harness processes and manages them and runs them in pty and shit should be bboxy450-harness-capture, or instead of capture use better name, reusable pieces down the line! lets implement this, architect, look at code, plan those crates, communicate to oracle, seems like we done with big chunk of work here, so time to split everything into reusable crates, make sure it still works and then start publishing crates!"
links:
  - { to: "[[harness-kits]]", rel: decides }
  - { to: "[[open-source]]", rel: amends }
  - { to: "[[licensing]]", rel: amends }
  - { to: "[[d39-suite-is-the-product]]", rel: builds-on }
  - { to: "[[d8-open-source]]", rel: amends }
  - { to: "[[d1-crate-topology]]", rel: builds-on }
created: "2026-10-24T15:28:01Z"
updated: "2026-10-24T15:28:01Z"
---
# Decision 274: the harness kits

Decided by the operator on 2026-10-24; planned by the Architect from a read-only map of the code the same day. Spec [[harness-kits]].

**Alternatives.** (a) Leave the adapters inside `bbt-mux` and the supervisor as an internal crate until some later plateau. (b) Two kits: `bboxy450-harness-wake-up`, the adapters module extracted with its own small vocabulary, a caller-supplied sink for the shim, and no dependency on the app's wire or host; and `bboxy450-harness-host`, the supervisor whole, published under the brand name after one leak of harness knowledge is turned into a configurable probe allowlist; `bbt-mux` maps the kits' types to its wire, one tested function per type. (c) One combined `bboxy450-harness` crate. (d) A third crate of shared types below both.

**Choice.** (b).

**Reason.** The operator's words: the hacks for tapping into harnesses are reusable knowledge any suite app wants, and they must say honestly whether a harness can be woken and what the fallback is, which `WakeMethod` and the readiness guard already do; the thing that runs programs in PTYs and keeps what they said is a different kit with different users. The map shows the seam is already almost clean: the module's only ties to the app are six borrowed types and one journal call, and the supervisor's only tie to a harness is one probe exemption. (a) keeps reusable code private for no reason now that the cockpit works. (c) couples a library of protocols to a process actor nobody else needs together. (d) is a crate for six enums; mapping functions in the app are smaller and tested.

**Rules.** [[harness-kits]] §1 to §6: what each kit takes and leaves; the wake-up crate's own vocabulary and the app's mapping; the host's probe allowlist and the no-harness-word test; layering place, brand names, licence rows; acceptance including the e2e promises that must stay green; the order HK-0 to HK-2 and then the publishing order, leaves first, by the release checklist, the Oracle publishing. `pi` and `omp` are named as future adapters of the wake-up crate and are not implemented anywhere today, which the map confirmed.

Owner: Corvin Duskmantle <ARCHITECT>.
