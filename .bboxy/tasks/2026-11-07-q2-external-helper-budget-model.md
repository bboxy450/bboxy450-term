---
type: task
title: 'DESIGN: should external background-QoS helper processes be 1 ms-sliced at all? (5 qualification rounds failed on physical timing)'
role: architect
allowed_paths:
- .bboxy/decisions/
- .bboxy/specs/
- .bboxy/interfaces/
- .bboxy/reports/
verify: test -n "$(ls .bboxy/decisions/*helper*budget* .bboxy/decisions/*external-helper* 2>/dev/null)"
status: accepted
id: 01M6VASDCG1SJBSGRVB2ACJMYJ
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-07T15:55:08.982Z
created: 2026-11-07T15:42:14.881Z
number: 93377961366277
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
verify_result:
  command: test -n "$(ls .bboxy/decisions/*helper*budget* .bboxy/decisions/*external-helper* 2>/dev/null)"
  exit_code: 0
  evidence:
  - criterion: Decision written under .bboxy/decisions
    reference: .bboxy/decisions/d93377961366280-dream-q2-external-helper-budget.md
  - criterion: Clauses of 65086/65489/66236 that change and stay
    reference: d93377961366280 section 'Clause disposition'
  - criterion: settlement_unknown ruling
    reference: proof-helper-probe.c:4383-4389 (sha e23f68d3); result.json settlement group_gone=true; local Darwin 27.0.0 kill(-pgid) EPERM witness
  - criterion: Spec/ICD updated through knowledge tools
    reference: specs/dreaming-mcp.md digest fbf88448; interfaces/dreaming-mcp.md digest ab56be9c; both bodies diff-equal to expected
  - criterion: Gates and broken controls
    reference: d93377961366280 section 'Verification' (Q2HelperTotalBudget, 8 controls)
report:
  summary: 'Chose option (b), recorded in decision d93377961366280-dream-q2-external-helper-budget. External Git/GPG helpers are no longer sliced into 1 ms grants. The new law: posix_spawn with SETPGROUP and background QoS, with an observed priority of 4 as a hard gate. CPU caps are ≤250 ms per command and ≤3 s per job, enforced by live RI reads with SIGKILL, and the leader''s own wait4 is the terminal verdict. The job pays 9× rest debt on that CPU after the reap. Wall time is bounded by the existing 30 s deadline. Memory keeps the existing shares plus a terminal ru_maxrss ≤16 MiB. Foreground is a single SIGSTOP pause inside the existing 100 ms gate. The 65086 1 ms law is unchanged for all work on our own threads. The 65126 per-grant section is superseded, and 65489 and 66236 are superseded for external helpers, including 66236''s clarifications 1 and 2, T and N_reaped. The decision''s clause table lists what stays. The reason: in run 4 the controller spent more CPU (3.10 ms) supervising than the helper spent working (2.36 ms), and slicing measures the run queue, not UI responsiveness. Settlement_unknown ruling: an expected exit race, misrecorded by a probe bug. kill(-pgid) returns EPERM on a zombie-only group, observed on this machine. The probe at :4383-4389 discards the errno and never retracts the unknown after its own reap and group-gone. The fix is to apply 66050''s pending-terminal state to the final kill. Run 4 stays FAILED. I updated the spec paragraph and its verification row and the ICD lifetime paragraphs through knowledge_write. Both bodies diff equal against the expected replacements. The new model Q2HelperTotalBudget has 5 invariants and 8 broken controls, each with a real witness. Nothing was implemented or committed. One host defect found: knowledge_write drops created/created_by on update unless they are resupplied; I restored them on the spec.'
  commit: null
  reported_by: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
  op_id: 01M6VBG5YVDCTA2ET6Z0TBZG13
  verify_revision: 0
  report: '[[reports/2026-11-07-q2-external-helper-budget-model]]'
provenance:
  digest: sha256:8a7f0d84f4dcb86515ab610c11989e7fa8c7fd12073b43fef9e8193c7cd689bf
  frontmatter_digest: sha256:16dc3c21b5ca9c46ceab4326509531ec06a780bec0a07db719816fac056f5fea
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: x88ZR+3gQYCsdYYynFdaXDfWjCen2IrIErRG08AtcSkS6AvEy7JCQvJWFzv6UANu84h4xG3uHH4jL6jM/vcmDQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150387
  event:
    id: 01M6VBH1WQRX03NC5NAHF5Y8PQ
    sequence: 150387
    hash: 725dc4b1a1e0537406e9e25e3c133c8eb310695c7d92acd10ebf5cc99c262258
---
# Is per-grant 1 ms slicing of an external helper process the right model?

ROLE: ARCHITECT. Make a design decision with evidence. Do not implement.

## History (Q2 proof-tools qualification, all on real Git/GPG helpers)

1. `Q = max(W) + C` charged wall STOP→ACK: 1.03 ms and 1.44 ms → decision 66236 (charge measured CPU once closure is proven).
2. 66236 run: an ordinary controller unit took 1.165 ms, and conservation was short by 85 µs → units split; fixed teardown T = 150 µs.
3. Fixed-T run: the member burned 933 µs in a ~1.1 ms CONT window → adaptive grant sizing.
4. Adaptive run (frozen 84158341 / e23f68d3 / 05eefb20 / 3974b380), run 01M6VAGHD4Y6MXHSASDMN15XE9, ledger SHA 0ee1e452…: grant 10, **Q = A 1,093,125 + C 171,250 = 1,264,375 ns**. The scheduler chose a 126 µs window, but **t0→STOP call = 1,087,000 ns** because the low-QoS controller thread was preempted while the helper ran; STOP→ACK 107 µs. Conservation PASSED. A settlement_unknown appeared (final unreaped group kill unresolved) even though the result records group gone.

So user-space SIGSTOP/SIGCONT slicing driven by a LOW-QoS controller (Dream runs at nice+10/utility by design) cannot guarantee a 1 ms per-grant bound on a loaded heterogeneous-core Mac. Each fix exposes the next physical limit.

## Question

The 1 ms cooperative quantum (65086) protects shared threads (UI, mux, reactor, control lanes) from monopolization. Git and GPG run as SEPARATE processes, which the OS already isolates by QoS class (background, E-cores, throttled IO). Decide:
(a) Keep 1 ms per-grant slicing for external helpers, and specify how STOP timing is guaranteed (for example, elevate ONLY the controller's timing thread to user-interactive during an open window, with a precise kernel timer), with the cost to the UI/QoS budget;
(b) Replace per-grant slicing for EXTERNAL helper processes with OS-enforced isolation (the helper at background QoS, with its process group in an owned session) plus TOTAL budgets: a CPU cap per job (measured by wait4/RI), a wall deadline (the existing 30 s), memory caps and honest overrun refusal. Keep the 1 ms law for all in-process work on shared threads. State exactly which 65086/65489/66236 clauses change and which stay;
(c) something else.

Weigh simplicity (operator: "LESS IS MORE", CPU and disk are scarce), the evidence above, what is actually protected (UI responsiveness), and verifiability (state the gates and broken controls). Also rule on the settlement_unknown above (bug or expected?). Write the decision under `.bboxy/decisions/` and update the spec/ICD lines through knowledge tools, not direct file edits. Report with task_report.

Context: decisions 65086, 65126, 65489, 66026, 66050, 66074, 66236 (with its Oracle clarifications 1, 2 and N_reaped); specs/dreaming-mcp.md, interfaces/dreaming-mcp.md, specs/dream-term-c1.md; tasks/2026-11-05-dream-mcp-verified-snapshot (Q2) and 2026-11-05-dream-proof-tools-provisioner (builder).

**Oracle correction:** in run 4, "the low-QoS controller thread was preempted" is the Oracle's INFERENCE. The ledger proves only the clocks (scheduler requested 126,042 ns; pre-CONT→STOP-call 1,087,000 ns; STOP→ACK 107,000 ns). The builder notes that clocks alone do not establish cause. If the decision depends on the cause, say how to establish it (for example, thread run-state or scheduling samples of the controller during the window).


## Report from seat:01M6VASNV9NAZRZ5JHQ7ER9PPX

Chose option (b), recorded in decision d93377961366280-dream-q2-external-helper-budget. External Git/GPG helpers are no longer sliced into 1 ms grants. The new law: posix_spawn with SETPGROUP and background QoS, with an observed priority of 4 as a hard gate. CPU caps are ≤250 ms per command and ≤3 s per job, enforced by live RI reads with SIGKILL, and the leader's own wait4 is the terminal verdict. The job pays 9× rest debt on that CPU after the reap. Wall time is bounded by the existing 30 s deadline. Memory keeps the existing shares plus a terminal ru_maxrss ≤16 MiB. Foreground is a single SIGSTOP pause inside the existing 100 ms gate. The 65086 1 ms law is unchanged for all work on our own threads. The 65126 per-grant section is superseded, and 65489 and 66236 are superseded for external helpers, including 66236's clarifications 1 and 2, T and N_reaped. The decision's clause table lists what stays. The reason: in run 4 the controller spent more CPU (3.10 ms) supervising than the helper spent working (2.36 ms), and slicing measures the run queue, not UI responsiveness. Settlement_unknown ruling: an expected exit race, misrecorded by a probe bug. kill(-pgid) returns EPERM on a zombie-only group, observed on this machine. The probe at :4383-4389 discards the errno and never retracts the unknown after its own reap and group-gone. The fix is to apply 66050's pending-terminal state to the final kill. Run 4 stays FAILED. I updated the spec paragraph and its verification row and the ICD lifetime paragraphs through knowledge_write. Both bodies diff equal against the expected replacements. The new model Q2HelperTotalBudget has 5 invariants and 8 broken controls, each with a real witness. Nothing was implemented or committed. One host defect found: knowledge_write drops created/created_by on update unless they are resupplied; I restored them on the spec.

```json
{
  "op_id": "01M6VBG5YVDCTA2ET6Z0TBZG13",
  "commit": null,
  "report": "[[reports/2026-11-07-q2-external-helper-budget-model]]",
  "verify_result": {
    "command": "test -n \"$(ls .bboxy/decisions/*helper*budget* .bboxy/decisions/*external-helper* 2>/dev/null)\"",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Decision written under .bboxy/decisions",
        "reference": ".bboxy/decisions/d93377961366280-dream-q2-external-helper-budget.md"
      },
      {
        "criterion": "Clauses of 65086/65489/66236 that change and stay",
        "reference": "d93377961366280 section 'Clause disposition'"
      },
      {
        "criterion": "settlement_unknown ruling",
        "reference": "proof-helper-probe.c:4383-4389 (sha e23f68d3); result.json settlement group_gone=true; local Darwin 27.0.0 kill(-pgid) EPERM witness"
      },
      {
        "criterion": "Spec/ICD updated through knowledge tools",
        "reference": "specs/dreaming-mcp.md digest fbf88448; interfaces/dreaming-mcp.md digest ab56be9c; both bodies diff-equal to expected"
      },
      {
        "criterion": "Gates and broken controls",
        "reference": "d93377961366280 section 'Verification' (Q2HelperTotalBudget, 8 controls)"
      }
    ]
  }
}
```
