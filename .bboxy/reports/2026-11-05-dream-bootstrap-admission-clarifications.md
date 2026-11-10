---
type: report
title: Superseding bootstrap probe-context and final-admission clarifications
task: '[[tasks/2026-11-05-dream-bootstrap-admission-boundary]]'
supersedes: '[[reports/2026-11-05-dream-bootstrap-admission-boundary]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T20:38:04.647Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6PPXMB7TQ3P9VVDRP0YABJX
updated: 2026-11-05T20:38:04.647Z
number: 93377961365264
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:a50eeb2e6735a3a8a210280bddc24fbb30952d3dfe0dacbee008371d332a551a
  frontmatter_digest: sha256:f98c5815d8d2f1df29a5d403331cd941fa8991d2e6be09e91da2c56132c5e237
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: A27AUzuyXSSrrqSjqRzjDoJfjIZ8eujAxEibOWNjOJpZOzxit6MGY6ey0HakuJDeFBffBFhR3mtoVEr4Su2QAA==
  log: .bboxy/log/2026-11-05.jsonl#142367
---
# Bootstrap clarification supplement

Task [[tasks/2026-11-05-dream-bootstrap-admission-boundary]]. Supersedes [[reports/2026-11-05-dream-bootstrap-admission-boundary]] only for these two clarified contract points/current digests. Initial append-only report remains unchanged (52e9b2edf410017206a053729b3f5b5c079a996d27fcc6a265f6a3ae63bd2e5c). commits:[].

## Exact Oracle rulings synchronized

1. [[decisions/d93377961365255-dream-bootstrap-source-free-probe-context]] (eab3a2f02228002dae1f0b58d59617082f635a28ab307f2bb0121e2f00cbd2e6): probe=true serializes context:null explicitly in private C(binding); fresh original op/run/nonce/birth, nonzero host launch incarnation and approved artifact/profile, no invented pre-seat/root authority. Child permanently refuses ALL Start source-free, including matching IDs; parent only paid Hello→EOF→actual reap. Real probe=false requires full current captured public context/closed-source authority before birth, never null/missing-proof fallback.
2. [[decisions/d93377961365259-dream-bootstrap-final-public-admission-control]] (be87f09614b620f371ff6dbc31800b5bf5b5ffab0376547e373b2f83413d0aa1): ONLY final permit and authorized Start handoff attempts classify prepared fresh PUBLIC proof+tiny transition/write as one inclusive≤1ms CONTROL step without recursive9x debt. Exact≤7 rows×16KiB bounded BEFORE fetch/hash (≤112KiB), fixed actor/file/revision/time/provider checks, prepared readonly session/cache, isolated owner/protected controls. One final check/attempt; no polling/cached/atomic-only permission or await reuse. Setup/other guards keep original work accounting. Unknown/busy/oversize/clock/>1ms refuses/revokes with effects retained; probe has no fake final actor proof. No generic SQLite/serde/source/payload/binary hash or credential/signature/provider-process exception.

No contradiction between these rulings remains. All prior pipe/layout/source-law/authority/ABA/CPU/native/debt/heap/readiness/30s/settlement/hints/wire/version text is preserved. No source, implementation assignment or acceptance change.

## Exact refs / etags / full readbacks

All writes used current fetched full ref (id,number,slug,path,digest,updated_at) and etag; existing frontmatter retained except host update/provenance stamps. Etag equals returned record digest. Full bodies equal submitted text, continued through next:null.

| Record ref | Prior etag/digest | New etag/digest / pages |
|---|---|---|
| decisions/d93377961365166-dream-bootstrap-private-permit-handoff.md (01M6PMWNX1CH778V5S0TY7J7K7, #93377961365166) | ee7aca9df05a97d83ab1082971c61f84fd62641a5a19d21b714489aa1036b47e | 293a94ff5b5bb1d838aa91666247c6f4e758d91ad8ba30eee5e01096cc01a9be /2 |
| specs/dream-term-c1.md (01M6NP71V62A4WE211ZW5FCRGH, #93377961364370) | d1abb6706471c4a6345a0c804815126ffa3e51170c363605b118ade6f40f8879 | b78d8fa46a88144c99ec25484acb7330e6cee468eba4b7332937e6e56ae9ad5d /2 |
| interfaces/dream-term-c1.md (01M6NP15EEZYG86TF9Z6GDMER7, #93377961364368) | 6d2a062e3161450ae00500574e7f3500427e48b1f35ceb315dec6d0c46d00118 | c79408f673d4f585c62210a18237762507619dd275c18badc2af6c008ccb0098 /2 |
| specs/dreaming-mcp.md (01M6P1J9AW2MJ6YN45BYMCDJNK, #93377961364658) | 13d880c67a67e9bf27e928b7e3fa944773bce3b13354d4377143e857e91aaf6a | eafe27fd9e0dea9aa488ddd15dfc581172df2981320d1eabd1d3d402fd5a1432 /2 |
| interfaces/dreaming-mcp.md (01M6P1DRCHV10ZBJRVR3NF6PGH, #93377961364656) | 67bdba9b7d70b13a4bdcea58cf5150a5248f4a157c3d1df2c6149660a68bb93f | 04dd34ee61e651a7b1d20e583b36a31cea0a787ca8d8027cfe2c43df9b37ddeb /3 |

65166 changed only its binding and finite-bookkeeping paragraphs; the four spec/ICDs received only the same two linked clarification paragraphs. Reversing exact edits reproduces every prior complete body byte-for-byte. Task role/verify/lifecycle and initial report were not edited by this seat.

Artifact_claim updates for65166 and C1 spec/ICD succeeded. Updated specs/dreaming-mcp claim01M6PPV8550YDFWQ5PKZHZ3TQ6 was refused “artifact record projection pending”; operation_status confirmed terminal rejected/retry_allowed:false. It was not redispatched or bypassed. This is artifact-metadata projection, not a document write/readback failure; the original responsibility claim already exists. Updated interfaces/dreaming-mcp claim01M6PPX6SVMEKYMNK689AE304C received the same refusal; operation_status also confirmed terminal rejected/retry_allowed:false. Neither was repeated. No green projection claim is inferred from storage.

## Source grounding / snapshot limits

Read-only /Users/feral/mydev/bboxy450-term-dream-c1, HEADd156e0cbae84aa5a9af768220d8200cf62c200de plus active author source/verification/attribution changes and untracked dream_bootstrap.rs. Nothing staged/edited/committed by this seat.

- registry/dream_c1.rs:1259-1307 Runtime::new creates capability probing without any seat/root Authority argument; grounds null-context probe. This in-flight source is not certified against approved-before-birth/permanent-no-Start law by a read.
- identity/status.rs:184,853-913 has PUBLIC_ROW_LIMIT16384, SQL CASE blob type/length bounds before returning the cell, checked borrowed-cell digest, six fixed keys plus optional device. No credential/account row contents were accessed.
- registry/dream_c1.rs:965-1015 uses bounded context fields, try_with_current_session and pre/post provider snapshots; :1018-1041 rows_match uses exact primary-key/CASE scalar SQL; :1043-1079 prepares the readonly session/cached statement/callback. Preparation stays ordinary work. These mechanisms are not a final inclusive1ms measurement.

At20:33–20:35:33Z core hashes unchanged: status.rs32de51ac2e6726eb64b7056e55a482b143d795861e53e4ee2a5e4945cc2071bf; registry/dream_c1.rsb6ada27e2e86dcbd6d4d3370436af576848c44a439c60f16003aabf9ed78da5e. Untracked wake-up dream_bootstrap.rs514b06fe11c51cfd0a22ede23117e9ae7fc251e5f89df47fa4b0c04ae62c043d was inventoried, not certified. Later source can stale anchors; root owns final-source acceptance.

## Actual verification / remaining evidence

git diff --check in exact /Users/feral/mydev/bboxy450-term-dream-c1 — exit0, no output; adjacent snapshot20:35:33Z. Tracked diff only; no untracked module correctness claim. No build/test/formal replay/runtime/native-app/provider/service/credential/delegation action.

Required existing witnesses add explicit-null/omitted/wrong-probe canonical vectors, permanently source-free hostile matching Start rejection, real-job null/missing refusal, one final check count, post-debt row/inode/grant/provider/ABA tamper, held SQLite/selection fail-closed with independent Status/Cancel, >1ms/unknown final control cost preserving actual Child/lease effects, no repeat/debt reset. Oracle's445–906us baseline is unproven final performance, not a pass here. Existing four-job/foreground/native-frame/heap/real-reap and canonical Make/model/process gates remain. This is contract/task evidence; amended component tasks still own implementation/current-source gates.
