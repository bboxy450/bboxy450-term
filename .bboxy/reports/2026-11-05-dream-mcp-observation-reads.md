---
type: report
title: Dream Q1 prepared own-seat observation search/read core
task: '[[tasks/2026-11-05-dream-mcp-observation-reads]]'
author: seat:01M6XXAAX0KD2M33WBR5DGXA2X
created: 2026-11-08T19:59:25.949Z
commits:
- 4d92b2b1ef87c90019dac6b6e5d3a867e3afdca3
- d45f8b64dcdfde7731ef5f1925668b17c7b98918
- 6dd42943fa8aef23db79bda01fdc2b3e104e9519
- a426328916d07b751229664f943213b8f8b1fee3
- da21c1bde2171af3359d68c5ef61c5617a744507
verify:
  command: make test-dream-mcp && make test-formal-contracts
  exit_code: 0
id: 01M6YBX1PQ7WYRFH35SMRVGR85
updated: 2026-11-08T19:59:25.949Z
number: 93377961366400
created_by: seat:01M6XXAAX0KD2M33WBR5DGXA2X
updated_by: seat:01M6XXAAX0KD2M33WBR5DGXA2X
authorship:
  signer: seat:01M6XXAAX0KD2M33WBR5DGXA2X
  kind: seat
  public_key: 8SpR7rvg5w93qFp5M4fgJljdQyA081x7LEtPc35IGVU=
  process_generation: '1'
provenance:
  digest: sha256:cdfa4b390563d2806448890cce9368a0fa2f46a9b8864f5269265600d7f0a9d0
  frontmatter_digest: sha256:54696b76c152df65fc78479718b3186efe0db5e4ef719ab7547ecf7d11ff5507
  signer: seat:01M6XXAAX0KD2M33WBR5DGXA2X
  sig: vJNT+5fHuxHZbIh8Ukkz5tPx+l4mfyfpMb3zKd3QGuWq0sPNB2FisYTRc3RBsS1SYOKnTNsnavjn0TNRLwk4Bg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150810
  event:
    id: 01M6YBX2AXS8D3FM6GR6BMQQ62
    sequence: 150810
    hash: 200c8008709024cb86e75e534460366d100e2bc9d6db6dfaf241ac647fea91cb
---
Landed the amended Q1 core at da21c1bde2171af3359d68c5ef61c5617a744507 through make land (exit 0). Full worker-close/pressure witness expansion and optional member links remain deferred, as detailed below. This report is a handoff for Oracle verification, not acceptance.

Implemented the Oracle-amended Q1 core in the isolated feat/dream-q1-observation-reads worktree, based on term origin/main 96dbc71ba3631ccb7907b3449ca76dd0f6e83749. Optional member links and the full C1 witness expansion remain deferred by the dispatch amendment.

Changed mechanisms:
- crates/bbt-proto/src/dream_read.rs: closed v1 search/read request, observation proof and response DTOs; exact row pins, separate index/member coverage, explicit not_evaluated trust, closed final outputSchema including only declared outer shared hints/op_id. Public module and fixture wiring are in src/lib.rs and tests/dream_read.rs.
- crates/bbt-mux/src/mcp/dream_read.rs: catalogue/strict admission, common shared hints::attach, and conservative byte accounting for every structured/text/_meta copy before accepting a whole item. mcp/mod.rs and mcp/tools.rs register the tools.
- crates/bbt-mux/src/logic/dream_read.rs and logic.rs: the existing admitted MCP/Bridge route captures public authority and moves reads off the logic owner; the shared final projection is idempotent and optional unrelated unread previews cannot overflow the charged Dream reply.
- crates/bbt-mux/src/registry/dream_c1.rs: ONE completed-and-settled hook prepares disposable reads only after durable C1 checkpoint/settlement; final settled control state is published before disposable preparation. Reads reuse the existing request cap and never admit a C1 job.
- crates/bbt-mux/src/registry/dream_read.rs and registry.rs: bounded per-context snapshots, opaque handles/cursors, literal metadata ordering and exact fingerprint filtering; validate immutable manifest/result/page bytes and exact row kind/fields. Source IR is only statted, never reread. Public Context/PublicFence and final captured-snapshot pointer are rechecked before presentation.
- API/process fixtures in crates/bbt-mux/tests/dream_read.rs and crates/bboxy450-term/tests/dream_read.rs; finite models in tests/formal_contracts/dream_read.rs and models.rs; verification/contracts.toml, corpus.toml and coverage.toml map the three families and causal controls.
- Makefile adds test-dream-mcp. The explicitly amended scripts/qos.sh, scripts/qos-policy.py and LAND.md register the Q1 timing fixtures and the existing registry::dream_c1::registry_tests:: / logic::dream_public_api_tests:: fixture families at default QoS. Existing 1 ms guards and assertions remain unchanged.
Existing ATTRIBUTION.md was read and preserved byte-for-byte with its package mirrors. New sources carry the canonical project/license/policy notice and accurate fixture adaptation credit.

Boundaries and gaps:
- No inference, KB/provider scan, lesson backend, feedback write, persistent memory sink, source rereplay, C1 wire change, second server, external provider/account experiment, desktop interaction, install or release.
- Projection bounds: 4 contexts, 128 entries/context, a 4 MiB reservation per snapshot from existing C1 Work credits, 64 opaque cursors with 60 s TTL, <=8 hits/page, <=16 KiB final serialized output and the stricter requested context/token budget. Oversized descriptors (>16 KiB), missing preparation or exhausted preparation return honest unavailability; reads do not rebuild.
- This slice emits zero optional member links, complete=false and honest total/omitted counts. The C1 artifact does not retain historical harness metadata; the required string is explicitly not_retained_by_c1 rather than reconstructed from source.
- Native term smoke proves admitted discovery and cold no-job reads. Positive/page/tamper/currentness proofs use an actual admitted MCP/Bridge over private completed C1 artifacts. A real scripted C1-worker authoritative-close-to-Q1 positive and the parent/child stall/flood/four-job/native-frame witness expansion were not added; existing C1 limits were not weakened. Detailed positive pending/original-operation recovery and nonzero member-link cases are not claimed for this slice; read calls have no new journaled domain effect.
- Models abstract finite identity generations, context/snapshot ownership, row binding, foreground eligibility, member honesty and reservation. Opaque native allocations, native timing/OS scheduling, larger/unbounded populations and arbitrary Rust correctness remain implementation-only gaps. Java TLC remains the reference.

Acceptance evidence (amended core versus deferred original breadth):
1. Native route: crates/bboxy450-term/tests/dream_read.rs:151 starts a private headless term, initializes the actual admitted MCP route, discovers both tools and asserts cold reads start no result job. Completed-artifact positives traverse the admitted socket/Bridge in crates/bbt-mux/tests/dream_read.rs:89. The real scripted worker-close-to-read positive and separate open/idle/turn-end/EOF close matrix are not newly proven here.
2. Currentness/refusal: registry/dream_read.rs:596 and :877 revalidate complete Context/PublicFence and immutable row bytes; tests/dream_read.rs:38, :89 and :296 prove foreign/malformed/cold/lesson, page tampering, public-head revocation/restoration ABA and in-flight snapshot invalidation refusal, with unchanged native/signing spies. Root/provider authority comparisons are in the shared full-context path; no new exhaustive root/provider process matrix is claimed.
3. Proof/schema/budgets: bbt-proto/src/dream_read.rs:412 exports closed final schemas. bbt-mux/src/mcp/dream_read.rs:43 and :53 use the ONE shared hint generator and charge final serialized copies. The structured-only fixture (tests/dream_read.rs:8) validates the advertised schema, mirrored text hints and <=4096 bytes. :196 proves the canonical two-session/count2 cluster plus two distinct episode origins and cursor binding; :272 causally rejects wrong row/kind, fabricated cluster origin, false member completeness and poisoned preview. Optional nonzero member-link, full 128-entry/near-limit stress and a pending real worker integration matrix remain deferred; zero links are explicit and honest.
4. Resource/foreground: registry/dream_c1.rs:2258 publishes terminal settlement before :2261 disposable preparation. registry/dream_read.rs:492 reserves bounded existing C1 Work credits; :957 proves foreground refusal precedes every preparation effect. Native cold smoke and admitted unprepared fixture prove misses never start work. The full parent+child stall/flood/four-job/control/native-frame expansion was deferred by amendment, so no new full C1 pressure-retention claim is made.
5. Formal: tests/formal_contracts/dream_read.rs:14 defines DreamReadOwnership/Snapshot/Eligibility and causal Foreign, ABA, MixedCursor, FakeLesson, RowBinding, ClusterOrigin, MemberCompleteness, Reservation and MissingPause controls. verification/contracts.toml:1886 and corpus/coverage map all twelve cases to source and real API/process witnesses. Opaque native allocations and timing remain explicitly unmodeled.
6. Scope/provenance: isolated worktree, exact allowed paths, source diff and existing attribution policy reviewed. Five signed Boxy commits carry the report trailer. No peer source or live C1 author worktree was overwritten.

Failure disposition:
- Earlier utility-QoS setup failures (C1 capture 1.097750 ms, commit 1.236292 ms) led to the signed task's Q1 timed-test inventory amendment. Q1 private fixtures isolate unrelated periodic identity maintenance and avoid unused journal setup; no read credential/signing assertion is removed.
- First make land exited 2 (script gate 77): Clippy items_after_test_module in the appended adapter methods. Fixed by moving methods ahead of test modules; no lint suppression.
- Next make land exited 2 (gate 77, nested nextest 100): existing C1 structured-only fixture journal first-step 1.017042 ms. The one-test default-QoS registration was explicitly amended.
- Following make land exited 2 (gate 77, nested nextest 100): existing durable-cancel C1 fixture 1.149167 ms. The Oracle replaced the narrowing one-test exception with the two explicit existing C1 fixture-family filters. No ignore, quarantine addition, runtime QoS change, budget relaxation or assertion weakening.
- test-licenses initially exited 2 for a root-only attribution appendix that diverged from package mirrors, then for missing/noncontiguous owned notices in two new fixtures. Removed only the unnecessary appendix and made the new fixture notices canonical; the existing attribution policy and all package copies remain unchanged.
- Toolchain rust-objcopy emitted its existing missing libLLVM.dylib warning; successful command exits below are actual, not inferred from warning text.

- The next formal gate correctly rejected the initial aggregate contract inventory: it requires one named positive plus declared controls per contract, not a synthetic three-positive aggregate. Split the entry into the three executed families, declared each expected safety control, restored the mandatory [make test-formal, make test] list and kept focused checks separately. make test-formal-infrastructure then exited 0 (33 tests); model expressions/runtime code were unchanged.

Verification and landed state:
- make test-dream-mcp && make test-formal-contracts — exit 0. Proto 3/3, mux 7/7, real private term process 1/1; formal contracts 45/45, QoS witnesses 2/2 plus 2/2, Python/native QoS checks 19/19 and latency_ok=true (250 ms limit). Logs: /tmp/dream-q1-committed-mcp.log and /tmp/dream-q1-committed-contracts.log. Subsequent changes only corrected the existing C1 test-family inventory and formal contract metadata; Rust/model expressions were unchanged.
- make test-formal-infrastructure — exit 0, 33 tests after the inventory correction.
- make land — exit 0 on da21c1bde2171af3359d68c5ef61c5617a744507; /tmp/dream-q1-land-mapped.log. Clippy -D warnings passed for all three affected crates; term 196/196 (113 existing policy skips), mux 888/888 (79 existing policy skips), proto 31/31. No new ignore/quarantine entry. Nested make test-formal FORMAL_REUSE=1 exited 0: 693 models, 34 crates, 258 contracts; native/TLC parity held. The twelve new cases compared full reachable states/edges and expected safety verdicts; prior unchanged cases reused source/tool/model-bound receipts through the canonical verifier.
- make check-formal-evidence — exit 0 on the final clean source. Receipt f14212217f744ebdf9ff33920e108c7d30346d43eb60cd90e0dfaf1689ca53b2. Retained evidence: /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/f14212217f744ebdf9ff33920e108c7d30346d43eb60cd90e0dfaf1689ca53b2/evidence.json (survives this worktree's target cleanup).
- make test-licenses — final exit 0: 34 workspace crates, canonical notices held; /tmp/dream-q1-final-licenses.log.
- git diff --check HEAD — exit 0; git status --short empty. HEAD and origin/main both da21c1bde2171af3359d68c5ef61c5617a744507. All five commits report G signatures and Boxy <bboxy450@gmail.com>.
- No separate test-boundaries, test-rich-launch-context, test-harness-native or full make test was run, per the trimmed amendment. The canonical formal target did run its required harness-native models.
Operational deviation: free disk briefly reached 7.8 GiB during formal expansion, below the requested 8 GiB floor. Removed only this worktree's inactive generated caches after open-file checks; after all Rust phases, retained the formal models/tools/archived receipt and removed inactive debug/intermediate exports. Final free disk was 12 GiB. No peer directory or process was touched.

New finite-corpus reference comparison:

| Model | Verdict | States | Edges |
| --- | --- | ---: | ---: |
| DreamReadOwnership | pass | 7 | 11 |
| BrokenDreamReadOwnershipForeign | safety | 8 | 12 |
| BrokenDreamReadOwnershipABA | safety | 8 | 12 |
| DreamReadSnapshot | pass | 7 | 11 |
| BrokenDreamReadSnapshotMixedCursor | safety | 12 | 16 |
| DreamReadEligibility | pass | 72 | 547 |
| BrokenDreamReadEligibilityFakeLesson | safety | 74 | 549 |
| BrokenDreamReadEligibilityRowBinding | safety | 74 | 549 |
| BrokenDreamReadEligibilityClusterOrigin | safety | 74 | 549 |
| BrokenDreamReadEligibilityMemberCompleteness | safety | 74 | 549 |
| BrokenDreamReadEligibilityReservation | safety | 74 | 549 |
| BrokenDreamReadEligibilityMissingPause | safety | 105 | 837 |

Raw logs remain outside Git. SHA256 summaries:
- committed-mcp.log: 1d9ad59c11c44d74d32136f0a4eceab652df1c52bf77838dae93ed56abd11b38
- committed-contracts.log: a8cc9f981aa46e9a39703d290dda7ff10f62d21a59365ecfc18254855ef6bb6c
- formal-infrastructure.log: 9681a670d19a1445602465220da066d880e77f73fd57e54a153e3a8edbb9a540
- land-mapped.log: 84b3efe3f1bbce4f12b3f6ad082a0fb1602f17d29bc20e792e110b3e6a961400
- formal-evidence.log: 610c8a4b7ed5fed14d59ab48f2b7e5e1fe8f57170e5efa4224aa1d42f3ef2976
- final-licenses.log: 2a4020d85a7c845f2b3c133ea234f61e9fee5dd04a443ba7178c717ff3176df6
All six filenames above have /tmp/dream-q1- as their path/prefix. Earlier failed gate logs are /tmp/dream-q1-land.log, -land-fixed.log, -land-final.log and -land-families.log; the failure dispositions above preserve their actual verdicts. Formal infrastructure publisher messages are mocked test output, not a publication.
