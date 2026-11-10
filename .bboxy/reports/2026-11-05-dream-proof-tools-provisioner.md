---
type: report
title: 'Dream proof tools: authenticated partial build; source closure and canonical Git probe blocked'
author: seat:01M6Q0FHRZV6MQA8PBPXG408N7
created: 2026-11-06T00:00:03.058Z
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
commits:
- 6eac12cf7c33bbc4e06fa797aa88aa76fac1a053
- 2fdb65482cb7faa842765e563428d28b2be91fc7
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M6Q2FF012E2AV49RNHKNMMES
updated: 2026-11-06T00:00:03.058Z
number: 93377961365790
created_by: seat:01M6Q0FHRZV6MQA8PBPXG408N7
updated_by: seat:01M6Q0FHRZV6MQA8PBPXG408N7
authorship:
  signer: seat:01M6Q0FHRZV6MQA8PBPXG408N7
  kind: seat
  public_key: PcKFbgDesoAlYaICRDceGYv79hGHd1L0ZA+qVT0MKXU=
  process_generation: '1'
provenance:
  digest: sha256:0bca4b647a2a6ecc31b212b738f44754565c53e8c3e288a48ae244649a4b234f
  frontmatter_digest: sha256:8d48e1806cc77b765dc22438a8feb210229b536d2f6f4c65a11cd3ac1c4ab9d0
  signer: seat:01M6Q0FHRZV6MQA8PBPXG408N7
  sig: khoxZtf8lFxZEmX6WiZh9E+4LJ+friAMBD3Ygk4uOMTq9qj6WUlND7TL4YCFHo3wTa+NXl3zApLzWxTo0+yTDA==
  log: .bboxy/log/2026-11-06.jsonl#146912
---
# Blocked; no qualified pair or runtime admission

Task [[tasks/2026-11-05-dream-proof-tools-provisioner]]; ICD [[interfaces/dream-proof-tool-qualification]] digest5f9dc6efafc0f88686faf90a5d5b4403f4c8695b881a45c1bf71995179169323. Read completely with65654/65126/65489. This is a partial implementation/evidence checkpoint. The task remains claimed, not completed; no successful qualification or acceptance is asserted.

## Exact decisions required

1. Actual authenticated GnuPG2.5.24 configure with --disable-gpgsm exits1 requiring Libksba>=1.6.3/API1. configure.ac:2013 unconditionally sets die=yes on missing KSBA, while the ICD permits exactly six sources and excludes Libksba. Root must amend ICD/task to pin that additional source, or explicitly authorize a narrowly reviewed configure change for the disabled component. No extra dependency, Homebrew binary library or configure waiver was used. Current run: target/dream-proof-tools/runs/01M6Q1RFBC70AVGR9WCZ93BA58/logs/032.{command.json,stdout,stderr,exit.json}; stdout SHA43aa287fdf68a9586befd4ea06f4b20642083c4a4ceef9c430a8cf9876c669fd, stderr SHA53d04375df65524066063de8361791d9653d9ee7254294a24833e44cf93d2d0e.
2. Frozen canonical proof-helper-probe.c:488–496 unconditionally emits verification_unproven and returns81 for Git; it cannot exercise a private unapproved real pair. Root/Q2 must freeze a suitable canonical interface. No probe edit/fork/replacement or competing controller/schema was made.

Root received blocker message01M6Q22CV3Y614SGVGM53ZKVVM in the delegation thread; Q2 received the essential frozen source/probe handoff01M6Q22CVQ8NXGZGEQD6SZNFFX. Receipt tools showed native-queued/harness acknowledgement, explicitly not verified model consumption. Independent checks completed without waiting for an ACK.

## Changes and signed source

Own tree /Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools, base d06e696f99472ed754e84353baf11a2974589d6d. git log -2 reports G signatures and Boxy <bboxy450@gmail.com> for both listed commits; both carry the Report trailer. No push/merge/release/global install/configuration, Cargo.lock/dependency change, operator key export/keygen, GUI/physical step, or live Q2/product source edit occurred.

- Makefile:10 adds the three targets with one Python recipe owner; README documents the unsupported boundary.
- scripts/dream-proof-tools.py:185 authenticates exact archive/key/signature bytes with the independently hash-pinned verifier, fresh public-only HOME/GNUPGHOME, env_clear equivalent explicit environment, no-options/batch/no-tty/no-autostart and no automatic key retrieval/import/location. Actual VALIDSIG primary/subkey/time and native revocation/expiry/error statuses decide. Git signs the bounded decompressed tar; the other five sign archive bytes.
- :225,266,306 implement fixed HTTPS fetch/cache, finite table-first safe extraction, fresh crypto before extraction, private build IDs and exact preimage/postimage checking. :364,416,442 retain real compiler/SDK/config/command/PID/exit/log inputs and build four private static libraries plus guarded Git with two niced jobs.
- :130,461,558,580 provide no-replace/fsync file publication, exact six-source lock and strict proposal/qualification/reference/budget helpers. They do not assemble/publish a completed candidate. Whole-build restart/currentness/cache and receipt-last integration remain incomplete.
- tests/fixtures/dream-proof-tools retains unchanged public certificates/signatures, sources.lock.json, five credited guard patches, all pre/postimage hashes and license-map.json. ATTRIBUTION.md preserves upstream GPL/LGPL terms, not a project relicensing. Zero-context patches now count LF rather than form-feed separators; a real patch-process regression checks every exact postimage.
- tests/dream_proof_tools.rs invokes the same recipe owner's six offline pin/custody/extraction/publication/patch regressions. tests/formal/dream_proof_tools.rs plus existing inventories add four finite families and18 causal controls.

Source is committed. Final git status additionally shows only M .bboxy/runs/formal-native.json, generated by the existing canonical formal target; it is retained and not added/force-added. No peer/admitted prefix cleanup occurred.

## Authentication and partial build

make fetch-dream-proof-tools exits0: six exact releases cryptographically verified. Git signer E1F036B1FEE7221FC778ECEFB0B5E88696AFE6CB binds primary96E07AF25771955980DAD10020D04E5A713660A7; GnuPG/library signatures validate under the specified release roots. Original cache and fresh-build raw statuses remain private. Source lock SHAd2567b05d644c1a1ce19262bc58530239eff7b8368e3a23b38708214c82882b6; patch manifest SHA64acf86967660a4244be343aded6916d1acf9c5f38670c312bf1f8a071fd7c06.

Actual bootstrap GPG matched authorized SHA21cb9adcad96b7be404014ff6e9fb837160a2135455c217dec9efe196cf0661a. Ordinary public import initially returned an absent-agent error despite no-autostart; its real failure is retained. Corrected public-only import-export conversion creates an owned legacy public keyring without agent startup. No nonzero authentication was relabelled success. This independent verifier/compiler/platform assumption is not installed-GPG runtime qualification.

Current build01M6Q1RFBC70AVGR9WCZ93BA58 retains argv/env/cwd/PID/stdout/stderr/exit/hashes. Four library configure/build/install sequences exit0; Git compile027, image inspections028/029 and version030 exit0; GnuPG configure032 exits1. Earlier SDK-header and aarch64 assembler failures remain in original runs. Explicit SDK flags and upstream portable Libgcrypt (--disable-asm/--disable-jent-support) fixed those build issues without replacing real crypto.

Compiler clang SHA1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9; SDK27.0 settings SHA7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb; OS27.0.0/arm64/build26A434. Generated Libgcrypt config selects GETENTROPY, not RNDUNIX; HMAC constructor/jitterentropy are disabled. Libgpg-error PTHREAD_IN_USE_DETECTION_HARD is undefined. Private guards refuse Git async/generic/shell fallback, service/library spawn and nPth creation seams. This is incomplete finite review, not universal lifetime proof.

Partial prefix target/dream-proof-tools/candidates/01M6Q1RFBC70AVGR9WCZ93BA58, inode178388034, mode0700: only staged executable is real Git2.54.0, SHAaa98b52a52b124e61f9a50118a636648962ead1de91cb832fa05c81988a67bf1, inode178405740/mode0700. Preserved license files exist. No GPG, qualification.json, receipt.candidate.json, receipt-derived candidate ID, approved flag, Root grant or runtime profile exists. otool lists CoreServices and system zlib/iconv/libSystem; recursive actual loaded closure/constructors/config/error/cleanup remain unqualified. Version/grep/symbol/link lists alone are not proof. Compiler/linker/platform fidelity and absence of arbitrary native compromise remain assumptions.

## Unchanged canonical component

Source exact39130bytes/SHA5493dbf0bda41ac33e4a27a9bc200f083e49501b0791d32cd255b731d6f82c33; four-file frozen consumer byte snapshot independently reproduced55aabc5e0efd081fe050db08a40e955ddb27ff5d8a442df1c6feb911ce8160af using required domain/NUL/name/u64LE lengths. No compiled current-Q2/clean-commit claim.

Run target/dream-proof-tools/runs/01M6Q1E18MP38G2ZHHZXEFFPPR retains source/exe/compiler/raw outputs. Exe SHAd39a9c4f1d64c9fb4c9a4fd0efe600d3f1dd142db1f2d52efa7fba388a9136a9. Ledger-controls exits0 (18 controls); native exits0 (actual reap, two members, one thread, stopped birth, audit-token/retired-version controls and member windows). Raw005.stdout SHA066188d0e5c5e428b06696349c8c73eb11b3f8354429602d2361bf46f5069a74; stderr SHAcbd906a83a23a36af23b0b0d52e09066945d891c2de008d49db5769fccf97032.

Birth847125ns CPU/1856000ns wall; W299000ns/A75500ns/C269750ns/Q568750ns; post876500ns; group RSS3375104bytes; child wait4 CPU14648000ns, leader roll-up18203000ns. These cover the canonical private C program only. No threshold waiver or native timing retry. Later checks revalidate exact retained bytes/compiler/SDK/OS and reuse that original outcome. Git mode was not invoked without a pair; its refusal is source evidence. No real Git→GPG or full-product certification follows.

## Actual verification

| Command | Exit/evidence |
|---|---|
| Exact task frontmatter command | 2: fetch0, builder Python1/make2 after configure1; later && commands not executed |
| make test-dream-proof-tools separately | 2: six regressions pass, original native/ledger results retained; explicit real-pair unsupported refusal |
| make test-formal-parity separately | 0:283 models/77 contracts/one crate, complete native/TLC state/edge/verdict parity; all22 new models/controls included |
| make test-licenses | 0: headers, suite policy,10 negative controls |
| nice -n 10 cargo test --offline --locked --jobs 2 --test dream_proof_tools -- --nocapture | 0: current Rust process witness invokes six Python regressions with cache |
| git diff --check | 0 |

Source-bound formal receipt verification/evidence/9478109d8abd9b49a7aa0a13837cc85a47a6ee65f8455b1292c502fdb1370d31/evidence.json has matching SHA; source2fdb65482cb7faa842765e563428d28b2be91fc7, source-dirty=false. Source/model/tool hashes and pinned Java21.0.13+9/TLC1.7.4 are retained beyond cleanup. Model source SHA224951be88782e30029725c85687b39177dd0e829a75b995e79e550277214e08. Contracts list bounds, only local-settlement weak fairness, IO/compiler/platform assumptions and no Rust/native refinement.

Final private log hashes (target/dream-proof-tools/final-*.log): verify-current3013bb0ffd5c24b8f2952e9df890e57b19e5e155f0c65c21b95b146b3db3eef0; test9b82b1793608a25910bfbc5b5d44310b52269533985c423415c325c2d372da58; formal3cc789ef9f5c5e47adcff8e7e53ce019cca225c29894628424f75925b16d0a4d; licensescb60731c8d0ccbaab1f2c5ad4cc0101ca142a1b2a86ad458697bceaf843ef20c; rust-unit919627521b3a62916bbb7a32bf1fdf77181a268bc1b6b698e93f0caa072e4c29.

## Unmet acceptance

Complete GPG build; finite all65126 command/signature/input/error/startup/constructor/library/platform review; real public-only presigned positives and invalid-proof negatives; actually reachable process/thread/service mutant execution losing SOURCE qualification; qualification/blob/receipt-last publication and full restart/currentness/cache integration; actual config/library/OS/compiler/receipt/forged/self/partial/stale admission negatives; canonical real-pair result; Root admission and Q2 current runtime receipt. Canonical ledger/model controls do not substitute for missing implementation witnesses. Qualifying publication, corrupt proof, four jobs/control/foreground/heap/debt/30s/reap/full exporter remain Q2-owned and untested here. Installed tools remain verification_unproven. No available lesson or arbitrary-native theorem is claimed.
