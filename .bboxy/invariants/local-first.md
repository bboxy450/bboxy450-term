---
id: 01M5NKJXM80FHQP9G2A6WA9TKB
type: invariant
number: 210
title: "the mux depends on no remote transport until the cloud phase opens"
status: active
check: sh scripts/local-first.sh
cost: a transport that arrives before the cockpit works makes every local path a special case of a network one; local-only shortcuts are forbidden the other way, so the boundary must stay honest from both sides
links:
  - { to: "[[vision]]", rel: constrains }
  - { to: "[[d1-providers]]", rel: constrains }
  - { to: "[[mux-daemon]]", rel: constrains }
  - { to: "[[mantras]]", rel: mantra }
created: 2026-10-24T00:04:50Z
updated: 2026-10-24T00:04:50Z
---
# The mux depends on no remote transport until the cloud phase opens

**Property.** `cargo tree -p bboxy450-term-mux` carries no QUIC, ssh or cloud SDK crate
(`quinn`, `russh`, `ssh2`, `aws-sdk*`, `reqwest`, `tonic`, `hyper-util`,
`rustls`). Operator-authorized launch cut 2 (#71, 2026-10-30) wraps configured
commands in local ssh/mosh/et PTY clients; it adds no network SDK or remote
supervisor provider. Cloud provider connections remain separate ([[vision]] Cockpit first;
decisions #7 and #25; [[d1-providers]] §3 acceptance 5, promoted here). Local
first means the local path is the real one; the cloud phase lifts this
invariant by decision, never by a dependency slipping in.

**Check.** `sh scripts/local-first.sh` — exit 0 held, non-zero prints the
offending tree lines. Fast, in the default set.

**Cost of a violation.** Every local path becomes a special case of a network
one before the network exists, and the boundary [[vision]] forbids shortcuts
across stops being honest from both sides.

**Evidence.** `sh scripts/local-first.sh` — `local-first: held` at the commit
that adds it.

Owner: Corvin Duskmantle <ARCHITECT>.
