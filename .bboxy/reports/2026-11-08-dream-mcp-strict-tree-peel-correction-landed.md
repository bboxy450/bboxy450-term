---
type: report
title: Dream Read Q2 strict tree-peel correction landed
task: '[[tasks/2026-11-05-dream-mcp-verified-snapshot]]'
author: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
created: 2026-11-08T05:53:31.097Z
commits:
- af5de27c33d46cdcb04510e863ef95fdc835c3ec
- a7c08e65df64c8ed8a4a0ce3ad9daf96cba90373
verify:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
supersedes: '[[reports/2026-11-08-dream-mcp-tree-pin-correction-landed]]'
id: 01M6WVG3DNVGHWT7PTE3M1KBZ4
updated: 2026-11-08T05:53:31.097Z
number: 93377961366336
created_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
updated_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  kind: seat
  public_key: 3lIiKAyaYQbqAYQ3xsrS+NTGLp9s//BjqCA4HkK5nF0=
  process_generation: '1'
provenance:
  digest: sha256:fb49c9d4e336a98a6507e2f80f9d22ccbaa5613f32948fcbb13a17bbd215e1db
  frontmatter_digest: sha256:ce33f1246ce768d1841092da8e51c15faf6c7ee05a5168cd814582d38c3c3947
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  sig: Hv15mwz8jDo9QDH3q4NLx/yxqUz8VUHN5Ae8ybQRfj/8xV86D2lKSxcCFeU0FadbECrM8/ID1oEOBk+WBxljDw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150562
  event:
    id: 01M6WVG51W76WHQKHTR0TS6WJF
    sequence: 150562
    hash: f0494acbdb0c2f28c15cc563e6ad0db80532069eaced2f95f47dac549b303693
---
Landed signed source60 `af5de27c33d46cdcb04510e863ef95fdc835c3ec` on `origin/main` with a clean worktree. Its production parser cut is signed source59 `a7c08e65df64c8ed8a4a0ce3ad9daf96cba90373`; source60 adds one narrowly authorized QoS ignore attribute. This append-only report supersedes [[reports/2026-11-08-dream-mcp-tree-pin-correction-landed]] (native checkpoint `4c8b296a86502102a62ac82f7b5083c0bfbe2435`). Source58's signed commit `ba22c67c488421f58faff3e5256f7fcfff6a5ab8`, report, Input58 and archive remain preserved.

The previous parser copied unknown braces literally. Oracle message `01M6WD5M8GCS46EZ7PXS6QEB0M` requires only the exact registered terminal Git peel `^{tree}` to remain literal; every other unknown placeholder, including `{tre}`, must refuse. C17 implements that rule. The actual exact task verification command below returned **0** on current source60, followed by current canonical landing gates and push0. **Real full-private-candidate 30s fit and tool qualification remain UNPROVEN; runtime_approved=false. No genuine builder candidate was created.**

## Changes and causal controls

| File | Bytes | SHA256 |
|---|---:|---|
| tests/fixtures/dream-read/proof-helper-probe.c | 320774 | 5178cef71ff230fe9af77fb5e183af709190e0cb833316ba5b8545a4bb4011ec |
| tests/fixtures/dream-read/private-engine-controls.c | 116835 | 9eae6b618e7f6489f0b45479dc0bf9c2d7cf43dbc578cad298fd1ef3787dd17a |
| tests/fixtures/dream-read/check-private-engine-controls.py | 37207 | a215f6b449b9eb37078f7fa0d6fd89fc2a82ab3187c191364a916213dda5b9c3 |
| tests/dream_c1.rs | 158331 | 0340e6cb42d8c0a6ea7ea61c1cc4137231b18f48ac6dd7b0d1882eac3c6b0b83 |

`engine_expand` now treats a caret immediately followed by an opening brace as one boundary: only exact terminal `^{tree}` is permitted. Unknown, malformed, nested, unclosed and stray braces refuse, including `{tre}`, bare `{tree}`, `^{tre}`, `^{commit}`, `^{blob}`, `^{tag}`, `^{}` and a tree peel with a trailing suffix. Seven existing named placeholders still expand, including composed commit/record colon syntax. The fixed16-form registry, standalone/embedded B signature-token bans, direct-form15 refusal and4096/4097 limits are unchanged. No model formula, public schema, wire, resource cap, deadline or production pacing changed.

Production form10 preparation checks valid and corrupt selected40-hex commits, argc19, terminating NULL, rev-parse and exact commit plus literal tree suffix. C17's narrow run had **3 strict compiles0 / baseline API0 / old-reject81 / old-copy-unknown81**, each rejection at its respective named positive-preparation or unknown-negative assertion. Raw `target/private-strict-tree-causal-5vlxo6sf/run.json`, SHA256 `f698b07d40a8f84a9b1fa93e777ee42361a1ee994a423127600cc4c16239b8aa`; log `4ae74a184ce04650e4be96a19fe1a0ee4485d9436577bcbd227976061882c9a3`.

C16 remains RED: strict baseline compile0, baseline API81, outer1, raw `target/private-strict-tree-causal-z8xq7zhg/run.json` SHA256 `54afd5e11e439028186bd42a9f827e439618d2a91008767565e907908840e73e`, log `a058cc08b20f8de470bf17f8efc86b7253b5e04b848d5bda36bb63d453c60d13`. Its production path could copy a caret and then expand a recognized placeholder inside an unregistered peel, such as `^{commit}`. The raw failure names the unknown-syntax assertion; its vector index was not separately logged. C17 corrected that boundary while retaining the same negative assertions. Old-reject now targets the actual whitelist gate; old-copy-unknown removes only the unknown-placeholder refusal. No unchanged lucky retry or weakened assertion was used.

The current integrated canonical C17 suite passed **68 baseline cases /41 source mutants /43 strict compiles /88 commands**, with unchanged signed source snapshots. Its actual current record is `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/formal-tree-pin-source58/harness-landing/private-engine-controls-s9_25d20/run.json`, SHA256 `117cba7e8ecf318064bb1aae9b68752caaa6987bddfd2ca1bab10bc2adb9c4b4`. All15 directly admissible forms still invoke actual production argv preparation; form15 remains implicit. This is source-known API/owned-fixture evidence, not installed Git/GPG qualification or a real candidate success.

## Actual exact verification and canonical landing

```
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
```

This exact POSIX `&&` command actually returned **0**, not a composed substitute. Actual runner record `R/strict-tree-exact-four-chain-v60.json`, log SHA256 `e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf`, HEAD `af5de27c33d46cdcb04510e863ef95fdc835c3ec`, source-before/after unchanged. `R=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3`. All four constituent Make commands therefore returned0. Environment: FORMAL_JOBS unset, RUSTC_WRAPPER=/usr/bin/nice, empty workspace wrapper, CARGO_INCREMENTAL=0; TARGET_DIR, CARGO_TARGET_DIR and BBOXY_CARGO_TARGET_DIR all point to the existing owned current-only432-model lane `R/formal-tree-pin-source58/harness-landing`. Its historical directory name is not a source receipt. The normal test controllers/TLC job stayed at ordinary priority; only rustc was backgrounded. No cap, Java heap, worker, timeout or assertion override was used.

This also resolves the exact-command mismatch reported by native message `01M6WJPKX6H1ZPNF2EGNBVZMZY`: this report's verify field and task_report use the task's literal command and the genuine current shell exit0. Prior source58 individual-phase successes and canonical completion remain factual; they are not relabelled as a whole-shell run.

The additional actual canonical completion command was `env -u FORMAL_JOBS RUSTC_WRAPPER=/usr/bin/nice RUSTC_WORKSPACE_WRAPPER= CARGO_INCREMENTAL=0 python3 -I -B target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/complete-current-canonical-land-v60.py`, exit0, log SHA256 `ce01369f2edfd5f14e8ef6a0b1f3f339962c42ed63f85503848a6d7483121bd2`. It validated the exact current four-chain record/hash, current C17 immutable snapshots, actual profiles/observers/formal receipt and unchanged Input59 ten-file bindings, then executed the unmodified `scripts/land.py::gate` and canonical signature/source/fetch/rebase/push functions. Its current strict all-targets and fixture Clippy checks and license/embedded-lint checks passed; lib nextest had188 passes/9 skips, and selected C1 integration had33 passes/7 ignores, all with0 failures; the current formal validator reused the completed current receipt without another Java run. No second MCP or exact four-chain run was claimed. It is a canonical-function completion, not a fabricated whole `make land` shell result.

Actual landing receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-canonical-receipt.json`, SHA256 `481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a`, full tracked-source digest `8030facb1b61ce73c3b7168ba7a3f09558d65584bfdb51db168a936759188a41`. Both outgoing commits verified G with approved fingerprint `9B07E849480C68328CA20684844ECC0587DE7AB0`, Boxy <bboxy450@gmail.com>. Actual fetch/push and post-push ancestor verification returned0; origin/main contains `af5de27c33d46cdcb04510e863ef95fdc835c3ec`. No reset, stash, broad stage, peer cleanup, installation, release or credential operation occurred.

## Current artifacts and finite verification

The exact four-chain's actual READ and C1 worker SHA256 is `84be81a06aa73e159c6531f1abe5d8b54515ddf31c520bb20414a951823bbf6d`. The later canonical lane independently built and tested C1 artifact `e55564213f5a17fac884c80c9f719f56bd8392e70d620f82281a2d4bb5dde0ba`; its separate actual profile is preserved below. These are two current build artifacts, not a same-image claim; their byte difference was not diagnosed. Source58's f887 artifact remains historical. The probe C bytes are embedded by resolve.rs, so changed C required current compiled-worker evidence. Actual generated metadata was preserved byte-for-byte after exact command0, including READ's39-input snapshot/stamps and independent read-only worker inodes. Nothing was reconstructed.

- /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/retained-af5de27c-actual-profiles-exact-four-chain/preservation.json — SHA256 80c3124a92d75f47d6a6c7a28c933ae3494df7ca641d8d4758fed18111270baa; 53 actual generated files; dream-c1-native-profile worker SHA256 84be81a06aa73e159c6531f1abe5d8b54515ddf31c520bb20414a951823bbf6d; dream-read-native-profile worker SHA256 84be81a06aa73e159c6531f1abe5d8b54515ddf31c520bb20414a951823bbf6d
- /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/retained-af5de27c-actual-canonical-C1-profile/preservation.json — SHA256 71224a300d06f1f22db5687aec148b30ebc5fa99640c9ce4d7ea07f9138146b8; 6 actual generated files; dream-c1-native-profile worker SHA256 e55564213f5a17fac884c80c9f719f56bd8392e70d620f82281a2d4bb5dde0ba

The unchanged canonical same-filesystem preservation helper retained the five actual observer JSON outputs with their original inodes and bytes: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-observer-evidence-m0tvklpk/preservation.json`, SHA256 `7dd7e458ec2d1be56197cc50ce87e831872b09918d4acf621d8a543805c76e59`. Its successful canonical MCP prerequisite follows from the real exact `&&` command0; the recorded derivation is explicit. All older observer artifacts remain separate.

Fresh native/Yarn and pinned Java TLC completed **432 models /93 contracts /1 crate, 3351917 states /11296388 labelled edges**, with complete state/edge sets and named expected verdict parity. Model/tool bytes are unchanged from source58; the actual current receipt is fresh, not relabelled cached TLC. TotalBudget's82 domains,83 assertions,18 causal controls and native cap100000 remain unchanged. This finite corpus does not establish arbitrary-native execution guarantees or complete TLC replacement.

Current immutable archive `/Users/feral/mydev/bboxy450-harness-dream-read-q2/verification/evidence/6e73cde0ff3876f56eed8fd71deccda16d9feb9d36adba83fa4801d4c2efcc11`, evidence SHA256 `6e73cde0ff3876f56eed8fd71deccda16d9feb9d36adba83fa4801d4c2efcc11`. All **4322 files /7214284397 logical bytes** are enumerated by `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-formal-all-raw-files.json`, SHA256 `920e107f1784fa618a8f3d51b2835ad0bcf3574085b0e74ccfa1ec51dee7eec3`. Canonical archive files0400/directories0500 are owned regular files/nlink1 with no symlinks; their bytes equal the actual writable working cache, with independent inodes. This run regenerated the authorized current-only cache and invoked actual Java; no retired MemberWindow files, exporter changes or cached-receipt fiction were introduced. Source58's immutable353b archive is preserved. Preflight allowed14.429GB new raw/archive plus6GiB build/margin; publication succeeded0, with12011053056 bytes free after preservation. No further cold reclamation was needed for this follow-up.

| Unchanged model/tool input | SHA256 |
|---|---|
| tests/formal/dream_read.rs | c02ee30b512ae15ae2335c5db76b4799aa54c67c5c9d709998344f6bfa01944a |
| tests/formal/models.rs | 4e992589a372c2b63bd47b05fdbb5c0639f5110e5e8b42f3b90130c231e237ca |
| verification/contracts.toml | 1deffb06e5e60d64e7b40f4d63432602ff2ced39958bda588b19d1cd26b52b66 |
| verification/corpus.toml | 3b2a226fc7fc8f8603e2a855af492207e698cfc01f42789280a9e952d2f7e8ce |
| verification/coverage.toml | bb4e09ad9b54700b81893d630e2577ad1a5db79a051a05a3a7ef46cfdd2e4038 |
| scripts/formal.py | 4b87e5b4f6388d8b1dbfe0ce5cf7f778462c6d05c589c5fb2d57d162d6a7cd79 |
| scripts/land.py | 36bd8eb4b0d6bcba5efd46ee94d5d0f035bb3c5ddb5c12aa02efb8dc0a859354 |
| verification/tools.lock.json | 45d8a35d0b533e53689efc46542315cf91713ee5821a2f002289cc0b8bc7a8dd |

Pinned TLC JAR SHA256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`; Temurin21.0.13+9 arm64 JRE archive `225bab0a09491ef06835488cd942f5d2befa9a0410a2484f0c8bbcf7506ec09a`.

## Input59 and remaining qualification boundary

Essential replacement tuple handoff **1415**, id/op `01M6WR8NE05B2N7TMC17MGRW83`, retired Input58/message1408 for current use while preserving it. It was stored; no builder model receipt or runtime approval is inferred. Input59 source/preparation commit `a7c08e65df64c8ed8a4a0ce3ad9daf96cba90373` remains valid under source60: the latter changes only tests/dream_c1.rs's ignore attribute, and all ten signed C/Rust/build/schema/vector input blobs are independently checked byte-identical. No duplicate tuple was invented.

Full tuple `R/strict-tree-input-source59-tuple.json`, SHA256 `aa84236c270d26e6ed16be5e6511a227cb045660ca181e3f02038ceef6fb41f6`; receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-strict-tree-source59/dream-read-c-preparation/compile-aljl1q9p/preparation-4bfa5ed6f1c8ff91a389313a50f4b216b838a84ea0cfd3df47bf93595a17d520.json`, SHA256 `4bfa5ed6f1c8ff91a389313a50f4b216b838a84ea0cfd3df47bf93595a17d520`; executable `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-strict-tree-source59/dream-read-c-preparation/compile-aljl1q9p/probe`, 251016 bytes, SHA256 `5348778976c7e9f98900461bf0d201d7ad1871e338a4e118139e307d1594f47e`, mode0500/nlink1, retained roots0700/files0400. Canonical preparation actual0/log `08638bf18cd015dee4f9a154601e3c49173b688b3af55a12b330d7b8fdb79745`. Independent audit actual0/log `59fb42038aff63ef8a996f9d1313cd9404bf233649b284af5809548ae7ea057c`, 997 checks /325 lexical+resolved headers; detail SHA256 `77f88d0b1d4af4dce0117f09c87e9e50b1b6a3529b848f8f06c91aca6ad1ad37`. Actual pthread/spawn.h two-argument QoS declaration and all retained header snapshots remain bound.

Compiler clang21.0.0 SHA256 `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`, exact flags `-v -O2 -std=c11 -Wall -Wextra -Werror -target arm64-apple-darwin -isysroot /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk`; SDK27.0 settings `7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb`; linker `374d9f0713bce2c8a9e1e82bde51615d5433516a10d99b63107eb6ee70542e4c`; arm64/Darwin27.0.0/build26A434. Exact signed schema/vector authority remains `b26d7cd3432e9cae3e6c68492afd73a82cc8f2e9`, schema15157 bytes/SHA256 `d70948ec375dae099ea1dac0b92dfb30ff87d7fcac94c98f42caa1c0ba2da8ec`, vectors3900950 bytes/SHA256 `bff32c1afb483980952a6761918d4005c9f1e12b66f3d7aaaa88f1b9afe09740`. Rust4 domain `177cff0e6a8a658a4514494e28506184754f15ba9c50a504dd8f02817db6850c` remains an exact source snapshot, compiled_product_currentness_proven=false, separate from actual worker profiles.

The full genuine private positive30s ordinary-floor fit and B startup/real proof-tool qualification remain **UNPROVEN** under the task's existing qualification disposition. The historical C15 engine-only703265000ns/67 units/603000000ns paid debt vector exercised5 guards and0 actual helper births; native/wait4 metrics were scripted and outer/inner/auth/compiler/SDK/OS/platform preflight was not exercised. It is not whole-path success. The earlier25.829118s global debt decomposed into584.118ms helper CPU debt plus25.245s ordinary/native/poll normalized floors; it did not prove a9×wall helper formula bug. No performance expansion or real candidate was added here.

## Readiness RED, authorized deferral and failure preservation

The first exact current-source59 chain returned **2**, log SHA256 `016a0f3354ff9fcc7b4b4e63ec6b03d5e104f304daba1c69d3a4f1971986f222`. C1 integration had32 passes /1 failure /6 ignored and stopped the chain before MCP/formal/embedded. `actual_store_fork_is_explicitly_unsupported_and_parent_rows_never_pass` failed its pre-permit source-bound waiting-marker poll at the unchanged250ms deadline; logged lifetime253ms, empty partial stderr and original PID67102 still alive. Parent birth measured599416ns CPU /2375458ns wall within its native caps; no permit was attempted. The original later settled through HeldDrop with exit2. Child startup CPU and a specific scheduling/load cause were not observed; neither is inferred from parent CPU or wall. The fork semantic cases had not started and are not counted as passed.

The effective task's ~22:00 load-only wall/preemption and ~23:55 remaining250ms Hello rulings authorize the QoS deferral; tests/dream_c1.rs is in allowed scope. Source60 added **only that test's existing-ruling ignore attribute**. Removing it reproduces the entire source59 file byte-for-byte: old SHA256 `36fe1d5b706ee664ef157e2bb6973c1d93b226d67d4218aa9a5d75eed8be4648`, new SHA256 `0340e6cb42d8c0a6ea7ea61c1cc4137231b18f48ac6dd7b0d1882eac3c6b0b83`; proof `R/source59-readiness-wall-deferral-byte-proof.json`. Body, assertions,1ms/250ms/30s limits and production code are unchanged. Actual RED profile `R/retained-a7c08e65-actual-C1-profile-RED/preservation.json` was copied before the test edit; no metadata was invented. This is a deferred runtime witness, not a product fix, a proven scheduler cause or a success reclassification.

Current unique task-specific deferrals are **26 =21 QoS +5 qualification**, individually listed below and in `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-exact-deferred-tests.json`, SHA256 `016fa4b411a077262efc3f9814c016ea4973b73dd6bbf6016203f6ca6619571b`. The retired grant-law physical fixed-oversize negative remains separately DEFERRED/UNPROVEN, never included in26 or called a pass.

| File/line | Deferred test | Class |
|---|---|---|
| src/dream/resolve.rs:4835 | foreground_pause_and_cancel_reach_real_group_while_proof_before_waits | QoS |
| src/dream/resolve.rs:4888 | current_group_rejects_old_hooks_after_distinct_real_group_settled | QoS |
| src/dream/resolve.rs:4915 | descendant_cleanup_panic_retains_its_actual_launch_and_fault_requirements | QoS |
| src/dream/resolve.rs:4924 | descendant_before_panic_retains_its_actual_launch_and_fault_requirements | QoS |
| src/dream/resolve.rs:4995 | admitted_public_key_uses_exact_stdin_after_physical_file_deleted | QoS |
| src/dream/resolve.rs:5072 | owned_stdin_streams_multiple_chunks_and_closes_eof | QoS |
| src/dream/resolve.rs:5096 | backpressured_stdin_does_not_block_foreground_or_cancel | QoS |
| src/dream/resolve.rs:5131 | spent_preparation_cancel_retains_disposal_and_actual_settlement | QoS |
| tests/dream_c1.rs:2391 | source_blob_sequence_version_and_custody_controls | QoS |
| tests/dream_c1.rs:3113 | paused_owner_does_not_delay_another_process_and_owned_kill_never_completes | QoS |
| tests/dream_c1.rs:3184 | actual_over_limit_ir_corpus_is_rejected_before_reading | QoS |
| tests/dream_c1.rs:3411 | four_children_keep_independent_controls_and_individual_memory_shares | QoS |
| tests/dream_c1.rs:3477 | near_limit_rejected_envelope_drains_before_process_done_and_join | QoS |
| tests/dream_read.rs:3224 | prepared_tool_capability_probe_has_stopped_start_and_actual_git_gpg_rollup | qualification |
| tests/dream_read.rs:3918 | read_bootstrap_held_birth_has_zero_admission_then_paid_hello_start_and_actual_reap | QoS |
| tests/dream_read.rs:3973 | read_bootstrap_probe_null_context_permanently_refuses_matching_start_before_source | QoS |
| tests/dream_read.rs:4011 | qualifying_four_jobs_complete_real_proofs_with_foreground_debt_memory_and_reap | qualification |
| tests/dream_read.rs:4222 | qualifying_helper_foreground_cancel_settles_actual_descendant_before_done | qualification |
| tests/dream_read.rs:4298 | helper_provision_capability_and_executable_guards_refuse_without_path_fallback | qualification |
| tests/dream_read.rs:4424 | signed_publication_exact_bundle_survives_offline_sources_and_refuses_tamper | qualification |
| tests/dream_read.rs:4617 | no_candidates_protocol_pause_cancel_and_descriptor_custody_are_separate | QoS |
| tests/dream_read.rs:4859 | selected_dense_owner_and_total_input_overflow_refuse_before_publication | QoS |
| tests/dream_read.rs:4927 | stdout_backpressure_and_activity_flood_leave_cancel_independent | QoS |
| tests/fixtures/dream-bootstrap/process.rs:220 | bootstrap_held_birth_has_zero_admission_and_independent_probe_then_real_completion | QoS |
| tests/dream_read.rs:4760 | selected_multi_mib_four_jobs_keep_control_and_measured_storage_limits | QoS |
| tests/dream_c1.rs:2966 | actual_store_fork_is_explicitly_unsupported_and_parent_rows_never_pass | QoS |

All completed R/run.py-schema records as of collection are retained in `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-all-completed-actual-runs.json`, SHA256 `5b8ec749a149f62600276b8e3c11578cd17507f92652f0cca05eb4e6d85ec0c5`: 380 records, 110 nonzero,0 log-hash mismatches. Custom historical operation results remain separately preserved. C16 RED, source59 exact-chain RED, original source50/57 failures, state-cap/type/liveness/TLC timeouts, metadata-wrapper errors and source57 archive ENOSPC retain their real exits. Earlier controller-ownerloss remains quarantine/unproven: Rust29617/29747,75435/75533,97797/97846; C36046/36792,57264/58336. Later absence or Root's exact-PID cleanup never certifies original own-reap/full cost. All old signed cuts, retired inventories, tuples and reports remain intact.

## Fourteen acceptance bindings

Full current source/hash/line map `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json`, SHA256 `411f526321d218c3e214705e300b22a9985440039ff2bec5dbd6feb848e62a07`. These references bind the eligible current command results and explicitly retain the deferrals/qualification limits above.

| Criterion | Requirement | Source and evidence |
|---|---|---|
| wire | Distinct prepare-read wire, exact context/manifest binding and bounded immutable pages/descriptor. | src/frontends/headless.rs:32; acceptance map #wire |
| bundle | Finite selected IR/JSONL/blobs, private Git, public policy/keys, catalogue and artifact-to-file custody; no arbitrary receipt paths/fallback. | src/dream/read.rs:295; acceptance map #bundle |
| published_lessons | Compatible adoption, current record/trial/publication/receipt/trust/transitive proof, safe lesson fields and verified_at_revision. | src/dream/read.rs:2286; acceptance map #published_lessons |
| resource | Shared64MiB input; parent32MiB/child96MiB, protected control/helper shares, bounded allocation/disposal, four jobs and actual joins. | src/dream/read_wire.rs:14; acceptance map #resource |
| cost_debt | Own-thread ordinary1ms/native2msCPU25mswall3msinclusive/full9x debt unchanged. Decision66280 removes external per-grant/Q/T/N; helpers run continuously at atomicBACKGROUND/observedPRI4 with250ms command/3s job totalCPU, actual per-member RI begin-to-begin+return gaps<=50ms, terminal originalwait4/rawovershoot/fresh9x postreap debt/terminalRSS16MiB. | src/dream/c1.rs:748; acceptance map #cost_debt |
| bootstrap | Canonical128B permit/lease, unchanged250ms readiness and30s maximum work deadline; no argv/channel/owner/source admission before permit, null-context probe rejection and actual settlement. | src/dream/bootstrap.rs:16; acceptance map #bootstrap |
| helper_custody | Original positivebirth/audit/PGID custody, bounded2members, positive ownWNOWAIT/actualownwait4, known-native census status, retainedchild positive exitedRI then fresh sameleader qualified componentwise rollup; live/escaped effects before reap seal, independentgroup proof/ownedquarantine. KnownleaderCPU paid even if unknownchild held; no postownreap signals. | src/dream/c1.rs:829; acceptance map #helper_custody |
| signature_B | Git-created verifier temp argv/custody GS1/GS2; matcher never directly expanded; original inode/hash/scratch cleanup or quarantine. | tests/fixtures/dream-read/proof-helper-probe.c:3358; acceptance map #signature_B |
| shared_codec_C | Exact signed shared schema/vectors, bounded strict decoder and seven-source/private pre-test bindings, no runtime self-approval. | tests/fixtures/dream-read/proof-helper-probe.c:723; acceptance map #shared_codec_C |
| terminal_verdicts | Independent control/persistence/terminal-cost verdicts, bounded retained errors and post-tail receipt; fixture setup cost cannot disappear. Effective23:10 fixes bind admitted inputFD+freshhash independently of replaced locator; distinguish native IO failure from paid work-deadline refusal; one-way terminal reporting cannot admit work and attempts finite/paid result after original30s expiry. Bounded path batching retains full actualCPU/debt. | tests/fixtures/dream-read/proof-helper-probe.c:3874; acceptance map #terminal_verdicts |
| retained_host | One durable A close, finite subscriber/socket drain and retained Host creating one nonfork B with managed launch inputs. | src/engine/mod.rs:510; acceptance map #retained_host |
| formal | Finite owner/actions/bounds/fairness and all deliberately brokencontrols; honest66280 retirement/inventory. Exactnative cap100000 and pinnedJava fullstates/labellededges/namedverdictparity. Incomplete/statecap/OOM is failed; no sourcecurrent claim before freshcanonicalgates. | tests/formal/dream_read.rs:2625; acceptance map #formal |
| provenance | Signed exact source, current worker/compiler/source-input profiles and formal model/tool evidence; immutable Cpreparation/source4readset/schema/vector/compilerheaderclosure tuple separate from runtimequalification. Retain all failed runs/sourcecuts; no oldtuplecurrent. | src/dream/read.rs:2648; acceptance map #provenance |
| landing | Canonical current-source gates and remote main containment before final handoff. | scripts/land.py:112; acceptance map #landing |
