---
type: report
title: Dream Read Q2 tree-pin correction landed; real qualification remains unproven
task: '[[tasks/2026-11-05-dream-mcp-verified-snapshot]]'
author: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
created: 2026-11-08T04:38:54.741Z
commits:
- ba22c67c488421f58faff3e5256f7fcfff6a5ab8
verify:
  command: env -u FORMAL_JOBS RUSTC_WRAPPER=/usr/bin/nice RUSTC_WORKSPACE_WRAPPER= CARGO_INCREMENTAL=0 python3 -I -B target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/complete-current-canonical-land-v58.py
  exit_code: 0
supersedes: '[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]]'
id: 01M6WQ7FQM567XQ4FQS32K0AZY
updated: 2026-11-08T04:38:54.741Z
number: 93377961366332
created_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
updated_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  kind: seat
  public_key: 3lIiKAyaYQbqAYQ3xsrS+NTGLp9s//BjqCA4HkK5nF0=
  process_generation: '1'
provenance:
  digest: sha256:9c11d5c01a70b00ae1910101c988f53f1d269a55aa03b674ed66991a3ca63aa6
  frontmatter_digest: sha256:fad1a31daeb5386b250f89574674716988c52e5c6f0a635a740c0c77c27a412a
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  sig: UTIwfocGBI/ytkuZ7G/ko9iNU0uwu6cw0zGGV7KZ54ajWEhJCsWgtyEALtusdbqHj2MPgoYsjhvQlfuSATTRAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150549
  event:
    id: 01M6WQ7HKTBMHKJ94S7JTXMASB
    sequence: 150549
    hash: 8216c3949008b916a9229b5e38ef681d802588e77d9399f211eafaa871f67aa3
---
Landed the tree-pin acceptance correction at signed commit `ba22c67c488421f58faff3e5256f7fcfff6a5ab8` in `/Users/feral/mydev/bboxy450-harness-dream-read-q2`; `origin/main` contains it and the worktree is clean. This append-only correction supersedes [[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]], preserving its source57 commit `d1fe3f4b20fe9b34e855215f1eb37227d9972a61`, committed report checkpoint `1d10b85e465d59e25927435ab593f6d5d1169f7a`, actual commands and all old failures.

The previous fixed-form support claim was incomplete: production `engine_expand` rejected the literal Git suffix `^{tree}`, while its source-known flow seam skipped actual argv preparation. C15 fixes that acceptance defect and makes the seam call the production preparation function. All four original task gates were run freshly on source58, including the actual changed READ/C1 worker. The separate real full-private-candidate **30-second fit remains UNPROVEN**. No genuine builder candidate was created, no tool qualification was claimed, and `runtime_approved=false` throughout. The effective task's qualification deferral remains applicable; this is a scoped implementation landing.

## Changed files and causal evidence

Only these three tracked files differ from source57. No model, tool, Rust, Make or test-ignore source changed in this correction.

| File | Bytes | SHA256 |
|---|---:|---|
| `tests/fixtures/dream-read/proof-helper-probe.c` | 320573 | `386cf55eb9b14ff0e77e453d76d8df0edfed30d06593fbcabd4bc0f1ed307d9f` |
| `tests/fixtures/dream-read/private-engine-controls.c` | 116413 | `2bdac30abe119c06d203ee6d1a6701e4e8da240e802bee3dbec83b56d2f32124` |
| `tests/fixtures/dream-read/check-private-engine-controls.py` | 37016 | `fb0728ae8b438a57b11d3efd65b1c8da147d5345ad0281e4ce1d011eed6d98f4` |

`proof-helper-probe.c:2872` now expands recognized named placeholders only and copies other complete/incomplete brace sequences literally within the unchanged 4096-byte bound. The fixed 16-form registry and upfront standalone/embedded `{git_signature_temp}` / `{signature}` bans are unchanged. Direct form15 remains forbidden.

`private-engine-controls.c:87` exercises actual production form10 preparation for both valid and corrupt selected commits: argc19, terminating NULL, `rev-parse`, and exact selected40hex plus `^{tree}`. Literal/malformed/nested braces,4096/4097 boundaries and embedded B-token refusals remain checked. The flow callback now calls actual `engine_prepare_argv` for all 15 directly admissible forms and asserts count15/mask32767. `check-private-engine-controls.py` adds the old-rejection mutant, which must fail that named preparation assertion.

- Narrow production positive exits0; `argv-tree-literal-reject` exits81 at the exact tree-pin preparation assertion. Raw `target/private-tree-pin-causal-degczfb7/run.json`, SHA256 `cbb4876152bfc78291fd26bfbee81944a0206b4590588b2ab3ab6ee59a11d480`; wrapper log `fb8fb3d5074750f06e80abef0f846196594ba1b319bd1492193130aa756615fd`.
- The current same-HEAD MCP gate reran the canonical C15 suite: **68baseline /40source mutants /42strict compiles /86commands**, actual0; `target/private-engine-controls-f3iz6zo3/run.json` SHA256 `f97b8aaac3a6b23ab74f8b8ce14ef9d28214f21ce64afe32dcb4e65c21cb0875`, immutable input snapshots match all three signed files. Every causal rejection remains an expected raw nonzero, never relabelled as a successful candidate.
- Earlier frozen-byte C15 canonical run also exited0 before the source58 commit: log `9b0ac90da53ef2725aa49b4e2bd59ff89a8f29fc5c9b7a40bb8e0584178de74a`, raw record SHA256 `9996b2a1259be10ad174107615a437ff1c0619e956c7d8e9b838b81d30d4b3ab`. Its recorded HEAD remains source57 with the exact then-uncommitted C15 bytes; it is not presented as a source58-HEAD run. Current MCP supplies the signed source58 run above.

## Actual verification and landing

The task's verify string remains `make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build`. Each constituent canonical command below actually exited0 on the same clean source58, serially, with `FORMAL_JOBS` unset, `RUSTC_WRAPPER=/usr/bin/nice`, empty workspace wrapper and `CARGO_INCREMENTAL=0`. There is no claim that a whole four-chain shell was executed or returned0.

| Current canonical phase | Actual argv | Exit | Log SHA256 |
|---|---|---:|---|
| C1 | `env -u FORMAL_JOBS RUSTC_WRAPPER=/usr/bin/nice RUSTC_WORKSPACE_WRAPPER= CARGO_INCREMENTAL=0 make test-dream-c1` | 0 | `221391d5409418494e7a3b8dcef30d38ff3a6c2d452d84c5316168031bed86ba` |
| MCP / READ / integrated C15 | `env -u FORMAL_JOBS RUSTC_WRAPPER=/usr/bin/nice RUSTC_WORKSPACE_WRAPPER= CARGO_INCREMENTAL=0 make test-dream-mcp` | 0 | `8524922ad5e5c536fc6aaf2da1226a5faeb730a6397995e3a2e732dddf58aee5` |
| Full native + pinned Java parity | `env -u FORMAL_JOBS RUSTC_WRAPPER=/usr/bin/nice RUSTC_WORKSPACE_WRAPPER= CARGO_INCREMENTAL=0 make test-formal-parity TARGET_DIR=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/formal-tree-pin-source58` | 0 | `805639f553f12db2d2c60d9acb80f8cddee0597ff71113a09b415c0064c5d79d` |
| Embedded build | `env -u FORMAL_JOBS RUSTC_WRAPPER=/usr/bin/nice RUSTC_WORKSPACE_WRAPPER= CARGO_INCREMENTAL=0 make check-embedded-build` | 0 | `e4578039d9d86ef363fc28fdc752af05913e26fa2fa13584b7db3368036ec50f` |

The actual composed completion command is:

```
env -u FORMAL_JOBS RUSTC_WRAPPER=/usr/bin/nice RUSTC_WORKSPACE_WRAPPER= CARGO_INCREMENTAL=0 python3 -I -B target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/complete-current-canonical-land-v58.py
```

It exited0, log SHA256 `1002799aa669eb7e0bde90af309332a43deb3113a2582c46caedbfecf0468ea0`. It checked the original phase records/exits/log hashes, same HEAD and frozen C snapshots, then executed the **unmodified** `scripts/land.py::gate`: licenses, embedded lints, both canonical strict Clippy configurations with `-D warnings`, current lib nextest (188 passed/9 skipped, no failures), and the canonical current formal-receipt validator. It used existing canonical signed rebase/fetch/source-digest/signature/push functions. The existing current formal receipt was consumed after validation, without another Java run; current MCP was not repeated. The changed paths relative to source57 are only C3, so the canonical gate's integration-selection rules are unchanged; the freshly completed full C1/MCP commands above provide those task phases.

Actual source-bound canonical receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-canonical-receipt.json`, SHA256 `46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6`, source digest `961e6b43a2f1cd5c057552798c71b39500cebacaf1c443a5b03cd4921eba4190`. Signed commit verification was actual0, fingerprint `9B07E849480C68328CA20684844ECC0587DE7AB0`, author/committer Boxy `<bboxy450@gmail.com>`. Fetch/push and the post-push ancestor check exited0. No original `make land` shell0 is fabricated. Source57's original `make land` exit2/archive ENOSPC and its separate successful canonical completion remain preserved as historical facts.

## Actual workers, native/reference evidence and immutable artifacts

The changed C blob is embedded by `src/dream/resolve.rs:354`, so source57's old `01bc...` worker cannot stand in for the corrected artifact. Fresh current normal READ and C1 worker SHA256 is **`f8876d9545171a46a1920c998da690bb01ddcd3b29a77747dd37181ed4d6620e`**. Actual metadata was copied byte-for-byte after each successful gate, with independent read-only worker inodes; nothing was reconstructed or relabelled from source57.

- `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/retained-ba22c67c-actual-read-profile-MCP-completed/preservation.json` SHA256 `406eac8e5788cca0397913a055619aad52e4dfd4c89596f9883441e8b1b16a7c`: 47 actual generated files, dream-read-native-profile worker f8876d9545171a46a1920c998da690bb01ddcd3b29a77747dd37181ed4d6620e.
- `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/retained-ba22c67c-actual-C1-profile-completed/preservation.json` SHA256 `cd70de3c8ad2e5d6400c9f15b63f5674c3da48bb1ff4247e3a8dfa3314c8a998`: 6 actual generated files, dream-c1-native-profile worker f8876d9545171a46a1920c998da690bb01ddcd3b29a77747dd37181ed4d6620e.

Current READ preserves its actual39-input snapshot/stamps. The canonical five observer JSON outputs were moved by the unchanged same-filesystem preservation helper, retaining their original inodes and bytes: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-observer-evidence-3f4kbt8p/preservation.json`, SHA256 `6edeb9cb66aa14874b542461fabd9a1b2057204adaedf2a3941e768f5126c84b`. Their real canonical MCP0 prerequisite was checked before preservation; all five source57 observer artifacts remain separately intact.

Fresh native and pinned Java TLC completed **432models /93contracts /1crate**, **3351917states /11296388labelled edges**, full state/edge equality and named expected verdicts. Native cap100000, pinned tool versions, Java heap/workers/timeouts, all 82 TotalBudget variable domains,83assertions and18causal controls are unchanged. No model change was needed for this pure fixed-argv parser correction. Current parity is a genuine fresh run, not a rewritten source57 TLC receipt.

Current immutable archive: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/verification/evidence/353b41612770b4fa439396ffb3e5822018690ac1eddbb9690296beeaf2d99b6e`; evidence SHA256 `353b41612770b4fa439396ffb3e5822018690ac1eddbb9690296beeaf2d99b6e`. All **4322files /7214245986logical bytes** are enumerated by `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-formal-all-raw-files.json`, SHA256 `7250eda36d37a2e05a2a0611faabbedf199c12e25e8782b352829c93ebf6b813`. The canonical archive has0400 files/0500directories, no symlinks or hardlinks; its writable landing cache is an APFS clone with distinct inodes/nlink1 and verified equal bytes. The cache's canonical current-source validator exited0. A fresh canonical target avoided inheriting obsolete graphs; no exporter/cache criterion changed and no old evidence was deleted.

Storage was preflighted. Owned cold default54/source57 archive+cache graph sharers were transparently compressed with lsof/hash/inode/mode/time journals before current publication. Evidence bytes and paths were retained, with changed physical inodes/ctime/flags explicitly journalled. Current archive publication succeeded0; free space after current archive/cache preparation was28297240576bytes. No peer worktree, peer target, credential, install, service or release operation occurred.

| Unchanged model/tool source | SHA256 |
|---|---|
| `tests/formal/dream_read.rs` | `c02ee30b512ae15ae2335c5db76b4799aa54c67c5c9d709998344f6bfa01944a` |
| `tests/formal/models.rs` | `4e992589a372c2b63bd47b05fdbb5c0639f5110e5e8b42f3b90130c231e237ca` |
| `verification/contracts.toml` | `1deffb06e5e60d64e7b40f4d63432602ff2ced39958bda588b19d1cd26b52b66` |
| `verification/corpus.toml` | `3b2a226fc7fc8f8603e2a855af492207e698cfc01f42789280a9e952d2f7e8ce` |
| `verification/coverage.toml` | `bb4e09ad9b54700b81893d630e2577ad1a5db79a051a05a3a7ef46cfdd2e4038` |
| `scripts/formal.py` | `4b87e5b4f6388d8b1dbfe0ce5cf7f778462c6d05c589c5fb2d57d162d6a7cd79` |
| `scripts/land.py` | `36bd8eb4b0d6bcba5efd46ee94d5d0f035bb3c5ddb5c12aa02efb8dc0a859354` |

Pinned TLC JAR SHA256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`; Temurin21.0.13+9 arm64 JRE archive `225bab0a09491ef06835488cd942f5d2befa9a0410a2484f0c8bbcf7506ec09a`. These finite checks do not prove arbitrary Rust/native executions or complete TLC replacement.

## Replacement immutable Input58 and the remaining budget boundary

Essential handoff1408, id/op `01M6WMN9ABS8G0TR5ZC9ZX5S6V`, explicitly retires Input54/message1396 for current use while preserving it. Storage acceptance is not builder model receipt or runtime approval. Full ten-input/compiler/linker/SDK/platform/header/source-readset tuple: `R/tree-pin-input-source58-tuple.json`, SHA256 `49bd13c6511b27d0233176bd69d45356239723c6226a99eca69c5f1d2afa47a2`; here `R=/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3`.

Source commit `ba22c67c488421f58faff3e5256f7fcfff6a5ab8`; immutable executable SHA256 `0759cc153f74ae23c22eb2e952aa7b0af149dfd7102af595bafaa6765c533bde` (251016bytes,0500,nlink1); preparation receipt SHA256 `e73178e5d779cca4d4ec1b0d6e1afd203746e8a2bad032e243d276f371d64ef3`. Canonical preparation actual0/log `53b240a7748d9e6d03d3d7c687381a6188fe2a0bac5e798d82450ba6cc3c429f`; independent audit actual0/log `d797a7b56b6677d1c89468f7b32e26569ddcd950f516267bfcabd664e4937b2b`,997checks/325headers, detail SHA256 `7a4ead81ee49b87ff6c9554c8b8659606672cd5548cb9b3f6bfd77f7527196d0`. Private roots0700/files0400; lexical and resolved pthread/spawn.h and its actual two-argument QoS declaration are bound.

Compiler is the recorded Xcode clang21.0.0 executable SHA256 `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`, strict optimized flags `-v -O2 -std=c11 -Wall -Wextra -Werror -target arm64-apple-darwin -isysroot <recorded MacOSX.sdk>`, SDK27.0 settings SHA256 `7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb`, linker SHA256 `374d9f0713bce2c8a9e1e82bde51615d5433516a10d99b63107eb6ee70542e4c`, Darwin27.0.0/build26A434/arm64. Exact signed schema/vector authority remains `b26d7cd3432e9cae3e6c68492afd73a82cc8f2e9`: schema15157bytes SHA256 `d70948ec375dae099ea1dac0b92dfb30ff87d7fcac94c98f42caa1c0ba2da8ec`; vectors3900950bytes SHA256 `bff32c1afb483980952a6761918d4005c9f1e12b66f3d7aaaa88f1b9afe09740`. The tuple preserves source4 domain `177cff0e6a8a658a4514494e28506184754f15ba9c50a504dd8f02817db6850c` as a source snapshot, separate from the actual READ worker profile.

The narrow source-known `flow-controls` positive command exited0 and measured703265000ns elapsed,67ordinary units,4082958ns raw ordinaryCPU,62917042ns floor surcharge,603000000ns fully paid global debt,14path units,5retained guards,15actual argv preparations/mask32767 and **0actual helper births**. All four variants passed their actual commands. Native/wait4 metrics there are scripted; its entry scope is command-engine-only. It does not exercise outer/inner authentication, all guards, compiler/SDK/platform validation, whole preflight or real helper lifetime. Raw record `target/private-tree-pin-engine-flow-i13333fj/run.json` SHA256 `87ece75c4d9772df66ee31f11f845460da07d2049d4e79cb3b32d4421e4ca710`; corrected consumption-only evidence SHA256 `69314f3d7f975d815feea718da096845dfbf1d7165c628058a01b695f4f84db8`. This is **not** a demonstrated real full-positive30s fit. The original wrapper exits1 because its evidence parser expected the wrong output field names; raw command0 and wrapper1 remain distinct, and the green command was not rerun to hide that wrapper failure.

Prior C13 raw accounting remains584118000ns helper debt plus25245000000ns ordinary/native/poll normalized floors (2805×9ms) =25829118000ns global debt. It did not prove a9×wall helper billing bug. C14's scoped currentness batching/attribution correction remains intact; the whole genuine candidate's30s ordinary-floor fit and B startup/real tool qualification remain the builder/proof-tools follow-up. No speculative batching redesign or peer-root read was added here.

## Acceptance map, deferrals and failures

The current14-entry source/hash/line map is `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json`, SHA256 `6a8388432fb048ece8b195c20db2cdd6f9c60809df647ac20725672b4255a26c`. It links these preserved requirements to source58's actual task/landing phases and inherited explicit boundaries:

| Criterion | Requirement | Source/evidence |
|---|---|---|
| wire | Distinct prepare-read wire, exact context/manifest binding and bounded immutable pages/descriptor. | `src/frontends/headless.rs:32`; full source/log binding in acceptance-map entry `wire`. |
| bundle | Finite selected IR/JSONL/blobs, private Git, public policy/keys, catalogue and artifact-to-file custody; no arbitrary receipt paths/fallback. | `src/dream/read.rs:295`; full source/log binding in acceptance-map entry `bundle`. |
| published_lessons | Compatible adoption, current record/trial/publication/receipt/trust/transitive proof, safe lesson fields and verified_at_revision. | `src/dream/read.rs:2286`; full source/log binding in acceptance-map entry `published_lessons`. |
| resource | Shared64MiB input; parent32MiB/child96MiB, protected control/helper shares, bounded allocation/disposal, four jobs and actual joins. | `src/dream/read_wire.rs:14`; full source/log binding in acceptance-map entry `resource`. |
| cost_debt | Own-thread ordinary1ms/native2msCPU25mswall3msinclusive/full9x debt unchanged. Decision66280 removes external per-grant/Q/T/N; helpers run continuously at atomicBACKGROUND/observedPRI4 with250ms command/3s job totalCPU, actual per-member RI begin-to-begin+return gaps<=50ms, terminal originalwait4/rawovershoot/fresh9x postreap debt/terminalRSS16MiB. | `src/dream/c1.rs:748`; full source/log binding in acceptance-map entry `cost_debt`. |
| bootstrap | Canonical128B permit/lease, unchanged250ms readiness and30s maximum work deadline; no argv/channel/owner/source admission before permit, null-context probe rejection and actual settlement. | `src/dream/bootstrap.rs:16`; full source/log binding in acceptance-map entry `bootstrap`. |
| helper_custody | Original positivebirth/audit/PGID custody, bounded2members, positive ownWNOWAIT/actualownwait4, known-native census status, retainedchild positive exitedRI then fresh sameleader qualified componentwise rollup; live/escaped effects before reap seal, independentgroup proof/ownedquarantine. KnownleaderCPU paid even if unknownchild held; no postownreap signals. | `src/dream/c1.rs:829`; full source/log binding in acceptance-map entry `helper_custody`. |
| signature_B | Git-created verifier temp argv/custody GS1/GS2; matcher never directly expanded; original inode/hash/scratch cleanup or quarantine. | `tests/fixtures/dream-read/proof-helper-probe.c:3353`; full source/log binding in acceptance-map entry `signature_B`. |
| shared_codec_C | Exact signed shared schema/vectors, bounded strict decoder and seven-source/private pre-test bindings, no runtime self-approval. | `tests/fixtures/dream-read/proof-helper-probe.c:723`; full source/log binding in acceptance-map entry `shared_codec_C`. |
| terminal_verdicts | Independent control/persistence/terminal-cost verdicts, bounded retained errors and post-tail receipt; fixture setup cost cannot disappear. Effective23:10 fixes bind admitted inputFD+freshhash independently of replaced locator; distinguish native IO failure from paid work-deadline refusal; one-way terminal reporting cannot admit work and attempts finite/paid result after original30s expiry. Bounded path batching retains full actualCPU/debt. | `tests/fixtures/dream-read/proof-helper-probe.c:3869`; full source/log binding in acceptance-map entry `terminal_verdicts`. |
| retained_host | One durable A close, finite subscriber/socket drain and retained Host creating one nonfork B with managed launch inputs. | `src/engine/mod.rs:510`; full source/log binding in acceptance-map entry `retained_host`. |
| formal | Finite owner/actions/bounds/fairness and all deliberately brokencontrols; honest66280 retirement/inventory. Exactnative cap100000 and pinnedJava fullstates/labellededges/namedverdictparity. Incomplete/statecap/OOM is failed; no sourcecurrent claim before freshcanonicalgates. | `tests/formal/dream_read.rs:2625`; full source/log binding in acceptance-map entry `formal`. |
| provenance | Signed exact source, current worker/compiler/source-input profiles and formal model/tool evidence; immutable Cpreparation/source4readset/schema/vector/compilerheaderclosure tuple separate from runtimequalification. Retain all failed runs/sourcecuts; no oldtuplecurrent. | `src/dream/read.rs:2648`; full source/log binding in acceptance-map entry `provenance`. |
| landing | Canonical current-source gates and remote main containment before final handoff. | `scripts/land.py:112`; full source/log binding in acceptance-map entry `landing`. |

The unchanged **25 task-specific deferrals =20 QoS +5 qualification** are named individually in `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/landed-source57-exact-deferred-tests.json`, SHA256 `4944bb4afb57cb45611059cce934ab34dd23d71963bf6230238b7d71817c2a2d`; they are not passes. This correction added no ignores, budget changes, threshold changes or assertion relaxations. The historical physical fixed-oversize negative under the retired grant law remains separately DEFERRED/UNPROVEN, never folded into the25 or counted as a pass.

Completed R/run.py-schema records/log hashes as of the inventory snapshot are in `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-all-completed-actual-runs.json`, SHA256 `498051b4c09e8cf8a7d7c92cf86876bc0d62d111890ffa0831a37cbd7bb068b0`: 364records, 108nonzero,0log-hash mismatches. Custom historical operation-result records remain separately preserved. Deliberately broken controls, old state-cap/type/liveness/TLC timeouts, old C pacing/owner-release failures, full50 exit2, source57 archive ENOSPC, the premature READ-preserver1 and flow-evidence-parser1 retain their original exits. The premature preserver attempted before its build completed; no metadata was invented, and a later distinct-name successful preservation followed actual build0. This correction's first final-evidence collector also exited1 because it mistook a historical custom rebase-result schema for a R/run.py record; its raw failure remains. The v2 collector uses the actual runner schema and consumed the existing gate results successfully, without repeating any product gate.

Historical controller-ownerloss is still quarantine/unproven, not own-reap success: Rust controller/helper pairs29617/29747,75435/75533,97797/97846 and C36046/36792,57264/58336 retain original raw records and exact-PID cleanup evidence. Later absence, Root cleanup or helper disappearance never certifies the original controller's reap/full bill. All old tuples/source cuts/retired model inventories and the previous committed report remain preserved. Current eligible commands/causal controls above pass; genuine tool qualification, real full-private30s fit, arbitrary-native guarantees and runtime approval remain outside this completion claim.
