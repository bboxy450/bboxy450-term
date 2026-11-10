---
type: report
title: 'Exact 66236 Q2 freeze: private positive fails ordinary unit and conservation'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6TPMTXM8MQCQXMCVBW60YGV
created: 2026-11-07T09:59:42.378Z
commits:
- a28283160d146f4c215181e466daae00e95bdb4c
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
supersedes: '[[reports/2026-11-07-dream-proof-tools-qos-retry]]'
id: 01M6TQ65HF3ADBED73W9XV35NA
updated: 2026-11-07T09:59:42.378Z
number: 93377961366253
created_by: seat:01M6TPMTXM8MQCQXMCVBW60YGV
updated_by: seat:01M6TPMTXM8MQCQXMCVBW60YGV
authorship:
  signer: seat:01M6TPMTXM8MQCQXMCVBW60YGV
  kind: seat
  public_key: bBPoi7SjWcPpxvEzz3+5Ck7p1MHLoPov5NPy/GmicZM=
  process_generation: '1'
provenance:
  digest: sha256:736b92a7667fe5369e5546ae6a53093467a14bc26d235efe6c2cd8907412c0e4
  frontmatter_digest: sha256:800d4813ef2724f7dc30ca93e8711b675914488eea15d4fd9b101c2fbaf79b2f
  signer: seat:01M6TPMTXM8MQCQXMCVBW60YGV
  sig: zJE9KNKCg3zqBFdyxjiw3dz2/AIin6BIgyjFgwSM8Mxrxo8wMgo6HHMlClo3XiHvtyuz56QHMltgiEKn9XTdAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150273
  event:
    id: 01M6TQ674SBZM0RAQ64JJ2ES93
    sequence: 150273
    hash: f1562b75ac697a5acaa3008f73513da0e1a21e2424693157009622e14d2575cd
---
The exact 66236 freeze was ingested and committed, but the positive PRIVATE component failed before grant 1. The current canonical verification sequence exits 2. This is a BLOCKED handoff, not successful qualification, runtime admission or landing. The Oracle's stop rule was obeyed: no timing retry, later component case, qualification, receipt or final test run followed the refusal.

The decision needed is disposition of this ordinary-unit/conservation refusal and a causally changed, explicitly frozen Q2 input or task scope. Q2 controller changes are outside this Executor's writing scope. Blocker DM 01M6TQ1AQ21ZKE7DJXBG6NZSM0 was stored and native-queued; model consumption was not proven by that receipt.

This report supersedes [[reports/2026-11-07-dream-proof-tools-qos-retry]] as the current handoff only. That report and [[reports/2026-11-06-dream-proof-tools-landed]] retain all historical failed runs and their distinct accounting rules.

## Source and exact frozen authority

Own worktree /Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools. Signed source a28283160d146f4c215181e466daae00e95bdb4c has Good Boxy signature, fingerprint 9B07E849480C68328CA20684844ECC0587DE7AB0. It descends from retained predecessor d02cdd9fe3406d213699ec82473ef6d9309528b2. The worktree is clean; no reset, stash, cleanup, push, merge, release or installation occurred.

Only two tracked files changed:
- scripts/dream-proof-tools.py:52 — replaces the full explicit ROOT_Q2_INPUT selection with the Oracle's new root/file identities, 17 receipt projection digests, signed source/preparation commits, exact compiler closure and Mach-O UUID. No decoder, accounting logic, threshold or controller changed.
- tests/fixtures/dream-proof-tools/sources.lock.json:1 — replaces only canonical_q2 source Ref/hash with the new 378117-byte C source. All seven source, release-key, signature and patch pins remain unchanged.

| Frozen input | Exact binding |
|---|---|
| Signed source/preparation commit | 39580f0ab216c1bb9a74e7bbb42477b8dd60d993 |
| C, 378117 bytes | 9b8fdd06d9940c52ed6fd9c164d620106ecfb8b70b1d9fae4a787cba40ad406a |
| Supplied probe, 286728 bytes | b57e79505b2da931fa877aa20954a5c129854cee7f272bc5bd1c5e5f4f947c61 |
| Preparation, 514982 bytes | 53cdeac02bc1d74868d57feafb642975bbae1b3fb2b6313f02e32e9e5472d0fc |
| Proposal read-set byte snapshot | a212cd65a796b3327246a5a037be8039beabeb354575a2455be309120102cc86 |

All supplied private files remain under /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/measured-66236-signed-preparation/dream-read-c-preparation/compile-rcznyik8. Their 0700 root, 0400 source/receipt and 0500 probe, one-link identities and hashes were checked. The adapter at scripts/dream-proof-tools.py:2355 checked the complete source/preparation snapshots, all 323 lexical header identities/aliases, retained outputs, actual compile/link inputs, compiler/SDK/OS identities, 17 projection bindings and exact Mach-O facts. No probe rebuild or substitution occurred. Both source signatures and required 2ce4359d ancestry exited 0. The source snapshot digest is byte provenance, not a compiled product-currentness or runtime-approval claim.

## Actual commands and evidence

Evidence root E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/measured-66236-20261107.

| Command | Actual result |
|---|---|
| python3 -I scripts/dream-proof-tools.py unit | exit 0, 56 tests; unit.log SHA830962eaaf060239289079f86ab15e063f55e816b3dd8a8ea4345f8a98fbfd69 |
| Full selected_q2_input offline ingestion | exit 0; E/ingestion.json and E/selection.json |
| git verify-commit 39580f0ab216c1bb9a74e7bbb42477b8dd60d993 | exit 0, Good Boxy; E/q2-signature.log |
| make fetch-dream-proof-tools | exit 0; all seven sources cryptographically authenticated with fresh public-only home |
| make build-dream-proof-tools | exit 2; recipe exit 1 after positive controller's actual exit 81 |
| make test-dream-proof-tools | NOT RUN after stop |
| make test-formal-parity | NOT RUN after stop |
| make test-licenses | NOT RUN after stop |
| git diff --check | exit 0 |
| git verify-commit a28283160d146f4c215181e466daae00e95bdb4c; required predecessor and Q2 ancestry | each exit 0; E/*signature.log, E/*required-base.log |

The task's exact frontmatter sequence, `make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`, was evaluated in order by an unattended stage driver, stopping at its first nonzero exit. Overall exit 2. E/verify-exit.json retains each stage's actual exit/timestamps and before/after load (7.8247/6.6675/6.0791 → 6.7720/6.9854/6.3477). E/verify.log SHA b8fcc8b4f5fe15eb70e77476471511e844e43150759ca1ec39a4a7e2bb344f73.

All 60 builder commands 000–059 exited 0; exact argv/environment/stdout/stderr/status are retained at R/logs, where R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6TPSX095ZEEF53Q73KCW8N6. This includes authenticated extraction, exact guarded postimages, supported configure/generated outcomes, two-job builds and actual link/archive/config/compiler records. Build success did not override the component failure.

## Failed original effect and full ledger

Original build_id 01M6TPSX095ZEEF53Q73KCW8N6, incarnation1514166109750291, prefix P=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6TPSX095ZEEF53Q73KCW8N6. Its intent remains unchanged (SHA f6359be9883640cbc21284066e3853d21301d6d4b61821926442b0c2927b74d5).

Positive descriptor run_id01M6TPYYH37DQGZZRZGD3D8GYE, nonce01M6TPYYH3JANJWXMNCE178KY4, input SHA f308c53034b5441d3586559b2e3f323c0853e5d59abd7c7cce089c6ff18adb82. Case directory C=R/components/positive.

- C/output/ledger.ndjson: 4 unaltered lines, 5234 bytes, SHA2132c4e23198d85b7d448f81c15334832b20712b43b4ca02215640b4fda52409.
- Ledger4 first/last failed ordinary unit: engine_stopped_corroboration at frozen C:3155, open_cpu130690333/end_cpu131855166; actual/inclusive1164833ns exceeds1000000ns. This is before grant1, so no measured grant Q exists; result Q_max_ns/A_ns/C_ns/W_ns are null, not invented zero.
- Ledger4 terminal conservation: startup114042ns + rounding2667ns = accounted upper116709ns; actual wait4 upper202000ns; excess85291ns, conservation_passed=0.
- C/stderr SHAe59ea758c526bba0311eb8853320a761c00589835b2f03dc8dfbb9bd9ea648b4 has exactly resource_limit count2: first ordinary engine unit exceeds one millisecond; last actual terminal CPU exceeds independently charged helper conservation bound. No missing middle error claim is made.
- Result is failed/settled, observed_proof=not_reached, attempts1, members_max1. Native CPU784916ns and wall784000ns stayed below their ceilings. The measured-quantum member/implicit-child paths were not reached.
- Controller PID71236 was actually reaped by the retained parent wait4: C/wait4.json SHA718e741023b3185b6c0bfc3348573420222a00422c7953498029836fb01071fb, actual_wait4=true, exit81. Helper PID72741 actual reap is in ledger3: created1/reaped1/group_gone=true/retired_coverage=true.
- C/stdout is 2003 unaltered bytes, SHAeb53d08ee0f70f1e72a48bc7ae7c8d8dd3f468e62d54c22233ebdeb069b0cc97, byte-equal to C/output/result.json. Its raw semantics remain runtime_approved=false, conditional_on_bound_source_review, whole_command_lifetime_proof=false, not_applicable_private_test.
- Independent final terminal receipt says control_verdict=failed, persistence_verdict=persisted and terminal_cost_verdict=passed; helpers1/1, final ordinary462000ns, debt18111543372ns charged=paid, retained_units=0/overflow0. Passing persistence/terminal cost does not rescue the failed engine control.

Only the positive case exists. R/terminal.json honestly records unsupported/effect_unknown=false. No later case, qualification.json or receipt.candidate.json exists. No timing retry occurred.

## Acceptance coverage and limitations

| Requirement | Evidence / disposition |
|---|---|
| Exact frozen source/probe/receipt; no rebuild or self-selection | New signed selection plus complete offline ingestion above; source-lock SHA5ce6dcaf5eba2443ce8944776ff3efd0a11d567e56ddfddc8f8b454f570f4a2f |
| Seven authenticated sources, guarded build, real compiler/config/link closure | scripts/dream-proof-tools.py:340/514/1799; R/authenticated.json SHA8bb6988c04506d061d121ef72a735be3d992d8f56f18fc96eada5bd0871950c2; seven auth_exit0; R/logs all60 exit0 |
| Complete bounded typed Basis and source-review bindings | scripts/dream-proof-tools.py:2073; P/provenance/pretest-basis.json 47112B SHA33b6632eb48c423353debadefbcb112ed4d9031e51eff95a206183c0e97fbaf7; generated/auth7, review8, forms16, exact C codec unchanged |
| Compiler/platform/source premise is conditional | Recipe SHA551312c3515bc5ebbca698dc7ffeacde31f703fccd37a1401f7bc5dbd579de1d; clang21 SHA1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9; SDK27 settings SHA7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb; Darwin27.0.0/build26A434/aarch64, cache UUIDc284a291b68b3d0da1a03f866f87efdc; actual tool/link records in Basis and handoff |
| Real Git/GPG proof plus corrupt-proof and causal cases | FAILED at first positive before proof/grant1. Remaining cases not run. Git SHA47aaae78292a5921557323b2d2576066e7d81a9fa921c08b94a7a69ea9731a02; GPG SHA34977040e60cbceb46263dcaa24a9e335fd25c44e1aab18c657874cd49c2c57b. Two candidate images, linked static/license/config/source data retained, no runtime qualification |
| Receipt-last immutable publication/reuse | Existing scripts/dream-proof-tools.py:2847 mechanism unchanged; correctly not invoked after failure. Candidate prefix0700/Basis0600 and original identity retained |
| Current-source formal, Rust integration and license gates | DEFERRED by explicit stop. Previous d02cdd9f354-model/87-contract parity and license/Rust successes in prior reports are historical, not current a2828316 proof. No new source-bound formal receipt is claimed |
| Preserve original failed effects | Both earlier ledger hashes independently rechecked unchanged: original413574d03deaf982b431a7b972b90cf9fb0745a133d55d3160a1d2abb110e01a; QoS retry6b6ec65a0e628578f22be29f0d8dcb6ce57624b3c82a1b151cc13d163b90219c |
| No runtime approval or full-product claim | No approval literal/Q2 product edit, release/merge or lesson publication. Q2 owns its separate full exporter/four-job/corrupt-proof/runtime gates |

The independent verifier/compiler/SDK/platform/native-code/reachability assumptions remain explicit. Metadata, source review, symbol inventory and this failed sample do not prove whole-command absence of unseen work or arbitrary native correctness. No threshold, debt law, memory/attempt bound or schema changed.

E/handoff.json is the complete local machine-readable evidence index: 28524 bytes, SHA1966429dba05ab6220ecdfccb953cc76514fa8310a40f7b990ce8ea5b7431c1c, owner0600 inside0700. It retains exact source/build/compiler/config/platform/auth pins, all failure accounting, raw artifact locators, command exits and historical ledger checks. Original evidence remains in place for Oracle/Q2 inspection. Report work completes this handoff; acceptance remains blocked on the failed positive.
