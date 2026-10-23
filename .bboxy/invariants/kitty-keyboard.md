---
id: 01M5MQHFVTTTQHQAE0GXFSS0B1
type: invariant
title: every key a pane receives is encoded once, by the kitty keyboard protocol against the seat's live mode
status: active
slow: true
gate: pending-k1
check: cargo test -p bbt-core --locked --lib kitty_keyboard_invariant -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a key that reaches the PTY by a second path, or encoded against a stale mode, is a terminal that lies to the program inside it
links:
  - { to: "[[terminal-core]]", rel: constrains }
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decision }
created: 2026-10-23T15:54:43Z
updated: 2026-10-23T15:54:43Z
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
cargo, so the check demands that exactly this named test ran and passed. K1
(Sable) creates `kitty_keyboard_invariant`, which must cover: encoding vectors
for every flag combination against the kitty spec, push/pop/query round trips,
legacy vectors with no flags, and a source scan proving no code outside
`keyboard.rs` turns a key event into PTY bytes. Fast (a unit test), so it joins
the default invariant set the day K1 lands: that commit removes `slow` and
`gate` from this file (the switch lands with the load, not before it).

**Cost of a violation.** A key that reaches the PTY by a second path, or is
encoded against a stale mode, is a terminal that lies to the program inside it:
a TUI that enabled flag 8 sees a bare byte where it expects an escape, or a
shell with no flags sees an escape where it expects a byte.

**Evidence.** None yet; the test does not exist. Until K1 lands this invariant
fails by design under `--slow` and is skipped by default.

Owner: Corvin Duskmantle <ARCHITECT>.
