---
type: task
title: Correct C1 native creation/publication accounting and paid rest debt
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-dream-native-c1
allowed_paths:
- src/dream/c1.rs
- tests/dream_c1.rs
- tests/formal/dream_c1.rs
- tests/formal/models.rs
- verification/corpus.toml
- verification/contracts.toml
- verification/coverage.toml
- Makefile
- README.md
- ATTRIBUTION.md
- reports/2026-11-05-dream-c1-native-debt-producer.md
- src/dream/bootstrap.rs
- src/dream/mod.rs
- src/main.rs
- tests/fixtures/dream-bootstrap/**
- reports/2026-11-05-dream-c1-native-bootstrap-producer.md
verify: make test-dream-c1 && make test-formal-parity && make check-embedded-build
depends_on:
- '[[tasks/2026-11-05-dream-c1-producer]]'
derived_from:
- '[[specs/dream-term-c1]]'
- '[[decisions/d93377961365086-dream-native-quantum-and-debt]]'
id: 01M6PJA7JQNQQWVP39VP25BJYZ
status: accepted
delegated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T22:45:03.970Z
created: 2026-11-05T19:17:33.748Z
number: 93377961365092
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
assignee: seat:01M6PJAVQFS8RT61ZQ86T7QRRR
artifacts:
- '[[01M6PKFWS9HMJSRGVYS0EEK5HY]]'
- '[[01M6PN64GHTFK3S10WWCZB37F9]]'
- '[[01M6PT5CHK5X2SBCV70B895GTD]]'
- '[[01M6PTXSFJZM7QGAMHVT47EP4Q]]'
- '[[01M6PW8R5VB58P7YZQCC8CHMAF]]'
- '[[01M6PXY3B2KF730HFZ6DCB3Q26]]'
- '[[01M6PY65BBV7R6PQ4XMP6MJ394]]'
verify_result:
  command: make test-dream-c1 && make test-formal-parity && make check-embedded-build
  exit_code: 0
  evidence:
  - criterion: Final committed source and all required commands/actual exits, plus licenses
    reference: target/dream-c1-native-evidence-d06e696f/exits.json; test-dream-c1.log; test-formal-parity.log; check-embedded-build.log; test-licenses.log; check-formal-evidence.log — all0 on d06e696f
  - criterion: Earliest source-free gate, exact canonical private128-byte authority and bounded env/fd/lease/deadline/probe fences
    reference: src/main.rs:9; src/dream/c1.rs:418; src/dream/bootstrap.rs:392; tests/fixtures/dream-bootstrap; final C1 log2codec+7process families+31integration cases
  - criterion: Exactly one parked2MiB compute owner, raw native boundaries and strict1ms ordinary/control/cleanup
    reference: src/dream/c1.rs:177,2426; final four-summary.json/four-maxima.json; native compute31167CPU/31000wall, publish135958CPU/149792wall ns
  - criterion: Post-end9x debt, actual retained effects, paced cleanup/credits, final join/reap and clock failure refusal
    reference: src/dream/c1.rs:61,141,818,2369; tests at2900,2951,3148,3204; final C1 unit/process log; four terminal remaining_debt_ns0
  - criterion: Pre-descriptor RAW capture and complete later terminal/post-reap observations, finite usage and no double counting
    reference: src/dream/c1.rs:2098,329; tests/dream_c1.rs:1826; report metric cutoffs; four-summary.json captured<=terminal and actual wait4 user/system tail
  - criterion: Actual ordinary-release/no-test-fixtures worker, source/compiler/hash binding and meaningful causal bypass negative
    reference: target/dream-c1-native-profile/worker-414be210e45ea228af5d85ee38f484ef7ed5918603a27d718cf9691d6aeaae81; evidence-d06e696f/profile; distinct fixture-worker247798... actual argv1/reaped91073/grant0/readiness_certifiedfalse
  - criterion: True four-job/multi-MiB completion, independent causal controls, heap/resources and unchanged limits
    reference: 'final C1 log/four-summary.json: all4 actual completed/reaped,2503989bytes each,two roots,200fresh Activity RTT p99=17us/max34us; strict app284250/control109917/cleanup67334ns; heap863760bytes'
  - criterion: Full model inventory, finite causal controls and native/Java graph/verdict parity with final source/model/tool hashes
    reference: verification/evidence/4a57cba19c2c18f5310bbdb32a896cfd11770fb49315c4696fb107811f88fba6/evidence.json — source d06e696f dirtyfalse,261models73contracts1crate; QuantumDebt19230states/56119edges, Bootstrap9272/35252
  - criterion: Failed baselines/effect truth, observational startup/native limits and consumer-owned exclusions retained
    reference: 'report #93377961365643; bab C1exit2;56ac Clippyexit2;ca96 C1exit2 with real61575666ns publish wall/effect1/resource_limit and wait; peer cold-copy dyld readiness failure aqd69hlb remains unproven'
  - criterion: Signed provenance, allowed paths, attribution, no consumer/Q2 edits/dependency or wire change, no landing
    reference: report all9 signed SHAs; git verify-commit HEAD exit0; final-evidence-summary.json56allowedfiles; ATTRIBUTION.md; status only M .bboxy/runs/formal-native.json; final essential profile handoffs01M6PW8R4W41BF3EB76R0VJBKA/01M6PW8R58XP6FATB5PJKFZ909/01M6PW8R5HNEW3RB3MPZMFN2NY
report:
  summary: 'Implemented the scoped producer native/debt correction and earliest private bootstrap permit on nine signed commits through d06e696f99472ed754e84353baf11a2974589d6d. Final C1, full261-model/73-contract native-TLC parity, embedded build, licenses and formal-evidence checks all exited0 unattended; immutable ordinary worker414be210... is source/compiler/features-bound and four multi-MiB workers completed/reaped with causal200 RTT p99=17us, app/control/cleanup maxima284250/109917/67334ns and zero remaining ledger debt. Full append-only report #93377961365643 retains the real61.6ms publication-effect failure, earlier failed gates, exact metric/start/end cutoffs, typed effect/clock/settlement witnesses, artifact hashes and current cold-copy pre-main250ms consumer failure. Only generated private formal receipt is dirty. No production pin, consumer combined conformance, acceptance or landing is claimed.'
  commit: d06e696f99472ed754e84353baf11a2974589d6d
  reported_by: seat:01M6PJAVQFS8RT61ZQ86T7QRRR
  op_id: 01M6PXE7RPXCFQRSECNFB5KKJ5
  verify_revision: 0
  report: '[[reports/2026-11-05-dream-c1-native-debt-producer.md]]'
provenance:
  digest: sha256:eadf1dc3fe578a68c08443ddc6de2484b218418bf6def519709f01cc02c2cc67
  frontmatter_digest: sha256:2b6336a054f44de86502b0bdd8f99e8948afe146433b5d639d7cf40f2f186797
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: jXoZe7/nlh50FDmpyyTSsIbMjN5wNhIUJk9zb/vLaYNBdbmcXAuYY00sfjbH+thUtUxmGhjNuYMerZ/PSq6nCw==
  log: .bboxy/log/2026-11-05.jsonl#145792
---
ROLE: EXECUTOR

## Goal and exact writing scope
Implement the narrow PRODUCER correction in [[decisions/d93377961365086-dream-native-quantum-and-debt]], current full [[specs/dream-term-c1]]/[[interfaces/dream-term-c1]]. Read them completely through continuations. This is a FRESH correction task; accepted C1 task/source47182320 stays frozen as historical evidence. ONE source writing worktree /Users/feral/mydev/bboxy450-harness-dream-native-c1, branch fix/dream-c1-native-debt, exact current-main/accepted base47182320be6435c860bc2176c2ddda582029fa63, created clean by Oracle. Read project/shared instructions/source/Git before edits. Never edit term consumer, Q2, old producer or dirty primary checkout. Allowed paths only; no new dependency/lockfile, global scheduler/allocator project, source eligibility expansion, generic resolver or wire/schema/algorithm/IR/control-version change. No recruit/task creation/merge/push/install/release/service/credentials. Signed source commits; Oracle verifies/lands. KB/report stays selected term root through authenticated tools; no KB commit required, raw fixture/logs outside Git. Attribution retained.

## Exact producer implementation
Actual macOS early entry runs c1::run before shared runtime. Preprovision exactly ONE existing dedicated compute owner before Hello/established pipe-control polling, fixed2MiB stack and parked until matching Start+foreground/currentness. No manifest/source access, new pool or model/provider bootstrap while parked. EOF/cancel/malformed/missing Start still joins/settles that actual parked owner. Builder::spawn alone gets compute_owner_create native tag; surrounding allocations/channel/argv/framing/control remain ordinary1ms units. Instrument subspans: prior whole-child-control1,316,083ns is unclassified, NOT automatically native. Keep strict established-control<=1ms; if another app segment exceeds, split/fix rather than exempt it. No native work on actual shared async reactor.

Only exact decision-enumerated native calls get observed<=2ms threadCPU/<=25mswall. Producer uses sole compute-owner creation and individually prechecked atomic no-replace publication, existing page/descriptor/count/output/deadline caps. Each work span <=1 tagged call and <=1ms surrounding appCPU, inclusive<=3ms. Parsing/hash/serde/SQLite/cleanup/control remain<=1ms. Measure/tag true call boundary, inclusive CPU/wall and app outside it; no blanketwrapper exception, clamping/subtraction, repeated retry or borrowed memory/control lane. All native/kernel CPU charged by the caller clock remains in totals, spawned owner/process CPU measured separately without double counting.

After each expensive span inclusive CPU C set per-run debt9*max(1ms,C), paid as monotonic wall rest AFTER span end before next expensive admission. No saved idle/IO credit, error/reset/duplicate/ABA debt bypass or parent/child parallel work. Pay bootstrap debt before Hello/first work; independent controls never await work debt/native IO. Normal app<=1ms retains>=9ms rest. Preserve original strict foreground/deadline admission,30s,2s+2s escalation, actual join/reap/quarantine,32/96MiB and protected2MiB/78+16+2 credit rules.

Capture typed native result/effect/handle and original nonce/operation before evaluating clock/timing/cancel. A process/thread/link can exist even when a verdict fails. Preserve known/unknown effect, inode/path or owner handle, paced cleanup and credit ordering through error/cancel/restart. Never repeat a possibly completed publication or claim no effect from timing failure; retain honest settlement/quarantine and exact resource error. Preserve accepted R1 returned-owned-result/partial cleanup fixes; do not regress them while changing native spans.

Keep ResultDescriptor.max_poll_cpu_ns RAW inclusive at the decision's documented PRE-DESCRIPTOR capture cutoff (before its own serialization/publication). Full terminal metrics cover that later descriptor work/native publication and all later cleanup/control/settlement; validate captured<=terminal overall. No self-referential resealing or wire change. Report bounded per-class count/maxCPU/maxwall, app/control/cleanup/inclusive maxima, full CPU/wall and debt/rest/effect observations in existing private diagnostics/runner receipts with a precise source/compiler/features profile. Existing bare<=3ms result is insufficient evidence; legacy1ms consumers may safely refuse. Freeze/document exact bounded metric names/units/cutoffs in source/report and hand consumer the signed milestone/artifact profile (essential boundary handoff, not routine status). Do not introduce unbounded per-call logging or hidden stderr/frame overflow. Consumer owns metric parser/admission, Command::spawn and SQLite split in its own tree.

## Evidence and required checks
The old four-job failure is preserved at /Users/feral/mydev/bboxy450-term/target/dream-c1-native-boundary-somrqvej: actual workspace runnerSHA16a746d6d115f75c33402ac61ded2f9ba6ac2272930acd0bd99febb7c9ce4edf, all4refused/reaped, no green claim. It differs from standalone artifact despite same source. Root-approved earlier binary remains historical, not native-profile conformance.
Update focused C1 gate/model inventory/contracts with C1QuantumDebt actions/finite state/weakfairness and causal controls from decision: unpaid rest, fake-native2msparser/control/cleanup, repeated/unresolved effect, missingcost, earlyexit/ownerrelease, clockfail/cancel/ABA/withheldcompletion. Pair exact typed effect/real thread or staged publication witnesses, including >2msCPU/>25mswall failure retaining actual effect and independent controls. No compile-fail substitute/fake positive or reduced corpus. Actual ordinary-release NOtest-fixtures worker from final committed source must have minimal cleared-environment Hello before source access/EOF/reap and true multi-MiB/four-job producer/private-parent control/resource success at new limits; final actual term parent/foreground/native-frame/authority/hints combined gates remain consumer-owned, unproven here. Require real process native interval/profile output and strict app/control/cleanup maxima. Document observational native/startup limits honestly, not universal OS bound.

## Coordination, gate and report
Q2 author01M6PBF0EX2R5RGQAXNQTR96Y0 has ongoing selected-input/shared-helper extraction on a separate branch. Do not copy/edit/reframe Q2; keep this C1 prerequisite patch self-contained/minimal so Oracle can land it first and Q2 can reconcile. Consumer01M6NPQVR1STF4MY8C3TV382TJ is implementing its amended same contract independently. One essential signed source/metric/provisioning handoff to both and Rook is authorized when interface ready; no acknowledgement waiting. Source dependencies remain honest until Oracle authorizes new pin.
Run only targeted iteration, one build/test at a time, two Cargo jobs. Final make test-dream-c1 && make test-formal-parity && make check-embedded-build must exit0 unattended, plus make test-licenses; refresh source/model/tool hashes/formal receipt on final signed source and after any rebase. Preserve exact log exits and ordinary-release/private fixture profiles/features/hashes. Full report through knowledge_write reports/2026-11-05-dream-c1-native-debt-producer.md: task link, signed commits, verifycommand/exit, evidence per requirement, source mechanism/line, failed-baseline vs final measurements, real effects/settlement, artifact paths and exclusions. task_report once; no routine ACK/progress/status, no publication/acceptance/landing claim.

## Earliest bootstrap permit — producer scope and obligation now authorized
Implement [[decisions/d93377961365166-dream-bootstrap-private-permit-handoff]] and synchronized current C1 spec/ICD. This is the recorded cross-process disposition you requested; no further approval/response wait. Signed206c/current ordinary hash92df6eb6890def6fe406c278c2a8a06c509605d2147ef2d6c53c0cd999da86ee and its passing243model receipt remain valuable BASELINE, not final no-overlap conformance. Accepted original47182320 task remains frozen. Same final verify command and other law, with expanded focused bootstrap coverage.
Producer owns ONE canonical private128-byte dream-bootstrap/1 parser/layout/binding fixture authority. New exact allowed src/dream/bootstrap.rs (optional adjacent module), src/dream/mod.rs minimal registration and src/main.rs ONLY if needed for earliest pre-runtime dispatch/unsupported-entry refusal; no general startup/UI redesign. Add canonical valid/invalid128-byte/binding vectors under tests/fixtures/dream-bootstrap/** and include actual parsing/custody/independent conformance in existing Make test-dream-c1. Consumer mirrors/reads this authority; send the signed exact layout/vector/entry-readiness milestone when ready, no separate competing format. Existing src/dream/c1.rs, tests/dream_c1.rs, formal/maps/Make/README/ATTRIBUTION paths cover the rest. Do not implement/edit consumer or Q2 source.
Gate BEFORE the first controlled early argv Vec/PathBuf/Arc span (current206c argv_at around425), not merely run/Builder. Only borrowed recognition/fixed env/fd/token parsing/one128-bytebuffer/sleeping wait are bounded measured control prelude, <=1KiBprotected state inside existing2MiB. NonmacOS reaches same gate before controlled runtime/CLI thread startup, or refuses unsupported entry; no ungated fallback, new platform guarantee or unrelated ordinary-launch behavior change. No manifest/source/cwd/HOME/provider/custody/privatekey discovery before matching admitted Start.
Use exact decision env BBOXY_DREAM_BOOTSTRAP=1:N:birth:binding (<=128ASCII), inherited one readable anonymouspipe N3..65535 and fixed BBDBOOT1 permit bytes/purpose/probe/birth/run/nonce/binding/incarnation/sequence1/absoluteCLOCK_MONOTONICdeadline/reservedzeros; no public JSON/schema/algorithm/IR/controlversion change. Reject wrong fd/type/identity/field/purpose/job basename/nonce/expiry/extra/truncated bytes/unknown profile; valid labels alone are not admission. Probe=true never accepts Start. Before every controlled bootstrap span and after debt wait recheck live lease/deadline; close/extra/HUP revokes new work, retain any in-flight native/thread effect and actual join. At most4prepermit interrupted waits/reads and<=4permitreads, poll remainingdeadline rather than spin. Readiness250ms from parent spawn-return and childentry min/originaljobdeadline must not reset/inflate.
Child may do argv/channel/sole parkedcompute only AFTER valid permit. It pays ALL bootstrap debt before retiring gate read end/emitting ordinary pinned Hello; no more expensive work/source until existing matching Start. Permit/lease checks/Hello/Start framing are explicitly finite<=1ms CONTROL bookkeeping with usage retained, not recursive work/debt grants. Do not relabel source/hash/serde/create/disposal loops control. This controls declared startup spans, not arbitrary dyld/kernel/threadentry instructions; retain complete observed process/startup usage with limits.
Parent/fixture owns pipe/writelease/Child/effect, pays launch+postsetup debt and currentness before single128byte grant, then CONTROL ONLY until paid-bootstrapHello. Producer test launchers must implement that exact private handoff, hold writelease throughHello, close afterHello; parent prepare/pay beforeStart. Missing/old/unapproved hash/profile MUST NOT be executed ungated as a capability probe; source-free probe uses same approvedgate/Hello/EOF/reap path with<=1/account. Manual missinggate refuses sourcefree. Update ordinary-release and private-test launcher/artifact receipts to declare gate support BEFORE birth while keeping hashes/profiles/features distinct. Existing test fixtures must use permitrun_id job basename and original identity, without weakening input/wire law.
Extend source/model/process causal evidence: parent held after actualspawn beforeverdict/debt/grant => zero child controlled argv/channel/Builder admissions (prove with sourceboundmarkers/receipts, notsilence); then paidpermit/bootstrapHello/preparation/Start/complete/reap. Cover false/expired/stale/partial/duplicate/revoked/foreign fd/token/EOF/crash/restart permits, native clockfailed realChild, unknown effect, probe and deliberate earliestgate bypass. Control/status/cancel/otherjobs stay responsive; preserve existing4job/multiMiB/heap/foreground/finite-cost/causalRTT and exactsettlement gates. Scope uses existing task source/formal/test paths plus canonical fixtures above; no extra worker/pool/globalqueue.
Final source/artifact/formal receipts MUST refresh after this implementation;206c passing gate cannot certify gatework. If earlier append-only report already exists, create reports/2026-11-05-dream-c1-native-bootstrap-producer.md with supersedes and same task/actualverify/commits, then task_report. Oracle reviews/lands and authorizes new consumer/Q2 baseline only afterward. Report actual failures/baselines/exclusions; no operator input needed.


## Final bootstrap admission clarifications — explicit probe context and bounded public control
[[decisions/d93377961365255-dream-bootstrap-source-free-probe-context]] clarifies65166: EVERY source-free probe=true binding explicitly serializes context:null (not omitted/empty/fixture/prior-seat context). Keep fresh original probeop/run/nonce/birth/nonzero hostlaunchincarnation, purpose and exact preapproved source/compiler/workerhash/profile, SAME128bytepipe/lease/debt/Hello/EOF/reap and<=1/account. No seat/root Authority is invented or required for capability-only probe. Probe=true permanently rejects ANY Start includingmatchingIDs, sourcefree with actual parkedowner join; no manifest/source/providerread. Probe=false still REQUIRES full current captured publiccontext+sourceclosedauthority beforebirth; null/missing is refusal, never a fallback. Canonical bindingvectors and real hostile-Start/no-source probe/finite-tail/ABA/deadline/EOF/reap negatives must cover this distinction. Runtime capability cache is not authority/sourceeligibility. Oldunknownbinary still never probedungated.
[[decisions/d93377961365259-dream-bootstrap-final-public-admission-control]] clarifies the TWO final worker handoff sites ONLY (one bootstrap-permitattempt and one authorizedStart): existing PREPARED fresh public authorization callback + tiny transfer/write is measured inclusive<=1ms admissionCONTROL on the existing ISOLATEDadmissionowner, no work9x debt/recheckrecursion. No SQL/IO on UI/mux/sharedasync/independentStatusCancel lane. Retain exact currentguard: <=7fixedpublicrowdigests, <=16KiB/cell (<=112KiBtotal) bounded BEFOREfetch/hash, exactboundedactorprimarykeys/scalars, file/inode/revision/time/publication/currentroot/provider checks, preparedreadonlycache/nonblockinglocks. Preparation/connections remain ordinarywork BEFOREdebt. All rawrow/head/grant/inode/revoke/expiry/ABA negatives remain; atomics/TTL/custody/fakeauthority cannotreplace it.
Only those publicproofdigests belong to finaladmissionCONTROL, not source/payload/manifest/result/binaryhash, genericSQLite/serde/KB/provider/helper work. Other guard callsites keep original accounting; no freepoll/recheckloop. Whole finalstepinclguard+grant/Start controlwrite <=1ms, clocksfinite and fullCPU/wall retained; busy/unknown/oversize/excess/changedauthority refuses/revokes, keeps actualChild/pipeeffect and settles, no retry/newbirth/debtreset. Sourcefreeprobe usesnullcontext/noactorproof. Require actual helddependency/after-debt tamper/clock/>1ms/misclassifiedpayload/duplicateattempt controls plus unchanged4job/foreground/nativeframe/heap/finitecost/causalRTT/reap gates. This adds no resource/wire/schema/algorithm/controlversion and no new source paths or verifycommand. Parent owns finalpublicchecks; producer/Q2 preserve the protocol/ownership meaning, no duplicated identity subsystem.


## Report from seat:01M6PJAVQFS8RT61ZQ86T7QRRR

Implemented the scoped producer native/debt correction and earliest private bootstrap permit on nine signed commits through d06e696f99472ed754e84353baf11a2974589d6d. Final C1, full261-model/73-contract native-TLC parity, embedded build, licenses and formal-evidence checks all exited0 unattended; immutable ordinary worker414be210... is source/compiler/features-bound and four multi-MiB workers completed/reaped with causal200 RTT p99=17us, app/control/cleanup maxima284250/109917/67334ns and zero remaining ledger debt. Full append-only report #93377961365643 retains the real61.6ms publication-effect failure, earlier failed gates, exact metric/start/end cutoffs, typed effect/clock/settlement witnesses, artifact hashes and current cold-copy pre-main250ms consumer failure. Only generated private formal receipt is dirty. No production pin, consumer combined conformance, acceptance or landing is claimed.

```json
{
  "op_id": "01M6PXE7RPXCFQRSECNFB5KKJ5",
  "commit": "d06e696f99472ed754e84353baf11a2974589d6d",
  "report": "[[reports/2026-11-05-dream-c1-native-debt-producer.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Final committed source and all required commands/actual exits, plus licenses",
        "reference": "target/dream-c1-native-evidence-d06e696f/exits.json; test-dream-c1.log; test-formal-parity.log; check-embedded-build.log; test-licenses.log; check-formal-evidence.log — all0 on d06e696f"
      },
      {
        "criterion": "Earliest source-free gate, exact canonical private128-byte authority and bounded env/fd/lease/deadline/probe fences",
        "reference": "src/main.rs:9; src/dream/c1.rs:418; src/dream/bootstrap.rs:392; tests/fixtures/dream-bootstrap; final C1 log2codec+7process families+31integration cases"
      },
      {
        "criterion": "Exactly one parked2MiB compute owner, raw native boundaries and strict1ms ordinary/control/cleanup",
        "reference": "src/dream/c1.rs:177,2426; final four-summary.json/four-maxima.json; native compute31167CPU/31000wall, publish135958CPU/149792wall ns"
      },
      {
        "criterion": "Post-end9x debt, actual retained effects, paced cleanup/credits, final join/reap and clock failure refusal",
        "reference": "src/dream/c1.rs:61,141,818,2369; tests at2900,2951,3148,3204; final C1 unit/process log; four terminal remaining_debt_ns0"
      },
      {
        "criterion": "Pre-descriptor RAW capture and complete later terminal/post-reap observations, finite usage and no double counting",
        "reference": "src/dream/c1.rs:2098,329; tests/dream_c1.rs:1826; report metric cutoffs; four-summary.json captured<=terminal and actual wait4 user/system tail"
      },
      {
        "criterion": "Actual ordinary-release/no-test-fixtures worker, source/compiler/hash binding and meaningful causal bypass negative",
        "reference": "target/dream-c1-native-profile/worker-414be210e45ea228af5d85ee38f484ef7ed5918603a27d718cf9691d6aeaae81; evidence-d06e696f/profile; distinct fixture-worker247798... actual argv1/reaped91073/grant0/readiness_certifiedfalse"
      },
      {
        "criterion": "True four-job/multi-MiB completion, independent causal controls, heap/resources and unchanged limits",
        "reference": "final C1 log/four-summary.json: all4 actual completed/reaped,2503989bytes each,two roots,200fresh Activity RTT p99=17us/max34us; strict app284250/control109917/cleanup67334ns; heap863760bytes"
      },
      {
        "criterion": "Full model inventory, finite causal controls and native/Java graph/verdict parity with final source/model/tool hashes",
        "reference": "verification/evidence/4a57cba19c2c18f5310bbdb32a896cfd11770fb49315c4696fb107811f88fba6/evidence.json — source d06e696f dirtyfalse,261models73contracts1crate; QuantumDebt19230states/56119edges, Bootstrap9272/35252"
      },
      {
        "criterion": "Failed baselines/effect truth, observational startup/native limits and consumer-owned exclusions retained",
        "reference": "report #93377961365643; bab C1exit2;56ac Clippyexit2;ca96 C1exit2 with real61575666ns publish wall/effect1/resource_limit and wait; peer cold-copy dyld readiness failure aqd69hlb remains unproven"
      },
      {
        "criterion": "Signed provenance, allowed paths, attribution, no consumer/Q2 edits/dependency or wire change, no landing",
        "reference": "report all9 signed SHAs; git verify-commit HEAD exit0; final-evidence-summary.json56allowedfiles; ATTRIBUTION.md; status only M .bboxy/runs/formal-native.json; final essential profile handoffs01M6PW8R4W41BF3EB76R0VJBKA/01M6PW8R58XP6FATB5PJKFZ909/01M6PW8R5HNEW3RB3MPZMFN2NY"
      }
    ]
  }
}
```
