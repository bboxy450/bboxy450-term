---
type: report
title: Fixed C1 native/debt producer interaction review at16d7406
task: '[[tasks/2026-11-05-dream-c1-native-fixed-review]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T19:53:45.939Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6PMCGGX8Q5GA3ZF4EZ0M61T
updated: 2026-11-05T19:53:45.939Z
number: 93377961365160
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:a5a15d9ade7960c0e85433cd4d59c37690c48cd24153cdcb96824348d8c0767b
  frontmatter_digest: sha256:7e8dc63ae825d0ff867f2380a6a15413a4e0567fbd8eaca14bc0e1952253a1bd
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: 9JkZnbr+npnsnmANKEciwv01ijT7T7vMR8LkeMuyHuqiLeGeRly1UrfaWXYE3XZhp0LIp/uBlVrTMk1Dzyu+AQ==
  log: .bboxy/log/2026-11-05.jsonl#141148
---
# Fixed C1 native/debt producer interaction review

Task: [[tasks/2026-11-05-dream-c1-native-fixed-review]]. Review cutoff **16d7406dbb62c939933391783666bd7c3d36c2c1**, producer change ffa09e52b8302ed74aaba4e4c7ede3bf653b7954, compared with accepted47182320be6435c860bc2176c2ddda582029fa63. Read-only worktree /Users/feral/mydev/bboxy450-harness-dream-native-c1. No code/spec edits, builds, tests/replays, delegation, acceptance or landing. commits:[].

**Disposition:** parked-owner, exact native boundaries, debt/effect custody and immutable capture cutoff are substantially implemented. The fixed revision has the known reversed-cleanup-clock defect plus two additional actionable evidence defects below. This is not component acceptance or consumer/native-frame/public-authority/hints certification.

## Findings at the fixed cutoff

| Finding / classification | Exact mechanism / bounded correction witness |
|---|---|
| F1 — actual defect, already reported by Rook | src/dream/c1.rs:755-766 cleanup_unit uses end.saturating_sub(at), so finite reversed clocks become0 while interval():134-139 rejects them. Known author correction remains required; no duplicate request/approval loop sent. Inject finite end<start into cleanup accounting; require sticky unknown cost/resource refusal with ownership retained, not cost_known=true. |
| F2 — actual metric/failure-path defect plus weak positive assertion | c1.rs:2332-2335 stores interval(at,end) into owner_cpu without propagating MAX to clock_failed; :2569 can retain a successful result and :327-329 can emit cost_known=true. tests/dream_c1.rs:1281-1282 accepts any owner_thread_cpu_ns>0, including MAX. Terminal getrusage failure similarly writes MAX at c1.rs:319-326, while its >0 test does not reject it. Only the final owner sample need fail/reverse; earlier valid budget samples do not protect this path. Propagate unavailable owner cost through existing failure/unknown-cost handling; reject nonfinite/sentinel owner/process metrics. Inject tail-only clock failure/reversal and unavailable usage, retain actual join and prove no certified Completed/known-cost claim. |
| F3 — actual causal-witness defect, not evidence the product control is slow | tests/dream_c1.rs:1712-1726 starts foreground=true, then times Activity(true) against the next generic Paused without first consuming initial compute Paused. Budget::unit_clock:692-694 can already have emitted that initial frame. Sequence continuity alone does not bind an Activity revision; reads can stay one request behind. The logged22us p99 therefore does not yet certify200 causal RTTs. Consume initial compute Paused before any timed Activity, keep one outstanding request and add an ignored/delayed-new-Activity control that cannot pass using old frames. No wire change needed. Four actual completions/reaps remain real. |

F2 sent to author/Rook in messages01M6PM28JSTTBT663FC55P4TES /01M6PM28K7S088T7YMJ932C7EJ. F3 sent in01M6PM4G40X6VK4GR98BDKM09K /01M6PM4G4CJMK8QKGRAW44DEMB. Tools returned stored/accepted, not proof of model receipt. No task/scope override or acknowledgement request. Later signed correction45752489 is visible; its changes are **outside this fixed review**, not certified here.

## Boundary mechanisms reviewed

All line anchors here are git show16d7406 objects, not moving checkout lines.

| Interaction | Source mechanism / assessment |
|---|---|
| Parked owner / bootstrap / control | c1.rs:373-419 early macOS entry precedes runtime; :2309-2401 creates one fixed2MiB compute thread, parks on one-slot channel, pays bootstrap debt before Hello. :2448-2484 sends matching Start with foreground/deadline; no compute source read before it. Compute :1868-1905 applies nice+10/utility and bounded manifest admission. Authoritative close validation remains selected()/inspect, with no change to accepted inspect.rs/d0.rs or C1 wire/schema in this diff. Host closure/authority/admission remains consumer-owned. |
| Independent control / real join | :2415-2629 uses fixed input/output batches, flat-control preflight, coalesced progress, monotonic Activity and exact run/nonce; ControlTick:262-290 records four subparts plus whole≤1ms independently of native work. :2561-2573 joins only finished compute and checks sticky errors before Done. ParkedOwner::drop:2298-2306 disconnects and joins a real parked handle on EOF/failed bootstrap; no source access is needed. Actual successful subprocess reap remains a separate parent requirement. |
| Native boundaries / debt | :170-260 brackets only Builder::spawn or exact renameatx_np/renameat2, with1/4097 call caps, native2msCPU/25mswall, application1ms/inclusive3ms. :197-206 preserves original errno/result before timing decisions. Accounting:101-132 pays after span end, charges inclusive CPU and sticky missing cost; Budget:625-750 checks foreground/deadline before expensive admission. :2114 and TerminalMetrics:299 repay tail debt before handoff/exit telemetry. Product parent→child debt/admission cannot be verified from this producer alone. |
| Publication effects / cleanup / storage | :1263-1402 stages private original-nonce/inode bytes, publishes without replacement, captures operation/dev/ino/errno and present/unknown custody before verdict, retains unknown names and never retries that effect. :784-840 and selected():1514 onward use paced owners; track_clock:667-669 retains credits through rejection disposal. Unknown filesystem outcomes remain explicit evidence for host settlement, not proof of absence. No universal Rust/native-memory proof follows from credits or samples. |
| Descriptor versus terminal cutoff | :2032-2047 captures RAW inclusive maximum before descriptor serialization/hash/publication(:2054-2061). :2067-2118 retains later cleanup/compute metrics; :292-352 emits bounded c1-native-debt/1 native/control/terminal summaries after join/final control, before telemetry/teardown. No clamping, subtraction or reseal. Full post-reap process usage is a separate tail witness; fixed16d tests use Child::try_wait, not wait4(:407-431). Later f1b6 adds that test-side evidence and is not silently attributed to16d. |

A consumer must validate the exact approved worker hash/source/compiler/features and metrics profile, bounded complete/unique diagnostic records, finite fields/cost_known, app/control/cleanup≤1ms, native≤2msCPU/25mswall, inclusive≤3ms, counts/effect custody, zero remaining debt, captured maximum equal to descriptor and≤terminal maximum, matching immutable output/nonce plus actual successful reap/current context. Scalar max_poll_cpu_ns≤3ms alone is insufficient. Terminal process/owner/span totals overlap and must not be added as independent CPU. The terminal cutoff excludes final formatting/process teardown; parent post-reap usage covers that tail. Missing/unknown evidence stays unproven; no new worker authority/version is implied.

## Actual retained evidence, not a rerun

- target/dream-c1-native-evidence/test-dream-c1.log:27-48 shows15 C1 unit passes, including native-return ownership, CPU/wall/missing-clock effects, errno capture and strict ordinary cleanup/control tests. These are API fixtures, including injected clocks; the AtomicBool control in the owned-thread test is **not** a real admitted MCP/held-native process/control witness.
- The same log:139-180 records four completed/reaped ordinary-release workers, each2503989 input bytes/202 envelopes, heap peaks≤863744B, declared peak33386773B, no overruns, remaining debt0. Observed control maxima45334..72375ns and per-job native creation/publication maxima are below the gates. The200 framed Paused measurements22us p99/2180us max have F3's causal limit; they are not stronger RTT proof.
- :213-225 records near-limit rejected-owner cleanup (321834ns max) and paused-peer/owned-kill tests; :226-234 records real Store7,359,851B with typed blobs and actual worker output. Existing tests:617-681 cover paused stale revision/cancel/EOF/deadline, :1109-1160 independent small worker≤2s/owned kill, :1364-1425 four owners with one resumed/others cancelled, :1495-1520 cancel after retained growth. These are real process coverage; they do not establish held-native timing-failure/control behavior or combined term UI/MCP latency.
- Retained exits.json records make test-dream-c1 exit0. Its logged commands actually cargo-build/copy the ordinary worker **before** cargo-test (Makefile:283-291), with DREAM_C1_RUNNER bound to that copy. No old cargo-test artifact is relabeled ordinary.
- test-formal-parity.log:242,290 runs the causal reachability/assertion test. :310-338 and retained receipt show QuantumDebt good19230states/56119edges,11 broken controls safety failures (TLC exit12), graph equality; no missing/unsupported pass inferred. Final receipt:553-554 inventories243 models/one crate/72 contracts, recorded exit0, source **f1b6f051032de24d3e94f8f73295f7826e8a0ea3**, not16d or4575.

Model source tests/formal/dream_c1.rs:537-859 contains actual guard mutations and named-property assertions; contracts.toml:812 onward maps implementation sources/assumptions. This is a finite two-call per-run projection versus one held peer, with app/native/debt buckets and epochs A1/B2/A3. It does not prove four-job scheduling, real timing/heap/bytes, OS return, arbitrary Rust or consumer behavior. Receipt model/source hashes match fixed c1.rs/model/contracts; its test file hash is later e28a10f0..., distinct from fixed06ec334f.... Actual state/edge parity is retained evidence, not a fresh invocation here.

## Provenance, cutoff and verification

Baseline→16d changes10 files; no inspect.rs,d0.rs,c1_wire.rs/schema/IR change. Root reports signatures verified; this review did not rerun signature checks. At first inspection19:45:59Z checkout HEAD was f1b6f051; at git diff --check19:51:06Z it was457524892b9fbac3be10a24735f90844297a4924. Only .bboxy/runs/formal-native.json was dirty. All source analysis above remains fixed git objects; later source/report/gates require Oracle readback.

| Artifact / fixed SHA256 |
|---|
| c1.rs: fdf3442811eaa3113943efcdd9bf0db84cd8ba972dc4db524abefed28f08a654 |
| tests/dream_c1.rs:06ec334f9bf50ca41995e715d7cbf4c7337396b647de7a105957fff945857247 |
| tests/formal/dream_c1.rs:69235c52ed9021179fab5314efbf1951a4d16986856bbd437f099be9a44d98af |
| verification/contracts.toml:98c25f14c2fec8058572d179fcdb08505ca5b2c2362d6e08e506bfc783ef5e43 |
| Ordinary worker:12ec960ea3ec830a039153146bd350bdd72188b15ce25b311b4a397175e1963d |
| Earlier cargo-test milestone:986fd7234573a67d05d402d620d98be979f85875886e550603f7469627d62920 |
| test-dream-c1.log:4a09d5ee77ff101fe6a7aac212ef3ee1ef69f89fdf8b3b02007fe4b6114b3b22 |
| test-formal-parity.log:1a2be17e92e5d519b0ca16a6297fe12175950de21e0591bb3d64c5aab2accb7a |
| Formal evidence.json:ceaf65fe5037eb55f1249ecd2852b0a5ff6c8bcc988087516b6a663466b2b827 |

profile/source-commit.txt names16d; hashes.txt matches fixed c1.rs/test/Cargo.toml8a89f520.../Cargo.lock18b9e96... and actual ordinary binary. compiler.txt records rustc1.99.0 b940084d7, aarch64-apple-darwin/LLVM23.1.1. Make's ordinary --release/default acp-agent build is distinct from feature-gated test-fixtures tests and cargo-test milestone. Source alone does not establish identical feature-unified binaries.

Current governing KB digests: spec f90719ede139ae36a1f5a7e55dbf259820796bbe295d5e224563bcd9d374e364; ICD66b9f28f3d3e8486e992ebf7e19e84a3652f1fe397066695b41e497ed62994fe; decision65086 466a568d5ed4de5b3ba26506c4f589d646b331120b8ec93ac106fac8802d8a00. Complete continuation bodies fetched.

**Only verification executed:** git diff --check in the exact producer tree — exit0, no output; checks current tracked diff, not the correctness of fixed objects or generated evidence. Read-only Git/source/hash/log/receipt inspections additionally ran. No product/full gate was run by this seat. Retained test/formal logs do not certify later4575 changes; renewed source-bound gates and F1–F3 readback remain with root. Combined parent/child, held-native process control, fresh public authority, native frames and shared hints remain explicit consumer/integration evidence; no new architecture requirement or original C1/Q2 scope change is imposed.
