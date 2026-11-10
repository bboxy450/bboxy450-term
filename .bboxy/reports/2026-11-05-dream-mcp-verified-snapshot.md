---
type: report
title: Q2 recovery and signature scratch custody checkpoint — qualification unfinished
author: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
created: 2026-11-06T15:10:59.507Z
task: '[[tasks/2026-11-05-dream-mcp-verified-snapshot.md]]'
commits:
- 5eb3f804b3036ac7408bd3dba31a543c7dfc72a7
- 4d1343236a5fcf76e3d66fcf08b5338edf9fbd2c
- ba6ef8b105d7abc89af58250e3d66349c9edce4e
- bacb52a6a6dabfa36a6c94450e3ad5e958d18a53
- 5c1190b907902533988348f4d3bc26ec39717902
- a0094e02705fc5b9a0c9c034d184d18c62fc8784
- 5c50020df8bb8d55702e01f63b4d1e91b65055c3
- 2f39650c0787d259ba9f2a92235795a6f7e6403b
verify:
  command: cargo test --offline --locked --release --jobs 2 --target-dir target --test dream_read signature_boundary_controls_refuse_before_helper_birth -- --test-threads=1 --nocapture
  exit_code: 0
id: 01M6RPKERV170TEPKTWWG6DHWM
updated: 2026-11-06T15:10:59.507Z
number: 93377961366012
created_by: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
updated_by: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
authorship:
  signer: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
  kind: seat
  public_key: p1xg5YnZmsPv/B3q4mjUXp1coPPED4IjwlMtDzJrDaE=
  process_generation: '1'
provenance:
  digest: sha256:7f7be4e6cf86ac7a2f146da0db6878536636ed7b489e5b2b33a22008c87de861
  frontmatter_digest: sha256:bfd7bdb8fc9d3bfbfc5946123cce00af028fbfaa0b9eeb414fa0eae565285106
  signer: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
  sig: sRtRej6ptYqjx2pftszYCXpEHoHGUXomiNm/cOdpWFYh4RIB1/T3ClZSCfzmFBjOsXnrFTwiNJFQwTHxYBW2Dg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149482
  event:
    id: 01M6RPKFHJW6TAJ1978YDMT4SM
    sequence: 149482
    hash: 5020573c3b851a1b7261e80d443f02950d62b6e75a01cd44855e39129a5288d7
---
Recovery checkpoint for [[tasks/2026-11-05-dream-mcp-verified-snapshot.md]]. This is **not the final Q2 report or a qualification/landing claim**. The task remains claimed and unfinished; no task_report, push, make land, installation or release was performed.

## Source and custody

Writing tree /Users/feral/mydev/bboxy450-harness-dream-read-q2, branch feat/dream-read-q2. Fetched origin/main a75eabfa and rebased the seven inherited signed changes, preserving upstream Python execution models and canonical landing workflow. Conflicts in Makefile and verification/{contracts,corpus,coverage}.toml were resolved as unions; TOML parsed and contract/corpus names remained unique. All eight branch commits report G signatures; git verify-commit 5eb3f804b3036ac7408bd3dba31a543c7dfc72a7 exited0 with key9B07E849480C68328CA20684844ECC0587DE7AB0, Boxy <bboxy450@gmail.com>.

Before rebase, copied ONLY generated .bboxy/runs/formal-native.json to target/dream-read-finish-recovery/formal-native-feefcc96.json (SHA256 c82a9e5aa7e99041af9288fa5422bd7100e776b61ff7be2ea305b2d04e70d66c), then restored that generated tracked file from its original HEAD. No source reset/stash, broad staging, forced KB addition or peer cleanup. Ten inherited untracked observer JSON files remain. The temporary Python bytecode produced by importing the canonical formal checker was removed by its exact owned path; no source files were removed. Final tracked worktree is clean.

New commit 5eb3f804b3036ac7408bd3dba31a543c7dfc72a7 fixes an actual GS2 custody defect: the old code could observe absence in a retained directory after the named output/t had been replaced. tests/fixtures/dream-read/proof-helper-probe.c::private_git_sig_gs2 now rechecks original held/named scratch identity before any unlink and after rehash; a changed scratch returns settlement_unknown, retains the original signature FD and quarantines. It does not claim an all-times filesystem theorem or that GPG opened this inode.

tests/fixtures/dream-read/signature-boundary-controls.c includes the canonical probe in a test-only translation unit. It exercises original empty/deleted-file settlement and normal/killed directory replacement with actual owned regular files, retains both original and replacement on refusal, and checks prebirth matcher/no-direct-verify-child/selected-case/no-rebind/umask/persisted-receipt/ledger-bound/debt APIs. These are API fixtures, not genuine Git/GPG or actual leader-reap observations. tests/dream_read.rs::signature_boundary_controls_refuse_before_helper_birth compiles/runs those controls and rejects a private source mutant removing both GS2 fences.

The finite GitSigTemp model adds one current-scratch boolean, a post-GS1/post-reap replacement action, refusal/quarantine guards and a named safety invariant; new BrokenHarnessDreamReadGitSigTempScratchReplacement omits the GS2 fence and must fail. Existing models/cases/caps remain. Registry, corpus, contracts and coverage name the new control and distinguish the API witness from pending genuine pair witnesses. Makefile and ApprovedRunner source lists now also bind c1_wire.rs, fingerprint.rs, entry.rs and the component fixture. ATTRIBUTION.md records the source/model/API contribution.

## Commands and actual exits

- git fetch origin main:0. First git rebase origin/main:1 with four conflicts; manual union resolution, TOML validation and GIT_EDITOR=true git rebase --continue:0. Rebased head before new fix4d1343236a5fcf76e3d66fcf08b5338edf9fbd2c.
- cargo test --offline --locked --release --jobs 2 --target-dir target --test dream_read signature_boundary_controls_refuse_before_helper_birth -- --test-threads=1 --nocapture:0. One passed, no ignored tests; observed original scratch settled, replaced normal/killed scratch quarantined, no foreign unlink; private omitted-fence mutant actually exited81 at the expected GS2 assertion. Log target/dream-read-finish-recovery/signature-boundary-controls-scratch.log SHA256 47aaaec9f83af1cb1e4fa61a6e6335352b49daa649dff58541208d1222aeeb69.
- cargo test --offline --locked --release --jobs 2 --target-dir target --test formal_models dream_read_git_signature_temp_binds_real_inode_and_both_selected_cases -- --test-threads=1 --nocapture:0. One passed, no ignored tests. Good model609 states; all nine named causal controls rejected. Log target/dream-read-finish-recovery/signature-model-scratch.log SHA256 6f83f0835b2c67cb96b826e6019c527c317000c4619bfc841b524a88076759f7.
- Narrow ten-model native/reference differential script:0. It calls the existing tests/formal/export.rs and scripts/formal.py, checks source pins, uses pinned upstream Java/TLC, and compares COMPLETE reachable states, labelled edges and named verdicts for good GitSigTemp plus all nine controls. Good:609states/716edges, Java graph/property exits0/0. ScratchReplacement:609states/720edges, Java graph/property exits0/12 expected safety. Every other control graph exit0/property exit12 with full equal graph sets. No timeout/cap/incomplete result accepted. This is only the signature family, not full repository parity. Exact compiler argv, checker/tool hashes, model/source hashes and every model result: target/dream-read-finish-recovery/signature-parity-3rlluy_z/evidence.json SHA256 246751c7399892e2b260b07f7324538c93760ad76376f22b18b659cc0fbb3a40.
- Direct current C fixture compile (-O2 -std=c11 -Wall -Wextra -Werror) and API run:0/0, retained target/dream-read-finish-recovery/signature-scratch-4nt8o49h/evidence.json with compiler/source/run bindings.
- rustfmt --edition2024 --config skip_children=true on changed Rust tests/formal files:0; git diff --check:0; TOML parsing/unique inventory checks:0; source commit and signature verification:0.

Current canonical probe SHA256 ec9c37602552b0a6d775ed75e673d194fa47e988e5481dfb3caa5a4b3958b015; component fixture2b235f42d9855840907a516957417027c03a0b0db9157378116de211783970c1; model107b945b71737573f6d5de38901b5ea74cf536402911fd5c4c685378aab14730. Narrow differential evidence contains remaining exact source/checker/JAR/Java hashes. It is not the Root-frozen source/executable/compiler/schema tuple for builder consumption.

## Acceptance disposition and blocker

1. Schema/goldens/hash conformance and unchanged inspect-c1: inherited implementations preserved, but no current full wire/C1 verification claim from this checkpoint.
2. Genuine signed publication/corruption/currentness/trust proof: UNMET. resolve.rs still correctly refuses an absent approved lifetime profile. Installed Xcode Git/Homebrew GPG remain unqualified; the private probe has no complete admitted real-pair path.
3. Bounded selected immutable source/page/result custody: inherited implementation retained; this checkpoint improves signature scratch custody only. Full selected-proof/currentness/descriptor witnesses remain required.
4. Parent+child multiMiB/overflow/four-job/stall/control/heap/reap: UNMET as final joined qualified evidence. No measurement threshold, allocation or runtime law changed. Current API test creates no Git/GPG helper.
5. Formal: changed signature family has full native/TLC graph+verdict parity and actual API/source-mutant controls; FULL current make test-formal-parity remains pending along with other unchanged process/model gates.
6. No Recall/adoption/trust claim or semantic change: preserved; no runtime approval or available lesson was minted.

Decision93377961366007 requires the existing builder to provide the sole signed tests/fixtures/dream-proof-tools/closure-boundary.schema.json and closure-boundary-vectors.json before consumer implementation. Neither exists in this assigned tree. The existing C decoder still has the superseded load-string/platform shape and its prebirth fail-closed guards remain. No competing codec, guessed closure or arbitrary candidate approval was introduced. task_status for tasks/2026-11-05-dream-proof-tools-provisioner.md rejected not-assigned-to-caller (op01M6RPCHCC2NXD1PY1J95E0WJ6). One blocker message01M6RPFK4DQ8H0025P101MZ2W8 requests the signed commit/path/hash artifact and authorized current builder routing from Root.

Pending after that handoff: consume/reconcile canonical C codec, finish exact bound semantics and private engine/result admission, freeze coherent source/executable/compiler/schema tuple through Root, obtain genuine builder component and independent approved-profile review, then actual qualified publication/corrupt/four-job/control/foreground/heap/reap gates. Finally run unchanged make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build and current make land, verify origin/main containment, and create the final landed report. This report does not substitute for any of those. Earlier inherited C1 and qualification failures remain historical failures; none was rerun for lucky timing or relabelled green.
