---
acceptance:
- Explicit live-swarm layout avoids constructing/cloning populated all-seat fallback fields that it immediately replaces; fallback behavior remains complete.
- The preserved equivalence cases and meaningful scoped-row/active-stage assertions pass across theme/density/live/fallback combinations at a fixed clock.
- Paint/pane geometry and layout semantics remain equivalent; a valid functional phase packet is retained without claiming noisy proxy timings prove speedup or solve scale budgets.
- Signed scoped candidate and actual focused/full gate results, unchanged resource/geometry limits and exact missing evidence are reported; no unverified closure or force landing.
allowed_paths:
- crates/bboxy450-term/src/scene.rs
- Makefile
- .bboxy/runs/issue-49-live-swarm-template.json
artifacts:
- '[[01M64S6D5YYEHYKF3ZYX60NF0V]]'
assignee: seat:01M649SZ31YZ93Z4VGP48Z0KH6
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T19:02:50.553Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
delegated_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64GP8DEFNJB91WRXT3HP7MN
number: 445
provenance:
  digest: sha256:789e800286fc34398f56c649f6fcd21a986494976d66bb64cd5ed235b0556e06
  frontmatter_digest: sha256:82c58ad6045ce6893e4fd354c249f2bdde171b93810040724cfee1e124d63b4b
  log: .bboxy/log/2026-10-29.jsonl#2746
  sig: uCJxcb0slAYuxShL/S46cLpCxHB9N5K6e9oDOkhvKSqbc0fo1PHLrET9nOHlYM3unPjiwVJhLb6JiVhPcdtkBw==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
report:
  commit: d70e94d0c299edb97d2bcfbb0161c155d236ea4a
  op_id: 01M64XE7A2WTJG6485ZS3TYKKH
  reported_by: seat:01M649SZ31YZ93Z4VGP48Z0KH6
  summary: 'Verification-field correction: actual standalone `make test-scene-template` exit0 on unchanged signed d70e94d0c299edb97d2bcfbb0161c155d236ea4a; 2 passed,0 failed,1 ignored functional phase. Raw /tmp/issue-49-d70-declared-verify.log SHA2566d4eae9029011a7174b6aa363e38f7ee853802b2c528ab4d27b291bf59aa1c8c, command/exit/source/hash /tmp/issue-49-d70-declared-verify.json. This replaces the previous verify_result.command=make test with the exact declared focused command; earlier standalone full make test0 and landing0 remain separate unchanged-source evidence. No source edits, repeated full gate,72 work, rollout or task acceptance. Resumed preserved scoped live-swarm template allocation cut: skip populated fallback construction when explicit live cards replace it; fallback and stage/paint/pane semantics preserved. Focused equivalence tests, byte-identical old/new production paint/pane control, and valid private functional phase packet retained. Final signed Boxy commit d70e94d0c299edb97d2bcfbb0161c155d236ea4a (G, approved 844ECC0587DE7AB0). Own worktree /Users/feral/mydev/bboxy450-term-issue-73-20261029 clean. Actual standalone `make test` exit0 on exact d70: /tmp/issue-73-d70-resource-rechecked.log and /tmp/issue-73-ci-d70-green.log (SHA256 eaf97eb3b4ffb33e91e8828d34f1dc21906ca84ca047eb5947ba9c7ff532d11d); normal 27 invariant checks0,11 slow skipped. Actual focused invocation `make test-chat-sections test-gui-events test-boundaries test-scene-template test-licenses test-task-contract test` at d70: all six focused targets passed; appended test failed only recovery free-disk guard (whole invocation exit2), then same-source standalone test0 after own inactive artifact preservation. Log /tmp/issue-73-rebased-d70-gates.log; old failed full /tmp/issue-73-ci-d70.log retained. Canonical `sh scripts/land.sh` exit0, /tmp/issue-73-author-landing-default.log/.exit; selected exact-source code gate exit0,20 affected invariants0,7 unaffected+11 slow skipped, compile595.726s/run259.168s, existing caps unchanged; remote refs/heads/main independently matched d70. Earlier optional --gate-log invocation exit2 gate_log_requires_wide_cut before push, then authorized default command succeeded; no bypass/force/manual push. No install/release/operator input/real SSH/model/desktop action. Reports are handoffs for independent verification, not acceptance. Evidence gap: no credible speedup/full967-scale-budget proof. Functional phase timing gate intentionally false per existing packet; observed CPU-p99/input-to-gpu timing failures remain reported. No limits relaxed or performance issue closed; Oracle independently verifies.'
role: executor
status: accepted
swarm_id: 01M649102FBS60QWNPZS5VAGF9
title: 'Issue49: resume preserved live-swarm template allocation cut'
type: task
updated: 2026-10-29T22:47:38.063Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
verify: make test-scene-template
verify_result:
  command: make test-scene-template
  evidence:
  - criterion: Explicit live-swarm layout avoids constructing/cloning populated all-seat fallback fields that it immediately replaces; fallback behavior remains complete.
    reference: 'crates/bboxy450-term/src/scene.rs:1175 — scoped=self.live&&!self.swarms.is_empty(); populate all-seat fallback seats/tabs/extra/closed only when !scoped, while explicit live swarm cards keep actual per-swarm rows/workdir/starting/closed/task behavior and unchanged active stage. Adapted credit retains original signed9739/f240 candidate provenance and project policy. No geometry/budget change. Exact standalone final-source declared verify: /tmp/issue-49-d70-declared-verify.json and /tmp/issue-49-d70-declared-verify.log; command make test-scene-template, exit0,d70,2 passed0failed,log SHA2566d4eae9029011a7174b6aa363e38f7ee853802b2c528ab4d27b291bf59aa1c8c.'
  - criterion: The preserved equivalence cases and meaningful scoped-row/active-stage assertions pass across theme/density/live/fallback combinations at a fixed clock.
    reference: 'scene.rs:2326,:2381 — fixed-clock cases4 theme/density combinations ×live/fallback; meaningful scoped open/closed/starting/task rows, selected and active pane/focus/bounds, original fallback preserved. Preserved invalid collapsed-card/explicit-vs-fallback fixture assumptions corrected rather than weaken real semantics. make test-scene-template passed focused and full d70; exact production old-population control passed same cases. Exact standalone final-source declared verify: /tmp/issue-49-d70-declared-verify.json and /tmp/issue-49-d70-declared-verify.log; command make test-scene-template, exit0,d70,2 passed0failed,log SHA2566d4eae9029011a7174b6aa363e38f7ee853802b2c528ab4d27b291bf59aa1c8c.'
  - criterion: Paint/pane geometry and layout semantics remain equivalent; a valid functional phase packet is retained without claiming noisy proxy timings prove speedup or solve scale budgets.
    reference: 'Old production population vs candidate exact exported paint+pane snapshots byte-identical: /tmp/issue-49-baseline-paint.json and /tmp/issue-49-candidate-paint.json both SHA256ef00fcf93b151c6bdd8052460d1c0ab75b460f63875f2d03a4a99d8656245180, actual cmp0. Valid functional phase at signed ea0d75da (scene hunk unchanged through final d70) /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/scene-template-phase-sblG5g/report.json and report.capture.json: make test-scene-template-phase child0,64 synthetic rows50 watched0real PTYs,5s overlap2s idle,38frames,idle_frames0,guard_violations0,failures[]. timing_gated=false; CPU-p99-over4ms/input-to-gpu-over-two-display-intervals remain timing_failures, host load13.5137→12.5117. Functional result is not speedup or967-scale evidence. Exact standalone final-source declared verify: /tmp/issue-49-d70-declared-verify.json and /tmp/issue-49-d70-declared-verify.log; command make test-scene-template, exit0,d70,2 passed0failed,log SHA2566d4eae9029011a7174b6aa363e38f7ee853802b2c528ab4d27b291bf59aa1c8c.'
  - criterion: Signed scoped candidate and actual focused/full gate results, unchanged resource/geometry limits and exact missing evidence are reported; no unverified closure or force landing.
    reference: '.bboxy/runs/issue-49-live-swarm-template.json (artifact01M64S6D5YYEHYKF3ZYX60NF0V,SHA256d5e4a882d95d89ae6ad8d02edd7028da540b5588e11ace70272924ac1ce42150) contains actual command/exit/hash/source history. Final signed d70 full make test0 and canonical script landing0, clean own tree/remote d70; all caps/geometry unchanged. Earlier noisy phase packet retained, no broad performance claims, no issue49/46–48/50/967-scale acceptance or closure. Geometry parity not rerun because byte-equivalent paint+pane and unchanged layout; actual timings not transferred to d70. Exact standalone final-source declared verify: /tmp/issue-49-d70-declared-verify.json and /tmp/issue-49-d70-declared-verify.log; command make test-scene-template, exit0,d70,2 passed0failed,log SHA2566d4eae9029011a7174b6aa363e38f7ee853802b2c528ab4d27b291bf59aa1c8c.'
  exit_code: 0
---
# Resume preserved issue49 cut

ROLE: EXECUTOR — Lumen Foxglove. Delegator Vesper Greywind. Same existing private writing worktree /Users/feral/mydev/bboxy450-term-issue-73-20261029 and branch exec-issue-73-20261029. Separate exact-path commit from issue73; do not mix their acceptance. You can implement this independent cut while task388's ICD is pending, after the current73 exact-source gate settles or is deliberately stopped as superseded using only your own recorded PIDs/logs. No repeated unchanged gate. A single final exact-source full gate may cover composed scoped commits once both freeze; do not claim an older-SHA pass for newer source.

Read issue49 and actual scene source, .bboxy/interfaces/lab-native.md, logic-gui.md and attribution. Preserved original signed candidate f2403dd26581ba45a9af371de1c4639796e2e8d9, rebased9739ea66a5bf1cd34fe18c6b694f91c0ad9f0548 (scene.rs only130lines). Inspect it; reuse/adapt accurately, do not trust its report or edit another worktree. Its two equivalence tests had never run. Simplest mechanism: only populate all-seat fallback SwarmCard.seats/tabs/extra when !live || swarms.is_empty(); explicit live swarms immediately replace those fields. Preserve all starting/closed/per-swarm rows, paint, pane bounds, focus, ordering and active stage.

Main has advanced to signed d93e96758086c067f1f001b08b403c2d6deffeea (six color-default/real-Codex evidence commits); inspect/rebase deliberately when source freezes. Do not reset other work or broaden this cut to colors. Never compare against a stale origin and call it shipped.

Add a canonical focused make test-scene-template for the existing relevant tests, plus the proportionate equivalence cases needed. If geometry/reference output changes, treat it as a regression to fix, or run the required existing parity gate after an actual authorized geometry decision; no new geometry/design is authorized here. Capture next valid functional phase partition using current owned fixture instrumentation, with actual source/count/load/failure status. The historical synthetic64/50 comparison was load-confounded and does not establish speedup or causal slowdown; do not lower workload, budgets or ignore idle-frame failure to claim success. This task does not solve46–48/50 or full967 performance.

Run focused Make, make test-licenses and final exact-signed-source make test when disk admission permits. Resource owners dispose only their own inactive artifacts, never peer caches/processes/targets or global config. Retain named refusals honestly and continue independent scoped work. No credentials/release/installed app reset, desktop input, recruitment or broad stage. Existing decision321 author scripts/land.sh flow is authorized only after all gates/acceptance, no force push. Report paths/lines, mechanisms, command exits, source SHA/signature and worktree state; Oracle verifies issue49 separately.

## Report from seat:01M649SZ31YZ93Z4VGP48Z0KH6

Resumed preserved scoped live-swarm template allocation cut: skip populated fallback construction when explicit live cards replace it; fallback and stage/paint/pane semantics preserved. Focused equivalence tests, byte-identical old/new production paint/pane control, and valid private functional phase packet retained. Final signed Boxy commit d70e94d0c299edb97d2bcfbb0161c155d236ea4a (G, approved 844ECC0587DE7AB0). Own worktree /Users/feral/mydev/bboxy450-term-issue-73-20261029 clean. Actual standalone `make test` exit0 on exact d70: /tmp/issue-73-d70-resource-rechecked.log and /tmp/issue-73-ci-d70-green.log (SHA256 eaf97eb3b4ffb33e91e8828d34f1dc21906ca84ca047eb5947ba9c7ff532d11d); normal 27 invariant checks0,11 slow skipped. Actual focused invocation `make test-chat-sections test-gui-events test-boundaries test-scene-template test-licenses test-task-contract test` at d70: all six focused targets passed; appended test failed only recovery free-disk guard (whole invocation exit2), then same-source standalone test0 after own inactive artifact preservation. Log /tmp/issue-73-rebased-d70-gates.log; old failed full /tmp/issue-73-ci-d70.log retained. Canonical `sh scripts/land.sh` exit0, /tmp/issue-73-author-landing-default.log/.exit; selected exact-source code gate exit0,20 affected invariants0,7 unaffected+11 slow skipped, compile595.726s/run259.168s, existing caps unchanged; remote refs/heads/main independently matched d70. Earlier optional --gate-log invocation exit2 gate_log_requires_wide_cut before push, then authorized default command succeeded; no bypass/force/manual push. No install/release/operator input/real SSH/model/desktop action. Reports are handoffs for independent verification, not acceptance. Evidence gap: no credible speedup/full967-scale-budget proof. Functional phase timing gate intentionally false per existing packet; observed CPU-p99/input-to-gpu timing failures remain reported. No limits relaxed or performance issue closed; Oracle independently verifies.

```json
{
  "commit": "d70e94d0c299edb97d2bcfbb0161c155d236ea4a",
  "op_id": "01M64WYXHYAEE7W7C7BR7ZTB8C",
  "verify_result": {
    "command": "make test",
    "evidence": [
      {
        "criterion": "Explicit live-swarm layout avoids constructing/cloning populated all-seat fallback fields that it immediately replaces; fallback behavior remains complete.",
        "reference": "crates/bboxy450-term/src/scene.rs:1175 — scoped=self.live&&!self.swarms.is_empty(); populate all-seat fallback seats/tabs/extra/closed only when !scoped, while explicit live swarm cards keep actual per-swarm rows/workdir/starting/closed/task behavior and unchanged active stage. Adapted credit retains original signed9739/f240 candidate provenance and project policy. No geometry/budget change."
      },
      {
        "criterion": "The preserved equivalence cases and meaningful scoped-row/active-stage assertions pass across theme/density/live/fallback combinations at a fixed clock.",
        "reference": "scene.rs:2326,:2381 — fixed-clock cases4 theme/density combinations ×live/fallback; meaningful scoped open/closed/starting/task rows, selected and active pane/focus/bounds, original fallback preserved. Preserved invalid collapsed-card/explicit-vs-fallback fixture assumptions corrected rather than weaken real semantics. make test-scene-template passed focused and full d70; exact production old-population control passed same cases."
      },
      {
        "criterion": "Paint/pane geometry and layout semantics remain equivalent; a valid functional phase packet is retained without claiming noisy proxy timings prove speedup or solve scale budgets.",
        "reference": "Old production population vs candidate exact exported paint+pane snapshots byte-identical: /tmp/issue-49-baseline-paint.json and /tmp/issue-49-candidate-paint.json both SHA256ef00fcf93b151c6bdd8052460d1c0ab75b460f63875f2d03a4a99d8656245180, actual cmp0. Valid functional phase at signed ea0d75da (scene hunk unchanged through final d70) /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/scene-template-phase-sblG5g/report.json and report.capture.json: make test-scene-template-phase child0,64 synthetic rows50 watched0real PTYs,5s overlap2s idle,38frames,idle_frames0,guard_violations0,failures[]. timing_gated=false; CPU-p99-over4ms/input-to-gpu-over-two-display-intervals remain timing_failures, host load13.5137→12.5117. Functional result is not speedup or967-scale evidence."
      },
      {
        "criterion": "Signed scoped candidate and actual focused/full gate results, unchanged resource/geometry limits and exact missing evidence are reported; no unverified closure or force landing.",
        "reference": ".bboxy/runs/issue-49-live-swarm-template.json (artifact01M64S6D5YYEHYKF3ZYX60NF0V,SHA256d5e4a882d95d89ae6ad8d02edd7028da540b5588e11ace70272924ac1ce42150) contains actual command/exit/hash/source history. Final signed d70 full make test0 and canonical script landing0, clean own tree/remote d70; all caps/geometry unchanged. Earlier noisy phase packet retained, no broad performance claims, no issue49/46–48/50/967-scale acceptance or closure. Geometry parity not rerun because byte-equivalent paint+pane and unchanged layout; actual timings not transferred to d70."
      }
    ],
    "exit_code": 0
  }
}
```


## Report from seat:01M649SZ31YZ93Z4VGP48Z0KH6

Verification-field correction: actual standalone `make test-scene-template` exit0 on unchanged signed d70e94d0c299edb97d2bcfbb0161c155d236ea4a; 2 passed,0 failed,1 ignored functional phase. Raw /tmp/issue-49-d70-declared-verify.log SHA2566d4eae9029011a7174b6aa363e38f7ee853802b2c528ab4d27b291bf59aa1c8c, command/exit/source/hash /tmp/issue-49-d70-declared-verify.json. This replaces the previous verify_result.command=make test with the exact declared focused command; earlier standalone full make test0 and landing0 remain separate unchanged-source evidence. No source edits, repeated full gate,72 work, rollout or task acceptance. Resumed preserved scoped live-swarm template allocation cut: skip populated fallback construction when explicit live cards replace it; fallback and stage/paint/pane semantics preserved. Focused equivalence tests, byte-identical old/new production paint/pane control, and valid private functional phase packet retained. Final signed Boxy commit d70e94d0c299edb97d2bcfbb0161c155d236ea4a (G, approved 844ECC0587DE7AB0). Own worktree /Users/feral/mydev/bboxy450-term-issue-73-20261029 clean. Actual standalone `make test` exit0 on exact d70: /tmp/issue-73-d70-resource-rechecked.log and /tmp/issue-73-ci-d70-green.log (SHA256 eaf97eb3b4ffb33e91e8828d34f1dc21906ca84ca047eb5947ba9c7ff532d11d); normal 27 invariant checks0,11 slow skipped. Actual focused invocation `make test-chat-sections test-gui-events test-boundaries test-scene-template test-licenses test-task-contract test` at d70: all six focused targets passed; appended test failed only recovery free-disk guard (whole invocation exit2), then same-source standalone test0 after own inactive artifact preservation. Log /tmp/issue-73-rebased-d70-gates.log; old failed full /tmp/issue-73-ci-d70.log retained. Canonical `sh scripts/land.sh` exit0, /tmp/issue-73-author-landing-default.log/.exit; selected exact-source code gate exit0,20 affected invariants0,7 unaffected+11 slow skipped, compile595.726s/run259.168s, existing caps unchanged; remote refs/heads/main independently matched d70. Earlier optional --gate-log invocation exit2 gate_log_requires_wide_cut before push, then authorized default command succeeded; no bypass/force/manual push. No install/release/operator input/real SSH/model/desktop action. Reports are handoffs for independent verification, not acceptance. Evidence gap: no credible speedup/full967-scale-budget proof. Functional phase timing gate intentionally false per existing packet; observed CPU-p99/input-to-gpu timing failures remain reported. No limits relaxed or performance issue closed; Oracle independently verifies.

```json
{
  "commit": "d70e94d0c299edb97d2bcfbb0161c155d236ea4a",
  "op_id": "01M64XE7A2WTJG6485ZS3TYKKH",
  "verify_result": {
    "command": "make test-scene-template",
    "evidence": [
      {
        "criterion": "Explicit live-swarm layout avoids constructing/cloning populated all-seat fallback fields that it immediately replaces; fallback behavior remains complete.",
        "reference": "crates/bboxy450-term/src/scene.rs:1175 — scoped=self.live&&!self.swarms.is_empty(); populate all-seat fallback seats/tabs/extra/closed only when !scoped, while explicit live swarm cards keep actual per-swarm rows/workdir/starting/closed/task behavior and unchanged active stage. Adapted credit retains original signed9739/f240 candidate provenance and project policy. No geometry/budget change. Exact standalone final-source declared verify: /tmp/issue-49-d70-declared-verify.json and /tmp/issue-49-d70-declared-verify.log; command make test-scene-template, exit0,d70,2 passed0failed,log SHA2566d4eae9029011a7174b6aa363e38f7ee853802b2c528ab4d27b291bf59aa1c8c."
      },
      {
        "criterion": "The preserved equivalence cases and meaningful scoped-row/active-stage assertions pass across theme/density/live/fallback combinations at a fixed clock.",
        "reference": "scene.rs:2326,:2381 — fixed-clock cases4 theme/density combinations ×live/fallback; meaningful scoped open/closed/starting/task rows, selected and active pane/focus/bounds, original fallback preserved. Preserved invalid collapsed-card/explicit-vs-fallback fixture assumptions corrected rather than weaken real semantics. make test-scene-template passed focused and full d70; exact production old-population control passed same cases. Exact standalone final-source declared verify: /tmp/issue-49-d70-declared-verify.json and /tmp/issue-49-d70-declared-verify.log; command make test-scene-template, exit0,d70,2 passed0failed,log SHA2566d4eae9029011a7174b6aa363e38f7ee853802b2c528ab4d27b291bf59aa1c8c."
      },
      {
        "criterion": "Paint/pane geometry and layout semantics remain equivalent; a valid functional phase packet is retained without claiming noisy proxy timings prove speedup or solve scale budgets.",
        "reference": "Old production population vs candidate exact exported paint+pane snapshots byte-identical: /tmp/issue-49-baseline-paint.json and /tmp/issue-49-candidate-paint.json both SHA256ef00fcf93b151c6bdd8052460d1c0ab75b460f63875f2d03a4a99d8656245180, actual cmp0. Valid functional phase at signed ea0d75da (scene hunk unchanged through final d70) /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/scene-template-phase-sblG5g/report.json and report.capture.json: make test-scene-template-phase child0,64 synthetic rows50 watched0real PTYs,5s overlap2s idle,38frames,idle_frames0,guard_violations0,failures[]. timing_gated=false; CPU-p99-over4ms/input-to-gpu-over-two-display-intervals remain timing_failures, host load13.5137→12.5117. Functional result is not speedup or967-scale evidence. Exact standalone final-source declared verify: /tmp/issue-49-d70-declared-verify.json and /tmp/issue-49-d70-declared-verify.log; command make test-scene-template, exit0,d70,2 passed0failed,log SHA2566d4eae9029011a7174b6aa363e38f7ee853802b2c528ab4d27b291bf59aa1c8c."
      },
      {
        "criterion": "Signed scoped candidate and actual focused/full gate results, unchanged resource/geometry limits and exact missing evidence are reported; no unverified closure or force landing.",
        "reference": ".bboxy/runs/issue-49-live-swarm-template.json (artifact01M64S6D5YYEHYKF3ZYX60NF0V,SHA256d5e4a882d95d89ae6ad8d02edd7028da540b5588e11ace70272924ac1ce42150) contains actual command/exit/hash/source history. Final signed d70 full make test0 and canonical script landing0, clean own tree/remote d70; all caps/geometry unchanged. Earlier noisy phase packet retained, no broad performance claims, no issue49/46–48/50/967-scale acceptance or closure. Geometry parity not rerun because byte-equivalent paint+pane and unchanged layout; actual timings not transferred to d70. Exact standalone final-source declared verify: /tmp/issue-49-d70-declared-verify.json and /tmp/issue-49-d70-declared-verify.log; command make test-scene-template, exit0,d70,2 passed0failed,log SHA2566d4eae9029011a7174b6aa363e38f7ee853802b2c528ab4d27b291bf59aa1c8c."
      }
    ],
    "exit_code": 0
  }
}
```
