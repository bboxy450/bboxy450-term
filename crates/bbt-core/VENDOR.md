# Vendored terminal model and parser

W1.1, 2026-10-21. Package `bbt-core` is distributed under Apache-2.0.

| Source | Exact revision | Original license | Local layout |
|---|---|---|---|
| [alacritty/alacritty](https://github.com/alacritty/alacritty/tree/542147067dd016fc5f49120ef0ba605a98a4e3a7/alacritty_terminal) `alacritty_terminal` 0.26.1-dev | `542147067dd016fc5f49120ef0ba605a98a4e3a7`, fetched at HEAD | Apache-2.0 | `src/vendor/alacritty/{grid,term,index.rs,event.rs,sync.rs}` |
| [alacritty/vte](https://github.com/alacritty/vte/tree/3b3da71c34cc1256c7e20981cf03f8eb95e08ffc) 0.15.0 | `3b3da71c34cc1256c7e20981cf03f8eb95e08ffc`, recorded by the released crate's `.cargo_vcs_info.json` | Apache-2.0 OR MIT | `src/vendor/vte/{mod.rs,params.rs,ansi.rs}` |

Upstream contributors include Joe Wilm and Christian Duerr; upstream attribution
is retained. `LICENSE-APACHE` and `LICENSE-MIT` accompany the sources. Neither
source distribution contains a NOTICE file. New fork code is by Boxy
<bboxy450@gmail.com>.

The repository-local ignored copies are `.kb/runs/clones/alacritty-w1-1` and
`.kb/runs/clones/vte-0.15.0`. The latter is copied from the installed registry
package, not from a moving vte Git branch. Upstream `lib.rs` maps to the local
module's `mod.rs`; the terminal model's module root lists only the retained
modules. Root reexports preserve the upstream crate-relative type paths.

## Kept and removed

Kept: grid storage, rows and reflow; term/cell/color and damage tracking; index,
event and sync helpers; the vte state machine and ANSI handler; their applicable
std unit tests.

Removed: `tty`, `event_loop`, `thread`, `vi_mode`, `selection`, `term/search`,
the terminal's vi/selection state and methods, and nine term tests for those
removed layers. No PTY, GUI, renderer or regular-expression search dependency
is in this crate. No-std-only parser branches/tests are omitted: this daemon
model always uses std and serde.

## Local changes

- `Term` in `src/lib.rs` provides the sized constructor, byte advancement, grid
  access, `viewport_only(&mut self)`, checkpoint/restore and unknown-OSC hook.
  `viewport_only` drops history from both screens, even with the primary inactive.
- Grid cursors and saved cursor templates are serialized. The snapshot also
  contains both grids, charset/tab/mode/palette/title/keyboard/config state,
  complete parser state and any synchronized-output buffer.
- Process-local timeout instants serialize as a remaining duration and rebase on
  deserialization. The owner still arms the sync timer and calls `stop_sync`;
  `sync_pending`/`sync_timeout` expose that state.
- `Handler::unhandled_osc` forwards unknown command parameters unchanged to
  `Event::Osc`; the wrapper invokes the hook and exposes the events. This covers
  OSC 7501/7502 without interpreting agent state in the terminal model.
- `take_events` drains title/clipboard/PTY-response and other events. Callbacks,
  event listeners and pending output effects are excluded from checkpoints;
  restoring retains the current runtime listener/hook and marks the grid damaged.
- Serde support is unconditional. `serde-big-array` serializes the 269-entry
  palette without dropping entries. Kitty keyboard support is enabled by the
  public constructor; terminal input encoding remains outside this crate.
- Two upstream parser assertions use `is_empty()` because serde_json adds a
  second `u8` equality implementation. The package-version test allows the
  fork's initial 0.0.1 rather than assuming alacritty's historical 0.10.1 floor.
  Doctest crate paths reflect the fork. Formatting uses workspace cargo fmt.

## Verification boundaries

The required tests cover a red SGR followed by 4096 bytes and a serialized
checkpoint, plus raw OSC 7501 delivery with BEL and ST. Additional tests cover
incomplete CSI/UTF-8/OSC, saved cursor and alternate screen, a pending 2026
buffer, preceding-character repetition and history removal. They exercise a
serde_json round trip into a fresh terminal, not just an in-memory clone.

The mux must wrap snapshots with version/cursor/integrity metadata and validate
that record before restore. This task defines no stable on-disk ABI, migration,
untrusted-checkpoint decoder, byte-ring eviction, PTY ownership, renderer,
protocol-wide resource policy or live harness compatibility certification.
The upstream parser remains 7-bit; 8-bit C1 OSC ingestion was not added.
Upstream reference-test corpora/vttest and cross-platform CI were not run here.
Actual commands, results and line accounting are in `.kb/runs/w1-1-*.log`.
