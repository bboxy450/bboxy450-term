---
id: 01M5WG2WS5S7QCE2Q4MR6J4WED
type: interface
number: 351
title: "ICD: identity ↔ app"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T16:18:20Z"
---
# ICD: identity ↔ app

| | |
|---|---|
| Side A | identity and custody (`bbt-mux` identity, `bboxy450-keychain` and its backends) |
| Side B | everything that signs: messages, records, tasks, ctl |
| The agreement | [[id1-seat-chain]]; [[identity]] |
| Amended by | #280 re-grant; #281 the custody facade; #313 the test-account label; #315 unlock under a hold; #285 continuity |
| What crosses | device grants and the seat chain; labels derived by the receiver, never caller-supplied |
| Timing | an unlock is one interactive access at most; a failed reopen recovers on the next refresh without another native attempt |
| States and lifecycle | a hold admits only status and unlock; a test account per night run in file custody; an operator-local reference before any device exists |
| Failure | signing can be held, delivery cannot; secrets never print |
| Versioning | the grant structs deny unknown fields; additions are new records, not new fields on old ones |
| Verified by | invariants messaging-never-held, ctl-never-prints-secrets, signed-main; promises I1 to I4 |
| Open | the native path's port onto the keychain facade, deferred |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.
