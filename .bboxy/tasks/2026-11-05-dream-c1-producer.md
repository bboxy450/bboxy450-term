---
type: task
title: Implement bounded post-session Dream C1 harness producer
worktree: /Users/feral/mydev/bboxy450-harness-dream-c1
role: executor
allowed_paths:
- src/dream/c1_wire.rs
- src/dream/c1.rs
- src/dream/inspect.rs
- src/dream/d0.rs
- src/dream/mod.rs
- src/frontends/headless.rs
- src/frontends/seat.rs
- src/gateway/host.rs
- src/engine/mod.rs
- src/backend/mod.rs
- src/backend/acp_agent.rs
- src/import/mod.rs
- src/import/watch.rs
- schema/dream-c1-v1.schema.json
- tests/dream_c1.rs
- tests/dream_replay.rs
- tests/seat_process.rs
- tests/fixtures/dream-c1/**
- tests/formal/dream_c1.rs
- tests/formal/models.rs
- verification/corpus.toml
- verification/coverage.toml
- verification/contracts.toml
- Makefile
- README.md
- ATTRIBUTION.md
- reports/2026-11-05-dream-c1-producer.md
- src/frontends/gui/bridge.rs
- tests/acp_launch_context.rs
- src/dream/fingerprint.rs
- src/main.rs
- src/mcp.rs
- tests/mcp_client.rs
- reports/2026-11-05-dream-c1-producer-review.md
verify: make test-dream-c1 && make test-formal-parity && make check-embedded-build
id: 01M6NPQKKAKT5PNJDEEY2A1QYQ
status: accepted
delegated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T17:00:31.693Z
created: 2026-11-05T11:15:31.904Z
number: 93377961364374
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
assignee: seat:01M6NPMWMJ27PAN93N9BY4KY5J
artifacts:
- '[[01M6NYFEM871JC9EBHS3BT4R2V]]'
- '[[01M6P0A1TK12DZS1XQYY8GCX32]]'
- '[[01M6P43M37RWGRD1QVN6YQV5E2]]'
- '[[01M6P5WJHFRY9VP8K7BFRED2D1]]'
- '[[01M6P6BEPJVE4J6MW6WD5T7J49]]'
- '[[01M6P9M1WQKG5XQT5PFSGZJJN3]]'
- '[[01M6PAF7WVG99456AC76KTRY1K]]'
verify_result:
  command: make test-dream-c1 && make test-formal-parity && make check-embedded-build
  exit_code: 0
  evidence:
  - criterion: Canonical wire/schema/fixtures and exact worker provisioning
    reference: src/dream/c1_wire.rs; schema/dream-c1-v1.schema.json; tests/fixtures/dream-c1; target/dream-c1-review-47182320-final.log; target/dream-c1-production-47182320/receipt.json
  - criterion: Selected closed IR/raw/blob/law validation, bounded D0 and single legacy close owner
    reference: src/dream/inspect.rs; src/dream/d0.rs; src/dream/mod.rs; tests/dream_c1.rs; tests/dream_replay.rs; target/dream-c1-review-47182320-final.log
  - criterion: Dedicated bounded compute/control, stepped rejected-result ownership, credit release and actual settlement
    reference: src/dream/c1.rs:248; src/dream/c1.rs:313; src/dream/c1.rs:341; src/dream/c1.rs:421; tests/dream_c1.rs:1354; target/dream-c1-review-47182320-final.log
  - criterion: Current-source ownership models, causal broken controls and pinned native/Java parity
    reference: tests/formal/dream_c1.rs:445; verification/evidence/a9099420f7d2414001cdd15b9b2867ec5c3c9c18a1284d4dc2f42e4995d10306/evidence.json; target/dream-c1-review-47182320-evidence-check.log
  - criterion: Canonical focused gate, affected existing process/privacy tests and attribution
    reference: Makefile:282; target/dream-c1-review-47182320-final.log; target/dream-c1-review-47182320-licenses.log
  - criterion: Signed commits, exact paths/mechanisms, measured exits, preserved artifacts/exclusions and clean tree
    reference: reports/2026-11-05-dream-c1-producer-review.md (#93377961364973); source47182320be6435c860bc2176c2ddda582029fa63
report:
  summary: 'R1 corrected in signed47182320be6435c860bc2176c2ddda582029fa63: successful typed returns retain an external stepped owner through CPU/heap/clock/control rejection, nested disposal completes before credit release/Done, with owned Drop probes and near-limit parsed-envelope failure/Cancel/process witnesses. Required make test-dream-c1 && make test-formal-parity && make check-embedded-build, licenses and current-source evidence validation all exit0 unattended;231-model/71-contract native/Java full state/edge/verdict parity. Actual dense rejected parse1562833ns remains a ResourceLimit negative; cleanup maximum348041ns, live credits0, actual exit1373ms. Corrected ordinary-release no-fixtures worker SHA2562c8469829bc94171048677c590b8427b48076512cb04178ad1b2bdcd711beabd handed off with receipt; old artifacts preserved. Own origin/main fetch/rebase exited0/already up to date, tracked tree clean. Append-only superseding report records complete source, logs, measurements and exclusions; consumer integrated results remain outside producer-only certification.'
  commit: 47182320be6435c860bc2176c2ddda582029fa63
  reported_by: seat:01M6NPMWMJ27PAN93N9BY4KY5J
  op_id: 01M6P9TW9Y0GVHXZVZXPN46BK9
  verify_revision: 0
  report: '[[reports/2026-11-05-dream-c1-producer-review.md]]'
rejection:
  reason: 'Final-source review confirms R1 at src/dream/c1.rs::Budget::unit: result=f() at:295 is implicitly dropped on unavailable end-clock (:297-298) or elapsed/heap rejection (:304-310), before the paced cleanup path:312. Actual Envelope-returning b.track at:870 can therefore destroy owned parsed state outside measured/paced cleanup. This is a concrete ownership/accounting defect; no runtime excess-latency measurement is claimed. Fix within existing C1 paths by retaining large returned ownership in an external stepped Paced slot across every rejection branch, or an equivalent bounded mechanism; merely adding one unbounded drop inside a cleanup closure is insufficient for a near-limit owned envelope. Add over-budget/clock-failure owned Drop-probe controls and near-limit parsed-envelope failure/cancel disposal witnesses proving measured/paced cleanup completes before Done/settlement and credits release correctly. Keep all limits; update relevant ownership model/broken control coverage honestly. Fetch/rebase own repo origin/main per operator; currently it is unchanged a2ca8f39, so don''t invent a term rebase. Re-run only changed focused checks and the prescribed current-source final gates, refresh retained formal evidence after the new signed source commit, and submit an append-only superseding report. Old4bdafb67 report/evidence remain baseline, not acceptance of the fix. Architect''s remaining fixed-source review continues; address any further precise findings within the same brief.'
  by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
provenance:
  digest: sha256:9a43c17411c66df71237f0022aac66abf85416c1890c1b46d1ccd6a16cd7eb33
  frontmatter_digest: sha256:b010040b03131bdcbae628f9b7de92e895b62fae7d601f52219c05ff39e317fa
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: 8p17rmKJnyrBf+kFCv3jDqP9RR64izBxqaxzCZqfjkD2eyvUuMqFYvKgs40WOOI8eNawTxQWTBx7DTiehxCeBQ==
  log: .bboxy/log/2026-11-05.jsonl#139159
---
ROLE: EXECUTOR

## Goal and governing contract
Implement the harness producer of [[specs/dream-term-c1]] and [[interfaces/dream-term-c1]], accepted by Rook after source review. Fetch both documents completely through knowledge_fetch, with continuations. Parent [[specs/dream-term-integration]] is context, not permission for later stages.
User approved building, then emphasized smooth post-session background execution, no blocking shared async loop/CPU bursts, isolated worker, actual session closure only, take time to get it right.

## Worktree and scope
Your ONE writing source worktree: /Users/feral/mydev/bboxy450-harness-dream-c1, branch feat/dream-host-c1, base a2ca8f393630081b6be583151293ae286dc2419e. Do not edit embedded/shared/sibling checkout source. Read shared/project instructions and relevant source before edits. Allowed source paths are in frontmatter; other files need a concise exact gap report. No new deps/lockfile changes or IR schema/version changes unless separately amended. Retain origin notices; update ATTRIBUTION.md accurately.
Knowledge/report tools remain bound to the selected term knowledge root, not a guessed worktree .bboxy. Latest operator overrides stale installed skill: KB records MAY be committed according to provider workflow. No KB commit is required in this source assignment; runtime input/raw logs/secrets remain outside source Git. Do not recruit, create tasks, publish, merge, install or push. Signed source commits are required; Oracle reviews/lands.

## Producer outcome
Implement the single canonical c1_wire serde/schemars authority, generated JSON schema and valid/invalid/canonical-hash vectors. Implement early dream inspect-c1 --job-dir dispatch BEFORE HOME/cwd/profile/provider/seat bootstrap. Hello must be available without reading the manifest or starting source work; read/validate manifest only after matching Start. This supports the consumer's off-owner artifact capability probe without a separate install/endpoint.
Dedicated low-priority worker ownership, bounded chunk/record CPU and memory/IO, pause/resume, independent Cancel/Activity control, protocol limits, fixed nofollow private paths, exact source/blob/raw-line validation and immutable result/pages MUST follow the C1 ICD. No global scan, Git/task/KB lookup, cache shard write, model/profile discovery or inference. Factor existing D0 semantics into selected-input inspect_closed, not blind replay(paths). Typed --dream-owner term-c1 suppresses legacy automatic D0 in all managed engine/ACP/import close paths without suppressing durable SessionClosed. Normal standalone/manual legacy behavior is outside C1 and must not silently change.
Reactor <=1ms; compute <=1ms CPU units then >=9ms cooperative wait; bounded workers/cancellation and real settlement. Inspect only durable terminal SessionClosed sources, never idle/turn-end/native EOF. Unknown foreground defers expensive work. Record measured heap and RSS separately; do not certify smoothness from spawn/offload names.

## Parallel handoff
The consumer implements against the frozen ICD independently. First produce the canonical wire/schema/fixture files, verify their generation/vector parsing and make one signed self-contained commit, then send ONE essential boundary milestone to Rook's inbox: commit SHA and fixture directory. This is an authorized shared-interface handoff, not a routine progress DM; continue with producer implementation without waiting. No copied second fixture authority.
The consumer may READ your source/fixtures; it must not edit them. Final code still requires the full gate below and report. Address direct interface feedback within scope; scope changes require task amendment.

## Acceptance
- [ ] Canonical wire/schema/fixtures and exact worker entry/provisioning protocol conform to ICD, with independent canonical vectors and missing/unknown/duplicate/oversize negatives.
- [ ] Closed selected-IR + BlobRef/raw-byte/law validation yields exact bounded D0 episode/cluster pages without source/cache/KB/inference side effects; legacy replay suppression has one owner and no duplicate closure work.
- [ ] Dedicated low-priority compute, bounded chunks/allocation, pause/control lanes, deadline/cancel/EOF, immutable no-clobber output and actual child/thread settlement satisfy source-bound tests; no shared async-loop blocking or unlimited CPU burst.
- [ ] HarnessDreamC1Inspection/Worker models, causal broken controls, API/process witnesses, source mappings and pinned native/Java parity evidence added/refreshed; unknown/incomplete/disagreement fails.
- [ ] Canonical make test-dream-c1 added for schema/vector/good/broken/process/large-input checks and affected existing dream_replay/seat_process coverage; required verify command exits 0 unattended.
- [ ] Signed commit(s), exact changed paths/mechanisms, measured checks/actual exits, remaining exclusions and worktree state reported.

## Verification and handoff
Iterate only the targeted gates; one build/test per seat, two Cargo jobs, no full-suite run beyond brief. Final verify: make test-dream-c1 && make test-formal-parity && make check-embedded-build. make test-licenses also required for generated source attribution. Use private fixture homes/identities/owned children; never operator keychain/click/network/live inference. No fabricated green check or threshold relaxation. A failing performance bound is evidence to fix or report, not a skip.
Create append-only reports/2026-11-05-dream-c1-producer.md through knowledge_write: task link, full signed commits, verify command and actual exit, evidence per criterion, measurements/log paths outside Git, exclusions. Then task_report once. Preserve current-source formal evidence after rebase; root Oracle will reuse your actual logs and inspect source before landing. No routine acknowledgement/progress/status messages.

## Exhaustive launch literal compatibility amendment
Add only dream_owner: crate::dream::c1::DreamOwner::Legacy to the exhaustive LaunchOptions literal in src/frontends/gui/bridge.rs around :1921. The literal is in a private GUI bridge profile-switch fixture and omits the new typed owner field; source review confirms this is required compatibility glue for the assigned owner option. That exact file is additionally authorized for this single field addition; do not alter GUI behavior or expand UI work. Existing verification and acceptance unchanged.


## ACP exhaustive literal compatibility amendment
Additional exact path tests/acp_launch_context.rs is authorized solely to add dream_owner: bboxy450_harness::dream::c1::DreamOwner::Legacy (or the equivalent existing import) to its two exhaustive LaunchOptions literals around :146 and :433. Source review confirms both omit the new field and preserve existing private ACP context/reopen semantics. Add --test acp_launch_context to make test-dream-c1's focused integration-test invocation so the existing generation/context checks actually run. The prescribed verify command remains make test-dream-c1 && make test-formal-parity && make check-embedded-build, with this expanded target coverage. No other behavior/scope expansion. Record the ACP target's actual result in the final report.


## Shared source-subset and close-owner decisions
[[decisions/d93377961364460-dream-c1-fork-source-subset]]: C1 rejects SessionMeta.fork at SessionStarted as unsupported_ir, before inherited rows/source work. Add a real Store::fork → close → C1 witness and malformed parent-row control, preserve bytes/no result, and explicitly exclude that canonical IR subset. Do not claim general fork-law equivalence from a non-fork incremental validator. Fetch the updated C1 ICD/spec; wire/error fields are unchanged.
[[decisions/d93377961364464-dream-c1-sole-post-close-owner]] confirms your proposed consolidation: one DreamOwner gate in dream/mod.rs suppresses BOTH automatic legacy D0 replay and IR advice scanning in TermC1 mode, used by engine/ACP/watch. Preserve durable SessionClosed and explicit Legacy behavior; no scanner implementation changes. Require the real large-JSON-blob close/foreground witness and zero legacy hook invocations. These are within current allowed source/test paths; verify command unchanged. Do not wait for another approval.


## Measured normalization cold-start refactor amendment
Authorized exact path src/dream/fingerprint.rs for a semantics-preserving split of its four fixed mask alternatives and per-pattern warmup, to address the measured 1,571,084 ns unit in target/dream-c1-process-fix.log:35. The 1 ms guard correctly failed and MUST NOT be relaxed or bypassed. Initialize/warm each pattern in its own measured/cooperatively yielded C1 unit, including first-use search setup where necessary; measure fresh real worker processes, not only a warmed test binary.
Preserve exact old leftmost match and alternative priority (quote, path, hex, number), case folding, whitespace behavior, output bytes and resulting fingerprints. Sequential replacement that changes overlap/priority is not equivalent. Add equivalence tests against the original combined expression, covering overlap/ties, quoted paths/hex/digits, unclosed quotes, Windows/Unix paths, escapes, Unicode and mixed inputs, plus existing exact fingerprint vectors. Include the fingerprint unit tests in make test-dream-c1 and rerun the real failure/retry/denial worker witness. No new dependency/lockfile, schema/threshold change, or broader normalization redesign. Existing required verify command is unchanged, with the expanded focused target coverage; record the failed measurement and final actual cold-start result in the report.


## Fixed shared heap allocation (current contract)
Implement [[decisions/d93377961364572-dream-c1-heap-allocation]] and updated C1 spec/ICD: producer child 96 MiB TOTAL live heap, comprising 94 MiB work + 2 MiB protected control/settlement reserve; parent owns the other 32 MiB, combined 128 MiB. No borrowing, larger limits or new wire fields. The decision's coverage and before-allocation/fallible reservation rules are normative, including child startup, parser/serde/temporary/library/helper capacities and realloc overlap. Merely lowering HEAP or sampling between units does not satisfy enforcement. Return resource_limit before an unbounded allocation-bearing effect; do not abort/unwind allocation failure in shared infrastructure. Keep control/cancel/error/settlement possible after work exhaustion. Extend the existing focused source/model/control witnesses for independent child overflow, actual parent+child large input, four jobs and separate actual-live/reserved/cumulative/RSS figures. Label max(parent)+max(child) as a conservative upper bound unless simultaneous samples are genuinely synchronized. Coordinate exact joined witness with the consumer within your current brief. Other CPU/source/settlement/verification requirements remain unchanged; existing paths cover the implementation.


## Worker startup path amendment
Authorized exact additional path src/main.rs ONLY for early, strictly recognized dream inspect-c1 worker dispatch before bbt_rt::handle()/shared runtime, ordinary headless/GUI setup and HOME/cwd/profile/provider/seat discovery. Source review confirms current main:14 initializes bbt_rt before the purported early headless C1 path; that startup cannot be excluded from the child/control ledger. Keep ordinary GUI/headless launches unchanged. The exact implementation may use the already-authorized C1 module for its isolated synchronous/dedicated owner entry; do not introduce another global runtime/pool or an unmetered bootstrap. Include bounded argv/environment/startup/control provisioning in the same child accounting/assumptions, and test the actual executable's early Hello with no HOME/profile/capability initialization. Existing wire, limits, source scope otherwise and verification remain unchanged; prior focused samples are baseline evidence, not final heap compliance.


## Superseding enforceable memory rule — replaces universal allocation wording above
Fetch [[decisions/d93377961364581-dream-c1-enforceable-memory-contract]] and CURRENT C1 spec/ICD. This explicitly supersedes the universal native/whole-process allocation-proof wording in the prior heap paragraph/decision: do not keep chasing a source-proof of every pthread/serde/native allocation or reject all otherwise supported jobs. Child share stays 96 MiB: 2 MiB preprovisioned protected control, 16 MiB tested helper/startup-overlap allowance, <=78 MiB other declared C1-owned working storage. Mechanically enforce declared owned capacity/retained-state credits before controllable growth with checked/fallible bounded operations and drop/transfer accounting. Model only that declared credit system. Opaque library/native/runtime overhead is explicitly measured/implementation-only, covered by the decision's fixed structure/thread/helper bounds and regression witnesses, not falsely proven by the ledger. Observed full-child heap <=96 MiB and corresponding combined observed <=128 MiB remain failing gates; helper headroom is not a proven factor or extra memory. Keep all original CPU/duty/latency/closure/settlement tests. The corrected rule defines JSON structural preflight and exact overflow/four-job/overlap/control evidence. Existing early main dispatch scope remains. No limit, wire/schema or dependency expansion. Source report must distinguish enforced credits, actual observed heap, native/startup overhead, cumulative bytes, RSS and coverage limits accurately.


## Private retained-Host same-generation witness amendment
[[decisions/d93377961364606-dream-c1-private-host-session-witness]] authorizes the proposed --dream-c1-fixture-next-session ULID in existing allowed headless.rs/seat_process.rs paths, ONLY compiled under the EXISTING test-fixtures feature and accepted only for explicit seat + json + scripted provider + TermC1 private fixture mode. Exactly one fresh nonfork B through real retained Host after A's durable SessionClosed is published, with actual owned socket/Hello and normal event flow. Reject otherwise; private homes/accounts/owned paths, no ambient profile/capability/credential fallback. No ordinary default/product lifecycle change, new feature/dependency, forged IR/events or relaxed guard. Make test-dream-c1 may enable existing test-fixtures for this named private witness; rejected combinations and exactly-one-transition behavior are required. Label it honestly as a retained-Host API/transport witness, not production CLI close-A/new-B support. Preserve and run ordinary close/exit, generation-change cancellation, final join/reap and Legacy behavior tests separately. Send the consumer the exact fixture invocation/commit as an essential joined-test boundary handoff once ready; continue other required work. Same verification command and other scope/limits remain.


## Preserve explicit private managed fixture inputs
[[decisions/d93377961364648-dream-c1-managed-fixture-inputs]] is the exact disposition requested by the producer/consumer. Under the SAME test-fixtures + seat/json/scripted/TermC1 guards, preserve validated --profile/private-c1 metadata including auto_compact:false, --mcp-config and --append-system-prompt via their normal explicit private paths/read_launch; no silent stripping or metadata substitution. Allow the exact managed environment names/private common-root layout in that decision, including precreated private harness profiles/sessions/run, sibling repository, owned overlay MCP/capability path metadata, and private CODEX/CLAUDE/ZDOTDIR paths. Validator must not read/log capability contents or grant authority from env labels. Reject credentials/unknown/mixed-root/symlink/bad-owner/mode inputs; no ambient provider/onboarding/custody fallback. The separate C1 inspection worker's cleared minimal environment is UNCHANGED. Implement within existing allowed headless/seat_process paths and include valid-managed + guard negatives; hand consumer exact invocation/readiness. Other limits and verify command unchanged.


## Managed MCP public-metadata redaction correction
Implement [[decisions/d93377961364668-dream-c1-managed-public-env-redaction]] in exact added paths src/mcp.rs and tests/mcp_client.rs (inline source tests and already allowed seat_process fixture also available). Only host-origin Launcher entries with the same-name exact $ reference for typed public BBOXY_SEAT_ID, BBOXY_GENERATION and BBOXY_ROLE avoid registration as substring secrets. Validate using their existing public types/shapes; no blanket origin/name-prefix exemption, capability change, redactor rewrite or removal of independently registered secrets. Literal/command/aliased/unknown/profile/ACP inputs keep prior redaction/guards. Add focused exact-classification controls and repeat actual managed MCP+typed UserTurn generation7/op ending07 through TurnQueued/TurnStarted/settlement with intact identifiers. Preserve existing credential stderr/spill redaction witnesses. Add the focused MCP tests to make test-dream-c1 as needed; same verify command. Forwarding and MCP must remain enabled; no source edit outside exact scope.


## Preserve managed explicit approval flag in private witness
[[decisions/d93377961364677-dream-c1-managed-approval-flag]] authorizes the exact preexisting managed --yolo option through the fixture-only argv allowlist, preserving normal policy and LaunchOptions.yolo propagation across retained A→B. Term launch.rs:231-233 already adds it. Do not silently strip it, infer/force it when absent, add aliases or rewrite policy. All test-fixtures + explicit seat/json/scripted/TermC1 + private validated paths/profile/MCP/environment and exactly-one-nonfork-transition guards remain. No product lifecycle, provider/custody/network/credential or separate inspect-c1-worker change. Within existing allowed paths, include actual managed --yolo in the joined positive, verify its ordinary propagation plus absent-flag semantics, and retain malformed/missing-feature/mode/path/unknown-option negatives even with --yolo present. Same final verification command and other limits/scope.


## Report from seat:01M6NPMWMJ27PAN93N9BY4KY5J

Producer implemented in six signed commits through4bdafb676467f4847d42c1ace154e8fe31521eb0; required make test-dream-c1 && make test-formal-parity && make check-embedded-build exited0 with CARGO_BUILD_JOBS=2, and make test-licenses exited0. Current-source227-model/70-contract native+Java state/edge/verdict parity and process/source/credit/large-input/managed generation7-op07/approval/privacy witnesses pass; tracked source tree is clean. Immutable ordinary-release no-test-fixtures worker SHA25698504f509158461773c16aa8588964eb0a433f5fb5e2879e2bb012ac7d276c22 and real Store/fork bundles are handed to the consumer. Full report contains exact source mechanisms, measured heap/RSS/credits, corrected performance failures, exclusions and retained logs; consumer/shared integrated parent-memory/native-frame/authority/hints gates are explicitly not claimed as producer-only evidence.

```json
{
  "op_id": "01M6P7FGRQBQ426KX3635HHMVH",
  "commit": "4bdafb676467f4847d42c1ace154e8fe31521eb0",
  "report": "[[reports/2026-11-05-dream-c1-producer.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Canonical wire/schema/fixtures and early worker provisioning",
        "reference": "src/dream/c1_wire.rs; schema/dream-c1-v1.schema.json; tests/fixtures/dream-c1; target/dream-c1-final-4bdafb67.log"
      },
      {
        "criterion": "Selected closed IR/raw/blob/law validation, bounded D0, one automatic close owner",
        "reference": "src/dream/inspect.rs; src/dream/d0.rs; src/dream/mod.rs:137; tests/dream_c1.rs; tests/dream_replay.rs; target/dream-c1-final-4bdafb67.log"
      },
      {
        "criterion": "Dedicated paced worker, protected credits, control/cancel/EOF/deadline, immutable output and actual settlement",
        "reference": "src/dream/c1.rs; tests/dream_c1.rs; target/dream-c1-final-4bdafb67.log"
      },
      {
        "criterion": "Current-source formal models, causal controls and pinned native/Java parity",
        "reference": "verification/evidence/930f78a4919fbe1a3e2a05e2263e45413f3718e740103230518cf4584f8469b6/evidence.json"
      },
      {
        "criterion": "Canonical focused gate, affected existing tests, licenses",
        "reference": "Makefile:282; target/dream-c1-final-4bdafb67.log; target/dream-c1-final-4bdafb67-licenses.log"
      },
      {
        "criterion": "Signed commits, changed paths, measurements, exclusions, artifact pins and clean worktree",
        "reference": "reports/2026-11-05-dream-c1-producer.md (#93377961364711)"
      }
    ]
  }
}
```


## Review correction report and operator refresh
The confirmed R1 owned-result disposal rejection is part of the original bounded lifecycle/CPU/credit acceptance. Preserve the original append-only report. Exact added report path reports/2026-11-05-dream-c1-producer-review.md is authorized for its superseding correction, with supersedes:[[reports/2026-11-05-dream-c1-producer]], new signed source/command/log/formal/artifact pins and actual limits. Same source paths/verify command. Operator explicitly ordered all seats to fetch and rebase onto their OWN repository's current origin/main; root confirmed harness upstream remainsa2ca8f39 while term main isde089ec2. An already-up-to-date producer rebase needs no gratuitous code change or unrelated rerun. Preserve the old immutable artifact for current consumer development, then hand off the corrected exact release artifact after verification; no landing/pin acceptance is yet claimed.


## Report from seat:01M6NPMWMJ27PAN93N9BY4KY5J

R1 corrected in signed47182320be6435c860bc2176c2ddda582029fa63: successful typed returns retain an external stepped owner through CPU/heap/clock/control rejection, nested disposal completes before credit release/Done, with owned Drop probes and near-limit parsed-envelope failure/Cancel/process witnesses. Required make test-dream-c1 && make test-formal-parity && make check-embedded-build, licenses and current-source evidence validation all exit0 unattended;231-model/71-contract native/Java full state/edge/verdict parity. Actual dense rejected parse1562833ns remains a ResourceLimit negative; cleanup maximum348041ns, live credits0, actual exit1373ms. Corrected ordinary-release no-fixtures worker SHA2562c8469829bc94171048677c590b8427b48076512cb04178ad1b2bdcd711beabd handed off with receipt; old artifacts preserved. Own origin/main fetch/rebase exited0/already up to date, tracked tree clean. Append-only superseding report records complete source, logs, measurements and exclusions; consumer integrated results remain outside producer-only certification.

```json
{
  "op_id": "01M6P9TW9Y0GVHXZVZXPN46BK9",
  "commit": "47182320be6435c860bc2176c2ddda582029fa63",
  "report": "[[reports/2026-11-05-dream-c1-producer-review.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Canonical wire/schema/fixtures and exact worker provisioning",
        "reference": "src/dream/c1_wire.rs; schema/dream-c1-v1.schema.json; tests/fixtures/dream-c1; target/dream-c1-review-47182320-final.log; target/dream-c1-production-47182320/receipt.json"
      },
      {
        "criterion": "Selected closed IR/raw/blob/law validation, bounded D0 and single legacy close owner",
        "reference": "src/dream/inspect.rs; src/dream/d0.rs; src/dream/mod.rs; tests/dream_c1.rs; tests/dream_replay.rs; target/dream-c1-review-47182320-final.log"
      },
      {
        "criterion": "Dedicated bounded compute/control, stepped rejected-result ownership, credit release and actual settlement",
        "reference": "src/dream/c1.rs:248; src/dream/c1.rs:313; src/dream/c1.rs:341; src/dream/c1.rs:421; tests/dream_c1.rs:1354; target/dream-c1-review-47182320-final.log"
      },
      {
        "criterion": "Current-source ownership models, causal broken controls and pinned native/Java parity",
        "reference": "tests/formal/dream_c1.rs:445; verification/evidence/a9099420f7d2414001cdd15b9b2867ec5c3c9c18a1284d4dc2f42e4995d10306/evidence.json; target/dream-c1-review-47182320-evidence-check.log"
      },
      {
        "criterion": "Canonical focused gate, affected existing process/privacy tests and attribution",
        "reference": "Makefile:282; target/dream-c1-review-47182320-final.log; target/dream-c1-review-47182320-licenses.log"
      },
      {
        "criterion": "Signed commits, exact paths/mechanisms, measured exits, preserved artifacts/exclusions and clean tree",
        "reference": "reports/2026-11-05-dream-c1-producer-review.md (#93377961364973); source47182320be6435c860bc2176c2ddda582029fa63"
      }
    ]
  }
}
```
