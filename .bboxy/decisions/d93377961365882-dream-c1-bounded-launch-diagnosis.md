---
type: decision
title: Diagnose the failed C1 launch without relaxing readiness
derived_from:
- '[[decisions/d93377961365736-dream-c1-canonical-make-worker-tuple]]'
- '[[tasks/2026-11-05-dream-c1-consumer]]'
id: 01M6Q3HRYKR5PJ0DVGPWDAABWC
updated: 2026-11-06T00:18:47.235Z
created: 2026-11-06T00:18:47.235Z
number: 93377961365882
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:a9f66a726b38393858ce4e3e4d5eb71f148c5e94a43c63afda9b874deda7944e
  frontmatter_digest: sha256:1fa0e9abb35f808b7965077c3e08abe889db8adcd6d7daa0d530031f16630f99
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: 4jiql8NJiVD3uBjplT1h09AM+0nwepVjGO/cUU1G8k+fhCoLBpufit/UH0kVkjXy+cK66+9KPCf7uU5ns/auCA==
  log: .bboxy/log/2026-11-06.jsonl#147600
---
# Instrument one private source-free launch, keep readiness failed

Date2026-11-06UTC. Consumer message01M6Q2E9CN6054KBX8GM7C0WRX and exact retained receipt bboxy-dream-c1-gate-qk7qyp0w establish: canonical d06/e003fd/12ba build exit0 with matching before/after closure; prepare overall exit2 at original250ms Hello. Exact child65287 actually reaped exit2. Parent permit was written128bytes18.770–18.793ms after spawnreturn, selector began18.847ms; no frame/early stderr was observed within readiness. Late parent grant is excluded for this sample, not every launch.
Root read hello.stderr: a114-byte dream-bootstrap waiting marker with argv/channel/owner admissions0 was collected after failure. Its emission/entry time is not recorded. It proves the borrowed gate's waiting stage was reached at some point; it does NOT establish dyld delay, timely child entry or a successful permit/Hello. hello.failed-readiness.sample.txt contains only sampling banners, no stack. Cause remains Unproven. Existing failed runs remain failed and must not be repeated unchanged.

## Decision
Within consumer's existing scripts/test-dream-c1.py and Makefile scope, authorize ONE materially instrumented private source-free diagnostic launch of the already-approved exact12ba artifact, through a canonical Make diagnostic target. Preserve original fresh op/run/nonce/birth/context:null,128-byte permit/lease, paid parent debt, pre-birth hash/profile,250ms verdict and actual exact-child settlement. Never send Start/read a manifest/source. No accepted producer source edit, toolchain change, binary substitution, warm-up launch, timeout increase or new runtime capability follows.

Add fixed bounded pre-exec clock/FD/type/access/priority evidence to the existing owned stderr pipe, with an exact marker <=256bytes prepared safely before exec; record monotonic emission versus parent observation distinctly. No arbitrary env/argv/capability dump. Timestamp each first observed gate marker/Hello, selector readiness/EOF/error and immediate lease retirement, keeping stderr <=16KiB. Continue draining only bounded diagnostic output while settling the original failed child; no late output may change its250ms verdict. Avoid a blocking read/diagnostic on the timing owner. One known post-failure sample of the exact retained own PID is permitted with explicit -file into fresh owned evidence and separate actual exit/hash; do not claim a stack if absent, and never delay revocation or reap for it. No desktop, entitlement, broad process discovery/kill, operator input or new product controller.

Diagnostic output must say diagnostic_only=true, runtime_conformance=false, and retain the original readiness verdict even if the extra instrumentation changes scheduling. Do not add DYLD injection/paths/environment flags or alter ordinary worker launch policy. Offline bounded Mach-O entry/load-command/code-signature metadata inspection of exact approved bytes is permitted; it is not execution/timing proof. This is a new measured question, not a lucky readiness retry. If no cause is isolated, retain Unknown and continue independent consumer work; no identical diagnostic loop.

If evidence identifies a consumer launch/observation defect within existing scope, correct it without weakening the current source/CPU/debt/readiness/authority/heap/control/frame/hints/reap law and run the required corrected-source gate once. A producer/build/interface change remains a concrete shared-boundary decision, not implicit authority. Final joined canonical Make gate must use ordinary uninstrumented launch with all current checks and actual readiness success. Root's tuple approval established pre-birth eligibility only, not runtime readiness.
