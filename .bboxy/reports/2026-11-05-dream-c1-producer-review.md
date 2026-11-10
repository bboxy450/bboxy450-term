---
type: report
title: Dream C1 producer R1 ownership correction and current-source verification
author: seat:01M6NPMWMJ27PAN93N9BY4KY5J
created: 2026-11-05T16:48:28.414Z
task: '[[tasks/2026-11-05-dream-c1-producer]]'
supersedes: '[[reports/2026-11-05-dream-c1-producer]]'
commits:
- a3425ac0a08e7184dc0a9198ddfa71f94a15ad54
- 5eec82a33959b764b32cbd7692cac2482fd3e009
- f5838ec407175d465cd436078580490130f4c99b
- bbae15b54f8e830542349797736aaa90b24ebefd
- cc2cb8e1addb2cd286e16a2e71457f39ef8a6f1c
- 4bdafb676467f4847d42c1ace154e8fe31521eb0
- 47182320be6435c860bc2176c2ddda582029fa63
verify:
  command: make test-dream-c1 && make test-formal-parity && make check-embedded-build
  exit_code: 0
id: 01M6P9S77JM70VCJBFZ65S0J9J
updated: 2026-11-05T16:48:28.414Z
number: 93377961364973
created_by: seat:01M6NPMWMJ27PAN93N9BY4KY5J
updated_by: seat:01M6NPMWMJ27PAN93N9BY4KY5J
authorship:
  signer: seat:01M6NPMWMJ27PAN93N9BY4KY5J
  kind: seat
  public_key: pqCo3ucLB9vSpeIqPXO76THHs02041zooYmQAtkBV1k=
  process_generation: '1'
provenance:
  digest: sha256:6733265f330ba848a8d709cafa9e248479266511146d468ec73e2aa88f8ff045
  frontmatter_digest: sha256:ab879ef41d13470651ad3ef133d560f23f81aa22c380f825a0b839dbf819def8
  signer: seat:01M6NPMWMJ27PAN93N9BY4KY5J
  sig: MdXYfs+eVqFZ+g/8U2JGbeJoRnCJwxUT8sQU+PUzO5C9OQIQz2M/HWgEQ5KewGHqvfn4OnjeiOvSF/fQABD3CQ==
  log: .bboxy/log/2026-11-05.jsonl#139099
---
# Dream C1 producer review correction

Task: [[tasks/2026-11-05-dream-c1-producer]]. Supersedes [[reports/2026-11-05-dream-c1-producer]] (#93377961364711). Governing documents remain [[specs/dream-term-c1]] and [[interfaces/dream-term-c1]]. Executor: Dream Producer, seat01M6NPMWMJ27PAN93N9BY4KY5J.

R1 was a real ownership defect in the baseline4bdafb67: Budget::unit could return on an unavailable end clock or rejected CPU/heap sample before disposing a successful owned return through paced cleanup. The old report and immutable artifacts remain historical baseline evidence. This correction retains successful typed returns in an external stepped slot across every verdict and releases their credits only after real disposal. It does not certify the consumer's integrated gate or accept/land the task.

## Source and refresh

Worktree /Users/feral/mydev/bboxy450-harness-dream-c1, branch feat/dream-host-c1. Final source47182320be6435c860bc2176c2ddda582029fa63 has signature status G, author/committer Boxy <bboxy450@gmail.com>, signing key9B07E849480C68328CA20684844ECC0587DE7AB0. Its Report trailer points to this report. All six earlier commits remain signed G:

- a3425ac0a08e7184dc0a9198ddfa71f94a15ad54
- 5eec82a33959b764b32cbd7692cac2482fd3e009
- f5838ec407175d465cd436078580490130f4c99b
- bbae15b54f8e830542349797736aaa90b24ebefd
- cc2cb8e1addb2cd286e16a2e71457f39ef8a6f1c
- 4bdafb676467f4847d42c1ace154e8fe31521eb0

Operator refresh completed: GIT_TERMINAL_PROMPT=0 git -c credential.helper= -c core.askpass=/usr/bin/false fetch origin refs/heads/main:refs/remotes/origin/main exited0; git rebase origin/main exited0, already up to date. Harness origin/main remains a2ca8f393630081b6be583151293ae286dc2419e. The term repository's unrelated main was not used. No push, merge, install, publication, dependency/lockfile or IR schema/version change.

Exact new changed paths and mechanisms:

- src/dream/c1.rs:248/273/313 — track's reserved credit surrounds unit_clock; a successful Result<T> enters an external Paced<Option<Result<T>>> before the end-clock/CPU/heap/control verdict. Every error branch keeps that slot until stepped disposal finishes. Only acceptance takes ownership out.
- src/dream/c1.rs:341/392/421 — cleanup clocks fail honestly instead of reporting zero. Paced disposal has a finite1024-step/400us batch bound and >=9ms waits even if clocks fail. Drain implementations empty nested JSON, typed vectors/maps/optional fields, Unknown payloads, usage and manifest inventories before fixed leaf destruction. Preflight bounds JSON depth32/4096 members/65536 values+keys; one JSON entry/scalar is removed per step. JSON object keys use a fixed512-byte or256KiB stack copy, with no cloned heap buffer; the non-inlined removal frame sits inside the existing2MiB compute stack. Library deallocation/syscall/timing coverage remains measured.
- src/dream/c1.rs:1155/1403 — selected large/tool envelopes stay in an external stepped owner through observation and all errors; no whole owned envelope is taken and implicitly dropped on a failed unit. Manifest validation follows construction of its guarded owner. Empty tiny-record slots avoid extra cleanup sleeps. D0 semantics and close ownership are unchanged.
- src/dream/c1.rs:1565 — the existing bounded stderr metrics retain their labels and add cleanup_cpu_ns and max_cleanup_unit_cpu_ns. No worker/domain wire change.
- src/dream/c1.rs:2084/2141 and tests/dream_c1.rs:1354 — owned Drop-probe CPU/clock/cancel controls, unavailable-start-clock refusal,262112-byte nested parsed-envelope rejection/cancel API witnesses, and actual private worker rejection/Done/exit witness.
- tests/formal/dream_c1.rs:445; tests/formal/models.rs:6885; verification/{contracts,coverage,corpus}.toml — finite returned-owner contract with CPU/clock/cancel verdicts, paced pieces, credit retention and Done. Independent unpaced-drop, early-credit-release and early-Done broken controls.
- README.md and ATTRIBUTION.md — describe the actual guarded teardown and retain this project's origin/credit policy.

No other tracked source paths changed in this correction. The generated tracked .bboxy/runs/formal-native.json was preserved to target/dream-c1-review-47182320-formal-native.json then restored; private .bboxy state was not added to Git. Final git status --short is empty.

## Commands and actual exits

One build/test at a time, two Cargo jobs. The final shell exported CARGO_BUILD_JOBS=2 and ran the required conjunction sequentially under set -e, followed by licenses; the whole invocation exited0.

| Command | Exit | Retained evidence |
|---|---:|---|
| make test-dream-c1 && make test-formal-parity && make check-embedded-build |0|target/dream-c1-review-47182320-final.log|
| make test-licenses |0|target/dream-c1-review-47182320-licenses.log|
| make check-formal-evidence |0|target/dream-c1-review-47182320-evidence-check.log|
| cargo build --offline --locked --release --jobs 2 --bin bboxy450-harness |0|target/dream-c1-review-47182320-production-build.log|
| frozen ordinary-release absent-HOME/manifest Hello handshake, then EOF |0|target/dream-c1-production-47182320/receipt.json|

Focused development witnesses also passed: target/dream-c1-r1-focused-complete.log (five Budget API tests) and target/dream-c1-r1-process-complete.log (actual near-limit process). Final canonical gate reran them against signed47182320, including the deterministic post-parse Cancel verdict. Initial target/dream-c1-r1-process.log failed its zero-overruns assertion on the deliberately dense rejected parse: the final negative witness preserves the actual CPU overrun, requires ResourceLimit when it occurs, and separately asserts bounded cleanup. It never calls that rejected unit smooth or completed. A later normal unsupported-version path at the same input also passed (target/dream-c1-r1-process-complete.log:913834ns maximum unit,1202ms actual process completion). No production threshold was increased.

The first standalone artifact-probe attempt closed stdin immediately and read empty output, so its JSON assertion failed before creating a receipt. The corrected probe keeps stdin open through Hello, then closes it and proves successful exit; only this actual passing handshake is recorded in the artifact receipt.

Final results:11 C1 unit controls;4 fingerprint tests;7 ACP launch-context;21 C1 wire/process/data;14 replay tests plus two ignored child entrypoints actually invoked by six parent process scenarios;15 ordinary seat-process;7 retained-Host;1 native environment helper;3 MCP source/provenance and18 MCP integration/privacy/cleanup tests. All passed unattended. Formal Rust12 export/model tests,40 nonvacuity controls,1 reactor-projection and8 formal-driver tests passed. Embedded clippy -D warnings, default and acp/standalone library+binary checks passed. License headers and10 license controls passed.

## R1 measurements and causal evidence

DropProbe retains eight owned pieces, each doing >=120us actual CPU before release. Its final destructor asserts that every piece was stepped and the1024-byte credit is still reserved. Tests force actual >2ms compute, unavailable end clock, and post-work Cancel; each returns only after the probe destructor ran, >=two paced waits elapsed, cleanup CPU includes the pieces, cleanup maximum <=1ms and live credits return0. A missing start clock never invokes work. This catches the original successful-return implicit destructor rather than merely returning ().

The near-limit API witness parses an actual262112-byte Envelope with4096 nested object/array entries and a large raw Arc. Separate injected verdict clocks exercise CPU refusal, unavailable end clock and exact Cancelled after successful parse, independent of parse speed. Cleanup uses the actual thread clock, zero live credits and multiple9ms waits; parser timing in those injected-clock cases is not a production performance claim. The production process witness supplies the same kind of near-limit owned header through real immutable files/pipes and uses actual clocks.

Final actual process witness:262144 input bytes read; dense parse1562833ns, overruns1, failed ResourceLimit; total cleanup3584792ns, maximum cleanup unit348041ns; Done and real process exit1373ms after Start, within the unchanged2s witness. Native heap live390144/high-water4342064 bytes; declared credit peak74112840/cumulative74113599; RSS12615680. All declared live credits are0 before metrics/settlement. No result/page or source mutation was produced. This is a causal rejected-unit control, not a successful-unit performance exception.

Representative final ordinary producer samples (all live declared credits0 and overruns0):

| Witness | Input bytes | Native heap live / peak | Declared peak / cumulative credits | RSS peak | Max CPU unit ns | Max cleanup unit ns |
|---|---:|---:|---:|---:|---:|---:|
| Large selected IR |2503989|390144 /862160|33386773 /2306379965|10059776|715958|88000|
| Four child controls, one large job |2644729|390144 /969680|17559183 /629512505|10289152|735458|71125|
| Real Store +typed JSON blob |7359851|390144 /875536|13125545 /326320046|9895936|786667|44500|
| Retained-state Cancel |393216|390080 /728368|19390838 /756399183|8962048|345000|55541|
| Dense child credit refusal |83053|390080 /654288|524816 /525574|7929856|396792|31250|

The real67387767-byte IR control was refused before source read (bytes0), heap peak391872, declared peak262162, RSS7634944, max CPU77750ns. Startup native heap390080; child78MiB work credits +16MiB tested helper/startup headroom +2MiB protected control remains96MiB. Stack2097152 bytes is separate. Metrics JSON extraction: target/dream-c1-review-47182320-metrics.json. Cumulative CREDITS are not native allocation totals; native cumulative allocation remains unmeasured. Heap/RSS are distinct observations, not an all-times allocator/native proof. Combined parent/child128MiB credit allocation and parent32MiB attribution remain consumer-owned integrated evidence.

## Acceptance matrix and retained formal evidence

1. Canonical wire/schema/fixtures and exact provisioning: final gate reran generated schema equality, independent Python canonical hashes and missing/unknown/duplicate/oversize/Unicode negatives. Sole authority remains src/dream/c1_wire.rs, schema/dream-c1-v1.schema.json and tests/fixtures/dream-c1. Schema SHA2565e3d8a602eeaa1da5cb97d0f6af0b4f331b216a02255d3dff990be84c30d6a30 and lock SHA25631113dd0968ace9daf43b1e610b53a3fc9b586e7ff9a707136ec25ab5e9f2c31 are unchanged. Actual frozen Hello precedes absent manifest/job access.
2. Selected closed IR/blob/raw-byte/nonfork law validation and bounded D0 semantics: source unchanged outside teardown; all21 C1 and14 replay tests pass, including actual Store multi-MiB/blob and real Store::fork->close unsupported header/malformed-parent controls. Sole TermC1 close owner still suppresses legacy automatic D0 and scanner work while preserving durable SessionClosed, tested in actual engine/ACP/import paths. No global scan, cache/KB/model/profile/inference effect.
3. Dedicated low-priority compute/control/limits/immutability/settlement: final process gates pass foreground pause, Cancel/EOF/deadline, independent four-child controls, source changes, no-clobber output and actual join. R1 now adds successful-return ownership across every rejection path and measured, paced typed disposal through credit release. Unknown foreground still defers work; real closure remains required.
4. Formal ownership and controls: full native/Java reachable-state/transition/verdict parity for231 models/71 contracts/1 crate. New good returned-owner17 states/16 edges; unpaced-drop control26/25, early-credit29/28, early-Done26/25, each expected safety failure. Source/current tool hashes verified by make check-formal-evidence. Archive verification/evidence/a9099420f7d2414001cdd15b9b2867ec5c3c9c18a1284d4dc2f42e4995d10306/evidence.json binds source47182320be6435c860bc2176c2ddda582029fa63, dirty=false. Native run retained at target/dream-c1-review-47182320-formal-native.json. TLC1.7.4 SHA256936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88; Java21.0.13+9 archive SHA256225bab0a09491ef06835488cd942f5d2befa9a0410a2484f0c8bbcf7506ec09a. Unknown/incomplete/disagreement fails. Finite models prove declared ownership/credit ordering, not arbitrary Rust or native timing/heap.
5. Canonical focused gate and affected coverage: Makefile:282 unchanged, full prescribed target and existing managed generation7/op07/permission/privacy tests rerun and pass on current signed source; no full unrelated suite.
6. Signed reviewable handoff: exact paths/mechanisms, actual exits, measured failures, exclusions, immutable artifact and clean tracked state recorded here. Rook still verifies and accepts.

## Corrected immutable handoff and exclusions

Ordinary production worker: /Users/feral/mydev/bboxy450-harness-dream-c1/target/dream-c1-production-47182320/bboxy450-harness, SHA2562c8469829bc94171048677c590b8427b48076512cb04178ad1b2bdcd711beabd. receipt.json beside it binds signed source47182320, ordinary release/default acp-agent, NO test-fixtures or extra features, rustc1.99.0 b940084d7eb6a299eb4bfeb8e34901bc051e7ac4, cargo1.99.0, aarch64-apple-darwin, actual build0 and cleared LANG/private TMPDIR/no HOME/manifest Hello0. Binary0555, receipt0444, directory0700. Local artifact claim01M6P9M1WQKG5XQT5PFSGZJJN3. Essential consumer handoff01M6P9M31DMNZJCJZ24TMKDS2V was durably stored and native-queued (harness Ack duplicate=false); model consumption remains unverified. Receipt operation01M6P9R2MVRBAHA4PA3D3H6PQP. Delivery is distinct from acceptance.

Preserved old immutable4bd production worker and separate bbae feature-seat artifact, with their exact historical hashes/build labels from the superseded report. Preserved copy-only real Store /private/tmp/dream-c1-producer-store-f5838ec4 and real fork /private/tmp/dream-c1-producer-fork-cc2cb8e1 bundles. No copied second fixture authority and no raw source data in Git.

C1 remains an explicitly nonfork, non-native-import subset; those headers reject unsupported_ir before replay. Standalone/manual Legacy behavior and default product lifecycle are unchanged. Private A->B retained Host remains feature-gated fixture only. Current-source native frames, parent attributable heap, combined synchronized memory, actor authority/revocation/ABA and shared structured MCP hints require the consumer/shared-host integrated gate; producer-only evidence does not certify those results. No operator input, physical action, keychain/custody probe, network inference or credentials were needed. No required producer check remains unrun. This append-only report is created through the selected term knowledge provider; the task explicitly does not require a KB Git commit.
