---
type: report
title: C1 fixed parent/child live heap allocation
task: '[[tasks/2026-11-05-dream-c1-heap-allocation]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T12:20:14.662Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6NTE2Y3TTWJ3C7PHMWZW28K
updated: 2026-11-05T12:20:14.662Z
number: 93377961364575
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:d78780b500fbd3ae50b1a0982e787393619eab4a3a8c6aac178672bb46d1ab74
  frontmatter_digest: sha256:51ca9dba85e1d6302287eb427fa59fe2824168d39ba06c1f04c80334e6026b0a
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: ibc2ZT+UPHjpipVXaoyah9zlbw+R58hm/tL4Yql8hzsapNhK54HSRiaTCmmazCoHKskSAaXUTAOFlzYjBtzzBQ==
  log: .bboxy/log/2026-11-05.jsonl#135365
---
# C1 live heap allocation

Resolved [[tasks/2026-11-05-dream-c1-heap-allocation]] with **parent 32 MiB + child 96 MiB = unchanged 128 MiB/job**. Each owner reserves 2 MiB inside its share exclusively for control/status/cancel/error/settlement (work shares 30/94 MiB). No borrowing, wire/version change, larger threshold or extra worker.

Changed only one Working memory table row in [[specs/dream-term-c1]] and added one budget paragraph in [[interfaces/dream-term-c1]]; exact readbacks matched. Created [[decisions/d93377961364572-dream-c1-heap-allocation]] with allocation coverage, preallocation enforcement, overflow behavior and joint-process witnesses.

## Grounding and enforcement gap

| Snapshot | Source evidence |
|---|---|
| Producer P /Users/feral/mydev/bboxy450-harness-dream-c1, HEAD 5eec82a33959b764b32cbd7692cac2482fd3e009, clean on initial inspection | src/dream/c1.rs SHA256 8235462e51025684d2fe1e661f786b42e0626d81985dfabd5185fd242d95f199; :30 allows independent 128 MiB; :114-132,:221-241 sample heap around units with nominal 16 MiB margin; :719-746 retains manifest/log/D0 state and :892 allocates control framing. |
| Consumer C /Users/feral/mydev/bboxy450-term-dream-c1, HEAD ad243195daf74153756308346d01a56390c1261c, dirty author tree | crates/bbt-mux/src/logic/dream_c1.rs SHA256 11bb0a2ab859298937c0cac850177edfa450ead08ebc39d773329d876238cc8b; :37 permits independent 128 MiB; :604-611 records selected estimates; :762-768,:997-1003,:1127-1131 estimate parser/output storage around ordinary serde/serialization. |

These mechanisms do **not** yet prove the combined limit. The split is implementable with fail-closed bounded local reservation: reserve a source-proved maximum simultaneous live capacity before allocation, include transient growth/clone/serde/native overlap, retain charges through actual free and cross-thread transfers, and use fallible bounded builders. Missing coverage fails resource_limit before effects; changing constants/advisory counters alone is insufficient. All job/background/control buffers count, including child startup heap and attributable native/SQLite buffers. Parent excludes unrelated host work, not its C1 helpers. No host abort or allocator-unwind strategy is authorized.

The decision requires actual live heap attribution/coverage separately from reservation high-water, cumulative allocated bytes and RSS. Separate max(parent)+max(child) is only a conservative bound, never a synchronized measured peak. Reservation invariants establish the all-times sum; sampled metrics cannot. OS stacks/mappings/kernel/file cache remain separate memory evidence.

Required witnesses: actual parent plus producer subprocess with multi-MiB input; independent work-budget exhaustion on each side, transient parser/realloc/serialization overlap, causal missing-reservation control, unchanged sources/immutable results, responsive cancellation/status/settlement from reserved storage, and four simultaneous admitted jobs. Existing CPU/duty/latency/formal/wire requirements and canonical make test-dream-c1 gates remain. No measurements, builds or tests were run here; implementation coverage is still the authors' obligation.

## Handoff and verification

Immediately sent the recorded shared decision/spec/ICD and precise mechanisms/witnesses under existing briefs to producer (01M6NTBYD388X4JP9YP5A6FVVQ), consumer (01M6NTBYDETD4TTRCEH30Z5JG6) and Rook (01M6NTBYDQBFEP2ZPDPTVDERM6). Receipts showed native-queued, model consumption unverified. No approval or acknowledgements requested.

Readback digests:
- Spec: ac351db8c0ce9ea7dcf330f6510770fd7c7ee8a7189746d2dd1453de9ab55a21.
- ICD: 5cf6baaeda30814eac41f494cad9246aec181bee0dc82ce80d5889b9b716caf6.
- Decision: fceeae2fbbbada9d1c2e18c8ee9344112153e723605cb99ee289b0d7d1230207.

At 2026-11-05 12:19:27 UTC, root git diff --check returned **exit 0**, no output (tracked diff only; untracked knowledge records excluded). Read-only date/Git/source/hash commands returned exit 0; exploratory missing-path searches and a truncated bulk read were not treated as complete evidence. Full current spec was fetched with its continuation and stable etag; no stale body was overwritten. Root HEAD ad243195daf74153756308346d01a56390c1261c; status contained only untracked knowledge records at cutoff. No source/submodule edits, builds/tests, new assignments, commits, merge or publish. Source snapshots can stale as authors continue.
