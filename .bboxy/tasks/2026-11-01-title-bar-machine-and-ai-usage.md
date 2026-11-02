---
artifacts:
- '[[01M6D19V63PD250TN3G7598HC2]]'
- '[[01M6D8RXNAT839QCTMVAD1C40Q]]'
- '[[01M6DD1MFGGH3AZNZMZMHXSSGM]]'
- '[[01M6DDR7EABZTE1QFRHWS6QR8T]]'
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
  digest: sha256:169fcc88a13b1d1ea224f383322bd162cbfdc266440cb35854c9842eff58384a
  frontmatter_digest: sha256:f0e1a6029c939b23bba9c85f5eebcfb935a336062aca7ead2a309c26ade6b615
  log: .bboxy/log/2026-11-02.jsonl#14959
  sig: DDbuiCO1YsWtdp+A5u35I5fXK8krWyeJ0rEkp4p2mEDB7vFhy/q0YEV6W8anZocyfWhjxVFg89vL3/LMgjX1Bg==
  signer: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
report:
  commit: a42653912d802ad1163f5bb6dba7db2a9a8272c0
  op_id: 01M6DD5551DJMYFRQDTT8817TX
  report: '[[reports/2026-11-01-title-bar-usage-retry-handoff.md]]'
  reported_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
  summary: 'The authorized make land-full retry ran once on unchanged signed 50e4a22d after the CPU count reached three. Ledger 34/34 and landing fixtures 140/140 passed; workspace build and strict workspace Clippy passed. The Rust suite then failed live::tests::knowledge_provider_events_publish_the_status_and_reread_tasks at live.rs:3771 before app launch: build the app before its real-process test. Its binary lookup uses the test executable''s grandparent; tests ran from target/private-build/debug/deps while the gate build target was the isolated slot target. live.rs is untouched by this feature. make land-full exited 2, land.sh 77; no landing/main SHA, and complete TLC/Yarn/formal and final smoke were not reached. Stopped without repair or further retry as #2197 requires. Current feature/provider/UI/native-model tests passed; sampler remains 0.014703% of one core. Full report #3172 supersedes #3165; signed reporting-only handoff a42653912d802ad1163f5bb6dba7db2a9a8272c0 is clean, ahead 3/behind 12, and final record checks exited 0.'
role: executor
status: reported
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
title: 'Title bar: machine health (CPU/mem/disk) and AI account usage left, via a new usage crate'
type: task
updated: 2026-11-02T06:04:40.041Z
updated_by: seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A
verify: make test
verify_result:
  command: make land-full
  evidence:
  - criterion: Machine usage strip and sampler budget
    reference: 50e4a22d:crates/bboxy450-usage/src/machine.rs:22,61,125,134 — processor deltas/Mach VM/HOME statfs; full retry machine counter test passed. /tmp/bboxy-usage-worker-cost.log SHA256 5a4b220a85b27ec65913f681e2967cf5c4ade777706712ca99d35d2856ff604c — 16 samples, 32.000964s, 0.004705 CPU s, 0.014703% one core; benchmark exit 0.
  - criterion: Read-only Claude/Codex/Grok quota-left providers and fixture coverage
    reference: 50e4a22d:crates/bboxy450-usage/src/{lib,io,claude,codex,grok}.rs and tests/providers.rs — credential precedence, redaction, sanitization, historical fallback, quota decoding, refresh/timeout and in-flight identity recheck; all seven provider tests passed in full retry at commands.log:5456-5467.
  - criterion: Independent cadences, timeout/stale/refresh/account-switch safety
    reference: 50e4a22d:crates/bboxy450-usage/src/poller.rs:46,59,84,135,200,275 — epoch/identity/pending fences and single-flight retained OS reads; all three machine/poller unit tests passed at retry commands.log:5446-5453.
  - criterion: Title-bar placement, hidden missing providers, error reasons, stale labels and colors
    reference: 50e4a22d:crates/bbt-ui/src/modules/title_bar.rs:126,238 and crates/bbt-ui-model/src/usage.rs; retry commands.log:5322 usage_strip_renders_snapshot_hides_absent_accounts_and_retains_error_reason passed; commands.log:2653 usage click and :2818 last-view event witnesses passed.
  - criterion: Formal inventory, native model and broken control
    reference: 50e4a22d:verification/{coverage,contracts,corpus}.toml and crates/bboxy450-term/tests/formal_contracts/usage_models.rs; retry commands.log:3004 project_contracts_and_broken_controls passed. Pinned Java TLC/Yarn complete gate not reached; full formal remains unverified.
  - criterion: Attribution and credential-free unattended verification
    reference: '50e4a22d:ATTRIBUTION.md, translated provider headers, LICENSE-TACHYON and tests/fixtures/README.md; prior make test-licenses exit 0 retained in #3165. No live provider/Keychain read, operator installation, release or lab modification.'
  - criterion: Current-source full gate and landing result
    reference: '/Users/feral/.local/state/bboxy450-term/landing/presubmits/50e4a22dd44a14bf5965b4bf330a347284a4104d-2693859f/commands.log SHA256 3237cd8ad1f7a67a095633f1fe96b3e2a295982395c7c77d35ee1459a6fc2fbe — make land-full exit 2, land.sh 77. App target: 202 passed/1 failed/6 ignored. Failure at lines 2828-2840, live.rs:3771 missing binary preflight; no app launch, no push, no landed main SHA.'
  - criterion: Signed committed handoff and record verification
    reference: a42653912d802ad1163f5bb6dba7db2a9a8272c0 signature G, Boxy key 844ECC0587DE7AB0; git status --porcelain empty; only three handoff records differ from 50e4a22d. make test-kb-reports exit 0; target/usage-authorized-retry-record-check.log SHA256 45163cf0b482974ab0b0c509fe8214c8022d00bc025587cda791ed7f03865308.
  exit_code: 2
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
Retry ruling (Oracle, 2026-11-01, after report #3165): the first `land-full` failed only on `scripts/test-resource-ledger.py:222` (expected 143, got -15). That's a load flake: it passed 3/3 when run alone on this branch, and the branch doesn't touch it. One more `make land-full` on 50e4a22d is authorized, under the same CPU cap. If anything fails again, stop and report; no further retry and no out-of-scope fix.
Route change (Oracle, 2026-11-01, after report #3172): `make land-full` failed twice, both times inside its separate build folder, on tests this change doesn't touch. The second failure, live.rs:3771, looks for the app binary in a different build folder from the one land-full builds. Drop land-full. Use the route other branches are landing with today: rebase 50e4a22d's branch onto current origin/main (signed), run `make test` once in this worktree's own target under the CPU cap, then `sh scripts/land.sh --gate-log target/ci-local.log`. One attempt. If a test this change touches fails, stop and report. If an untouched test fails, report it with the log line and the Oracle decides.
Handover (Oracle, 2026-11-02): the first Executor seat stalled when its Codex auth expired (06:14 UTC). This task is reassigned to a fresh Codex seat, which now owns `/Users/feral/mydev/bboxy450-term-usage` alone. Resume from branch head c5aa7df2 and carry out the Route change above. The implementation and earlier records are already committed; don't redo them.

## Communication
Quiet. One final `task_report`. Message the Oracle only for a blocker.


## Report from seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A

Implemented the machine/Claude/Codex/Grok title-bar strip with read-only providers, bounded independent workers, snapshot-only UI, error/stale labels, refresh and native poller model/broken control. Source 4f3e1fa8; signed handoff 50e4a22dd44a14bf5965b4bf330a347284a4104d, clean worktree, two signed commits ahead of origin/main. Targeted provider/UI/app/native-contract, Clippy, license and final record checks exited 0; scheduled sampler measured 0.014703% of one core. The single make land-full attempt exited 2 (land.sh 77): untouched scripts/test-resource-ledger.py:222 expected 143 but received -15. Per the brief, I stopped without retry or out-of-scope repair. No landing or main SHA; exact-source complete TLC/Yarn/formal and smoke remain unverified. Full report #3165 supersedes #2980 and retains command logs/hashes.

```json
{
  "commit": "50e4a22dd44a14bf5965b4bf330a347284a4104d",
  "op_id": "01M6D8WWVQ2A1291ZFX18H01QF",
  "report": "[[reports/2026-11-01-title-bar-usage-gate-handoff.md]]",
  "verify_result": {
    "command": "make land-full",
    "evidence": [
      {
        "criterion": "Machine CPU/memory/HOME-volume disk percentages",
        "reference": "4f3e1fa8:crates/bboxy450-usage/src/machine.rs:22,61,125,134 — CPU deltas, Mach VM and statfs; unit/counter-wrap witnesses exit 0."
      },
      {
        "criterion": "Claude, Codex and Grok quota left, extensible provider abstraction",
        "reference": "4f3e1fa8:crates/bboxy450-usage/src/{lib,claude,codex,grok}.rs; tests/providers.rs — injected Tachyon fixtures, credential precedence and decode/error witnesses exit 0."
      },
      {
        "criterion": "Read-only credentials and secret-free failures",
        "reference": "4f3e1fa8:crates/bboxy450-usage/src/lib.rs:39,51 and io.rs — no mutation/refresh API, redacted Debug, fixed errors, interaction-disabled Keychain, bounded redirect-disabled HTTP."
      },
      {
        "criterion": "Background cadence, deadlines, manual refresh and sampler CPU budget",
        "reference": "4f3e1fa8:crates/bboxy450-usage/src/poller.rs:84,135,200; /tmp/bboxy-usage-worker-cost.log SHA256 5a4b220a85b27ec65913f681e2967cf5c4ade777706712ca99d35d2856ff604c — 16 scheduled samples/32.000964s, 0.004705 CPU s = 0.014703% of one core; exit 0."
      },
      {
        "criterion": "Title-bar position, hidden missing providers, error reason, stale labels and threshold colors",
        "reference": "4f3e1fa8:crates/bbt-ui/src/modules/title_bar.rs:126,238 and crates/bbt-ui-model/src/usage.rs; targeted usage/UI tests exit 0; app usage_ click and startup-snapshot witnesses exit 0."
      },
      {
        "criterion": "Stale/refresh/timeout/account-switch formal inventory and broken control",
        "reference": "4f3e1fa8:verification/{coverage,contracts,corpus}.toml and crates/bboxy450-term/tests/formal_contracts/usage_models.rs — native model/broken-control test exit 0; full TLC/Yarn gate not reached, therefore unverified."
      },
      {
        "criterion": "Attribution and unattended fixture-based tests",
        "reference": "4f3e1fa8:ATTRIBUTION.md, crates/bboxy450-usage/LICENSE-TACHYON, source headers and tests/fixtures/README.md; make test-licenses exit 0; no live credentials, Keychain/network requests, operator app install or lab change."
      },
      {
        "criterion": "Final committed report graph and signed worktree",
        "reference": "50e4a22dd44a14bf5965b4bf330a347284a4104d — signature G, Boxy key 844ECC0587DE7AB0; git status --porcelain empty. make test-kb-reports exit 0; target/usage-final-record-check.log SHA256 cc1e57a75529e5e4c6a5a10b8b10489e5f0f9f760f0e07a9e693f6ef8ade217b."
      },
      {
        "criterion": "Full gate and landing result",
        "reference": "make land-full exit 2; scripts/land.sh exit 77 presubmit_failed. /Users/feral/.local/state/bboxy450-term/landing/presubmits/4f3e1fa89302b0323cbc9d16a7a6ddd2e6885ae8-aaa0e376/commands.log SHA256 676534aa6bd8a750953bdfcbaa610b1bd20a5f1a9b70f5deed62a5201b3bb19b — layering held, 140 landing fixtures passed, resource-ledger test failed 143 versus -15. Stopped without retry; no landed main SHA."
      }
    ],
    "exit_code": 2
  }
}
```


## Report from seat:01M6CVHSYF1RWHXXBHGJ3V5Q7A

The authorized make land-full retry ran once on unchanged signed 50e4a22d after the CPU count reached three. Ledger 34/34 and landing fixtures 140/140 passed; workspace build and strict workspace Clippy passed. The Rust suite then failed live::tests::knowledge_provider_events_publish_the_status_and_reread_tasks at live.rs:3771 before app launch: build the app before its real-process test. Its binary lookup uses the test executable's grandparent; tests ran from target/private-build/debug/deps while the gate build target was the isolated slot target. live.rs is untouched by this feature. make land-full exited 2, land.sh 77; no landing/main SHA, and complete TLC/Yarn/formal and final smoke were not reached. Stopped without repair or further retry as #2197 requires. Current feature/provider/UI/native-model tests passed; sampler remains 0.014703% of one core. Full report #3172 supersedes #3165; signed reporting-only handoff a42653912d802ad1163f5bb6dba7db2a9a8272c0 is clean, ahead 3/behind 12, and final record checks exited 0.

```json
{
  "commit": "a42653912d802ad1163f5bb6dba7db2a9a8272c0",
  "op_id": "01M6DD5551DJMYFRQDTT8817TX",
  "report": "[[reports/2026-11-01-title-bar-usage-retry-handoff.md]]",
  "verify_result": {
    "command": "make land-full",
    "evidence": [
      {
        "criterion": "Machine usage strip and sampler budget",
        "reference": "50e4a22d:crates/bboxy450-usage/src/machine.rs:22,61,125,134 — processor deltas/Mach VM/HOME statfs; full retry machine counter test passed. /tmp/bboxy-usage-worker-cost.log SHA256 5a4b220a85b27ec65913f681e2967cf5c4ade777706712ca99d35d2856ff604c — 16 samples, 32.000964s, 0.004705 CPU s, 0.014703% one core; benchmark exit 0."
      },
      {
        "criterion": "Read-only Claude/Codex/Grok quota-left providers and fixture coverage",
        "reference": "50e4a22d:crates/bboxy450-usage/src/{lib,io,claude,codex,grok}.rs and tests/providers.rs — credential precedence, redaction, sanitization, historical fallback, quota decoding, refresh/timeout and in-flight identity recheck; all seven provider tests passed in full retry at commands.log:5456-5467."
      },
      {
        "criterion": "Independent cadences, timeout/stale/refresh/account-switch safety",
        "reference": "50e4a22d:crates/bboxy450-usage/src/poller.rs:46,59,84,135,200,275 — epoch/identity/pending fences and single-flight retained OS reads; all three machine/poller unit tests passed at retry commands.log:5446-5453."
      },
      {
        "criterion": "Title-bar placement, hidden missing providers, error reasons, stale labels and colors",
        "reference": "50e4a22d:crates/bbt-ui/src/modules/title_bar.rs:126,238 and crates/bbt-ui-model/src/usage.rs; retry commands.log:5322 usage_strip_renders_snapshot_hides_absent_accounts_and_retains_error_reason passed; commands.log:2653 usage click and :2818 last-view event witnesses passed."
      },
      {
        "criterion": "Formal inventory, native model and broken control",
        "reference": "50e4a22d:verification/{coverage,contracts,corpus}.toml and crates/bboxy450-term/tests/formal_contracts/usage_models.rs; retry commands.log:3004 project_contracts_and_broken_controls passed. Pinned Java TLC/Yarn complete gate not reached; full formal remains unverified."
      },
      {
        "criterion": "Attribution and credential-free unattended verification",
        "reference": "50e4a22d:ATTRIBUTION.md, translated provider headers, LICENSE-TACHYON and tests/fixtures/README.md; prior make test-licenses exit 0 retained in #3165. No live provider/Keychain read, operator installation, release or lab modification."
      },
      {
        "criterion": "Current-source full gate and landing result",
        "reference": "/Users/feral/.local/state/bboxy450-term/landing/presubmits/50e4a22dd44a14bf5965b4bf330a347284a4104d-2693859f/commands.log SHA256 3237cd8ad1f7a67a095633f1fe96b3e2a295982395c7c77d35ee1459a6fc2fbe — make land-full exit 2, land.sh 77. App target: 202 passed/1 failed/6 ignored. Failure at lines 2828-2840, live.rs:3771 missing binary preflight; no app launch, no push, no landed main SHA."
      },
      {
        "criterion": "Signed committed handoff and record verification",
        "reference": "a42653912d802ad1163f5bb6dba7db2a9a8272c0 signature G, Boxy key 844ECC0587DE7AB0; git status --porcelain empty; only three handoff records differ from 50e4a22d. make test-kb-reports exit 0; target/usage-authorized-retry-record-check.log SHA256 45163cf0b482974ab0b0c509fe8214c8022d00bc025587cda791ed7f03865308."
      }
    ],
    "exit_code": 2
  }
}
```
