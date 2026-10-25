---
id: "01M5T135QBAWKH59VEGDB5690A"
type: "task"
number: 329
title: "Diagnose fifty-seat spawn ENOENT under load"
status: "open"
role: "executor"
owner: "Sable Embertail"
verify: "cargo test -p bbt-supervisor --locked --test supervisor fifty_seats_share_one_supervisor_poller -- --exact"
created: "2026-10-25T17:17:52.235467Z"
updated: "2026-10-25T17:17:52.235467Z"
---
# Diagnose fifty-seat spawn ENOENT under load

Owner: Sable Embertail <EXECUTOR>, assigned by Vesper over ICQ (`3346ceb9`, `c040ab7d`).

Quarantined in [[quarantine]]: one of fifty /bin/zsh -f spawns returned Io ENOENT under three-gate load (gate 65927a6c); five isolated controls passed. The missing path or failing syscall was not retained.

Sable diagnostic checkpoint c96152f2 names spawn stage, program/cwd/PTY paths and path existence at failure without argv/environment values. Its real missing-executable regression and supervisor 72/0 checks pass. Thirty exact fifty-seat runs at load 18.8–19.7 also passed, with no added workers. **Cause remains unproven; this is not a fix claim.**

Acceptance: capture the failing path/stage under the reported load; fix the cause or fixture, retaining all fifty distinct echoes and the one-poller contract. No product retry or deadline loosening. Remove the quarantine row only with the named cause and regression proof.
