---
type: decision
title: Pin seventh-source Libksba and canonical unapproved real-pair probe
date: 2026-11-06
derived_from:
- '[[tasks/2026-11-05-dream-ksba-private-probe-boundary]]'
- '[[interfaces/dream-proof-tool-qualification]]'
id: 01M6Q30HSTYA49MXGRT3N3Q4CK
updated: 2026-11-06T00:38:55.149Z
created: 2026-11-06T00:09:22.438Z
number: 93377961365875
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:cb1edd878f6c89ea3c4e5e8124f6939257a6a2b452fac34d8b2aa34a64bc6116
  frontmatter_digest: sha256:c8b257fec5577f0194418b7940db156fa4d5696f3abcf5cc445397b3d558073a
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: CA/l/hLogJyQGsdhff5jmqX2MBtx2iaD919gINw0eAujMY/tS0Xdx9BTdexTWbItUtIYJUyFjntPiI9zH60LAg==
  log: .bboxy/log/2026-11-06.jsonl#147851
---
# Add Libksba and a private unapproved candidate probe

Date2026-11-06 UTC (task-path date2026-11-05). Task [[tasks/2026-11-05-dream-ksba-private-probe-boundary]]. Refines only frozen inputs/private component handoff in [[interfaces/dream-proof-tool-qualification]] and Q2 capability wording. Root's immediate ruling is implemented; no source/build/probe execution or assignment here.

## Evidence, alternatives, choice

Builder snapshot2fdb65482cb7faa842765e563428d28b2be91fc7; only generated formal receipt dirty at00:03:25UTC. Retained logs/032.exit.json reports configure exit1, stdout43aa287fdf68a9586befd4ea06f4b20642083c4a4ceef9c430a8cf9876c669fd. Extracted GnuPG configure.ac:68–69 requires KSBA API1/version≥1.6.3; :841 checks it and :2013–2022 fails unconditionally. common/Makefile.am:96 includes ksba-io-support.c, whose :40 includes ksba.h. This is not merely an unused configure test.

Choose authenticated **Libksba1.8.1**, appended as source7; reject bypassing configure, borrowing installed bytes or widening source count generically. [Official release page](https://gnupg.org/download/index.html) lists1.8.1; [upstream configure](https://raw.githubusercontent.com/gpg/libksba/libksba-1.8.1/configure.ac):47–54 declares API1 and libgpg-error≥1.28.

- Archive https://gnupg.org/ftp/gcrypt/libksba/libksba-1.8.1.tar.bz2, expected SHA256 **c2f84393011827219ae117131dba8e7684c2bed0961eed11b0642c2acba440b5** ([primary build metadata](https://formulae.brew.sh/api/formula/libksba.json)).
- Signature same URL + .sig; fetched294bytes, SHA256 **af99f325f81eb395d740bf48dc8e89903b141c10f69244e46e4caa1caec0577e**. Issuer metadata lists Werner6DAA6E64A76D2840571B4902528897B826403ADA and NiibeAC8E115BF73E2D8D47FA9908E98E9B2D19C6C8BD, direct-primary signing; no new root. Existing official key bundle SHA6f57d0e17fefd2238bd037aebf03efa960641629dd61dd9d47f122a829f6e375 and upstream signature/key rules apply. Builder must verify the compressed archive signature and expected actual primary/subkey binding/time/revocation before extraction. Metadata/pins were checked; no cryptographic verification or successful build is claimed here.
- Keep first six sources/order/pins unchanged, append Libksba; source-lock and qualification sources require EXACTLY7, unchanged formats/32KiB receipt/128KiB combined prepared evidence. Build libgpg-error before Libksba, and Libksba before GnuPG. Use private dependency prefix; actual GnuPG m4/ksba.m4:112–123 supports preferred --with-libksba-prefix (not a guessed flag). Record generated configure/link evidence. Actual linkage determines staged runtime versus build-only closure; loaded/static-linked KSBA code, dependencies, initializers and lifetime assumptions remain reviewed. No “unused KSBA” configure patch is selected.
- Original six authenticated cache entries may be reused only after existing checks. Changed lock/source/build/patch/probe/profile invalidates every prior candidate/approval; a same version string does not restore it.

Frozen canonical probe SHA5493dbf0bda41ac33e4a27a9bc200f083e49501b0791d32cd255b731d6f82c33 git_capability:488–496 correctly returns81 unconditionally. Native/ledger controls are separate evidence. Choose one explicit private-candidate mode in the Q2-owned canonical probe for **conditional measurement of a source-built real pair**. Normal git mode stays unavailable; reject a generic “true” bypass or a builder fork/controller. Exact private input/result and validation/ordering are in the builder ICD's Private component subsection, one owner.

## Owner obligations and guarantee limits

Q2owner01M6PBF0EX2R5RGQAXNQTR96Y0 implements that private path using the SAME existing per-slot/clock/stop/roster/CPU/debt/foreground/memory/reap machinery. Builder01M6Q0FHRZV6MQA8PBPXG408N7 appends/pins/authenticates/builds KSBA and produces immutable pre-test source/build/config/closure/command/review/tool bindings. Builder consumes Root's later frozen Q2 source/hash, never edits/forks the canonical probe. Root alone amends live tasks and later admits runtime authority.

Pre-test input is bound to exact scoped candidate bytes and reviewed-source **test premise**; it never contains its own/future result/qualification/receipt hash. Expected input digest gives integrity, not authority. No descriptor flag or successful component sample supplies runtime approval. Output always runtime_approved=false, separates conditional source-review premise from actual measured outcomes, and explicitly disclaims whole-command absence-of-unseen-event proof. Raw result stays Q2-owned/unaltered in builder evidence. Existing approved-provisioner/receipt materialization and hash-cycle avoidance remain.

Keep all1ms ordinary/native2msCPU25mswall/3msinclusive/9x debt/256 attempts/one-group2processes1thread/30s/four-job/heap/foreground/control/reap/bootstrap/source/proof laws. No public worker DTO/protocol/IR/control-version change, new approval registry, controller or memory entitlement. Private developer qualification is not a live Dream/lesson path.

## Required causal evidence

| Guard / finite-model extension | Real bounded unattended witness |
|---|---|
| Source count exactly7 and pinned KSBA root/archive/signature; drop count/root/config/closure guard | Missing/substituted/stale/same-version KSBA or old six-source lock refuses; actual source authentication/configure/build/link evidence before candidate readiness. |
| Separate normal/private mode and input hash/identity/source-review binding | Normal installed git mode remains81; missing/forged/stale/wrong-root descriptor fails before helper creation; real source-built private candidate executes only the fixed public command set. No unapproved output is accepted by runtime/Q2. |
| State: scoped input/effect/incarnation A1/B2/A3, conditionalPremise, mode, phase, slots/coverage/debt/settlement, runtimeApproved=false | Duplicate/restart/foreign nonce/changed artifacts and clock/deadline/foreground/cancel; no new original effect or debt loss, no hidden slot closure. |
| Drop per-slot/current-exec/CPU/controller/pipe/debt/control/heap/reap guard | Existing canonical causal controls plus actual Git→GPG/corrupt proof under unchanged limits; observed metrics distinguish failure/unknown from zero. No source-lifetime theorem from census. |

Extend existing source-bound formal models/controls, native/TLC parity and builder/Q2 Make gates; no rerun here. Current failure is not converted to success. Source-built GPG, private component positive/negatives, complete source/closure review, Root runtime admission and Q2 qualifying publication/four-job evidence remain unproven until actual implementation checks.

## Completion: exact inner pre-test tuple

The outer FilePins alone left arbitrary inner bytes and ambiguous arm64/aarch64 labels. Choose **one shared strict private-pretest-basis/1 manifest with six fixed projections**, rather than six independently maintained documents. [[interfaces/dream-proof-tool-qualification#strict-pre-test-basis-one-manifest-six-fixed-projections]] now owns exact Build/Auth/Patch/Platform/Closure/Config/Commands/Fixture/Review fields, bounds, canonical hashing and pre-birth equality/coverage/currentness checks. The six pins must be identical; parse once. Fixed command/env arrays and eight source-review coverage obligations are explicit. Existing qualification/source-lock records remain provenance, not a second approval store.

Canonical architecture is aarch64; ONLY raw Darwin arm64 is normalized at capture, with exact thin ARM64 Mach-O identity and recorded compiler/SDK/OS/cache/library bindings. Outer-only/unknown/dropped coverage, wrong projection, stale same-version/foreign arch/root/config/fixture/probe inputs remain review_unbound before helper birth. Source review/compiler/platform completeness is an explicitly conditional trusted premise; a status string, hashed prose or private result never grants runtime authority or proves unseen-event absence.

No self/future/current-probe-source hash cycle: scoped host supplies expected frozen source/exe/input bindings after source freeze; projections hash independently, then review/basis/outer/result/qualification/proposal/Root-profile/receipt follow in that order. Descriptor16KiB/basis48KiB/source-lock32KiB/raw-result64KiB are individual bounds within the SAME128KiB combined evidence admission; receipt32KiB and all CPU/native/debt/control/heap/closure/reap laws remain. No public DTO/wire/version change.

[[reports/2026-11-05-dream-private-pretest-inner-schema#complete-typed-worked-input]] contains a complete explicitly fictional typed example and in-memory canonical byte arithmetic only, not a candidate/authentication/probe success. Builder emits actual source/build/link/config/fixture tuple; Q2 implements the one strict canonical decoder and causal no-birth negatives; Root freezes/amends live consumption. Normal git81/private runtime_approved=false remains. This closes the schema hole, without approving the installed tools or executing new source/probes.
