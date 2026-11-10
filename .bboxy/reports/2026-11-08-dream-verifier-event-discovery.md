---
type: report
title: Landed Dream verifier event discovery and INPUT62
task: '[[tasks/2026-11-08-dream-q2-verifier-event-discovery]]'
author: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
created: 2026-11-08T11:32:27.545Z
commits:
- 4967eb4378e3166aa7532630ee427b69d41a7dae
- 7ab80fac868b53b1dfc690ef774fc3bfabbb18b8
verify:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
id: 01M6XEWQZ3J1G1JVMDE6WNE7RG
updated: 2026-11-08T11:32:27.545Z
number: 93377961366359
created_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
updated_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  kind: seat
  public_key: 3lIiKAyaYQbqAYQ3xsrS+NTGLp9s//BjqCA4HkK5nF0=
  process_generation: '1'
provenance:
  digest: sha256:4ce08be94726d2915bb44f5d7216b525744925e92375b33a7d3931d8977c1c39
  frontmatter_digest: sha256:c6f6a08b6a166e18272c12bc4451a0f858ed53559dc2011befbfb5c5259e173b
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  sig: HpTDDmFVmZah0u+5Z6hPwDl6FNFgf0r4Lfry0UHmDVvL/Sci6vaWlGOc9FA/KzZ8GJpfFTkbVW8AmZY/NxC7Dg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150651
  event:
    id: 01M6XEWRW88F6Q6WTHP23RJY6D
    sequence: 150651
    hash: acdc913cd9208018d3b8cc2259619daaede84711a00aa08a02cc11fe5d0b8418
---
Landed Task66356 at signed final commit `7ab80fac868b53b1dfc690ef774fc3bfabbb18b8` on `bboxy450/bboxy450-harness` main, including signed implementation checkpoint `4967eb4378e3166aa7532630ee427b69d41a7dae`. Fresh fetch/ancestor check returned0; origin/main equals final HEAD and the worktree is clean. Both commits have signature G from Boxy key `9B07E849480C68328CA20684844ECC0587DE7AB0`, with Boxy <bboxy450@gmail.com> as author/committer.

All relative artifact names below are under **E = `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-verifier-event-01M6RZZXSHZT3KHBK1QC7K3XB3`**. Current Task authority is amended etag `162e70d5d6a7c4a3333fc7c078f926bdde044e4c1dd0039d6fe700ad0cac6bea`; the complete native record is retained as `native-task-amendment-between-polls-v1.json`, SHA256 `aee3adc56999f7369a5f8fb5de8de8289ba04ccb6f54ee7141e68b51d570c144`. This report covers the new event-discovery Task only; all previously accepted tasks/reports/INPUT61 remain frozen.

## Acceptance and implementation

- `tests/fixtures/dream-read/proof-helper-probe.c:2516` registers an owned close-on-exec kqueue EVFILT_PROC watch on the original leader for NOTE_FORK|NOTE_EXEC|NOTE_EXIT, after posix_spawn and before the first controlled continuation (`:3232`). Registration failure remains coverage_unknown. The existing sampler remains active. The bounded event wait (`:2552`) stays within the existing50ms sample ceiling; actual controller/native work and descriptor operations retain their ordinary/native accounting.
- `proof-helper-probe.c:2895` uses the existing census immediately after an observed fork. Child registration follows existing lineage/current-incarnation/full-token capture, then watches NOTE_EXEC|NOTE_EXIT so the unchanged sampler/GS1 path can run promptly. Event records never supply a child PID, birth/member count, GS1, cost, exit/reap or trust proof. First discovery cause is immutable; coalesced records are not birth counts, and initial leader EXEC can legitimately remain0.
- `:2919` services events before terminal breaks, `:3286` drains before original consuming wait4, and `:3409`/`:3444` retain the queue through retirement/quarantine. Queue close at2579 requires actual original ownreap and positive group-gone, is attempted once and is measured/paid. A missed-verifier refusal may physically settle its known original effect; it does not invent child identity/cost proof or force an unneeded permanent quarantine solely from the diagnostic.
- The exact distinct coverage details are **"verifier forked and exited before census"** and **"signature leader created no verifier"**. Missing-verifier classification precedes the independent GS2 error, preserving the first failure. The existing exactly-one-retained-verifier, GS1/GS2, currentness, strict tree peel and all CPU/wall/RSS/debt/lifetime laws remain. No new STOP/grant was added; existing GS1/foreground/cancel effects keep their prior authority.
- `private-engine-controls.c:1198` exercises the real owned scripted leader: queued fork after child reap, actual never-fork leader, and injected registration EIO before CONT. Every physical case retains actual original ownwait4/groupgone/full CPU debt and charged queue close. `:1156` separately supplies a100us production-API timeline, explicitly not an OS timing claim. Six C mutants break pre-run registration, event-as-proof, terminal drain, poll-only behavior, registration failure or queue lifetime and fail intended causal assertions. Physical poll-only and terminal-drain assertions occur AFTER actual settlement/payment/queue-close and retained numeric evidence.
- The fresh current canonical C suite inside the exact MCP gate passed: **83 baselines /54 source mutants /56 strict compiles /125 commands**. Receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-verifier-event-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical/private-engine-controls-67lj55ow/run.json`, SHA256 `677aab9ad9df97573aec9a29e0ddc09c10c5da7b3b6989a281b5eed35eab1eda`, contains actual commands/exits, C3 snapshots and raw ledgers; current_sources_unchanged=true. Existing signature positive/negative engine controls and strict parser/peel cases remain green under their source-known fixture premises, not real installed-tool qualification.

## Measured witness and Oracle amendment

The FIRST C24 measured physical run remains **FAILED1** at `event-c24-measured-physical-v1.json`, logSHA256 `d1244ae68183a48d8b73eedcca5b47ed69b5f1b0ab0dec752699b379bc49b9ff`. Its strict compile exited0; physical command exited81 at the then-required illustrative<1ms assertion; no mutants were reached. Detail `verifier-event-measured-aybz1sqw/run.json`, SHA256 `5be5ae8e89d8c50cfc2cb79722091b0500711c0ed08a55ed2483c02463491eea`.

Actual child41988 had RI result0/errno0, start38468975781051→exit38468975821295, timebase125/3, rounded-up lifetime **1,676,834ns**. The interval lay strictly between actual census ticks38468975339769 and38468976083904. Fork event1/missed=true, actual original ownwait4/groupgone, full172,998,000ns debt paid and queue-close0 were recorded BEFORE the assertion. No load cause was invented.

ONE native blocker was stored1445/id `01M6XBFMYDTCRPHK30A61VMH8V`. The authenticated Oracle amendment explicitly accepted that measured between-poll witness, replaced the illustrative submillisecond condition with measured birth+exit strictly between actual census polls, and prohibited timing retries. C25 removes ONLY that physical threshold in the fixture/parser and updates its diagnostic prose; all raw identity/timebase/ceil-conversion/strict-poll/event/custody/cost assertions remain. The original outer1 is never relabelled0. No narrow baseline timing retry occurred. Only the previously unreached physical poll-only and terminal-drain mutants were run next: current tail5 commands actual0, detail `verifier-event-missing-bwtk4a5u/run.json`, SHA256 `1811573eb7f0395db973f04a4388d3be076f3e89310b92a49f7fe12975eb0a43`.

The required final canonical suite then ran fresh under the amended criterion. Its actual baseline child47710 lifetime was **1,404,375ns**, with independent raw RI/timebase/PID/census conversion proof in the canonical receipt above. This is an actual between-census witness, never a physical100us/submillisecond claim or a claim of guaranteed verifier capture.

## Source, model and fields

Only eight allowed code paths changed from accepted basef6dffea0: C probe/controls/driver, `tests/formal/dream_read.rs`, `tests/formal/models.rs`, and `verification/{contracts,corpus,coverage}.toml`. Rust product sources, top-level integration/ignore files, `tests/formal/dream_c1.rs`, Makefile and scripts/land.py remain unchanged. Source cut `signed-source-cut-result-v2.json`, SHA256 `fae6b6e5282af7e07c069f23a2efe921d5e10dd3e5f6192766827afbb298e839`, records all exact bytes and both signed checkpoints.

| Current C source | Length | SHA256 |
|---|---:|---|
| proof-helper-probe.c | 338238 | `01e8578163768f9d463f70f1a1cc698a8f073e0ea91f911a7dd8c51465a5496d` |
| private-engine-controls.c | 145749 | `3741ebfd590867f49c156b7cd636135aa7c75ffef17f40533ebe8271da80c91a` |
| check-private-engine-controls.py | 50064 | `ae98c475ffa44ea1a4a557a74709839f5ae89b5290cf48add1b4cf77d58c73fc` |

C25 final freeze SHA256 `ce8673224c959523b3a6880f960ba323e126415007e46a87d76ae4d53860d2a8`; prior C21/C22/C23/C24 exact sources, receipts and the RED are preserved. C25 production probe is byte-identical to C22; later changes add measurement/physical causal proof and the exact authorized fixture criterion correction.

**No new terminal fields; terminal_fields_changed=false.** New ledger kind `verifier_event_discovery` (`proof-helper-probe.c:3030`) has:

| Field | Exact type/meaning |
|---|---|
| leader_fork_events | u64, coalesced leader records carrying NOTE_FORK |
| leader_exec_events | u64, coalesced leader records carrying NOTE_EXEC |
| child_discovered_via | enum string none\|poll\|fork_event; immutable first census cause |
| fork_seen_child_missed | bool; observed fork plus known empty census before any retained child |

All are original-owner snapshots: do not sum repeated rows or infer a process count. Additional raw ledger diagnostics are `registered_mask:u32`, `registration_errno:[i32;2]` (leader/child), `wait_errno:i32`, `queue_fd_held:u32` indicator, `close_attempted:u32` indicator, `close_errno:i32`, and `notification_counts_not_birth_counts:bool=true`. Existing terminal and ledger process-create excess fields remain `process_create_wall_excess_ns:u64`/`process_create_wall_excess_count:u64`, with unchanged known/scope annotations and refused-call accounting.

The separate finite family **HarnessDreamReadVerifierEvents plus five BrokenHarness controls** covers poll-only, late registration, event-as-proof, early close and skipped terminal drain. It has32 variables/11 correlated same-owner timelines/16 strong assertion macros. Existing434 model constructors/actions/properties/assertions/fairness are unchanged. Formal source cut `formal/freeze-v2.json`, SHA256 `f4abb7fd8a3167024c02f2c2405fd63cdde96ad610ed75c0f522c646367a083d`; v2 changes only coverage prose to match measured between-census lifetime, with exact token/AST identity.

Current full native/pinned-Java parity: **440 models /94 contracts /1 crate**, **3,376,317 states /11,364,544 labelled edges**, with every full graph and named safety/deadlock/liveness verdict equal. State cap100000, tools, heap, workers, timeouts and fairness are unchanged; complete_TLC_replacement=false. Native event delivery/coalescing/clock success, real syscall/descriptor/accounting behavior and GS1 bytes/inodes remain implementation witnesses. The model does not promise OS capture, a successful clock or arbitrary tool correctness.

## Actual verification and preservation

The exact required command `make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build` ran ONCE at final signed HEAD, actual0, `2026-11-08T10:44:03.173199+00:00`→`2026-11-08T11:18:19.989988+00:00`. Runner JSONs retain exact argv/head/source-before/source-after/actual exit/logSHA; all rows below record source_unchanged=true. Canonical flags used unset FORMAL_JOBS, RUSTC_WRAPPER=/usr/bin/nice, empty RUSTC_WORKSPACE_WRAPPER and CARGO_INCREMENTAL=0; no gate/cap/assert/ignore override.

| Command or gate | Actual exit | Retained log |
|---|---:|---|
| C22 narrow API/owned-process/six-causal controls; earlier C22 source retained | 0 | `event-c22-causal-v1.log` SHA256 `112c8ea8bd250d7c6bad56e1a6dbb377bcc3b4a00015eee122e93e6e29a03b47` |
| C23 actual poll-only/terminal-drain causal refinement; no physical lifetime claim | 0 | `event-c23-physical-tail-v1.log` SHA256 `b873a64c997921d532899d270b5094bf5b3cffaa10662ebdba0ff4260f6ec04d` |
| Current C25 missing physical causal tail (5 commands; no baseline timing retry) | 0 | `event-c25-missing-causal-tail-v1.log` SHA256 `4dfc8f2dadce906ec0e30be4e69b4e322d317439523269e2787dd46e35090409` |
| Exact new finite-model test:1 passed, all6 graphs/controls | 0 | `event-verifier-model-focused-v1.log` SHA256 `099bb87daedfdd42a8d0085364832e88c429a34b5fe9408bcf99e28f3cf6358c` |
| Precommit strict Rust Clippy/all targets; Rust+product C unchanged afterward | 0 | `event-strict-all-targets-v1.log` SHA256 `ba593c8251f2746a48179f3e6f4b6ffc09dcc06ca5423256b5a98a3c5794e0f5` |
| Precommit strict Rust Clippy/fixtures; Rust+product C unchanged afterward | 0 | `event-strict-fixtures-v1.log` SHA256 `023ffebb272a5cff85fe990c37781efce6e8c9dda8a1b5f41ccb6b73b4f569d7` |
| Exact required four-command string ONCE on final signed head | 0 | `event-exact-four-chain-v1.log` SHA256 `4d6663040e21ce643de2e88c7c9a74494f374f64cbf3cfb0d6a2486a196fabf4` |
| Canonical check-formal-evidence on independently cloned current archive | 0 | `event-formal-clone-validator-v1.log` SHA256 `5108dacf1bb4ab4e27ae0b7261ac7694840d4e04f0f29186f2c27587b460c2d6` |
| Canonical prepare-dream-read-c-probe for exact final source | 0 | `event-preparation-input62-v1.log` SHA256 `15afbd49365f042f09b0fdd96242dc5747aa8ea78f54a91098fe0783bfc5402e` |
| Independent preparation audit, actual997 checks/325 headers | 0 | `event-independent-audit-input62-v1.log` SHA256 `8ae5245446d215e81f28b8eb742b0af437ca729188897c2f399869fbe90aee12` |
| Literal canonical make land | 0 | `event-canonical-land-v1.log` SHA256 `5766ac876f82f010352d790b707b90d2328576bd2537e0bac28f3d5ddc1ef18d` |
| Fresh fetch, remote contains signed HEAD, worktree clean | 0 | `event-remote-verification-v1.log` SHA256 `05208d3933ff908f9988f260334f1d0ff792255ca5f229030775c607625f6ca0` |

Actual completed C1/READ profile metadata was copied byte-for-byte immediately after the exact chain, with independent readonly workers: `retained-7ab80fac-actual-profiles-exact-four-chain/preservation.json`, SHA256 `e98459ce291a02eeb7c990c71a59a6b010361748815f04024c3d439b0435de16`. Current worker SHA256 `8bf260626fcffca11efd2b9aafd4190749e9e4030d4bd491e448bc365274f522`; READ39 source inputs. No old metadata was reconstructed. The canonical land.py preservation helper retained all five observer JSONs on their same inodes after actual MCP0; detail SHA256 `70080fd0c964b33415fa7f05cc39842fe1e24f01fe9c56fde2f41808431f5ec8`.

Immutable current formal archive `/Users/feral/mydev/bboxy450-harness-dream-read-q2/verification/evidence/20b1ab965ec30a14a9450dbb85c3a9264317f5d1500790e7aee285e9cb6e6473` has evidence SHA256 `20b1ab965ec30a14a9450dbb85c3a9264317f5d1500790e7aee285e9cb6e6473`. Every4402 raw file (7,431,390,709 bytes) was hashed before sealing files0400/directories0500, with original modes/inodes journaled. Manifest SHA256 `2fa73050f9bcafa70445fe5c4f88e6558c48e39d60c8c847eda99c1ea33602a2` binds all raw bytes and the `cp -cR` writable clone at `canonical/harness-landing/formal`, with distinct inodes/nlink1 and equal hashes. Canonical check-formal-evidence0 validates the clone. Disk was preflighted30.65GB free; only owned prior-green working-lane caches were COW-seeded after lsof0, preserving immutable INPUT61/current-green archives. No peer cleanup or failed-formal full-cache retention was added; this Task had no failed formal run.

Literal canonical landing `make land TARGET_DIR=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-verifier-event-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical` returned0, ran strict selected checks/current-source formal validators and pushed both signed commits. The already completed exact C1/MCP command was not fabricated as a Land row; canonical path-based landing selected its normal checks. Receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-verifier-event-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical/harness-land-receipt.json`, SHA256 `c6e461f966a7cadf2d7f21d5a9345ebd62c219e6d27fa94d8cee2c5b710319f0`. Fresh remote verification detail SHA256 `e2c3a1aa44bef14b704d5eaa440e8223fc46bc0df8c16739dd76a6c00cdc1f6d` proves origin/main contains final HEAD and the tree is clean.

## INPUT62 and limits

ONE required INPUT62 tuple was stored1448/id `01M6XEN9XDZ52N5XSD6DK948M3` in original Delegate thread `287ZPT89ZK9RRPDBSPGR1JWKBB`. Storage is not model receipt or runtime approval; no ACK or builder execution is claimed.

Tuple `INPUT62-final-tuple-v1.json`, SHA256 `ed5a20501df0327296578e30095521815b7125c78edd3fc5dc91e633d073c09b`, includes all ten exact input paths/lengths/SHA256/input commits, source4 snapshot/domain, actual compiler/version/argv/SDK/platform/linker, complete header closure, executable, receipt/audit and current evidence. Source/preparation commit both `7ab80fac868b53b1dfc690ef774fc3bfabbb18b8`. Receipt SHA256 `889764ffa41be7bbf486be80afb49e74e5b58d9f81a522a1516609043468dd1d`; immutable251272-byte probe SHA256 `bab19b967fed5adc1ac82f278bc5869657fc54242ef522c0e4fe897a6b09523c`. Independent audit SHA256 `207d4db89f1789fb2c69fef6ee5aea88ff67f60ee50413dae8e61875f99786a5`: actual0, recomputed997 hash checks/325 headers (not copied from INPUT61),0700 roots/0400 files/0500 executable/nlink1 and source signatures. Actual lexical/resolved `sys/event.h` snapshot is17944 bytes, SHA256 `435ed8fe99ae70c19404c3da14be6e2d217686ac256aed3b5df721b353dbb350`; pthread/spawn closure is also audited. Compiler SHA256 `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`. Snapshot domain `8e6d9207723751cfb936587ecac7f06673321acbca05be739613e0736c2e4ee8` alone is not a compiled-product-currentness theorem; current actual product/profile bindings are separately retained above.

**runtime_approved=false; no genuine builder run.** Previous builder `01M6X8ZSXWJQAF6DZH8VE88G5F` remains FAILED81 per the Oracle-verified Task (partial ledger digestb3239d0a...d5b4 only; no peer raw path read or invented full digest). Its15/15 settlement and23.140558494s paid debt describe that failed stage, not current successful qualification. Event capture/GS1, real tool qualification and whole-private30s fit remain unproven. All existing66280/native/ordinary/helper CPU,9x debt,66331, strict peel,30s/16MiB and exact settlement rules remain; no performance expansion.

Existing26 Rust deferrals remain unchanged/nonpasses (21 QoS,5 qualification), exact names at `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-process-create-wall-ceiling-01M6RZZXSHZT3KHBK1QC7K3XB3/current-historical-deferred-tests-v1.json`, SHA256 `8d21bc964e7f15c33d58f4547a81528e9e0f375db980621c6b903258e3e06607`. No new ignore/waiver or timing retry was added. Historical retired-law physical oversize negative remains separately deferred/unproven. All tests ran unattended without operator/GUI/credentials; all prior accepted records, immutable inputs, raw failures and their limitations are preserved.
