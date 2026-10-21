# Assignment B — verification evidence

2026-10-21 · Nyx Copperwing <EXECUTOR>.

[[2026-10-21-agent-messaging-research]] · [[agent-messaging]] · [[messaging]]

## Scope and provenance

Repository writes are limited to `research/agent-messaging.md`, `specs/messaging.md` and this evidence record under `.kb/`. No application code, task board, vision or credentials were edited. ICQ progress messages used the supported CLI; its runtime database was not opened or manually modified. Other agents are concurrently updating this repository; no repository commit is attributed to this Executor.

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

At 11:59 PDT, `research/transport-and-identity.md` was still absent. Oracle follow-up `8f4caeac-80f0-4d87-b9bd-37be82cd1b1d` removed that dependency and explicitly requested the decided Quinn + custom punch + E2E relay baseline, with a conditional Iroh alternative. A provisional transport section was written; the parallel review was resolved below.

The parallel review subsequently landed in commit `b2659e5` and was read in full before handoff. Its Quinn/noq and self-hosting findings were checked against local Iroh 1.3.0 `Cargo.toml.orig:7`, `:44`, `src/endpoint.rs:596`, `:1993` and the primary self-hosting guide. The transport section was amended; broader company and OAuth assertions were not copied as independently verified facts. The updated vision's Google-in-v0.1 / Apple-deferred scope is acknowledged, without changing the account-key proposal.

## Final acceptance after integration

| Command/check | Actual result |
|---|---|
| `ls .kb/research/agent-messaging.md .kb/specs/messaging.md` | Exit 0, both present. |
| `grep -c 'Gap:' .kb/specs/messaging.md` | Exit 0, 7 explicit gaps. |
| `rg -n '^## [1-6]\.' .kb/research/agent-messaging.md` | Exit 0, all six question sections at lines 13, 30, 45, 59, 72, 83. |
| Python qualified-source-pointer and whitespace check on both deliverables | Exit 0, 35 unique `ICQ/`, `Quinn/`, `Iroh/` file:line pointers; zero invalid file/line references or formatting failures. Both files end in newline. |
| `git diff --check` | Exit 0 after transport integration. |
| `git status --short` / `git log -1 --format='%h %s'` | Exit 0; HEAD `b2659e5`, research already in HEAD; final spec and this evidence record modified. Other agents' untracked research/lab files were left untouched. No commit created by this Executor. |

Acceptance mapping: the six research sections answer every brief question or mark the gap; transport and identity contain local ICQ source pointers; spec lines 7 and 15 choose rebuild-on-Quinn and state its ownership cost; lines 55 and 84 define the signed role envelope and receiver-side priority policy. The account chain, scheduler and PTY handling are proposals, not implemented features.

— Nyx Copperwing <EXECUTOR>
