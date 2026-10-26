---
id: 01M5WG2WSA0RT6C76A2EXF7KJB
type: interface
number: 353
title: "ICD: night runner ↔ app"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T19:49:14Z"
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
| States and lifecycle | a throwaway account per run; headless; nothing built or signed at night; **the promise scripts run with the same private HOME and harness environment as the app they test**, never the caller's real HOME (the Oracle, 2026-10-26: `e2e_night.rs:602` ran scripts with the real HOME while only the app got the private one at `App::start`, `:479`; the T6 and T9 guards caught it; Aster fixes it). **The night's environment is constructed, never inherited** (the Oracle, 2026-10-26, a security gap: launched from a Claude Code session, the apps and scripts inherited that session's messaging socket, token and child-session marker, the last of which silenced native transcripts and turned T6 and T9 red only when a Claude session ran the night): the shared private-home setup builds the environment from an allowlist (the private `HOME`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `PATH`, locale, the test account's variables) and passes nothing else from the caller; Aster fixes it |
| Failure | an untracked partial or failure is red; a tracked one is amber; never a human in the loop |
| Versioning | a script's coverage header is the source of its record |
| Verified by | invariant e2e-runs-unattended; every promise |
| Open | promises still partial outside the terminal group; the terminal group ran 9 of 9 full on 2026-10-26 (Aster, the canonical night script, exit 0 in 137 s, ten apps on file custody, no window); the scripts' real-HOME inheritance fixed at e28aa9bd, one private 0700 home with `HOME`, `CLAUDE_CONFIG_DIR` and `CODEX_HOME` shared by the app and its scripts, a symlinked home refused |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.
