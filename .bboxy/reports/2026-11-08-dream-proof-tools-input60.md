---
type: report
title: INPUT60 exact ceiling/diagnostics ready; native attempt gated by disk capacity
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T07:47:44.771Z
commits:
- 68f6d062d83af635dfc87e95beaf834b2c9766ca
supersedes: '[[reports/2026-11-08-dream-proof-tools-input58.md]]'
verify:
  command: python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target
  exit_code: 0
id: 01M6X218RQZ99ZKQMEDGXRGX64
updated: 2026-11-08T07:47:44.771Z
number: 93377961366343
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:c9e2a37880984eada2fb5464eef5ca3f498287246ae4bf1dc044e0469a157dbf
  frontmatter_digest: sha256:dd6a487b7e6708e5bfc09805a18bad38c610ac43336dbdbdf3c790d5bb85b798
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: tBSNx03bhC+ETmONqFk3Tj9kS1LL/P0iEVd+lQPB7hL4Aoo9/lpTz4aeMc803QxOTEhLI5gsASW6gQVEWJ4GAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150592
  event:
    id: 01M6X21A3AF82KR2MHVPHY7DAH
    sequence: 150592
    hash: 657b2cba0385cdda592296b28b94aaba0b2032945779e8c392a33ed8cc0e15d9
---
INPUT60 implementation and offline verification are complete, but the authorized native attempt has NOT started: the task's pre-build requirement is strictly more than8GiB available, and the measured available space is below that gate. No source fetch/build or native candidate/probe launch was attempted, and no preserved artifact was deleted to bypass the gate. runtime_approved=false.

## Signed change and authorized contract

Signed commit `68f6d062d83af635dfc87e95beaf834b2c9766ca` changes only `scripts/dream-proof-tools.py` and `tests/fixtures/dream-proof-tools/sources.lock.json`.

- ROOT_Q2_INPUT and canonical_q2 consume the exact INPUT60 tuple: source/preparation commit `1602aff72e961e493cca14feb559d6a323733f67`; C325317B SHA `ed71ff1afc65da2fc5ca1161ff16fc6e4b2c03c8f6b2ac947f0d43ad757a552f`; supplied probe251016B SHA `67060d75347926eb9d4f8a64006d9444b0fa0f71fdeca010fef92f9c4e025e8d`; receipt537042B SHA `6d1b82a8f4fdb67da05b865a1ccddf8203764757d82dda57773d926fd8aa5584`.
- Native result wall maximum changes25ms→250ms only for the frozen producer's process_create metric, implementing [[decisions/d93377961366331-dream-q2-process-create-wall-ceiling]]. C:3022 is its only writer and C:3607 applies the process_create limit. CPU2ms, application1ms, inclusive3ms and all other native class25ms ceilings remain unchanged; no historical result is relabelled.
- Exact terminal whitelist adds only four authorized names: process_create_wall_excess_ns/count as canonical full-width u64, process_create_wall_excess_known as numeric0/1, and process_create_wall_excess_scope equal to invocation_sum_once_per_prepared_spawn. Unknown/missing/duplicate fields still refuse; existing verdict/persistence/debt/settlement checks remain.
- Inert regression controls accept34ms and250ms process creation, reject251ms, retain CPU/inclusive failure gates, exercise the new u64/flag/literal domains and require every field/reject duplicates and an unknown extra. No Q2 source/controller, wire/schema/version, new library/dependency, model state machine or runtime authority was added. Existing attribution/upstream notices remain attached.

The receipt is retained under:
`/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-process-create-wall-ceiling-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input60/dream-read-c-preparation/compile-jqefbsuk`.
The proposal-only read-set digest is now `8e6d9207723751cfb936587ecac7f06673321acbca05be739613e0736c2e4ee8`; this is an exact frozen source snapshot, not a compiled-current/runtime approval claim.

## Actual offline evidence

E = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input60-20261108`.

| Criterion | Command/result and evidence |
|---|---|
| Signed selected provenance and main binding | Source git verify-commit exit0/Good approved Boxy fingerprint9B07E849480C68328CA20684844ECC0587DE7AB0; af5de27c ancestry and source→local origin/main ancestry both exit0. E/preflight.json SHA `cf30abb75f95ca453aba81f2fa8b519aff3e85322cead448b85f463afc8284c0`. This verifies the local tracking ref, not a new remote fetch. |
| Actual exact10 source and codec byte bindings | All10 git-show checks exit0; exactly c1.rs/read.rs/resolve.rs/proof-helper-probe.c differ from INPUT58, the other6 are byte-identical. Two codec blob checks exit0 match b26d7cd3 authority. E/source-byte-checks.json SHA `13ad599f7ef136afbc8ffac1c94d5a72a9a93881b57e1291fb52413c638d0ea9`. |
| Complete immutable offline ingestion | exit0: source/executable/receipt length/hash/identity, private root, all18 projection bindings, compiler/linker/SDK/platform/header/retained-output/Mach-O checks. E/ingestion.json SHA `eeb64697e162267700c16c15ac5c61163e73b6a5b1fc466408751c8ee6895bd6`; E/selection.json SHA `c2b937b6cb254852f13f808cddbd24c1de80dbe8afa7377e30a77a830348a63b`. No execution/approval follows merely from ingestion. |
| Exact four-field amendment and per-class metric meaning | E/terminal-binding.json SHA `a335203ed4da4d8bf7ac057fc4b701497f168e768d0a1b10d01e047b76cecc37`: exactly4 additions/no removals and exact frozen producer field set. Decision66331 is the authority; other class bounds are not raised. |
| Unit controls | `python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target`: **66 tests exit0**. E/unit.log SHA `f18add4ca5b986e33a4f2bd985ea6b6cbe685310ad5671772ecd4d7eae9aedf9`; E/unit-exit.json SHA `783dd25e9e88a9072fb1075496d4db20b3e009ab5107b2f724d773089a1cb8c0`. Synthetic parser controls are not a native qualification witness. |
| History and current code | All10 earlier ledgers retain exact dev/inode/length/SHA; E/preserved-ledgers.json SHA `17cac3d128957b53801a335452b79c45f1f24981ab838da563a59cfcf8946cc4`. Current recipe SHA `cd4095417b06309f7849f73955d3ccde279edb4270d41a0e28a9cd217790d76b`. HEAD signature exit0/Good Boxy, git diff --check0 and git status --short0/empty, E/final-state.json SHA `3502bd7cc5351a367af93d1f3ffd33017da768b4caedae7d297009bbd0eecce4`. |

## Concrete blocker and remaining proof

The actual command `/bin/df -k /Users/feral/mydev/bboxy450-harness-dream-proof-tools` exited0 and returned **8,255,056KiB available**, below the strict **>8,388,608KiB (8GiB)** pre-build gate. E/disk-gate.json SHA `67137ac4e810d5c6e2b052b20b172905b21643f2f8561d93308ba51010d6469e` includes the real stdout, sample time, comparison and native_attempt_started=false. Earlier measurements in the same work were also below8GiB. No build was started on a failed disk gate. No cleanup of own retained failure evidence, predecessor/peer worktrees or candidate prefixes was attempted.

Required decision/action: release sufficient space above the existing gate, or specify exact authorized disposable paths with their disposition. All already authorized implementation and offline checking is complete and reviewable before that decision. The single INPUT60 native attempt remains unconsumed. Blocker DM `01M6X1VQXTFFYVJA7N94Q3T6R8` records the exact gate and scope issue; message_receipts reports native-queued, model consumption unverified.

Telemetry limit relevant to the requested actual guard count: the frozen C stores its actual runtime count in private_guards.count (cap256, deduplicated by root/path) but does not serialize that count in result/terminal output. No new INPUT60 basis/run exists yet. Actual runtimeG, final helper/raw/floor debt split and observed process_create wall excess are therefore **unobserved**, not zero or an estimate relabelled as telemetry. A later basis can support a separately labelled source/metadata-derived guard-set cardinality; a directly measured counter requires Q2 instrumentation and an explicitly frozen input. No unauthorized fifth terminal field, tracer or cloned controller was introduced.

The exact task command remains `make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`. It was NOT executed on68f6d062. Its last actual execution was INPUT58/f6cdc7c4, exit2, preserved in [[reports/2026-11-08-dream-proof-tools-input58.md]] (#93377961366329). The current offline unit exit0 does not replace that failed full-task result or establish current final source-bound formal evidence. Fresh source crypto/build, positive/components, formal parity/license final gates, qualification/receipt and landing remain outstanding.

No native attempt, source fetch/build, new candidate/basis/descriptor, qualification, receipt, runtime approval, push/merge/install/release or threshold waiver occurred for INPUT60. Prior INPUT58 run stays FAILED81 under its original25ms rule. This is a blocked pre-build handoff, not task completion.
