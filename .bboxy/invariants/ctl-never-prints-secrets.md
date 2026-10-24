---
id: 01M5Q8G3K8T7466MQH8JA5NEXG
type: invariant
number: 275
title: "ctl never prints a capability or a Keychain secret"
status: active
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
— exit 0 held. C-0 created it: every noun-verb verb and every flat alias, on
success and on failure, runs against a fixture app; stdout and stderr are
searched for the capability as lowercase and uppercase hex, standard and
URL-safe base64 with and without padding, and serde's byte array.

**Cost of a violation.** A capability in a reply is operator authority copied
into every script's output and every pasted transcript; the door stops being a
door.

**Evidence.** `crates/bboxy450-term/tests/ctl.rs` `ctl_never_prints_secrets`
(C-0, 2026-10-24, Seraph); `app endpoint` prints `run_dir`, `path` and
`generation` only (`ctl_noun_verb_grammar_replies_and_errors`). The fixture runs
on memory custody, so no Keychain material exists to search for; the
capability is the secret every reply could carry. Held at the C-0 gate (`.bboxy/runs/ctl-c0-ci.log`,
e7cfe4fd, `ctl_never_prints_secrets ... ok`, invariants 20/0) and on the merged
tree (Vesper, 19/0 with this check passing).

Owner: Corvin Duskmantle <ARCHITECT>.
