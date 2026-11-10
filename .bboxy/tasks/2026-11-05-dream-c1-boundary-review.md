---
type: task
title: Review in-flight C1 worker/host boundaries and smooth post-session execution
allowed_paths:
- reports/2026-11-05-dream-c1-boundary-review.md
verify: git diff --check && git -C /Users/feral/mydev/bboxy450-harness-dream-c1 diff --check && git -C /Users/feral/mydev/bboxy450-term-dream-c1 diff --check
id: 01M6NR5T4MD2PCNE3VN9YKP9EP
status: accepted
delegated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T11:52:26.760Z
created: 2026-11-05T11:40:45.959Z
number: 93377961364454
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
  command: git diff --check && git -C /Users/feral/mydev/bboxy450-harness-dream-c1 diff --check && git -C /Users/feral/mydev/bboxy450-term-dream-c1 diff --check
  exit_code: 0
  evidence:
  - criterion: Source-grounded findings/disposition with exact snapshot hashes and lines
    reference: '[[reports/2026-11-05-dream-c1-boundary-review]] Findings and disposition; Snapshot/provenance'
  - criterion: Both sides closure/nonblocking/ownership/settlement/schema boundary and existing paths
    reference: '[[reports/2026-11-05-dream-c1-boundary-review]] Boundary mechanisms inspected; F1-F6; [[decisions/d93377961364464-dream-c1-sole-post-close-owner]]'
  - criterion: No static-review claim of green builds, performance or formal verdict; no source edits
    reference: '[[reports/2026-11-05-dream-c1-boundary-review]] Verification evidence and limits; exact three-tree diff check exit 0'
report:
  summary: Completed the read-only moving-snapshot review of producer/consumer closure, runtime isolation, provenance, restart/cancel, settlement and wire compatibility. Report digest 86278c4d77b8415b9da3d4a3fd10130ef73ac07b111ebf6c013a4416b0a2f6af records six findings, exact source hashes, direct author feedback and static corrections observed for page integrity, predecessor replay fences and cancelled-state preservation. Cancellation operation identity/deferred settlement and immediate upstream foreground pause remained unresolved in the inspected source; runtime/formal/process verdicts remain unproven. After the report's source cutoff, Rook resolved F6 scope with decisions/d93377961364464-dream-c1-sole-post-close-owner (suppress automatic D0 and IR advice under one TermC1 gate), supplementing the explicit fork-exclusion decision; those choices still need final-source implementation/witness verification. Exact three-tree git diff --check exited 0; no source edits, builds/tests, commits or production actions.
  commit: null
  reported_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  op_id: 01M6NRSWF5BGBY8J6FK52MXM9B
  verify_revision: 0
  report: '[[reports/2026-11-05-dream-c1-boundary-review.md]]'
provenance:
  digest: sha256:e81747e81b96af466ecb37fe4034d34007988bc0c662f4d035de94e52f0c5e9b
  frontmatter_digest: sha256:4e12a88ca4c799e7255b992dbd21bc2e27a8f75b9dbaf18977b4abab7da7cc70
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: clbC2SA2CrryofWqfdkgxmxQTtSpL/ZMgrzcc++9mIKtT2ikIBhZVIt429fI8JUwgNxrvn5QSvLcV/mLTLd/Aw==
  log: .bboxy/log/2026-11-05.jsonl#134759
---
ROLE: ARCHITECT

## Goal
User approved C1 implementation and explicitly requires smooth post-session dreaming with no blocking async loop/CPU bursts. The producer/consumer now contain implementation. Independently review the interfaces and runtime invariants early, while authors continue. Use [[specs/dream-term-c1]] and [[interfaces/dream-term-c1]], not a broader design exercise.

## Read-only scope
Producer source: /Users/feral/mydev/bboxy450-harness-dream-c1, task [[tasks/2026-11-05-dream-c1-producer]], seat 01M6NPMWMJ27PAN93N9BY4KY5J.
Consumer source: /Users/feral/mydev/bboxy450-term-dream-c1, task [[tasks/2026-11-05-dream-c1-consumer]], seat 01M6NPQVR1STF4MY8C3TV382TJ.
Knowledge root remains /Users/feral/mydev/bboxy450-term/.bboxy; use knowledge tools. Write ONLY the allowed report. No code/spec changes, builds/tests that contend with authors, assignments/recruitment, merge/push/install, role or task scope changes. Read actual Git state/diff/source and record content hashes/revisions for this moving snapshot; no source outside these authorized trees except baseline comparison. Never read credentials/capabilities/raw account logs.

## Review
Focus on actual mechanisms: source SessionClosed and one legacy/term owner; exactly-once close/manual/restart keys; authoritative source/session/epoch/raw-byte/blob evidence; inherited context and ABA; dedicated low-priority worker versus hot/shared runtime work; bounded per-unit CPU/heap/IO and duty cycle; foreground pause control; independent cancel/status despite saturated outputs; process/thread settlement and immutable results; schema mirror/conformance and provisional/capability probing.
Producer additions are src/dream/{c1,inspect,c1_wire}.rs and changed d0/close hooks; consumer adds wake-up/dream_c1.rs and mux logic/dream_c1.rs/catalogue/control. Check incremental transcript validation against actual protocol laws rather than trusting test names. Check that new source/status persistence does not move long SQLite/serialization/hash loops onto the mux owner. Treat tests/models as claims until inspected; identify missing causal controls/real witnesses.
Send ONLY important actionable findings, with file:line/mechanism and a concrete fix/witness, directly to the responsible Executor within its current brief, cc Rook if it affects the shared boundary or needs a scope/spec decision. Such feedback is authorized review, not authority to change their task. Do not block authors or request acknowledgements. No polling/status requests. Continue other review while a finding is addressed.

## Acceptance / handoff
- [ ] Source-grounded findings/disposition distinguish actual defect, proposed concern and unproven measurement, with exact snapshot hashes/lines.
- [ ] Both sides' closure/nonblocking/ownership/settlement/schema boundary reviewed, including existing path interactions.
- [ ] No claim of green builds/performance/formal verdict from static review, and no author source edits.
Run only the frontmatter git diff --check command (record actual exit; checks tracked diff only). Create append-only report with task, commits: [], verify actual command/exit, actionable findings/messages, limits and evidence. task_report once. A later source change can stale this snapshot; root Oracle owns final-source acceptance. Keep the report concise; no 96-command register or repeated parent prose.

## Report from seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR

Completed the read-only moving-snapshot review of producer/consumer closure, runtime isolation, provenance, restart/cancel, settlement and wire compatibility. Report digest 86278c4d77b8415b9da3d4a3fd10130ef73ac07b111ebf6c013a4416b0a2f6af records six findings, exact source hashes, direct author feedback and static corrections observed for page integrity, predecessor replay fences and cancelled-state preservation. Cancellation operation identity/deferred settlement and immediate upstream foreground pause remained unresolved in the inspected source; runtime/formal/process verdicts remain unproven. After the report's source cutoff, Rook resolved F6 scope with decisions/d93377961364464-dream-c1-sole-post-close-owner (suppress automatic D0 and IR advice under one TermC1 gate), supplementing the explicit fork-exclusion decision; those choices still need final-source implementation/witness verification. Exact three-tree git diff --check exited 0; no source edits, builds/tests, commits or production actions.

```json
{
  "op_id": "01M6NRSWF5BGBY8J6FK52MXM9B",
  "commit": null,
  "report": "[[reports/2026-11-05-dream-c1-boundary-review.md]]",
  "verify_result": {
    "command": "git diff --check && git -C /Users/feral/mydev/bboxy450-harness-dream-c1 diff --check && git -C /Users/feral/mydev/bboxy450-term-dream-c1 diff --check",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Source-grounded findings/disposition with exact snapshot hashes and lines",
        "reference": "[[reports/2026-11-05-dream-c1-boundary-review]] Findings and disposition; Snapshot/provenance"
      },
      {
        "criterion": "Both sides closure/nonblocking/ownership/settlement/schema boundary and existing paths",
        "reference": "[[reports/2026-11-05-dream-c1-boundary-review]] Boundary mechanisms inspected; F1-F6; [[decisions/d93377961364464-dream-c1-sole-post-close-owner]]"
      },
      {
        "criterion": "No static-review claim of green builds, performance or formal verdict; no source edits",
        "reference": "[[reports/2026-11-05-dream-c1-boundary-review]] Verification evidence and limits; exact three-tree diff check exit 0"
      }
    ]
  }
}
```
