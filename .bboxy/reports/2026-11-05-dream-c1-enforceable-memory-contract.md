---
type: report
title: C1 corrected working-storage, overhead and public-authority contract
task: '[[tasks/2026-11-05-dream-c1-enforceable-memory-contract]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T12:43:30.077Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6NVRNTJHECBDJD95WP8DRKY
updated: 2026-11-05T12:43:30.077Z
number: 93377961364583
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:3ab1ec251e197b97dcff5146e454b374bb160368d3bf325f975069de020737e6
  frontmatter_digest: sha256:9ca183ac1c38afbf56026e2f84375f46bc64820694a615169ec3555293f4e182
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: VPNxnM587D6roP3LCYbkuWaMxHZJQesYzrcpbVru+1exKE1LMxgDkN145tEfD3vGp6aXRsRO2Cmfb8LWWk3DDQ==
  log: .bboxy/log/2026-11-05.jsonl#135483
---
# C1 enforceable memory and public-authority correction

Completed [[tasks/2026-11-05-dream-c1-enforceable-memory-contract]], including amended brief digest 583e8b9a0fa9f6409e473ac044c0b66034728a3092f0c545dd56f5e453e045f8.

[[decisions/d93377961364581-dream-c1-enforceable-memory-contract]] supersedes the unsupported universal guarantee in [[decisions/d93377961364572-dream-c1-heap-allocation]]. Updated affected budget/model-matrix wording in [[specs/dream-term-c1]] and memory/public-authority paragraphs in [[interfaces/dream-term-c1]]. The earlier decision remains historical.

- Keep 32 MiB parent / 96 MiB child and protected 2 MiB control shares. Enforce declared application buffer/retained-state capacity credits before controllable growth, with fallible handling and overlap accounting. Inside work shares, 8/16 MiB helper headroom is an engineering allocation validated by witnesses, not an allocator proof or bytes-times-factor.
- Opaque serde/BTree/regex/SQLite/native/thread/runtime overhead is bounded-use, measured implementation coverage. Keep observed parent attributed live working heap ≤32 MiB and full child observed heap ≤96 MiB as failing regression ceilings. Withdraw the absolute all-times whole-process/native 128 MiB claim. Distinguish live/cumulative/credits/native/RSS and separate peak sums from synchronized samples.
- Fix practical JSON depth/container/count, retained-state, owner/thread/connection/cache/row limits. Keep actual parent+child multi-MiB/overflow/four-job/control witnesses, causal missing-reservation control and all CPU/duty/latency/closure/settlement gates. Models prove declared credits only; unmodeled opaque allocations must be listed.
- No host-global SQLite hard_heap_limit or allocator switch. SQLite native total is not capped by lookaside/cache settings. No default hard OS-memory mechanism or arbitrary Rust/native allocation proof is demanded.
- C1 consumes a host-authenticated public incarnation/revocation fence, with actual public-proof/revision checks and monitor/final-presentation revalidation. It never calls custody/signing probes or identity.status/admission fallback. Missing/stale authority yields existing stale_context/no expensive work; revocation/expiry/root/ABA invalidate the fence and cancel current work.

## Source evidence and necessary glue

Moving P HEAD 5eec82a33959b764b32cbd7692cac2482fd3e009; C HEAD ad243195daf74153756308346d01a56390c1261c. Both author trees were dirty at final inspection; no source was changed by this seat.

| Inspected bytes | Mechanism |
|---|---|
| P src/dream/c1.rs SHA256 1d9db94d158760b18bcc024461831c1e1d6c8e12837ae06963a2bb37f5d5b93b | :30-32 already labels 96 MiB sampling observational; :551 retains source buffer; :936-950 fixes framing/flat-control shape. Later source edits may stale these lines. |
| P src/dream/c1_wire.rs SHA256 3bd4e6d219212412b3a0eafc12c994421cc2212cd89f8afa89319f2b0a0234d1 | :493-507 canonical serde builds Value/Vec temporaries; :565-569 bounded framing still invokes serde. |
| C logic/dream_c1.rs SHA256 93d9bd574a6196d4f60f6ef9e14547a747e3e930555d7512fafac6813c19849b | :604-611 selected estimate counter; :750-771 and :996-1032 source/parser/canonical overlap. Counter alone is not enforceable storage admission. |
| C registry/dream_c1.rs SHA256 e1cafc037af1475463dfeda4855f114564ad3d90a1059652e2a457e30f4a1483 | :391-399 capture calls signing_hold. |
| C identity/chain.rs SHA256 801f5b94d0c6a29881052fff482fe4efb38fb2f94e266fefe752269f1146e238 | :134-155 is public device-current verification; :204-220 signing_hold includes custody.load at :212. |
| C identity/status.rs SHA256 d75160d5fb4415f4289e2a6d0dc52adcbddec42d75e1889fd74fe44396fb31a8 | :237-252 admission decorates into that custody-sensitive path; :283-287 status cache alone is not fresh authenticated authority. |

Necessary narrow glue relayed to Rook: public-fence publisher/accessor in identity/status.rs, custody-free verifier wrapper around chain.rs::device_current, then existing logic dream_authority/provenance and registry/routing epoch wiring (logic.rs:3147-3154,6207-6252; registry/routing.rs:62-68). Identity admission/cutover/revocation/expiry and seat/grant/mute/root/provider owners publish/invalidate real revisions; bounded current public-head checks cover external changes. This is proposed required glue, not a claim it already exists or an implementation assignment. No identity/persistence redesign or credential contents were read.

## Handoff and verification

Corrected decision and exact next witnesses sent to producer 01M6NVNWKB6ZD11TVKP53QH8X9, consumer 01M6NVNWN8TBDWMCHBSBJ9Q2YJ and Rook 01M6NVNWNQSX1ESKK8PKSVBPY4 under existing briefs. Receipts show native-queued; model consumption unverified. No approval/acknowledgement requested.

Full current records, including spec continuation, were fetched with etags and read back exactly. Final digests: spec df8480d5ec640f4275653b88b0b6444cf41ac8b5cdfa7e050d5b11495b17f323; ICD 9da5ebcba8100eee3215b74f925ce0bf14a7dbd68b91b46c541cb0491f8a4e0e; decision 973bbe6b4b3725fcef67f7f428b736652af9bda4097c35263ee9ef85a996741b. One ICD write was explicitly rejected as workspace_pending; reorientation confirmed the same root/workspace and unchanged record, then a fresh attempt succeeded. No unknown mutation was assumed applied.

Root git diff --check returned **exit 0**, no output, at 2026-11-05 12:42:39 UTC. It checks tracked diff only, excluding untracked knowledge files. Read-only Git/source/hash commands returned real results; exploratory missing-path searches were not source evidence. No product tests, builds, heap/performance measurements, OS/library archaeology, source/submodule edits, new assignments, commits, merge or publication. Implementation, measurements and current-source acceptance remain with the component authors and Rook.
