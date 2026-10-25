---
id: 01M5NB35Z4EDCHAAFEPW7W1AFQ
type: invariant
number: 212
title: "the sidebar and 50 watched panes hold frame time at 967 seats"
status: pending
slow: true
gate: pending-r5d
check: GATE_SEATS=967 GATE_WATCHED=50 GATE_SECONDS=10 GATE_BUDGET=8.33 sh scripts/gates.sh frame
cost: the cockpit hitches under load exactly when the operator needs to read it; a regression in sidebar layout, chrome text or the watched-grid path shows here first
links:
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[2026-10-22-t-terminal-fidelity]]", rel: evidence }
created: 2026-10-23T21:36:26Z
updated: "2026-10-25T09:54:49Z"
---
# The sidebar and 50 watched panes hold frame time at 967 seats

**Property.** 967 real PTY seats (the macOS ceiling of 999 minus 32, decisions
#19 and #20), 50 of them watched as live grids, firehose load for 10 s then 10 s
idle: UI frame work p99 at most **8.33 ms** (provisional: the 120 Hz ceiling the spec names, replaced by median + 25 % of three settled runs), UI CPU p99 under 4 ms, input
within 2 intervals, bus within its limit, and the run settles to IDLE after the
producers stop (C1 §5 gate 4: 8.33 ms at 120 Hz is the ceiling the budget sits
under).

**Check.** `GATE_SEATS=967 GATE_WATCHED=50 GATE_SECONDS=10 GATE_BUDGET=8.33 sh scripts/gates.sh frame`
(`crates/bboxy450-term/src/stall/load.rs:1288`, `frame_work_p99_ms` against
the budget; `:1276` the fixed 4 ms CPU limit) — exit 0 held, non-zero above
budget or unsettled. Slow (a release build plus about 20 s and a window
capture): runs under the local CI's slow flag in a gate slot, never on a
handoff.

**Cost of a violation.** The cockpit hitches under load exactly when the
operator needs to read it; a regression in sidebar layout, chrome text or the
watched-grid path shows here first.

**Evidence.** The first settled runs (Rook, 2026-10-24 and 2026-10-25, after the stale display tick and the queued-command re-arm were removed at 8240d359 and 2ad1d733): the settle guard passes with idle at 0 frames and 0 callbacks, and the frame gate fails on its own predicates, CPU p99 518.7 ms and input-to-GPU 8 618 ms against the unchanged targets. The measured run on 0e97dc32 with Nyx's R5i diagnostics names both producers: on the UI side 4 321 scene projections for 42 encoded frames (Rook's fix: project only for a frame that will be encoded); on the logic side 6 824 serial one-seat `ReadRing` RPCs, 93 % of the input-to-GPU, gate [[ring-reads-batched]]. The budget stays 8.33 ms: a measured miss is the gate working, not a reason to move the target. Window 3 (Aster, 691ad3c, 2026-10-23): exit 1 on the settle guard after 100.7 s, no report; all 50 parsed cursors lag the ring tail, logic about 1 100 events behind, cause unproven. Blocked on logic ingestion, not the renderer.
Window 2 (`.bboxy/runs/r5-window2-b450a99.log`) ended
`r5-unsettled-after-producers-stopped` with no report; R5d (865443f) added the
per-seat settle diagnostics that name why. Budget = median of three settled
runs + 25 %, written here when the numbers exist.

Owner: Corvin Duskmantle <ARCHITECT>.
