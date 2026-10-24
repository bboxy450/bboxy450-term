---
id: 01M5P5ERB2AW01WTGAFZFM9T1T
type: decision
number: 259
title: "the swarm's job is a working app with Claude and Codex inside it, judged through e2e records the in-app seats run and the Oracle reads from outside"
status: decided
decided: "2026-10-24"
operator:
  - "swarms job is to have working app, with codex and claude inside of it and you from here being able to talk to them and ask for their feedback and doing full ond e2e tests for you (write e2e test docs in our knowledge base) harnesses there basically need to make sure all promised functionality is there, is intuitive is works, thats acceptance cretirea"
links:
  - { to: "[[e2e-acceptance]]", rel: decides }
  - { to: "[[vision]]", rel: amends }
  - { to: "[[work-items]]", rel: decides }
  - { to: "[[d31-device-bound-operator-journal]]", rel: builds-on }
  - { to: "[[d28-structured-task-verification]]", rel: builds-on }
created: "2026-10-24T05:17:08Z"
updated: "2026-10-24T19:18:29Z"
---
# Decision 259: the overnight north star

Decided by the operator before sleeping, 2026-10-24; written by the Architect at the Oracle's ask so Executors start in parallel. Spec [[e2e-acceptance]].

**Alternatives.** (a) Acceptance by Executor handoff reports and the Oracle's reading, as today. (b) Acceptance by e2e records in the knowledge base, one per promised function, run by Claude and Codex seats inside the app with our tools, reported with `verify_result`, read by the Oracle from outside through an operator-side CLI over the GUI socket under the existing operator principal. (c) A separate test harness driving the app from outside through the desktop.

**Choice.** (b).

**Reason.** The operator's words: the swarm's job is a working app with the harnesses inside it, the Oracle talking to them and asking for feedback, and full e2e tests, "all promised functionality is there, is intuitive, works". (b) tests the product the way it is used, by the agents that live in it, through the seat wiring we already have, and makes the result a record anyone can open; (a) tests parts, not the promise; (c) violates the desktop rule and tests a different thing than the operator will use. No new authority: the Oracle's CLI does exactly what the operator can do in the window, journaled the same way.

**Rules.** `e2e` is a record type with the frontmatter in the spec, numbered, written only through the app so it is signed by the running seat; every promise in the checklist has one; `untested`, `fail`, `blocked` and `manual` are honest states and the view says why; seats use MCP tools and the app's own capture, never the operator's desktop; feedback goes as a DM for the day and as a comment record for the RFC loop; the Oracle accepts from the checklist view, never from a report. Rejected: hand-written results (inferred grade at best), a test harness outside the app, acceptance by summary.

Owner: Corvin Duskmantle <ARCHITECT>.

**Amended by [[d282-app-tests-itself-unattended]] (operator, 2026-10-24 morning):** the night runs with no operator present: automation identity in file custody, headless app, one runner, nothing built or signed at night; the one remaining operator act is the automation device's enrollment, one root signature, once.
