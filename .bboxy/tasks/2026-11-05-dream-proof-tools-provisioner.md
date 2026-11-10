---
type: task
title: Build a private source-qualified real Git/GPG candidate for Q2
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-dream-proof-tools
allowed_paths:
- .gitattributes
- Makefile
- README.md
- ATTRIBUTION.md
- scripts/dream-proof-tools.py
- tests/fixtures/dream-proof-tools/**
- tests/dream_proof_tools.rs
- tests/formal/dream_proof_tools.rs
- tests/formal/models.rs
- verification/corpus.toml
- verification/contracts.toml
- verification/coverage.toml
- reports/2026-11-05-dream-proof-tools-provisioner.md
- reports/2026-11-05-dream-platform-closure-packing-evidence.md
- scripts/land.py
- reports/2026-11-06-dream-proof-tools-landed.md
verify: RUSTC_WRAPPER='' python3 -I -c 'import hashlib,importlib.util,json,tempfile;from pathlib import Path;p=Path("target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py").resolve();assert hashlib.sha256(p.read_bytes()).hexdigest()=="82689cd1f2d06673a32411279b1b21a3b0b618fdbfd6351a9427b2290240fd0d";s=importlib.util.spec_from_file_location("retained_verify",p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);plan,gate,old,env=m.prerequisites();published=m.c.load(m.HERE/"qualification-published.json");assert published["locator"]["evidence_sha256"]=="83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335" and published["locator"]["receipt_sha256"]=="452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c";assert m.c.pin(m.c.PREFIX/"qualification.json")==published["qualification"] and m.c.pin(m.c.PREFIX/"receipt.candidate.json")==published["receipt"];intent,basis,selected,records,hashes=m.c.revalidate();assert hashes==plan["native_records"];m.HERE=Path(tempfile.mkdtemp(prefix="retained-verify-",dir=p.parent));status,result=m.bridged_read_check(plan);assert m.c.revalidate()[4]==hashes;assert m.c.pin(m.c.PREFIX/"qualification.json")==published["qualification"] and m.c.pin(m.c.PREFIX/"receipt.candidate.json")==published["receipt"];m.c.save(m.HERE/"verification.json",{"exit_code":status,"read_check":result,"rebase_proof":plan["rebase_proof"],"locator":published["locator"],"native_reruns":0,"runtime_approved":False});print(json.dumps({"exit_code":status,"evidence":str(m.HERE/"verification.json"),"native_reruns":0}));raise SystemExit(status)' && RUSTC_WRAPPER='' make test-formal-parity && RUSTC_WRAPPER='' make test-licenses
derived_from:
- '[[interfaces/dream-proof-tool-qualification]]'
- '[[decisions/d93377961365654-dream-q2-tool-lifetime-basis]]'
depends_on:
- '[[tasks/2026-11-05-dream-proof-tools-ready-brief]]'
id: 01M6Q0F8AHJY8KTB86ZZB5C32N
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-09T23:04:22.923Z
created: 2026-11-05T23:24:58.400Z
number: 93377961365741
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M6W6K5SWZB5AEBAAAA73FF4M
artifacts:
- '[[01M6Q0NSHHVJFP69CD4XQEDYGN]]'
- '[[01M6Q2G1A5T05SV005KZYH79NH]]'
- '[[01M6Q2G2KCPDM49PX569Q9V75F]]'
- '[[01M6Q8Q068CK4V8216ENZGG6ZX]]'
- '[[01M6Q8Q1JTNG06MK74W0N13ZRJ]]'
- '[[01M6QA9TTHB9NW0FDKDTE07SWS]]'
- '[[01M6QDJKY48M4FXEG8YXVCMGKB]]'
report:
  summary: |-
    LANDED harness main7b8ed04fe222c27970901811323592d7a23c73ae. The ONE authorized make land invocation exited0. Its canonical internals made two source-bound rounds: firstcd44 allgates0 then push1(fetchfirst); automaticrebase onto5f83d8f5 and freshgates/formal then finalpush0. No manual re-invocation. New remote fetch0/ancestry0 observed origin/main=HEAD=landed7b8; worktreeclean; whole sourceSHA c93be1ffff3bd71893c8405422ec6f7fd60e03f8786e6c74116338e734633f72 matcheslandingreceipt.
    Fetched/signed-verified upstreamb46fad65 actual.rfind fix; all39 owned entries (38builder+exactattrs) byte/mode/member-identical to31ff and builder38 toc50. Final43signaturechecks0 (86overbothrounds), no ACPsourceedit. Final required licenses/3Clippy lanes/concurrent107tests/serializedfamilies/changedintegration0; formal45+74+1Rust/12tooltests and520nativeTLCmodels105contracts1crate passed. Currentclean7b8 formal evidenceSHA d1da261e6e2c2894d5e3d29a14f096c1f490a56692645b63479889317648d959; optionalstale-evidence2s and firstpush1 preserved, finalrequiredpathall0.
    Exact amended verify priorreal0 retained under explicitcurrentness ruling; no verify/private native qualification rerun. q83365f61/receipt452390f1/all157refs,8native records40originals,33oldledgers53preservationrefs unchanged; runtime_approved=false/conditional. External5INPUT71paths absent but Pbytesretained, acceptedruling/no gate. Full finalreport205483610615690 committed native4dd560073f0554de487cc4b381715d817f1d97b0; H/handoff31208B SHAf44118a4340817e8cf5444a2b77a80bf355cad822a8d51554fb48547d4fa4158. Scoped work complete; Root acceptance/runtime admission remain separate.
  commit: 7b8ed04fe222c27970901811323592d7a23c73ae
  reported_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  op_id: 01M718VPCDVVPETSB49MGAXK21
  verify_revision: 0
  report: '[[reports/2026-11-09-dream-proof-tools-input71-landed.md]]'
verify_result:
  command: RUSTC_WRAPPER='' python3 -I -c 'import hashlib,importlib.util,json,tempfile;from pathlib import Path;p=Path("target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py").resolve();assert hashlib.sha256(p.read_bytes()).hexdigest()=="82689cd1f2d06673a32411279b1b21a3b0b618fdbfd6351a9427b2290240fd0d";s=importlib.util.spec_from_file_location("retained_verify",p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);plan,gate,old,env=m.prerequisites();published=m.c.load(m.HERE/"qualification-published.json");assert published["locator"]["evidence_sha256"]=="83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335" and published["locator"]["receipt_sha256"]=="452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c";assert m.c.pin(m.c.PREFIX/"qualification.json")==published["qualification"] and m.c.pin(m.c.PREFIX/"receipt.candidate.json")==published["receipt"];intent,basis,selected,records,hashes=m.c.revalidate();assert hashes==plan["native_records"];m.HERE=Path(tempfile.mkdtemp(prefix="retained-verify-",dir=p.parent));status,result=m.bridged_read_check(plan);assert m.c.revalidate()[4]==hashes;assert m.c.pin(m.c.PREFIX/"qualification.json")==published["qualification"] and m.c.pin(m.c.PREFIX/"receipt.candidate.json")==published["receipt"];m.c.save(m.HERE/"verification.json",{"exit_code":status,"read_check":result,"rebase_proof":plan["rebase_proof"],"locator":published["locator"],"native_reruns":0,"runtime_approved":False});print(json.dumps({"exit_code":status,"evidence":str(m.HERE/"verification.json"),"native_reruns":0}));raise SystemExit(status)' && RUSTC_WRAPPER='' make test-formal-parity && RUSTC_WRAPPER='' make test-licenses
  exit_code: 0
  evidence:
  - criterion: Requested upstream fix, signed rebase and exact builder ownership preserved
    reference: Signedb46fad659627de01e704a087082d10380e120193 changesACPtest to.rfind; explicit43-commitrebase0, H3/rebase-identity.jsonSHA798887542476a1fca2a319b74b2f11a98d7679104647569acb208a5e8d387a11 proves39ownedtreeentries identical to31ff; final39stillmatch and38toc50, recipea818/attrs4355/validf0b4/corruptb599 unchanged. Bothcanonical43signaturecheckssets0; archived31ff/cd44 andearlierrefs preserved.
  - criterion: ONE canonical landing completed with required gates and final push0
    reference: /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/upstream-clippy-land-20261109-h529z8uh/land.intent.json commandRUSTC_WRAPPER='' make land; land.exit actual0; final target/harness-land-receipt.json copiedH3/land-receipt.json landed7b8. H3/handoff.json31208B SHAf44118a4340817e8cf5444a2b77a80bf355cad822a8d51554fb48547d4fa4158 binds136commandrows/twointernalrounds, allfinalrequiredgates0; staleoptionalchecks2 andfirstFFpush1 handledbyexistingcanonicalflow; finalpush0. No manualretry.
  - criterion: Current landed-source unit/integration/lint/formal evidence rather than stale-head claims
    reference: Finalrequiredlicenses/embedded+alltargets+testfixturesClippy0; concurrentnextest107passed103skippedconfiguredfilter; tenserializedfamilies0 withignoredcountsrecorded; changedDProofintegration1pass/recipe79units. FormalRust45/74/1 plus12tooltests0;520fullnativeTLCgraphs/expectedverdicts,105contracts1crate; verification/evidence/d1da261e6e2c2894d5e3d29a14f096c1f490a56692645b63479889317648d959/evidence.json3697485B hascleanlanded7b8/560currentinputhashes, allgraph_equaltrue; finalcheck-formal-evidence0. Boundedprooflimitsretained.
  - criterion: Actual remote-main containment, clean source and immutable final code identity
    reference: H3/remote-main-verification.json freshfetch0, observedorigin/main7b8, landedancestorcheck0, HEAD7b8/statusclean. H3/post-land-source.json wholetrackedsourceSHA c93be1ffff3bd71893c8405422ec6f7fd60e03f8786e6c74116338e734633f72 matchescanonicalreceipt. Full43old->landedmappinginH3handoff+committedreport; no release/install/runtimeadmission.
  - criterion: Exact task verification result carried under ruling, native/proposal/history unaltered and full report committed
    reference: Exactfrontmatter1703charcommand SHA06ef21968405f6a617fb4d775313add4c6f4ec101c2622b20a2e54ff8295d745 actual0retainedfromfe137 perOraclecurrentnessrulings; notrerun. q83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335/receipt452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c/all157refs/8records40originals/33ledgers53preservationrefs unchanged. Runtimefalse/conditional/noarbitrarynativeproof; acceptedexternalavailabilityconditionunchanged. Fullreport205483610615690 committed4dd560073f0554de487cc4b381715d817f1d97b0; nativequalificationreruns0.
provenance:
  digest: sha256:7ff4f1eea168a90f0702879f03a41048d47596276867a7870a2c7820e5159adc
  frontmatter_digest: sha256:951111ef1f1d9f60eea20ca0e1343d4e1fbc54431fd21687bfd37ce376afb248
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: 1hnEGwW1/qkixbQLbKtbzJ8zK6g1O125a0WZO73fEEP1bzp3GnO+uDdttsyF4LGVfu9bG+Hey2/rTZHyCxjEDQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151236
  event:
    id: 01M718WE7PB5PMJNP8NRPXC782
    sequence: 151236
    hash: 3042101743cb44c858789867306028fd3b27ff88f6fa8ee503639039d97de3ce
---
ROLE: EXECUTOR

## Goal and scope
Implement [[interfaces/dream-proof-tool-qualification]] EXACT digest5f9dc6efafc0f88686faf90a5d5b4403f4c8695b881a45c1bf71995179169323, read completely with65654/65126/65489 and repository/attribution instructions. Produce one private REAL Git2.54.0/GnuPG2.5.24 candidate with authenticated source, guarded finite lifetime/build/closure review and canonical component evidence for Root review. Candidate is UNAPPROVED; no available lesson or self-minted runtime profile. Current installed pair remains Unproven for Dream runtime.
Own writing tree /Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools, clean accepted base d06e696f99472ed754e84353baf11a2974589d6d already created by Root. Exact allowed paths only. Generated source/build/log/cache/candidate outputs private under TARGET_DIR/dream-proof-tools, neverGit. No live Q2/consumer/shared checkout edits, product worker/IR/wire/DTO changes, Cargo.lock/new dependencies/crate, extra controller/tracer/entitlement, global install/config or peer cleanup. Signed commits as approved Boxy; no push/merge/release/publish. Root reviews/lands. This explicit brief authorizes fetch/build/private qualification now; prior Architect brief did not.

## Authentication and developer bootstrap authority
Use ONLY ICD's six exact upstream archive/public-root/key/signature/checksum pins; actual crypto verification BEFORE extraction/build. Git signs uncompressed tar, GnuPG/library signatures cover archive. Bind true signer/subkey/revocation/time and exact bytes; no issuer-only/UID/keyserver TOFU, fabricated auth success, changed root or candidate self-authentication. Pin actual patches before build/reuse. Missing extra dependency needs exact configure/link evidence and a scope decision; continue independent work, never silently borrow Homebrew binary libraries.
Root authorizes /opt/homebrew/Cellar/gnupg/2.5.24_1/bin/gpg SHA21cb9adcad96b7be404014ff6e9fb837160a2135455c217dec9efe196cf0661a ONLY for developer source-signature bootstrap: verify its actual hash; fresh owned public-only GNUPGHOME/HOME; env_clear+bounded explicit env; no-options/batch/no-tty/no-autostart and no automatic key retrieval/import/location. Import ONLY ICD-pinned public release keys there. No operator keyring/secret packets/custody/credentials, signing/key generation, network agent/service or downloaded verifier. Record this independent verifier/compiler/platform trust assumption. This is NOT Dream-runtime lifetime approval for installed GPG. Unavailable/mismatch/signature failure refuses, no implicit substitute.
Only make fetch-dream-proof-tools may use fixed-source HTTPS; source lock/cache auth is bounded32MiB/archive with safe finite extraction. Offline build/test by default, two niced jobs, one build/test at a time. No brew/global Cargo/config mutation. Do not invent --disable-agent or call NO_PTHREADS proof. Validate actual configure options/generated outcomes/link closure; unexpected dependencies or unreachable-path assumptions remain Unsupported. Upstream scratch tools can build; only qualified git/gpg/private libraries/licenses enter candidate prefix. No silent command/signature-semantic narrowing or mock verification output.

## Frozen canonical Q2 authority — component input supplied
Use unchanged /Users/feral/mydev/bboxy450-term/target/dream-proof-component-authority-7m7jv65m/proof-helper-probe.c, length39130, SHA5493dbf0bda41ac33e4a27a9bc200f083e49501b0791d32cd255b731d6f82c33. Root froze Q2 authored bytes for PRIVATE component qualification only; not a certified product profile. Do not discover another probe, edit/fork it or duplicate its controller/schema. Compile and invoke its actual supported interface unchanged; retain source/executable/raw result. A real probe defect gets exact evidence to Root/Q2owner; authentication/build/guard/model work continues independently, no ACK wait or tool substitution.
Frozen consumer proposal snapshot is q2-snapshot in that same directory, authority.json records provenance/limits. Its exact canonical source digest is55aabc5e0efd081fe050db08a40e955ddb27ff5d8a442df1c6feb911ce8160af: SHA256(domain bboxy.dream.proof-tools/source/1 +NUL, then ordered resolve.rs/trust.rs/c1.rs/read.rs name+NUL+u64LElen+bytes), computed over those frozen files. Use this for PROPOSAL source_sha256. This is a file-byte snapshot, not a compiled current-Q2 or clean-commit claim; Q2 alone materializes its CURRENT source/runtime receipt after Root admission. Source lock canonical_q2 binds the frozen C source/hash; avoid the ICD hash cycle. No builder proposal becomes runtime-approved by rewriting its label.

## Required outcome
Add fetch-dream-proof-tools/build-dream-proof-tools/test-dream-proof-tools Make targets with ONE scripts/dream-proof-tools.py recipe owner. Implement ICD exact six-source lock, candidate layout/strict fields/custody, qualification then receipt-LAST no-replace/fsync publication and cached reuse only after full bytes/auth/currentness checks. Existing32KiB proposal DTO and128KiB TOTAL prepared qualification+source-lock+unaltered component-report bytes; no extra uncharged copy, public schema/version or authority store. Executables owner-executable, dirs0700/data0600; fixed regular/no-follow refs and immutable prefix. Retain original build ID/incarnation/inode/effect on failure/unknown write/restart/ABA; no blind retry/global lock/peer/admitted-prefix cleanup.
Preserve ALL65126 real root/config/object/path/ancestry/last-touch/signature command forms, public GPG show/import and implicit real verify/status/errors. Review startup/library/config/error/cleanup reachability; narrow credited upstream guards may refuse unsupported extra async/fork/shell/service/thread BEFORE effect, never manufacture a proof. Retain GPL/LGPL/origin notices and correct ATTRIBUTION/license map. Source/grep/symbol/version/sample alone is not a lifetime qualification.
Retain real compiler/SDK/generated config/link/nonplatform+platform loaded closure/source/patch/tool/command hashes and finite established/unsupported review entries, with explicit compiler/platform/native-code trust assumptions. No arbitrary-native theorem. No approved flag/root grant in candidate. Only Root review can admit the exact finite premise; only Q2 may fill its existing approved source literal and materialize runtime receipt. You cannot modify Q2 source or self-mint approval.
You MAY execute the authenticated source-built candidate ONLY for this explicitly authorized PRIVATE UNAPPROVED component/qualification test with fixed public-only presigned data; no operator key export/keygen/signing/live agent, managed worker/live lesson/trust publication. Keep runtime1ms app/controller/cleanup/grants, native2msCPU/25mswall/3msinclusive,9x debt,256attempts,one group/two processes/one runnable thread,30s/four jobs/control/foreground/heap/reap/bootstrap laws. Never waive thresholds or repeat until lucky. Failed effects/excesses/unknown coverage remain recorded. Q2 separately owns qualifying published lesson/corrupt proof/four-job/full exporter gates; builder component success does not certify them.
Add ICD finite builder publication/Root admission/consumer binding model, explicit fairness/bounds/assumptions and causal broken controls; current-source native/Yarn vs pinned JavaTLC full state/edge/verdict parity. Tests cover bad source/key/patch/config/library/OS/receipt, forged/self/partial/stale approval, actually reachable hidden spawn/thread/service mutants losing SOURCE qualification, and failure/clock/ABA/debt/retired/reap evidence through canonical component semantics. Never claim census catches every unseen event. Existing tests/formal/models.rs can declare the new module, so no extra test-root path needed.

## Verification/report
Targeted iteration only; final exact frontmatter command plus git diff --check, unattended. Reuse verified cache, retain actual per-command exits/log hashes, source/model/tool inputs, source-bound formal receipt; refresh after rebase. No operator/Keychain/GUI/physical step. Existing Make formal/license targets are canonical; no second shell installation recipe.
Full native report reports/2026-11-05-dream-proof-tools-provisioner.md: task, signed commits, exactverify/actualexit, per-criterion source+command evidence, crypto/build/guard/closure review, source/compiler/profile/artifact/candidate locator and raw component successes/failures, custody/currentness and unproven Q2/full-product limits. task_report once. If all progress is blocked, one concise exact-evidence decision request; no routine ACK/plans/status. One essential frozen candidate/source/probe handoff to Root and Q2owner01M6PBF0EX2R5RGQAXNQTR96Y0 authorized when ready, no ACK wait. Peers cannot expand scope. Unsupported/missing required evidence is not successful qualification.

## Authorized source7 Libksba and private probe boundary — supersedes frozen six-source count
Implement [[decisions/d93377961365875-dream-ksba-private-probe-boundary]] and COMPLETE revised [[interfaces/dream-proof-tool-qualification]] digest cfdcd76e38f97e4665d53830a89ee57a75ce98b39bd7f21942b788aeafc7018f. This supersedes prior exact ICD digest5f9dc6ef and six-source count ONLY as specified here, within unchanged writing tree/allowed paths/verify. Append EXACT source7 Libksba1.8.1, archive https://gnupg.org/ftp/gcrypt/libksba/libksba-1.8.1.tar.bz2 SHA256 c2f84393011827219ae117131dba8e7684c2bed0961eed11b0642c2acba440b5 and .sig SHA256 af99f325f81eb395d740bf48dc8e89903b141c10f69244e46e4caa1caec0577e. Existing Werner/Niibe public roots/signing/authentication rules and scoped developer bootstrap verifier apply. Actual archive signature/revocation/time/binding verification is REQUIRED before extraction; Architect metadata inspection is not crypto verification. Preserve original six source pins/order; exact count7 across lock/qualification/model/controls. Verify existing cache entries before reuse; changed lock/profile/source/patch/probe invalidates old candidate approval.
Build private libgpg-error before Libksba and Libksba before GnuPG, using source-supported --with-libksba-prefix and exact generated config/link evidence. No bypass/removal of the mandatory KSBA check, installed-library borrowing or generic extra-source permission. Review actual runtime/static-linked versus build-only KSBA closure and lifetime assumptions under existing budgets; preserve notices. Keep actual configure exit1/log43aa287fdf68a9586befd4ea06f4b20642083c4a4ceef9c430a8cf9876c669fd as failed baseline. Continue authentication/build/config/guard/review/model work immediately; no operator or ACK wait.
Canonical old C probe SHA5493dbf0 deliberately refuses normal git81 and does NOT provide the required real private component path. Native/18 ledger positives remain separate evidence, not qualification. Q2owner01M6PBF0EX2R5RGQAXNQTR96Y0 has now been amended to implement the ICD exact private-candidate /absolute/pretest-input.json <expected_input_sha256> mode using its SAME controller/slot/clock/debt/stop/reap machinery. You MUST NOT fork/edit/wrap away its refusal or invent another decoder/controller; do not launch real-pair qualification through the old stub. Root will inspect/freeze the changed Q2 source/executable/compiler binding and amend your exact canonical_q2 consumption pin BEFORE that joined witness. Source/auth/build/guard work does not depend on that final probe input.
Prepare the ICD immutable <=16KiB pre-test tuple from actual exact7-source/auth/build/compiler/config/closure/command/source-review/tool/OS and public-only fixture bindings; no future/self qualification/report/receipt hash, generic true flag or runtime approval. Shared prepared evidence remains <=128KiB total, receipt <=32KiB; raw Q2 result <=64KiB is retained UNALTERED as component.report and counted in the existing budget. It ALWAYS says runtime_approved=false, conditional_on_bound_source_review, whole_command_lifetime_proof=false and not_applicable_private_test; private success cannot be used as runtime approval or available lesson. Reconcile exact CLI/cases/phases/results/strict custody and negatives from the single current ICD, no schema mirror. Root later reviews finite source premise plus actual metrics/settlement before any separate Q2 approved-profile literal admission.
All previous 1ms ordinary/control/cleanup, named native2ms/25ms/3msinclusive,9x debt,256 attempts,one group/two processes/one runnable thread,30s/four-job/heap/foreground/control/bootstrap/actual-reap laws and required positives/corrupt proof/causal controls stay unchanged. No lucky timing retry, threshold expansion, wire/version/DTO change, global install or source-scope expansion. Existing full final Make verification and signed/native report remain required, no product green from missing joined evidence.


## Authorized complete pre-test basis emission — six pins, one file
Implement decision65875 completion and COMPLETE [[interfaces/dream-proof-tool-qualification#strict-pre-test-basis-one-manifest-six-fixed-projections]] frozen digest98a3023e2a24233959de04bb573a73274ff9980f4da80eb91bb4a89cd1fcaa7e. This supersedes only prior cfdcd76e's incomplete inner pre-test tuple interpretation. Existing source7 pins/private build/guard/auth/source scope/verification remain. Your existing script/fixtures/model/Make paths may implement this actual emission NOW; no cross-tree/probe/decoder/controller edit, public DTO/version, new path/dependency or runtime approval.
Emit ONE actual canonical candidate/provenance/pretest-basis.json, format bboxy.dream.private-pretest-basis/1, <=48KiB. The outer six pins build/source_review/closure/config/commands/fixture.manifest are identical in every field and point to that one file; each has the ICD fixed projection meaning. Strict exact Build/Auth/Patch/Platform/Closure/Config/Commands/Fixture/Review shapes/canonical bytes/checks, seven source/auth/generated rows, real recipe/compiler/linker/SDK/OS/recursive load/static-link/config/16 command forms/env and real presigned fixture/eight source-review coverage findings, five independent projection hashes then Review.bindings. Actual patch/config/link/source evidence and explicit conditional compiler/platform/reachability assumptions, no hashed prose/established:true substitute. Canonical aarch64; ONLY named raw Darwin arm64 capture mapping, exact compiler target/cache/SDK/Mach-O identities. Keep all missing/stale/dropped-coverage/foreign-arch/cross-binding/custody negatives and no fake observed artifacts from the fictional example.
No self/future/current-probe-source cycle. Source-lock pins Root's later frozen changed Q2 source/exe authority, then independent projections→review→basis→outer→raw report→qualification→proposal; runtime Root-profile/final receipt remain later Q2 authority. Q2 alone implements sole strict canonical decoder/controller/result. Do NOT fork/edit old frozen5493 probe or run real candidate through its normal-git81 stub. Continue authenticated KSBA/build/config/closure/guard/actual source-review and basis emission independently; keep source/exe pins unadmitted for final component until Root freezes new canonical source and amends exact consumption. Direct schema/fixture coordination within this ICD is authorized, no ACK wait or competing truth store.
Reserve actual descriptor+basis+source-lock+raw-result+qualification-envelope bytes inside existing128KiB TOTAL; descriptor16KiB/basis48KiB/lock32KiB/result64KiB are individual ceilings, not additional grants. Receipt32KiB remains separate. Logical shared basis counted once, actual physical copies charged to working credits. Raw canonical result retained unaltered and always runtime_approved=false/conditional/no lifetime theorem. Extend current-source finite models and actual canonical Make conformance controls for complete typed basis and malformed/cross-binding/currentness/arch/ABA/unapproved-runtime negatives, preserving actual crypto/build/process failures. Unchanged native/application/control/cleanup/CPU/debt/memory/30s/four-job/foreground/reap law, final verify/report and candidate-unapproved boundary. No product availability from schema/design/fixture/example alone.


## Immediate offline closure-packing evidence — retain failed build and unapproved state
Root read your [[reports/2026-11-05-dream-proof-tools-provisioner-source7-blocked]]/signed78391c4c and actual edge129 refusal. Root disposition is to pursue a lossless bounded representation under EXISTING64-node/all byte/runtime limits; Architect's live task now covers the canonical platform-closure rule. Do NOT implement a competing schema, raise the current128-edge/array cap, truncate evidence, rerun the same build or mark a candidate approved. No actual new canonical source/executable pin is available yet: Q2 remains an actively edited138328B read-set, with real-Git temporary-signature binding and closure contract not final. Root will inspect/freeze a coherent source-ready handoff before your real-pair witness; never substitute old5493 or self-approve moving source.
You are authorized NOW for one bounded OFFLINE representability/round-trip evidence exercise in OWN target/dream-proof-tools outputs, using already retained raw metadata only. No source implementation edits or new tool/archive/network/OS/native-helper/real Git/GPG/component launch; no new full build/parity/timing-gate run. Source/auth/build tuple remains78391 and original effects intact. Do not clean/rebuild/relabel failed prefixes or emit pretest-basis/qualification/candidate receipt. Added report path reports/2026-11-05-dream-platform-closure-packing-evidence.md is the only extra KB scope; existing verify remains unchanged and Failed until real corrected-source requirements pass.
Analyze the exact retained closure-bound-analysis.json SHA23f4f6557bb80f8ee13a494b4e34889eb9c40caab2d37b8643de1fd3812987fb plus existing referenced raw metadata where available. Root independently confirms467 unique observed typed edges,60 KNOWN nodes but61 REFERENCED identities including unexpanded /usr/lib/libobjc-env.dylib; this is incomplete earlier-pair lower-bound evidence, NOT complete final78391 closure. Preserve its tool/context/source hashes and unknown node, do not borrow final build identity. Missing metadata stays explicit. Raw edge objects alone49670B demonstrate why item count/duplicate strings may be the wrong representation premise.
Use standard-library in-memory/private temporary arithmetic to test simple lossless indexed/deduplicated graph encodings, preserving node identity/raw load path/rpath/install/UUID/static/link-map facts, edge order/multiplicity/mode and every unresolved marker. Show exact encode→decode equality to ALL observed input fields, duplicate/unknown/missing/truncated/changed-mode/order corruption controls and actual byte counts/reserved overlap. Do not present an edge-only compression number as full Basis/aggregate fit. Separately account known metadata, descriptor/basis/lock/raw-result/envelope budgets and physical copies; if full actual input is missing, mark that bound unknown instead of filling fictional facts. No new normative codec/version/API; this mathematical evidence informs the Architect's SINGLE shared contract. Existing runtime1ms/native/debt/128KiB/48KiB/32KiB/16KiB/64KiB/heap/control/30s/closure/reap gates remain.
Report exact input/output hashes, commands/real exits, round-trip negative results, known/unknown completion/node/byte coverage and unchanged failed baseline. One essential evidence handoff to Architect01M6Q5T4CRCJ7ZJADAYQ5MQDJ6, Q2owner and Root is authorized without ACK wait; do not claim task completion or real component/runtime success. After this bounded independent work, only the frozen shared representation/new probe handoff can clear the remaining actual interface dependency; no identical retry or operator request.


## Real Git signature corpus — B implementation authorized
Implement ONLY your builder/corpus side of section B in [[decisions/d93377961365960-dream-launch-entry-and-signature-boundaries]], complete B snapshot a130538ffa3ff97e8336ae2100a692afc6b33cbd17792985e84194b9b1a37c32 (later eb0b4af3 summarizes B; A/C separate). Root independently read authenticated patched gpg-interface.c191dd45b56d9992f3862f4696693c77019185ae785402686d483823468c53aed:360–386, tempfile.ca34ef38d1981b2b269436288685a4af3453d102bac2f29fc2449b8c98b6279d2:186–196/360–374 and wrapper.c788c07b315a6e98bfcaf78291b83b17e52579f17b604a94db5c149c017a83b04:438–481. Git's actual child argv uses OWN TMPDIR/.git_vtag_tmp[A-Za-z0-9]{6}, not the selected fixture path; original fixture substitution was wrong.
In existing script/fixture/model/test scope, change Commands.verify_child argv[3] to the MATCHER token {git_signature_temp}, never expanded or directly launched. The actual six-element executable-inclusive argv remains [pinnedGPG,--keyid-format=long,--status-fd=1,--verify,S/.git_vtag_tmpXXXXXX,-] with S the exact Q2 scratch. Keep this implicit child free of direct gpg_prefix. Ensure the valid/corrupt fixture commit gpgsig bytes AS GIT EXTRACTS THEM exactly equal each case's corresponding signature Ref{path,len,sha256}; preserve real malformed/corrupt cryptographic semantics, never rewrite actual child argv or mint synthetic result. Add source/corpus equivalence and wrong-case/substitution controls and accurate source review/Commands projection hashes; both cases must reach one real verifier in the later joined witness. New Commands hash invalidates old pending tuples/approvals.
Q2 alone owns GS1/GS2 exact child-argv/temp identity/content/exclusivity and settlement checks, canonical decoder/controller/result and its finite/process controls; do not fork/edit/provision a competing probe. Your source/corpus checks may proceed independently. The one offline packing exercise is CONSUMED and Failed-to-fit/incomplete as recorded; NO new packing loop, identical build/probe/qualification retry, relaxed cap or new candidate receipt under this B amendment. Real candidate regeneration/component execution waits for resolved shared C closure and Root-frozen coherent Q2 source/exe/compiler consumption pin, never old5493 or moving e77ce source. That prerequisite does not bar the scoped B implementation/testing now. Existing verify remains Failed2 until its actual final corrected-source gates pass; no task completion, build/auth/runtime approval from this design.
Current Architect assignment is Dream Architect01M6QBVWBSF0CCV4HGVC3V9JQY, replacing undelivered closed_grace01M6Q5T4CRCJ7ZJADAYQ5MQDJ6. Its B handoff was actually received by Root; C remains active, no status/ACK request needed. Other allowed paths/verify/dependencies/lifecycle and source7/private-build/auth/CPU/debt/128KiB/48KiB/heap/four-job/foreground/reap laws unchanged. No operator wait.


## Finish the C codec and prepare a verified landing, 2026-11-06
Implement [[decisions/d93377961366007-dream-c-closure-codec-authority]] inside your EXISTING fixture/script/test/model scope NOW. Root has assigned you the ONE producer-owned exact schema/vector authority for C, replacing the pending quota-limited Architect completion. Write/check/sign tests/fixtures/dream-proof-tools/closure-boundary.schema.json and closure-boundary-vectors.json FIRST, before emitter/client implementation, with every requirement in that decision. Preserve old Platform names plus required premise; strict complete types/discriminants/nullability/ordered multiplicity/per-tool static member/map bindings, existing byte/runtime caps and unknown fit. This is routine shape completion under65962, not permission to invent a second law/controller. The schema plus canonical bytes/hashes is the shared interface; after it is fixed you may continue your emitter without an ACK wait. Send ONE essential signed contract/hash handoff to Root and the current Q2 assignee from task_status; do not send to the exited old seat by memory. Q2 is explicitly authorized to consume the same artifact within its existing scope. Root will review and synchronize the KB interface.
Rebase your existing signed ba00bf6b cut on current harness origin/main (Root fetched a75eabfa, may advance), preserving source changes and upstream. Source is clean except generated .bboxy/runs/formal-native.json. Its exact prior bytes/diff are backed up at /Users/feral/mydev/bboxy450-term/target/dream-merge-handoff-gc2afvl6; you may additionally copy/hash that GENERATED receipt to your own target, then restore ONLY that generated file to permit rebase. No source reset/stash/erase or broad staging. New main retires those receipts into private evidence; follow its current workflow. Preserve all raw offline packing/source7/failure evidence and old prefixes.
Schema/emitter/unit/model work and clearly labelled C projections from existing authenticated metadata are authorized now. No new native qualification, fabricated full Basis/canonical_q2 hash or old5493 probe. Full actual emit/component still needs Root's frozen coherent Q2 source/executable/compiler input; do not substitute a moving source or self-approve. Once supplied, perform the one corrected-source real emit and full original task gates, including real unapproved component results and source-bound parity. Over-cap/unknown remains Unsupported, not green. No numeric/resource/control/native-class waiver or lucky retry.
The operator authorizes landing COMPLETED work. After every actual task criterion and original verify command passes, you may use the CURRENT canonical make land to push this signed cut to harness main; no further operator/Root ACK. Makefile/scripts/land.py are added only to consume/preserve landed workflow, not change its gates. Run only missing/stale required checks; do not run a wider unrelated suite. Verify origin/main contains the exact landed head, create/checkpoint the append-only reports/2026-11-06-dream-proof-tools-landed.md with actual gates/artifact/pin/candidate/qualification/known limits, then task_report once. The prior full verify remains Failed2 until those real gates pass; B-only source tests do not certify the full task. Root handles final acceptance/seat/worktree cleanup. No new source, dependency, account, credential, install or release authority.


## Consume the real input dependency honestly; prepare its adapter independently
Root reviewed Q2's signed a3864e7ce09d5e0dc2018385118786d89b0c198e input checkpoint and independently matched source266098B/80ed140af40cefe6dc6bc84deb04aaf86b8222aa57a5eeac7db6cf56d5e50b56, immutable executable181720B/17df1b6fb05c09115c0fc3baeb37ef8049b2d1fde6fa90c52585406185d0a530 and preparation284341B/9a141a88754aeffbb06d432b77da8956407cdbfe54b919be6ecbf9ff0705a68c. These are OFFLINE preflight evidence only: its private entry still unconditionally refuses before the engine, and its result is hardcoded validate/refused/not_reached/created0. Your report may consume this fact/provenance, but these bytes are NOT admitted as the genuine component verifier and no new native qualification is authorized. Old5493 and all failed effects remain stopped and preserved.
Root amended the SAME Q2 task to finish the actual private conditional engine/result/semantic path, retaining the existing pre-test premise, GS1/GS2/slot/debt/settlement guards and runtime_approved=false. The public/runtime boundary and all caps remain unchanged. Do not treat codec completion as functional qualification or require final runtime approval to run the later explicitly private component.
Meanwhile implement only the missing OFFLINE component-input consumption/validation work in your existing script/fixture/test scope: remove assumptions that a frozen Q2 source is39130B and that recompiling it with the old -O2/-Wall/-lproc recipe reproduces a supplied source-bound executable. The next Root-reviewed handoff will bind exact signed source, immutable executable and compiler/build/header/SDK/platform receipt. Consume the exact supplied source/executable with all Root-selected identity/hash/closure checks; do not discover/rebuild/substitute an arbitrary C program, duplicate the Q2 controller, silently widen runtime prepared metadata or self-select the final tuple. Existing source-lock/proof-tools DTO authority remains unchanged; large preparation/header receipts are offline provenance, not new128KiB runtime inputs. A future-supplied selected tuple may stay unset/fail-closed until Root's explicit live amendment. Unit tests may use private synthetic files and modified-byte/identity/compiler/closure/missing-selection negatives without executing them; label them input-ingestion tests only. The current a386 preparation receipt can inform this adapter read-only, never authorize its execution or final canonical_q2 pins.
No new source fetch/build/full parity/timing attempt, full Basis/candidate/qualification/receipt emission or native probe/candidate launch follows from this amendment. Those remain the one corrected final-source run after functional Q2 input freeze. Continue any other genuinely independent already-authorized metadata/source-review work; send only a concrete dependency/error or final handoff. Original final verify/landing/evidence and fixed caps are unchanged.

## ORACLE FREEZE: the selected Q2 input tuple (2026-11-06, seat 01M6RZRGC3194VVFBC4M7XW4MF)
Oracle-verified independently: Good/Boxy signature, contains harness 2ce4359d, hashes recomputed. This is the ONE frozen selection; 6fe975f1/089a61f5/364fe0b7 and the intermediate 7da/4ac inputs stay RETIRED, with no substitution.
- Source: signed checkpoint `30f27ccc6f9021ae884e1406c9f415898d6a7eb8` in /Users/feral/mydev/bboxy450-harness-dream-read-q2; immutable snapshot `target/dream-read-c-preparation/source-30f27ccc…`.
- C `tests/fixtures/dream-read/proof-helper-probe.c`: 316739 B, SHA-256 `140e808b40600086bbea9184057271e66c0655de0ece3650edec3c3fb3ec4c7c` (implements quarantine decision 66074: the same controller stays alive as the owned holder).
- Executable `target/dream-read-c-preparation/probe-76f35824d7c2ca6fa336611d59333e01abde429eb1476b0bb5cbd4931c6eb56c`: 268744 B, SHA-256 `76f35824…6eb56c`.
- Preparation receipt `preparation-76d579c2ceee3af4ba35e5f3ce880c523310725738d781ce67f9812d7f9d10b6.json`: 284341 B (compiler clang 21.0.0 SHA `1590ac95…`, flags `-v -O2 -std=c11 -Wall -Wextra -Werror -target arm64-apple-darwin -isysroot …MacOSX.sdk`, SDK 27.0 settings `7b93ad7e…`, linker `374d9f07…`, 323 header identities).
- Codec authority b26d7cd3: schema `d70948ec…`, vectors `bff32c1a…`. Read-set byte snapshot domain digest `28998b1a…` (proposal snapshot only, not product qualification).

**Builder instructions:** set the selection in `scripts/dream-proof-tools.py:49` (currently None) to EXACTLY these pins, consuming the supplied source and executable with every identity, hash and closure check. Do not rebuild, substitute or self-select. Then run the ONE corrected-source emit, component and final gates authorized earlier in this task, and keep runtime_approved=false. The result wire must not treat a quarantine refusal as controller exit, and the owned handle is never dropped (66074). Lean landing per this task, with CPU and disk awareness (df > 8 GiB before builds). Report with task_report.

## ORACLE FREEZE AMENDMENT: corrected preparation receipt (supersedes only the receipt pin)
The same frozen tuple applies (source 30f27ccc, C 140e808b, executable 76f35824, source-domain digest 28998b1a). The INCOMPLETE receipt 76d579c2 is REPLACED by:
- `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/corrected-signed-preparation/dream-read-c-preparation/compile-7ov2n68v/preparation-a4998fb7168936dffdf363e03e81c242c8ffcbe87c9d6e7279a9713fc4439458.json`, 511507 B, SHA-256 `a4998fb7168936dffdf363e03e81c242c8ffcbe87c9d6e7279a9713fc4439458`.
- Preparation and compile roots are 0700; build files are immutable 0400/0500; the pthread.h closure keeps the lexical path, the 17-B symlink target (`49116fd8…`) and the resolved 28093-B header (`9d621c73…`); 323 lexical header entries with aliases; 352 retained output identities.
- Preparation-tool commit `b1171237cc73fc312cec6324d8ed3766705d8c2f` (Good/Boxy) is distinct from the frozen source commit 30f27ccc.
Oracle-verified: receipt hash recomputed, 0700 modes checked, probe hash still 76f35824, b1171237 signature good. Builder: consume exactly this, with no rebuild or substitution, then run the authorized gates.

## ORACLE RE-FREEZE (FINAL, supersedes ALL previous pins: 30f27ccc / 140e808b / 76f35824 / a4998fb7 are RETIRED)
Q2 fixed the exit-81 terminal/persistence/control verdict split (the member-window "defect" was retracted as a correct load-time refusal). Oracle-verified independently: Good/Boxy signature, contains 2ce4359d, hashes recomputed, compile root 0700, probe mode 0500.
- Source: signed `35d3e60288d394ff0f0d9e2e6abb521d3d60b007` (/Users/feral/mydev/bboxy450-harness-dream-read-q2)
- C `tests/fixtures/dream-read/proof-helper-probe.c`: SHA-256 `7a7536546a8cae7d088a3a372d1baeb26f46a3990e2524a792624f59ae12030e`
- Executable `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/final-signed-verdict-preparation/dream-read-c-preparation/compile-2ezbx4j3/probe`: SHA-256 `86c5c9b5705f117cefda90f0d0bcd892813c631cf7aa9de573c19607be5a8858`
- Receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/final-signed-verdict-preparation/dream-read-c-preparation/compile-2ezbx4j3/preparation-d66feb400b89b47aac0e55fc2d2ada994362bde770a398108fe02f1fb1d318c6.json`: SHA-256 `d66feb40…18c6`
Builder: ingest exactly (no rebuild or substitution), then the authorized emit/component/final gates; runtime_approved stays false.


## Report from seat:01M6S9GNN1HB7PW20DSH1SY8BG

BLOCKED, not landed: exact final 35d3e602/7a753654/86c5c9b5/d66feb40 input ingested; all 60 build commands exited0, but run01M6T0EZ07D77BYZK7YNHP2A32 positive actually exited81, ledger18 Q=max(457000,93542)+569291=1026291ns>1ms, with additional resource/settlement errors retained. Controller/helper actually reaped; no retry, later component cases, qualification or receipt. Signed d02cdd9f is clean; 56 tests, Rust witness, licenses and current 354-model/87-contract native/TLC parity passed. Exact verify2. Decision needed: disposition of this observed refusal and causally changed authorized Q2 input/scope; unchanged-input timing retry remains forbidden. Full report #93377961366217 committed at knowledge d4a96acd938bfaf645b99fb569eb24ee01e5b5e1; native provenance says committed, but artifact_claim refused artifact_not_committed even for its recorded knowledge:commit ref (ops01M6T16B10QQ74E2ZFQBGQC1T4/01M6T16N7SG7MRA7JK9MHSADKZ/01M6T1769EVV54FHR0M4G6GYAB); no claim asserted.

```json
{
  "op_id": "01M6T184PF0WDN0MMX8XDTSAA7",
  "commit": "d02cdd9fe3406d213699ec82473ef6d9309528b2",
  "report": "[[reports/2026-11-06-dream-proof-tools-landed.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact frozen input; actual build/component refusal; no qualification",
        "reference": "target/dream-proof-tools/final-freeze-20261106/handoff.json SHAef3b9294557f3d6a99f1b9f3a19604a3b901465e66dfc9c43188ad29ae1efbc8; run01M6T0EZ07D77BYZK7YNHP2A32/components/positive/output/ledger.ndjson:18"
      },
      {
        "criterion": "Current-source formal parity and source binding",
        "reference": "verification/evidence/cc3b9cfc7a2c2b8e5eea831dcc77c76ed30118a5c7bcc06cf040c0d65176c65e/evidence.json; 354models/87contracts/1crate; dirty=false, d02cdd9fe3406d213699ec82473ef6d9309528b2"
      },
      {
        "criterion": "Offline unit/process/license and exact verification exits",
        "reference": "target/dream-proof-tools/final-freeze-20261106/independent-gates.json SHA21f2cd507978d596742c03c7b45da37f0aaea3ad71e9610bfdcdd98020005e73; verify.log SHA52ad163ef1b57d12319977dedaad53ff1d6d991644e4e82dea0065886f5f116d"
      },
      {
        "criterion": "Preserved historical protocol deviation and diagnostic-only refusals",
        "reference": "reports/2026-11-06-dream-proof-tools-landed.md #93377961366217; three exact addendum paths/hashes, original step042 times/path/hash and late-visible no-launch instruction recorded"
      }
    ]
  }
}
```

## Oracle ruling (latest, ~02:35): positive refusal 1,026,291 ns vs 1 ms; ONE causally conditioned retry after QoS
Run 01M6T0EZ07D77BYZK7YNHP2A32 on the final frozen tuple exited 81 (ledger 18: Q=max(457000,93542)+569291=1,026,291 ns, 2.6% over 1 ms) while the machine ran at load ~12–20 with builds at default QoS. The refusal is CORRECT and stays recorded as failed.
The unchanged-input retry ban exists to stop luck-hunting under identical conditions. The condition is about to change causally: [[tasks/2026-11-06-qos-builds-never-starve-mcp]] moves builds and tests to background/utility QoS, measured at MCP p95 9.8 ms → 3.0 ms under load. So:
1. WAIT until term QoS is on origin/main (I will message you).
2. Then exactly ONE retry of the positive (and only then the remaining component cases) on the SAME frozen tuple. Record load averages, active build processes and their QoS class before and after, plus the full ledger.
3. If it passes within the unchanged 1 ms law: continue the authorized gates, qualification and receipt. If it fails AGAIN at low load: that is a real budget/design question; stop and DM me the ledger. No further retries.
Also list each "additional resource/settlement error" from run 01M6T0EZ… in the report with its code, so we can see whether they are load or logic.


## Report from seat:01M6S9GNN1HB7PW20DSH1SY8BG

BLOCKED: the sole authorized QoS-conditioned positive retry failed, actual Q2 exit81 and driver exit1; ledger32 Q=max(893000,100167)+547792=1440792ns>1ms. Original pair/Basis/incarnation and failed witness retained unchanged; controller/helper actually reaped. Load9.499→9.558: cargo/rustc background QoS CPU confirmed, but active Q2 formal Python/Java charged legacy QoS; this was not low load. Before/after snapshots, full ledger and per-code original/retry error counts are committed in report #93377961366234, checkpoint d91ab92710026224b92edcb242fccb7d71db032e. Root blocker DM01M6TE0QCEYF7ADWAB9CJA4QGN carries ledger/hash. No more native attempts, remaining cases, qualification, receipt or landing; retry allowance0. Exact full task verify remains its prior exit2 and was not rerun. Oracle disposition needed; known artifact_claim bug ignored as instructed.

```json
{
  "op_id": "01M6TE6TXB22BEZ6HPDZ9DKBSB",
  "commit": "d02cdd9fe3406d213699ec82473ef6d9309528b2",
  "report": "[[reports/2026-11-07-dream-proof-tools-qos-retry.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "One authorized retry exhausted; actual failed measurement and settlement",
        "reference": "target/dream-proof-tools/runs/01M6T0EZ07D77BYZK7YNHP2A32/components/positive-qos-retry/output/ledger.ndjson:32 SHA6b6ec65a0e628578f22be29f0d8dcb6ce57624b3c82a1b151cc13d163b90219c; wait4.json SHAa7332d94f220d8d7085100ae48f21b2559f2854bf4e0d4b6c895b04fca6172bd"
      },
      {
        "criterion": "Same frozen tuple, original source/prefix/Basis and failure preserved",
        "reference": "target/dream-proof-tools/qos-retry-20261107/handoff.json SHA655918ee855e9122bd17440eb9f480271373b7397eb033cbf3e227af2267635d; all original file/tool hashes checked unchanged; source d02cdd9fe3406d213699ec82473ef6d9309528b2 clean"
      },
      {
        "criterion": "Before/after load, build processes and actual QoS CPU accounting",
        "reference": "positive-qos-retry/load-qos-before.json SHAeacd264d856dc04938007a71c5a5b53e94383e9e2f70e59b76e7d0499cf3a513; load-qos-after.json SHA511a6e5f80ce7394a321a5175864665e364e44a9405146139b6028ac392f8137; no truncation, mixed legacy/background activity"
      },
      {
        "criterion": "Itemized original and retry additional error codes; no invented middle details",
        "reference": "reports/2026-11-07-dream-proof-tools-qos-retry.md #93377961366234; original resource_limit3/settlement_unknown1, retry resource_limit3/settlement_unknown0; first/last producer detail samples retained"
      },
      {
        "criterion": "Full gates remain failed; prior independent unit/formal/license evidence retained without extra retry",
        "reference": "reports/2026-11-06-dream-proof-tools-landed.md #93377961366217 and final-freeze-20261106/verify-exit.json: exact full verify2; source unchanged, no new native or test runs after sole retry failure"
      }
    ]
  }
}
```

## Oracle status (2026-11-07 ~07:25): BLOCKED on design; builder seat closed, worktree retained
Report #93377961366234 (checkpoint d91ab927) records both positive refusals (Q = 1,026,291 ns and 1,440,792 ns vs 1 ms) with ledgers and load/QoS snapshots; the retry allowance is spent (0). The blocker is a DESIGN question, now [[tasks/2026-11-07-q2-member-window-quantum-design]] (Architect seat 01M6TE23PXGX5VN9SFP2H9NYDP). Builder seat 01M6S9GNN… was closed to free resources; the worktree /Users/feral/mydev/bboxy450-harness-dream-proof-tools and all evidence are KEPT. After the Architect's decision, a fresh executor resumes here: re-freeze if the source changes, otherwise run the authorized gates under the new accounting rule.


## Report from seat:01M6S9GNN1HB7PW20DSH1SY8BG

BLOCKED on the Oracle's design ruling01M6TE1SPJGYMG5T5JV76V32GD and tasks/2026-11-07-q2-member-window-quantum-design. Both correct positive refusals remain retained: original ledger18 Q1026291ns and sole QoS-conditioned retry ledger32 Q1440792ns versus unchanged1ms law; retry allowance0. Both controllers/helpers actually reaped. No further native runs, tests, builds, qualification, receipt or landing. Signed source d02cdd9fe3406d213699ec82473ef6d9309528b2 and clean worktree retained; full retry/error/QoS report #93377961366234 committed at d91ab92710026224b92edcb242fccb7d71db032e, preceding full report #93377961366217 committed at d4a96acd938bfaf645b99fb569eb24ee01e5b5e1. Report work is complete; worktree and all private evidence remain for the fresh Executor after the Architect's decision.

```json
{
  "op_id": "01M6TE9SZTEH0H1MYRGK8T26KP",
  "commit": "d02cdd9fe3406d213699ec82473ef6d9309528b2",
  "report": "[[reports/2026-11-07-dream-proof-tools-qos-retry.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Original final-frozen positive refusal, full retained ledger",
        "reference": "/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6T0EZ07D77BYZK7YNHP2A32/components/positive/output/ledger.ndjson:18; SHA413574d03deaf982b431a7b972b90cf9fb0745a133d55d3160a1d2abb110e01a; Q1026291ns>1000000ns, actual controller exit81"
      },
      {
        "criterion": "Sole QoS-conditioned positive retry refusal, full retained ledger",
        "reference": "/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6T0EZ07D77BYZK7YNHP2A32/components/positive-qos-retry/output/ledger.ndjson:32; SHA6b6ec65a0e628578f22be29f0d8dcb6ce57624b3c82a1b151cc13d163b90219c; Q1440792ns>1000000ns, actual controller exit81"
      },
      {
        "criterion": "Committed report, before/after load/QoS and per-code errors; original effects preserved",
        "reference": "reports/2026-11-07-dream-proof-tools-qos-retry.md #93377961366234; checkpoint d91ab92710026224b92edcb242fccb7d71db032e; target/dream-proof-tools/qos-retry-20261107/handoff.json SHA655918ee855e9122bd17440eb9f480271373b7397eb033cbf3e227af2267635d"
      },
      {
        "criterion": "Design dependency and explicit no-more-runs ruling",
        "reference": "Oracle message01M6TE1SPJGYMG5T5JV76V32GD; task latest Oracle status: BLOCKED on tasks/2026-11-07-q2-member-window-quantum-design, worktree/evidence kept, fresh Executor resumes after decision"
      }
    ]
  }
}
```

## ORACLE FREEZE, 66236 (2026-11-07 ~09:10): supersedes ALL earlier pins (35d3e602/7a753654/86c5c9b5/d66feb40 and older are RETIRED)
Q2 implemented decision 66236 (measured quantum Q = A_measured + C_active ≤ 1 ms once closure is proven; W ≤ 25 ms wall per grant; terminal conservation) plus the Oracle's implicit-child clarification (the leader's RI child roll-up under 5 conditions), and the ordinary-unit fix. Oracle-verified: Good/Boxy signature, contains 2ce4359d, hashes recomputed, compile root 0700, probe 0500, receipt 0400, clean tree.
- Source: signed `39580f0ab216c1bb9a74e7bbb42477b8dd60d993`
- C: SHA-256 `9b8fdd06d9940c52ed6fd9c164d620106ecfb8b70b1d9fae4a787cba40ad406a`
- Probe `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/measured-66236-signed-preparation/dream-read-c-preparation/compile-rcznyik8/probe`: SHA-256 `b57e79505b2da931fa877aa20954a5c129854cee7f272bc5bd1c5e5f4f947c61`
- Receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/measured-66236-signed-preparation/dream-read-c-preparation/compile-rcznyik8/preparation-53cdeac02bc1d74868d57feafb642975bbae1b3fb2b6313f02e32e9e5472d0fc.json`
- Codec b26d7cd3: schema d70948ec…, vectors bff32c1a…; read-set source-domain a212cd65… (byte snapshot only)
- Q2's independent audit: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/measured-66236-preparation-independent-audit.json` (SHA ba450694…, 672 checks, exit 0)
**Builder:** ingest exactly; the accounting rule is now 66236. Run the positive and the remaining component cases, then the final gates, qualification and receipt. The retry allowance is RESET for this new tuple to the normal policy: no unchanged-input timing retry; any refusal stops and goes to the Oracle with its ledger. runtime_approved stays false.


## Report from seat:01M6TPMTXM8MQCQXMCVBW60YGV

BLOCKED: ingested the exact 66236 tuple39580f0a/9b8fdd06/b57e7950/53cdeac0; signed a2828316 changes only its complete selection and source-lock pin. Unit56, offline ingestion, seven-source crypto and all60 build commands exited0, but the sole positive actually exited81 before grant1: engine_stopped_corroboration ordinary1164833ns>1ms and terminal conservation excess85291ns. Controller/helper actually reaped; raw four-line ledger/result/stderr preserved. Exact verify2; no timing retry, later cases, final tests, qualification or receipt. runtime_approved=false; clean tree and predecessor/evidence preserved. Oracle disposition plus causally changed authorized Q2 input/scope required. Full report #93377961366253 committed at knowledge eaf74dd427ae4a8516d39547eaed2dc245671114; blocker DM01M6TQ1AQ21ZKE7DJXBG6NZSM0 native-queued.

```json
{
  "op_id": "01M6TQ7EDXBHCE8WZDGRCNBZ53",
  "commit": "a28283160d146f4c215181e466daae00e95bdb4c",
  "report": "[[reports/2026-11-07-dream-proof-tools-measured-66236.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact 66236 frozen authority consumed, source/preparation/closure/custody identities retained",
        "reference": "scripts/dream-proof-tools.py:52 and tests/fixtures/dream-proof-tools/sources.lock.json:1; target/dream-proof-tools/measured-66236-20261107/ingestion.json; signed a28283160d146f4c215181e466daae00e95bdb4c"
      },
      {
        "criterion": "Sole positive refusal and full ordinary/conservation evidence; no grant-Q reclassification",
        "reference": "target/dream-proof-tools/runs/01M6TPSX095ZEEF53Q73KCW8N6/components/positive/output/ledger.ndjson:4 SHA2132c4e23198d85b7d448f81c15334832b20712b43b4ca02215640b4fda52409; ordinary1164833ns and conservation202000>116709ns"
      },
      {
        "criterion": "Actual controller/helper reap, unaltered result and complete error diagnostics",
        "reference": "components/positive/wait4.json SHA718e741023b3185b6c0bfc3348573420222a00422c7953498029836fb01071fb; stdout SHAeb53d08ee0f70f1e72a48bc7ae7c8d8dd3f468e62d54c22233ebdeb069b0cc97; stderr SHAe59ea758c526bba0311eb8853320a761c00589835b2f03dc8dfbb9bd9ea648b4; resource_limit2"
      },
      {
        "criterion": "Actual stage exits, unit56 and seven-source/all60-command successes; final gates explicitly not run",
        "reference": "target/dream-proof-tools/measured-66236-20261107/verify-exit.json and verify.log SHAb8fcc8b4f5fe15eb70e77476471511e844e43150759ca1ec39a4a7e2bb344f73; handoff.json SHA1966429dba05ab6220ecdfccb953cc76514fa8310a40f7b990ce8ea5b7431c1c"
      },
      {
        "criterion": "Committed full report, predecessor and historical failed effects preserved; stop and no runtime approval",
        "reference": "reports/2026-11-07-dream-proof-tools-measured-66236.md #93377961366253; knowledge checkpoint eaf74dd427ae4a8516d39547eaed2dc245671114; no later component/qualification/receipt; git diff --check0 and clean tree"
      }
    ]
  }
}
```

## ORACLE FREEZE, fixed-T (2026-11-07 ~12:40): supersedes ALL earlier pins (39580f0a/9b8fdd06/b57e7950/53cdeac0 and older are RETIRED)
Implements 66236 plus Oracle clarifications 1 (implicit-child roll-up), 2 (fixed teardown ceiling T = 150 µs per reaped process; residual charged in full) and the N_reaped definition, plus Q2's split of the engine_stopped_corroboration unit and its paid terminal-custody corrections. Oracle-verified: Good/Boxy signature, contains 2ce4359d, hashes recomputed, compile root 0700, source and receipt 0400, probe 0500.
- Source: signed `bec674fba7a823c36802d1ff3e4380a2ae3e25cb`
- C `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/fixed-T-66236-signed-preparation/dream-read-c-preparation/compile-vxqar2c6/proof-helper-probe.c`: SHA-256 `29d55d64120d6ce498316577af5c4c4650dae521d9a66dfb6e50284f6b342646`
- Probe `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/fixed-T-66236-signed-preparation/dream-read-c-preparation/compile-vxqar2c6/probe`: SHA-256 `d2deef94ba6f28236d5c2ef7ade4e757bbe5e615917b81f929f23632a5329ba3`
- Receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/fixed-T-66236-signed-preparation/dream-read-c-preparation/compile-vxqar2c6/preparation-d3d3cdd123adeb98179c3d9ce76297bef5297aa6a8a0d0618f3f0bd39828b022.json`
- Codec b26d7cd3 / schema d70948ec / vectors bff32c1a unchanged; four-Rust SourceSnapshot digest 7f7edaa9… (byte snapshot only)
- Q2 independent audit: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/fixed-T-preparation-independent-audit-detail.json` (SHA 59322c94…, 992 hash checks, exit 0)
**Builder:** ingest exactly; run the positive, then the remaining component cases, final gates, qualification and receipt. Any refusal: stop and DM the ledger; no unchanged-input retry. runtime_approved stays false.


## Report from seat:01M6TPMTXM8MQCQXMCVBW60YGV

BLOCKED: exact fixed-T tuplebec674fb/29d55d64/d2deef94/d3d3cdd1 ingested, including the task-named compiler input with full identity/signed-snapshot equivalence. Signed codeff2e650b; signed empty report-link checkpoint59a301e4 is clean HEAD. Unit58, offline ingestion, seven-source crypto and all60 build commands exited0. Sole positive actually exited81 at measured grant51: A932917+C245583=Q1178500ns>1ms; the second ordinary diagnostic is its total supplied-cost gate, not separate >1ms controller CPU. Fixed-T conservation passed, N_reaped1, residual120401ns retained once/paid. Controller/helper actually reaped. Exact verify2; no retry/later cases/final tests/qualification/receipt; runtime_approved=false. Oracle disposition and causally changed frozen Q2 input/scope needed. Full report #93377961366270 committed at knowledge531f67cf7ed3e7645e986c4c4fd686391c02951f; blocker DM01M6V1SGCYDTM412N98YF17GKC stored.

```json
{
  "op_id": "01M6V21QDM76CX1D1EPV2E82AQ",
  "commit": "ff2e650b694f33908dd34c18415cf0aee9041e41",
  "report": "[[reports/2026-11-07-dream-proof-tools-fixed-t-66236.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact fixed-T source/compiler-input/probe/receipt and complete custody/compiler bindings",
        "reference": "scripts/dream-proof-tools.py:52 plus strict compiler-identity/snapshot check; tests/fixtures/dream-proof-tools/sources.lock.json:1; target/dream-proof-tools/fixed-T-66236-20261107/ingestion.json; actual code ff2e650b694f33908dd34c18415cf0aee9041e41"
      },
      {
        "criterion": "Correct measured CPU refusal without retry; full raw ledger",
        "reference": "target/dream-proof-tools/runs/01M6V1HTRXVYXXW9CK3CWM2CT2/components/positive/output/ledger.ndjson:53 SHAc7a63a73efef4bf8a5cc4bb94013b8ddb75657f232bb3bca49e32f37f32edd5d; A932917+C245583=1178500ns; ledger54 actual246500/supplied1178500/finalcost1179417"
      },
      {
        "criterion": "Fixed-T conservation and full residual payment; actual controller/helper reap",
        "reference": "same ledger55–56: wait4_upper6265000<=6348433, N_reaped1, residual_cost120401 paid1; wait4.json SHA15ace01de7bf418f8f49f5773a189ea6076b8ae049a43da1903c5eade10a8cfc, actual controller exit81; helper18027 actual reap"
      },
      {
        "criterion": "Unaltered result, complete resource errors, passed persistence/terminal gates do not override failed control",
        "reference": "positive/stdout SHAc3d88573e491eecb02669ff4d0e612c4b64a3aa15b709d36af42d1e564fcfcd1 equals output/result.json; stderr SHA9ebe16ed96a7f923ffc7ba4486d96ffec2867c20924c4f81f586307a4f14dbfa has resource_limit2/control failed/terminal passed; final debt25453614753 paid"
      },
      {
        "criterion": "Actual command exits, complete report, preserved predecessors, no later cases/qualification/runtime approval",
        "reference": "fixed-T-66236-20261107/verify-exit.json; verify.log SHA6fd0511f184184222043999f92a8b800dec7c51e961ae8b3376c31f63ac567fd; handoff.json SHAb1a4917fd25c7cb62ff443ac1bbd986d51dbd16e7978d6c02a2dfec35fe98564; report #93377961366270 checkpoint531f67cf7ed3e7645e986c4c4fd686391c02951f"
      }
    ]
  }
}
```

## Oracle note (2026-11-07 ~13:20): the fixed-T tuple bec674fb / 29d55d64 / d2deef94 / d3d3cdd1 is RETIRED by its producer
Q2 found an Attach-panic / no-CONT settlement hang (an expected unactivated child reservation without a pre-return latch left terminal guard 149 held forever). Its builder run 01M6V1HTRXVYXXW9CK3CWM2CT2 stays recorded as a FAILED positive (grant sizing) and nothing was qualified on it. Wait for Q2's next tuple, which carries BOTH the settlement-latch fix and adaptive grant sizing; the Oracle re-freezes once.

## ORACLE FREEZE, adaptive (2026-11-07 ~15:20): supersedes ALL earlier pins (bec674fb and older are RETIRED)
Carries the settlement-latch fix (Attach-panic/no-CONT paid-UNKNOWN), ADAPTIVE grant sizing (physical positive passed in Q2's runs: Q = 805 µs, W = 438 µs, own wait4), fixed T = 150 µs conservation and 66236 with its clarifications. Oracle-verified: Good/Boxy signature, contains 2ce4359d, hashes recomputed, compile root 0700, source and receipt 0400, probe 0500.
- Source: signed `84158341180f011d09dc0e7d561f91b244d381ae`
- C `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/adaptive-maxlive-final-signed-preparation/dream-read-c-preparation/compile-v5v4t16m/proof-helper-probe.c`: SHA-256 `e23f68d3384418160fdc41813844876ec14b28b704bf7f916fad86a48dd992cc`
- Probe `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/adaptive-maxlive-final-signed-preparation/dream-read-c-preparation/compile-v5v4t16m/probe`: SHA-256 `05eefb2039d573e6d9fdd5b41e17db29f73615a04b9194c74e9f9f177572ae68`
- Receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/adaptive-maxlive-final-signed-preparation/dream-read-c-preparation/compile-v5v4t16m/preparation-3974b380a0ed86ce06087f79ac14bff150f4c31fc0717ffbfa09cba24f6ee47b.json`
- Schema d70948ec / vectors bff32c1a unchanged; source-domain dcf0e5df… (byte snapshot only)
- Q2 audit: prepare log 6d2f9015…, independent audit log d8655ef8… (992 hashes)
**Broken-control note (Oracle 14:50 ruling):** the scheduler-fixed-oversize negative is the separately labelled real C API control; the physical negative is DEFERRED/UNPROVEN, not a qualification prerequisite.
**Builder:** ingest exactly; positive, then components, final gates, qualification and receipt. Any refusal: stop and DM the ledger; no unchanged-input retry. runtime_approved stays false.


## Report from seat:01M6TPMTXM8MQCQXMCVBW60YGV

BLOCKED: exact adaptive84158341/e23f68d3/05eefb20/3974b380 ingested; signed913224f5 clean, unit58/offline ingestion/seven-source crypto/all60 build commands exit0. Sole positive direct GPG show_keys(form13) actually exited81 at grant10: measured A1093125+C171250=Q1264375ns>1ms. Scheduler requested126042ns; preCONT→STOP-call1087000ns, STOP→ACK107000ns; clocks alone do not establish cause. Conservation passed/residual955ns paid once; controller/helper actually reaped. Complete diagnostics retain resource_limit2 plus settlement_unknown1(final unreaped group kill unresolved), despite later recorded1/1/group-gone; no failure erased. Exact verify2; no retry/later cases/final tests/qualification/receipt, runtime_approved=false. Oracle disposition and changed authorized Q2 input/scope needed. Physical oversize negative stays deferred/unproven per14:50 ruling. Full report #93377961366278 committed at knowledge58cc002d9a182843572e31a9e2912372a011b718; blocker DM01M6VAR8TRB1Z0XX39YW0PKA3Y stored.

```json
{
  "op_id": "01M6VAYY2YPDK2X0E1BKR2FTG9",
  "commit": "913224f577368c9bbc70c9afce57a91e099957c6",
  "report": "[[reports/2026-11-07-dream-proof-tools-adaptive.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact adaptive source/probe/receipt and actual compiler-input/custody bindings",
        "reference": "scripts/dream-proof-tools.py:52; tests/fixtures/dream-proof-tools/sources.lock.json:1; target/dream-proof-tools/adaptive-20261107/ingestion.json; signed913224f577368c9bbc70c9afce57a91e099957c6"
      },
      {
        "criterion": "Sole positive measured quantum refusal, adaptive prediction and actual timestamps",
        "reference": "target/dream-proof-tools/runs/01M6VAGHD4Y6MXHSASDMN15XE9/components/positive/output/ledger.ndjson:21,25 SHA0ee1e452c2cda20a2faf816a831218ec49055bb7d055583cefba2d03e8185585; Q1264375; requested126042/preCONT-to-STOP1087000/STOP-to-ACK107000ns; no retry"
      },
      {
        "criterion": "Conservation/residual payment and actual reaps; earlier failed kill effect preserved",
        "reference": "same ledger22–24: conservation_passed1, wait4_upper2426000<=2587545, residual955 paid once; wait4.json SHAad380cc9c90bda1b7e3f5f315fb1871b4a8bc86d37553e280cdc238c135b3148 actual controller exit81; stderr SHA5767948c2a80dd1b1c9d6f711d4be8907b6e1bb3bec42a33963adb5c444b7684 retains resource_limit2/settlement_unknown1"
      },
      {
        "criterion": "Unaltered unapproved result, real exits and complete retained evidence",
        "reference": "positive/stdout SHA36c74dbc2ddb9eeadba6586c9dd0fc068deb8dd39559c7d24b1727d5d3f3144b equals output/result.json; adaptive-20261107/verify-exit.json and verify.log SHA79c8cd10075277abe996aa3c442390d8ead0c010354cdc14edeb9a1ee7425983; handoff.json SHA7621577553dfe62673b31b8796c5e351ae5d7adfa5ab863af7ead55113960aec"
      },
      {
        "criterion": "Committed report, current final gates not run, predecessors preserved and physical oversize limitation explicit",
        "reference": "reports/2026-11-07-dream-proof-tools-adaptive.md #93377961366278; checkpoint58cc002d9a182843572e31a9e2912372a011b718; only positive exists, no qualification/receipt, all4 historical ledger hashes unchanged; current formal/integration/licenses not claimed"
      }
    ]
  }
}
```

## Oracle note (2026-11-07 ~18:15): the adaptive tuple 84158341 / e23f68d3 / 05eefb20 / 3974b380 is RETIRED by its producer
Decision 66280 supersedes per-grant slicing for external helpers, so this tuple cannot qualify as current. Its builder run 01M6VAGHD4Y6MXHSASDMN15XE9 stays FAILED. Wait for Q2's single 66280 tuple (total CPU/wall/memory budgets, background-QoS spawn gate, pending-terminal final kill, Q2HelperTotalBudget with 8 causal controls); the Oracle re-freezes once and spawns a fresh builder.

## ORACLE FREEZE, decision 66280 (2026-11-07 ~22:30): supersedes ALL earlier pins (84158341 and older are RETIRED)
Implements decision [[decisions/d93377961366280-dream-q2-external-helper-budget]]: external Git/GPG helpers are NOT 1 ms-sliced; spawn with posix_spawn + SETPGROUP + background QoS (observed priority 4 is a HARD gate); CPU caps ≤250 ms per command and ≤3 s per job via live RI plus SIGKILL; the leader wait4 is the terminal verdict; 9× rest debt after the reap; 30 s wall; ru_maxrss ≤16 MiB; foreground = one SIGSTOP within 100 ms; final-kill EPERM on a zombie-only group handled as 66050 pending-terminal. Oracle-verified: Good/Boxy signature, contains 2ce4359d, hashes recomputed, compile root 0700, receipt 0400, probe 0500.
- Source: signed `557c8b47a474373acd42e759a7f7ac3efc72fc39`
- C: SHA-256 `eca43c1bbf4a165119ead5c7da2cc1c24ff25914c00ef25c172096f8b4595b0b` (retained at `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-66280-source46/dream-read-c-preparation/compile-lnfh07dr/proof-helper-probe.c`)
- Probe `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-66280-source46/dream-read-c-preparation/compile-lnfh07dr/probe`: SHA-256 `ac4df370e9e108b9d7c65cf83b1a8a5ecfde6f15650fc172a31eb6909b3c7cdc`
- Receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-66280-source46/dream-read-c-preparation/compile-lnfh07dr/preparation-ec4c2eee90bc74d0cae67991b145e4fc99c729898cc56e1cddb1964490a48326.json`
- Schema d70948ec / vectors bff32c1a unchanged; read-set source domain 5837e5ab… (byte snapshot only)
- Q2 independent audit `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/66280-preparation-independent-detail-v46.json` (SHA d5fab0b9…, 997 hash checks, exit 0)
**Builder (fresh seat):** ingest exactly. Qualification now checks 66280 TOTAL budgets (no per-grant Q). Run the positive, then components, final gates, qualification and receipt. Any refusal: stop and DM the ledger; no unchanged-input retry. Q2 notes that the B startup barrier and the real-tool qualified roll-up remain unproven, and missing proof refuses; report them honestly. runtime_approved stays false.


## Report from seat:01M6W0XSKRWPBS72T4MJZ0WGT7

BLOCKED: exact 66280 tuple 557c8b47/eca43c1b/ac4df370/ec4c2eee ingested; signed 12d72305 clean. Unit 59, offline ingestion, seven-source crypto and all 60 build commands exited 0. Sole positive controller 45375 actually reaped exit 81 after 30 s deadline; stdout empty and result.json absent. Ledger records two PRI 4 helpers, terminal wait4 21.965/20.967 ms, total 42.932 ms, both reaped/group-gone/debt paid; final diagnostics retain deadline 18, settlement_unknown 1, source_changed 1 and incomplete persistence. Exact verify exit 2; no retry, later cases, final gates, qualification or receipt; runtime_approved=false. Need Oracle disposition and causally changed authorized Q2 input/scope. Raw evidence and all six historical ledgers retained. Full report #93377961366298 committed at knowledge a29b41ca9050a8f4f9d9d2b353cd9c45168d58f3; blocker DM 01M6W1DJPE26VXCWQR73FXZAV6 native-queued, consumption unverified.

```json
{
  "op_id": "01M6W1JKCD9JQ4VHTWHNYXWFVH",
  "commit": "12d72305bf74abbadd46ae27abb91b508f40cd89",
  "report": "[[reports/2026-11-07-dream-proof-tools-total-66280.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact frozen authority and compiler/header/SDK/custody bindings",
        "reference": "scripts/dream-proof-tools.py:52; tests/fixtures/dream-proof-tools/sources.lock.json:1; target/dream-proof-tools/total-66280-20261107/ingestion.json SHA e2022c3cbf3dcd1cba3955d3dd977f9692907d8e1302cddf52f889999f7ed1a7; signed 12d72305bf74abbadd46ae27abb91b508f40cd89"
      },
      {
        "criterion": "Decision 66280 total-budget acceptance, retired Q/late_stop removed, actual unit controls",
        "reference": "scripts/dream-proof-tools.py:2953,2983,3115,4253; unit-current.log SHA 7d65cf5412d9fb25053696d811cccbf422dc62c5148c71c74e284e82fb41040f; 59 tests exit0"
      },
      {
        "criterion": "Sole positive deadline refusal and preserved raw evidence",
        "reference": "target/dream-proof-tools/runs/01M6W168YT0EFZRE5EHTE2MYTW/components/positive/output/ledger.ndjson:2,7 SHA c1a78d9d4c52b176e0d385b793f5625e0a9bfad61c55789f38b84cb74f42202c; stderr SHA 898575447c242bfd48e9fcdca86ef9332e5e80470107ad79b35bd874ede59e33; empty stdout, result.json absent"
      },
      {
        "criterion": "Actual controller wait4 and recorded helper settlement/debt without claiming proof completion",
        "reference": "positive/wait4.json SHA d8d643db2ba65362eb5a7377afcb634b90968a00340afd2fe2c4f0bc0ed9dd25 actual exit81; ledger helper CPU 21965000/20967000ns, PRI4, ownwait4/group-gone/paid; terminal helper2/2 and debt25838388000ns paid"
      },
      {
        "criterion": "Real exits, predecessor preservation and explicit remaining gates/qualification limits",
        "reference": "total-66280-20261107/verify-exit.json exit2; verify.log SHA 85ec52189a2227677bf72a34e55030d3dd5a71f0c914e007e7a3632e64444407; handoff.json SHA 61e1c214e342bf7222e827c3972bd3994d650b1f9132b3db479afea05a637f97; all six historical ledgers unchanged, no later cases/final gates/qualification/receipt"
      },
      {
        "criterion": "Committed full append-only report and clean signed worktree",
        "reference": "reports/2026-11-07-dream-proof-tools-total-66280.md #93377961366298; knowledge checkpoint a29b41ca9050a8f4f9d9d2b353cd9c45168d58f3; git diff --check0 and git status --short empty"
      }
    ]
  }
}
```

## Oracle note (2026-11-08 ~00:00): provisioning read-set refresh RECORDED, no run
Q2 refreshed the current read-set to signed `622491aff7ee6f2e73e4a9f972bddba4be18851a` (receipt `bf16e4897c31525d3ac7ccd2ce8038619395d41fb9c73b8d39caf34812133098`; read-set domain 177cff0e…). The C probe and executable are byte-IDENTICAL (C eca43c1b, probe ac4df370); only a test-only read epoch-target correction changed (diff vs 557c8b47: 3 test files). Oracle-verified: Good signature, probe and receipt hashes. It does NOT yet fix the run 01M6W168YT0EFZRE5EHTE2MYTW failure (a 30 s job-deadline hang after the helpers finished: source_changed on the descriptor pathname, settlement_unknown on stdout emission, result.json absent). So NO builder run is started; the next freeze comes with that functional fix.

## ORACLE FREEZE, result-path (2026-11-08 ~01:15): supersedes ALL earlier pins (557c8b47 / 622491af and older are RETIRED for current qualification)
Decision 66280 budgets plus the result-path fixes from the run 01M6W168YT0EFZRE5EHTE2MYTW failure: input bound to the admitted fd/dev/ino/mode/size plus a fresh bounded SHA (a replaced or edited source refuses); the paid deadline refusal is separated from native pathname failure; the 30 s work deadline is unchanged, with one-way terminal-only reporting after expiry (no new engine work; finite paid refusal/file/stdout attempts with actual errno retained); bounded path walking under 400 µs checkpoints. Oracle-verified: Good/Boxy signature, contains 2ce4359d, hashes recomputed, root 0700, receipt 0400, probe 0500.
- Source: signed `79fa0d9d266cc9ccda367dc340aeae12d2c76964` (pin-only checkpoint 87985698… implements the 00:40 upstream refresh)
- C: SHA-256 `33ce96860b0a8ba4d3bbfd984d5649348878ef3bf8ea51e9a37e6469a8f23b4e`
- Probe `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-result-path-source50/dream-read-c-preparation/compile-r6zut56v/probe`: SHA-256 `f74ab7346d851f92dd0a428ab267dc0fc0d0cb464c63c24f2f3da450318e549c`
- Receipt `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-result-path-source50/dream-read-c-preparation/compile-r6zut56v/preparation-8708273f762772745777aae92498483b79e97f587cb672692b7cf467dc72473e.json`
- Q2 independent audit: `…/result-path-preparation-independent-detail-v50.json` (SHA 602974f2…, 997 files checked, exit 0)
**Builder (fresh seat):** ingest exactly; positive under 66280 TOTAL budgets; then components, final gates, qualification and receipt. Any refusal: stop and DM the ledger; no retry. B startup barrier and real-tool roll-up are expected unproven (refuse honestly). runtime_approved stays false.

**Oracle note (~01:40):** the builder's offline finding (the new C terminal record has 13 fields absent from the builder's exact whitelist; C 3934 vs Python 3024) is a schema-sync gap. The builder is authorized to extend its exact whitelist ONLY by the 13 field names the frozen C source emits, each typed and bounded exactly as in the signed C/schema, with a negative test that an unknown extra field still refuses. The positive refusal itself was the debt-formula bug, routed to Q2.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

BLOCKED: exact result-path freeze 79fa0d9d/33ce9686/f74ab734/8708273f consumed; signed 5b8a1108 clean. Offline ingestion, 59 units, seven-source crypto and all60 build/config/link commands exit0. Sole positive run 01M6W6RKAZEN3P4CHXXEPKPQNS controller36779 actually reaped exit81: deadline, proof not reached, 6/6 helpers settled/PRI4, total wait4 CPU64.902ms, final debt25.829118s paid. Raw stdout=result.json1998B; persistence/stdout errno0; terminal deadline18 only. Exact verify2; no retry/later cases/final gates/qualification/receipt; runtime_approved=false. Need Oracle disposition plus causally changed authorized Q2 input/scope; separate new13-field terminal adapter mismatch recorded (not reached failure). Full report #93377961366307 committed at knowledge d02773298e29c355d86dbc7946667b009a0f2723. artifact_claim nevertheless refused artifact_not_committed op01M6W74GX5EQWWQ8PG6W088TJT; no claim asserted. Blocker DM01M6W6YTG77Q0MRGMEVD5S0J3Y native-queued, consumption unverified.

```json
{
  "op_id": "01M6W75SVW13K2GRCHQQG2N25B",
  "commit": "5b8a11083f7d0edfe7ff5f3187578fd8c09af6d1",
  "report": "[[reports/2026-11-07-dream-proof-tools-result-path.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact Oracle-frozen source/probe/receipt and compiler/header/SDK custody binding",
        "reference": "scripts/dream-proof-tools.py:52,2342; tests/fixtures/dream-proof-tools/sources.lock.json:1; target/dream-proof-tools/result-path-20261107/ingestion.json SHAb3494ab59e18804b6e8bbe49462da857d08eb70bddefd59ffae926115327abdd; preflight.json per-command0"
      },
      {
        "criterion": "Fresh source crypto, full source/config/link build and offline controls",
        "reference": "runs/01M6W6RKAZEN3P4CHXXEPKPQNS/authenticated.json SHA5c3162508f76753f4812a2219c57be65fdace3424ce0f02e8b7439f0819dcb52; all60 logs/*.exit.json exit0; result-path-20261107/unit.log SHA744f2190bea2e9e5137879869cf7ed6919ef66175642dd38a0443189c0a797b9,59tests0"
      },
      {
        "criterion": "Sole positive actual refusal, raw unchanged output and settled effects",
        "reference": "runs/01M6W6RKAZEN3P4CHXXEPKPQNS/components/positive/output/ledger.ndjson SHAba5b53d8c53b2cb5fd7fc9a380ae9dd39e11322c9b4b80436c55a62da1925829 lines2,7,12,17,22,27; wait4.json SHA98d9bac7b0b022ec4fa40d39709cadf7e658ccc3da361b2dc9b53741217becb4 exit81; stdout=result.json SHAe48546d70ea66cc72ee2a8941fc11a32ff0066cd3ad777c00d2be0d92ca2d121"
      },
      {
        "criterion": "Deadline-only terminal/persistence/debt result and no later gates or runtime claim",
        "reference": "positive/stderr SHA33206a5b1bdbec84a19e782db2a4826527da37ba0f61a2bd2f09f1f7f924fc92; result-path-20261107/verify-exit.json exact2; verify.log SHA317f010e49125386ac1f569e26506eeffa9182b10e372a09c66db3d2f77b6704; qualification/receipt absent"
      },
      {
        "criterion": "History preservation, unchanged original incarnation/inode and independent consumer gap",
        "reference": "result-path-20261107/handoff.json SHA9750b33b12c6f1f8b3edfd274ab756f4d77d11a21edec38fe0b0d9324fb7e961; all7 old ledgers unchanged; terminal-shape.json C3934/Python3024 thirteen extra fields, no checker reached/replay"
      },
      {
        "criterion": "Committed append-only full handoff, signature and clean writing tree",
        "reference": "reports/2026-11-07-dream-proof-tools-result-path.md #93377961366307; knowledge d02773298e29c355d86dbc7946667b009a0f2723 publication committed; signed5b8a11083f7d0edfe7ff5f3187578fd8c09af6d1; git diff --check0,status empty"
      }
    ]
  }
}
```


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

Authorized offline sync DONE; full task remains stopped for next Q2 freeze. Signed 5c717eac changes only scripts/dream-proof-tools.py: exact13 terminal names, frozen boolean/flag/u32/u64/first-unit bounds; unknown/missing fields still refuse. Unit61 exit0; offline comparison matches exact frozen terminal fields and still rejects the unchanged real deadline. All prior raw effects/evidence preserved, git diff --check0/status clean. No new candidate/probe/build/timing/parity/qualification/receipt/landing or runtime approval. Full canonical command was not rerun under this offline-only amendment; retained last actual 5b8a1108 verification remains2. Committed append-only report #93377961366309, knowledge checkpoint 5b432afa5151c8ba1435732ff8124175474b2fe8. Parked for next authorized freeze; Oracle-routed debt bug remains Q2-owned.

```json
{
  "op_id": "01M6W7EGCNRMNKY9WXQNER82FC",
  "commit": "5c717eacf0d632897436923899c48bf1d39c4a2a",
  "report": "[[reports/2026-11-07-dream-proof-tools-terminal-sync.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Offline amendment exact thirteen-field whitelist and frozen source types/bounds",
        "reference": "scripts/dream-proof-tools.py:3024; source SHA c230a8bde757fb8e21efc92eed37d001e7fc61387ca59614893eeeb5d40dfc77; target/dream-proof-tools/terminal-sync-20261107/offline-compatibility.json SHA f86b1c49b0ff516f522d7eef530170a001134871f6b463d115d2163169d814ac"
      },
      {
        "criterion": "Required unknown-field negative, full offline parser controls and boundary tests",
        "reference": "scripts/dream-proof-tools.py:4404,4420; unit command exit0,61tests; terminal-sync-20261107/unit.log SHA603e397c6ef3034b3205f30e3c700fd36f3728cef79137cab36b86237a1f083f"
      },
      {
        "criterion": "No new timing attempt; unchanged real deadline remains refused and original evidence preserved",
        "reference": "terminal-sync-20261107/preserved-evidence.json SHAd37c1b5a61207d7c0d8348b87c326ee3cff16be3bd7a04b94c3d65dad209015e; offline-compatibility.json refuses raw deadline as canonical independent control verdict; prior result-path full command2 retained,not rerun"
      },
      {
        "criterion": "Signed clean handoff with explicit full-task gaps and committed report",
        "reference": "signed5c717eacf0d632897436923899c48bf1d39c4a2a; terminal-sync-20261107/final-state.json signature0,diff-check0,status empty; report#93377961366309 checkpoint5b432afa5151c8ba1435732ff8124175474b2fe8"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT54 (2026-11-07). Supersedes the result-path freeze as the current C input.

Verified by the Oracle from the tree:
- Source a6468cc3473a8a6f6b76cd5dd48e97d90c6a0b42: `git verify-commit` Good, Boxy 9B07E849…7AB0. 2ce4359 is an ancestor. Schema/vector authority b26d7cd3 Good and an ancestor.
- Probe: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-cpu-debt-source54/dream-read-c-preparation/compile-qiog_m7e/probe, 251016 B, sha256 a1f58b34d93ee821486c5e5bc9b844f4ccecca2183ff5f4501733437ad531abd, mode 0500.
- Receipt: preparation-c2d3b8c0dce891e0a60109602217ceefe019beb96a826ea77bc035ed400909ca.json; its sha256 matches the name.
- Files and directories: root 0700; 0 files outside modes 0400/0500; 0 files with nlink>1; 0 directories outside 0700.
- All 10 named sources match `git show a6468cc:<path>` byte for byte. Seven are in source-a6468cc…; Makefile, check-c-preparation.py and prepare-c-probe.py are in preparation-a6468cc….
- Independent audit file: sha256 20b55baa…e474, matches.

Correction accepted. 79fa0d9d already billed helpers at 9×max(1 ms, CPU). The 25.83 s debt was 0.584 s of helper debt plus 2805 × 9 ms floors from tiny currentness units, so there was no wall-multiplier bug. C14 batches those currentness checks into one ordinary unit, below 400 µs / 64 steps. Run 01M6W6RKAZEN3P4CHXXEPKPQNS stays FAILED 81. runtime_approved=false is unchanged.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: rerun qualification from the positive using exactly this tuple.

## ORACLE RULING, INPUT54 whitelist and ancestry correction (2026-11-07)

Correction: the INPUT54 freeze said b26d7cd3 is an ancestor of a6468cc. That is wrong: `git merge-base --is-ancestor` exits 1, which the builder found. The schema/vector authority is blob identity. At both commits, closure-boundary-vectors.json is blob 54cfab3b0e6cc8996bbc942cc2b0128278fccfaf and closure-boundary.schema.json is blob 96e2839a9d2b6031fb0138934eef96850e9b7a60. The b26d7cd3 signature is Good. INPUT54 stands; the tuple is unchanged.

Whitelist amendment: the checker additionally accepts exactly these 10 terminal fields from frozen proof-helper-probe.c (~line 3970 in a6468cc), and no others:
- u64: booked_ordinary_units, engine_charge_calls, poll_closure_calls, ordinary_cost_ns, ordinary_raw_cpu_ns, ordinary_floor_surcharge_ns, helper_cost_total_ns.
- Booleans `true`/`false`: ordinary_raw_complete, debt_components_match.
- Literal: debt_scope=whole_controller_cumulative (any other value is refused).

Unknown, missing and duplicate fields are still refused. Add unit tests for each new type and literal and a refusal for an extra unknown field. Then run the sole native attempt with INPUT54.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT54 PREFLIGHT BLOCKED before any native attempt. Exact a6468cc3/8a99da16/a1f58b34/c2d3b8c0 ingested and pinned in signed a42a789e;61 offline units exit0; tree clean,8 old ledgers unchanged. Frozen C3970 introduces10 unconditional terminal names outside the task's explicit ONLY13 parser scope, guaranteeing exact-whitelist refusal. git b26d7cd3→a6468cc ancestry exits1 despite freeze claim; source signature/2ce4359d ancestry exit0, all10 signed source and2 authority blobs match (12 command checks exit0). Need task amendment for exactly10 typed diagnostics and disposition of false ancestry, or corrected tuple. No source fetch/build/candidate/probe/components/final gates/qualification/receipt/landing; runtime_approved=false. Full canonical command NOT RUN on a42a789e; last actual verification remains prior result-path2. Report#93377961366317 committed knowledge84427be75899a4479a19fadc979e5c5b7a8239b5. Blocker DM01M6WC82Z40E9C7XMNCX51K4YZ native-queued, consumption unverified. Prior Oracle debt-bug diagnosis retracted per INPUT54; historical run staysFAILED81.

```json
{
  "op_id": "01M6WCFJ254K435T69KFT1SH9D",
  "commit": "a42a789ecef84b2778463de290274aa15596ad24",
  "report": "[[reports/2026-11-08-dream-proof-tools-input54.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact INPUT54 source/executable/preparation ingestion and signed pin",
        "reference": "scripts/dream-proof-tools.py:52; sources.lock.json:1; target/dream-proof-tools/input54-20261108/ingestion.json SHAe48bae872e26e6e12b339822bc9f4f7ef6df22a51e6d6b43a9ec2b467b719cfc; selection.json SHAe2d739ffd094df040f0757eb6afa154267b604669569a76346068f2e51392dca"
      },
      {
        "criterion": "Real signature/ancestry and immutable source/codec byte evidence",
        "reference": "input54-20261108/preflight.json SHA3f602891f61940b8c59489c356350b081a58b6360bc2f3860fc4ec58e3db795a exits0,0,1; signed-source-byte-checks.json SHA39ba2b8a6f7015b9016ca7827e83d7934d5021fe43b180aa027e43389c1914af 12checks0"
      },
      {
        "criterion": "Concrete incompatible terminal scope found without a native attempt",
        "reference": "input54-20261108/terminal-gap.json SHA06b04896f637ee88db368f141ba1c4bb25432cc34f2ea5a5feeb4bbf5803458d; frozen C3970 extra10 vs script3024; current ONLY13 task clause preserved; no native launch"
      },
      {
        "criterion": "Offline controls and preserved prior failures; no fresh full verification claim",
        "reference": "input54-20261108/unit.log SHA65db2524f399132fbffe7f9d5fd3abc9b25f4441219927fc784cb65d7ef9770c 61tests0; predecessor-ledgers.json 8unchanged; full command not rerun, retained prior result-path-20261107/verify-exit.json2"
      },
      {
        "criterion": "Signed clean handoff and committed append-only report",
        "reference": "a42a789ecef84b2778463de290274aa15596ad24 Good/Boxy; input54-20261108/final-state.json SHA69d89fd645f3846806bd919b93151650aeeb7eb103a04968fa5b826858c9d299 signature0,diffcheck0,status empty; report#93377961366317 checkpoint84427be75899a4479a19fadc979e5c5b7a8239b5"
      }
    ]
  }
}
```


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

BLOCKED after the sole INPUT54 native positive. Signed9081e326 implements exactly10 authorized typed fields with closed unknown/missing/duplicate controls;62 offline tests0. Corrected blob authority and complete exact a6468cc3/8a99da16/a1f58b34/c2d3b8c0 ingestion0; seven-source crypto and all60 build commands0. Run01M6WCKTFETFJ7KPVM42K4MZEK controller64265 actualwait4 exit81, resource_limit='fixed argv environment or effect preparation refused', phasegit_reads, proof not_reached,12/12 helpers reaped/group-gone/PRI4, helperCPU82.997ms, final debt19.826973s paid. Ledger /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6WCKTFETFJ7KPVM42K4MZEK/components/positive/output/ledger.ndjson SHA02ae389da7a20f2bad4f73d5fbf48449c19d6d001740db387d3504faad08023e. Source inference: form10 tree_pin {commit}^{tree} hits unrecognized {tree} in engine_expand C2882-2887 before next birth; generic raw error itself does not isolate expression. Exact verify2; no retry/later cases/final gates/qualification/receipt/landing; runtime_approved=false. Need Q2 disposition/fix plus causally changed frozen tuple. Full committed report#93377961366320 knowledgeacb1f7cfc176425b1d53588d9539ab120f0a1672; clean signed tree;8 old ledgers unchanged.

```json
{
  "op_id": "01M6WD4S07D9H33B92ND092T61",
  "commit": "9081e326c3703bd73c36e3eb32a4085c2edd2458",
  "report": "[[reports/2026-11-08-dream-proof-tools-input54-attempt.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Authorized exact10 parser fields and corrected blob authority",
        "reference": "scripts/dream-proof-tools.py:3024; input54-attempt-20261108/offline-binding.json SHA6796d0a829b197c4b1b329be8a5449b83a65fe3420033cb9cf97073331d620b1 exact10 additions/no removals,producer fieldset,four blob checks0; unit62exit0 logSHAd98cead13288267f9d368991dbacc4c80487fd16c0f05c38f228a8bb9df27485"
      },
      {
        "criterion": "Exact frozen input, authenticated sources, private source/config/link build and basis",
        "reference": "input54-attempt-20261108/ingestion.json SHAe48bae872e26e6e12b339822bc9f4f7ef6df22a51e6d6b43a9ec2b467b719cfc; runs/01M6WCKTFETFJ7KPVM42K4MZEK/authenticated.json SHAa35b5797f541173cc1e9a3c1451aa44c64cc9575d050d457f2ea26c85235f8b7; all60 logs exits0; basis47112B SHA f3df0066bf3e16cc572d1c72253fa8d69c877b1357149098bebcdd0b6f17805b"
      },
      {
        "criterion": "Sole positive actual refusal, unaltered output, own reap and complete known settlement",
        "reference": "positive/output/ledger.ndjson SHA02ae389da7a20f2bad4f73d5fbf48449c19d6d001740db387d3504faad08023e 12ownwait4/PRI4/group-gone helpers; wait4.json SHA7ab3202b190576bee1b25f835e17a6c8d876da7712d3755b43be4192c3f1440d exit81; stdout=result.json SHA90d4d708193419795891802d05654632ab1bfb16d9722aa81d07d47f31a10369"
      },
      {
        "criterion": "Actual terminal/debt/persistence evidence and scoped source diagnosis without a repeat",
        "reference": "positive/stderr SHA571468fbffe24db67852707d849aa694f33b2aae44d83c0123eb7105efecb63e resource1 only,errno0,debt19826973000paid,exact fields; input54-attempt-20261108/source-diagnosis.json SHA4202a17bc5fd461e9149080eeb252848b099aa73e2bdb6e05a6358a5826c62b9 static inference C1318/2882-2887/2906/3243/3599"
      },
      {
        "criterion": "Exact final verify failure, preserved history and explicit missing gates",
        "reference": "input54-attempt-20261108/verify-exit.json2; verify.log SHAa4a2509f0db0d5b27bf45a69657606044313606f392aa5f58bda1358d546be89; handoff.json SHAc34ba6b9a35f9c4ac6161d6192b75ab9ffbf62b9ca2e38a7d4e6d44f99203ff4 8oldledgers unchanged,onlypositive,qualification/receipt absent,no final parity or product claim"
      },
      {
        "criterion": "Committed full report, source signature and clean worktree",
        "reference": "reports/2026-11-08-dream-proof-tools-input54-attempt.md #93377961366320 knowledgeacb1f7cfc176425b1d53588d9539ab120f0a1672; signed9081e326c3703bd73c36e3eb32a4085c2edd2458; final-state.json SHAb44cb7ff4922836957cce9210d690ee650a0992173bd7db8b2880e56e66cf4dc signature0,diffcheck0,status empty"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT58 (2026-11-08). Supersedes INPUT54 as the current C input.

Verified by the Oracle from the tree:
- Source ba22c67c488421f58faff3e5256f7fcfff6a5ab8: `git verify-commit` Good (Boxy). 2ce4359 and landed main d1fe3f4b are ancestors.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-tree-pin-source58/dream-read-c-preparation/compile-0l7qmwgc/
- Probe: 251016 B, sha256 0759cc153f74ae23c22eb2e952aa7b0af149dfd7102af595bafaa6765c533bde, mode 0500.
- Receipt: preparation-e73178e5d779cca4d4ec1b0d6e1afd203746e8a2bad032e243d276f371d64ef3.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 named inputs match `git show ba22c67:<path>`. proof-helper-probe.c is now 386cf55e…; the other nine are unchanged from INPUT54.
- Schema/vector blobs are identical to b26d7cd3 (blob authority; not an ancestry claim).
- Independent audit file: sha256 7a4ead81…, matches.

Change since INPUT54 (proof-helper-probe.c, engine_expand only): named placeholders are expanded and any other {…} is copied literally, so {commit}^{tree} now yields exact git argv. This is broader than my tightened ruling, which asked to refuse unknown braces. I accept it because every input to engine_expand is a compile-time constant (fixed prefix, 16-form registry, env values), and Q2's 15-form loop runs the real engine_prepare_argv and checks the exact argv. Upfront bans on {git_signature_temp} and {signature} remain. The 10 terminal field names are unchanged from INPUT54.

Not proven: Q2's 703 ms seam run was scripted, with no real helper births. Whether a full real positive fits 30 s is decided by the builder's run.
runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt, from the positive, using exactly this tuple.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

BLOCKED: exact INPUT58 ba22c67c/386cf55e/0759cc15/e73178e5 ingested; signed f6cdc7c4 clean. Source/main ancestry, all10 input bytes/nine unchanged, codec blobs and terminal compatibility verified;62 units0, fresh7-source crypto0, all60 build commands0. Sole run01M6WMTMXKA8BFZHWGQP7FX028 positive controller52183 actualwait4 exit81: first creation nativewall34.686ms>25ms (excess9.686ms), CPU0.563ms/inclusive0.689ms; proof not_reached. Helper52844 killed/own-reaped/group-gone; QoS/roster unobserved, not PRI4 evidence. Final debt7.380s paid = helper9ms + ordinary rawCPU474.276375ms + ordinary floors6896.723625ms; source raw-complete/debt-match true, arithmetic matches; no deadline. Ledger /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6WMTMXKA8BFZHWGQP7FX028/components/positive/output/ledger.ndjson SHA99d0a76574a689ec4006bb0e90754cd57cb62739c7ea943343afd29fd369a720. Exact verify2; no retry/later cases/final gates/qualification/receipt/landing; full-positive30s fit remains unproven, runtime_approved=false. Need Oracle disposition and causally changed authorized input/scope before further attempt. Report#93377961366329 committed knowledge0c378625c93031635d8893ba08bad123aa528d3d;9 old ledgers unchanged.

```json
{
  "op_id": "01M6WN8GFCNEEZ4BA3SRNE9E0C",
  "commit": "f6cdc7c4769e82dd0999cdf9e40caf37e2f1a28e",
  "report": "[[reports/2026-11-08-dream-proof-tools-input58.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact INPUT58 immutable selection, signed provenance, unchanged input/codec/terminal compatibility",
        "reference": "scripts/dream-proof-tools.py:52; sources.lock.json:1; input58-20261108/preflight.json SHAb70adfada1350cd75a8b3058bd6a4ed35aa0626f67a80d21784997263eba8b62 all0; ingestion.json SHA8b3f504eea0cecaee8e01cbd4ccd79d3925825979c093c4a035cf6c02826bb36; source-byte-checks.json 12checks0"
      },
      {
        "criterion": "Existing strict controls, actual fresh source authentication and private build",
        "reference": "input58-20261108/unit.log SHA08d4b6b2ad4c7724ca4817851f321dce1f4aeb308b3ee2c87f3feb282f72d6dd 62tests0; runs/01M6WMTMXKA8BFZHWGQP7FX028/authenticated.json SHA07732fe371140a638be58fce3c058e8951f65b1e4ab97adb0528a90806bfb128 7crypto0; all60 logs exits0"
      },
      {
        "criterion": "Sole known native-wall breach and original actual settlement, no hidden timing retry",
        "reference": "positive/stdout=result.json SHAaad80f30b14fc2a77b26253e21a6f31009c3c7da2ebc1e2031294748bdb61575 nativewall34686000ns; frozen C2969-2985 gate; positive/wait4.json SHAa801048690e1ac2507a2091fde961daae03c1904ed858a2e1f2236e936570526 actual81; ledger SHA99d0a76574a689ec4006bb0e90754cd57cb62739c7ea943343afd29fd369a720 actualownwait4/groupgone,QoS unobserved"
      },
      {
        "criterion": "Requested final helper/ordinary-floor debt split with paid raw persistence",
        "reference": "input58-20261108/debt-split.json SHA7a170165380b81306d7449c632215e12803af58c9ed76ea44cdb4bf56220011d: helper9000000ns,ordinary raw474276375ns,floors6896723625ns,total7380000000paid; stderr SHA2dab1bf8f23b465da4645537166a88be8ce5e627956d6dfb72a2bb9005ff6e94 resource1 only,errno0"
      },
      {
        "criterion": "Exact verification failure, preserved history and explicit outstanding gates",
        "reference": "input58-20261108/verify-exit.json2; verify.log SHA45a688108ea398f6b2f59f3d67def247c6cf4362d7ac32bcd97b3bf50a06f5c1; handoff.json SHA495d72b8b76583205a7152ef0895add4d4c293e3c81c8fdbd5978f05d8a65e0e,9priorledgersunchanged,onlypositive,qualification/receipt absent,no current final parity or full30s claim"
      },
      {
        "criterion": "Signed clean handoff and committed complete native report",
        "reference": "f6cdc7c4769e82dd0999cdf9e40caf37e2f1a28e Good/Boxy; input58-20261108/final-state.json SHA7f097fe2abf6d7019dbad79f79975881f0612736921ca230b84536c9a1bda623 signature0,diffcheck0,status empty; report#93377961366329 checkpoint0c378625c93031635d8893ba08bad123aa528d3d"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT60 (2026-11-08). Supersedes INPUT58 (INPUT59 was never frozen). Implements decision 66331.

Verified by the Oracle from the tree:
- Source 1602aff72e961e493cca14feb559d6a323733f67: `git verify-commit` Good (Boxy). It is on harness origin/main, af5de27c is an ancestor, and Q2's worktree is clean.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-process-create-wall-ceiling-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input60/dream-read-c-preparation/compile-jqefbsuk/
- Probe: 251016 B, sha256 67060d75347926eb9d4f8a64006d9444b0fa0f71fdeca010fef92f9c4e025e8d, mode 0500.
- Receipt: preparation-6d1b82a8f4fdb67da05b865a1ccddf8203764757d82dda57773d926fd8aa5584.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 inputs match `git show 1602aff:<path>`. Changed since INPUT58: c1.rs ee08d9cd…, read.rs fa1587ea…, resolve.rs a362c6e7…, proof-helper-probe.c ed71ff1a…. Schema/vector blobs are identical to b26d7cd3.
- Audit file aab25da4… and tuple file b95da468… match.
- In proof-helper-probe.c: line 188 sets native_wall_limit to 250 ms for NATIVE_PROCESS_CREATE and 25 ms for every other class.

Whitelist amendment: the checker additionally accepts exactly these 4 terminal fields, and no others.
Q2's handoff named only 2; the frozen format string (~line 4023) has 4.
- u64: process_create_wall_excess_ns, process_create_wall_excess_count.
- process_create_wall_excess_known: the values 0 or 1 only.
- Literal: process_create_wall_excess_scope=invocation_sum_once_per_prepared_spawn.
Unknown, missing and duplicate fields are still refused. Add a unit test per type and literal.

Not proven: whether a full 16-form positive fits 30 s. Q2's source estimate puts ordinary floors at 6.8 s for G=16, 11.6 s for G=32, 21.1 s for G=64 and 78 s for G=256. The actual G is unknown, so the run decides. Report the actual guard count G and the final debt split.
runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt, from the positive.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT60 PRE-BUILD BLOCKED; native attempt unconsumed. Signed68f6d062 clean pins exact1602aff7/ed71ff1a/67060d75/6d1b82a8, accepts only4 typed excess fields and applies66331 only to process_create wall250ms; CPU2ms/app1ms/inclusive3ms unchanged.66 units0, complete immutable ingestion0, signature/main/source/codec checks0;10 previous ledgers unchanged. Existing >8GiB gate fails: df exit0 available8255056KiB <=8388608KiB; disk-gate.json SHA67137ac4e810d5c6e2b052b20b172905b21643f2f8561d93308ba51010d6469e. Need space released above gate or exact disposable-path disposition; no cleanup/native/fetch/build/components/final gates/qualification/receipt/landing. Actual G/debt split/excess unobserved; frozen C does not serialize private_guards.count, so derived G must remain distinct from measured telemetry. Full canonical command NOT RUN on68f6d062; last actual INPUT58 verification remains2. runtime_approved=false. Committed report#93377961366343 knowledge0fb7d28591e44389666346411e481cc0f97585fa; blockerDM01M6X1VQXTFFYVJA7N94Q3T6R8 native-queued, consumption unverified.

```json
{
  "op_id": "01M6X235C18RR96H52A6N0T4M7",
  "commit": "68f6d062d83af635dfc87e95beaf834b2c9766ca",
  "report": "[[reports/2026-11-08-dream-proof-tools-input60.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact INPUT60 selected immutable files, signed provenance and six unchanged source inputs",
        "reference": "input60-20261108/ingestion.json SHAeeb64697e162267700c16c15ac5c61163e73b6a5b1fc466408751c8ee6895bd6; preflight.json SHA cf30abb75f95ca453aba81f2fa8b519aff3e85322cead448b85f463afc8284c0 all0; source-byte-checks.json 12checks0 and6unchanged; sources.lock.json canonical_q2"
      },
      {
        "criterion": "Exact4 diagnostics, process-create-only ceiling and strict unit controls",
        "reference": "scripts/dream-proof-tools.py component_result/component_terminal; terminal-binding.json SHAa335203ed4da4d8bf7ac057fc4b701497f168e768d0a1b10d01e047b76cecc37; unit.log SHAf18add4ca5b986e33a4f2bd985ea6b6cbe685310ad5671772ecd4d7eae9aedf9 66tests0 (34/250admitted,251/CPU/inclusive refused; u64/flag/literal/unknown/missing/duplicate)"
      },
      {
        "criterion": "Concrete unmet pre-build disk invariant; sole native attempt not spent",
        "reference": "input60-20261108/disk-gate.json SHA67137ac4e810d5c6e2b052b20b172905b21643f2f8561d93308ba51010d6469e df0/8255056KiB vsstrict8388608; no cleanup or native attempt; blockerDM01M6X1VQXTFFYVJA7N94Q3T6R8"
      },
      {
        "criterion": "Historical failure integrity and honest outstanding verification/telemetry",
        "reference": "input60-20261108/preserved-ledgers.json SHA17cac3d128957b53801a335452b79c45f1f24981ab838da563a59cfcf8946cc4 10unchanged; full command NOT RUN, retained INPUT58 verify2; nativeG/debt/excess unobserved, private_guards.count notserialized"
      },
      {
        "criterion": "Signed clean source cut and committed full handoff",
        "reference": "68f6d062d83af635dfc87e95beaf834b2c9766ca Good/Boxy; input60-20261108/final-state.json SHA3502bd7cc5351a367af93d1f3ffd33017da768b4caedae7d297009bbd0eecce4 signature0,diffcheck0,status empty; report#93377961366343 checkpoint0fb7d28591e44389666346411e481cc0f97585fa"
      }
    ]
  }
}
```


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

BLOCKED after the one INPUT60 positive; unchanged signed68f6d062. Disk gate rechecked14649316KiB>8GiB, exact ingestion0, fresh7-source crypto0, all60 build commands0; prior66units0 remain valid. Run01M6X25J5JYW6CS88P06H0QYNF controller77044 actualwait4 exit81: coverage_unknown live observation, stderr taskall pid78272 result0 errno3. Raw stdout/result.json quarantine snapshot retains created8/reaped7/groupgonefalse; later SAME seq8 settled_refusal ledger ownwait4/groupgone and final terminal settled8/8; controller actually reaped, failures not cleared. Ledger /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6X25J5JYW6CS88P06H0QYNF/components/positive/output/ledger.ndjson SHA931699ad1dd65cbd18e7cc1067ae0a833cd4f2bc4858904e6cc7dcea1714a4c2. Source/basis-derived G43=26candidate+17fixture (NOT measured). Final debt15.513484500s paid: helper0.753219s + ordinary raw2.032131753s + floors12.728133747s; process_create excess0/count0/known1. No deadline; full16-form/30s fit remains unproven. Exact verify2; no retry/later cases/final gates/qualification/receipt/landing; runtime_approved=false. Need Oracle disposition of observation/settlement cutoffs and causally changed authorized input before another attempt. Committed report#93377961366345 knowledged2080cf587495ef30f6d42509e254aae7fa58fca;10 historical ledgers unchanged, tree clean.

```json
{
  "op_id": "01M6X2QS0JAETQMDQSKGJ81WNQ",
  "commit": "68f6d062d83af635dfc87e95beaf834b2c9766ca",
  "report": "[[reports/2026-11-08-dream-proof-tools-input60-attempt.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Passed disk gate, unchanged exact INPUT60 source/probe/receipt and current source checks",
        "reference": "input60-attempt-20261108/disk-gate.json SHA88aaf4003224e7644d81f76e9b2a4373c3659607a9d23c8e096a1ea93a96b87d df0/14649316KiB; ingestion0; signed68f6d062; original preflight66tests0 logSHAf18add4ca5b986e33a4f2bd985ea6b6cbe685310ad5671772ecd4d7eae9aedf9"
      },
      {
        "criterion": "Actual fresh authentication, private build and bounded basis",
        "reference": "runs/01M6X25J5JYW6CS88P06H0QYNF/authenticated.json SHAed85b0b87e4b6176533cc4357a945c810822b384debd3aef33955fc675147df9 all7crypto0; all60 logs exits0; basis47112B SHAbe187a6691637ef9e6c6da5a3504ce778655a126ef477b946e2cfc89b2482681; descriptor3432B SHA72440acdd6276af2de185978721964ea716f8ef15c53e875d00fbf9b0e03106d"
      },
      {
        "criterion": "Sole real observation refusal with original raw quarantine and later same-effect physical settlement",
        "reference": "positive/stdout=result.json SHAacdcc846610eff82bcdb1cfa6011d5ddbcf845e3ab6d554d887f46c09f9058f2 quarantine8/7; stderr SHA0a12bc85160cac259973d64bfc5d17f9cf889e82c2a603a388ab184c78de8f28 taskall0/errno3,coverage1/settlement2; ledgerSHA931699ad1dd65cbd18e7cc1067ae0a833cd4f2bc4858904e6cc7dcea1714a4c2 seq8 finalsettled_refusal; wait4SHA6981403ed91193d59084186c0c78b87a83e45de584ba5ff7cc9df78653b2836d actual81"
      },
      {
        "criterion": "Requested explicitly derived G and final debt/excess, preserving cutoff distinction",
        "reference": "input60-attempt-20261108/guard-derivation.json SHA262e733ba1c67e53b7864e17ad6ba2e7b3c8134c0a7da67a1d87a01a860a57e5 source/basisG43 notmeasured; debt-split.json SHA4fddfae73a68e80121194a20a021ca7b2fafc5ed4d64784be46340b794ee3390 helper753219000ns/raw2032131753ns/floor12728133747ns,total15513484500paid,excess0/count0/known1; rawA73456000 vsfinal83691000"
      },
      {
        "criterion": "Real exact verification exit and honest remaining gates/history",
        "reference": "input60-attempt-20261108/verify-exit.json2; verify.log SHA66e931e6384980c8059f1638f4a9708587bc3c6f6f507a59a14f9a9615197a8a; handoff.json SHA31f054264655585a0c2e6f9eac3e38d2bfb540f3198f753cc741bfb491b61f88 10oldledgers unchanged,onlypositive,qualification/receipt absent,full30s unproven"
      },
      {
        "criterion": "Signed clean worktree and committed append-only full native report",
        "reference": "68f6d062d83af635dfc87e95beaf834b2c9766ca Good/Boxy; final-state.json SHA3502bd7cc5351a367af93d1f3ffd33017da768b4caedae7d297009bbd0eecce4 signature0,diffcheck0,status empty; report#93377961366345 checkpointd2080cf587495ef30f6d42509e254aae7fa58fca"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT61 (2026-11-08). Supersedes INPUT60. Fixes the live-sample ESRCH race.

Verified by the Oracle from the tree:
- Source f6dffea0ff8038240de796e237362577e58fe843: `git verify-commit` Good (Boxy). It is on harness origin/main, 1602aff7 (INPUT60) is an ancestor, and Q2's worktree is clean.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-live-sample-esrch-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input61/dream-read-c-preparation/compile-wcb1m3yz/
- Probe: 251064 B, sha256 c50c2ae7d6f8aad7f3c0c99214e0683ea7b78cdf686f6acba8ca31fe340a2009, mode 0500.
- Receipt: preparation-eb5f1696cba83e117d354c040b50f0950d5f0fd7847f398255d0a699f73dd987.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 tuple inputs match both `git show f6dffea0:<path>` and the snapshot. Only proof-helper-probe.c changed (9bacbb04…). Schema/vector blobs are identical to b26d7cd3.
- Audit file 533b1629… and tuple file ff1c31cf… match.

Change: engine_sample_terminal_after_error (~2642). On ESRCH, the exit check is re-run only for an owned member whose current pid, birth, start and full token match exactly one retired record. Every other case keeps coverage_unknown.
The private_candidate_terminal format string is byte-identical to INPUT60, so the builder's 4-field whitelist (68f6d062) applies unchanged.

Not proven: the 30 s fit. The last run reached 15.5 s of debt after 8 helpers.
runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt, from the positive.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT61 PRE-BUILD BLOCKED; sole native attempt unconsumed. Signed1691e394 clean pins exact f6dffea0/9bacbb04/c50c2ae7/eb5f1696. Complete immutable ingestion0, signed main/source/codec checks0, nine unchanged inputs and byte-identical terminal;66 units0; no checker/budget/schema change. Disk gate fails: df0 available7883320KiB, required>8388608KiB, evidence disk-gate.json SHA4e3d6d12eddd2f6ef2978613e072276abcaf01d8208fb63c067c10c57d299b01. Need space released above gate or exact disposable-path disposition; no cleanup, canonicalfetch/build, candidate/probe, components/final gates/qualification/receipt/landing; runtime_approved=false.11 prior ledgers/sources/prefixes preserved. DerivedG/debt/excess unobserved until attempt; previous derivedG ruling stands. Full task command NOT RUN on1691e394; last actual INPUT60/68f6d062 verification remains2. Full report#93377961366352 committed knowledge7f457a715496827b514df9448b16c66d0b540fd9. BlockerDM01M6X8QM8ZYM2E17WVYAZFC3KP native-queued, consumption unverified.

```json
{
  "op_id": "01M6X8Y3VBKHPQKNGFH94PMTXB",
  "commit": "1691e3947ca04739f1301b596eae388f9c6f2aef",
  "report": "[[reports/2026-11-08-dream-proof-tools-input61.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact INPUT61 signed source/main provenance and immutable source/probe/preparation binding",
        "reference": "input61-20261108/preflight.json SHAbb1434b1ee4783a7312a258d507bd950d7a0eb48a9955a8bf1e2c90e324c9a00 all0; ingestion.json SHAf74badbe7e8ba94874dbc4133017287199cd3be2e7d4fc40d8ed1036f704d86a; selection.json SHA1c8b5fb173c6e2454509ac79e5ab5494d25dd58f11b993c3c6b0bd657247a6a6"
      },
      {
        "criterion": "Nine unchanged signed inputs, codec authority and identical current terminal contract",
        "reference": "input61-20261108/source-byte-checks.json SHA2d0388505e4f97d041b410b49550b9e57c4c1e0d890e703959d25c283072f376 12checks0/nineunchanged; terminal-binding.json SHAbcc85453f47220c568044c3d0e7411c2fae771aeb201fd79b306aa83ca0d9cd1 exactfields/byteidentical; checkerunchanged"
      },
      {
        "criterion": "Actual offline controls",
        "reference": "python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target exit0,66tests; input61-20261108/unit.log SHA1de2fba6d4e9aed1eb7ae8cc4a115c036be0aecafe38c29f38d2c14a48f11e88"
      },
      {
        "criterion": "Concrete capacity blocker before any native attempt and preserved originals",
        "reference": "input61-20261108/disk-gate.json SHA4e3d6d12eddd2f6ef2978613e072276abcaf01d8208fb63c067c10c57d299b01 df0/7883320KiB vsstrict8388608; preserved-ledgers.json SHA77329ea7194d7f3bdbb09ba9ccc30c989bf5996a28a76c12c90683fc1799a244 11unchanged; nativeattemptnotstarted"
      },
      {
        "criterion": "Honest current verification limits, signed clean handoff and committed full report",
        "reference": "fullcanonicalcommandNOTRUN,currentunit0; lastactualINPUT60verify2 retained; 1691e3947ca04739f1301b596eae388f9c6f2aef Good/Boxy; final-state.json SHA2766808c7ff471065d82a6604c21d1a68e79dc91eefefbeb86ac3ae572be4fd3 signature0,diffcheck0,status empty; report#93377961366352 checkpoint7f457a715496827b514df9448b16c66d0b540fd9"
      }
    ]
  }
}
```


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

BLOCKED after the sole INPUT61 positive on signed, clean 1691e394. Actual controller PID71655 wait4 exit81; canonical verify exit2. Raw result is quarantined: GS2 lacks retained GS1 custody, with coverage_unknown `signature lacks exactly one retained verifier`; attempts16, created/reaped15/15, proof not_reached. Signature leader ledger has actual own-wait4/group-gone, but no retained child identity/admission, so neither no verifier creation nor a Q2 defect is established. Ledger /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6X8ZSXWJQAF6DZH8VE88G5F/components/positive/output/ledger.ndjson SHA256 b3239d0a53190cb5775030a0d50d6c210f1d8a52e5e759bab4ef420cefe7b5d4. Derived G43=26 candidate+17 fixture, NOT measured. Final paid debt23.140558494s = helper1.348533s + ordinary raw3.196116747s + floors18.595908747s. Process_create excess0/count0/known1; no deadline observed, full30s proof fit unproven. Fresh7 authenticators and60 build commands exit0; current66 offline units previously0. No retry/later components/final gates/qualification/receipt/landing; runtime_approved=false. Need Oracle disposition of verifier/GS1 custody and a causally changed authorized input or scoped repair before another attempt. Full report #93377961366354 committed at knowledge76ac94b3e9a9328ed65c0cdd2807b9d692ca5c8b; 11 historical ledgers unchanged.

```json
{
  "op_id": "01M6X9JNC7M1SKWKG0F51T3SVG",
  "commit": "1691e3947ca04739f1301b596eae388f9c6f2aef",
  "report": "[[reports/2026-11-08-dream-proof-tools-input61-attempt.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact frozen INPUT61 source/executable/preparation and passed disk gate",
        "reference": "input61-attempt-20261108/disk-gate.json SHA256 dc904a5e83ad8c67e2b85c2329a612b300d587e721d333b8f24e67e2f9dac961: df exit0,30421104KiB>8388608; ingestion.json SHAf74badbe7e8ba94874dbc4133017287199cd3be2e7d4fc40d8ed1036f704d86a exit0; source f6dffea0, probe c50c2ae7, receipt eb5f1696"
      },
      {
        "criterion": "Current offline controls and fresh real source authentication/build",
        "reference": "input61-20261108/unit.log SHA1de2fba6d4e9aed1eb7ae8cc4a115c036be0aecafe38c29f38d2c14a48f11e88:66tests exit0; runs/01M6X8ZSXWJQAF6DZH8VE88G5F/authenticated.json seven auth_exit0; logs/000–059.exit.json all0; handoff.json binds every record"
      },
      {
        "criterion": "Sole real positive refusal retained without rewriting raw proof or physical settlement",
        "reference": "positive/stdout=output/result.json SHAaffc0ae8ff0addf9ddf4f63ac2865eb0f00ffee51c236ec2ae317bfbf9ab9fd3 quarantined15/15/proofnotreached; stderr SHA8c97d9b1abb10815f7e75354e82ffc717a8fedabe9fdce0bab2724a23e2afd88 coverage1/settlement2; ledger SHAb3239d0a53190cb5775030a0d50d6c210f1d8a52e5e759bab4ef420cefe7b5d4; actual wait4 SHAa8f5bd0c10ba51ca5c63342a1f4c2e04b3ee5a6882ff18900abe082c5f932a16 exit81"
      },
      {
        "criterion": "Requested derived G and final debt/excess with explicit cutoff limits",
        "reference": "guard-derivation.json SHA7cd59e151bf76a87b9c72a627d8d0f0e25757deb0a778a9529c224fd5144ca74: derived43 notmeasured; telemetry.json SHA4a80bb8292c09fa5605ba424b5e6062a1930d416c4edfcb4f4b54e893f7a273c: helper1348533000/raw3196116747/floors18595908747ns,total23140558494paid,excess0/count0/known1,no deadline"
      },
      {
        "criterion": "Actual canonical verification exit, honest missing gates and preserved predecessors",
        "reference": "verify-exit.json exit2; verify.log SHA5f36c4ffbc5ae5f00eeeae7857faad0488c7b000b540147bacbbaaa1852ca770; handoff.json SHA9508cc05828f56f162de8f387a8a84b5b3f7cb15989189b29f638183847a10ad: onlypositive,qualification/receiptabsent; preserved-ledgers.json SHA764d87f32e4ed0c0e72b1f724234f21598000e4f6a5fe258a1b677d0da0d9f06:11unchanged"
      },
      {
        "criterion": "Signed clean code and committed full append-only report",
        "reference": "1691e3947ca04739f1301b596eae388f9c6f2aef Good Boxy; final-state.json SHA207c47c4ff7396759914986dd1372d3e8d6a6edbcd68d4569260bac873307249:signature0,diffcheck0,status0empty; report #93377961366354 knowledge checkpoint76ac94b3e9a9328ed65c0cdd2807b9d692ca5c8b"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT62 (2026-11-08). Supersedes INPUT61. Adds event-driven verifier discovery (kqueue NOTE_FORK/EXEC/EXIT).

Verified by the Oracle from the tree:
- Source 7ab80fac868b53b1dfc690ef774fc3bfabbb18b8 and intermediate 4967eb43: `git verify-commit` Good (Boxy). The source is on harness origin/main, f6dffea0 (INPUT61) is an ancestor, and Q2's worktree is clean.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-verifier-event-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input62/dream-read-c-preparation/compile-qre_uuye/
- Probe: 251272 B, sha256 bab19b967fed5adc1ac82f278bc5869657fc54242ef522c0e4fe897a6b09523c, mode 0500.
- Receipt: preparation-889764ffa41be7bbf486be80afb49e74e5b58d9f81a522a1516609043468dd1d.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 tuple inputs match both `git show 7ab80fac:<path>` and the snapshot. Only proof-helper-probe.c changed (01e85781…). Schema/vector blobs are identical to b26d7cd3.
- Audit file 207d4db8… and tuple file ed5a2050… match.
- The private_candidate_terminal format string is byte-identical to INPUT61, so the builder's terminal whitelist applies unchanged.
- The C source contains both distinct refusals: "verifier forked and exited before census" and "signature leader created no verifier".

Ledger amendment: if the builder validates ledger record kinds, it additionally accepts exactly one new kind, verifier_event_discovery. Its fields are those listed in Q2's tuple:
- leader_fork_events u64
- leader_exec_events u64
- child_discovered_via none|poll|fork_event
- fork_seen_child_missed bool
- the raw mask, registration/wait/close errno and queue fields, with their tuple types.
Rows are per-owner snapshots; never sum repeated rows.

Not proven:
- That a real gpg verifier is retained. Q2's witness shows the event path missed a child of about 1.4-1.7 ms; the wake-to-census latency is not stated.
- The 30 s fit. The last run reached 23.14 s at the signature stage.
runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt, from the positive.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

BLOCKED: sole INPUT62 attempt run01M6XEW5VT8F8HF56S96KB592B is live quarantined; controller94751 and original wait parent47777 remain held (exec session65180). Actual controller/canonical exits UNKNOWN; no new verify_result supplied because no canonical exit exists. Signature seq16 ledger91 records fork_events1, exec0, viafork_event, missedfalse, registered3, errno0, queueheld1/closeattempt0; stderr GS1 binds verifier96572. Raw coverage_unknown is `retired child lacks qualified nonduplicated same-leader rollup`: ledger89 mask27 missing terminal bit4, timestamps0, sticky child_cost_unproven1; frozen C cannot recover the predicate in hold. Ledger /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6XEW5VT8F8HF56S96KB592B/components/positive/output/ledger.ndjson SHA25675984cf519a44f9ebb858e6f8a4434af3668d362ed7b8bb9343345cb32ddd607. Wire created16/reaped15, proofnotreached; snapshot paid debt23.149177875s, booked helper debt1.278441s; final raw/floor split unavailable because no final terminal. Derived G43, retained process_create excess0/count0/allknown1. Units66 exit0,7authenticators/60buildcommands0; report metadata verify is units ONLY. No retry/signals/later gates/qualification/receipt; runtime_approved=false. Need Oracle disposition of original live quarantine/parent custody and missing terminal-child CPU witness before any new tuple attempt. Signed clean0068adc9;12historicalledgers unchanged. Full report#93377961366362 committed knowledge8be5b897511ad4ac665cf9c44b8129df48d71e7d; pending-handoff SHA952fb83c5f46853294b40968cb50244c1f1cdf17ac482ddc24f0067e1442abef.

```json
{
  "op_id": "01M6XFBKPQ7344GRJ01R8AJHMY",
  "commit": "0068adc9ed2597153f85ba1a8aa485c835f6054e",
  "report": "[[reports/2026-11-08-dream-proof-tools-input62.md]]",
  "verify_result": null
}
```


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT62 authorized teardown complete; run remains FAILED/quarantined. Before signals, recorded ps for94751/47777 and all controller children (none); lsof exits0, numeric FDs55controller/5parent. TERM only94751 returned0; after10.008731916s fresh ps showed same controller/parent/executable alive, then KILL same94751 returned0. Original parent47777 actually wait4-reaped controller with status9/exit−9 (not81); parent exited through normal recipe refusal path1 reported by Make, original canonical command exited2. No other signal, no retry. Ledger75984cf5…, raw stdout/result b618e91d… and stderr b68f7d75… unchanged;12historicalledgers preserved. No final terminal/debt split/qualification; runtime_approved=false. Full report#93377961366365 committed knowledge48ae8881464bcf6d681cc59817c695640ca3fb93; run quarantine-disposition-20261108/handoff.json SHA51a40ae61ea06a0779c70d92634afff7909193a092ed2595781770c7c167ad29. Signed clean0068adc9; parked pending INPUT63 freeze.

```json
{
  "op_id": "01M6XFRQ11AGW66EVXA06202TW",
  "commit": "0068adc9ed2597153f85ba1a8aa485c835f6054e",
  "report": "[[reports/2026-11-08-dream-proof-tools-input62-disposition.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Pre-signal requested process/children/FD inventory and exact owned identity",
        "reference": "runs/01M6XEW5VT8F8HF56S96KB592B/quarantine-disposition-20261108/{ps-before,controller-identity-before,fd-counts-before}.json: ps/lsof0, controller94751 parent47777, nochildren, numericFD55/5; Oracle authorization01M6XFE9AJVMYG5SA98NSSD3CB"
      },
      {
        "criterion": "Only authorized PID signaled with >=10s escalation and actual original-parent reap",
        "reference": "D/sigterm-outcome.json0; ps-after-10s.json elapsed10008731916ns sameidentity; sigkill-outcome.json0 target94751only; components/positive/wait4.json SHA9c152f4afdb1e95fb5581ed13e1982e2c03a69ab6c1ce1143e5aac8ae6197c8d actual_wait4true,status9,exit−9; D/actual-exits.json"
      },
      {
        "criterion": "Actual original canonical command completes failed; no false native81 or successful settlement",
        "reference": "input62-20261108/verify-exit.json actual2; verify.log9878B SHA20e526be2e01d1ef7dc51caf169ca6aa85930dff3ffa668b0bf32a67a569673b, parentrecipeMakeError1; D/ps-after.json exit1/no94751or47777; no parent signal"
      },
      {
        "criterion": "Raw refusal, requested event row, unavailable final debt and predecessor integrity preserved",
        "reference": "ledgerSHA75984cf519a44f9ebb858e6f8a4434af3668d362ed7b8bb9343345cb32ddd607; stdout/resultSHAb618e91dba27fdcd2fa9a62d0130a9ad44057bda5e8fb619a8091508683acec8; GS1stderrSHAb68f7d75f78eb74346f8756051862b956744b255bed18774365c1fa63841ac37; D/preserved-evidence.json12historical+5current unchanged; nofinalterminal/debtsplit"
      },
      {
        "criterion": "Signed clean source, no new attempt and committed append-only disposition report",
        "reference": "0068adc9ed2597153f85ba1a8aa485c835f6054e Good Boxy; D/final-state.json SHA5a9fe8231db5ff63fc6871a1830eed293f1f6fe2a625f937016a1c4aede62090 gitverify0/diffcheck0/status0empty,onlypositive/noqualification; report#93377961366365 knowledge48ae8881464bcf6d681cc59817c695640ca3fb93"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT63 (2026-11-08). Supersedes INPUT62. The child knote NOTE_EXIT becomes the exit witness for a verifier reaped by its leader.

Verified by the Oracle from the tree:
- Source 390b5836fb3693e3c4861a98def1ef8b53d54d5b and its parent b1df2c4c: `git verify-commit` Good (Boxy). The source is on harness origin/main, 7ab80fac (INPUT62) is an ancestor, and Q2's worktree is clean.
- b1df2c4c..390b5836 is exactly one line, the authorized QoS #[ignore] in resolve.rs.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input63-final/dream-read-c-preparation/compile-9b5gmpk0/
- Probe: 268024 B, sha256 c483e7ee0ce682b131df910c31f45036bb0bdbf6cb7d442140304b403ccf9780, mode 0500.
- Receipt: preparation-888182901a213dfb915f1d9e14ba044da319c8aabf542e800c2830e4357e03c0.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 tuple inputs match both `git show 390b5836:<path>` and the snapshot. Changed: resolve.rs (da9fc20f…, the ignore line) and proof-helper-probe.c (54fcf3f3…). Schema/vector blobs are identical to b26d7cd3.
- Audit file 3282144d… and tuple file 0200af25… match.
- The private_candidate_terminal format string is byte-identical to INPUT62.

Ledger amendment: if the builder validates ledger fields, it additionally accepts exactly one new field, retained_child_accounting.child_exit_witness, a JSON string with the values ri, kqueue_note_exit or none.

Not proven: real gpg retention through to settlement, and the 30 s fit. The last run was at 23.15 s at the signature stage.
runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt, from the positive.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT63 positive PASSED actualwait4 exit0/check0, proofvalid, settled16/16, finalterminalpassed/error0. Signature seq16 leader53506: fork1/exec0/viafork_event/missedfalse/mask3/errno0/queueclosed; child53509 witnesskqueue_note_exit,mask31/proven1/unproven0,GS1→GS2 sameinode/hash. Final paid debt24.863725881s = helper1.376091s + raw4.634007003s + floors18.853627878s; excess0/count0/known1; derivedG43. Reporting29.144471s into29.834517s admitted interval,690.046ms remaining: this positive fits, no general guarantee. BLOCKED in same invocation's corrupt_proof: controller53612/parent5130 remain live, canonical/corrupt actual exitsUNKNOWN. Ledger /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6XPVSR73HYNHFAB3ECZSZX8/components/corrupt_proof/output/ledger.ndjson SHA131ccb59fd58e1ae7c4b48b50bc7a9917aa771f89c255d9aea22f51619246feb row87 engine_checkpoint C2687 actual1020958ns vs1ms/supplied1020166ns; raw resource_limit, child_witnessnone/mask0/unproven1,quarantined16/15. Need Oracle disposition of ordinary-unit failure and original live53612/5130 custody, changed authorized input/scoped repair before another attempt. No retry/signals/final gates/qualification/receipt/landing; runtime_approved=false. No new canonical verify_result supplied because no actual exit; report metadata is66units exit0 only.7freshauth/60buildcommands0. Signed clean9ff0f6ac;13historicalledgers unchanged. Full report#93377961366372 committed knowledgec12a0d87b3181b7891e021331b0ae8c5bebbd945; pending-handoff SHA59d65db741756f5bddadc184fd2b02782c970a33bacdae5c5887826a867c3918.

```json
{
  "op_id": "01M6XQBEK7BJTZT85QRVPREEN7",
  "commit": "9ff0f6acbe32c7dfb0b06dcb870783defc8489f7",
  "report": "[[reports/2026-11-08-dream-proof-tools-input63.md]]",
  "verify_result": null
}
```


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT63 authorized corrupt-proof teardown complete. Positive remains PASSED actual0/check0, proofvalid,16/16, fork_event+kqueue_note_exit/mask31/proven1; final debt24.863725881s (helper1.376091/raw4.634007003/floors18.853627878), excess0, derivedG43. Whole run remains FAILED/quarantined. Before signals: ps53612/5130, nochildren; lsof0 numericFD56controller/5parent. TERM only53612 returned0; after10.008976375s sameidentity still alive, KILL samePID returned0. Original parent5130 actualwait4 status9/exit−9, then normal recipe refusal exit1 reported by Make; original canonical command actual2. No other signals or retry. Raw positive/corrupt/13historicalledgers unchanged; corrupt final debt unavailable. Full report#93377961366376 committed knowledgee61402fadfe51eb0105557e928046121fbbd4e68. Original run/quarantine-disposition-20261108/handoff.json SHA5eace0c561b8bec73866c820bc551abc48a2d2b5d8c2aca17f94bc326988f192. Signed clean9ff0f6ac; no later cases/final gates/qualification/receipt/landing; runtime_approved=false. Parked until INPUT64 freeze.

```json
{
  "op_id": "01M6XQNRCYSZF185NY798DT4S4",
  "commit": "9ff0f6acbe32c7dfb0b06dcb870783defc8489f7",
  "report": "[[reports/2026-11-08-dream-proof-tools-input63-disposition-final.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Passing positive remains passed with requested verifier witness and final debt",
        "reference": "runs/01M6XPVSR73HYNHFAB3ECZSZX8/components/positive/wait4.json actual0;checked.json0;stderr terminal16/16,error0;ledger4b3e368ba8b576bd7152f56ce12b524324eaf0f68323855fabc624b707ec9342 eventfork_event/childwitnesskqueue_note_exit/mask31; input63-20261108/joined-telemetry.json finalpaid24863725881ns"
      },
      {
        "criterion": "Required pre-signal ps/children/FD records and sole authorized controller identity",
        "reference": "run/quarantine-disposition-20261108/{ps-before,controller-identity-before,fd-counts-before,authorization}.json: ps/lsof0, controller53612/parent5130,nochildren,numericFD56/5; Oraclemessage01M6XQD30GHZ27WVPNT0MCQ908"
      },
      {
        "criterion": "Exact escalation after >10s and original actual reap; no parent/other signal",
        "reference": "D/sigterm-outcome.json0;ps-after-10s.json10008976375ns sameidentity;sigkill-outcome.json0 target53612only;corrupt_proof/wait4.json SHA8142049213720a8cb69b211f17f8efb1362945e60f5fbe8109d368514f47f36b actualstatus9/exit−9; D/actual-exits.json parentnormalrecipe1"
      },
      {
        "criterion": "Real completed canonical failure and unchanged raw refusal",
        "reference": "input63-20261108/verify-exit.json2;verify.log9947B SHA32752bbe57cb75e806253cb5f8912920a43056b5976c4d848a7edeab6c41e823;corrupt ledgerSHA131ccb59fd58e1ae7c4b48b50bc7a9917aa771f89c255d9aea22f51619246feb andstdoutSHA710e4d73263c0e3a920e8dfda89428ad2fe57485f1301f13d84c463afad985bb unchanged"
      },
      {
        "criterion": "Preserved predecessors, signed clean code, no retry/qualification and committed report",
        "reference": "D/preserved-evidence.json16currentfiles+13historicalledgers unchanged;D/final-state.json SHA1b32d8c8b738f491bf0f1dc0423ec5211dce422894b5f5a3361d2804dcd4c13b gitverify0/diffcheck0/status0empty,noqualification/receipt;report#93377961366376 knowledgee61402fadfe51eb0105557e928046121fbbd4e68"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT64 (2026-11-08). Supersedes INPUT63. Splits the over-1 ms sample unit and batches cheap checks.

Verified by the Oracle from the tree:
- Source 1ec942b280cfe123ecaf70e2dbf5db8ccf2cbe3b: `git verify-commit` Good (Boxy), and 390b5836 (INPUT63, on main) is an ancestor. Q2's worktree is clean. NOT YET on origin/main: Q2's final gates are running.
- This freeze is conditional on Q2 landing exactly 1ec942b2. If Q2 lands a different source, this freeze is void and Q2 sends a new tuple.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input64/dream-read-c-preparation/compile-4ckdm3ob/
- Probe: 268232 B, sha256 ac88eabe597c2603a0dd21b0c689cb96c22d4d14522d51736fecc89e96301ef5, mode 0500.
- Receipt: preparation-66bac599863ce564debe34a2482a6fd4d648772e39855479aded0f0ddaa083e5.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 tuple inputs match both `git show 1ec942b2:<path>` and the snapshot. Only proof-helper-probe.c changed (15fb63e8…). Schema/vector blobs are identical to b26d7cd3.
- Audit file 8aa0d6d8… and tuple file 60d674e5… match.
- The terminal format string is byte-identical to INPUT63. There are no new ledger fields.

Margin: the >=3 s figure is a source-conditional projection only; this run measures it.
runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt (the positive, then corrupt_proof, in the same invocation as before).


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

BLOCKED INPUT64 sole positive run01M6XWN035QSQRFAB367J0AZG0; controller89578/parent41501 remain live with owned handle (exec30398), actual controller/canonical exitsUNKNOWN. Raw coverage_unknown retired child lacks qualified nonduplicated same-leader rollup: signature leader90581 seq16 eventfork1/viafork_event/missedfalse/mask3/queueheld1; child90584 created1/image_admitted0, witnesskqueue_note_exit,rollup_mask30 missingbit1/proven0/unproven1, stderrempty/noGS1. Ledger /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6XWN035QSQRFAB367J0AZG0/components/positive/output/ledger.ndjson SHA8fabd6b5e8974ea5f50019f90c6bb7abcd2393da7104c08ca047046340b551d9. Positive snapshot paid12.647472372s, derived bookedhelper1.450737s/ordinaryaggregate11.196735372s; finalraw/floors/margin unavailable. LastleaderRI partialmargin15.309174s only, ordinary_max_ns660041, retainedexcess0/count0/allknown1; derivedG43. corrupt_proof NOT RUN, requested metrics unavailable. Need Oracle disposition of missing live image/GS1 admission and original89578/41501 custody, then changed authorized tuple/scoped repair. No retry/signals/finalgates/qualification/receipt/landing; runtime_approved=false. Source1ec942b2 signature+390/2ce4359 ancestry0; maincontainmentobserved1 is explicitly NOT a gate, no differing landedinput observed.66units/7auth/60buildcommands exit0; no canonical verify_result because no actual exit, reportmetadataunitsONLY. Signed clean4332bd53,15historicalledgers unchanged. Full report#93377961366380 knowledge41e8941285a660da5016fae1bd68c53c5fe5552f; pending-handoff SHA f2613bc0da3aeacd06e8ccdbbbd557c5e4569f46e764bac4fa84a988d0b48f08.

```json
{
  "op_id": "01M6XX5G5AQRVBSFH7Z7QBRDMJ",
  "commit": "4332bd5327906bac33cd94c4e2e17dad904d869e",
  "report": "[[reports/2026-11-08-dream-proof-tools-input64.md]]",
  "verify_result": null
}
```


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT64 authorized teardown complete; freezeVOID and run remainsFAILED/quarantined. Before signals: ps89578/41501, nochildren; lsof0, numericFD54controller/5parent. TERM only89578 returned0; after10.007874250s freshsameidentity stillalive, KILL samePID returned0. Original parent41501 actualwait4 status9/exit−9, then normal recipe refusal exit1 reported by Make; original canonical command actual2. No other signals/retry. Rawstdout/result26a85314…/ledger8fabd6b5…/emptystderr unchanged;7currentfiles+15historicalledgers preserved. No finaldebt/margin/terminal, corruptnotrun; runtime_approved=false. Full report#93377961366382 committed knowledgefe8ce1f6c973a2a9096fa5a0468526bbbdf6d58a; original run/quarantine-disposition-20261108/handoff.json SHA998fb9f9cc5e9d4e36d51b2f94ea95620dc8a0e25b86d73dbbef36ce15414d08. Signed clean4332bd53; no latercases/finalgates/qualification/receipt/landing. Parked until INPUT65 freeze.

```json
{
  "op_id": "01M6XXE6TRZNPBGJW9NZPCK8W9",
  "commit": "4332bd5327906bac33cd94c4e2e17dad904d869e",
  "report": "[[reports/2026-11-08-dream-proof-tools-input64-disposition.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Required process/children/FD inventory and exact original controller identity before signals",
        "reference": "runs/01M6XWN035QSQRFAB367J0AZG0/quarantine-disposition-20261108/{ps-before,controller-identity-before,fd-counts-before,authorization}.json: ps/lsof0,89578parent41501,nochildren,numericFD54/5; Oracle01M6XX6FVXRM391Y7XHDS7Y8M4"
      },
      {
        "criterion": "Only authorized89578 signaled after >=10s with actual original-parent reap",
        "reference": "D/sigterm-outcome.json0;ps-after-10s.json10007874250ns sameidentity;sigkill-outcome.json0 target89578only;positive/wait4.json SHA17bc9704074996cb9413c3a3a19b4b8ea074181624c11a3efd9427b57e3595d0 actualstatus9/exit−9;D/actual-exits.json"
      },
      {
        "criterion": "Actual completed original canonical failure and normal recipe parent exit",
        "reference": "input64-20261108/verify-exit.json2;verify.log9878B SHA11820ab5450ca6a206bbabc588d37db886298196cb4b69fc5caaa41a9d44852a, MakeError1;D/ps-after.json expectedexit1/noPIDs; no parent signal"
      },
      {
        "criterion": "Unaltered raw failed effect, unavailable final metrics and preserved historical evidence",
        "reference": "positive/stdout=resultSHA26a8531436559432fdc252610b04730e7a6e2beacb7129e8a616941dd1336ad4;output/ledgerSHA8fabd6b5e8974ea5f50019f90c6bb7abcd2393da7104c08ca047046340b551d9;stderr0B;D/preserved-evidence.json7current+15historical unchanged;noGS1/finalterminal/corruptcase"
      },
      {
        "criterion": "Signed clean source, void freeze/no retry and committed full disposition report",
        "reference": "4332bd5327906bac33cd94c4e2e17dad904d869e GoodBoxy;D/final-state.json SHA09daa8df1fb985ae355a0f610b56f0ea1e7b2b5dd5c16e5412ca4bc330cded5b gitverify0/diffcheck0/status0empty,noqualification/receipt,parkINPUT65; report#93377961366382 knowledgefe8ce1f6c973a2a9096fa5a0468526bbbdf6d58a"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT65 (2026-11-08). Replaces void INPUT64. Restores same-wake verifier admission (option A plus a bounded kevent wait); keeps the unit split and floor batching.

Verified by the Oracle from the tree:
- Source 28c91dff6411ef18e8fec1eea68328e090cac8e3: `git verify-commit` Good (Boxy). Rebased on harness main f848c03f; 390b5836 (INPUT63) is an ancestor. Q2's worktree is clean. NOT YET on origin/main: Q2's final gates are pending.
- This freeze is CONDITIONAL on Q2 landing exactly 28c91dff. If Q2 lands different source, it is void and Q2 sends a new tuple.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input65/dream-read-c-preparation/compile-cr0g4pl5/
- Probe: 268392 B, sha256 bc9e220cbc8f68cfc64f69f8ad2ee4fefee45ca383744cb76b323602b9e3bf1f, mode 0500.
- Receipt: preparation-ef15bc9f8757ae3d9c39764e6397f53262809ed7259a59a8f65bdca700a8edf4.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 tuple inputs match both `git show 28c91dff:<path>` and the snapshot; proof-helper-probe.c is ca5bd97c…. Schema/vector blobs are identical to b26d7cd3.
- Audit file 89f672ed… and tuple file 3d961ab8… match.
- The terminal format string is byte-identical to INPUT63. There are no new ledger fields. The C source contains "verifier exited before admission".

Margin: source-conditional only; this run measures it.
runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt (the positive, then corrupt_proof).


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

BLOCKED INPUT65 sole run01M6Y3GQNVZDMH82PY9DNMKAT9: positive actualownwait4 PID62745 exit81, canonical2. Raw proofvalid/settled16/16/groupgone/retiredcoverage true but outcomefailed/controlfailed coverage_unknown finalcaseconjunction. Established false term: members_max1 vs C4117 required2; field only initialleader/census writes3524/3237–3240, samewake admission doesn't update it. Exact missedcensus timing inferred; other opaque conjunctionflags not serialized. Signatureleader63912 child63916: fork_event, image_admitted1, witnesskqueue_note_exit/mask31/proven1/unproven0, GS1→GS2 sameinode/hash, queueclosed. Ledger /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6Y3GQNVZDMH82PY9DNMKAT9/components/positive/output/ledger.ndjson SHAdb7e2fbbbfbc3aaf567279b519bf5691f38860e12777e0204e8fb838a024ab20. Finalpaid11.890215s = helper1.414215/raw1.525853250/floors8.950146750s. Admitted29.857377s, entry→report13.689567s, reporting-startmargin16.167810s; ordinarymaxraw749833ns/tail464542ns; excess0/count0/known1, derivedG43. Corrupt_proof NOT RUN. Need Oracle disposition of complete-case members_max bookkeeping and changed authorized tuple/scoped repair before another attempt. No retry/teardown/finalgates/qualification/receipt/landing; runtime_approved=false.66units/7auth/60buildcommands0; signed clean c5230f53,16historicalledgers unchanged. Full report#93377961366389 committed knowledge7adbe7223994bafe3004c6e9897e4ca54e84b4e7; handoffSHA4f009bf700da852a594f5af459b13b5a475259770a18faf4d90aa62f3a8e9cc5.

```json
{
  "op_id": "01M6Y3ZMTYV6X5W459447XKESS",
  "commit": "c5230f53ddb2be8e620ac40a5690eef529d3c214",
  "report": "[[reports/2026-11-08-dream-proof-tools-input65.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact signed INPUT65 freeze and current offline controls without main-containment gate",
        "reference": "input65-20261108/{selection,ingestion,preflight,source-byte-checks,terminal-binding,tuple-binding,scope-audit}.json: signature+f848/390/2ce ancestry0,9unchanged,maincontainment1 advisory;unit.logSHAa0202e077433c41478d33b015c75f4bb378c1311d6e36992db28f0a19879973e66tests0;disk26615356KiB>8388608"
      },
      {
        "criterion": "Actual fresh real source authentication/build and failed canonical exit",
        "reference": "runs/01M6Y3GQNVZDMH82PY9DNMKAT9/authenticated.json7auth0;logs000–059.exit.json60commands0;input65-20261108/verify-exit.json2;verify.log9665B SHA4bf341ba42c06ef8ff0520c01b712495588eabe96269960461329fcbe4db4c06"
      },
      {
        "criterion": "Natural actual controller81 with unchanged valid-proof/settlement wire and producer coverage failure",
        "reference": "positive/wait4.json SHA78a5227d440859a243d6897bf5f40b560ae3b3b7f0da885fcafdb70fea48f8bb actual81;stdout=resultSHA609a83cbecec0f656de8b89f17b386b0e437a04f79bae88bef48d14899bb5e02 settled/proofvalid16/16/members1/errorcoverage;C4116–4121 requiresmembers2"
      },
      {
        "criterion": "Requested signature discovery/admission/exit witness and final debt/margin/ordinary values",
        "reference": "positive/ledgerSHAdb7e2fbbbfbc3aaf567279b519bf5691f38860e12777e0204e8fb838a024ab20 rows89/91 image1,kqueue_note_exit,mask31/fork_event/queueclosed;stderrSHA57c17fb75343174afcc6a6417438edd58feb3070143266d65ff10cf424e616e7 GS1/GS2+final;telemetry.json finalpaid11890215000ns helper1414215000/raw1525853250/floors8950146750,margin16167810000ns,ordinary749833/464542,excess0"
      },
      {
        "criterion": "No retry or false qualification, predecessor integrity and signed committed handoff",
        "reference": "input65-20261108/preserved-ledgers.json16unchanged;final-state.json gitverify0/diffcheck0/status0empty,onlypositive,noqualification/receipt;handoffSHA4f009bf700da852a594f5af459b13b5a475259770a18faf4d90aa62f3a8e9cc5; c5230f53ddb2be8e620ac40a5690eef529d3c214 GoodBoxy;report#93377961366389 knowledge7adbe7223994bafe3004c6e9897e4ca54e84b4e7"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT66 (2026-11-08). Supersedes INPUT65. members_max is now counted on every admission path, and the final-case conjunction terms are serialized.

Verified by the Oracle from the tree:
- Source 7449a048ad08e62e491be971e65c53695e3be940: `git verify-commit` Good (Boxy), and 28c91dff (INPUT65) is an ancestor. Q2's worktree is clean. NOT YET on origin/main.
- This freeze is CONDITIONAL on Q2 landing exactly 7449a048; otherwise it is void.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input66/dream-read-c-preparation/compile-ifwmr6v1/
- Probe: 268616 B, sha256 4ba0533e6806678fcf12529192ef0f06a2381d9b2d74e4a1d0a0e78ab2d060b9, mode 0500.
- Receipt: preparation-7c9fa3868bebdb25b55b6e59c65ea7d5327ee6e0577bc4258c41f83eda0e2b5a.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 tuple inputs match both `git show 7449a048:<path>` and the snapshot; proof-helper-probe.c is fb34071e…. Schema/vector blobs are identical to b26d7cd3.
- Audit file 3113034d… and tuple file f943a805… match.
- engine_member_live_admission (~2609) counts actual live, identity-complete, current members. The census update (~3257) is retained.
- The terminal format string is byte-identical to INPUT65.

Ledger amendment: if the builder validates ledger kinds, it additionally accepts exactly one new kind, final_case_conjunction, with these fields:
- the 28 bool term keys listed in Q2's INPUT66 handoff (E/C35-ledger-fields.json sha256 c45e2253…);
- coverage_passed and resources_passed (bool);
- first_failing_term (string: one of the term keys, or "none");
- capture_cutoff (string literal before_this_diagnostic_format_write_fsync_close);
- kind (string literal final_case_conjunction);
- runtime_approved (bool false).

runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt (the positive, then corrupt_proof).


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT66 positive and corrupt_proof PASSED actualwait4/check0, settled16/16; proofvalid/completed and proofinvalid/expected_rejection. Both ledger92 final_case_conjunction rows have exact28boolterms alltrue, coverage/resources true, first_failing_term none at before-diagnostic-write cutoff; subsequent terminalcost/persistence/controlpassed. Positive finalpaid11.425986s = helper1.165986/raw1.228146003/floors9.031853997s; reportingmargin16.594758s in29.827851s, ordinarymax519875ns (tail426542). Corrupt finalpaid11.215962s = helper1.090962/raw1.219613994/floors8.905386006s; margin16.985548s in29.991914s, ordinary558833ns (tail433708). Both excess0, derivedG43. BLOCKED overall: same invocation proceeded to required pause_resume, actualPID68179 exit81/proofnotreached/settled1/1, resource_limit RI start age103.627ms>50ms; original canonicalexit2. Pause ledger /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6Y8VDN0ZJ8XMYC5D04V0FGJ/components/pause_resume/output/ledger.ndjson SHA37f5572314e00c5655a97128947cc52c4cc2eff32a217c1a8a1b66e878c8890a line2 historicalmax; line3 pauseACK/userstable/noresume; later12.597ms RI interval doesnoterase failedmax or identify exact cause. No conjunctionrow after earlypausefailure. Need Oracle disposition/changedauthorized input or scopedrepair; no retry/teardown/finalgates/qualification/receipt/landing; runtime_approved=false.66units/7auth/60buildcommands0, signed clean41c3982d,17historicalledgers unchanged. Full report#93377961366396 committed knowledge2a6a922852c360928fae551c2ce07dad7d522f1c; handoffSHA1adbee2f670a538b2de829ebdde882f9b8b85d75316b90062249550eb2ebe122.

```json
{
  "op_id": "01M6Y9G9GK82H0H1TA3CAT180E",
  "commit": "41c3982df58218ea4c4754febb653183ca512d63",
  "report": "[[reports/2026-11-08-dream-proof-tools-input66-final.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact signed frozen INPUT66 input, conditional SHA authority and unchanged adapter controls",
        "reference": "input66-20261108/{preflight,selection,ingestion,source-byte-checks,terminal-binding,tuple-binding,scope-audit}.json: sig/28/f848/2ce ancestry0,9unchanged,maincontainmentadvisory;unitlogSHAb959426e07d358b951e612ca703f916f93dc0dfb0e2e6c6dfcd0e1e810368ed7 66tests0;disk26449856KiB"
      },
      {
        "criterion": "Both requested real proof cases pass actual reaps/checks and every frozen conjunction term",
        "reference": "positive wait4SHAf345025f6ac6d8c4a4729fdb935fffdffb8c826a6295a25e1aea0ada0c03116d0/check0; corruptwait4SHAd3d82cac8bc26fbd716d0e0c2e69f1c7ffe553ee598e58027175b11442ff2794 0/check0; ledgers0652954d…/1f2ab3ee…line92 all28true/firstnone;conjunction-contract-check.json exactC35c45e2253…"
      },
      {
        "criterion": "Requested outcomes, final debt components, margin and ordinary maxima retained without cutoff conflation",
        "reference": "input66-20261108/telemetry.json pospaid11425986000ns helper1165986000/raw1228146003/floors9031853997,margin16594758000,ordinary519875/426542;corruptpaid11215962000ns helper1090962000/raw1219613994/floors8905386006,margin16985548000,ordinary558833/433708;bothfirstnone/excess0"
      },
      {
        "criterion": "Actual pause-control failure stops original full command, no retry or false task green",
        "reference": "pauseledgerSHA37f5572314e00c5655a97128947cc52c4cc2eff32a217c1a8a1b66e878c8890a gap103627000ns>50000000;pausewait4SHA838cda9a8d66c60d75ff327f48ef1312028cfc6f67265b9d0175cc059dd34fc3 actual81;verify-exit.json2/logSHA4dae5d52c12c30d070e1f976119cd8b9f50ca18a3a02dd9c7936ddee9dfdf65a9798B;no latercases/finalgates/receipt"
      },
      {
        "criterion": "Fresh real auth/build, historical preservation and signed committed full handoff",
        "reference": "authenticated.json7auth0/logs000–059all0;preserved-ledgers.json17unchanged;final-state.json GoodBoxy/diffcheck0/status0empty,originalprefix187302846,noqualification;41c3982df58218ea4c4754febb653183ca512d63;report#93377961366396 knowledge2a6a922852c360928fae551c2ce07dad7d522f1c"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT67 (2026-11-08). Supersedes INPUT66. Pause/resume: excludes the positively-stopped interval, bounds SYSTEM accrual during a stop (at most 50 us, charged), adds a bounded stopped-ACK kevent wait, and moves the member-peak controls to the API.

Verified by the Oracle from the tree:
- Source 1f1b30ddec05e5d8e092d91b92c161bd94d1cc38: `git verify-commit` Good (Boxy), and it is ON harness origin/main; 4a62faf1 is an ancestor. Q2's worktree is clean.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input67/dream-read-c-preparation/compile-lrlj5_sr/
- Probe: 285448 B, sha256 65da9c9e97a130ddf7962e3df03d41f58c226a48458066610e793b159c65195d, mode 0500.
- Receipt: preparation-d6a854c8fd0e4f1784ddd56d72da9bf4f645bce126b66e417aa1ede920efac14.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 tuple inputs match both `git show 1f1b30dd:<path>` and the snapshot: read.rs is c2df3b34… and proof-helper-probe.c is 98a0ff5f…. Schema/vector blobs are identical to b26d7cd3.
- Audit file 03224241… and tuple v2 file 48c72b08… match.
- The terminal format string is byte-identical to INPUT66.

Ledger amendment: if the builder validates ledger fields, it additionally accepts exactly the fields listed in E/INPUT67-complete-ledger-delta-C36-C40-C45-v2.json (sha256 2eed29e1…, in Q2's directory). These include:
- helper_total_budget.stopped_interval_ns, running_gap_max_ns and stopped_system_ns (u64);
- retained_child_accounting.max_actual_wait_ns (u64);
- the ACK/RI/SYSTEM/wait diagnostics with the types in that file.

runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt (positive, corrupt_proof, pause_resume and the later cases in the same invocation).


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT67 run 01M6Z2NJ9Q3GETXWTJ43M9FZGC remains FAILED overall, canonical actual exit2. Positive PASSED actualwait4/check0 valid proof16/16 and corrupt_proof PASSED actualwait4/check0 invalid proof16/16; both28 conjunction terms true/first_failing_term none. Final paid debt positive11.942559s (helper0.971559/raw2.004379119/floors8.966620881), reporting margin15.687303s; corrupt12.091266s (helper1.021266/raw2.041550253/floors9.028449747), margin15.409834s. Pause actualPID91279 exit81, settled8/8/proofnotreached/coverage_unknown 'foreground STOP pending independent terminal settlement'; conjunction not emitted. FrozenC3585–3586 proves nonzero kill(-pgid,SIGSTOP), errno stored but NOT serialized; ESRCH/exit race unproven. Need Oracle disposition/corrected authorized input or scoped repair before any new native attempt. Pause finalpaid9.261603s (helper0.486603/raw1.389384747/floors7.385615253), margin19.229082s; owner maxima stopped178573000ns/running13108000ns/SYSTEM0, only owners1–2 ACKed/resumed. Lastseq8/PID92030 noACK/stopped0/running10158000ns; all8ownwait4/groupgone, no teardown. All3excess0, derivedG43. Offline exact clock auxiliary diagnostic and narrowly typed reap_delay earlier-wire/later-settlement binding repaired with negative controls;68units0,7auth0,60buildcommands0; signed clean db0b6000003dac6402a6da6a4fb2477eab1dd7d2. No retry/signals/latercases/finalgates/qualification/receipt/landing, runtime_approved=false. Twenty prior ledgers unchanged. Full report#93377961366425 committed knowledge68af29c3c33f701d6a0361a4bae3338dc00b1249; E=target/dream-proof-tools/input67-20261108, handoffSHAfc1117a41ca3786462709ae1a1f8ba9dd6569444396c75fa22f71a7bc9f49e1e.

```json
{
  "op_id": "01M6Z3GS1Q1EG1NBT964155EV0",
  "commit": "db0b6000003dac6402a6da6a4fb2477eab1dd7d2",
  "report": "[[reports/2026-11-08-dream-proof-tools-input67.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact frozen INPUT67 signature, main/ancestry, ten input bytes, schema/vector identity and disk gate",
        "reference": "target/dream-proof-tools/input67-20261108/{preflight,source-byte-checks,selection,ingestion,terminal-binding,tuple-binding,launch-disk-gate}.json: signed1f1b30dd/allchecks0;read.rs/Cchanged8unchanged;schema/vector blob identity;b26 ancestry not claimed;disk14248544KiB>8388608"
      },
      {
        "criterion": "Scoped offline classification fixes and strict rejection controls",
        "reference": "scripts/dream-proof-tools.py:3121 exact66331 clock detail;3137 narrow reap_delay actual81/typed earlier1/0→finalsettled1/1;4618–4668 negative controls;unit-exit0/68tests/unitlogSHA72ba9c314045360fbfb6b5479329617903744f4e93362cd5da24f42f213ba767;ownership/model actions unchanged;native clock/reap not reached"
      },
      {
        "criterion": "Both real proof cases independently pass with every final conjunction term and physical settlement",
        "reference": "runs/01M6Z2NJ9Q3GETXWTJ43M9FZGC/components/{positive,corrupt_proof}/wait4.json+checked.json actual0/check0;ledger92 all28true/firstnone;ledgerSHAs76a12b5f03ed5489873c28e3783d22be596be72a9a2b689aa3573e67ecc9fba6 and37468c728ee30a77f05a1c20c328362538b78702a5cbaaea000d2c096b51f0d2"
      },
      {
        "criterion": "Actual pause failure and requested stopped/SYSTEM metrics retained without invented errno",
        "reference": "pauseledgerSHA6fd8e6af15f596dd8f253da2b1affc88d634112acea6abfb7ba766fcec8eb67e lines2/8 ACKedowners stopped178573000/159244000ns, maxrunning13108000ns;line44 failedowner92030 stopped0/running10158000/SYSTEM0;all8wait4/groupgone1;stderr2 failure;C3585–3586 nonzeroSTOP;errno unlogged;wait4actual81"
      },
      {
        "criterion": "Final debt components and admitted-interval margins, distinct raw-result and terminal cutoffs",
        "reference": "E/telemetry.json SHA4660b058f4d45a9a2acf96a3e6921f3e65c272843126dcb8879bf36443faaf86: posdebt11942559000ns/margin15687303000;corrupt12091266000/15409834000;pause9261603000/19229082000;all debtcomponents=9xcost;excess0;derivedG43"
      },
      {
        "criterion": "Honest failed full verification, preserved historical evidence and committed report",
        "reference": "E/verify-exit.json2/verifylogSHA03cee31022351715b20dafdf8d8b95190860f53e38a1e6589b809668df76e56c;7auth/60build0;E/handoff.jsonSHAfc1117a41ca3786462709ae1a1f8ba9dd6569444396c75fa22f71a7bc9f49e1e verifies20historicledgers/signedclean/noqualification/no retry/signals;report#93377961366425 knowledge68af29c3c33f701d6a0361a4bae3338dc00b1249"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT68 (2026-11-08). Supersedes INPUT67. Foreground STOP vs helper-exit race: a failed group SIGSTOP (ESRCH, or EPERM on a zombie-only group) becomes terminal-before-stop, resolved only by the original leader's own wait4 with the group gone.

Verified by the Oracle from the tree:
- Source a68d200268ca816a6741e00c6cd334b28628f805: `git verify-commit` Good (Boxy), and it is ON harness origin/main; 645fec78 is an ancestor. Q2's worktree is clean.
- The workflow-only clone archive commit d49f0145 (scripts/formal.py) landed before it.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-pause-stop-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input68/dream-read-c-preparation/compile-p9dauq95/
- Probe: 285784 B, sha256 87ce8d48283f9a3f7eab41cbf8e073694d3b76ba2066a6223f94c18393d47131, mode 0500.
- Receipt: preparation-7f736d1abc0b493aa0ec12adc443594e5bd9e1ecb65df306101b3dfa1321eb05.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 tuple inputs match both `git show a68d2002:<path>` and the snapshot: resolve.rs is 3215c192… and proof-helper-probe.c is 3c18a373…. Schema/vector blobs are identical to b26d7cd3.
- Audit file dfc4a023… and tuple file e0f3ba9d… match.
- The terminal format string is byte-identical to INPUT67.

Ledger amendment: if the builder validates ledger fields, it additionally accepts exactly these fields in the existing pause_user_and_terminal_rss row:
- u32 flags: pause_original_owned, pause_pending_terminal, pause_stop_attempted, pause_terminal_candidate, pause_terminal_preobserved, pause_terminal_resolved, pause_zombie_only_proven;
- u64: pause_stop_call_ns, pause_stop_return_ns, pause_terminal_observed_ns, pause_wait4_observed_ns, pause_zombie_only_observed_ns;
- i32: pause_stop_errno, pause_stop_result;
- the string enum pause_stop_order, with the values given in Q2's INPUT68 tuple.

runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt (all cases, in one invocation).


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT68 run01M6ZMJP97QRM0YS85A806NJZW FAILED: sole canonical invocation actual exit2; positive controllerPID5606 actualwait4exit81/status20736. Raw proofnotreached/quarantined10created9reaped, first coverage_unknown current helper audit identity unavailable (frozenC3076 known_control_token refresh); final terminal settled10/10, cost/persistencepassed but controlfailed. No final_case_conjunction/first_failing_term emitted. stderr taskall ESRCH PID6305 was recovered (seq5ledger26 ownwait4/groupgone1), PID6458 seq10 reached audit identity refusal; exact failing token subread is unrecorded. Subsequent live BSD identity predicate C3180 failed before group_kill_attempted; ledger56→62 preserves same owner and settles by ownwait4, ledger67 queueclosed. Thus no failed SIGKILL syscall established and no live quarantine/teardown remains. Positive finalpaid10.133055s = helper0.953055/raw1.386400122/floors7.793599878; reportingmargin18.377769s in29.813229s, raw/terminal ordinary483875/458166ns, excess0, derivedG43. Reporting precedes late ownwait4, which has18.324511s remaining; neither margin is post-exit. All seven latercases, including corrupt_proof/pause_resume, notrun; pause metrics unavailable. Need Oracle/Q2 disposition of audit-token/final-identity refusal and correctedauthorized input or scopedrepair before another native attempt. No retry/signals/finalgates/qualification/receipt/landing; runtime_approved=false. Signed clean6e9031570f9f1c990a8a76284497b41924f3f27d changes exactpins only;68units0/7auth0/60buildcommands0,23historicalledgersunchanged. Report#93377961366443 committed knowledge8f505413519e778177bffbf2d9aaaddf1233feb0; E=target/dream-proof-tools/input68-20261108, handoffSHA76f0be1df5a91de8af79128a99197f6eb776ba6d662dde208392e1edd2bfd421.

```json
{
  "op_id": "01M6ZN1E05CZMB6K5EJ6SA9BA1",
  "commit": "6e9031570f9f1c990a8a76284497b41924f3f27d",
  "report": "[[reports/2026-11-08-dream-proof-tools-input68.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact signed frozen INPUT68 ingestion and unchanged narrow authority",
        "reference": "input68-20261108/{preflight,selection,ingestion,source-byte-checks,terminal-binding,tuple-binding,scope-audit}.json: a68d2002 GoodBoxy/main/645fec78+2ceancestry0;C3c18a373/probe87ce8d48/receipt7f736d1a;2changed8unchanged;schema/vector blob identity;raw ledger retained without parser/schema change;68unit0 logSHAb9a886f068a1eadb5f3e1cf3c152a9693bd3d0f125bfd85aa5a6b534d332509e"
      },
      {
        "criterion": "Real source authentication/build and sole native attempt with actual exits",
        "reference": "runs/01M6ZMJP97QRM0YS85A806NJZW/authenticated.json7auth0/logs000–059all0;launch-disk-gate34039372KiB>8388608;positive/wait4SHAcd1fe3708fc5624116d86f72fc491f1498cbc4e7c7dc30905336227894a579e9 actual81;verify-exit2/log9665B SHA1df8726cebb5b6238e11f204cd34377623b124917205ef628772fb513f2bc422"
      },
      {
        "criterion": "First native failure and same-original late settlement distinguished from verdict",
        "reference": "positive/resultSHAfeb1680e240acf1307df3dc8aebdb5333b448c867db14fcccae108d36387d726 coverage_unknown/quarantined10/9;ledgerSHAe99a6878654f62f236f1d150a4089f13792bdbeeb55a81e32c76588cdbc78ead line56 owner6458 quarantined,62 samebirth/start ownwait4/groupgone1 andcostpaid,67queueclosed;stderr6 settled10/10/controlfailed;C3076 auditrefusal/C3180 prekillidentityfailure;no killattempt"
      },
      {
        "criterion": "Debt split, margin, ordinary maximum and unavailable later-case metrics honestly reported",
        "reference": "E/telemetry.json SHA96e51b93e8b1e2ebb0e27eea7b488fd67037cb07c7794434182c72d5bc0acb3e: finalpaid10133055000ns=953055000helper+1386400122raw+7793599878floors;reportingmargin18377769000ns;latewait4margin18324511000;ordinary483875/458166;excess0/G43derived;no conjunction;other7casesnotrun/pause metrics unavailable"
      },
      {
        "criterion": "Recorded errno versus unknown token/final-identity substage kept distinct",
        "reference": "E/failure-analysis.json SHAfd4919a6e2c7d2367280cda4137df9c7d91546d55cfa2d3600d4d56a39b41641 binds C161/166/3046–3054/4130–4149;PID6305 ESRCH recovered seq5line26;PID6458 audit refresh failed;which token stage/BSD predicate errno unknown;final kill raw error label doesnotprovefailedSIGKILL"
      },
      {
        "criterion": "Preserved historical evidence, signed clean code and committed failed-gate report",
        "reference": "E/handoff.json SHA76f0be1df5a91de8af79128a99197f6eb776ba6d662dde208392e1edd2bfd421 verifies23priorledger hashes/dev/inode unchanged,prefix189155930retained,signedGoodBoxy/diffstatus0empty;onlyownnewbytecodecleanup;no retries/signals/qualification/receipt/landing/finalgates;report#93377961366443 nativecheckpoint8f505413519e778177bffbf2d9aaaddf1233feb0"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT69 (2026-11-09). Supersedes INPUT68. Systematic owned-helper observation exit-race handling through one shared terminal path, with fault injection at all 27 observation sites.

Verified by the Oracle from the tree:
- Source 637cb234d93d08d0c8fcfb820b7e48f9538bc0e1: `git verify-commit` Good (Boxy), and it is ON harness origin/main; a68d2002 (INPUT68) is an ancestor. Q2's worktree is clean.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-q2-systematic-exit-race-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input69/dream-read-c-preparation/compile-ewu_xvzb/
- Probe: 286472 B, sha256 6c63469ff20f4a433c2fafb3a4aed7c91e037e46862da937fc316c9c140b07ba, mode 0500.
- Receipt: preparation-987b8b56107a59230c0e353a7defbe1fef2ff50f7f3903a4c72a9fc290851a3a.json; its hash matches the name. Tuple file 83682788… matches.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 10 tuple inputs match both `git show 637cb234:<path>` and the snapshot; only proof-helper-probe.c changed (d1ec0741…). Schema/vector blobs are identical to b26d7cd3.
- The terminal format string is byte-identical to INPUT68.

Ledger amendment: if the builder validates ledger fields, it additionally accepts exactly:
- obs_terminal_site: a string from the fixed 27-value enum in Q2's INPUT69 tuple;
- obs_terminal_errno: i32.

runtime_approved=false.

Builder 01M6W6K5SWZB5AEBAAAA73FF4M: one native attempt (all cases, in one invocation).


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT69 run01M7035811Q2HTZ37K75WQ5K3C BLOCKED/FAILED with original invocation OWNED PENDING. Positive PASSED PID89282 actualwait4/check0, validproof16/16, ledger108 all28terms true/first_failing_term none, finalpaid13.286537622s=helper1.352025/raw2.462987619/floors9.471525003, reportingmargin14.438642s in29.858592s, ordinary799625/540375ns,excess0,derivedG43. Corrupt controller90389 remains owned by wait parent43277 (psSNs,parentS,no directchildren,lsof55numericFDlines); rawquarantined16created15reaped/proofnotreached, coverage_unknown foreign or reversed live helper resource identity. C3220–3222 is a semantic RI-start/counter predicate after successful read; member/comparison/errno/cause not serialized. Leader91377 is ownwait4/groupgone/paid at ledger100, but child91380 ledger103 has image_admitted0/rollup0/child_cost_unproven1/witnessnone; C4516–4521 explains16/15 despite ownergroupgone, stickychildunproven/C4457+4592 prevents hold completing. Need exact Oracle quarantine disposition for90389/43277 and Q2 source disposition/refreeze before another attempt. Corrupt rawdebt12.354342750s only; no finaldebt/margin/conjunction/stderr/wait4; canonical actual exit unknown, no verify_result submitted. Observation-terminal rows: positive16/corrupt14, all escaped_cleanup_bsd/code3; no other sites evidenced, never births. Pause/latercases notrun; stop/order metrics unavailable. No retry/signals/teardown/finalgates/qualification/receipt/landing; runtime_approved=false.68units/7auth/60buildcommands0; signed clean pin-only eb42d39aafb86d9de7ded7ec30d9bc00a13297cf,24priorledgers unchanged. Full report#93377961366454 committed knowledgef1c677083b500be0ce7c0a1119e3dfb9efc1d0db; E=target/dream-proof-tools/input69-20261109, pending-handoffSHAf4dc7f47e89493e509c20fa7d7e3a27a810485ab8a4ff4d44861f73a721c054b. Report metadataverify68unit0ONLY, notcanonicalgreen; execsession92254 retains original command.

```json
{
  "op_id": "01M703QJV40AMM2XA4ZXMASJVG",
  "commit": "eb42d39aafb86d9de7ded7ec30d9bc00a13297cf",
  "report": "[[reports/2026-11-09-dream-proof-tools-input69.md]]",
  "verify_result": null
}
```


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT69 authorized quarantine teardown COMPLETE under Oracle message01M703R0TFV6W7FRQR6KKQEX6G. Inventoried exactcontroller90389/parent43277/no children, lsofFD55/5; verified executable/input/start identity before signals and afterTERM. TERM90389 returned0; samePIDstillSNs31.057555875s later; KILL90389 returned0. No other PID signalled. Original43277 actualwait4 for90389 records exit-9/rawstatus9, SHA60cfcc80718002765d1758f852cc10fdf99411d9d5f9ee68dd7bab7812bb896c; both90389/43277 absent after completion. Originalexecsession92254 canonical actualexit2, verifylog9947B SHAcaff595b806792a04da28e9f99e6c806ca634bb3836543ac6818dafc85afa6f4; make observedrecipeError1, separate parentrawstatus notavailable. Positive PASSED0/check0/all28true/16settled, finaldebt13.286537622s/reportmargin14.438642s preserved. Corrupt staysFAILED/quarantined/raw16/15, no childadmission/rollup/coverage; teardown creates no proof. Corruptstderrstill empty, finaldebt/split/margin/terminal unavailable; raw12.354342750s only. All8currentrawartifacts and24historicalledgers byte/hash/dev/inode unchanged; originalprefix189320907 retained. No retry/finalgates/qualification/receipt/landing. Signedcleaneb42d39aafb86d9de7ded7ec30d9bc00a13297cf unchanged. Full final disposition report#93377961366457 committed knowledge525c12eb6150e888b9710cc8b07701ddc7b07d35 supersedes pending state#66454; R/quarantine-disposition-20261109/handoff.json SHAdbdf814dbce29a9b81e30e0ba00f305c34bb81e214d22c72a663aef70e578503. INPUTloopPAUSED/parked for Architectreview; runtime_approved=false.

```json
{
  "op_id": "01M7041XWYNZ6VHYA25ANF3X7Z",
  "commit": "eb42d39aafb86d9de7ded7ec30d9bc00a13297cf",
  "report": "[[reports/2026-11-09-dream-proof-tools-input69-disposition.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Authorized inventory and same-original controller identity before exact-PID teardown",
        "reference": "runs/01M7035811Q2HTZ37K75WQ5K3C/quarantine-disposition-20261109/authorization.json message01M703R0TFV6W7FRQR6KKQEX6G;inventory-before.json ps90389ppid43277pgid90389/startNov9 04:07:45/exactprobe+inputSHAcd1525c5;nochildren;lsofactual0/FD55and5"
      },
      {
        "criterion": "Exactly authorized TERM then delayed KILL, real actual wait4 and canonical exit",
        "reference": "D/term-result.json0;after-term-check.json elapsed31057555875ns/sameidentitySNs;kill-result.json0 only90389;corrupt/wait4.jsonSHA60cfcc80718002765d1758f852cc10fdf99411d9d5f9ee68dd7bab7812bb896c actual-9/raw9;E/verify-exit.json actual2/logSHA caff595b806792a04da28e9f99e6c806ca634bb3836543ac6818dafc85afa6f4;finalps90389,43277absent"
      },
      {
        "criterion": "Positive independent pass preserved; corrupt failure and missing final accounting remain honest",
        "reference": "positivewait4/check0,ledger108 all28true/firstnone/16settled/finalpaid13286537622ns;corrupt raw16/15 child91380 image0/rollup0/unproven1/witnessnone;stderr0bytes, no terminal/conjunction/finalsplit/margin;no signal status treated as qualification"
      },
      {
        "criterion": "Raw/historical evidence and code/prefix identities preserved without another attempt",
        "reference": "D/preservation.json all8rawfiles+24historicalledgers length/hash/dev/inode unchanged;prefix189320907retained;D/final-state.json GoodBoxyeb42d39a/diffcheck0/status0empty;no new source/retry/qualification/receipt/landing/finalgates;runtimefalse"
      },
      {
        "criterion": "Committed final handoff and explicitly paused INPUT loop",
        "reference": "reports/2026-11-09-dream-proof-tools-input69-disposition.md #93377961366457 nativecheckpoint525c12eb6150e888b9710cc8b07701ddc7b07d35 supersedes pending#66454;D/handoff.jsonSHAdbdf814dbce29a9b81e30e0ba00f305c34bb81e214d22c72a663aef70e578503;OraclepausedloopforArchitectreview/noINPUT70"
      }
    ]
  }
}
```


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

Offline shim preparation DONE only; remain parked for INPUT70 freeze. Read decision66459 INCLUDING correction: RLIMIT_CPU itemswithdrawn, failed live reads ignored, successfulovercap kill backstop/terminalwait4CPU proof; no law implementation guessed. Current task/ICDstillINPUT69, exactcanonical_q2 sourceRef/hash/10inputs/git+gpg roots and fields have no shimcontract; mapped precise selection/prepare/compiler-output/source-lock/closure-review/compiledDREAM_PROOF_GPG/gitgpg.program/descriptor/accounting/result-terminal/inertfixture seams at existing source lines. No code/source-lock/authority/schema/budget/model change, no build/test/native attempt or cleanup. HEAD eb42d39aafb86d9de7ded7ec30d9bc00a13297cf unchanged/clean; actual gitdiffcheck0/status0empty;24historicalledgers unchanged. RetainedINPUT69 Basis47112B leaves2040B of48KiB; known descriptor+basis+lock+raw result59664Bpositive/59777Bcorrupt excluding qualificationenvelope and unknownshim additions, so INPUT70 fit unproven underunchangedcaps. Exact shim source/exe/preparation, compiler/header bindings, source-lock/descriptor/closure contracts, verifierargv and resultsemantics will come fromfreeze; no fabricatednames or pins. Full report#93377961366464 committed knowledgec41118de22400956f6cf298ef10bac1aefd966cf; private target/dream-proof-tools/shim-offline-preparation-20261109/handoff.json SHAb65131a743862e48e47bf727f3583dde017632ef35fa03055a49f82b7ff550cd. Reportmetadataverify gitdiffcheck0 only; canonicalverify NOT rerun, INPUT69 actual2/failed status and teardown report#66457 unchanged; runtime_approved=false.

```json
{
  "op_id": "01M70G26HQBDTCR9V7KZAEQ5VZ",
  "commit": "eb42d39aafb86d9de7ded7ec30d9bc00a13297cf",
  "report": "[[reports/2026-11-09-dream-proof-tools-shim-offline-preparation.md]]",
  "verify_result": null
}
```

## ORACLE RULING (2026-11-09): basis ceiling headroom for INPUT70's shim bindings
The builder reports that the INPUT69 basis is 47,112 B of its 48 KiB ceiling (2,040 B free). The known descriptor+basis+lock+raw result is 59,664 B (positive) / 59,777 B (corrupt), well inside the 128 KiB TOTAL.
Ruling: the pre-test basis individual ceiling may rise from 48 KiB to 56 KiB, ONLY to carry the exec-shim bindings from decision 66459 (shim source/exe hashes, its allowlist/form table digest and its compiler binding).
Unchanged:
- the 128 KiB total;
- the descriptor 16 KiB, lock 32 KiB, raw result 64 KiB and receipt 32 KiB ceilings;
- the node/array/edge/depth limits.
The shim executor must bind the shim by hash and reuse the existing projections where possible, without restating them. A basis over 56 KiB is refused, with its exact size reported.

## ORACLE RULING (2026-11-09): git's allowed child becomes the exec shim (decision 66459)
The pinned git patch (git-2.54.0.patch:16) lets git spawn only DREAM_PROOF_GPG, a compile-time define set at scripts/dream-proof-tools.py:847 to <candidate>/bin/gpg. With gpg.program=shim, git therefore refuses before effect (shim executor witness, research #93377961366466). That is correct behaviour; the define must point at the shim.

Phase A (authorized NOW, build-only, no native qualification run):
- Add one recipe parameter, DREAM_PROOF_GPG_TARGET, defaulting to <candidate>/bin/gpg (current behaviour unchanged when unset).
- Build ONE private witness git into target/dream-proof-tools/shim-witness-<date>/ with DREAM_PROOF_GPG = the absolute shim path the shim executor (seat 01M70FJGXV7W40W80200Z0TDN5) supplies.
- Hand the executor that git's absolute path and sha256, plus its patch/recipe hashes. Make no candidate/qualification/receipt changes, and do not run the native probe.
Phase B (with the INPUT70 freeze):
- Install the frozen shim binary at <candidate>/libexec/dream-exec-shim (mode 0500, hash-bound in the source lock/basis).
- Set DREAM_PROOF_GPG to that path; the shim execs <candidate>/bin/gpg.
- Re-review the source closure for the patch and define change.
Sign the commits; the patch's other guards are unchanged.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

Phase A COMPLETE. Signed33c7a896bedd80968a6a1dafcccb64eec0fbe605 adds explicit keyword-only DREAM_PROOF_GPG_TARGET; None preserves candidate/bin/gpg exactly, ambient env ignored, unsafe/nonabsolute/nonnormalized macro paths refused before effects.69units0, cleanstatus/diffcheck0/GoodBoxy. ONE build-only witness Git at /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-witness-20261109/bin/git SHA1955c95c2daa1ea5ebb8914fd32d0efe2f89d8fec6c64b1982fded31a1f0da81,3766096B. Compiled DREAM_PROOF_GPG exactly /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/prepared-witness/dream-read-c-preparation/compile-t01d4h8x/dream-exec-shim (peer message01M70HZAZQPMJPQ8W7VD1ZV106, shimSHAeea84097...,receiptff0a7b...,signedb665Good). Gitpatch unchangedSHAa48b33776ae5a88308e096ea62640487706e360c374ec7eaf64d50adf47818b8; recipeSHAad3fd381408f358edf6417bbaa2988c55d50d30d8d8ddf4ddd03c2727a96d4fc. ExactCFLAGS in W/logs/008.command.json and src/git-2.54.0/GIT-CFLAGS SHA4cb9fe77ad64491677de89e401e30d7dd0df3c9aac6e4486e2287f2bd3601467; same literal in linkedGit/defaultW/bin/gpg literalabsent. Seven fresh source authentications/exactpatchpre-postimages0;11commands000–010all0, exactlyone make0080/two nicedjobs. Result DM01M70J4DM1MW037N9QAQM4THVM sent shimexecutor withpath/hash/patch/recipe/compiler/define evidence; receipt nativequeued, modelconsumptionunverified. No witnessexecution/nativeprobe/qualification/receipt/candidate/source-lock/budget/law/model change or retry;26historicalledgers unchanged. Full report#93377961366470 committed knowledge1974dcadeed8e78bcc4e85e1be6c0957fcffa3ab; E=target/dream-proof-tools/shim-phase-a-20261109/handoff.json SHA257cd784d4d1581bd0ba84e65780961fd4c62584d415e77dfd7e227f04d47bed. Reportmetadata verifies actualmake0 only, no canonicalverify_result submitted because fullqualification notauthorizedinPhaseA; INPUT69 actual2/failed status remains. Parked for INPUT70/PhaseB; runtime_approved=false.

```json
{
  "op_id": "01M70JAN679ARCGRPP763Z9XRA",
  "commit": "33c7a896bedd80968a6a1dafcccb64eec0fbe605",
  "report": "[[reports/2026-11-09-dream-proof-tools-shim-phase-a.md]]",
  "verify_result": null
}
```

## ORACLE RULING (2026-11-09): one revised Phase A witness build plus an --exec-path diagnostic
Supersedes "Build ONE private witness git" in the Phase A ruling above, ONLY as stated here.
- Authorized: exactly ONE additional build-only witness git in a FRESH directory, target/dream-proof-tools/shim-witness-20261109-b/. DREAM_PROOF_GPG is the shim executor's revised absolute path (…/witness-current/dream-exec-shim, as given in its message 01M70J9H1FSP0CFG9500NZ9PX6). Use the same 33c7a896 recipe and DREAM_PROOF_GPG_TARGET parameter.
- The first witness (shim-witness-20261109/bin/git, sha256 1955c95c…) stays untouched as retained evidence.
- Authorized: `<witness-b git> --exec-path`, which only prints the compiled exec path. No fixtures, probe, qualification, candidate, source-lock or receipt change.
- Send the shim executor the new git's path, its sha256 and the exec-path output, and record both witnesses in your report.
- Any further witness build needs a new ruling in this file.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

Revised Phase A COMPLETE under newlyrecorded task ruling etag2dd5db6d49cf3776c852b8bfc48043b2b1bcaadfb071863963df0234f570622a. ONE additional fresh witness /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-witness-20261109-b/bin/git SHA5981ccf53ca713c4c80030f2a872f31696e00a03729e6fc463e31a8f17cd0265,3766096B/mode0700, make008actual0. DREAM_PROOF_GPG exactly /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/witness-current/dream-exec-shim;008argv/GIT-CFLAGS SHA0e78b7ad04265c4df4d5f0cf446dc76e7d70f3ed8bf6258d895411874e6e00a0/linkedliteral bindit. Only newGit execution011 --exec-path actual0, stderrempty, stdout exact /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-witness-20261109-b/libexec/git-core plusLF, SHA3cc8afae3ae2c884700923b489bd2ca55859272ac50a0400cef4d751c581f7d2. All12commands0/7freshsourceauth0; one additionalmake, twoPhaseA builds total. First witness1955c95c2daa1ea5ebb8914fd32d0efe2f89d8fec6c64b1982fded31a1f0da81 andfiveoriginalartifactpins/26historicalledgers untouched. Recipead3fd381408f358edf6417bbaa2988c55d50d30d8d8ddf4ddd03c2727a96d4fc/patcha48b33776ae5a88308e096ea62640487706e360c374ec7eaf64d50adf47818b8 unchanged; signedclean33c7a896, previous69units0 onidenticalcode. DM01M70JRXTHK8VPHXNS19TQKG27 deliverednewpath/hash/execpath/evidence toshimexecutor (nativequeued, modelreceiptunverified). No fixture/shim/probe/qualification/candidate/source-lock/receipt changes; targetbindingisstagingpathliteral, no frozen70shimapproval. Report#93377961366474 committed knowledgee51f4050d8d548c482cf2c7e265d51b7f8007cc3 recordsboth/supersedesactivehandoff#66470; E=target/dream-proof-tools/shim-phase-a-20261109-b/handoff.json SHA90eb46d27108d3a8982e33babf316b681e984edbc25676ebac96f4441aff0273. No fullcanonicalverify_result supplied: canonicalnotrerun/INPUT69actual2failed remains. Secondandlastwitnessunderthisruling; parkedINPUT70/PhaseB, runtime_approved=false.

```json
{
  "op_id": "01M70JXMAPYTHRWX4SDYTTM5PR",
  "commit": "33c7a896bedd80968a6a1dafcccb64eec0fbe605",
  "report": "[[reports/2026-11-09-dream-proof-tools-shim-phase-a-revised.md]]",
  "verify_result": null
}
```

## ORACLE FREEZE, INPUT70 (2026-11-09). Supersedes INPUT69. Exec-shim enforcement (decision 66459 + correction + Amendment 2). Phase B is AUTHORIZED.

Verified by the Oracle from the tree:
- Source 76045587db4a96612e2406b9bb08d4191c626253: `git verify-commit` Good (Boxy), and it is ON harness origin/main; 637cb234 (INPUT69) is an ancestor. The shim executor's worktree is clean.
- The verify record (required-four-v2.json) shows exit 0 at that head, and canonical make land exited 0.
- Tuple: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/INPUT70-source-freeze-tuple.json, sha256 3d42b6ecbdd2a3b5da9ac27a31a6cc7ed1c971300937616d567a8dd7dda0b7d4.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/prepared-input70/dream-read-c-preparation/compile-u3u9hnyr/
- Probe: 303256 B, sha256 aa6e1b28bdc60cf2b4154df616768dfaad743fb51c3028e74a6067a94c8db00b, mode 0500.
- Shim executable: dream-exec-shim, 35520 B, sha256 4e95b2a83a34a6328d3c794133c3569dff7e5f1b2ee2c9bfc1d58cd08447527d, mode 0500.
- Shim source: dream-exec-shim.c, sha256 34a9a6ee…. Shim header: dream-exec-shim.h, sha256 d86a1129…. Both equal `git show 76045587`.
- Receipt: preparation-c24749c7481562f853f776dc0004d8197bd25efc3b6508890dfdcf94f228a56b.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 12 tuple inputs match both git and the snapshot, now including the shim source and header.
- Basis: the shim delta is 343 B, projecting about 47,455 B, so it fits even the original 48 KiB; the 56 KiB headroom is unused.

Phase B (builder):
1. Install the frozen shim executable (the exact sha256 above) at <candidate>/libexec/dream-exec-shim, mode 0500, hash-bound in the source lock/basis.
2. Rebuild git with DREAM_PROOF_GPG_TARGET = <candidate>/libexec/dream-exec-shim through the 33c7a896 recipe parameter. The patch is unchanged.
3. Re-review the source closure for the define/target change, and record the documented limitations from decision 66459 Amendment 2 (coalesced NOTE_FORK, setsid orphan) as explicit assumptions.
4. Pin INPUT70 (probe, shim, receipt, inputs).
5. Accept exactly the ledger/basis field changes listed in the tuple's new_internal_ledger_fields_manifest (field-changes.json, sha256 c731c632…) and new_private_basis_fields. The terminal/public wire is unchanged unless the tuple says otherwise.
6. Make ONE native attempt with all cases in one invocation.

runtime_approved=false.

## ORACLE RULING (2026-11-09): INPUT70 clock_failure control may carry one causal auxiliary coverage error
In the INPUT70 frozen producer (C ~4482/4495-4509), the clock_failure control injects clock_unknown BEFORE the suspended shim's initial CONT. The shim therefore never runs and cannot emit its in-process exec receipt, so retirement (C ~4752) adds coverage_unknown "helper lacks its one in-process exec receipt". That second error is a direct consequence of the first, not an independent defect.
The builder's strict clock adapter may accept, ONLY for the clock_failure control case:
- first error clock_unknown (the injected one), plus the previously authorized exact resource_limit diagnostic;
- plus EXACTLY ONE auxiliary coverage_unknown with that exact detail text;
- and ONLY IF the ledger shows no CONT was ever sent to that shim and no exec receipt exists.
Anything else stays refused: any other auxiliary error, that coverage_unknown in any other case, or a CONT/exec receipt present alongside it. Add a checker unit test for the allowed combination and a refusal for each of those other cases. No producer change and no law change.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT70 Phase B implemented, signed and clean at99bda9ca5592ae8e4ce2d4098c80a506a6c5364c;78 units0, frozen offline ingestion0,7 fresh source authentications0 and60 build commands0. ONE native invocation R=01M70S5HNZYVKVYAE6SWSPJ1NE FAILED on positive: controller54442 actualwait4 exit81/raw20736, settled16/16, no quarantine. Frozen C cleanup BSD classifier still refuses coverage_unknown 'owned observation unavailable: escaped_cleanup_bsd' (C4663–4668→2849–2852→3430–3450), outside shim failed-read tolerance. Fatal errno/PID not serialized;15 errno3 rows are recovered takeovers.16 shim receipts all checks true/priority4, including verifier55601, GS1/GS2 bound; no shim refusal reported. No final_case_conjunction/first_failing_term; proof not_reached. Final debt14.684130s=helper2.183130/raw2.454971625/floors10.046028375, cutoff margin12.819209s, ordinary_max850792ns, excess0. Basis47490B/nonshim47147/delta343. Later7cases/finalgates/qualification/receipt notrun; canonicalactual2. No retry/signals/producer edit;26 historicalledgers andbothPhaseA witnesses unchanged. Full report#93377961366480 committed knowledge5f782fb970287d67ad9200f92830a42f656a3338; E/handoffSHA59a6e21ca9a7c9f0966023bebc5b4ffd6692bdfa321ef6db1b44b0a4f1279c50. Oracle/Q2 disposition/newfreeze needed before another native attempt; runtime_approved=false.

```json
{
  "op_id": "01M70SKZ4GF8M8136RTWAPQEN3",
  "commit": "99bda9ca5592ae8e4ce2d4098c80a506a6c5364c",
  "report": "[[reports/2026-11-09-dream-proof-tools-input70.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact INPUT70 source/preparation/shim pins, fixed installation, compiler define and closure assumptions",
        "reference": "scripts/dream-proof-tools.py:52,769,863,2110,2443;E/offline-ingestion.json0;phase-b-build-bindings.jsonSHA5d4fcfa09cd7ba0efe9893c9044af034bb228dd6e522aa9149f9200f517b9e28;shim4e95b2a8/mode0500;Git01c737bd;GIT-CFLAGSdc8d20f4;review.lock.json eight findings/explicit66459 limits"
      },
      {
        "criterion": "Closed preparation/Basis/custody and narrowly authorized causal clock checker controls",
        "reference": "78units0 E/unit-v3.logSHA66a3ac67f0002a550776c808ad6597b30982e33ebd41844823ae81a61e2b8cb2;shim grammar/resealed inner/header/define/custody/currentness/budget negatives;clock descriptor-bound no-CONT/no-exec and extra-count/detail/evidence refusals;clock native not reached"
      },
      {
        "criterion": "Real original positive outcome and complete physical settlement",
        "reference": "R/components/positive/wait4.jsonSHA797e680a6127d28cb517cd125b2ff841807a72af97a865c0c432b17e698e6fea actual81/raw20736;stdout/persisted resultSHAcbc8759960fc7fcc1d1d79359e0127b9008bc63463931302e5373ac094b24f4c;16/16 groupgone/retired true;Ccleanup coverage refusal;no conjunction or semantic proof;final ps54442 absent"
      },
      {
        "criterion": "Unaltered shim/accounting/telemetry evidence and no invented fatal errno",
        "reference": "R positive ledger76423B SHA8e27eff280930094a7488224a6ff5336b66de25a1a5e4795393df7fc6e6e5a35;16successfulshimrows;15recovered errno3 takeovers;fatal errno/PID unavailable;terminal14.684130s split and12.819209s cutoffmargin/ordinary850792/excess0;E/result-evidenceSHA4e212089366c5065063218bdd996aca32f419ea6fa4a64b5889402ea259aba7f"
      },
      {
        "criterion": "Actual canonical refusal, retained history and committed handoff with honest unrun gates",
        "reference": "E/verify-exit.json actual2;verify.logSHA0cb6ba3029a40375bb2fd695f4d1ffac16a300c331638c383ed0e09346383203;7auth/60commands0;26oldledgers unchanged;witnessGit1955c95c/5981ccf5 unchanged;GoodBoxy99bda9ca/diffcheck0/clean;report66480 native5f782fb970287d67ad9200f92830a42f656a3338;latercases/finalgates/qualification/receipt unrun"
      }
    ]
  }
}
```

## ORACLE FREEZE, INPUT71 (2026-11-09). Supersedes INPUT70. Post-reap escape check: ESRCH means not escaped, other read failures are diagnostics, only a positive same-incarnation live hit refuses (66459 A2).

Verified by the Oracle from the tree:
- Source ad6c8b72fe0bb766164c879acd27c1ece2e565a7: `git verify-commit` Good (Boxy), and it is ON harness origin/main; 76045587 (INPUT70) is an ancestor. The worktree is clean. The tuple records verify 0 and land 0.
- Tuple: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-postreap-escape-witness/INPUT71-source-freeze-tuple.json, sha256 f8d82d84b92557d346bc4942d398a21a841addd5769420d914d55c26519cccf1.
- Directory: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/harness-landing/dream-read-c-preparation/compile-_ybqbhvt/
  DO NOT DELETE: it sits inside the harness-landing build cache, so exclude it from any cache cleanup.
- Probe: 303256 B, sha256 9a9ee37905cdbe6d04c933c7ffc902d06fa29345418dbcd4a59161c4ebf078dc, mode 0500.
- Shim executable: 4e95b2a83a34a6328d3c794133c3569dff7e5f1b2ee2c9bfc1d58cd08447527d (byte-identical to INPUT70, so the Phase B install at <candidate>/libexec/dream-exec-shim stays valid).
- Shim source: 34a9a6ee…. Shim header: d86a1129…. Both are unchanged.
- Receipt: preparation-ab2227a84f7e1e522bf890910492d7fe1841773a16c98de658de62d41613d521.json; its hash matches the name.
- Modes: 0 files outside 0400/0500, 0 with nlink>1, 0 directories outside 0700.
- All 12 tuple inputs match git and the snapshot; proof-helper-probe.c is 077d63a1….

The builder reuses its Phase B (99bda9ca) and the clock_failure checker ruling, re-pins to INPUT71, accepts exactly the field changes the INPUT71 tuple lists (if any), and makes ONE native attempt with all cases.
runtime_approved=false.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

INPUT71 run 01M70WYNQV6MRHPMDTEMR8Q1CJ: positive and corrupt_proof actual exit0/check0, pause_resume specified exit81/check0, all three with28/28 terms true, proof valid/invalid/valid and16/16 settled. cancel and stale_incarnation exit81/check0. Canonical actual exit2 at clock_failure: actual81/settled1/1, but my extra99bda9ca host gate requires primary clock count1/equal endpoints. Frozen C emits the deterministic count3 cascade (first injected creation clock, last closed engine unit unknown CPU), with exactly one authorized coverage auxiliary and one exact resource diagnostic. Direct retained result and pre-CONT ledger checks both0; no CONT or exec receipt. This is my overly strict host assumption, not a producer defect. Decision needed: confirm that count3 causal primary shape and permit correcting only that restriction; any new native invocation needs a new ruling because the sole attempt is consumed. No retry/retrospective pass/signals; all six controllers reaped, no teardown. Debt totals/margins positive14.393115s/13.293223s, corrupt13.600584s/14.392810s, pause14.549598s/13.251674s;49 successful shim receipts, no shim refusal or escape-unavailable rows (ESRCH silence is conditional). Pause STOP race errno1 resolved. debt_overlap/reap_delay/final gates/qualification/receipt unrun.78 units, frozen ingestion,7auth and60build commands all0; signed clean pin-only a7bb5535,27 historical ledgers unchanged. Full report#93377961366492 committed knowledge56e83bb6dfa01bcfa0a782a7119a2571110e7c12; E/handoff SHA cb3d3ddefbfd885f9ca6a832af42200487eac39cc2b4d224d7242aaa278e1bdc. runtime_approved=false.

```json
{
  "op_id": "01M70XJYG8T8A29W41Z9MK1PX1",
  "commit": "a7bb5535010affe9ddebc3b4407b794ef060ca26",
  "report": "[[reports/2026-11-09-dream-proof-tools-input71.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exact INPUT71 selection, unchanged shim, closed offline ingestion and conditional escape semantics",
        "reference": "scripts/dream-proof-tools.py:52,2492,2745; signed sourcead6c8b72 and12git/snapshot inputs E/frozen-input-checks.json; tuplef8d82d84…cccf1/receipt ab2227a8…d521/probe9a9ee379…78dc; offline ingestion0 SHA b7db0c4b6d6ef2715050cb887fda209113493ed9cd70f1d7b0825fdd8c486b9b; field manifest8aeb4c12…f941; README conditional group_gone; shim4e95b2a8/mode0500"
      },
      {
        "criterion": "Three real proof cases passed without changing actual exit semantics",
        "reference": "R positive wait4 exit0/checked0/28true/firstnone/valid; corrupt exit0/checked0/28true/firstnone/invalid; pause exit81/checked0/28true/firstnone/valid; each16/16 settled. Ledgers c44a018ee3be936629905c5c05685a59d1aca9da38c273af16cf6822c77c3e7b, b2cbdd6d77ab52a8b7207d098d463bcb18cc9e669fee747557a69b840c36c991,58e791bf27126fbffce0b6a08474d76529363fd6dcea4d6f7688a3ced6b282fc"
      },
      {
        "criterion": "Causal controls and exact host clock blocker preserved with no waiver or retry",
        "reference": "cancel/stale actual81 checked0; clock actual81 raw20736, histogram3clock/1coverage/1resource; ledger8d1f9d6b215fccd2006c0a8a8c719fe17485f06b4a93541e2b9bcd068053cbcd activation/birth/start0/noexec/noCONT/reaped1; E/clock-checker-diagnosis.json result0/causal0/terminalrefused SHA0b36b75cad53521327b393b40d5768d7ae2f6b4a978275215271ec457ff04850; own added primary-count gate3371–3374 identified; latercases unrun"
      },
      {
        "criterion": "Debt, margins, shim and escape/pause diagnostics retained without summing repeated rows",
        "reference": "E/result-evidence.json SHA65b6d8ad81d6b8d010c66edc9382405cd71cfd964cfd6e21985ba78c275ae916;49 alltrue priority4 shim receipts; escape diagnostic rows0 does not prove absence; all final charged=paid; clock raw incomplete with derived27ms unknown debt; pause original per-owner errno/order/resolved/interval fields and errno1 resolved ownwait4 race in committed report"
      },
      {
        "criterion": "Actual canonical exit, offline checks, clean signature and preserved history",
        "reference": "E/verify-exit actual2; verify.log SHA3f51ee48c904f71e22986348abe764d6a0e2e6f7443ecd3c8221972aaf1f5cbb;78unit0 logSHA97cf123a29ac4f21329007367fcdcaa18a207b4e1f8e6fc64107c093efc89411;7auth/60commands0;GoodBoxya7bb5535/clean/diffcheck0;27historical ledgers unchanged;six controllers absent; no final formal/qualification/receipt; report66492 native56e83bb6dfa01bcfa0a782a7119a2571110e7c12"
      }
    ]
  }
}
```

## ORACLE RULING (2026-11-09): INPUT71 run result; clock_failure primary-count correction; ONE new native attempt
Run 01M70WYNQV6MRHPMDTEMR8Q1CJ (builder handoff sha256 cb3d3ddefbfd…):
- positive, corrupt_proof and pause_resume: 28/28 terms true each, 16/16 settled;
- cancel and stale_incarnation passed (exit 81 / check 0 as specified);
- 49 shim receipts, no shim refusal;
- the pause STOP race resolved.
The canonical run exited 2 at clock_failure ONLY because the builder's own host gate (99bda9ca) required a primary clock count of 1 with equal endpoints. The frozen C deterministically emits a count-3 causal cascade: the injected creation clock first, then the follow-on, then the last closed engine unit with unknown CPU. It carries exactly one authorized auxiliary coverage error and one exact resource diagnostic, with no CONT and no exec receipt (both checked directly by the builder).
Ruling:
1. Correct ONLY that host restriction. For the clock_failure control, accept a primary clock count of exactly 3 in that exact causal order: first = the injected creation clock, last = the closed-engine-unit unknown CPU. Every other clock count, ordering or endpoint shape stays refused. Keep the earlier one-auxiliary-plus-resource-diagnostic ruling and the no-CONT/no-exec-receipt requirement unchanged. Add checker unit tests: count 3 in this order passes; counts 1, 2 and 4 refuse; a reordered count 3 refuses.
2. Authorized: ONE new native invocation on the SAME frozen INPUT71 (no producer change), all cases. The result of run 01M70WYNQV6MRHPMDTEMR8Q1CJ is not retroactively reclassified; it stays exit 2.
runtime_approved=false.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

ALL8 INPUT71 native checks PASSED in new run01M70XWHWKFCD1W6ZZMQB50WG1, including corrected clock_failure, debt_overlap and reap_delay. Positive/corrupt/pause each28/28 true; actual exits0/0/81 preserved. All controllers actually reaped; reap_delay initial quarantined1/0 wire stays unchanged while final terminal settles1/1.50 successful shim receipts, no shim refusal/escape-unavailable rows; conditional escape semantics retained. Signed clean c50efb4e changes ONLY primary clock exactly3/ordered endpoints and tests, including no-aux bypass negatives;79 units0. Canonical actual2: final command060 make formal-parity/licenses fails beforeRustcompile because configured /opt/homebrew/bin/sccache rustc -vV exits2, 'path must be shorter than SUN_LEN'. HOME/TMPDIR122B; offending socket path unrecorded.7auth/commands000–059 all0;060 is2, formal/licenses unverified, no qualification/receipt. Decision needed: authorize final-gates-only continuation with RUSTC_WRAPPER='', reuse/revalidate original8 records WITHOUT native rerun, preserve060 and existing unsupported R/terminal.json, and define a collision-safe append-only completion record (publish_candidate3050 otherwise collides after receipt). Production recipe/c50 retained source hash a8187a0c unchanged; no environment/source patch or continuation attempted. Prior71 run stays2;33 historical ledgers unchanged. Full report#93377961366496 committed knowledge22f2ac21102226dd8c7b9751b641a426160e7ff3; E/handoffSHA4cededbe4b20e588550dec565a0d5c19c7d0fbb248f12239c4ae18d71c3af240. runtime_approved=false.

```json
{
  "op_id": "01M70YWFQG1TG8FQDVP4423SG9",
  "commit": "c50efb4e155b919f1d5a34620465ce14be725bd6",
  "report": "[[reports/2026-11-09-dream-proof-tools-input71-clock-correction.md]]",
  "verify_result": {
    "command": "make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Exactly authorized clock correction with complete positive/negative checker coverage",
        "reference": "scripts/dream-proof-tools.py:3359–3366 outer clock-case gate; count3 exact endpoints; tests reject1/2/4/reversed3/oldsingle/changedendpoints with and withoutaux; unchanged exactresource/onecoverage/noCONT/noexec/result/debt/persistence;79unit0 E/unit-final-v2.log SHA6287f5f32c0fa4e5ac3332b41633af867bf5443ccb0d774266f0b2675421c3e9;GoodBoxyc50efb4e155b919f1d5a34620465ce14be725bd6/clean"
      },
      {
        "criterion": "All eight real native cases accepted with actual exit and original-wire semantics preserved",
        "reference": "R/component-checks.json eight check0; positive0/corrupt0/pause81 each28true firstnone valid/invalid/valid; cancel/stale/clock/debt_overlap/reap_delay81 expectedcontrols;finalterminals16/16 or1/1; reap partial1/0 wire preserved and originalwait4 proveslaterexit; E/result-evidenceSHA17225997801ea0923fa9c6ee4243db75d254ad5e1f86bc2104760e4abb702e19"
      },
      {
        "criterion": "Per-case debt/margin/ordinary/shim/escape/pause telemetry retained without invented completeness",
        "reference": "Committed report66496 tables all8 debts/margins/maxunits; clock raw incomplete with derived27ms unknown-cost remainder; allcharged=paid/no deadlines/excess0;50alltrue priority4 shimreceipts;escape diagnostic0 is conditional/ESRCHsilent;pause original15ownerrows;Rrawledgers/stdout/result equality inE"
      },
      {
        "criterion": "Exact final-gate failure, no formal/license verdict or candidate publication",
        "reference": "R/logs/060.command.json SHA164ca15ae2d5977e8fe5fca1a993a616876da3f04aa0cd366535d61b68f5346d;exit2;stderrSHA0e0803ec81065cd71453252163922ddb97cdb4e48f4a8b84af6aadb00b640339 shows sccache SUN_LEN before rustc -vV;CARGO_HOMEconfiguredwrapper/TMPDIR122B;E/final-gate-diagnosisSHAcbf7e21a59454a7621f63df8fea1788c8d1934ad7d36becd80275940413c936a;no qualification/receipt;unsupported terminal retained"
      },
      {
        "criterion": "Canonical actual2, preserved previous effect and committed full blocker handoff",
        "reference": "E/verify-exit actual2;verify.log SHA20c49fe0236f1fa50707886041e248f1bc4978d5a76dddaa2e38aa8a8ae36015;7auth/first60commands0/0602;33historicalledgers unchanged;old71exitSHA8b2fd604abbab82e5e3d43858c568b4eeb91284615e6c37828f114b81abd9f64 remains2;8controllersabsent/no signals;report66496 native22f2ac21102226dd8c7b9751b641a426160e7ff3;continuation notattempted"
      }
    ]
  }
}
```

## ORACLE RULING (2026-11-09): INPUT71 all 8 native checks PASSED; final-gates-only continuation
Run 01M70XWHWKFCD1W6ZZMQB50WG1 (builder handoff sha256 4cededbe…, report #66496):
- ALL 8 native checks passed: positive, corrupt_proof, pause_resume (each 28/28), cancel, stale_incarnation, clock_failure (corrected count-3 gate, c50efb4e), debt_overlap and reap_delay;
- all controllers were actually reaped;
- 50 shim receipts, with no shim refusal and no escape-unavailable rows.
Canonical command 060 (make test-formal-parity / test-licenses) exited 2 BEFORE any Rust compile: the configured /opt/homebrew/bin/sccache failed `rustc -vV` with "path must be shorter than SUN_LEN" (its socket path is too long under the private HOME/TMPDIR). This is build-tooling only and unrelated to the candidate.
Authorized:
1. A final-gates-only continuation running the remaining canonical steps (060 onward: formal parity, licenses and the remaining final gates) with RUSTC_WRAPPER='' (sccache disabled for these steps only). No other environment change, no source/recipe change; c50efb4e and the production recipe source hash a8187a0c are unchanged.
2. Reuse and REVALIDATE (hashes, custody, currentness) the 8 original native records from run 01M70XWHWKFCD1W6ZZMQB50WG1. NO native rerun.
3. Preserve the failed 060 record and the existing unsupported R/terminal.json unchanged, as history.
4. Write the continuation as an APPEND-ONLY completion record under a NEW collision-free name (for example final-gates-continuation-<ULID>.json) that binds: the run ID, the 8 record hashes, the failed 060 record hash, the exact continuation argv/env delta (RUSTC_WRAPPER='') and every gate exit. Publish the candidate/receipt under a name that cannot collide with publish_candidate 3050; never overwrite an existing receipt.
5. If every final gate is 0: produce the qualification/receipt as the task specifies, with runtime_approved still FALSE (approval is a separate Root review step) and conditional_on_bound_source_review.
Report once.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

Final-gates-only continuation revalidated all8 original native records, current c50/a818 recipe/fixtures/prefix/Basis/tools/platform and actual-wait bindings; native reruns0. Command061 used EXACTLY060 env plus RUSTC_WRAPPER=''. Sccache problem resolved: Rust compiled; formal_models26, formal_nonvacuity55, reactor_projection1 all passed. Then scripts/formal.py --native refused 'upstream main SOURCE hash changed; re-copy scripts/formal.py';061/driver actual2. Local vendored pins all match; diagnosed bboxy450-term main96dbc71b has formal.py5097490d versus declared09ca7496, test-formal.py11c3d433 versus7627077e; verify.rs unchanged. This is outside current only-wrapper/no-source-change ruling and allowed paths. Decision needed: authorized formal-tool synchronization/frozen-upstream disposition plus retained-c50 source/currentness binding and another final-gates-only continuation; no native replay requested. No bypass, source/env expansion, qualification or receipt.060 and unsupported terminal preserved; append-only R/final-gates-continuation-01M70ZC391WFNE6Z2D8XS5EG5W.json12477B SHA4bc524861b9ff36b8f24e551858ddbecf2f49566ba18bf4d0e5f2a6c0cf58f83 binds run/8records/060/envdelta/061exit. All33 historical ledgers unchanged; c50 GoodBoxy/clean. Full report#93377961366498 committed knowledge8197259adfc76975489ac026b6ba430a99036a4d; E/final-gates-continuation-handoffSHAcbabd980b5d18b84096ea4c2712b644e2560f6d38426f960e4d700aa643c8334. Formal parity/licenses/remaining gates unverified; runtime_approved=false.

```json
{
  "op_id": "01M70ZW8FB2Q7J5WS94RT63DS4",
  "commit": "c50efb4e155b919f1d5a34620465ce14be725bd6",
  "report": "[[reports/2026-11-09-dream-proof-tools-input71-final-gates-continuation.md]]",
  "verify_result": {
    "command": "/usr/bin/make test-formal-parity test-licenses TARGET_DIR=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "Original8 native records reused only after actual custody/hash/currentness and semantic revalidation",
        "reference": "E/continuation-plan.json native_records8 hashes/FileRefs; exactR/component-checks equality to prior E/result-evidenceSHA17225997801ea0923fa9c6ee4243db75d254ad5e1f86bc2104760e4abb702e19;existing component_result/component_terminal all0;HEAD/script/retainedrecipe c50/a818;prefixinode/fixtures/Basis/source/preparation/compiler/SDK/tools/shim/platform checked;native_reruns0"
      },
      {
        "criterion": "Only permitted wrapper delta; sccache fixed and Rust tests actually passed",
        "reference": "R/logs/061.command.jsonSHA61384a40e5e608171873a2bb2e994f86ec2e17935de55b5caef564a3b786ca4d environment equals060 plusRUSTC_WRAPPER='';061.stdout7297B SHAb3bf08546236a50240bdc4906eac3109fcc2f8ba97bc3853dd268427962ffcb4 has26/55/1passed0failed;compile1m15s"
      },
      {
        "criterion": "Exact remaining formal-source blocker and unrun gates, without changing or bypassing pins",
        "reference": "scripts/formal.py:284–295 upstreammain SOURCE comparison;061.stderr7718B SHAf42eb15760340ca8df8563c3b50558023f20d4069f85ace90a5bada1a06f0a2a;061exit2 SHA22685235c92cdaeabe88fec56c6a805ac425d084e545abafb7c0d6127afbbe76;E/formal-source-pin-diagnosisSHAbd9acafa1143c9454a727ccfdb33a96e94e8618ef6840c64b50c6ef5171b0357 shows local pinsmatch/main scriptsdrift;no TLC/parity/license verdict"
      },
      {
        "criterion": "Append-only completion, historical failure preservation and no publication",
        "reference": "R/final-gates-continuation-01M70ZC391WFNE6Z2D8XS5EG5W.json12477B SHA4bc524861b9ff36b8f24e551858ddbecf2f49566ba18bf4d0e5f2a6c0cf58f83 outcome final_gate_failed binds original8records/060hashes/terminalhash/envdelta/061actual2;060+terminal byte/dev/ino unchanged;no prefix qualification/receipt;publication driver unexecuted"
      },
      {
        "criterion": "Unchanged production source and committed full blocker report",
        "reference": "GoodBoxy c50efb4e155b919f1d5a34620465ce14be725bd6/diffcheck0/status0empty;script+retainedrecipe a8187a0ca0b093bdf4e82c2f3e172b664f60a4e5fadc2cb0d50256b8c5d62edd;all33oldledgers unchanged;report66498 native8197259adfc76975489ac026b6ba430a99036a4d;originalcanonical2 retained and separatecontinuation2;no native rerun"
      }
    ]
  }
}
```

## ORACLE RULING (2026-11-09): rebase onto harness main for the formal-tool sync; second final-gates-only continuation
Continuation 061 (record R/final-gates-continuation-01M70ZC391WFNE6Z2D8XS5EG5W.json, sha256 4bc52486…) got past the sccache problem; formal_models, nonvacuity and reactor_projection passed. Then scripts/formal.py --native refused "upstream main SOURCE hash changed; re-copy scripts/formal.py". The builder branch is based on the OLD harness 38d99096, which declares stale upstream hashes. Current harness origin/main (ad6c8b72, which is also INPUT71's exact source) already declares the term-main hashes, formal.py 5097490d and test-formal.py 11c3d433.
Authorized:
1. Rebase feat/dream-proof-tools onto harness origin/main (ad6c8b72 or later). Resolve conflicts ONLY within the builder's existing allowed paths; do NOT edit verification/upstream* yourself, since main's copies are taken as-is. Re-sign every rebased commit. Record a mapping from old commit to new commit (c50efb4e -> its rebased SHA).
2. Currentness binding: after the rebase, verify byte-for-byte that the builder-owned files (scripts/dream-proof-tools.py, tests/fixtures/dream-proof-tools/**, its tests and models) are IDENTICAL to c50efb4e (recipe a8187a0c etc.). If they are identical, the 8 native records of run 01M70XWHWKFCD1W6ZZMQB50WG1 stay valid; record that proof. If anything differs, stop and report.
3. Make a SECOND final-gates-only continuation (from 060 on, RUSTC_WRAPPER='' as before, no native rerun), appended under a new collision-free record name that binds the rebase mapping and the identity proof. Preserve 060, 061 and terminal.json as history.
4. If all gates are 0, produce the qualification/receipt (runtime_approved=false, conditional_on_bound_source_review) under a collision-free name.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

Second final-gates-only continuation completed. Rebased all42 commits onto exact origin/main ad6c8b72 and re-signed each; c50 -> fe137b8801ef3eb2a07710367422a0da2713f519. All38 dedicated builder files are byte-identical; shared registry preserves main plus exactly3 builder lines; all main corpus/contracts retained. No upstream-tool edits. Revalidated original8 native checks with hashes/custody/currentness/actual wait4; native reruns0. Actual062/063 formal parity and064 licenses all0; 520 full native/TLC model comparisons,105 contracts,26/55/1 Rust tests,12 tool tests and10 license tests passed. Unchanged recipe.check passed79 unit tests and retained component validation through the recorded exact commit-origin lookup bridge; literal new-HEAD make target is not claimed. Qualification39083B SHA83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335 and receipt969B SHA452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c published without replacement in original candidate prefix. Append-only completion44d2565dd55863ff470cbe1d04c7e4c37c3630de43fa9b21013c07fcae586001 binds mapping/proof/original8records/old060061/envdelta/all0 gates/artifacts. Old060/061 actual2, unsupported terminal, original canonical2 and33 historical ledgers unchanged. GoodBoxy HEAD/clean. runtime_approved=false, conditional_on_bound_source_review. Full report205483610615676 committed native77af5dc71b16ef76809b895a2f0aaa69a111b785; final handoff SHA8cf1796c6400e1c423bd117824e62d476bfed2c26d61846a5b981f9b8e745f8a.

```json
{
  "op_id": "01M712A3EQJBJ81Q9BYMVRD3H1",
  "commit": "fe137b8801ef3eb2a07710367422a0da2713f519",
  "report": "[[reports/2026-11-09-dream-proof-tools-input71-rebased-qualification.md]]",
  "verify_result": {
    "command": "python3 -I target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py finish",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Signed rebase and exact builder ownership/currentness proof; no manual upstream synchronization",
        "reference": "N/rebase-identity-proof.json21583B SHA66e0ba90a9843b976b761432078458cac20183b374a061e69451c4dfad35dd4a records42 signed old->new mappings,38 byte-identical dedicated files, exact3 builder registry lines, preserved447+73 corpus/96+9 contracts and coverage scope. Full42 mapping also committed in report. git upstream-tool diff against ad6 exit0; recipe a8187a0ca0b093bdf4e82c2f3e172b664f60a4e5fadc2cb0d50256b8c5d62edd."
      },
      {
        "criterion": "Only permitted wrapper environment delta and all actual remaining final gates pass",
        "reference": "R/logs/062.command+exit+stdout/stderr actual0 env exactly060 plusRUSTC_WRAPPER='';063formal0;064licenses0. Formal evidence verification/evidence/92313b580f77dd3fa1b2fb897210ce6dd28abc45600781acffd407fb7814d87f/evidence.json clean fe137 source520models105contracts1crate; tests26/55/1,formal-tool12,licenses10 pass. Publication/finish driver actual0; full gate refs and environment records in completion."
      },
      {
        "criterion": "Native records reused only after semantic/custody/currentness revalidation; truthful origin bridge and canonical failure history",
        "reference": "R/component-checks eight0 match E/result-evidenceSHA17225997801ea0923fa9c6ee4243db75d254ad5e1f86bc2104760e4abb702e19 and original wait4/raw artifactpins; reruns0. N/read-check-currentness-bridge.json8446B SHA3a046f13d09378919bd1929b5e649a52fe26a5072b20c123839b3a7fc564b6e1 records actualfe inputs, sole commit-origin lookup c50 after exact noncommit equality, unchanged recipe.check0/79tests0; literal newHEADmake not claimed. Original canonical2 unchanged."
      },
      {
        "criterion": "Collision-free append-only completion, qualification/receipt validated without replacement and false runtime approval",
        "reference": "R/final-gates-continuation-01M710HG4BP3TMGXY3BJH571MZ.json19413B SHA44d2565dd55863ff470cbe1d04c7e4c37c3630de43fa9b21013c07fcae586001 all4gates0 binds originalrecords/060061/history/env/mapping/proof/driver. P/qualification.json39083B SHA83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335; P/receipt.candidate.json969B SHA452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c. Aggregate99125B/Basis47490B; constructors/schema/inventory/source/budget/postpublication validators passed. Receipt lastprefixwrite; runtimefalse/conditional/census limitations retained."
      },
      {
        "criterion": "Preservation, per-case telemetry and committed full handoff",
        "reference": "N/final-handoff-evidence.json8964B SHA8cf1796c6400e1c423bd117824e62d476bfed2c26d61846a5b981f9b8e745f8a asserts060/061/unsupportedterminal/priorcontinuation/eightnativeartifacts/33oldledgers unchanged; GoodBoxyfe137,statusclean,diffcheck0. Full report records8 actual exits/check0s,firstterms,debt splits/margins/ordinarymax,50clean shim receipts,escape/pause fields and reap_delay partial-wire/later-wait distinction. Report205483610615676 native checkpoint77af5dc71b16ef76809b895a2f0aaa69a111b785."
      }
    ]
  }
}
```


## ORACLE AMENDMENT (2026-11-09): verify changed to the retained-candidate chain
The original literal verify (fetch/build/test/formal/licenses) would start a fresh build and native qualification, which is forbidden after the qualified run 01M70XWHWKFCD1W6ZZMQB50WG1.
verify is now the builder-proposed chain (message 01M713EMFS6YAED0M5P6VDCPNG). It calls only existing functions:
- prerequisites/revalidate plus bridged_read_check (79 units plus retained-candidate validation against fe137b88);
- assertions that the loader hash 82689cd1… and the qualification 83365f61 / receipt 452390f1 pins hold before and after;
- then make test-formal-parity and make test-licenses with RUSTC_WRAPPER=''.
native_reruns=0. Run it ONCE, then make land ONCE, then task_report. (The frontmatter was edited directly because the paged fetch truncated the body.)


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

BLOCKED landing: exact amended1703-character verify ran ONCE and exited0 at fe137b88 (79units, retained bridge/pins,520 native/TLC models/105contracts, licenses); native qualification reruns0. make land ran ONCE and exited2: git diff --check origin/main...HEAD rejects valid.commit and corrupt.commit lines6/13, each exact space+LF required as Git continuation markers for blank armored signature lines. No stripping, gate bypass, retry or push; landed_sha=null.
Canonical land fetched/rebased onto main d64b817091189f106c8bf245408a915683f754c4; current clean signed HEAD e90728a1b095e136db9745519333fc44810cbcb9, all42 signatures0. Complete old->new mapping and38 identical builder tree entries recorded; prelandfe preserved at archive/dream-proof-tools-verified-fe137-20261109. q83365f61/receipt452390f1, original8native artifacts,33oldledgers and060/061/terminal/history all unchanged. No post-rebase formal/landing gates claimed; runtime_approved=false.
Decision needed: authorize narrowly path-scoped whitespace policy for exactly the two frozen fixtures without changing bytes (.gitattributes outside current allowed_paths), plus e907 currentness/verification binding and any new landing allowance. Full report205483610615679 committed native1df858af8ed26288b79e86c2d34e6d8cb1422f5c; H/hand off SHA08dc3b2a6d46ff464697826475d39665e4935b9c22f28a6dc678622a1ed144ff.

```json
{
  "op_id": "01M71508QBSMPST4Q5VH15S3B2",
  "commit": "e90728a1b095e136db9745519333fc44810cbcb9",
  "report": "[[reports/2026-11-09-dream-proof-tools-input71-landing-whitespace-blocked.md]]",
  "verify_result": {
    "command": "RUSTC_WRAPPER='' python3 -I -c 'import hashlib,importlib.util,json,tempfile;from pathlib import Path;p=Path(\"target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py\").resolve();assert hashlib.sha256(p.read_bytes()).hexdigest()==\"82689cd1f2d06673a32411279b1b21a3b0b618fdbfd6351a9427b2290240fd0d\";s=importlib.util.spec_from_file_location(\"retained_verify\",p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);plan,gate,old,env=m.prerequisites();published=m.c.load(m.HERE/\"qualification-published.json\");assert published[\"locator\"][\"evidence_sha256\"]==\"83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335\" and published[\"locator\"][\"receipt_sha256\"]==\"452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c\";assert m.c.pin(m.c.PREFIX/\"qualification.json\")==published[\"qualification\"] and m.c.pin(m.c.PREFIX/\"receipt.candidate.json\")==published[\"receipt\"];intent,basis,selected,records,hashes=m.c.revalidate();assert hashes==plan[\"native_records\"];m.HERE=Path(tempfile.mkdtemp(prefix=\"retained-verify-\",dir=p.parent));status,result=m.bridged_read_check(plan);assert m.c.revalidate()[4]==hashes;assert m.c.pin(m.c.PREFIX/\"qualification.json\")==published[\"qualification\"] and m.c.pin(m.c.PREFIX/\"receipt.candidate.json\")==published[\"receipt\"];m.c.save(m.HERE/\"verification.json\",{\"exit_code\":status,\"read_check\":result,\"rebase_proof\":plan[\"rebase_proof\"],\"locator\":published[\"locator\"],\"native_reruns\":0,\"runtime_approved\":False});print(json.dumps({\"exit_code\":status,\"evidence\":str(m.HERE/\"verification.json\"),\"native_reruns\":0}));raise SystemExit(status)' && RUSTC_WRAPPER='' make test-formal-parity && RUSTC_WRAPPER='' make test-licenses",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Exact amended task verify executed once with real exit0",
        "reference": "/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/retained-verify-and-land-20261109-3vbwvoeg/verify.intent.json binds exact1703-character command SHA06ef21968405f6a617fb4d775313add4c6f4ec101c2622b20a2e54ff8295d745, task etagb86140c6, fe137head and >8GiB preflight; verify.exit.json actual0. Fresh retained-verify-kalzfjzz/verification.json SHA c4025dd3f24bb633d716d40a1fcb2603ae3c68936574f3992f00f855d70a774d;79testsOK; preserved qualification/receipt and8native;520 full parity/105contracts/1crate,26+55+1 Rust/12tool/10license tests all0."
      },
      {
        "criterion": "Sole canonical landing refused honestly before push; no retry or fixture mutation",
        "reference": "/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/retained-verify-and-land-20261109-3vbwvoeg/land.exit.json actual2; land.stdout four trailing-whitespace diagnostics valid.commit/corrupt.commit:6,:13; land.stderr script77/Make2; target/harness-land-receipt.json copy H/harness-land-receipt.json final git diff --check row2, no git push row; downstream landing gates unrun. Required space+LF bytes hex200a preserved; source parser1934–1945 treats stripped line as header/body separator."
      },
      {
        "criterion": "Current rebase provenance and distinction from successfully verified head",
        "reference": "Canonical rebase0 onto d64b817091189f106c8bf245408a915683f754c4; currentHEAD e90728a1b095e136db9745519333fc44810cbcb9,42 signature rows0; H/handoff.json complete42fe->new mapping. H/post-rebase-identity.json proves38 complete builder tree entries/modes/membership identical tofe/c50, treecomparisonSHA bafd23dca77f924bf372658e39df56c5d5f003d24b13c07fb798f6c7b6b738e1. Prefe archive ref preserved. Formal evidence92313b58 bindsfe; e907 formal/landing not completed, landed_sha=null."
      },
      {
        "criterion": "Conditional qualified artifacts and append-only historical evidence preserved",
        "reference": "H/handoff.json17317B SHA08dc3b2a6d46ff464697826475d39665e4935b9c22f28a6dc678622a1ed144ff; q39083B SHA83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335 and receipt969B SHA452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c bytes/dev/ino unchanged;8nativeartifacts+33oldledgers+060061terminal/priorcompletion/proofpins unchanged. No native qualification rerun, runtimeapproval, cleanup, reset/stash or manualsourcefix. Full committed report1df858af8ed26288b79e86c2d34e6d8cb1422f5c records exactscope decision required."
      }
    ]
  }
}
```

## ORACLE RULING (2026-11-09): whitespace exemption for the two raw commit fixtures; land e90728a1
The amended verify ran ONCE at fe137b88 and exited 0 (79 units, retained bridge/pins, 520 native/TLC models, 105 contracts, licenses; native reruns 0). make land then exited 2 on `git diff --check`: tests/fixtures/dream-proof-tools/corpus/valid.commit and corrupt.commit lines 6 and 13 end in space+LF. In a raw git commit object those bytes are REQUIRED: they are the continuation marker for blank lines inside the armored gpgsig header. Stripping them would corrupt the fixtures.
Authorized:
1. Create .gitattributes (now in allowed_paths) containing EXACTLY these two lines, and nothing else:
   tests/fixtures/dream-proof-tools/corpus/valid.commit -whitespace
   tests/fixtures/dream-proof-tools/corpus/corrupt.commit -whitespace
   Fixture bytes stay unchanged (verify their sha256 before and after). No other path is exempted.
2. Currentness binding: e90728a1 is fe137b88 rebased onto harness main d64b8170, and the builder-owned files are byte-identical (Oracle checked: `git diff --stat fe137b88 e90728a1` over the builder paths is empty; 42 commits signed). The verify exit 0 at fe137b88 therefore stands for e90728a1 plus the .gitattributes commit; no verify rerun is needed.
3. Run make land ONCE more (with RUSTC_WRAPPER emptied if needed). If it is red for any other reason, report it; no loop.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

Exact .gitattributes implementation DONE in signed Boxy9674e87c214006dbe319efbe399b515f49e97785 (parent e907); only the two authorized lines,132B SHA4355695061f09cd9f23c77fbfe27d67f224d685e36912c37f46d29ba8104c481. Both707B fixture SHA256s identical before/after; check-attr0 confirms only their whitespace unset, script/README unspecified; treeclean.
BLOCKED before additional make land: disk preflight actual1, free8338808832B <= required strict8589934592B, shortfall251125760B. Signature/currenthead/parent/status/priority checks passed. No new make land start/PID record; allowance UNCONSUMED. No verify/native rerun, cleanup, source normalization, merge or push. Prior verify0 stands under Oracle currentness ruling; runtime_approved=false.
Need shared space with margin above8GiB or an exact disposable-cache disposition. Full report205483610615682 committed native1fe026ae1eac7987f8d02d6411384959c185556b; evidence target/dream-proof-tools/attributes-land-20261109-ugyrwtdp/land-preflight.json SHA251cf523a99813e79eea00484f0bd24c50a100e4cbd638daf7ff64b62916cf97. Prior failed landing receipt preserved; no landed SHA.

```json
{
  "op_id": "01M715APPJEMFMZM6WCPMBF0AB",
  "commit": "9674e87c214006dbe319efbe399b515f49e97785",
  "report": "[[reports/2026-11-09-dream-proof-tools-input71-attributes-landing.md]]",
  "verify_result": {
    "command": "RUSTC_WRAPPER='' python3 -I -c 'import hashlib,importlib.util,json,tempfile;from pathlib import Path;p=Path(\"target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py\").resolve();assert hashlib.sha256(p.read_bytes()).hexdigest()==\"82689cd1f2d06673a32411279b1b21a3b0b618fdbfd6351a9427b2290240fd0d\";s=importlib.util.spec_from_file_location(\"retained_verify\",p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);plan,gate,old,env=m.prerequisites();published=m.c.load(m.HERE/\"qualification-published.json\");assert published[\"locator\"][\"evidence_sha256\"]==\"83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335\" and published[\"locator\"][\"receipt_sha256\"]==\"452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c\";assert m.c.pin(m.c.PREFIX/\"qualification.json\")==published[\"qualification\"] and m.c.pin(m.c.PREFIX/\"receipt.candidate.json\")==published[\"receipt\"];intent,basis,selected,records,hashes=m.c.revalidate();assert hashes==plan[\"native_records\"];m.HERE=Path(tempfile.mkdtemp(prefix=\"retained-verify-\",dir=p.parent));status,result=m.bridged_read_check(plan);assert m.c.revalidate()[4]==hashes;assert m.c.pin(m.c.PREFIX/\"qualification.json\")==published[\"qualification\"] and m.c.pin(m.c.PREFIX/\"receipt.candidate.json\")==published[\"receipt\"];m.c.save(m.HERE/\"verification.json\",{\"exit_code\":status,\"read_check\":result,\"rebase_proof\":plan[\"rebase_proof\"],\"locator\":published[\"locator\"],\"native_reruns\":0,\"runtime_approved\":False});print(json.dumps({\"exit_code\":status,\"evidence\":str(m.HERE/\"verification.json\"),\"native_reruns\":0}));raise SystemExit(status)' && RUSTC_WRAPPER='' make test-formal-parity && RUSTC_WRAPPER='' make test-licenses",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Exact authorized two-path attributes, signed commit and unchanged presigned fixtures",
        "reference": ".gitattributes132B SHA4355695061f09cd9f23c77fbfe27d67f224d685e36912c37f46d29ba8104c481; signed9674e87c214006dbe319efbe399b515f49e97785 changesonlythatfile. H2/before.json SHA0cfee9778f70ffcc079e200d93f587747503759c031748724a29735dd49781a0 and after.json SHAb6c4d7f4b24078c0ca488a281d12fb44d4cafca9280c2c05620e391e5edf624d retain validf0b48d87/corruptb5995d22 exact707B fixture hashes; check-attr actual0, otherpaths unspecified."
      },
      {
        "criterion": "Additional landing remains unused because its standing disk precondition failed",
        "reference": "Read-only Pythonpreflight execbced0c actual1: free8338808832B not>8589934592B; strictgate shortfall251125760B. Signature0/expectedparent e907/current9674/clean/priority0 passed. H2/land-preflight.json639B SHA251cf523a99813e79eea00484f0bd24c50a100e4cbd638daf7ff64b62916cf97; no additional land invocation record, source/compile/native/verify rerun or cleanup."
      },
      {
        "criterion": "Prior verification retained exactly under explicit Oracle currentness ruling, without claiming a rerun",
        "reference": "Task etaga2680aba sectionwhitespaceexemption says fe137verify0 standsfore907+exactattributescommit; frontmatterverifycommand retainedverbatim1703chars SHA06ef21968405f6a617fb4d775313add4c6f4ec101c2622b20a2e54ff8295d745. Original H1/verify.intent+exit0 and79unit/retainedcheck/520TLC/105contracts/licenses evidence unchanged. Nofreshverifyat9674 claimed."
      },
      {
        "criterion": "Historical receipt and full blocker handoff retained with false runtime approval",
        "reference": "H2/predecessor-land-receipt.json10293B SHAa741881a90ebcbdd65c11c632f395c617a26064f0dd243082f3ca073c1df9c59 preservespreviousland2/nopush. q83365f61/receipt452390f1 remainoriginalconditionalartifacts; runtimefalse. Native report205483610615682 checkpoint1fe026ae1eac7987f8d02d6411384959c185556b includes exactrequiredspace/cache decision; no landedSHA/new attempt."
      }
    ]
  }
}
```


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

BLOCKED after the ONE additional make land invocation: actual Make2/script77, cargo all-targets Clippy101 at tests/acp_agent.rs:1189–1192, .last() on DoubleEndedIterator (help .next_back()). That file is byte-identical to origin/main and outside allowed_paths; last modified by d64b8170. The attributes fix worked: git diff --check0, licenses0 and embedded-lints0. No push/landedSHA or retry; additional allowance now consumed.
Canonical land rebased43 signed commits onto main759f96fcc7a5feada2df53b86a7f78006785afd2; current clean HEAD31ff83c585dd1def7f68fbc38db5d92c12dc722e. All43 signatures0; builder38 tree entries identical tofe/c50; exact132B attrs and frozen fixture hashes unchanged. Prior verify0 retained under ruling; no verify/native qualification rerun. q83365f61/receipt452390f1,8native artifacts,33oldledgers and060061terminal/history preserved; runtime_approved=false.
Need upstream ACP fix or explicit path/scope plus further currentness/landing disposition. Separately the5 external INPUT71 authority paths are now absent after Q2target cleanup; P retains matching C/probe/preparation/shim bytes, so this is availability, not the Clippy cause or a new gate. Full report205483610615686 committed native8f6dc5dc883d98588ed28bf3d7a2acc166d3eee8. H/landing-result.json13745B SHA3bcd0656ff7208cdfb550a5e39ddf4a04e44d0ebb928c64feb50085cf8a2dcb0; availability2137B SHA9aa7b18cf602ed8ea002fa04972f88efa78208c13c9b5a37ec24342ddb2204c9.

```json
{
  "op_id": "01M7160WA6S2BP25FV4Z80TNEK",
  "commit": "31ff83c585dd1def7f68fbc38db5d92c12dc722e",
  "report": "[[reports/2026-11-09-dream-proof-tools-input71-attributes-land-clippy-blocked.md]]",
  "verify_result": {
    "command": "RUSTC_WRAPPER='' python3 -I -c 'import hashlib,importlib.util,json,tempfile;from pathlib import Path;p=Path(\"target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py\").resolve();assert hashlib.sha256(p.read_bytes()).hexdigest()==\"82689cd1f2d06673a32411279b1b21a3b0b618fdbfd6351a9427b2290240fd0d\";s=importlib.util.spec_from_file_location(\"retained_verify\",p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);plan,gate,old,env=m.prerequisites();published=m.c.load(m.HERE/\"qualification-published.json\");assert published[\"locator\"][\"evidence_sha256\"]==\"83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335\" and published[\"locator\"][\"receipt_sha256\"]==\"452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c\";assert m.c.pin(m.c.PREFIX/\"qualification.json\")==published[\"qualification\"] and m.c.pin(m.c.PREFIX/\"receipt.candidate.json\")==published[\"receipt\"];intent,basis,selected,records,hashes=m.c.revalidate();assert hashes==plan[\"native_records\"];m.HERE=Path(tempfile.mkdtemp(prefix=\"retained-verify-\",dir=p.parent));status,result=m.bridged_read_check(plan);assert m.c.revalidate()[4]==hashes;assert m.c.pin(m.c.PREFIX/\"qualification.json\")==published[\"qualification\"] and m.c.pin(m.c.PREFIX/\"receipt.candidate.json\")==published[\"receipt\"];m.c.save(m.HERE/\"verification.json\",{\"exit_code\":status,\"read_check\":result,\"rebase_proof\":plan[\"rebase_proof\"],\"locator\":published[\"locator\"],\"native_reruns\":0,\"runtime_approved\":False});print(json.dumps({\"exit_code\":status,\"evidence\":str(m.HERE/\"verification.json\"),\"native_reruns\":0}));raise SystemExit(status)' && RUSTC_WRAPPER='' make test-formal-parity && RUSTC_WRAPPER='' make test-licenses",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Newly authorized canonical landing executed once after successful disk/current-head preflight",
        "reference": "/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/attributes-land-20261109-ugyrwtdp/land-resumed-preflight.json records>8GiB/priority0/clean9674/unchangedfixture+exactattrs; land.intent recordsRUSTC_WRAPPER='' make land; land.exit actual2. No new verify/native qualification run. Previousfailedpreflight and predecessorlandreceipt preservedseparately."
      },
      {
        "criterion": "Exact two-file whitespace policy works without changing signed fixture bytes",
        "reference": ".gitattributes132B SHA4355695061f09cd9f23c77fbfe27d67f224d685e36912c37f46d29ba8104c481 unchanged; validf0b48d87/corruptb5995d22 fixtureSHA retained. H/land-receipt.json git diff --check0/test-licenses0/check-embedded-lints0; no blanket suppression."
      },
      {
        "criterion": "Actual out-of-scope upstream Clippy failure, no push or retry",
        "reference": "H/land.stderr tests/acp_agent.rs1189:22–1192 clippy::double_ended_iterator_last: events.iter().filter(...).last(), helpnext_back; alltargetscommandcargo101; script77/Make2. git diff --exit-code origin/main HEAD --tests/acp_agent.rs0 and lastcommitd64b8170. Canonicalreceiptno gitpush/no landedfield; downstreamgatesunrun. No source edit toACPfile or warning suppression."
      },
      {
        "criterion": "Signed rebase mapping, retained native/candidate/history and truthful prior verification",
        "reference": "H/landing-result.json13745B SHA3bcd0656ff7208cdfb550a5e39ddf4a04e44d0ebb928c64feb50085cf8a2dcb0 records43 old967-era->current mappings/signature0, head31ff83c585dd1def7f68fbc38db5d92c12dc722e onto759f96fc/clean,38buildertreeidentical, q83365/receipt452390 samebytes/devino,8native+33historical+060061terminal/priorproof allpins preserved. Priorfeverify0 carriedbyruling, no new31ffformalresult claimed; runtimefalse; old967archiveref preserved."
      },
      {
        "criterion": "External authority availability accurately distinguished from retained artifact bytes",
        "reference": "H/external-authority-availability.json2137B SHA9aa7b18cf602ed8ea002fa04972f88efa78208c13c9b5a37ec24342ddb2204c9: original5Q2targetpathsabsent; P C449970B077d63/probe303256B9a9ee/preparation501669Bab2227/shim35520B4e95 hashesmatch; sourcead6. No restoration/substitution/currentexternalvalidation/newfreeze inferred. Report205483610615686 native8f6dc5dc883d98588ed28bf3d7a2acc166d3eee8 committed fullsource/log/hash/evidence/blocker."
      }
    ]
  }
}
```

## ORACLE RULING (2026-11-09): upstream clippy break, one more land
make land (#2) on 31ff83c5 went red only on all-targets clippy at tests/acp_agent.rs:1189-1192 (`.last()` on a DoubleEndedIterator). That file is byte-identical to harness origin/main (broken upstream by d64b8170), and it lies outside this task. The Oracle fixes it upstream on harness main (`.last()` → `.next_back()`, a one-line change on its own signed commit).
Once that fix is on origin/main:
1. Rebase onto it; builder-owned files must stay byte-identical (same check as before).
2. Run make land ONCE more. Currentness binding: the verify exit 0 still stands under the earlier ruling. No verify or native rerun.
The availability note is accepted. The Oracle deleted the Q2 target/ during disk cleanup, which removed the 5 external INPUT71 authority paths. P retains matching bytes. This changes no gate.


## Report from seat:01M6W6K5SWZB5AEBAAAA73FF4M

LANDED harness main7b8ed04fe222c27970901811323592d7a23c73ae. The ONE authorized make land invocation exited0. Its canonical internals made two source-bound rounds: firstcd44 allgates0 then push1(fetchfirst); automaticrebase onto5f83d8f5 and freshgates/formal then finalpush0. No manual re-invocation. New remote fetch0/ancestry0 observed origin/main=HEAD=landed7b8; worktreeclean; whole sourceSHA c93be1ffff3bd71893c8405422ec6f7fd60e03f8786e6c74116338e734633f72 matcheslandingreceipt.
Fetched/signed-verified upstreamb46fad65 actual.rfind fix; all39 owned entries (38builder+exactattrs) byte/mode/member-identical to31ff and builder38 toc50. Final43signaturechecks0 (86overbothrounds), no ACPsourceedit. Final required licenses/3Clippy lanes/concurrent107tests/serializedfamilies/changedintegration0; formal45+74+1Rust/12tooltests and520nativeTLCmodels105contracts1crate passed. Currentclean7b8 formal evidenceSHA d1da261e6e2c2894d5e3d29a14f096c1f490a56692645b63479889317648d959; optionalstale-evidence2s and firstpush1 preserved, finalrequiredpathall0.
Exact amended verify priorreal0 retained under explicitcurrentness ruling; no verify/private native qualification rerun. q83365f61/receipt452390f1/all157refs,8native records40originals,33oldledgers53preservationrefs unchanged; runtime_approved=false/conditional. External5INPUT71paths absent but Pbytesretained, acceptedruling/no gate. Full finalreport205483610615690 committed native4dd560073f0554de487cc4b381715d817f1d97b0; H/handoff31208B SHAf44118a4340817e8cf5444a2b77a80bf355cad822a8d51554fb48547d4fa4158. Scoped work complete; Root acceptance/runtime admission remain separate.

```json
{
  "op_id": "01M718VPCDVVPETSB49MGAXK21",
  "commit": "7b8ed04fe222c27970901811323592d7a23c73ae",
  "report": "[[reports/2026-11-09-dream-proof-tools-input71-landed.md]]",
  "verify_result": {
    "command": "RUSTC_WRAPPER='' python3 -I -c 'import hashlib,importlib.util,json,tempfile;from pathlib import Path;p=Path(\"target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py\").resolve();assert hashlib.sha256(p.read_bytes()).hexdigest()==\"82689cd1f2d06673a32411279b1b21a3b0b618fdbfd6351a9427b2290240fd0d\";s=importlib.util.spec_from_file_location(\"retained_verify\",p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);plan,gate,old,env=m.prerequisites();published=m.c.load(m.HERE/\"qualification-published.json\");assert published[\"locator\"][\"evidence_sha256\"]==\"83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335\" and published[\"locator\"][\"receipt_sha256\"]==\"452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c\";assert m.c.pin(m.c.PREFIX/\"qualification.json\")==published[\"qualification\"] and m.c.pin(m.c.PREFIX/\"receipt.candidate.json\")==published[\"receipt\"];intent,basis,selected,records,hashes=m.c.revalidate();assert hashes==plan[\"native_records\"];m.HERE=Path(tempfile.mkdtemp(prefix=\"retained-verify-\",dir=p.parent));status,result=m.bridged_read_check(plan);assert m.c.revalidate()[4]==hashes;assert m.c.pin(m.c.PREFIX/\"qualification.json\")==published[\"qualification\"] and m.c.pin(m.c.PREFIX/\"receipt.candidate.json\")==published[\"receipt\"];m.c.save(m.HERE/\"verification.json\",{\"exit_code\":status,\"read_check\":result,\"rebase_proof\":plan[\"rebase_proof\"],\"locator\":published[\"locator\"],\"native_reruns\":0,\"runtime_approved\":False});print(json.dumps({\"exit_code\":status,\"evidence\":str(m.HERE/\"verification.json\"),\"native_reruns\":0}));raise SystemExit(status)' && RUSTC_WRAPPER='' make test-formal-parity && RUSTC_WRAPPER='' make test-licenses",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Requested upstream fix, signed rebase and exact builder ownership preserved",
        "reference": "Signedb46fad659627de01e704a087082d10380e120193 changesACPtest to.rfind; explicit43-commitrebase0, H3/rebase-identity.jsonSHA798887542476a1fca2a319b74b2f11a98d7679104647569acb208a5e8d387a11 proves39ownedtreeentries identical to31ff; final39stillmatch and38toc50, recipea818/attrs4355/validf0b4/corruptb599 unchanged. Bothcanonical43signaturecheckssets0; archived31ff/cd44 andearlierrefs preserved."
      },
      {
        "criterion": "ONE canonical landing completed with required gates and final push0",
        "reference": "/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/upstream-clippy-land-20261109-h529z8uh/land.intent.json commandRUSTC_WRAPPER='' make land; land.exit actual0; final target/harness-land-receipt.json copiedH3/land-receipt.json landed7b8. H3/handoff.json31208B SHAf44118a4340817e8cf5444a2b77a80bf355cad822a8d51554fb48547d4fa4158 binds136commandrows/twointernalrounds, allfinalrequiredgates0; staleoptionalchecks2 andfirstFFpush1 handledbyexistingcanonicalflow; finalpush0. No manualretry."
      },
      {
        "criterion": "Current landed-source unit/integration/lint/formal evidence rather than stale-head claims",
        "reference": "Finalrequiredlicenses/embedded+alltargets+testfixturesClippy0; concurrentnextest107passed103skippedconfiguredfilter; tenserializedfamilies0 withignoredcountsrecorded; changedDProofintegration1pass/recipe79units. FormalRust45/74/1 plus12tooltests0;520fullnativeTLCgraphs/expectedverdicts,105contracts1crate; verification/evidence/d1da261e6e2c2894d5e3d29a14f096c1f490a56692645b63479889317648d959/evidence.json3697485B hascleanlanded7b8/560currentinputhashes, allgraph_equaltrue; finalcheck-formal-evidence0. Boundedprooflimitsretained."
      },
      {
        "criterion": "Actual remote-main containment, clean source and immutable final code identity",
        "reference": "H3/remote-main-verification.json freshfetch0, observedorigin/main7b8, landedancestorcheck0, HEAD7b8/statusclean. H3/post-land-source.json wholetrackedsourceSHA c93be1ffff3bd71893c8405422ec6f7fd60e03f8786e6c74116338e734633f72 matchescanonicalreceipt. Full43old->landedmappinginH3handoff+committedreport; no release/install/runtimeadmission."
      },
      {
        "criterion": "Exact task verification result carried under ruling, native/proposal/history unaltered and full report committed",
        "reference": "Exactfrontmatter1703charcommand SHA06ef21968405f6a617fb4d775313add4c6f4ec101c2622b20a2e54ff8295d745 actual0retainedfromfe137 perOraclecurrentnessrulings; notrerun. q83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335/receipt452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c/all157refs/8records40originals/33ledgers53preservationrefs unchanged. Runtimefalse/conditional/noarbitrarynativeproof; acceptedexternalavailabilityconditionunchanged. Fullreport205483610615690 committed4dd560073f0554de487cc4b381715d817f1d97b0; nativequalificationreruns0."
      }
    ]
  }
}
```
