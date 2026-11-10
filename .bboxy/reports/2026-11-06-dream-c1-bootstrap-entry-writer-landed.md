---
type: report
title: Ordinary Dream entry and bounded writer landed; authorized final report
author: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
created: 2026-11-06T16:14:47.757Z
task: '[[tasks/2026-11-05-dream-c1-bootstrap-entry]]'
supersedes: '[[reports/2026-11-06-dream-c1-bootstrap-entry-landed]]'
commits:
- 94e7e7af75b951c50a8659fe29a533d9a2b2a60a
- fe2bf755c96a162ede58f450d4d2e5331c0c1d42
- d027a16b33aa6a4d78797f9eaf93901f762e28bd
- 9165b7d50818d11d11eb6a0d9fb753f8c54d805e
- 2ce4359df034405043d419fe550b0f6cafe7887e
verify:
  command: make check-embedded-build && make land
  exit_code: 0
id: 01M6RT88SYDBK4R1B7P2MT1Y1B
updated: 2026-11-06T16:14:47.757Z
number: 93377961366034
created_by: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
updated_by: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
authorship:
  signer: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
  kind: seat
  public_key: aaB9vpN0zTwKyfaUMhUFUSvyTZ6x1t9lQBaEEuGygFk=
  process_generation: '1'
provenance:
  digest: sha256:e38cd01da1988554bcdee34b35b56a754fcdc45f69d6e73c4e8dbaf682314f53
  frontmatter_digest: sha256:f580bca768d52ca8cbd30b9bd01f0089d390fb3083c9bcb233f9c637f62d6e6c
  signer: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
  sig: j7DbQPR01yZPkqWgNzPWbm2DHSUD71QZGp0KRyObIRLcBunAkeq4i9C+0xcVRJwbz9QpuLQh0j8W/s1suz9UCQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149565
  event:
    id: 01M6RT8A1JAHYG22THWYRBZ3VZ
    sequence: 149565
    hash: 6075fce25b89504ba756a492ae46729f0cc0f81ebbc548a4172d51dae7db0167
---
# Ordinary Dream entry landed; final source gate passed

Task [[tasks/2026-11-05-dream-c1-bootstrap-entry]], amendment digest65d82d0bd9090ae582f170b27f220c2f27ecbdcc9402ef4cbd1054b8aff8843e, verify revision1. This is the NEW append-only superseding report authorized by that brief. [[reports/2026-11-05-dream-c1-bootstrap-entry]] and [[reports/2026-11-06-dream-c1-bootstrap-entry-landed]] remain immutable records of their failed gates.

The successful gate ran under authorization digest5940bfda8e0621a8e151b6996e395bdc80c9f043b440ae945c77da70fd277765. This report now uses the EXACT writer-landed locator named in the subsequent report-path amendment. The preliminary [[reports/2026-11-06-dream-c1-bootstrap-entry-fixed-writer-landed]] remains immutable; its source, proof and exit0 are unchanged. This is a locator correction with no source change, worker birth, test or gate rerun.

## Result and source custody

Exact required command `make check-embedded-build && make land` exited0 on2ce4359df034405043d419fe550b0f6cafe7887e. Canonical source push landed that exact signed head on bboxy450/bboxy450-harness main. Subsequent `git ls-remote origin refs/heads/main` returned exact2ce; local origin/main also exact2ce. Clean worktree; git diff --check exit0. No additional positive probe, warm-up, fallback or whole-gate retry followed this successful command.

Canonical receipt source digest ca81748af30bde0df19e04e0859ab92ce6d6a731e899cbbebe4fa4aa93e53481. Full command ran2026-11-06T15:54:45.500765Z–16:03:24.063021Z, ordinary controller nice0/compiler nice10, Rust jobs2/testthreads2. Diagnostic-only flags, expected-hash diagnostic flag and test filter were absent. Existing canonical Java checker pool/one-worker TLC configuration was unchanged. Public verification material only entered the private test HOME/GNUPGHOME.

All own commits verify G, Boxy <bboxy450@gmail.com>, key9B07E849480C68328CA20684844ECC0587DE7AB0:

- 94e7e7af75b951c50a8659fe29a533d9a2b2a60a: ordinary entry, units/process controls, model/maps/docs.
- fe2bf755c96a162ede58f450d4d2e5331c0c1d42: split record sampling/format storage from permit decoding.
- d027a16b33aa6a4d78797f9eaf93901f762e28bd: allocation mutant must fail for actual controlled admission after valid entry.
- 9165b7d50818d11d11eb6a0d9fb753f8c54d805e: immutable parent native observations/effects and injected guard boundaries.
- 2ce4359df034405043d419fe550b0f6cafe7887e: fixed-storage parent diagnostics, bounded charged phases and numeric receipts/failures.

The signed rebase preserved upstream a75eabfa4c19ab1af2cd192809d75fe510dff1e3. Historical pre-rebase39514ce7/c0830533/ed3d4ffc bindings remain in prior reports/logs. Own source changes are ONLY bootstrap.rs, tests/dream_c1.rs, tests/formal/dream_c1.rs, tests/formal/models.rs, verification/contracts.toml/corpus.toml/coverage.toml, README.md and ATTRIBUTION.md. No Makefile/scripts/land.py edits. Root retains worktree/seat cleanup.

## Acceptance matrix

| Criterion | Source and command evidence |
|---|---|
| Ordinary exact entry sample and record | src/dream/bootstrap.rs:376/383/428/460: fixed Entry, existing e then monotonic m, stack proc_pid_rusage V4, then mach_absolute_time g; unrolled fixed-order line before unchanged waiting. No ordinary feature gate, heap, extraFD/thread/env/argv/profile/public wire field. Final canonical37-test integration passed. |
| Raw failures/unknown and primary clocks | bootstrap.rs:416 clears ONLY s/u/k/r/p and ok0 iff rusage fails. e/m/g retained, successful zero raw counters still ok1; existing clocks fail honestly. Raw s/g/u/k/r/p stay verbatim UNVERIFIED auxiliary, never converted/subtracted/classified or admission evidence. |
| Fixed full writer and one charge | bootstrap.rs:313 fixed256B Line, borrowed send reuses original pipe/nonblocking checks and exact one-write length; :605/624 full prelude observation before any permit read includes both records/sampling/format/fd checks. Original1ms limit and finite clocks,128B permit, lease/debt/Hello/Start/admission laws unchanged. |
| Maximum bytes/allocation/failure units | bootstrap.rs:909 exact maximum-u64211B INCLUDING LF, exact failure bytes and allocator0 with actual allocating control; :943 finite observation, reversal/overflow, one-charge/control-limit units. Final affected library116/116 passed,1 unrelated skipped. |
| Actual private process order/custody/CPU | tests/dream_c1.rs:1291 strict entry→waiting checker enforces e<=m<=parent observation; e<parent spawn return is legitimate. :3490 seven actual Gate variants cover ordinary/EOF/pipe/overbudget/late/duplicate/noobserve with actual exact-PID settlement. Full integration37/37 passed. |
| Causal prepermit allocation mutant | dream_c1.rs:3410 first validates actual entry trace, then requires actual controlled-admission rejection; fresh private fixture d211… was reaped PID43126. It is not the ordinary worker. |
| Retain existing real regression | dream_c1.rs:3310 retains four ordinary release workers/multiMiB/200 actual causal PRIMED roundtrips, ignored/stale and delayed-fresh controls, debt/cleanup/actual reap. Final run passed; p99=17us/max=21us, all four reaped and remaining_debt_ns0. Existing near-limit/error/cancel/teardown regressions passed in the same37/37 gate. |
| Model phases/owners/actions/fairness | tests/formal/dream_c1.rs:1298/1350 explicitly bounds exec→entry→entry_emitted→waiting_emitted→permit_read→Hello/refusal, initial counters, sole child ownership, atomic actions and local weak fairness; correct order/count/allocation/CPU invariants and intended late/allocation/duplicate/noobserve controls. OS scheduling/syscalls/heap/clocks/permit delivery are implementation-only. |
| Inventory and exact native/Java parity | models.rs:6887 and verification maps add ONLY5 entry models/1 contract. All upstream270/75 rows retained verbatim, now275 models/76 contracts. Full final native/Yarn versus pinned Java TLC reachable states, labeled edges and expected verdicts matched for ALL275; no cap/control removal, timeout or TypeOK green. |
| Parent diagnostics preserve original effects | dream_c1.rs:336/362 immutable original optional CPU/wall/PID/run/artifact; applied guard remains separate. :413 exact bounded identity alphabets and enum settlement labels avoid JSON escaping ambiguity. :501/531/570/582/628/661/703 implement fixed typed/stack formatting, one checked write, measured owned disposal, numeric cost/effect receipt and separately measured/checked/paid terminal receipt. No native law or worker source changed. |
| Parent causal and allocation checks | dream_c1.rs:833/915/954/1112 process-free maximum/escaping-refusal/overflow/short/error/clock/budget/debt controls; actual formatter and overflow allocations0, real Value-tree allocating mutant detected; all five cost phases reject independently, actual owned-string disposal precedes ending sample, failed main write is fenced before independently parseable numeric receipt. Four focused pure tests and strict Clippy exited0 before the final gate; final gate reran all37 integration tests. |
| Attribution/docs | ATTRIBUTION.md:194 credits ordinary record, stack sampling/formatting, finite models and private controls as project-owned with absolute policy URL. Existing project headers/canonical URL/license remain. README.md:230 explains exact ordinary record, full-width raw fields, unknown/clock semantics and no cost authority. |

## Measured final-run metrics and cutoffs

71 original parent native-birth rows and71 bookkeeping receipts, all actual settlements true. All original CPU/wall/unknown predicates false; maximum original birth CPU967584ns and wall2177584ns. Existing negative controls may deliberately alter applied-guard samples; original observations are separately retained. Numeric core bookkeeping maxima: format25583ns, one-write85208ns, owned disposal13917ns; zero unknown/overbudget core costs.

Receipt cutoff is explicitly after_birth_write_and_disposal. It contains those completed numeric start/end/CPU/owed/limit/overflow observations and exact requested/returned/errno output effect. Later receipt formatting/write are still independently measured, checked<=1ms and paid checked9x debt. Successful tail ending samples are outside the emitted receipt cutoff; they are not represented as zero. Any tail refusal returns its exact optional numeric samples in the typed failing-test outcome. External already-failing test-harness error reporting is explicitly unmeasured; there is no recursive receipt-for-receipt or claim of self-measuring final bytes.

This final gate's private Gate API ordinary witness PID41660: entry→waiting before permit, prepermit21625ns, max control10959ns, total24584ns, one permit read. Actual2ms observed-overbudget refusal and skipped-observation wrong-admission control both remained causal. Full raw traces/PIDs/counters are retained; raw kernel values supply no phase-cause proof.

Four positive multiMiB workers: heap peaks863568–864144B, RSS10305536–10321920B, max inclusive/application units309667–552000ns, max cleanup58250ns, max control67042ns, remaining debt0 and actual reap for each. These are this run's fixture observations, not universal OS latency or whole-process resource theorems.

Max ordinary entry211B INCLUDING LF in256B storage. Actual object layout: Gate192, Token52, Permit112, Line264, Usage32, Entry72, stat144, timespec16, V4raw296B. bootstrap.rs:500 retains legacy aggregate892B; :516 explicitly counts live metadata/parameters/Entry/Line/raw scratch: sample840B, format708B, within original declared1KiB state/2MiB protected reserve.

Exact ordinary351 frame reservations remain Gate::wait304 + waiting_record400 + Entry::sample352=1056B, plus early_worker_entry608 and unmeasured external frames. Source-owned object bounds do NOT prove cumulative frames<1KiB or whole-process stack; no cap/waiver/total-stack theorem is asserted. Root reviewed this framing in its task dispositions. Raw ordinary prologues/compressed disassembly remain retained.

New parent diagnostic buffer2048B is separate test-controller storage after actual settlement. Conservative independent maximum-field formatter tests yield birth878B/receipt1755B including LF, allocator0 in both successful and overflow paths; fixed alphabets/labels reject invalid escaping input before output. Partial/error output receives no retry. Numeric receipt retains short/error effects; a partial main row gets LF separation inside the same bounded receipt attempt.

## Exact source/compiler/artifact profile

Make prepared ordinary worker SHA3511aa485d47535ed2607c9083a587603dea3fb0b90e0bf6a9d471d67eda280d, release/default+acp-agent, NO test-fixtures, dream-bootstrap/1. Source/compiler/hash/support receipts were captured BEFORE managed worker births; final profile binds source2ce. Compiler rustc1.99.0 b940084d7eb6a299eb4bfeb8e34901bc051e7ac4, LLVM23.1.1, aarch64-apple-darwin. Original and final feature/build paths remained Make-owned.

Production/source SHA256:

- src/main.rs: 933e39cb206c63d5a3002d41bdaddb4603cee46edf47ede032ff974f71ba0ca9
- src/dream/mod.rs: e796f566f4cbf90a8416a4a8898a115dae60db42fe62321e61cd40683465880b
- src/dream/c1.rs: d044a795700ab346569a2ab4e82b2345c87e805a50d4fc2e7f3d7b28bb6f202a
- src/dream/bootstrap.rs: f6183cfa1f3eea408a853e171f44d456e5e8bf9c177fc088cc184634642aab64
- tests/dream_c1.rs: 0695614cd78902dee1078a8e4d6801399124b876697ec96c8414990cdff5a2e1
- fixed permit vectors: de919016ac6e5a464de60c86b2bf3bf6b54edc08e29c05f89688a5c80da37307
- Cargo.toml: 8a89f52077beeda99b07d12a505c374f5d1fa97dc9df5d5715866330985b897c
- Cargo.lock: 18b9e96af1b5aa1bc0b6693c1557daca9195ee80d062d22be48b843abdbf8ac9

Fresh deliberately rejected private producer fixture SHAd21151657a595aa99ff66fa1142493e778df24cb3eae90476d5b390e97173281 uses test-fixtures. Private Gate test-executable and pure formatter probes are distinct again; the pure-test binary SHAfbf15a63cc721ee7f6c3075a39e2b9cbd29cd095e8cec87d35b7546e867caceb and compiler/source receipt are in writer-correction-evidence.json. Its global allocation probe exists only in the private integration test executable; ordinary worker bytes remained unchanged.

Offline otool -l/-L/-tvV and dyld_info -imports/-inits exit0 comparison against retained d06/414be210e45ea228af5d85ee38f484ef7ed5918603a27d718cf9691d6aeaae81: libraries/initializer set unchanged; new _proc_pid_rusage/_mach_absolute_time imports use existing libSystem. Exact load commands, addresses/UUID/layout differences and entry prologues are retained. No startup equivalence/cause claim follows. rust-objcopy warned SIGABRT/missing @rpath/libLLVM.dylib; builds still exited0 and exact bytes are retained; no toolchain workaround was introduced.

## Verification commands and full formal evidence

Final outer command exited0 and canonical20-command receipt records every argv/cwd/exit:

- check-embedded-build0; license/embedded lint0; all-target/fixture strict Clippy0.
- affected library116 passed/1 skipped; C1 integration37 passed/0 failed/0 filtered.
- native model/nonvacuity/projection15+43+1 passed; Python formal controls9 passed.
- full275 native/TLC graph-and-verdict parity0; final current-source check-formal-evidence0.
- git push origin HEAD:main0; later ls-remote0 and git diff --check0.
The preliminary non-required check-formal-evidence Make exit2 was an expected absent-current-receipt cache miss before fresh full parity. It was followed by actual parity and final validation0, not reused as proof.

Current archive verification/evidence/f7fc307ce5fcd01de6a4422f83fd3769a86250b1e573ff1da07fe37a1085ffc3/evidence.json has SHA f7fc307ce5fcd01de6a4422f83fd3769a86250b1e573ff1da07fe37a1085ffc3, source2ce, dirtyfalse,275 models/76 contracts/1 crate. Every TLA/source/model/tool/input hash and contract metadata is retained with full graphs. complete_tlc_replacement=false and implementation_refinement_proven=false.

Entry model states/edges: correct11/10 pass; late17/16 safety; allocation11/10 safety; duplicate20/21 safety; noobserve12/11 safety. Four intended controls are real reachable violations, not TypeOK checks. TLC1.7.4 SHA936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88; Java21.0.13+9 archive SHA225bab0a09491ef06835488cd942f5d2befa9a0410a2484f0c8bbcf7506ec09a. Finite parity is not arbitrary Rust correctness or complete TLC compatibility.

Pre-gate process-free command:
`RUSTC_WRAPPER=/usr/bin/nice CARGO_BUILD_JOBS=2 cargo test --offline --locked --release --jobs 2 --target-dir target --features test-fixtures --test dream_c1 native_birth_diagnostic_ -- --test-threads=1 --nocapture` exit0,4 passed/33 filtered, ZERO managed worker births. Strict fixture-target Clippy --no-deps -D warnings exit0. The initial new-writer Clippy101 was write_with_newline; corrected before signing. Earlier pure tests passed before the partial-write fence control was added; final pure tests reran0. Initial full-repository fmt --check failed on pre-existing unrelated formatting, with no broad edits; only the diagnostic region was formatted. All actual earlier failures remain in command ledgers/logs.

## Earlier failures and unknowns remain unchanged

- d027 required command exit2:116 library pass/1skip, C1 integration30/33; three original native-birth refusals had missing CPU/wall numbers. WHICH predicate and its cause remain UNKNOWN.
- ONE signed9165 focused diagnostic exit0 remained diagnostic_only=true/runtime_conformance=false: PID40851 CPU303916ns/wall1371917ns exit0 and PID41035 CPU339625ns/wall1036083ns exit1, exact wait4 settled. It never established the cause of d027.
- First full9165 gate exit2:28/34 integration; all65 original births finite/admitted/settled, but six post-settlement monolithic diagnostics exceeded1ms and omitted their exact unit CPU. New bounded writer corrects that observed controller design/evidence defect. The successful2ce run does not prove old timing causes were repaired.
- Original395/82d and c083/16a chains failed, with original ENOSPC/CPU/wall logs preserved. Historical ed3 source-bound266-model/74-contract parity recovery stays historical. Old12ba/16a launches were never retried here.

This is source landing and measured producer conformance of the final run ONLY. Canonical term1.99.1/new-hash pre-birth approval, copied-worker joined250ms term gate, exact-running std1.99.1/XNU semantics and total external stack remain outside this execution. No term profile/pin, production install/release or account/credential action. Old artifact approvals do not carry over. Raw auxiliary counters remain UNVERIFIED.

## Retained evidence and native report custody

Final raw directory /private/tmp/dream-entry-c1-landing-2026-11-06:

- verify-land-3.log256311B SHA01abf522d7e11d954b69d5021730c04b7240a5304d37c1bda2dd993bb87803fa.
- verify-land-3-receipt.json and canonical-land-3-receipt.json: source/command/PID/times/actual exits/push.
- verify-land-3-births.json, verify-land-3-bookkeeping.json, verify-land-3-c1-summary.json and gate3-four-worker-metrics.json.
- profile-2ce-gate3 plus target/dream-c1-native-profile: before-birth ordinary source/compiler/hash/support receipts.
- formal-evidence-f7fc.json plus full current archive/target/harness-landing/formal graphs.
- writer-correction-evidence.json and writer-{clippy,pure-tests}-*.log preserve exact pure commands/exits/source/compiler/test-artifact hashes.
- verify-land-1/2 logs/receipts/rows, diagnostic-only9165 log/profile/rows and old-profile-ed3-pre-rebase remain untouched.
Older complete evidence remains /private/tmp/dream-entry-c1.rkP9l5 outside Git. A read-only final aggregation initially mistook the nested birth object in a bookkeeping receipt for a separate outer event; corrected to parse the first outer event, with no worker/test rerun.

Before current formal generation,2662 task-owned old cache files222081922B were byte-verified identical to retained ed3 archive and removed ONLY from unused target/formal/models to reduce duplicate storage. Full hashes/manifests and every exact original byte remain in the immutable ed3 archive; active canonical target was target/harness-landing/formal. No peer/worktree cleanup or source evidence was removed; no current graph was compressed or checker changed.

Native artifact_claim for the corrected test file was rejected "artifact digest conflicts with observed revision" despite Git/source byte verification; no claim effect is asserted. Operation01M6RRWWXXSHZTHT30B13PTHW1 is retained; an attempted digest-format correction was explicitly rejected. Original unknown native operations were queried by original IDs, found unknown/ledger-missing/retry_allowed=false and not replayed.

knowledge_commit is still absent from this client catalogue. This new report is read back as a draft; the task explicitly assigns Root checkpointing when that tool is unavailable. No shell bypass, .bboxy force-add, report mutation or acceptance claim. Executor task_report carries this source/evidence handoff for Root verification.
