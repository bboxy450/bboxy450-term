---
type: research
title: Q2 proof-tool source/build lifetime basis and installed provenance gap
derived_from:
- '[[tasks/2026-11-05-dream-q2-tool-lifetime-basis]]'
- '[[decisions/d93377961365489-dream-q2-member-window-accounting]]'
id: 01M6PXSFB10CV5T851FYFTCK98
updated: 2026-11-05T22:38:07.635Z
created: 2026-11-05T22:38:07.635Z
number: 93377961365652
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:114b9842f1767bd61295de50b5dd6023342f474b605032729cf153174d4fe960
  frontmatter_digest: sha256:077620e9a7af5769ebfbae862288f8a8536d660e5a44c43fb1f1c45e86e1dc66
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: k9X09m3rP0Qa1eRVqYcDFcFG8yq8tV7jPA5ARlAjZveTIKJa49zEiT0j3GsnW/grFRT6LZFkLsLd//jaZpziAg==
  log: .bboxy/log/2026-11-05.jsonl#145636
---
# Q2 proof-tool lifetime basis

Retrieved 2026-11-05. Task [[tasks/2026-11-05-dream-q2-tool-lifetime-basis]]. Read-only investigation; no candidate executable, build, test, provisioning probe or service was run. The product rule measures bounded attributable background execution, foreground responsiveness and complete settlement. It cannot measure universal absence of arbitrary native work by sampling.

**Finding:** the current Xcode Git/Homebrew GPG pair is **verification_unproven** for whole-command two-process/one-runnable-thread capability. Available metadata identifies it; neither metadata nor the existing sampled probe supplies a reviewed source-to-build lifetime argument. Do not retry qualifying work or expose a lesson under these feature strings. Existing fixed-slot/bootstrap/negative-control work can continue.

## Installed evidence

Metadata read at 22:34 UTC in /Users/feral/mydev/bboxy450-harness-dream-read-q2, HEAD3d9fa84f196ea9b1b643939e7ae035a7dd0a6c9c plus active edits.

| Item | Observed bytes / meaning |
|---|---|
| /var/db/xcode_select_link | Resolves /Applications/Xcode.app/Contents/Developer. Fixture provision_command env_clear at tests/dream_read.rs:167–174 removes DEVELOPER_DIR; :284–318 uses xcrun --find then canonicalizes Git. I did not rerun xcrun; this is the selected-directory metadata basis, not a new observed launch. |
| Xcode Developer/usr/bin/git | Embedded string 2.54.0 (Apple Git-157); man page likewise Git2.54.0. SHA256 **9a1c8fc68dc75e1b3c0cd8e5ad9d13ac9bc92cb53c9578b3cff4beaf2e9b1e70**,3,837,392bytes. Public code-signature display identifies com.apple.git/Apple Software Signing, team59GAB85EFG. This is metadata, not an independent source/build/lifetime verification. |
| CLT /Library/Developer/CommandLineTools/usr/bin/git | Different SHA256 **a73bf622a2e470d5d57a4b1d5aef1e8680e67278018d4858a2f93825b7d595c7** at same size. Same-looking installs cannot be substituted by version alone. |
| /opt/homebrew/bin/gpg | Resolves /opt/homebrew/Cellar/gnupg/2.5.24_1/bin/gpg; SHA256 **21cb9adcad96b7be404014ff6e9fb837160a2135455c217dec9efe196cf0661a**,1,110,064bytes. |
| GPG INSTALL_RECEIPT.json | SHA256 **f6314d786319be912d2a52f82909c770dc8c069db6c2bb875da6ca536d1cd198**; arm64/clang/built_as_bottle/poured_from_bottle, stable2.5.24, no tap_git_head; named runtime dependencies. Identifies package provenance claims, not compiled config, loaded closure or absence of descendants/threads. |
| GPG installed .brew/gnupg.rb | SHA256 **cb39cf745dea2547a67bb6e78175a6b29c0faa6f4c7ea5f0ad367a7e01f44d00**. Tar URL gnupg-2.5.24, SHA256 bf149d01a2b9fcc0e4589b8ae8697d3d5c557ea48ed95a3fa55dd3b1187e6039, revision1, two backports42386bcf/5523cec6, configure recipe and dependency declarations. Not an exact local compilation/link receipt. |
| Platform | Xcode27.0 build25183.107.5; macOS27.0.1 build26A434; SDK macosx27.0. SDK event.h:355–362 explicitly says NOTE_TRACK/NOTE_TRACKERR/NOTE_CHILD unsupported since10.5. Header SHA256 **435ed8fe99ae70c19404c3da14be6e2d217686ac256aed3b5df721b353dbb350**. No NOTE_TRACK design selected. |

otool metadata: Git loads CoreServices/CoreFoundation/libSystem/zlib/iconv; GPG additionally loads gettext/libintl, sqlite, libgcrypt, readline, libassuan, nPth and libgpg-error. A package dependency list is not the actual recursive loaded-image closure or reachable call graph. Individual nPth1.8/libgcrypt1.12.4/libassuan3.0.2/libgpg-error1.61 install receipts were read; all are bottle/clang claims without an exact source-to-loaded-object lifetime proof.

## Primary-source mechanisms and limits

All URLs below were actually fetched today; source tags are upstream comparisons, **not proofs that vendor binaries equal them**. No comprehensive call-graph audit is claimed.

| Source | Supported claim / consequence |
|---|---|
| [Apple Git.plist](https://raw.githubusercontent.com/apple-oss-distributions/Git/main/Git.plist):5–24 | Fetched public main declares Git2.50.1/import2025-07-24/commit d82adb61ba2fd11d8f2587fca1b6bd7925ce4044 and Apple modifications. It does not bind the observed Apple Git-157/2.54.0 binary. Requested Git-157 source paths returned404; not proof that no matching source exists elsewhere. |
| [Git2.54.0 gpg-interface.c](https://raw.githubusercontent.com/git/git/v2.54.0/gpg-interface.c):335–368 | Selected OpenPGP signature verification constructs the GPG command, pipes payload/output and parses status. This is the required real Git→GPG path, not authority to omit signature verification or substitute fixture output. |
| [Git2.54.0 run-command.c](https://raw.githubusercontent.com/git/git/v2.54.0/run-command.c):731–833,1126–1180 | Generic launch has fork/exec and ENOEXEC shell fallback; asynchronous work uses pthread_create, or fork under NO_PTHREADS. Therefore NO_PTHREADS alone does not prove the two-process ceiling. The exact approved commands must exclude/review every reachable extra async/helper path, including config-dependent hooks, pager, fsmonitor, lazy fetch and error paths. |
| [Git Darwin build defaults](https://raw.githubusercontent.com/git/git/v2.54.0/config.mak.uname):156–164 | Defaults may include fsmonitor/thread infrastructure and CoreServices. Flags/config and linked libraries matter; “readonly Git” is not a build capability. |
| [GnuPG2.5.24 gpg.c](https://raw.githubusercontent.com/gpg/gnupg/gnupg-2.5.24/g10/gpg.c):3764–3792,3834–3837 | GPG processes common component configuration and initializes nPth. Neither fact proves that a second thread actually runs in this subset; neither permits assuming no thread/daemon path exists. |
| [nPth1.8 source](https://raw.githubusercontent.com/gpg/npth/npth-1.8/src/npth.c):245–276,346–368 | npth_init initializes synchronization; npth_create calls pthread_create. Init alone is not evidence of an extra thread. Reachability and transitive helpers must be qualified. |
| [GnuPG call-keyboxd.c](https://raw.githubusercontent.com/gpg/gnupg/gnupg-2.5.24/g10/call-keyboxd.c):129–216; [call-agent.c](https://raw.githubusercontent.com/gpg/gnupg/gnupg-2.5.24/g10/call-agent.c):234–242 | Optional keyboxd/agent service paths exist, with autostart flags. Fixed private public-only configuration must exclude them, not merely sample no child once. |
| [GPG options](https://www.gnupg.org/documentation/manuals/gnupg/GPG-Configuration-Options.html):202–265,535–541 | common.conf can select keyboxd; no-autostart suppresses starting specified services, not every native process/thread possibility. Effective configuration includes common.conf, fixed public keyring and environment, including implicit Git verification. No operator keyring or daemon connection is authorized. |
| [GnuPG configure source](https://raw.githubusercontent.com/gpg/gnupg/gnupg-2.5.24/configure.ac) / [Homebrew API](https://formulae.brew.sh/api/formula/gnupg.json) | Public release/build options and current source/bottle/patch hashes are available. The current API formula checksum differs from installed .brew bytes; no bottle was downloaded or installed-byte equivalence verified. These are provenance leads, not local audit verdicts. |

Apple stopped identity/resource interfaces remain the runtime mechanism under65126/65489: suspended native birth, current audit-task+PID/start identity, bounded stop/roster observations, nonduplicated CPU/roll-up, real exit. They corroborate an approved finite tool basis; they cannot enumerate transient events that occurred entirely between observations. Public XNU source and SDK declarations are not an all-times scheduler/thread theorem.

## Concrete disposition

No sufficient lifetime basis for the installed pair was established. Choose a **private source-built, source-qualified real Git/GPG pair** as the smallest new product prerequisite for Q2 qualifying publication; no new tracer/controller/entitlement. Begin qualification from upstream Git2.54.0 and GnuPG2.5.24 plus explicitly selected backports/dependencies, authenticate immutable source pins before a future build, retain generated configs/compiler/link closure and signed artifact bindings. This task authorizes none of that execution.

The qualification must cover ALL65126 commands: selected Git root/object/path/ancestry/signature reads, public GPG show/import and Git's implicit verify. Prove as an explicitly trusted source/build assumption that those reachable paths spawn no helpers except one direct GPG, add no runnable helper thread and do not escape group/custody; include malformed public input, errors and cleanup. Preserve exact proof semantics/signatures. Audit startup/library constructors and dynamic loading as well as application code; name remaining OS/libSystem trust assumptions rather than claiming an unprivileged sample proved them. If the source-qualified build cannot satisfy the fixed subset/bounds, it remains unsupported; do not inflate budgets or silently narrow proof.

Source audit is a finite trusted basis, not a theorem over arbitrary binaries, compromised libraries, kernels or scheduling. Runtime bounds/clocks/control/accounting/settlement are independently checked and may still refuse a source-qualified tool. A future build gate must explicitly reject unsupported reachable spawn/thread/service branches; simply disabling pthreads, keyboxd installation or setting feature strings is insufficient.

Qualification record contents and no-wire-change binding, currentness, model assumptions/causal witnesses are fixed in the companion decision. There is no universal inventory daemon, no wait for operator and no newly available lesson. Q1 observation work and Q2's independent fixed-slot/causal/bootstrap/regression work remain useful without this prerequisite.

## Local source cutoff

At22:34 source hashes: resolve.rs **9e244932341d7de5498180bf0ecd0073dcf19d2a2f6150692b7f17ab115ecf8d**, c1.rs **88d8ab9ef8b467499da2fb336a8c9a07d6de15d1f01bbfd0d90d9ccd199bbecf**, tests/dream_read.rs **d320f8cf954ecd254d448941124ae72b30be1d7daf5e0ada0323376b7df9fcd7**, probe.c **c0b0ba11bfa0561b7f4d09d5f8c4db12e4a6d3b4ad474266bbf3a276eea1b9b3**. Source is active/dirty, not a shipped claim.

tests/dream_read.rs:515–520 mints feature claims from128KiB sampled evidence into32KiB receipt. resolve.rs:250–334 checks strict receipt custody/source/OS/probe/hash shape and feature membership; this supplies identity/binding checks but no newly established lifetime proof. probe.c:620's zero-grandchild census cannot rule out a fast exited child. No observed successful capability/performance/formal/process verdict is claimed.
