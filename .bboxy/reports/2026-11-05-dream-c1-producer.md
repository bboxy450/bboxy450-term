---
type: report
title: Dream C1 producer implementation and verified source handoff
task: '[[tasks/2026-11-05-dream-c1-producer]]'
author: seat:01M6NPMWMJ27PAN93N9BY4KY5J
created: 2026-11-05T16:07:17.348Z
commits:
- a3425ac0a08e7184dc0a9198ddfa71f94a15ad54
- 5eec82a33959b764b32cbd7692cac2482fd3e009
- f5838ec407175d465cd436078580490130f4c99b
- bbae15b54f8e830542349797736aaa90b24ebefd
- cc2cb8e1addb2cd286e16a2e71457f39ef8a6f1c
- 4bdafb676467f4847d42c1ace154e8fe31521eb0
verify:
  command: make test-dream-c1 && make test-formal-parity && make check-embedded-build
  exit_code: 0
id: 01M6P7DTE62RHXGSBRVM6WT3RA
updated: 2026-11-05T16:07:17.348Z
number: 93377961364711
created_by: seat:01M6NPMWMJ27PAN93N9BY4KY5J
updated_by: seat:01M6NPMWMJ27PAN93N9BY4KY5J
authorship:
  signer: seat:01M6NPMWMJ27PAN93N9BY4KY5J
  kind: seat
  public_key: pqCo3ucLB9vSpeIqPXO76THHs02041zooYmQAtkBV1k=
  process_generation: '1'
provenance:
  digest: sha256:7a9cff49ebb2d6474278b64a5622e62a871e26f2e8ff2065b2d71b37286aa53e
  frontmatter_digest: sha256:7730a1f697545db0c964b9f4c6a332a06aa75501c95130c2d187c61aee32f896
  signer: seat:01M6NPMWMJ27PAN93N9BY4KY5J
  sig: kc+/1NSnnYF7rSPoFaiVyf3hMT0wIgg1BGFjho/S6YagYwsvnhwsypj8QcZVMZxNHnLiCNIJhkfAfytHvSZwDw==
  log: .bboxy/log/2026-11-05.jsonl#137548
---
# Dream C1 producer handoff

Task: [[tasks/2026-11-05-dream-c1-producer]]. Executor: Dream Producer. Source worktree: /Users/feral/mydev/bboxy450-harness-dream-c1, branch feat/dream-host-c1, base a2ca8f393630081b6be583151293ae286dc2419e. Source at verification: 4bdafb676467f4847d42c1ace154e8fe31521eb0, tracked tree clean.

Implemented the producer and the amended private managed witness under [[specs/dream-term-c1]] and [[interfaces/dream-term-c1]]. Required producer command and licenses exited0 unattended. This report hands source/evidence to Rook for review; it does not accept the task or claim the consumer's complete integrated native/authority/parent-memory gate.

## Signed source

All six commits have git signature status G and Report trailers pointing here:

- a3425ac0a08e7184dc0a9198ddfa71f94a15ad54 — sole canonical wire/schema/hash corpus; essential boundary milestone sent to Rook.
- 5eec82a33959b764b32cbd7692cac2482fd3e009 — selected producer, ownership, source/process/formal gates.
- f5838ec407175d465cd436078580490130f4c99b — enforced declared storage credits, minimal startup and paced teardown.
- bbae15b54f8e830542349797736aaa90b24ebefd — staged record CPU, managed retained-Host witness and exact public-MCP metadata classification.
- cc2cb8e1addb2cd286e16a2e71457f39ef8a6f1c — measured batching of tiny records within unchanged CPU/deadline bounds.
- 4bdafb676467f4847d42c1ace154e8fe31521eb0 — bounded incremental raw-line hashes, shared pinned D0 origin, paced large-record disposal and actual fork export.

No push, merge, install, publication, dependency/lockfile change or canonical IR version/schema change. ATTRIBUTION.md and source/runtime-fixture origin notices retain the project policy.

## Commands and actual results

The shell exported CARGO_BUILD_JOBS=2 before the final command. One build/test invocation ran at a time.

| Command | Actual exit | Evidence outside source Git |
|---|---:|---|
| make test-dream-c1 && make test-formal-parity && make check-embedded-build | 0 | /Users/feral/mydev/bboxy450-harness-dream-c1/target/dream-c1-final-4bdafb67.log |
| make test-licenses | 0 | target/dream-c1-final-4bdafb67-licenses.log |
| cargo build --offline --locked --release --jobs 2 --bin bboxy450-harness | 0 | target/dream-c1-production-worker-build.log |
| actual ordinary-release cleared-env Hello probe, absent HOME/manifest/job directory | 0 | immutable worker receipt described below; Hello schema hash matched and directory remained absent |
| DREAM_C1_EXPORT_BUNDLE=/private/tmp/dream-c1-producer-store-f5838ec4 cargo test --offline --locked --release --jobs 2 --test dream_c1 real_store_multi_mib_ir_and_typed_blob_bundle -- --nocapture | 0 | target/dream-c1-store-export.log |
| DREAM_C1_EXPORT_FORK_BUNDLE=/private/tmp/dream-c1-producer-fork-cc2cb8e1 cargo test --offline --locked --release --jobs 2 --test dream_c1 actual_store_fork_is_explicitly_unsupported_and_parent_rows_never_pass -- --nocapture | 0 | target/dream-c1-fork-export.log |

Final focused results:9 C1 unit controls;4 fingerprint equivalence/vector tests;7 ACP launch-context tests;20 C1 wire/process/data tests;14 replay tests plus two deliberately ignored child entry points actually invoked by parent process witnesses;15 ordinary seat-process tests;7 named retained-Host tests;1 native-locale helper test;3 MCP provenance/classification tests;18 MCP integration/credential/cleanup tests. All passed. Formal Rust12 model/export tests,40 nonvacuity controls and1 reactor-projection test passed; formal driver8 tests passed. Embedded clippy -D warnings, default library and acp/standalone library+binary checks passed. License headers and10 license controls passed. Existing feature-test support emits two dead_code warnings; no warnings were waived in the required clippy gate.

## Acceptance evidence and changed mechanisms

1. **Canonical authority/provisioning.** src/dream/c1_wire.rs:493/512/575 owns typed closed DTOs, scalar/runtime validation, recursive canonical sorting, generated schemars authority and bounded runtime canonical output. schema/dream-c1-v1.schema.json and tests/fixtures/dream-c1/{valid,invalid,vectors,lock.json} are the only shared fixture/schema authority. tests/dream_c1.rs verifies generated equality, independent Python stdlib hashes, all-DTO missing/unknown/duplicate controls, oversize/unsafe-number/enum/hex/ULID negatives and Unicode byte limits. Schema SHA2565e3d8a602eeaa1da5cb97d0f6af0b4f331b216a02255d3dff990be84c30d6a30; corpus lock SHA25631113dd0968ace9daf43b1e610b53a3fc9b586e7ff9a707136ec25ab5e9f2c31. JSON Schema character lengths do not replace runtime UTF-8 byte and cross-field/cardinality validation. src/main.rs:11 dispatches the strict five-argument worker before bbt_rt; c1.rs:55 borrows bounded argv and emits Hello before any manifest/source access. Matching Start alone admits manifest work. No HOME/profile/provider/seat discovery in that entry.

2. **Exact selected closed data/D0 and sole close owner.** c1.rs:646 reads only fixed private descriptor-relative nofollow inputs, validates uid/mode/nlink/stable fd-path metadata, ≤64KiB IO and original line hashes, exact source/close/blob pins and complete inventories. inspect.rs:47/403 enforces the documented nonfork subset and rejects unsupported semantics. d0.rs:307/312/876 factors the existing observer and selected inspect_closed API; C1 supplies exact streamed lowercase raw hashes while public/legacy observe retains its original hashing. There is no replay(paths), discovery, task/KB join, shared cache write or inference fallback. Canonical bounded episode/cluster pages are no-clobber exclusive publications, with result.json last. dream/mod.rs:137 is the single DreamOwner gate suppressing BOTH automatic legacy D0 and IR advice for TermC1; engine/mod.rs, backend/acp_agent.rs, gateway/host.rs and import/watch.rs preserve the durable close and propagate owner. headless/LaunchOptions glue, one GUI exhaustive literal and two ACP fixture literals preserve Legacy defaults. tests/dream_replay.rs proves exact D0 selected semantics and actual engine/ACP/watch close behavior with zero TermC1 hook counters. The managed close witness includes4194347-byte JSON blobs and200 foreground rounds: p99_us6078/7614/6881 (actual maxima7545/8006/10381), without starting the old whole-blob scanner.

3. **Isolation, pacing, controls, memory and settlement.** c1.rs:258 measures thread CPU per unit, enforces ≤1ms and ≥9ms rest, nice+10/utility compute and foreground/deadline checks. Small records ≤8KiB/≤128 values+keys share a measured unit; large records have separate framing/preflight/parse/observation/disposal, with raw SHA updates ≤64KiB. c1.rs:320 and Paced teardown account success/error/cancel destruction before actual JoinHandle settlement. c1.rs:1326/1419 bounds flat control framing, keeps main/control independent, rejects partial EOF/wrong nonce/duplicate/oversize, handles pipe pressure, and emits one Done only after compute exit/join. c1.rs:1858/1876/1984 enforces nonblocking atomic declared-work credits BEFORE controlled growth, retained-state/old+new overlap and transfer/drop; fallible bounded Vec/canonical builders; JSON depth32, members4096/container and total65536 values+keys before serde. Child share is78MiB declared work +16MiB tested helper/startup allowance +2MiB protected control, totaling96MiB. No borrowing, global allocator or SQLite hard-heap change. Runtime records actual native live/high-water heap, RSS, declared live/peak/cumulative credits separately; native cumulative allocation is explicitly unmeasured. Native/library overhead is measured implementation coverage, not a universal all-times allocation proof. tests/dream_c1.rs includes genuine >64MiB IR, four actual children, independent controls/kill/EOF/deadline, retained-state Cancel, dense child refusal, real Store/blob data, immutable results and source-replacement failures. C1 unit tests detect actual missing reserve, old+new refusal, cross-thread credit release, bad CPU and bad inline reactor controls. Inspect only durable terminal SessionClosed; idle/turn-end/native EOF do not qualify.

4. **Formal and causal controls.** tests/formal/dream_c1.rs, tests/formal/models.rs, verification/{corpus,coverage,contracts}.toml map inspection, worker, declared storage, private retained Host and public-metadata registration to source/API/process witnesses and causal controls (close/raw/clobber/legacy hooks, foreground/early Done/cancelled publish, reserve bypass, early B, over-scrub and blanket exemption). Current-source native/Java full reachable-state, edge and verdict parity held for227 variants/70 contracts/1 inventoried crate. Receipt: verification/evidence/930f78a4919fbe1a3e2a05e2263e45413f3718e740103230518cf4584f8469b6/evidence.json, source4bdafb676467f4847d42c1ace154e8fe31521eb0, dirty=false. Preserved native output: target/dream-c1-4bdafb67-formal-native.json. TLC1.7.4 SHA256936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88; Java21.0.13+9 archive SHA256225bab0a09491ef06835488cd942f5d2befa9a0410a2484f0c8bbcf7506ec09a. Tools were locally cached, with no network/global install. Unknown/incomplete/disagreement remains a failure. Models prove their finite declared abstractions, not arbitrary Rust/native timing/heap or full TLC completeness.

5. **Private real managed handoff and gate.** Makefile:282 runs the expanded unattended target. headless.rs/test-fixtures + tests/seat_process.rs implement decisions64606/64648/64677: one actual Host/PID/nonfork A→B after flushed durable A close, actual owned UDS Hello/input, normal final close/join, physical EOF disables transition, no default-build product lifecycle extension. Explicit private common-root profile/model/auto_compact:false, MCP and appended context survive. --yolo is preserved only when supplied; real safe writes in A/B succeed with it and are denied without it; invalid modes/paths/env still refuse. Actual generation7/op_ids ending07 have matching nonduplicate ACK plus exact durable queued/started/ended lifecycle in BOTH sessions. Profile credentials/unknown/mixed-root/symlink/mode inputs refuse; capability contents are never opened/logged by the fixture validator. mcp.rs:763/783 implements decision64668: ONLY host Launcher same-name exact references for validated public seat/generation/role avoid NEW substring-secret registration. Literal/command/alias/profile/ACP/unknown and independently registered secrets retain previous protections. Real MCP credential stderr/spill and capability/cleanup regressions all pass; no public labels grant authority.

6. **Reviewable signed handoff.** Exact changed source paths: src/main.rs; src/dream/{c1_wire,c1,inspect,d0,fingerprint,mod}.rs; src/frontends/{headless.rs,gui/bridge.rs}; src/{engine/mod.rs,gateway/host.rs,backend/acp_agent.rs,import/watch.rs,mcp.rs}. Tests: tests/{dream_c1,dream_replay,seat_process,acp_launch_context}.rs, tests/formal/{dream_c1,models}.rs and all tracked files in tests/fixtures/dream-c1. Artifacts/config/docs: schema/dream-c1-v1.schema.json, verification/{corpus,coverage,contracts}.toml, Makefile, README.md, ATTRIBUTION.md. tests/mcp_client.rs is authorized but unchanged; its18 existing tests are newly included in the focused gate. No other source paths changed. The only tracked generated .bboxy/runs/formal-native.json change was copied to retained evidence then restored; .bboxy private state was not committed.

## Measured final producer samples

All bytes below are actual diagnostic figures from the final command; all samples end with declared live0 and overruns0. Child startup native high-water390080 bytes. Control reserve2097152; helper reserve16777216; declared credit ceiling100663296; compute stack reservation2097152 is separate from heap. Full-child malloc_zone_statistics(NULL) size_in_use/max_size_in_use samples/high-water include minimal startup and linked/runtime native allocations; getrusage RSS is separate. Unsampled/native mappings/fragmentation do not become a formal bound.

| Actual witness | Input bytes read | Native heap live / high-water | Declared peak / cumulative credits | Peak RSS | Max unit ns |
|---|---:|---:|---:|---:|---:|
| Large selected IR40turns |2503989|390144 /862160|33386773 /2306379965|9879552|370667|
| Four child controls, one large job |2644729|390144 /969104|17559183 /629512505|10092544|632500|
| Real Store IR+typed JSON blob |7359851|390144 /875536|13125545 /326320046|9469952|372000|
| Retained-state Cancel |393216|390080 /728560|19390838 /756399183|8847360|303958|
| Dense declared child overflow |83053|390080 /654288|524816 /525574|7880704|154708|
| Actual67387767-byte IR rejected pre-read |0|390080 /390848|262162 /262162|7667712|53000|

Real failure/retry/denial worker: native heap739520 peak, RSS11075584; max unit442250ns; four fixed mask cold units177042/88625/104000/125500ns. Native cumulative allocated bytes are unmeasured; the cumulative CREDIT figures above are explicitly neither allocator totals nor concurrent live heap. Parent attributable≤32MiB and combined observed≤128MiB, ≥240 same-app frames, current public authority, account/root slots and integrated structured MCP hints are consumer/shared-host gates. No producer-only sample certifies them; max(parent)+max(child), if supplied, is only a conservative observed-peak bound. Their final integrated result is not claimed here.

## Immutable handoffs and coverage limits

Ordinary production worker: target/dream-c1-production-4bdafb67/bboxy450-harness, SHA25698504f509158461773c16aa8588964eb0a433f5fb5e2879e2bb012ac7d276c22. Build: ordinary release, default acp-agent, NO test-fixtures/extra features; receipt.json beside it. Exact cleared LANG/private TMPDIR env, no HOME/manifest, Hello exits0 and schema hash matches. Separate feature seat baseline target/dream-c1-fixture-bbae15b5/bboxy450-harness SHA25615664858bf26388116c86e159c1bc179f1bcec8b494312cfb13f663e216dc767 is explicitly a release/test-fixtures artifact at bbae15b5, never relabeled production. Consumer decision64684 uses its own optimized+assertions profile/native/seat artifact and records both hashes; producer Cargo.toml is unchanged.

Copy-only real Store source bundle: /private/tmp/dream-c1-producer-store-f5838ec4/sessions/01M6P07HR5HSQX4HKZHB93ADE8.jsonl (3165504 bytes, SHA25678e0e3bf737c045238d36653d58c2a9a8744729f8dc0c01b6ffe54a943397151); typed JSON blob4194347 bytes, SHA256dedd492b646fe01afc438938725a361d87f8eefa8bc0072f6095e45356378ec8. Copy-only valid real fork: /private/tmp/dream-c1-producer-fork-cc2cb8e1/sessions/01M6P5Q5AAXYS1K6T3YM9D8SKZ.jsonl (1191 bytes, SHA25685b30f555837c8ad3e241de2a55bc09a9a4fd41f8c3c639a822d2f7a69cec2aa, close_seq5, raw close hash9a98ef57d0ee181b1bc91bb3df89b861350f739d8225813163361ec4c83eb7df). manifest.json beside each export supplies exact raw pins; consumer constructs its current host-admitted Context. Source data were not rewritten or added to Git. Consumer received exact immutable binary/build/hash and both bundles.

C1 explicitly rejects SessionMeta.fork and native-import source headers as unsupported_ir under decisions64460; canonical fork support outside C1 and normal Legacy/manual behavior remain. The real Store fork and malformed parent controls preserve bytes and produce no result. This is a nonfork law subset, not a claim of arbitrary protocol equivalence. Production CLI close-A/new-B is excluded; only the feature-gated retained-Host transport witness supports that sequence. No source IR version change, global scan, KB/cache/inference effect, runtime credentials/Keychain/custody lookup, physical input or operator account was needed. Source commits used the configured signing key.

Material failed measurements were fixed, not skipped: original combined normalization1571084ns split into per-pattern/shared warm units with original-expression equivalence; combined large-record observation1286917ns was changed to bounded streamed raw hashing and separate disposal; excessive tiny-record sleeps reached the unchanged30s deadline, fixed by measured tiny-record batching (unchanged80-turn test12.36s). See target/dream-c1-process-fix.log, target/dream-c1-f5838ec4-two-jobs-failure.log, target/dream-c1-bbae15b5-deadline.log and target/dream-c1-final-cc2cb8e1.log. Private fixture baseline exposed CoreFoundation locale injection, umask permissions and real MCP generation-as-secret actor corruption; the final guarded real managed tests pass. No thresholds were raised and no causal controls were removed to obtain green.

The report is written through the selected term knowledge provider as authorized; no KB Git commit is required by this source assignment. Rook still reviews the source and performs acceptance/landing. Current-source formal evidence must be refreshed after any rebase/source change.
