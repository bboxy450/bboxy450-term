---
type: report
title: Q2 member-window accounting disposition and evidence limits
task: '[[tasks/2026-11-05-dream-q2-member-window-disposition]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T22:16:25.501Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6PWHKS10YHZJYFN6YBPG40S
updated: 2026-11-05T22:16:25.501Z
number: 93377961365536
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:d317f6181995e36723ddeea9869eb64f75415424dc23ced7bdc748b6fb3c65aa
  frontmatter_digest: sha256:ea16424542fc2c763e65475d641919ae3beb9a65fd5d009932bcf954fa51a2ac
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: 9gqNxjTNpG9TaCKB9OMZg7OtiAYAt/xkURsCEr4dgKDvZEnjPWlzqHCwgGIxuSCA4eFh30iPrFkga7+gazdiCg==
  log: .bboxy/log/2026-11-05.jsonl#145027
---
Task: [[tasks/2026-11-05-dream-q2-member-window-disposition]]. Commits: [].

## Disposition

Created [[decisions/d93377961365489-dream-q2-member-window-accounting]] and narrowly synchronized only Q2 helper-lifetime wording and its verification matrix in [[specs/dreaming-mcp]] / [[interfaces/dreaming-mcp]]. Existing refs/etags/frontmatter were preserved on updates; host authorship/provenance timestamps changed normally.

Chosen correction: conservative possible-runnable windows for each reserved logical process slot, beginning BEFORE actual CONT and ending only after positive current-incarnation stop/exit. Unknown/newborn/fork/exec coverage remains open from the prior uncertainty boundary until complete roster closure. Grant Q=max(sum of slot-window unions, nonduplicated active helper CPU)+ALL active controller/pipe CPU remains≤1ms. Reserved process count is not evidence that every slot remained runnable through the last acknowledgement.

Only work after an actual stable stopped AND closed roster may become separate ordinary≤1ms paced corroboration. Checks needed to establish closure remain active work. Every work unit and newly discovered late-kernel charge creates fresh post-observation9x debt; no helper/main proof/hash/parser admission before closure/corroboration/debt is complete. No false blocked/zero-delta endpoint, timestamp backdating, debt reset, free control, native wrapper exemption or resource/threshold inflation. Exact tooling/coverage not established stays verification_unproven; known breached timing/caps remain resource_limit. Current qualifying lesson availability is unproven, not enabled by this decision.

Preserved65126 tool forms/hash/build/OS/source provisioning, START_SUSPENDED, one group/two processes/one runnable thread per provisioned process,256 creation reservations,30s/two-root/four-account jobs, control/foreground/heap/reap, bootstrap65166/65255/65259, source/publication/strict wire/hints. C1 quantum65086, accepted producer and wire/IR/version were not changed. Oracle live-task amendment is required before Q2 implements the new rule.

## Actual evidence and source anchors

Read-only tree: /Users/feral/mydev/bboxy450-harness-dream-read-q2. HEAD at 22:09:48 and 22:15:12 UTC was `3d9fa84f196ea9b1b643939e7ae035a7dd0a6c9c`. Active author changes: Makefile; src/dream/{c1,read,resolve,worker}.rs; tests/dream_read.rs; tests/fixtures/dream-read/proof-helper-probe.c; tests/formal/{dream_read,models}.rs; verification/{contracts,corpus,coverage}.toml. These are moving authored bytes, not all committed/shipped behavior. No changes by this seat. The inspected source hashes matched at final readback; later changes can stale this snapshot.

- c1.rs:1151–1182: finish_grant uses expected_members*elapsed+lane_cpu through final closure; :1185–1598 includes identity/image/usage/RSS and roster closure in sample; :1679–1775 charges its controller CPU while the grant is running. Source provides finite identities/stops/roster mechanisms, not a measured new endpoint implementation.
- Frozen probe :64–125 stamps control observations after the actual call and requires stopped+zero running threads; :376–415 handles Git-clone→GPG task-token change/fresh stop; :445–551 uses2*last-stop wall plus controller CPU, with some later stopped corroboration already separate.
- read.rs:1121–1168,1216–1385,1410–1490 owns usage/reconciliation/retirement/debt/actual custody handoff. These paths must consume the same exact ledger; a pipe tail cannot disappear or be charged twice merely because closure happened on the other owner.
- resolve.rs:270–334 binds source/OS/capability receipt; :1373–1380 prepares suspended native creation; :1581–1678 reserves creations and retains original effect identity before timing disposition.
- tests/dream_read.rs:284–318,515–520 resolves explicit Xcode Git and absolute Homebrew GPG, then binds absolute paths/hash/build/probe/source/OS. The failed retained attempt has no completed capability stdout/receipt certifying its actual tool pair. No new tool version or successful availability is asserted.

Retained `target/dream-read-control-state-x6qosf3h/status` is **81**. clang.stderr and stdout.json are empty. stderr.log:3–4 reports **sample140 failed**: conservative helper1,046,000ns + active controller636,500ns =1,682,500ns; measured helper delta2,792ns; late kernel0. Halving the multiplier alone gives523,000+636,500=**1,159,500ns**, still above1ms. The decision does not reinterpret that sample as passing or promise that per-member endpoints alone fix it.

Exact inspected SHA256:

| Bytes | SHA256 |
|---|---|
| src/dream/c1.rs | `05d54fba1c1be5f5cee07215098cecc24117081614e310b62fa5aae90289d150` |
| src/dream/read.rs | `1d60463b70862b578c497d888441b06a971f0f7fa539a7a3ce7efd20ef6597aa` |
| src/dream/resolve.rs | `e1127094bd03c4feddc7ac583ccb989dc7f2dcdda74b51374d94c3260dd78558` |
| source AND frozen probe copy | `6315a34b672cdb597fb4a5a073d59a406dba3849b9b0c67fcb09787c4f7cae59` |
| tests/dream_read.rs | `d320f8cf954ecd254d448941124ae72b30be1d7daf5e0ada0323376b7df9fcd7` |
| Makefile | `a4190b69ebbe2833a7b658ea6232253c2c2e9318832fbd41cc8d1c4797d35468` |
| failed stderr.log | `7c2588c3191653a5506b16ee13ac55e413757fb55969379ed6e8f52147c8b91a` |
| failed status | `5316ca1c5ddca8e6ceccfce58f3b8540e540ee22f6180fb89492904051b3d531` |

Primary-source check (read only, fetched2026-11-05): [Apple proc_info.h](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/sys/proc_info.h) :119–142 defines observed task/thread fields; [Apple proc_info.c](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/kern/proc_info.c) :2164–2174 fills BSD then task data separately. Thus the returned struct alone is not a transactional stop/exec/roster seal. This is an inference from public source, not proof of the local kernel build. No broader OS/tool research, service call or private capability/credential access.

## Required executable evidence

Decision defines finite two-slot window/closure/CPU/debt/incarnation/fork-exec model variables, actions and weak internal control/settlement fairness. Named causal controls cover unequal stops, unknown birth/exec, false blocked-stop inference, controller moved before closure, late CPU, missing/retired coverage, debt reset/overlap, actual CPU>estimate and failed stop/clock/ABA. Actual source/model/runtime witnesses must show those guards cause the intended failures.

Existing branch Makefile:302–317 defines make test-dream-mcp; make test-formal-parity remains the inherited source-bound native/Yarn/TLC gate. Neither ran here. Still required: exact provisioned Git→GPG qualifying publication success and corrupted-proof failure, raw per-unit clock/member/roster/coverage/controller/pipe/debt receipts, four qualifying jobs and unchanged foreground/control/heap/30s/real-reap gates. An unsupported finite mechanism must refuse honestly. Source declarations, samples or design records do not prove these runtime obligations.

## Knowledge readbacks and communication

Full continuations were fetched before updating and after writing. All resulting bodies matched exactly (decision1page/spec2pages/ICD3pages); no truncated suffix was omitted.

| Record | Previous digest → exact readback digest |
|---|---|
| decisions/d93377961365489-dream-q2-member-window-accounting.md | new → `39d74ff3dee59a762d3ec176a7530652578507efacf7189a4f33151f3c547ea1` |
| specs/dreaming-mcp.md | `eafe27fd9e0dea9aa488ddd15dfc581172df2981320d1eabd1d3d402fd5a1432` → `c4b47c58b7152996265dfe49a3cba112a17d2c5fff892c0d20ba444321e8f519` |
| interfaces/dreaming-mcp.md | `04dd34ee61e651a7b1d20e583b36a31cea0a787ca8d8027cfe2c43df9b37ddeb` → `2f9f238035325377db7650a3a531a8ab9ff6d95418cc850c1af6bb06eb9f3409` |

Existing65126 was read fully, digest `e8b872dc5ec5006d19cf33bfea70bd10f1c5aeb357ade6e19ee7fff2722775f2`, and left unchanged. Initial abbreviated lookup was not_found; corrected exact locator succeeded.

Sent exact decision/obligations to Q2 author message **01M6PWF5Z3D005BMR3JV1D1JYW** and Rook **01M6PWF5ZR63QP28SRJ6HSBMB3**. Both stored/accepted; model receipt is not inferred. Two initial requests with an invalid attachment shape were explicitly rejected, with no message effect; corrected requests omitted that attachment. No ACK wait, assignment, scope override or original task amendment was performed.

## Verification and exclusions

`git diff --check` in the exact Q2 source tree exited **0**, no output, at final22:15:12 UTC check. Other executed commands were read-only date/git state/diff-stat, rg source searches, and Python byte reads/hashes, all exit0; knowledge fetch/write/readback and the two official page fetches completed as described. No build, test, capability probe, process launch, model execution, performance measurement, source edit, delegation/recruitment, merge, install, service/account/credential or native app action. Root owns live-task amendments, final-source verification and acceptance.
