---
id: 01M71Y25MDFAQWE4JC4H1V0XBW
number: 1794287670925982
type: report
title: Restore exited sessions without reclaiming historical workspace leases
task: '[[tasks/2026-11-04-codex-trusted-handoff]]'
commits: []
verify:
  command: sh scripts/quarantine-test.sh --locked --release -p bboxy450-term-mux --lib -E 'test(retained_exit_restore) | test(managed_workspace_)'
  exit_code: 0
author: seat:01M71Y252EEXCTDHVJTJSX7JVZ
created: '2026-11-10T05:14:30.925674+00:00'
---
Direct operator request: fix the Startup failed / Restore sessions / logic exited during startup error, without restarting Supervisor because harnesses are working. This standalone execution identifier is not a registered native seat; the historical task provides delivery context only.

Cause: logic.log ends in workspace_stale_holder. The existing Supervisor retains seven process records: four live and three exited. Exited seat 01M71TWWSEW21999BGTJY4WTCE / PID 99314 has a historical holder-seat index to a workspace now held by 01M71TXG8HDSP2XBCH84F7XR0X / PID 851. Worker::attach unconditionally treated that historical observation as a fresh managed-writer launch confirmation; its correct stale-holder refusal then aborted the entire startup. No live database or workspace record was edited to recover it.

Mechanism:
- crates/bbt-mux/src/mcp/workspaces.rs: observe_launch now accepts Supervisor SeatInfo and returns without touching writer state when exited is present. Live observations still call the existing strict launched seat/generation/PID check.
- crates/bbt-mux/src/logic.rs: attach passes the owned process metadata through; exited sessions can still restore their captured history.
- crates/bbt-mux/src/mcp/workspaces/tests.rs: recreates reassignment with the historical reverse index intact, verifies byte-identical replacement/failed workspace state after historical restore, and confirms the old live actor still receives workspace_stale_holder. Existing lost-response, PID and generation refusal checks remain.
- ManagedWorkspaceOwnership's native builder and generated TLA/config artifacts now project retained-exit observation versus a reassigned historical lease. The good model cannot fail startup from that observation; the broken control has an isolated stale-history counterexample. verification/contracts.toml states the abstraction and remaining native scheduling/process coverage.

Verification: final focused nextest selection exited 0, 18 passed and 519 unrelated tests skipped. An initial test-only ExitInfo namespace error exited 101 and was corrected before the passing run. The bounded native model exporter exited 0; pinned Java TLC and native Rust reachable-state and transition sets match exactly: good 16 states / 28 edges (TLC exit 0), broken 25 states / 40 edges (expected safety refusal, TLC exit 12). The broken graph independently contains [1,0,0,0,1,1], where only the stale historical restore fails. This is a selected model check, not full formal/Yarn conformance or arbitrary Rust proof. The temporary exporter reused the repository's existing export format and was removed after producing the private graph evidence.

Supervisor PID 84759, birth [1794273616,96520], and original harness PIDs 86143, 9999, 34927 and 19453 remain running. A bounded supervisor metadata probe was performed only while Logic was confirmed dead; its temporary client disconnected without restarting or signalling any process. Never acquire a diagnostic Logic lease after the real Logic resumes.

Signed app build and Logic/UI activation remain pending at this source commit. The normal startup path adopts the live Supervisor and does not relaunch its seats; no --recover-seats, ctl app stop or make restart-recover is permitted. Final delivery must attest the new rendered source, ready events/frames and unchanged Supervisor/harness birth identities. The previous refresh helper requires live Logic for its preflight, so activation must use the canonical prepared installer and existing guarded GUI launch/readiness functions while the failed Logic is absent.

No full CI, make land, full formal/Yarn corpus, smoke or visual suite ran. This scoped fix retains the operator's signed direct-push/[skip ci] workflow. Existing missing-libLLVM stripping and unused Dream helper warnings were nonfatal. Command/exit/source-hash evidence is .bboxy/runs/2026-11-09-retained-exit-workspace-restore.json; raw logs and native/TLC graphs remain private at /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-restore-fix-20261109-x6y64eqb.
