---
type: task
title: 'DESIGN: Q2 member-window accounting vs the 1 ms quantum law with real Git/GPG helpers'
role: architect
allowed_paths:
- .bboxy/decisions/
- .bboxy/specs/
- .bboxy/interfaces/
- .bboxy/reports/
verify: test -n "$(ls .bboxy/decisions/*q2*quantum* 2>/dev/null)"
status: accepted
id: 01M6TE1P7SN2YGKXXG0Z5GJM1C
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-07T07:25:31.353Z
created: 2026-11-07T07:19:58.159Z
number: 93377961366233
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M6TE23PXGX5VN9SFP2H9NYDP
verify_result:
  command: test -n "$(ls .bboxy/decisions/*q2*quantum* 2>/dev/null)"
  exit_code: 0
  evidence:
  - criterion: decision with alternatives/choice/reasons/invariants/broken controls
    reference: decisions/d93377961366236-dream-q2-measured-quantum-and-wall-ceiling.md
  - criterion: ICD line updated
    reference: interfaces/dreaming-mcp.md Q formula sentence (Helper lifetime section)
  - criterion: spec lines updated
    reference: specs/dreaming-mcp.md Q2 helper paragraph + verification row 'Q2 helper window/debt coverage'
report:
  summary: 'Ruling (c), decision d93377961366236. Once closure is proven (all slots positively ACK-stopped or exited, roster closed, no unknown slot, counters read post-ACK, A_slot ≤ W_slot), a grant is charged Q = A_measured + C_active ≤ 1 ms. A_measured is the kernel per-process CPU delta plus reaped-descendant roll-up. Unproven coverage keeps the 65489 charge max(W,A)+C. Two new hard gates: W ≤ 25 ms wall per grant, and terminal conservation (post-reap wait4 ≤ Σ charged CPU + allowance). The 1 ms law, 9× debt and all other gates are unchanged. (b) is rejected because STOP→ACK is not proven-stopped time. (a) alone is rejected because W, up to 893 µs, is set by the scheduler. Evidence from the run 2 ledger (6b6ec65a…): max(A+C) was 664,918 ns over 30 grants; ΣA matched the ri tick delta within 9 ns; final wait4 matched final ri; the roster stage is the largest part of C. The "ordinary engine unit" resource_limit is separate and stays a failure. Runs 1 and 2 are not reclassified. ICD/spec lines in dreaming-mcp were amended by direct file edit, so their provenance signature is stale and needs an Oracle re-sign. The Oracle must amend the Q2 brief before implementation.'
  commit: null
  reported_by: seat:01M6TE23PXGX5VN9SFP2H9NYDP
  op_id: 01M6TEA1Z87CZKARJN58M2E7A7
  verify_revision: 0
  report: '[[reports/2026-11-06-q2-member-window-quantum-design.md]]'
provenance:
  digest: sha256:eb2787888761fa4f70bb7b8548dd7163a61b9f5c16691ab42e1ee6fdfefae64f
  frontmatter_digest: sha256:4d446b693bcfdbe7831a5a82ba3a8e9dd47c6cd21afb38086088e2de49f97781
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: h93VoIEoSlffG5lchYUkQ/z6OWnJlY+LGecAEyyQN/WEOYD2zztZ8TVw7pV1ZVTW7Fj8yeBSorKsFYp7fMbWBA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150227
  event:
    id: 01M6TEBWZ348HGNM2CTCPSD0GF
    sequence: 150227
    hash: b91a95ca0f096a966859aa9952861c3297e30bcc90c496d322034e23ae376990
---
# Can Q2's positive qualification ever satisfy the 1 ms law?

ROLE: ARCHITECT. Make a design decision with evidence. Do not implement.

## Evidence (two real runs on the FINAL frozen tuple 35d3e602 / C 7a753654 / probe 86c5c9b5 / receipt d66feb40)

- Run 1 (load ~12–20, builds at default QoS): exit 81. Ledger 18: `Q = max(457000, 93542) + 569291 = 1,026,291 ns`. resource_limit ×3 (first member-window, last ordinary engine overrun), settlement_unknown ×1 (final unreaped group kill unresolved).
- Run 2, the one QoS-conditioned retry (QoS landed on term main as the ee60f818 series; load ~9.5 because Q2's formal Python/Java ran at default priority): exit 81. Ledger 32: `Q = max(893000, 100167) + 547792 = 1,440,792 ns`. resource_limit ×3, settlement_unknown ×0. Full ledger `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6T0EZ07D77BYZK7YNHP2A32/components/positive-qos-retry/output/ledger.ndjson` (SHA-256 6b6ec65a…219c), with load/QoS snapshots before and after.
- Q2's own analysis (task 2026-11-05-dream-mcp-verified-snapshot, latest sections): W in these windows is **elapsed wall** from STOP to ACK. CPU inside the windows is tiny (e.g. A = 6.7 µs, C = 404 µs in an earlier run). Under decision 65489 an open window cannot be split, backdated or credited.

## Question

Q is charged as max(member window) + controller cost and is compared with the 1 ms per-unit law (65086). The controller's cost alone is about 0.55 ms. Any real helper STOP→ACK window above about 0.45 ms of wall time therefore fails, and macOS stop/ack latency with real Git/GPG members routinely exceeds that even at moderate load. Decide ONE of:

(a) The law is right and Q2 must restructure (for example, shrink controller cost C, or change how member windows are accounted), naming exactly what is allowed;
(b) member-window time that is wall time waiting on a stopped external process is charged to a separately bounded wait class, not to the 1 ms CPU quantum, with its own ceiling, evidence and broken control (similar to the 65086 native lane);
(c) something else, with reasoning.

Write the decision under `.bboxy/decisions/` (alternatives, choice, reasons, invariants affected, verification/broken controls) and update the relevant spec/ICD lines. Report with task_report.

Context: decisions 65086 (native quantum and debt), 65489 (member-window accounting), 65126 (helper creation/lifetime), 66050 (failed-stop proof), 66074 (owned quarantine); `.bboxy/specs/dream-term-c1.md`; `.bboxy/interfaces/dream-term-c1.md`.


## Report from seat:01M6TE23PXGX5VN9SFP2H9NYDP

Ruling (c), decision d93377961366236. Once closure is proven (all slots positively ACK-stopped or exited, roster closed, no unknown slot, counters read post-ACK, A_slot ≤ W_slot), a grant is charged Q = A_measured + C_active ≤ 1 ms. A_measured is the kernel per-process CPU delta plus reaped-descendant roll-up. Unproven coverage keeps the 65489 charge max(W,A)+C. Two new hard gates: W ≤ 25 ms wall per grant, and terminal conservation (post-reap wait4 ≤ Σ charged CPU + allowance). The 1 ms law, 9× debt and all other gates are unchanged. (b) is rejected because STOP→ACK is not proven-stopped time. (a) alone is rejected because W, up to 893 µs, is set by the scheduler. Evidence from the run 2 ledger (6b6ec65a…): max(A+C) was 664,918 ns over 30 grants; ΣA matched the ri tick delta within 9 ns; final wait4 matched final ri; the roster stage is the largest part of C. The "ordinary engine unit" resource_limit is separate and stays a failure. Runs 1 and 2 are not reclassified. ICD/spec lines in dreaming-mcp were amended by direct file edit, so their provenance signature is stale and needs an Oracle re-sign. The Oracle must amend the Q2 brief before implementation.

```json
{
  "op_id": "01M6TEA1Z87CZKARJN58M2E7A7",
  "commit": null,
  "report": "[[reports/2026-11-06-q2-member-window-quantum-design.md]]",
  "verify_result": {
    "command": "test -n \"$(ls .bboxy/decisions/*q2*quantum* 2>/dev/null)\"",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "decision with alternatives/choice/reasons/invariants/broken controls",
        "reference": "decisions/d93377961366236-dream-q2-measured-quantum-and-wall-ceiling.md"
      },
      {
        "criterion": "ICD line updated",
        "reference": "interfaces/dreaming-mcp.md Q formula sentence (Helper lifetime section)"
      },
      {
        "criterion": "spec lines updated",
        "reference": "specs/dreaming-mcp.md Q2 helper paragraph + verification row 'Q2 helper window/debt coverage'"
      }
    ]
  }
}
```
