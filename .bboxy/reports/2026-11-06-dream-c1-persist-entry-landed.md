---
type: report
title: Dream C1 PERSIST and exact-file admission source landing with failed runtime evidence
task: '#93377961364376'
author: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
created: 2026-11-06T19:39:56.630Z
commits:
- 59dd7f0092618a0546806c751fd7d79aa1637299
- fa4e021be687f2aafc93073aa79a74b88b0dc34e
- 68c8f23cfc037f30fd6faa651e26dfcc193c109f
- 4015256d56a153fd7af2c3a12acf71f0207f1b17
- c7637ddaacb83db3aac125407277f670334c1598
- 7c466707e486a9a0244e64ba92584f89ae733e07
- 151e11103beb361ee3a16be5ca8f0b6b2a0f0917
- 0c22507bd9999dd3771f1db57f6081c8aeb0bbb3
- 2d2d3c53f8c688b983b9cb4bd213af9c5ff93b52
- 4d0295067402477029da38d9949885972bb67168
- 11a75ae415118bd41a2de56afc87942179e2fd8f
- 8e1c750ca81247fa3e1d378ba71390c3fda49db0
- 77e7a49a476cf9294bd034a7832920b018c20710
- c9673d4b531fbb11128fc87f86bdd9ac9e73fa32
- 2b6868ca4eb66c91ec0e2c5d33fdd0e2345156d2
- c73bca69f7d0d6edffe1a02ba5ef09dd43bf2f74
- cf43c92bb2924ffa465f6f3745c12941f680b863
- cba756bda37af78bce38fd7713fdfac6bb54149a
- f7e3ba2c7978599770dde7104c93cfcdea513194
- 8f1b16b892f8e45eafad81cb1ea22606b2ef3782
- f9352701ba567e9e4a528bd241018535fe7bce14
verify:
  command: make test-dream-c1 && make test-formal && make test-formal-contracts
  exit_code: 2
id: 01M6S5ZXGRGP50SJM0NV59ZY2W
updated: 2026-11-06T19:39:56.630Z
number: 93377961366096
created_by: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
updated_by: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
authorship:
  signer: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
  kind: seat
  public_key: vTy8kxzALPVGVJxPquLZelvl5D6TNzgsirkLE/DA5ek=
  process_generation: '1'
provenance:
  digest: sha256:64901014a9f3af7abb04d7f20398600a53d07854126721b5f92b6eeb8435f987
  frontmatter_digest: sha256:cb8648fe18e59b8590358a8f5af958c2ad478efa3c35a2dc9d6fd8027cb992da
  signer: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
  sig: 8V7rjWLfY9e2vFWzyqquaEymg5VJXiseeWbPcZZUvj+Pq3SbFmePgmKdTNrzhhMsSx7ZiBfpRhioYK1hqmrvAA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149775
  event:
    id: 01M6S5ZYE5AS2XA71W5833WC4W
    sequence: 149775
    hash: 058e3c8fbc505e9385c994e1d3c39b242938527ca0eb71f81c91b49acf6ed44a
---
Source-only landing record for task #93377961364376 (tasks/2026-11-05-dream-c1-consumer). Oracle message 01M6S3848WM40WWQ3MFXN1189Y authorizes landing signed source through make land, keeping cold latency and a failed ordinary worker run as disclosed limitations. It does not authorize installing or approving the runtime tuple. This report records the pre-landing verified source; the containing source/knowledge commits and final landing command log identify the final cut. No successful landing is asserted before that command completes.

Source checkpoint f9352701ba567e9e4a528bd241018535fe7bce14, branch feat/dream-term-c1, worktree /Users/feral/mydev/bboxy450-term-dream-c1. All inherited signed source and predecessor changes are preserved. Rebase onto term origin/main85aa28c2 preserved approved harness2ce4359; Cargo.lock unchanged. Every code commit uses the approved Boxy signing identity.

Changes and mechanisms:
- crates/bbt-mux/src/registry/dream_c1.rs:214,285,419 — private C1 catalogue uses PERSIST, synchronous FULL, busy0 and atomic ready/op/run/SourceKey/cursor persistence. Supported existing-WAL transition remains SQLite-owned, preserving rows/inode; no manual WAL/SHM deletion. All SQLite stays ordinary1ms work with9x rest. Focused PERSIST COMMIT remained under1ms, so conditional native durable_commit was NOT introduced.
- registry/dream_c1.rs:1923 and logic/dream_c1.rs — cache-miss Status/Cancel probes only its own host-bound readonly header before cold public preparation. Fixed header prepare/step/finalize has one strict1ms unit, separate open/busy/close debt. Busy/missing refuses early; success still requires full current public actor/context authority. Cached controls retain independent prepared-session admission and zero device/signing custody.
- identity/status.rs and public proof path — staged existing prepare_session_cooperative/public_connection_cooperative boundaries retain original grant/root/seat/provider/inode/head/revoke/ABA negatives, fail closed, and yield/pay between owned ordinary units.
- scripts/test-dream-c1.py, Makefile, logic/dream_c1.rs::runner_provision and term/tests/dream_c1.rs — exact-file first-launch provisioning under decision66076: one cleared-environment --version exec, durable O_EXCL/no-follow started claim before exec with file/directory fsync, 2s wall/8KiB independent output limits, exact PID reap, persisted failure/unknown/no retry and binding to canonical path/device/inode/hash/mtime/ctime. Missing/foreign/nonzero/late/unknown admission is refused before Dream birth/Start. Existing successful receipt bytes/inode are retained; private copies use canonical make admit-dream-c1-file for their own file. Public protocol/profile versions and original250ms Hello clock are unchanged.
- The private admission reader alone permits signed signal exit statuses; public JSON retains nonnegative integer defaults. Controls exercise per-image once, replacement, nonzero, timeout, output overflow and actual reap with private non-Dream executables.
- tests/formal_contracts/dream_c1.rs and verification/{contracts,corpus,coverage}.toml — catalogue PERSIST/header-refusal abstraction plus TermDreamC1FirstLaunchAdmission and its deliberately broken Start-without-admission control. Finite admission must precede Start on the current immutable file; failure/replacement/unknown cannot start or retry. Parent coverage includes all five pinned HarnessDreamC1BootstrapEntry models. Existing100000-state cap, state/labelled-edge/named-verdict parity, all prior controls and cost/rest/custody constraints remain.
- f7e3ba2c adds fixed-size cfg(test) phase samples and an explicitly ignored real MCP cold/warm diagnostic. It checks routing/custody, not latency acceptance.
- Existing integrated C1 source retains own-seat authoritative closed nonfork IR, source-free permit/bootstrap, bounded independent owners, immutable descriptors/results, started fences/dedup, foreground pause and actual process retirement. Root attribution/companion notices preserve the project origin and respectful-slop policy. None of these source mechanisms substitutes for a successful joined runtime witness.

Actual verification and failures:
- PERSIST focused7 tests:6passed/1failed (held-control1013401us), exit100, /tmp/dream-c1-persist-focused.log. No COMMIT overrun; cold cumulativeCPU6537042ns, maxordinary437833ns, maxcleanup113416ns. WAL migration/crash/reopen/concurrent-original-op passed.
- First header correction3 tests2passed/1failed324895us, exit100, /tmp/dream-c1-persist-header-focused.log. Fixed bounded-header3/3 exit0, /tmp/dream-c1-persist-fixed-header-focused.log: four held-catalogue MCP/GUI calls max196524us, zero extra signing/custody calls, foreground callback and structured-only success/page/refusal valid. Four calls are not200-call performance proof.
- make test-boundaries exit0, /tmp/dream-c1-persist-test-boundaries.log, sourcec73bca69 before later tuple/admission changes. It covers real private process/login/native-signing/lifecycle witnesses; no operator interaction. Not claimed as a final-source rerun.
- make test-dream-c1-profile initially exit2 on private negative signal-status parsing; corrected gate exit0, /tmp/dream-c1-admission-profile-fixed.log. Failed log retained.
- Optimized native_provision_ filter:7/7 exit0, /tmp/dream-c1-admission-native-provision.log. Exact command cargo nextest run --locked --cargo-profile dream-c1-fixture -p bboxy450-term-mux --lib -E 'test(native_provision_)' --no-capture with existing cache env/one test thread. Fake validator images are explicitly not actual approved-worker success.
- Native finite admission contract witness exit0, /tmp/dream-c1-admission-formal-native.log (default unoptimized+debuginfo; no timing claim).
- make check-rich-host exit0, /tmp/dream-c1-admission-check-rich-host.log, strict all-target lint on8f1b16b; f935 differs only in Python fsync and Make help.
- make test-formal FORMAL_REUSE=1 exit0 on cleanf935:661models/34crates/246contracts, full state/labelled-edge/named-verdict parity, canonical unchanged-model reuse; /tmp/dream-c1-66076-formal.log. Immutable current-source receipt03538af1d44168dcdeeffa7f6d17597a0fc56e030be441c642cec39decd098cb at /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/03538af1d44168dcdeeffa7f6d17597a0fc56e030be441c642cec39decd098cb/evidence.json.
- make test-formal-contracts exit0:43tests plus strict lint, /tmp/dream-c1-66076-formal-contracts.log.
- make test-licenses and make check-formal-evidence exit0, /tmp/dream-c1-66076-{licenses,check-formal-evidence}.log.
- The prescribed AND-chain fails at make test-dream-c1 with exit2. Formal/contracts above were run independently, not falsely reported as an AND-chain success. Latest canonical invocation /tmp/dream-c1-final-test-dream-c1.log, receipt /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-trzjuydv/receipt.json:12 forwarding checks and ordinary locked release build passed; exact hash2b85c711 retained; provisioning failed before Hello/Start. It is a failed runtime gate allowed only as disclosed source-landing limitation by Oracle override.

Actual ordinary worker failure:
Source2ce4359df034405043d419fe550b0f6cafe7887e, rustc1.99.1/vVsha e003fd3d385fcd0de16bc6b049124efb20207831dd810fd4ab39ff079bbb12a5, ordinary release default acp-agent/no test-fixtures, SHA2b85c7114545f35a62842115ae91e08e1a22fbfcd24dfd27e3aa45fa1d3837cb. Both Python/runtime tuple literals are bound to this reviewed build; these literals do not constitute a passed runtime acceptance.
Original canonical worker trial failed250ms: PID13754, entry signed spawn-return bounds288623000..362555000ns,73932000ns observation uncertainty, entry-to-prewrite58000ns. Entry and waiting once, late; noHello/Start/source/manifest/job. Lease/stdin retired, actual exit2/reaptrue/settledtrue/no quarantine. No exec/loader/kernel/scheduling cause inferred; auxiliary counters raw/unverified. /tmp/dream-c1-approved-test-dream-c1.log and /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-1sqmqa2v/. Original failure remains failed.

Decision66076 prerequisite failed:
make admit-dream-c1-file on the exact reviewed file exited2; /tmp/dream-c1-admission-original-file.log. --version PID54097 took22.184ms, exit1, reapedtrue; original claim e0da8a1bc85f81c1b83976a28003ae4b6ab01445605656db2a44bc6bae2bd37d retained. The pinned headless parser supports --help, but rejects --version. Stderr43-byte SHA384f5815544500b951c2a9626d173ff0f5e572c2a28df4f0e4a280dbd85333aa matches its known unsupported-option message; raw output was not retained, only bounded hashes.
The required final make test-dream-c1 rebuilt/replaced the image metadata through canonical Cargo. It produced a distinct exact-file claim4d84dfdbb7e2806f5ce1ac1d121449dd26e5a2a779fe8abd180bd18e62208e25: PID97577,308.784ms, exit1/reapedtrue, same binary hash and43-byte stderr hash. No failed claim was deleted or reset, no --help fallback or ad hoc inode replacement was used, and no further Dream birth occurred. The additional ordinary trial remains not reached because provisioning never qualified. Runtime tuple remains UNAPPROVED; source only is authorized to land.

Known limits and acceptance disposition:
- Cold/warm real same-process MCP diagnostic exited0 for routing/custody only: /tmp/dream-c1-admission-cold-warm.log and /tmp/dream-c1-cold-warm-phase-summary.json. Status1.7532575s then1.458710541/1.490842375s; page3.1525905s then2.800326916/3.027203167s. Cold is first measured call after fixture admission, not a fresh machine. Warm2 prepare384.819459ms/679667nsCPU; retire188.314333ms/137042ns; pagejournal432.947792ms/626667ns; domain+retire1744.176458ms/5990833ns cumulativeCPU. Phase totals include separately repaid1ms/9x cuts, not maxunit timing. Binary hash was not retained before later rebuild, so diagnostic only. No OS/QoS/sleep cause proved. Oracle explicitly makes these measured limits a follow-up, not source-land blocker.
- Full four-job joined readiness, parent+child heap/RSS, same-app240frames/input/message latency,200-call controls, independent-root progress and real-worker foreground/reap pipeline remain UNPROVEN/not reached. Unit/API/model greens do not replace them.
- Real bundle-files copies worker+internal receipt and installation relocates Contents. Exact path/inode/time admission does not transfer; writing internal receipt after signing invalidates bundle seal. No bundle/install recipe or signing workaround was added. Final-path admission storage/signing disposition and supported side-effect-free mode remain runtime follow-up; no install/deploy/live-supervisor touch.
- No native durable_commit class, retry, timer reset, lowered gate, unsigned commit, fake green receipt or binary/compiler fallback was used.
- Owner-safe cleanup unlinked1063 closed obsolete duplicate compiler libraries/metadata after exact lsof/stat/hash checks; all executables/workers/formal tools/graphs/receipts/raw logs retained. Manifest /tmp/dream-c1-cleanup-nov6-duplicates.json SHA c2f0eb384444bb7fda1c7b212c564197c5574acc55f4d3ccd2af2d7eaa77bd56. Logical1483463232bytes reclaimed, no claim of equivalent machine-wide physical recovery.

Report/task tooling provenance: predecessor unknown operation01M6RYQ9VK9JSRHVTNMYH7PW0P remains unknown/unretried; earlier distinct write01M6S2SH03N0NWNFRQF1A1N5V8 rejected report_task_denied and created no document. Both saved request files remain outsideGit. This is a NEW report path, not an edit/retry of either. Source evidence index /tmp/dream-c1-first-launch-admission-evidence.json SHA21695af6eca0975c6dc10a04b87ce5477d1b59453e0d1df0f2de729d7cbd26f3; latest failed-gate result is separately retained. Native knowledge publication/checkpoint is separate from source push and task acceptance. Oracle owns final task lifecycle.
