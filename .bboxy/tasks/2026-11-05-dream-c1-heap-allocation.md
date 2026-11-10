---
type: task
title: Allocate C1 combined parent/child heap ceiling without widening it
allowed_paths:
- specs/dream-term-c1.md
- interfaces/dream-term-c1.md
- decisions/dream-c1-heap-allocation.md
- reports/2026-11-05-dream-c1-heap-allocation.md
verify: git diff --check
id: 01M6NT5TRR249DH25T2R39F5NM
status: accepted
delegated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T12:22:23.473Z
created: 2026-11-05T12:15:44.598Z
number: 93377961364570
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
  - criterion: Fixed split, coverage, enforcement/overflow/control reserve and true combined-bound semantics
    reference: '[[decisions/d93377961364572-dream-c1-heap-allocation]]; [[specs/dream-term-c1]] Working memory; [[interfaces/dream-term-c1]] fixed v1 allocation paragraph'
  - criterion: Immediate shared-boundary feedback to both current authors and Rook
    reference: '[[reports/2026-11-05-dream-c1-heap-allocation]] Handoff: message IDs and native-queued receipts'
  - criterion: Current source/revisions, budget-only KB changes, actual verification and exclusions
    reference: '[[reports/2026-11-05-dream-c1-heap-allocation]] Source hashes, current etag/full readbacks, git diff --check exit0, commits[]'
report:
  summary: Fixed the C1 live heap allocation at parent32 MiB + child96 MiB, each including a protected2 MiB control reserve, without expanding the combined128 MiB, wire/version or other limits. Updated only the spec Working memory row and ICD budget paragraph with current refs/etags; decision decisions/d93377961364572-dream-c1-heap-allocation defines all-live-buffer/native/helper coverage, preallocation reservations and fail-closed resource_limit, separate actual live/reserved/cumulative/RSS evidence and joint-process/independent-overflow/four-job/control witnesses. Sent the authorized boundary decision immediately to both Executors and Rook; native-queued receipts are not model-receipt claims. Existing parent estimates and child between-unit samples do not yet prove enforcement; report retains precise source hashes/gaps and no measurements. Full readbacks matched; git diff --check exit0; no source edits/builds/tests/commits or assignments.
  commit: null
  reported_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  op_id: 01M6NTEEXDV1T102JFV9D5ETZS
  verify_revision: 0
  report: '[[reports/2026-11-05-dream-c1-heap-allocation.md]]'
provenance:
  digest: sha256:25918c486e61281a33400a73252fd94123dc836cbcfb5d1b0d55403f856e6abb
  frontmatter_digest: sha256:cc17680756dd9ac5b867481b8d0b54d383ec77995855f64d7c58f8681511bada
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: YyO7WZ61WmXwF7OfOKqFszipvW70ew//aHRaGn5+4PXLOF3Mpq5uVH3QnCeh7T+f8aUIWhyycF+5UWpZQCOkCw==
  log: .bboxy/log/2026-11-05.jsonl#135389
---
ROLE: ARCHITECT

## Exact gap and decision
Consumer reports its bounded parent buffers plus producer's independent 128 MiB allowance can exceed the C1 combined 128 MiB working-allocation ceiling. Resolve ONLY this shared budget now. Proposed allocation is parent 32 MiB + child 96 MiB (total 128), with no wire/schema/version/threshold expansion and no automatic swapping/borrowing. Root supports this conservative split if source inspection confirms it can be enforced; otherwise choose a tighter explicit split within the SAME 128 MiB and give source reasons. Do not widen the total or quietly omit parser/serde/output/control buffers.

## Scope / participants
Read-only code:
P /Users/feral/mydev/bboxy450-harness-dream-c1, producer task [[tasks/2026-11-05-dream-c1-producer]], seat 01M6NPMWMJ27PAN93N9BY4KY5J.
C /Users/feral/mydev/bboxy450-term-dream-c1, consumer task [[tasks/2026-11-05-dream-c1-consumer]], seat 01M6NPQVR1STF4MY8C3TV382TJ.
Knowledge root remains selected term root. Fetch current [[specs/dream-term-c1]] / [[interfaces/dream-term-c1]] with continuations and current refs/etags before updating. Only allowed knowledge records; no source changes/builds/assignments/merge/publish or unrelated architecture. Source is moving: record exact inspected bytes/limits; do not claim measurements from source.

## Required disposition
Define what counts toward each live heap allowance, enforce-before-allocation/reservation/allocator mechanism, overflow/resource_limit behavior, control lane reserve, measured actual live heap vs cumulative allocations vs RSS, and how the true combined peak is bounded. Separately observed max(parent)+max(child) is a conservative upper bound, not a synchronized measured simultaneous peak; label accordingly. Child/background threads and context/control/parser/result buffers count; OS mappings/file cache and RSS remain explicitly separate evidence. Account-wide <=4 jobs still cannot bypass allocation admission.
Communicate the chosen allocation immediately as an authorized shared-boundary decision to BOTH Executors and Rook, with exact spec/decision locators and source mechanisms/witness to implement under their current briefs; no approval wait or request for acknowledgements. Authors continue other work meanwhile.
Update only the budget paragraph/table in current C1 spec/ICD and create the concise decision. No wire field needed: these are fixed v1 implementation ceilings. Require a real parent+child large-input/process witness, controlled over-allocation on each side, independent cancellation/control responsiveness and unweakened limits. A tracked/advisory counter alone is not proof that actual allocations are covered.

## Handoff
Run git diff --check, record actual exit and source/revision references. Append-only report with task, commits: [], verify actual command/exit, chosen split, enforcement gaps and precise author feedback. task_report once. Keep it brief; no broader redesign or repeated source audit.

## Report from seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR

Fixed the C1 live heap allocation at parent32 MiB + child96 MiB, each including a protected2 MiB control reserve, without expanding the combined128 MiB, wire/version or other limits. Updated only the spec Working memory row and ICD budget paragraph with current refs/etags; decision decisions/d93377961364572-dream-c1-heap-allocation defines all-live-buffer/native/helper coverage, preallocation reservations and fail-closed resource_limit, separate actual live/reserved/cumulative/RSS evidence and joint-process/independent-overflow/four-job/control witnesses. Sent the authorized boundary decision immediately to both Executors and Rook; native-queued receipts are not model-receipt claims. Existing parent estimates and child between-unit samples do not yet prove enforcement; report retains precise source hashes/gaps and no measurements. Full readbacks matched; git diff --check exit0; no source edits/builds/tests/commits or assignments.

```json
{
  "op_id": "01M6NTEEXDV1T102JFV9D5ETZS",
  "commit": null,
  "report": "[[reports/2026-11-05-dream-c1-heap-allocation.md]]",
  "verify_result": {
    "command": "git diff --check",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Fixed split, coverage, enforcement/overflow/control reserve and true combined-bound semantics",
        "reference": "[[decisions/d93377961364572-dream-c1-heap-allocation]]; [[specs/dream-term-c1]] Working memory; [[interfaces/dream-term-c1]] fixed v1 allocation paragraph"
      },
      {
        "criterion": "Immediate shared-boundary feedback to both current authors and Rook",
        "reference": "[[reports/2026-11-05-dream-c1-heap-allocation]] Handoff: message IDs and native-queued receipts"
      },
      {
        "criterion": "Current source/revisions, budget-only KB changes, actual verification and exclusions",
        "reference": "[[reports/2026-11-05-dream-c1-heap-allocation]] Source hashes, current etag/full readbacks, git diff --check exit0, commits[]"
      }
    ]
  }
}
```
