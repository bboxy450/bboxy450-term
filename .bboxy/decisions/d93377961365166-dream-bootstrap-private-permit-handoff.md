---
type: decision
title: Gate the first controlled Dream worker startup span with an inherited per-job permit
date: 2026-11-05
derived_from:
- '[[tasks/2026-11-05-dream-bootstrap-admission-boundary]]'
- '[[decisions/d93377961365086-dream-native-quantum-and-debt]]'
id: 01M6PMWNX1CH778V5S0TY7J7K7
updated: 2026-11-05T20:34:47.655Z
created: 2026-11-05T20:02:35.394Z
number: 93377961365166
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:ad51e273c70facf498bf414ac5134dc2f08760f584441c0772dd4712a33142d0
  frontmatter_digest: sha256:45963188f9908f121a7fdc3525acff9e45f65029fda7ffff51827261f13a7331
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: Igt8oDmWyLYcRbY2Yif/4RQrEiGCOpY8xVQeu+/r6l5fijfeSfFryh7kzTaU0n0SrDOvzPStWu3hQY8HbTXCBA==
  log: .bboxy/log/2026-11-05.jsonl#142279
---
# Per-job Dream worker bootstrap admission

Date2026-11-05. Resolves [[tasks/2026-11-05-dream-bootstrap-admission-boundary]] under [[decisions/d93377961365086-dream-native-quantum-and-debt]]. Applies to managed C1 and inherited Q2 worker birth, not Q2 Git/GPG helper65126. No public JSON/serde/schema/IR/algorithm/control-version change.

## Evidence / alternatives / choice

Consumer wake-up dream_c1.rs:1290-1359 prepares/spawns the worker; logic/dream_c1.rs:4811-4859 retains the Child, evaluates native verdict, pays parent debt, then reads Hello. Producer at206c8bff6a3746c77bad88d909e610669cee84a5 c1.rs:405-439 first allocates/measures/rests argv/path work, then run():2349-2397 creates the parked compute owner before Hello. Thus child controlled work can already overlap parent process creation/debt. Hello→Start only prevents later source work. Earlier fixed16d evidence is unchanged; no old gate is certified here.

Options: (A) privately gate the first controlled child startup span; (B) admit compound parent/child startup with conservative summed cost and rest. Choose A: it gives one explicit owner transfer without inventing a combined-cost oracle or broad native exception. Local9x clocks alone are not aggregate exclusivity proof.

## Owners / private channel / exact binding

One anonymous pipe per launch, prepared on the existing isolated parent owner in ordinary bounded work; no new thread/process/pool/global gate. Parent owns its write end/Child/effect record; child receives only its read end. Reserve actual descriptor N≥3, N≤65535; inherit that exact fd by clearing CLOEXEC only on it in the existing async-signal-safe child setup. All other copies/ends remain CLOEXEC/closed. Do not overwrite stdio or std::process's exec-error channel. Parent records exact N/pipe identity in its prepared command/launch receipt, retains custody until spawn returns, then closes its extra read copy. Three public protocol pipes remain unchanged.

The sole extra cleared-environment entry is:
BBOXY_DREAM_BOOTSTRAP=1:<N decimal>:<birth ULID>:<binding 64 lowercase hex>
(value≤128 ASCII bytes). No new CLI argument. Reject missing/duplicate/invalid field, unsupported profile, non-pipe/non-readable inherited fd or pipe identity mismatch. A label or token without the actual inherited pipe carrying a valid permit never admits bootstrap. This is scheduling custody, not new public authority.

binding=SHA256(C({profile:"dream-bootstrap/1",purpose:"inspect-c1"|"read-prepare",probe:bool,op_id,run_id,nonce,incarnation,context,worker_sha256})). For a real job Parent uses full already admitted immutable public context/original request identity; a source-free probe uses explicit context:null, not an omitted field. No source/manifest/provider read computes this binding. Incarnation is the existing monotonic per-run host launch incarnation, never a reused display identity. Birth is fresh once for the original owned launch; no replacement birth after an unknown effect. Parent freezes these values before native creation. Child compares pipe bytes with inherited birth/binding and job basename; later Start must match permit run_id/nonce and existing captured host context/manifest rules. Probe has its own fresh original identity and probe=true; both sides permanently refuse ALL Start source-free, including matching IDs. The pipe is no substitute for the public admission fence or source closure. [[decisions/d93377961365255-dream-bootstrap-source-free-probe-context]]: source-free probe=true serializes context:null explicitly in C(binding), never omitted/empty/previous-seat/fixture context. Keep fresh original op/run/nonce/birth, nonzero monotonic host launch incarnation and approved source/binary/compiler/hash/profile; no actor/root authority is fabricated before Runtime::new's capability probe. Probe permanently rejects ALL Start, even matching IDs, without manifest/source/provider read; parent uses paid Hello→EOF→actual reap only. Real probe=false requires full captured current public context and closed-source authority before birth, with no null/missing-proof fallback. This changes only private canonical binding semantics, not public Context/worker/schema/IR fields.

Exactly one128-byte binary permit, no JSON or unbounded decoder:

| Byte range | Fixed v1 representation |
|---|---|
| 0..8 | ASCII BBDBOOT1 |
| 8 | purpose:1 inspect-c1,2 read-prepare; matches exact argv dispatch |
| 9 | probe:0 or1 |
| 10..16 | zero |
| 16..32 | birth ULID's canonical16 bytes |
| 32..48 | run_id canonical16 bytes; matches job basename |
| 48..64 | nonce canonical16 bytes |
| 64..96 | binding SHA25632 bytes |
| 96..104 | incarnation unsigned64 little-endian, nonzero |
| 104..112 | permit sequence unsigned64 little-endian, exactly1 |
| 112..120 | absolute CLOCK_MONOTONIC deadline in ns, checked unsigned64 little-endian |
| 120..128 | zero |

No field contains credentials, capability-file bytes, paths, transcript or signing authority. Bound parsed gate state/record/environment scratch to≤1KiB per side, charged inside existing2MiB protected control shares (not new memory). Expensive hashing/command/pipe preparation remains ordinary≤1ms work with its existing rest. Descriptor inheritance is part of the already enumerated process-create preparation, not a new native class.

## Cutoff / lease / handoff

1. **Child gate is BEFORE first controlled expensive startup work**, specifically before current early_worker_entry's argv_at/argv Vec/PathBuf/Arc construction (206c lines425-438), and before run/channel/Builder work. Borrowed argc/argv recognition, bounded environment/fd parsing, one fixed permit buffer and sleeping pipe wait are the source-free control prelude. Non-macOS entry must reach this same gate before runtime/CLI control-thread creation; unsupported entry/platform refuses before that controlled work. Do not gate only Builder/run while leaving the earlier argv span free.
2. Parent reserves/pays ordinary preparation, admits exact process_create, retains actual Child/PID/start/group/nonce/effect before any clock/verdict. While child waits it may perform only bounded control bookkeeping. Parent evaluates full inclusive spawn/application cost and pays9*max(1ms,C) following span end, including required pre-permit setup/identity work and its debt. Then recheck current public fence, incarnation, foreground, cancel and deadline.
3. Parent transfers expensive-work ownership **before** the permit write; no subsequent parent snapshot/proof/journal/hash/native unit while child bootstrap owns it. Write exactly128 bytes once on the preconfigured nonblocking write end, then **keep that end open as a revocation lease through Hello**. Only full successful write admits a grant. Partial/error/EINTR/unknown write consumes that attempt, closes the lease and settles; no retry with a new record/token/worker. The single-writer empty pipe has no prior data. Child collects≤128 bytes in at most4 reads, rejects malformed/extra data, and admits exactly one bootstrap.
4. Child validates the permit, deadline and live lease before the argv span and every following controlled bootstrap application/native admission, including after debt waits. Extra bytes/EOF/HUP revoke admission; check without consuming unbounded data. Duplicate/delayed-invalid permits cannot start another bootstrap. An in-flight bounded native call cannot be recalled; retain its effect then settle. Existing bootstrap application≤1ms, compute_owner_create≤2msCPU/25mswall with inclusive≤3ms and one2MiB parked owner remain; each span pays9x debt. The parked thread's fixed source-free receive/exit bookkeeping is measured startup overhead, not source work or a second expensive owner.
5. Only after ALL child bootstrap controlled work and its debt are finished may the child retire its gate read end and emit ordinary Hello, parked before Start. No later expensive bootstrap/source work until Start. Parent accepts Hello only for the pinned bootstrap-capable source/binary/profile, then closes the write end and transfers expensive ownership back to parent. Hello alone from an old binary is insufficient. Parent may now perform existing bounded snapshot/proof/receipt work while child remains parked.
6. Before sending matching authorized Start, parent retires its expensive metadata/snapshot owner and pays its remaining debt, rechecks current context/foreground/deadline, and transfers work ownership to child. Start's bounded framing/write and corresponding receive are control bookkeeping. After Start, parent remains control/status/cancel only until child work+debt+real settlement hands ownership back. Existing completion/currentness/output validation rules still govern presentation.

**Finite bookkeeping, not recursive grants:** permit recognition/read/write, lease checks, Hello/Start framing/ownership transitions and bounded refusal/close are measured≤1ms control operations on existing off-UI owners. They use protected control storage and their CPU is included in retained control/full process usage; they do not recursively create another expensive-work9x debt/grant. No source IO/hash/serde scan, command preparation, compute creation or cleanup loop may be relabeled bookkeeping. Waiting uses blocking poll with remaining deadline, not spin; allow at most4 interrupted pre-permit waits/reads before honest refusal. Post-permit checks are fixed per actual bounded bootstrap span. Existing per-handler1ms/latency gates and startup/control totals remain visible, with no claim whole-process/control CPU is≤10%. All actual expensive spans still repay full9x debt and never overlap across the handoff. [[decisions/d93377961365259-dream-bootstrap-final-public-admission-control]]: only the two final handoff attempts (permit and authorized Start) use the PREPARED fresh public-authority callback plus tiny transition/write as one inclusive≤1ms CONTROL step, without recursive9x work debt. Use the existing isolated admission owner/protected reserve and independent Status/Cancel; no SQLite/IO on hot/message/UI/reactor owners. Allow only≤7 fixed public rows, each≤16KiB bounded BEFORE fetch/hash (≤112KiB digest input), exact bounded actor scalar/primary-key predicates, account-file/inode/revision/time/issuer/head/grant/device and nonblocking provider/session currentness, including post-transaction checks. Connection/statement/key/snapshot and binding preparation remain ordinary work before debt payment. At most one final check per actual handoff attempt; no polling, cached/atomic-only permission substitute, stale reuse across await or changed accounting for other guard calls. Unknown/busy/oversize/clock/>1ms refuses/revokes and retains real Child/pipe/lease effects. Probe has no fake final actor proof. No source/payload/binary hash, generic SQLite/serde scan, credential/signature/provider process or blanket helper exception; all previous limits/settlement stay.

This gate controls application-defined expensive work, not arbitrary dyld/kernel/allocator/thread-entry instructions that can execute during process/thread birth. Retain measured pre-permit control CPU, complete owner/process usage and native startup overhead with coverage limits; no all-times OS preemption or whole-process10% claim. No recursive gate inside pthread startup is required.

## Time / failure / compatibility

Reuse the existing250ms Hello/bootstrap readiness window from spawn return; do not restart it at permit release, reset it across partial reads, or increase it. Child pre-permit entry wait is independently≤250ms; effective permit deadline is min(entry+250ms, parent readiness deadline, remaining original job deadline). Parent writes absolute monotonic deadline after measuring spawn return; clock failure/reversal/overflow refuses permit. Source/preparing30s wall deadline and probe timeout/≤1-account cap remain unchanged. Scheduling uncertainty may fail readiness honestly; not a loop retrying until a fast sample.

| Event | Required custody/disposition |
|---|---|
| Parent native timing/clock failure, unknown creation result or stale authority | Never issue permit; close only owned lease, retain actual returned Child/process/effect if any and settle/reap, otherwise unknown-owned/quarantine. No second spawn/token/debt reset. |
| Foreground/cancel/revoke/deadline while waiting or bootstrap | Independent parent Status/Cancel remains usable. Close lease and stdin/normal cancellation route as applicable; prohibit new child spans at the next boundary. Before Start no source was read. Foreground before grant may wait within the same readiness/job deadline; after grant revoke rather than pretend bootstrap was preempted or resend a permit. A later explicit run is a new authorized operation only after original settlement. |
| Parent EOF/crash/restart | Kernel closes writer; child refuses/settles, including actual join of any parked owner already created. New daemon cannot reconstruct/reopen the old anonymous pipe or adopt its permit. Existing interrupted/quarantine/original-op and slot rules apply until actual exit/effects known. |
| Bad/stale/duplicate/expired permit, unexpected bytes, pipe not inherited, EOF before/full permit | No additional controlled admission; fail framing/bootstrap with existing usage/protocol failure semantics, no Hello success/source/manifest read. Preserve any already-produced thread/process effect and original identity; actual exit/reap before release. |
| Child does not return/exits without qualifying Hello | Parent remains bounded control/settlement only; exact owned2s+2s termination/escalation and quarantine, never fake Hello/paid handoff. |
| Matching Hello then authority/foreground change | Keep child parked, withhold Start/expensive parent admission, use existing cancellation/settlement. Equality of old names A→B→A never revives a permit or context. |

Host must know bootstrap support **before executing a candidate binary**, from its approved packaged/source/binary/compiler/features receipt declaring dream-bootstrap/1. Gate-capability detection cannot safely start an unknown/old worker and hope it waits. Existing≤1/account source-free probe uses the same pipe/permit/debt/Hello/EOF/reap path and cannot enlarge authority. No separate installation recipe, runtime download or new public protocol version. Missing profile/hash compatibility disables managed Dream with runner_unavailable; ordinary seats/legacy Dream remain usable. A new inspect-c1/read-prepare worker launched manually without this private gate refuses before expensive startup; tests/private standalone launchers must provide the same gate. No ungated compatibility fallback. Ordinary-release worker versus instrumented fixture/probe artifacts remain distinctly hashed/source-bound.

## Exact glue / verification

Producer src/dream/c1.rs owns the private permit parser/wait/lease guard, earliest dispatch cutoff, source-free bootstrap admissions and paid-Hello transition; reuse existing accounting/owner/cleanup. A small adjacent private module is acceptable only by Oracle's allowed-path amendment. It owns the private128-byte fixture/layout. No change to c1_wire.rs/schema/IR/algorithm. Consumer wake-up src/dream_c1.rs owns prepared pipe/inheritance/lease/Worker custody and mirrored fixed-layout conformance; mux logic/dream_c1.rs owns original launch binding, public guards, full parent verdict/debt, owner transfer, revoke, paid-Start and retained receipts; catalogue/registry retain existing run identity, no new truth store. Canonical Make/source receipts add profile/conformance witnesses within existing test-dream-c1/provisioning.

Q2 c1.rs::run<J>/ParkedCompute, worker.rs facade and read.rs early path must consume the same producer-owned gate before prestart; its outer parent uses the identical handoff. Do not copy another gate/controller into selected Git/GPG proof logic. Oracle amends live components; original accepted task stays frozen.

Finite model extension: birth=absent/owned/unknown, permit=none/partial/valid/revoked, bound incarnation A1/B2/A3, cost_known, parent_debt/child_debt0..27, expensive_owner=parent/childBootstrap/parentPrepare/childWork/settlement/none, child_waiting/bootstrapped/hello/start/exited, current/foreground/cancel, one permit attempt and one child. Initially no child/permit/Hello/Start, parent owner/debt0, current source closed eligibility assumed only at admission. Actions Prepare/Birth/RetainEffect/ParentVerdict/PayParent/Grant/ReadGrant/ChildSpan/PayChild/Hello/ParentPrepare/PaidStart/Revoke/EOF/Restart/ObserveExit. Weak fairness only internal control/refusal/rest/settlement, not OS return or delivery. Unknown effects retain slot. Model guards: no child expensive span before valid current permit+paid parent debt; no parent expensive span during child bootstrap/work; no Hello before paid child debt; no Start before paid parent preparation; no stale/duplicate grant or unknown-effect retry; controls enabled throughout. Broken controls bypass gate, admit argv/Builder before gate, overlap parent, grant with unpaid/missing cost, reset debt, reuse A1/pipe/nonce after A3 or release unknown birth.

Real required witness holds parent immediately after actual spawn return, before cost verdict/debt/permit: child is alive but performs no controlled argv/channel/compute-owner span; other jobs/status/cancel remain responsive. Then qualifying paid permit→child bootstrap/debt→Hello→parent preparation/debt→Start→completed/real reap. Add delayed/partial/duplicate/stale/revoked/EOF permits, foreign fd/token/expired clock, parent crash/restart, native-clock-failed real Child, unknown effects and probe negatives, plus a deliberately bypassed earliest gate that the witness rejects. Use source-bound test markers/retained owner receipts, not output silence alone. Existing real4-job/multi-MiB/overflow/control/native-frame/foreground/heap gates remain; no fake worker/threshold increase/operator input. Models prove the declared handoff only; actual OS/CPU/wall/heap/artifact behavior remains process evidence. No tests ran for this decision.
