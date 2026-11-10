---
type: task
title: Disposition repeated Q2 helper grant failures with sound member accounting
role: architect
worktree: /Users/feral/mydev/bboxy450-harness-dream-read-q2
allowed_paths:
- decisions/dream-q2-member-window-accounting.md
- specs/dreaming-mcp.md
- interfaces/dreaming-mcp.md
- reports/2026-11-05-dream-q2-member-window-disposition.md
verify: git diff --check
derived_from:
- '[[decisions/d93377961365126-dream-q2-helper-creation-and-lifetime-budget]]'
- '[[specs/dreaming-mcp]]'
- '[[interfaces/dreaming-mcp]]'
id: 01M6PW49N03HPN6NAVCERFCM7Q
status: accepted
delegated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T22:19:28.298Z
created: 2026-11-05T22:09:05.937Z
number: 93377961365420
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
assignee: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
artifacts:
- '[[01M6PWJ7EHC4EP0Q0J1QDW5D6R]]'
- '[[01M6PWJAFJFSB3RQ5ZCW3PYTVH]]'
verify_result:
  command: git diff --check
  exit_code: 0
  evidence:
  - criterion: Source-grounded exact member-window and stopped-unit disposition
    reference: decisions/d93377961365489-dream-q2-member-window-accounting.md
  - criterion: Narrow spec/ICD synchronization with complete readback hashes
    reference: reports/2026-11-05-dream-q2-member-window-disposition.md#knowledge-readbacks-and-communication
  - criterion: Finite model, causal controls and real qualifying witnesses preserved
    reference: decisions/d93377961365489-dream-q2-member-window-accounting.md#implementation-boundary-and-verification
  - criterion: Actual source/log hashes, failing baseline and verification limits
    reference: reports/2026-11-05-dream-q2-member-window-disposition.md#actual-evidence-and-source-anchors
report:
  summary: Created decision65489 and synchronized only Dream MCP Q2 helper-lifetime/matrix clauses, with complete exact continuation readbacks. Chosen rule uses conservative per-member pre-CONT→positive current-incarnation stop/exit windows, preserves unknown fork/exec coverage through closed roster, combines all active controller/pipe CPU within1ms, and permits separate paced corroboration only after a proven stopped/closed boundary with fresh9x debt. Status81/1.6825ms baseline remains failed; simply2→1 still1.1595ms fails. Retained source/log hashes and exact source race cutoff; missing coverage remains Unproven, all native/heap/control/four-job/30s/reap gates unchanged. Sent obligations to author/Rook, requiring Oracle live-task amendment. Report digest1d1cea1782abd0eb63f63112002f6356fddd6dce4ea6bf05d686be5d82c8e5c5. git diff --check exit0; no source/build/test/probe/performance or acceptance actions.
  commit: null
  reported_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  op_id: 01M6PWJSHEW74037DJFEWA03FR
  verify_revision: 0
  report: '[[reports/2026-11-05-dream-q2-member-window-disposition.md]]'
provenance:
  digest: sha256:2354b8403a6a8c917e10d67959d464c2c14a1f85997884efb2c40555cb9506c2
  frontmatter_digest: sha256:998812ccbc0d2a6dabb0d93d4253e4407eb537ed3b414a9d8d1ee03819ab218f
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: G6h8AgG4BAVGfKUsyz0csdDw7ssNrEWVCoJjN445C4yYj83dfHsVcBhulD7iWvV01RtJ2IQjWm0f0wGz6szlAg==
  log: .bboxy/log/2026-11-05.jsonl#145228
---
ROLE: ARCHITECT

## Goal and immediate direction
Resolve the concrete Q2 availability/accounting boundary reported in message01M6PW0R08S96TMVMKHZHPWTGB. Repeated1.013/1.039/1.6825ms failures mean the conservative full-window premise needs examination before another identical attempt. Preserve ordinary1ms application/controller/cleanup, complete CPU/debt/custody and all C1/MCP bounds. Evaluate the requested demonstrated per-member stopped/runnable-window accounting instead of blanket n=2*last-stop wall. Choose and write one source-grounded exact contract correction if sound; unsupported tools remain verification_unproven if coverage cannot be established. This authorizes design/spec/decision work now, not Executor source changes, threshold waiver or production availability. No operator or ACK wait. Q2 author continues independent work.

## Exact current evidence
Tree /Users/feral/mydev/bboxy450-harness-dream-read-q2, HEAD3d9fa84f (signed) plus active author edits; read/hash snapshot and name races. Retained target/dream-read-control-state-x6qosf3h/status=81, clang.stderr empty/author compile0, stderr.log last lines sample140: n2-wall1,046,000ns + active controller636,500ns =1,682,500ns; actual helper delta2,792ns, late_kernel0. This is Failed, not a qualifying helper success; root read actual source/status/log. Merely changing two to one would still charge523,000+636,500=1,159,500ns and FAIL. Do not promise per-member accounting alone fixes this sample.
Frozen probe copy target/.../proof-helper-probe.c around430–551: resume_at before groupCONT,50us spin thenSTOP; leader control_stop_ack gives observed_at, child roster/group census and stopped_signature_child may extend all_stopped_at; active_end sampled after those checks; charge=max(actual helper delta,2*(all_stopped_at-resume_at))+active_controllerCPU; separately paid stopped corroboration/late-kernel reconciliation. Current production c1.rs Helper.finish_grant around1111–1142 uses expected_members*elapsed+lane_cpu; helper_tick around1578–1665 runs full sample before closure. These are current-source anchors, not fixed-after-author edits. c1.rs sample around1145–1505 and probe control_observe/stopped_signature_child carry process/thread/incarnation/exec-stop details. read.rs/resolve.rs own helper custody/accounting/reap. Inspect exact pinned native capability/provisioned Git/GPG, no broad host scan.

## Required decision/interface content
Distinguish reserved possible process count from demonstrated runnable-owner coverage, actual helper CPU, conservative possible-runnable charge, active controller/pipe CPU and separately paced confirmed-stopped corroboration. If permitting member windows, specify exact conservative start/end clock boundaries (before continue through positive stop/exit confirmation), unknown/newborn/implicit fork/exec membership intervals and full stop-delivery latency. A last-seen sleeping/blocked member or zero sampled delta is not proof of no runnable interval. Potential descendant birth/exec changes cannot create an uncharged gap; before-roster uncertainty stays conservatively covered. Keep one group/two processes, one runnable thread per provisioned process, original PID/birth/task/audit incarnation and complete roster closure. Specify how observed kernel endpoints/source support the claim; do not infer exact stop time retroactively from a late sample.
Controller bookkeeping performed after a demonstrated stable stopped/closed roster may be a separate ordinary bounded paced unit ONLY with actual causal boundary, preserved authority/activity/lease/currentness and no hidden helper work. Full active controller CPU remains combined with contemporaneous helper charge; all stopped corroboration/pipe/late-kernel/retirement CPU remains charged once, finite,<=1ms per ordinary unit and creates fresh9x debt after observation. No native classification for lifecycle/proof wrapper, no free control, no double billing or erased debt. Never resume/proof/hash/parser while unfinished stop/coverage or any debt. Missing/reversed clocks, late kernel work, missing descendants/reap/coverage fail honestly with retained exact effect/quarantine; sampled CPU is reconciliation, not permission to waive a conservative bound.
Preserve65126 exact tool allowlist/hash/version/START_SUSPENDED provisioning,256 attempts/two members,30s/four jobs/foreground/control/heap/reap, bootstrap65166/65255/65259 and source/publication/strict wire/hints laws. Only inherited Dream MCP helper lifetime paragraphs/matrix change, no C1 native quantum/wire/IR/version/accepted component rewrite. If no sound finite rule fits source/OS, explicitly retain current provisioned tools Unproven and name the exact missing mechanism; do not encode blanket failure as a passing available lesson.

## Deliverable and evidence
Write narrow decision decisions/dream-q2-member-window-accounting.md (host returns numbered locator), sync only affected Dream MCP spec/ICD paragraphs and matrix, preserve refs/etags/frontmatter. Include finite model actions/state/owners/bounds/fairness and broken controls: unequal per-member stop delay; unknown birth/exec window; false stopped/blocked inference; controller moved before closure; late kernel CPU; missing/retired coverage; debt reset/overlap; actual CPU above conservative estimate; failed stop/clock/ABA. Require real provisioned Git→GPG qualifying positive and corrupt-proof negative, four jobs/control/foreground/heap/reap plus per-unit raw clock/coverage receipts and causal failures. No performance measurement or suite pass claimed by design review. No source/build/test/probe rerun, source edit, delegation or acceptance.
Send exact decision/obligation to Q2 author01M6PBF0EX2R5RGQAXNQTR96Y0 and Rook when ready; author needs Oracle live-task amendment before source change under new rule. Full append-only report reports/2026-11-05-dream-q2-member-window-disposition.md with task link, commits[], exact read/hash snapshots, choice/reasons, updated doc readbacks/digests, actual git diff --check exit, failures and unproven evidence. task_report once. No routine status/ACKs.

## Report from seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR

Created decision65489 and synchronized only Dream MCP Q2 helper-lifetime/matrix clauses, with complete exact continuation readbacks. Chosen rule uses conservative per-member pre-CONT→positive current-incarnation stop/exit windows, preserves unknown fork/exec coverage through closed roster, combines all active controller/pipe CPU within1ms, and permits separate paced corroboration only after a proven stopped/closed boundary with fresh9x debt. Status81/1.6825ms baseline remains failed; simply2→1 still1.1595ms fails. Retained source/log hashes and exact source race cutoff; missing coverage remains Unproven, all native/heap/control/four-job/30s/reap gates unchanged. Sent obligations to author/Rook, requiring Oracle live-task amendment. Report digest1d1cea1782abd0eb63f63112002f6356fddd6dce4ea6bf05d686be5d82c8e5c5. git diff --check exit0; no source/build/test/probe/performance or acceptance actions.

```json
{
  "op_id": "01M6PWJSHEW74037DJFEWA03FR",
  "commit": null,
  "report": "[[reports/2026-11-05-dream-q2-member-window-disposition.md]]",
  "verify_result": {
    "command": "git diff --check",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Source-grounded exact member-window and stopped-unit disposition",
        "reference": "decisions/d93377961365489-dream-q2-member-window-accounting.md"
      },
      {
        "criterion": "Narrow spec/ICD synchronization with complete readback hashes",
        "reference": "reports/2026-11-05-dream-q2-member-window-disposition.md#knowledge-readbacks-and-communication"
      },
      {
        "criterion": "Finite model, causal controls and real qualifying witnesses preserved",
        "reference": "decisions/d93377961365489-dream-q2-member-window-accounting.md#implementation-boundary-and-verification"
      },
      {
        "criterion": "Actual source/log hashes, failing baseline and verification limits",
        "reference": "reports/2026-11-05-dream-q2-member-window-disposition.md#actual-evidence-and-source-anchors"
      }
    ]
  }
}
```
