---
type: task
title: GUI process burns ~71% CPU in steady state; logic holds 1.2 GiB RSS
role: executor
allowed_paths:
- crates/bboxy450-term/src/
- crates/bbt-ui/src/
- crates/bbt-render/src/
- crates/bboxy450-term/tests/
- crates/bbt-ui/tests/
- verification/
- .bboxy/reports/
- Makefile
- .bboxy/specs/attention.md
- .bboxy/specs/rim-glow.md
- scripts/qos.sh
- scripts/qos-policy.py
- LAND.md
- .bboxy/tasks/2026-11-06-gui-process-cpu.md
- crates/bbt-doc/tests/corpus/correction_fixtures.rs
verify: cargo nextest run -p bboxy450-term -p bboxy450-ui -E 'test(/idle|frame|redraw/)' && make test-formal-contracts
status: accepted
id: 01M6S4G4371KJKEN71AN5Y0AAQ
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-07T13:09:41.220Z
created: 2026-11-06T19:13:50.090Z
number: 93377961366091
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M6TQ312P1ZD2JPS1NQS3Y8JD
verify_revision: 1
verify_result:
  command: cargo nextest run -p bboxy450-term -p bboxy450-ui -E 'test(/idle|frame|redraw/)' && make test-formal-contracts
  exit_code: 0
  evidence:
  - criterion: Profile six private seats without touching the operator app
    reference: /tmp/bboxy-idle-cpu-0dtepapd/gui.sample.txt:55 — unchanged-frame App::redraw/solve_ui/layout work; /tmp/bboxy-idle-cpu-wp5oombf/gui.sample.txt:59 — retained draw_attention. All six sample/vmmap status records per fixture exit 0.
  - criterion: Quiet <=5%, visible attention <=10% and <=30 fps, occlusion suspends pulse
    reference: /tmp/bboxy-idle-cpu-og65p_im/report.json — 0.0995%/0 frames; /tmp/bboxy-idle-cpu-rbkzfngf/report.json — 5.1739%, 24.9739 fps, 251 pulse/0 full; /tmp/bboxy-idle-cpu-wp676_sc/report.json — actual occlusion, 0%/0 frames.
  - criterion: Redraw coalescing, retained rim damage and genuine broken controls
    reference: main.rs:5560/8167, gpu.rs:555/1519/2549/2598; /tmp/gui-cpu-native-rebased.log — 2 tests/7 scenarios exit 0; /tmp/bboxy-idle-cpu-vlhci48m, 0ifcw_9_ and 7mz_i_st/report.json — continuous/full-scene controls produce real frames and fail the budget.
  - criterion: Logic RSS bounds/leak investigation with per-structure breakdown
    reference: reports/2026-11-06-gui-process-cpu.md — structure/ownership table and invariant coverage limits; /tmp/bboxy-idle-cpu-wp5oombf/logic-vmmap-{0,1}.txt — stable 23.3M footprint and 5932K allocated; original production 1.2 GiB attribution remains unproven.
  - criterion: Required verification and mandatory formal evidence
    reference: /tmp/gui-cpu-verify-final.log — 49 selected tests, 44 contracts, Clippy, 18 QoS tests/probes exit 0; /tmp/gui-cpu-land-final.log — 460 app tests and 681 models/34 crates/255 contracts exit 0; /tmp/gui-cpu-evidence-check.log — clean source 8ced58cb2ed3e710d0351ce114469c3b8d6296ea, receipt f5e81d4f88bd322d6a79a68184b9c7b719b40a65950f0b75cbf69163518ab7be.
  - criterion: Signed lean landing, committed report and exact KB export
    reference: /tmp/gui-cpu-land-final.log — LANDED 8ced58cb2ed3e710d0351ce114469c3b8d6296ea, exit 0; both commits verify key 9B07E849480C68328CA20684844ECC0587DE7AB0; HEAD == origin/main, clean git status; report SHA256 229c3a8572d7ee91ca79ca715b51fdeb2e92fea7fe1c04510e7ee57057a99139 matches committed KB checkpoint 45c607204b9fd7736ee43f1a307b0a1f1e122c31.
  - criterion: Artifact responsibility workflow limitation
    reference: knowledge_fetch reports publication_state committed and checkpoint 01M6V0T4068W85A45XCK4SB3NF; provenance_get records produced_by this seat. artifact_claim operations 01M6V0TC3PHX8GWRN40HYCF691 and 01M6V0V4BSYZ35FP1CNMRA7SMS both refused artifact_not_committed; no bypass/retry after the second refusal.
report:
  summary: 'Landed signed commits 8d7c331a05e8a21388a88e9de54a4a659ac18c8e (implementation) and 8ced58cb2ed3e710d0351ce114469c3b8d6296ea (exact committed KB report export); HEAD equals origin/main and the worktree is clean. Six-seat native measurements: quiet 0.10% core/0 frames, visible waiting 5.17%/24.97 retained fps/0 full frames, occluded 0%/0 frames; all three real redraw controls refute their budgets. Lean landing, 460 app tests, corpus checks, affected-dependent checks, licenses, explicit native gate and 681-model native/TLC parity pass; final formal receipt f5e81d4f88bd322d6a79a68184b9c7b719b40a65950f0b75cbf69163518ab7be verifies clean current HEAD. Report includes private sample/vmmap evidence and the per-structure RSS inventory: fresh logic allocation plateau is stable, aggregate workspace indexes lack eviction bounds, and the original 1.2 GiB remains unattributed. No reinstall. Workflow gap: artifact_claim returned artifact_not_committed twice for the committed KB checkpoint; no explicit claim was recorded.'
  commit: 8ced58cb2ed3e710d0351ce114469c3b8d6296ea
  reported_by: seat:01M6TQ312P1ZD2JPS1NQS3Y8JD
  op_id: 01M6V21F204QX875YT931C99YS
  verify_revision: 1
  report: '[[reports/2026-11-06-gui-process-cpu.md]]'
provenance:
  digest: sha256:90f531c0572b7a83e6285a81deca67fc72f4a691b52dbd0e81a29d3510634b7b
  frontmatter_digest: sha256:69b8a3256ea81254731e1be94b59d61725a41879fe133d1590076876ecbae642
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: fPFgwccCbx4MjITKMNsHVxPwrHquWWf+b6T9Vw8EfNd/cEdG46UTMT2UQQ6sbu3WWRphO8mwKN0s/JHtdrmEAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150348
  event:
    id: 01M6V222TE2FMPNHMYT9R4QHFZ
    sequence: 150348
    hash: c2d1761c9765aed93c71529e355920e889abfb283e50ff6c4a19484b51c90891
---
# The GUI process is a large CPU consumer on its own

`ps` on 2026-11-06 at 11:13, with the app showing about 6 seats and no user input: `bboxy450-term --app-ready` (pid 39645) at **70.8% CPU**, `logic` (24354) at 27.5% with **1.2 GiB RSS**, and WindowServer at 29%. With 4 P-cores, the UI alone takes most of one, and under build load that cascades into MCP timeouts.

1. **Profile it.** Use a headless or private instance with the same seat count (`sample`, or `--capture` with frame timing), never the operator's live app. Find out what redraws without input: animation ticks, status polling, seat face refresh, markdown re-layout, or repeated work from unchanged frames.
2. **Fix it.** Idle with no input and no output should approach 0% (frame-on-change only); budget idle steady state at ≤5% of one core with 6 seats. Coalesce output-driven redraws to the display refresh.
3. **Logic RSS.** Check whether 1.2 GiB is bounded rings/caches as specced (seats-400-memory-idle invariant) or a leak; report the per-structure breakdown.
4. **Gate it.** Add an idle-CPU and redraw-count test plus a broken control (forced continuous redraw must fail).

Lean landing; no reinstall. Write the report `reports/2026-11-06-gui-process-cpu.md` and call `task_report` once.

**Oracle ruling (pulse vs idle):** KEEP decision d44's behaviour: a waiting seat pulses strongly until handled (attention.md §2, rim-glow.md §9). Do NOT reduce it to one bounded pulse. Make it cheap instead:
1. **Rate cap:** drive the pulse at ≤30 fps (a 0.9 s swell is smooth at that rate), never at the display's full refresh rate.
2. **Damage-limited:** redraw only the waiting rows'/panes' rim rects, not the whole scene.
3. **Suspend when not visible:** no pulse redraws while the window is occluded, minimized, or the app is hidden (NSWindow occlusion state). The desktop notification still fires, so attention is not lost.
4. **Budgets:** idle with no waiting seat ≤5% of one core; idle with waiting seats pulsing while visible ≤10% of one core; occluded ≈0.
Add one line to rim-glow.md and attention.md recording these rendering rules (behaviour unchanged). The verify package is `bboxy450-ui` (not bbt-ui); Makefile granted for the canonical idle-CPU/redraw gate. Broken control: a pulse at full rate or full-scene damage must fail the budget.

**Amend (Oracle):** granted scripts/qos.sh, scripts/qos-policy.py and LAND.md ONLY to register the new test-gui-cpu target and its idle_cpu measurement tests as explicit default-QoS timed witnesses (the parent orchestrator unclamped), keeping background compiler policy. NOTE: the One Test Binary executor is concurrently migrating qos-policy.py to module-path selection; keep your edit minimal and rebase cleanly if it lands first.

**Amend (Oracle):** (1) Export authorized: commit the EXACT committed knowledge-base bytes of your report and of this task record (.bboxy/tasks/2026-11-06-gui-process-cpu.md) into the code tree with your work (project rule: the report travels with the commit); no edits to their content. (2) Granted crates/bbt-doc/tests/corpus/correction_fixtures.rs (~241–243) ONLY to update the stale expected error text from 'author must name a seat' to the validator's current message ('report author must name your authenticated seat:ULID, not a display name'), keeping the same rejection case (an unknown or foreign author is still refused). No validator change.


## Report from seat:01M6TQ312P1ZD2JPS1NQS3Y8JD

Landed signed commits 8d7c331a05e8a21388a88e9de54a4a659ac18c8e (implementation) and 8ced58cb2ed3e710d0351ce114469c3b8d6296ea (exact committed KB report export); HEAD equals origin/main and the worktree is clean. Six-seat native measurements: quiet 0.10% core/0 frames, visible waiting 5.17%/24.97 retained fps/0 full frames, occluded 0%/0 frames; all three real redraw controls refute their budgets. Lean landing, 460 app tests, corpus checks, affected-dependent checks, licenses, explicit native gate and 681-model native/TLC parity pass; final formal receipt f5e81d4f88bd322d6a79a68184b9c7b719b40a65950f0b75cbf69163518ab7be verifies clean current HEAD. Report includes private sample/vmmap evidence and the per-structure RSS inventory: fresh logic allocation plateau is stable, aggregate workspace indexes lack eviction bounds, and the original 1.2 GiB remains unattributed. No reinstall. Workflow gap: artifact_claim returned artifact_not_committed twice for the committed KB checkpoint; no explicit claim was recorded.

```json
{
  "op_id": "01M6V21F204QX875YT931C99YS",
  "commit": "8ced58cb2ed3e710d0351ce114469c3b8d6296ea",
  "report": "[[reports/2026-11-06-gui-process-cpu.md]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term -p bboxy450-ui -E 'test(/idle|frame|redraw/)' && make test-formal-contracts",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Profile six private seats without touching the operator app",
        "reference": "/tmp/bboxy-idle-cpu-0dtepapd/gui.sample.txt:55 — unchanged-frame App::redraw/solve_ui/layout work; /tmp/bboxy-idle-cpu-wp5oombf/gui.sample.txt:59 — retained draw_attention. All six sample/vmmap status records per fixture exit 0."
      },
      {
        "criterion": "Quiet <=5%, visible attention <=10% and <=30 fps, occlusion suspends pulse",
        "reference": "/tmp/bboxy-idle-cpu-og65p_im/report.json — 0.0995%/0 frames; /tmp/bboxy-idle-cpu-rbkzfngf/report.json — 5.1739%, 24.9739 fps, 251 pulse/0 full; /tmp/bboxy-idle-cpu-wp676_sc/report.json — actual occlusion, 0%/0 frames."
      },
      {
        "criterion": "Redraw coalescing, retained rim damage and genuine broken controls",
        "reference": "main.rs:5560/8167, gpu.rs:555/1519/2549/2598; /tmp/gui-cpu-native-rebased.log — 2 tests/7 scenarios exit 0; /tmp/bboxy-idle-cpu-vlhci48m, 0ifcw_9_ and 7mz_i_st/report.json — continuous/full-scene controls produce real frames and fail the budget."
      },
      {
        "criterion": "Logic RSS bounds/leak investigation with per-structure breakdown",
        "reference": "reports/2026-11-06-gui-process-cpu.md — structure/ownership table and invariant coverage limits; /tmp/bboxy-idle-cpu-wp5oombf/logic-vmmap-{0,1}.txt — stable 23.3M footprint and 5932K allocated; original production 1.2 GiB attribution remains unproven."
      },
      {
        "criterion": "Required verification and mandatory formal evidence",
        "reference": "/tmp/gui-cpu-verify-final.log — 49 selected tests, 44 contracts, Clippy, 18 QoS tests/probes exit 0; /tmp/gui-cpu-land-final.log — 460 app tests and 681 models/34 crates/255 contracts exit 0; /tmp/gui-cpu-evidence-check.log — clean source 8ced58cb2ed3e710d0351ce114469c3b8d6296ea, receipt f5e81d4f88bd322d6a79a68184b9c7b719b40a65950f0b75cbf69163518ab7be."
      },
      {
        "criterion": "Signed lean landing, committed report and exact KB export",
        "reference": "/tmp/gui-cpu-land-final.log — LANDED 8ced58cb2ed3e710d0351ce114469c3b8d6296ea, exit 0; both commits verify key 9B07E849480C68328CA20684844ECC0587DE7AB0; HEAD == origin/main, clean git status; report SHA256 229c3a8572d7ee91ca79ca715b51fdeb2e92fea7fe1c04510e7ee57057a99139 matches committed KB checkpoint 45c607204b9fd7736ee43f1a307b0a1f1e122c31."
      },
      {
        "criterion": "Artifact responsibility workflow limitation",
        "reference": "knowledge_fetch reports publication_state committed and checkpoint 01M6V0T4068W85A45XCK4SB3NF; provenance_get records produced_by this seat. artifact_claim operations 01M6V0TC3PHX8GWRN40HYCF691 and 01M6V0V4BSYZ35FP1CNMRA7SMS both refused artifact_not_committed; no bypass/retry after the second refusal."
      }
    ]
  }
}
```
