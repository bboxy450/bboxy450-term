# Rook reboot handoff — 2026-10-26

The operator ordered immediate wrap-up for a frozen typing/paste dialog, no new tasks, and all unfinished work retained on disk and GitHub. Oracle Vesper was sent the exact request and asked whether responsive (ICQ 6f4af850-a25d-4401-815e-a816c09b8977, recipient-stored; no reply yet, model receipt unproven). Do not restart or drive the operator desktop.

## Repository and checkpoint

- Worktree: /Users/feral/mydev/bboxy450-term-rook
- Branch: rook-error-journal, based on a2444802a7e7178374b7840565a27c030ad2e603.
- This is an UNFINISHED, unlanded checkpoint, not release-ready. No new presubmit or scale run is authorized by this handoff. Global wrap/green merges belong with Vesper, current instructions at ~/.agents/AGENTS.md and repository AGENTS.md.
- Durable evidence/source backup: /Users/feral/.local/state/bboxy450-term/resume/2026-10-26-rook/
- Detailed issue: https://github.com/bboxy450/bboxy450-term/issues/56
- Global release handoff: https://github.com/bboxy450/bboxy450-term/issues/62
- Main remote had advanced to 0efb8522 at this observation. Shared main checkout was older (98220c2a); fetch and inspect actual Git before acting. Never assume the shared checkout equals origin/main.

## Assigned error journal (#56)

Oracle brief aec5dcde-7846-42c4-a0c0-3ac77021e3f4: one append-only bounded rotated diagnostics.jsonl under the run dir; hook nonzero exit/name/harness/seat/stderr last4KiB/duration, MCP and ctl errors, logic/supervisor exits, plugin/provider/spawn errors; redacted, no environment or secrets, best effort and nonblocking. ctl debug errors [--since T] [--seat S]; unread count visible in HUD. Diagnose first-prompt hook failure of operator Codex seat 01M5WZKA3MZC8BRJ601TFF7FH1. ICD .bboxy/interfaces/app-harness.md; never write ~/.codex or ~/.claude.

Implemented but not fully verified:
- bbt-rt::error_journal: nonblocking UnixDatagram producers, one owned disk thread, private0600 files,1MiB active +2 archives, fixed-schema JSON, stderr4KiB redaction, offline filtered reads/ack cursor, count-change callbacks; missing/full sink drops immediately. Five focused tests passed.
- App and headless sink startup plus unread-errors HUD; root propagated as BBOXY_RUN_DIR into seat, companion and MCP environments.
- DebugRequest::Errors and ctl debug errors, including file-only offline read. --since currently Unix milliseconds; unfiltered reads acknowledge retained rows; filters do not acknowledge others.
- Failure capture at shim/MCP/ctl/plugin/provider/logic-worker/supervisor-worker/spawn/parent process-exit boundaries.
- cockpit backend child strips inherited parent harness-session metadata, preserving HOME/PATH/auth config; one deterministic environment assertion was added but not run.
- bbt-rt adds existing libc as mandatory and serde_json; lock change only serde_json dependency metadata. Nyx was warned about Cargo overlap (cc3abb87-e011-4e87-8092-c2defeb7eb35). Preserve release version/package renames on rebase.

Actual verification:
- journal core: 5 passed,0 failed, command and raw log in durable directory; SHA d4caec5ec2c121e69638dbab526a3d5f3c9818d95d68054ef53386146f77479a.
- cargo check --locked -p bboxy450-term --tests (own target, jobs2, dev/test O1 debug0): EXIT0,2m43s. Compile only, no integration test execution. integration-check-1.json records command/hash.
- git diff --check passed. No strict Clippy or author gate yet. No owned command remains running.

Known completion gaps (do not mark #56 fixed):
1. Native global hook stderr capture is not connected. App overlay shim always exits0 today, so it cannot by itself explain native exit1. Installed Codex hook/completed contains event/sourcePath/status/duration/error entries but no explicit exitCode. thread/read does NOT subscribe; only thread/resume joins the running thread. Oracle ruling requested in e28ffec5-dda0-4df8-8097-ae0060eaa60e for a generation-cancelled observer of verified private loaded thread, no config/permission overrides. No ruling received and NO subscription/mutation added. Async hook notifications are unavailable by documented API.
2. Original operator stderr/root cause remains UNPROVEN. First read-only ctl screen succeeded; viewport no longer held Hookfailed. Subsequent app gui.endpoint disappeared (through latest check), ctl exit4 endpoint_unavailable. No operator app input/restart was attempted.
3. Strong candidate: inherited CLAUDE_CODE_SESSION_ID / CLAUDE_CODE_MESSAGING_SOCKET or mismatched CODEX_THREAD_ID. Installed icq0.21.0 main.rs1367-1389 returns 'hook session differs from inherited current session' / 'ambiguous inherited hook session' before daemon registration. The cockpit child inherited these before this patch. Need controlled fake-env early-error reproduction and actual native failure evidence; do not claim the operator cause from this alone.
4. Add debug::PARSE_SAMPLES entry for the new errors verb (existing exhaustive verb test will otherwise fail). Add focused API wire-golden compatibility, MCP/ctl/hook/HUD and env regression tests; inspect/refine exact-once failure reporting and UI visibility. Existing compile success does not verify runtime.
5. Finish formatting/review and land via scripts/land.sh only when complete and green. No packaging/publishing/release/tag action was performed here.

Native local references: /Users/feral/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/icq-0.21.0/src/main.rs; installed codex schema remains in the original evidence root, codex-schema/ServerNotification.json and ClientRequest.json. Official API https://learn.chatgpt.com/docs/app-server (hook/completed synchronous only; thread/read is not subscription).

## Already completed — do not redo

- Frontend retained probe lock fix #360 landed f33e9fe61a86050677c64ba2b746af1fb530b670; evidence b5990dde8f349dfe551ddf4d4d8b6c4b1be63a2b, .bboxy/runs/frontend-probe-lock-release.log. New deterministic retained-descriptor regression, owner tests5/0, strict0/fast invariants14/author0. Original explicit unlock/fork fix8755bd11 also landed. No quarantine row remains.
- Per-frame/GPU telemetry8295dd5265fbd8439ee8da3c247e5d75ddabb8fc landed. No new967 run in this cut.
- Scale gaps already filed as GitHub #46-50, no duplicates: frame20.710417ms>8.33; CPU20.361334>4; input/GPU1398.155791ms>26.666667 actual75Hz; Write-to-LogicOutputRead1348.697583ms>8.33; bus9999.018339/s<10000. Template candidate9739ea66 remains parked/unlanded/unverified on rook-frame-scene-template; do not merge it as a measured fix.
- Old untracked .bboxy/runs/w1-* files predate this task. Left untouched and unstaged.

## Safe restart

Read this file and issue#62 first. Check date, Git status/log/actual diffs, then current authenticated Oracle inbox. All evidence and source are local. Do not replay old briefs as new work. Vesper is the sole ruling/spec contact: Corvin and Wren sessions are shut down. No new tasks under the operator wrap-up order. Git author Boxy <bboxy450@gmail.com>; every commit signed with9B07E849480C68328CA20684844ECC0587DE7AB0. GitHub plugin is logged in as retired Gonzih, do not use it for writes; local gh api user verified bboxy450.


## Oracle ruling received during shutdown — cf405d78-f563-426a-8ec4-19c22737c945

This addendum supersedes the earlier pending-ruling/no-reply statements for the subscription question only. It does not revoke the operator's direct shutdown/no-new-tasks order, and no code, test, gate or native subscription was restarted.

Vesper approved the narrow Codex observer: only after verified loaded thread plus owner PID; `thread/resume` with `excludeTurns:true`; zero config, permission or model fields; generation-cancelled; observation only; never attach this observer to the operator's running seat. Hook wrapping remains rejected.

Sequence when work is explicitly resumed: cut1 lands the journal core, ctl errors and MCP/plugin/spawn/worker failure capture as a separate complete, green cut. Cut2 adds the Codex hook/completed subscription on the same branch line and records the seam as one line in `.bboxy/interfaces/app-harness.md`. No further Oracle approval is needed between those cuts. The actual operator hook's original stderr/cause remains unproven; the ruling is not runtime evidence.


## Resumed by the operator — 2026-10-26 22:00 UTC

Direct instruction `continue` resumes #56. Implement the Oracle-approved two cuts, with no unrelated work. The parent-harness environment correction is deferred to cut2 alongside actual hook evidence; cut1 remains the error journal and central capture. Rebased onto current origin/main, preserving Sable's CliOutput/BrokenPipe behavior at the ctl stdout seam.
