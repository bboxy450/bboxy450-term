---
id: "035MBM4HSJBWX8J2QT78JGNS9J"
type: "task"
number: 119
title: "W3.2c — bboxybook: shared fixtures, then input/focus, launcher, signals→fx"
created: "2026-10-22T00:33:02Z"
updated: "2026-10-24T00:10:38Z"
---
# W3.2c — bboxybook: shared fixtures, then input/focus, launcher, signals→fx

Owner: Vesper Greywind <ORACLE>. Assignee: Nyx Copperwing <EXECUTOR>. Status: ASSIGNED. Spec: [[gui-components]] §7 steps 8–10, [[renderer]] §4.5. Follows W3.2b (checkpoint ef6c7ff).

Allowed: `crates/bboxybook/**`, `crates/bbt-ui-model/**`, `crates/bbt-render/src/chrome/**`, `scripts/parity.sh`, `Cargo.lock`, `.bboxy/runs/w3-2c-*.log`.

0. **Fixture alignment first.** The W3.2b parity index compares different data: the web "10 seats" scene (term 3 / winnow 6 / ontology 1, with a stage pane) against the native showcase fixture (term 5 with shadows, no stage). That mismatch, not rendering, is most of the 93% changed-pixel scores. Wren moved the lab's demo data to `lab/web/fixtures.js`; bboxybook loads that file (parse the JS object literal, or have the lab emit `fixtures.json` from it) so web and native render identical models per scene and state. Re-run `scripts/parity.sh`; report the new over-tolerance count and the ten worst triplets by component. Expect the remaining diffs to be real: font weight and metrics, badge styling, stage/pane absence (blank bodies are allowed; the frame is not).
1. Input and focus (§7 step 8): hit test, click vs drag threshold, dwell timer, Tab, divider and sidebar resize, drag-to-split, the shortcuts table; synthetic-event tests.
2. Launcher and overlays (§7 step 9) from `launcherItems()` in the lab, the four lexical tiers, presets.
3. Signals → fx (§7 step 10): `FxFrame` from the model, ripple queue, bounce, pulse; tests that activity never ripples and a click ripples once.
4. Parity re-run after each step; the index is what the Oracle reviews.

Verify: `cargo test -p bbt-ui-model -p bboxybook --locked` green; `scripts/parity.sh` writes the index. Report by DM with the counts and the ten worst, one line on the channel.
