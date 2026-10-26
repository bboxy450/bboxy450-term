---
id: 01M5WG2WRTHRG8989VV3VRNE7V
type: interface
number: 347
title: "ICD: ctl ↔ app"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T16:18:20Z"
---
# ICD: ctl ↔ app

| | |
|---|---|
| Side A | the `ctl` client (`bboxy450-term ctl`) |
| Side B | the window or headless owner behind the opt-in endpoint |
| The agreement | [[ctl-cli]] §1 to §6 |
| Amended by | #260 opt-in endpoint and ancestry refusal, told why on the wire; #269 door and issuer; #272 one grammar; #320 dispatch by noun; #328 waiting reads |
| What crosses | one JSON-lines reply shape whose data is the window action's own reply; errors as kind, message and hint |
| Timing | a relayed verb replies after the frame that shows its end state and names that frame; waits are bounded, at most 60 s per read |
| States and lifecycle | the endpoint file exists only when the app runs with `--ctl`; `app start` replies after render readiness; `app stop` is the window's own close |
| Failure | exit codes 0 to 4; a peer descended from the app's supervisor is refused and always told why; no PID-kill fallback |
| Versioning | a verb edits only its noun's file; each verb listed once in its noun's table |
| Verified by | invariants ctl-opt-in, ctl-never-prints-secrets; the ctl library tests; every night script drives the app through it |
| Open | the operator's attention setter arrives with MA-8 |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.
