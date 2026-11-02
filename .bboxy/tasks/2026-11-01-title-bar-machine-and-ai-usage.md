---
artifacts:
- '[[01M6D19V63PD250TN3G7598HC2]]'
assignee: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
authorship:
  kind: seat
  process_generation: '1'
  public_key: +NU+89ZpBMFsX8bRpUDlYMkga5k1WphdhD6WumIHeWQ=
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
created: 2026-11-02T00:01:20.315Z
created_by: seat:01M6CQB5DB5RA6QDF9N162NF8S
delegated_by: seat:01M6CQB5DB5RA6QDF9N162NF8S
id: 01M6CRYZ7V2KVDBFFWPEE4SWXS
number: 2197
provenance:
  digest: sha256:29a7a5478b5fdd70b02b1a01cad7ca5736eba2b86b9ae602cdce01531bfa42e7
  frontmatter_digest: sha256:a42d269f84dcee08df32f7e2d7f901750e0f565001ef7ecb7b783c45ab62c2d0
  log: .bboxy/log/2026-11-02.jsonl#12730
  sig: kzJsGLyyY54yGDPWJqC905mrvitnusyrX0rs2459CKSd9yGpjEFdiwALRahQTAigDDEn48/x5nrOwdbJaJZ+Ag==
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
role: executor
status: claimed
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
title: 'Title bar: machine health (CPU/mem/disk) and AI account usage left, via a new usage crate'
type: task
updated: 2026-11-02T02:27:08.569Z
updated_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
verify: make test
---
# Title bar: machine health and AI account usage left

ROLE: EXECUTOR. Operator request 2026-11-01 (Oracle Vesper Greywind brief). Worktree: `/Users/feral/mydev/bboxy450-term-usage` (branch `title-bar-usage`, from origin/main).

## Outcome
The window title bar's right side, just left of the existing appearance (sun) button, shows a compact live strip:

`CPU 42% · MEM 71% · DISK 63%  │  claude 58% left · codex 80% left · grok 90% left`

- Machine: whole-machine CPU usage %, memory used %, disk used % of the volume holding `$HOME`.
- AI accounts: percent of quota **left** for each signed-in provider. Providers that aren't installed are hidden. A failed read shows `—` and a short reason in the hover/accessible label. Stale values are marked, never shown as current.
- Colour: muted normally, waiting colour at 80%+ used, errored colour at 95%+ (machine) or below 10% left (AI).

## New crate `crates/bboxy450-usage` (lib)
Reads usage for AI harnesses/providers. **Translated from Tachyon** (MIT, https://github.com/Gonzih/tachyon, `Sources/Tachyon/Providers/*.swift`, `Model/UsageModel.swift`). Put a "Translated from" header with that URL on each ported file and add a line to ATTRIBUTION.md.
- Mirror Tachyon's provider abstraction, one module per provider. v1: **Claude** (`ClaudeProvider.swift`: credential order env `CLAUDE_CODE_OAUTH_TOKEN` → macOS Keychain service `Claude Code-credentials` → `~/.claude/.credentials.json`; `GET https://api.anthropic.com/api/oauth/usage`), **Codex** (`CodexProvider.swift`: `$CODEX_HOME`/`~/.codex/auth.json`; `GET https://chatgpt.com/backend-api/wham/usage`; fall back to the latest rollout `token_count` rate-limit event, marked stale), **Grok** (`GrokProvider.swift`: auth.json entries issued by `https://auth.x.ai`; `GET https://cli-chat-proxy.grok.com/v1/billing?format=credits`). The trait must make adding omp/openrouter/cursor later a one-module change. Don't port them now.
- **Credentials are read-only.** Never refresh, rotate or write any token or credential file. No token in logs, errors or snapshots. On 401, return an auth-error state ("Run `codex login`" etc.).
- Use the HTTP client already in the workspace. Don't add a new one or a tokio runtime if one is already there.

## Machine stats
Use direct macOS calls (`host_statistics64`/`host_processor_info`, `statfs`) via the existing `libc`/mach usage in the tree, or a small vetted dependency if that's clearly cheaper. CPU % is a delta between two samples.

## Budgets (operator: the machine is CPU-bound)
- Machine sample every 2 s, AI providers every 5 min (plus manual refresh on click). Every read runs off the UI/frame thread with a timeout. The UI renders only the last snapshot and never waits on a read.
- The sampler itself must cost well under 0.5% of one core. Measure it and report the number.

## Tests (unattended; no operator, Keychain, network or real credentials)
- Provider decode and error states come from fixture JSON copied from Tachyon's `Tests/TachyonTests/*` (credit them), with injected fetchers and credential readers. Keychain and network are behind traits, faked in tests.
- Title bar: a `bbt-ui` test that the strip renders from a snapshot, hides missing providers, and shows `—` with the reason on error.
- Formal: add the crate to `verification/coverage.toml`. The poller (stale/refresh/timeout, account switch A → B → A must never show A's numbers under B) gets a small model plus a broken control, or an explicit implementation-only account if you can justify it in `verification/contracts.toml`.

## Scope
Allowed: `crates/bboxy450-usage/**`, `crates/bbt-ui/src/modules/title_bar.rs` and its tests, the title-bar wiring/model in `crates/bbt-ui-model`, the minimal `crates/bboxy450-term` wiring, `Cargo.toml`/`Cargo.lock`, `verification/**`, `ATTRIBUTION.md`, `.bboxy/reports/`.
Scope addendum (Oracle, 2026-11-01, on the Executor's new-crate gate request): also `scripts/layering.sh` (ORDER entry for bboxy450-usage), `.bboxy/specs/licensing.md` (the crate's row), the generated per-crate notice/license copies written by `make sync-licenses`, and any other file LAND.md rule 2 requires for a new crate.
Scope addendum 2 (Oracle, 2026-11-01, corpus gate): also import this task record itself, `.bboxy/tasks/2026-11-01-title-bar-machine-and-ai-usage.md`, copied byte-for-byte from the main checkout into the worktree and included in the amended commit, so report #2980 resolves its task.
Scope addendum 3 (Oracle, 2026-11-01, standing rule): import, byte-for-byte from the main checkout's `.bboxy/`, the latest copy of this task and any existing record (artifacts, reports, tasks) that the committed record graph requires to resolve. Example: `.bboxy/artifacts/01M6D19V63PD250TN3G7598HC2.md`. No per-file request is needed. Don't edit those records; just copy them.
Not allowed: no credential writes, no publishing, no installing over the operator's app, no changes to the lab reference (UI parity for this strip is advisory, not a gate).

## Verify
Iterate with targeted `cargo test -p bboxy450-usage -p bbt-ui` (two jobs, worktree-local target). The CPU is scarce: run full `make test` once on the final signed commit, then land with `sh scripts/land.sh --gate-log target/ci-local.log`. Report the landed main SHA, sampler CPU cost, and evidence per outcome bullet.
Route after the corpus-gate amendment (Oracle, 2026-11-01): keep the failed log unchanged as evidence. Amend the commit, signed, staging the exact paths. Then run `make test` once more on the amended SHA. `land.sh` accepts only a green normal log for the exact SHA, so this rerun is required, not optional. Then `sh scripts/land.sh --gate-log target/ci-local.log`. If that second full run fails on something unrelated to this change, report it rather than retrying.
Route amendment (Oracle, 2026-11-01, supersedes the route above): main moves during a full run, so use `make land-full` instead. It rebases first, runs the full `make test` once, then lands with the incremental rechecks and current formal receipts. This brief authorizes that landing. CPU rule (operator, 2026-11-01): while working, run only targeted checks for the crates and gates you touched. Start `make land-full` only when no other full run is going on the machine; check with `pgrep -fl "ci-local.sh|make test$"` and wait if one is. Amended (Oracle, 2026-11-01): requiring zero other runs starves with three swarms, so start `make land-full` once at most 3 other `ci-local.sh` runs are going, or after 45 minutes of waiting regardless. Same rule: if it fails on something unrelated to this change, report it rather than retrying.

## Communication
Quiet. One final `task_report`. Message the Oracle only for a blocker.
