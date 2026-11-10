---
type: decision
title: Require a qualified source/build basis for Q2 proof-tool lifetime
date: 2026-11-05
derived_from:
- '[[research/dream-q2-tool-lifetime-basis]]'
- '[[tasks/2026-11-05-dream-q2-tool-lifetime-basis]]'
- '[[decisions/d93377961365489-dream-q2-member-window-accounting]]'
id: 01M6PXVX4BK5YAVZK7ZFEK2AVY
updated: 2026-11-05T22:39:27.244Z
created: 2026-11-05T22:39:27.244Z
number: 93377961365654
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:1bd1ace753479333997664b78289665ce42b40d53dc222c8bb42423dd5f8d0b0
  frontmatter_digest: sha256:4931ec3d3b1c9c6337cbe1f559a4a5c9ce1c6c0e33814705605ac91b4c11ae20
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: o3fwycNqMEK6KMq8Jt1cle75CU9WamCtwPp42ZiwEsgK6zhoNAPye8KRyirmdseV9qZY7jhDl75skIBVFS0EDQ==
  log: .bboxy/log/2026-11-05.jsonl#145661
---
# Q2 finite tool lifetime is a qualified source/build assumption

Date2026-11-05. Task [[tasks/2026-11-05-dream-q2-tool-lifetime-basis]], evidence [[research/dream-q2-tool-lifetime-basis]]. Refines ONLY capability interpretation in [[interfaces/dreaming-mcp]] / [[specs/dreaming-mcp]] under65126/65489. No C1/native/worker/read wire, schema, IR, version or accepted component change.

## Alternatives, choice and immediate effect

1. Treat sampled TASKALLINFO/RI/Trace2 and feature strings as proof of all lifetime births/threads: rejected; fast unseen work is not excluded.
2. Add NOTE_TRACK/new tracer/entitlement/controller: rejected; installed SDK declares NOTE_TRACK unsupported, and another monitoring architecture is outside scope.
3. Use a finite reviewed source/build/tool-subset basis plus independent runtime corroboration: chosen as a **prerequisite**, not an already met capability.

Current Apple Git-157/2.54.0 SHA9a1c8fc6 and Homebrew GPG2.5.24_1 SHA21cb9adc remain verification_unproven. Apple's signature and Homebrew bottle receipts identify provenance claims, not exact reachable lifetime behavior. Public Apple Git main is2.50.1, not a source/build binding for this2.54.0 artifact. GPG's reachable libraries/configuration likewise lack a completed exact-build closure qualification. No qualifying retry/available lesson is authorized. Independent Q2 fixed-slot, causal, bootstrap and regression work continues.

The smallest selected product prerequisite is one private **source-built real Git/GPG pair qualified for the unchanged65126 commands**, using immutable authenticated upstream Git2.54.0 and GnuPG2.5.24 sources, explicitly selected backports and exact dependency/config/build pins. It preserves public show/import and implicit real signature verify; no fixture replacement, omitted trust check or accepted input narrowing to obtain a pass. This decision does not authorize building/installing/launching it or assign implementation. Oracle selects/amends a later scoped prerequisite task. If the pair cannot meet the existing bounds, Q2 remains unsupported; changing proof technology is a separate decision.

## What is trusted and what is checked

The rule actually measures bounded attributable background execution/coverage, foreground responsiveness and complete settlement. It is not a theorem about arbitrary binaries, opaque OS work, hostile libraries or OS scheduling.

| Basis | Exact obligation / evidence |
|---|---|
| Trusted source/build premise | For ALL approved command/config/input/error/cleanup paths, direct Git/GPG has one runnable thread; Git adds at most its one approved GPG child and awaits it; GPG adds no process/thread/service; no group escape. Include startup, constructors, linked/runtime-loaded code and malformed public bytes. This is an explicitly reviewed finite program assumption. A grep, missing dynamic symbol, source version or successful sample is not that review. |
| Build provenance | Authenticate source/release commit/archive+patch hashes; retain compiler/SDK/build flags/generated config, effective argv/env/files, linker map and actual executable/recursive nonplatform library hashes, plus explicit pinned platform/framework/libSystem assumptions. Sign/bind produced artifacts to that receipt. Source source-built does not automatically mean qualified. |
| Actual executable checks | Before birth: existing current tool/file/source/OS/approval/custody guards, immutable receipt/evidence binding, command/input limits. During execution: existing suspended birth, identity/incarnation, conservative slot windows, controller/pipe/CPU/debt, foreground/cancel/deadline and memory gates. After: full proof/audit, roll-up consistency and real reap/group/retired coverage or quarantine. |
| Unmodeled trust boundary | Compiler correctness, audited-source fidelity, named platform implementation and absence of arbitrary-code compromise remain assumptions. Runtime samples do not prove those assumptions. A known or suspected violation invalidates the profile; trust/refusal must not hide it. No all-times preemption/native-heap guarantee follows. |

Do not claim that NO_PTHREADS suffices: Git run-command can fork for async work in that mode. Do not claim nPth init creates an extra thread by itself: its source distinguishes init and create. The qualification must resolve actual reachability. It must rule out common.conf/keyboxd, agent/dirmngr/photo/helper paths and library work under the exact admitted public-only configuration, not modify the signature semantics or rely on operator state. An unreachable disabled component is a source/build argument, not “it was not installed, therefore impossible.”

## Bounded packaging without a wire change

Use the existing32KiB bboxy.dream.proof-tools/1 receipt and128KiB prepared capability-evidence budget; no field/version/feature-string invention is needed. Qualification is host-prepared immutable evidence, not a caller-provided policy claim or new memory authority.

| Existing binding | Required qualification content/admission |
|---|---|
| git/gpg absolute_path,sha256,build,features | Exact two real artifacts; no Xcode/CLT/PATH/version equivalence. features are assertions admitted only for the matching qualified artifact pair. |
| source_sha256, capability.probe_source_sha256/probe_exe_sha256 | Exact harness guard/provision source and independent component-witness source/executable; source changes invalidate old evidence. |
| capability.evidence_sha256 | Hash of bounded prepared evidence whose host approval binds command subset, source/build/dependency/config manifest, explicit assumptions, review/check results and runtime witness receipts. A syntactically valid hex string is not verification. No worker-side ad hoc filesystem discovery or synchronous KB/provenance scan. |
| os.release/arch plus prepared evidence | Bind actual OS build/SDK/platform and relevant loaded-image closure in evidence. OS/framework/library/tool/config/loader change invalidates qualification, including a symlink retarget or same-version rebuild. |
| Host-approved provisioning | Existing approved producer/provisioning source must refuse to mint/admit a claimed lifetime capability unless the exact immutable qualification is approved and bound through current artifact authority. Missing/stale/unsigned/mismatched qualification is Unproven BEFORE birth. Standalone without that prepared approval has no ambient fallback. |

The fixed host qualification manifest fits inside existing prepared evidence; long audit/build logs are external immutable evidence references with prevalidated bounded hashes, never live tool-call scans. Record explicit source file/command roots and dependency membership, exact hashes, generated configuration and proof obligations with approved/not-established outcomes. Existing strict receipt DTO stays unchanged; the prepared provisioner/guard must actually validate the qualification, rather than just append new strings. This is a missing prerequisite, not a claim that current resolve.rs:250–334 already implements it.

No general audit framework, binary rewriter, live tracing daemon or global lock/pool is selected. Source-qualified private artifacts use the canonical future repository Make provisioning gate; no second system installation or credentials. This task specifies the prerequisite, not a speculative make target claimed to exist.

## Runtime semantics and retained law

Keep256 consumed attempts including implicit child, one group/two processes/one runnable thread per provisioned process, native creation2msCPU/25mswall/3msinclusive, ordinary application/controller/cleanup/grants1ms, fresh9x debt, 30s/two-root/four-account,32/96MiB and78/16/2MiB shares/control reserves, foreground/pause/control latency, actual effects/reap/quarantine, bootstrap65166/65255/65259 and complete original proof/publication/strict wire/hints.65489 still covers unknown pre-roster/fork/exec windows conservatively; source qualification does not excuse unknown runtime continuity.

Missing or unsupported lifetime basis/OS/accounting=verification_unproven; invalid actual signature/publication=verification_failed; known count/time/output/memory breach=resource_limit. Changed evidence/tool/source=current existing source_changed/refusal semantics. No success/available lesson, refreshed trust or output certification before complete proof plus actual settlement. Retain original op/effect/PID-start/audit incarnation, nonrefunded attempts and debt through failure/ABA/restart. Do not release an uncertain owned slot or retry a new helper to erase unknown effects. Existing sources/memories never gain authority from this qualification.

## Verification and limits

Extend existing finite Q2 model with basisApproved/sourceBuild/commandConfig/loadedClosure/osMatch booleans, two principals/slots, incarnation A1/B2/A3, bounded attempts/debt/window coverage and trust. Initially basis absent, no effect, untrusted. Atomic actions PrepareQualification/Admit/Create/ForkExec/Observe/CloseRoster/Charge/Rest/Invalidate/Revoke/Retire/Reap/Trust. Trusted tool boundedness is an explicitly listed model premise for approved profiles, not an invariant secretly “proved” by excluding hidden events. A second adversarial model/control permits hidden/transient process/thread/escape and must show loss of coverage/refusal; source/build qualification is the separate obligation that rules those paths out for the approved program. Internal control/settlement is weakly fair; no arbitrary IO/OS success assumption.

| Causal controls | Actual unattended evidence required |
|---|---|
| Assert feature strings without approved basis; omit/change library/OS/tool/config/source hash; missing/stale receipt | Pre-birth refusal with zero helper user execution; same-version CLT/Xcode difference and library/symlink change controls. |
| Hidden fast process/thread, GPG daemon, unexpected fork/exec/group escape, missing/retired member | Deliberately different private mutant artifacts must fail approval before birth; source-audit/build controls must catch reachable added spawn/thread sites. Their test markers establish that census-only monitoring can miss short work; do not falsely advertise runtime detection of every unseen event. |
| Drop source assumption while still trusting; reuse A1 after A3 | Model must expose the intended unaccounted-work violation, not a compile error; profile invalidation prevents old approval restoration. |
| Omit descendant CPU/roll-up, stop/pipe/controller/late-kernel charge or reset debt/settlement | Real actual tool receipt and source-bound model/process controls, retained counters/endpoints, failed clocks/cancel/revoke and exact real exit/quarantine. |

A future source-qualified real pair must pass existing make test-dream-mcp and make test-formal-parity, full finite state/transition/verdict parity against pinned Java TLC, qualifying published lesson and corrupt-proof negative, four real jobs/control/foreground/heap/30s/reap plus runtime window/coverage receipts. The prerequisite build/provenance check belongs in the repository's canonical Make workflow when implemented; it is not an installed target now. Source checks/models are not substituted for actual process or performance gates.

No source/build/test/probe/runtime measurement occurred here. Current pair availability remains unproven; only research/decision/contract wording changed. Detailed source/metadata hashes and retrieval limits are in the research and task report.
