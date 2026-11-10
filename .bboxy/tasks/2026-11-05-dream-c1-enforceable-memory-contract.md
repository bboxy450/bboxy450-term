---
type: task
title: Correct C1 memory contract to enforceable working storage and honest native overhead
allowed_paths:
- specs/dream-term-c1.md
- interfaces/dream-term-c1.md
- decisions/dream-c1-enforceable-memory-contract.md
- reports/2026-11-05-dream-c1-enforceable-memory-contract.md
verify: git diff --check
id: 01M6NV8MB1XAFAB199JFCQKZPC
status: accepted
delegated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T12:45:08.937Z
created: 2026-11-05T12:34:44.002Z
number: 93377961364578
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
role: architect
assignee: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
verify_result:
  command: git diff --check
  exit_code: 0
  evidence:
  - criterion: Practical superseding memory contract with unchanged shares, protected control, enforceable domain and honest overhead/measurement/model limits
    reference: '[[decisions/d93377961364581-dream-c1-enforceable-memory-contract]]; [[specs/dream-term-c1]] Working memory/model matrix; [[interfaces/dream-term-c1]] fixed memory allocation'
  - criterion: Amended native SQLite/public authority boundary, existing glue and immediate author feedback
    reference: '[[decisions/d93377961364581-dream-c1-enforceable-memory-contract]] Public authority fence; [[reports/2026-11-05-dream-c1-enforceable-memory-contract]] source glue and three native-queued messages'
  - criterion: Current source/revision evidence, actual diff check, scoped KB edits and no measurements/tests claimed
    reference: '[[reports/2026-11-05-dream-c1-enforceable-memory-contract]] hashes, full etag readbacks, exit0 and exclusions; commits[]'
report:
  summary: Corrected the premise in superseding decision d93377961364581 and affected C1 spec/ICD budget/matrix/public-authority wording. Keep32/96 MiB shares, protected2 MiB controls and all CPU/duty/latency/closure/settlement gates; enforce declared application-owned storage credits before controllable growth while explicitly measuring opaque serde/BTree/regex/SQLite/native/runtime overhead. Observed32/96/combined128 ceilings still fail regression gates; no all-times whole-process/native proof is claimed and no global allocator/SQLite limit is required. Defined bounded schemas/containers/threads/helpers, overlap headroom, real joint-process/overflow/four-job/control tests and formal coverage limits. Amended authority correction removes C1 credential/signing probes in favor of host-authenticated current public-incarnation/revocation fences; exact identity/status.rs + chain.rs glue and routing/invalidation/zero-custody witnesses sent to both authors/Rook under existing briefs. Full readbacks matched, git diff --check exit0, exact hashes/gaps recorded; no source edits/tests/measurements/builds/commits or new assignments.
  commit: null
  reported_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  op_id: 01M6NVS32KGN5MD24RZM9BN82T
  verify_revision: 0
  report: '[[reports/2026-11-05-dream-c1-enforceable-memory-contract.md]]'
provenance:
  digest: sha256:4aa2efebec9f31c52dbdf6915c7b04f459cd1307f1616983f028b42c809e5464
  frontmatter_digest: sha256:4684b8c5538bb3dc453827cceb2fb48dfe434511eeee166596c7cbe643edd271
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: RaUmWAW4xe1c3TEuIIfMFqKIPGLXgKpV64C+5gusvX/wrn81Jiteap6DX/8ocbx5vEX/eZhlfl56Zx4/W9tOAA==
  log: .bboxy/log/2026-11-05.jsonl#135504
---
ROLE: ARCHITECT

## Root disposition: correct the premise, keep the product moving
Producer's source audit found std::thread::Builder -> native pthread_create allocation outside its Rust ledger, and infallible serde Content/escape/BTree growth. The last decision's universal source-proved before-every-allocation invariant is not supported by an enforceable mechanism here. We have now repeatedly narrowed implementation to chase that premise. Per the project's simplicity/verification rules, correct the law; do not demand a proof of arbitrary Rust/OS allocation or turn every C1 run into unsupported/resource_limit.
Root direction: retain the 32 MiB parent / 96 MiB child allocation, protected control shares, fixed input/output/worker counts and ALL CPU/duty/latency/closure/settlement gates. Separate mechanically enforced C1-owned working-storage admission from measured native/runtime/transient overhead. Stop claiming an absolute all-times whole-process/native 128 MiB heap cap unless an actual supported OS/allocator mechanism establishes it. No silent relabeling of measurements as invariants, no budget inflation or fictitious ledger coverage.

## Scope
Read only the two author trees:
P /Users/feral/mydev/bboxy450-harness-dream-c1, task [[tasks/2026-11-05-dream-c1-producer]], seat 01M6NPMWMJ27PAN93N9BY4KY5J.
C /Users/feral/mydev/bboxy450-term-dream-c1, task [[tasks/2026-11-05-dream-c1-consumer]], seat 01M6NPQVR1STF4MY8C3TV382TJ.
Selected knowledge root is unchanged. Fetch CURRENT C1 spec/ICD and [[decisions/d93377961364572-dream-c1-heap-allocation]]. Write only allowed records via knowledge tools; no source edits/tests/builds/new assignments or broad library/OS research. This is a concise contract correction, not another architecture programme.

## Required corrected contract
Define the exact bounded application-owned storage domain (large/source/line/retained parser/state/result/control buffers and concurrency), its reservations before growth and fallible handling for controllable allocations. Give practical fixed-schema input/container/depth/count bounds and conservative justified overlap allowances for serde/regex/SQLite helpers; state which are measured/implementation-only assumptions rather than absolute allocator proofs. Do not invent an arbitrary factor and call it proven.
Account for native thread/runtime/allocator overhead explicitly through fixed owner/thread counts, minimal worker startup and retained actual high-water/RSS evidence, with stated coverage limits and honest resource failure. A private child may fail/exit and be reaped/quarantined honestly; shared-host abort/global allocator switch remains forbidden. Do not require an unimplemented hard OS memory mechanism as the C1 default.
Keep control/status/cancel usable from preprovisioned bounded storage; expensive work cannot consume its reserve. Keep 32/96 named allocation shares and measured regression ceilings: an observed breach fails its relevant gate, but samples are not all-times guarantees. Distinguish parent attributed live working storage, child full observed heap, cumulative allocation, native overhead, stack/mapping/RSS, and sum-of-separate-peaks versus synchronized samples.
Retain real parent+child multi-MiB/overflow/four-job/control tests and causal missing-reservation control for the mechanically enforced domain. Formal model proves only that declared credit system; unmodeled opaque allocations must be listed, not abstracted away as proven Rust correctness. No CPU/heap test skip or threshold increase.

## Communication and handoff
Create a superseding decision recording alternatives, this root choice/rationale and exact guarantee limits; update ONLY affected C1 budget paragraphs/matrix wording. Immediately send both authors and Rook the practical corrected rule and precise next implementation/witness steps under their current briefs, without another approval wait. Root will amend live tasks to reference it. They continue other bounded work meanwhile.
Report concise source evidence, decisions/locators, actual git diff --check exit, no tests/measurements claimed, commits: []. task_report once. Work to ready; no pthread implementation archaeology or general custom allocator project.

## Concrete native/authority boundary for this same correction
Consumer additionally confirmed that SQLite's hard_heap_limit is process-global, and lookaside does not establish a per-connection total heap cap. Do not require that nonexistent per-job guarantee or alter the host-global allocator/SQLite limits; account native SQLite as bounded-use, measured/implementation-only overhead with fixed row/blob/query/cache/connection/concurrency limits and explicit failure/control evidence, consistent with the corrected guarantee domain.
Source review also confirms consumer Authority::capture (registry/dream_c1.rs around :391) calls keys.signing_hold(), whose ChainSeatKeys path runs device_current and custody.load (identity/chain.rs:134-155,204-220). Resolve the authority/cache boundary in the C1 contract: jobs use an already authenticated host admission/public incarnation/revocation fence and bounded current-state checks, never credential custody load, private-key/signing probe, Keychain/GPG prompt or a weaker self-asserted cache. Missing or stale admitted public authority is unavailable/deferred, not a credential fallback. Specify who publishes/invalidate that public fence on actual identity/grant/root changes; preserve ABA, revocation and final-presentation revalidation. Do not redesign general identity/persistence or require it to become globally native-free. Provide exact necessary existing source glue to Rook if outside authors' allowed paths. Keep this within the same practical memory/authority correction and immediately relay the resolved boundary to both authors. No new wire/data authority claim, no raw credentials access and no broader audit.


## Report from seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR

Corrected the premise in superseding decision d93377961364581 and affected C1 spec/ICD budget/matrix/public-authority wording. Keep32/96 MiB shares, protected2 MiB controls and all CPU/duty/latency/closure/settlement gates; enforce declared application-owned storage credits before controllable growth while explicitly measuring opaque serde/BTree/regex/SQLite/native/runtime overhead. Observed32/96/combined128 ceilings still fail regression gates; no all-times whole-process/native proof is claimed and no global allocator/SQLite limit is required. Defined bounded schemas/containers/threads/helpers, overlap headroom, real joint-process/overflow/four-job/control tests and formal coverage limits. Amended authority correction removes C1 credential/signing probes in favor of host-authenticated current public-incarnation/revocation fences; exact identity/status.rs + chain.rs glue and routing/invalidation/zero-custody witnesses sent to both authors/Rook under existing briefs. Full readbacks matched, git diff --check exit0, exact hashes/gaps recorded; no source edits/tests/measurements/builds/commits or new assignments.

```json
{
  "op_id": "01M6NVS32KGN5MD24RZM9BN82T",
  "commit": null,
  "report": "[[reports/2026-11-05-dream-c1-enforceable-memory-contract.md]]",
  "verify_result": {
    "command": "git diff --check",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Practical superseding memory contract with unchanged shares, protected control, enforceable domain and honest overhead/measurement/model limits",
        "reference": "[[decisions/d93377961364581-dream-c1-enforceable-memory-contract]]; [[specs/dream-term-c1]] Working memory/model matrix; [[interfaces/dream-term-c1]] fixed memory allocation"
      },
      {
        "criterion": "Amended native SQLite/public authority boundary, existing glue and immediate author feedback",
        "reference": "[[decisions/d93377961364581-dream-c1-enforceable-memory-contract]] Public authority fence; [[reports/2026-11-05-dream-c1-enforceable-memory-contract]] source glue and three native-queued messages"
      },
      {
        "criterion": "Current source/revision evidence, actual diff check, scoped KB edits and no measurements/tests claimed",
        "reference": "[[reports/2026-11-05-dream-c1-enforceable-memory-contract]] hashes, full etag readbacks, exit0 and exclusions; commits[]"
      }
    ]
  }
}
```
