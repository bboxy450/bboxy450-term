---
type: report
title: Recovery write-route, ownership and canonical integration witness
task: '[[tasks/2026-11-03-logic-recovery-write-route-witness]]'
commits:
- 40f45c5838f521bd70e9b8ff907caca3f8784eb9
verify:
  command: sh scripts/land.sh --full
  exit_code: 0
author: seat:01M6HGAF8KJMA0860GJ6VYM9E2
created: 2026-11-04T00:36:38.715Z
id: 01M6HZS1MN9EG04Y5FEE38HA3B
updated: 2026-11-04T00:36:38.715Z
number: 19654
created_by: seat:01M6HGAF8KJMA0860GJ6VYM9E2
updated_by: seat:01M6HGAF8KJMA0860GJ6VYM9E2
authorship:
  signer: seat:01M6HGAF8KJMA0860GJ6VYM9E2
  kind: seat
  public_key: LdPOs/V/k0C8r5wqVnsrJ6J98dX/dmczezJ1B3ZFzis=
  process_generation: '1'
provenance:
  digest: sha256:91b0cc98599db6db36a63efb8de63606dbd42ffa7943b23183d04b0229eb617a
  frontmatter_digest: sha256:e6d85b1c6950cfa0de5ab18d9bb44a48be1a8adb2784b81dc7c41ecd83eff9a9
  signer: seat:01M6HGAF8KJMA0860GJ6VYM9E2
  sig: IRYgzM7Nexd/ILdaEJmm/NYo/1pd+iHCIg9pQare5/vwQISjysskP12PfSLaA4cMkkuWT9gLUcJbAwU8A35pCQ==
  log: .bboxy/log/2026-11-04.jsonl#109771
---
# Recovery write route and ownership integration

Task: [[tasks/2026-11-03-logic-recovery-write-route-witness]]. Decisions: [[decisions/d18864-per-root-projection-writer-ownership]] and [[decisions/d19427-canonical-paged-jsonl-recovery-integration]].

The implementation landed as signed 40f45c5838f521bd70e9b8ff907caca3f8784eb9. The canonical sh scripts/land.sh --full exited 0, running normal make test on that exact clean source: 2623 tests passed, complete native/TLC parity for 290 models, 34 crates and 119 contracts, private headless smoke, and 27 invariant checks with zero failures. The landing receipt records zero incremental rechecks/retries. This report is created afterward for a separate signed records-only commit; its commit and checked reuse outcome are supplied by task_report rather than predeclared here.

## Implementation and acceptance evidence

- crates/bbt-mux/src/provenance/projection.rs:37: a nonblocking canonical-root descriptor lease excludes concurrent JSONL writers. Index carries the lease through actual blocking document IO, not merely its awaiter. crates/bbt-mux/src/logic/knowledge_effects.rs:181 acquires before journal admission and preserves replay, capacity, root/revision/actor checks; release_claim checks Weak identity. Cancellation, worker restart or a stale completion cannot release another live job's descriptor/claim. Workspaces and Engine acquire before background projection and skip a busy root; other roots and control reads continue.
- crates/bbt-mux/src/index.rs:962: the legacy synchronous Index API retains the root-number lock on its IO caller before taking a projection claim. Owner-loop entry points use a preclaimed Index and a nonblocking number-lock attempt. The unchanged cross-process NUM2 test confirms fresh unique numbers 42 through 49 after an external lock releases. The real-owner witness also holds that allocator while an admitted mutation returns explicit unknown without a file, GUI/MCP continue, and a subsequent new operation commits after release.
- crates/bbt-mux/src/provenance/projection/recovery.rs:136: one canonical separate rebuild record retains verified cursor, fixed head and publication count. Recovery/cleanup/publication remain bounded to 16 rows/parts per turn, with the 8 ms work check; ordinary append keeps the upstream 128-row/8-ms bound. projection.rs:307 syncs each touched file once per page, then directory entries, before SQLite cursor durability. Publication syncs source/destination directories before recording progress. SQL sequence/hash/prev and envelope signatures remain checked; unpublished references remain hidden.
- projection.rs:70/159 and recovery.rs:344 explicitly recognize legacy STATE.repair instead of silently accepting its staged cursor. Private partial-staging and interrupted-publication fixtures conservatively restart through the canonical rebuild, reopen, verify exact signed sequence and eventual references, and preserve unknown legacy temporary trees untouched. The upstream JsonlRepair/BrokenJsonlRepair declaration remains intact; its representation-dependent native witness at projection.rs:395 now reads the separate durable cursor.
- projection.rs:159 and recovery.rs:90 distinguish missing/invalid state, cursor/head/hash mismatch, invalid/missing parts, length/mtime mismatch, append AlreadyExists and legacy repair. Starting/trigger metadata is retained once per transition across paged cleanup/restart. It includes the safe part/sequence and expected/observed stamps where useful, root hash and transition identity, without document contents. Missing/corrupt/torn fixtures assert distinct reasons and retained transition identity.
- crates/bbt-mux/src/logic/hol_tests.rs:372 submits the actual admitted knowledge_write during a 4096-signed-row rebuild, waits for its signed write_prepared intent, concurrently queries GUI hello/self_status, checks existing reply/shutdown budgets and durable restart. Latest focused evidence: write reply 965.919 ms was explicit unknown/activity projection backlog; document_present=false and written_event=false. No write success is claimed. The exact signed intent survives restart; GUI/MCP replied in 0.407 ms, shutdown took 0.997 ms and the durable cursor resumed from 1023. The nonwrite witness replied in 12.674 ms and shut down in 0.917 ms.
- crates/bbt-mux/src/logic/ownership_tests.rs:147/374 forces actual admitted A IO to remain held while a same-root provenance turn is attempted. It asserts no cursor advance, confirmed B write/fetch and GUI/MCP service within 10 seconds, cancelled awaiter, shutdown within five seconds, restart and A-B-A routing while the original IO still owns its lease, stale-completion identity, no duplicate operation, refusal/panic cleanup and external-number-lock service. The ordinary ctl_task_transitions_preserve_verification_and_operation_identity assertions are unchanged and pass.

The original fix/model inputs 0d879fbb/1037f3ca and 2e69a2f1 were consumed with approved signature evidence; the eight HOL declarations remain byte-identical to the signed model input. Signed report-ledger/startup-input repairs ca07d897, 9b6b0ebd, 7a6dc40e and 262ce5d7 were integrated directly. Correction pins remain b09c0677ea9964b927303fa4f02592ed2d20f6e8 -> 798669285e8bf927548b249450f9ff58a1d99b7b. Attribution and historical authorship limits remain; the malformed standalone fix report was not imported.

## Verification and negative controls

All commands below used two Cargo jobs, two Rust/SWARM test threads, private fixtures and owned child cleanup. No operator, desktop or Keychain interaction was required.

- make test-jsonl-recovery TARGET_DIR=target: exit 0, 12 recovery/migration/crash/diagnostic tests plus the native JsonlRepair healthy/broken check. /tmp/hol-canonical-recovery.log.
- cargo nextest run -p bboxy450-term-mux --test storage_bus_index --success-output immediate: exit 0, 14 parent tests, including unchanged NUM2 and its owned children. /tmp/hol-number-allocation.log.
- cargo nextest run -p bboxy450-term-mux --lib --success-output immediate -E 'test(ownership_tests) | test(hol_tests)': exit 0, four parent tests including the held-number-allocator extension. /tmp/hol-number-owner-process.log.
- cargo clippy -p bboxy450-term-mux --lib --tests -- -D warnings: exit 0. Earlier combined mux/app strict lint also passed. /tmp/hol-number-clippy.log.
- make test-boundaries TARGET_DIR=target: exit 0 on 7ba597dd; all 21 ctl cases confirmed, plus the required wiring/signing/login/shell/cockpit cases. /tmp/hol-final-boundaries.log. The subsequent 40f45c58 delta changes only the JsonlRepair check-array metadata; git diff of crates is empty.
- The final-source production guard control replaces only file.try_lock with an unconditional success. Nextest exits 100 at “same-root provenance overlapped admitted IO”; the wrapper verifies that causal assertion and restores exact bytes. Projection SHA256 before/restored: 88fcef0c09e232a10b15f7dcfbdcd969085d972de86c20eda1a0560978fc7a18; mutant: 00474b2ca9c32bd44cf833227b204482017bb7b1be3dffcd8607f64cac4a984f. Index and witness hashes are retained in target/hol-final-ownership-mutant.json. /tmp/hol-final-ownership-mutant.log. No mutant remains.
- Earlier removed 16-row/8-ms recovery guards produced nextest exit 100, a 10-second admission timeout and missing-intent assertion; the recovery file was restored byte-identically. /tmp/hol-write-removed-guards.log and target/hol-write-removed-guards.json. This is supporting pre-batching evidence, not a new wall-clock claim about the later batching integration.
- sh -n on both harness generators and unchanged scripts/generated-sources.sh: exit 0. Only matching emitted headers/JS first lines changed; no generator ran. Signed-base payload hashes remain demo 94bd0b538b98f81910008a83ce117365b0059ac38ec5bd626c0018abb79d6886 and ACP 6db900bfe8003ab588f92656145fa68d77f8ec88b7586925d4af25413932dbdc. ACP still explicitly names external recordings and labels protocol/mod.rs solely as a schema reference. Startup-modal test ordering preserves the exact test module and remaining production statements; its existing test and strict app lint exit 0.
- sh scripts/land.sh --full: exit 0 on 40f45c5838f521bd70e9b8ff907caca3f8784eb9. Canonical log: /Users/feral/.local/state/bboxy450-term/landing/presubmits/40f45c5838f521bd70e9b8ff907caca3f8784eb9-07e7b1ea/commands.log; SHA256 cc53be84a6ab628e9585267c9b584a6802c6a55ed4c10b525c5ceedbc3a802e7. /tmp/hol-final-complete-land.log records LANDED. Normal mode retained its 213 runner skips and 11 slow-invariant skips; no additional skip/quarantine or gate exception was introduced.

Earlier red evidence was resolved rather than waived: b469df27 boundaries exited 2 with File exists; 8011ef8b full landing exited 77 for upstream startup item ordering; the next attempt exited 78 on the independent 2e481e4f persistence conflict, resolved under decision #19427. 123eeb2f ran 2622 passing tests and one failing NUM2 case, corrected above. 7ba597dd passed all 2623 tests but formal self-tests rejected upstream JsonlRepair's noncanonical check array; the exact mandatory pair was restored and all 22 gate self-tests passed. Those logs remain under the corresponding landing presubmits and /tmp/hol-*.log. No failed result is represented as green.

## Formal receipt and limits

The clean complete receipt binds source 40f45c5838f521bd70e9b8ff907caca3f8784eb9, 290 models, 34 crates and 119 contracts. Persistent evidence is /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/ca67c9bedfd076917a63b87217ddb0633a4d4e0d844734d7c41cd7314795eb01/evidence.json (SHA256 ca67c9bedfd076917a63b87217ddb0633a4d4e0d844734d7c41cd7314795eb01). The official formal.py --verify-evidence check against this archive also exits 0.

Native and pinned Java TLC agree on all states/edges and expected verdicts for the new pairs: each healthy writer model has 5124 states/28105 edges; broken exclusion 11604/76699, broken lifetime 10164/60568 and broken availability 5124/27409. Native tests replay exclusion, cancelled-awaiter and stale A1 completion after new A2/epoch 2, including fair liveness counterexamples. Corpus/contracts/coverage map these witnesses without duplicating contracts.

Bounds are two roots, A at most two operation/descriptor tokens, B one, and A-B-A binding epochs. Completed-reference drop, background return and queued-read dispatch are weakly fair; IO completion, routing, cancellation and callback arrival are not assumed fair. Stable canonical directories/inodes and cooperating advisory-lock users are assumed. The eight incident models and writer models are hand-written bboxy450_tlc::Model, not protocol!-generated product reducers. Parity checks that finite declaration; direct macro-to-worker refinement, arbitrary Rust/OS correctness and stalled-filesystem wall-clock latency remain unproven. Physical syscall costs, real power-loss behavior and legacy orphan cleanup remain implementation assumptions/limits, with the specified crash fixtures supplementing the models.

The exact historical 11:14:24 recovery trigger was not logged. Oracle evidence places the abandoned .projection-01M6HDB1BT547A3XKCM72EW0R0 birth at 2026-11-03T19:14:24.762990Z, after the last canonical scoped task write at 19:14:21.039Z. Correct post-recovery stamps cannot reconstruct that old branch. The sample's 3053/3412 samples in append -> File::sync_all -> fcntl do not establish SQLite lock blame. The synchronous full-history replay mechanism, the separately reproduced concurrent-writer race, and future diagnostic trigger evidence are distinguished here.
