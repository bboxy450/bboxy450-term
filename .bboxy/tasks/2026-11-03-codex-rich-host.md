---
type: task
title: 'Host the shared rich harness view in term: socket delivery, resume and native pane'
status: claimed
role: executor
assignee: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
delegated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
swarm: 01M649102FBS60QWNPZS5VAGF9
worktree: /Users/feral/mydev/bboxy450-term-codex-rich
allowed_paths:
- crates/bbt-mux/**
- crates/bboxy450-harness-wake-up/**
- crates/bbt-proto/**
- crates/bbt-ui-model/**
- crates/bboxy450-term/**
- crates/bbt-ui/**
- tests/**
- verification/**
- scripts/**
- Makefile
- Cargo.toml
- Cargo.lock
- ATTRIBUTION.md
- .bboxy/reports/**
- .bboxy/runs/codex-rich-host.json
verify: make test-parity
parent: 01M6HCH1SJHDSB6SDAC2ZF1BXX
id: 01M6HCYDWBAX2FDYGMSPB12HRN
updated: 2026-11-03T19:07:49.037Z
created: 2026-11-03T19:07:32.055Z
number: 15252
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
provenance:
  digest: sha256:f2d58dc8e900291765a3658ee68ceed63f423a0445f61448b989bb11f1cbaf67
  frontmatter_digest: sha256:9b343eb088d1ff92eb50fbb85ae1c7e9d669a88421cf49014da7831e91fca3df
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: fvVyxlxp9JkCVtwuEvDx/9KLv68c/w0VifR1Vvw/sLiCAsoyikVott6DeZQasAs/rYn/NfpaMhI7oL7ujLheAQ==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#90987
---
ROLE: EXECUTOR. Work ONLY in /Users/feral/mydev/bboxy450-term-codex-rich, branch codex-rich-pane-20261103, created from origin/main 1affb986. The operator's screenshot is a PTY line-mode harness, and asks for the standalone native harness face embedded in term. Implement term Side A of the existing rich-pane ICD, preserving the session process and sharing HarnessView rather than building another renderer.

Read own AGENTS.md/ATTRIBUTION.md/vision, .bboxy/interfaces/app-harness.md, harness origin/main .bboxy/interfaces/rich-pane.md (Side A §2–§9), d5977 dependency direction and the binding A3/A4 brief at /Users/feral/mydev/bboxy450-term-wt-architect/.bboxy/tasks/2026-11-02-rich-pane-a34-delivery.md. That brief defines the delivery phase; this direct operator task additionally authorizes the following A1/A2 native pane phase. Complete the delivery boundary first and then the view. No global settings, credentials, OS clicks/keystrokes or operator screen capture.

A3/A4: term-chosen <=96-byte socket path in launch argv; async per-seat connection, Hello/session/epoch tracking and persisted session ID; generation fencing and reconnect; seat_input_text/composer submits idempotent UserTurn over Unix BEFORE PTY arbitration, correct Ack/Busy/Closed/unknown outcomes, zero PTY bytes. Add bboxy450-harness resume admission to seat_spawn with validated session format and a real interrupted/resumed process witness; the harness CLI already supports seat --resume. Read the ACTUAL current origin/main code, not stale line numbers from handoffs.

A1/A2: per-pane Terminal/HarnessRich face; HarnessRich for announced harness sessions; render the shared HarnessView Frame with the existing renderer in the pane, forward its composer/stop/approvals over the socket, persist the face preference, keep PTY ownership and detach/reopen semantics. Missing socket honestly shows starting/unavailable and the Terminal toggle. No UI/logic-thread IO. Keep the explicit aggregate drain/projection/memory budgets in the ICD; measure and label remaining limits honestly.

Coordinate with rich-socket seat 01M6HCVV32KS845WHVD5FCX7QE for B1/B2 and with standalone recovery. Implement against private scripted Unix fixtures immediately; do not park waiting for a peer. Test real integrated B1/B2 when available using a pinned candidate with one toolkit copy; record the exact harness commit. Do not move the committed submodule pin to an unpublished commit: report the required pin to the Oracle. Term main currently has a layering failure after the 0c40316 pin; inspect and reuse the already-prepared signed fix dfb21967c0157aebcadec5b028d300abd86203ce if applicable, with a real layering check. Do not weaken the layering gate.

Verification: formal-first state variables/bounds/owners/atomic actions, safety/liveness/fairness; good and broken controls for delivery duplication, stale identity A→B→A, reconnect/detach, and routing. Private scripted process witness for each ICD acceptance clause, zero PTY byte assertions, resume witness, and an offscreen capture of the same fixture in standalone vs rich pane with row mismatch <=0.01. Run make test-harness-native, make test-boundaries and make test-parity on the final candidate, using the canonical targets and current-source native/TLC formal evidence. Full machine gates run one at a time, self-service with process evidence; no operator input.

No recruitment, push, merge, publication or trusted install. Exact staging, signed commits, preserve existing source. Full report .bboxy/reports/2026-11-03-codex-rich-host.md with Report trailer, harness hash, file:line mechanisms, real exits, budget/parity numbers, negative controls and gaps; register through knowledge tools and task_report.