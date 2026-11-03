---
type: task
title: 'Recover rich-pane B1/B2: seat socket lifecycle and Unix bridge'
status: claimed
role: executor
assignee: seat:01M6HCVV32KS845WHVD5FCX7QE
delegated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
swarm: 01M649102FBS60QWNPZS5VAGF9
worktree: /Users/feral/mydev/bboxy450-harness-pane-b12
allowed_paths:
- src/frontends/**
- src/gateway/**
- src/main.rs
- tests/**
- verification/**
- scripts/**
- Makefile
- ATTRIBUTION.md
- .bboxy/reports/**
- .bboxy/runs/codex-rich-socket.json
verify: RUSTC_WRAPPER= make test
parent: 01M6HCH1SJHDSB6SDAC2ZF1BXX
id: 01M6HCYCQB66JTBKPHE1YE4QJB
updated: 2026-11-03T20:45:18.183Z
created: 2026-11-03T19:07:30.619Z
number: 15249
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
provenance:
  digest: sha256:9b6a7548a4c6748dc5949c112218d8995532f0e4c4c84c408d1e73322c68b4d1
  frontmatter_digest: sha256:6fb96150357b39ec9271bb93c3226aa505d846fb65d8c935067f17dd3cce210e
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: rchtQDEflZTZn2tBoizUdE1zGRToP3tNTHG6h3tRGEEZnNVWUyofQyAsUiGEfCAUYE5KxadXo+Ls1nwhA/4WDw==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#100165
---
ROLE: EXECUTOR. Direct operator continuation under the Codex Oracle: the harness pane is still the line-mode frontend; implement the shared native rich face. Work ONLY in /Users/feral/mydev/bboxy450-harness-pane-b12, current branch pane/b1-b2. The previous session was interrupted with 20 source files already changed. Preserve and inspect that work; no reset, stash, broad stage or deletion. Baseline hashes are /tmp/bboxy-oracle-takeover-01M6H97SAX3GN1BYB4YG76P3P0/preservation-before.json. Read current AGENTS.md, ATTRIBUTION.md and the ICD FROM origin/main (the shared root checkout is old): .bboxy/interfaces/rich-pane.md, gateway.md, embed-seat.md and log-ir-v1 spec. Rebase/integrate current main safely after committing/recovering the interrupted source. Leave original branch tips reachable.

Outcome: Side B §2/§3/§9 of rich-pane: seat --socket binds mode 0600 plus the per-session socket; switch rebinds, resume/restart reconnects, exit unlinks; session/epoch fencing and deterministic op IDs prevent duplicate or misrouted turns. Bridge::connect feeds the same Mailbox/FrameState on bbt_rt; Unix and InProc produce the same frame/layout. Keep the standalone and embedded face shared.

Use the existing uncommitted tests/support/seat_socket.rs, seat_process and gui_bridge work as evidence, not as proof. Extend formal contracts/model plus broken controls for lifecycle, cancellation, stale/duplicate completion and A→B→A. Explicitly record timing/fairness bounds and implementation-only behavior. No operator click, real credential, Keychain prompt or OS input; scripted/private identities only. Readiness/admission must remain honest.

Do not recruit, push, merge, publish or install. Preserve metadata/raw logs outside Git; signed source/report commits and exact path staging. Full report .bboxy/reports/2026-11-03-codex-rich-socket.md with Report trailer, file:line mechanisms and actual exits. Register report through knowledge tools for task_report. Canonical verify is RUSTC_WRAPPER= make test; also run GUI/bridge/socket process witnesses and current native/Java formal parity. One full make test at a time machine-wide: inspect running canonical gates, continue other useful checks while one is active, then self-serve the free gate; never wait for operator approval or end the turn merely waiting on a background command.

Coordinate with the Codex rich-host task for term Side A and the standalone recovery task. Send a short socket schema/ready commit handoff when the consumer can test; it is feedback, not scope expansion. Report once done, or a real all-work blocker with evidence.

Final-source checkpoint, 2026-11-03: recovery is published through origin/main af65c6cf82d9dde6314b1771b9314061b0e24212 (ea75307 implementation plus indexed report/summary). The Oracle independently verified all 270 canonical input hashes, signed commits, native/Java receipts, signed installation and installed private startup smoke. Your 46190c6 source predates that recovery and is preliminary. Preserve the existing run/results, then integrate/rebase published main before final canonical verification; keep quiet Native/Usage geometry, phase heartbeats, IR1.2 origin/Quiet semantics, startup bitmap alias and all controls alongside B1/B2. Do not report/land the older code as the final combined candidate.

ICD §9's budget witness is included in this task's existing scope: extend tests/gui_budget.rs to four concurrent views on one injected clock; aggregate drain <=0.3 ms and projection <=0.6 ms with no dropped admitted work, including a deliberate budget-tail guard failure and restored green. Per-view timing alone is not this witness. Coordinate the term bench row with rich-host; label actual measurements and unmeasured margins accurately. A new Codex native-visual task (seat 01M6HHK04M27ANJ2YPTBSC4B0Y) owns only the two carried gui/face.rs presentation defects and two audit passes; no Bridge/seat/gateway/IR changes. Continue useful work and give a short ready signed source handoff; no operator wait.