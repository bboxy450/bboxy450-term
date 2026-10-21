# Assignment B — verification evidence

2026-10-21 · Nyx Copperwing <EXECUTOR>.

[[2026-10-21-agent-messaging-research]] · [[agent-messaging]] · [[messaging]]

## Scope and provenance

Wrote only `research/agent-messaging.md`, `specs/messaging.md` and this evidence record under `.kb/`. No application code, task board, vision, credentials or runtime database was edited by this assignment. Other agents are concurrently updating this repository; no repository commit is attributed to this Executor.

`icq --version` — exit 0, installed CLI 0.20.2.

Read `/Users/feral/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/icq-0.20.2/` with `rg -n` and `nl -ba … | sed -n …p`. Targeted source reads — exit 0. `Cargo.toml.orig:3` version 0.20.2; `:6` MIT; `.cargo_vcs_info.json:3` revision `6524c3b689dc70ff9d379192f8a57a62c0c047ef`. Local Iroh 1.3.0 and Quinn 0.11.12 manifests/source were also read; source versions remain explicit in [[agent-messaging]]. No package installation or build was needed.

Representative ICQ source fingerprint — Python SHA-256, exit 0: `54af9e15bfa2da1efd31a7f1f55aa8efe99e314d056bea19b4b443bc915befb4`. Algorithm: for each relative path in the following order, hash UTF-8 path, NUL, raw file bytes, NUL:

```text
Cargo.toml.orig
src/lib.rs
src/transport.rs
src/rendezvous.rs
src/rendezvous/relay.rs
src/identity.rs
src/session.rs
src/store.rs
src/wire.rs
src/discovery.rs
src/groups.rs
src/harness.rs
src/native.rs
src/runtime.rs
extensions/bridge.mjs
```

Browsed primary Quinn API and Iroh project/license/API documentation. The ICQ GitHub commit-tree fetch failed in the web fetcher; local packaged source and its VCS metadata supplied the ICQ evidence instead. No claim that a GitHub fetch succeeded.

## Checks already run

| Command/check | Actual result |
|---|---|
| `ls .kb/research/agent-messaging.md .kb/specs/messaging.md` | Exit 0; both requested files exist. |
| `grep -c 'Gap:' .kb/specs/messaging.md` | Exit 0; 6 explicit gap paragraphs in the initial spec. |
| Python range check of every explicit `ICQ/path:line` in research | Exit 0; 31 unique pointers, zero invalid files/line numbers. Range validation is not semantic proof. Mechanisms were read from the source. |
| `git diff --check` | Exit 0 for tracked changes at that check; newly created untracked files are not covered by Git's tracked diff. |
| `git log -1 --oneline` | Exit 0; observed `a3c8939 kb: task H subagent detection, iTerm reference screenshot` from the concurrent repository history. |

Exploratory failures: the first source search used a nonexistent `src/network*` shell glob — exit 1, corrected by listing and searching real source paths. A role/priority search limited to wire/runtime/store/attention files returned no matches — exit 1; later commands read the actual envelope and queue implementation. The initial repository had an unborn `main`, so the initial `git log` returned 128; later history was read successfully. None is a passing acceptance check.

## Deliberately not checked

No compiled embedding proof, local ICQ test suite, live NAT matrix, relay load test, 1000-seat benchmark, secure key-enrollment experiment, identity-provider integration, or generic PTY injection test. No live harness received a synthetic research probe. No ICQ private key, private group invitation, passphrase, raw inbox transcript or runtime SQLite content was copied into this repository.

The transport integration review is pending the Oracle-requested parallel `research/transport-and-identity.md`; the spec's transport section has not been written yet.

— Nyx Copperwing <EXECUTOR>
