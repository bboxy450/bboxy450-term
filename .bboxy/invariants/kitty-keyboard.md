---
id: 01M5MQHFVTTTQHQAE0GXFSS0B1
type: invariant
title: every key a pane receives is encoded once, by the kitty keyboard protocol against the seat's live mode
status: active
check: cargo test -p bboxy450-term --locked --lib kitty_keyboard_invariant -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a key that reaches the PTY by a second path, or encoded against a stale mode, is a terminal that lies to the program inside it
links:
  - { to: "[[terminal-core]]", rel: constrains }
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decision }
created: 2026-10-23T15:54:43Z
updated: 2026-10-23T16:20:00Z
---
# One key encoder, the kitty keyboard protocol

**Property** (operator, 2026-10-23: "for terminals we should adopt Kitty keyboard
protocol … document that as a core invariant"; decision #32). Every key event a
terminal pane receives is encoded by **the one app encoder**
(`crates/bboxy450-term/src/keyboard.rs`, a port of alacritty's
`input/keyboard.rs build_sequence`, Apache-2.0) against the **focused seat's live
`TermMode`** as carried in `Screen.modes`: the full kitty progressive-enhancement
stack (flags 1 disambiguate, 2 report events, 4 alternate keys, 8 all keys as
escapes, 16 associated text; push, pop and query answered from the seat's stack)
and correct legacy encoding when no flags are set. **No second key path to the
PTY:** paste, operator typing and injected text all pass through the same
encoder or are bytes the encoder never sees (bracketed paste is wrapped, not
encoded). bbt-core already keeps the stacks (`C/lib.rs:77` `kitty_keyboard: true`).

**Check.** `cargo test -p bboxy450-term --locked --lib kitty_keyboard_invariant -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The pipe is deliberate: a filter that matches no test exits 0 in
cargo, so the check demands that exactly this named test ran and passed. The
root library test `kitty_keyboard_invariant` covers all 32 flag combinations,
push/pop/query through a real `bbt_core::Term` and its cached `Screen.modes`,
legacy vectors with no flags, and a source guard restricting key interpretation
to `keyboard.rs` and the app's local shortcut router. It checks that both press
and release terminal branches delegate to the same encoder, and that the winit
callback forwards its opaque bytes against the focused live screen's modes.
This fast test is in the default invariant set.

**Cost of a violation.** A key that reaches the PTY by a second path, or is
encoded against a stale mode, is a terminal that lies to the program inside it:
a TUI that enabled flag 8 sees a bare byte where it expects an escape, or a
shell with no flags sees an escape where it expects a byte.

**Evidence.** `crates/bboxy450-term/src/lib.rs:kitty_keyboard_invariant` invokes
the encoding, core-stack, local-chord and source-guard checks in `keyboard.rs`.
The pure encoder accepts explicit Caps/Num bits. The macOS adapter reads current
Caps Lock from `NSEvent +modifierFlags` using the existing objc2 dependency;
macOS has no Num Lock toggle, so that bit is explicitly off. The `NumericPad`
origin flag is never interpreted as Num Lock. A deterministic vector checks
this distinction; no GUI key injection is used. Command remains local, including
repeat/release after Command itself was released, and decision #26 pane routing
is unchanged. K1's command outputs and exact-commit CI are in its run log.

Owner: Corvin Duskmantle <ARCHITECT>.
