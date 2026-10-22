# T — terminal speed and fidelity under real harness output (epic)

Owner: Vesper Greywind <ORACLE>. Operator, 2026-10-22: "a binary that mimics heavy output, embedded into PTY viewers: it prints random Codex logs in a loop from memory, testing rendering speed and PTY fidelity, making sure we render both text and terminal output sexy, with antialiasing going in our renderer." Part of the visual foundation ([[vision]]) and the load for R5 ([[runtime]] §6).

## T1 — `bboxy-firehose`: a heavy-output PTY fixture binary

Assignee: Rook, after W1.2c, before W2.4. Allowed: new `crates/bboxy-firehose/**`, root `Cargo.toml`, `Cargo.lock`, `.bboxy/runs/t1-*.log`.
- One small binary. All content is embedded in memory (`include_bytes!`), and nothing is read at runtime.
- **Corpus:** synthetic, never real transcripts (no private content). It is generated to look and behave like Claude Code and Codex sessions: SGR colours (16, 256 and truecolor), bold, italic, dim, underline styles, reverse; unicode, CJK wide characters, emoji with ZWJ sequences, combining marks; box drawing and powerline; diffs, tool-call blocks, spinners and progress bars redrawn with CR and cursor moves, the alternate screen, scroll regions, OSC 8 links, OSC 7501 status, and synchronized output (mode 2026). The existing captures `.bboxy/runs/osc7501-*.bin` are included as corpus entries.
- **Modes:**
  - `--rate` bytes/s or lines/s, or `max`;
  - `--pace real` (human-like bursts and pauses) or `flood`;
  - `--loop`, `--seed` (deterministic), `--scene` to pick one corpus entry, `--list`.
- `--check`: prints the expected final screen hash for a seed and scene, so fidelity tests can compare.
Verify: `cargo test -p bboxy-firehose`; running it in iTerm2 for one each of the 10 scenes looks right (screenshots in the run log); `--rate max` sustains at least 200 MiB/s to /dev/null.

## T2 — terminal render fidelity in the live app

Assignee: Sable, after C1 live seats (cells need the real path). Seats run `bboxy-firehose` scenes; panes are read as images at 1x and 2x, both themes. Fix at the layer: grayscale AA with the linear-corrected blending terminal text keeps ([[renderer]] §1a), box drawing and powerline as procedural sprites, wide and emoji cells, underline styles, cursor shapes, selection. Reference: the same seed and scene in iTerm2 and Ghostty, screenshotted side by side for the image read, plus the `--check` screen hash against our grid. Speed: 15 visible seats with `--pace real` and 1000 seats with `flood` under the hitch meter, which is R5's load.
