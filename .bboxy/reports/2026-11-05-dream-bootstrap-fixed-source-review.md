---
type: report
title: Fixed bab543e bootstrap interaction review and retained gate disposition
task: '[[tasks/2026-11-05-dream-bootstrap-fixed-source-review]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T21:59:40.810Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6PVK2AJ55JJPJZNB1B9NH4K
updated: 2026-11-05T21:59:40.810Z
number: 93377961365404
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:3e94b63c164c5a313aad43e1b30a0ad84214db4d72bdfcba5b13c1c32efa3119
  frontmatter_digest: sha256:26e1c8e01f1130eac2b40641665a9b68c52b5daaf413b4ae276a6b41f8b99517
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: zqitaa+2Wy9pXgp7yODBpWHB2GSe60VyI8dGly0BW3teFsxrpIo6AZOU9eC3z5JkMRr+B3OpgWHwBIwxKSqGBA==
  log: .bboxy/log/2026-11-05.jsonl#143953
---
Task: [[tasks/2026-11-05-dream-bootstrap-fixed-source-review]].

Fixed-source review completed; this report does not accept the producer, approve a production pin, or certify the consumer. The principal finding is a failed required causal process control in the exact retained bab543e gate. Later correction evidence reported by Rook is recorded separately.

## Cutoff and method

Reviewed Git objects `bab543e2116d0bf08488fbd753a6114f30d615a4` against `206c8bff6a3746c77bad88d909e610669cee84a5` in `/Users/feral/mydev/bboxy450-harness-dream-native-c1`. Governing contracts: [[decisions/d93377961365166-dream-bootstrap-private-permit-handoff]], [[decisions/d93377961365255-dream-bootstrap-source-free-probe-context]], [[decisions/d93377961365259-dream-bootstrap-final-public-admission-control]], [[decisions/d93377961365086-dream-native-quantum-and-debt]].

Read-only Git/source/log/hash inspections; no build, test, Python conformance, formal replay, signature verification, source/spec edit or merge. Root supplied signature verification; I did not repeat it. Initial local observation at 21:48:17 UTC and final check at 21:54:38 UTC found HEAD `56ac567cc0149c6c62202e91ba47e8b5ffe00c2f`; only generated `.bboxy/runs/formal-native.json` was dirty. Substantive findings below concern fixed bab543e objects. Later clock-propagation/fixture changes are outside this certification cutoff.

Actual verification: `git diff --check` in that exact worktree exited **0**, with no output. This checks the then-current tracked working diff, not a product gate or the validity of the fixed commit.

## Findings and disposition

**F1 — actual failed required witness; not a demonstrated ordinary-source gate bypass.** `target/dream-c1-native-evidence-bab543e2/exits.json` records `make test-dream-c1` exit **2**, source bab543e, elapsed 260.681 seconds. Its `test-dream-c1.log:452–474` runs the distinct test-fixtures earliest-bypass artifact, then fails at `tests/dream_c1.rs:616`: the waiting marker was absent after 251 ms, the actual child remained alive, and no partial marker was collected. The test never reached the assertions that must detect the deliberately allocated pre-permit work (`tests/dream_c1.rs:2360–2410`). Accordingly that canonical run does not prove the causal negative, even though its preceding positive tests passed. The reason for the missing marker is not established by static inspection.

Sent exact evidence and bounded disposition to author and Rook: messages `01M6PV4QG4B9GZ17BBGZB4W51B` and `01M6PV4QH6YTB30WWAGHHFAPVQ`. Requested an actual mutant-marker/rejection/reap witness under unchanged 250 ms positive bounds, with final-source logs; no retry-until-lucky or approval wait. Tool storage is not proof of model receipt.

Subsequent Rook message `01M6PVCF8XA736X9VRFTB9FSXJ` states that signed 56ac567c changes the private invalid-child fixture to revoke/no-grant, actually reap, collect a bounded marker and apply the unchanged checker with `readiness_certified=false`; its later retained target reportedly exits 0 and observes `argv_admissions=1`. Rook independently consumed F1 and recorded [[notes/dream-bootstrap-bab543-interface-milestone-2026-11-05]]. I have not certified that later source or log. F1 remains the honest bab543e verdict; the later disposition belongs to root's final-source review.

**Metric coverage limit.** `c1.rs:477–490` measures and rests the pre-run argv/path span, before `run` creates its Accounting state around :2391. Its maximum is retained and its rest is physically performed, but the run-local `span_cpu_ns/debt_total_ns/rest_observed_ns` counters must not alone be described as summing every early startup span/rest. Full process usage and these scoped counters have different cutoffs. Likewise paid-Hello diagnostics capture before their own emission: `bootstrap.rs:531–554` observes emission and reader close afterward and returns that usage for terminal/control accounting. Neither an immutable pre-seal descriptor nor an earlier diagnostic can include its future publication by self-reference.

No additional ordinary-source admission bypass was demonstrated in this fixed-source review. This is a static conclusion with the process/formal limits below, not a universal scheduling, allocator or OS proof.

## Boundary review

All file anchors in this table are relative to the fixed producer tree.

| Boundary | Actual mechanism and reviewed limit |
|---|---|
| Canonical private shape | `src/dream/bootstrap.rs:37–101,131–222`: bounded token, exact 128-byte codec, fixed purpose/probe/reserved bytes, nonzero incarnation, sequence 1 and bounded identities. Binding serializes explicit `context:null` for probe and rejects absent real context. |
| Host versus child authority | `bootstrap.rs:214–222,748–763`: child binds supplied run/birth/digest/purpose; host-binding fixtures separately compare expected FD/nonce/probe/incarnation/birth. An opaque digest does not recompute host public authority or authorize discovery. Structural/static-deadline vectors are not runtime lease/clock witnesses. |
| Earliest gate | `src/main.rs:9–16`, `c1.rs:405–506`: borrowed recognition/token/job basename precede allocation of argv/path/Arc; Gate wait and first admission precede that first controlled expensive startup span. Unsupported entry refuses rather than using a late async fallback. Loader/kernel startup remains separately observed, not preempted by this local gate. |
| Actual reader custody | `bootstrap.rs:266–310,370–497`: one transferred OwnedFd; read-only anonymous pipe, full u64 inode/device stamps, bounded fixed storage, nonblocking poll/read, finite partial/EINTR handling and original 250 ms/deadline checks. Darwin read/write endpoint stamps are independently checked, not incorrectly equated. |
| Revocation and control | `bootstrap.rs:498–529,559–629`: monotonic/identity/EOF/extra-byte checks, per-span admission checks, bounded borrowed environment/argv recognition. Fixed diagnostic storage and private-pipe-only nonblocking writes avoid changing an operator terminal. |
| Debt to Hello | `c1.rs:2386–2490`: admitted setup, exact native Builder span, actual ParkedOwner retained before verdict, debt payment, frame/Hello preparation, reader retirement and usage merge precede Hello emission. Gate checks follow rest and precede controlled spans. Parent is expected to remain source-free until the paid Hello handoff. |
| Probe and Start | `c1.rs:2538–2547`: probe cannot accept any Start; real Start requires matching run/nonce, bounded timeout and job identity. Compute owner is parked on its channel until Start. Unknown/failing creation retains its actual owner for disconnect/join; no replacement worker or lost handle is authorized. |
| Existing native/cleanup law | Compared the c1 changes with 206c: native/debt accounting, finite-tail owner-result handling, cleanup/credits and descriptor capture remain in the interaction path. Public C1 wire/schema, IR, algorithm and source inspection are unchanged by this bootstrap diff. Later intermediate-clock propagation corrections at 56ac are not covered here. |

## Real witnesses versus claims

The matching log is **`target/dream-c1-native-evidence-bab543e2/test-dream-c1.log`**, not the older mutable generic evidence path.

- Lines 95–180 contain seven passing bootstrap process tests. Actual fixture code `tests/dream_c1/process.rs:219–574` covers held-parent birth/marker, missing environment, malformed/partial/extra/duplicate/expired/revoked records, hostile probe Start, wrong nonce, unknown parent clock/stale binding, closed lease after Hello, foreign FD and refusal/reap paths.
- The held-parent fixture has real child/PID custody, pre-established independent probe and bounded status checks. It is process/API coverage, not a real admitted outer MCP/public-authority/foreground UI result. Unknown-clock and stale-binding injections do not by themselves prove production host ABA.
- `process.rs:54–177` bounds output and observes actual exit; absence of Hello is supplemental evidence. Positive source-bound admission markers and the deliberate allocation mutation are the causal evidence, and the latter failed in F1.
- Log :217 reports **200 actual primed control round trips**, drained initial pause, ignored-revision timeout and delayed fresh-activity control, **p99 27 µs/max 30 µs**. Following entries show four ordinary-release multi-MiB workers complete and reap; :366 reports **30 passed, 0 failed** for that integration invocation. These are real subresults, not a green overall target.
- Log :458 names the separate test-fixtures bypass binary SHA `5accb56e25742a459309607e5235887a7f25bf1c4bb49211a401063a2e02198e`. It cannot stand in for the ordinary artifact.

Reviewed `tests/formal/dream_c1.rs:860–1290`, registrations and source mapping: finite birth/permit/known-cost/debt/Hello/Start/lease/probe/context/incarnation actions and 17 causal controls. Mutants explicitly target earliest bypass, parent overlap, unpaid grant/Hello/Start, unknown cost/effect release/retry, debt reset, stale ABA/duplicate/revoked grant, probe/null work and control cost/payload/retry. Nonvacuity and named-property failure assertions are present. The finite abstraction does not prove OS timing, bytes, heap, exit fairness or arbitrary Rust. Three modeled controlled startup classes do not substitute for all four concrete admission counters.

No matching bab543e bootstrap formal receipt was located in inspected retained evidence. The two inspected older receipts bind 206c8bff/f1b6 source and contain zero bootstrap models. Source assertions and author-reported targeted model success remain distinct from a retained current-source native/TLC gate. I did not rerun them.

## Source and artifact pins

SHA256 of exact Git bytes / retained files:

| Item | SHA256 |
|---|---|
| bab bootstrap.rs | `1c69a2d42142cf046075d97148e2317d0c9594292c24da411be0a44c5225e555` |
| bab c1.rs | `64c2304570bc47c96dae2bc99d0e7122b02f474dd1340f1cd08f0ed5dc803e56` |
| bootstrap vectors.json | `de919016ac6e5a464de60c86b2bf3bf6b54edc08e29c05f89688a5c80da37307` |
| bootstrap process.rs | `82171dbf2bca52baf186b3e5bca13a008dde88324088a20139d626e70fea50c1` |
| formal dream_c1.rs | `8d2e6f48049364d3d06c0fa02f4087edf6f454d6350d10f28ca776e14b880d2b` |
| matching gate log | `b5697e01e3042760fa88fa3b154af64e71195ff0e1e4bc1aafb68c44994ea635` |
| matching exits.json | `ede848582b5e15011aba243f9707a714abc02d42918c088633fbd0b8f01ace80` |

Ordinary immutable binary: `target/dream-c1-native-profile/worker-bacace4bf5e2ab4d4c91bf07e5e22c81885d967448dff87efbe6eaf6f81ca13e`, actual SHA256 equals suffix, 26,224,320 bytes. Frozen bab profile/source/compiler/hash/support receipts are under the matching evidence directory's `profile/`. Log :3–12 builds before testing with offline/locked release/default acp-agent and no test-fixtures; compiler rustc 1.99.0 (b940084d7), aarch64-apple-darwin/LLVM 23.1.1; support `dream-bootstrap/1`. Fixture `tests/dream_c1.rs:325–400` verifies hash/stamps before birth. This is private fixture provisioning evidence, not the term production authority verifier. The mutable worker alias and later ef7 artifact are excluded.

## Handoff limits

Reviewed the requested parser/lease, early admission, debt/Hello/Start, parked ownership/reap, private artifact custody and causal test/model interactions. Sent F1 promptly; root owns final disposition. Required remaining evidence is a final-source complete canonical gate, successful causal bypass witness, matching formal receipts and fresh ordinary artifact/profile bindings. Parent authoritative guard/identity, independent admitted MCP controls, foreground/native-frame/combined heap/hints and consumer conformance remain consumer-owned. No universal OS preemption, whole-native-memory or production-authority guarantee follows from these producer fixtures.

Changes by this task: this knowledge report only; commits: []. No source, submodule, service, account, credential or running product changes.
