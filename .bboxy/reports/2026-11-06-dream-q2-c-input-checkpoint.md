---
type: report
title: Q2 signed shared C private verifier input checkpoint
author: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
created: 2026-11-06T16:34:58.368Z
task: '[[tasks/2026-11-05-dream-mcp-verified-snapshot.md]]'
commits:
- a3864e7ce09d5e0dc2018385118786d89b0c198e
- 7a24cf5c38844a45783f3fb2e35fb8380dead8f8
verify:
  command: make prepare-dream-read-c-probe
  exit_code: 0
id: 01M6RVD7CVPJWYE4DN3W203WAF
updated: 2026-11-06T16:34:58.368Z
number: 93377961366041
created_by: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
updated_by: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
authorship:
  signer: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
  kind: seat
  public_key: p1xg5YnZmsPv/B3q4mjUXp1coPPED4IjwlMtDzJrDaE=
  process_generation: '1'
provenance:
  digest: sha256:3dd0ea6e0744fdd5d6831b91847a356613b4d87bff0329173ee3c816d06de50a
  frontmatter_digest: sha256:60fe5b2c5d0e955baab59021517456a5a8b982bf636be50c81c9b914e5d47904
  signer: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
  sig: Zz0JCei4INqX+lATabIQ5DR3uFBci23W0P7KIoG4clapQHXS0qVXEu88wtiD/5YjAaVoNo4bYUHgepMJuh0lAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149586
  event:
    id: 01M6RVD89QWEHTRDP1K5XZWJ12
    sequence: 149586
    hash: b9b96c097ccbdcc106b3a1b5707b9da68491f0e1a679d32ea07d1b7a5990df83
---
Signed private verifier INPUT checkpoint for [[tasks/2026-11-05-dream-mcp-verified-snapshot]]. This is the ordered source dependency handoff, not a task completion, landing or qualification. The original task verify and every original acceptance gate remain binding.

Signed source commit: `a3864e7ce09d5e0dc2018385118786d89b0c198e`, Boxy fingerprint `9B07E849480C68328CA20684844ECC0587DE7AB0`, `%G? G`. Byte-identical builder artifacts imported in signed `7a24cf5c38844a45783f3fb2e35fb8380dead8f8` from original signed `b26d7cd3432e9cae3e6c68492afd73a82cc8f2e9`. No unrelated builder commit imported.

The single canonical `proof-helper-probe.c` now parses the builder-owned ordered image/static/per-output graph, Platform6 premise, all five exact load modes and repeated occurrences, global IDs/file paths, reachable ownership/map rows, offline Ref consistency and 64node/64array/128edge/scalar-depth10 bounds. It compares actual selected Mach-O modes/names/order/subtype/UUID and both platform fences; observed failure remains sticky. Provenance/source/map/archive/compiler/SDK payloads are not opened by runtime metadata parsing. Current candidate/config/fixture and prepared-document reads retain original paced custody. Exact prepared-size and parser/registry storage are reserved before the first controllable touch. The lexical candidate resolver cannot leave and re-enter the prefix; candidate-prefix paths cannot masquerade as cache targets. Disposal failures propagate before proof consumption.

Preparation receipt: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-preparation/preparation-9a141a88754aeffbb06d432b77da8956407cdbfe54b919be6ecbf9ff0705a68c.json`, len 284341, SHA256 `9a141a88754aeffbb06d432b77da8956407cdbfe54b919be6ecbf9ff0705a68c`. This is an OFFLINE build/input receipt with323 retained header identities/copies, never a prepared runtime descriptor/Basis/catalogue or extra runtime allocation. It includes exact source snapshot, actual compiler/linker identities, compiler version/target/flags, verbose real compile/link invocation, SDK settings/platform, before/after header/tool equality and immutable probe identity.

Immutable probe: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-preparation/probe-17df1b6fb05c09115c0fc3baeb37ef8049b2d1fde6fa90c52585406185d0a530`, len 181720, SHA256 `17df1b6fb05c09115c0fc3baeb37ef8049b2d1fde6fa90c52585406185d0a530`.

Compiler: `/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/clang`, len 124676976, SHA256 `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`; Apple clang version 21.0.0 (clang-2100.3.34.2), reported target `arm64-apple-darwin27.0.0`, canonical `aarch64-apple-darwin`. Actual flags: `['-v', '-O2', '-std=c11', '-Wall', '-Wextra', '-Werror', '-target', 'arm64-apple-darwin', '-isysroot', '/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk']`. SDK 27.0 settings SHA256 `7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb`. Linker SHA256 `374d9f0713bce2c8a9e1e82bde51615d5433516a10d99b63107eb6ee70542e4c`.

Exact signed inputs (path — length — SHA256):

- `Makefile` — 23602 — `260f0acb461e5e62643e91d97b52d44746db3e2e94d8cbe55aad9d8f7dc306ad`
- `src/dream/c1.rs` — 284736 — `1033065f5ee48ce882282e1b4e231307c324ef0eb1b7099dac1009e1f8a8516b`
- `src/dream/read.rs` — 143302 — `d58e9e2b54051b516a31a255e16eec88aaec06406324bea1b4bc55d314bfbf3e`
- `src/dream/resolve.rs` — 168454 — `bd1a5c8509f8d237d635c553bd8171561b1e128526583ebf064ed7053752f2f3`
- `src/dream/trust.rs` — 54997 — `5f9e18af286da3c868d250c4c50657c7c8ec27b0ed5e6e91559bf13e5940e575`
- `tests/fixtures/dream-proof-tools/closure-boundary-vectors.json` — 3900950 — `bff32c1afb483980952a6761918d4005c9f1e12b66f3d7aaaa88f1b9afe09740`
- `tests/fixtures/dream-proof-tools/closure-boundary.schema.json` — 15157 — `d70948ec375dae099ea1dac0b92dfb30ff87d7fcac94c98f42caa1c0ba2da8ec`
- `tests/fixtures/dream-read/prepare-c-probe.py` — 9874 — `a4466c33fc019f9730150b0d987687956ab0608b34805e4237e87df425d2b0b7`
- `tests/fixtures/dream-read/proof-helper-probe.c` — 266098 — `80ed140af40cefe6dc6bc84deb04aaf86b8222aa57a5eeac7db6cf56d5e50b56`

The four proposal Rust sources are byte snapshots from the signed commit, with existing NUL-terminated `bboxy.dream.proof-tools/source/1` domain digest `5407b1439d947c22bcab8c482e288227f95a3367d291ca62b60ec5d54d5cb91a`. No product executable/currentness is claimed by that digest. Source snapshot: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-preparation/source-a3864e7ce09d5e0dc2018385118786d89b0c198e`.

Actual commands and retained evidence:

- `make prepare-dream-read-c-probe` — exit0; `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-finish-recovery/c-input-preparation.log`, SHA256 `267d7e2e27e71f0b9e497fbb391ac97bfc47157b9fd66b58a0620367b1550168`.
- `make test-dream-read-closure-codec` — exit0; `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-finish-recovery/closure-codec-signed-input.log`, SHA256 `cd8abb40b4d076b06e587b5e653af8fa62b265d4ee5c6523f87a2629391b07fe`.
- `cargo test --offline --locked --release --jobs 2 --target-dir target --test dream_read private_candidate_outer_codec_refuses_noncanonical_and_overflow_before_birth -- --test-threads=1 --nocapture` — exit0; `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-finish-recovery/closure-migration-outer-regression.log`, SHA256 `fbc183574671de150fb64cf18ea540c738678e9e569f2e24f4a6cf109e8972cd`.
- `cargo test --offline --locked --release --jobs 2 --target-dir target --test dream_read signature_boundary_controls_refuse_before_helper_birth -- --test-threads=1 --nocapture` — exit0; `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-finish-recovery/closure-signed-signature-regression.log`, SHA256 `7bb0d9abd3da343cd302f6a3ae2536f097384405980f6d1b5e5ce8b9cfbf0aa5`.
- Narrow exact signed-model export and pinned native/Java TLC graph/verdict comparison — exit0 for all9 models, `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-finish-recovery/closure-signed-model/evidence.json`, SHA256 `3d284580b564f653616ba9e1480a72c3c18161aa47aa07e2d1729e098a2a2b31`. Good40states/39labelled edges; eight causal controls each graph exit0 and named safety verdict exit12. This is not full repository parity. Exact signed model copy SHA256 `7b29bddfe3fef1c0b4e7f9060046746d1383d1a3353cf07f8838cbde0af1c06c`.

The independent consumer passed all324 immutable vectors (44accept/280reject) with target-specific closure/fragment/grammar/offline framing/copy accounting and13 independently hashed frozen synthetic contexts. An additional split-buffer control checks aggregate per-kind accounting. The source-free wrapper also refuses prefix escape/re-entry and candidate-prefix/cache confusion. Outer27parser cases and GS2 actual owned-directory normal/killed/replaced controls pass; the GS2 private source mutant actually exits81. These are parser/API/file-custody witnesses with zero Git/GPG helper creation.

Remaining boundaries: synthetic map blobs and LF synthetic archive indexes do not qualify a real link-map/archive parser; raw canceled loader components still depend on the admitted offline no-symlink/source premise. No unseen transient cache/native heap/lifetime theorem follows from samples or codec success. Private prepare/semantic paths retain honest coverage_unknown/review_unbound, normal Git remains81, runtime_approved=false, basis=conditional_source_review, whole_command_lifetime_proof=false, private_test=not_applicable_private_test. No real candidate or credential/signing probe ran during preparation. Root must inspect/freeze this exact private input before any genuine builder component; installed Git/GPG remain unqualified.

Independent retained-Host repair remains a distinct working change; its updated source plus finite models are not part of this C checkpoint. Prior make test-dream-c1 exit2 and selected IO-helper6pass/5fail remain preserved in [[reports/2026-11-06-dream-q2-independent-checks-corrected]] and target evidence. No original gate is waived or represented as green by this checkpoint. Full current original verify, genuine signed-publication positive/corrupt negative, qualification/resources/control/settlement and final landing/report are unfinished. No task_report, push or landing has occurred.
