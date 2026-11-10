---
type: report
title: Phase A explicit verifier target and one private shim-bound Git witness
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T16:29:22.079Z
commits:
- 33c7a896bedd80968a6a1dafcccb64eec0fbe605
verify:
  command: /usr/bin/make -j2 CC=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/clang 'CFLAGS=-O2 -g0 -isysroot /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk -DDREAM_PROOF_GPG='"'"'"/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/prepared-witness/dream-read-c-preparation/compile-t01d4h8x/dream-exec-shim"'"'"'' 'LDFLAGS=-isysroot /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk -Wl,-map,/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-witness-20261109/logs/git.link-map' BASIC_LDFLAGS= NO_PTHREADS=YesPlease NO_CURL=YesPlease NO_EXPAT=YesPlease NO_OPENSSL=YesPlease NO_GETTEXT=YesPlease NO_TCLTK=YesPlease NO_PERL=YesPlease NO_PYTHON=YesPlease USE_LIBPCRE= NO_FINK=YesPlease NO_DARWIN_PORTS=YesPlease NO_APPLE_COMMON_CRYPTO=YesPlease NO_HOMEBREW=YesPlease prefix=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-witness-20261109 git
  exit_code: 0
id: 01M70J93CR2DBC0DQS7XBEAY3X
updated: 2026-11-09T16:29:22.079Z
number: 93377961366470
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:575479105e393deedf25b84bbfe2de40b0ef350d05a43d8e743dfa5e98a7b2a8
  frontmatter_digest: sha256:8584b1eceacb5612f1369a6411e80c1ddb37d6643c3bec60a41c76738b18c887
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: sZ7l9tkqfuiI94LxJsbt+Jwt8p74vINXQsRXNoEAWhzMGcScP+v53NmLtiAyYbaEIIcESGLaCfeXgdFqOBmhCQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151055
  event:
    id: 01M70J94X9F96WFGARDQDZP3B5
    sequence: 151055
    hash: f8ac8ef7b1df6346e13651e238b2082e09ad04d8572962a93380778c14c9854e
---
Phase A is COMPLETE. Added the explicit DREAM_PROOF_GPG_TARGET recipe parameter with the existing candidate/bin/gpg default, built exactly one private witness Git against the supplied shim path, and sent the witness path/SHA256 plus patch/recipe hashes and exact define evidence to Dream Exec Shim. No witness execution, native probe, qualification, receipt or production-candidate change occurred. runtime_approved=false.

Task [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], ruling "git's allowed child becomes the exec shim", authorizes this build-only Phase A. Phase B remains deferred to INPUT70. This does not change INPUT69's failed qualification/actual canonical exit2 or grant runtime approval.

## Signed implementation

Own tree /Users/feral/mydev/bboxy450-harness-dream-proof-tools. Signed code commit33c7a896bedd80968a6a1dafcccb64eec0fbe605 modifies only scripts/dream-proof-tools.py:30insertions/2deletions. git verify-commit exited0, Good Boxy key9B07E849480C68328CA20684844ECC0587DE7AB0; final git diff --check0 and git status --porcelain0/empty. Commit carries Report: [[reports/2026-11-09-dream-proof-tools-shim-phase-a.md]].

- git_cflags at:843–853 resolves the default to exactly the prior candidate/bin/gpg string. Explicit overrides must be bounded, absolute, normalized, literal-safe paths; make/shell/C metacharacters and lexical aliases are refused before any build command/write.
- build_git at:856/860 accepts keyword-only DREAM_PROOF_GPG_TARGET=None and passes it to that flag constructor. Its existing compile_sources caller remains unchanged. Ambient environment cannot select the target.
- Test:3886–3899 proves default byte equality, ambient-variable isolation, exact explicit substitution and rejection of unsafe/nonabsolute/nonnormalized/oversized targets.
- The Git patch and its other spawn/thread guards are unchanged. This is a build-literal parameter; no ownership, lifecycle, concurrency or model action changed. Specialized unit/build checks apply; no new stateful contract was introduced.

Unit command "python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target" actually exited0,69tests. Log SHA256aeaff7d69aae24a654cd2c99e9ae625d0f642f3f0ca07a30ca985e817ef88aac. No full qualification/formal/native smoke verdict is claimed by those units.

## One private build and source provenance

W=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-witness-20261109.
E=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-phase-a-20261109.

The requested exact shim path arrived in executor message01M70HZAZQPMJPQ8W7VD1ZV106:
    /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/prepared-witness/dream-read-c-preparation/compile-t01d4h8x/dream-exec-shim
Its SHA256eea840970ddc82f6461081175beda63e15843e7f06efb1bfb3683742b5bfb4af, mode0500 and owned regular/single-link identity were checked before and after building. Sibling preparation-ff0a7b1747a287e98f6171210b4449dcf95ee74a485056912baa9e698cc62a7c.json is526183bytes/mode0400 with matching SHA256. Source/preparation commitb665598781ff6edfdf9108360b1561376ab95881 independently verifies Good Boxy. These bind the supplied witness input, not an INPUT70 authority or runtime grant.

Fresh owned source trees were extracted from the seven existing pinned caches only after actual independent public-only signature authentication. W/authenticated.json has7auth_exit0 rows; all six unchanged profile patches passed exact pre/postimage verification. W/source-snapshot.json hashes the complete fresh source table; snapshot SHA256e05af70b4126db817145ebc5149409330be3b42a4a72ad3733c54d71ab14c78d. No predecessor source tree was built or mutated.

Only build_git was called, with W as both fresh build root and private witness prefix, and the explicit supplied target. compile_sources/build_new/qualify_candidate were not called. W is outside the production candidates directory. Disk gates exceeded8GiB; the preparation sample was24585400KiB. E/witness-launch-binding.json retains the fresh pre-make disk sample, source signature, exact shim/receipt identities and unchanged code state.

Build commands000–005 patched the fresh sources,006–007 located compiler/SDK,008 was the ONLY make -j2 ... git invocation,009–010 were otool metadata reads. All11 commands exited0. Existing command() applied nice10 and bounded explicit environment; no Homebrew library discovery was accepted. No retry, cleanup, overwrite or second witness build occurred.

Compiler:
    /Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/clang
Compiler SHA2561590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9. SDK path/settings hash and build argv/env are retained in W/source-ready.json and command records.

## Delivered artifact and exact define evidence

Private witness:
    /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-witness-20261109/bin/git
3766096bytes, mode0700, inode189483646, SHA2561955c95c2daa1ea5ebb8914fd32d0efe2f89d8fec6c64b1982fded31a1f0da81.

Git patch SHA256, unchanged:
    a48b33776ae5a88308e096ea62640487706e360c374ec7eaf64d50adf47818b8
Recipe SHA256:
    ad3fd381408f358edf6417bbaa2988c55d50d30d8d8ddf4ddd03c2727a96d4fc

W/logs/008.command.json records the exact CFLAGS argument including:
    -DDREAM_PROOF_GPG='"/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/prepared-witness/dream-read-c-preparation/compile-t01d4h8x/dream-exec-shim"'
W/src/git-2.54.0/GIT-CFLAGS969bytes SHA2564cb9fe77ad64491677de89e401e30d7dd0df3c9aac6e4486e2287f2bd3601467 independently retains that same define. W/logs/008.exit.json records actual make exit0. E/build-verification.json contains its exact shell-quoted argv, cwd and log locators; the report verify.command is that recorded invocation, not a rerun.

The supplied absolute shim literal is present with NUL termination in linked Git; the unused W/bin/gpg default literal is absent. This and the unchanged run-command.c patch establish the build binding; they are not a runtime execution witness. I did not run the produced Git or shim.

## Handoff and preserved limits

Result DM01M70J4DM1MW037N9QAQM4THVM was sent to seat01M70FJGXV7W40W80200Z0TDN5 replying to its supplied-path message. It includes the Git path/hash, shim path, patch/recipe/compiler hashes, exact define evidence and private-build-only limits. Receipt evidence is native-queued (native_id01a1c122-36d7-7fe0-98e5-f7b622fcc0f6); model consumption is unverified. No ACK wait or repeat send is needed.

W/witness-build-complete.json binds the executable, shim, receipt, compiler, recipe, exact flags and completed build. E/handoff.json12778bytes SHA256257cd784d4d1581bd0ba84e65780961fd4c62584d415e77dfd7e227f04d47bed binds real unit/auth/build exits, signed clean code checks, exact compiler records and raw evidence.
E/build-verification.json1526bytes SHA256b4138c61e66b647a99f654d4acc4d260a3b02e4e214505d45744d601b3bd0dfb contains the precise make verification command.

The source-lock fixture and all26 historical component ledgers retain their pre-work hash/length/device/inode. All old candidate/run/source evidence remains intact. No source-lock pin, candidate prefix, qualification/report wire, receipt, basis ceiling, runtime law or approved flag changed. Existing source headers and ATTRIBUTION.md credit/license policy remain attached.

Phase A's parameter, single build, verification and explicitly authorized executor handoff are complete. The full canonical command was not rerun because it would invoke qualification outside this phase. Its last actual exit2 remains historical failed evidence. Phase B's frozen libexec installation, source-closure review and qualification wait for the INPUT70 freeze.
