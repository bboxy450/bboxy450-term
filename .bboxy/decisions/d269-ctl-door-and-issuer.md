---
id: 01M5PT7ZYQF6XF2PJPKF7QJ7F3
type: decision
number: 269
title: "a ctl message names its door and its issuer so header and content agree; work travels only as a delegated task, never as instructions in a DM; a peer cannot override a task's own stop line"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[e2e-acceptance]]", rel: decides }
  - { to: "[[l9-messaging-tasks]]", rel: decides }
  - { to: "[[d260-ctl-authority]]", rel: amends }
  - { to: "[[d31-device-bound-operator-journal]]", rel: builds-on }
  - { to: "[[work-items]]", rel: builds-on }
created: "2026-10-24T11:20:26Z"
updated: "2026-10-24T11:20:26Z"
---
# Decision 269: a ctl message names its door and its issuer

An e2e finding, 2026-10-24 (Sable's rerun of Vesper's A-0 dry run): a real in-app Claude seat refused a DM sent through `ctl` on three grounds, each correct. The task's own line "stop on debug custody" cannot be overridden by a peer message. The header said operator with the nil seat while the content signed as Vesper, so the identities disagreed. The task was addressed to a Codex seat. An identical earlier run had complied, so the refusal was judgement, which is the point of having a seat judge.

**Alternatives.** (a) Tell seats to trust `ctl` messages as the operator's and ignore the content's signature. (b) The envelope and the native frame of a `ctl`-originated message carry `door: ctl` and `issued_by: <actor label>` (for example "Vesper, Oracle") inside the signed body, so the seat reads "the operator principal, via ctl, issued by Vesper (Oracle)" and the content agrees with the header; authority stays the operator principal's (the device signs, #31), the issuer is attribution. Work travels only as a delegated task (`ctl delegate`, a task record with brief and `verify`), and the only instruction-bearing message a seat receives is the one `Delegate` sends with the task path ([[work-items]]); a DM carries questions and feedback and a seat may refuse instructions in one. A peer message never overrides a task's own stop or scope line; only a `task_*` transition or a new task does. (c) Give Vesper a seat key and have `ctl` sign as Vesper.

**Choice.** (b).

**Reason.** The seat refused because the message was dishonest about who was speaking and used the wrong channel for work; (a) asks seats to stop noticing, which is the opposite of what the e2e plan is for. (c) would make the Oracle's outside session a seat, a second identity path #41's round rejected, and #260 already keeps `ctl` under the operator principal with its own audit kind; the kind was visible in the journal and not to the seat, which (b) fixes. Work as tasks is already the rule for the app ([[work-items]] Delegate); `ctl` inherits it rather than inventing a DM-as-brief path. The stop-line rule is the task's own contract holding against any peer, the Oracle included.

**Rules.** [[l9-messaging-tasks]] §2a: operator-principal envelopes gain `door` (`gui | ctl`) and `issued_by` (an actor label, free text, attribution not authority), both inside the signed body and shown in the native frame. [[e2e-acceptance]] §4 Assignment: work goes by `ctl delegate` with a task that has a brief and `verify`; a DM is for questions and feedback; a seat may refuse instructions in a DM and say why; a peer message never overrides a task's own stop or scope line.

Owner: Corvin Duskmantle <ARCHITECT>.
