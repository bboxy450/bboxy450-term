---
type: report
title: INPUT54 pinned and ingested; preflight terminal scope and ancestry discrepancies
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T01:30:26.214Z
commits:
- a42a789ecef84b2778463de290274aa15596ad24
supersedes: '[[reports/2026-11-07-dream-proof-tools-terminal-sync.md]]'
verify:
  command: python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target
  exit_code: 0
id: 01M6WCECP4FFPE1FPHSGR6PYQQ
updated: 2026-11-08T01:30:26.214Z
number: 93377961366317
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:7df0e8b18f7647b37fce472cf3bcaa3cb7a308ef42a7df3ef879b63ff5bec3cc
  frontmatter_digest: sha256:cff788d03c667640f6bb86c774b959578d3373b45f77e74f751e889df4e57fc0
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: ipNBFJ/a/sgF3WX5rOmfdPY/kV7BtpTnguk35/xgzKkorZuPbOLCova/Ck43iQGZ/+Lm8NLcG4U13VBAyy2mCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150501
  event:
    id: 01M6WCEE4SYW2VBXJYY8JNH5X4
    sequence: 150501
    hash: 1fcc92359ce8570b81f379a35d6d3222efdd4da006b30b9bbfcf9d0813506dd7
---
INPUT54 is pinned and successfully ingested offline. Qualification was stopped before any candidate/probe launch because the frozen producer introduces ten terminal fields beyond the task's explicit ONLY13 whitelist amendment, and the recorded schema-authority ancestry claim is false. No native attempt was consumed. Exact scope and evidence were sent in blocker DM 01M6WC82Z40E9C7XMNCX51K4YZ; delivery evidence is native-queued, model consumption unverified.

## Changes and immutable input

Signed commit `a42a789ecef84b2778463de290274aa15596ad24` changes only `scripts/dream-proof-tools.py` (ROOT_Q2_INPUT identities, projection bindings and Mach-O UUID) and `tests/fixtures/dream-proof-tools/sources.lock.json` (canonical_q2 pin). The thirteen-field terminal validator from 5c717eac is preserved without unauthorized expansion. No reset/stash, peer write, source rebuild/substitution, new dependency, credential/global configuration change, push/merge/install/release or publication occurred.

Selected source/preparation commit `a6468cc3473a8a6f6b76cd5dd48e97d90c6a0b42`; retained root:
`/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-cpu-debt-source54/dream-read-c-preparation/compile-qiog_m7e`.

- C: 320416 B, SHA `8a99da16ce02ce018eee5355f07620f62ebd4917a7007a6c92e2813e53711351`.
- Probe: 251016 B, SHA `a1f58b34d93ee821486c5e5bc9b844f4ccecca2183ff5f4501733437ad531abd`.
- Receipt: 533552 B, SHA `c2d3b8c0dce891e0a60109602217ceefe019beb96a826ea77bc035ed400909ca`.
- Proposal-only read-set digest remains `177cff0e6a8a658a4514494e28506184754f15ba9c50a504dd8f02817db6850c`. runtime_approved=false; no compiled-current-product or runtime qualification claim.

E = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input54-20261108`.

## Actual preflight results and scope gap

| Check | Actual result / evidence |
|---|---|
| Source signature | `git -C /Users/feral/mydev/bboxy450-harness-dream-read-q2 verify-commit --raw a6468cc3473a8a6f6b76cd5dd48e97d90c6a0b42` exit0, VALIDSIG approved Boxy fingerprint 9B07E849480C68328CA20684844ECC0587DE7AB0. |
| Required main ancestry | `git ... merge-base --is-ancestor 2ce4359d a6468cc3473a8a6f6b76cd5dd48e97d90c6a0b42` exit0. |
| Claimed shared-authority ancestry | `git ... merge-base --is-ancestor b26d7cd3432e9cae3e6c68492afd73a82cc8f2e9 a6468cc3473a8a6f6b76cd5dd48e97d90c6a0b42` exit1. This conflicts with the task's INPUT54 freeze text; it is not a signature failure. All three commands and exact exits are retained in E/preflight.json SHA `3f602891f61940b8c59489c356350b081a58b6360bc2f3860fc4ec58e3db795a`. |
| Source bytes and codec authority | All10 named receipt inputs equal `git show a6468cc:<path>`, each actual command exit0. Both schema/vector blobs also equal `git show b26d7cd3:<path>`, exit0. The canonical schema/vector hashes remain d70948ec…/bff32c1a…, despite b26d7cd3 not being a Git ancestor. E/signed-source-byte-checks.json SHA `39ba2b8a6f7015b9016ca7827e83d7934d5021fe43b180aa027e43389c1914af`. Byte identity does not make the false ancestry assertion true. |
| Complete offline immutable ingestion | exit0 through the existing adapter: private root/file identities, exact source/probe/receipt, all selected projection bindings, compiler/linker/SDK/platform/header/retained outputs and Mach-O checks. E/ingestion.json SHA `e48bae872e26e6e12b339822bc9f4f7ef6df22a51e6d6b43a9ec2b467b719cfc`; E/selection.json SHA `e2d739ffd094df040f0757eb6afa154267b604669569a76346068f2e51392dca`. Ingestion is not authority to expand the parser or runtime approval. |
| Offline existing controls | `python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target`: 61 tests exit0; E/unit.log SHA `65db2524f399132fbffe7f9d5fd3abc9b25f4441219927fc784cb65d7ef9770c`; E/unit-exit.json records the real exit. These test current supported input/parser behavior; they do not demonstrate compatibility with the ten unhandled producer fields. |
| History and code state | All8 pre-existing ledgers preserve dev/inode/length/hash; E/predecessor-ledgers.json SHA `f4c45ac9272c7b85fb662b37d230dd604334b0bbdc483b572de8dc79584da44e`. Current recipe SHA `0cd63b912bcc6f6bf6ae034e6870c93d689cedcd40acddbab2fcdda9b186b569`; `git verify-commit --raw HEAD` exit0/Good Boxy; `git diff --check` exit0; `git status --short` exit0/empty. E/final-state.json SHA `69d89fd645f3846806bd919b93151650aeeb7eb103a04968fa5b826858c9d299`. |

The first preflight batch stopped with AssertionError when it encountered the unexpected ancestry exit1, before any source write or native candidate effect. A diagnostic read-only batch retained each individual real exit; the false ancestry check was not retried again. Independent byte binding/ingestion and pinning then completed as authorized. Disk available before build consideration was 44861128 KiB (>8 GiB); no build was started.

Frozen C:3970 emits these ten unconditional fields absent from script:3024:
`booked_ordinary_units, engine_charge_calls, poll_closure_calls, ordinary_cost_ns, ordinary_raw_cpu_ns, ordinary_raw_complete, ordinary_floor_surcharge_ns, helper_cost_total_ns, debt_components_match, debt_scope`.
E/terminal-gap.json SHA `06b04896f637ee88db368f141ba1c4bb25432cc34f2ea5a5feeb4bbf5803458d` records exact producer/consumer field sets and source hash. There are no consumer-only fields; all thirteen previous fields and result JSON/exit-tail shapes remain compatible. A semantically passing positive would necessarily fail the current exact terminal whitelist. A native run was not spent on this known mismatch.

Read-only source findings for the required decision: seven new fields are uint64 decimals; ordinary_raw_complete/debt_components_match are textual booleans; debt_scope is the literal whole_controller_cumulative (C:422,427–428,3977–3978). C:558–561 defines overflow-safe total-component debt matching; raw completeness excludes clock_unknown and retained overflow. This is a source finding, not a new controller/schema implementation or a grant to change the existing ONLY13 scope.

## Remaining work and exact decision

Oracle must amend the task to consume exactly the ten new typed/bounded diagnostics while retaining unknown-field rejection and appropriate negatives, and disposition the ancestry discrepancy (for example, explicitly rely on the already verified immutable codec blobs without claiming Git ancestry), or provide a corrected frozen tuple. The existing explicit ONLY13 amendment does not authorize these ten additional names. The new source/executable/receipt were not substituted.

No source fetch/auth/build, candidate prefix/basis/descriptor, positive/component run, fresh formal parity/license final gates, qualification, receipt or landing occurred for INPUT54. Full task command remains `make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`; it was NOT executed on a42a789e. Its last actual execution remains the preserved result-path 5b8a1108 run, exit2. Current offline exit0 must not be treated as a full verification pass. Existing models/negative controls are unchanged; this pin-only cut supplies no fresh final source-bound formal evidence.

The INPUT54 task corrects the prior Oracle debt diagnosis: helpers already paid 9×max(1ms, CPU); the measured large cumulative debt included ordinary-unit floors, and C14 batches tiny currentness checks. The previous claim of a wall-multiplier/debt-formula bug is retracted by that amendment. Original run01M6W6RKAZEN3P4CHXXEPKPQNS stays FAILED81 and every raw result remains unchanged. This report supersedes current handoff context, not historical failures. No task completion or runtime approval is claimed.
