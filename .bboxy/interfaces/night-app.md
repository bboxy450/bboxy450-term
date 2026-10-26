---
id: 01M5WG2WSA0RT6C76A2EXF7KJB
type: interface
number: 353
title: "ICD: night runner ↔ app"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T16:18:20Z"
---
# ICD: night runner ↔ app

| | |
|---|---|
| Side A | the e2e runner and night scripts (`bboxy450-term e2e`, `scripts/e2e`) |
| Side B | the whole app, headless, on a throwaway account |
| The agreement | [[e2e-acceptance]] §2 to §4a |
| Amended by | #282 the app tests itself unattended; #316 a pass means the whole promise; #313 the test-account label |
| What crosses | one e2e record per promise with coverage and missing assertions, signed by the night's account |
| Timing | each group ends by verdict within its bound |
| States and lifecycle | a throwaway account per run; headless; nothing built or signed at night |
| Failure | an untracked partial or failure is red; a tracked one is amber; never a human in the loop |
| Versioning | a script's coverage header is the source of its record |
| Verified by | invariant e2e-runs-unattended; every promise |
| Open | 50 of 55 promises partial on 2026-10-25 |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.
