---
type: report
title: Q2 independent parser, hashing, helper-accounting and model checkpoint — final gates red
author: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
created: 2026-11-06T16:06:05.787Z
task: '[[tasks/2026-11-05-dream-mcp-verified-snapshot.md]]'
supersedes: '[[reports/2026-11-05-dream-mcp-verified-snapshot.md]]'
commits:
- f5b0363c59e24ea33f577906415cbee0233f4bc7
- 5eb3f804b3036ac7408bd3dba31a543c7dfc72a7
verify:
  command: make check-embedded-build
  exit_code: 0
id: 01M6RSRB1M3ZCTVH5JN4KT33SP
updated: 2026-11-06T16:06:05.787Z
number: 93377961366025
created_by: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
updated_by: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
authorship:
  signer: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
  kind: seat
  public_key: p1xg5YnZmsPv/B3q4mjUXp1coPPED4IjwlMtDzJrDaE=
  process_generation: '1'
provenance:
  digest: sha256:c0be9590937a75335a53d05f4178f4b8c759d7178ab835c89ec07b0a0983826a
  frontmatter_digest: sha256:32b40eed55f8a4d53b77f61e881decd494cb44ebcf04a135816f40594605742c
  signer: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
  sig: 9RjfCjocoF01Rq6oUJWXhwylf/Cul1RNMTreAXarBSzYhbvTyb1n6kYT8PkQX82xaz2mYC6h/d9gPvrMtrV3DQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149531
  event:
    id: 01M6RSRCA7SVR91GN0VRYVK2JD
    sequence: 149531
    hash: e8399596678fe6f58e9b8e728fdc6d64e9e38b5ee62de551a6c798b5c2b87a17
---
Independent continuation checkpoint for [[tasks/2026-11-05-dream-mcp-verified-snapshot.md]], following [[reports/2026-11-05-dream-mcp-verified-snapshot.md]]. **Q2 remains unfinished and unqualified.** No task_report, landing, push, installation, service/credential action or release. This supersedes the prior checkpoint's current-status section, preserving all historical results.

## Signed source and mechanisms

Code commit f5b0363c59e24ea33f577906415cbee0233f4bc7, parent5eb3f804b3036ac7408bd3dba31a543c7dfc72a7. git verify-commit exited0, Ed25519 key9B07E849480C68328CA20684844ECC0587DE7AB0, Boxy <bboxy450@gmail.com>. Branch feat/dream-read-q2 remains based on origin/main a75eabfa. Tracked tree is clean; all ten inherited observer JSON files remain untouched.

- src/dream/c1.rs:1348 / helper_account_controller_clock:3120 — substantive grant/observation finalization after the earlier CPU cutoff now has a measured, charged tail. Open uncertainty stays in the original grant. Proven stopped/settled work receives one fresh ordinary debt unit. No extra unit on idle control ticks; pending proof/post CPU/late CPU/corroboration flags and revisions remain intact. Unknown/over-limit tail preserves identity and finite failure debt. Subsequent scalar receipt/debt instrumentation remains inside the enclosing raw control measurement, with no recursive work/debt grant or arbitrary-native theorem.
- src/dream/c1.rs:6471,6533 — clock-injected API controls prove exactly-once tail charging, unchanged idle debt, pending observation preservation, sticky failed-clock/limit refusal and retained identity. This is bookkeeping coverage, not a real Git/GPG lifetime witness.
- src/dream/resolve.rs:141 — compiled canonical probe-source SHA256 now passes through measured16KiB checkpoints. The prior whole-source hash could exceed the ordinary1ms batch and stop before the intended qualification refusal. Exact digest bytes and existing admission semantics are unchanged. No approved profile was populated.
- tests/dream_read.rs:2857 and tests/fixtures/dream-read/private-input-controls.c —27 synthetic source-free outer decoder vectors exercise the actual canonical C parser: full-width u64/i64/u32 bounds; overflow; zero incarnation; decimal grammar; duplicate/missing/unknown/null fields; noncanonical spaces/BOM/LF/escapes; invalid UTF8/surrogates/NUL; enum/root failures; truncation and16KiB bound. Parsing creates no helper or qualification. This does not implement/substitute the pending builder-owned inner closure codec.
- tests/formal/dream_read.rs:1895,5320 — new AccountingTailDebt causal control preserves the original member-window bounds/actions and fails when post-seal work loses fresh debt. The control observer initially caused TLC branch expansion/timeouts. Final implementation selects every affected failed-boundary predicate and evaluates its Boolean value once; every named invariant stays in the model. An exhaustive native comparison checks EVERY original-versus-optimized state, labelled adjacency entry and named violation trace for all28 controls, not just counts. This changes no reachable graph, bound, case, cap, fairness assumption or checker timeout.
- tests/formal/models.rs and verification/{contracts,corpus,coverage}.toml — register the additional paired control and source/API mapping. Corpus now394 records; TOML parsed and record names are unique.
- Makefile / ApprovedRunner source list — bind the new C parser fixture in the existing private source snapshot.
- scripts/land.py:102 — changed dream_read integration runs the EXISTING canonical make test-dream-mcp, including its exact worker/profile preparation and proof/resource checks, instead of an unprepared generic nextest invocation. No gate is waived. A simulated routing/error test executes no publication and confirms the canonical Q2 gate runs once, its failure propagates, and formal verification stays required.
- ATTRIBUTION.md retains project name, canonical policy URL and credit for these implementations.

## Actual commands, exits and evidence

All logs are local under target/dream-read-finish-recovery. SHA256 values below cover exact retained bytes.

| Command / scope | Actual result | Evidence |
|---|---|---|
| make test-dream-c1, BEFORE the new tail/source-hash changes | exit2; C1 integration31/31 and replay16/16 passed, two retained-host seat_process tests failed | current-c1-after-rebase.log; b5ceb96795067512b5787135e3bf1d1cbe85f3e5652217903ade2a9c8ecd8915 |
| cargo test --offline --locked --release --jobs2 --target-dir target --lib dream::c1::helper_admission_tests:: -- --test-threads=1 --nocapture | exit0,7passed | controller-finalization-tail.log; 08321fb1e89c971510f71aecdffc89133474b4129ae895501078b52c4ca6ddee |
| cargo test with the same offline/locked/release/jobs2/target settings, --test dream_read private_candidate_outer_codec_refuses_noncanonical_and_overflow_before_birth -- --test-threads=1 --nocapture | exit0,27vectors, one test passed; helper_created=0/runtime_approved=false | private-outer-codec.log; 6562557d15197b6756f66678f81d0f1dcad66dae1bf6b8c387adf87c35c59435 |
| cargo test with those settings, --lib prepared_qualification_absent_and_forged_claims_refuse_before_native_birth -- --test-threads=1 --nocapture | initial exit101 at1,072,541ns ordinary unit, then exit0 AFTER the paced-source-hash implementation; intended absent/forged/stale/private-result refusals, zero native calls | qualification-refusal.log SHA36cb522bab891e910c782e64fa886db3184dab1babc4353205192f419dca961b; qualification-refusal-paced-source.log SHAddc5f69be00624bc6ec4008bf02d13155f97407942f9eb067a006683137b352f |
| cargo test --test formal_models dream_read_member_windows_preserve_required_closure_and_additive_debt (same settings) | exit0; all required causal verdicts including new tail control | member-window-tail-model.log SHA07020e48bc2ddafc64e49b627f99df73b23a3bfb7ea4f86f77ac2a672011d3fe |
| cargo test --test formal_models member_window_observer_guard_keeps_every_original_state_edge_and_named_verdict (same settings) | exit0; full graph/trace equality for all28 controls | member-window-compact-boolean-equivalence.log SHA826dbb520d68ec0b9d99af558d157ea975f44e0c0676b562aaa4f82777bae773 |
| source-bound Python narrow differential using unchanged tests/formal/export.rs and scripts/formal.py | exit0; all29 member-window models fully explored by native and pinned Java TLC; complete state+labelled-edge sets equal; good Java exits0/0,28 controls0/12; original native graphs equal | member-window-compact-parity-5y7oamqw/evidence.json SHA168765a4c7989af862514c41e6c185fc4baba762ac29714610ecb26eadccd256 |
| cargo test --lib dream::read_wire:: / dream::worker::tests / dream::read::helper_tests (same settings) | exit0 each;3/6/17tests passed | scoped-wire.log SHA94f47f5cd6d62f463107ffcb9014ba169901e22610ea7a4934a4b622a64aa26e; scoped-custody.log SHAa90cac5290f96f39dbab6265abffd1aa2a922d37d2d41ad83336127ec615f53d; scoped-helper-clock.log SHA56d37b0bba9f6e8778eae3452145c0192a4bb4237f15d11cf798122271b8a93f |
| cargo test --lib selected_helper_process_tests (same settings) | exit101;6passed/5failed; NOT GREEN | scoped-helper-process.log SHAf7eccdf127f42c1516695e869c6d6083e05867ed853cec3e536ffc73302fe041 |
| make check-embedded-build AFTER the paced-source change | exit0: strict embedded clippy, default library and ACP/standalone compilation | embedded-after-paced-source.log SHAb523df41b84ddf8b95f2e6ef30a823ae8a18c6be53890b4b18d30520fdff974c |
| Python simulated landing routing/error exercise | exit0; no actual GPG/export/publish/land invoked | landing-q2-routing.log SHA515e4e7f2c696c129c5e75c760aca28ce0bde1952b4a206b6bfa2ceaf2f732aa |
| scoped rustfmt / git diff --check / TOML and inventory validation / git commit+signature verification | exit0 | exact source in signed commit |

The29-model good graph has52,106states/150,635labelled edges. AccountingTailDebt has51,295states/148,259edges and the expected safety verdict. No timeout, cap, partial exploration, TypeOK-only result or backend disagreement was accepted. This is NARROW family parity, not the final whole-repository make test-formal-parity.

Three earlier narrow TLC attempts failed with the unchanged60s limit on UnequalStop: original full observer, selected predicates alone, and Boolean wrapper alone. Their exact TLA/CFG/native graphs/logs remain at member-window-tail-parity-zv14qkhp, member-window-observer-parity-c0kajqnm and member-window-boolean-parity-f7pawsqu. A proposed .ite expression did not compile against the actual pinned0.1.1 checker; its failed build is member-window-predicate-equivalence.log SHA4cde5a7ba185a515423b2a0cd535388dc2d34f9a5bcfd26aa78303483772bbdd. The final combined supported-Boolean/predicate refactor passed exact original graph equivalence and all29 pinned TLC comparisons. No timeout/cap/tool/bound was raised; these were implementation changes, not lucky repeats.

Exact source hashes, actual rustc/clang paths/versions/hashes and native-checker rlib hash are in independent-bindings.json, now naming commitf5b0363c59e24ea33f577906415cbee0233f4bc7. The differential evidence additionally records Java/JAR/source/model hashes, compiler argv and every graph/verdict exit. It is not a frozen qualification tuple or a worker/runtime approved profile.

## Red gates and precise next prerequisites

1. **Shared C schema is still absent.** Root message01M6RPRAJ96AHZJWAVWVGHJP4J confirms the signed builder-owned closure-boundary.schema.json / closure-boundary-vectors.json do not yet exist and names builder seat01M6Q0FHRZV6MQA8PBPXG408N7. I retained the old decoder's honest prebirth refusal and created no competing inner shape. Coherent C source/executable/compiler/schema handoff, real builder component, Root-reviewed runtime premise and qualifying signed positive/corrupt/four-job preparation remain UNMET.
2. **Required C1 gate is red outside the currently permitted gateway/test write scope.** tests/seat_process.rs:983 reports missing durable SessionClosed in dream_c1_managed_retained_host_preserves_private_launch_inputs and dream_c1_retained_host_creates_one_nonfork_b_after_a_and_keeps_real_input. Those retained-host paths were not changed here. Blocker message01M6RRW8GFEJPD6PCRB9Y5P7CX asks Root to route the gate to its owner or amend exact necessary scope/checks. Receipt says native-queued, not model-consumed. It was not rerun for lucky timing.
3. **Owned passive-helper process conformance is still red.** Five cases fail: admitted public-key stdin transport, distinct-group stale-hook sequencing, descendant/panic witness, foreground pause/cancel, and multi-chunk stdin/EOF. The actual log includes two-second fixture-result/wait timeouts and genuine inclusive charges above1ms, e.g.1,004,117ns for a two-slot grant and1,011,539ns for the stdin witness. Exact effects/retirement remain in the retained receipts. These are failures, not proof of tool qualification or available lessons. No threshold inflation, timing retry, test skip, claimed all-times native/heap theorem or relaxed coverage was introduced.
4. Final complete command remains make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build. It has NOT passed. Full signed-publication/corrupt/currentness/trust/graph, real four-job foreground/control/memory/30s/reap witnesses and whole-corpus current-source parity remain mandatory.
5. No Runtime/Recall/adoption/trust promotion, new producer wire/IR/version/hint fields, source build, provider fallback, installation or publication occurred. Installed tools and absent native/provider bridges remain honestly unproven.

The previous native report checkpoint is committed at ddd831793b0f03bf65ea172c627a84e4374f26ba. artifact_claim nevertheless refused its returned artifact ID and committed report locator as artifact_not_committed (last op01M6RPMA8R8FR7P7HB3C6HTHCY); no claim success is invented or bypassed. This does not replace verification or acceptance. Root retains task/landing/cleanup authority; this checkpoint leaves the assignment active.
