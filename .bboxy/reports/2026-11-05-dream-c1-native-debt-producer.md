---
type: report
title: C1 native producer debt and earliest bootstrap verification
author: seat:01M6PJAVQFS8RT61ZQ86T7QRRR
created: 2026-11-05T22:29:08.722Z
task: '[[tasks/2026-11-05-dream-c1-native-debt-producer]]'
commits:
- ffa09e52b8302ed74aaba4e4c7ede3bf653b7954
- 16d7406dbb62c939933391783666bd7c3d36c2c1
- f1b6f051032de24d3e94f8f73295f7826e8a0ea3
- 457524892b9fbac3be10a24735f90844297a4924
- 206c8bff6a3746c77bad88d909e610669cee84a5
- bab543e2116d0bf08488fbd753a6114f30d615a4
- 56ac567cc0149c6c62202e91ba47e8b5ffe00c2f
- ca96dcd7e5ed6da45385746f7fa4a12fd12ac647
- d06e696f99472ed754e84353baf11a2974589d6d
verify:
  command: make test-dream-c1 && make test-formal-parity && make check-embedded-build
  exit_code: 0
id: 01M6PX8ZY2HZ7TMC18RY8FQRPZ
updated: 2026-11-05T22:29:08.722Z
number: 93377961365643
created_by: seat:01M6PJAVQFS8RT61ZQ86T7QRRR
updated_by: seat:01M6PJAVQFS8RT61ZQ86T7QRRR
authorship:
  signer: seat:01M6PJAVQFS8RT61ZQ86T7QRRR
  kind: seat
  public_key: BUWjYod/PEjChsPHmfz/QocQZiaar0Kx4vjimII/0bc=
  process_generation: '1'
provenance:
  digest: sha256:5c9bf3d2c0441a3d38b60669abcb8deb812e3ff3793aebdcdd0ff432f611d581
  frontmatter_digest: sha256:224053f6bbab9b3bd63b13636ed38f7560563ff09e9d1663751318db8ec4a901
  signer: seat:01M6PJAVQFS8RT61ZQ86T7QRRR
  sig: BjTI8naGIbqB0lkBENme1eIt+Z+8TNkzEtOG0VnCCt5mVYbrPb8W/Zan62JUzCjkIgHKUYUORPE6aW6TNubBCw==
  log: .bboxy/log/2026-11-05.jsonl#145527
---
Task: [[tasks/2026-11-05-dream-c1-native-debt-producer]].
Implemented producer decisions [[decisions/d93377961365086-dream-native-quantum-and-debt]], [[decisions/d93377961365166-dream-bootstrap-private-permit-handoff]], [[decisions/d93377961365255-dream-bootstrap-source-free-probe-context]] and [[decisions/d93377961365259-dream-bootstrap-final-public-admission-control]], using the complete selected [[specs/dream-term-c1]] and [[interfaces/dream-term-c1]].

The producer checks below passed on signed source **d06e696f99472ed754e84353baf11a2974589d6d**. This is an Executor handoff, with no acceptance, production pin, publication or landing claim. The consumer's actual term-parent/authority/native-frame/foreground/hints combined gate remains unproven here. Its cold copied-artifact readiness failure is retained below.

## Source, scope and verification

Worktree: `/Users/feral/mydev/bboxy450-harness-dream-native-c1`; branch `fix/dream-c1-native-debt`; frozen accepted base `47182320be6435c860bc2176c2ddda582029fa63`. All 56 changed files are within the amended exact allowed paths. No consumer, Q2, old producer or primary-checkout source was edited. No dependency/lockfile, schema/wire/algorithm/IR/control-version change, new pool, global scheduler, install, release, merge or push.

All nine source commits have Git signature status G under Boxy <bboxy450@gmail.com>, machine key 9B07E849480C68328CA20684844ECC0587DE7AB0; `git verify-commit HEAD` exited 0 with a good signature. Chronological commits:

- `ffa09e52b8302ed74aaba4e4c7ede3bf653b7954`
- `16d7406dbb62c939933391783666bd7c3d36c2c1`
- `f1b6f051032de24d3e94f8f73295f7826e8a0ea3`
- `457524892b9fbac3be10a24735f90844297a4924`
- `206c8bff6a3746c77bad88d909e610669cee84a5`
- `bab543e2116d0bf08488fbd753a6114f30d615a4`
- `56ac567cc0149c6c62202e91ba47e8b5ffe00c2f`
- `ca96dcd7e5ed6da45385746f7fa4a12fd12ac647`
- `d06e696f99472ed754e84353baf11a2974589d6d`

Verification used a sequential runner with CARGO_BUILD_JOBS=2, one target at a time, stopping on the first nonzero result. The required chain `make test-dream-c1 && make test-formal-parity && make check-embedded-build` therefore has exit 0; each constituent invocation and actual exit is retained in `/Users/feral/mydev/bboxy450-harness-dream-native-c1/target/dream-c1-native-evidence-d06e696f/exits.json`.

| Command | Exit | Elapsed seconds |
|---|---:|---:|
| `make test-dream-c1` | 0 | 435.669 |
| `make test-formal-parity` | 0 | 270.188 |
| `make check-embedded-build` | 0 | 32.572 |
| `make test-licenses` | 0 | 0.430 |
| `make check-formal-evidence` | 0 | 0.275 |

Logs are `/Users/feral/mydev/bboxy450-harness-dream-native-c1/target/dream-c1-native-evidence-d06e696f/<target>.log`. Checks ran unattended. C1 included 2 bootstrap codec/environment units, 19 C1 units, 4 fingerprint units, all 31 C1 integration cases, ACP launch-context, dream replay, seat process, the separate actual bypass-effect negative, retained-seat fixtures, headless fixture environment, MCP source tests and 18 MCP client cases. Two existing dream-replay dispatcher/worker entry cases are marked ignored in their normal aggregate and are explicitly invoked as child tests by the process witnesses; no new ignored test or reduced corpus was introduced. These are the scoped gates, not a claim that every repository target or live GUI gate ran.

Worktree verification after these commands: only `M .bboxy/runs/formal-native.json`, the generated private formal receipt; no uncommitted source or untracked source file. Nothing under .bboxy was added to Git. The archive survives target cleanup and the formal source-dirty predicate is false. No rebase occurred; a future rebase requires fresh evidence.

## Mechanism and acceptance mapping

All source lines refer to this worktree at the signed final source.

| Requirement | Implementation and executable evidence |
|---|---|
| Earliest gate before owned argv/path/Arc or runtime | `src/main.rs:9` calls non-inlined `c1::early_worker_entry` before ordinary startup; ordinary CLI future/GUI work is behind `ordinary_main`. `src/dream/c1.rs:418` samples/recognizes borrowed bounded arguments and waits for the permit before owned argv. GNU Linux recognizes and refuses unsupported C1 before runtime; late async C1 entry refuses. Actual execution here is Darwin only. |
| One parked dedicated owner | `src/dream/c1.rs:2426` admits channel preparation and only the one Builder::spawn after the live permit. Fixed stack_size=2097152, parked receiver until matching Start; expensive compute additionally checks foreground/currentness/deadline. No source/manifest/provider/model discovery while parked. `ParkedOwner:2369` retains the actual handle through EOF/malformed/missing Start/timing failure and joins it. |
| Precise native boundary and strict ordinary spans | `src/dream/c1.rs:177` times only the supplied exact callable; producer call sites are sole Builder::spawn and prechecked no-replace publication `RunIo::publish:1322`. At most one callable per span; native caps 2ms caller CPU/25ms wall, surrounding separately sampled application CPU 1ms, raw inclusive CPU 3ms. Native CPU remains in raw inclusive totals. Parsing, hash/serde, control and cleanup retain 1ms. No blanket control wrapper or 3ms parser allowance. |
| Post-end debt, independent controls | `Accounting:61`, `Budget::native:685`, `unit_clock:739` and `cleanup_unit_clock:818` create 9*max(1ms, inclusive C) rest after the span, then pay before another expensive admission. No saved IO/idle credit. Bootstrap is paid before Hello. Established control stays on its own main owner while compute rests/blocks. Finite permit/lease/Hello/Start bookkeeping is control, not recursive work debt. |
| Actual result/effect before verdict | `NativeOutcome:150` captures the returned typed handle/status before timing/cancel verdict; original errno is captured before later clock probes and restored for receipt capture. `PublicationCustody:606` retains original nonce/operation/dev/inode/effect and uncertainty; no possibly completed publication is retried. Unit witnesses pair real thread/hard-link effects with failed CPU/wall/clock verdicts and an unknown publication result. |
| Cleanup and R1 ownership | Existing returned-owned-result/Paced disposal and credit ordering remain. Rejected nested/partial owned results drain in paced bounded steps before credits release. `ParkedOwner::settle_clock:2376` additionally measures sender disposal plus actual join as ordinary settlement, folds raw caller CPU into application/inclusive/main-owner maxima, refuses unknown/>1ms, pays its post-span debt and does not dispose twice. Real parked join tests at `c1.rs:2900` cover valid/reversed/unavailable/over-limit clocks and actual exit. No native join exception. |
| Sticky clock/finite tail | `observed_interval:141` marks invalid intermediate samples even with finite outer endpoints. Control parts, inner cleanup pacing and final post-tick samples use it. `owner_result:304`, `process_usage:312`, `settled_result:2348` propagate unknown owner-tail cost before Done or nonzero final exit after immutable Done. Real-effect units at 2951/3148/3204 and the integration finite-usage gate reject MAX, not merely values >0. |
| Raw descriptor capture and terminal tail | `c1.rs:2098` takes the immutable raw PRE-DESCRIPTOR maximum before its own serialization/hash/publication. Terminal observations cover later descriptor work, cleanup, joined ownership and final control/settlement; tests require captured<=terminal. No self-referential reseal or wire change. Parent wait4 receipt observes complete reaped-process CPU including final telemetry/exit tail. |
| Resource/deadline rules | Existing strict foreground/deadline admission, original 30s budget, 2s+2s escalation/actual reap/quarantine and 32/96MiB and 78+16+2 credit split remain. `tests/dream_c1.rs:1877` asserts memory/credits; large/multiple-page/near-limit/returned-owned-result/failure tests remain in the full gate. |
| Attribution | Existing origin notices retained. ATTRIBUTION.md credits project-owned native/debt/bootstrap/source/model/fixture work; source/checker headers carry the canonical project/license/policy URLs. |

### Canonical bootstrap and real custody

`src/dream/bootstrap.rs` is the sole producer authority; `src/dream/mod.rs` registers it. Fixed private profile **dream-bootstrap/1**: environment `BBOXY_DREAM_BOOTSTRAP=1:N:birth:binding`, ASCII<=128, N decimal3..65535, leading zeroes allowed, canonical uppercase ULID birth and lowercase SHA256 binding. Duplicate env values, wrong profile/fd/type/owner/identity, invalid fields, padding, extra/truncated bytes and expiry refuse.

The 128 bytes are BBDBOOT1 at0..8, purpose at8 (1 inspect-c1/2 read-prepare), probe at9, reserved zeros10..16, birth/run/nonce at16..32/32..48/48..64 as 16-byte big-endian ULIDs, binding32bytes at64..96, incarnation/sequence/deadline at96/104/112 as u64LE, and reserved zeros120..128. Incarnation is nonzero, sequence exactly1, deadline absolute CLOCK_MONOTONIC ns. The canonical sorted compact UTF8 JSON binding includes profile,purpose,probe,op_id,run_id,nonce,incarnation,context,worker_sha256. Every probe serializes explicit context:null and permanently refuses ANY Start, including matching IDs; real work requires full context before birth, with no null fallback. Child compares its opaque original binding/purpose/birth/run basename and later Start run/nonce; it does not derive actor authority or recompute the parent's context hash.

Gate at `bootstrap.rs:392` owns the unique inherited anonymous readable pipe, retains its own full-u64 dev/inode/uid stamp and checks lease/deadline before every controlled bootstrap admission and after each debt wait. Darwin pipe reader/writer inodes differ and are never compared to each other. Compiler-checked declared gate/control state <=1KiB is inside the protected2MiB. Bounded borrowed env/argv recognition, one128B buffer, at most4 permit reads/4 interrupted waits and remaining-deadline sleeping poll; no prepermit source/hash/JSON/runtime/owned argv. Extra bytes/HUP/EOF revoke further work; actual thread effects still settle. Reader retires after bootstrap debt before ordinary Hello; parent writer is held through qualifying Hello.

Readiness is never reset: child entry+250ms, original parent spawn-return+250ms and original job deadline are intersected. Parent fixture `HeldWorker:436` retains returned Child/lease before fallible ending-clock verdict, pays native launch/setup debt, makes one128B write, validates Hello against the original deadline including a post-decode sample, closes the lease only after Hello, then prepares/pays before original bound Start. Fixed IDs are parsed before birth. No retry/new birth after unknown/partial grant.

`tests/fixtures/dream-bootstrap` supplies immutable 34 record/environment cases (4 valid,13 invalid binary,9 invalid env,8 identity mismatches), 8 probe/work binding cases, exact JSON/hash sidecars, independent Python standard-library checker and Rust conformance. Their static deadline is deliberately not a live admission timestamp. The process module tests seven families: held actual birth before verdict/grant with zero-admission marker and independent already-prepared peer control/EOF; source-free missing env; sixteen malformed/partial/extra/duplicate/expired/revoked cases; foreign/closed/writable/regular descriptors and bad tokens; unknown native clock/stale ABA binding with actual Child retained; hostile matching probe Start/wrong nonce; and valid lease retirement/Start/complete/reap. Fixture original-binding checks are not consumer public-authority proof.

The deliberately invalid test-fixtures artifact really allocates before permit. It receives NO grant, its lease is revoked immediately, and the same waiting-marker guard rejects actual argv_admissions=1 through bounded actual reap. Final log records actual_reaped_pid=91073, grant_count=0, readiness_certified=false. Silence, delayed startup or a missing marker cannot satisfy this negative. Ordinary positive artifacts never enable the mutation.

The old main really reserved 38000 bytes before early dispatch; the final artifact main reserves16 bytes then calls early_worker_entry and branches to ordinary_main only on refusal to select C1. Raw artifact-bound prologue prefixes are `/Users/feral/mydev/bboxy450-harness-dream-native-c1/target/dream-c1-native-evidence-d06e696f/before-main-prologue.txt` (immutable ca96 artifact55c3787e...) and `after-main-prologue.txt` (immutable final414be...). This isolates ordinary application startup, not dyld/kernel/thread-entry latency.

## Frozen private metric profile and cutoffs

All duration values are integer ns, counts/bytes are integers. No public frame/schema/version change. Exact source/README profile authority: README native table and bootstrap paragraphs; `c1.rs:329` and `bootstrap.rs:421/535`.

| Record | Exact bounded fields and meaning |
|---|---|
| native tag=compute_owner_create or publish_noreplace | count,cpu_ns,wall_ns,max_cpu_ns,max_wall_ns,inclusive_max_cpu_ns,inclusive_max_wall_ns,app_max_cpu_ns,effects,failures_with_effect. Count caps1/4097. Exact callable CPU/wall plus separately sampled outside application intervals; raw inclusive cost is not subtraction/clamping. |
| terminal_metrics profile=c1-native-debt/1 cutoff=joined_final_control_pre_telemetry | captured_max_cpu_ns,inclusive_max_cpu_ns,app_max_cpu_ns,control_max_cpu_ns,inclusive_max_wall_ns,span_cpu_ns,owner_thread_cpu_ns,process_user_cpu_ns,process_system_cpu_ns,wall_ns,debt_total_ns,rest_observed_ns,remaining_debt_ns. Includes descriptor work and later cleanup/join/control before final telemetry formatting/exit. |
| cost_known | true/false; any unavailable/reversed measured boundary is sticky unknown, never zero/wrapping certification. An unavailable inclusive CPU interval receives conservative27ms mandatory-settlement rest; finite outer raw cost remains finite even if an inner sample invalidates certification. |
| control_parts | input_max_cpu_ns,protocol_max_cpu_ns,settlement_max_cpu_ns,output_max_cpu_ns,total_cpu_ns. Established whole ticks independently retain the strict1ms maximum. |
| publication_custody | run_id,nonce,operation (page index or4096=result),dev,ino,effect (0 absent/1 present/2 unknown),errno,cleanup_uncertain,path_mapping,temp_mapping. Latest original operation only, not per-call unbounded logging. |
| existing metrics | compute_cpu_ns,max_unit_cpu_ns,cleanup_cpu_ns,max_cleanup_unit_cpu_ns; heap/RSS/credits/normalization fields. Compute cutoff before owner exit; native_cumulative_bytes=unmeasured remains explicit. |
| parent_reap | reap_user_cpu_ns,reap_system_cpu_ns,parent_elapsed_wall_ns,cutoff=actual_wait4_exit. Full reaped-process usage includes telemetry/exit tail; parent elapsed wall includes launch/collection overhead. |
| dream-bootstrap state=waiting profile=dream-bootstrap/1 | argv_admissions=0,channel_admissions=0,owner_admissions=0 before controlled startup; actual invalid fixture reports argv1. |
| dream-bootstrap state=paid_hello | those admissions plus frame_admissions,permit_reads,control_cpu_ns,control_max_cpu_ns; four fixed admissions, numbers captured before this record's own emission. Final merged control/process observations include emission and reader retirement. |

Start cutoffs matter: the Accounting span/debt/rest ledger and its wall origin are created in run after the separately measured early argv/path/Arc unit. That earlier <=1ms unit is included in the raw main-owner maximum, pays its own9*max(1ms,C) sleep before rechecking the live gate, and cannot enter run unpaid. Its manual rest is not included in debt_total_ns/rest_observed_ns; those fields describe the Accounting ledger. Full process CPU covers all startup; parent_reap wall covers birth through exit/collection. span_cpu_ns, owner_thread_cpu_ns and process CPU are separate overlapping observations and must not be added. No pre-descriptor sample is claimed to measure future telemetry/teardown. A legacy scalar<=3ms is insufficient; legacy1ms consumers may safely refuse.

### Final ordinary four-job observation

Final ordinary worker is a plain cargo build --offline --locked --release --jobs2 --bin bboxy450-harness, default acp-agent features, **NO test-fixtures**. Four real jobs across two declared roots each processed2,503,989bytes/202envelopes/40turn fixture data, completed one page and result, and were actually wait4-reaped. The 200 Activity(true) RTTs used drained initial compute Paused, one outstanding fresh revision, ignored-stale timeout and withheld-LF fresh-Activity timeout then response. p99=17us, max=34us; this is causal request RTT evidence for this run.

Maxima below are across four jobs, not one synthetic combined job:

| Observation | Maximum |
|---|---:|
| raw captured/terminal inclusive/application CPU | 284250ns |
| control CPU | 109917ns |
| cleanup unit CPU | 67334ns |
| bootstrap control unit CPU | 27334ns |
| native compute creation CPU/wall | 31167/31000ns |
| native compute inclusive CPU/wall/outside app CPU | 31417/31542/250ns |
| native publication CPU/wall | 135958/149792ns |
| native publication inclusive CPU/wall/outside app CPU | 136208/150417/376ns |
| per-worker native counts/effects | compute1/1; publication2/2; failures_with_effect0 |
| observed ledger debt / remaining at terminal | 5526000000ns /0 for every job |
| terminal wall | 20054981792ns |
| heap peak / RSS peak | 863760/10174464bytes |
| declared reserved peak / credit ceiling | 33386794/100663296bytes |
| stack/control/helper reservations | 2097152/2097152/16777216bytes |

All four cost_known=true, no overruns, declared live storage0 after cleanup and captured<=terminal. Per-job exact CPU/rest/wall/post-reap fields are in `/Users/feral/mydev/bboxy450-harness-dream-native-c1/target/dream-c1-native-evidence-d06e696f/four-summary.json`; maxima in four-maxima.json. Across jobs span_cpu<=20798242ns, owner_thread_cpu<=113845250ns and terminal process user/system maxima92443000/81557000ns. For the largest complete reaped CPU observation, job2 wait4 user=92459000ns and system=81582000ns (174041000ns total), versus terminal user92443000/system81557000; the later tail is observed, not guessed. These overlapping CPU measures are not summed with span/owner CPU. Observed ledger rest maxima19871551350ns includes oversleep; it is not saved credit.

## Artifact/compiler and formal binding

Ordinary immutable artifact: `/Users/feral/mydev/bboxy450-harness-dream-native-c1/target/dream-c1-native-profile/worker-414be210e45ea228af5d85ee38f484ef7ed5918603a27d718cf9691d6aeaae81`, SHA256 **414be210e45ea228af5d85ee38f484ef7ed5918603a27d718cf9691d6aeaae81**, responsibility [[01M6PW8R5VB58P7YZQCC8CHMAF]]. Source/compiler/hash/bootstrap-support receipts are preserved under `/Users/feral/mydev/bboxy450-harness-dream-native-c1/target/dream-c1-native-evidence-d06e696f/profile`. The mutable worker path is a staging path, not an old artifact identity.

Compiler rustc1.99.0, commit b940084d7eb6a299eb4bfeb8e34901bc051e7ac4, aarch64-apple-darwin, LLVM23.1.1. Fixture-only immutable negative artifact is `/Users/feral/mydev/bboxy450-harness-dream-native-c1/target/dream-c1-native-evidence-d06e696f/fixture-worker-2477980187741aa7738619b6a82c5c7d00e063b1ba717774c52d250313281526`, SHA256 2477980187741aa7738619b6a82c5c7d00e063b1ba717774c52d250313281526, default acp-agent +test-fixtures, declared before birth and recorded in fixture-profile.json. It is not ordinary-release or readiness conformance.

Key hashes:
- `Cargo.lock`: `18b9e96af1b5aa1bc0b6693c1557daca9195ee80d062d22be48b843abdbf8ac9`
- `Cargo.toml`: `8a89f52077beeda99b07d12a505c374f5d1fa97dc9df5d5715866330985b897c`
- `src/dream/bootstrap.rs`: `14b3155d23b47e1ac45b9d023bbea00cab730c3043fbf954dc95402d9ae63b05`
- `src/dream/c1.rs`: `63b703f3ddaf693f23f3598c6d121945d1333c098af773db1d53feb0fd97d471`
- `src/main.rs`: `933e39cb206c63d5a3002d41bdaddb4603cee46edf47ede032ff974f71ba0ca9`
- `tests/formal/dream_c1.rs`: `8d2e6f48049364d3d06c0fa02f4087edf6f454d6350d10f28ca776e14b880d2b`
- `tests/formal/models.rs`: `78867a8ced58228b017b8770fba229b18eabe9f0be648522c69661477f472172`
- `verification/contracts.toml`: `d88fab95e0ab1e856024fed525e384d9c5d9834acfea370cde4ae9f27454b7a4`
- `verification/corpus.toml`: `28c10908715400a3af09ea0230e8edb591b9c222322c55e84cbee41234735f88`
- `verification/coverage.toml`: `4341715aeba160042053ff9452cc78a0e66139e7f8c24d75098d6763b6188cf0`
- Bootstrap vectors.json: `de919016ac6e5a464de60c86b2bf3bf6b54edc08e29c05f89688a5c80da37307`.

Formal archive: `/Users/feral/mydev/bboxy450-harness-dream-native-c1/verification/evidence/4a57cba19c2c18f5310bbdb32a896cfd11770fb49315c4696fb107811f88fba6/evidence.json`, receipt digest **4a57cba19c2c18f5310bbdb32a896cfd11770fb49315c4696fb107811f88fba6**, source d06e696f99472ed754e84353baf11a2974589d6d, dirty=false, all261 models/73 contracts/one crate inventoried, complete native/Java state and labeled-edge parity plus registered safety/deadlock/liveness verdicts. check-formal-evidence exited0 on that source.

C1QuantumDebt:19230states/56119edges,11 causal broken variants; BootstrapHandoff:9272states/35252edges,17 causal broken variants. Controls include unpaid rest/Hello/Start/grant, fake-native parser/control/cleanup, hidden/missing CPU, repeated/unresolved effects, early owner release/presentation, clock/cancel/ABA, withheld completion, earliest argv/channel/Builder bypass, parent/child overlap, unknown birth release/retry, revoked/stale pipe/nonce, duplicate permit/admission, probe Start, null real-work context and misclassified/over-limit final control. Model source at tests/formal/dream_c1.rs:551/867, inventory in models.rs/corpus/contracts/coverage.

Bounds: two symmetric jobs projected onto a focused two-native-call run versus a held peer; CPU/debt finite0..3/0..27 abstract ms, three A1/B2/A3 epochs, typed absent/present/unknown effect. Bootstrap models one birth/permit, three major argv/channel/Builder classes, parent/child bootstrap/preparation/work ownership, null-probe/full-context choice and <=2 public admission attempts with <=7 prepared chunks. Framing bytes, exact FD/parser/clock/profile behavior and compiler/OS behavior are implementation witnesses, not formal refinement. Weak fairness covers enabled internal control/verdict/rest/settlement; no fairness is assumed for native OS return, delivery or observed exit. Withheld effects retain ownership/quarantine. Prepared public authorization is an external consumer assumption, not invented authority or a producer identity subsystem.

Native checker/Yarn dependency0.1.1 remains pinned. TLC1.7.4 SHA256 936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88; Java21.0.13+9 SHA256 225bab0a09491ef06835488cd942f5d2befa9a0410a2484f0c8bbcf7506ec09a. Source/model/tool inputs and TLA digests are retained in the archive. Finite-corpus parity is not arbitrary Rust correctness or complete TLC replacement.

## Failed baselines, coordination and limits

- Original four-job failure remains untouched at `/Users/feral/mydev/bboxy450-term/target/dream-c1-native-boundary-somrqvej`, actual runner16a746d6d115f75c33402ac61ded2f9ba6ac2272930acd0bd99febb7c9ce4edf, all4refused/reaped. Same source did not imply same binary as standalone. Accepted47182320 and its approved older binary remain historical.
- Native-only source206c passed C1/parity/build/licenses before the private bootstrap amendment; it cannot certify cross-process no-overlap. Old22us was queued paused-stream timing. Fresh-revision/ignored/delayed causality was fixed before the final run.
- bab543e2 canonical `target/dream-c1-native-evidence-bab543e2/exits.json`: C1 exit2. Its ordinary30-test and four-worker measurements are real, but deliberate fixture missed the source-bound marker at251ms while child remained alive, before the intended checker. Earlier /tmp/dream-bootstrap-bypass*.log failures are retained; missing-marker cause was not established. Corrected negative requires a real effect through revocation/reap, not a timeout pass or a relaxed250ms bound.
- 56ac567c canonical C1 exit0/261-model parity exit0, then embedded-build exit2 on chunks_exact_to_as_chunks Clippy error. The codec iterator correction changes no bytes/grammar; final build is0.
- ca96dcd7 canonical C1 exit2: a REAL publish_noreplace call observed61575666ns wall,157125ns CPU,61578250ns inclusive wall, actual effect present. It reported failures_with_effect=1 and Done Failed/resource_limit despite a real result publication (original op4096,dev16777234,ino178180733). The test actually waited the child before its Completed assertion failed; its three paused peers had already cancelled/reaped. There was no repeated publication or limit waiver. The later four-job run in that same failed command passed but did not make the command green. This is actual OS wall-overrun/effect-retention evidence; injected >2ms CPU/>25ms wall unit probes separately retain real owners/links and are not claims that those OS calls really took the injected time.
- A 64KiB fixture binary-approval hash unit exceeded1ms in /tmp/dream-bootstrap-bypass-revoked.log. Approval hashing was split to16KiB units with the same1ms and9x rest; no exemption or clamping. An initial formal model exceeded its state cap during development; paid scratch was canonicalized without removing the corpus/outcomes, and final full graphs/controls are preserved. One exploratory command used nonexistent --test formal and exited101; corrected formal_models command and canonical gates ran successfully.
- Consumer independent private copied d06/414be probe still failed original250ms before gate entry: `/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-aqd69hlb`, peer message01M6PWCBXKZPXC3G1AZXXG4ZVX. Its post-failure owned-PID sample was in dyld before main; sampling may perturb later execution, so no baseline duration is inferred. Earlier bab copied-artifact failures and one195.324ms first-marker/32.631ms child-run success remain distinct observations at the peer's retained paths. They are not producer final-gate success or a universal readiness bound. Exact local plain-build successes do not erase cold-copy failure.
- Final source/profile/immutable artifact handoff went to consumer01M6PW8R4W41BF3EB76R0VJBKA, Q2 01M6PW8R58XP6FATB5PJKFZ909 and Oracle/Rook01M6PW8R5HNEW3RB3MPZMFN2NY. Receipts show native-queued for Q2; storage/queueing is not model acknowledgement. Consumer explicitly independently verified the tuple in its reply. Older self-addressed handoff was corrected with an explicit destination. No acknowledgement wait or production repin.
- Consumer owns actual parent Command::spawn, SQLite splitting/fresh bounded public authorization callbacks, <=1/account runtime probe cache, actual authority/root/public-row tamper checks, native frames/foreground/hints and combined gates. Producer fixtures do not prove these and no source pin was changed.
- Observed native/startup allocations are not universal OS/loader bounds. No native heap/kernel cumulative-memory certification, arbitrary-platform execution guarantee, global scheduler/allocator correctness, arbitrary Rust refinement or live user GUI conformance is claimed. All native/kernel CPU visible to process clocks remains in full process observations; overlapping totals are not double-counted.

Project/origin: bboxy450-harness, https://github.com/bboxy450/bboxy450-harness . Respectful-slop policy and retained attribution: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md .
