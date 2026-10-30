---
id: 01M5P696C140A6AGTP25HXT9BB
type: invariant
number: 261
title: "no gui.endpoint exists unless ctl is enabled, and the app's own seats are refused at the door"
status: active
check: cargo test -p bboxy450-term-mux --locked --test gui_endpoint ctl_endpoint_opt_in_and_ancestry_refusal -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: an endpoint file that exists when nobody asked is operator authority lying on disk for every same-user process; a seat admitted at the operator's door is a seat holding the operator's hand
links:
  - { to: "[[d260-ctl-authority]]", rel: decision }
  - { to: "[[e2e-acceptance]]", rel: constrains }
  - { to: "[[mux-daemon]]", rel: constrains }
created: "2026-10-24T05:31:34Z"
updated: "2026-10-25T10:31:36Z"
---
# No gui.endpoint exists unless ctl is enabled, and the app's own seats are refused at the door

**Property.** With `ctl` off (the default), no `gui.endpoint` file exists in any
run directory of the app, before, during or after a run. With `ctl` on, the file
exists only while logic runs, mode 0600, and is gone on exit; a connection whose
process ancestry contains this app's supervisor is refused with the reason
named, and the refusal is journaled; an admitted `ctl` operation is journaled
with kind `ctl` ([[d260-ctl-authority]]; [[e2e-acceptance]] §3).

**Check.** `cargo test -p bboxy450-term-mux --locked --test gui_endpoint ctl_endpoint_opt_in_and_ancestry_refusal -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The test does not exist yet; A-1 creates it (a run without the
flag writes no file; a run with it writes and removes one; a child spawned under
the supervisor is refused by name; an outside peer is admitted and audited as
`ctl`), and its commit removes `slow` and `gate` from this file.

**Cost of a violation.** An endpoint file that exists when nobody asked is
operator authority lying on disk for every same-user process; a seat admitted
at the operator's door is a seat holding the operator's hand. The guard is not
an OS boundary and the spec says so; this invariant keeps the guard honest, not
the OS.

**Evidence.** None yet (pending A-1).

**Status flipped to active (Architect, 2026-10-25):** its check `ctl_endpoint_opt_in_and_ancestry_refusal` runs in the default tier and passes on every gate of 2026-10-24 and 2026-10-25; the flip records what the tree already proves, found when `scripts/pending-flags.sh` refused a pending invariant without its skip flags.

Owner: Corvin Duskmantle <ARCHITECT>.
