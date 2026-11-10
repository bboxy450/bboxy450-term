---
type: report
title: Q2 readiness on accepted C1 baseline and exact selected evidence bundle
task: '[[tasks/2026-11-05-dream-q2-accepted-baseline]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T17:16:11.843Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6PBC03AVAAZZQYMWQQJPHBK
updated: 2026-11-05T17:16:11.843Z
number: 93377961365036
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:d31bb30657d7cb52e63651767738cd87b1b5556d7bc0080e940b204064f84850
  frontmatter_digest: sha256:0e29b745dd6365ad8475bcd2d1b56d8414605fb316818aee634254ec244ce46b
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: czOroUpswJgoYmNWhJcHp8N+eYT1nsmXPSrpjjEznh5VMKrds19+z1zfCrZxqiS5iuJlXSVyPoKBeeaFEPc0BA==
  log: .bboxy/log/2026-11-05.jsonl#139391
---
# Q2 accepted-baseline readiness

Task: [[tasks/2026-11-05-dream-q2-accepted-baseline]] (01M6PAV8B0JAZP2JE31VFWHGN9). Updated the original native [[tasks/2026-11-05-dream-mcp-verified-snapshot]] (01M6P1PTMD3SDRJ4JQFFV2AXV8), preserving its verifier/exporter intent. It is ready for Oracle delegation; no Executor was recruited or assigned here.

## Exact baseline and dependency

Actual initial/final Git reads in /Users/feral/mydev/bboxy450-harness-dream-read-q2 returned HEAD47182320be6435c860bc2176c2ddda582029fa63, branch feat/dream-read-q2, empty status. This exact isolated source writing tree/base/branch replaces the draft placeholder in its body. No worktree frontmatter, owner, role, status, identity, dependency or other lifecycle field was added/changed.

[[decisions/d93377961364675-dream-mcp-prepared-reads-delivery]] permits Q2 after accepted C1 producer independently of Q1. The current assignment records that C1 is merged/accepted; the source HEAD matches. The term C1 consumer remains active/unaccepted and is not a Q2 start dependency. Q3/real term integration still needs its own accepted components and public/provider evidence.

## Bounded producer-consumer ambiguity resolved

The previous read-input schema supplied pins/policy hash but no exact physical bytes/mount/artifact binding. A resolver could otherwise interpret those IDs as permission for discovery, or relocate signed receipts by rewriting their paths. [[interfaces/dreaming-mcp]] now adds only the missing EvidenceBundle boundary and corresponding selected-read rules; [[specs/dreaming-mcp]] links this packaging and accepted independent producer baseline.

The bundle inventories immutable private files, exact root/main bindings, host-selected public policy/key bytes, finite current-head/transitive record catalogue and original logical receipt/sink→physical-file mappings. Start hashes the complete manifest; each file is descriptor-relative/nofollow with length/hash/stable identity checks. Records/policy/currentness remain independently verified by existing trust/publication semantics, never by a supplied verified flag. No ambient HOME, global resolver/catalogue, uncommitted fallback, provider fallback, receipt-directed arbitrary path or network discovery is permitted.

All source/blob/Git/key/artifact bytes share the existing64MiB input allocation; manifest256KiB, finite file/root/catalogue and existing structural/resource bounds apply. It adds no memory/CPU/disk allowance. Signed bytes are preserved through logical mappings. Missing evidence/provider bridge is Unproven, invalid proof fails, and a valid empty selection differs from unavailable evidence. Snapshots are verified_at_revision, with actual original live public/provider/root revalidation remaining consumer-owned.

The actual scripted qualifying publication is built before the read worker using existing lower/publish/outer-run fixtures. The worker verifies/exports it; it does not synthesize, run trials, publish, adopt, enroll trust or append Recall. Structured host hints remain solely in the shared term MCP projection; all producer wire/artifact/proof bytes remain hint-free.

## Actual source grounding and allowed-path changes

Paths/lines are from accepted47182320:

| Source mechanism | Readiness implication |
|---|---|
| src/dream/c1.rs:224-336 Budget/unit_clock; :313 paced returned-result slot; :358-419 Paced; :421-736 disposal::Drain; :738 Dir; :2352 storage | Narrow crate-private exposure or local extraction can reuse accepted priority/CPU/pause/control/credits/custody/stepped teardown. Preserve the accepted rejected-result/failed-clock correction; no C1 rewrite or second scheduler. |
| src/main.rs:9-17 early C1 dispatch before bbt_rt | Separate read-prepare/1 must use equivalent minimal early dispatch/provisioned sibling route, preserving inspect-c1 wire/capability. |
| src/dream/resolve.rs:115-150 explicit root binding; :323-388 whole-directory/whole-log loader; :414-444 task catalogue | Add selected input/catalogue construction rather than calling the default discovery APIs. |
| src/dream/trust.rs:465-591 policy/public-key verification; :660-692 exact committed anchor | Add bounded selected policy/mount/key view, preserving actual signature/ancestry/producer approval. Host IDs alone are not trusted bytes. |
| src/dream/audit.rs:198-229 current memory catalogue; :484-505 Current view; receipt.rs::check_outer/check_publication/check_adoption; recall.rs:28-125 | Factor selected paced read validation/rendering, including outer seal, active compatible pair and current record. Whole async audit/Recall preparation is not automatically C1-budget compliant; append is excluded. |
| tests/dream_recall.rs:26-68 actual lower publication/settlement/outer finish; existing dream good/sinks/trust fixtures | Concrete real already-published scripted positive, followed by missing/changed input/currentness/proof negatives, needs no term consumer/provider account. |

Added allowed paths only: src/dream/c1.rs, src/dream/worker.rs, src/dream/trust.rs, src/dream/receipt.rs, src/dream/projection.rs, src/dream/seal.rs, tests/dream_c1.rs. Their brief limits them to shared primitive reuse, selected read-only proof/currentness and paced disposal/control; all other original allowed paths remain.

Q2 verify now:
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build.
The added existing C1 gate protects shared-primitives regression. test-dream-mcp remains a proposed target for the future Executor to implement; none of these product/formal gates ran here. Host verify_revision advanced1→2. Any future acceptance must use the amended gate and fresh source-bound evidence.

## Record preservation and full readback

| Changed artifact | Before digest | Final digest |
|---|---|---|
| [[tasks/2026-11-05-dream-mcp-verified-snapshot]] | e60d926e91051b7b93acc18c8f001e6b9263d5395069d3fb0b9218bdd6b3cbcb | a8fb05fc052495c0594c37ebfbccca1cc37b4146b141d2717a60429d7e4dc516 |
| [[interfaces/dreaming-mcp]] | c9497e6ff5315ec076706fc5e55d6eb8542bb474b1745c596ab7f4380326508e | e8482aa9968b673d92123f4678484047aa4d23c327b646a02c0a636a0f25d80d |
| [[specs/dreaming-mcp]] | b79ec04a5b931cc129fcf27521ed5b53bbc6632394ae112ed898aed8c876821d | 060eef6fcb26c79145625918b6ba3f3661a146cd7e0bcb92426eadf998814a94 |

Complete fetches/continuations preserve original refs/etags. Full amended bodies matched returned bodies, with no truncated remainder. Q2 frontmatter changes are only permitted allowed_paths/verify plus host updated/verify_revision/provenance; all remaining fields compare unchanged. Native task_status independently returned open, no assignee, verify_revision2. Spec/ICD frontmatter changed only host authorship/update/provenance fields. Original Q1/Q3 and live C1 briefs were not written.

Source SHA256 anchors: c1.rs cd934e73511ba035773dbce18a0e9486ea7b9e30586cfa0bfb8643eb68d5aeb9; resolve.rs c02970d89b68ccc9ec218f722bdd81478fb67181de96615582f5e73487babeab; trust.rs456e6232ef9c3579967e3f72a75aa21d3a05255af27869abb395267911e6a07e; audit.rs ecb094a64cecc08a177b43b695c4111c2696a86ec1d0364a70ba0ea8b42ed544; recall.rs b91910994fd748b8131ca0c2ab79789b40ca8751320f073c20c9111d5e6eb4c5; receipt.rs3b820de0b19e20e4dec9dd9e2717dcb2f4efe9dd1d5cb7b46e74f583333539bf.

## Actual check and remaining evidence

At2026-11-05 17:15:03 UTC, git diff --check in the exact Q2 source worktree returned **exit0**, empty output; HEAD/branch/status reads returned0 and remained clean. Read-only source/hash commands returned0. KB changes were made only through the selected term knowledge provider.

No source edits, builds/tests/formal runs, service/account/credential use, install, assignment/recruitment, commit, acceptance, merge/push or consumer bump. Real producer/private-parent resource/protocol witnesses can run independently; they cannot certify actual term parent/native frame/public-authority integration. Real lesson availability still depends on a qualifying published corpus and trustworthy current provider proof; unsupported native/draft evidence remains unavailable. No benchmark threshold, C1 wire/limit, inference/publication behavior or universal native-memory claim was expanded.
