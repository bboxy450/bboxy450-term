---
type: report
title: 'Dream proof tools B: exact commands, temporary matcher and corpus bytes'
author: seat:01M6Q0FHRZV6MQA8PBPXG408N7
created: 2026-11-06T03:13:29.799Z
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
commits:
- ba00bf6b711ab17c26a966503f35d1f2b44fbac2
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
supersedes: '[[reports/2026-11-05-dream-proof-tools-signature-corpus-b]]'
derived_from:
- '[[decisions/d93377961365960-dream-launch-entry-and-signature-boundaries]]'
id: 01M6QDHNA8MDK7EQCGF9XG2P16
updated: 2026-11-06T03:13:29.799Z
number: 93377961365978
created_by: seat:01M6Q0FHRZV6MQA8PBPXG408N7
updated_by: seat:01M6Q0FHRZV6MQA8PBPXG408N7
authorship:
  signer: seat:01M6Q0FHRZV6MQA8PBPXG408N7
  kind: seat
  public_key: PcKFbgDesoAlYaICRDceGYv79hGHd1L0ZA+qVT0MKXU=
  process_generation: '1'
provenance:
  digest: sha256:c562371a55bbcaaea405a2a5c756647979137b921a846d13f759519de98daf22
  frontmatter_digest: sha256:9833eca8dabf65d60851df726fbff44504e13c932fbc5785bd351d55e4e5dae5
  signer: seat:01M6Q0FHRZV6MQA8PBPXG408N7
  sig: MWpSsJ0ZyQdgkjD9nAwP08DELvw7DDGj/nPip28I9m7dwf9jZDGikYDrEfAmaItfudPn/BFSc0eASdEgAIAuAg==
  log: .bboxy/log/2026-11-06.jsonl#149222
---
Builder/corpus section B is implemented in signed commit ba00bf6b711ab17c26a966503f35d1f2b44fbac2 (actual git signature status G, Boxy <bboxy450@gmail.com>). This is a partial implementation handoff, not task completion, candidate regeneration, source/build approval or a real verifier witness.

Authority is the current assigned task [[tasks/2026-11-05-dream-proof-tools-provisioner]], updated2026-11-06T02:57:38.818Z/body SHAde46c83ba695e108cc490612a258b59f79362e26659192b40941f99c3c3483a1, explicitly authorizing ONLY builder/corpus B of [[decisions/d93377961365960-dream-launch-entry-and-signature-boundaries]]. B snapshot a130538ffa3ff97e8336ae2100a692afc6b33cbd17792985e84194b9b1a37c32 is retained by that decision; complete current combined readback eb0b4af386b26d8f8bdc1d58d6632cb4478ff3ffc1970fc696d88730b83dc20f summarizes the same B. A/C are separate. Canonical builder ICD fetch still returns98a3023e with the old operand; the signed task amendment is the implementation authority for this correction. No new or moving Q2 probe was read or consumed.

Changes and mechanism:
- scripts/dream-proof-tools.py command_profile now emits exactly {git_signature_temp} at verify_child argv[3]. This is a matcher only; it is never expanded or directly launched. The existing five arguments plus pinned GPG image still describe the actual six-element child, with no direct gpg_prefix added. All other fifteen forms/prefixes stay unchanged.
- Git creates OWN TMPDIR/.git_vtag_tmp[A-Za-z0-9]{6}, writes exactly the signature bytes extracted from the selected commit, closes before child birth, and deletes after reap. No fixture path is substituted into actual argv. Q2 alone owns GS1/GS2 actual argv/temp inode/content/exclusivity/settlement and their process controls.
- git_sha1_commit_parts is an independent byte-format check inspired by the pinned Git source. It unfolds one continuation space, preserves every LF/raw byte, selects SHA-1 gpgsig, excludes alternate signature headers from payload, and leaves the message remainder untouched. It is NOT OpenPGP verification or a substitute Git implementation.
- corpus_signature_bindings checks actual object ID, selected-case Ref path/length/hash/bytes, extracted payload and distinct actual valid/corrupt signature bytes. read_corpus applies these checks before fixture assembly effects.
- capture_review refuses stale/substituted Commands before source reads. bind_current_candidate checks current Commands hash before candidate reuse/binding. The corrected hash therefore invalidates old pending tuples, alongside existing source/fixture currentness checks.
- Source review now includes gpg-interface fixed verify arguments, tempfile creation/close/delete/mode, wrapper exact write and O_EXCL/suffix generation. git_child has15 real spans (within16) and an explicit Q2-owned GS1/GS2/exclusive-scratch premise. review_findings checks all eight findings against the retained authenticated/patched snapshot; actual Commands hashing feeds the later full Review.bindings.
- The existing one recipe owner has a focused check --signature-corpus-only path. Its early branch runs only five B source/corpus tests and read/hash evidence. No Git/GPG/probe/build/native helper, source fetch or packing occurs through this branch.
- tests/formal/dream_proof_tools.rs and the three canonical verification inventories add the builder-only SignatureCorpus model, positive nonvacuity and seven causal controls: matcher substitution, extracted byte mismatch, wrong case, stale Commands, stale case, expansion, direct child launch. There are no Q2 GS1/GS2/temp/process/controller states in this model.
- Fixture documentation and ATTRIBUTION distinguish copied Git corpus bytes from independently inspired byte checks and preserve original upstream licenses. Corpus/commit/signature/payload bytes were NOT regenerated or changed.

Exact source/corpus evidence:
Read set is the Root-specified retained authenticated/patched R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6Q80DBV75S95DWFHZWDVE2F/src/git-2.54.0. Each file is checked against both its source-snapshot entry and the frozen digest:
| Source | Length / SHA256 | Mechanism |
|---|---|---|
| commit.c |53014 / d02e4488fc271f1421dc7e825fd6d78244f4db1b47716927bc5dd8e7bb6c4583|1136–1139 SHA-1 header;1186–1229 LF-inclusive extraction/alternate exclusion;1333–1339 lengths onward|
| gpg-interface.c |31997 /191dd45b56d9992f3862f4696693c77019185ae785402686d483823468c53aed|52–53 verify args;349–386 create/write/close/argv/wait/delete|
| tempfile.c |9578 /a34ef38d1981b2b269436288685a4af3453d102bac2f29fc2449b8c98b6279d2|186–202 TMPDIR creation;298–323 close;360–374 delete|
| tempfile.h |10160 /20c6e57636813f056554fdbb3a20652257aac3d93fd7dd367ec7b785f6f53a1a|187–189 mode0600|
| wrapper.c |19196 /788c07b315a6e98bfcaf78291b83b17e52579f17b604a94db5c149c017a83b04|250–323 exact writes/errors;438–481 six alphanumerics and O_CREAT/O_EXCL/O_RDWR|
| run-command.c |50187 /03bf7f114875b964cb9397893b3ddceafd4d71f51b908a9c02ae6ca7e2a4d67f|fixed real child and one attempt consumed before effect|

| Case | Commit object | Extracted signature bytes/SHA256 |
|---|---|---|
| valid |cbceced299d68bdfce3b95f182698f280f1da743|228 /d716c2d0807ca1339c774486d55d31768fa9f953df90872f2ceef4cb4568c792|
| corrupt |0cf6ea6f9b0da82362299e1ccad64e4a5d028de3|228 /cd9777c930d18c7b5b3603ad58a6f63bd64fca702d87bc4e518fca5c1cf0a17b|

Both extracted signatures equal the corresponding original .asc Ref exactly: seven LF bytes, zero CR, blank armor line and final LF preserved. Both extracted payloads equal valid-payload.txt,218 bytes/SHA58c4ecb71bef6681cc31d9513ac85bbbd130d4d15438e5c57802a983f6091fb3. Alternate gpgsig-sha256 is not concatenated or selected for SHA-1. Wrong-case Ref swaps fail even after an attacker makes each swapped file's own length/checksum self-consistent; malformed/trimmed LF, payload substitution, message-as-header interpretation and alternate-header selection are covered. No cryptographic success/rejection is inferred. Both cases still require one actual later verifier through canonical Q2.

Hashes:
- Old Commands f1ae4c60cb6cf1744e9cbb9b2979ca25214dc2363f7c68ddef2ff9cc00fa8361.
- Corrected Commands8a5b82ec847ac70d98ab947fe0c732a8def4fdc7adfc690791bda45b7db7e85d.
- Updated source review template0b8ded57980ee71ba29ab4ed7afdaa8ffafc268befbf769ae72d392cfe36d477.
- Actual source-bound eight findings54dc70291bbfa3d85601a40fe7f6af27ebb1a263a241dfd557387f1ce5b4c912; its Commands binding is the corrected hash above.
- Full Review projection hash is UNKNOWN/null: other real projections, including resolved C closure, do not exist. No fictional bindings are filled.
- Unchanged corpus lock7717ad1de31391ead47a498a32c732066e0677fbe981d52fe8af613a95b6e8e0.

Actual unattended checks:
| Command | Actual result |
|---|---|
| python3 -B -I scripts/dream-proof-tools.py check --signature-corpus-only |exit0; five focused tests, pinned source/equivalence findings retained|
| CARGO_BUILD_JOBS=2 /usr/bin/nice -n 10 cargo test --offline --locked --jobs 2 --test formal_models dream_proof_tools_signature_corpus -- --nocapture |exit0; two tests explore good model and all seven named broken controls|
| make test-licenses |exit0; source/license gate plus10 controls|
| git diff --check |exit0|
| git log -1 --format='%H %G? %an <%ae> %s' |exit0; ba00bf6b… G, Boxy identity|

D=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools:
- runs/01M6QD93H2M0SA4V34ZG6FVVBG/signature-corpus.json length2930/SHAd8eeca37d7568b9b627dfb6ffe0e6ed2061090fcd49e56f8355b2d741f0af779.
- That run's recipe.py is byte-identical to current script, SHA f5e0d0ba7f5063d9cf284288c6c425a43380d736a293384483b17aeabca260d0.
- signature-B-final-source-corpus.log length1193/SHA8301264d6e9d3de3150d032a213134425e0b41e88ce432d58d2d18762e3c4409.
- signature-B-formal-native.log length560/SHA9b9f6172d0ea2cf15491e3a6166149fba64473c49fa1ce394bb7375f78fc71ea.
- signature-B-licenses.log length413/SHA12c6c47f8b52a71c8f2e0eee23ae5daa8c29c4e75c4ccc42dc844c526bd2f411.

The inventory now declares315 variants after the eight B variants. Full native/Java parity was NOT rerun under this amendment. Previous307-model receipt4ede8b… is bound to78391 and does not certify ba00bf6b; current-source full parity/receipt must be refreshed at the authorized final corrected-source gate. No new full build, packing loop, real pair/probe/qualification/timing retry or candidate receipt occurred. Existing task verify remains Failed2 from its retained full-run baseline and was NOT claimed rerun against ba00bf6b.

Worktree contains only the canonical generated M .bboxy/runs/formal-native.json after the signed source commit; it is not staged/cleaned. No source key, tool binary, archive, patch, library, OS assumption, resource limit or existing failed-prefix identity was changed. No Q2 source/decoder/controller edit, cross-owner write, public DTO/version, new dependency/crate, push/merge/install or runtime grant occurred.

B's source/corpus work is complete and reviewable. It does not resolve C's shared closure contract or supply Root's coherent frozen Q2 source/executable/compiler pin. Those remain the real candidate regeneration/component prerequisites. All1ms/native2ms/25ms/3ms/9x/256/member/thread/30s/four-job/heap/control/foreground/bootstrap/reap and128KiB/48KiB/32KiB limits stay unchanged. Overall task_report remains deferred because the complete real component and final corrected-source gates have not passed.
