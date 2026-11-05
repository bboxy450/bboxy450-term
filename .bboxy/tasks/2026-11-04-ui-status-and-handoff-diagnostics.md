---
type: task
title: Keep harness status titles concise and diagnose GUI-only compatibility refusal
role: executor
parent: 01M6KEVVYNZW5H21RPTFW1RVDR
worktree: /Users/feral/mydev/bboxy450-term-ui-status
allowed_paths:
- crates/bboxy450-harness-wake-up/src/observe.rs
- crates/bboxy450-harness-wake-up/src/harness_log.rs
- crates/bbt-ui/src/modules/strip.rs
- crates/bbt-ui/src/modules/seat_row.rs
- crates/bboxy450-term/src/scene.rs
- crates/bboxy450-term/src/cockpit.rs
- crates/bboxy450-term/src/cockpit/gui_handoff.rs
- crates/bboxy450-term/src/cockpit/shell_path.rs
- crates/bboxy450-term/tests/**
- crates/bboxy450-harness-wake-up/tests/**
- Makefile
- .bboxy/reports/**
- .bboxy/runs/codex-ui-status.json
verify: make verify-ui-status
acceptance:
- Thinking/status presentation remains clean; multiline raw commands cannot escape header/sidebar geometry
- Raw command details remain in tool content, while pane/status titles are concise bounded single-line labels
- Read-only preserve-backend refusal distinguishes schema/version/PATH mismatches without leaking secrets or weakening guards
- Focused geometry/parser/readonly diagnostic tests and affected compilation pass; no backend restart or operator input
id: 01M6KGR1300A1GX09F02F0WX1F
status: accepted
delegated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T15:18:39.802Z
created: 2026-11-04T14:52:27.762Z
number: 93377961359734
created_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
assignee: seat:01M6KGM9DJ6TH7222SFTQM2D0Y
verify_result:
  command: make verify-ui-status
  exit_code: 0
  evidence:
  - criterion: Thinking/status presentation remains clean; multiline raw commands cannot escape header/sidebar geometry
    reference: crates/bboxy450-term/src/scene.rs:2613; native layout and existing view_switch_header_geometry_and_hits passed
  - criterion: Raw command details remain in tool content, while pane/status titles are concise bounded single-line labels
    reference: crates/bboxy450-harness-wake-up/tests/harness_status.rs:196; crates/bbt-ui/src/modules/seat_row.rs:14; crates/bbt-ui/src/modules/strip.rs:55
  - criterion: Read-only preserve-backend refusal distinguishes schema/version/PATH mismatches without leaking secrets or weakening guards
    reference: crates/bboxy450-term/src/cockpit/gui_handoff.rs:317; crates/bboxy450-term/src/cockpit/gui_handoff.rs:372; .bboxy/runs/codex-ui-status.json production_probe
  - criterion: Focused geometry/parser/readonly diagnostic tests and affected compilation pass; no backend restart or operator input
    reference: make verify-ui-status exit 0; /tmp/codex-ui-status-verify-passed.log sha256:842cfbc72b8bcb2ad2dbe9956ba567b8c99816e52501ac91524ea6d42087d6c2; crates/bboxy450-term/tests/ui_status_probe.rs:15
report:
  summary: 'Implemented bounded single-line harness operation captions and clipped sidebar/strip text while retaining Thinking, raw tool details and action geometry; added allowlisted expected/observed compatibility diagnostics without changing preservation guards. Signed source 727ee898740e31f38d59461c4f157547b1495cd4 and exact indexed report 8743dae8c1e6c977d32e0ca538526cbb423d48b6; both signatures verified, worktree clean. make verify-ui-status exit 0 (14 harness + 5 app tests and affected strict compile); make test-licenses exit 0. Bounded public production probe intentionally refused with exit 1: PATH fingerprint expected 78c1d194ec384ba9, observed d74860b838acf725; all five versions match. No refresh/restart/detach/install/integration performed. Indexed report #93377961360452 carries evidence; compact receipt is .bboxy/runs/codex-ui-status.json. Artifact responsibility claim was terminally rejected as held/artifact record projection pending (operation 01M6KHYG08RGYMKW7H6QBFT0EP, confirmed by operation_status); no claim was assumed or retried.'
  commit: 8743dae8c1e6c977d32e0ca538526cbb423d48b6
  reported_by: seat:01M6KGM9DJ6TH7222SFTQM2D0Y
  op_id: 01M6KHZZ2YP8PZKSAYDGQ6GTH6
  verify_revision: 0
  report: '[[reports/2026-11-04-ui-status-delivery.md]]'
provenance:
  digest: sha256:488b92d3e77f3160d74d786f005ab3ab6f915832f4559847116185d51af71538
  frontmatter_digest: sha256:fa89d31a5ddff97b2a2bfddbd2569b8969530f59768b81628c3f23e1f3ffc26f
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: KG2HOzcqTp/5Se9ubl+cThjBUqN/k7lG1su1OCrhdNStCpYC3AjcYzS8Fzzfy9nH9E3ROksOrloBV5gGinIwBg==
  log: .bboxy/log/2026-11-04.jsonl#119475
---
ROLE: EXECUTOR. Real bboxy450-harness/Codex ACP worker seat01M6KGM9DJ6TH7222SFTQM2D0Y. First orient and read this signed task. Own worktree /Users/feral/mydev/bboxy450-term-ui-status, branch codex-ui-status-20261104, basecadf6700cbdb3d3575b9fc0dad7567cb5c96f0c9 (all current term main and Oracle-owned commits, harness pind0e0c9a). Oracle owns ALL Git main updates/integration; do not fetch/pull/rebase/merge/cherry-pick, update pins, publish or install. Implement only the bounded source fixes and signed commits. Another producer worker owns GUI tool-card/profile/ACP model-effort code; no overlapping producer changes here.

1. User screenshots show full multiline Python/heredoc command source leaking into the harness pane title and sidebar seat row. The concise 'acp:codex-acp · Thinking' state looks right and must stay. Trace the actual title/detail projection and fix at the display boundary with defensive single-line/width/height limits. Use the existing operation/kind or short phase label for a running tool, never an entire command/script as a window title. Keep full raw command/output accessible in the existing tool details. Preserve role/name/cwd, status freshness, activity and generation ownership, keyboard/hit targets and tooltip bounds; ordinary terminal titles should retain sensible semantics. Scope paths above cover observer and existing strip/seat-row views; no new UI framework, persistence or backend protocol change. Add native geometry/parser tests with multiline heredocs, very long words, Unicode and narrow/wide panes. No real desktop input.

2. Parent cannot currently activate a GUI-only update because the existing public read-only flag --gui-handoff-probe --preserve-backend-check refuses 'incompatible Logic schema or launch PATH; refresh forbidden'. Installed/running termca7cde96 PID49372, actual repository /Users/feral/Library/Application Support/bboxy450-term/workspace; default production run directory under Application Support/bboxy450-term/run. The probe is safe and performs non-claiming compatibility checks; do not weaken it, skip PATH checks, refresh/restart/kill owners, detach the real GUI or access live capability files/credentials. Both normal and standard OS/Homebrew inherited PATH probes refused. Improve this existing readonly diagnostic to distinguish exact expected/observed protocol/control/workspace versions versus PATH fingerprint mismatch, exposing ONLY public PID/births/versions/fingerprints or safe reason codes. Keep every guard refusal and identity check intact. Use private fixtures to prove diagnostics do not include token/capability/secret values or admit a cold start. If your new guarded public probe can safely identify the current production refusal without effects, run it bounded and report the precise nonsecret mismatch; otherwise report the exact missing observation. Root will decide the compatible launch environment/activation. No guessing or operator restart requirement.

Use canonical Make target verify-ui-status for selected meaningful tests plus affected strict compile only, two Cargo jobs, own target. No entire test/parity/formal/runtime corpus. Pure formatting/geometry and readonly diagnostics do not need a new whole-model campaign. Preserve all existing production handoff and stale-owner guards. Approved signed Boxy commits, exact staging; no broad add/reset/stash/OS automation/other worktrees. One concise blocker only when all work stops; otherwise one indexed append-only report reports/2026-11-04-ui-status-delivery.md, exact indexed bytes in own report commit with Report trailer, compact run JSON and task_report. Parent performs all integration and GUI-only installation while preserving the other swarm's live backend/sessions.

## Report from seat:01M6KGM9DJ6TH7222SFTQM2D0Y

Implemented bounded single-line harness operation captions and clipped sidebar/strip text while retaining Thinking, raw tool details and action geometry; added allowlisted expected/observed compatibility diagnostics without changing preservation guards. Signed source 727ee898740e31f38d59461c4f157547b1495cd4 and exact indexed report 8743dae8c1e6c977d32e0ca538526cbb423d48b6; both signatures verified, worktree clean. make verify-ui-status exit 0 (14 harness + 5 app tests and affected strict compile); make test-licenses exit 0. Bounded public production probe intentionally refused with exit 1: PATH fingerprint expected 78c1d194ec384ba9, observed d74860b838acf725; all five versions match. No refresh/restart/detach/install/integration performed. Indexed report #93377961360452 carries evidence; compact receipt is .bboxy/runs/codex-ui-status.json. Artifact responsibility claim was terminally rejected as held/artifact record projection pending (operation 01M6KHYG08RGYMKW7H6QBFT0EP, confirmed by operation_status); no claim was assumed or retried.

```json
{
  "op_id": "01M6KHZZ2YP8PZKSAYDGQ6GTH6",
  "commit": "8743dae8c1e6c977d32e0ca538526cbb423d48b6",
  "report": "[[reports/2026-11-04-ui-status-delivery.md]]",
  "verify_result": {
    "command": "make verify-ui-status",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Thinking/status presentation remains clean; multiline raw commands cannot escape header/sidebar geometry",
        "reference": "crates/bboxy450-term/src/scene.rs:2613; native layout and existing view_switch_header_geometry_and_hits passed"
      },
      {
        "criterion": "Raw command details remain in tool content, while pane/status titles are concise bounded single-line labels",
        "reference": "crates/bboxy450-harness-wake-up/tests/harness_status.rs:196; crates/bbt-ui/src/modules/seat_row.rs:14; crates/bbt-ui/src/modules/strip.rs:55"
      },
      {
        "criterion": "Read-only preserve-backend refusal distinguishes schema/version/PATH mismatches without leaking secrets or weakening guards",
        "reference": "crates/bboxy450-term/src/cockpit/gui_handoff.rs:317; crates/bboxy450-term/src/cockpit/gui_handoff.rs:372; .bboxy/runs/codex-ui-status.json production_probe"
      },
      {
        "criterion": "Focused geometry/parser/readonly diagnostic tests and affected compilation pass; no backend restart or operator input",
        "reference": "make verify-ui-status exit 0; /tmp/codex-ui-status-verify-passed.log sha256:842cfbc72b8bcb2ad2dbe9956ba567b8c99816e52501ac91524ea6d42087d6c2; crates/bboxy450-term/tests/ui_status_probe.rs:15"
      }
    ]
  }
}
```
