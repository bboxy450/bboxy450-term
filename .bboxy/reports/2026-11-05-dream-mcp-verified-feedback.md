---
type: report
title: 'Dream Q3: pinned codec, honest unavailable lessons and reported-only feedback refusals'
task: '[[tasks/2026-11-05-dream-mcp-verified-feedback]]'
author: seat:01M6YC2FT3YFH5DH6NJX7N8VJX
created: 2026-11-09T05:45:57.804Z
commits:
- 1a7e2814ad751911b9a88c3f2ee05fcd6e0a5e10
- 1d28995f21dccc5cd9c68cd31f194d1d8ea6d5ea
- 42bf96aa8b5b291ec893428e872fa7591b4a0c45
- 31e13b90ab0b330748d77e225794f3292a2cea29
- 2a372e10bc17096f8278b2f7d5b132952d0f7664
- 91fc1c4ad0a311bd2eaa99e8251018e47c73f320
verify:
  command: make test-dream-mcp && make test-formal-contracts
  exit_code: 0
id: 01M6ZDF0FJ8SKR05FPYZEKSGPD
updated: 2026-11-09T05:45:57.804Z
number: 93377961366438
created_by: seat:01M6YC2FT3YFH5DH6NJX7N8VJX
updated_by: seat:01M6YC2FT3YFH5DH6NJX7N8VJX
authorship:
  signer: seat:01M6YC2FT3YFH5DH6NJX7N8VJX
  kind: seat
  public_key: NM35glDu1XyemjvPzSibMSHIWxscqbEJFCzph0wmsPI=
  process_generation: '1'
provenance:
  digest: sha256:27ac67faaf9791bd8ceec8316ec215be0d9ea20ca939d30c882d9eb8138e05cb
  frontmatter_digest: sha256:60bb84407db0041c0c8ca563fc2731b53c74e2558fbfa683100bd18138cf0f08
  signer: seat:01M6YC2FT3YFH5DH6NJX7N8VJX
  sig: kuPq8kugsue/htsVUv7LbwCEdsepEjsImlXvxn62VGv17sfepWzoZTUG85AFASkIbMbSPnIA9L9qcLLQeWQ/Dw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150943
  event:
    id: 01M6ZDF1CNGZ02BWCXMT8V4856
    sequence: 150943
    hash: d392b4c0e4e6d3af1bc534931d76e885309aec5864be52982c626ffe426dfa36
---
Landed the narrowed Q3 assignment on term main at **91fc1c4ad0a311bd2eaa99e8251018e47c73f320**, with harness **645fec78174f59b06a9d6973271fc5e8d4ef013e**. The final canonical landing exited 0. All six source commits have valid Git signatures; the tracked worktree and submodule were clean, and `git ls-remote origin refs/heads/main` returned the landed term SHA.

This report implements the Oracle's “NARROW this landing” and “harness-bump landing; wake-up C1 compatibility” amendments to [[tasks/2026-11-05-dream-mcp-verified-feedback]], followed by the signed peer dispositions selecting the final harness pin and the new-source landing. Governing behavior remains [[interfaces/dreaming-mcp]], [[specs/dreaming-mcp]] and [[decisions/d93377961364692-dream-mcp-structured-host-hints]]. The original positive-admission scope is deliberately unfinished and assigned to [[tasks/2026-11-08-dream-q3b-readprepare-transport]].

## Delivered behavior and source evidence

- `crates/bboxy450-harness-wake-up/src/dream_read.rs:415` pins the read-prepare/1 producer codec; `:422` recursively sorts canonical JSON, `:438` bounds decoding, `:449` checks Hello protocol/algorithm/schema/IR compatibility, and `:488` validates entries. This is a strict codec mirror, not a ReadPrepare process transport or snapshot installer. The module is exported from `src/lib.rs`.
- `crates/bboxy450-harness-wake-up/tests/dream_read.rs:54` consumes the producer DTO/negative corpus; `:89` independently checks schema, lock, fixture hashes and canonical vectors; `:135` rejects changed source revisions. Ten producer DTOs are covered by four tests, without an external service.
- `crates/bbt-mux/src/registry/dream_read.rs:654` returns honest `no_published_input` for lessons; historical observations do not become qualifying offers (`:877`). The lesson search/read/context/evidence shapes and compact recovery hints expose unavailable state; no draft or historical fallback is promoted.
- `crates/bbt-proto/src/dream_read.rs:226` defines strict feedback requests. At `:238`, evidence is bounded to eight pins and notes to 512 UTF-8 bytes, without NUL; explicit null and unknown fields refuse. The only successful assessment DTO is `reported_only` (`:269`). Used remains a caller report, never verified use.
- `crates/bbt-mux/src/registry/dream_read.rs:649` refuses feedback for every unknown offer. This host currently makes no qualifying offers, so this slice records no positive feedback or adoption claim. `:139` journals original-operation/request-digest outcomes through the existing own-seat generic journal, checks current public authority, parses only declared outer host fields separately, and rechecks authority before reply. Lost-response retries replay the original refusal; the same operation with changed arguments returns `op_id_reused`.
- `crates/bbt-mux/src/logic/dream_read.rs:29` creates only the existing journal handle; `:51` keeps read/journal work off the logic owner. This is not the deferred offered-content provenance journal, and no Dream operation-status support was added.
- `crates/bbt-proto/src/dream_read.rs:534` retains closed domain variants; `:589` declares outer `op_id` and shared-hints fields in the advertised output schema. `crates/bbt-mux/src/mcp/dream_read.rs:51` uses the common host projection, and `:61` charges the full serialized response against 16 KiB and caller context budgets. Whole pages/hits reserve guidance space before selection. No independent renderer, control-version bump, nested payload mutation or global unknown-field relaxation was introduced.
- `crates/bbt-mux/src/mcp/hints.rs:68` gives original-op recovery guidance without inventing Dream operation-status support; `:81` explains offer/report/use/Recall limits; `:88` explains unavailable lesson/proof/transport state. Guidance survives a client that discards content and metadata in favor of structuredContent. Repeated socket/bridge projection is idempotent.
- `crates/bbt-mux/tests/dream_read.rs:326` exercises real admitted retries; `:339` lost-reply equality, `:343` changed arguments, `:363` foreign offer/evidence, `:371` final schema and matching text hints, `:380` poison-note non-echo, `:395` no positive report, and `:400` zero host custody/signing spies. Existing private native-client coverage at `crates/bboxy450-term/tests/dream_read.rs:151` launches a throwaway headless app/account; `:218` discovers tools, `:240` checks cold unavailable reads, and `:285` checks structured guidance. No live account, operator prompt or provider call was needed.
- `crates/bboxy450-harness-wake-up/src/dream_c1.rs:1817` canonicalizes recursively despite joined serde_json preserve_order features; `:1940` emits the fixed Start envelope in producer order. `tests/dream_c1.rs:92` compares raw prepared bytes to the signed producer fixture before the framing LF. Existing round-trip assertions remain. No readiness window, resource budget or transport behavior was weakened.
- `scripts/landing.py:557` detects a real gitlink-only harness update. Its feature/owner selection (`:605`, `:628`) checks the harness library with term's features, runs harness lib tests, and runs full tests/doctests for all term consumers and transitive dependents. Harness-package tests/*.rs remain owned by its own canonical gate. `scripts/land-fast.sh:59` consumes this plan; `scripts/test-landing.py` adds seven HarnessBump cases and explicit support-file ownership fixtures. LAND.md documents the policy.
- `scripts/qos-policy.py:19` registers the existing `dream::read::helper_tests::` timing family at default QoS under the standing rule. Assertions, timing windows and budgets are unchanged. `dream_c1` was already in TIMED_TARGETS (`:15`); no new registration or ignore was needed for the later readiness disposition.
- `crates/bboxy450-term/tests/formal_contracts/dream_read.rs:17`, `verification/corpus.toml` and `verification/contracts.toml:1922` add DreamReadFeedback and six broken controls: foreign offer, foreign evidence, ABA, digest mismatch, duplicate promotion and authority promotion. The contract explicitly distinguishes prospective eligible-offer modeling from the shipped refusal-only witness. `verification/coverage.toml:254` maps the complete 442-model imported harness corpus: 161 previously unmapped Q2 models (16 positive, 145 controls) were added in producer order. No checker, corpus verdict or coverage enforcement was loosened.

Generated source retains compact origin/policy credit. Root ATTRIBUTION.md and upstream notices remain intact; license checks passed.

## Verification and current-source receipts

The final command was **`BBOXY_CACHE_MODE=off make land` — exit 0**, log `/tmp/dream-q3-bd8e-disposition-land.log`, SHA-256 `525941172ee0169e11100a4813e5b00f07147cd081a8ed63ee5b96db912573d2`. Its canonical rebase produced the six SHAs listed below. It printed `LANDED 91fc1c4ad0a311bd2eaa99e8251018e47c73f320`; no failing test was retried in that run.

| Final landing selection | Actual result |
| --- | --- |
| Clippy of the five native term consumers, all targets, warnings denied; harness production lib check with embedded features | exit 0 |
| bboxy450-harness lib, embedded-only | 208 passed; 11 skipped |
| bboxy450-harness-host full tests | 58 passed; 43 skipped; doctests exit 0 |
| bboxy450-harness-wake-up full tests | 111 passed; 5 skipped; one doctest passed |
| bboxy450-term full tests | 662 passed; 125 skipped; doctests exit 0 |
| bboxy450-term-mux full tests | 890 passed; 79 skipped; doctests exit 0 |
| bboxy450-term-proto full tests | 90 passed; zero skipped; doctests exit 0 |
| Formal stack, including native model checks, contracts, negative controls, export/metadata and harness conformance | exit 0 |
| Final formal-contract selection inside the formal gate | 45 passed |
| Final parity receipt | 861 models; 275 contracts; 34 crates; native/TLC graph/verdict parity held; dirty=false |

These are **2,019 selected package tests with 263 reported skips**, plus doctests and formal checks. “Skipped” is the runner's reported count, including selection and ignore policy; it is not a claim that every repository test ran. Expected negative-control diagnostics (including feature-isolation refusal and scripted publisher retry fixtures) appear in the successful formal stack and are not a real publication or a failed outer gate.

The final source-bound formal receipt is:
`/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/721c95e37193dfe492d25a900c9934a9487abe4d66d34d93099b6777c7cca2e1/evidence.json`
with matching evidence SHA-256 `721c95e37193dfe492d25a900c9934a9487abe4d66d34d93099b6777c7cca2e1`, source `91fc1c4ad0a311bd2eaa99e8251018e47c73f320`. All 861 model/tool/graph results used validated reuse from receipt `32a5b1153c5d94da1206391c5dda2233aac65faeadd9486b46d1f6bc87cda960`, after current-source native phases and coverage validation; no fresh per-model Java exploration ran in this final pass. This is finite bounded conformance, not a complete TLC replacement or arbitrary Rust correctness proof.

Tool pins in the receipt: TLC 1.7.4 SHA-256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`; Java 21.0.13+9 Darwin-arm64 archive SHA-256 `225bab0a09491ef06835488cd942f5d2befa9a0410a2484f0c8bbcf7506ec09a`; checker SHA-256 `c3db469a2f535f13f4d2a37dc2f1d872af3388c399171f0d31260a794050aa65`. Source, generated model and graph hashes are retained in the receipt.

**`BBOXY_CACHE_MODE=off make check-formal-evidence` — exit 0** after landing, log `/tmp/dream-q3-final-check-formal-evidence.log`, SHA-256 `1ed085eb2df1c39d3be23d8459fdd48391592fce5c5445d1e599c81bff6dfd13`, validates this exact clean source.

Focused verification required by the task was completed sequentially, both exit 0:

| Command | Evidence |
| --- | --- |
| `BBOXY_CACHE_MODE=off make test-dream-mcp` | 34 passed: 16 C1 + four producer-codec + four proto + nine mux + one private native-client test. `/tmp/dream-q3-bc4-mcp-resume.log`; SHA-256 `f576c1ed8ca4057e741eecdadc5332c33a3eb0de0d6b96d80d955e84138e522b` |
| `BBOXY_CACHE_MODE=off make test-formal-contracts` | 45 contract tests, two runtime QoS, two app QoS, 19 Python tests and latency probes passed. `/tmp/dream-q3-bc4-contracts-resume.log`; SHA-256 `70e0e600b46d2e14008db817648a1da08fd363d3ac549ddd0fcbc4cad0e53b68` |
| `make test-landing` | 226 fixture tests, zero failures; `/tmp/dream-q3-landing-final.log`; SHA-256 `91cd4a7f79103c3afa2ef0c2a92df856e43f50f241da9b5cd3b4d156dd280349` |
| `make test-licenses` | 34 crate notices and 6/10/9 fixture suites passed; `/tmp/dream-q3-licenses-2.log`; SHA-256 `4cfdbd198212dae27719f5474cf8a16ea306847f6158592e505ed8fd453810cc` |
| `git diff --check`; signature and remote-main checks | exit 0; all six source signatures G; remote main matched HEAD; tracked tree clean |

The focused MCP/contracts logs precede the final pin (source then 27134e2, harness bc4f7fad). They are not relabeled as final-source runs: the final landing ran the full affected consumer suites and formal stack against harness 645fec, covering the same tests on the landed source. The Makefile extends test-dream-mcp with the producer codec and C1 byte checks.

## Acceptance accounting and limits

| Requirement | Evidence or disposition |
| --- | --- |
| Producer pin/conformance | Four codec tests pass against the actual pinned submodule corpus; final full wake-up suite passes. Original mirror origin is f848c03ff0462e55dfa0b17fcfd550626a52db2b; schema SHA-256 `0afcbe084c959118cea8d3b49008e5cd9654d7ff5caa8565bf3691fe2ce7a08a`, corpus-lock SHA-256 `00f0e61b07be56f1019227a50ad1ada55835bdfbaced9d1b3e5830c2e6247665`. |
| Missing/stale/foreign/authority changes | Existing Dream read witnesses and final full mux suite pass, including actual grant/revocation/restore and stale/foreign refusals with zero host custody/signing spies. No positive lesson page exists to claim Q2 seal/grant admission. |
| Unknown-outcome feedback, body mismatch, foreign offer/evidence, poison | Real socket/bridge retry test passes. Unknown offers refuse and refusals dedupe. Strict note/evidence bounds hold; no knowledge, policy, ranking or trust mutation. |
| Shared structured hints/schema/budgets | Structured-only fixtures validate final hinted schema, matching text hints, original operation/pins and repeated projection; existing whole-page/near-limit and poisoned historical-text controls pass. Positive historical observation results do not establish a lesson offer. |
| Real backend/native routes | Existing admitted engine/socket/bridge and scripted native fixture pass on the same backend. No new writer, Recall item, successful injection, citation-as-injection or model-use claim. |
| Feedback/ownership/snapshot state and controls | New feedback model plus six controls and existing ownership/snapshot controls pass. Prospective positive feedback and full ReadPrepare lifecycle/process settlement remain Q3b. |
| Eight benefit scenarios and live-model validation | Explicitly deferred. No benefit score or improvement claim; no provider/model experiment. |
| Existing C1/resource/four-job/large-input health | All selected term consumer tests and harness lib tests pass. Harness-package integration tests were explicitly removed from term's pointer-bump gate by the Oracle; they were not re-established here. The original “all gates/no skip” line is not claimed fulfilled. |

Harness 645fec is the Oracle-selected signed descendant containing terminal-loss recovery (6409ebc3), Q2/fixed retirement lineage (bc4f7fad), final card (80af03fd), ACP test gating (92cdaef9), harness serialized timing lane (4a62faf1) and INPUT67 (1f1b30dd). That harness lane is implemented in its own landing script; term's embedded lib test selection does not reproduce the harness's separate serialized release lane.

The accepted pin carries an upstream KNOWN-DEFECT ignore for
`dream::resolve::selected_helper_process_tests::external_total_budget_terminal_wait4_refuses_unseen_and_rolled_overrun`,
tracked by `tasks/2026-11-08-harness-known-bill-group-proof-race`. This is part of the eleven harness-lib skips; its behavior remains unverified by this landing. No harness source was edited in this task.

Deferred to Q3b: real read-prepare transport/provisioning and Hello-before-input process wiring; logic dream_c1 producer runtime pin; provider published-proof export; real settled snapshot installation/invalidation; offered-content provenance; Dream operation_status; positive lesson/feedback admission; heavier process witnesses and eight benefit scenarios. No native memory, credentials, provider synthesis, trial, publication, install, service signup or spend occurred.

## Failed/interrupted runs and dispositions

All reported failures were preserved rather than bypassed. Subsequent runs below used changed source/pins or an explicit Oracle disposition.

- Initial `LAND_MODE=wide make land`: 2,571 tests ran, 2,516 passed, 55 failed. The later doctest compiler was stopped after the Oracle withdrew that gate (owned wrapper exit 130, gate 77, make 2). `/tmp/dream-q3-land.log` and the Oracle copy at the main checkout's `target/oracle-evidence/dream-q3-wide-land-2026-11-08.log` preserve output. `/tmp/dream-q3-blocked-evidence.json`, SHA-256 `3827186e4a72e1b5eb47cad971355e982fe2f5fdb59ea0981f0659a9073b557a`, names all 55 failures, including all 16 term-crate failures (three wake-up canonical-byte, twelve mux, one proto). The final full consumer suites pass all sixteen; no remaining term test failure is suppressed.
- First lean landing: make 2 on conflicting f848 versus operator 520823 gitlink. A transient MERGE_RR.lock initially impeded auto-abort; after it was absent, explicit own rebase-abort exited 0. Oracle authorized keeping the operator pin while rebasing. Log `/tmp/dream-q3-lean-land.log`.
- bc4f7fad and 80af03fd landings: make 2/gate 77, compile E0433 in two unguarded harness ACP relay tests. Logs `/tmp/dream-q3-bc4-land.log`, `/tmp/dream-q3-80af-land.log`. Oracle held work, then supplied a pin containing the feature guards.
- 4a62faf1 landing: make 2/gate 77, two embedded-only harness lib failures (python_emit key order and inline-code chip witness). `/tmp/dream-q3-4a62-land.log`. Oracle supplied 645fec with both fixes and the disclosed upstream ignore.
- First 645fec landing: all 2,018 selected package tests passed; formal parity refused the missing parent mapping of imported Q2 models. `/tmp/dream-q3-645f-land.log`, SHA-256 `9bf6ab1a387ff2ed11e3c62339d344b848c9ea71f54a9d5137914d243b434a4e`. I corrected the 161 missing mappings, preserving coverage enforcement.
- Coverage-fix landing: make 2/gate 77 on **bboxy450-harness-wake-up::dream_c1::actual_private_process_clears_authority_environment_and_uses_fixed_argv**, at `tests/dream_c1.rs:361`: the private protocol fixture missed its original framing-readiness window. 110 passed, one failed, five skipped. Log `/tmp/dream-q3-645f-coverage-land.log`, SHA-256 `ec1bc0ce641908d32a299d07a71d152ec5095268cfffc57a4c0269778e008813`. The Oracle classified the intermittent failure and explicitly authorized the current new-source run after FORMAL_REUSE completed. That run passed all 111 wake-up tests. The target was already in TIMED_TARGETS; the conditional readiness ignore was **not added**, and its window/assertions are unchanged.
- `BBOXY_CACHE_MODE=off make test-formal FORMAL_REUSE=1`: exit 0 on the coverage fix before final rebase; 861 models/275 contracts/34 crates, receipt 32a5b115… above. `/tmp/dream-q3-coverage-formal.log`, SHA-256 `0846fc73f812e52d60e653274ae094eb5a5f2a4149ae006cb10fcba2add917c0`. The final canonical land refreshed source binding to 91fc1c4.

Optional sccache stalled without a compiler; only the recorded own Cargo process was interrupted, then the supported BBOXY_CACHE_MODE=off override was used. During an earlier compile disk fell below 8 GiB, so that own process was stopped and the Oracle notified. Only exact obsolete generated binaries in this worktree were removed, with receipts `/tmp/dream-q3-obsolete-wide-binaries-cleanup.json` and `/tmp/dream-q3-obsolete-pin-artifacts-cleanup.json`. No peer targets were deleted. The successful final landing stayed above the floor and ended with about 13 GiB free. No broad process kill or shared cache/config change occurred.

## Signed source handoff

1. `1a7e2814ad751911b9a88c3f2ee05fcd6e0a5e10` — codec, honest lesson/feedback replies, shared hints, tests/formal controls.
2. `1d28995f21dccc5cd9c68cd31f194d1d8ea6d5ea` — canonical C1 signed-byte compatibility.
3. `42bf96aa8b5b291ec893428e872fa7591b4a0c45` — pointer-only consumer gate, fixtures and QoS family.
4. `31e13b90ab0b330748d77e225794f3292a2cea29` — fixed-Q2 lineage pointer update.
5. `2a372e10bc17096f8278b2f7d5b132952d0f7664` — final embedded-compatible accepted harness pin.
6. `91fc1c4ad0a311bd2eaa99e8251018e47c73f320` — complete imported Q2 coverage mapping.

Each carries `Report: [[reports/2026-11-05-dream-mcp-verified-feedback]]`. The final net diff contains 23 allowed files, no harness-source edits and no net Cargo.lock change against the rebased main. Source commits are already on remote main. This report is checkpointed separately through the native knowledge provider; .bboxy is never force-added to the source repository. Oracle acceptance and closeout remain separate.
