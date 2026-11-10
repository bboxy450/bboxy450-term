---
type: report
title: Q2 current-source checkpoint — blocked by explicit scope and verification gaps
task: '[[tasks/2026-11-05-dream-mcp-verified-snapshot]]'
author: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
created: 2026-11-06T23:49:39.944Z
commits:
- b1171237cc73fc312cec6324d8ed3766705d8c2f
- 9a0945c830e1e298cc932ae47dc37efff7d8227a
- f5dca2b9dc8fb0afa941358c362378cf79af99c9
- 364349ae2ad5ccf1d8d91bc51e2b0c56b36ffd08
- 0173551016f4ce41988edcc8a1d525ee52d917e9
verify:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 2
supersedes: '[[reports/2026-11-06-dream-mcp-verified-snapshot-landed]]'
id: 01M6SM94ZQB72CD4GH94WX6Y7B
updated: 2026-11-06T23:49:39.944Z
number: 93377961366170
created_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
updated_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  kind: seat
  public_key: 3lIiKAyaYQbqAYQ3xsrS+NTGLp9s//BjqCA4HkK5nF0=
  process_generation: '1'
provenance:
  digest: sha256:395140484c44899b3ee598d39c037a14a8244b5e37c23615c54967f2d777be93
  frontmatter_digest: sha256:0d395003ed15e31e3f6a07c8bd43c0a01e146420413532b2462b4e2fa8c89754
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  sig: qNidvsQDZYcQaHpv886ClFNB7HD0ZylcozBLEKlZojCjzww2LmqQNDwypqD8M/TqF0dXyS3YVNca8hc4LMK3DA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150015
  event:
    id: 01M6SM96K466FVGT9E6ST1EYGV
    sequence: 150015
    hash: 92b0569490b37ba72b4b26b6bfcd93e699cfacf982432b7146f6f438a304d31b
---
Blocked at signed code checkpoint `0173551016f4ce41988edcc8a1d525ee52d917e9`. No landing, push, runtime approval or task completion is claimed. This report supersedes [[reports/2026-11-06-dream-mcp-verified-snapshot-landed]] for current status; that historical checkpoint and every failed run remain preserved. The current task amendment is etag `082a7c602aa6e9d27eb8458ff008ca17abd6bc417349d6b71bd682dd12f20c9c`.

All paths below use writing tree `/Users/feral/mydev/bboxy450-harness-dream-read-q2`. `R` means its `target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3` directory. `R/final-verification-manifest-v2.json`, SHA256 `d84abbb14ceadaf7fc3b4f18b117e0998118fadb0b873bbafa29ad8e968a9e4e`, records exact commands, heads, real exits, before/after source hashes, log hashes and all25 signed commits. Tracked source is clean. Fifteen untracked observer JSON files remain; eleven other matching observer records are tracked. The ten inherited untracked records and five records produced by the current sink tests are preserved.

The five new signed commits are b1171237 (preparation retention), 9a0945c8 (initial quiescence, diagnostics and models), f5dca2b9/364349ae (authorized test deferrals), and 01735510 (Entry observer and failed-readiness custody). All25 commits above local origin/main16d0cdeb7f1dd3bdb4bc31cd13e108a1e4e1f97b returned Git signature status G with Boxy identity. Earlier code and its reports remain ancestry, including the live-quarantine/closed-cost C checkpoint30f27ccc and spent-preparation correctionf4d3c4de.

The corrected frozen builder INPUT is complete preparation, without real qualification. C source remains316739B/SHA256 `140e808b40600086bbea9184057271e66c0655de0ece3650edec3c3fb3ec4c7c`, signed source commit `30f27ccc6f9021ae884e1406c9f415898d6a7eb8`. The immutable268744B executable remains `76f35824d7c2ca6fa336611d59333e01abde429eb1476b0bb5cbd4931c6eb56c`. Preparation implementation is signed `b1171237cc73fc312cec6324d8ed3766705d8c2f`; source_commit and preparation_commit are deliberately distinct.

`make prepare-dream-read-c-probe TARGET_DIR=target/dream-read-c-audit/corrected-signed-preparation DREAM_READ_C_SOURCE_COMMIT=30f27ccc6f9021ae884e1406c9f415898d6a7eb8` exited0. The retained511507B receipt is:
`target/dream-read-c-audit/corrected-signed-preparation/dream-read-c-preparation/compile-7ov2n68v/preparation-a4998fb7168936dffdf363e03e81c242c8ffcbe87c9d6e7279a9713fc4439458.json`,
SHA256 `a4998fb7168936dffdf363e03e81c242c8ffcbe87c9d6e7279a9713fc4439458`.
Actual compiler source/output/log paths remain present, owner directories0700, retained files0400/0500 and single-link. All352 declared output identities, lengths, modes and hashes were independently checked. The323-entry lexical header closure retains pthread.h's17B symlink target `pthread/pthread.h` and its resolved28093B header with an explicit relationship; it does not require two fictitious clang dependency keys. The first ad hoc check wrongly required two keys and exited1; the corrected relationship check exited0. `R/corrected-preparation-cross-binding-check.json` SHA256 `17648faba9d06dc29090db59a66ef59a7c651f371e00720ba7b45f4e308d524b` records this distinction. It was moved out of the preparation root, preserving its inode, so the retained tuple has no extra unmanifested validation file.

Compiler remains Apple clang21.0.0/clang-2100.3.34.2, executable SHA256 `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`; strict optimized flags and exact SDK27.0/compiler/linker/header closure are in that receipt. The proposal read-set digest remains `28998b1acd4468df4d5b6ce908567451c31c5c0d5d0063fd9760f82c820c188b`, an exact30f source snapshot, not a current compiled-product claim. Signed shared schema/vectors remain b26d7cd3, SHA256 d70948ec375dae099ea1dac0b92dfb30ff87d7fcac94c98f42caa1c0ba2da8ec / bff32c1afb483980952a6761918d4005c9f1e12b66f3d7aaaa88f1b9afe09740. These large compiler/corpus artifacts stay offline; no128KiB prepared-evidence allowance was enlarged.

The corrected receipt was delivered to Oracle01M6RZRGC3194VVFBC4M7XW4MF and builder01M6S9GNN1HB7PW20DSH1SY8BG in messages01M6SEM8544SE80H4X1AAWXTJY and01M6SEM8600FRSQ8H79NBY5C3Y. Receipts showed native-queued, with model consumption explicitly unverified. The retired76d579c2 preparation receipt is not substituted back. No real private candidate was run by this seat; no private result was promoted to runtime approval.

`src/dream/c1.rs:1634,2523,2584` fixes a concrete creation-phase asymmetry: an exact initial SSTOP4/one-thread/one-running kernel tail was already pending in the first sample but fatal in the mandatory second observation. Only before creation settlement/ready/running/grants/STOP, with original identity/path and prior plus fresh RI zero user CPU, it now remains pending. It grants no ACK/CONT and requires a fresh complete sample under the original25ms creation bound and full measured cost. Runtime, status/thread and user-growth negatives still refuse. Fixed-size paired TASK/RI diagnostics in c1.rs/read.rs preserve raw clock/counter fields without changing endpoints or authority.

The real START_SUSPENDED causal control exited0: one injected second snapshot, pending_count1, full_samples2, final4/1/0, user0, no ready/running/grant/STOP before admission, actual original reap. `R/initial-continuity-pending-a5t9u_hi/native.log` SHA256 `2f7a1328bfc817745d8700595dd2fcfcdd6404874977a5d97a4b3c4ddac5426c`. Four new finite models cover initial/second raw observations, bounded fresh generation, owner/pin/deadline, paid known cost and runtime misuse. Good plus AcceptPending/RuntimePending/SkipFresh controls matched pinned TLC:464states/472labelled edges, Java exits0/12/12/12. Receipt `R/initial-quiescence-parity-mz1zsdao/terminal-parity-evidence.json`, SHA256 `6b4ba1795d56fc67d3eb51bd5250b5ff3a51d434361be13194f43495c5c6dd79`. Corpus is438models/91contracts/one crate. This does not establish the historical guard92 kernel cause or arbitrary native correctness.

`tests/support/seat_socket.rs` now retains bounded stdout/stderr, stage/subscriber/durable-event evidence, private failure HOME and actual owned-child settlement. Failure readers share an absolute250ms wait, joining finished handles and honestly recording detached readers/EOF. Normal success requires complete captured stdout. The original2s assertions remain. Abandon, GUI lifecycle and held-reader controls each exited0 once. Actual PIDs46070 and78246 reaped; the latter returned while its controlled peer remained open, then observed real EOF after release. Current-source C1 integration later passed39/39, replay14/14 plus its two existing child entrypoint ignores (parents actually invoke them), and seat_process17/17 including GUI/headless ABA/resume and both evidence controls. The original GUI timeout remains preserved, without a claimed historical cause. Current retained control custody manifest `R/gui-retained-current-364/custody.json` SHA256 `654a8f11f5e0790332ec05b21ed9e865a39d32dbaadf25e54afa06cb239ed02c`.

Seven measured load-only witnesses retain every assertion and the exact authorized QoS ignore text: Before and Cleanup descendant arms, public-key transport, current-group2s wall completion, backpressured stdin, owned stdin, and zero-burn preparation cancel. Attach and both real2ms destructor controls remain active. No production debt floor, cost, clock, window or deadline changed. Classification receipts are `R/load-sensitive-helper-classification-enh60qow/classification.json` SHA256 `66352d228f9fb35b1ebbcee0479c00b9a3fd292b78d0c8d0b864aa5ff053f480` and `R/spent-cancel-load-classification-8dvh4eet/classification.json` SHA256 `538a2ffa2c33f8fdb6665c5e765e4ed6d14b93753e9fc7f3746f5a6e04a36215`. Cleanup's hook did execute during resource-refused retirement, but its required descendant-publication precondition was not established. No historical A>W failure was declared fixed by these deferrals.

`tests/dream_read.rs:1788+` repairs the actual upstream Entry-prelude compatibility gap. It accepts exactly one bounded canonical Entry record then the original exact waiting record, validates ordered numeric/status/e<=m<=observation fields, preserves raw capture and keeps the paid marker span and absolute250ms deadline. Sixteen real owned-pipe cases plus fragmented reads passed1/1; `R/read-entry-marker-pipe-control.log` SHA256 `c90c4bbf0047264141c9c9901e16ba7b345c3b5c4eedc180569910e2a62f4575`. Six affected worker tests advanced past the former marker failure:5passed, no_candidates failed later at Hello; `R/read-entry-marker-affected-six.log` SHA256 `aebcb2880b64a26eae91dec2a98c1ac29432ea07d4c7d0db817d6a4cacfa4468`.

The same fixture now records post-poll time/PID/run/revents/status and actual EOF2s→TERM2s→Child KILL/wait outcomes, preserves failed HOME, and bounds stderr64KiB/receipt8KiB. Its first compile failed E0689 before any child; explicit usize fixed it. The first real control reaped PID73365 in EOF phase but failed its privacy assertion because default HOME was0755. Explicit0700 builders correct that defect. The corrected control exited0, reaped PID80512/exit0 without TERM/KILL and retained a0700 HOME/0600 files. `R/read-hello-failure-owned-receipt-private-control.log` SHA256 `4085032738d2edc948d617634880051fd1b5eff7aba1077ca44b2113c4b21630`; retained799B receipt `R/retained-hello-receipt/worker-failure-80512.json` SHA256 `a46c3437751b966cd1a1447a5a5610e7bb4acc83627bbb0fb5fe364c09325bc8`. These observations do not certify helper settlement or CPU cost. No no_candidates retry, timing ignore or deadline extension followed.

Actual command outcomes, with full argv/head in the manifest:

| Command/stage | Exit and evidence |
|---|---|
| Original exact task chain, at9a0945c8 |2; `current-verify-after-initial-quiescence.log` SHA256 `58ff94c6146c8370877ad6b6a35cb11543bd3639c189b748a2de0e43c73a6c71`. Bootstrap4/4, C1lib34/34, fingerprint4/4; ACP7/8 failed before later stages. |
| Separate canonical MCP after initial correction |2; `missing-mcp-after-initial-quiescence.log` SHA256 `2a440f2ef4c1ecd24349655960a1373e00332da20c561007c112309a47077fe2`; helper7pass/3fail/3ignored, supplying the next load classifications. |
| Canonical MCP atf5dca2b9 |2; `current-mcp-after-load-classification.log` SHA256 `de0c111cb30eda70619c39d5a827d4c60f64007b49bec4aafd89d17e8873e763`; helper6pass/2fail/6ignored. Foreground A>W stayed active; zero-burn cancel later split. |
| Missing C1 integration at364349ae |0; `missing-current-c1-integration.log` SHA256 `2f13b85eb3ee5750015b825891967da5741a3d2c58c7f465e4d638847f63e0e6`;39 C1,14 replay,17 seat tests pass as detailed above. |
| Missing MCP downstream at364349ae |101; `missing-current-mcp-downstream-corrected-plan.log` SHA256 `2a2da9e8ac9ac729b5e0270c65b3a1fe791c10f455af04d3968e34537a123696`. Helper accounting18/18, audit7/7, Recall5/5, seal7pass/1existing ignore, sinks10/10, trust8/8; dream_read3pass/12fail. Six marker failures were subsequently addressed; normal capability controls also refused before qualification. |
| Full current formal parity at01735510 |2; `final-current-formal-438.log` SHA256 `513b61e7bdd9d88d0c2f2ac2b71c849009829510a36b2ce05d5e6e2614c7b96a`. Native438 models and30+58+1 native/nonvacuity/projection tests passed. TLC completed37 model verdicts, then failed the existing C1 ABA control safety/liveness mismatch. Full current parity is NOT green. |
| Current embedded build |0; `final-current-embedded.log` SHA256 `7df84a5018a703bf35c439e91db0f4883cec78eceb017d804c879e6433fe9260`; embedded clippy and default/ACP-standalone checks passed. |
| Current formal receipt check |2; `final-formal-receipt-status.log` SHA256 `b463d94f96a81dd1b76c1d2d6bd424e9d168d8dedfc7c1a8f83f1995265a1a2c`; old target receipt namesf4d3c4de and cannot stand in for this source. |
| Current read-worker preparation |0; nine unmodified preparation commands extracted from the signed Make recipe; `final-current-read-profile-preparation.log` SHA256 `a10b84a72e0ba57f3c2c1bae194ec652ef8e5545a3d1fda90b70988106a61f36`. Worker3c745b019f37cc235441fea54819747a93acd3cdb92f968dd65a9911c02131d1, source domaine163109e0d4ab4f4492bcee17de7923a6030ae389dc468d6e3d61eb984926929, head01735510. Bootstrap artifact approval is separate from proof-tool approval. |

The initial downstream plan assertion miscounted nine recipe commands as ten and exited1 before effects; its log remains. The corrected plan executed all nine with real exits. One read-only comparison used a mistyped archive locator and failed before writing evidence; the corrected exact locator comparison exited0. Neither is hidden as a successful test. Canonical synthetic codec324 vectors, private engine controls/mutants and outer parser27 cases passed in the MCP runs, labelled synthetic/source-known with real_candidate_created0/runtime_approved=false.

Every original acceptance class remains dispositioned:

| Acceptance class | Current disposition |
|---|---|
| Distinct prepared-read wire, immutable logical input bindings, strict schema/hash/currentness, no source replay/provider/trust promotion | Signed implementation preserved; wire3/3, worker6/6 and typed negative/sink/trust evidence above. No qualifying published lesson available is claimed. |
| Real qualifying signed publication, corrupted-proof negative, complete private tool/source/platform qualification and reviewed runtime profile | UNPROVEN. Builder valid/corrupt/component evidence and independent Root-approved provisioner literal are still prerequisites. Installed normal Git stays81. |
| Native original-effect/member/STOP/exec/debt/late-cost/retirement custody | Initial phase correction and strict native/accounting controls passed; foreground's unexplained A>W remains refused and unresolved. C source/live-quarantine/closed-cost evidence from the previous checkpoint stays preserved. |
| Heap/input/overflow/four-job/foreground/status/cancel/actual reap | Accounting18/18 and five repaired bootstrap/resource tests pass; genuine qualifying four-job/helper controls did not complete. Seven classified helper witnesses are deferred; no_candidates Hello remains unproven. |
| GUI/headless retained close and ABA/resume | Current changed fixture family17/17 plus C1/replay pass. ACP downstream tool-readiness gate separately fails and discarded its cause. |
| Formal native/TLC graphs and named safety/deadlock/liveness verdicts | Focused initial4 parity passes; full438 run fails an unchanged, dual-failure C1 ABA control. Current receipt check correctly rejects stale evidence. |
| Signed landing |25 signed commits, tracked clean; full task verify and all acceptance criteria have not passed, so make land was not run. |

Exact decisions needed:

1. Grant `tests/formal/dream_c1.rs` for the existing broken control repair. At:680, control9/kind2 gates EVERY publication verdict on epoch2. Cancellation plus native return at epoch0 cannot take a verdict; phase6 has no quarantine action and context changes are unfair. Preserve ordinary verdict/debt progress at all epochs and apply effect loss only under actual ABA. Keep good behavior, properties and fairness; do not change expected verdict, seed, or retry the same model.
   Current and archived ABA TLA/CFG/metadata/native states/edges/traces are byte-identical:13902states/36975labelled edges. Both native traces already contain CapturedEffectSurvivesAllVerdicts safety and CancelledOwnerSettlesOrHonestlyQuarantines liveness failures. Archived TLC reached Inv16 first; current periodic temporal checking found Live2 first. `R/formal-aba-control-readonly-comparison.json` SHA256 `3378f641b367b0d3b2ee86942183ea9c605624629cb53839a04d5bcf54fc25df`. TLA SHA256 b06ad4913d2ec0258f12db93f4bd600d2e95ace02e89998c6609e185e48a9b33. Java/TLC pins match. Historical archive9192fc89 remains historical, not a current receipt.
2. Grant `tests/acp_launch_context.rs` and `tests/fixtures/acp-agent/launch_parity.py` for bounded RPC-stage/error/receipt retention and a deterministic startup-order control. `bboxy_python:null` at ACP assertion:285 does not identify the cause: launch_parity.py:67–90 collapses RPC/parse/pipe/orientation failures to None, discards stderr, and the fixture removes journals during unwind. Relevant production modules are unchanged from origin/main. A context-publication readiness race is a source hypothesis, not this run's proven cause. No dependency install or Python product correction is justified by the retained log.
3. Disposition foreground native attribution under65489: W120000ns, A139625ns, C317341ns/Q456966ns. RI first-ACK read occurs275000ns after the positive TASK endpoint; baseline RI precedes preCONT by37414000ns. Different query cutoffs are proved; attribution of the19625ns excess to late kernel work is not. Existing source refusal remains correct. No clamp, endpoint extension or unapproved timing ignore was applied.
4. Disposition the remaining Hello witness: pre-poll remaining44,089,000ns, poll0/errnoNone/child statusNone, valid Entry+waiting, no paid_hello. Original post-poll clock and numeric reap receipt were absent; the new collector/control fixes that evidence gap prospectively. Original cause remains unproven. Root must also complete the independent builder qualification/profile prerequisite; no private preparation success supplies runtime authority.

Disk work stayed in this tree. Two lsof checks preceded each unlink plan and returned1 with empty stdout/stderr (no open debug files). The first pass pruned92 cold compiler files/1,232,195,584 allocated bytes; the post-embedded pass pruned1035 completed debug compiler-cache files/1,438,162,944 bytes. Total2,670,358,528 allocated bytes unlinked; zero directories, executables, immutable evidence, receipts, formal tools or peer paths deleted. Each selected inode/stat/SHA and deletion is journaled under `R/disk-prune-20261106` and `R/disk-prune-completed-debug-20261106`; plan SHA256 86240e0c913789798ada9b96105684f666c546854dea8d2fb38a393c7d2409fc / ac53c1b984a493c742789b4df95e0f008268f42587468848dac844e0908c9421. Machine free space changed with concurrent activity, so reclaimed cache bytes are not presented as a net free-space increase. All18GiB predecessor recovery/formal evidence, the historical10GiB archive and current formal tools/outputs remain. Both inherited/current pycache copies were retained by same-inode moves with custody JSON rather than discarded.

No source, task scope, model expectation, public wire, process budget, prepared-metadata allocation, runtime trust or landing rule was silently widened.