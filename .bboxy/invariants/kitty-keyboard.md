---
id: 01M5MQHFVTTTQHQAE0GXFSS0B1
number: 149
type: invariant
title: every key a pane receives is encoded once, by the kitty keyboard protocol against the seat's live mode
status: active
check: cargo test -p bboxy450-term-core --locked --lib kitty_keyboard_invariant -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed' && cargo test -p bboxy450-term --locked --lib keyboard_adapter_uses_neutral_core_and_live_modes -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a key that reaches the PTY by a second path, or encoded against a stale mode, is a terminal that lies to the program inside it
links:
  - { to: "[[d32-kitty-keyboard]]", rel: decision }
  - { to: "[[d36-apache-code-placement]]", rel: decision }
  - { to: "[[terminal-core]]", rel: constrains }
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decision }
created: 2026-10-23T15:54:43Z
updated: 2026-10-26T04:41:03Z
---
# One key encoder, the kitty keyboard protocol

**Property** (operator, 2026-10-23: "for terminals we should adopt Kitty keyboard
protocol … document that as a core invariant"; decision [[d32-kitty-keyboard|#32]]). Every key event a
terminal pane receives is encoded by **the one encoder**
(`crates/bbt-core/src/keyboard.rs`, a port of alacritty's
`input/keyboard.rs build_sequence` at 5421470, Apache-2.0, which is why it
lives in the Apache-only terminal crate, decision [[d36-apache-code-placement|#36]]; the app maps the winit
key event to bbt-core's neutral key model and calls it) against the **focused seat's live
`TermMode`** as carried in `Screen.modes`: the full kitty progressive-enhancement
stack (flags 1 disambiguate, 2 report events, 4 alternate keys, 8 all keys as
escapes, 16 associated text; push, pop and query answered from the seat's stack)
and correct legacy encoding when no flags are set. **No second key path to the
PTY:** paste, operator typing and injected text all pass through the same
encoder or are bytes the encoder never sees (bracketed paste is wrapped, not
encoded). bbt-core already keeps the stacks (`C/lib.rs:77` `kitty_keyboard: true`).

**Check.** `cargo test -p bboxy450-term-core --locked --lib kitty_keyboard_invariant -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed' && cargo test -p bboxy450-term --locked --lib keyboard_adapter_uses_neutral_core_and_live_modes -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The pipe is deliberate: a filter that matches no test exits 0 in
cargo, so the check demands that exactly this named test ran and passed. The exact core-library test covers all 32 flag sets, a real Term's
push/pop/query and alternate-screen stacks and legacy vectors. The exact app-library
test retains the source guard over its single adapter/router. The app also tests Term -> Screen.modes ->
neutral encoder and keeps Command routing local. This fast check is active in
the default invariant set. Toolkit types and native lock reads stay in the app;
the Apache-only sequence builder and neutral model stay in bbt-core ([[d36-apache-code-placement|#36]]).

**Cost of a violation.** A key that reaches the PTY by a second path, or is
encoded against a stale mode, is a terminal that lies to the program inside it:
a TUI that enabled flag 8 sees a bare byte where it expects an escape, or a
shell with no flags sees an escape where it expects a byte.

**Evidence.** `crates/bbt-core/src/lib.rs:kitty_keyboard_invariant` calls
the flag matrix, real core-stack and legacy checks in
`crates/bbt-core/src/keyboard.rs`. `bboxy450-term` retains the source-guard
assertions in `src/keyboard.rs`, together with the thin adapter and its
published Screen mode bridge. The adapter reads Caps Lock from macOS
`NSEvent +modifierFlags`; macOS has no Num Lock toggle, so that bit is off.
`NumericPad` is not Num Lock; explicit neutral lock vectors test both lock bits.
No GUI key injection is used. K1's real commands, statuses and CI are retained
in its run logs. Apache attribution and the modified-by line remain in core.

Owner: Corvin Duskmantle <ARCHITECT>.
