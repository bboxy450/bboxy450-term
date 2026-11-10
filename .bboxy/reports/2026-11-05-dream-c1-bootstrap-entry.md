---
type: report
title: Dream ordinary entry implementation and retained failed landing gate
author: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
created: 2026-11-06T15:16:46.996Z
task: '[[tasks/2026-11-05-dream-c1-bootstrap-entry]]'
commits:
- 94e7e7af75b951c50a8659fe29a533d9a2b2a60a
- fe2bf755c96a162ede58f450d4d2e5331c0c1d42
- d027a16b33aa6a4d78797f9eaf93901f762e28bd
- 9165b7d50818d11d11eb6a0d9fb753f8c54d805e
verify:
  command: make check-embedded-build && make land
  exit_code: 2
id: 01M6RPY23JVP0KJSRMY7RJ3B1A
updated: 2026-11-06T15:16:46.996Z
number: 93377961366018
created_by: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
updated_by: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
authorship:
  signer: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
  kind: seat
  public_key: aaB9vpN0zTwKyfaUMhUFUSvyTZ6x1t9lQBaEEuGygFk=
  process_generation: '1'
provenance:
  digest: sha256:a17dff5480e9f4def7990fc287961ac002a182b35e848ec0a018ab3d901663a3
  frontmatter_digest: sha256:605272f8eb09eaf19c7b1ba0b495cf5787db4dc0695350c87248e7815fbab5c6
  signer: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
  sig: pOOQMVsUx0kHts0wTqnZ6WX1Q7ZPqVJdhTQx13DC1aft77+0ncEun+z0NnajBknpxTINeqUKqZZZKPQ2akvYAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149500
  event:
    id: 01M6RPY2W5GQB0JFFNRA75RS4N
    sequence: 149500
    hash: f19d19a26c0b67b82b504aaebcd7d2159b3aca2261cc2e1985d9afab43ff2934
---
# Ordinary entry implementation; required landing gate failed

Task: [[tasks/2026-11-05-dream-c1-bootstrap-entry]]. Current authenticated brief digest cfd34fde2ebde83903705aa72b8228671fe4a37d998b2c092d464c651096c2b2, verify revision 1. This report records the scoped implementation and the single authorized changed-controller diagnostic. It is NOT task completion, source landing, runtime conformance, or term-profile approval.

## Source and custody

Exclusive tree /Users/feral/mydev/bboxy450-harness-dream-entry-c1, branch feat/dream-entry-c1. Signed rebase onto a75eabfa4c19ab1af2cd192809d75fe510dff1e3 preserved upstream. Current signed commits, all G / Boxy <bboxy450@gmail.com> / 9B07E849480C68328CA20684844ECC0587DE7AB0:

- 94e7e7af75b951c50a8659fe29a533d9a2b2a60a: ordinary entry, unit/process controls, model/maps/docs.
- fe2bf755c96a162ede58f450d4d2e5331c0c1d42: separate record scratch from permit decoding.
- d027a16b33aa6a4d78797f9eaf93901f762e28bd: require allocation mutant rejection for actual admission after valid entry trace.
- 9165b7d50818d11d11eb6a0d9fb753f8c54d805e: parent native-birth numeric/effect diagnostics and process-free injected boundaries.

Original signed 39514ce7f7025a051443b414bcd9b28f8f154d58 / c0830533ede0e90cf3061ffe0251b22d1c1c74bc / ed3d4ffc8ae9bb3aa5b5afffbcd2ebcd0bd3073a remain historical evidence bindings. No changes to Makefile/scripts/land.py. Own changes are bootstrap.rs, dream_c1 integration/formal source, model registration, contracts/corpus/coverage and README. Existing ATTRIBUTION.md already names bboxy450-harness, canonical GitHub URL, governing license and reuse policy; source headers retain it.

No push or landing occurred. Last required command failed before push. Worktree was clean after 9165; git diff --check exit0. Root owns eventual worktree/seat cleanup.

## Acceptance evidence

| Criterion | Source mechanism and evidence |
|---|---|
| Exact ordinary entry before unchanged waiting | src/dream/bootstrap.rs:376 Entry holds eight u64 plus status; :383 samples existing e, monotonic m, stack proc_pid_rusage V4, then mach_absolute_time; :428 unrolled fixed-order formatter; :460 waiting_record reuses one Line, emits entry then original waiting. Gate::wait :582 calls it before read_permit. No ordinary feature gate. |
| Raw failure status | bootstrap.rs:416 rusage_status sets ok iff status==0 and clears only s/u/k/r/p on failure. e/m/g remain. No raw counter conversion, subtraction, timing classification or admission authority. CLOCK failures remain errors; non-Darwin sample is unknown. |
| Fixed bounded nonallocating writer | bootstrap.rs:313 Line owns fixed256-byte bytes plus len; borrowed send appends LF, validates original stderr pipe/nonblocking ownership, and one write must equal full length. No allocator/extraFD/thread/public field. :909 exact max-u64 full211-byte line, exact failure bytes, real allocation counter0 plus actual Vec mutant>0. |
| Charged complete prelude | bootstrap.rs:605 noninlined read_permit begins with sole usage.observe(prelude_at) :624 before any permit read; this includes record sampling/formatting/fd checks/both writes. :943 finite clock/single accounting/overflow/control-budget unit. Existing1ms limit and permit128B/lease/debt/clocks remain. |
| Real order/CPU/failure witnesses | tests/dream_c1.rs:610 strict key/order/status/clock trace checker; e<parent spawn return is accepted, e<=m<=observation required. :2809 actual private Gate process fixture covers ordinary/EOF/pipe/2ms overbudget/late/duplicate/noobserve; :2964 grant only where expected; :3037 noobserve actually admits hidden2ms while observed refuses. :2729 actual private allocation mutant validates entry first, then requires controlled-admission rejection; exact PID reaped. |
| Model and causal controls | tests/formal/dream_c1.rs:1298 explicit phases exec/entry/emitted/waiting/read/Hello/refused, finite counters/initials/owners/actions; :1350 demands correct progress/refusal and intended late/allocation/duplicate/noobserve violations. models.rs:6887 registers all5; contracts.toml:868 states bounds/abstractions; coverage.toml:160 and corpus map them. No checker/cap/control removed. |
| Preserve existing regressions/inventory | Existing four-ordinary-worker/multiMiB/200 actual PRIMED RTT/debt/cleanup/reap gate remains tests/dream_c1.rs:2629; near-limit/retained teardown remain :2302/:2414. Rebased inventory275 models/76 contracts contains all upstream270/75 records verbatim plus5/1; merged-inventory.json checks this. Current full gate remains red, so retained source is not a current full-regression pass. |
| Permanent parent refusal observations | tests/dream_c1.rs:330 finite optional interval; :338 NativeBirthObservation separates CPU>2000000ns, wall>25000000ns and unknown; :365 immutable PID/run/hash/original samples; :379 row records original and actually applied guard, permit attempt and actual settlement; :402 formatting/output/allocation cleanup occur after settlement inside a separate paid CPU span. :436 injected exact boundaries and omitted CPU/wall/unknown-zero controls execute no worker. :768 spawn Result and ending samples captured before any error formatting. Option moves Held→Worker, emitted once after exact wait4 or existing owned settlement. No threshold/native class/clock-origin/spawn/debt/lease/reap weakening. |

## Line, storage and compiler-frame evidence

Max line211B INCLUDING LF, eight unpadded maximum20-digit u64 fields plus ok1; fixed buffer256B. Layout measurements: Gate192, Token52, Permit112, Line264, Usage32, Entry72, stat144, timespec16, V4 raw296 bytes. bootstrap.rs:500 retains legacy aggregate892B assertion. :516 source-owned phase overlap explicitly counts metadata/parameters, Entry, Line, raw V4 and clock scratch; max sample840B / format708B. Record/sample/send return before permit/decode storage exists.

These source-owned objects are NOT a claim that cumulative machine frames fit1KiB. Exact ordinary351 disassembly reserves Gate::wait304 + waiting_record400 + Entry::sample352 =1056B at the sampling call; early_worker_entry adds608B; external callees are excluded. Other frames: Line::send192, read_permit688. Root inspected this framing before its landing amendment. No new cap/waiver or total-stack theorem is asserted. The existing protected2MiB reserve is unchanged. Raw prologues/full compressed disassembly and frame-sizes-351.json are retained. The earlier16a had the same sample-chain reservations. No unmeasured raw-kernel counter is used as CPU proof.

Historical private Gate witness at unchanged bootstrap f618: ordinary PID4849 entry→waiting before grant, measured prepermit31166ns/control max25458ns/total35499ns. EOF PID4850 refused Revoked; stderr failure PID4852 refused Pipe; actual2ms burn PID4853 refused Limit; late PID4855 and duplicate PID4856 were causally rejected by strict checker; skipped observation PID4858 wrongly admitted with recorded prepermit4625ns. These are private test-executable witnesses, not ordinary-worker readiness or current rebased full-gate success. final-entry-controls.log and final-allocation-causal-reason.log preserve exact bytes/PIDs/fixture hashes and exits.

## Exact ordinary artifact and private fixture distinction

Current ordinary SHA256 3511aa485d47535ed2607c9083a587603dea3fb0b90e0bf6a9d471d67eda280d, release/default+acp-agent, NO test-fixtures, dream-bootstrap/1. Compiler rustc1.99.0 b940084d7eb6a299eb4bfeb8e34901bc051e7ac4, LLVM23.1.1, aarch64-apple-darwin. Make owns ordinary prepare and subsequent fixture recipes. Source/compiler/hash/support receipts were captured before worker births at d027; unchanged production sources/compiler/exact351 bytes were checked before the sole9165 diagnostic. Make refreshed the profile with9165 before those births. Parent expected-hash guard refuses a changed artifact BEFORE birth.

Production source SHA256:
- src/main.rs 933e39cb206c63d5a3002d41bdaddb4603cee46edf47ede032ff974f71ba0ca9
- src/dream/mod.rs e796f566f4cbf90a8416a4a8898a115dae60db42fe62321e61cd40683465880b
- src/dream/c1.rs d044a795700ab346569a2ab4e82b2345c87e805a50d4fc2e7f3d7b28bb6f202a
- src/dream/bootstrap.rs f6183cfa1f3eea408a853e171f44d456e5e8bf9c177fc088cc184634642aab64
- vectors de919016ac6e5a464de60c86b2bf3bf6b54edc08e29c05f89688a5c80da37307
- Cargo.toml 8a89f52077beeda99b07d12a505c374f5d1fa97dc9df5d5715866330985b897c
- Cargo.lock 18b9e96af1b5aa1bc0b6693c1557daca9195ee80d062d22be48b843abdbf8ac9

9165 test source SHA4c5ef3c852c8a7876b1503ea59e5b1737a8c23029b3bc5d729b945689d34ee5d; d027 test source SHA f80e87065d6d61715da7a7be056510eab727be18fed961f1e16f7625e361f655. Private earlier actual-allocation mutant SHA7921decebc9dbed725d1a441e84c2e67d9c1b9d05c9035f164fd94f73cb5e9cd uses test-fixtures and remains rejected. Older82d/16a artifacts and profiles retained; no further execution of16a or prohibited12ba. No term1.99.1 artifact/profile gate was executed by this seat.

Offline otool -l/-L/-tvV, dyld_info -imports/-inits exit0. Compare retained d06/414be210e45ea228af5d85ee38f484ef7ed5918603a27d718cf9691d6aeaae81 baseline with exact new ordinary load commands/imports/init/prologue. Libraries and initializer set unchanged; new _proc_pid_rusage and _mach_absolute_time imports use existing libSystem. Addresses/UUID/layout differ. This is an offline dependency comparison, NOT a causal startup equivalence/latency claim. Exact logs/manifests retain differences.

## Verification: real results, including failures

Required CURRENT command: make check-embedded-build && make land, actual outer exit2 at d027. check-embedded-build exited0. Canonical land controller nice0/compiler nice10, jobs2/testthreads2, private test HOME/GNUPGHOME with public verification material. Licenses, embedded lints and all-target/fixture Clippy passed; library116 passed/1 skipped. Prepared C1 integration30 passed/3 failed at parent native-birth guard: immutable_results_are_never_clobbered; near_limit_rejected_envelope_drains_before_process_done_and_join; source_blob_sequence_version_and_custody_controls. Cargo101/integration Make2/land.py77/outer Make2. Numerical failing CPU/wall samples were absent, so WHICH predicate failed and its cause remain UNKNOWN. Current formal stage was NOT reached. No push. Log verify-land-1.log109352B SHA5f67520f0cbbb87adac45ac8e57c0e18955353d952ed46e227c9152b7229ef40; failed-land-receipt-d027.json and blocked-gate-d027.json preserve exact results and source digest d096bd87c04f6d9f6ef870617d246a748056392ea5764116966271eed9d15056.

Earlier original full chain make test-dream-c1 && make test-formal-parity && make check-embedded-build && make test-licenses FAILED at395/82d (23/9, wrapper1/Make2; ENOSPC possible log gap) and c083/16a (28/4, Make2). No identical retry or reinterpretation as success. Later narrow entry/allocation controls passed separately. Earlier native15+43+1 tests and8 Python controls passed; all266 original native/Java full reachable-state/labeled-edge/verdict checks matched, but archive twice failed ENOSPC. Lossless graph compression plus existing evidence/archive functions recovered source-bound ed3 receipt bdbfb2f1ebf1586e2a6031d9b69b52fc982b592d76d1a4ccc67dba20341b1d2f, dirtyfalse,266 models/74 contracts; existing make check-formal-evidence exit0 then. This is historical ed3 proof, NOT current275/76 parity. No timeout/TypeOK/cap/control waiver. TLC1.7.4 SHA936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88; Java21.0.13+9 archive SHA225bab0a09491ef06835488cd942f5d2befa9a0410a2484f0c8bbcf7506ec09a. Historical source/model/tool hashes remain in archived evidence.

9165 controller checks:
- cargo clippy --offline --locked --jobs2 --target-dir target --features acp-agent,test-fixtures --test dream_c1 --no-deps -- -D warnings: final exit0.
- cargo test --offline --locked --release --jobs2 --target-dir target --features test-fixtures --test dream_c1 native_birth_diagnostic_and_guard_injected_boundaries -- --exact --test-threads=1 --nocapture: final exit0,1 passed,33 filtered, ZERO workers.
- Initial diagnostic Clippy failed101 from a shadowed local binding; corrected before signing/actual diagnostic. Intermediate and final logs retained.
- cargo fmt --all -- --check exit1 from existing repository-wide formatting differences; changed new hunks had no reported differences. No unrelated formatting edits.
- rust-objcopy repeatedly warned SIGABRT missing @rpath/libLLVM.dylib; actual builds exited0. Artifact identity is retained; toolchain not changed.
- git diff --check exit0.

## ONE authorized changed-controller diagnostic

AFTER signing9165, exact make test-dream-c1-integration DREAM_C1_TEST_FILTER=immutable_results_are_never_clobbered ran ONCE with diagnostic_only=true/runtime_conformance=false and expected ordinary351 hash. Exit0,1 focused test,33 filtered; normal bounded fixture exactly TWO births, no extra test children. Both original observations match applied guard; both CPU/wall/unknown predicates false, permit attempted, actual exactPIDwait4 settlement:

| PID | Run | CPU start→end ns | Native CPU ns | Native wall ns | Actual exit |
|---|---|---|---|---|---|
|40851|01M6RPMF3BWQ9KV8NAQKXRGN2N|92992875→93296791|303916|1371917|0|
|41035|01M6RPMF3BWQ9KV8NAQKXRGN2N|94285541→94625166|339625|1036083|1|

Second exit1 is expected StorageUnavailable for immutable output; focused assertion passed. This diagnostic DOES NOT establish the earlier failing samples or cause, full regression conformance, current formal parity, landing or term approval. It authorizes no further ordinary run/warmup. Root must rule on source-grounded correction/native boundary before another positive gate. Essential numeric/effect/code handoff stored as message01M6RPNWC2SRBZVPVXMR75YPAS; storage is not proof of model consumption.

## Retention and remaining work

Raw logs remain outside Git:
- /private/tmp/dream-entry-c1.rkP9l5/: full old command-ledger.json, original failures, narrow controls, offline baseline/ordinary comparisons, native/tool/evidence recovery hashes.
- /private/tmp/dream-entry-c1-landing-2026-11-06/: failed canonical log/receipt, exact failed-profile-d027-351, old-profile-ed3-pre-rebase, current diagnostic-profile-9165, focused-diagnostic-9165.log/-receipt.json/-births.json, diagnostic-controller-checks.json, merged-inventory.json, load/import/init/prologue/asm data and SHA manifests.
Focused log6351B SHAde0fc8633b59412d0bd0b06544d225c26d1b0abff49d7324108c958a378c4e74.

Original unknown native operations were queried at their original IDs after transport recovery: unknown/ledger missing/retry_allowed=false; effects not replayed. Fresh report creation is separate source-bound work. knowledge_commit is advertised in orient but not exposed in the current client tools; no checkpoint is claimed. Root may checkpoint via its granted tool.

Unproven: current full275-model/76-contract parity and full native C1 gate; all three d027 failure causes; exact-running std1.99.1/XNU source and raw-kernel semantics; cumulative external stack; term canonical joined250ms readiness; source landing. No old artifact approval carries forward. No production pin/install/merge/account/credential action. The required verify remains FAILED, and task remains active.
