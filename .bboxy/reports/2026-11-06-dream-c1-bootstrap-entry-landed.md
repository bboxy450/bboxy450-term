---
type: report
title: Dream entry lossless-controller gate failed; source not landed
author: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
created: 2026-11-06T15:28:15.747Z
task: '[[tasks/2026-11-05-dream-c1-bootstrap-entry]]'
supersedes: '[[reports/2026-11-05-dream-c1-bootstrap-entry]]'
commits:
- 94e7e7af75b951c50a8659fe29a533d9a2b2a60a
- fe2bf755c96a162ede58f450d4d2e5331c0c1d42
- d027a16b33aa6a4d78797f9eaf93901f762e28bd
- 9165b7d50818d11d11eb6a0d9fb753f8c54d805e
verify:
  command: make check-embedded-build && make land
  exit_code: 2
id: 01M6RQK24P9M56H78J3ZRX1VM2
updated: 2026-11-06T15:28:15.747Z
number: 93377961366020
created_by: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
updated_by: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
authorship:
  signer: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
  kind: seat
  public_key: aaB9vpN0zTwKyfaUMhUFUSvyTZ6x1t9lQBaEEuGygFk=
  process_generation: '1'
provenance:
  digest: sha256:b23c58e436b34a6ff1e1c8b81eb9b586bcda1c5683949d3b1da1f1d5dbf6a317
  frontmatter_digest: sha256:c5c8fbee8efe0189c5737d98e1872bf9e03638e08b640d5410d58486f1a7ad91
  signer: seat:01M6QDKWCA62KCAKG3P2Z7ZVCD
  sig: w0YBcYKZDIKZMOFTEUShmudUtrdohjkV7pD/nOPAWAH3Sb7qLAMqCblTSLR2ifdDBY5PwPutf+/bM8d+wIHODw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149505
  event:
    id: 01M6RQK3GBPAC5K7AEM2HSAC8T
    sequence: 149505
    hash: 9aef33f95a2f1c090213f8949e2b5416d2a329e4be4f6a3aecd68af951a6a2b3
---
# Lossless-controller canonical gate failed; no landing

Task [[tasks/2026-11-05-dream-c1-bootstrap-entry]], current amendment digest56b5870e777bb92ca7c77297cdecf93ab703677459744c03650e92d30d27d217, verify revision1. This append-only report supersedes [[reports/2026-11-05-dream-c1-bootstrap-entry]] as the current handoff. That earlier full report remains immutable and retains per-criterion implementation, original failures, full source/model/tool hashes and artifact/frame evidence.

## Verdict and exact command

ONE newly authorized complete command ran on signed9165b7d50818d11d11eb6a0d9fb753f8c54d805e:
`make check-embedded-build && make land` — outer EXIT2.

check-embedded-build passed0. Canonical signed rebase left HEAD9165 unchanged on origin/main a75eabfa4c19ab1af2cd192809d75fe510dff1e3. Signatures, license controls, embedded lints, all-target/fixture Clippy passed; library116 passed/1 skipped. Full C1 integration ran all34 tests:28 passed/6 failed. Cargo101/integration Make2/land.py77/outer Make2. Formal stage NOT reached, push NOT attempted. No retry, warm-up, focused probe, fallback or hidden positive rerun followed.

All SIX failures were the new post-settlement diagnostic bookkeeping CPU predicate >1000000ns at tests/dream_c1.rs:434, with error "native diagnostic bookkeeping exceeds1ms". The exact bookkeeping CPU numbers were NOT logged; only their failing bound is known. This is a NEW source-bound failure, not a native-birth CPU/wall refusal and not evidence about d027's prior failure cause.

Native-birth diagnostic rows retained65 original observations, all optional CPU samples known, all original CPU/wall/unknown predicates false, all65 actual settlements true. Maximum original native CPU1541667ns, maximum original native wall5897083ns. Applied-guard observations may intentionally differ in existing injected negative controls; original observations remain separately immutable.

## Failed test, original effect and actual settlement

Each row below immediately precedes the corresponding diagnostic-cost refusal. All use exact ordinary3511aa485d47535ed2607c9083a587603dea3fb0b90e0bf6a9d471d67eda280d and original native guard admission; none has an unknown original CPU or over-budget original CPU/wall predicate.

| Failed test | PID / run | Original native CPU ns | Original native wall ns | Actual settlement |
|---|---|---:|---:|---|
|child_declared_storage_overflow_precedes_serde_and_controls_still_settle|91470 / 01M6RQ8MM72JZRHXS0M5J19RTT|537708|1324166|exactPIDwait4, exit1|
|foreground_pause_stale_activity_cancel_eof_and_deadline_settle|92049 / 01M6RQ8XJ9ZQ88PNPD2RKKJY5D|893000|1963208|owned settlement + cached Child status, exit0|
|four_children_keep_independent_controls_and_individual_memory_shares|92071 / 01M6RQ8XPDJWKHY5MMKRW0WGQZ|1215834|3139292|owned settlement + cached Child status, exit0|
|multiple_pages_preserve_exact_counts_and_order|96129 / 01M6RQAV6WMFBXJXZQ01PVAZD5|329250|1040792|exactPIDwait4, exit0|
|partial_frame_eof_wrong_nonce_and_oversize_settle_as_protocol_failure|97614 / 01M6RQBG7M3AFPDP4QQSG634PN|854666|2750500|owned settlement + cached Child status, exit2|
|source_blob_sequence_version_and_custody_controls|99634 / 01M6RQC91E8RGMSBM5WR8ADGME|569500|1177667|exactPIDwait4, exit1|

The measurement at tests/dream_c1.rs:393 encloses JSON row construction, serialization/output and allocation cleanup, followed by finite ending sample, actual paid rest, and unchanged1ms control refusal. The source uses allocated serde_json::Value and direct streamed serialization to stderr. The measured span exceeded its bound; no subcomponent timing or syscall-cause attribution is established.

The concrete correction to disposition is bounded serialization of the same retained row into fixed storage with one output write, avoiding allocated Value/direct small writes, while retaining finite numeric bookkeeping cost on refusal. This is a source-grounded proposal, NOT implemented or tested by another worker birth. No threshold, native class or authority change is proposed.

## Preserved implementation and exercised controls

The source remains the same ordinary entry implementation:
- src/dream/bootstrap.rs:376/383/428/460 — exact fixed-key ordinary entry sample/format/write, before unchanged waiting; e/m primary monotonic chain, all s/g/u/k/r/p raw UNVERIFIED auxiliary.
- bootstrap.rs:416 — rusage failure clears only s/u/k/r/p; clock/pipe failures still refuse.
- bootstrap.rs:605/624 — complete record remains inside sole original prepermit usage observation; permit128B, lease, paid debt, readiness and owned admission law unchanged.
- bootstrap.rs:909/943 — max-u64 exact211B including LF, allocation0 with real allocation control, exact failure bytes and finite clock/single-charge checks.
- tests/dream_c1.rs:610/2809 — actual entry order/clock/process checks; late, duplicate, noobserve, EOF, pipe and actual2ms overbudget controls retained.
- tests/dream_c1.rs:2729 — real prepermit allocation mutant still rejected specifically for controlled admission after valid entry validation, actual PID90324 reaped. Fresh private test-fixture artifact SHA d21151657a595aa99ff66fa1142493e778df24cb3eae90476d5b390e97173281; it is not the ordinary351 worker.
- tests/dream_c1.rs:2629 — four real ordinary multiMiB workers/200 causal PRIMED roundtrips, independent control/debt/cleanup/reap gate passed within this otherwise FAILED full run; p99=38us, max=49us, raw job rows retained. This does not turn the full gate green.
- tests/formal/dream_c1.rs:1298/1350, tests/formal/models.rs:6887 and maps — correct entry plus four intended causal controls; all upstream270 models/75 contracts retained verbatim, now275/76. Current full parity remains unproven because gate stopped before formal.
- ATTRIBUTION.md:194 — the task adds five lines crediting ordinary entry, stack-only raw sampling/formatting, finite models and private causal controls as project-owned, retaining the absolute governing policy URL. README documents ordinary record/unknown semantics. No Makefile/scripts/land.py edits.

Current ordinary full-run private Gate process witness PID88818 measured prepermit32625ns/control max21500ns/total41625ns, entry→waiting before permit. EOF88819 refused Revoked; invalid stderr88821 refused Pipe; actual2ms overbudget88822 refused Limit; late88824 and duplicate88826 caused intended trace rejection; skipped observation88827 wrongly admitted with reported prepermit13250ns. Exact bytes, effects and settlements retained in the full log. These are test-executable witnesses, distinct from ordinary worker readiness.

Storage/max line unchanged:211B including LF in256B buffer; Gate192/Token52/Permit112/Line264/Usage32/Entry72/stat144/timespec16/V4raw296. Source-owned sample max840B/format708B, legacy892B aggregate assertion. Exact ordinary frame reservations: Gate::wait304 + waiting_record400 + Entry::sample352=1056B, plus early_worker_entry608 and unknown external frames. Source object bounds do not prove cumulative frames<1KiB; no cap/waiver/total-stack theorem is claimed. Existing2MiB reserve unchanged; Root reviewed framing before authorizing gates.

## Source/artifact/profile binding

Current signed cut:94e7e7af75b951c50a8659fe29a533d9a2b2a60a, fe2bf755c96a162ede58f450d4d2e5331c0c1d42, d027a16b33aa6a4d78797f9eaf93901f762e28bd, 9165b7d50818d11d11eb6a0d9fb753f8c54d805e. All G, Boxy <bboxy450@gmail.com>, key9B07E849480C68328CA20684844ECC0587DE7AB0. HEAD/source unchanged during gate; worktree clean; git diff --check exit0. origin/main remains a75, so own source is NOT landed.

Make prepared ordinary SHA3511aa485d47535ed2607c9083a587603dea3fb0b90e0bf6a9d471d67eda280d before births, release/default+acp-agent, NO test-fixtures, dream-bootstrap/1; rustc1.99.0 b940084d7eb6a299eb4bfeb8e34901bc051e7ac4, LLVM23.1.1, aarch64-apple-darwin. Exact before-birth source/compiler/feature/hash/support receipts preserved at failed-profile-9165-351; production bootstrap SHA f6183cfa1f3eea408a853e171f44d456e5e8bf9c177fc088cc184634642aab64, test source SHA4c5ef3c852c8a7876b1503ea59e5b1737a8c23029b3bc5d729b945689d34ee5d. All remaining source/lock hashes appear in that profile and the preceding full report. Diagnostic-only flags/filter/expected-hash environment were explicitly absent for this complete gate; controller nice0, compiler nice10, jobs2, testthreads2; no global configuration changed.

Exact same351 bytes retain prior offline Mach-O comparison to d06/414 baseline: unchanged libraries/initializer set, new raw sampling imports via existing libSystem, changed addresses/UUID/layout. No startup-cause/equivalence inference. Prior report retains max stack overlap, full commands, archived model/tool/input hashes and original artifact provenance. rust-objcopy still warned SIGABRT/missing @rpath/libLLVM.dylib; actual compiler/build gates exited0, no toolchain repair was performed.

## Historical failures stay failed

d027 full required command exit2 remains,30/33 integration and THREE original native admission refusals with missing CPU/wall numbers. Cause UNKNOWN. The ONE earlier signed9165 focused diagnostic exit0 remains diagnostic_only=true/runtime_conformance=false, PID40851 CPU303916ns/wall1371917ns exit0 and PID41035 CPU339625ns/wall1036083ns exit1, both exact wait4 settled. Its success never repaired or explained d027.

Older395/82d and c083/16a required chains failed, with retained ENOSPC/CPU/wall evidence. Historical ed3 full266-model/74-contract native/TLC parity and recovered source-bound receipt remain separate evidence, not current275/76 proof. Old12ba/16a launches remain stopped; no term canonical1.99.1/250ms probe/profile approval or universal native scheduling claim.

## Raw retained evidence and exact next decision

Directory /private/tmp/dream-entry-c1-landing-2026-11-06:
- verify-land-2.log126799B, SHA1fdbcd9614026cc5b58153e54cafe66168c78e366583bc9e1669b1b3bf855e77.
- verify-land-2-receipt.json: exact outer command/start/end/source/PID/exit.
- canonical-land-2-receipt.json: actual canonical commands and their exits.
- verify-land-2-births.json: all65 raw original/applied-guard/PID/run/hash/permit/settlement rows.
- blocked-gate-9165.json: six failed tests, preceding numeric original rows, explicit unknown exact bookkeeping cost, no retry authority.
- failed-profile-9165-351: before-birth ordinary profile/source/compiler/hash receipts.
- previous failed logs, one diagnostic-only log/receipt/rows/profile, old byte-bound artifacts, offline load/import/init/frame/asm data and SHA manifest remain untouched. Earlier raw evidence remains /private/tmp/dream-entry-c1.rkP9l5 outside Git.

Required verify remains FAILED; no task completion/report-as-done, source push, cleanup or further positive run. Decision needed from delegating Root: disposition/authorize the scoped bounded parent diagnostic writer and numeric bookkeeping-refusal evidence, then define the next materially changed gate. All source/native/CPU/wall/control/heap/entry/lease/debt/reap thresholds remain mandatory. knowledge_commit is not exposed by this client; this report is a read-back draft until Root checkpoints it through its available native tool.
