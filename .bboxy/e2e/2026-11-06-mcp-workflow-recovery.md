---
type: e2e
title: Retained MCP workflow error recovery
function: Normalize report identity, initialize reserved task custody and close confirmed orphan seats
group: knowledge-and-control
status: passed
coverage: Four focused native regressions; affected app and mux tests; process boundaries; license checks; 556 formal models with native/TLC parity across 206 contracts and 34 crates
last_run: 2026-11-06T19:12:47.765572+00:00
expected: Valid reports and blank reservations complete their supported workflows while current authority, source revisions and process ownership remain fenced
result: Signed source 3771188e82b78886cea9bac1789ad7dd7dfcaaa4 landed on main and installed. GUI and controller updated; supervisor and all six open seats retained.
run_by: operator
evidence:
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/errors.json
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/errors-final.json
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/focused.log
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/boundaries.log
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/licenses.log
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/land.log
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/activate.log
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/logic-refresh.json
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/seats-pre-logic-refresh.json
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/seats-post-logic-refresh.json
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/knowledge-status-after.json
- /Users/feral/.local/state/bboxy450-term/debug-errors-20261106/cleanup-manifest.json
steps:
- Read bounded retained errors and reproduce current failures in an isolated worktree
- Repair report locator handling, blank task reservation initialization and orphan seat closure
- Run focused, boundary, license and lean landing checks with formal evidence
- Install signed source, refresh controller without replacing supervisor or seats, remove unused owned artifacts
id: 01M6S1ZXN4PEB09K2VAWP7P2YH
updated: 2026-11-06T19:13:37.273Z
number: 93377961364443
created_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
updated_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
authorship:
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  kind: local-dev
  principal: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  device_id: 01M62F1BBJT8MW0F3YMZJ8BENM
  public_key: a8bbsPrJyl7hZraHSoUBkc76MQiRVYXx/kgj6IWVFt0=
provenance:
  digest: sha256:750ac0635c8e15c6e9daaada46a7e04d66e607b07dcabda82d13bcf0939a56aa
  frontmatter_digest: sha256:0db0ecf51f9b80cdb88eb5e5aac0d6fc184525b66eb0edac84619c6636be2bec
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: mhZctanz6CeWdcrzWkMzKnaYoFuG7qU8sljcb6xfckV7ryCCAy5y8zfwbYea8g6bXDCa5osnrifgJ1xW3oltAQ==
  log: bboxy://activity/index/384249
  event:
    id: 01M6S4FR1X5RX3Q70H9KQ62ERJ
    sequence: 384249
    hash: 4062edc81662ad70f17a37f9e3da9c3f70005acb8030d31260bd24e568862f43
---
# Retained MCP workflow recovery

Task: [[#93377961364442]]. Worktree: /Users/feral/mydev/bboxy450-term-debug-errors.

The production ctl debug errors read acknowledged 148 unread entries (unread now 0). Its bounded page contained 256 retained entries and reported 3168 older omitted entries; no unbounded-history claim is made. Current reproduced defects were wrapped report locators reaching strict providers, allocated blank tasks with no initialization bridge, and orphan seat closure depending on a viewport. Older timeouts, unavailable records and deliberate authority/payload refusals remain historical evidence, not permission to replay unknown writes or weaken checks.

The host now strips wiki presentation syntax for both report lookup and the report-to-task lookup. A strict provider regression checks aliases, anchors, native prefixes, numbers, traversal refusal and mismatched task rejection.

An Oracle can initialize only a blank, open, unowned reservation through a current ref/etag. It supplies the brief, verify and allowed_paths while preserving all other metadata. The host binds delegator and swarm; any existing brief, custody or evidence blocks adoption. Native API regressions cover Worker and forged-field refusal, stale ref and etag rejection, a competing Oracle, protected existing intent and successful delegation with the populated brief.

Viewport-free closure applies only after the correct supervisor positively reports no owned process for that seat, with no attached/pending runtime and no logic yield before current caller/target and registry checks. It retains history without signaling a PID. Tests cover successful headless closure, same-operation replay, stale generations, foreign swarm refusal and supervisor-owned processes lacking a viewport. Live seats are not closed by the audit.

Verification completed on signed main commit 3771188e82b78886cea9bac1789ad7dd7dfcaaa4:

- make test-mcp-recovery: exit 0, four focused native regressions passed.
- make test-boundaries: exit 0, scripted real process/lifecycle checks passed.
- make test-licenses: exit 0.
- make land: exit 0, affected-crate clippy and app/mux tests passed (446 app tests and 478 mux/MCP tests passed, with 12 and 25 skipped respectively).
- Formal receipt 580838fc8f9da322bf8ffd639b8525b2f2b176ffe52e18c2cbb793ec47375df5: clean source, 556 models, 206 contracts, all 34 crates, native/TLC parity. New bounded TaskReservation and OrphanClose models include independent failing controls. The existing pinned harness required its missing DreamC1 coverage registration; no harness code was changed. Full make test was not run; the lean affected gate was used.

Source evidence: crates/bbt-mux/src/mcp/knowledge.rs:303 normalizes provider record identity and :354 fences blank task initialization; crates/bbt-mux/src/mcp/work_items.rs applies it to task/report lookups; crates/bbt-mux/src/logic.rs:5422 verifies supervisor absence and current authority before retaining a closed orphan. Native regressions start at crates/bbt-mux/tests/mcp.rs:7627.

The signed app uses the available existing approved distribution identity. The first installation handoff stopped at a read-only probe timeout without changing the installed app. The prepared-bundle retry succeeded. Because the wire-compatible old controller was retained, its exact recorded PID/birth was checked against the kernel before graceful SIGTERM; the new GUI then restored controller ownership from the installed binary. Controller PID changed from 57760 to 30877; supervisor PID 80176 and its birth identity were unchanged. GUI PID 93024 reports source 3771188e and backend/gui/events ready. All 35 retained seat identities, generations, sessions, lifecycle states and process identities matched before/after, including all six open seats. Harmony GitKB source revision 20 remains selected with the same approved hash, a live provider process and no refusal.

The later bounded error read acknowledged 23 additional entries (unread now zero), including the old-controller report-path failure, expected invalid payload/unsupported-provider/missing-record refusals, and transport/plugin errors around the intentional controller replacement. Two task/create diagnostics are redacted by product policy and are not claimed diagnosed. Historical unknown writes were not replayed. No peer task was accepted, no reservation adopted and no live seat closed as part of this audit.

The isolated verification worktree, branch and about 7.4 GiB of build data were removed after confirming landing, cleanliness and no open files. A further 687095808 allocated bytes of this run's unused release artifacts were removed using the saved manifest and an open-file check. The old installation backup was preserved because six live processes still mapped it; shared caches, other install backups, user state and peer work were untouched. Free disk space remained about 17 GiB as other sessions continued working.

The source checkout is clean and origin/main matches the signed commit. This private record and the native audit task are deliberately checkpointed in the built-in KB; .bboxy content is not added to source Git. Production failed/unknown operations have not been retried.
