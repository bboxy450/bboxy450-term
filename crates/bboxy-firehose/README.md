# bboxy-firehose

Ten embedded terminal scenes for PTY, parser and renderer checks. Visible content
is synthetic. The original Claude OSC 7501 status-only captures are embedded
unchanged; all ten original Codex capture files are empty. The Codex protocol
scene labels that gap and emits explicitly synthetic status sequences.

```sh
cargo build --release -p bboxy-firehose
target/release/bboxy-firehose --list
target/release/bboxy-firehose --scene unicode --seed 42
target/release/bboxy-firehose --pace real --seed 42 --loop
target/release/bboxy-firehose --rate 8MiB/s --loop
target/release/bboxy-firehose --rate 30lines/s --loop
target/release/bboxy-firehose --rate max --loop 10000000 > /dev/null
target/release/bboxy-firehose --check --scene palette --seed 42
```

Playback defaults to one flood-speed pass over all ten scenes in seeded order.
`--scene` fixes the bytes independently of the seed; the seed controls scene
order and real-mode burst sizes and pause durations. Real pacing is a deterministic
schedule, subject to OS scheduling. Rate caps use cumulative bytes or LF characters
per second, including control bytes and newlines. `real` and a numeric rate apply
both limits. Flood/max builds a batch of complete cycles once (about 1 MiB) and
writes that batch repeatedly. Memory stays bounded even with an infinite loop.

The binary reads no files, stdin, corpus assets or terminal dimensions at runtime.
Corpus files and hashes use `include_bytes!`. It uses the clock and sleeps for
pacing, and stdout for playback. On Unix, SIGINT and SIGTERM request a stop and
restore primary-screen mode, SGR, scroll margins, links, sync mode and cursor
visibility before exiting with `128 + signal`. SIGKILL cannot restore a terminal.
Broken pipes exit successfully. No renderer, mux, app or UI code is involved.

## Scenes

| Scene | Exercises |
|---|---|
| codex | synthetic exec/tool blocks, code diff, SGR colours, bold and dim |
| claude | synthetic tools, nested agent output, italic, Unicode |
| palette | 16, 256 and RGB colours; bold, italic, dim, inverse, strike; five underline styles and colour |
| unicode | combining marks, CJK wide cells, ZWJ and skin-tone emoji, box drawing, powerline, blocks, braille |
| progress | CR spinners and bars, erase-line, relative cursor movement |
| alternate | tool on alternate screen, restoration of the primary transcript |
| scroll-links-sync | bounded scroll region, OSC 8 link, synchronized frame, ST status sequences |
| osc7501-claude | original `.bboxy/runs/osc7501-claude.bin`, 482 control-only bytes |
| osc7501-preflight | original `.bboxy/runs/osc7501-claude-preflight.bin`, 303 control-only bytes |
| osc7501-codex | ten empty original `osc7501-codex*.bin` entries; labelled synthetic exercise |

`corpus/generate.py` is an author-time script. It writes the embedded `.ansi`
assets from synthetic literals and the original control-only captures; running
the binary never invokes it. It refuses to embed new nonempty Codex captures
without a content review. Tests compare the Claude originals byte for byte.

## Frozen screen hashes

`--check` prints expectations from `corpus/screen-hashes.txt`, without parsing the
scene or emitting ANSI. Each hash describes the final **80 columns × 24 rows**
primary viewport after the scene's prefix and suffix. Larger or smaller grids,
or a different semantic schema, are different comparisons. Hashes do not prove
pixel fidelity, font fallback or correct ZWJ shaping.

Schema v1 is compact UTF-8 JSON, no whitespace or trailing newline:

```text
[columns, rows, cursor_row, cursor_column, terminal_mode_bits, [cell, ...]]
cell = [unicode_scalar, foreground, background, flag_bits,
        [combining_scalars, ...], underline_colour_or_null, hyperlink_or_null]
colour = [0, palette_index] | [1, named_colour_number] | [2, red, green, blue]
hyperlink = [explicit_id, uri]
```

Cells are ordered row-major, including blank and wide-spacer cells. Flags,
named colours and terminal modes use the current `bbt-core` numeric values.
Hash the encoded JSON with 64-bit FNV-1a: offset `cbf29ce484222325`, prime
`100000001b3`, wrapping multiplication. Output is 16 lowercase hex digits.
`tests/corpus.rs:screen` is the executable schema. No implicit hyperlink IDs
are used. These are regression expectations frozen from the current parser;
independent iTerm2/Ghostty image reads remain the visual gate.

```sh
cargo test -p bboxy-firehose
cargo fmt --check -p bboxy-firehose
cargo clippy -p bboxy-firehose --all-targets -- -D warnings
python3 crates/bboxy-firehose/tests/throughput.py
node crates/bboxy-firehose/reference/capture.mjs --strict
```

The corpus test checks every frozen hash with whole-scene, 1, 7, 64 and 4096-byte
PTY chunks, plus specific effects (decorations, wide/combining cells, cursor
rewrites, alt-screen restoration, region boundaries, OSC 8, and balanced sync).
Hash changes require inspecting the changed scene and recording the reason.

The pinned local xterm.js reference and its capture instructions are in
`reference/README.md`. Checked-in `reference/evidence/` contains the ten scene
PNGs, two intermediate-state PNGs, input/image digests and a cell-by-cell
cross-check. The run logs under `.bboxy/runs/t1-*.log` record actual validation
and the deferred iTerm2/Ghostty checkpoint.
