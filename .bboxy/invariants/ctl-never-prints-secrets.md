---
id: 01M5Q8G3K8T7466MQH8JA5NEXG
type: invariant
number: 275
title: "ctl never prints a capability or a Keychain secret"
status: pending
slow: true
gate: pending-c0
check: cargo test -p bboxy450-term --locked --test ctl ctl_never_prints_secrets -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a capability in a reply, a log or an error is operator authority copied into every script's output and every pasted transcript; the door stops being a door
links:
  - { to: "[[ctl-cli]]", rel: constrains }
  - { to: "[[d260-ctl-authority]]", rel: decision }
  - { to: "[[d272-ctl-one-grammar]]", rel: decision }
created: "2026-10-24T15:29:32Z"
updated: "2026-10-24T15:29:32Z"
---
# ctl never prints a capability or a Keychain secret

**Property.** No `ctl` reply, stream line, `--human` rendering, error, hint or
log line contains the GUI capability (the 32 random bytes of the endpoint, in
any encoding), a seat capability, or any Keychain-held key material; `app
endpoint` prints the run directory, the socket path and the generation and
nothing else ([[ctl-cli]] §5). The same holds for `debug logs` and `debug
journal`, which redact those fields rather than skip the line.

**Check.** `cargo test -p bboxy450-term --locked --test ctl ctl_never_prints_secrets -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The test does not exist yet; C-0 creates it (every verb run
against a fixture app with a known capability, every output byte searched for
the capability in hex and base64 and for the fixture's key material) and its
commit removes `slow` and `gate` from this file.

**Cost of a violation.** A capability in a reply is operator authority copied
into every script's output and every pasted transcript; the door stops being a
door.

**Evidence.** None yet (pending C-0).

Owner: Corvin Duskmantle <ARCHITECT>.
