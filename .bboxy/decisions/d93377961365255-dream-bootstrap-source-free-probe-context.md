---
type: decision
title: Use explicit null context only for source-free bootstrap capability probes
links:
- to: '[[decisions/d93377961365166-dream-bootstrap-private-permit-handoff]]'
  rel: clarifies
- to: '[[tasks/2026-11-05-dream-c1-consumer]]'
  rel: implementation
id: 01M6PNT0F117NM3W74JW4FC5KD
updated: 2026-11-05T20:26:31.521Z
created: 2026-11-05T20:18:36.467Z
number: 93377961365255
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:25fd37b89f2d6703c6360f577ee58246be047f4cd3876e1913f37ef229169f85
  frontmatter_digest: sha256:e239170d05f1fda778ed13b5117f9854c93c639b8756121d74a7d93c8c4c6d3a
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: mFqUu0AThsrq5mjSW0nIgS6PUxsEOFX8pHpxUfEOkOEuoI4lpGi1KKumUM1xygKeo0BgwxM8xWuH2FjcWglKDQ==
  log: .bboxy/log/2026-11-05.jsonl#142013
---
Date2026-11-05. Oracle immediate disposition for consumer message01M6PNP40S2JRMG874QQSDQDBH under [[decisions/d93377961365166-dream-bootstrap-private-permit-handoff]]. Runtime capability probe occurs before a managed seat/root Authority exists; delaying it until a closed seat would not establish TermC1 launch/close ownership before that session. No source or authority may be invented to fill this field.

Every source-free probe=true binding serializes context:null; probe=false requires full captured current public context. Serialize the context field explicitly as JSON null in C(binding), never omit it or substitute an empty/fixture/previous-seat context. Keep fresh original probe op_id/run_id/nonce/birth, nonzero monotonic host launch incarnation, exact purpose and approved worker/source/compiler/hash/bootstrap profile; all privatepipe/128bytepermit/lease/deadline/debt/Hello/EOF/reap and<=1/account rules remain. Context-null capability probing grants no seat/root/source/KB/provider permission. It is not a fallback for a real job or missing public proof.

Producer treats probe=true as permanently non-startable: any Start (including exact matching IDs) refuses source-free and settles the actual parked owner; no manifest/source/provider read. Parent never sends Start on a probe, retires lease after qualifying Hello, closes normal stdin and proves actual exit/reap. It may cache only bounded runner capability for the exact pinned source/binary/profile/incarnation; no authority or source eligibility cache. Unknown old binary/profile still cannot be executed ungated. probe=false requires full captured current public context and valid original authority/source closure; null/missing context refuses before birth, with no invented/public-status/custody/fixture fallback. Runtime may discover/publicly bind the real seat context only later through existing admission.

Alternative was deferring probes until live closed-seat Authority; rejected because it misses early typed TermC1 ownership and introduces unnecessary identity/provider dependency into source-free capability discovery. No permit binary layout, worker JSON/schema/IR/algorithm/control version, budget, actor grant or permission expansion. The binding's context variant is a private canonical-hash input distinction already selected by probe flag. Q2 read-preparation probes inherit the same capability-only distinction; real published-read jobs keep full proof/context requirements. Helper65126 remains separate.

Extend canonical binding vectors and real source-free probe witness for explicit null + no manifest even on hostile Start, omitted context, wrong probe bindings and null/missing REAL-job context negatives, fresh identity/ABA/duplicate/deadline/EOF, known profile and actual join/reap. Existing gate/formal/process commands remain. This is an implementation authorization/contract clarification, not a passing source/runtime result. Architect synchronizes65166 and current spec/ICD narrow probe clause, preserving its baseline report with an append-only supplement; Oracle amends all live component briefs.