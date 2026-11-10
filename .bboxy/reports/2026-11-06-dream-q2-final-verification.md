---
type: report
title: 'Dream Read Q2 current-source checkpoint: formal scheduling and upstream GUI blockers'
task: '[[tasks/2026-11-05-dream-mcp-verified-snapshot]]'
author: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
created: 2026-11-07T03:14:33.434Z
commits:
- 3ed45c8e1e35021b364a40c8d1f6ce22313bc46e
- 532d770867089faa7247d1bde37a10c526a468e1
- 775917fae20eea7290e46f215f5af65c6b67178b
- a721742c7cb8f0d332c25275b8dd789bd083c6ce
- 452536944b4c9dd39c2b83ae174c39b5ed207686
- 50e672c02e2ef5b53562634be0a5fbdc529561b7
- 5ad1708cae3f5800908b625b17f69c5cdb84fc18
- 25750398c3f6cf775a6e6d2348f7a4014b3acd32
- 6b224ff8e75f435c321aec12852163dd80757ee6
- 0e4d00e8a669fe3880fbfb9f80792ca246b3da8f
- dcb0e8db713a99073629e0424b4360e92ddc9b85
- c5ac857e3eb6cc8d4666abaf765009da8e127063
- 121d062e6a3e8a86f58366c995593cca768fd99b
- f5b2af3678bcedd993f9d4a64afbbb2b7076e4f8
- 75ee45487b4a5f6c4bb70b0e4eda93af02dcb935
- bc04108cb23e01190994f4cd9b691dbfb2ed5cc9
- 4c4e72cf57fefbac75696016d4d954e03bff9030
- 6f13ce9fffd654b98cd05bb91168d7bd9f57167f
- 1ca5ba96940bea139d832201bda25b1fa97e5a41
- d1e5e518eca04d2ded0f96464c27c53f622688ad
- 33b32a14294763d88b0b05af59245e9b39de7e80
- 94c9c59d4e8adffe0f749aa01ea842e3474f5dc0
- ecfea996f117ad0a1281648995a247605f17e21f
- 6802a155ca24409dc1f3debba8a4e183b08e71d9
- bca7f09448a7cbd057888ebb3e073a1c4ee74fb4
- 944f99bbe306454c76eb7f0f266f20e18db34e30
- c657bc94bc5e73bd22036afc642fca17bbc24ae8
- 3d5e00252bf897e9017e9c2c1a8e81aa05cf2757
- 23c751ed4af22853680de7a98673a67c17a2a349
- c447f2039f04f25045719107f96cab734b5f8b61
- 54050b6eed60964348362cfcf8691a7b93462166
- a9f00eea1f4c52fe68185ae4bf826736cd605211
- a13e8237c8d1b34a7a4e7cf449f1a3ef4e157d19
verify:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 2
supersedes: '[[reports/2026-11-06-dream-q2-current-source-verification]]'
id: 01M6T00AHMJASXXNFGC3FQAD4B
updated: 2026-11-07T03:14:33.434Z
number: 93377961366209
created_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
updated_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  kind: seat
  public_key: 3lIiKAyaYQbqAYQ3xsrS+NTGLp9s//BjqCA4HkK5nF0=
  process_generation: '1'
provenance:
  digest: sha256:cf23273518f253398d8e666d94d8c11460dd58c1e2fbdd75d40864b91c49dab1
  frontmatter_digest: sha256:0a1353493791eaf7f037c8ff9b88376881f732f3cdd7862b19214dc9ab1187e1
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  sig: V8eK3O+HR3cajjMfQnEX8CP2pwVvqxDewI0dbGYcr3v545AWH8O7o3ewujfbXQq4zUHNSNjUjr7mXfnx6AR0Bg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150132
  event:
    id: 01M6T00BY37ENH49WW1CCSHNBP
    sequence: 150132
    hash: f6b3b419723af76e8a582836a84ef48d8f2803eadbbfb8b7afde9a8d91b12b0b
---
Current-source verification is blocked by two out-of-scope verification failures. The Q2 source is signed at `3ed45c8e1e35021b364a40c8d1f6ce22313bc46e`; it is **not landed**. This append-only checkpoint supersedes [[reports/2026-11-06-dream-q2-current-source-verification]] and preserves every earlier red result.

All relative paths below are in `/Users/feral/mydev/bboxy450-harness-dream-read-q2`. `R` means `target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3`. The effective task etag is `4732f05c51b943d97babc6dcbdab1613172385cb098eefa4e0ae1c7254202a02`. No production budget, deadline, endpoint, qualification or runtime-approval waiver was taken.

The exact required command was:

```sh
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
```

It ran unattended with the compiler wrapper `/usr/bin/nice` and `CARGO_INCREMENTAL=0`, at unchanged HEAD and source hashes from 2026-11-07T02:33:37Z to 03:02:13Z. Actual chain exit: **2**.

| Required gate | Actual result |
|---|---|
| `make test-dream-c1` | Exit 0, including 38 C1 cases, existing bootstrap controls, actual StoreFork and seat/socket checks. One authorized QoS deferral remains. |
| `make test-dream-mcp` | Exit 0, including wire/schema/source guards, source-known C controls, initialized four-job and flood/resource witnesses, adjacent Dream checks and ACP 10/10. The 18 other authorized deferrals below remain deferred. |
| `make test-formal-parity` | Exit 2. Native exploration passed 448 models and 5,841,767 states. Only 176 reference parity rows completed before a fixed 60-second TLC graph timeout. No current full reference receipt exists. |
| `make check-embedded-build` | Not reached in the chain. Run separately afterward at the same source: exit 2, because upstream GUI `face.rs:2381` violates `clippy::clone_on_copy`. |

Full chain log: `R/final-required-source-3ed45c8.log`, SHA256 `5a7fd1596bcfe9b898da3bd461d73097bf1e8540de695287c3749fd09e562253`. Structured failure summary: `R/final-required-source-3ed45c8-failure-summary.json`, SHA256 `80720ed07ef235730ae147572f7467711fdab0b6002626f56ae5a27716222e0b`. Separate embedded log: `R/current-embedded-after-formal-timeout.log`, SHA256 `a16953710688f88f544c0db4830d01fed6dff3884f73dbdd4985cdb9465ae947`; its command/source/exit JSON SHA256 is `14ea4239913d02140119d6cac8e242559f0f3d36fafcc6c6bdedc9e2e9dfe1b6`.

The current C1 and Read worker files are both 33,165,888 bytes, SHA256 `250d263def464cbd07ea2cd096b856af822768991fe99b003c123e747c84c053`. Their hashes manifests are respectively `b37c9ecad2df86df0fda15c8d45a625bdde4d282e0920486e3e64f1403a3896c` and `1fa1d1efe6fa013532da1d966c2e24735fd5e6630e128d0721eb8be2d814570e`. Read source-input JSON SHA256: `c4dbc62d7d4070603f0407ea647b2fe24ce9ada49de38507b65820f6b06339a6`. These identify this run; they do not qualify the proof-tool profile.

The formal failure is `BrokenHarnessDreamReadQualificationPrivateRuntimeApprovalGraph`. Its current TLA, graph/property CFG, native states, native edges and metadata are byte-identical to the retained successful historical row in `verification/evidence/9192fc89b072e2734a11a4b222901be54d7791469551ee4d6799c4e50fbb695f`. That historical graph finished in 25 seconds with 38,962 states and 114,633 edges; the current graph timed out at 60 seconds. Current TLC/JRE archive pins match the historical receipt; the historical extracted Java executable was not separately recorded. Contention is an inference, not an established cause. The old archive is historical source `f4d3c4defadd7e3ae27c8342cbe15a63f06d5f56`, not current-source success.

Diagnosis: `R/formal-runtime-approval-identity-diagnosis.json`, SHA256 `e47032c75fc10acb05678ac0b80e46167decc31cea2a321ef68805f68eee7f14`. Fresh failed graph log SHA256: `2010e2e94029afa656a748704ba7d0aed8e9df8edac817656fc47f7adf9119a7`. Native evidence SHA256: `5b94ee8150bfd2dff7b95007409ed2bc63bc1a217811964982a4036036b3c2c8`. The adjacent old property log was not freshly produced in this attempt.

The GUI failure comes from existing upstream commit `6d9fc2e1869c1ce1333f959f36ed133824f825e0`: `(rect, paint.clone())` copies a `Paint` which implements `Copy`. The reviewable one-line correction is `(rect, *paint)`. Q2 has not edited that GUI path.

The exact scope decision needed is to grant `scripts/formal.py` and `scripts/test-formal.py` for the reviewed optional reference-scheduling control below, and either assign the GUI correction upstream for rebase or grant the single `face.rs:2381` correction. Those three paths are outside the current allowed paths. No actual patch was applied and no unchanged full-suite retry or timeout/cap/coverage weakening was performed.

The unapplied scheduling proposal `R/proposed-formal-jobs.patch` (SHA256 `e2bb6513df686cfd91825655a3ca8d96a8a2a3dc19ea1754291fe227d2a8e421`) validates `FORMAL_JOBS=1..4`, default 4, and records/validates the actual reference row concurrency in the receipt. It changes neither the 60-second per-process timeout nor any model, reachable graph comparison, expected verdict, pinned tool, inventory, source-hash or freshness gate. After an explicit scope amendment, the proposed next full run would use `FORMAL_JOBS=1`; no claim is made that this alone will resolve the timeout.

The complete temporary-copy unit suite passed 17 tests, exit 0, with no Java/model execution. Log: `R/proposed-formal-jobs-full-suite-fixed-text-receipt.log`, SHA256 `a758130acab30dcafbb5559ae4317daaf1753c84cdfe09e7a36e2b6c77b105d0`. Controls reject missing, out-of-range and forged-type worker receipts while retaining the existing source, cleanliness, input, tool, inventory and model-hash checks. Earlier proposal versions, the initial import error and the failed bytes/string fixture mock are preserved; only the final corrected unit suite is green. Review manifest: `R/proposed-formal-jobs-full-suite-review.json`, SHA256 `3c4d3e9d120db2feb16bb88d03fd012e7c4b2f393ccf0d8c1884dfa43c127138`.

The separate unapplied GUI proposal `R/proposed-gui-clone-copy.patch` has SHA256 `568a56b8d551a200422062241f6243508afbcc217db0245bf40534564bbb5ccf`. It contains only the one-line clone-to-copy correction. Neither proposal was applied to actual source, compiled as a GUI change, committed or published.

| Acceptance area | Source evidence and current boundary |
|---|---|
| Sibling wire and strict schema | `src/frontends/headless.rs:32`, `src/dream/read_wire.rs:637` and `:694`: read-prepare/1 sibling, strict DTO/schema/goldens and independent canonical hash checks. Current MCP gate exit 0. |
| Exact selected sources and proof | `src/dream/read.rs:2112`, `:2147`, `:2398`: exact manifest, inventory and descriptor custody, selected-only Resolver and trust/proof path. Shared 64 MiB input ceiling retained. No provider/catalogue/draft fallback or publication adoption. |
| Safe outputs and honest refusal | `src/dream/read.rs:2439`, `:2610`, `:2719`, `:2722`: failed/unproven distinction, currentness checks, bounded pinned fields, 16 KiB pages, 4 MiB aggregate and descriptor-last closure. `verified_at_revision` is not live consumer authority. |
| Ownership, time and memory | Original PID/PGID/FD custody, own reap/group-gone and terminal proof survive refusal. Original ordinary 1 ms, enumerated native 2 ms CPU/25 ms wall, application 1 ms/inclusive 3 ms, fresh 9x rest and max(W,A)+C bounds remain. Parent 32 MiB and child 78+16+2 MiB remain. RSS/opaque memory coverage is measured, not an allocator theorem. |
| Process witnesses | Source-known C and bounded failure controls, initialized four-job resources and original three StoreFork cases passed. Cold-positive readiness and qualified proof jobs remain unestablished. |
| Formal evidence | Scoped changed-model native/TLC graph/verdict controls passed below. Current complete native exploration passed. Current full reference parity, all landing gates and source-bound archive remain unproven. Finite parity never proves arbitrary Rust/C/kernel correctness. |
| Qualification and landing | Five qualification tests belong to the proof-tools task under the latest ruling. Runtime approval remains false. No make land, merge, push or remote-containment claim is made. |

Git state: 33 branch commits above last fetched `origin/main=216cf0d4b31ee6503479f0a31ded85174ef15352`; every branch commit reports valid signature `G`. Tracked worktree and index are clean. Five newly generated observer JSONs are preserved untracked at their original paths; no clean status is claimed. Exact paths, inodes, sizes, hashes and signature list: `R/current-verification-blocker-state.json`, SHA256 `d4a91b6d7bdc3ad876c6cb6b92554d7c66ef9f1451e24c818bf6a4ef7ae385a9`. Earlier work, evidence and checkpoint branches were retained through signed rebases; no reset, stash or drop was used.

Repairs and scoped checks:

* Initial second-snapshot exact SSTOP4/one-thread/one-running is pending only before settlement/ready/running/grants/STOP with original identity/path/fresh zero-user RI; no ACK/CONT, original25ms/fresh full sample remain. Wrong phase/status/thread/user refuses. Real START_SUSPENDED control0/initial4 parity0 (464states/472edges); historical guard92 cause unproved. Evidence: R/initial-continuity-pending-a5t9u_hi/native.log and R/initial-quiescence-parity-mz1zsdao/terminal-parity-evidence.json (digests in superseded report).
* Spent argv/env/actions preparation disposed within every measured postspawn/failure cutoff, independent actual PID/FD custody retained. Active2ms Attach/ordinary-destructor controls preserve full cost/no CONT on refusal/actual reap.
* ABA9 permits ordinary verdict/debt at A1/B2; effect mutation only actual A3. Good/export unchanged19230states/56119edges; broken21090/59769 only intended custody safety. Rustc/native0/parity0, R/quantum-aba-verdict-progress-9uz24k5h/parity-evidence.json SHAb5620c8ce4999951f28db3470e7b3f13d70ba344b2d1d80eb656dcb2ade2c65a. First unsupported-ite compile1 preserved; no seed/verdict workaround.
* C retains bounded per-code counts/overflow/detail samples and independent control/persistence/terminal verdicts. Actual publication/stdout facts distinguish persisted/not_persisted/unknown. Diagnostics follow paid tail; own final receipt write has explicit cutoff, no self-durability/lifetime claim.55 source-known controls/11 mutants compile0/run81, normalGit81/malformed-prebirth81, candidate executions=0/runtime_approved=false. Run: target/dream-read-c-audit/independent-verdict-closure/private-engine-controls-ns8d3d8r/run.json SHAaf30fb6f6b9e84300e94fb1a64a06a067499c8357854a8107fe5f12726c3102d. Four verdict graphs154/154/154/181states match Java0/12/12/12; wrapper1 was corrected mandatory-check inventory field. Artifact/inventory0 reused unchanged outputs without retry: R/private-verdict-axes-h0kzkcsu/artifact-and-inventory-evidence.json SHA4892684b5e39123fa84c13c8afc9a2d8c9d5e701b577893f7dfe8d5490c8b60e.
* C prompt lower-bound guards/bounded stage clocks/late-STOP and over-controller negatives retain original W. Oracle retracted alleged C quantum defect: W8381000/A6667/C404417 correctly refuses delayed ACK. Rust W120000/A139625 remains unattributed interval-upper-bound refusal. No clamp/subtraction/split/backdate/new baseline.
* ACP bounded failure/journal evidence plus native context before Python request: held-order control0/downstream1/1 pass, actual original reaps. R/acp-held-startup-order-corrected-control.log and R/acp-downstream-after-bounded-startup-fix.log; latter SHA4291c798fdfb516e92f8c7e082f9fc28b3e7cee7e324a98514ef10ac37c37f70.
* parent_paid now pays BEFORE refusal: real2000458ns CPU, owed18004122/rest22512750ns, control0 R/parent-paid-debt-control.log SHA87c3bf60123fd26ed257745cba5ccd19ea1eb1e654c912b3ae8ea5058268be4c. Each guard row/five preparations/pre-marker/each marker/held callback uses disjoint bounded paid units; outer aggregate removed. Reader setup also parent_paid(reader_at) before unchanged CPU/wall assertions. Original deadlines never reset.

One separate UNGRANTED source-free probe precedes Read four-job creation: unique IDs/context:null/missing path, zero Hello/permit/Start/source/owner; OWN lease close and ONLY private typed revocation caught. Other failures propagate. PID73904 actual EOF/reap2/no TERM/KILL, receipt retained; then four distinct resource-worker reaps. Exact1/1 pass18.28s: R/four-jobs-with-cold-ungranted-eof-control.log SHA341786fe8673f6cb3b05d16e631ce675018e4743d905cfc576b18582e818e1f0. Bytes9515172, heap399568/RSS17252352B, maxUnit604667ns/overruns0, credits36439736/100663296B, control448833ns, debt6489000000/rest7020874690/remaining0ns; helpers0. Initialized positive resource coverage, NOT cold-positive readiness/qualification or accepted failed-positive retry.

Analogous C1 ungranted EOF initialization precedes original THREE StoreFork cases with unchanged assertions/limits. InspectC1 silent EOF is canonical, unlike ReadPrepare refusal text; own child/lease/stdout/stderr EOF/reap2/no signals required. Preparation0 log SHA6a98823be6b98ca2cd1ed532d111c44fda85c3521f3a5c3cd8b56f5c5427de2c; StoreFork1/1 pass24.78s R/store-fork-with-cold-ungranted-eof-control.log SHAc889fdab31233ef761575efc81a30cf6c4b34775493b44c032fe78fb0b1d17a2. No cold-positive claim.

New frozen C INPUT, preparation only: signed35d3e60288d394ff0f0d9e2e6abb521d3d60b007 retained after rebase. C331208B/7a7536546a8cae7d088a3a372d1baeb26f46a3990e2524a792624f59ae12030e; exe269176B/86c5c9b5705f117cefda90f0d0bcd892813c631cf7aa9de573c19607be5a8858; receipt514287B/d66feb400b89b47aac0e55fc2d2ada994362bde770a398108fe02f1fb1d318c6. Directory target/dream-read-c-audit/final-signed-verdict-preparation/dream-read-c-preparation/compile-2ezbx4j3. Preparation0; R/final-c-tuple-audit.json SHA21a7d6be6098f59d0630b50ffaa2dbd6643c1d998a24ea7efc37ab4db07232b7 verifies352 output modes/hashes/323 headers/pthread symlink relation;0700 roots/0400 source/0500 exe/single-link. Apple clang21 SHA1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9, strict-O2/C11/Werror/arm64 SDK27/compiler/linker/header closure in receipt. Original b26d7cd3 schema/vector SHA d70948ec375dae099ea1dac0b92dfb30ff87d7fcac94c98f42caa1c0ba2da8ec/bff32c1afb483980952a6761918d4005c9f1e12b66f3d7aaaa88f1b9afe09740 unchanged. The current C and four Rust source files byte-match the retained snapshot: domain fbc2c762fb99e3537bb146056b35241bc7e581899bdc2086d97857d05afb52c8 is bytes ONLY, not compiled-current-Rust-product proof. No real candidate was executed. Qualification and runtime approval remain false. This complete tuple is ready for the Oracle's builder freeze; no delivery to the builder or model-consumption acknowledgement is claimed. Old140e/76f/a499 historical.

Scoped model SHA256: dream_c1.rs a91d5ec35e5db9cd87cc2ad6a4a39d7cb9019a63b1b90857278d57281cc8178c; dream_read.rs78ed0c09bc005b4c2fc891b7c7ce4e85459dfba440f13dc81c660bba6f0d06e6; models.rs3d2fc6efaa67aaaf00cad32ff3b2ac513da2606742637ce1b82c8890fe56c94c. Full current inventories/source/tools must bind final formal receipt. Current extracted Java executable/TLC jar SHA0f642542ea1f0be6957557c0db96653ec128113bb5a4aaa7c2534d1d72ddb51f /936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88. Finite parity is not arbitrary Rust/C/kernel correctness or complete TLC replacement.

All19 deferrals are DEFERRED, NOT PASSED. R/authorized-deferrals-after-dense.json SHAb95504270910cdd03a41d529df4f02908b8036b3696d2f4e36059a8903254358 predates later test hashes; named list/reasons still19, not current-source proof. Qualification reason: separately qualified proof tools, re-enable via tasks/2026-11-05-dream-proof-tools-provisioner. QoS exact reasons: load-sensitive budget/wall window OR wall/preemption window, re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp. Assertions unchanged.

Qualification5 (tests/dream_read.rs):

- `prepared_tool_capability_probe_has_stopped_start_and_actual_git_gpg_rollup`
- `qualifying_four_jobs_complete_real_proofs_with_foreground_debt_memory_and_reap`
- `qualifying_helper_foreground_cancel_settles_actual_descendant_before_done`
- `helper_provision_capability_and_executable_guards_refuse_without_path_fallback`
- `signed_publication_exact_bundle_survives_offline_sources_and_refuses_tamper`

QoS resolve8 (src/dream/resolve.rs):

- `foreground_pause_and_cancel_reach_real_group_while_proof_before_waits`
- `current_group_rejects_old_hooks_after_distinct_real_group_settled`
- `descendant_cleanup_panic_retains_its_actual_launch_and_fault_requirements`
- `descendant_before_panic_retains_its_actual_launch_and_fault_requirements`
- `admitted_public_key_uses_exact_stdin_after_physical_file_deleted`
- `owned_stdin_streams_multiple_chunks_and_closes_eof`
- `backpressured_stdin_does_not_block_foreground_or_cancel`
- `spent_preparation_cancel_retains_disposal_and_actual_settlement`

QoS read5 (tests/dream_read.rs):

- `member_windows_use_post_ack_unequal_endpoints_and_closed_roster`
- `read_bootstrap_held_birth_has_zero_admission_then_paid_hello_start_and_actual_reap`
- `read_bootstrap_probe_null_context_permanently_refuses_matching_start_before_source`
- `no_candidates_protocol_pause_cancel_and_descriptor_custody_are_separate`
- `selected_dense_owner_and_total_input_overflow_refuse_before_publication`

QoS C1 oversize1 (tests/dream_c1.rs):

- `actual_over_limit_ir_corpus_is_rejected_before_reading`

Older reds stay red. Prior report and R/final-verification-manifest-v2.json SHAd84abbb14ceadaf7fc3b4f18b117e0998118fadb0b873bbafa29ad8e968a9e4e preserve older GUI/readiness/compile/formal/verification failures. Six later exact-chain exits were2:

| Run | SHA256; disposition |
|---|---|
| final-rebased-required-verify | 9c3539adba6b9a672470f4b444f8290c7c0bc7463f4e431e0e85ff1a91599c15; oversize Hello deferred. |
| final-required-after-readiness-deferral | ab6c7be4a99edea165dccf5643584e2295956823731738893dbfb3b2d1d85d87; held Hello deferred. |
| final-required-after-held-entry-deferral | 925e0e5f71ee869c9c27df3055aa70871b21b9afb512acf82a621095076f4069; probe overrun; paid refusal fixed. |
| final-required-after-paid-failure-fix | 04c4e334dcb19766ea833e60f0166556ef0a46c201e2bbbbaf576b6afeba1301; aggregate overrun; bounded units. |
| final-required-after-bounded-parent-and-late-dense-deferral | 18b69cb8960a9fd24ff4033e8d19dae342dc3f8c3ee45d948f771d98311272d6; four-job Hello; separate EOF control. |
| final-required-with-cold-eof-control | 0012330e30e9f42315458560f15fbc7bd30ce8fab4e0f589148ebce5dc7a5e37; StoreFork Hello; separate EOF control. |

Narrow selected_85fe470923e3fa1f654d684c0e34cb4b5eec0201d88405bc7ee87bbd13e2d454 was101 despite four-job/two-root passes; Dense Hello failed. No partial/historical/scoped success substitutes for full current green; negative controls accept no arbitrary failed positive.

Preservation:15 observer JSONs moved SAME inodes to R/retained-observer-summaries, manifest dc6ea7c26ebc467e7b7e8f2f11ca78dbdc57120dcec5e6d09290ba5100136335.609 cold owned recovery graphs transparently compressed17205149696→1078435840 allocated bytes, exact paths/bytes/hashes/inode journal retained; R/compression-recovery-v2-result.json SHA9f0a14b0f7aaf9c79fffc22eb34a3e73c21cd0336ea3ae5acb52b631de887e2c; plan93d34ce0dbcc7b820caec83bf6ed038aca4470b10bbfe55b156b2bc66bed7a36. First856MiB unsupported-copy assertion failed, originals untouched, only created verified duplicate removed. Immutable archives/tools/receipts/red logs remain. Earlier2670358528B cold-debug pruning journaled in old report, not net free-space claim. No peer process/worktree cleanup or credentials/private keys/raw transcripts added.

Remaining boundaries: qualification belongs to proof-tools; source/platform/cache/lifetime premises conditional; RSS measured coverage; cold-positive readiness and19 deferred tests unestablished. Artifact_claim rejection is the recorded harness bug, not task-report blocker. This is a blocked checkpoint, not a completion or acceptance report. No task_report, landing, merge, push or new real candidate execution has occurred.
