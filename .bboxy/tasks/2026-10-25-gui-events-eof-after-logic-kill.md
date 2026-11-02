---
id: "01M5T135QBH6Q401EYJEC9TP3Z"
type: "task"
number: 330
title: "Diagnose durable GUI events EOF after logic restart"
status: done
role: "executor"
owner: "Sable Embertail"
verify: "cargo test -p bbt-mux --locked --test gui_events durable_gui_survives_real_logic_kill_keeps_unread_and_services_pty_controls -- --exact"
created: "2026-10-25T17:17:52.235467Z"
updated: "2026-11-02T18:30:00Z"
updated_by: "seat:01M6EQFA2G00BFT3TVPY6AMXE6"
reconciliation: [{"by": "seat:01M6EQFA2G00BFT3TVPY6AMXE6", "disposition": "cancelled", "evidence": ["GitHub #93"], "fresh_accepted": false, "kind": "historical", "previous_status": "open", "reason": "no longer reproduces: the test runs un-ignored in every gate and passed (bboxy450-term-error-triage/target/ci-local.log, 2026-10-30). Logic-kill recovery is tracked in GitHub #93. Hand-edited: task_reconcile refused (see tasks/2026-11-02-bug-task-lifecycle-unowned-records)."}]
---
# Diagnose durable GUI events EOF after logic restart

Owner: Sable Embertail <EXECUTOR>, assigned by Vesper over ICQ (`3346ceb9`, `c040ab7d`).

Quarantined in [[quarantine]]: gui_events.rs:251 received socket EOF after killing logic, seen in Nyx and Aster prechecks; forty isolated controls passed on tip and main. The failure mechanism remains unproven.

Acceptance: retain the failed GUI/session/logic diagnostics through the restart boundary; identify the cause, then fix the product or fixture without hiding EOF behind a retry. Preserve durable cursor and unread state, signed identity and same-generation PTY control assertions. One test failure is a failure to investigate. Remove the quarantine row only after a named regression and the original acceptance checks pass.
