---
type: interface
title: Private proof-tool builder to Q2 qualification and approval boundary
derived_from:
- '[[decisions/d93377961365654-dream-q2-tool-lifetime-basis]]'
- '[[tasks/2026-11-05-dream-proof-tools-ready-brief]]'
links:
- to: '[[interfaces/dreaming-mcp]]'
  rel: refines
id: 01M6PZ9KVVDT0HJEB361TJQC9H
updated: 2026-11-06T16:59:33.363Z
created: 2026-11-05T23:04:25.428Z
number: 93377961365730
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:22c21d2dfa5efba67a567f87546ae60de37bcbc31cce0811850c19dd0b3539e4
  frontmatter_digest: sha256:0c2df15de2e3d0b28245831ccf9c56f84e696b3db2aed9a4e61df90f218436bb
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: XS4F5qc6Af5mnO+WsVcaQMuXTIfe+UJDJhIgY5twg0Y/hrTuWicwgk45LxZL+o4nvqWOmID7oW1QWmvWUIDlBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149620
  event:
    id: 01M6RWT8QX7VCPZ8VXP2BCFY60
    sequence: 149620
    hash: 32f3345121b1e22c298786f5bd98f6754ea10ae81d2e18b0c8c72cd894a2d81f
---
# Proof-tool builder → Q2 qualification handoff

Implements [[decisions/d93377961365654-dream-q2-tool-lifetime-basis]]; inherits65126/65489 and [[interfaces/dreaming-mcp]]. Task [[tasks/2026-11-05-dream-proof-tools-ready-brief]]. This is a build/qualification ICD, not runtime approval or a new scheduler. Installed tools remain Unproven.

| Owner | Produces / consumes |
|---|---|
| Provisioning Executor01M6Q0FHRZV6MQA8PBPXG408N7 | Private real Git/GPG candidate, source/build/closure review and component witness evidence. Cannot mint runtime trusted approval. |
| Root Oracle | Independently reviews exact source/build/process/formal evidence, admits the finite premise through the existing approved provisioning source/artifact authority, or refuses. No operator prompt. |
| Q2 author01M6PBF0EX2R5RGQAXNQTR96Y0 | Owns existing helper/probe/controller and strict proof-tools/1 DTO; validates prepared host-approved binding BEFORE helper birth. Builder does not fork that product implementation. |

Builder writing tree /Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools; current [[tasks/2026-11-05-dream-proof-tools-provisioner]] controls its baseline/scope. Read-only snapshot2fdb65482cb7faa842765e563428d28b2be91fc7; only Root amends that live task. Q2 source/probe stays its owner's responsibility; this ICD grants no cross-tree writes.

## Frozen inputs and authentication

Expected release pins below were fetched from primary release/build metadata2026-11-05. Archives/signatures were NOT cryptographically verified or built in this design task. Executor must authenticate before extraction; a published checksum is not a signature verdict.

| Source | Exact archive URL / expected SHA256 |
|---|---|
| Git2.54.0 | https://www.kernel.org/pub/software/scm/git/git-2.54.0.tar.xz — f689162364c10de79ef89aa8dbf48731eb057e34edbbd20aca510ce0154681a3 |
| GnuPG2.5.24 | https://gnupg.org/ftp/gcrypt/gnupg/gnupg-2.5.24.tar.bz2 — bf149d01a2b9fcc0e4589b8ae8697d3d5c557ea48ed95a3fa55dd3b1187e6039 |
| Libgpg-error1.61 | https://gnupg.org/ftp/gcrypt/libgpg-error/libgpg-error-1.61.tar.bz2 — 7a85413f2bc354f4f8aa832b718af122e48965e9e0eb9012ee659c13c6385c93 |
| Libgcrypt1.12.4 | https://gnupg.org/ftp/gcrypt/libgcrypt/libgcrypt-1.12.4.tar.bz2 — d77f68f48879510e79a2f65977ccc68981781ea0923e5bdffac2a193ea3d660e |
| Libassuan3.0.2 | https://gnupg.org/ftp/gcrypt/libassuan/libassuan-3.0.2.tar.bz2 — d2931cdad266e633510f9970e1a2f346055e351bb19f9b78912475b8074c36f6 |
| nPth1.8 | https://gnupg.org/ftp/gcrypt/npth/npth-1.8.tar.bz2 — 8bd24b4f23a3065d6e5b26e98aba9ce783ea4fd781069c1b35d149694e90ca3e |
| Libksba1.8.1 (source7) | https://gnupg.org/ftp/gcrypt/libksba/libksba-1.8.1.tar.bz2 — c2f84393011827219ae117131dba8e7684c2bed0961eed11b0642c2acba440b5 |

Authentication roots/files:

- Git detached signature https://www.kernel.org/pub/software/scm/git/git-2.54.0.tar.sign, fetched566bytes/SHA2561083f5c8eafe129da815da3f78cbc5865bdaec29be0d3969c104d3b003801775. Verify against the **uncompressed tar stream**, not xz bytes. Expected signing subkey E1F036B1FEE7221FC778ECEFB0B5E88696AFE6CB, primary96E07AF25771955980DAD10020D04E5A713660A7. Primary maintainer public bytes https://github.com/gitster.gpg, SHA2562a3477639ac9bff6c814a679983408c70e5320402a38d41a74b2df0b206e9bfc; packet fingerprints/subkey presence were inspected, signatures were not verified. Identity link: [upstream maintain-git](https://raw.githubusercontent.com/git/git/v2.54.0/Documentation/howto/maintain-git.adoc) names Junio/gitster. TLS/public-maintainer binding is an explicit bootstrap trust input; no UID-only/keyserver TOFU.
- GnuPG/library signatures are archive URL + .sig. Public keys https://gnupg.org/signature_key.asc, SHA2566f57d0e17fefd2238bd037aebf03efa960641629dd61dd9d47f122a829f6e375; [official fingerprints](https://gnupg.org/signature_key.html): Werner6DAA6E64A76D2840571B4902528897B826403ADA / NiibeAC8E115BF73E2D8D47FA9908E98E9B2D19C6C8BD. Each fetched signature contains issuer metadata for both; require a valid release signature under an expected bound primary, with subkey binding/revocation/time checked, never accept issuer metadata alone. GPG.sig SHA2566cc9dad0a8ad9a27e842c06daf696c94db5c83237933687e773c02b04857fb1a; error.sig9b20f1e6046aaa3fe16fcd65587fd0fa5e34ceb4830f8eadc856c3e80ad57ad3; gcrypt.sig8be53f19f43ffa0297aceb520be717de539d1bdad45b5538a57fd95610b1e2db; assuan.sig5aa3c5cea6f42bcb96be9e1dd922b33ccb05e2f0565e93674e85ddcf8cd78e86; npth.sigeb9314cf3867a438cb0fdd371125d44c57ba66086f152199ad5f40a9c9679acb. Libksba signature URL is its archive URL + .sig, SHA256af99f325f81eb395d740bf48dc8e89903b141c10f69244e46e4caa1caec0577e; same Werner/Niibe primary and direct-signing fingerprints/rules, no new root.
- Retain fetched key/signature bytes in source fixtures; pin all hashes in sources.lock.json. Never import into operator keyrings or auto-refresh a changed root. Existing independently trusted verifier is a developer bootstrap input, not the newly built candidate: the observed installed GPG SHA21cb9adcad96b7be404014ff6e9fb837160a2135455c217dec9efe196cf0661a may be scoped by Root for public-only source verification; record that bootstrap assumption. A different/unavailable verifier or key mismatch refuses authentication; no self-verification by the downloaded candidate.
- Git checksum from [kernel signed checksum list](https://www.kernel.org/pub/software/scm/git/sha256sums.asc); dependencies from exact upstream archive URLs and [Homebrew source metadata](https://formulae.brew.sh/api/formula/libgcrypt.json) (also /libgpg-error.json,/libassuan.json,/npth.json). Hash equality plus actual upstream signature verification is required. [Kernel guidance](https://www.kernel.org/signature.html) distinguishes autosigned mirror checksums from developer signatures; [GnuPG integrity guidance](https://gnupg.org/download/integrity_check.html) forbids trusting a newly downloaded verifier to establish its own source.

Build dependency order: error → gcrypt/assuan/Libksba; nPth independent; all five private dependency packages precede GnuPG. Source-lock order preserves the original six entries and appends Libksba as7; build order is dependency-driven. [[decisions/d93377961365875-dream-ksba-private-probe-boundary]] records the actual unconditional KSBA configure failure and common ksba.h use. Use authenticated private Libksba1.8.1/API1 with existing private libgpg-error1.61 (upstream minimum1.28); GnuPG m4/ksba.m4 supports preferred --with-libksba-prefix=<private-deps>. Do not bypass/patch out the check. Actual generated configure/link closure determines whether KSBA is staged runtime or build-only; loaded/static-linked code/initializers/lifetime assumptions remain reviewed. Original six cache entries may be reused only after all checks; changed lock/source/patch/profile invalidates old candidate approval. Git's SDK/system dependencies and no ambient Homebrew/substitution law remain. No extra agent executable enters prefix.

Minimum upstream backports for this **public-only** subset: none selected. The fetched42386bcf8861f04c1bdec5cbb139a03da83d8ef9 /5523cec6913bcbd1a374d183899a6f298d2c0e32 Homebrew patches change unprotected secret-key conversion, which65126 forbids; do not use this as permission to accept secret packets. Their optional adoption needs explicit patch hashes/review, not an unrecorded host-formula update. Profile guards below are separate credited project patches.

Source-lock schema (builder-owned, ≤32KiB, copied unchanged into the128KiB evidence allocation):
~~~text
{format:"bboxy.dream.proof-tool-sources/1",
 sources:[{name:String,version:String,archive_url:String,archive_sha256:Hex,
   signature_url:String,signature_sha256:Hex,compression:"xz"|"bz2",
   signed_bytes:"uncompressed_tar"|"archive",roots:[Fingerprint40],
   signing_subkeys:[Fingerprint40],public_key:FileRef}],
 backports:[], profile_patches:[FileRef],
 bootstrap_verifier:{absolute_path:String,sha256:Hex},
 canonical_q2:{source:FileRef,sha256:Hex}}
~~~
Exactly seven sources in table order; download ceiling32MiB each. Git signed_bytes=uncompressed_tar; GnuPG/libraries=archive. Relative public key/patch/source refs are retained under candidate provenance; no mutable keyserver references. Root fills canonical_q2 with its chosen frozen probe before the component witness; builder cannot discover/approve a replacement. Missing that witness input does not prevent authenticated fetch/build/guard work. Profile-patch hashes are real implementation outputs pinned before candidate build/reuse, not fabricated values in this design. Fingerprint40 is40uppercase hex; roots/subkeys are nonempty finite lists matching the roots above. All URL/path strings≤4096bytes, names/version≤128bytes; no unknown fields.

## Exact build profile and review obligations

No guessed configure flags. Pin release recipes only after checking their --help/source and generated config; treat unrecognized-option warnings or unexpected linked images as unsupported.

| Component | Finite review / permissible narrow fail-closed guard |
|---|---|
| Git | Cover ALL65126 root/config/object/path/ancestry/last-touch/signature forms, their setup/object-decompression/alternate/config/error/cleanup paths. Map actual reachable spawn/thread sites and constructors. run-command.c start_async must not create a thread **or fork** for this profile; start_command permits only the one absolute approved GPG verify child from gpg-interface.c's actual signature path, count consumed before effect/no refund. Reject generic command, pager/hook/fsmonitor/fetch, shell fallback/ENOEXEC and extra async branches. Keep actual Git object/signature/status semantics; no manufactured verification output. NO_PTHREADS is not a proof. |
| GPG/dependencies | Preserve --show-keys, public --import and exact implicit --verify/status-fd behavior. Public packet validation rejects secret/sign/decrypt/card operations before any service work. Review gpg command dispatch, call-agent/keyboxd/dirmngr, exec helpers, nPth/thread creation, libgcrypt/error/assuan initializers and all reachable error paths. Any unsupported service/thread/child branch fails before effect; private patches may guard these existing source seams, not interpose a new OS monitor. |
| Configuration/build | [configure.ac](https://raw.githubusercontent.com/gpg/gnupg/gnupg-2.5.24/configure.ac):115–131 makes agent required, so do NOT invent --disable-agent. --disable-exec exists(:317–326), but does not alone establish service/thread reachability. Supported switches for card/dirmngr-autostart/sqlite and generated component/TOFU/readline/LDAP/TLS outcomes must be inspected. Build scratch may contain upstream tools; only qualified git/gpg/libraries are staged. Absence of an installed agent is not the source guard. |
| Libraries/platform | Record recursive actual loaded closure, rpaths/install names, constructors/dlopen and selected code paths; no mutable /opt/homebrew/opt dependency or DYLD injection. Name exact pinned platform/libSystem assumptions separately. Symbol/grep inventories assist the call-path review; they do not constitute it. Unexpected image/thread/helper keeps candidate unsupported. |

Source review is a bounded trusted premise for these exact builds/commands/inputs, not a theorem about arbitrary binaries, compiler correctness or kernel scheduling. Patch only upstream source under private build work, retain patches under fixtures with upstream GPL/LGPL/origin notices and precise source links; preserve licenses in candidate. No production harness/worker/wire patch belongs to builder.

## Files, fields and custody

All generated files private under D=$TARGET_DIR/dream-proof-tools; no global install/config. Fresh build_id canonical ULID and monotonic builder incarnation. Layout:

~~~text
D/cache/<archive-sha256>/{archive,signature,auth.json}     # verified source reuse
D/runs/<build_id>/{src,build,public-home,logs}            # mutable owned work
D/candidates/<build_id>/{bin/git,bin/gpg,lib,licenses,provenance/blobs}
D/candidates/<build_id>/qualification.json              # <=128KiB evidence
D/candidates/<build_id>/receipt.candidate.json           # <=32KiB; published LAST
~~~

Candidate prefix is chosen before build; staged libraries must be relocatable within that prefix or retain its exact final absolute paths. Never move/rewrite an admitted candidate. Candidate ID is external SHA256(receipt.candidate.json); do not place its own hash in its content. Qualification precedes receipt; qualification never references the future receipt digest, avoiding cyclic seals.

JSON is UTF-8 canonical sorted keys/no LF; strict known fields, finite arrays, checked integers≤2^53−1. Hex=64lowercase; paths relative/no traversal within prefix except explicit absolute SDK/compiler/platform references≤4096bytes. FileRef={path:String,len:u64,sha256:Hex}; hashes cover exact bytes. No symlink/regular-file ambiguity.

**receipt.candidate.json is exactly existing proof-tools/1 DTO**, unchanged:
{schema,source_sha256,os:{release,arch},git:{absolute_path,sha256,build,features},gpg:{same},capability:{probe_source_sha256,probe_exe_sha256,evidence_sha256,features}}.
source_sha256 in the builder's proposal records its frozen consumer snapshot, not builder hash. It is NOT the final runtime receipt. After admission, Q2 alone materializes the unchanged DTO with its current approved provisioning/guard source hash and binds the resulting receipt bytes through the existing expected-receipt hash. Builder proposal bytes are never silently relabelled runtime-approved. evidence_sha256 hashes qualification.json. Feature strings are candidate assertions, not runtime approval.

**qualification.json fields (one128KiB total, no extra uncharged evidence copy):**
~~~text
{format:"bboxy.dream.proof-tool-qualification/1",
 build_id:Ulid, incarnation:u64, builder:{commit:Sha40,source_sha256:Hex},
 source_lock:FileRef,
 sources:[{name,archive_sha256,signature_sha256,signer_primary,signer_subkey,
           verifier_sha256,auth_exit:i32,auth_status_sha256}],
 patches:[FileRef],
 compiler:{path,sha256,version:FileRef}, sdk:{path,version,settings_sha256},
 platform:{os_build,release,arch,assumptions:[{id,evidence:FileRef}]},
 tools:{git:FileRef,gpg:FileRef}, loaded_closure:[{file:FileRef,kind:"private"|"platform",
             identity:String,assumption:String|null}],
 configs:[FileRef], command_profile_sha256:Hex,
 review:[{id,source_refs:[FileRef],finding:"established"|"unsupported",reason:String}],
 component:{authority_source:FileRef,probe_exe:FileRef,report:FileRef},
 checks:[{command:String,exit_code:i32,source_sha256:Hex,log:FileRef}],
 logs:[FileRef], candidate_status:"review_ready"|"unsupported"}
~~~
All properties required; nullable only as declared. Unannotated hash fields/suffixes are Hex; commit is40lowercase hex; signer_primary/signer_subkey are40uppercase hex (direct primary signing repeats the primary). Remaining unannotated scalar fields are UTF-8 strings, auth_exit/exit_code are i32; incarnation is1..2^53−1, len is0..2^53−1. IDs/names≤128B, reason/assumption/identity≤2048B, commands≤4096B. Arrays≤64 entries each, patches≤8, sources exactly seven for selected closure, tools exactly two; component.report references **unaltered canonical Q2 capability-result bytes**; Q2 owns their interpretation/fixtures and supplies the frozen authority source, so builder creates no second schema mirror or claimed pre-existing strict validator. FileRef structure/custody/length/hash is strict. qualification.json plus prepared source-lock bytes plus component-report bytes together must fit the EXISTING128KiB evidence allowance; no uncharged referenced input copy. Refuse rather than clip required proof. Large logs/source trees stay private outside Git; retain referenced source/config/log bytes as owned regular provenance/blobs/<sha256> files in the sealed candidate root, with exact len/hash and no traversal. Primary source-lock bytes are copied unchanged into provenance. They are review evidence, not data to scan during a live call. Their bytes are not implicitly additional runtime inputs. Source-lock and compact summaries are admitted/charged within the same evidence budget; host prevalidates long-log hashes offline.

Local metadata observed: SDK27.0 settings SHA7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb; ToolchainInfo.plist SHA b2e3b746c87ddfd6d2145656d87569054807e2196d6f46ef09dec3923ed31541 only identifies XcodeDefault. Executor must record actual compiler executable/hash/version, linker/SDK and generated link/config outputs; this ICD does not fabricate them.

Publication: owner-only0700 dirs/0600 data, exclusive files, no-follow custody, same-filesystem temp→no-replace publication, fsync bytes/parent directory. Publish qualification then receipt last; no marker/exit alone admits a prefix. Existing cache winner is reused only after full byte/auth/custody verification; no global lock/queue. Unknown write/rename retains original build_id/temp/inode/effect and reports unresolved; inspect that effect before retry, never overwrite a same-name different candidate. Crash before receipt leaves unapproved incomplete work. Cache corruption/auth failure quarantines that exact owned entry. Cleanup deletes only recorded owned paths/PIDs, never shared caches/peer artifacts or an admitted prefix; published evidence/logs remain until Root dispositions it. Restart may reuse authenticated cache, never reuse an old approval for changed bytes/incarnation.

## Approval boundary: no second truth store

Builder returns locator {prefix,build_id,receipt_sha256,evidence_sha256} plus report/source commit; candidate files contain NO approved flag, root signature or runtime grant.

Root review admits the finite premise through **existing approved provisioning source/artifact authority**. Concrete minimal Q2 consumption: one optional approved profile literal in Q2's existing resolve/provisioning owner, initially absent, binding qualification evidence/tool/closure/config/platform and upstream/builder source pins. It MUST NOT embed its own/current Q2 provisioner hash or the final runtime receipt hash: those would create a self-reference. Q2's current source/artifact is externally approved by the existing producer mechanism, then Q2 materializes the final receipt as above. Builder proposal digest remains review provenance, not a compiled runtime admission key. Only the Root-reviewed Q2 source/artifact update may fill this literal; builder scope excludes those files. This is an explicit pin in the already approved provisioner, not an ambient KB registry/service, mutable candidate trust store or new worker field. Root's KB note/artifact_claim or a builder “pass” alone cannot set it.

Q2 compares prepared candidate bytes and current actual paths/OS/libraries/config/source against that approved tuple off-owner, before birth; uses existing public authority and suspended/native/helper admission fences. No recursive Git/KB/custody scan on hot owners. Missing/stale/forged/mismatched tuple=verification_unproven; changed admitted bytes follow source_changed; bad proof=verification_failed; measured resource breach=resource_limit. Keep original effects/debt/identity through refusal and actual reap/quarantine. Root approval admits the lifetime premise only, never a lesson/publication/trust promotion.

## Private component: source-built candidate, never runtime approval

[[decisions/d93377961365875-dream-ksba-private-probe-boundary]] adds ONLY canonical probe CLI mode:
~~~text
proof-helper-probe private-candidate /absolute/pretest-input.json <expected_input_sha256>
~~~
argc=4; no aliases/extra flags/env approval. Normal git mode retains exit81/verification_unproven; native and ledger-controls retain their existing behavior. Q2 owns this mode/decoder/results and uses its SAME per-member/clock/stop/roster/CPU/debt/foreground/heap/reap machinery, no new controller. Builder never edits/forks it. Root later freezes changed source/executable/hash for builder consumption.

Private input is canonical UTF-8 JSON≤16KiB, depth≤6, fixed required fields/unknowns rejected:
~~~text
{kind:"private_candidate_test",run_id:Ulid,nonce:Ulid,incarnation:u64,
 deadline_ns:u64,os_build:String,arch:String,
 probe_source_sha256:Hex,probe_exe_sha256:Hex,
 prefix:DirPin,source_lock:FilePin,build:FilePin,source_review:FilePin,
 closure:FilePin,config:FilePin,commands:FilePin,
 git:FilePin,gpg:FilePin,
 fixture:{repo:DirPin,manifest:FilePin,valid_commit:Sha40,corrupt_commit:Sha40,
          public_keys:FilePin},
 output:DirPin,
 case:"positive"|"corrupt_proof"|"pause_resume"|"cancel"|
      "stale_incarnation"|"clock_failure"|"debt_overlap"|"late_stop"|"reap_delay"}
DirPin={path:String,dev:u64,ino:u64,uid:u32}
FilePin={root:"candidate"|"fixture",path:String,len:u64,sha256:Hex,
 dev:u64,ino:u64,mtime_sec:i64,mtime_nsec:u32}
~~~
u64 uses checked full-width parsing; incarnation nonzero monotonic and deadlines same-boot monotonic, original total≤30s including validation/settlement. mtime_nsec<1e9. Paths≤4096UTF-8bytes, no NUL/traversal/symlink; prefix/fixture are exact owned roots, output is fresh owned0700 per-run directory. File paths are relative to declared prefix/fixture, regular one-link owner files; git/gpg executable and existing custody/mode checks apply. Absolute equality/version alone is insufficient. Cases are fixed Q2-owned causal fixture scenarios, not commands or authority flags; positive and real corrupt-proof cases remain distinct.

Input contains ONLY existing immutable **pre-test** artifacts: exact7-source lock/auth record, actual builder/compiler/config/closure/command/source-review manifest and real git/gpg pins, public-only fixture commits/keys. It NEVER refers to its own/future report, final qualification, receipt or approval hash. Pre-test source review binds these actual artifacts/assumptions and fixed65126 command set; no generic true/approved flag is accepted. Expected input hash comes from the explicitly scoped test-host invocation, not the descriptor itself. That binds integrity, not a runtime grant or cryptographic proof of the source premise; private invocation/output cannot create product authority.

Before any helper birth: bounded read + expected hash, strict shape/size/custody, fixed source/build/config/command/closure/tool/OS/probe and case/fixture binding; reject missing/changed/forged/stale/same-version/wrong-root values. Validate the pre-test review against its exact source tuple, never treat its text as Root runtime approval. Hash/read validation and preparation are bounded measured ordinary units with fresh debt; no wrapper/native exemption. Metadata input/descriptor copies remain charged in existing allocations and aggregate prepared evidence≤128KiB, receipt≤32KiB; public keys/captures keep existing2MiB/4KiB IO limits. No account/keyring/source sweep or inference.

Phases: validate → claim original run/nonce/input/incarnation → public_keys → git_reads → git_signature → settling → settled (or owned quarantine). Direct public show/import uses the existing one-member helper path/private public-only home; real Git readonly/signature calls use exact65126 argv/config and its existing two-slot unknown-fork/exec coverage. Main expensive preparation and helper grants never overlap or bypass debt. pause/cancel/clock/ABA/failure scenarios drive existing canonical fixture control hooks; component results do not certify external MCP/UI latency. Status/control/foreground remain independently tested by Q2's product gates.

Per-run exclusive claim/result publication is not a global lock. Duplicate same original operation/input returns its existing result or owned pending state, no second helper; changed input refuses. Restart/unknown effects retain exact child/PID-start/audit/nonce/slots/debt and source pins, reaping or quarantine before any replacement/trust. No success/owner release before actual group/retired-member settlement. All30s/1ms/native2ms25ms/3msinclusive/9x rest/256 attempts/two members/one thread/four jobs/heap/control laws remain.

Q2-owned stdout result≤64KiB canonical JSON+LF, exact fields:
~~~text
{profile:"read-helper-native/1",kind:"private_candidate_result",
 run_id:Ulid,nonce:Ulid,incarnation:u64,input_sha256:Hex,case:Case,
 probe:{source_sha256:Hex,exe_sha256:Hex},
 binding:{source_lock:Hex,build:Hex,source_review:Hex,closure:Hex,
          config:Hex,commands:Hex,git:Hex,gpg:Hex},
 basis:{kind:"conditional_on_bound_source_review",review_sha256:Hex,
        whole_command_lifetime_proof:false},
 runtime_approved:false,availability:"not_applicable_private_test",
 phase:Phase,outcome:"completed"|"expected_rejection"|"refused"|"failed"|"quarantined",
 error:{code:Code,detail:String}|null,
 observed_proof:"valid"|"invalid"|"not_reached",
 metrics:{ordinary_max_ns:Ns,controller_max_ns:Ns,cleanup_max_ns:Ns,
 native_cpu_max_ns:Ns,native_wall_max_ns:Ns,inclusive_max_ns:Ns,
 W_ns:Ns,A_ns:Ns,C_ns:Ns,Q_max_ns:Ns,post_cpu_ns:Ns,late_kernel_ns:Ns,
 debt_charged_ns:Ns,debt_paid_ns:Ns,helper_group_rss_peak_bytes:Ns,
 probe_rss_peak_bytes:Ns,metadata_bytes:u64,attempts:u16,members_max:u8},
 settlement:{created:u16,reaped:u16,group_gone:bool,retired_coverage:bool}}
~~~
Case is input enum; Phase is phases above including quarantined. Code is input_invalid/source_changed/review_unbound/coverage_unknown/clock_unknown/resource_limit/signature_unexpected/cancelled/deadline/settlement_unknown; detail≤512bytes. Ns=u64|null; null only for unknown/unobserved failed metrics, never invented zero; known reversal fails. Metrics sum complete observed work/debt without clamping; RSS is labelled RSS, not whole-process heap. Per-grant/phase raw clocks/identity/window ledgers are retained as bounded private stderr/log evidence, not omitted behind maxima. Existing canonical source controls define their exact raw fields. Whole-command absence of unseen work stays a conditional source premise even when observed coverage/metrics succeed.

Exit0 ONLY for positive completed valid proof or the specified real corrupt-proof expected_rejection, all gates and actual settlement satisfied. Other refusals/failures use81; unresolved owned quarantine is not reported as settled or successful, and existing control/settlement ownership remains until exact effects are disposed. No result contains available/runtime-approved or verified lesson. Runtime admission must reject this result kind; builder preserves exact raw bytes through component.report FileRef, never converts them into approval/rewrites fields. Root later reviews complete evidence and Q2 final integration remains separate.

Required negatives: normal-mode installed refusal vs private real candidate; missing/forged/stale/same-version/wrong-root/tool/OS/config/source/probe mismatch before birth; unapproved private result fed to runtime is refused; duplicate/restart/ABA/nonce/clock/debt/late-stop/reap controls fail causally through same canonical mechanisms. Root supplies final changed source pin and amends consumption; builder may continue KSBA/auth/config/guard work meanwhile.


## Strict pre-test basis: one manifest, six fixed projections

Completion of decision65875, dated2026-11-06 UTC. This subsection governs the **inner private pre-test inputs**, not the public proof-tools/1 DTO or final qualification format above. Q2 must return review_unbound **before helper birth** for an outer-only descriptor, opaque prose, missing inner fields or incomplete coverage. No installed pair becomes qualified.

Builder owns canonical `candidate/provenance/pretest-basis.json`, format `bboxy.dream.private-pretest-basis/1`; Q2 owns its one strict decoder in the canonical probe. The six outer FilePins `build,source_review,closure,config,commands,fixture.manifest` MUST be identical in EVERY field and pin this ONE file. Their meaning is respectively `.build,.review,.closure,.config,.commands,.fixture`; no independently interpreted sibling documents, six copies, inferred projection names or extra authority store. Read/hash/parse once, with every actual retained copy charged. Existing outer input/result fields and public worker schemas are unchanged.

All fields shown below are REQUIRED, unknown/duplicate fields fail, and only shown null unions allow null. JSON UTF-8 without BOM/LF, recursively ASCII-key sorted, no whitespace, shortest decimal integers (no -0/exponent/floats/nonfinite); strings use JSON escapes for quote/backslash and the standard short control escapes (\\b, \\t, \\n, \\f, \\r); lowercase \\u00xx for remaining controls, otherwise literal UTF-8. Reject NUL in any string, invalid UTF-8/surrogates and noncanonical bytes. This matches builder canonical()/strict_json rather than hashing a reparsed permissive input. SHA256 always covers those exact canonical bytes; projection digests cover a projection ALONE with the same encoding. Lists retain declared order. No Unicode/path/case normalization.

Private inner integers are checked full-width u64/u32/i64/i32 as annotated (not floating-point); incarnation>0, mtime_nsec<1e9. Hex=64 lowercase hex; Sha40=40 lowercase hex; Fingerprint40=40 uppercase hex; Uuid=32 lowercase hex; Ulid=[0-7][0-9A-HJKMNP-TV-Z]{25}. Text is UTF-8; IDs/names≤128B, rationales≤512B, paths/argv/env values≤4096B, version/reported_target≤256B. Depth≤10; any array≤64 except edges≤128 and fixture.files≤32; auth/generated exactly7, patches≤8, patch.files≤16, findings exactly8, spans≤16/finding, commands.forms exactly16, link_maps exactly2. No empty IDs, spans or findings.

Basis≤48KiB, source-lock≤32KiB, outer descriptor≤16KiB, canonical raw result≤64KiB including its LF. **descriptor+basis+source-lock+raw result+qualification envelope≤128KiB combined**, counting shared basis bytes once logically and all physical copies in existing working-storage credits. Reserve actual component lengths plus a bounded result allowance BEFORE reading/growing; individual maxima do not override aggregate. The final qualification envelope may repeat compact facts only if charged. Receipt remains≤32KiB separately; public keys/capture/line/control allocations retain their existing limits. Large archives, raw source/config/link/build logs are offline review artifacts referenced by exact len/hash, not silently admitted extra metadata or a live recursive scan.

~~~text
Basis={format:"bboxy.dream.private-pretest-basis/1",
 build:Build,closure:Closure,config:Config,commands:Commands,
 fixture:Fixture,review:Review}
Ref={path:Text,len:u64,sha256:Hex}
AbsIdentity={path:Text,len:u64,sha256:Hex,dev:u64,ino:u64,
 mtime_sec:i64,mtime_nsec:u32}
Platform=ClosureSchema.$defs.Platform  # exact signed C authority below

Build={build_id:Ulid,incarnation:u64,
 builder:{commit:Sha40,source_sha256:Hex},source_lock:Ref,
 auth:[Auth],patch_lock:Ref,patches:[Patch],
 recipe:{file:Ref,commands:Ref,env:Ref},
 compiler:{image:AbsIdentity,version:Ref,target:"aarch64-apple-darwin",
           reported_target:Text},
 linker:{image:AbsIdentity,version:Ref},
 sdk:{path:Text,canonical_name:Text,version:Text,settings:Ref},
 platform:Platform,generated:[{source:SourceID,config:Ref,summary:Ref}],
 tools:{git:AbsIdentity,gpg:AbsIdentity}}
Auth={name:SourceID,archive_sha256:Hex,signature_sha256:Hex,
 signer_primary:Fingerprint40,signer_subkey:Fingerprint40,
 verifier_sha256:Hex,auth_exit:i32,auth_status_sha256:Hex,status:Ref}
Patch={source:Text,patch:Text,sha256:Hex,
 files:[{path:Text,before:Hex,after:Hex}]}

Closure=ClosureSchema  # exact signed root schema; no alternate Node/edge/Macho form

Config={gpg_conf:Ref,common_conf:Ref,env:[{name:Text,value:Text}]}
Commands={profile:"65126",git_prefix:[Text],gpg_prefix:[Text],forms:[Form]}
Form={id:CommandID,tool:"git"|"gpg",argv:[Text],members:1|2,
 stdin:"none"|"public_keys"|"commit_payload"}

Fixture={repo:DirPin,files:[Ref],ancestor:Sha40,valid_commit:Sha40,
 corrupt_commit:Sha40,policy_branch:Text,record_path:Text,record_blob:Sha40,
 origin_url:Text,public_keys:Ref,signer_primary:Fingerprint40,
 valid_payload:Ref,valid_signature:Ref,corrupt_signature:Ref}

Review={bindings:{build:Hex,closure:Hex,config:Hex,commands:Hex,fixture:Hex},
 findings:[{id:ReviewID,sources:[SourceID],commands:[CommandID],
 phases:["entry","normal","error","cleanup","initializer"],
 spans:[{source:SourceID,path:Text,sha256:Hex,first:u32,last:u32}],
 patches:[Hex],guard:Guard,
 finding:"conditional_source_review",rationale:Text}]}
~~~

SourceID/order is EXACTLY `git,gnupg,libgpg-error,libgcrypt,libassuan,npth,libksba` with the seven versions/URLs/archive/signature/key/root/subkey/signed-byte pins in Frozen inputs. Build.auth/generated use that order, no duplicate/omitted source. Source-lock is the existing strict proof-tool-sources/1 object: exact seven source rows, backports:[], profile_patches, pinned independent bootstrap verifier and canonical_q2 source Ref/hash; unknown fields fail. Auth is the actual builder authenticate() record plus its status Ref: archive/signature/verifier/signers equal the selected pinned row, auth_exit=0, auth_status_sha256=status.sha256. Source authentication is performed by the builder with the independent public-only verifier; source-key bytes/status/log refs remain preserved offline. A zero or hashed status string supplied by a caller is not independently verified authentication. Q2 compares the tuple against its frozen scoped qualification premise; it does not run a new source-authentication GPG helper before admission.

Patch rows use the EXISTING patches.lock array shape (source is versioned, e.g. git-2.54.0). patch_lock pins its exact canonical array, Build.patches equals it; its patch SHA/order equal source_lock.profile_patches; each before/after binds exact extracted/postpatch file bytes, no fuzzy application. Preserve the actual selected patch list, not illustrative hashes below. recipe.file.sha256=builder.source_sha256; recipe.commands/env pin actual command records/environment, generated rows pin actual config headers/summary for ALL seven, not guessed configure flags. Compiler/linker executable hashes/version bytes, generated configure/link outputs and SDK settings are bound to that build. These are immutable offline provenance, not another executable discovery recipe.

**Architecture/platform identity.** Canonical outer.arch, Build/Closure.platform.arch, final receipt.os.arch and consumer-approved profile all equal `aarch64`. Capture accepts raw Darwin `arm64`→`aarch64` ONLY; already aarch64 stays so. Reject arm64e/x86_64/unknown/case variants as architecture labels; never normalize an arbitrary target. Mach-O CPU_TYPE_ARM64=16777228, thin candidate tools with captured exact subtype0; platform-library subtype/UUID are exact captured platform bindings, not a new architecture alias. Compiler canonical target is aarch64-apple-darwin; reported_target must match `(arm64|aarch64)-apple-darwin[0-9]+(\.[0-9]+){0,2}`, checked against platform.release after only that named arch mapping. Darwin release matches [0-9]+(\.[0-9]+){1,2}; os_build [0-9]{2}[A-Z][0-9]{1,6}[a-z]?; SDK version [0-9]+(\.[0-9]+){1,2}, canonical_name exactly "macosx"+version. SDK path is absolute, settings Ref is its exact captured SDKSettings.json bytes; compiler/linker paths are absolute fixed resolved images, not PATH/xcrun at test time. Record the exact Platform6 tuple defined by the signed C authority below; candidate file/Mach-O UUID currentness and OS/build/cache currentness remain mandatory. Platform-internal per-image UUIDs are offline provenance under the explicit trusted boundary, not prepared authorization metadata. Unsupported cache/platform identity cannot be guessed.

**Closure.** The signed single C authority in Closure boundary below defines the exact candidate-complete root, image/load/target/static/link-map shapes, canonical grammar and cross-bindings. Build.platform uses its SAME Platform6 definition. Root git/gpg bytes and outer FilePin dev/ino/len/hash/mtime must match the actual prefix+relative tool paths. All candidate images, linked static members, initializers and source/auth/generated records remain covered by the selected source review and actual output bindings; Libksba evidence is required even when build-only. Complete candidate facts and bound platform identity do not prove all dynamic mappings or helper behaviour; real runtime corroboration remains required.

**Config and command set.** Config.env contains EXACTLY the eleven unique name/value pairs in Fixed command and environment values below, in that order; no inherited env, agent/socket/loader/config/search extension. {home}/{scratch} are this run's fresh private directories only; PATH=/usr/bin:/bin does not authorize executable search. GPG config bytes exactly `batch\nno-tty\nno-autostart\nno-auto-key-retrieve\nno-auto-key-import\nauto-key-locate clear\nno-auto-check-trustdb\n`; common.conf is empty. Direct GPG uses the fixed gpg_prefix; implicit Git→GPG uses its reviewed actual child argv/private home/config, not an injected alternate prefix. Commands is EXACTLY profile65126, the two prefix arrays and sixteen Form rows in Fixed command and environment values below; arrays are ordered equality, no removed options, additional argv or free shell string. All placeholders have typed expansion only: gpg=Build.tools.gpg.path; commit=fixture valid/corrupt SHA40 for the selected case; ancestor/policy_branch/record=the fixture fields; there is no expandable {signature} placeholder: verify_child argv index 3 is the matcher {git_signature_temp} defined in Signature temporary below, never expanded. Prefix absolute executable is separate from argv and pinned; cwd is the owned fixture repo. Public key/payload stdin is exact pinned bytes. One signature form admits the existing one Git+one GPG group; verify_child describes that implicit child and is not a third direct command. The other direct forms have one member. Keep all accepted proof/signature checks and error/status semantics, including negative corrupt proof; this finite fixture profile does not weaken product selected-proof requirements.

**Fixture.** All Fixture Ref paths are relative to fixture.repo (other Ref paths relative to candidate prefix unless explicitly AbsIdentity/SDK). files is a lexically byte-sorted complete inventory≤32 owned regular files of this private presigned corpus, including public keys/payload/signatures; no symlinks/alternates/worktrees/hooks/foreign files, no operator keys or signing at test time. Directory identity equals outer.fixture.repo; keys match outer.fixture.public_keys root=fixture/path/len/hash; valid/corrupt commit fields equal outer fields and are distinct. policy_branch is an explicit refs/heads/... value, record_path a safe repository-relative path (no .git component), origin_url fixed ≤4096B without credentials. Fixture fixes ancestry/tree/blob/public signer/payload/signatures; Git/GPG commands must establish those semantic relationships AFTER admission, not assume them from JSON. Complete pre-birth inventory/custody/hash is bounded and paced; it is not an account/repository source sweep. Presigned real positive/corrupt fixtures remain a required implementation artifact; the example creates none.

**Review coverage, not a boolean grant.** Exactly eight findings with these ID/guard/minimum-source obligations. Each lists actual command IDs it covers, all five entry/normal/error/cleanup/initializer phases, concrete nonempty postimage file spans (1≤first≤last), and applicable patch hashes from Build.patches; its sources must be covered by spans. The union covers all seven sources and all sixteen command forms; these are minimum coverage checks, not a claim that one span proves an entire source. Source spans must resolve in the frozen authenticated+patched source snapshot retained by builder (patched file hashes equal the recorded after value); no forged free-form evidence path. A missing required guard, source, phase, command or span is review_unbound.

| ReviewID | Guard | Required source coverage |
|---|---|---|
| git_async | deny_async | git: async/thread entry, failure and cleanup |
| git_child | one_verify_child | git + gnupg: fixed implicit verify exec and no fallback/extra child |
| git_shell | deny_shell | git: shell/exec/error paths |
| gpg_service | deny_service | gnupg + libgpg-error + libassuan: service/socket/spawn paths |
| gpg_thread | deny_thread | gnupg + libgcrypt + npth: thread/service/constructor/error paths |
| libraries | review_only | all five library sources incl. KSBA: reachable parse/init/cleanup and transitive calls |
| input_config | fixed_inputs | git + gnupg: all16 forms, public-only inputs/config/lookup/error behavior |
| platform | review_only | git + gnupg bindings plus exact compiler/linker/SDK/OS closure assumptions |

Machine checks prove finite shapes/equality/hashes/custody/coverage and observed runtime outcomes. The source-path reachability judgement, faithful compiler/linker/bootstrap verifier, complete source/build closure and OS-library behavior are explicit **conditional trusted premises** reviewed by Root later, not a theorem checked by the probe. `finding:"conditional_source_review"` plus text never establishes them or grants runtime authority. Private tests intentionally measure under that scoped premise; hidden-process/thread mutants must invalidate the premise/capability even if a census misses them. Product runtime still requires the independent Root-approved provisioning source/artifact binding.

### Signature temporary

Canonical law for the verify_child signature operand. Rationale and source read set: [[decisions/d93377961365960-dream-launch-entry-and-signature-boundaries]] B. In real Git 2.54.0, the operand is a Git-owned temporary holding the signature bytes Git extracts from the commit object: gpg-interface.c:360–386, tempfile.c:191–195, wrapper.c:438–481. It is never the fixture path.

1. **Token.** verify_child argv index 3 is the matcher `{git_signature_temp}`. It is never expanded, and every expander refuses it. The admitted child's argv is exactly `[Build.tools.gpg.path, --keyid-format=long, --status-fd=1, --verify, V, -]`, where `V == S + "/.git_vtag_tmp" + c1..c6`, each `ci ∈ [A-Za-z0-9]`, and S is the exact `{scratch}` bytes Q2 placed in TMPDIR. The match is byte equality, with no normalisation and no `/tmp` fallback. The gpg_prefix is absent.
2. **Selected bytes.** B is the selected case's fixture Ref: `corrupt_proof` → `corrupt_signature`, otherwise `valid_signature`. Corpus obligation: each fixture commit's gpgsig bytes, as extracted by Git, equal its case's Ref bytes, and both cases reach the one verify child. A mismatch fails. Nothing is rewritten.
3. **Pre-birth.** Q2 retains an `O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC` scratch_fd on its fresh, empty 0700 scratch, recording dev/ino. The controller umask has no bit in 0600, and that umask is recorded. If any of this is unmet, or the source is unresolved, the result is `review_unbound` before any helper exists.
4. **GS1.** GS1 runs at the first observation in which member index1's current incarnation path equals Build.tools.gpg.path, before Q2 acks or continues that incarnation, while it is unreaped (pid, birth, pidversion current). Git cannot have deleted the file yet, because pipe_command has not returned. Checks:
   - **argv:** `sysctl{CTL_KERN,KERN_PROCARGS2,pid}` into a fixed 8 KiB buffer, bracketed by `proc_pidpath_audittoken` returning the same val[7] before and after. The argv must equal rule 1 exactly.
   - **file:** `openat(scratch_fd, name, O_RDONLY|O_NOFOLLOW|O_CLOEXEC|O_NONBLOCK)` must succeed. fstat must show S_ISREG, nlink==1, `(mode&07777)==0600`, uid==euid, dev==scratch dev and size==B.len. A pread of size+1 bytes must return exactly B.len bytes, with SHA256==B.sha256. `fstatat(AT_SYMLINK_NOFOLLOW)` must give the same dev/ino.
   - **exclusivity:** the scratch directory holds exactly {name}, enumerated with a bound of 8 entries.
   - **retain:** keep the fd and GS = {name, dev, ino, len, sha256, pid, pidversion}.
5. **GS2.** After the leader is reaped, name must give ENOENT. The held fd must show nlink==0 with the same dev/ino/size, and its rehash must be equal; then the fd is closed. If the file is still present after a normal exit, the result is `signature_unexpected`. On cancel/kill, owned cleanup unlinks the file only if dev/ino == GS; otherwise the result is `settlement_unknown` and the scratch goes to owned quarantine.
6. **Codes.** Shape, name, extra entry, ENOENT at GS1, ELOOP, link, mode, uid, dev, size, hash and GS2 identity failures give `signature_unexpected`. Observation unavailable, oversize or racing gives `coverage_unknown`.
7. **Budget.** There is no new process, thread, slot, controller, DTO, wire, version or limit. CPU is on the existing controller grant. I/O is ≤8 KiB plus 2×(B.len+1). Two fds are held. Two private stderr lines `git_sig_temp {phase,pid,pidversion,name,dev,ino,nlink,mode,size,sha256,t_ns}` are written, each ≤512 B. The changed Commands bytes give a new commands hash and invalidate old pending tuples.
8. **Residual premise.** GS binds the identity, custody and content of one file at two points. It does not prove that gpg opened that inode before GS1. That relies on the private 0700 scratch having no other writer.

### Closure boundary

The ONE C authority selected by [[decisions/d93377961366007-dream-c-closure-codec-authority]] completes [[decisions/d93377961365962-dream-platform-closure-boundary]]. It replaces obsolete Node/edge/load-string definitions, LF-joined member hashes, cache-only shortcuts and Platform spelling variants. Both owners consume these SAME signed bytes, including all x-canonical-grammar/x-budgets/x-constraints; structural JSON Schema validation alone is insufficient.

| Artifact at signed builder commit b26d7cd3432e9cae3e6c68492afd73a82cc8f2e9 | Bytes | SHA256 |
|---|---:|---|
| tests/fixtures/dream-proof-tools/closure-boundary.schema.json | 15157 | d70948ec375dae099ea1dac0b92dfb30ff87d7fcac94c98f42caa1c0ba2da8ec |
| tests/fixtures/dream-proof-tools/closure-boundary-vectors.json | 3900950 | bff32c1afb483980952a6761918d4005c9f1e12b66f3d7aaaa88f1b9afe09740 |

ClosureSchema is the exact root schema; its $defs.Platform defines BOTH Build.platform and Closure.platform: arch,darwin_machine,release,os_build,shared_cache_uuid,premise=trusted_platform_boundary/1. The two tuples equal in every field. Canonical arch=aarch64, with only the declared raw Darwin arm64 alias. Old darwin/cache_uuid spellings fail.

- Candidate facts are complete: roots git/gpg, every reachable candidate dylib, original ordered raw loads WITH modes/multiplicity, rpaths/UUID/file/source bindings. Targets are exact in-prefix candidate images under the artifact's ownership/resolution rules, or normalized /usr/lib/ or /System/Library/ install names in the bound shared cache. Unknown/ambiguous, @rpath, foreign/noncache and direct-root libobjc-env remain Unsupported. OS-internal images/edges/UUIDs stay offline trusted-platform provenance; no runtime catalogue.
- Static/map bindings cover EACH reachable (tool,output) and agree for shared outputs. Selected member occurrences retain link-map order and repeats. Exact members_sha256 framing is SHA256(concat(u32BE(byte_length(raw_name)) || raw_name)); no LF delimiter, normalization, sorting or deduplication. Empty selection has no StaticUse. Actual archives/indexes/maps are recomputed offline and bound by bounded Refs, not uncharged runtime authority.
- Before birth and after actual settlement, Q2 compares candidate pins and its own _dyld_get_shared_cache_uuid, kern.osversion, kern.osrelease and arch with the prepared tuple. Observed mismatch is sticky across A→B→A (review_unbound before birth, invalidated after), preserving original effects/debt/reap. Equal checkpoints do not prove unobserved transient changes, arbitrary mappings or platform/helper behaviour; cleared Config.env and conditional source/platform premises remain.
- Caps and all runtime laws are unchanged:64nodes including statics, arrays64,128 raw-load/static-use occurrences, depth10 including leaves, Basis49152B, aggregate131072B, receipt32768B, descriptor16384B, source-lock32768B, result65536B and charged physical copies. The corpus is OFFLINE test data. Actual final-pair fit is UNKNOWN; over-cap or unknown/unsupported binding stays Unsupported. Format identifiers/public wire/version are unchanged; new canonical projections invalidate old tuples.

Root verified the Boxy signature and exact hashes, read the schema, reproduced the author subset/context checker (324 vectors,44accept/280reject, exit0), and independently checked canonical artifacts, all vector lengths/hashes/IDs and13context hashes (exit0). External metaschema validation was unavailable. Whole-Closure, schema-fragment, grammar, framing and accounting vectors have distinct scopes. These checks confer no real Basis/authentication/fit/qualification/runtime approval or lesson availability; final emitter/consumer/formal/positive+corrupt-proof gates still require the frozen Q2 source/executable/compiler tuple.

### Pre-birth cross-binding and failure order

1. Scoped private invocation/expected_input_sha256, strict canonical outer/basis/source-lock, size reservations, owned roots/inodes/no-follow/modes, same-boot deadline/current incarnation and no original-effect reuse with changed bytes. Six projection pins identical; Build.source_lock Ref equals outer source_lock bytes/path. No helper may exist yet; failed identity/clock retains original ownership and honest refusal.
2. Frozen probe source/exe in outer equals actual scoped host provision and source_lock.canonical_q2; it is NOT a hash hardcoded in its own C source. Build source/auth/patch/config/generated/recipe/compiler/SDK/closure/commands and Review.bindings match exact prepared tuple, not a version string. Each binding is SHA256(canonical projection), with no review/self digest. Changes need new descriptor/incarnation, no silent rebind.
3. Tools/loaded images/OS/config and fixture equality/coverage checks above, off hot owners in ordinary bounded units with debt. Hold verified handles/identities where supported and revalidate the exact bounded currentness tuple immediately before birth and after stop/settlement; a mutation invalidates result. No hash is treated as a reusable authorization token. No filesystem discovery from IDs, live KB/custody scans, source reruns or invisible copy allocations.
4. Only after those checks may the private canonical controller create its original helper under existing bootstrap, slots, suspended-start, member-window/debt/foreground/reap law. Independent status/cancel/pause stays live. Prefix/config/source/OS/probe/fixture mismatch before birth is review_unbound (bad syntax=input_invalid); actual changed retained bytes are source_changed; resource/clock/coverage/settlement failures retain their existing honest codes/custody. Normal git mode stays81, private runtime_approved=false.

Acyclic order is frozen Q2 source → built probe → source-lock → five independent projections → review → basis → outer descriptor → raw component report → qualification → candidate proposal → Root-approved consumer profile → runtime receipt. Probe source/exe MUST NOT embed its own source/exe hash, basis/source-lock/report/receipt hashes; expected values come from the explicitly scoped host after source freeze. No pre-test object refers to its own/future result/qualification/receipt. Outer build/source_review/closure/config/commands result bindings deliberately have the SAME basis hash; their fixed projections carry meaning.

Required decoder/conformance controls in BOTH owners' canonical Make gates: complete typed input parses, wrong/extra inner field, wrong projection digest, six unequal FilePins, six-source/dropped auth/patch/generated/loaded node/review span/command/phase, stale same-version binary/SDK/library, arm64 supplied as canonical outer label, x86/unknown/fat/wrong Mach-O, wrong compiler target/cache UUID, foreign root/config/key/commit, substituted probe or current-source self-hash cycle all refuse **before any helper birth**. Normal-mode refusal vs explicitly private real-pair execution, raw unapproved output rejected by runtime and existing per-slot/clock/ABA/debt/control/reap negatives remain. Extend the existing finite model with innerBound/coverage/arch/currentness and Parse/Bind/Change/Reject/Admit actions, initially unbound, two candidates and A1/B2/A3 epochs; weak fairness only for internal progress. Broken Bind-before-coverage, stale-incarnation and unapproved-to-runtime transitions must fail. Models abstract source/compiler/OS assumptions explicitly; no claim of arbitrary-native proof or runtime performance from these schema examples.

### Fixed command and environment values

These are normative profile65126 values for Commands and Config.env; paths/placeholders expand only as defined above. IDs are the exact CommandID enumeration. The complete nonauthoritative worked Basis/outer input is retained with this completion at [[reports/2026-11-05-dream-private-pretest-inner-schema#complete-typed-worked-input]]. No example artifact hash grants authority.

~~~json
{"commands":{"profile":"65126","git_prefix":["--no-pager","--no-optional-locks","-c","core.fsmonitor=false","-c","core.hooksPath=/dev/null","-c","credential.helper=","-c","protocol.allow=never","-c","gpg.format=openpgp","-c","gpg.program={gpg}","-c","gpg.openpgp.program={gpg}"],"gpg_prefix":["--no-options","--no-autostart","--batch","--no-tty"],"forms":[{"id":"root_bare","tool":"git","argv":["rev-parse","--is-bare-repository"],"members":1,"stdin":"none"},{"id":"root_top","tool":"git","argv":["rev-parse","--show-toplevel"],"members":1,"stdin":"none"},{"id":"policy_main","tool":"git","argv":["rev-parse","--verify","{policy_branch}"],"members":1,"stdin":"none"},{"id":"origin_main","tool":"git","argv":["rev-parse","--verify","refs/remotes/origin/main"],"members":1,"stdin":"none"},{"id":"object_type","tool":"git","argv":["cat-file","-t","{commit}"],"members":1,"stdin":"none"},{"id":"ancestry","tool":"git","argv":["merge-base","--is-ancestor","{ancestor}","{commit}"],"members":1,"stdin":"none"},{"id":"tree_record","tool":"git","argv":["ls-tree","-z","{commit}","--","{record}"],"members":1,"stdin":"none"},{"id":"show_record","tool":"git","argv":["show","{commit}:{record}"],"members":1,"stdin":"none"},{"id":"last_touch","tool":"git","argv":["rev-list","-1","{commit}","--","{record}"],"members":1,"stdin":"none"},{"id":"blob_pin","tool":"git","argv":["rev-parse","{commit}:{record}"],"members":1,"stdin":"none"},{"id":"tree_pin","tool":"git","argv":["rev-parse","{commit}^{tree}"],"members":1,"stdin":"none"},{"id":"origin_url","tool":"git","argv":["config","--get","remote.origin.url"],"members":1,"stdin":"none"},{"id":"signature","tool":"git","argv":["log","-1","--format=%G?%x00%GF%x00%GP","{commit}"],"members":2,"stdin":"none"},{"id":"show_keys","tool":"gpg","argv":["--with-colons","--show-keys","-"],"members":1,"stdin":"public_keys"},{"id":"import_keys","tool":"gpg","argv":["--import","-"],"members":1,"stdin":"public_keys"},{"id":"verify_child","tool":"gpg","argv":["--keyid-format=long","--status-fd=1","--verify","{git_signature_temp}","-"],"members":1,"stdin":"commit_payload"}]},"env":[{"name":"GIT_CONFIG_NOSYSTEM","value":"1"},{"name":"GIT_CONFIG_GLOBAL","value":"/dev/null"},{"name":"GIT_TERMINAL_PROMPT","value":"0"},{"name":"GIT_ASKPASS","value":"/usr/bin/false"},{"name":"SSH_ASKPASS","value":"/usr/bin/false"},{"name":"LANG","value":"C"},{"name":"LC_ALL","value":"C"},{"name":"PATH","value":"/usr/bin:/bin"},{"name":"HOME","value":"{home}"},{"name":"GNUPGHOME","value":"{home}"},{"name":"TMPDIR","value":"{scratch}"}]}
~~~

## Canonical implementation brief outline

ROLE: EXECUTOR; prospective tree/branch/base above; one writer. Proposed exact source scope:

- Makefile, README.md, ATTRIBUTION.md.
- scripts/dream-proof-tools.py (one recipe owner; Make calls its fetch/build/check subcommands, no second wrapping installation script).
- tests/fixtures/dream-proof-tools/**: source lock/public release keys/signatures, reviewed source patches/command map, public-only presigned corpus, negative controls and attribution/license map.
- tests/dream_proof_tools.rs; tests/formal/dream_proof_tools.rs; tests/formal/models.rs; verification/{corpus,contracts,coverage}.toml; required test module declaration only if actual baseline needs it.
- Task report through native knowledge tools. No new crate, broad worker/proof/IR/API edits, accepted C1 change or live Q2 write.

Add **make fetch-dream-proof-tools**, **make build-dream-proof-tools**, **make test-dream-proof-tools**. Only fetch has network; fixed HTTPS pins/expected roots, bounded32MiB/archive download, reject unsafe extraction paths/special files, exact patch application/no fuzzy fallback. Build/test offline by default; missing authenticated cache fails actionably. Reuse only matching source/recipe/compiler/SDK/patch/closure hashes. Build with two niced jobs in private TARGET_DIR; no brew/global Cargo/config/install/credential/agent mutation. Developer provisioning is not a product CPU-law exception.

Public-only homes/keys, fixed config/env and prerecorded public signatures; no operator key export, signing/key generation or live agent for qualification. Preserve ALL65126 command forms and real Git/GPG status/error semantics. Qualification test may run a private unapproved candidate solely under its explicit test brief, labelled unapproved; never invoke a falsely approved managed worker or live lesson. Reuse the canonical pending Q2 helper/probe at a Root-frozen source hash; don't implement a competing controller or certify lifetime with Trace2. Q2 source at audit: resolve9e244932…; probe5493dbf0… was moving. Root must put the **full chosen current probe/source hash** in Executor brief before its component witness; source authentication/build/guard work is independent and can proceed.

## Executable verification matrix

Keep existing runtime1ms application/controller/cleanup/grants, native2msCPU/25mswall/3msinclusive,9x debt, one group/two processes/one runnable thread,256 attempts,30s/four jobs, memory/control/foreground/reap and bootstrap laws. Component evidence does not certify Q2 full preparation.

| Finite model / causal broken control | Real script/process witness |
|---|---|
| Vars: two builders, two candidates, epochs A1/B2/A3, auth/build/closure/witness booleans, stage absent/partial/published/approved/revoked, originalEffect, debt0..2, consumerBound/trusted; Init none approved. Actions Fetch/Auth/Build/Review/Publish/RootAdmit/Bind/Change/Fail/Restart/Retire. Only internal progress weakly fair, never assume IO returns. | Current-source Yarn/native + pinned Java TLC full state/transition/verdict parity; unknown/incomplete/timeout fails. Models prove declared credits/bindings under explicit compiler/platform/source assumptions, not arbitrary native correctness. |
| Let builder self-approve; approve partial/failed auth/closure; reuse old hash after A→B→A | Mutated receipt/evidence/library/config/OS/compiler/source or interrupted publication, stale valid-looking candidate and forged feature claims all fail before runtime birth. |
| Remove one source guard; permit extra async/fork/shell/service/thread/constructor path | Deliberately reachable private mutants must lose source/build qualification; explicit unapproved component test records short-lived process/thread marker/exit. Census may miss it; do not claim universal runtime detection. |
| Lose original effect/unknown write, premature cleanup; debt reset/overlap, clock/reap/retired coverage loss | Crash/restart/unknown publication and owned exact-PID settlement; canonical Q2 component negatives cover per-member stop/exec/hidden work/CPU roll-up/control/foreground/heap/reap under unchanged limits. No fake “available” result. |
| Signature temporary: tmp {absent,git(i),planted,swapped} × argv {temp(i),fixture,other} × gs {none,bound,settled} × leader {running,reaped,killed} × case {valid,corrupt}; ok ⇒ GS bound to Git's inode with B and absent at reap. Broken controls: accept fixture path; skip nlink; skip GS2; skip exclusivity; wrong-case hash | Real valid/corrupt pair with actual generated argv, plus negatives: fixture-path substitution, same-name ABA swap, symlink, hardlink, planted-before-birth, extra-after-birth, swapped Ref, early deletion via canonical control hook, SIGKILL leftover cleaned only on dev/ino match. Foreign uid: model plus review only (not reproducible unattended). |
| Closure boundary: platform {P1,P2}, basisPlatform, rootLoads ⊆ {cache,candidate,rpath,unknown}, published {none,partial,full}; approved ⇒ full ∧ loads ⊆ {cache,candidate} ∧ platform equal at birth and settle. Broken controls: accept rpath/unknown; skip post-settle check; accept partial | Codec round-trip of exact bytes, order and multiplicity; refusals: dropped/reordered root load, changed mode, unknown/duplicate node, unreachable static, forged UUID, non-cache/@rpath target, root→libobjc-env, wrong member hash/count, changed link-map Ref, unknown field, oversize/depth, truncated/partial publication; cache-UUID/OS/arch change including A→B→A. |

Builder gates: three new Make targets as applicable, make test-formal-parity, make test-licenses, actual git diff --check; baseline has existing formal/license targets. Retain source/model/tool hashes and original command/exits. No duplicate whole Q2 integration run: its owner separately must demonstrate qualifying published lesson+corrupt-proof negative, four real jobs/control/foreground/resources/actual reap under make test-dream-mcp. Existing source-gap research remains referenced, not restated.

Ready dependencies: Root amends existing owner tasks; builder authenticates seventh-source KSBA and resolves actual generated/link/source review, while Q2 supplies the newly frozen private-candidate probe. Independent build/source work continues. No source/build/test/formal/runtime capability verdict is claimed by this contract.


Private-input readiness: a signed decoder/build checkpoint is preparation evidence; a permanently refusing entry is not a qualifying controller. Private admission checks the bound pre-test source/platform premise, never a future qualification or runtime approval. Actual command/proof/cost/settlement outcomes must reach the result through the same owned engine; normal Git refusal and runtime_approved=false remain.

Failed STOP terminal proof follows [[decisions/d93377961366050-dream-failed-stop-independent-terminal-proof]]; errno never acknowledges stop.
