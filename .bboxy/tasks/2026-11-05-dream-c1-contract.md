---
type: task
title: Freeze the minimal C1 Dream inspection contract for implementation
allowed_paths:
- specs/dream-term-c1.md
- interfaces/dream-term-c1.md
- reports/2026-11-05-dream-c1-contract.md
verify: git diff --check
id: 01M6NNCSYCDXV8Y0ZG6YFTGD8K
status: accepted
delegated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T11:14:02.313Z
created: 2026-11-05T10:52:09.521Z
number: 93377961364366
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
  - criterion: Exact wire/control/error contract and runner provisioning grounded in existing code.
    reference: '[[interfaces/dream-term-c1]] — all DTOs/framing/hash/errors, single wire owner and conformance corpus; term main.rs:10061-10063, wake-up launch.rs:810-834, adapters/mod.rs:72-98 and Makefile:282-284,645-651.'
  - criterion: C1 source/effect ownership, permission, bounds, duplicate/restart/cancel/ABA and durable results fully specified without new inference or broad scanning.
    reference: '[[specs/dream-term-c1]] — Closure/admission/lifecycle, limits and case table; [[interfaces/dream-term-c1]] — own-seat permission, SourceKey/original-op identity, exact selected source/blob checks, independent controls, output custody. Existing detached/full-file D0 paths explicitly suppressed/replaced for term-c1.'
  - criterion: Concrete implementation file split and formal/broken-control/process test matrix with Make commands.
    reference: '[[specs/dream-term-c1]] — Two implementation components and checks, minimal finite models/initial state/actions/fairness, causal controls, private measured stall/flood/foreground/large-input/native witnesses; proposed make test-dream-c1 on both sides and actual existing formal/landing gates distinguished.'
  - criterion: New spec/ICD and append-only report created; git diff --check exit recorded, no source changes.
    reference: '[[reports/2026-11-05-dream-c1-contract]] — spec digest c3eb58e89bf45294d899b67d2cf41f26a1a5faac9340870f32daff94d99eb2fa, ICD 0741de61162cd8f8308046cf286f588d3fc0e9250f50541a0ed8845db4e067dd, exact readbacks, git diff --check exit 0, unchanged HEADs/harness clean, 29 read/check commands and commits=[].'
report:
  summary: C1 spec and ICD are ready for producer/consumer implementation. They fix the single harness-owned serde/schema/fixture authority, provisioned sibling headless runner, exact control/manifest/result/error wire, own-seat closed-source custody, one deduplicated close-owner path suppressing legacy replay, dedicated bounded low-priority workers, foreground pause, independent cancellation/status, restart/ABA/settlement and immutable paged results. Named formal/control/process and measured stall/flood/large-input/input/message/frame gates are explicit future requirements. Full readback matched both documents; git diff --check exited 0. Term ad243195 and harness a2ca8f39 unchanged; no source edits, builds/tests, commits or assignments. Report records exact source evidence, digests, commands and exclusions.
  commit: null
  reported_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  op_id: 01M6NPJS6R596PNFY139PGA2VF
  verify_revision: 0
  report: '[[reports/2026-11-05-dream-c1-contract.md]]'
provenance:
  digest: sha256:60c460395527034f619bdda263a0810869b6c8eb222983911addaff72b3acf21
  frontmatter_digest: sha256:f2b4075263cb5a1687308f150f372e0352d84509c364d5269a812aaedcfbba2e
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: 3bQSw7zT7XK3GBlSAq5pDyo882boHfVeq6pT+HxU5UPmgpobGgkop/t+AGMBdneX1xB1POAS8m2OaSrATQ1nBw==
  log: .bboxy/log/2026-11-05.jsonl#134075
---
ROLE: ARCHITECT

## Goal
Operator approved building [[specs/dream-term-integration]]. Freeze ONLY its C1 own-seat closed-IR inspection slice so two Executors can implement the producer and consumer without guessing the wire boundary. Keep it short and implementation-ready; do not repeat the full research or redesign later cuts.

## Scope
Workdir /Users/feral/mydev/bboxy450-term. Read-only source in term and embedded harness. Write only the allowed knowledge records via knowledge_write. No source edits, builds, recruitment, assignment, merges or production actions. Existing authenticated Architect Sable Quill; orient/knowledge_orient as appropriate.
Current baseline term ad243195 / harness a2ca8f39; recheck. Read [[specs/dream-term-integration]], [[interfaces/dream-term]], [[research/dream-term-integration]]. Latest operator says knowledge records MAY be Git-backed; ignore stale installed skill's blanket private/uncommitted wording. Do not invoke retired providers.

## Required contract
Define exact versioned serde/JSON request/result/error fields and framing, source-manifest/closed-log/blob validation, artifact location ownership and limits; exact Inspect/Status/Cancel control DTOs and permission/read checks for C1. Select the simplest existing launch route after inspecting term's harness-exec and harness headless dispatch, avoiding a second installation recipe or requiring a separately installed executable without a provisioning path. Reuse current transport/generation fences.
Decide explicit semantics for missing/unavailable closed IR, source mutation/torn JSONL, original op/request digest duplicates, cancellation/deadline/settlement, daemon restart and A→B→A, durable bounded status/pages, independent jobs and exactly what inspection emits from current D0 APIs. Keep IR bytes unchanged; no inference, trials, KB writes, trust enrollment, source corpus sweeps, swarm expansion or new UI.
Keep this C1 allocation self-contained with named bounds from the accepted plan; no blanket free-space gate, no global single-worker bottleneck. If accepted plan has an implementation gap resolve it in this contract with rationale.
Split exact source/file ownership into harness producer and term consumer; shared wire schema has ONE owner and explicit cross-language/crate conformance fixtures. Define minimal C1 formal variables/actions/invariants + causal broken controls and real API/process witnesses; give canonical Make targets each side must add/run. Identify which existing tests to reuse and a proportionate final landing gate. Do not invent an installed target or claim tests ran.
Work to ready, not past it. Prefer concise typed examples/table over prose. Name true remaining blockers precisely rather than designing every future stage.

## Acceptance
- [ ] Exact wire/control/error contract and runner provisioning grounded in existing code.
- [ ] C1 source/effect ownership, permission, bounds, duplicate/restart/cancel/ABA and durable results fully specified without new inference or broad scanning.
- [ ] Concrete implementation file split and formal/broken-control/process test matrix with Make commands.
- [ ] New spec/ICD and append-only report created; git diff --check exit recorded, no source changes.

## Handoff
Create report with task link, commits: [], verify{command:"git diff --check",exit_code:actual}, locators, evidence and exclusions; task_report once ready. No progress messages unless all work stops.

## Operator refinement: smooth background execution
Operator: “we need to make sure that our dreaming system is not blocking any threads, it is just smoothly running in the background … smooth and sexy”. Make responsiveness an observable C1 acceptance requirement, not merely a spawn_background label. No filesystem read/hash/JSON parsing/Git/provider call/process wait or contended lock on UI or mux/async reactor owners; use bounded off-owner IO/CPU work with small batches/yields and explicit per-poll CPU budget. Merely calling spawn_blocking with an unbounded whole-corpus scan is insufficient. Budget exhaustion/busy shards return immediately or defer that job; no unbounded task/thread creation or global worker queue that serializes unrelated jobs. Separate cancellation/status control from a saturated result lane. Own and settle worker processes while allowing unrelated shards, messaging, input and rendering to advance. Define real private stall/flood/large-input witnesses and measured responsiveness deadlines against existing frame/runtime budgets, plus causal controls that move work onto the hot owner or wait on the busy job and MUST fail. This refinement applies to later D1/trial/provider work too. No new operator interaction or global configuration. Preserve the narrow C1 scope and one shared wire owner.


## Operator refinement: post-session worker, not live-session dreaming
Operator is fixing provenance CPU bursts on another machine and explicitly asks for the same care here: “no blocking async loop, separate thread for worker, smart, dont be reactive, take your time”, then “its a dreaming system, after session is done we dream, not during”. This is the current trigger law: only authoritative completed/closed source sessions are eligible; no turn-end, idle-state, UI detach, observation gap, native EOF or missing peer substitutes for SessionClosed. Inspection and eventual automatic dreaming consume immutable closed IR only. No automatic Dream inference or consolidation runs in the producing live session; schedules/quota/manual triggers cannot bypass source closure. If new foreground work resumes, defer new expensive stage admissions and cooperatively yield/pause between bounded work units, retaining progress; no global all-seats-idle chokepoint.
Choose a genuinely isolated dedicated worker thread/process for hashing/parsing/replay/IO, with an explicit low CPU scheduling share and bounded messages, rather than placing CPU or synchronous filesystem work on the shared async event loop. Reuse existing per-seat/session off-thread primitives where possible; no new global pool bottleneck. Work from the close event plus retained cursors/fingerprints; no periodic account-wide rescans or duplicate-close/restart CPU bursts. Tests must hold a source open/idle/turn-ended and assert zero dreaming, then close it and assert exactly one admitted job, including duplicate close and restart. Test an intentionally slow/CPU-heavy worker while async input/message/status/cancel and unrelated shards stay responsive. Keep large input CPU/heap measured in focused tests. The operator prefers taking time to get this right over a reactive quick patch. Preserve the accepted C1 outcome and do not implement later inference/adoption.


## Report from seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR

C1 spec and ICD are ready for producer/consumer implementation. They fix the single harness-owned serde/schema/fixture authority, provisioned sibling headless runner, exact control/manifest/result/error wire, own-seat closed-source custody, one deduplicated close-owner path suppressing legacy replay, dedicated bounded low-priority workers, foreground pause, independent cancellation/status, restart/ABA/settlement and immutable paged results. Named formal/control/process and measured stall/flood/large-input/input/message/frame gates are explicit future requirements. Full readback matched both documents; git diff --check exited 0. Term ad243195 and harness a2ca8f39 unchanged; no source edits, builds/tests, commits or assignments. Report records exact source evidence, digests, commands and exclusions.

```json
{
  "op_id": "01M6NPJS6R596PNFY139PGA2VF",
  "commit": null,
  "report": "[[reports/2026-11-05-dream-c1-contract.md]]",
  "verify_result": {
    "command": "git diff --check",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Exact wire/control/error contract and runner provisioning grounded in existing code.",
        "reference": "[[interfaces/dream-term-c1]] — all DTOs/framing/hash/errors, single wire owner and conformance corpus; term main.rs:10061-10063, wake-up launch.rs:810-834, adapters/mod.rs:72-98 and Makefile:282-284,645-651."
      },
      {
        "criterion": "C1 source/effect ownership, permission, bounds, duplicate/restart/cancel/ABA and durable results fully specified without new inference or broad scanning.",
        "reference": "[[specs/dream-term-c1]] — Closure/admission/lifecycle, limits and case table; [[interfaces/dream-term-c1]] — own-seat permission, SourceKey/original-op identity, exact selected source/blob checks, independent controls, output custody. Existing detached/full-file D0 paths explicitly suppressed/replaced for term-c1."
      },
      {
        "criterion": "Concrete implementation file split and formal/broken-control/process test matrix with Make commands.",
        "reference": "[[specs/dream-term-c1]] — Two implementation components and checks, minimal finite models/initial state/actions/fairness, causal controls, private measured stall/flood/foreground/large-input/native witnesses; proposed make test-dream-c1 on both sides and actual existing formal/landing gates distinguished."
      },
      {
        "criterion": "New spec/ICD and append-only report created; git diff --check exit recorded, no source changes.",
        "reference": "[[reports/2026-11-05-dream-c1-contract]] — spec digest c3eb58e89bf45294d899b67d2cf41f26a1a5faac9340870f32daff94d99eb2fa, ICD 0741de61162cd8f8308046cf286f588d3fc0e9250f50541a0ed8845db4e067dd, exact readbacks, git diff --check exit 0, unchanged HEADs/harness clean, 29 read/check commands and commits=[]."
      }
    ]
  }
}
```
