---
type: report
task: '[[tasks/2026-11-07-q2-member-window-quantum-design]]'
author: seat:01M6TE23PXGX5VN9SFP2H9NYDP
created: 2026-11-07T07:24:22.158Z
commits: []
verify:
  command: test -n "$(ls .bboxy/decisions/*q2*quantum* 2>/dev/null)"
  exit_code: 0
id: 01M6TE9R8N62Z4G2CR8PAHGC8W
updated: 2026-11-07T07:24:22.158Z
number: 93377961366237
created_by: seat:01M6TE23PXGX5VN9SFP2H9NYDP
updated_by: seat:01M6TE23PXGX5VN9SFP2H9NYDP
authorship:
  signer: seat:01M6TE23PXGX5VN9SFP2H9NYDP
  kind: seat
  public_key: ekEPUX1K5XlMq/b2IQsO5MVAYyZdO/cYw7QmNPKD22s=
  process_generation: '1'
provenance:
  digest: sha256:7d682986613bd5b8c7bc5e19463c4ab9e27c6cf72b8d627bea9889494dee4c7e
  frontmatter_digest: sha256:e0d17065df2591cc8ffd9623ac93eb0958d35eea229c1cc9adf3df600fac78a6
  signer: seat:01M6TE23PXGX5VN9SFP2H9NYDP
  sig: LAgTeuolsUO6rvNT1xBjlDysAMgBqoemLUr/ZqIW/eeJEBrzykqmhwYXjjJKE6uFHxcdvOOrw3TZsSf1gtGGCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150217
  event:
    id: 01M6TE9SDQRD0NFS84ZX33J5AD
    sequence: 150217
    hash: f10aa34ecce578f05c178a75cda08c624c59bd3f6d65bf96ac746b74d14d82b8
---
# Q2 member-window vs 1 ms quantum: ruling (c)

Decision: [[decisions/d93377961366236-dream-q2-measured-quantum-and-wall-ceiling]].

## Ruling

Neither (a) alone nor (b). Chosen (c): once closure is proven (every slot ACK-stopped or exited, roster closed, no unknown slot, counters read post-ACK against the pre-CONT baseline, A_slot ≤ W_slot), charge `Q = A_measured + C_active ≤ 1 ms`. A_measured is the kernel per-process CPU delta plus reaped-descendant roll-up. Unproven coverage keeps the 65489 window charge `max(W,A)+C`. New hard gates: `W ≤ 25 ms` wall per grant, and terminal conservation (post-reap `wait4` user+system ≤ Σ charged CPU + allowance). The 1 ms law, 9× debt and every other gate are unchanged.

Why not (b): during STOP→ACK the member is not yet proven stopped, so dropping W without a measurement removes the only CPU bound. Why not (a): W reached 893 µs, and that figure comes from the scheduler, not from Q2 code. Shrinking C cannot make it robust.

## Evidence (run 2 ledger, SHA-256 6b6ec65a…219c, all 32 lines read)

- 30 grants: W 213–893 µs, A 63.6–207 µs, C 216–548 µs; max(A+C) = 664,918 ns; A/W ≤ 0.46.
- ΣA 2,957,509 ns vs ri tick delta 2,957,500 ns; final wait4 3,063,000 ns vs final ri 3,064,333 ns. The counters reconcile.
- `roster` closure stage is the largest C term (up to 398 µs CPU).
- result.json: the third resource_limit, "ordinary engine unit exceeds one millisecond", is outside this formula and stays a failure.

## Changes

- New decision d93377961366236 via knowledge_write (signed by this seat).
- `interfaces/dreaming-mcp.md`: amended the Q formula sentence (Helper lifetime section) with basis, wall gate, conservation.
- `specs/dreaming-mcp.md`: Q2 helper paragraph and verification-matrix row for Q2 helper window/debt coverage cite the decision, its three invariants and six broken controls.

## Gaps

- The spec/ICD edits were made as exact file edits, not knowledge_write (bodies are 25 and 51 KiB). knowledge_fetch shows the new text, but the documents' provenance signature still names the previous signer/digest. The Oracle may re-sign them by a knowledge_write round trip or commit.
- No build, model or test ran; this is a design decision. Runs 1 and 2 stay exit 81; nothing was reclassified.
- Not examined: run 2 debt totals (~18.6 s); 65654 out-of-group daemon exclusion (still assumed).
- Oracle must amend the live Q2 brief before the author implements (c1.rs basis/A_measured, read.rs conservation, probe/tests witnesses, model invariants).
