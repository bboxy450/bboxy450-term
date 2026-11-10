---
type: task
title: Disposition measured C1 indivisible-native CPU versus cooperative quantum
role: architect
worktree: /Users/feral/mydev/bboxy450-term-dream-c1
allowed_paths:
- specs/dream-term-c1.md
- interfaces/dream-term-c1.md
- specs/dreaming-mcp.md
- interfaces/dreaming-mcp.md
- decisions/*dream*quantum*
- reports/2026-11-05-dream-c1-native-quantum-disposition.md
verify: git diff --check
id: 01M6PHF28EXY6W5V9RDSFFJSVZ
status: accepted
delegated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T19:19:45.500Z
created: 2026-11-05T19:02:43.868Z
number: 93377961365081
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
assignee: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
verify_result:
  command: git diff --check
  exit_code: 0
  evidence:
  - criterion: Grounded native versus cooperative boundary with finite admission/debt/effect law
    reference: decisions/d93377961365086-dream-native-quantum-and-debt.md — Exact enforceable law; Debt, controls and real effects
  - criterion: No unjustified SQLite/control/parser waiver; preserve hard foreground and qualifying real gates
    reference: reports/2026-11-05-dream-c1-native-quantum-disposition.md — Disposition and evidence classification; Source-grounded changes required
  - criterion: Frozen descriptor pre-seal cutoff and terminal coverage, formal/causal/real witness mapping
    reference: decisions/d93377961365086-dream-native-quantum-and-debt.md — Producer classification and frozen wire; Minimal formal and real gates; interfaces/dream-term-c1.md
  - criterion: Full readbacks/digests, actual clean-whitespace check and active-source warning
    reference: reports/2026-11-05-dream-c1-native-quantum-disposition.md — Artifact digests; Actual check and source-only race warning
report:
  summary: 'Recorded source-grounded decision65086 and narrowly synchronized C1 spec/ICD plus inherited Dream MCP budgets. Cooperative application/parser/cleanup/shared reactor/control work stays1ms; only named isolated creation/no-replace publication calls have observed2ms CPU/25ms wall allocation, finite counts, full inclusive charging/rest debt and retained effects/IDs. SQLite configure must split; the1.316ms child whole tick remains unclassified and needs instrumentation/split, no whole-tick exception. Preserved every foreground/latency/four-job/heap/source/actual-reap gate; current four refusals and separate parser failures remain failures. Incorporated Oracle cutoff correction: max_poll is captured before its own descriptor serialization/publication, terminal metrics cover later work and validate captured<=terminal, no wire bump/resealing/clamping. Full five-artifact/report readbacks match. Consumer git diff --check exited0; report separately records active source-hash race and actual workspace binary/hash provenance. No source/build/test/assignment actions; fresh producer correction and Q2 inheritance go through Oracle amendments.'
  commit: null
  reported_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  op_id: 01M6PJ8P9ZMZ4GAYTYS39E2F28
  verify_revision: 0
  report: '[[reports/2026-11-05-dream-c1-native-quantum-disposition.md]]'
provenance:
  digest: sha256:5cca672a4a3888f7f35ebca318a246abeea0b856dcb0692772ba194165a426c1
  frontmatter_digest: sha256:c491b6246b463aab143c540ce8b1439a2678fc5acdd3da5ff6946314dc89935e
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: lGaHIM7HeUAB2z832/OrOF67NMLx2S5NioMgEUWzodBNkFNKEQ7fItF0yqIgHZeQs3lKT5SgYUWKsztLPOO8Cg==
  log: .bboxy/log/2026-11-05.jsonl#140535
---
ROLE: ARCHITECT

## Goal / authorization
Resolve the real C1 execution-budget boundary now, in a small executable/verifiable contract. User's goal is smooth post-SessionClosed background work with isolated owners, independent control, no blocked foreground/shared async loop, no recurring CPU storms. Distinguish that goal from a universal preemptive OS syscall CPU promise. This is a spec/interface/decision assignment only: no source edits, recruitment, assignment, merge or native app actions. Consumer continues independent parser/own-seat/native/formal work; do not park the swarm.

## Evidence to inspect, not self-report acceptance
Consumer tree /Users/feral/mydev/bboxy450-term-dream-c1: signed HEADd156e0cbae84aa5a9af768220d8200cf62c200de plus active owned changes (read-only). Embedded producer accepted47182320, unchanged code pinned, actual workspace ordinary-release binary SHA16a746d6d115f75c33402ac61ded2f9ba6ac2272930acd0bd99febb7c9ce4edf. This differs from prior standalone retained artifact hash: retain actual compiler/features/source receipt rather than claiming artifact identity from source alone.
Actual /tmp/dream-c1-prepared-four-real.log exits100 (15tests,12pass/3fail) and /private/tmp/dc1-prepared-four-231baemq/runtime-isolated-four-heap.json. Four actual jobs all refused/settled/reaped, none completed. job0 native process spawn1,089,208ns, wholeunit1,093,000ns. job1 producer control/reactor1,316,083ns; this is a whole measured control tick, not yet proven a single syscall. job2 checkpoint configure1,135,292ns (whole1,138,750ns), NOT automatically indivisible. job3 single Publication::link hard-link1,510,916ns; outcomeStorageUnavailable after budget rejection. Parent reactor maxima0/175083/109417/58750ns. Treat these as failure evidence, not working multi-job proof. Parser cleanup failures remain separate and are being corrected; do not grant them an exception.
Source: consumer logic/dream_c1.rs::publish_paced ~2952 splits create/write/sync/metadata/link/check/unlink/dirsync; :2965 one hardlink. ::checkpoint_stage~3060 wraps actual Journal::configure_existing aggregate at3074. ::native_dispatch_finish~3586 and execute spawn~4091-4094 measure spawn separately but still strict1ms. Wake-up dream_c1.rs::Worker::spawn_prepared ~1260 actual Command::spawn; inspect all work around it before calling the whole function indivisible. Accepted harness src/dream/c1.rs::run~1756-2012 control tick includes input/output/start/control/join bookkeeping and possibly compute thread creation~1854. cli/serve async wrapper~94-146 exists but actual macOS early_worker_entry directly calls run before runtime. Root has read these source seams, not run a fresh benchmark. Read actual tick/profiling evidence before causal classification.

## Required decision
Read current C1 spec/ICD fully plus enforceable-memory and optimized-fixture decisions. Existing CPU row says expensive units<=1ms CPU then>=9ms rest,<=10%core/job, while also admitting blocking syscall wall time can outlive a unit on its isolated owner; reactor handler<=1ms. Name the actual enforceable application-work/cooperative scheduling boundary and how native/kernel time is charged. Either prescribe a specific demonstrated primitive/split preserving the current rule, or record a narrow source-grounded contract correction for genuine indivisible operations. Do not simply inflate every quantum/latency gate, exempt arbitrary wrappers/SQLite/serde/destructors, subtract native time from reported total, disable clocks, retry until lucky, or relabel a shared reactor as a worker. No OS absolute runtime guarantee without a real mechanism. Any native allowance needs finite operation/owner/admission bounds, separately measured CPU+wall+failures, duty/debt/rest accounting preventing repeated bursts, foreground/deadline cancellation and actual side-effect/settlement semantics. Reconcile max_poll_cpu_ns with any distinct native metrics without silently changing frozen wire/schema. A Result after a syscall may have produced a real effect even if budget fails: preserve that custody/operation identity.

Keep hard UI/input/message/status/cancel/pause/four-job/heap/source/real-reap gates and resource ownership. If an actually shared reactor contains native launch, move launch to the bounded isolated owner; don't waive it. Child control1.316ms needs actual classification before exemption. Native measured overshoot is not a proof of application inability or a pass. Actual qualifying four-job completion/control evidence on the corrected final source remains mandatory. Formal checks only model scheduling/native effects/debt/settlement abstractly, real max/latency remains process evidence with causal over-budget/repeated-native controls. Propose exact finite model variables/actions/controls and bounded source/API witnesses that make the law checkable.

## Output
Write decision alternatives/choice/reason/date with exact source/measurement/primary OS/tool documentation if needed; narrowly synchronize C1 spec/ICD and inherited Dream MCP budget wording only as required. Do not add a general scheduler/runtime project or demand a new all-times native proof. Identify exact consumer and accepted producer changes needed; accepted task stays frozen, producer correction if needed becomes a fresh assignment. Q2 current active author must receive the same boundary via Oracle amended task, not out-of-scope source editing. Full readbacks/digests, git diff --check exit0 for inspected current tree (separate source-only warning if active changes race), append-only report task/commits:[]/verifyactualexit and precise unresolved/required real evidence, task_report once. Oracle issues implementation amendments immediately after inspecting the disposition.

## Report from seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR

Recorded source-grounded decision65086 and narrowly synchronized C1 spec/ICD plus inherited Dream MCP budgets. Cooperative application/parser/cleanup/shared reactor/control work stays1ms; only named isolated creation/no-replace publication calls have observed2ms CPU/25ms wall allocation, finite counts, full inclusive charging/rest debt and retained effects/IDs. SQLite configure must split; the1.316ms child whole tick remains unclassified and needs instrumentation/split, no whole-tick exception. Preserved every foreground/latency/four-job/heap/source/actual-reap gate; current four refusals and separate parser failures remain failures. Incorporated Oracle cutoff correction: max_poll is captured before its own descriptor serialization/publication, terminal metrics cover later work and validate captured<=terminal, no wire bump/resealing/clamping. Full five-artifact/report readbacks match. Consumer git diff --check exited0; report separately records active source-hash race and actual workspace binary/hash provenance. No source/build/test/assignment actions; fresh producer correction and Q2 inheritance go through Oracle amendments.

```json
{
  "op_id": "01M6PJ8P9ZMZ4GAYTYS39E2F28",
  "commit": null,
  "report": "[[reports/2026-11-05-dream-c1-native-quantum-disposition.md]]",
  "verify_result": {
    "command": "git diff --check",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Grounded native versus cooperative boundary with finite admission/debt/effect law",
        "reference": "decisions/d93377961365086-dream-native-quantum-and-debt.md — Exact enforceable law; Debt, controls and real effects"
      },
      {
        "criterion": "No unjustified SQLite/control/parser waiver; preserve hard foreground and qualifying real gates",
        "reference": "reports/2026-11-05-dream-c1-native-quantum-disposition.md — Disposition and evidence classification; Source-grounded changes required"
      },
      {
        "criterion": "Frozen descriptor pre-seal cutoff and terminal coverage, formal/causal/real witness mapping",
        "reference": "decisions/d93377961365086-dream-native-quantum-and-debt.md — Producer classification and frozen wire; Minimal formal and real gates; interfaces/dream-term-c1.md"
      },
      {
        "criterion": "Full readbacks/digests, actual clean-whitespace check and active-source warning",
        "reference": "reports/2026-11-05-dream-c1-native-quantum-disposition.md — Artifact digests; Actual check and source-only race warning"
      }
    ]
  }
}
```
