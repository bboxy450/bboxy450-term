---
type: report
title: 'C1 Dream inspection contract: implementation handoff'
task: '[[tasks/2026-11-05-dream-c1-contract]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T11:12:28.790Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6NPJ0EQ7HV7DRGFDKZJFA50
updated: 2026-11-05T11:12:28.790Z
number: 93377961364372
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:af8d41236018182c014eba872cd04de39c013f739fdf65f7b705e2fb3e63b3fe
  frontmatter_digest: sha256:783b94077e3a0306432e0a3d85994b1a15892d83eebd80ba3783cad44ab815a6
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: 9DZMSjlIUBTLTRufBHsGBielZ45EY+8b0ikUFJkgjXVs1h/LkwJZ2keigRncBQYADVBZl685EBDqVkZ7HIHdDQ==
  log: .bboxy/log/2026-11-05.jsonl#134064
---
# C1 Dream contract handoff

Task: [[tasks/2026-11-05-dream-c1-contract]] (01M6NNCSYCDXV8Y0ZG6YFTGD8K).
Author: Sable Quill, authenticated Architect seat 01M6NJ9RS9EXJ7M5AK1AY1B3JR, generation 1.
Brief followed: current amended task digest 8f8854b7c75865a2756153229e7e70ae54f8582d8d689d68c04b06c3af663904, including isolated post-session worker and observable responsiveness refinements.

## Outcome and locators

Created only the two scoped C1 documents and this append-only report, via knowledge_write:

| Record | Record ID | Current digest |
|---|---|---|
| [[interfaces/dream-term-c1]] | 01M6NP15EEZYG86TF9Z6GDMER7 | 0741de61162cd8f8308046cf286f588d3fc0e9250f50541a0ed8845db4e067dd |
| [[specs/dream-term-c1]] | 01M6NP71V62A4WE211ZW5FCRGH | c3eb58e89bf45294d899b67d2cf41f26a1a5faac9340870f32daff94d99eb2fa |

Both writes returned lossy=false; complete paged knowledge_fetch readbacks matched the exact intended bodies. Parent integration/research/ICD records were read, not changed. Knowledge records may be Git-backed; no stale private/uncommitted-only rule was adopted.

The contract selects direct launch of the already built/bundled sibling harness, with a proposed early inspect-c1 headless dispatch and one harness-owned serde/schema/fixture authority. Public Inspect/Status/Cancel, worker Hello/Start/Activity/Cancel/Progress/Done, manifests, pages/results, errors and scalar/framing/hash rules are specified. Neither this new subcommand nor make test-dream-c1 exists yet; they are explicit implementation outputs.

C1 source closure has one ownership story: term-c1-managed harness sessions suppress the existing detached D0 replay, normal durable SessionClosed plus retained per-seat bookmarks trigger/coalesce the host run, and an isolated bounded worker consumes only immutable selected inputs. An open/idle/turn-ended source, native EOF or observation gap cannot start inspection. Foreground work on the owning seat defers/pauses expensive units without becoming an all-seats-idle gate. C1 emits event-only D0 episode/fingerprint/cluster metadata and explicit not-requested/not-evaluated knowledge/trust labels, with no inference, task catalogue, KB/provider call, global source sweep or shared Dream cache.

The producer/consumer file split, fixed resource limits, duplicate/cancel/restart/ABA/result custody semantics and minimal formal/process/control matrix are in the spec. Responsiveness is a future measured acceptance gate with isolated stalls/CPU/flood, bounded worker units, independent control lanes, independent-root progress, poll/input/message/frame deadlines and causal hot-owner/busy-wait controls.

## Evidence per acceptance criterion

| Criterion | Evidence |
|---|---|
| Exact wire/control/error contract and provisioned runner grounded in code | ICD scalar/control/manifest/worker/result definitions; term main.rs:10061-10063 and wake-up launch.rs:810-834 establish that harness-exec is a seat-readiness exec wrapper, not an internal Dream worker. Existing bundle-relative adapters/mod.rs:72-98 and Makefile:282-284,645-651 supply the chosen sibling executable. Harness main.rs:9-14 and headless.rs:71,81-84 establish the required early dispatch seam. |
| Fully specified own-seat source/effect ownership, bounds, duplicates/restart/cancel/ABA and durable results | Spec Closure/admission/lifecycle, limits and lifecycle case table; ICD permissions, SourceKey/original-op digests, exact source/blob checks, nonce/framing and immutable result validation. Existing d0.rs:656-670,207-243,372-434,492-517,621-627 identifies why blind reuse of the close/replay wrapper would duplicate work or broaden IO. New selected-input bounded D0 entry point is explicit. |
| Exact component split and formal/control/process checks with Make commands | Spec Two implementation components and checks: harness producer owns canonical wire/schema/fixtures and selected D0/close suppression; term consumer owns typed mirrors, private catalogue/jobs, transport/MCP/ctl. Minimal finite initial states/owners/actions/assumptions, named causal controls and real private stall/flood/large-input/native/process witnesses. Both repositories must add make test-dream-c1. Existing harness make test-formal-parity/check-embedded-build and term formal/contracts/land targets are distinguished; baseline harness has no make land. All future checks are unrun here. |
| New spec/ICD/report, actual diff check and no source changes | git diff --check exited 0 with empty output after document writes. Both HEADs unchanged, harness status empty; term status contains only untracked KB records listed below. No production/submodule edit, stage, commit, build, test, recruitment, assignment, merge, install, publication, credential access or desktop action performed. |

Source references use the named crate roots as specified in the documents. The source map is narrower than the parent research: only C1 producer, source observation, process/dispatch/provisioning and canonical gate seams were inspected anew.

## Verification, commands and worktree state

At 2026-11-05T11:10:49Z, date -u returned Thu Nov 5 11:10:49 UTC 2026 (exit 0).
Term git rev-parse HEAD: ad243195daf74153756308346d01a56390c1261c (initial and closing, exit 0).
Embedded harness HEAD: a2ca8f393630081b6be583151293ae286dc2419e (initial and closing, exit 0).
git -C crates/bboxy450-harness status --short: empty, exit 0.
Required git diff --check: empty, exit 0.

Closing term git status --short, exit 0, before this report:

    ?? .bboxy/interfaces/dream-term-c1.md
    ?? .bboxy/interfaces/dream-term.md
    ?? .bboxy/reports/2026-11-05-dream-term-integration-architecture-review.md
    ?? .bboxy/reports/2026-11-05-dream-term-integration-architecture.md
    ?? .bboxy/research/dream-term-integration.md
    ?? .bboxy/specs/dream-term-c1.md
    ?? .bboxy/specs/dream-term-integration.md
    ?? .bboxy/tasks/2026-11-05-dream-c1-contract.md
    ?? .bboxy/tasks/2026-11-05-dream-term-integration-architecture.md

This report adds only .bboxy/reports/2026-11-05-dream-c1-contract.md. Earlier records/task briefs were pre-existing; they were not edited in this C1 assignment. No Git commit was created (commits=[]), and nothing was staged or pushed. The selected provider supplied authenticated record provenance. git diff --check covers tracked changes; exact tool readbacks separately verify these untracked document bodies.

Twenty-nine shell commands exited 0. One initial bounded package-location probe named absent scripts/macos-app.sh; the final pipeline still exited 0 and printed a discovery diagnostic. Actual Makefile build/package lines were subsequently read directly; the failed path was not treated as source absence or a successful product check.

~~~
01 exit=0 rg -n 'harness-exec|build-harness|bboxy450_harness::|dream|run_cli' crates/bboxy450-term/src/main.rs crates/bbt-mux/src/adapters/mod.rs crates/bboxy450-harness/src/frontends/headless.rs Makefile
02 exit=0 nl -ba crates/bboxy450-harness/src/dream/d0.rs | sed -n '8,52p;396,500p;547,655p'
03 exit=0 git rev-parse HEAD
04 exit=0 git -C crates/bboxy450-harness rev-parse HEAD
05 exit=0 nl -ba crates/bboxy450-term/src/main.rs | sed -n '10045,10071p'
06 exit=0 rg -n 'harness_exec|harness-exec|exec_harness|headless::|frontends::headless' crates/bboxy450-term/src crates/bbt-mux/src/adapters crates/bboxy450-harness/src/lib.rs
07 exit=0 nl -ba crates/bboxy450-harness/src/frontends/headless.rs | sed -n '7,89p'
08 exit=0 nl -ba crates/bboxy450-harness/src/dream/d0.rs | sed -n '244,358p;492,547p'
09 exit=0 nl -ba crates/bboxy450-harness/src/runtime.rs | sed -n '1,145p'
10 exit=0 rg -n 'exec_when_ready|bboxy|Command|exec' crates/bbt-mux/src/adapters/launch.rs crates/bbt-mux/src/adapters/mod.rs
11 exit=0 nl -ba crates/bboxy450-harness/src/main.rs | sed -n '7,125p'
12 exit=0 nl -ba Makefile | sed -n '2190,2215p'
13 exit=0 rg --files crates/bboxy450-term/src/ctl crates/bbt-mux/src/logic crates/bbt-mux/tests | rg 'mod.rs|oracle|control|history|harness|rich'
14 exit=0 nl -ba crates/bboxy450-harness/src/store.rs | sed -n '18,67p;597,600p'
15 exit=0 rg -n 'pub fn exec_when_ready|harness-exec|fn.*harness|bboxy450-harness' crates/bboxy450-harness-wake-up/src/launch.rs
16 exit=0 nl -ba crates/bbt-mux/src/adapters/mod.rs | sed -n '68,105p'
17 exit=0 nl -ba crates/bboxy450-harness-wake-up/src/launch.rs | sed -n '55,135p'
18 exit=0 [discovery diagnostic present] rg -n 'bboxy450-harness|HARNESS' scripts/macos-app.sh scripts/*.sh Makefile | head -n 50
19 exit=0 rg --files crates/bbt-mux/src crates/bbt-proto/src crates/bboxy450-term/src | rg '(^|/)(logic|mcp|ctl|lib|proto|control)(/mod)?\.rs$|/mcp/mod.rs|/ctl/mod.rs'
20 exit=0 nl -ba crates/bboxy450-harness-wake-up/src/launch.rs | sed -n '804,861p'
21 exit=0 nl -ba Makefile | sed -n '278,288p;642,658p'
22 exit=0 rg --files crates/bboxy450-harness/tests crates/bboxy450-term/tests/formal_contracts | rg 'formal|dream_replay|seat_process|models.rs|rich'
23 exit=0 rg -n '^(land|push|test-formal-contracts|test-dream-c1)[ :]|land-fast|land.sh' Makefile crates/bboxy450-harness/Makefile
24 exit=0 date -u
25 exit=0 git diff --check
26 exit=0 git rev-parse HEAD
27 exit=0 git status --short
28 exit=0 git -C crates/bboxy450-harness rev-parse HEAD
29 exit=0 git -C crates/bboxy450-harness status --short
~~~

Knowledge operations: orient/knowledge_orient; authenticated task fetches including both amendments; full parent spec/ICD/research fetches with continuation; scoped knowledge_write and complete readback of the C1 documents; explicit message acknowledgements; creation of this report. No provider was invoked outside the selected host interface.

## Gaps and exclusions

No product, schema, formal/Yarn/TLC, unit/integration/smoke/e2e, responsiveness, CPU/heap/RSS or installed-artifact check ran. The contract gives the implementation gates and required measurable outcomes; it does not claim measured smoothness. Tests must use private homes/accounts and recorded owned children, with no operator input.

Required implementation dependencies are the new producer schema/worker/bounded D0 API and legacy-close suppression, the consumer catalogue/control/process integration, their conformance witnesses and deliberate submodule integration. The existing baseline runtime cannot be labelled C1-conformant before those changes pass. Missing closed-source provenance or a producer capability is an explicit unavailable/deferred outcome, not a guessed source, secret prompt or separately installed helper. No later Dream stage, new UI, general provider bridge or broader audit was designed or assigned.
