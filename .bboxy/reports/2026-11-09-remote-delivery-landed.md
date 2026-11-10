---
type: report
title: 'RemoteDelivery refinement and generated card landed: final evidence'
task: '[[tasks/2026-11-09-remote-delivery-conformance-and-card]]'
author: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
created: 2026-11-10T07:49:11.870Z
commits:
- 4ea5787d9844a7a46b310466deebbeb5fc2dcac1
- 97c1a4d870a8dce35dab20ad688ee86cc00300d9
- e5eb99944d217430c6b528174554e70f9b8e8691
verify:
  command: make land (run by Oracle; /tmp/oracle-land2-01M71H0S.log)
  exit_code: 0
supersedes: '[[reports/2026-11-09-remote-delivery-refinement-and-card]]'
id: 01M726XBQ6R5TX46GQP23N87E5
updated: 2026-11-10T07:49:11.870Z
number: 1794292861627429
created_by: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
updated_by: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
authorship:
  signer: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
  kind: seat
  public_key: xS6ZxckblswqLUO9Zm1+nxVwiXjlRrhSxibqVH2YIJg=
  process_generation: '2'
provenance:
  digest: sha256:18efc3435e470cdceff1f8b2c0026cc188745c43c20a3b4266bd736783e4d555
  frontmatter_digest: sha256:4d80d1d754831e835d3071b776dd6db6ff8a676100591f56e23e260681c6729d
  signer: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
  sig: qWifE8deQDiIQB8lv/aEkZ/vePs/7UsiprIP5YpH1SN5AeVe+uCAvu72gk5G3Bb1D6RMi/laeyuNpd3KKKDzDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151404
  event:
    id: 01M726XD61GB201WPQ9ZYQ0EF4
    sequence: 151404
    hash: 3fdb241ac089f63520e6a0ae7e481b49d2082a9a029ec68fd3a3b1b2839a5c93
---
The three implementation commits are on origin/main, with the final source at **e5eb99944d217430c6b528174554e70f9b8e8691**. The Oracle ran the successful final `make land`; I inspected its actual log, source history, signatures and retained formal receipt for this close-out. No build or test was run during close-out.

This supersedes [[reports/2026-11-09-remote-delivery-refinement-and-card]] and, through it, [[reports/2026-11-09-remote-delivery-conformance-and-card]]. Their blocked conclusions are historical. This report is a separate native knowledge checkpoint; the source commit trailers already identify this report locator.

## Landed source and custody

| Signed commit | Mechanism |
|---|---|
| `4ea5787d9844a7a46b310466deebbeb5fc2dcac1` | Exhaustive runtime refinement walk, runtime broken control, generated component card, stale-card and Mermaid checks, contract mapping. |
| `97c1a4d870a8dce35dab20ad688ee86cc00300d9` | Check the term consumer's actual feature closure; retain both isolation rules and the real ACP negative control; sync parent-record fixtures and restore mandatory contract check labels. |
| `e5eb99944d217430c6b528174554e70f9b8e8691` | Replace stale harness helper-budget inventory, add eleven executed contract names, sync the replacement spec-language diagram fixture. |

Close-out inspections: `git log -3 --format='%H %G? %GS'` exited 0, showing **G** signatures by Boxy (bboxy450) <bboxy450@gmail.com> for all three. `git branch -r --contains e5eb99944` exited 0 and listed origin/main. `git status --porcelain=v1` exited 0 with no output. HEAD is the final landed SHA; the source worktree is clean. Rebase changed earlier reported SHAs; the three above are the landed identities.

## Files and mechanisms

- `crates/bboxy450-term-remote/tests/delivery_witness.rs:210` — runtime operation mapping calls the real Outbox/Inbox APIs. Flush maps to ordered Send events; actual Inbox arrivals alone create Applied frames.
- `delivery_witness.rs:376` — breadth-first enumeration, shortest-path replay with fresh runtime/journals, every enabled operation, decision/Fresh-versus-Apply/projected-state comparisons, all three invariants, and an observation-quotient consistency check. It reuses the model encoding, event map and checked graph.
- `delivery_witness.rs:456` — pins coverage; `:472` uses the same walk with a journal recreated empty after crash and requires an Apply/applied disagreement on a crash path.
- `crates/bbt-yarn/src/verify.rs:638` — card module inside the existing project-owned verification module; `src/lib.rs:67` reexports `bbt_yarn::card`. Reuses Model::check, ProtocolMetadata and the private token hash. No new crate, dependency, parser or hash primitive.
- `verify.rs:658` — deterministic graph quotient, first matching view predicate, merged labelled edges, omitted self-loops, and refusal of incomplete checks, violations, unnamed states or more than twelve view nodes. Includes claims, checked bounds, temporal scope, all case labels, waivers, assumptions and source identity. Inline Switch tests cover collapse, deduplication, determinism, scope/hash, failures and the thirteen-node limit.
- `crates/bboxy450-term-remote/tests/delivery_card.rs:14` — message-A views from model variables; existing serde_json reads the registry's single-line JSON-compatible TOML assumption string. Compares the generated card and fixture; only BBOXY_UPDATE_CARDS=1 writes them.
- `crates/bboxy450-term-remote/cards/RemoteDelivery.md` — five nodes, six quotient edges, 52 reachable model states, source hash `fnv1a64:2e535fa905c04e68`; generated “Does not promise” includes no forged Applied frames, no fairness and no liveness.
- `crates/bbt-mermaid/tests/fixtures.rs:40` includes the card fixture in the existing parse/layout/geometry/determinism checks; `:360` checks equality with the card's Mermaid block. Fixture: `tests/fixtures/cards/RemoteDelivery.mmd`.
- Nine literal record fixtures under `crates/bbt-mermaid/tests/fixtures/bboxy/`: `2026-11-05-python-sdk-proposal-superseded-1.mmd`, `programmatic-execution-patterns-1.mmd`, `programmatic-tool-access-1.mmd`, `spec-first-os-{1,2,3}.mmd`, `spec-language-1.mmd`, and `workspace-provisioning-{1,2}.mmd`. They satisfy the existing tracked-document corpus check as parent records arrived. `tests/fixtures/README.md` carries exact origin credits. The final spec-language fixture matches the replacement diagram, with no orphan retained.
- `crates/bbt-yarn/README.md` documents the hosted card API.
- `scripts/harness-submodule.py:71` keeps the pure two-rule validator over explicit feature sets. `:87` reads `cargo tree -p bboxy450-term --locked -e normal,build --prefix none -f '{p}|{f}'`, unions repeated selected package rows and handles Cargo's deduplication marker. ACP control adds `--features bboxy450-harness/acp`. Workspace metadata remains for its separate resolution check.
- `scripts/test-harness-submodule.py:29` keeps the live consumer/control test; `:50` checks that standalone defaults do not leak, consumer ACP is rejected, repeated instances merge, and malformed output is refused. Existing cases remain, including preserve_order without acp-agent.
- `verification/contracts.toml:880` / `:891` keep mandatory `make test-formal` and `make test` check labels. RemoteDelivery records the operation walk and explicit causal-ack assumption, retaining every prior assumption/property/bound. `verification/coverage.toml:262` names the new witness; its harness row inventories executed replacement models and the eleven omitted contract names.

## Exhaustive walk and acceptance evidence

State: two identities A/B, separate outbox and applied-journal owners, harness-owned connection/message/Applied frames, initially empty and connected. Operations are Submit, Flush, Deliver, Ack, Drop, Reconnect and Crash. Each replay compares real API behavior with the generated step; Flush is a sequence of model Send actions. Safety is REMOTE-DELIVERY-001..003. No eventual-progress or scheduling fairness claim is made.

`/tmp/bboxy-refinement-walk-01M71H0S.log:40` prints the pinned counts; `delivery_witness.rs:462` checks them:

| Measure | Count |
|---|---:|
| Reachable harness observation states | 72 |
| Enabled harness transitions | 502 |
| Model states reached, including intermediate flush steps | 50 / 52 |
| Model edges exercised | 298 / 572 |
| Unexercised model edges | 274 |

Unexercised edges by kind: **AckArrive 84, Arrive 84, Drop 4, Reconnect 4, RemoteCrash 4, Send 86, Submit 8**. The walk is exhaustive over the runtime harness alphabet and observation quotient, not every independently enabled model edge. Independent sends, unavailable link deliveries, intermediate batched states and empty-flush stutters account for this distinction.

Runtime negative control: **Submit(a), Flush, Deliver(a), Crash** produces **FailureKind::Applied**, because the recreated memory journal lost a while the model retained it. An IO/build/unrelated failure cannot satisfy the test. The positive FileJournal walk and existing model/resume broken controls pass.

The card is generated by the checked graph, capped at twelve nodes and concretely five. A scratch spec guard changed by `&& true` compiled and ran; the stored-card assertion failed (nextest exit **100**) with hash `2e535fa905c04e68 -> 4cc39d846ea92b48`. Separate log assertions required completed compilation, that stale-card failure and both hashes, and rejected compile failures (exit 0). Scratch source remains outside Git at `/tmp/bboxy-card-scratch-01M71H0S.rs`; temporary integration tests were removed. Evidence: `/tmp/bboxy-card-scratch-final-01M71H0S.log:26`. The affected-crate verify command and final landing both run the real card and Mermaid checks.

## Mapping corrections and abstraction gaps

The initial direct probes exposed two real API/model mismatches: Outbox accepts an Applied receipt before a modeled Inbox arrival; sendable sends the entire batch rather than one message. Oracle Amendment 1 corrected the mapping: causal link ownership plus batched refinement. No runtime behavior was changed to conceal those results.

- Accepted acknowledgements retire their message frame from the projected link, matching model inflight retirement. Applied frames remain for duplicate acknowledgements. Late message frames after retirement are outside this projection.
- Empty Flush is a stutter and verifies candidate Sends would be ignored; it does not claim to execute all model self-loops.
- Drop/Reconnect have no runtime decision return. Classification uses the harness-owned link flag; later flush/ack calls check actual Outbox behavior. Private down/inflight fields are not directly exposed.
- The environment does not forge Applied frames; each is produced by a real Inbox arrival on the current connection. This is an explicit assumption, not transport-authentication proof.
- Two identities only. Journal-and-apply is atomic in the model; real fsync precedes caller apply, so a crash between them can lose a turn and is unknown. Mid-operation crashes, IO failure injection, arbitrary IDs/payloads, SSH/authentication, session epochs/resume, cross-host routing and arbitrary Rust execution are unproved here.
- The card collapses message B and connection state; it is a view, not a replacement model. Metadata has no outcome-kind field, so it displays every case label, including reject/ignore outcomes. It makes no liveness claim.
- Final native/TLC graph parity remains bounded evidence; it does not establish a complete TLC replacement or arbitrary implementation correctness.

## Every landing failure and its disposition

All Executor `make land` failures below have outer Make exit **2**, land-fast exit **77**, and retained logs in /tmp. No gate was bypassed.

| Executor log basename | Failing step / cause | Correction |
|---|---|---|
| bboxy-card-land-01M71H0S.log | init-submodules: workspace-wide feature union included standalone harness acp defaults after parent a58a0fcfb; actual embedded consumer selected only embedded. | Amendment 3 authorized consumer-closure resolution in the two scripts. Both isolation rules retained. |
| bboxy-card-land-final-01M71H0S.log | formal infrastructure: missing mandatory checks for RemoteResume (same labels required on RemoteDelivery); two Python errors. | Restored the registry's make test-formal/make test labels, no semantic weakening. Formal infrastructure then passed 33 tests. |
| bboxy-card-land-retry-01M71H0S.log | java-parity inventory: unexecuted model referenced by bboxy450-harness. Exactly 20 retired Q2HelperTotalBudget names remained from parent 91fc1c4ad after harness b665598 replaced that family. | Removed exactly those 20; added all 22 uncovered executed Helper/Budget replacements. Other rows/fields unchanged. |
| bboxy-card-land-budget-sync-01M71H0S.log | java-parity: project contract missing from crate coverage. Eleven already executed contract names absent from every row. | Added those eleven to the harness row; Oracle's precheck printed OK before retry. |
| bboxy-card-land-inventory-final-01M71H0S.log | Mermaid corpus: every_bboxy_mermaid_block_has_a_fixture failed after Oracle record push e6b328869 replaced the spec-language diagram. | Copied the current source block into spec-language-1.mmd, retired the obsolete fixture content and kept provenance. |
| bboxy-card-land-held-base-01M71H0S.log | java-parity: ENOSPC writing target/formal/models/DreamReadOwnershipGraph.log. | Reported exact failing tail; disk and control transport recovered externally. No checks bypassed or peer processes/targets removed by me. |

The eleven inventory additions are HarnessDreamProofToolsBinding, HarnessDreamProofToolsClosureBoundary, HarnessDreamProofToolsComponentHost, HarnessDreamProofToolsCustody, HarnessDreamProofToolsHiddenWork, HarnessDreamProofToolsPretestBasis, HarnessDreamProofToolsPublication, HarnessDreamProofToolsSignatureCorpus, HarnessDreamProofToolsSourceSeven, HarnessDreamReadForegroundStopTerminal and HarnessDreamReadObserveOrTerminal. Replacement helper details and the exact removed 20 names remain in `/tmp/bboxy-helper-budget-inventory-01M71H0S.json`.

Before landing, implementation checks also caught: initial probe enum-pattern compilation (101, fixed syntax), intended direct-API counterexamples (100, Oracle mapping amendment), six baseline missing Mermaid fixtures (100, restored), source-notice classification of an attempted separate Yarn card file (make test-licenses 2, folded into existing owned module), generated extra EOF blank line (diff check 3, corrected generator), and duplicate inner/outer inline docs (clippy 101, removed duplicate doc). These were fixed; no assertion, lint or license check was suppressed.

## Executor commands and actual exits

Cargo invocations used the existing scripts/cache-env.sh. Repeated runs are listed only where a failure, source change or final-source check distinguishes them.

| Command | Exit | Evidence |
|---|---:|---|
| cargo nextest run -p bboxy450-term-remote --test delivery_conformance_probe --no-capture | 101, then 100 | /tmp/bboxy-remote-conformance-probe-01M71H0S-build-error.log; corrected enum pattern, then both diagnostic counterexamples executed in /tmp/bboxy-remote-conformance-probe-01M71H0S.log. |
| cargo nextest run -p bboxy450-term-remote --test delivery_witness --no-capture | 0 | /tmp/bboxy-refinement-walk-01M71H0S.log; counts and negative-control reason printed. |
| cargo nextest run -p bboxy450-yarn --features model --lib -E 'test(card::)' --no-capture | 0 | /tmp/bboxy-card-unit-01M71H0S.log. |
| BBOXY_UPDATE_CARDS=1 cargo nextest run -p bboxy450-term-remote --test delivery_card --no-capture | 0 on generation runs | /tmp/bboxy-card-generate-01M71H0S.log, bboxy-card-generate-final-01M71H0S.log, bboxy-card-generate-source-final-01M71H0S.log. |
| cargo nextest run -p bboxy450-term-remote --test delivery_card_scratch --no-capture | 100 on both deliberate mutations | /tmp/bboxy-card-scratch-01M71H0S.log and bboxy-card-scratch-final-01M71H0S.log; final-log Python assertions exit 0. |
| cargo clippy -p bboxy450-term-remote -p bboxy450-yarn --all-targets -- -D warnings | 0; 101 after inline move; final 0 | /tmp/bboxy-card-clippy-01M71H0S.log, bboxy-card-clippy-source-final-01M71H0S.log, bboxy-card-clippy-final-01M71H0S.log. |
| cargo nextest run -p bboxy450-term-remote -p bboxy450-yarn -p bboxy450-mermaid && make test-formal-contracts | 100 initially; then 0 twice | Initial 125/126 passed, six missing literal fixtures; final 126/126 affected-crate tests, 45 native formal tests, app clippy, QoS witnesses, 19 Python tests and expected-failing QoS controls. Logs bboxy-task-verify-01M71H0S.log, bboxy-task-verify-final-01M71H0S.log, bboxy-task-verify-source-final-01M71H0S.log. |
| make test-licenses | 2 initially; final 0 | bboxy-card-licenses-01M71H0S.log; bboxy-card-licenses-final-01M71H0S.log. |
| git diff --no-index --check /dev/null crates/bboxy450-term-remote/cards/RemoteDelivery.md | 3 | Detected generated extra EOF blank line; fixed before final staged diff check. |
| git add -- <exact scoped paths>; git diff --cached --check; signed commits / amended signed commits | 0 | No broad staging. Original source commit 29220555af79f797132315679986228879b54654 and rebased fd38ec4c0196cf3437653a05112938ab56fd6d49 verified with approved Boxy key; final rebased identities above. |
| cargo tree --locked -p bboxy450-term -e normal,build,features -i bboxy450-harness | 0 | The consumer selected embedded only; diagnosed workspace union before changing the check. |
| python3 -I scripts/harness-submodule.py --check-json-features | 1 before; 0 after | Before: parent feature isolation rejected: bboxy450-harness/acp. After: consumer JSON features (ACP-agent includes preserve_order): alloc, default, float_roundtrip, std. Logs bboxy-harness-feature-before-01M71H0S.log / bboxy-harness-feature-after-01M71H0S.log. |
| python3 -I scripts/harness-submodule.py --check-json-features --acp-control | 1, expected | bboxy-harness-feature-control-01M71H0S.log: parent feature isolation rejected: bboxy450-harness/acp, serde_json/preserve_order. Live unit test asserts exact exit 1 and acp diagnostic. |
| python3 -I scripts/test-harness-submodule.py | 0 | 11 tests; bboxy-harness-feature-tests-01M71H0S.log and bboxy-harness-feature-tests-final-01M71H0S.log. |
| python3 -I scripts/test-harness-submodule.py && python3 -I scripts/harness-submodule.py --check-json-features && ! python3 -I scripts/harness-submodule.py --check-json-features --acp-control && cargo nextest run -p bboxy450-term-remote -p bboxy450-yarn -p bboxy450-mermaid && make test-formal-contracts | 0 | Full amended verify command; /tmp/bboxy-task-verify-amended-01M71H0S.log. 11 Python feature tests, 126 affected-crate tests, 45 native formal tests, QoS witnesses, 19 Python QoS tests and controls. |
| python3 -I scripts/test-formal.py | 0 | 33 tests after restoring mandatory labels; /tmp/bboxy-formal-infrastructure-final-01M71H0S.log. |
| python3 -I /tmp/oracle-formal-precheck.py | 0 | Both precheck logs print precheck: OK after eleven additions; reuses actual corpus/coverage/contract checks against generated manifest. |
| make land (six Executor attempts) | 2 each | Exact failure table above, underlying land-fast 77. |
| Read-only close-out date, history/status/signature/containment, source/diff/log and receipt hash inspections | 0 | No builds; receipt SHA-256 matches its directory name; clean source worktree and all landed signatures G. |

Non-test diagnostic no-match rg / already-exited ps / optional missing-log reads in earlier work returned 1 as recorded in the superseded reports. They are not verification evidence. The first close-out knowledge lookup of this not-yet-created report returned not_found; existing reports were found and read before writing this one. No report write success is inferred from that lookup.

## Oracle takeover and final gate evidence

After disk recovery and the app restart, the Oracle took over landing in this worktree. I obeyed its instruction to run no make/cargo/git while it owned the run; orient and task_status worked at process generation 2.

An earlier Oracle log, `/tmp/oracle-land-01M71H0S.log`, ends with **Terminated: 15** during test-formal, without a final exit receipt. Its termination cause and shell exit were not retained in the evidence I inspected; I do not infer them or count it as a pass.

The Oracle ran **`make land` → exit 0**, final log `/tmp/oracle-land2-01M71H0S.log`. It contains two complete landing-gate passes, not two full `make test` runs:

- First gate make exits 0 at line 2477; push rejected non-fast-forward. At line 2485 land rebases/re-signs and repeats the affected checks.
- Final pass runs clippy for Mermaid/remote/Yarn, Mermaid fixtures, remote bin build and lib/bin/delivery_card/delivery_witness tests, Yarn lib tests, dependent checks for book/doc/harness/term/mux, and `make test-formal FORMAL_REUSE=1`. Each recorded child command exits 0.
- Line 4958: clean current-source formal receipt for **e5eb99944d217430c6b528174554e70f9b8e8691**, **943 models, 35 crates, 288 contracts**, native/TLC parity held. Reuse is explicitly source/tool/model bound by the existing gate.
- Line 4962: **LANDED e5eb99944d217430c6b528174554e70f9b8e8691**. Line 4963: final land exit **0**, wall 8002.834 seconds.

Retained receipt: `/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/74c9bed7047316797e046c8c9ca82051240165d52965e39f1a8b7e9c7781f879/evidence.json`. I read it: source equals final SHA, dirty=false; SHA-256 is **74c9bed7047316797e046c8c9ca82051240165d52965e39f1a8b7e9c7781f879**.

Final gate log SHA-256: **1a1ce28c9d0bc5337f121c8e836a2bb2f6fc4c5100f2982c25df7e83d44655d0**. Executor amended-verify log SHA-256: **b7fbf7d88364a1a2b91fc3acf8ea6f3138355b6f69a90876f284b30914528452**. Walk log SHA-256: **dd3e880d3b99cceadf55b99d065f3cc13cac9f3b5791f8e332b3270566d0aab4**. Scratch final log SHA-256: **0365d49f8138c3389db569317066ade27be103ef3854d46a66a7dbadbf6aad0e**.

## What was removed

Removed the scripted single-trace witness and its step helper, the initial direct-API diagnostic probe, and temporary scratch integration tests. Folded the attempted standalone card file into the existing verification module and restored private token_hash visibility. Removed duplicate module documentation instead of suppressing a lint. Replaced the feature check's workspace-union interpretation with the consumer closure. Removed exactly 20 retired Q2HelperTotalBudget inventory entries and replaced obsolete spec-language fixture content. No production link/delivery behavior, protocol syntax, invariant, rule, check or prior assumption was removed.

## Gaps and What generalizes

Full `make test`, installation, a separate `make smoke`, GUI parity and process-boundary suites were not run for this unchanged runtime/UI. The successful lean landing and formal gate, affected tests and declared abstraction bounds are the evidence claimed. Existing rust-objcopy missing-libLLVM warnings and ignored non-root patch warnings did not prevent execution; no toolchain was changed.

**What generalizes:** the next component can reuse `bbt_yarn::card::generate`, its existing metadata/model/check/hash primitives and Mermaid corpus tests. View predicates and assumptions remain component-owned. BFS plus replay applies to another API once its atomic operations, batching and environment are written down; the remote alphabet, observation projection and journal factory stay test-local. Feature isolation must query the consuming package's selected closure, and formal coverage inventories must track the pinned producer's executed model family. No new generic runtime-walker layer was added.

Implementation and landing are complete. Acceptance, task closure and seat closure remain with the Oracle.
