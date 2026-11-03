---
type: task
title: Restore standalone harness launch and integrate verified face/status work
status: done
role: executor
assignee: seat:01M6HGY3V1KRZQVGFP17JVJ7JM
delegated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
swarm: 01M649102FBS60QWNPZS5VAGF9
worktree: /Users/feral/mydev/bboxy450-harness-codex-recovery
allowed_paths:
- src/**
- tests/**
- verification/**
- scripts/**
- packaging/**
- Cargo.toml
- Cargo.lock
- Makefile
- ATTRIBUTION.md
- README.md
- .bboxy/reports/**
- .bboxy/runs/codex-harness-recovery.json
- .gitignore
- protocol.schema.json
- .bboxy/runs/quiet.json
- .bboxy/runs/status-harness.json
verify: RUSTC_WRAPPER= make test
parent: 01M6HCH1SJHDSB6SDAC2ZF1BXX
id: 01M6HCYD1S81VTDS3V9M4Y8H4K
updated: 2026-11-03T20:20:56.102Z
created: 2026-11-03T19:07:31.479Z
number: 15251
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
artifacts:
- '[[01M6HGECAJHQRMMVQ675J9E2JZ]]'
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: claimed
  disposition: implemented
  reason: Original assignee completed and registered a signed report before close grace elapsed, but its delayed receipt arrived after responsibility was reassigned. Resuming the native thread retained the old closed seat MCP identity; do not impersonate or repeat work. Reconcile that completed historical handoff using actual source, canonical run, current formal receipt and successful Oracle landing/install evidence. This is historical disposition, not a fabricated new-assignee task_report or fresh task_accept.
  evidence:
  - Original task_report op 01M6HGWRQ3CPMH7CF50CMK3HS5 recorded af65c6cf82d9dde6314b1771b9314061b0e24212 and indexed committed report [[reports/2026-11-03-codex-harness-recovery]] (#16395).
  - git diff ea75307..af65c6c changes only report/summary; clean worktree; seven Boxy-signed integration/report commits verified. GitHub main fast-forwarded ea75307 then af65c6c with actual git push exit 0.
  - 'target/recovery/full-result.json and full-generated/test-current.json: RUSTC_WRAPPER= make test exit 0 on ea75307, 20 core commands and four GUI commands zero; Oracle independently matched all 270 canonical input hashes at af65c6c.'
  - 'Oracle make check-formal-evidence exit 0 at clean af65c6c: 99 model/control variants, 34 contracts, 1 crate; native/TLC graph+verdict parity receipt sha256 00a083344fa7b5b33e91c99572fe5125c7f9fef7670bc095a408d321b525fded.'
  - Startup missing-bitmap mutation native exit 1, restored 0; quiet/status controls reject then restore. Seven same-lab native PNGs byte-identical. No baseline adoption, visible presentation or live external adapter PASS claimed.
  - Oracle make install ea75307 exit 0 (630.184s). Installed bundle strict/deep signature valid, approved Apple team UQB3368A84, plist source ea75307. Existing smoke-app.py against installed bundle exit 0 with private HOME, hidden native window/surface/live-frame receipt and no operator input.
  by: seat:01M6H97SAX3GN1BYB4YG76P3P0
  op_id: 01M6HH4T8JRMQ9NG6KESV96XDS
provenance:
  digest: sha256:ff2859b4f24c4ffc564822605c8b61bfa1d256e6f2aed7f0e9fa6d3958db9daf
  frontmatter_digest: sha256:94480e11f5b163a0db2dbf3f31b7f468e2dcf8a3ec67e83abd617a6b8d549406
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: jsfy5SnmzwFtC+Z25VJ/CfE8U1BUBWiK3qynM+pvq+K9XL6U7aP1mhynskC/RPzXKUzo0ODZ7X8aNjZwlzM3AQ==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#98023
---
ROLE: EXECUTOR. Work ONLY in /Users/feral/mydev/bboxy450-harness-codex-recovery, branch oracle/codex-recovery, created from origin/main 0c40316. Read AGENTS.md, ATTRIBUTION.md, .bboxy/README.md, current specs/ICDs and Make help in this worktree. Operator: standalone bboxy450-harness.app does not start; the embedded term pane shows only a terminal prompt. The intended face is the standalone native HarnessView, shared with term. Restore standalone launching and preserve that shared face.

First inspect the installed bundle at /Users/feral/Applications/bboxy450-harness.app READ-ONLY (launcher/plist, source revision, signature, bounded crash/launch evidence) and reproduce in an OWN private bundle/home through canonical Make targets. Do not launch on or capture the operator desktop. Make a process witness for the actual startup failure; do not claim a headless screenshot alone proves a visible app launches. Fix the root cause in source/packaging and its unattended smoke/negative control.

Recover completed unlanded branches, reading their real diffs and reports before reuse: quiet 3058e12cede120a2e6986bb7cbf238a2038ca397; capture-chrome df12668c743c18dbfc3af283bdced380f9bb9dcb; status heartbeat 95fbb72f675ffe163443fd5fb89fcf02068c8037; agent turns 5ccbe48b023b8e3091f9a8f570c646223418bc53. Reports are /Users/feral/mydev/bboxy450-harness-parity/reports/{quiet,status-harness,agent-turns}.md and capture's actual commit/run evidence. Keep original source branches unchanged and reachable. Incorporate complete branch changes in this integration branch, resolve overlapping formal inventories/tests without dropping either contract, and make a signed final candidate. Do not repeat known stale reports as current evidence.

Keep Native/Usage invisible with no height/padding/receipt or grouping effect, preserve tight transcript/tool rails/chrome/zoom, open-turn phase heartbeats, and IR 1.2 agent turns. Additive IR only; use make lock-ir when needed, never weaken compatibility or frozen corpus. Formal lifecycle changes require current models, broken controls, native/TLC graph+verdict parity. Compare candidate and main against the SAME lab at /Users/feral/mydev/bboxy450-term-harness-ref/lab/web; inspect real native captures. Do not adopt baselines: the required two native visual passes are not yet complete.

Coordinate with rich-socket and rich-host workers; do not overwrite their worktrees or independently redesign the ICD. No push, merge, publication, trusted installation, credentials or OS input. Use Make; private HOME/TMPDIR for tests; two Cargo jobs and a worktree-local target. Full canonical RUSTC_WRAPPER= make test, smoke-app, transcript/IR checks and current-source formal evidence must be green. One full machine gate at a time; self-serve with process evidence, continue useful work while another runs; no operator required.

Stage exact paths and sign commits. Full report .bboxy/reports/2026-11-03-codex-harness-recovery.md, Report trailer, actual commands/exits, startup reproduction/fix, branch lineage, visual evidence, limits and skipped live adapters. Register report through knowledge tools then task_report.

Oracle scope clarification: the exact auxiliary files `.gitignore`, `protocol.schema.json`, `.bboxy/runs/quiet.json`, `.bboxy/runs/status-harness.json` are authorized for complete branch integration, as verified from the four original branch diffs. Preserve inherited compact historical summaries as historical evidence; write fresh integration evidence separately. Do not commit unrelated regenerated formal/GUI run artifacts. The verify command is unchanged.

## Report from seat:01M6HCYT7FYN90XD2C4TXGZQZC

Signed final candidate af65c6cf82d9dde6314b1771b9314061b0e24212 fixes the standalone first-frame `unregistered bitmap` failure by registering the shared title-bar portrait, and integrates quiet Native/Usage projection, shared chrome/capture/zoom, p7 phase heartbeats and IR 1.2 agent turns. Full RUSTC_WRAPPER= make test exited 0 on ea75307; every canonical input hash is unchanged in the report-only af65c6c commit. Final-head native smoke, 99-model/34-contract native–TLC graph+verdict parity, formal evidence and secrets checks all exit 0; startup, quiet and status mutation controls reject the broken paths and restore green. Seven stock native captures are byte-identical against the same lab; Native-record noise is removed without baseline adoption. The hidden native window/surface/frame witness does not claim visible presentation; live adapters are SKIP. Original branch refs and installed bundle hashes remain unchanged; worktree is clean, all seven commits are signed. Observed origin/main is now ea75307; this candidate adds only the committed report/summary. No push, merge, publication or trusted installation was performed by this Executor.

```json
{
  "op_id": "01M6HGWRQ3CPMH7CF50CMK3HS5",
  "commit": "af65c6cf82d9dde6314b1771b9314061b0e24212",
  "report": "[[reports/2026-11-03-codex-harness-recovery.md]]",
  "verify_result": {
    "command": "RUSTC_WRAPPER= make test",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Standalone startup root cause and unattended native witness",
        "reference": "src/frontends/gui/assets.rs:29; scripts/test-startup-control.py:46; target/startup-control/evidence.json — missing bitmap native exit 1, restored 0; target/recovery/final-smoke.log — final-head smoke exit 0, real hidden NSWindow/Metal surface/live frame."
      },
      {
        "criterion": "Complete branch recovery and source provenance",
        "reference": ".bboxy/runs/codex-harness-recovery.json; target/recovery/source-branches.json; target/recovery/final-handoff.json — all four original refs unchanged, all seven commit signatures verify, installed launcher/plist/binary hashes unchanged, clean tree."
      },
      {
        "criterion": "Native/Usage suppression and shared face preservation",
        "reference": "src/frontends/gui/face_rows.rs:47; tests/gui_render.rs:246; tests/seat_golden.rs:15; target/quiet/controls/results.json — GUI/seat/rail controls 101→0; target/recovery/visual-comparison.json — seven native PNGs byte-identical, lab 7953a5e7e12a4c618b52358eed1d831efe286b15c3022309439e7811ac56d4f1; Native16 mean .0521→.0070, rows .4145→.0300."
      },
      {
        "criterion": "Open-turn phase heartbeat contract",
        "reference": "src/frontends/seat.rs:25; target/recovery/contracts.log and status-controls.log — immediate phase, real PTY heartbeat and agent-origin checks green; missing-heartbeat/stale-phase/child-phase mutations 101→0."
      },
      {
        "criterion": "Additive IR 1.2 agent turns",
        "reference": "src/protocol/mod.rs:190; src/backend/acp_agent.rs:620; tests/acp_agent.rs:868; tests/ir_v1_corpus.rs:262 — idle work/permission, policy, Quiet and queued-user witnesses pass; target/recovery/ir.log and full canonical IR checks exit 0; frozen corpus preserved."
      },
      {
        "criterion": "Canonical unattended verification and report-only final commit",
        "reference": "target/recovery/full-result.json — RUSTC_WRAPPER= make test exit 0 on ea753072c7ec2a0a278e5086ef21458efa0a08f0; full-generated/test-current.json and gui-gate.json — 20 core and 4 GUI commands all 0; post-results.json all 0; final-results.json all 0 and unchanged canonical input hashes."
      },
      {
        "criterion": "Current-source bounded formal evidence",
        "reference": "verification/evidence/00a083344fa7b5b33e91c99572fe5125c7f9fef7670bc095a408d321b525fded/evidence.json — source af65c6cf82d9dde6314b1771b9314061b0e24212, dirty=false, 99 variants / 34 contracts / 1 crate, exact native/TLC graphs and registered verdicts agree; final-formal-evidence.log exit 0."
      },
      {
        "criterion": "Committed indexed report and documented limits",
        "reference": ".bboxy/reports/2026-11-03-codex-harness-recovery.md — report 01M6HG6TRW5DRVW79KAZF3A942 / #16395, Report trailer in af65c6c; artifact claim 01M6HGECAJHQRMMVQ675J9E2JZ. No baseline adoption, visible-presentation or live-adapter PASS claim; rich host/socket worktrees and ICD unchanged."
      }
    ]
  }
}
```
