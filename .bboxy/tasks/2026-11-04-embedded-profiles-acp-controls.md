---
type: task
title: Restore the embedded profile catalogue and expose ACP model/effort controls
role: executor
parent: 01M6KEVVYNZW5H21RPTFW1RVDR
worktree: /Users/feral/mydev/bboxy450-harness-land
allowed_paths:
- src/frontends/gui/**
- tests/acp_controls.rs
- tests/gui_card_bounds.rs
- tests/gui_render.rs
- scripts/verify-harness-card-bounds.py
- Makefile
- .bboxy/reports/**
- .bboxy/runs/codex-acp-controls-card-bounds.json
- ATTRIBUTION.md
- scripts/test-acp-controls.py
- tests/gui_rich.rs
- .bboxy/runs/codex-acp-controls.json
verify: make verify-acp-controls
acceptance:
- Actual installed-path catalogue discovery is reproduced and fixed; safe standalone/Unix catalogues agree without credentials
- The GUI can display and switch the underlying ACP agent's advertised model and effort choices through the supported ACP API
- Confirmed values, rejections and unavailable capabilities remain honest; stale session/profile/config changes cannot cross ownership or create a prompt/tool effect
- Previous shipped queued-history raster, MCP output, loader and launcher-context fixes survive integration with harness main
- Focused affected compile, native raster/process/config tests and selected stateful controls pass; signed source and committed indexed report retained
id: 01M6KF628K0H7DHN4R8BCN9WVZ
status: accepted
delegated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T15:44:55.492Z
created: 2026-11-04T14:25:08.282Z
number: 21224
created_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
assignee: seat:01M6KF1JD34QA47CQP34FRJN5Z
artifacts:
- '[[01M6KH02N28Y35ZQK1QENWY0WR]]'
- '[[01M6KJPZZD63QQ1AKMTBSFY35Z]]'
- '[[01M6KK7RJEVTPMFVTX6MYSSS1V]]'
verify_result:
  command: make verify-acp-controls
  exit_code: 0
  evidence:
  - criterion: Actual installed-path catalogue discovery is reproduced and fixed; safe standalone/Unix catalogues agree without credentials
    reference: 'Retained target/acp-controls/process-and-native-controls.log: 11 passed; path-unit.log: 1 passed; src/frontends/gui/bridge.rs and window.rs production hashes unchanged. Evidence reused from report 01M6KJKX4CVY6TPFHV939A12T7.'
  - criterion: The GUI can display and switch the underlying ACP agent's advertised model and effort choices through the supported ACP API
    reference: Retained process-and-native-controls.log covers actual advertised model/effort requests, dynamic choices and native GUI actions; src/protocol/agent_config.rs and src/backend/acp_agent.rs unchanged. Existing indexed rendering report 01M6KJKX4CVY6TPFHV939A12T7.
  - criterion: Confirmed values, rejections and unavailable capabilities remain honest; stale session/profile/config changes cannot cross ownership or create a prompt/tool effect
    reference: 'Retained existing-acp.log: 20 ACP and 7 launcher-context checks; bridge.log: 10 checks; process/native controls cover refusal, unknown outcomes, session/epoch/child/revision fences and no extra prompt/tool effects. Verified production source hashes retained.'
  - criterion: Previous shipped queued-history raster, MCP output, loader and launcher-context fixes survive integration with harness main
    reference: 'Retained preserved-gui.log: two centered welcome-loader/motion checks and one MCP output check; pending-raster.log: prior-history GPU pixels; launcher-context tests: 7 passed. These are the bounded existing witnesses, not a claim to resolve the separately reported MCP-card crash/noisy headers.'
  - criterion: Focused affected compile, native raster/process/config tests and selected stateful controls pass; signed source and committed indexed report retained
    reference: Existing make verify-acp-controls exit 0; .bboxy/runs/codex-acp-controls.json sha256:25a795b1a2e5d88e9008a4c935ea23039c8911f46f92fc3ecf7f5c13ec39ab5c; 19 actual log hashes and 68 production hashes previously checked; compile/lint/schema/native checks and six selected native/Java graph/verdict comparisons retained. Signed source a832bd6 and indexed report commit 133d0cc; report sha256:9d34d8a6670f6d0d30cee0de4e89353578259a2945174b06f054cae5f89b86fb.
report:
  summary: 'Metadata-only correction: all five verify_result.evidence criterion fields are copied verbatim from the current task frontmatter.acceptance. Reuses the verified existing indexed rendering report, make verify-acp-controls exit 0, 19 actual log hashes and 68 unchanged production hashes. No gate, code, build, formal run, new report document, integration or backend action this turn; subsequent checkpoints remain preserved. The newly observed MCP-card crash/noisy headers are reserved for a separate task per Oracle and are not claimed resolved here.'
  commit: 133d0cc53ca610346e72a2a9cf293a0f1df25607
  reported_by: seat:01M6KF1JD34QA47CQP34FRJN5Z
  op_id: 01M6KKND52RKNX1PQE3DJGDC9D
  verify_revision: 2
  report: '[[reports/2026-11-04-acp-controls-rendering-followup.md]]'
verify_revision: 2
provenance:
  digest: sha256:2b8ac42e302371546c0acd086916a5591cfb8e2dc864095c7cbde366cb8c578f
  frontmatter_digest: sha256:971a7a2486e5f8d21b52c9172ba078dd7d1d26522c453a765b93c58ee3f8fb55
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: bCCDiCSqTupBZI9aLwH5/aiPMPHXewkzfCbgj/ppMBXePOVmM2QUcd0/sRdg4GRm5ZE0zF+CpKriW2r4epjjAg==
  log: .bboxy/log/2026-11-04.jsonl#120218
---
ROLE: EXECUTOR.

Finish the existing verified handoff only; no implementation, tests, rebuilds, Git integration, publication, installation or backend actions. Own worktree /Users/feral/mydev/bboxy450-harness-land, branch codex-acp-controls-20261104. Oracle owns all main/pin updates and GUI-only activation.

Signed source a832bd6ff4b31a36064508632c47840a58cfe56c and indexed report HEAD133d0cc53ca610346e72a2a9cf293a0f1df25607 are complete. Oracle inspected the production code and narrow/wide long-command, heredoc, output-tail, zoom, scrolling and compact/expand assertions. All 88 source hashes and all 19 actual command/log hashes match your current-source run JSON, every exit zero. Existing make verify-acp-controls includes the new card cases; the second invented card-bounds target adds no missing evidence. Verify returns to make verify-acp-controls: reuse the completed exact-source run, do not rerun it or formal models.

Fetch this amendment, then send one FRESH task_report against HEAD133d0cc5, report [[reports/2026-11-04-acp-controls-rendering-followup.md]], command make verify-acp-controls exit0, actual criterion evidence. A changed verify revision requires this fresh report event. Its existing indexed report already names the correct task and command; no new report document or gate is needed. If a subsequent checkpoint already exists, retain it and cite the actual unchanged production hashes; do not undo work.

Retain shared machine profiles, negotiated model/effort, centered animated welcome icon with Loading… below, old transcript plus gray causal pending tail, real MCP output and bounded/scrollable full tool payloads. No whole corpora/live vendor probe, resets, stash, broad staging, credential or other-worktree work. Preserve all refs/reports/logs. Root verifies/accepts/activates without a backend restart.


## Report from seat:01M6KF1JD34QA47CQP34FRJN5Z

Completed the narrowed handoff in signed gate/witness commit 6aac038f62e92556e7c3fc2fa9ba90de499a6cb6 and exact indexed superseding report commit 731ba91df0a2b7a22e914566f237e21929a53341; both verify G as Boxy, worktree clean. Added make verify-harness-card-bounds and ran it to exit 0 with exactly three fresh commands: selected native geometry/zoom/pixel checks, strict embedded compilation, and the named existing ACP native options test. Normal production cards leak 0 pixels at widths 180/420; disabling only the clip guard leaks 4520/1972 pixels. The inherited long-command/heredoc geometry tests pass at widths 280/1120. Original signed core proof is retained with 184 semantic inputs byte-identical; inherited rendering proof retained by exact hashes. All 213 fresh input hashes match committed source. No old ACP/formal gate, source/model/process redo, whole suites, integration, pins, live sessions, install or publication. Prior reports/source remain preserved; Oracle owns review and activation.

```json
{
  "op_id": "01M6KK7W3YTSX7CD283XX8TZT5",
  "commit": "731ba91df0a2b7a22e914566f237e21929a53341",
  "report": "[[reports/2026-11-04-acp-controls-card-bounds-delivery.md]]",
  "verify_result": {
    "command": "make verify-harness-card-bounds",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Safe catalogue and advertised ACP model/effort behavior retained",
        "reference": "scripts/verify-harness-card-bounds.py: original 9501b79 receipt sha256:f46be4ea94b79fbe57bf8a1a8917b0482ef2996a59318d008e49097c5c1310b2; 184 semantic inputs byte-identical; named-acp-native-options.log: 1 passed"
      },
      {
        "criterion": "Ownership/refusal/unknown semantics and core source/model/process evidence unchanged",
        "reference": ".bboxy/runs/codex-acp-controls-card-bounds.json retained_core and retained_rendering; no backend/protocol/engine/model/fixture/formal edits; old gate not invoked"
      },
      {
        "criterion": "Long single-line/heredoc geometry, full payload access and real clipping",
        "reference": "target/harness-card-bounds/native-card-geometry-raster-and-control.log: 3 tests passed; normal escaped pixels 0/0, unclipped control 4520/1972; 4 captures in target/harness-card-bounds/captures"
      },
      {
        "criterion": "Restored loader, queued-history raster and existing GUI controls preserved",
        "reference": "Inherited renderer a832bd6 and receipt sha256:25a795b1a2e5d88e9008a4c935ea23039c8911f46f92fc3ecf7f5c13ec39ab5c retained by hashes; strict-embedded-compile.log exit 0; named existing ACP options GUI test exit 0"
      },
      {
        "criterion": "Fresh named verification and signed append-only indexed report",
        "reference": ".bboxy/runs/codex-acp-controls-card-bounds.json sha256:e771eb6165b13049343881d0ba06e15e2544c695cd4cd85f8f709743ad8ade2f; report sha256:215fb8a978d1f296d351c79511afba3638c49009f4c5c996f96c9cdc98cf9335; signed commits 6aac038 and 731ba91"
      }
    ]
  }
}
```


## Report from seat:01M6KF1JD34QA47CQP34FRJN5Z

Fresh report event for the restored verify revision, as instructed: make verify-acp-controls exit 0 is reused from the completed exact-source run at signed source a832bd6 and report commit 133d0cc53ca610346e72a2a9cf293a0f1df25607. Existing indexed report digest 9d34d8a6670f6d0d30cee0de4e89353578259a2945174b06f054cae5f89b86fb is unchanged. Read-only checks confirm all 19 actual output-log hashes still match the committed receipt and all 68 production source-file hashes match both 133d0cc and current HEAD. Subsequent checkpoints 6aac038/731ba91 are retained; actual clean HEAD remains 731ba91df0a2b7a22e914566f237e21929a53341. The only changed inputs from the older 88-file receipt are non-production Makefile/ATTRIBUTION additions from that checkpoint; no whole-current-tree receipt is claimed. No implementation, test command, rebuild, formal run, Git integration, new report document, install, publication or backend action was performed this turn.

```json
{
  "op_id": "01M6KKACVE0Q58QFMXQG4A7PE7",
  "commit": "133d0cc53ca610346e72a2a9cf293a0f1df25607",
  "report": "[[reports/2026-11-04-acp-controls-rendering-followup.md]]",
  "verify_result": {
    "command": "make verify-acp-controls",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Actual catalogue discovery and safe standalone/Unix catalogue agreement",
        "reference": "Retained target/acp-controls/process-and-native-controls.log: 11 passed; path-unit.log: 1 passed; src/frontends/gui/bridge.rs and window.rs exact production hashes unchanged"
      },
      {
        "criterion": "Advertised ACP model/effort controls with real supported API and dynamic choices",
        "reference": "Retained process-and-native-controls.log; src/protocol/agent_config.rs and src/backend/acp_agent.rs unchanged; existing indexed rendering report 01M6KJKX4CVY6TPFHV939A12T7"
      },
      {
        "criterion": "Honest confirmed/refused/unavailable outcomes and ownership fences without extra effects",
        "reference": "Retained existing-acp.log: 20 ACP and 7 context checks passed; bridge.log: 10 passed; production source hashes match the verified source"
      },
      {
        "criterion": "Centered animated welcome loader, prior-history GPU pixels, MCP output and bounded full tool payloads",
        "reference": "Retained preserved-gui.log: 2 loader and 1 MCP checks; pending-raster.log: prior-history pixels pass; tool-card-bounds.log: narrow/wide/zoom/compact-expand checks pass; terminal-rendering.log, tight-tool-heads.log and markdown-scroll.log: exits 0"
      },
      {
        "criterion": "Focused compiled/native/formal evidence and fresh report for current verify revision",
        "reference": "Reused .bboxy/runs/codex-acp-controls.json sha256:25a795b1a2e5d88e9008a4c935ea23039c8911f46f92fc3ecf7f5c13ec39ab5c; all 19 actual log hashes and 68 production hashes rechecked read-only; report sha256:9d34d8a6670f6d0d30cee0de4e89353578259a2945174b06f054cae5f89b86fb"
      }
    ]
  }
}
```


## Report from seat:01M6KF1JD34QA47CQP34FRJN5Z

Metadata-only correction: all five verify_result.evidence criterion fields are copied verbatim from the current task frontmatter.acceptance. Reuses the verified existing indexed rendering report, make verify-acp-controls exit 0, 19 actual log hashes and 68 unchanged production hashes. No gate, code, build, formal run, new report document, integration or backend action this turn; subsequent checkpoints remain preserved. The newly observed MCP-card crash/noisy headers are reserved for a separate task per Oracle and are not claimed resolved here.

```json
{
  "op_id": "01M6KKND52RKNX1PQE3DJGDC9D",
  "commit": "133d0cc53ca610346e72a2a9cf293a0f1df25607",
  "report": "[[reports/2026-11-04-acp-controls-rendering-followup.md]]",
  "verify_result": {
    "command": "make verify-acp-controls",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Actual installed-path catalogue discovery is reproduced and fixed; safe standalone/Unix catalogues agree without credentials",
        "reference": "Retained target/acp-controls/process-and-native-controls.log: 11 passed; path-unit.log: 1 passed; src/frontends/gui/bridge.rs and window.rs production hashes unchanged. Evidence reused from report 01M6KJKX4CVY6TPFHV939A12T7."
      },
      {
        "criterion": "The GUI can display and switch the underlying ACP agent's advertised model and effort choices through the supported ACP API",
        "reference": "Retained process-and-native-controls.log covers actual advertised model/effort requests, dynamic choices and native GUI actions; src/protocol/agent_config.rs and src/backend/acp_agent.rs unchanged. Existing indexed rendering report 01M6KJKX4CVY6TPFHV939A12T7."
      },
      {
        "criterion": "Confirmed values, rejections and unavailable capabilities remain honest; stale session/profile/config changes cannot cross ownership or create a prompt/tool effect",
        "reference": "Retained existing-acp.log: 20 ACP and 7 launcher-context checks; bridge.log: 10 checks; process/native controls cover refusal, unknown outcomes, session/epoch/child/revision fences and no extra prompt/tool effects. Verified production source hashes retained."
      },
      {
        "criterion": "Previous shipped queued-history raster, MCP output, loader and launcher-context fixes survive integration with harness main",
        "reference": "Retained preserved-gui.log: two centered welcome-loader/motion checks and one MCP output check; pending-raster.log: prior-history GPU pixels; launcher-context tests: 7 passed. These are the bounded existing witnesses, not a claim to resolve the separately reported MCP-card crash/noisy headers."
      },
      {
        "criterion": "Focused affected compile, native raster/process/config tests and selected stateful controls pass; signed source and committed indexed report retained",
        "reference": "Existing make verify-acp-controls exit 0; .bboxy/runs/codex-acp-controls.json sha256:25a795b1a2e5d88e9008a4c935ea23039c8911f46f92fc3ecf7f5c13ec39ab5c; 19 actual log hashes and 68 production hashes previously checked; compile/lint/schema/native checks and six selected native/Java graph/verdict comparisons retained. Signed source a832bd6 and indexed report commit 133d0cc; report sha256:9d34d8a6670f6d0d30cee0de4e89353578259a2945174b06f054cae5f89b86fb."
      }
    ]
  }
}
```
