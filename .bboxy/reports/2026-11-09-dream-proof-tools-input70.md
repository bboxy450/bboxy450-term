---
type: report
title: 'INPUT70 Phase B: shim bound; positive fails in frozen cleanup observation'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T18:36:40.448Z
commits:
- 99bda9ca5592ae8e4ce2d4098c80a506a6c5364c
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M70SJ7BD6MCG5YFXFGQMYP4A
updated: 2026-11-09T18:36:40.448Z
number: 93377961366480
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:8a899a7355bae0a5436e89c6544f53b1c6fae10a4200a7164c3b5aba4baeef8b
  frontmatter_digest: sha256:20f83f7d20d088d6c324a6b7f6b221bf702535a77763d720fd6603bdeb6e9e65
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: qAvO667u9V+NiBOGKG6LiDbBGZnhpDb3Ehs2zhzqpdo4UZhnBVmoTIzdTCtX6t7vnVLJ9oC0cvYRkhjvhc1oAg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151094
  event:
    id: 01M70SJ870BC6H457HPMGCY3XY
    sequence: 151094
    hash: 7d36202cbe83f0df7cd52d71fd79e7e42203acbe99962219c299d93e52ce3d35
---
INPUT70 Phase B is implemented in signed commit `99bda9ca5592ae8e4ce2d4098c80a506a6c5364c`. The sole native invocation FAILED on the positive, with actual controller wait4 exit81 and canonical Make exit2. All16 helpers settled; there is no live quarantine. The remaining seven cases and final gates were not run. No retry, signals, qualification, candidate receipt, push, merge or runtime approval occurred.

Authority: [[tasks/2026-11-05-dream-proof-tools-provisioner]], INPUT70 freeze and subsequent clock auxiliary ruling, task etag `2a82a249465dda2af0ababda3287a15d5b867c851d0c08951f0a974e54e90176`; [[decisions/d93377961366459-dream-external-helper-shim-enforcement]] including its correction and Amendment2. This report follows the two retained Phase A witnesses in [[reports/2026-11-09-dream-proof-tools-shim-phase-a-revised]]; neither witness was rebuilt or changed here.

Evidence roots:
- E = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input70-20261109`.
- R = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M70S5HNZYVKVYAE6SWSPJ1NE`.
- P = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M70S5HNZYVKVYAE6SWSPJ1NE`.
- C = frozen `.../bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/prepared-input70/dream-read-c-preparation/compile-u3u9hnyr/proof-helper-probe.c`.

## Changes and binding evidence

`scripts/dream-proof-tools.py:769,863` installs the supplied shim at P/libexec/dream-exec-shim, mode0500, and builds Git with the explicit DREAM_PROOF_GPG_TARGET pointing there. Actual GIT-CFLAGS and the linked Git retain that exact absolute path; build command039 exited0. The original Git patch is unchanged, SHA256 `a48b33776ae5a88308e096ea62640487706e360c374ec7eaf64d50adf47818b8`. P/bin/git is3766096B/0700, SHA256 `01c737bd489e1a53a8fcbcd9058a8cc593e94c852b1a64d4bf983aec69ae2baf`. GIT-CFLAGS SHA256 `dc8d20f490ef0c595d262b432f95e482eee1f6666e75bdecb44abf29a4023583`; exact command039 metadata SHA256 `ba175e1caa68cbc2fa79855a95c8243ea34660cec0cfab6f88e3f4ef1a0d02a7`. E/phase-b-build-bindings.json retains identities and literal checks.

The frozen signed producer is `76045587db4a96612e2406b9bb08d4191c626253`; actual verify-commit was Good Boxy, origin/main containment and INPUT69 ancestry checks exited0. Tuple SHA256 `3d42b6ecbdd2a3b5da9ac27a31a6cc7ed1c971300937616d567a8dd7dda0b7d4`; preparation receipt SHA256 `c24749c7481562f853f776dc0004d8197bd25efc3b6508890dfdcf94f228a56b`; selected probe303256B SHA256 `aa6e1b28bdc60cf2b4154df616768dfaad743fb51c3028e74a6067a94c8db00b`. All12 inputs matched signed git bytes and retained snapshots. E/frozen-input-checks.json preserves actual commands. Schema/vector authority uses unchanged blob identities, without an invented ancestry claim.

`scripts/dream-proof-tools.py:2443,2492` validates the closed shim preparation projection, selected only by the independently frozen bindings.shim hash: source/header/executable custody, exact signed bytes, allowlist spans, compiler and three controller defines, both compiler/dependency commands and retained logs, union header closure, link inputs and retained inventory. The only newly permitted private lexical header is the exact prepared dream-exec-shim.h. Historical no-shim inert fixtures remain separate test inputs. Frozen ingestion with execution seams forbidden exited0; E/offline-ingestion.json SHA256 `1ff814b79be6f347ce13590dcf0fc889e19be2bc19a68b2457c12e16b4659733`.

`scripts/dream-proof-tools.py:2110–2134` binds exactly the four top-level Basis shim hashes:
- exe `4e95b2a83a34a6328d3c794133c3569dff7e5f1b2ee2c9bfc1d58cd08447527d` (35520B,0500);
- source `34a9a6ee959843fc3192d64fac05b5439175ffb8dd15cc927b65fd4a81b102cc`;
- allowlist `5c9b450c3b1f9558cee49f154d8c771e525ccb8b9303cface9414f957a6cefc9`;
- compiler `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`.

Header SHA256 `d86a11290509faffecd5e3fa82292d55ad8ac977c75107d93bb171567066e512` is bound through signed preparation inputs. The closed source-lock grammar remains canonical_q2 source Ref/hash; its connection to the shim is the frozen probe/preparation compiler defines plus the four Basis hashes, not an invented source-lock field or third closure root. Compiler equality, fixed-path hash and0500 custody are rechecked before descriptors and cached validation. Only this fixed shim path gets the0500 inventory/reference exception. Two git/gpg closure roots and five review projection hashes remain.

Actual Basis is47490B, non-shim47147B, shim delta343B, SHA256 `dfd40a99c3c35279b3dbf67c4c35eda8151e919fadcd52642e740a779c75b1ee`. It fits the original48KiB; 56KiB headroom is unused. `prepared_budget:2915` permits extra bytes only for the four valid shim hashes, retains non-shim48KiB and every other/128KiB limit, and reports exact size above57344B. Descriptor3439B, lock7176B, raw result2024B; before-launch descriptor+Basis+lock+reserved65536B result =123641B. Final qualification-envelope fit is unproven because qualification was never reached.

`review.lock.json` re-reviews Git's guarded child/define and preserves eight source findings; README documents INPUT70. Explicit bound assumptions record that NOTE_FORK events can coalesce and a setsid orphan can evade later child/group censuses. Supported-tool source/compiler/platform review remains conditional; silence proves no absence. Leader own-wait4 supplies CPU verdicts; sampled members_max is now a typed u8 diagnostic, not an exact2 custody gate. No public terminal/wire expansion was introduced. The four-hash artifact maps into existing complete-tuple/custody model predicates; source comments and verification/coverage.toml describe the concrete checks and implementation-only/native assumptions. No ownership/publication transition or new lifecycle model was invented.

`clock_failure_pre_cont_ledger:3188` and `component_terminal:3246` implement the recorded exception only for one exact injected clock error plus exactly one missing-exec-receipt coverage diagnostic (and the previously authorized exact resource diagnostic). It requires descriptor-bound ledger evidence for the settled first owner, activation0, no resume/child/exec receipt and actual own-wait4/group/debt settlement. Both initial and foreground CONT assign activation first in frozen source. Unknown error combinations, extra coverage count, changed detail, foreign/missing/duplicate evidence, CONT and exec receipts refuse. The full native clock case was not reached, so this has pure checker evidence only.

## Actual verification and retained failures

- Final pure command `python3 -B -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target`:78 tests, exit0. E/unit-v3.log13429B SHA256 `66a3ac67f0002a550776c808ad6597b30982e33ebd41844823ae81a61e2b8cb2`. Includes new shim grammar, resealed-inner-binding, dependency, custody, budget, diagnostic member count and causal clock controls.
- Initial full unit exit1 and offline lock equality assertion exposed a trailing LF introduced while repinning canonical fixture JSON. Restored both fixtures' required no-LF encoding before native work; retained E/unit.log. Unit-v2 and final unit-v3 exited0. Earlier focused inert tests also caught/fixed a symlink exception assertion and the descriptor clock fixture's missing shim data. These were offline test corrections, not native retries.
- Seven fresh source authentications exited0 before extraction/build. R/authenticated.json lists git, gnupg, libgpg-error, libgcrypt, libassuan, npth, libksba; all60 recorded build commands000–059 exited0. Bootstrap verifier and faithful compiler/SDK/platform remain explicit trust premises.
- Exact canonical command `make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`:actual exit2. E/verify.log9681B SHA256 `0cb6ba3029a40375bb2fd695f4d1ffac16a300c331638c383ed0e09346383203`; E/verify-exit.json records the original shell's exit. Disk gate at invocation was15006154752B >8589934592B.
- Source `git verify-commit --raw HEAD`, `git diff --check` and `git status --short`:exit0, Good Boxy, empty worktree. E/final-state.json retains results. No final formal/parity/license/check/qualification gates ran after the positive refusal; no claim of a green canonical stack or refreshed formal receipt.

## Native outcomes

| Case | Actual outcome | first_failing_term | Final debt / cutoff margin |
|---|---|---|---|
| positive | FAILED, actual wait4 exit81, settled16/16, group_gone/retired_coverage true | Unavailable: no final_case_conjunction row emitted |14.684130000s /12.819209000s |
| corrupt_proof | Not run after first refusal | Unavailable | Unavailable |
| pause_resume | Not run | Unavailable | Unavailable |
| cancel | Not run | Unavailable | Unavailable |
| stale_incarnation | Not run | Unavailable | Unavailable |
| clock_failure | Not run | Unavailable | Unavailable |
| debt_overlap | Not run | Unavailable | Unavailable |
| reap_delay | Not run | Unavailable | Unavailable |

Positive controller54442: actual wait4 raw20736/exit81, user133188000ns/system385345999ns. R/components/positive/wait4.json SHA256 `797e680a6127d28cb517cd125b2ff841807a72af97a865c0c432b17e698e6fea`. Final ps for54442 returned1 with no row; no teardown/signals are needed or performed. Stdout2024B SHA256 `cbc8759960fc7fcc1d1d79359e0127b9008bc63463931302e5373ac094b24f4c` equals persisted result.json exactly. Result is outcome=failed, phase=settled, proof=not_reached, coverage_unknown `owned observation unavailable: escaped_cleanup_bsd`; checked.json is absent.

Final cumulative debt split, each charged once from terminal totals (9×cost):
- helper2183130000ns;
- ordinary raw2454971625ns;
- ordinary floors10046028375ns;
- sum14684130000ns = charged = paid.
ordinary_max_ns850792; terminal-cost verdict passed; no deadline observed. Admitted interval29745374000ns; elapsed to reporting cutoff16926165000ns; remaining12819209000ns. This margin is deadline minus reporting_started, not a measurement of the final receipt's future write. Process-create excess known1/count0/ns0. Sampled members_max0 is diagnostic. Leader-wait4 job CPU242570000ns.

The unaltered76423B ledger SHA256 `8e27eff280930094a7488224a6ff5336b66de25a1a5e4795393df7fc6e6e5a35` contains16 exec_shim receipts, all in_process_checks=true/self_priority4/fork_notes_are_diagnostic=true. No shim refusal is reported. Signature leader55596/form12 and verifier55601/form15 have successful shim receipts. GS1/GS2 retain inode189796453/hash `d716c2d0807ca1339c774486d55d31768fa9f953df90872f2ceef4cb4568c792`, nlink1→0, pidversion0. Final verifier event row122 has fork_events1/exec_events1/child_discovered_via=none, shim_enforced=true/live_read_failures0; fork counts are diagnostics. Child row120 has image_admitted1, exit_witness=none/rollup_mask0; these are not restored gates. Pause-case fields remain unavailable because pause_resume did not run; zero stop fields in the positive are not a pause witness.

## Frozen producer failure and disposition needed

C:4663–4668 still runs engine_cleanup_escaped before original reap, including shim_active, and calls engine_observe_bsd. C:2849–2852 routes an unavailable/short read into the old classifier; C:3430–3450 can make it fatal without the classifier's original terminal evidence. Failed-read tolerance in engine_shim_sample_member (C:3507–3509) does not cover that cleanup path. This is the concrete remaining external-observation refusal against decision66459's stated diagnostic premise; no host waiver or producer edit was made.

Ledger lines3,11,...,116 contain15 recovered observation_terminal takeovers at escaped_cleanup_bsd with errno3. These are recovered observations: the fatal read's errno and PID are NOT serialized. Assigning the fatal read errno3 or verifier55601 would be inference. Last child row has escaped checks/kill attempts/failures all0; leader group-kill-attempted0, so no positive escape violation is recorded. The failed command exits the engine at C:5266 before conjunction capture at C:5270; no first_failing_term was serialized, and it must not be reported as `none`. The raw proof remains not_reached.

Oracle/Q2 disposition and a new freeze are needed before another native attempt. All26 historical ledgers retain exact length/hash/dev/inode. Original source/preparation outputs, previous prefixes and both Phase A witnesses remain retained. No cleanup, alternative controller, relaxed gate, native retry, admission or publication occurred.

E/result-evidence.json SHA256 `4e212089366c5065063218bdd996aca32f419ea6fa4a64b5889402ea259aba7f` binds raw files, terminal accounting, receipts and preservation. E/handoff.json1125B SHA256 `59a6e21ca9a7c9f0966023bebc5b4ffd6692bdfa321ef6db1b44b0a4f1279c50`. runtime_approved=false; current candidate remains unapproved, with no qualification.json or receipt.candidate.json.
