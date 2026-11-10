---
type: report
title: 'Adaptive Q2 freeze: direct GPG measured quantum and retained settlement refusal'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6TPMTXM8MQCQXMCVBW60YGV
created: 2026-11-07T15:44:20.052Z
commits:
- 913224f577368c9bbc70c9afce57a91e099957c6
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
supersedes: '[[reports/2026-11-07-dream-proof-tools-fixed-t-66236]]'
id: 01M6VAX6TBQRDJ8EQ9RN5PZDXA
updated: 2026-11-07T15:44:20.052Z
number: 93377961366278
created_by: seat:01M6TPMTXM8MQCQXMCVBW60YGV
updated_by: seat:01M6TPMTXM8MQCQXMCVBW60YGV
authorship:
  signer: seat:01M6TPMTXM8MQCQXMCVBW60YGV
  kind: seat
  public_key: bBPoi7SjWcPpxvEzz3+5Ck7p1MHLoPov5NPy/GmicZM=
  process_generation: '1'
provenance:
  digest: sha256:e27dd633eb83dc302ce48090551a56626d262d9db1195ed9810b1ee454698441
  frontmatter_digest: sha256:524a7356c971e0720c98ce4b0edf1d73254bb2efbe071e5bd2a258568aa3309c
  signer: seat:01M6TPMTXM8MQCQXMCVBW60YGV
  sig: 7QF33bKtFGtY31CAbxbxGf/mZ7Yh8n4u4I2oEll2c+Xn1bddDkjTvtUQPFbWZh2EEnGGLvezo4RIJk6wchrdAA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150368
  event:
    id: 01M6VAX85ZQ7TW0MARQN3VP7MR
    sequence: 150368
    hash: 03f6aaaad17d918119a68d7cac207650017e01fd330e3014a7ca6ef5fef66502
---
The exact adaptive freeze was ingested, signed and exercised once. The first positive PRIVATE component failed during direct GPG show_keys (form13), before Git/proof/implicit-child execution: grant10 measured A1093125 + C171250 = Q1264375 ns exceeds1ms. Overall canonical verification exit2. No unchanged-input retry, later component, final test, qualification, receipt or landing followed the refusal. runtime_approved remains false.

Oracle disposition and a causally changed authorized Q2 input or scope are needed. The controller/scheduler is outside this Executor's writing scope. Blocker DM01M6VAR8TRB1Z0XX39YW0PKA3Y was stored; storage is not proof of model receipt. This report supersedes [[reports/2026-11-07-dream-proof-tools-fixed-t-66236]] as the current handoff, preserving all historical failures.

## Source and exact frozen authority

Worktree /Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools. Signed HEAD913224f577368c9bbc70c9afce57a91e099957c6 has Good Boxy signature, fingerprint9B07E849480C68328CA20684844ECC0587DE7AB0. It descends from retained59a301e4/ff2e650b/a2828316/d02cdd9f. Worktree clean; no reset, stash, cleanup, push, merge, release, install or history rewrite.

Only scripts/dream-proof-tools.py:52 (explicit complete selected tuple/comments) and tests/fixtures/dream-proof-tools/sources.lock.json:1 (canonical_q2 C Ref/hash/length) changed. Recipe logic after the selection, source guards, tests, native controller, schema, limits and upstream authentication/patch pins are unchanged. Existing exact root compiler-input identity, signed-snapshot byte equivalence and no-follow/final currentness checks remain.

| Frozen input | Exact binding |
|---|---|
| Signed source/preparation commit | 84158341180f011d09dc0e7d561f91b244d381ae |
| Task-named C,417272B | e23f68d3384418160fdc41813844876ec14b28b704bf7f916fad86a48dd992cc |
| Supplied probe,303608B | 05eefb2039d573e6d9fdd5b41e17db29f73615a04b9194c74e9f9f177572ae68 |
| Preparation,520542B | 3974b380a0ed86ce06087f79ac14bff150f4c31fc0717ffbfa09cba24f6ee47b |
| Four-Rust SourceSnapshot byte digest | dcf0e5df5155129735ab918f8f72d02271cd4ddcb120c43d89cac408695cbfbe |

Frozen root F=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/adaptive-maxlive-final-signed-preparation/dream-read-c-preparation/compile-v5v4t16m. Selected source is exactly F/proof-helper-probe.c and equals the actual recorded compiler input identity. The separate signed snapshot C has equal bytes. Probe and receipt are the task-named immutable supplied artifacts, never rebuilt/substituted.

Full offline ingestion checked root0700/source+receipt0400/probe0500, one-link identities, source/preparation snapshots,323 lexical header inputs and aliases,352 retained outputs,17 receipt projection hashes, actual compiler/input/output/linker/SDK/platform/build-closure and exact Mach-O facts. Source signature and required2ce4359d ancestry exit0. The source-domain digest is byte provenance, not compiled product-currentness or runtime authority.

The freeze's explicit qualification limitation is retained: scheduler-fixed-oversize negative is the separately labelled real C API control under the Oracle14:50 ruling; its physical negative remains DEFERRED/UNPROVEN, not a prerequisite. This Executor did not run or relabel that physical negative. The producer's earlier physical positive is preparation evidence and does not substitute for this failed builder witness.

## Actual commands

Evidence E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/adaptive-20261107.

| Command | Actual result |
|---|---|
| python3 -I scripts/dream-proof-tools.py unit | exit0,58 tests; E/unit.log SHAc9309871fb39ace692e8e8c31c9714fdd4901ca1b80bb73d59ecfcd717f7a915 |
| selected_q2_input full offline ingestion | exit0; E/ingestion.json, selection.json |
| git verify-commit84158341180f011d09dc0e7d561f91b244d381ae | exit0, Good Boxy; E/q2-signature.log |
| make fetch-dream-proof-tools | exit0; all7 source signatures freshly authenticated in private public-only home |
| make build-dream-proof-tools | exit2; recipe exit1 after positive controller actual exit81 |
| make test-dream-proof-tools | NOT RUN after stop |
| make test-formal-parity | NOT RUN after stop |
| make test-licenses | NOT RUN after stop |
| git diff --check | exit0 |
| Current HEAD signature, predecessor ancestry and Q2 required ancestry | each exit0; E/*signature.log and E/*required-base.log |

The exact frontmatter sequence `make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses` was evaluated left-to-right by the unattended stage driver and stopped at its first nonzero exit; overall2. E/verify-exit.json retains actual stage exits/timestamps/load. E/verify.log SHA79c8cd10075277abe996aa3c442390d8ead0c010354cdc14edeb9a1ee7425983. Free bytes before build17647771648 (>8GiB). Load7.8940/6.7817/7.8477 →11.7349/8.6582/8.3853. No causal claim that load explains the measured CPU failure.

All60 builder commands000–059 exited0, with exact argv/environment/stdout/stderr/exit records at R/logs. R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6VAGHD4Y6MXHSASDMN15XE9. This includes authenticated extraction, exact guarded source postimages, source-supported configure/generated outcomes, two-job builds and actual compiler/SDK/config/archive/link-map/load evidence.

## Sole positive: full failed effect and settlement evidence

Original build_id01M6VAGHD4Y6MXHSASDMN15XE9, incarnation1534830775365375. Prefix P=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6VAGHD4Y6MXHSASDMN15XE9. Intent SHA20d1cfbcb4bd4a84e77aa8ae4955c207fa62e684faa6e2ca9edd7f3f04fd9773 retains original identity/effect.

C=R/components/positive. Descriptor run_id01M6VANNS2TTE6HCJ3PZPVK0TN, nonce01M6VANNS2HXHK26MGFJ40NXSF, input SHA f3ec43329f6bcc09add584d9d239a2409e85d28cee18a6a785801ad40f4bc69f.

- C/output/ledger.ndjson:25 unaltered lines,33905B, SHA0ee1e452c2cda20a2faf816a831218ec49055bb7d055583cefba2d03e8185585. Frozen C:1413 defines form13 as direct GPG show_keys, executed first at:5051; no Git signature/implicit child is demonstrated.
- Ledger21: grant10 measured A1093125 + C171250 = Q1264375ns,264375ns over1ms. W1194000ns remains below25ms. Closed1/unknown_mask0/ack_mask1, post-ACK/identity/conversion masks1. Baseline31966→endpoint58201 ticks with timebase125/3 agrees with A.
- Ledger25: adaptive run_used/run_cap126042ns, predicted aggregate_A_target338042, controller reserve461958, STOP reserve212000. Actual t0=1535019262817000→STOP call1535019263904000 is1087000ns; STOP call→ACK endpoint1535019264011000 is107000ns. Most observed overshoot precedes STOP; these timestamps alone do not establish why. Fresh_checks3135; next_run63021 is a prediction, not an actual retry or credit.
- Ledger22: failed unit engine_finish_grant at frozen C:3606. Actual/inclusive controller CPU172166ns, supplied actual171250, supplied total cost1264375; included tail916 gives final cost1265291ns. The ordinary-unit error is this COST violation, not a separate >1ms local controller CPU span. Stopped units130/max CPU347000ns.
- Ledger23–24: conservation PASSED. wait4 upper2426000 ≤ independent_helper_credit2425045 + rounding12500 + N_reaped1×T150000 =2587545ns; excess0. Direct actual reaps1, qualified child reaps0. Raw residual0; rounded required/retained residual cost955ns debited once and paid. Fixed T explicitly is not credit.
- C/stderr SHA5767948c2a80dd1b1c9d6f711d4be8907b6e1bb3bec42a33963adb5c444b7684 has resource_limit2 (first canonical member-window closure refused; last ordinary engine unit exceeds one millisecond) PLUS settlement_unknown1 (first=last final unreaped group kill unresolved). All diagnostics are retained without erasing the earlier failed kill effect. Exact kill errno is absent from this evidence.
- C/stdout:2011 unaltered bytes, SHA36c74dbc2ddb9eeadba6586c9dd0fc068deb8dd39559c7d24b1727d5d3f3144b, byte-equal to C/output/result.json. Failed/settled, proof not_reached, attempts1/members1; helper created1/reaped1, group_gone and retired_coverage true. Private conditional/runtime_approved=false/no whole-command theorem/not_applicable_private_test semantics stay intact.
- Controller PID32846 was actually reaped by the retained parent: C/wait4.json SHAad380cc9c90bda1b7e3f5f315fb1871b4a8bc86d37553e280cdc238c135b3148, actual_wait4=true/exit81. Ledger21/24 retain helper34689 actual reap, identity and usage.
- Final receipt: control_verdict=failed, persistence_verdict=persisted, terminal_cost_verdict=passed; debt19586387619ns charged=paid, retained_units0/overflow0. The physically observed later reap/group-gone state and passed persistence/terminal/conservation do not erase the retained settlement_unknown or CPU failure.

Only the positive case exists. R/terminal.json records unsupported/effect_unknown=false. No qualification.json/receipt.candidate.json exists. No later native attempt followed the failure.

## Acceptance coverage and remaining work

| Requirement | Evidence / disposition |
|---|---|
| Exact selected source/probe/preparation; custody/currentness | Signed complete selection, task-named actual compiler input, full offline ingestion above; no substitution or rebuild |
| Seven authenticated sources, exact patches and real build | R/authenticated.json SHA7c477baa12aa1de7e87e8a2af2ba7fc95ed472b18e6ed4edf998212b3804c338; auth_exit0 for7; source-lock SHA5e4646c8f44ba9bec219c67d74b2a7c2a464f28a9c7b4787b3b52ed7f6de8b54; unchanged roots/patches;60 command exits0 |
| Typed bounded Basis/source review/config/commands/closure | P/provenance/pretest-basis.json47112B SHA3a23cb6798c0fa4144f597666c2d152989cf222e6f8b47408d6d656fba3750d8; auth/generated7, review8, forms16; actual source/config/link/archive/load/compiler facts retained |
| Exact conditional compiler/platform/source premise | Recipe SHA807cbdde5960e93d1285bb4cc508ae11f9db886d6a4b33e73f3c50b91640f9f2; clang21 SHA1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9; SDK27 settings SHA7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb; Darwin27.0.0/build26A434/aarch64/cache UUIDc284a291b68b3d0da1a03f866f87efdc |
| Real positive/corrupt proof/remaining causal cases | FAILED at first direct public-key helper grant10. Later cases/Git/proof/implicit child not reached. Git SHAef03bc262ffe5047c488c33a6b925a87847ae717dc6afc39cc8d3519d7e875de; GPG SHA0bb12336d9b42ca2657ab7cfaf21d703f7ad16d802521acc1dc64cb573f82f13 |
| Receipt-last publication/cache reuse and original effects | Mechanism unchanged, correctly not reached after refusal; prefix0700/Basis0600, original incarnation/intent/raw artifacts retained |
| Current-source formal/Rust integration/license gates | NOT RUN after explicit stop. No913224f5 source-bound formal receipt or integration/license success is claimed; older d02cdd9f successes remain historical |
| Historical failed evidence retained | All4 earlier ledger hashes independently rechecked unchanged:413574d03deaf982b431a7b972b90cf9fb0745a133d55d3160a1d2abb110e01a;6b6ec65a0e628578f22be29f0d8dcb6ce57624b3c82a1b151cc13d163b90219c;2132c4e23198d85b7d448f81c15334832b20712b43b4ca02215640b4fda52409;c7a63a73efef4bf8a5cc4bb94013b8ddb75657f232bb3bca49e32f37f32edd5d |
| Runtime/full-product boundaries | No approval literal/Q2 product edit/runtime receipt/lesson/release/merge. Q2 separately owns full exporter/four-job/control/corrupt-proof/runtime gates; physical oversize negative remains explicitly deferred/unproven |

The independent verifier/compiler/SDK/platform fidelity and bounded source reachability review remain conditional trusted premises. Metadata, source inventories and this failed sample do not prove arbitrary native correctness or absence of unseen work.

E/handoff.json is the complete local evidence index:33543B, SHA7621577553dfe62673b31b8796c5e351ae5d7adfa5ab863af7ead55113960aec, owner0600 inside0700. It retains exact selected/build/auth/config/compiler/platform pins, failed grant/scheduler timestamps, conservation/teardown/raw artifact locators, actual exits and historical checks. Report work completes this handoff; acceptance remains blocked on the positive.
