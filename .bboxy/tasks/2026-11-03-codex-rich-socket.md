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
updated: 2026-11-03T19:07:44.139Z
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
  digest: sha256:a7cefdb32cef70a34a8bf3b646a035f9a952e7d4ecde0e75840637a0a5d22690
  frontmatter_digest: sha256:4daa31723d97142d99bad1a8cbaa4fa7a3b5b97dbf147b33b1e69984d3d0919c
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: 4ICnss+xua9IAUAczM2bAAkhnBOtQ7UGAx8LFwmJM5hJxdyUHaGMInenfWvdRt/1zoVcHQ+L2IWhvaJqaUnYBw==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#90975
---
ROLE: EXECUTOR. Direct operator continuation under the Codex Oracle: the harness pane is still the line-mode frontend; implement the shared native rich face. Work ONLY in /Users/feral/mydev/bboxy450-harness-pane-b12, current branch pane/b1-b2. The previous session was interrupted with 20 source files already changed. Preserve and inspect that work; no reset, stash, broad stage or deletion. Baseline hashes are /tmp/bboxy-oracle-takeover-01M6H97SAX3GN1BYB4YG76P3P0/preservation-before.json. Read current AGENTS.md, ATTRIBUTION.md and the ICD FROM origin/main (the shared root checkout is old): .bboxy/interfaces/rich-pane.md, gateway.md, embed-seat.md and log-ir-v1 spec. Rebase/integrate current main safely after committing/recovering the interrupted source. Leave original branch tips reachable.

Outcome: Side B §2/§3/§9 of rich-pane: seat --socket binds mode 0600 plus the per-session socket; switch rebinds, resume/restart reconnects, exit unlinks; session/epoch fencing and deterministic op IDs prevent duplicate or misrouted turns. Bridge::connect feeds the same Mailbox/FrameState on bbt_rt; Unix and InProc produce the same frame/layout. Keep the standalone and embedded face shared.

Use the existing uncommitted tests/support/seat_socket.rs, seat_process and gui_bridge work as evidence, not as proof. Extend formal contracts/model plus broken controls for lifecycle, cancellation, stale/duplicate completion and A→B→A. Explicitly record timing/fairness bounds and implementation-only behavior. No operator click, real credential, Keychain prompt or OS input; scripted/private identities only. Readiness/admission must remain honest.

Do not recruit, push, merge, publish or install. Preserve metadata/raw logs outside Git; signed source/report commits and exact path staging. Full report .bboxy/reports/2026-11-03-codex-rich-socket.md with Report trailer, file:line mechanisms and actual exits. Register report through knowledge tools for task_report. Canonical verify is RUSTC_WRAPPER= make test; also run GUI/bridge/socket process witnesses and current native/Java formal parity. One full make test at a time machine-wide: inspect running canonical gates, continue other useful checks while one is active, then self-serve the free gate; never wait for operator approval or end the turn merely waiting on a background command.

Coordinate with the Codex rich-host task for term Side A and the standalone recovery task. Send a short socket schema/ready commit handoff when the consumer can test; it is feedback, not scope expansion. Report once done, or a real all-work blocker with evidence.