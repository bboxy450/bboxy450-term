---
type: report
title: INPUT60 sole positive observation refusal; derived G43 and final debt/excess retained
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T07:58:26.674Z
commits:
- 68f6d062d83af635dfc87e95beaf834b2c9766ca
supersedes: '[[reports/2026-11-08-dream-proof-tools-input60.md]]'
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M6X2MVJZ8MPNZ4AYQNVX0SAN
updated: 2026-11-08T07:58:26.674Z
number: 93377961366345
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:20eecb6a42f9fcac3fb088cbcb49f0c779955760476e24d0123f64f66d2e3a4c
  frontmatter_digest: sha256:88a96940b98c34fc2f6db733595e347a977f6719682f5644ef02abf11c4e8c6d
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: tB3rCnorxGsQkbcDiuWuut7HqSTnr9fvbnmge93QWv+8FQOCZ5XR1CwfopXBapOZL0l2EUphTXrkIbrsle+ECQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150599
  event:
    id: 01M6X2MWYJAD6N09DVM9YWGXFZ
    sequence: 150599
    hash: 95469ab73420b2acda0faedb89b39f4243d12da84c6e58cb468d7c610680f2f0
---
After the Oracle freed disk space, the previously unconsumed INPUT60 native attempt was run exactly once on unchanged signed commit `68f6d062d83af635dfc87e95beaf834b2c9766ca`. The positive failed with **coverage_unknown: live helper observation unavailable**, actual controller exit81 and full task verification exit2. There was no30s deadline error or process_create wall excess. The original refusal/result bytes remain unchanged; subsequent owned quarantine settlement is recorded separately. No retry, later component case, qualification or receipt followed. runtime_approved=false.

## Resumption, source and custody

The task's INPUT60 tuple remains source1602aff7/Ced71ff1a/probe67060d75/receipt6d1b82a8 exactly as committed and fully described in [[reports/2026-11-08-dream-proof-tools-input60.md]] (#93377961366343). No new source/code/guard/controller/DTO/schema/version/budget change was made for this attempt. All current four terminal fields and decision66331's process_create-only250ms wall ceiling apply; CPU2ms/application1ms/inclusive3ms and other native-class25ms limits remain unchanged.

The Oracle reported deleting seven regenerable Q2 formal/TLC caches; this seat did no cleanup. Fresh actual df returned **14,649,316KiB > 8,388,608KiB**, exit0, so the pre-build gate passed. E/disk-gate.json SHA `88aaf4003224e7644d81f76e9b2a4373c3659607a9d23c8e096a1ea93a96b87d`. The earlier failed disk reading remains preserved in the prior report. Complete current immutable ingestion again exited0 before launch. All10 prior ledgers retain exact dev/inode/length/SHA.

E = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input60-attempt-20261108`.
R = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6X25J5JYW6CS88P06H0QYNF`.
P = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6X25J5JYW6CS88P06H0QYNF`.

Original build incarnation1593191137086916 and candidate dev/inode16777234/186415271 are unchanged. Only R/components/positive exists. Source-built Git SHA `0ea97ef302e208f67a4a71cea1b9cff08c1e4be2280e507917ec93ba42b37459`; GPG SHA `d63c47f2aff14de0305e84b5c7d0626e6be45920bee5ef4e60db83d07bead023`. They remain unapproved/incomplete artifacts.

## Observed refusal, raw snapshot and later physical settlement

R/components/positive/wait4.json proves the parent actually reaped controllerPID77044, exit81, userCPU69362000ns/systemCPU242175000ns/ru_maxrss4276224B; SHA `6981403ed91193d59084186c0c78b87a83e45de584ba5ff7cc9df78653b2836d`.

Unaltered stdout and output/result.json are byte-identical2016B, SHA `acdcc846610eff82bcdb1cfa6011d5ddbcf845e3ab6d554d887f46c09f9058f2`. Their original publication snapshot says phase/outcome=quarantined, error=coverage_unknown/"live helper observation unavailable", proof not_reached, attempts8, members_max1, created8/reaped7, group_gone=false, retired_coverage=false. A=73456000ns and paid debt15268369500ns are that earlier snapshot. These values were not rewritten after the held effect resolved.

Stderr2702B SHA `0a12bc85160cac259973d64bfc5d17f9cf889e82c2a603a388ab184c78de8f28` records the actual failed observation:
`observe_failed op=taskall pid=78272 result=0 errno=3`.
It then retains coverage_unknown count1 and settlement_unknown count2, with first detail "live final group kill lacks original current group identity" and last detail "final kill raw error unresolved". No clock/deadline/resource/source error is recorded. The raw text alone is not proof of a delivered kill: sequence8's retained records explicitly say group_kill_attempted=0, group_kill_errno=0.

The final independent terminal receipt says stage=settled, helper_created/reaped8/8, result_complete=true, control_verdict=failed, persistence=persisted, persistence/stdout errno0, terminal_cost=passed, complete paid tail/diagnostics. Final error histogram still retains the3 failures. This later physical settlement does not make the earlier failed control/proof valid.

output/ledger.ndjson has46 lines/27127B, SHA `931699ad1dd65cbd18e7cc1067ae0a833cd4f2bc4858904e6cc7dcea1714a4c2`. It preserves:
- Seven completed helper records.
- Sequence8/PID78272's original quarantined record (actual_own_wait4=0/group_gone=0).
- The SAME original sequence8/PID78272's later settled_refusal record (actual_own_wait4=1/group_gone=1, terminalCPU10235000ns, debt paid). Its quarantined flag remains1 as historical untrusted state; it was not changed into a success.
All8 final original-sequence records show own wait4/group-gone and observed leader PRI4. The ninth helper_total_budget row is sequence8's earlier snapshot, not a ninth helper or another charge. This handoff deduplicates by original sequence for final totals and retains all history. Final known helper CPU is83691000ns:17108000,21044000,8340000,6422000,6223000,5488000,8831000,10235000. No child/verifier/roll-up/B-startup or actual signature proof was reached.

Known process-create maxima are CPU897042ns, wall896000ns and inclusive1029500ns; ordinary_max761708ns in the original result. Final ordinary observation has its later cutoff599792ns. These are below their applicable gates. **process_create_wall_excess_ns=0, count=0, known=1**, scope=invocation_sum_once_per_prepared_spawn. No >25ms creation happened in this run; the new250ms ceiling was not itself exercised by an observed excess. Previous INPUT58's34.686ms refusal remains FAILED81 under its original rule.

## Requested G and final debt split

Per the Oracle's clarification in message01M6X1XFF9NBR86HMC627SNNSX, report G as **source/basis-derived, not measured**:

**G=43 = 26 candidate-root guards +17 fixture-root guards.**
Derive unique literal(root,path) pairs, requiring repeated paths to agree on length/hash, from descriptor source-lock/shared basis/git/gpg; config refs; closure image/static/archive-index/link-map refs; and the complete fixture inventory. Tool/image and key/inventory references deduplicate. Absolute compiler/SDK/source-review references do not become guards through the unused absolute-guard path. The frozen source's insertion/dedup and preparation flow are at C:1755–1769,1942–1949,2027,3742–3779. The observed helper births establish that inner preparation reached the engine; the internal counter is still not directly serialized. G's cap256 is not substituted for its value.

E/guard-derivation.json7667B SHA `262e733ba1c67e53b7864e17ad6ba2e7b3c8134c0a7da67a1d87a01a860a57e5` retains the complete43-entry root/path/length/hash set plus exact basis/descriptor/source bindings. Basis47112B SHA `be187a6691637ef9e6c6da5a3504ce778655a126ef477b946e2cfc89b2482681`; descriptor3432B SHA `72440acdd6276af2de185978721964ea716f8ef15c53e875d00fbf9b0e03106d`.

Use the FINAL cumulative terminal fields and latest original helper records, not the earlier raw result or sums of global watermarks. ordinary_raw_complete=true, debt_components_match=true; final charged=paid15,513,484,500ns.

| Component | Billed cost before9× rest | Rest debt |
|---|---:|---:|
| Helpers,8 final original sequences | 83,691,000ns | **753,219,000ns** |
| Ordinary reported raw CPU | 225,792,417ns | **2,032,131,753ns** |
| Ordinary minimum-floor surcharge | 1,414,237,083ns | **12,728,133,747ns** |
| Ordinary subtotal | 1,640,029,500ns | 14,760,265,500ns |
| Total | 1,723,720,500ns | **15,513,484,500ns charged=paid** |

booked_ordinary_units1640, closed_unit_count1548, engine_charge_calls91/poll_closure_calls1, path_walk_components2752/path_walk_units901 are retained. E/debt-split.json SHA `4fddfae73a68e80121194a20a021ca7b2fafc5ed4d64784be46340b794ee3390` records all exact component fields, per-helper deltas, raw snapshot A73456000ns versus final helper total83691000ns, and checked arithmetic. The differences are explicit observation cutoffs, not clamped/erased costs.

No deadline was observed (all deadline diagnostic fields0); this failed prefix therefore does **not** settle whether the complete16-form positive fits30s. No extrapolated floor estimate is substituted for that missing positive.

## Verification, limits and required disposition

| Acceptance line | Actual evidence/result |
|---|---|
| Exact source/probe/receipt and corrected ceiling/fields | Unchanged signed68f6d062 and exact INPUT60 selection reingested0. Existing four-field/typed/literal/unknown/missing/duplicate and 34/250/251ms/CPU/inclusive controls are preserved; preflight66 unit tests exited0 on these exact source bytes, recipe SHA `cd4095417b06309f7849f73955d3ccde279edb4270d41a0e28a9cd217790d76b`. No unnecessary unit rerun after no source change. |
| Source authentication/private build | Fresh seven-source crypto checks0; R/authenticated.json SHA `ed85b0b87e4b6176533cc4357a945c810822b384debd3aef33955fc675147df9`. All60 numbered build/config/link commands000–059 exit0 with original command/PID/log/exit evidence. Guarded upstream sources/config/static/link/compiler/SDK/platform closure retained. |
| Actual bounded prepared basis/custody | Basis and descriptor above; source-lock7176B SHA `476d2f01b75e5d4ae017eb062adba00eb2940aebabe3f7eb080be5328786ff2a`. All8 finite review findings and exact Darwin27.0.0/OS26A434/SDK27.0/cachec284a291… assumptions remain conditional; no arbitrary-native or universal-census theorem. |
| Sole actual positive/refusal/effect ownership | FAIL81 coverage_unknown plus retained settlement diagnostics; raw quarantine snapshot followed by independently retained same-effect physical settlement and actual parent reap. No replacement controller/helper, cleanup, threshold waiver or timing retry. |
| Requested G/debt/excess | DerivedG43 explicitly labelled; final helper/raw/floor debt split and excess0/count0/known1 above. Prefix only; no full30s qualification claim. |
| Components/final gates/publication | NOT RUN after positive refusal: corrupt-proof/pause/cancel/ABA/clock/debt/reap cases, fresh formal parity/license final gates, qualification/receipt/landing. P/qualification.json and P/receipt.candidate.json absent. No current final source-bound formal evidence, product/e2e availability or runtime approval claimed. |
| History and worktree | All10 prior ledgers preserve dev/inode/length/SHA; original new candidate dev/inode16777234/186415271 unchanged. HEAD68f6d062 Good/Boxy fingerprint9B07E849480C68328CA20684844ECC0587DE7AB0; git diff --check0, git status --short0/empty. E/final-state.json SHA `3502bd7cc5351a367af93d1f3ffd33017da768b4caedae7d297009bbd0eecce4`. No push/merge/install/release/publication or peer cleanup by this seat. |

Exact full task command `make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses` actually exits **2** on68f6d062. Fetch succeeds; build Python returns1 for the preserved positive verdict, make returns2 and short-circuits remaining targets. E/verify.log SHA `66e931e6384980c8059f1638f4a9708587bc3c6f6f507a59a14f9a9615197a8a`; E/verify-exit.json SHA `0807664a84af4180b7e0748baa839c899784d3a4df2a7e87fa5cf50c73064d10`.

E/handoff.json130383B SHA `31f054264655585a0c2e6f9eac3e38d2bfb540f3198f753cc741bfb491b61f88` binds original identities, exact source/auth/build/config/conditional closure, all60 real command exits, original result/terminal cutoffs, all helper history plus deduplicated final totals, G derivation, debt/excess and remaining gates. It is private offline provenance, not an additional runtime prepared-input allowance. Evidence dirs/files are0700/0600.

Required Oracle decision: disposition of the ESRCH/taskall live-observation refusal and the later same-effect physical settlement, then a causally changed authorized source/executable/receipt/scope before another attempt. No claim is made that a transient-exit race is harmless or that these errors may be cleared automatically. This completes the authorized single INPUT60 attempt as a failed overall qualification handoff. The earlier disk blocker is resolved; every historical failure remains unchanged.
