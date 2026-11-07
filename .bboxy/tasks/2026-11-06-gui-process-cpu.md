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
status: claimed
id: 01M6S4G4371KJKEN71AN5Y0AAQ
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-07T12:21:15.523Z
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
provenance:
  digest: sha256:eb1dbd11b7009009f1114a632669f9afdf52ccb81ddbfb7ab47f85d7fde9c9dc
  frontmatter_digest: sha256:ae755bfa57e494f8fa1851115226a450b7c04d6f534138abbd83c9a46ff9f2f0
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: vCfVBHcx+aTWIhBvhuBhk8G97HLjlQmgLqyPQ848p54z2awXo72FE34Fs2qW4F3uF2RoHBKN3XdoOIvLmrJUBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150311
  event:
    id: 01M6TZ9D7PSXSY4YHGMVQMQMQG
    sequence: 150311
    hash: 0fa9df241f9cced2abed5940cb777b45d95c888d333e40e9c20dc5d8fa6b2c3d
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
