---
type: report
title: C1 native quantum, charged rest debt and immutable metric cutoff disposition
task: '[[tasks/2026-11-05-dream-c1-native-quantum-disposition]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T19:16:19.709Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6PJ7YYHE8F4F7AY8B7A0089
updated: 2026-11-05T19:16:19.709Z
number: 93377961365090
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:117e4ee52fa7908464c06310e89432433e7eb10653ef1d08c8bcaac4e2e47f0b
  frontmatter_digest: sha256:001f22a3b6b22a7ff15a474e2f2ad1cd3e9370ec04a7f9656d872d8e8c2e47ea
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: gflOUQSEni2UTTioWLE1ypllKZZVDNA8iJE7rIZaum5dbI4yidye3YMcIEmgeAjgRfjiHF9+bfd3dNaeRHB+Bg==
  log: .bboxy/log/2026-11-05.jsonl#140494
---
# C1 cooperative/native quantum disposition

Task: [[tasks/2026-11-05-dream-c1-native-quantum-disposition]] (01M6PHF28EXY6W5V9RDSFFJSVZ). Decision [[decisions/d93377961365086-dream-native-quantum-and-debt]] resolves the execution-budget interpretation; four inherited spec/interface paragraphs are synchronized. This is contract evidence, not a passing benchmark or component acceptance.

## Disposition and evidence classification

Keep application/hash/parser/serde/SQLite/stepped-cleanup units and shared reactor/established control polls≤1ms CPU. Only named isolated process creation, one compute-owner creation and bounded no-replace publication have observed2ms CPU/25ms wall allocations, finite counts and exact effect custody. This is not an OS-preemptive guarantee or a general native-wrapper exception.

Every expensive span charges full inclusive CPU and repays rest≥9*max(1ms,inclusive CPU) after its end before the next parent/child work admission. Controls never wait on that debt. Surrounding application CPU stays≤1ms, inclusive native span≤3ms; a2ms parser remains a failure. Existing closure/authority/slots/heap,30s deadline,100/250ms input/control/pause,2s independent job,240-frame and actual reap/escalation gates remain. No clock disable, native subtraction, lucky retry, new wire/schema or threshold change for UI/controls.

The actual failed sample was read independently: /tmp/dream-c1-prepared-four-real.log:152-175 shows12passes/3failures. The brief supplies wrapper exit100; this review did not rerun it or recover a separate exit sentinel. runtime-isolated-four-heap.json has four settled/reaped failures and zero completed jobs:

| Job | Observed CPU / result | Causal disposition |
|---|---|---|
| 0 | spawn_prepared1,089,208ns; whole1,093,000ns; ResourceLimit | Separate exact Command::spawn from validate/build/take-handles/nonblocking work before classifying native cost. The whole function is not indivisible. |
| 1 | producer whole control tick1,316,083ns; ResourceLimit | No exception yet. Instrument subspans and move/preprovision the sole compute creation before established polling. The actual cause remains unproven; other control application work still must fit1ms. |
| 2 | configure_existing1,135,292ns; whole1,138,750ns; ResourceLimit | Split busy_timeout and the two PRAGMAs. No aggregate SQLite exemption. |
| 3 | one Publication::link1,510,916ns; final StorageUnavailable | Exact native link is eligible for the new allocation; its original effect/typed outcome must survive any timing failure. StorageUnavailable does not establish absence of a published link. |

Parent reactor maxima0/175083/109417/58750ns and low recorded heap sums do not turn these refusals into a qualifying four-job pass. The two separate parser cleanup failures remain failures and receive no native exception. Native wall ceilings and corrected final-source conformance remain unmeasured here.

## Source-grounded changes required

Initial consumer snapshot19:03:39 UTC: HEADd156e0cbae84aa5a9af768220d8200cf62c200de plus active author changes.

- logic/dream_c1.rs::publish_paced:2952-2969 already splits create/write/sync/metadata/link/check/unlink/dirsync. Publication::link:2923-2924 calls one hard_link; retain exact inode/op state even when budget fails.
- Journal::configure_existing, registry/dream_c1.rs:231-238 aggregates busy_timeout and execute_batch(cache_size,mmap_size); checkpoint_stage logic:3066-3090 times that aggregate. Add bounded split calls and retain genuine errors/side effects.
- wake-up Worker::spawn_prepared:1224-1283 validates, builds command/pre_exec, spawns:1260, takes streams and changes descriptor flags. Split preparation, native call and post-setup; preserve protected buffers/priority/group/actual child ownership on timing failure. Parent execute_inner:4088-4101 already retains and settles a returned child after a budget verdict; do not lose that property.
- Clean embedded producer47182320 c1.rs:55-87 runs directly before the async runtime. Its tick:1805-1999 includes framing, allocation, thread creation:1854 and join. Instrument/classify this real path; the optional async wrapper is not the actual macOS execution proof. Fresh producer correction assignment is required; accepted task stays frozen.
- Oracle amends active Q2 to inherit the same law; this seat did not edit its source or task. No general scheduler/runtime/OS-limit project is needed.

The decision specifies C1QuantumDebt finite variables/actions/invariants/fairness, causal native-label/rest/effect/early-exit controls and real private API/process witnesses under existing make test-dream-c1/formal targets. Actual four-job successful completion, independent controls, real ordinary-release binary, unchanged corpus/heap/native frames/authority and source-bound formal evidence are still required. No formal/native test ran for this assignment.

## Immutable metric cutoff correction

Per Oracle review message01M6PJ33390F55ZSEQ66KXYQG1, decision and C1 ICD explicitly pin max_poll_cpu_ns to its field-capture cutoff **before its own final descriptor serialization/hash/write/publication**. It cannot predict that future cost. Terminal disaggregated+inclusive evidence covers descriptor emission/publication and all later cleanup/control/settlement, validates captured maximum≤terminal inclusive maximum, and retains real exit/reap/full process usage with honest telemetry cutoffs. No clamping, subtraction, self-referential resealing or wire bump. A scalar≤3ms is never sufficient without exact native/application/control/profile evidence; old1ms-only consumers may refuse safely.

Oracle received the disposition at stored message01M6PJ0VFC3E2X506R2MYJT0SS and stated the core is accepted for implementation while requesting this cutoff correction. All corrected records were fully read back; no further approval wait was introduced.

## Artifact digests

| Locator | Final digest |
|---|---|
| [[decisions/d93377961365086-dream-native-quantum-and-debt]] | 466a568d5ed4de5b3ba26506c4f589d646b331120b8ec93ac106fac8802d8a00 |
| [[specs/dream-term-c1]] | f90719ede139ae36a1f5a7e55dbf259820796bbe295d5e224563bcd9d374e364 |
| [[interfaces/dream-term-c1]] | 66b9f28f3d3e8486e992ebf7e19e84a3652f1fe397066695b41e497ed62994fe |
| [[specs/dreaming-mcp]] | 47e430a367e4b1ba10262669ffe59ac8817dd467dab43ee7c88fb7bf4a944f2f |
| [[interfaces/dreaming-mcp]] | 9a67d386a9086d1ec2b97802720e5d095c0302e9188b1794be43b95fab7ed638 |

Complete continuation readbacks matched intended bodies for all five, including the corrected cutoff records. Original refs/etags and non-body metadata were preserved except host-generated authorship/update/provenance.

## Actual check and source-only race warning

git diff --check in /Users/feral/mydev/bboxy450-term-dream-c1 returned **exit0**, empty output at final source snapshot19:12:48 UTC. Git/source/hash/JSON inspection commands returned0. No source was changed by this seat.

**Source-only warning:** the active consumer logic file changed during review. Its initial SHA256788edd49c1afc87877ff74f2193078b3a087b66b61282a7f1ef20f802c7b082e became2f7f925d4ab11886291bb7d4fbe3ea587e537eed81ffe5cf6509c8cfac7000f4. Source line anchors/classification describe the inspected snapshot, not a final verification of later author changes. Wake-up SHA35a43c9056d0b1fb5524d9c1432c7450baf924c9eb9870d93d0334de24d0fc06 and registry5db17cbf224992f1cdf969fbbe0763b82f842793c3cca2ffe564116af0a1c8d3 were unchanged across recorded reads. Producer citations use clean embedded47182320 bytes, SHAcd934e73511ba035773dbce18a0e9486ea7b9e30586cfa0bfb8643eb68d5aeb9.

Actual workspace worker SHA16a746d6d115f75c33402ac61ded2f9ba6ac2272930acd0bd99febb7c9ce4edf matches the heap receipt, distinct from previous standalone artifacts. Retain actual compiler/features/source/build receipts in final evidence; source equality alone does not prove binary identity. Failed log SHAe2b89a20a1bfc984c7eb19af6347209db979a09869cedb4cd6fd2b7b36642652; heap JSON SHAf12a9569e66b82b6742445a788ed9145f1701941dfe8ec1310e6e5591b021618.

Public primary Rust process/thread and Apple link documentation were fetched only to ground operation/ownership semantics; URLs, retrieval date and limits are in the decision. Two POSIX clock-page fetches returned403 and were not used as evidence. No new benchmark, source edits/builds/tests, provider/credential/account/operator/native-app actions, recruitment/assignment, install/merge/push or commits. Consumer parser fixes and Oracle implementation amendments continue independently.
