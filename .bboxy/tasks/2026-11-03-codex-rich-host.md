---
type: task
title: 'Host the shared rich harness view in term: socket delivery, resume and native pane'
status: rejected
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
- crates/bboxy450-harness
verify: make test-parity
parent: 01M6HCH1SJHDSB6SDAC2ZF1BXX
id: 01M6HCYDWBAX2FDYGMSPB12HRN
updated: 2026-11-03T20:49:04.238Z
created: 2026-11-03T19:07:32.055Z
number: 15252
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
verify_result:
  command: BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-parity
  exit_code: 2
  evidence:
  - criterion: A3/A4 delivery, stable op ID, typed outcomes, zero PTY bytes, interrupted/resumed process
    reference: /tmp/codex-rich-native-current.log; crates/bbt-mux/tests/mcp.rs:5069; .bboxy/runs/codex-rich-host.json
  - criterion: Shared native view, composer/Stop/approval/profile, cursor reattach and key routing
    reference: crates/bboxy450-term/tests/rich_client.rs:59; /tmp/codex-rich-native-current.log
  - criterion: Native standalone vs term pane parity
    reference: 'target/rich-pane-parity/report.json: row_structure_mismatch=0.0; identical capture SHA256 0e0ead94269625e5eb98bcebb27a398702f593c5ecd01463272c6806f33a591e'
  - criterion: Formal safety/liveness, four good/broken pairs, full graph equality
    reference: /tmp/codex-rich-formal-complete.log; evidence SHA256 cca2509fa9222bc0d2648a95b43453ba43677c6c1b6768f4a0ec81b956fd262d; 284 models, 34 crates, 116 contracts; dirty=true
  - criterion: Boundary and persistence regressions
    reference: '/tmp/codex-rich-boundaries.log: exit 0, 39 passed; /tmp/codex-rich-window-journal.log: 8 passed'
  - criterion: Strict all-target lint, license/notice and layering gates
    reference: '/tmp/codex-rich-quality.log: exit 0; /tmp/codex-rich-layering-final.log: held, exit 0'
  - criterion: Acceptance blocker and limits
    reference: 'reports/2026-11-03-codex-rich-host.md; /tmp/codex-rich-parity.log: dirty source, exit 2; /tmp/codex-rich-receipt.log: clean source required, exit 2'
report:
  summary: 'BLOCKED on final integration, not ready for acceptance. Signed candidate 678eb9fc implements A3/A4 socket delivery/resume and A1/A2 shared HarnessView panes, tested against private signed harness d5e7249a01bf4830c79979aa7a8e2f17753be83b. Native/process, boundaries, strict lint/licenses and 284-model native/TLC parity passed; offscreen row mismatch 0.0, four-view composition p99 0.033417 ms (load/heap limits remain unmeasured). Full test-parity and clean formal receipt both exit 2 because the unpublished candidate gitlink is intentionally unstaged. Need the published combined standalone+B1/B2 SHA and a signed task amendment allowing its gitlink, then remaining combined-pin detach/IR1.2/budget checks. Full committed report #17675 names every gap and evidence. Four artifact claims were rejected provider_busy, explicitly no effect admitted; their operation IDs are committed in the run summary. No push, merge, publication or trusted install.'
  commit: 678eb9fc14ff4937af81363f3269d830967c9c52
  reported_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  op_id: 01M6HJNRV0DWENPY72MTW4HF2P
  verify_revision: 0
  report: '[[reports/2026-11-03-codex-rich-host.md]]'
rejection:
  reason: 'Preliminary report #17675 is retained and useful, but verify command differs from the canonical task and exits 2; clean formal evidence also exits 2, private gitlink is unstaged, and combined App detach/status/IR1.2/budget plus stale Cancel witness are outstanding. Continue under the signed amendment authorizing exact gitlink path and independent remaining work. A new superseding final report must include the published combined pin and all current-source checks; no operator approval is needed.'
  by: seat:01M6H97SAX3GN1BYB4YG76P3P0
provenance:
  digest: sha256:1a4bc98b3729a0d587a986c69c0421b5e13c01a502a8d03d311e8b6f1f63b2d0
  frontmatter_digest: sha256:b3c936add189c7c98b59f6af06275b6b1ff26ef6379404438f6cbe37d1a4d861
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: Wg4PG577TB1kjFB2KXCy2z7iwNpsAQ8YJJIFCpyGpZ+OkDKQ0xhbVv4dx8+mPlr+2xWfgOdYCq7M7ZRhGMaSDA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#100558
---
ROLE: EXECUTOR. Work ONLY in /Users/feral/mydev/bboxy450-term-codex-rich, branch codex-rich-pane-20261103, created from origin/main 1affb986. The operator's screenshot is a PTY line-mode harness, and asks for the standalone native harness face embedded in term. Implement term Side A of the existing rich-pane ICD, preserving the session process and sharing HarnessView rather than building another renderer.

Read own AGENTS.md/ATTRIBUTION.md/vision, .bboxy/interfaces/app-harness.md, harness origin/main .bboxy/interfaces/rich-pane.md (Side A §2–§9), d5977 dependency direction and the binding A3/A4 brief at /Users/feral/mydev/bboxy450-term-wt-architect/.bboxy/tasks/2026-11-02-rich-pane-a34-delivery.md. That brief defines the delivery phase; this direct operator task additionally authorizes the following A1/A2 native pane phase. Complete the delivery boundary first and then the view. No global settings, credentials, OS clicks/keystrokes or operator screen capture.

A3/A4: term-chosen <=96-byte socket path in launch argv; async per-seat connection, Hello/session/epoch tracking and persisted session ID; generation fencing and reconnect; seat_input_text/composer submits idempotent UserTurn over Unix BEFORE PTY arbitration, correct Ack/Busy/Closed/unknown outcomes, zero PTY bytes. Add bboxy450-harness resume admission to seat_spawn with validated session format and a real interrupted/resumed process witness; the harness CLI already supports seat --resume. Read the ACTUAL current origin/main code, not stale line numbers from handoffs.

A1/A2: per-pane Terminal/HarnessRich face; HarnessRich for announced harness sessions; render the shared HarnessView Frame with the existing renderer in the pane, forward its composer/stop/approvals over the socket, persist the face preference, keep PTY ownership and detach/reopen semantics. Missing socket honestly shows starting/unavailable and the Terminal toggle. No UI/logic-thread IO. Keep the explicit aggregate drain/projection/memory budgets in the ICD; measure and label remaining limits honestly.

Coordinate with rich-socket seat 01M6HCVV32KS845WHVD5FCX7QE for B1/B2 and with standalone recovery. Implement against private scripted Unix fixtures immediately; do not park waiting for a peer. Test real integrated B1/B2 when available using a pinned candidate with one toolkit copy; record the exact harness commit. Do not move the committed submodule pin to an unpublished commit: report the required pin to the Oracle. Term main currently has a layering failure after the 0c40316 pin; inspect and reuse the already-prepared signed fix dfb21967c0157aebcadec5b028d300abd86203ce if applicable, with a real layering check. Do not weaken the layering gate.

Verification: formal-first state variables/bounds/owners/atomic actions, safety/liveness/fairness; good and broken controls for delivery duplication, stale identity A→B→A, reconnect/detach, and routing. Private scripted process witness for each ICD acceptance clause, zero PTY byte assertions, resume witness, and an offscreen capture of the same fixture in standalone vs rich pane with row mismatch <=0.01. Run make test-harness-native, make test-boundaries and make test-parity on the final candidate, using the canonical targets and current-source native/TLC formal evidence. Full machine gates run one at a time, self-service with process evidence; no operator input.

No recruitment, push, merge, publication or trusted install. Exact staging, signed commits, preserve existing source. Full report .bboxy/reports/2026-11-03-codex-rich-host.md with Report trailer, harness hash, file:line mechanisms, real exits, budget/parity numbers, negative controls and gaps; register through knowledge tools and task_report.

Outstanding term recovery, consumed before the final full gate: integrate the completed status-term component from preserved branch fix/harness-status-term, tip 76e8ba8ed29f6cc8a64e9b9ec0620d8301f85ac1. Its two implementation commits are 3dbc5e20d6673dc7447d6e907b073c75b57a7695 and beea4c84bd1fe311c6852a507f40e2ba326873a4; 1ea058b4 and 76e8ba8e are signed reports. Read the actual diffs and .bboxy/reports/2026-11-03-status-term{,-verified,-phase-1c,-phase-1c-verified}.md in /Users/feral/mydev/bboxy450-term-status. Preserve that original worktree/ref unchanged. Recover the complete status mechanism and its tests/models in your own integration branch; do not wholesale import unrelated generated crate ATTRIBUTION changes or the stale harness pin. Related code/report/script paths are already in your allowed source scope. Resolve overlapping SeatInfo/gui fields, launch code and formal inventories while retaining both contracts.

Status acceptance is ICD app-harness phase 1c: current-generation Reported activity wins over cadence/lower_recent; native session log is matched through kernel peer PID/uid and exact Hello session, read off-thread with bounded append cursor; original ts_ms is retained and EOF/Hello do not refresh it; only an open turn goes stale after 60 seconds, detail says no word for 1m and rim dims while activity remains Reported; close clears errors. Existing real quiet-tool 20s OSC-only and log-only, approval/error/close/staleness witnesses and actual cadence mutation must be run on the combined candidate. Keep native/TLC status models and broken controls alongside RichHost. Historical reports explicitly do NOT claim current full CI: canonical make test-parity, native, boundaries and current formal evidence on the final combined source still decide. This is recovery of the remaining interrupted cohort's completed component, not permission to change credentials, publish, install or expand into unrelated features.

## Report from seat:01M6HCYVGE7CJ8KK6PB20SPF3D

BLOCKED on final integration, not ready for acceptance. Signed candidate 678eb9fc implements A3/A4 socket delivery/resume and A1/A2 shared HarnessView panes, tested against private signed harness d5e7249a01bf4830c79979aa7a8e2f17753be83b. Native/process, boundaries, strict lint/licenses and 284-model native/TLC parity passed; offscreen row mismatch 0.0, four-view composition p99 0.033417 ms (load/heap limits remain unmeasured). Full test-parity and clean formal receipt both exit 2 because the unpublished candidate gitlink is intentionally unstaged. Need the published combined standalone+B1/B2 SHA and a signed task amendment allowing its gitlink, then remaining combined-pin detach/IR1.2/budget checks. Full committed report #17675 names every gap and evidence. Four artifact claims were rejected provider_busy, explicitly no effect admitted; their operation IDs are committed in the run summary. No push, merge, publication or trusted install.

```json
{
  "op_id": "01M6HJNRV0DWENPY72MTW4HF2P",
  "commit": "678eb9fc14ff4937af81363f3269d830967c9c52",
  "report": "[[reports/2026-11-03-codex-rich-host.md]]",
  "verify_result": {
    "command": "BBOXY_HARNESS_CANDIDATE=d5e7249a01bf4830c79979aa7a8e2f17753be83b make test-parity",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "A3/A4 delivery, stable op ID, typed outcomes, zero PTY bytes, interrupted/resumed process",
        "reference": "/tmp/codex-rich-native-current.log; crates/bbt-mux/tests/mcp.rs:5069; .bboxy/runs/codex-rich-host.json"
      },
      {
        "criterion": "Shared native view, composer/Stop/approval/profile, cursor reattach and key routing",
        "reference": "crates/bboxy450-term/tests/rich_client.rs:59; /tmp/codex-rich-native-current.log"
      },
      {
        "criterion": "Native standalone vs term pane parity",
        "reference": "target/rich-pane-parity/report.json: row_structure_mismatch=0.0; identical capture SHA256 0e0ead94269625e5eb98bcebb27a398702f593c5ecd01463272c6806f33a591e"
      },
      {
        "criterion": "Formal safety/liveness, four good/broken pairs, full graph equality",
        "reference": "/tmp/codex-rich-formal-complete.log; evidence SHA256 cca2509fa9222bc0d2648a95b43453ba43677c6c1b6768f4a0ec81b956fd262d; 284 models, 34 crates, 116 contracts; dirty=true"
      },
      {
        "criterion": "Boundary and persistence regressions",
        "reference": "/tmp/codex-rich-boundaries.log: exit 0, 39 passed; /tmp/codex-rich-window-journal.log: 8 passed"
      },
      {
        "criterion": "Strict all-target lint, license/notice and layering gates",
        "reference": "/tmp/codex-rich-quality.log: exit 0; /tmp/codex-rich-layering-final.log: held, exit 0"
      },
      {
        "criterion": "Acceptance blocker and limits",
        "reference": "reports/2026-11-03-codex-rich-host.md; /tmp/codex-rich-parity.log: dirty source, exit 2; /tmp/codex-rich-receipt.log: clean source required, exit 2"
      }
    ]
  }
}
```


Oracle continuation after preliminary report #17675: preserve that signed report and its truthful exit-2/dirty-source evidence as history. The exact gitlink path crates/bboxy450-harness is now authorized for the final PUBLISHED combined harness SHA; no private/unpublished candidate pin may be committed. The socket author is integrating published af65c6c and adding §9's aggregate witness. Do useful independent work now: consume the earlier status-term recovery amendment, fix/test stale Cancel identity, add the canonical bench row and prepare the actual App detach/reattach witness; the final pin/gates follow publication without operator approval.

Concrete Oracle review: rich_pane.rs:108 fences stale/unready Enter/Click, but Input::Cancel at :120 bypasses captured Request.session. Shared HarnessView::cancel_prompt synchronizes the CURRENT approval and can send Deny. Fence Cancel against the displayed/captured identity/readiness before any operation; prove an Escape delayed from A cannot deny B's approval (including epoch/generation distinctions where needed), with a deterministic process/API witness and causal broken control. The app-level rich detach/reattach case and make bench-budgets row in ICD §9 are required; the 4x paints clone microbenchmark is not total budget proof. Keep actual under-load and heap limits UNMEASURED when unmeasured. Canonical verify remains exactly make test-parity, with final native, boundary and clean formal checks; the preliminary override command/exit2 cannot satisfy it. Send a new append-only report superseding #17675 once all final checks are current.