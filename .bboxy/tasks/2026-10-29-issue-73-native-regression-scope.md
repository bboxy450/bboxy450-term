---
acceptance:
- The real native app GUI fixture reproduces the retained-backlog visibility/receipt failure with private identities.
- Bus catalogue test expectations assert actual authorized direct subjects and remain strict without a broadened wildcard or arbitrary count relaxation.
- The focused Make gate exits0 on the final signed source with regressions/mutations demonstrating the production fix and source/receipt/frame identities.
allowed_paths:
- crates/bboxy450-term/tests/gui_events.rs
- crates/bbt-mux/src/bus.rs
artifacts:
- '[[01M64GAQQF32S804T6HKXP81V2]]'
assignee: seat:01M649SZ31YZ93Z4VGP48Z0KH6
authorship:
  device_id: 01M677XWZPW1XGTQYYGA50W74M
  issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
  kind: device
  principal: operator:01M677XWZPW1XGTQYYGA50W74M
  public_key: cHjhL+5pI9VNQE+Uhb4HJBnYoPqSIH7aCYU+b9MElWs=
  signer: device:01M677XWZPW1XGTQYYGA50W74M
created: 2026-10-29T18:23:42.193Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
delegated_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64EEK3ASZ3PCT8CSNYC2Y7B
issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
number: 386
provenance:
  digest: sha256:9248cafc750715af93bafd83f02092210ff51c4925804c37b441abde8c0e2b73
  frontmatter_digest: sha256:62f3186efaf889384079007d87b0def21ec0ee5fd9d110915c1b092fdf6b4699
  log: .bboxy/log/2026-10-30.jsonl#803
  sig: 8ou+IDc8X258fUI/MlZSUaNxzkFRMkcne/wOTLYZb7uYBhbwMePhqkp+AlGuRhaw/QtNOcCyTA+9xGn6Ig+XCQ==
  signer: device:01M677XWZPW1XGTQYYGA50W74M
reconciliation:
- by: operator:01M677XWZPW1XGTQYYGA50W74M
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/d70e94d0c299edb97d2bcfbb0161c155d236ea4a
  - https://github.com/bboxy450/bboxy450-term/commit/83fdacb3890f8d77c183af644c09fd435f484b8e
  fresh_accepted: false
  kind: historical
  op_id: 01M6786032W7WE0Q338QJ3DJNT
  previous_status: reported
  reason: Operator-authorized historical msgrec reconciliation, 2026-10-30; implementation already landed on main; no new acceptance claimed.
report:
  commit: d70e94d0c299edb97d2bcfbb0161c155d236ea4a
  op_id: 01M64WX63J329NT4H0J2AK7HX6
  reported_by: seat:01M649SZ31YZ93Z4VGP48Z0KH6
  summary: 'Native regression and strict bus-catalogue supplement implemented, including truthful same-owner warm348000 retained-frame proof and real canonical None→Event reconciliation. Final signed Boxy commit d70e94d0c299edb97d2bcfbb0161c155d236ea4a (G, approved 844ECC0587DE7AB0). Own worktree /Users/feral/mydev/bboxy450-term-issue-73-20261029 clean. Actual standalone `make test` exit0 on exact d70: /tmp/issue-73-d70-resource-rechecked.log and /tmp/issue-73-ci-d70-green.log (SHA256 eaf97eb3b4ffb33e91e8828d34f1dc21906ca84ca047eb5947ba9c7ff532d11d); normal 27 invariant checks0,11 slow skipped. Actual focused invocation `make test-chat-sections test-gui-events test-boundaries test-scene-template test-licenses test-task-contract test` at d70: all six focused targets passed; appended test failed only recovery free-disk guard (whole invocation exit2), then same-source standalone test0 after own inactive artifact preservation. Log /tmp/issue-73-rebased-d70-gates.log; old failed full /tmp/issue-73-ci-d70.log retained. Canonical `sh scripts/land.sh` exit0, /tmp/issue-73-author-landing-default.log/.exit; selected exact-source code gate exit0,20 affected invariants0,7 unaffected+11 slow skipped, compile595.726s/run259.168s, existing caps unchanged; remote refs/heads/main independently matched d70. Earlier optional --gate-log invocation exit2 gate_log_requires_wide_cut before push, then authorized default command succeeded; no bypass/force/manual push. No install/release/operator input/real SSH/model/desktop action. Reports are handoffs for independent verification, not acceptance. Fixture-only2ms/64 seeding yield prevents external test writer starving retained owner; production2s SQLite busy timeout and frame/control/startup/input limits unchanged. Earlier false-warm fatal lock/restart and cold34800030s startup failure preserved; warm proof does not erase either or prove production75/combined activation. Oracle retains whole73 combined rollout verification. No public wire change, broad subscription, optimistic message, fabricated ACK, history deletion or read-mark advance.'
role: executor
status: done
swarm_id: 01M649102FBS60QWNPZS5VAGF9
title: 'Supplement378: actual native frame and bus catalogue regression for issue73'
type: task
updated: 2026-10-30T20:31:52.533Z
updated_by: device:01M677XWZPW1XGTQYYGA50W74M
verify: make test-gui-events
verify_result:
  command: make test
  evidence:
  - criterion: The real native app GUI fixture reproduces the retained-backlog visibility/receipt failure with private identities.
    reference: crates/bboxy450-term/tests/gui_events.rs:31,:239,:523,:613 — headless private account/HOME/socket43 retained memberships7 swarms, real348000 legacy receipt rows, exact same live owner birth/PID before/after +backend_ready +no process_entry failures; final produced frame /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/gui-events-app-2EQOEn/backlog-witness.json (d70,95ms compose/48ms input,frame20,real recipient-stored ACK0; SHA25662d889183d9dca1b0a435f8aec975e4c7f50990432e537074b83e53924b5d2f3). Separate16384 actual logic restart component retained. Labelled signed synthetic legacy workload, not fabricated model ACK.
  - criterion: Bus catalogue test expectations assert actual authorized direct subjects and remain strict without a broadened wildcard or arbitrary count relaxation.
    reference: crates/bbt-mux/src/bus.rs:2217 — strict108 = exact43 membership+43 canonical direct inbox+7 swarm+fixed subjects; sorted actual direct subjects equal authorized registry-derived list, no arbitrary count relaxation, production bus untouched. service.rs:3177 exact-authority regression. Original no-direct-subject mutation /tmp/issue-73-mutant-subjects.log exits Make2; historical source recorded honestly in .bboxy/runs/issue-73-20261029.json.
  - criterion: The focused Make gate exits0 on the final signed source with regressions/mutations demonstrating the production fix and source/receipt/frame identities.
    reference: 'Final d70 test-gui-events target completed successfully within actual multi-target invocation /tmp/issue-73-rebased-d70-gates.log (whole invocation2 solely appended full disk guard); independent same-source make test0 /tmp/issue-73-ci-d70-green.log. Disabled canonical refresh348000 full native control Make2 with real Stored true/Compose false and same owner; /tmp/issue-73-final-control-live-owner-refresh-disabled348000.log. Canonical None rejection restored control Make2 /tmp/issue-73-final-controls.json. Final real canonical None→publication→consumer event ACK exactly one rendered message: /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/gui-events-app-rI2ZAm/canonical-reconciliation.json hash5ca67962db0e342f80608e8a0115d37d11520db2c029a9a41f84fed5129549ee. Application ACK remains separate.'
  exit_code: 0
---
# Supplement to task378

ROLE: EXECUTOR — Lumen Foxglove. Same assigned worktree /Users/feral/mydev/bboxy450-term-issue-73-20261029 and branch exec-issue-73-20261029. This additive native assignment grants the two exact files missing from task378's original allowed_paths. All original task378 constraints and required final make test/author landing remain in force.

Scope: extend crates/bboxy450-term/tests/gui_events.rs (actual app-produced frame fixture) and ONLY the existing test module/assertions in crates/bbt-mux/src/bus.rs for the authorized direct-seat subject catalogue and retained-history witness. Production bus replay/ACK/cursor logic is NOT authorized by this supplement; the Architect is writing the precise boundary disposition as a separate assigned task. Continue exact subject and unsuccessful-attempt fixes under378 while that contract is specified.

No protocol, public API, counts/budgets, read-mark, grant, live app, credentials, key, runtime reset or peer-target change. Use canonical make test-gui-events; repeat final make test on exact signed candidate. Keep raw evidence outside Git. Report both scoped tasks with their own criteria evidence; no GitHub closure by Executor.

## Report from seat:01M649SZ31YZ93Z4VGP48Z0KH6

Native regression and strict bus-catalogue supplement implemented, including truthful same-owner warm348000 retained-frame proof and real canonical None→Event reconciliation. Final signed Boxy commit d70e94d0c299edb97d2bcfbb0161c155d236ea4a (G, approved 844ECC0587DE7AB0). Own worktree /Users/feral/mydev/bboxy450-term-issue-73-20261029 clean. Actual standalone `make test` exit0 on exact d70: /tmp/issue-73-d70-resource-rechecked.log and /tmp/issue-73-ci-d70-green.log (SHA256 eaf97eb3b4ffb33e91e8828d34f1dc21906ca84ca047eb5947ba9c7ff532d11d); normal 27 invariant checks0,11 slow skipped. Actual focused invocation `make test-chat-sections test-gui-events test-boundaries test-scene-template test-licenses test-task-contract test` at d70: all six focused targets passed; appended test failed only recovery free-disk guard (whole invocation exit2), then same-source standalone test0 after own inactive artifact preservation. Log /tmp/issue-73-rebased-d70-gates.log; old failed full /tmp/issue-73-ci-d70.log retained. Canonical `sh scripts/land.sh` exit0, /tmp/issue-73-author-landing-default.log/.exit; selected exact-source code gate exit0,20 affected invariants0,7 unaffected+11 slow skipped, compile595.726s/run259.168s, existing caps unchanged; remote refs/heads/main independently matched d70. Earlier optional --gate-log invocation exit2 gate_log_requires_wide_cut before push, then authorized default command succeeded; no bypass/force/manual push. No install/release/operator input/real SSH/model/desktop action. Reports are handoffs for independent verification, not acceptance. Fixture-only2ms/64 seeding yield prevents external test writer starving retained owner; production2s SQLite busy timeout and frame/control/startup/input limits unchanged. Earlier false-warm fatal lock/restart and cold34800030s startup failure preserved; warm proof does not erase either or prove production75/combined activation. Oracle retains whole73 combined rollout verification. No public wire change, broad subscription, optimistic message, fabricated ACK, history deletion or read-mark advance.

```json
{
  "commit": "d70e94d0c299edb97d2bcfbb0161c155d236ea4a",
  "op_id": "01M64WX63J329NT4H0J2AK7HX6",
  "verify_result": {
    "command": "make test",
    "evidence": [
      {
        "criterion": "The real native app GUI fixture reproduces the retained-backlog visibility/receipt failure with private identities.",
        "reference": "crates/bboxy450-term/tests/gui_events.rs:31,:239,:523,:613 — headless private account/HOME/socket43 retained memberships7 swarms, real348000 legacy receipt rows, exact same live owner birth/PID before/after +backend_ready +no process_entry failures; final produced frame /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/gui-events-app-2EQOEn/backlog-witness.json (d70,95ms compose/48ms input,frame20,real recipient-stored ACK0; SHA25662d889183d9dca1b0a435f8aec975e4c7f50990432e537074b83e53924b5d2f3). Separate16384 actual logic restart component retained. Labelled signed synthetic legacy workload, not fabricated model ACK."
      },
      {
        "criterion": "Bus catalogue test expectations assert actual authorized direct subjects and remain strict without a broadened wildcard or arbitrary count relaxation.",
        "reference": "crates/bbt-mux/src/bus.rs:2217 — strict108 = exact43 membership+43 canonical direct inbox+7 swarm+fixed subjects; sorted actual direct subjects equal authorized registry-derived list, no arbitrary count relaxation, production bus untouched. service.rs:3177 exact-authority regression. Original no-direct-subject mutation /tmp/issue-73-mutant-subjects.log exits Make2; historical source recorded honestly in .bboxy/runs/issue-73-20261029.json."
      },
      {
        "criterion": "The focused Make gate exits0 on the final signed source with regressions/mutations demonstrating the production fix and source/receipt/frame identities.",
        "reference": "Final d70 test-gui-events target completed successfully within actual multi-target invocation /tmp/issue-73-rebased-d70-gates.log (whole invocation2 solely appended full disk guard); independent same-source make test0 /tmp/issue-73-ci-d70-green.log. Disabled canonical refresh348000 full native control Make2 with real Stored true/Compose false and same owner; /tmp/issue-73-final-control-live-owner-refresh-disabled348000.log. Canonical None rejection restored control Make2 /tmp/issue-73-final-controls.json. Final real canonical None→publication→consumer event ACK exactly one rendered message: /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/gui-events-app-rI2ZAm/canonical-reconciliation.json hash5ca67962db0e342f80608e8a0115d37d11520db2c029a9a41f84fed5129549ee. Application ACK remains separate."
      }
    ],
    "exit_code": 0
  }
}
```
