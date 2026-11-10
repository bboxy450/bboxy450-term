---
type: report
title: 'Dream proof tools source7 build: blocked by actual platform graph bound'
author: seat:01M6Q0FHRZV6MQA8PBPXG408N7
created: 2026-11-06T01:48:23.811Z
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
commits:
- 78391c4c82e94fc9f10bad2998c700c6a9278bb7
- 9e48c55ddc6e829d148f2ea096191eaae2139ac9
- c32f6f83f3d04c97f6a0f25f18cf14fcc1523a51
- 5fe62b9c70c2ff8079f67608dd1e5c5577c7cdcd
- 1b166a3ed93ce32099c5a79bf23890f56ee2b7bd
- 2fdb65482cb7faa842765e563428d28b2be91fc7
- 6eac12cf7c33bbc4e06fa797aa88aa76fac1a053
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
supersedes: '[[reports/2026-11-05-dream-proof-tools-provisioner]]'
id: 01M6Q8NV63BA3ZMNBDA3BMRS1T
updated: 2026-11-06T01:48:23.811Z
number: 93377961365933
created_by: seat:01M6Q0FHRZV6MQA8PBPXG408N7
updated_by: seat:01M6Q0FHRZV6MQA8PBPXG408N7
authorship:
  signer: seat:01M6Q0FHRZV6MQA8PBPXG408N7
  kind: seat
  public_key: PcKFbgDesoAlYaICRDceGYv79hGHd1L0ZA+qVT0MKXU=
  process_generation: '1'
provenance:
  digest: sha256:9e1e6f4533c3d656ce956142407363c705cb112cb8364c50d839390a6830e035
  frontmatter_digest: sha256:b4b164034ac329deaf7c04dd7183476b6b25da78c04d760d134ea075c1f1620c
  signer: seat:01M6Q0FHRZV6MQA8PBPXG408N7
  sig: cbr1F5zElAm27QYjFSEqyaZXv9BO+iSnoF9nvtG2X2idBV7HWf4aWl4NazyTEzdnNB2BrGlsLv+H4VFln/9+BQ==
  log: .bboxy/log/2026-11-06.jsonl#148573
---
This is a BLOCKED, UNAPPROVED implementation handoff, not successful qualification or task completion. It supersedes the earlier six-source blocker report. The real seven-source Git/GPG pair now builds, but the actual recursive Darwin load graph exceeds the frozen private schema's 128-edge bound. No edge was dropped and no bound was raised. The task also still pins the old canonical C probe, whose normal Git mode refuses81; Root has not supplied a changed source/executable consumption pin.

Task [[tasks/2026-11-05-dream-proof-tools-provisioner]], executor seat01M6Q0FHRZV6MQA8PBPXG408N7. Own tree /Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools, base d06e696f99472ed754e84353baf11a2974589d6d. Last authorized task update read2026-11-06T00:42:09.080Z. Applicable complete ICD digest98a3023e2a24233959de04bb573a73274ff9980f4da80eb91bb4a89cd1fcaa7e includes the source7 and shared typed pre-test basis amendments. All native/application/control/cleanup, 2ms CPU/25ms wall/3ms inclusive, 9x debt, 256 attempts, member/thread, 30s/four-job, foreground/heap/bootstrap/reap laws remain unchanged.

Signed commits, all actual git log %G? = G, author Boxy <bboxy450@gmail.com>:
- 6eac12cf7c33bbc4e06fa797aa88aa76fac1a053 — initial authenticated provisioning and finite models.
- 2fdb65482cb7faa842765e563428d28b2be91fc7 — exact LF-based zero-context patches.
- 1b166a3ed93ce32099c5a79bf23890f56ee2b7bd — authenticated Libksba and private build custody.
- 5fe62b9c70c2ff8079f67608dd1e5c5577c7cdcd — fixed system-image hardlinks distinguished from strict private refs.
- c32f6f83f3d04c97f6a0f25f18cf14fcc1523a51 — actual link/platform provenance, typed producer, presigned corpus and source guards.
- 9e48c55ddc6e829d148f2ea096191eaae2139ac9 — repaired errno header after retained compiler failure.
- 78391c4c82e94fc9f10bad2998c700c6a9278bb7 — actual native image mapping/UUID binding and compiled entry mutants.

Only allowed source paths were committed. git status --short contains only M .bboxy/runs/formal-native.json, a canonical formal-target generated side effect, deliberately not staged or cleaned. No push, merge, release, global install/config, Cargo.lock, new dependency/crate, live Q2/worker/IR/wire/DTO edit or peer cleanup occurred. Native reports remain local host records, outside source Git commits.

Actual verification:
| Command | Actual exit / evidence |
|---|---|
| make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses | 2 at build closure refusal; final current-source run. Later chain stages did not run in this command. |
| python3 -B -I scripts/dream-proof-tools.py unit | 0; 16 tests, platform-fix-unit.log |
| python3 -B -I scripts/dream-proof-tools.py check --source-only | 0; authenticated full snapshot and three compiled entry mutants, entry-mutant-source-check-final.log |
| make test-dream-proof-tools | 2; its 16 unit tests pass, old canonical native/18-control results are verified/reused, then deliberate missing real-pair qualification refusal |
| make test-formal-parity | 0; 307 models, 79 contracts, 1 crate; complete reachable states/labeled edges and property verdicts agree with pinned Java TLC |
| make test-licenses | 0; source/license gate plus 10 negative controls |
| make check-formal-evidence | 0; current source/model/tool receipt validated |
| git diff --check | 0 |

The Make commands ran unattended. Final build/formal/test invocations used nice10 and CARGO_BUILD_JOBS=2; each upstream make gets exactly two niced jobs, one build/test at a time. No timing gate was repeated until lucky. Only fetch can use the fixed-source HTTPS paths.

Private evidence root D=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools. Log hashes, exact bytes:
| D-relative file | Length / SHA256 |
|---|---|
| current-exact-verify.log | 16170 / 25ee8b854f0f9d49aa309a3b609b1af17a75367fe23822681be7e8508d120784 |
| current-component-gate.log | 3489 / 76cf7d43fdf045f202ba0f2ade0d6b2c48d0d79a159ad8abc47bb2d5a7ed1db2 |
| current-formal-parity.log | 31454 / bc5178641e7efa0c8927b32852ce59d23753e2c1a6dcf59c0dfb789b335e84b8 |
| current-licenses.log | 413 / f07e5597d5e119418a576f6cd2bf72970a9673533ca53d8d9e639cfa7feb6aba |
| current-formal-currentness.log | 511 / e3e304de4ad92463785385ea27458b7b6c0fe044d63fc43c4364c9e41a547a5f |
| entry-mutant-source-check-final.log | 1510 / 75af343cec61d03ff04de08076072bad74d32ac7bf168297b40d2fe0a50c6141 |
| current-private-build-evidence.json | 65294 / 39b4fb883c2325c99a71ffff41856d9276c002105796cf6d8aedb607af5526d4 |

current-private-build-evidence.json records actual build identity, intent, compiler/platform, all seven authentication records, static closure, tool lengths/hashes/inodes/modes, and every command exit with stdout/stderr hashes. Original raw files remain at their recorded locations; this summary is not runtime authority.

Authentication and sources:
scripts/dream-proof-tools.py:233 authenticates BEFORE extraction. Exact lock sources.lock.json length7175/SHA4e05da4cb7c611f9f5dc21062d4c1f87fab14f52cc24918a1a3137e1f7d83947 preserves all original six pins/order and appends Libksba1.8.1 archivec2f84393011827219ae117131dba8e7684c2bed0961eed11b0642c2acba440b5/signatureaf99f325f81eb395d740bf48dc8e89903b141c10f69244e46e4caa1caec0577e. Final fetch and final build each actually verified all seven source signatures using the exact authorized installed verifier21cb9adcad96b7be404014ff6e9fb837160a2135455c217dec9efe196cf0661a. Git verification covers the uncompressed tar; the other six signatures cover archive bytes. Git actual signer is E1F036B1FEE7221FC778ECEFB0B5E88696AFE6CB bound to primary96E07AF25771955980DAD10020D04E5A713660A7; GPG/library actual retained signer is an allowed Werner primary6DAA6E64A76D2840571B4902528897B826403ADA. Actual GPG VALIDSIG, time/expiry/revocation/error checks, pinned certificates, import-export public conversion, stdout/stderr and true exits remain in each fresh owned public-home. No issuer-only or UID acceptance, keyserver/automatic retrieval, downloaded verifier, secret import, operator key export/key generation or fixture signing was used. Independent bootstrap verifier, compiler/linker/SDK and OS fidelity remain explicit trusted premises, not Dream runtime lifetime approval.

Archive size32MiB, expansion256MiB, finite table checks and safe extraction reject traversal/special/hardlinks; safe Git source symlinks are made after regular writes. Exact patch pre/postimages and full extracted-source membership/mode hashes are checked before compile. Source cache reuse hashes bytes and freshly authenticates; corruption quarantines the exact owned cache entry. patches.lock.json length2975/SHA69a92d768edd0298340240b27bc9d47d7c02704c6009647341535e8b2970cb71 pins six actual profile patches.

Real build, not a mock:
Final run D/runs/01M6Q80DBV75S95DWFHZWDVE2F and prefix D/candidates/01M6Q80DBV75S95DWFHZWDVE2F retain the original build/incarnation/intent and prefix dev16777234/ino178609123/mode0700.
- bin/git length3766096/SHA4b5bd9ea58bf816daff747d361e58ebf8dc65cfbf98f66116769554159a2e345, ino178627998, mode0700; real --version output2.54.0 at logs/042.stdout.
- bin/gpg length2405760/SHA03e7ec6e8cfe564a8645ac8bd096dee8e5a3736519b202e64fe6742d13fe6079, ino178631911, mode0700; compiled real authenticated2.5.24 source/config, no standalone real-pair verification was substituted for the missing canonical mode.
- final logs/044 is real GnuPG configure exit0 with the required private KSBA prefix; logs/045 is full upstream make exit0. Agent remains a required upstream scratch build component; no agent executable is staged.
- Dependency order error→gcrypt/assuan/npth/ksba→GnuPG; all five private libraries are static. Source-supported options are checked against actual --help, unknown-option warnings refuse. Generated config excludes card, readline, LDAP, TOFU, libgcrypt HMAC constructor and jitterentropy. Actual compiler uses upstream portable libgcrypt implementation because the original Xcode assembler failed; public algorithms/compressed input support remain.
- Documented NO_HOMEBREW disables the actual unwanted gettext include discovery found in the prior build. NO_PTHREADS already disables Git's Darwin fsmonitor backend; BASIC_LDFLAGS removes its otherwise unconditional unused CoreServices framework. Final GIT-CFLAGS/GIT-LDFLAGS are checked for Homebrew/usr-local/CoreServices contamination.
- Git direct loads are system zlib/iconv/libSystem; GPG additionally system bzip2. Actual maps and ar member indexes stage9 private archives: libgit, libgcrypt, libgpg-error, libassuan, libnpth and four GnuPG local archives. Libksba is required for compile/build but absent from both actual linked runtime maps; no ksba-io-support object is selected.

Compiler SHA1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9; linker SHA374d9f0713bce2c8a9e1e82bde51615d5433516a10d99b63107eb6ee70542e4c; Xcode SDK27.0 settings SHA7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb. Actual Darwin27.0.0/raw arm64 maps ONLY to canonical aarch64; OS build26A434; actual shared-cache UUIDc284a291b68b3d0da1a03f866f87efdc. Compiler reported arm64-apple-darwin27.0.0. Tool/SDK/generated config/link-map bytes and command/env hashes are retained, with fixed developer-tool identities.

Guard and source review:
The six credited upstream patches retain GPL/LGPL/origin/policy notices. Git refuses async before fork/thread, consumes exactly one real implicit verify-child permit before effect, requires the absolute candidate GPG/exact child arguments, denies generic shell/gitcmd/env/dir and ENOEXEC shell fallback. GPG defaults autostart/lookup off, rejects unsupported commands/config before helper initialization, rejects secret tags immediately before skip/copy, and avoids agent lookup only under the explicit real no-secret-home premise. Libgpg-error/assuan/nPth spawn/thread seams refuse before effects. A separate real source audit found libgcrypt ambient hwf.deny/FIPS and syslog paths; its patch refuses unbound config before reads and syslog before socket/service, using raw stderr/_exit78 rather than manufacturing crypto success. Generic error/cleanup/initialization/platform assumptions remain conditional.

review.lock.json length12667/SHAd2eaa9a60f0f87fae01dc0214c20e8c7606808432c26a6061826c96afe74c2c3 contains eight required findings, all seven sources, all sixteen65126 forms, all five phases, concrete postimage spans and compact conditional rationales. capture_review at1284 fills hashes from the retained authenticated/patched snapshot and actual patch list, checks coverage and computes five projection hashes. It is NOT an established lifetime theorem. No grep/symbol/version/sample/census claim establishes all unseen native work.

Actual closure blocker:
emit_closure at949 parses real thin ARM64 tools, map-selected archives, actual load commands/rpaths/UUIDs and recursive platform edges. SDK-declared own-task Mach/dyld APIs bind cache/actual loaded images; fixed dyld_info corroborates raw platform metadata. Universal disk images (libsystem_kernel includes x86/arm64/arm64e slices) are matched by actual loaded UUID; cache construction's rewritten __TEXT address is taken from its real mapping, never a first disk slice or architecture alias. Headers/raw outputs/actual per-image metadata remain private. This is developer offline provenance, not a second runtime controller or a census completeness claim.

Final build explicitly refuses the129th edge, libdyld.dylib→libunwind.dylib, with terminal.outcome=unsupported/effect_unknown=false. The earlier metadata diagnostic over the retained real pair found at least458 actual load edges plus9 static edges,49 known platform images/9 static nodes/2 roots; libobjc-env remained unexpanded, so this is a LOWER BOUND, not a complete graph claim. D/runs/01M6Q76F3VZQAXQPYTGXNC2V3S/closure-bound-analysis.json SHA23f4f6557bb80f8ee13a494b4e34889eb9c40caab2d37b8643de1fd3812987fb labels that offline analysis as unqualified and retains every observed edge. Production bounds stayed128, no truncation. No third same-gate build is attempted.

Typed pre-test producer and publication:
prepare_basis at1317 constructs real Build/Auth/Patch/compiler/linker/SDK/generated/config/16-form/Fixture/Review projections, one canonical <=48KiB basis and shared six-pin meaning, with current recipe/fixture checks. Large raw artifacts stay offline as immutable refs, not claimed extra live metadata. prepared_budget at1549 charges descriptor+basis+lock+raw result+qualification under128KiB; receipt stays32KiB. Candidate strict refs, no-follow/one-link modes, exclusive fsync/no-replace publication, receipt-LAST, full-byte/currentness/auth reuse and original unknown effect/ABA retention are implemented at369/566/591/1497/1606/1620/1645. Unit controls exercise corruption, bad pins, partial publication, symlink/hardlink/traversal, original inode/effects, strict JSON and total limits. Full joined seal/reuse is NOT verified: closure refusal happens BEFORE basis emission. There is no pretest-basis.json, descriptor, qualification.json, receipt.candidate.json or external candidate receipt ID. The schema's complete actual basis size, canonical consumer negatives and final publication path remain unproven, not successful from the presence of producer code.

Public-only fixed corpus:
Fixture files reconstruct Git2.54.0's deterministic t7510 presigned commit/ancestor/trees/blobs, preserving both signature headers and ONLY its PUBLIC key armor. No signing or secret fixture-packet decoding/import/export produced them. Valid commitcbceced299d68bdfce3b95f182698f280f1da743; corrupt commit0cf6ea6f9b0da82362299e1ccad64e4a5d028de3 changes one signature byte with only armor CRC recomputed. Record second/blobd00491fd7e5bb6fa28c517a0bb32b8b506539d4d is newly introduced by the selected commit, so last-touch is meaningful. Actual object relationships/inventory are tested, not cryptographic validity inferred from hashes/CRC. corpus.lock.json SHA7717ad1de31391ead47a498a32c732066e0677fbe981d52fe8af613a95b6e8e0 records crypto unverified until real canonical probe. Prefix fixture assembly is Python loose-object serialization with <=32 regular files, no ambient Git/config/key operations.

SOURCE mutant witness:
D/runs/01M6Q7WSMDGKK0BS2QA4D2497R/source-custody.json records full snapshot SHAe05af70b4126db817145ebc5149409330be3b42a4a72ad3733c54d71ab14c78d and three compiler-built changes at actual Git main entry. Each loses SOURCE qualification before any hypothetical helper; objects have live undefined fork/pthread_create/connect calls and are NOT linked/executed. Object SHAs are2b0421f9d34304f6bb48fbea94463509d258751c886aa503f3c3ecdd48eb9b7d/c2ef256c8019020ddf6a96e1b15716715b98de9b3c2145c01dfe11d486a87831/0703c3f4607ee0a4c9274ae2f0296f2a9b3e3bd7ba63e4c273aeba83ae6f3576. This improves appended-marker controls but does NOT replace canonical actually observed hidden native-effect mutants. Initial missing OpenSSL profile defines and Git-banned strcpy compile failures remain in runs01M6Q7STZD0XZVX1WV3EAJ9ZPH/01M6Q7VP08MK3YXWZ87M47C1W7; repaired entry-mutant source check actually exits0.

Canonical component authority remains old and separate:
Source path /Users/feral/mydev/bboxy450-term/target/dream-proof-component-authority-7m7jv65m/proof-helper-probe.c length39130/SHA5493dbf0bda41ac33e4a27a9bc200f083e49501b0791d32cd255b731d6f82c33, executable in D/runs/01M6Q1E18MP38G2ZHHZXEFFPPR SHA d39a9c4f1d64c9fb4c9a4fd0efe600d3f1dd142db1f2d52efa7fba388a9136a9. ledger-controls raw004.stdout exit0/SHA134b471c3a0c789c944bb77f4aeb4539d203d8f2d16f437358ad345c97389070; native005.stdout exit0/SHA066188d0e5c5e428b06696349c8c73eb11b3f8354429602d2361bf46f5069a74; stderrSHAcbd906a83a23a36af23b0b0d52e09066945d891c2de008d49db5769fccf97032. Birth CPU847125ns/wall1856000ns; W299000/A75500/C269750/Q568750ns; post876500ns; group RSS3375104bytes; actual wait/reap observed. These original finite native/18-ledger results were reused without a timing retry. They do not execute this real pair, qualify the source premise, certify current product/lesson/four-job/full-exporter gates or admit runtime. Old normal Git mode unconditionally refuses81 and was not bypassed/forked. Frozen Q2 four-file proposal source SHA55aabc5e0efd081fe050db08a40e955ddb27ff5d8a442df1c6feb911ce8160af remains a file-byte snapshot only; no current-Q2 clean commit/runtime receipt claim.

Retained failed baselines:
- Original six-source GnuPG configure032 exit1, stdout43aa287fdf68a9586befd4ea06f4b20642083c4a4ceef9c430a8cf9876c669fd, run01M6Q1RFBC70AVGR9WCZ93BA58. Mandatory KSBA check was preserved, never bypassed.
- First seven-source post-build UTF8 map decode failure run01M6Q62JCZMME1M3XFA0J0T916: maps contain real binary literal strings; fix parses only raw object-table bytes without changing maps.
- Libgcrypt guard compile025 exit2 missing errno, run01M6Q7263AP4DE5HTNSKYMRMJR; exact repaired postimage syntax compile exit0 at01M6Q759W4P7PVK7TFYBJVQAAY.
- Native platform disk/cache mapping failure and lower-bound graph evidence remain at01M6Q76F3VZQAXQPYTGXNC2V3S; its failure/outcome/identity were preserved while explicitly source-bound offline metadata review was appended, never relabelled a completed build.
No original build/incarnation/inode/effect is cleaned or silently replaced. Unchanged failed inputs refuse rather than inventing a new attempt.

Formal:
tests/formal/dream_proof_tools.rs includes publication/Root admission/consumer binding, custody/failure/ABA, hidden-work source premise, exact SourceSeven, complete typed basis/coverage/arch/currentness/six projections and14 causal basis controls. Explicit finite two-candidate A1/B2/A3 bounds, internal weak fairness and compiler/source/platform/native assumptions remain declared in contracts/coverage. models.rs uses the existing root. Full native/Yarn projection and pinned Java21.0.13+9/TLC1.7.4 exploration agrees on all307 corpus models' full reachable states, labeled edges and safety/deadlock/liveness verdicts; this finite parity proves no arbitrary Rust/native refinement or full TLC replacement. Current source78391c4c82e94fc9f10bad2998c700c6a9278bb7, dirty=false,79 contracts. Persistent receipt verification/evidence/4ede8bbadba566bab0cb5cc7801ced14e82987229e0540098678e372489c7c7e/evidence.json has that same SHA. make check-formal-evidence validates source/model/tool inputs. Rebase/source change requires refresh.

Acceptance disposition:
| Requirement | Evidence / limit |
|---|---|
| one Make recipe owner, seven pins, actual crypto before build | Implemented and actually exercised; Makefile11/14/17, script233/405, final auth records |
| real Git/GPG/private dependencies and correct licenses | Real compile/link0; exact files above; ATTRIBUTION/license-map and canonical license gate0 |
| all65126 forms, public show/import and real implicit verify semantics | Producer retains all16 exact forms; narrow source guards reviewed; joined real execution NOT reached |
| startup/error/cleanup/library/platform lifetime qualification | Conditional source coverage only; recursive128-edge requirement FAILS, no lifetime theorem |
| compiler/generated configs/link/nonplatform/platform closure | Real retained provenance; static closure captured; full recursive typed closure Unsupported at129th edge |
| exact typed basis/descriptor/projection/custody negatives | Producer/model/unit coverage exists; actual complete canonical basis/consumer gate NOT reached |
| qualification then receipt-LAST and cached currentness | Implemented controls; real full seal/reuse NOT exercised, no candidate receipt minted |
| real positive/corrupt proof/component cases | Fixed public-only corpus assembled/tested structurally; crypto/real-pair cases NOT reached |
| hidden spawn/thread/service, clock/ABA/debt/retired/reap | Compiled entry source mutants and18 old canonical controls retained; required joined canonical process/control cases remain missing |
| native/Java formal parity and currentness |307 models/79 contracts0 plus currentness0, bounded conditional model only |
| untouched live Q2/runtime authority/product gates | Preserved; installed tools and all private artifacts remain Unproven/UNAPPROVED |

Exact decision needed from Root/Architect: disposition the incompatible actual Darwin recursive graph (at least458 load edges+9 static versus the frozen128-edge schema) as Unsupported, or amend a lossless bounded platform-closure binding with complete retained evidence and unchanged runtime/resource laws. The Executor cannot omit edges or invent a competing schema. Before joined execution, Root must also freeze and amend the changed canonical Q2 source/executable consumption pin. No operator input, approval prompt, timing threshold expansion or lucky retry is requested. Other authorized independent source/build/guard/unit/formal/license work is now exhausted; task_report has not been submitted as completion.
