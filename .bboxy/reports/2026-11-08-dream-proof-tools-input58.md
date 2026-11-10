---
type: report
title: INPUT58 sole positive refuses34.686ms native wall; final helper/floor debt split retained
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T04:03:23.292Z
commits:
- f6cdc7c4769e82dd0999cdf9e40caf37e2f1a28e
supersedes: '[[reports/2026-11-08-dream-proof-tools-input54-attempt.md]]'
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M6WN6EGDSB1R4AF0PV5TB6YY
updated: 2026-11-08T04:03:23.292Z
number: 93377961366329
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:4d6217e0a919290e2b0410156358ced8f0a84d34d3497d3be4696ed4b1e9edd7
  frontmatter_digest: sha256:534e344805522e7bc7f0ab0c8f76e74805f89eb27ca097b94898d8878ec2707f
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: z/FbwT858VxmRK+QJnwUTDRexL5lcRqW6egoJHksF++IGcJkYKZNdccrPvyi4zGTAXsnw2yB0ZpRcrzAqZbRDw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150540
  event:
    id: 01M6WN6G4AASVRX99YVJ7YF54J
    sequence: 150540
    hash: 178291fc6435b93161b5492a7ad0e3affc3b6f4e36bdd9b3a4b314ac0b570c4a
---
INPUT58 was consumed exactly and its sole native positive failed at the first process-creation wall gate: **34,686,000 ns observed > 25,000,000 ns**, excess9,686,000 ns. Actual controller exit81; exact task verification exit2. The original helper was killed/reaped/group-gone, and all recorded rest debt was paid. No retry or later component/final gate followed. runtime_approved=false; full-positive fit inside30s remains unproven because this earlier gate refused.

## Change, authority and immutable inputs

Signed commit `f6cdc7c4769e82dd0999cdf9e40caf37e2f1a28e` changes only `scripts/dream-proof-tools.py` ROOT_Q2_INPUT pins/projection bindings/Mach-O UUID and `tests/fixtures/dream-proof-tools/sources.lock.json` canonical_q2. The exact 9081e326 terminal validator is unchanged. No Q2 source/probe rebuild, substitution, expander copy, guard/model/DTO/version/budget change, new dependency, operator credential/global configuration, installation or publication occurred. Source and attribution/license notices remain attached.

The task's ORACLE FREEZE, INPUT58 supersedes INPUT54:
- Source/preparation commit `ba22c67c488421f58faff3e5256f7fcfff6a5ab8`.
- C320573B SHA `386cf55eb9b14ff0e77e453d76d8df0edfed30d06593fbcabd4bc0f1ed307d9f`.
- Supplied executable251016B SHA `0759cc153f74ae23c22eb2e952aa7b0af149dfd7102af595bafaa6765c533bde`.
- Preparation533552B SHA `e73178e5d779cca4d4ec1b0d6e1afd203746e8a2bad032e243d276f371d64ef3`, under Q2's `prepared-tree-pin-source58/dream-read-c-preparation/compile-0l7qmwgc`.
- The other nine named inputs are byte-identical to INPUT54; all10 match the selected signed commit. Codec authority remains the identical b26d7cd3 blobs (vectors54cfab3b… and schema96e2839a…), not a b26d7cd3 ancestry assertion. Terminal/result shapes remain compatible. The frozen expander now preserves the literal tree syntax, as the Oracle explicitly accepted; this run did not reach the real tree-pin form, so it does not validate that form dynamically.

E = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input58-20261108`.
R = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6WMTMXKA8BFZHWGQP7FX028`.
P = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6WMTMXKA8BFZHWGQP7FX028`.

Actual preflight: source signature exit0/approved Boxy fingerprint9B07E849480C68328CA20684844ECC0587DE7AB0; ancestry2ce4359d and landed d1fe3f4b each exit0. E/preflight.json SHA `b70adfada1350cd75a8b3058bd6a4ed35aa0626f67a80d21784997263eba8b62`. All10 git-show byte checks and two codec blob checks exit0, E/source-byte-checks.json SHA `0d6cbcc99ba5fb5909191131357303732f9628acb519a2bc79fd87d366f887dc`. Complete private/compiler/header/SDK/platform/retained-output/Mach-O ingestion exit0, E/ingestion.json SHA `8b3f504eea0cecaee8e01cbd4ccd79d3925825979c093c4a035cf6c02826bb36`. Exact terminal field equality is recorded in E/terminal-binding.json SHA `6310928dd6f06979704f2dd2766fe75a67488ae3f48b59023dc06bcba971bfa8`. These are offline integrity checks, not a runtime grant.

## Actual first-helper refusal and settlement

Original build incarnation1579202078534041; candidate dev/inode16777234/186020530 unchanged. Only R/components/positive exists. Source-built Git SHA `fdae24c3e8345c9c2773cc48d20b76a46628a0bfe42c0ddaa915c95baed6a019`; GPG SHA `6bb144463acd1723e31e59a91016fee2d0bfa7348db68b38cf1c9c9ce8029864`. Neither is qualified/admitted.

R/components/positive/wait4.json records parent-owned actual controller reap PID52183, exit81, userCPU14775000ns/systemCPU40438999ns/ru_maxrss2408448B; SHA `a801048690e1ac2507a2091fde961daae03c1904ed858a2e1f2236e936570526`.

Unaltered stdout and persisted output/result.json are byte-identical2020B, SHA `aad80f30b14fc2a77b26253e21a6f31009c3c7da2ebc1e2031294748bdb61575`: phase=settled, outcome=failed, error=resource_limit/"unchanged native2ms wall25ms application1ms inclusive3ms gate", proof not_reached; attempts1, created/reaped1/1, group_gone=true, retired_coverage=true. Observed nativeCPU563000ns and inclusive689000ns remain below their2ms/3ms caps; nativewall34686000ns exceeds25ms. Ordinary_max400167ns is below1ms. There is no deadline error; all deadline observation fields are0. No threshold was widened.

The frozen C maps this exact native result in engine_birth:2969–2985: prepared_spawn returns and retains the original PID/effect; raw CPU/wall and inclusive/application costs are recorded; the four unchanged thresholds are then checked. The wall comparison is the observed failing member of that conjunction. The code returns before the first engine_sample_member/QoS/roster step. This identifies the gate; it does not establish why the OS call took34.686ms or certify any unseen work.

Ledger output/ledger.ndjson is6 lines/2796B, SHA `99d0a76574a689ec4006bb0e90754cd57cb62739c7ea943343afd29fd369a720`. Its one helper PID52844 records group kill attempted/errno0, actual_own_wait4=1, group_gone=1, quarantined=0, owner_lost=0, terminalCPU218000ns and terminalRSS32768B. Birth/start/QoS/member-sample fields remain unobserved (zeros/mask0), priority[0,0]; these are NOT positive PRI4 or identity-sampling evidence. Live helper-group RSS is explicitly null in the result; it is not an invented zero. There is no implicit GPG-child/roll-up/B-startup/cryptographic-proof evidence.

Stderr2285B SHA `2dab1bf8f23b465da4645537166a88be8ce5e627956d6dfb72a2bb9005ff6e94` retains exactly one resource_limit diagnostic, no clock/deadline/source/settlement error; persistence=persisted, persistence/stdout errno0, terminal_cost=passed and complete result/tail/error diagnostics. All current terminal fields are present. Main result metrics and final terminal cost/debt have their distinct explicit cutoffs; later paid reporting costs are retained rather than hidden behind earlier maxima.

## Requested final debt split

The final terminal says ordinary_raw_complete=true, debt_components_match=true, debt_scope=whole_controller_cumulative, with charged=paid7,380,000,000ns. Use the component cost fields and the single helper's explicit delta, never sum global-watermark receipts as if they were independent helper debts.

| Component | Billed cost before9× rest | Rest debt |
|---|---:|---:|
| Helper, max(1ms, terminalCPU218000ns) | 1,000,000ns | **9,000,000ns** |
| Ordinary reported raw CPU | 52,697,375ns | 474,276,375ns |
| Ordinary minimum-floor surcharge | 766,302,625ns | **6,896,723,625ns** |
| Ordinary subtotal | 819,000,000ns | 7,371,000,000ns |
| Total | 820,000,000ns | **7,380,000,000ns charged=paid** |

booked_ordinary_units819, engine_charge_calls4, closed_unit_count815, path_walk_components776/path_walk_units250 are retained. The helper record explicitly reports helper_cost_ns1000000 and helper_debt_delta_ns9000000, matching the calculation. E/debt-split.json SHA `7a170165380b81306d7449c632215e12803af58c9ed76ea44cdb4bf56220011d` preserves exact fields and checked arithmetic. This is accounting for the observed failed prefix, not a forecast for the missing full command sequence.

## Verification, acceptance gaps and handoff

| Requirement | Actual evidence |
|---|---|
| Private exact frozen tuple, custody and currentness | Successful full ingestion/preflight above; no source/executable rebuild or fallback. Proposal read-set digest177cff0e… remains a byte-snapshot claim only. |
| Existing source/guard/parser controls | `python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target`:62 tests exit0; E/unit.log SHA `08d4b6b2ad4c7724ca4817851f321dce1f4aeb308b3ee2c87f3feb282f72d6dd`. Validator/test behavior is unchanged by this pin-only cut. |
| Authentication before private build | All7 fresh source crypto checks exit0; R/authenticated.json SHA `07732fe371140a638be58fce3c058e8951f65b1e4ab97adb0528a90806bfb128`. All60 numbered build/config/link commands000–059 exit0 with original command/PID/raw log/exit records. Existing exact source/patch/config/recursive platform/static/link evidence retained, not replaced by version or symbol claims. |
| Actual prepared basis and bytes | Basis47112B SHA `f7b055702807ccf0fb1cfffe125f33fbea6f926210e14181cf1021c915dce3ce`; lock7176B SHA `a435c6faee772af623473723058381b8e1a9e616bda31360a2ed0c838e621a93`; descriptor3439B SHA `db5268c91f7352d23435831dbf7886864f22aa5bd3fbc4d344cbdcf439e21dc3`. Exact compiler/SDK27.0/Darwin27.0.0/OS26A434/cachec284a291… and all8 finite source-review findings retained. Compiler/platform/native reachability remain explicitly conditional assumptions, not an arbitrary-native theorem. |
| Sole real positive; original effect/debt/settlement | FAIL at known34.686ms native-wall breach; actual controller and sole helper reaped/group-gone, debt paid, raw bytes preserved as above. No replacement, hidden counter reset or timing retry. |
| Later components/final gates/qualification/receipt | NOT RUN: corrupt-proof/pause/cancel/ABA/clock/debt/reap cases, fresh formal parity/license final gates and qualification/publication. No qualification.json or receipt.candidate.json. No complete30s fit, real tree-pin success, current final source-bound formal evidence, product/e2e or runtime approval claim. |
| Historical custody and signed clean work | All9 previous ledgers preserve exact dev/inode/length/SHA; no reset/stash/delete/peer cleanup. Candidate inode unchanged. f6cdc7c4 Good/Boxy; git diff --check0, git status --short0/empty; E/final-state.json SHA `7f097fe2abf6d7019dbad79f79975881f0612736921ca230b84536c9a1bda623`. No push/merge/landing/install/release. |

Exact task command `make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses` actually exits **2** onf6cdc7c4. Fetch succeeds, build Python returns1 for the preserved positive refusal, make returns2 and short-circuits the later targets. E/verify.log SHA `45a688108ea398f6b2f59f3d67def247c6cf4362d7ac32bcd97b3bf50a06f5c1`; E/verify-exit.json SHA `ce0cda795de5f056f1e82a728b7a715c1f5a0231f1ef886407670298b8044373`.

Full private machine handoff E/handoff.json98161B SHA `495d72b8b76583205a7152ef0895add4d4c293e3c81c8fdbd5978f05d8a65e0e` binds original identity/inode, exact source/compiler/profile/artifacts, authentication and60 real command exits, raw hashes, actual settlement, debt split and outstanding gates. Evidence dirs/files are0700/0600; this is offline provenance, not another runtime metadata allowance.

Required Oracle decision: disposition of this observed native25ms-wall failure and a causally changed authorized input/scope before any further attempt. No unchanged-input retry or threshold waiver occurred. This completes the single authorized INPUT58 attempt as a failed qualification handoff; overall task remains blocked. Earlier failures, including INPUT54's expander refusal, remain unchanged.
