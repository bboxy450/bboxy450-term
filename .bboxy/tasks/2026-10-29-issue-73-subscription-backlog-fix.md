---
acceptance:
- Exact authorized direct-seat message subjects are included without broad wildcard grants or expanded seat authority.
- Private retained backlog and stale/not-ready wake retries reproduce the original visibility failure.
- A fresh composer send paints with its real delivery receipt within existing frame/control budget while durable history, cursor/ACK semantics and terminal fairness remain intact.
- Repeated unsuccessful wake receipts are bounded/coalesced; recovery deletes no message and silently advances no read mark.
- Required Make commands exit0 on signed source, production regression/mutation evidence is retained, and author landing succeeds under decision321.
allowed_paths:
- crates/bbt-mux/src/messaging/**
- crates/bbt-mux/tests/gui_events.rs
- crates/bbt-mux/tests/messaging_flow.rs
- crates/bbt-mux/tests/messaging_suppression.rs
- crates/bboxy450-term/tests/messaging_ui.rs
- crates/bboxy450-term/tests/messaging_flow.rs
- Makefile
- scripts/test-gui-events.py
- .bboxy/runs/issue-73-20261029.json
artifacts:
- '[[01M64GAQKFPXFMD5NH9MA6VETD]]'
assignee: seat:01M649SZ31YZ93Z4VGP48Z0KH6
authorship:
  device_id: 01M677XWZPW1XGTQYYGA50W74M
  issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
  kind: device
  principal: operator:01M677XWZPW1XGTQYYGA50W74M
  public_key: cHjhL+5pI9VNQE+Uhb4HJBnYoPqSIH7aCYU+b9MElWs=
  signer: device:01M677XWZPW1XGTQYYGA50W74M
created: 2026-10-29T18:16:36.727Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
delegated_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64E1KKJZSJMBN4GE8F33W70
issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
number: 378
provenance:
  digest: sha256:2382d781d5ce73a1447b4bfda52a52580e04400cb9b667e6f5f1a65fe5953214
  frontmatter_digest: sha256:8a6e03de7c79c7d5ac2ab9bce517fcb221fd48aed8176f3c25cbc4aad764827b
  log: .bboxy/log/2026-10-30.jsonl#773
  sig: nj8Xms4r5cPKwiv72Yj5O2mOFKMZccvWClTvE82tQbxbG8k/HFILPS8Hqd1zPnPseUKn8o7U7345FwvZn2MDBw==
  signer: device:01M677XWZPW1XGTQYYGA50W74M
reconciliation:
- by: operator:01M677XWZPW1XGTQYYGA50W74M
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/d70e94d0c299edb97d2bcfbb0161c155d236ea4a
  - https://github.com/bboxy450/bboxy450-term/commit/83fdacb3890f8d77c183af644c09fd435f484b8e
  fresh_accepted: false
  kind: historical
  op_id: 01M6782SR37RJR4Y9H29SVBG7R
  previous_status: reported
  reason: Operator-authorized historical msgrec reconciliation, 2026-10-30; implementation already landed on main; no new acceptance claimed.
report:
  commit: d70e94d0c299edb97d2bcfbb0161c155d236ea4a
  op_id: 01M64WWNH5ZP5S6BKY475HM9S8
  reported_by: seat:01M649SZ31YZ93Z4VGP48Z0KH6
  summary: 'Implemented exact authorized direct-seat publication, durable semantic wake-receipt coalescing, strict retained-history/native regressions, and canonical Recent read-through under supplemental386/447/449. Final signed Boxy commit d70e94d0c299edb97d2bcfbb0161c155d236ea4a (G, approved 844ECC0587DE7AB0). Own worktree /Users/feral/mydev/bboxy450-term-issue-73-20261029 clean. Actual standalone `make test` exit0 on exact d70: /tmp/issue-73-d70-resource-rechecked.log and /tmp/issue-73-ci-d70-green.log (SHA256 eaf97eb3b4ffb33e91e8828d34f1dc21906ca84ca047eb5947ba9c7ff532d11d); normal 27 invariant checks0,11 slow skipped. Actual focused invocation `make test-chat-sections test-gui-events test-boundaries test-scene-template test-licenses test-task-contract test` at d70: all six focused targets passed; appended test failed only recovery free-disk guard (whole invocation exit2), then same-source standalone test0 after own inactive artifact preservation. Log /tmp/issue-73-rebased-d70-gates.log; old failed full /tmp/issue-73-ci-d70.log retained. Canonical `sh scripts/land.sh` exit0, /tmp/issue-73-author-landing-default.log/.exit; selected exact-source code gate exit0,20 affected invariants0,7 unaffected+11 slow skipped, compile595.726s/run259.168s, existing caps unchanged; remote refs/heads/main independently matched d70. Earlier optional --gate-log invocation exit2 gate_log_requires_wide_cut before push, then authorized default command succeeded; no bypass/force/manual push. No install/release/operator input/real SSH/model/desktop action. Reports are handoffs for independent verification, not acceptance. Remaining whole73 acceptance: Oracle''s combined main activation with Rook43/marker11 and preserved-owner production post-refresh proof. Cold348000 startup remains separately unproven: /tmp/issue-73-read-through-full.log Make2 unchanged30s guard. Earlier false-warm owner death/database-is-locked is preserved; corrected harness2ms/64 fixture-only seeding yield checks identical owner/backend_ready/no logic process_entry failures, without production SQLite/clock changes. No zero-error/issue75 closure claim. Raw logs/fixture identities outsideGit; only command/exit/hash summaries in .bboxy/runs.'
role: executor
status: done
swarm_id: 01M649102FBS60QWNPZS5VAGF9
title: Fix retained direct-seat chat visibility and repeated unsuccessful wake receipts (GitHub73)
type: task
updated: 2026-10-30T20:30:07.738Z
updated_by: device:01M677XWZPW1XGTQYYGA50W74M
verify: make test-chat-sections test-gui-events test-boundaries
verify_result:
  command: make test
  evidence:
  - criterion: Exact authorized direct-seat message subjects are included without broad wildcard grants or expanded seat authority.
    reference: crates/bbt-mux/src/messaging/service.rs:1029 — operator_subjects walks only existing authorized seats tree Sink /message leaves; no grant mutation/wildcard. Regression :3177 compares before/after authority. crates/bbt-mux/src/bus.rs:2217 strict108 subjects with exact registry-derived direct inbox equality; test-only bus change, limits64/4096 retained.
  - criterion: Private retained backlog and stale/not-ready wake retries reproduce the original visibility failure.
    reference: crates/bbt-mux/tests/messaging_flow.rs:534 — actual Session stale/not-ready128 attempts +reopen64 retain attempt journal, bounded projections, no native writer effect. crates/bboxy450-term/tests/gui_events.rs:613 — private signed synthetic legacy receipt workload348000,43 memberships/7 swarms; real native app/ctl/frame witness. /tmp/issue-73-mutant-subjects.log and /tmp/issue-73-mutant-receipts.log actual Make2; causal source/mutation identities in .bboxy/runs/issue-73-20261029.json and /tmp/issue-73-mutations.json.
  - criterion: A fresh composer send paints with its real delivery receipt within existing frame/control budget while durable history, cursor/ACK semantics and terminal fairness remain intact.
    reference: /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/gui-events-app-2EQOEn/backlog-witness.json SHA25662d889183d9dca1b0a435f8aec975e4c7f50990432e537074b83e53924b5d2f3 at d70 — real fresh01M64SWF4VEVA5DHV62FPC7061 frame20 compose95ms/terminal48ms, recipient-stored delivered1 ACK0, same owner96707 birth[1793310160,811104],read unchanged,lane[56,25,348030]→[56,35,348030]. Headless offscreen PNG/raw fixture retained outsideGit. History chain/first row verified; consumer remains behind retained high, no replay drain claim.
  - criterion: Repeated unsuccessful wake receipts are bounded/coalesced; recovery deletes no message and silently advances no read mark.
    reference: crates/bbt-mux/src/messaging/ledger.rs:173,:996,:1711 — persisted semantic receipt state coalesces only identical outcomes after normalizing attempt/eligible_ms; all attempt journal/current receipts remain, meaningful outcome/note/evidence changes emit. messaging_flow.rs:686 unknown restart never retried. Retry250ms unchanged; no silent read/application ACK advance. Native-written explicitly distinct from recipient model ACK.
  - criterion: Required Make commands exit0 on signed source, production regression/mutation evidence is retained, and author landing succeeds under decision321.
    reference: Actual focused target successes, exact signed full make test0 and default author landing0 detailed in summary. Final read-through native negative control /tmp/issue-73-final-control-live-owner-refresh-disabled348000.log Make2 (Stored true, Compose false,15s budget, same retained owner/no process_entry failure); /tmp/issue-73-final-controls.json hashes. Final native None→actual Event witness /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/gui-events-app-rI2ZAm/canonical-reconciliation.json SHA2565ca67962db0e342f80608e8a0115d37d11520db2c029a9a41f84fed5129549ee. Approved agreements adopted under449.
  exit_code: 0
---
# Retained chat visibility and wake-receipt backlog

ROLE: EXECUTOR — Lumen Foxglove. Delegator: Vesper Greywind, Oracle. Worktree /Users/feral/mydev/bboxy450-term-issue-73-20261029; branch exec-issue-73-20261029; base014e555ca341cf5f4037360a74bf75a7c96c23a4. Operator has explicitly authorized the issue-fixing cohort with a zero-open-issues goal. This scoped task is the implementation authorization; the prior DM was not.

Outcome: satisfy GitHub issue73's complete four-line acceptance. Read the issue body, current source, .bboxy/vision.md, interfaces/messaging-seats.md and interfaces/logic-gui.md, decisions/d285-no-gated-communications.md, ATTRIBUTION.md and make help. The current GUI subject catalogue omits canonical direct-seat message publication; retained unsuccessful attempts flood receipts. Fresh-account and65-subject tests miss the retained-history failure. Do not interpret queued/written transport as model ACK.

Allowed paths are in frontmatter. Keep the existing public contract/thresholds. Exact per-seat subscriptions derive only from authorized subjects. No global queue/lock, broad subscription grant, dropped history, silent ACK/read-mark advance, arbitrary timeout/sleep increase or product retry to hide unknown effects. If a cross-boundary contract gap appears, report the exact missing ICD line to the Oracle/Architect before changing the contract. Rook owns the separate Index/Knowledge repair; coordinate any shared Make hunk in messages within this brief.

Verification: make test-chat-sections test-gui-events test-boundaries. Add a focused canonical Make target only if existing targets cannot express the actual retained-backlog regression. Retain a production regression or mutation that fails for each causal fix, and an honest actual produced-frame/receipt witness. Run make test on the final signed candidate. Raw logs/fixture identities stay outside Git; .bboxy/runs contains only command/exit/hash summary. Two Cargo jobs, own target, no operator input, no Keychain or desktop control, no real SSH/model calls.

Signed Boxy commits only. Author self-service landing through scripts/land.sh is authorized by repository decision321 (read it); never hand-push, force-push, merge another branch, publish/install a release or touch credentials. If landing refuses, retain its real code and report/re-brief the bounded cause; do not bypass it. A report is not acceptance. Report task with exact commit, commands/actual exits, file:line mechanisms per criterion, gaps, actual landing result and worktree status. Do not close GitHub73 yourself; the Oracle independently verifies and closes. No recruitment or scope expansion.

This is a fresh, explicitly delegated assignment after restoring valid KB organization. The earlier unassigned task-create operation01M64DN5VHVZ3BNG1N34DKW7MN remains recorded as unknown; healthy native lookup now confirms its requested task path is absent. It is not replayed or delegated. All raw clone bytes and old paths were preserved; the native Index now reads the vision successfully. Report the earlier unknown operation only through the separately designed recovery API, not an automatic write retry.


## Report from seat:01M649SZ31YZ93Z4VGP48Z0KH6

Implemented exact authorized direct-seat publication, durable semantic wake-receipt coalescing, strict retained-history/native regressions, and canonical Recent read-through under supplemental386/447/449. Final signed Boxy commit d70e94d0c299edb97d2bcfbb0161c155d236ea4a (G, approved 844ECC0587DE7AB0). Own worktree /Users/feral/mydev/bboxy450-term-issue-73-20261029 clean. Actual standalone `make test` exit0 on exact d70: /tmp/issue-73-d70-resource-rechecked.log and /tmp/issue-73-ci-d70-green.log (SHA256 eaf97eb3b4ffb33e91e8828d34f1dc21906ca84ca047eb5947ba9c7ff532d11d); normal 27 invariant checks0,11 slow skipped. Actual focused invocation `make test-chat-sections test-gui-events test-boundaries test-scene-template test-licenses test-task-contract test` at d70: all six focused targets passed; appended test failed only recovery free-disk guard (whole invocation exit2), then same-source standalone test0 after own inactive artifact preservation. Log /tmp/issue-73-rebased-d70-gates.log; old failed full /tmp/issue-73-ci-d70.log retained. Canonical `sh scripts/land.sh` exit0, /tmp/issue-73-author-landing-default.log/.exit; selected exact-source code gate exit0,20 affected invariants0,7 unaffected+11 slow skipped, compile595.726s/run259.168s, existing caps unchanged; remote refs/heads/main independently matched d70. Earlier optional --gate-log invocation exit2 gate_log_requires_wide_cut before push, then authorized default command succeeded; no bypass/force/manual push. No install/release/operator input/real SSH/model/desktop action. Reports are handoffs for independent verification, not acceptance. Remaining whole73 acceptance: Oracle's combined main activation with Rook43/marker11 and preserved-owner production post-refresh proof. Cold348000 startup remains separately unproven: /tmp/issue-73-read-through-full.log Make2 unchanged30s guard. Earlier false-warm owner death/database-is-locked is preserved; corrected harness2ms/64 fixture-only seeding yield checks identical owner/backend_ready/no logic process_entry failures, without production SQLite/clock changes. No zero-error/issue75 closure claim. Raw logs/fixture identities outsideGit; only command/exit/hash summaries in .bboxy/runs.

```json
{
  "commit": "d70e94d0c299edb97d2bcfbb0161c155d236ea4a",
  "op_id": "01M64WWNH5ZP5S6BKY475HM9S8",
  "verify_result": {
    "command": "make test",
    "evidence": [
      {
        "criterion": "Exact authorized direct-seat message subjects are included without broad wildcard grants or expanded seat authority.",
        "reference": "crates/bbt-mux/src/messaging/service.rs:1029 — operator_subjects walks only existing authorized seats tree Sink /message leaves; no grant mutation/wildcard. Regression :3177 compares before/after authority. crates/bbt-mux/src/bus.rs:2217 strict108 subjects with exact registry-derived direct inbox equality; test-only bus change, limits64/4096 retained."
      },
      {
        "criterion": "Private retained backlog and stale/not-ready wake retries reproduce the original visibility failure.",
        "reference": "crates/bbt-mux/tests/messaging_flow.rs:534 — actual Session stale/not-ready128 attempts +reopen64 retain attempt journal, bounded projections, no native writer effect. crates/bboxy450-term/tests/gui_events.rs:613 — private signed synthetic legacy receipt workload348000,43 memberships/7 swarms; real native app/ctl/frame witness. /tmp/issue-73-mutant-subjects.log and /tmp/issue-73-mutant-receipts.log actual Make2; causal source/mutation identities in .bboxy/runs/issue-73-20261029.json and /tmp/issue-73-mutations.json."
      },
      {
        "criterion": "A fresh composer send paints with its real delivery receipt within existing frame/control budget while durable history, cursor/ACK semantics and terminal fairness remain intact.",
        "reference": "/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/gui-events-app-2EQOEn/backlog-witness.json SHA25662d889183d9dca1b0a435f8aec975e4c7f50990432e537074b83e53924b5d2f3 at d70 — real fresh01M64SWF4VEVA5DHV62FPC7061 frame20 compose95ms/terminal48ms, recipient-stored delivered1 ACK0, same owner96707 birth[1793310160,811104],read unchanged,lane[56,25,348030]→[56,35,348030]. Headless offscreen PNG/raw fixture retained outsideGit. History chain/first row verified; consumer remains behind retained high, no replay drain claim."
      },
      {
        "criterion": "Repeated unsuccessful wake receipts are bounded/coalesced; recovery deletes no message and silently advances no read mark.",
        "reference": "crates/bbt-mux/src/messaging/ledger.rs:173,:996,:1711 — persisted semantic receipt state coalesces only identical outcomes after normalizing attempt/eligible_ms; all attempt journal/current receipts remain, meaningful outcome/note/evidence changes emit. messaging_flow.rs:686 unknown restart never retried. Retry250ms unchanged; no silent read/application ACK advance. Native-written explicitly distinct from recipient model ACK."
      },
      {
        "criterion": "Required Make commands exit0 on signed source, production regression/mutation evidence is retained, and author landing succeeds under decision321.",
        "reference": "Actual focused target successes, exact signed full make test0 and default author landing0 detailed in summary. Final read-through native negative control /tmp/issue-73-final-control-live-owner-refresh-disabled348000.log Make2 (Stored true, Compose false,15s budget, same retained owner/no process_entry failure); /tmp/issue-73-final-controls.json hashes. Final native None→actual Event witness /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/gui-events-app-rI2ZAm/canonical-reconciliation.json SHA2565ca67962db0e342f80608e8a0115d37d11520db2c029a9a41f84fed5129549ee. Approved agreements adopted under449."
      }
    ],
    "exit_code": 0
  }
}
```
