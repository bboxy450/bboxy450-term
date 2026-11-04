# bboxy450-usage

Read-only machine and AI account usage for bboxy450-term.
Canonical source: https://github.com/bboxy450/bboxy450-term.
Credit policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md.

Claude, Codex and Grok providers and quota selection are translated from
[Tachyon](https://github.com/Gonzih/tachyon) (MIT; LICENSE-TACHYON).
Add a provider by implementing `Provider` in its own module and registering it
in `providers()`. Credentials and fetches are injected; no credential mutation
or token refresh API exists. Neither errors nor published values contain secrets.

Workers use the caller's Tokio runtime and existing reqwest client library.
Each provider has an independent single-flight worker (300 s between polls;
manual refresh coalesces); the machine worker samples every 2 s. Each read has
an 8 s visible deadline. Blocking OS work cannot be forcibly cancelled: one
timed-out call is retained per owner, and further calls are refused until it
finishes; its late result is discarded. This bounds stuck threads independently
of clicks. Credential revalidation occurs before publishing each HTTP result.
Account identity is observed on polling, not continuously between observations.
An identity/attempt generation fence clears old numbers before a refresh and
rejects timed-out, duplicate and A -> B -> A completions.

Anonymous Codex rollout history is shown only while signed out, explicitly
stale and account-unverified. An authenticated API failure shows a reason;
it cannot borrow another login's anonymous rollout. Providers without an
installation/configuration are absent. Keychain reads explicitly forbid
authentication UI. Tests only inject fake credential readers and fetchers.

On macOS CPU is whole-machine tick deltas (first sample warms up), memory is
active, wired and compressed physical pages (reclaimable file cache excluded),
and disk is allocated blocks on the volume containing HOME. Kernel/OS
semantics, IO latency and actual HTTP schemas require implementation checks;
the bounded UsagePoller model checks the owner transitions, not arbitrary Rust.
